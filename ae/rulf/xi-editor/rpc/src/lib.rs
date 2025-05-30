//! Generic RPC handling (used for both front end and plugin communication).
//!
//! The RPC protocol is based on [JSON-RPC](http://www.jsonrpc.org/specification),
//! but with some modifications. Unlike JSON-RPC 2.0, requests and notifications
//! are allowed in both directions, rather than imposing client and server roles.
//! Further, the batch form is not supported.
//!
//! Because these changes make the protocol not fully compliant with the spec,
//! the `"jsonrpc"` member is omitted from request and response objects.
#![allow(clippy::boxed_local, clippy::or_fun_call)]
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate crossbeam_utils;
extern crate serde;
extern crate xi_trace;
#[macro_use]
extern crate log;
mod error;
mod parse;
pub mod test_utils;
use std::cmp;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use serde::de::DeserializeOwned;
use serde_json::Value;
use xi_trace::{trace, trace_block, trace_block_payload, trace_payload};
pub use crate::error::{Error, ReadError, RemoteError};
use crate::parse::{Call, MessageReader, Response, RpcObject};
/// The maximum duration we will block on a reader before checking for an task.
const MAX_IDLE_WAIT: Duration = Duration::from_millis(5);
/// An interface to access the other side of the RPC channel. The main purpose
/// is to send RPC requests and notifications to the peer.
///
/// A single shared `RawPeer` exists for each `RpcLoop`; a reference can
/// be taken with `RpcLoop::get_peer()`.
///
/// In general, `RawPeer` shouldn't be used directly, but behind a pointer as
/// the `Peer` trait object.
pub struct RawPeer<W: Write + 'static>(Arc<RpcState<W>>);
/// The `Peer` trait represents the interface for the other side of the RPC
/// channel. It is intended to be used behind a pointer, a trait object.
pub trait Peer: Send + 'static {
    /// Used to implement `clone` in an object-safe way.
    /// For an explanation on this approach, see
    /// [this thread](https://users.rust-lang.org/t/solved-is-it-possible-to-clone-a-boxed-trait-object/1714/6).
    fn box_clone(&self) -> Box<dyn Peer>;
    /// Sends a notification (asynchronous RPC) to the peer.
    fn send_rpc_notification(&self, method: &str, params: &Value);
    /// Sends a request asynchronously, and the supplied callback will
    /// be called when the response arrives.
    ///
    /// `Callback` is an alias for `FnOnce(Result<Value, Error>)`; it must
    /// be boxed because trait objects cannot use generic paramaters.
    fn send_rpc_request_async(&self, method: &str, params: &Value, f: Box<dyn Callback>);
    /// Sends a request (synchronous RPC) to the peer, and waits for the result.
    fn send_rpc_request(&self, method: &str, params: &Value) -> Result<Value, Error>;
    /// Determines whether an incoming request (or notification) is
    /// pending. This is intended to reduce latency for bulk operations
    /// done in the background.
    fn request_is_pending(&self) -> bool;
    /// Adds a token to the idle queue. When the runloop is idle and the
    /// queue is not empty, the handler's `idle` fn will be called
    /// with the earliest added token.
    fn schedule_idle(&self, token: usize);
    /// Like `schedule_idle`, with the guarantee that the handler's `idle`
    /// fn will not be called _before_ the provided `Instant`.
    ///
    /// # Note
    ///
    /// This is not intended as a high-fidelity timer. Regular RPC messages
    /// will always take priority over an idle task.
    fn schedule_timer(&self, after: Instant, token: usize);
}
/// The `Peer` trait object.
pub type RpcPeer = Box<dyn Peer>;
pub struct RpcCtx {
    peer: RpcPeer,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// An RPC command.
///
/// This type is used as a placeholder in various places, and can be
/// used by clients as a catchall type for implementing `MethodHandler`.
pub struct RpcCall {
    pub method: String,
    pub params: Value,
}
/// A trait for types which can handle RPCs.
///
/// Types which implement `MethodHandler` are also responsible for implementing
/// `Parser`; `Parser` is provided when Self::Notification and Self::Request
/// can be used with serde::DeserializeOwned.
pub trait Handler {
    type Notification: DeserializeOwned;
    type Request: DeserializeOwned;
    fn handle_notification(&mut self, ctx: &RpcCtx, rpc: Self::Notification);
    fn handle_request(
        &mut self,
        ctx: &RpcCtx,
        rpc: Self::Request,
    ) -> Result<Value, RemoteError>;
    #[allow(unused_variables)]
    fn idle(&mut self, ctx: &RpcCtx, token: usize) {}
}
pub trait Callback: Send {
    fn call(self: Box<Self>, result: Result<Value, Error>);
}
impl<F: Send + FnOnce(Result<Value, Error>)> Callback for F {
    fn call(self: Box<F>, result: Result<Value, Error>) {
        (*self)(result)
    }
}
/// A helper type which shuts down the runloop if a panic occurs while
/// handling an RPC.
struct PanicGuard<'a, W: Write + 'static>(&'a RawPeer<W>);
impl<'a, W: Write + 'static> Drop for PanicGuard<'a, W> {
    fn drop(&mut self) {
        if thread::panicking() {
            error!("panic guard hit, closing runloop");
            self.0.disconnect();
        }
    }
}
trait IdleProc: Send {
    fn call(self: Box<Self>, token: usize);
}
impl<F: Send + FnOnce(usize)> IdleProc for F {
    fn call(self: Box<F>, token: usize) {
        (*self)(token)
    }
}
enum ResponseHandler {
    Chan(mpsc::Sender<Result<Value, Error>>),
    Callback(Box<dyn Callback>),
}
impl ResponseHandler {
    fn invoke(self, result: Result<Value, Error>) {
        match self {
            ResponseHandler::Chan(tx) => {
                let _ = tx.send(result);
            }
            ResponseHandler::Callback(f) => f.call(result),
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Timer {
    fire_after: Instant,
    token: usize,
}
struct RpcState<W: Write> {
    rx_queue: Mutex<VecDeque<Result<RpcObject, ReadError>>>,
    rx_cvar: Condvar,
    writer: Mutex<W>,
    id: AtomicUsize,
    pending: Mutex<BTreeMap<usize, ResponseHandler>>,
    idle_queue: Mutex<VecDeque<usize>>,
    timers: Mutex<BinaryHeap<Timer>>,
    needs_exit: AtomicBool,
    is_blocked: AtomicBool,
}
/// A structure holding the state of a main loop for handling RPC's.
pub struct RpcLoop<W: Write + 'static> {
    reader: MessageReader,
    peer: RawPeer<W>,
}
impl<W: Write + Send> RpcLoop<W> {
    /// Creates a new `RpcLoop` with the given output stream (which is used for
    /// sending requests and notifications, as well as responses).
    pub fn new(writer: W) -> Self {
        let rpc_peer = RawPeer(
            Arc::new(RpcState {
                rx_queue: Mutex::new(VecDeque::new()),
                rx_cvar: Condvar::new(),
                writer: Mutex::new(writer),
                id: AtomicUsize::new(0),
                pending: Mutex::new(BTreeMap::new()),
                idle_queue: Mutex::new(VecDeque::new()),
                timers: Mutex::new(BinaryHeap::new()),
                needs_exit: AtomicBool::new(false),
                is_blocked: AtomicBool::new(false),
            }),
        );
        RpcLoop {
            reader: MessageReader::default(),
            peer: rpc_peer,
        }
    }
    /// Gets a reference to the peer.
    pub fn get_raw_peer(&self) -> RawPeer<W> {
        self.peer.clone()
    }
    /// Starts the event loop, reading lines from the reader until EOF,
    /// or an error occurs.
    ///
    /// Returns `Ok()` in the EOF case, otherwise returns the
    /// underlying `ReadError`.
    ///
    /// # Note:
    /// The reader is supplied via a closure, as basically a workaround
    /// so that the reader doesn't have to be `Send`. Internally, the
    /// main loop starts a separate thread for I/O, and at startup that
    /// thread calls the given closure.
    ///
    /// Calls to the handler happen on the caller's thread.
    ///
    /// Calls to the handler are guaranteed to preserve the order as
    /// they appear on on the channel. At the moment, there is no way
    /// for there to be more than one incoming request to be outstanding.
    pub fn mainloop<R, RF, H>(
        &mut self,
        rf: RF,
        handler: &mut H,
    ) -> Result<(), ReadError>
    where
        R: BufRead,
        RF: Send + FnOnce() -> R,
        H: Handler,
    {
        let exit = crossbeam_utils::thread::scope(|scope| {
                let peer = self.get_raw_peer();
                peer.reset_needs_exit();
                let ctx = RpcCtx {
                    peer: Box::new(peer.clone()),
                };
                scope
                    .spawn(move |_| {
                        let mut stream = rf();
                        loop {
                            if self.peer.needs_exit() {
                                trace("read loop exit", &["rpc"]);
                                break;
                            }
                            let json = match self.reader.next(&mut stream) {
                                Ok(json) => json,
                                Err(err) => {
                                    if self.peer.0.is_blocked.load(Ordering::Acquire) {
                                        error!("failed to parse response json: {}", err);
                                        self.peer.disconnect();
                                    }
                                    self.peer.put_rx(Err(err));
                                    break;
                                }
                            };
                            if json.is_response() {
                                let id = json.get_id().unwrap();
                                let _resp = trace_block_payload(
                                    "read loop response",
                                    &["rpc"],
                                    format!("{}", id),
                                );
                                match json.into_response() {
                                    Ok(resp) => {
                                        let resp = resp.map_err(Error::from);
                                        self.peer.handle_response(id, resp);
                                    }
                                    Err(msg) => {
                                        error!("failed to parse response: {}", msg);
                                        self.peer.handle_response(id, Err(Error::InvalidResponse));
                                    }
                                }
                            } else {
                                self.peer.put_rx(Ok(json));
                            }
                        }
                    });
                loop {
                    let _guard = PanicGuard(&peer);
                    let read_result = next_read(&peer, handler, &ctx);
                    let _trace = trace_block("main got msg", &["rpc"]);
                    let json = match read_result {
                        Ok(json) => json,
                        Err(err) => {
                            trace_payload("main loop err", &["rpc"], err.to_string());
                            if let Some(idle_token) = peer.try_get_idle() {
                                handler.idle(&ctx, idle_token);
                            }
                            peer.disconnect();
                            return err;
                        }
                    };
                    let method = json.get_method().map(String::from);
                    match json.into_rpc::<H::Notification, H::Request>() {
                        Ok(Call::Request(id, cmd)) => {
                            let _t = trace_block_payload(
                                "handle request",
                                &["rpc"],
                                method.unwrap(),
                            );
                            let result = handler.handle_request(&ctx, cmd);
                            peer.respond(result, id);
                        }
                        Ok(Call::Notification(cmd)) => {
                            let _t = trace_block_payload(
                                "handle notif",
                                &["rpc"],
                                method.unwrap(),
                            );
                            handler.handle_notification(&ctx, cmd);
                        }
                        Ok(Call::InvalidRequest(id, err)) => peer.respond(Err(err), id),
                        Err(err) => {
                            trace_payload("read loop exit", &["rpc"], err.to_string());
                            peer.disconnect();
                            return ReadError::UnknownRequest(err);
                        }
                    }
                }
            })
            .unwrap();
        if exit.is_disconnect() { Ok(()) } else { Err(exit) }
    }
}
/// Returns the next read result, checking for idle work when no
/// result is available.
fn next_read<W, H>(
    peer: &RawPeer<W>,
    handler: &mut H,
    ctx: &RpcCtx,
) -> Result<RpcObject, ReadError>
where
    W: Write + Send,
    H: Handler,
{
    loop {
        if let Some(result) = peer.try_get_rx() {
            return result;
        }
        let time_to_next_timer = match peer.check_timers() {
            Some(Ok(token)) => {
                do_idle(handler, ctx, token);
                continue;
            }
            Some(Err(duration)) => Some(duration),
            None => None,
        };
        if let Some(idle_token) = peer.try_get_idle() {
            do_idle(handler, ctx, idle_token);
            continue;
        }
        let idle_timeout = time_to_next_timer
            .unwrap_or(MAX_IDLE_WAIT)
            .min(MAX_IDLE_WAIT);
        if let Some(result) = peer.get_rx_timeout(idle_timeout) {
            return result;
        }
    }
}
fn do_idle<H: Handler>(handler: &mut H, ctx: &RpcCtx, token: usize) {
    let _trace = trace_block_payload("do_idle", &["rpc"], format!("token: {}", token));
    handler.idle(ctx, token);
}
impl RpcCtx {
    pub fn get_peer(&self) -> &RpcPeer {
        &self.peer
    }
    /// Schedule the idle handler to be run when there are no requests pending.
    pub fn schedule_idle(&self, token: usize) {
        self.peer.schedule_idle(token)
    }
}
impl<W: Write + Send + 'static> Peer for RawPeer<W> {
    fn box_clone(&self) -> Box<dyn Peer> {
        Box::new((*self).clone())
    }
    fn send_rpc_notification(&self, method: &str, params: &Value) {
        let _trace = trace_block_payload("send notif", &["rpc"], method.to_owned());
        if let Err(e) = self.send(&json!({ "method" : method, "params" : params, })) {
            error!("send error on send_rpc_notification method {}: {}", method, e);
        }
    }
    fn send_rpc_request_async(
        &self,
        method: &str,
        params: &Value,
        f: Box<dyn Callback>,
    ) {
        let _trace = trace_block_payload("send req async", &["rpc"], method.to_owned());
        self.send_rpc_request_common(method, params, ResponseHandler::Callback(f));
    }
    fn send_rpc_request(&self, method: &str, params: &Value) -> Result<Value, Error> {
        let _trace = trace_block_payload("send req sync", &["rpc"], method.to_owned());
        self.0.is_blocked.store(true, Ordering::Release);
        let (tx, rx) = mpsc::channel();
        self.send_rpc_request_common(method, params, ResponseHandler::Chan(tx));
        rx.recv().unwrap_or(Err(Error::PeerDisconnect))
    }
    fn request_is_pending(&self) -> bool {
        let queue = self.0.rx_queue.lock().unwrap();
        !queue.is_empty()
    }
    fn schedule_idle(&self, token: usize) {
        self.0.idle_queue.lock().unwrap().push_back(token);
    }
    fn schedule_timer(&self, after: Instant, token: usize) {
        self.0.timers.lock().unwrap().push(Timer { fire_after: after, token });
    }
}
impl<W: Write> RawPeer<W> {
    fn send(&self, v: &Value) -> Result<(), io::Error> {
        let _trace = trace_block("send", &["rpc"]);
        let mut s = serde_json::to_string(v).unwrap();
        s.push('\n');
        self.0.writer.lock().unwrap().write_all(s.as_bytes())
    }
    fn respond(&self, result: Response, id: u64) {
        let mut response = json!({ "id" : id });
        match result {
            Ok(result) => response["result"] = result,
            Err(error) => response["error"] = json!(error),
        };
        if let Err(e) = self.send(&response) {
            error!("error {} sending response to RPC {:?}", e, id);
        }
    }
    fn send_rpc_request_common(
        &self,
        method: &str,
        params: &Value,
        rh: ResponseHandler,
    ) {
        let id = self.0.id.fetch_add(1, Ordering::Relaxed);
        {
            let mut pending = self.0.pending.lock().unwrap();
            pending.insert(id, rh);
        }
        if let Err(e)
            = self.send(&json!({ "id" : id, "method" : method, "params" : params, }))
        {
            let mut pending = self.0.pending.lock().unwrap();
            if let Some(rh) = pending.remove(&id) {
                rh.invoke(Err(Error::Io(e)));
            }
        }
    }
    fn handle_response(&self, id: u64, resp: Result<Value, Error>) {
        let id = id as usize;
        let handler = {
            let mut pending = self.0.pending.lock().unwrap();
            pending.remove(&id)
        };
        match handler {
            Some(responsehandler) => responsehandler.invoke(resp),
            None => warn!("id {} not found in pending", id),
        }
    }
    /// Get a message from the receive queue if available.
    fn try_get_rx(&self) -> Option<Result<RpcObject, ReadError>> {
        let mut queue = self.0.rx_queue.lock().unwrap();
        queue.pop_front()
    }
    /// Get a message from the receive queue, waiting for at most `Duration`
    /// and returning `None` if no message is available.
    fn get_rx_timeout(&self, dur: Duration) -> Option<Result<RpcObject, ReadError>> {
        let mut queue = self.0.rx_queue.lock().unwrap();
        let result = self.0.rx_cvar.wait_timeout(queue, dur).unwrap();
        queue = result.0;
        queue.pop_front()
    }
    /// Adds a message to the receive queue. The message should only
    /// be `None` if the read thread is exiting.
    fn put_rx(&self, json: Result<RpcObject, ReadError>) {
        let mut queue = self.0.rx_queue.lock().unwrap();
        queue.push_back(json);
        self.0.rx_cvar.notify_one();
    }
    fn try_get_idle(&self) -> Option<usize> {
        self.0.idle_queue.lock().unwrap().pop_front()
    }
    /// Checks status of the most imminent timer. If that timer has expired,
    /// returns `Some(Ok(_))`, with the corresponding token.
    /// If a timer exists but has not expired, returns `Some(Err(_))`,
    /// with the error value being the `Duration` until the timer is ready.
    /// Returns `None` if no timers are registered.
    fn check_timers(&self) -> Option<Result<usize, Duration>> {
        let mut timers = self.0.timers.lock().unwrap();
        match timers.peek() {
            None => return None,
            Some(t) => {
                let now = Instant::now();
                if t.fire_after > now {
                    return Some(Err(t.fire_after - now));
                }
            }
        }
        Some(Ok(timers.pop().unwrap().token))
    }
    /// send disconnect error to pending requests.
    fn disconnect(&self) {
        let mut pending = self.0.pending.lock().unwrap();
        let ids = pending.keys().cloned().collect::<Vec<_>>();
        for id in &ids {
            let callback = pending.remove(id).unwrap();
            callback.invoke(Err(Error::PeerDisconnect));
        }
        self.0.needs_exit.store(true, Ordering::Relaxed);
    }
    /// Returns `true` if an error has occured in the main thread.
    fn needs_exit(&self) -> bool {
        self.0.needs_exit.load(Ordering::Relaxed)
    }
    fn reset_needs_exit(&self) {
        self.0.needs_exit.store(false, Ordering::SeqCst);
    }
}
impl Clone for Box<dyn Peer> {
    fn clone(&self) -> Box<dyn Peer> {
        self.box_clone()
    }
}
impl<W: Write> Clone for RawPeer<W> {
    fn clone(&self) -> Self {
        RawPeer(self.0.clone())
    }
}
impl Ord for Timer {
    fn cmp(&self, other: &Timer) -> cmp::Ordering {
        other.fire_after.cmp(&self.fire_after)
    }
}
impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Timer) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_notif() {
        let reader = MessageReader::default();
        let json = reader
            .parse(r#"{"method": "hi", "params": {"words": "plz"}}"#)
            .unwrap();
        assert!(! json.is_response());
        let rpc = json.into_rpc::<Value, Value>().unwrap();
        match rpc {
            Call::Notification(_) => {}
            _ => panic!("parse failed"),
        }
    }
    #[test]
    fn test_parse_req() {
        let reader = MessageReader::default();
        let json = reader
            .parse(r#"{"id": 5, "method": "hi", "params": {"words": "plz"}}"#)
            .unwrap();
        assert!(! json.is_response());
        let rpc = json.into_rpc::<Value, Value>().unwrap();
        match rpc {
            Call::Request(..) => {}
            _ => panic!("parse failed"),
        }
    }
    #[test]
    fn test_parse_bad_json() {
        let reader = MessageReader::default();
        let json = reader
            .parse(r#"{"id": 5, "method": "hi", params: {"words": "plz"}}"#)
            .err()
            .unwrap();
        match json {
            ReadError::Json(..) => {}
            _ => panic!("parse failed"),
        }
        let json = reader.parse(r#"[5, "hi", {"arg": "val"}]"#).err().unwrap();
        match json {
            ReadError::NotObject => {}
            _ => panic!("parse failed"),
        }
    }
}
#[cfg(test)]
mod tests_llm_16_2_llm_16_1 {
    use super::*;
    use crate::*;
    use std::boxed::Box;
    #[derive(Clone)]
    struct MockCallback;
    trait Callback {
        fn call(self: Box<Self>, result: Result<Value, Error>);
        fn box_clone(&self) -> Box<dyn Callback>;
    }
    impl Callback for MockCallback {
        fn call(self: Box<Self>, result: Result<Value, Error>) {}
        fn box_clone(&self) -> Box<dyn Callback> {
            Box::new((*self).clone())
        }
    }
    #[test]
    fn test_call() {
        let _rug_st_tests_llm_16_2_llm_16_1_rrrruuuugggg_test_call = 0;
        let _rug_ed_tests_llm_16_2_llm_16_1_rrrruuuugggg_test_call = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_4_llm_16_3 {
    use super::*;
    use crate::*;
    use std::boxed::Box;
    use std::clone::Clone;
    use crate::IdleProc;
    use crate::Peer;
    use serde_json::Value;
    use crate::error::Error;
    use crate::Callback;
    use std::time::Instant;
    #[derive(Clone)]
    struct MockPeer;
    impl Peer for MockPeer {
        fn box_clone(&self) -> Box<dyn Peer> {
            Box::new(MockPeer)
        }
        fn send_rpc_notification(&self, method: &str, params: &Value) {
            unimplemented!()
        }
        fn send_rpc_request_async(
            &self,
            method: &str,
            params: &Value,
            f: Box<dyn Callback>,
        ) {
            unimplemented!()
        }
        fn send_rpc_request(
            &self,
            method: &str,
            params: &Value,
        ) -> Result<Value, Error> {
            unimplemented!()
        }
        fn request_is_pending(&self) -> bool {
            unimplemented!()
        }
        fn schedule_idle(&self, token: usize) {
            unimplemented!()
        }
        fn schedule_timer(&self, after: Instant, token: usize) {
            unimplemented!()
        }
    }
    impl IdleProc for MockPeer {
        fn call(self: Box<Self>, token: usize) {}
    }
    #[test]
    fn test_call() {
        let _rug_st_tests_llm_16_4_llm_16_3_rrrruuuugggg_test_call = 0;
        let rug_fuzz_0 = 0;
        let mock_peer = Box::new(MockPeer);
        mock_peer.call(rug_fuzz_0);
        let _rug_ed_tests_llm_16_4_llm_16_3_rrrruuuugggg_test_call = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_23 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_cmp() {
        let _rug_st_tests_llm_16_23_rrrruuuugggg_test_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let instant1 = Instant::now();
        let instant2 = instant1 + Duration::from_secs(rug_fuzz_0);
        let timer1 = Timer {
            fire_after: instant1,
            token: rug_fuzz_1,
        };
        let timer2 = Timer {
            fire_after: instant2,
            token: rug_fuzz_2,
        };
        debug_assert_eq!(timer1.cmp(& timer2), Ordering::Greater);
        debug_assert_eq!(timer2.cmp(& timer1), Ordering::Less);
        debug_assert_eq!(timer1.cmp(& timer1), Ordering::Equal);
        let _rug_ed_tests_llm_16_23_rrrruuuugggg_test_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_24 {
    use std::cmp;
    use std::time::Instant;
    use super::*;
    use crate::*;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let timer_1 = Timer {
            fire_after: Instant::now(),
            token: rug_fuzz_0,
        };
        let timer_2 = Timer {
            fire_after: Instant::now(),
            token: rug_fuzz_1,
        };
        let timer_3 = Timer {
            fire_after: Instant::now(),
            token: rug_fuzz_2,
        };
        debug_assert_eq!(timer_1.partial_cmp(& timer_1), Some(cmp::Ordering::Equal));
        debug_assert_eq!(timer_1.partial_cmp(& timer_2), Some(cmp::Ordering::Greater));
        debug_assert_eq!(timer_2.partial_cmp(& timer_1), Some(cmp::Ordering::Less));
        debug_assert_eq!(timer_2.partial_cmp(& timer_3), Some(cmp::Ordering::Equal));
        debug_assert_eq!(timer_3.partial_cmp(& timer_2), Some(cmp::Ordering::Equal));
        debug_assert_eq!(timer_1.partial_cmp(& timer_3), Some(cmp::Ordering::Greater));
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_partial_cmp = 0;
    }
}
