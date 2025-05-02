//! Types and helpers used for testing.
use std::io::{self, Cursor, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use serde_json::{self, Value};
use super::{Callback, Error, MessageReader, Peer, ReadError, Response, RpcObject};
/// Wraps an instance of `mpsc::Sender`, implementing `Write`.
///
/// This lets the tx side of an mpsc::channel serve as the destination
/// stream for an RPC loop.
pub struct DummyWriter(Sender<String>);
/// Wraps an instance of `mpsc::Receiver`, providing convenience methods
/// for parsing received messages.
pub struct DummyReader(MessageReader, Receiver<String>);
/// An Peer that doesn't do anything.
#[derive(Debug, Clone)]
pub struct DummyPeer;
/// Returns a `(DummyWriter, DummyReader)` pair.
pub fn test_channel() -> (DummyWriter, DummyReader) {
    let (tx, rx) = channel();
    (DummyWriter(tx), DummyReader(MessageReader::default(), rx))
}
/// Given a string type, returns a `Cursor<Vec<u8>>`, which implements
/// `BufRead`.
pub fn make_reader<S: AsRef<str>>(s: S) -> Cursor<Vec<u8>> {
    Cursor::new(s.as_ref().as_bytes().to_vec())
}
impl DummyReader {
    /// Attempts to read a message, returning `None` if the wait exceeds
    /// `timeout`.
    ///
    /// This method makes no assumptions about the contents of the
    /// message, and does no error handling.
    pub fn next_timeout(
        &mut self,
        timeout: Duration,
    ) -> Option<Result<RpcObject, ReadError>> {
        self.1.recv_timeout(timeout).ok().map(|s| self.0.parse(&s))
    }
    /// Reads and parses a response object.
    ///
    /// # Panics
    ///
    /// Panics if a non-response message is received, or if no message
    /// is received after a reasonable time.
    pub fn expect_response(&mut self) -> Response {
        let raw = self
            .next_timeout(Duration::from_secs(1))
            .expect("response should be received");
        let val = raw.as_ref().ok().map(|v| serde_json::to_string(&v.0));
        let resp = raw.map_err(|e| e.to_string()).and_then(|r| r.into_response());
        match resp {
            Err(msg) => panic!("Bad response: {:?}. {}", val, msg),
            Ok(resp) => resp,
        }
    }
    pub fn expect_object(&mut self) -> RpcObject {
        self.next_timeout(Duration::from_secs(1)).expect("expected object").unwrap()
    }
    pub fn expect_rpc(&mut self, method: &str) -> RpcObject {
        let obj = self
            .next_timeout(Duration::from_secs(1))
            .unwrap_or_else(|| panic!("expected rpc \"{}\"", method))
            .unwrap();
        assert_eq!(obj.get_method(), Some(method));
        obj
    }
    pub fn expect_nothing(&mut self) {
        if let Some(thing) = self.next_timeout(Duration::from_millis(500)) {
            panic!("unexpected something {:?}", thing);
        }
    }
}
impl Write for DummyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8(buf.to_vec()).unwrap();
        self.0
            .send(s)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))
            .map(|_| buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Peer for DummyPeer {
    fn box_clone(&self) -> Box<dyn Peer> {
        Box::new(self.clone())
    }
    fn send_rpc_notification(&self, _method: &str, _params: &Value) {}
    fn send_rpc_request_async(
        &self,
        _method: &str,
        _params: &Value,
        f: Box<dyn Callback>,
    ) {
        f.call(Ok("dummy peer".into()))
    }
    fn send_rpc_request(&self, _method: &str, _params: &Value) -> Result<Value, Error> {
        Ok("dummy peer".into())
    }
    fn request_is_pending(&self) -> bool {
        false
    }
    fn schedule_idle(&self, _token: usize) {}
    fn schedule_timer(&self, _time: Instant, _token: usize) {}
}
#[cfg(test)]
mod tests_llm_16_42 {
    use super::*;
    use crate::*;
    use serde_json::Value;
    #[test]
    fn test_request_is_pending() {
        let _rug_st_tests_llm_16_42_rrrruuuugggg_test_request_is_pending = 0;
        let peer = test_utils::DummyPeer;
        debug_assert_eq!(peer.request_is_pending(), false);
        let _rug_ed_tests_llm_16_42_rrrruuuugggg_test_request_is_pending = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_43 {
    use super::*;
    use crate::*;
    use crate::test_utils::DummyPeer;
    use serde_json::Value;
    use std::time::Instant;
    #[test]
    fn test_schedule_idle() {
        let _rug_st_tests_llm_16_43_rrrruuuugggg_test_schedule_idle = 0;
        let rug_fuzz_0 = 123;
        let dummy_peer = DummyPeer;
        let token = rug_fuzz_0;
        dummy_peer.schedule_idle(token);
        let _rug_ed_tests_llm_16_43_rrrruuuugggg_test_schedule_idle = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_44 {
    use super::*;
    use crate::*;
    use std::time::Instant;
    #[derive(Clone, Debug)]
    struct MockPeer;
    impl Peer for MockPeer {
        fn box_clone(&self) -> Box<dyn Peer> {
            Box::new(self.clone())
        }
        fn send_rpc_notification(&self, _method: &str, _params: &Value) {}
        fn send_rpc_request_async(
            &self,
            _method: &str,
            _params: &Value,
            f: Box<dyn Callback>,
        ) {
            f.call(Ok("dummy peer".into()))
        }
        fn send_rpc_request(
            &self,
            _method: &str,
            _params: &Value,
        ) -> Result<Value, Error> {
            Ok("dummy peer".into())
        }
        fn request_is_pending(&self) -> bool {
            false
        }
        fn schedule_idle(&self, _token: usize) {}
        fn schedule_timer(&self, _time: Instant, _token: usize) {}
    }
    #[test]
    fn test_schedule_timer() {
        let _rug_st_tests_llm_16_44_rrrruuuugggg_test_schedule_timer = 0;
        let rug_fuzz_0 = 123;
        let peer: Box<dyn Peer> = Box::new(MockPeer);
        let time = Instant::now();
        let token = rug_fuzz_0;
        peer.schedule_timer(time, token);
        let _rug_ed_tests_llm_16_44_rrrruuuugggg_test_schedule_timer = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_52_llm_16_51 {
    use std::io::{self, Write};
    use crate::test_utils::DummyWriter;
    use std::sync::mpsc::{self, Sender};
    #[test]
    fn test_flush() {
        let _rug_st_tests_llm_16_52_llm_16_51_rrrruuuugggg_test_flush = 0;
        let (tx, _) = mpsc::channel();
        let mut writer = DummyWriter(tx);
        let result = <crate::test_utils::DummyWriter as Write>::flush(&mut writer);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_52_llm_16_51_rrrruuuugggg_test_flush = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_53 {
    use std::io::{self, Write};
    use std::sync::mpsc::Sender;
    use crate::test_utils::DummyWriter;
    #[test]
    fn test_write() {
        let _rug_st_tests_llm_16_53_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = b"test";
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut writer = DummyWriter(tx);
        let buf: &[u8] = rug_fuzz_0;
        let result = writer.write(buf);
        debug_assert!(result.is_ok());
        debug_assert_eq!(result.unwrap(), buf.len());
        let _rug_ed_tests_llm_16_53_rrrruuuugggg_test_write = 0;
    }
}
