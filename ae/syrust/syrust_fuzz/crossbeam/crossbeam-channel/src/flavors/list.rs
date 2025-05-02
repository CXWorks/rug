//! Unbounded channel implemented as a linked list.
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{self, AtomicPtr, AtomicUsize, Ordering};
use std::time::Instant;
use crossbeam_utils::{Backoff, CachePadded};
use crate::context::Context;
use crate::err::{RecvTimeoutError, SendTimeoutError, TryRecvError, TrySendError};
use crate::select::{Operation, SelectHandle, Selected, Token};
use crate::waker::SyncWaker;
const WRITE: usize = 1;
const READ: usize = 2;
const DESTROY: usize = 4;
const LAP: usize = 32;
const BLOCK_CAP: usize = LAP - 1;
const SHIFT: usize = 1;
const MARK_BIT: usize = 1;
/// A slot in a block.
struct Slot<T> {
    /// The message.
    msg: UnsafeCell<MaybeUninit<T>>,
    /// The state of the slot.
    state: AtomicUsize,
}
impl<T> Slot<T> {
    /// Waits until a message is written into the slot.
    fn wait_write(&self) {
        let backoff = Backoff::new();
        while self.state.load(Ordering::Acquire) & WRITE == 0 {
            backoff.snooze();
        }
    }
}
/// A block in a linked list.
///
/// Each block in the list can hold up to `BLOCK_CAP` messages.
struct Block<T> {
    /// The next block in the linked list.
    next: AtomicPtr<Block<T>>,
    /// Slots for messages.
    slots: [Slot<T>; BLOCK_CAP],
}
impl<T> Block<T> {
    /// Creates an empty block.
    fn new() -> Block<T> {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
    /// Waits until the next pointer is set.
    fn wait_next(&self) -> *mut Block<T> {
        let backoff = Backoff::new();
        loop {
            let next = self.next.load(Ordering::Acquire);
            if !next.is_null() {
                return next;
            }
            backoff.snooze();
        }
    }
    /// Sets the `DESTROY` bit in slots starting from `start` and destroys the block.
    unsafe fn destroy(this: *mut Block<T>, start: usize) {
        for i in start..BLOCK_CAP - 1 {
            let slot = (*this).slots.get_unchecked(i);
            if slot.state.load(Ordering::Acquire) & READ == 0
                && slot.state.fetch_or(DESTROY, Ordering::AcqRel) & READ == 0
            {
                return;
            }
        }
        drop(Box::from_raw(this));
    }
}
/// A position in a channel.
#[derive(Debug)]
struct Position<T> {
    /// The index in the channel.
    index: AtomicUsize,
    /// The block in the linked list.
    block: AtomicPtr<Block<T>>,
}
/// The token type for the list flavor.
#[derive(Debug)]
pub struct ListToken {
    /// The block of slots.
    block: *const u8,
    /// The offset into the block.
    offset: usize,
}
impl Default for ListToken {
    #[inline]
    fn default() -> Self {
        ListToken {
            block: ptr::null(),
            offset: 0,
        }
    }
}
/// Unbounded channel implemented as a linked list.
///
/// Each message sent into the channel is assigned a sequence number, i.e. an index. Indices are
/// represented as numbers of type `usize` and wrap on overflow.
///
/// Consecutive messages are grouped into blocks in order to put less pressure on the allocator and
/// improve cache efficiency.
pub struct Channel<T> {
    /// The head of the channel.
    head: CachePadded<Position<T>>,
    /// The tail of the channel.
    tail: CachePadded<Position<T>>,
    /// Receivers waiting while the channel is empty and not disconnected.
    receivers: SyncWaker,
    /// Indicates that dropping a `Channel<T>` may drop messages of type `T`.
    _marker: PhantomData<T>,
}
impl<T> Channel<T> {
    /// Creates a new unbounded channel.
    pub fn new() -> Self {
        Channel {
            head: CachePadded::new(Position {
                block: AtomicPtr::new(ptr::null_mut()),
                index: AtomicUsize::new(0),
            }),
            tail: CachePadded::new(Position {
                block: AtomicPtr::new(ptr::null_mut()),
                index: AtomicUsize::new(0),
            }),
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
        let mut tail = self.tail.index.load(Ordering::Acquire);
        let mut block = self.tail.block.load(Ordering::Acquire);
        let mut next_block = None;
        loop {
            if tail & MARK_BIT != 0 {
                token.list.block = ptr::null();
                return true;
            }
            let offset = (tail >> SHIFT) % LAP;
            if offset == BLOCK_CAP {
                backoff.snooze();
                tail = self.tail.index.load(Ordering::Acquire);
                block = self.tail.block.load(Ordering::Acquire);
                continue;
            }
            if offset + 1 == BLOCK_CAP && next_block.is_none() {
                next_block = Some(Box::new(Block::<T>::new()));
            }
            if block.is_null() {
                let new = Box::into_raw(Box::new(Block::<T>::new()));
                if self.tail.block.compare_and_swap(block, new, Ordering::Release)
                    == block
                {
                    self.head.block.store(new, Ordering::Release);
                    block = new;
                } else {
                    next_block = unsafe { Some(Box::from_raw(new)) };
                    tail = self.tail.index.load(Ordering::Acquire);
                    block = self.tail.block.load(Ordering::Acquire);
                    continue;
                }
            }
            let new_tail = tail + (1 << SHIFT);
            match self
                .tail
                .index
                .compare_exchange_weak(
                    tail,
                    new_tail,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                )
            {
                Ok(_) => {
                    unsafe {
                        if offset + 1 == BLOCK_CAP {
                            let next_block = Box::into_raw(next_block.unwrap());
                            self.tail.block.store(next_block, Ordering::Release);
                            self.tail.index.fetch_add(1 << SHIFT, Ordering::Release);
                            (*block).next.store(next_block, Ordering::Release);
                        }
                        token.list.block = block as *const u8;
                        token.list.offset = offset;
                        return true;
                    }
                }
                Err(t) => {
                    tail = t;
                    block = self.tail.block.load(Ordering::Acquire);
                    backoff.spin();
                }
            }
        }
    }
    /// Writes a message into the channel.
    pub unsafe fn write(&self, token: &mut Token, msg: T) -> Result<(), T> {
        if token.list.block.is_null() {
            return Err(msg);
        }
        let block = token.list.block as *mut Block<T>;
        let offset = token.list.offset;
        let slot = (*block).slots.get_unchecked(offset);
        slot.msg.get().write(MaybeUninit::new(msg));
        slot.state.fetch_or(WRITE, Ordering::Release);
        self.receivers.notify();
        Ok(())
    }
    /// Attempts to reserve a slot for receiving a message.
    fn start_recv(&self, token: &mut Token) -> bool {
        let backoff = Backoff::new();
        let mut head = self.head.index.load(Ordering::Acquire);
        let mut block = self.head.block.load(Ordering::Acquire);
        loop {
            let offset = (head >> SHIFT) % LAP;
            if offset == BLOCK_CAP {
                backoff.snooze();
                head = self.head.index.load(Ordering::Acquire);
                block = self.head.block.load(Ordering::Acquire);
                continue;
            }
            let mut new_head = head + (1 << SHIFT);
            if new_head & MARK_BIT == 0 {
                atomic::fence(Ordering::SeqCst);
                let tail = self.tail.index.load(Ordering::Relaxed);
                if head >> SHIFT == tail >> SHIFT {
                    if tail & MARK_BIT != 0 {
                        token.list.block = ptr::null();
                        return true;
                    } else {
                        return false;
                    }
                }
                if (head >> SHIFT) / LAP != (tail >> SHIFT) / LAP {
                    new_head |= MARK_BIT;
                }
            }
            if block.is_null() {
                backoff.snooze();
                head = self.head.index.load(Ordering::Acquire);
                block = self.head.block.load(Ordering::Acquire);
                continue;
            }
            match self
                .head
                .index
                .compare_exchange_weak(
                    head,
                    new_head,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                )
            {
                Ok(_) => {
                    unsafe {
                        if offset + 1 == BLOCK_CAP {
                            let next = (*block).wait_next();
                            let mut next_index = (new_head & !MARK_BIT)
                                .wrapping_add(1 << SHIFT);
                            if !(*next).next.load(Ordering::Relaxed).is_null() {
                                next_index |= MARK_BIT;
                            }
                            self.head.block.store(next, Ordering::Release);
                            self.head.index.store(next_index, Ordering::Release);
                        }
                        token.list.block = block as *const u8;
                        token.list.offset = offset;
                        return true;
                    }
                }
                Err(h) => {
                    head = h;
                    block = self.head.block.load(Ordering::Acquire);
                    backoff.spin();
                }
            }
        }
    }
    /// Reads a message from the channel.
    pub unsafe fn read(&self, token: &mut Token) -> Result<T, ()> {
        if token.list.block.is_null() {
            return Err(());
        }
        let block = token.list.block as *mut Block<T>;
        let offset = token.list.offset;
        let slot = (*block).slots.get_unchecked(offset);
        slot.wait_write();
        let msg = slot.msg.get().read().assume_init();
        if offset + 1 == BLOCK_CAP {
            Block::destroy(block, 0);
        } else if slot.state.fetch_or(READ, Ordering::AcqRel) & DESTROY != 0 {
            Block::destroy(block, offset + 1);
        }
        Ok(msg)
    }
    /// Attempts to send a message into the channel.
    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        self.send(msg, None)
            .map_err(|err| match err {
                SendTimeoutError::Disconnected(msg) => TrySendError::Disconnected(msg),
                SendTimeoutError::Timeout(_) => unreachable!(),
            })
    }
    /// Sends a message into the channel.
    pub fn send(
        &self,
        msg: T,
        _deadline: Option<Instant>,
    ) -> Result<(), SendTimeoutError<T>> {
        let token = &mut Token::default();
        assert!(self.start_send(token));
        unsafe { self.write(token, msg).map_err(SendTimeoutError::Disconnected) }
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
                    unsafe {
                        return self
                            .read(token)
                            .map_err(|_| RecvTimeoutError::Disconnected);
                    }
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
            let mut tail = self.tail.index.load(Ordering::SeqCst);
            let mut head = self.head.index.load(Ordering::SeqCst);
            if self.tail.index.load(Ordering::SeqCst) == tail {
                tail &= !((1 << SHIFT) - 1);
                head &= !((1 << SHIFT) - 1);
                if (tail >> SHIFT) & (LAP - 1) == LAP - 1 {
                    tail = tail.wrapping_add(1 << SHIFT);
                }
                if (head >> SHIFT) & (LAP - 1) == LAP - 1 {
                    head = head.wrapping_add(1 << SHIFT);
                }
                let lap = (head >> SHIFT) / LAP;
                tail = tail.wrapping_sub((lap * LAP) << SHIFT);
                head = head.wrapping_sub((lap * LAP) << SHIFT);
                tail >>= SHIFT;
                head >>= SHIFT;
                return tail - head - tail / LAP;
            }
        }
    }
    /// Returns the capacity of the channel.
    pub fn capacity(&self) -> Option<usize> {
        None
    }
    /// Disconnects the channel and wakes up all blocked receivers.
    ///
    /// Returns `true` if this call disconnected the channel.
    pub fn disconnect(&self) -> bool {
        let tail = self.tail.index.fetch_or(MARK_BIT, Ordering::SeqCst);
        if tail & MARK_BIT == 0 {
            self.receivers.disconnect();
            true
        } else {
            false
        }
    }
    /// Returns `true` if the channel is disconnected.
    pub fn is_disconnected(&self) -> bool {
        self.tail.index.load(Ordering::SeqCst) & MARK_BIT != 0
    }
    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        let head = self.head.index.load(Ordering::SeqCst);
        let tail = self.tail.index.load(Ordering::SeqCst);
        head >> SHIFT == tail >> SHIFT
    }
    /// Returns `true` if the channel is full.
    pub fn is_full(&self) -> bool {
        false
    }
}
impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        let mut head = self.head.index.load(Ordering::Relaxed);
        let mut tail = self.tail.index.load(Ordering::Relaxed);
        let mut block = self.head.block.load(Ordering::Relaxed);
        head &= !((1 << SHIFT) - 1);
        tail &= !((1 << SHIFT) - 1);
        unsafe {
            while head != tail {
                let offset = (head >> SHIFT) % LAP;
                if offset < BLOCK_CAP {
                    let slot = (*block).slots.get_unchecked(offset);
                    let p = &mut *slot.msg.get();
                    p.as_mut_ptr().drop_in_place();
                } else {
                    let next = (*block).next.load(Ordering::Relaxed);
                    drop(Box::from_raw(block));
                    block = next;
                }
                head = head.wrapping_add(1 << SHIFT);
            }
            if !block.is_null() {
                drop(Box::from_raw(block));
            }
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
    fn register(&self, _oper: Operation, _cx: &Context) -> bool {
        self.is_ready()
    }
    fn unregister(&self, _oper: Operation) {}
    fn accept(&self, token: &mut Token, _cx: &Context) -> bool {
        self.try_select(token)
    }
    fn is_ready(&self) -> bool {
        true
    }
    fn watch(&self, _oper: Operation, _cx: &Context) -> bool {
        self.is_ready()
    }
    fn unwatch(&self, _oper: Operation) {}
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::flavors::list::Block;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_198_rrrruuuugggg_test_rug = 0;
        let block: Block<i32> = Block::<i32>::new();
        let _rug_ed_tests_rug_198_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::flavors::list::Block;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_199_rrrruuuugggg_test_rug = 0;
        let mut p0: Block<i32> = Block::new();
        p0.wait_next();
        let _rug_ed_tests_rug_199_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_201 {
    use super::*;
    use crate::flavors::list::ListToken;
    use std::ptr;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_201_rrrruuuugggg_test_default = 0;
        let default_token: ListToken = <ListToken as Default>::default();
        debug_assert_eq!(default_token.block, ptr::null());
        debug_assert_eq!(default_token.offset, 0);
        let _rug_ed_tests_rug_201_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::flavors::list::Channel;
    use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::ptr;
    use crossbeam_utils::CachePadded;
    use crate::{Sender, Receiver};
    use crate::flavors::list::{Position, SyncWaker};
    #[test]
    fn test_new_channel() {
        let _rug_st_tests_rug_202_rrrruuuugggg_test_new_channel = 0;
        Channel::<i32>::new();
        let _rug_ed_tests_rug_202_rrrruuuugggg_test_new_channel = 0;
    }
}
#[cfg(test)]
mod tests_rug_203 {
    use super::*;
    use crate::flavors::list::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_203_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<u32> = Channel::new();
        p0.receiver();
        let _rug_ed_tests_rug_203_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::flavors::list::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_204_rrrruuuugggg_test_rug = 0;
        let mut p0 = Channel::<i32>::new();
        Channel::<i32>::sender(&p0);
        let _rug_ed_tests_rug_204_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use crate::flavors::list::Channel;
    use crate::select::Token;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_207_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::<i32>::new();
        let mut p1: Token = Token::default();
        p0.start_recv(&mut p1);
        let _rug_ed_tests_rug_207_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_214 {
    use super::*;
    use crate::flavors::list::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_214_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        debug_assert_eq!(p0.capacity(), None);
        let _rug_ed_tests_rug_214_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_215 {
    use super::*;
    use crate::flavors::list::Channel;
    #[test]
    fn test_disconnect() {
        let _rug_st_tests_rug_215_rrrruuuugggg_test_disconnect = 0;
        let mut p0: Channel<i32> = Channel::new();
        p0.disconnect();
        let _rug_ed_tests_rug_215_rrrruuuugggg_test_disconnect = 0;
    }
}
#[cfg(test)]
mod tests_rug_216 {
    use super::*;
    use crate::flavors::list::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_216_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        debug_assert!(p0.is_disconnected());
        let _rug_ed_tests_rug_216_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::flavors::list::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_218_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        debug_assert_eq!(p0.is_full(), false);
        let _rug_ed_tests_rug_218_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_229 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::list::Sender;
    use std::time::Instant;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_229_rrrruuuugggg_test_rug = 0;
        let mut p0: Sender<'_, i32> = unimplemented!();
        p0.deadline();
        let _rug_ed_tests_rug_229_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::list::Sender;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_233_rrrruuuugggg_test_rug = 0;
        let mut p0: Sender<'_, i32> = unimplemented!();
        <Sender<'_, i32> as SelectHandle>::is_ready(&p0);
        let _rug_ed_tests_rug_233_rrrruuuugggg_test_rug = 0;
    }
}
