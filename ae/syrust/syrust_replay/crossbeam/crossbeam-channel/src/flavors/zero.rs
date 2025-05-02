//! Zero-capacity channel.
//!
//! This kind of channel is also known as *rendezvous* channel.
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use crossbeam_utils::Backoff;
use crate::context::Context;
use crate::err::{RecvTimeoutError, SendTimeoutError, TryRecvError, TrySendError};
use crate::select::{Operation, SelectHandle, Selected, Token};
use crate::utils::Spinlock;
use crate::waker::Waker;
/// A pointer to a packet.
pub type ZeroToken = usize;
/// A slot for passing one message from a sender to a receiver.
struct Packet<T> {
    /// Equals `true` if the packet is allocated on the stack.
    on_stack: bool,
    /// Equals `true` once the packet is ready for reading or writing.
    ready: AtomicBool,
    /// The message.
    msg: UnsafeCell<Option<T>>,
}
impl<T> Packet<T> {
    /// Creates an empty packet on the stack.
    fn empty_on_stack() -> Packet<T> {
        Packet {
            on_stack: true,
            ready: AtomicBool::new(false),
            msg: UnsafeCell::new(None),
        }
    }
    /// Creates an empty packet on the heap.
    fn empty_on_heap() -> Box<Packet<T>> {
        Box::new(Packet {
            on_stack: false,
            ready: AtomicBool::new(false),
            msg: UnsafeCell::new(None),
        })
    }
    /// Creates a packet on the stack, containing a message.
    fn message_on_stack(msg: T) -> Packet<T> {
        Packet {
            on_stack: true,
            ready: AtomicBool::new(false),
            msg: UnsafeCell::new(Some(msg)),
        }
    }
    /// Waits until the packet becomes ready for reading or writing.
    fn wait_ready(&self) {
        let backoff = Backoff::new();
        while !self.ready.load(Ordering::Acquire) {
            backoff.snooze();
        }
    }
}
/// Inner representation of a zero-capacity channel.
struct Inner {
    /// Senders waiting to pair up with a receive operation.
    senders: Waker,
    /// Receivers waiting to pair up with a send operation.
    receivers: Waker,
    /// Equals `true` when the channel is disconnected.
    is_disconnected: bool,
}
/// Zero-capacity channel.
pub struct Channel<T> {
    /// Inner representation of the channel.
    inner: Spinlock<Inner>,
    /// Indicates that dropping a `Channel<T>` may drop values of type `T`.
    _marker: PhantomData<T>,
}
impl<T> Channel<T> {
    /// Constructs a new zero-capacity channel.
    pub fn new() -> Self {
        Channel {
            inner: Spinlock::new(Inner {
                senders: Waker::new(),
                receivers: Waker::new(),
                is_disconnected: false,
            }),
            _marker: PhantomData,
        }
    }
    /// Returns a receiver handle to the channel.
    pub fn receiver(&self) -> Receiver<'_, T> {
        Receiver(self)
    }
    /// Returns a sender handle to the channel.
    pub fn sender(&self) -> Sender<'_, T> {
        Sender(self)
    }
    /// Attempts to reserve a slot for sending a message.
    fn start_send(&self, token: &mut Token) -> bool {
        let mut inner = self.inner.lock();
        if let Some(operation) = inner.receivers.try_select() {
            token.zero = operation.packet;
            true
        } else if inner.is_disconnected {
            token.zero = 0;
            true
        } else {
            false
        }
    }
    /// Writes a message into the packet.
    pub unsafe fn write(&self, token: &mut Token, msg: T) -> Result<(), T> {
        if token.zero == 0 {
            return Err(msg);
        }
        let packet = &*(token.zero as *const Packet<T>);
        packet.msg.get().write(Some(msg));
        packet.ready.store(true, Ordering::Release);
        Ok(())
    }
    /// Attempts to pair up with a sender.
    fn start_recv(&self, token: &mut Token) -> bool {
        let mut inner = self.inner.lock();
        if let Some(operation) = inner.senders.try_select() {
            token.zero = operation.packet;
            true
        } else if inner.is_disconnected {
            token.zero = 0;
            true
        } else {
            false
        }
    }
    /// Reads a message from the packet.
    pub unsafe fn read(&self, token: &mut Token) -> Result<T, ()> {
        if token.zero == 0 {
            return Err(());
        }
        let packet = &*(token.zero as *const Packet<T>);
        if packet.on_stack {
            let msg = packet.msg.get().replace(None).unwrap();
            packet.ready.store(true, Ordering::Release);
            Ok(msg)
        } else {
            packet.wait_ready();
            let msg = packet.msg.get().replace(None).unwrap();
            drop(Box::from_raw(packet as *const Packet<T> as *mut Packet<T>));
            Ok(msg)
        }
    }
    /// Attempts to send a message into the channel.
    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        let token = &mut Token::default();
        let mut inner = self.inner.lock();
        if let Some(operation) = inner.receivers.try_select() {
            token.zero = operation.packet;
            drop(inner);
            unsafe {
                self.write(token, msg).ok().unwrap();
            }
            Ok(())
        } else if inner.is_disconnected {
            Err(TrySendError::Disconnected(msg))
        } else {
            Err(TrySendError::Full(msg))
        }
    }
    /// Sends a message into the channel.
    pub fn send(
        &self,
        msg: T,
        deadline: Option<Instant>,
    ) -> Result<(), SendTimeoutError<T>> {
        let token = &mut Token::default();
        let mut inner = self.inner.lock();
        if let Some(operation) = inner.receivers.try_select() {
            token.zero = operation.packet;
            drop(inner);
            unsafe {
                self.write(token, msg).ok().unwrap();
            }
            return Ok(());
        }
        if inner.is_disconnected {
            return Err(SendTimeoutError::Disconnected(msg));
        }
        Context::with(|cx| {
            let oper = Operation::hook(token);
            let packet = Packet::<T>::message_on_stack(msg);
            inner
                .senders
                .register_with_packet(oper, &packet as *const Packet<T> as usize, cx);
            inner.receivers.notify();
            drop(inner);
            let sel = cx.wait_until(deadline);
            match sel {
                Selected::Waiting => unreachable!(),
                Selected::Aborted => {
                    self.inner.lock().senders.unregister(oper).unwrap();
                    let msg = unsafe { packet.msg.get().replace(None).unwrap() };
                    Err(SendTimeoutError::Timeout(msg))
                }
                Selected::Disconnected => {
                    self.inner.lock().senders.unregister(oper).unwrap();
                    let msg = unsafe { packet.msg.get().replace(None).unwrap() };
                    Err(SendTimeoutError::Disconnected(msg))
                }
                Selected::Operation(_) => {
                    packet.wait_ready();
                    Ok(())
                }
            }
        })
    }
    /// Attempts to receive a message without blocking.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let token = &mut Token::default();
        let mut inner = self.inner.lock();
        if let Some(operation) = inner.senders.try_select() {
            token.zero = operation.packet;
            drop(inner);
            unsafe { self.read(token).map_err(|_| TryRecvError::Disconnected) }
        } else if inner.is_disconnected {
            Err(TryRecvError::Disconnected)
        } else {
            Err(TryRecvError::Empty)
        }
    }
    /// Receives a message from the channel.
    pub fn recv(&self, deadline: Option<Instant>) -> Result<T, RecvTimeoutError> {
        let token = &mut Token::default();
        let mut inner = self.inner.lock();
        if let Some(operation) = inner.senders.try_select() {
            token.zero = operation.packet;
            drop(inner);
            unsafe {
                return self.read(token).map_err(|_| RecvTimeoutError::Disconnected);
            }
        }
        if inner.is_disconnected {
            return Err(RecvTimeoutError::Disconnected);
        }
        Context::with(|cx| {
            let oper = Operation::hook(token);
            let packet = Packet::<T>::empty_on_stack();
            inner
                .receivers
                .register_with_packet(oper, &packet as *const Packet<T> as usize, cx);
            inner.senders.notify();
            drop(inner);
            let sel = cx.wait_until(deadline);
            match sel {
                Selected::Waiting => unreachable!(),
                Selected::Aborted => {
                    self.inner.lock().receivers.unregister(oper).unwrap();
                    Err(RecvTimeoutError::Timeout)
                }
                Selected::Disconnected => {
                    self.inner.lock().receivers.unregister(oper).unwrap();
                    Err(RecvTimeoutError::Disconnected)
                }
                Selected::Operation(_) => {
                    packet.wait_ready();
                    unsafe { Ok(packet.msg.get().replace(None).unwrap()) }
                }
            }
        })
    }
    /// Disconnects the channel and wakes up all blocked senders and receivers.
    ///
    /// Returns `true` if this call disconnected the channel.
    pub fn disconnect(&self) -> bool {
        let mut inner = self.inner.lock();
        if !inner.is_disconnected {
            inner.is_disconnected = true;
            inner.senders.disconnect();
            inner.receivers.disconnect();
            true
        } else {
            false
        }
    }
    /// Returns the current number of messages inside the channel.
    pub fn len(&self) -> usize {
        0
    }
    /// Returns the capacity of the channel.
    pub fn capacity(&self) -> Option<usize> {
        Some(0)
    }
    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        true
    }
    /// Returns `true` if the channel is full.
    pub fn is_full(&self) -> bool {
        true
    }
}
/// Receiver handle to a channel.
pub struct Receiver<'a, T>(&'a Channel<T>);
/// Sender handle to a channel.
pub struct Sender<'a, T>(&'a Channel<T>);
impl<T> SelectHandle for Receiver<'_, T> {
    fn try_select(&self, token: &mut Token) -> bool {
        self.0.start_recv(token)
    }
    fn deadline(&self) -> Option<Instant> {
        None
    }
    fn register(&self, oper: Operation, cx: &Context) -> bool {
        let packet = Box::into_raw(Packet::<T>::empty_on_heap());
        let mut inner = self.0.inner.lock();
        inner.receivers.register_with_packet(oper, packet as usize, cx);
        inner.senders.notify();
        inner.senders.can_select() || inner.is_disconnected
    }
    fn unregister(&self, oper: Operation) {
        if let Some(operation) = self.0.inner.lock().receivers.unregister(oper) {
            unsafe {
                drop(Box::from_raw(operation.packet as *mut Packet<T>));
            }
        }
    }
    fn accept(&self, token: &mut Token, cx: &Context) -> bool {
        token.zero = cx.wait_packet();
        true
    }
    fn is_ready(&self) -> bool {
        let inner = self.0.inner.lock();
        inner.senders.can_select() || inner.is_disconnected
    }
    fn watch(&self, oper: Operation, cx: &Context) -> bool {
        let mut inner = self.0.inner.lock();
        inner.receivers.watch(oper, cx);
        inner.senders.can_select() || inner.is_disconnected
    }
    fn unwatch(&self, oper: Operation) {
        let mut inner = self.0.inner.lock();
        inner.receivers.unwatch(oper);
    }
}
impl<T> SelectHandle for Sender<'_, T> {
    fn try_select(&self, token: &mut Token) -> bool {
        self.0.start_send(token)
    }
    fn deadline(&self) -> Option<Instant> {
        None
    }
    fn register(&self, oper: Operation, cx: &Context) -> bool {
        let packet = Box::into_raw(Packet::<T>::empty_on_heap());
        let mut inner = self.0.inner.lock();
        inner.senders.register_with_packet(oper, packet as usize, cx);
        inner.receivers.notify();
        inner.receivers.can_select() || inner.is_disconnected
    }
    fn unregister(&self, oper: Operation) {
        if let Some(operation) = self.0.inner.lock().senders.unregister(oper) {
            unsafe {
                drop(Box::from_raw(operation.packet as *mut Packet<T>));
            }
        }
    }
    fn accept(&self, token: &mut Token, cx: &Context) -> bool {
        token.zero = cx.wait_packet();
        true
    }
    fn is_ready(&self) -> bool {
        let inner = self.0.inner.lock();
        inner.receivers.can_select() || inner.is_disconnected
    }
    fn watch(&self, oper: Operation, cx: &Context) -> bool {
        let mut inner = self.0.inner.lock();
        inner.senders.watch(oper, cx);
        inner.receivers.can_select() || inner.is_disconnected
    }
    fn unwatch(&self, oper: Operation) {
        let mut inner = self.0.inner.lock();
        inner.senders.unwatch(oper);
    }
}
#[cfg(test)]
mod tests_rug_268 {
    use super::*;
    use crate::flavors::zero::Packet;
    #[test]
    fn test_empty_on_stack() {
        let _rug_st_tests_rug_268_rrrruuuugggg_test_empty_on_stack = 0;
        let packet: Packet<i32> = Packet::<i32>::empty_on_stack();
        let _rug_ed_tests_rug_268_rrrruuuugggg_test_empty_on_stack = 0;
    }
}
#[cfg(test)]
mod tests_rug_270 {
    use super::*;
    use crate::flavors::zero::Packet;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let msg = rug_fuzz_0;
        let p0: i32 = msg;
        Packet::message_on_stack(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_273 {
    use super::*;
    use crate::flavors::zero::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_273_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        p0.receiver();
        let _rug_ed_tests_rug_273_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_274 {
    use super::*;
    use crate::flavors::zero::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_274_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        p0.sender();
        let _rug_ed_tests_rug_274_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_280 {
    use super::*;
    use crate::{flavors::zero::Channel, SendTimeoutError};
    use std::{sync::Arc, time::{Duration, Instant}};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let msg = rug_fuzz_0;
        let channel = Channel::<&str>::new();
        let deadline = Some(Instant::now() + Duration::from_secs(rug_fuzz_1));
        debug_assert_eq!(channel.send(msg, deadline), Ok(()));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_281 {
    use super::*;
    use crate::flavors::zero::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_281_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        p0.try_recv().ok();
        let _rug_ed_tests_rug_281_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_283 {
    use super::*;
    use crate::flavors::zero::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_283_rrrruuuugggg_test_rug = 0;
        let p0: Channel<i32> = Channel::<i32>::new();
        p0.disconnect();
        let _rug_ed_tests_rug_283_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_284 {
    use super::*;
    use crate::flavors::zero::Channel;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_284_rrrruuuugggg_test_len = 0;
        let p0 = Channel::<i32>::new();
        debug_assert_eq!(p0.len(), 0);
        let _rug_ed_tests_rug_284_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_285 {
    use super::*;
    use crate::flavors::zero::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_285_rrrruuuugggg_test_rug = 0;
        let p0: Channel<i32> = Channel::new();
        debug_assert_eq!(p0.capacity(), Some(0));
        let _rug_ed_tests_rug_285_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_286 {
    use super::*;
    use crate::flavors::zero::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_286_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        debug_assert_eq!(p0.is_empty(), true);
        let _rug_ed_tests_rug_286_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_287 {
    use super::*;
    use crate::flavors::zero::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_287_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::<i32>::new();
        p0.is_full();
        let _rug_ed_tests_rug_287_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_289 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::zero::Receiver;
    use std::time::Instant;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_289_rrrruuuugggg_test_rug = 0;
        let receiver: Receiver<'_, i32> = unimplemented!();
        receiver.deadline();
        let _rug_ed_tests_rug_289_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_297 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::zero::Sender;
    use std::time::Instant;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_297_rrrruuuugggg_test_rug = 0;
        let mut p0: Sender<'_, i32> = unimplemented!();
        p0.deadline();
        let _rug_ed_tests_rug_297_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_301 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::zero::Sender;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_301_rrrruuuugggg_test_rug = 0;
        let mut p0: Sender<'_, i32> = unimplemented!();
        p0.is_ready();
        let _rug_ed_tests_rug_301_rrrruuuugggg_test_rug = 0;
    }
}
