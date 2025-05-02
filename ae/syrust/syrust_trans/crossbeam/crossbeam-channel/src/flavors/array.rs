//! Bounded channel based on a preallocated array.
//!
//! This flavor has a fixed, positive capacity.
//!
//! The implementation is based on Dmitry Vyukov's bounded MPMC queue.
//!
//! Source:
//!   - http://www.1024cores.net/home/lock-free-algorithms/queues/bounded-mpmc-queue
//!   - https://docs.google.com/document/d/1yIAYmbvL3JxOKOjuCyon7JhW4cSv1wy5hC0ApeGMV9s/pub
//!
//! Copyright & License:
//!   - Copyright (c) 2010-2011 Dmitry Vyukov
//!   - Simplified BSD License and Apache License, Version 2.0
//!   - http://www.1024cores.net/home/code-license
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::ptr;
use std::sync::atomic::{self, AtomicUsize, Ordering};
use std::time::Instant;
use crossbeam_utils::{Backoff, CachePadded};
use crate::context::Context;
use crate::err::{RecvTimeoutError, SendTimeoutError, TryRecvError, TrySendError};
use crate::select::{Operation, SelectHandle, Selected, Token};
use crate::waker::SyncWaker;
/// A slot in a channel.
struct Slot<T> {
    /// The current stamp.
    stamp: AtomicUsize,
    /// The message in this slot.
    msg: UnsafeCell<MaybeUninit<T>>,
}
/// The token type for the array flavor.
#[derive(Debug)]
pub struct ArrayToken {
    /// Slot to read from or write to.
    slot: *const u8,
    /// Stamp to store into the slot after reading or writing.
    stamp: usize,
}
impl Default for ArrayToken {
    #[inline]
    fn default() -> Self {
        ArrayToken {
            slot: ptr::null(),
            stamp: 0,
        }
    }
}
/// Bounded channel based on a preallocated array.
pub struct Channel<T> {
    /// The head of the channel.
    ///
    /// This value is a "stamp" consisting of an index into the buffer, a mark bit, and a lap, but
    /// packed into a single `usize`. The lower bits represent the index, while the upper bits
    /// represent the lap. The mark bit in the head is always zero.
    ///
    /// Messages are popped from the head of the channel.
    head: CachePadded<AtomicUsize>,
    /// The tail of the channel.
    ///
    /// This value is a "stamp" consisting of an index into the buffer, a mark bit, and a lap, but
    /// packed into a single `usize`. The lower bits represent the index, while the upper bits
    /// represent the lap. The mark bit indicates that the channel is disconnected.
    ///
    /// Messages are pushed into the tail of the channel.
    tail: CachePadded<AtomicUsize>,
    /// The buffer holding slots.
    buffer: *mut Slot<T>,
    /// The channel capacity.
    cap: usize,
    /// A stamp with the value of `{ lap: 1, mark: 0, index: 0 }`.
    one_lap: usize,
    /// If this bit is set in the tail, that means the channel is disconnected.
    mark_bit: usize,
    /// Senders waiting while the channel is full.
    senders: SyncWaker,
    /// Receivers waiting while the channel is empty and not disconnected.
    receivers: SyncWaker,
    /// Indicates that dropping a `Channel<T>` may drop values of type `T`.
    _marker: PhantomData<T>,
}
impl<T> Channel<T> {
    /// Creates a bounded channel of capacity `cap`.
    pub fn with_capacity(cap: usize) -> Self {
        assert!(cap > 0, "capacity must be positive");
        let mark_bit = (cap + 1).next_power_of_two();
        let one_lap = mark_bit * 2;
        let head = 0;
        let tail = 0;
        let buffer = {
            let mut v: Vec<Slot<T>> = (0..cap)
                .map(|i| {
                    Slot {
                        stamp: AtomicUsize::new(i),
                        msg: UnsafeCell::new(MaybeUninit::uninit()),
                    }
                })
                .collect();
            let ptr = v.as_mut_ptr();
            mem::forget(v);
            ptr
        };
        Channel {
            buffer,
            cap,
            one_lap,
            mark_bit,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
            senders: SyncWaker::new(),
            receivers: SyncWaker::new(),
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
        let backoff = Backoff::new();
        let mut tail = self.tail.load(Ordering::Relaxed);
        loop {
            if tail & self.mark_bit != 0 {
                token.array.slot = ptr::null();
                token.array.stamp = 0;
                return true;
            }
            let index = tail & (self.mark_bit - 1);
            let lap = tail & !(self.one_lap - 1);
            let slot = unsafe { &*self.buffer.add(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);
            if tail == stamp {
                let new_tail = if index + 1 < self.cap {
                    tail + 1
                } else {
                    lap.wrapping_add(self.one_lap)
                };
                match self
                    .tail
                    .compare_exchange_weak(
                        tail,
                        new_tail,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                {
                    Ok(_) => {
                        token.array.slot = slot as *const Slot<T> as *const u8;
                        token.array.stamp = tail + 1;
                        return true;
                    }
                    Err(t) => {
                        tail = t;
                        backoff.spin();
                    }
                }
            } else if stamp.wrapping_add(self.one_lap) == tail + 1 {
                atomic::fence(Ordering::SeqCst);
                let head = self.head.load(Ordering::Relaxed);
                if head.wrapping_add(self.one_lap) == tail {
                    return false;
                }
                backoff.spin();
                tail = self.tail.load(Ordering::Relaxed);
            } else {
                backoff.snooze();
                tail = self.tail.load(Ordering::Relaxed);
            }
        }
    }
    /// Writes a message into the channel.
    pub unsafe fn write(&self, token: &mut Token, msg: T) -> Result<(), T> {
        if token.array.slot.is_null() {
            return Err(msg);
        }
        let slot: &Slot<T> = &*(token.array.slot as *const Slot<T>);
        slot.msg.get().write(MaybeUninit::new(msg));
        slot.stamp.store(token.array.stamp, Ordering::Release);
        self.receivers.notify();
        Ok(())
    }
    /// Attempts to reserve a slot for receiving a message.
    fn start_recv(&self, token: &mut Token) -> bool {
        let backoff = Backoff::new();
        let mut head = self.head.load(Ordering::Relaxed);
        loop {
            let index = head & (self.mark_bit - 1);
            let lap = head & !(self.one_lap - 1);
            let slot = unsafe { &*self.buffer.add(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);
            if head + 1 == stamp {
                let new = if index + 1 < self.cap {
                    head + 1
                } else {
                    lap.wrapping_add(self.one_lap)
                };
                match self
                    .head
                    .compare_exchange_weak(
                        head,
                        new,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                {
                    Ok(_) => {
                        token.array.slot = slot as *const Slot<T> as *const u8;
                        token.array.stamp = head.wrapping_add(self.one_lap);
                        return true;
                    }
                    Err(h) => {
                        head = h;
                        backoff.spin();
                    }
                }
            } else if stamp == head {
                atomic::fence(Ordering::SeqCst);
                let tail = self.tail.load(Ordering::Relaxed);
                if (tail & !self.mark_bit) == head {
                    if tail & self.mark_bit != 0 {
                        token.array.slot = ptr::null();
                        token.array.stamp = 0;
                        return true;
                    } else {
                        return false;
                    }
                }
                backoff.spin();
                head = self.head.load(Ordering::Relaxed);
            } else {
                backoff.snooze();
                head = self.head.load(Ordering::Relaxed);
            }
        }
    }
    /// Reads a message from the channel.
    pub unsafe fn read(&self, token: &mut Token) -> Result<T, ()> {
        if token.array.slot.is_null() {
            return Err(());
        }
        let slot: &Slot<T> = &*(token.array.slot as *const Slot<T>);
        let msg = slot.msg.get().read().assume_init();
        slot.stamp.store(token.array.stamp, Ordering::Release);
        self.senders.notify();
        Ok(msg)
    }
    /// Attempts to send a message into the channel.
    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        let token = &mut Token::default();
        if self.start_send(token) {
            unsafe { self.write(token, msg).map_err(TrySendError::Disconnected) }
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
        loop {
            let backoff = Backoff::new();
            loop {
                if self.start_send(token) {
                    let res = unsafe { self.write(token, msg) };
                    return res.map_err(SendTimeoutError::Disconnected);
                }
                if backoff.is_completed() {
                    break;
                } else {
                    backoff.snooze();
                }
            }
            if let Some(d) = deadline {
                if Instant::now() >= d {
                    return Err(SendTimeoutError::Timeout(msg));
                }
            }
            Context::with(|cx| {
                let oper = Operation::hook(token);
                self.senders.register(oper, cx);
                if !self.is_full() || self.is_disconnected() {
                    let _ = cx.try_select(Selected::Aborted);
                }
                let sel = cx.wait_until(deadline);
                match sel {
                    Selected::Waiting => unreachable!(),
                    Selected::Aborted | Selected::Disconnected => {
                        self.senders.unregister(oper).unwrap();
                    }
                    Selected::Operation(_) => {}
                }
            });
        }
    }
    /// Attempts to receive a message without blocking.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let token = &mut Token::default();
        if self.start_recv(token) {
            unsafe { self.read(token).map_err(|_| TryRecvError::Disconnected) }
        } else {
            Err(TryRecvError::Empty)
        }
    }
    /// Receives a message from the channel.
    pub fn recv(&self, deadline: Option<Instant>) -> Result<T, RecvTimeoutError> {
        let token = &mut Token::default();
        loop {
            let backoff = Backoff::new();
            loop {
                if self.start_recv(token) {
                    let res = unsafe { self.read(token) };
                    return res.map_err(|_| RecvTimeoutError::Disconnected);
                }
                if backoff.is_completed() {
                    break;
                } else {
                    backoff.snooze();
                }
            }
            if let Some(d) = deadline {
                if Instant::now() >= d {
                    return Err(RecvTimeoutError::Timeout);
                }
            }
            Context::with(|cx| {
                let oper = Operation::hook(token);
                self.receivers.register(oper, cx);
                if !self.is_empty() || self.is_disconnected() {
                    let _ = cx.try_select(Selected::Aborted);
                }
                let sel = cx.wait_until(deadline);
                match sel {
                    Selected::Waiting => unreachable!(),
                    Selected::Aborted | Selected::Disconnected => {
                        self.receivers.unregister(oper).unwrap();
                    }
                    Selected::Operation(_) => {}
                }
            });
        }
    }
    /// Returns the current number of messages inside the channel.
    pub fn len(&self) -> usize {
        loop {
            let tail = self.tail.load(Ordering::SeqCst);
            let head = self.head.load(Ordering::SeqCst);
            if self.tail.load(Ordering::SeqCst) == tail {
                let hix = head & (self.mark_bit - 1);
                let tix = tail & (self.mark_bit - 1);
                return if hix < tix {
                    tix - hix
                } else if hix > tix {
                    self.cap - hix + tix
                } else if (tail & !self.mark_bit) == head {
                    0
                } else {
                    self.cap
                };
            }
        }
    }
    /// Returns the capacity of the channel.
    pub fn capacity(&self) -> Option<usize> {
        Some(self.cap)
    }
    /// Disconnects the channel and wakes up all blocked senders and receivers.
    ///
    /// Returns `true` if this call disconnected the channel.
    pub fn disconnect(&self) -> bool {
        let tail = self.tail.fetch_or(self.mark_bit, Ordering::SeqCst);
        if tail & self.mark_bit == 0 {
            self.senders.disconnect();
            self.receivers.disconnect();
            true
        } else {
            false
        }
    }
    /// Returns `true` if the channel is disconnected.
    pub fn is_disconnected(&self) -> bool {
        self.tail.load(Ordering::SeqCst) & self.mark_bit != 0
    }
    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        (tail & !self.mark_bit) == head
    }
    /// Returns `true` if the channel is full.
    pub fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);
        head.wrapping_add(self.one_lap) == tail & !self.mark_bit
    }
}
impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        let hix = self.head.load(Ordering::Relaxed) & (self.mark_bit - 1);
        for i in 0..self.len() {
            let index = if hix + i < self.cap { hix + i } else { hix + i - self.cap };
            unsafe {
                let p = {
                    let slot = &mut *self.buffer.add(index);
                    let msg = &mut *slot.msg.get();
                    msg.as_mut_ptr()
                };
                p.drop_in_place();
            }
        }
        unsafe {
            Vec::from_raw_parts(self.buffer, 0, self.cap);
        }
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
        self.0.receivers.register(oper, cx);
        self.is_ready()
    }
    fn unregister(&self, oper: Operation) {
        self.0.receivers.unregister(oper);
    }
    fn accept(&self, token: &mut Token, _cx: &Context) -> bool {
        self.try_select(token)
    }
    fn is_ready(&self) -> bool {
        !self.0.is_empty() || self.0.is_disconnected()
    }
    fn watch(&self, oper: Operation, cx: &Context) -> bool {
        self.0.receivers.watch(oper, cx);
        self.is_ready()
    }
    fn unwatch(&self, oper: Operation) {
        self.0.receivers.unwatch(oper);
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
        self.0.senders.register(oper, cx);
        self.is_ready()
    }
    fn unregister(&self, oper: Operation) {
        self.0.senders.unregister(oper);
    }
    fn accept(&self, token: &mut Token, _cx: &Context) -> bool {
        self.try_select(token)
    }
    fn is_ready(&self) -> bool {
        !self.0.is_full() || self.0.is_disconnected()
    }
    fn watch(&self, oper: Operation, cx: &Context) -> bool {
        self.0.senders.watch(oper, cx);
        self.is_ready()
    }
    fn unwatch(&self, oper: Operation) {
        self.0.senders.unwatch(oper);
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::flavors::array::{Channel, Slot};
    use std::cell::UnsafeCell;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::marker::PhantomData;
    use std::mem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_163_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let cap: usize = rug_fuzz_0;
        let p0: usize = cap;
        let _channel: Channel<i32> = Channel::<i32>::with_capacity(p0);
        let _rug_ed_tests_rug_163_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::flavors::array::Channel;
    use crate::select::Token;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_166_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<usize> = unimplemented!();
        let mut p1: Token = Token::default();
        p0.start_send(&mut p1);
        let _rug_ed_tests_rug_166_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::flavors::array::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_172_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Channel<i32> = Channel::with_capacity(rug_fuzz_0);
        p0.try_recv();
        let _rug_ed_tests_rug_172_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::array::Sender;
    use std::time::{Instant, Duration};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_190_rrrruuuugggg_test_rug = 0;
        let mut p0: Sender<'_, i32> = unimplemented!();
        p0.deadline();
        let _rug_ed_tests_rug_190_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::array::Sender;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_194_rrrruuuugggg_test_rug = 0;
        let mut p0: Sender<'_, i32> = unimplemented!();
        p0.is_ready();
        let _rug_ed_tests_rug_194_rrrruuuugggg_test_rug = 0;
    }
}
