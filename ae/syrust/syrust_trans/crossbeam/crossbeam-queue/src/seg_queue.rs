use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::{self, AtomicPtr, AtomicUsize, Ordering};
use crossbeam_utils::{Backoff, CachePadded};
use crate::err::PopError;
const WRITE: usize = 1;
const READ: usize = 2;
const DESTROY: usize = 4;
const LAP: usize = 32;
const BLOCK_CAP: usize = LAP - 1;
const SHIFT: usize = 1;
const HAS_NEXT: usize = 1;
/// A slot in a block.
struct Slot<T> {
    /// The value.
    value: UnsafeCell<MaybeUninit<T>>,
    /// The state of the slot.
    state: AtomicUsize,
}
impl<T> Slot<T> {
    /// Waits until a value is written into the slot.
    fn wait_write(&self) {
        let backoff = Backoff::new();
        while self.state.load(Ordering::Acquire) & WRITE == 0 {
            backoff.snooze();
        }
    }
}
/// A block in a linked list.
///
/// Each block in the list can hold up to `BLOCK_CAP` values.
struct Block<T> {
    /// The next block in the linked list.
    next: AtomicPtr<Block<T>>,
    /// Slots for values.
    slots: [Slot<T>; BLOCK_CAP],
}
impl<T> Block<T> {
    /// Creates an empty block that starts at `start_index`.
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
/// A position in a queue.
struct Position<T> {
    /// The index in the queue.
    index: AtomicUsize,
    /// The block in the linked list.
    block: AtomicPtr<Block<T>>,
}
/// An unbounded multi-producer multi-consumer queue.
///
/// This queue is implemented as a linked list of segments, where each segment is a small buffer
/// that can hold a handful of elements. There is no limit to how many elements can be in the queue
/// at a time. However, since segments need to be dynamically allocated as elements get pushed,
/// this queue is somewhat slower than [`ArrayQueue`].
///
/// [`ArrayQueue`]: struct.ArrayQueue.html
///
/// # Examples
///
/// ```
/// use crossbeam_queue::{PopError, SegQueue};
///
/// let q = SegQueue::new();
///
/// q.push('a');
/// q.push('b');
///
/// assert_eq!(q.pop(), Ok('a'));
/// assert_eq!(q.pop(), Ok('b'));
/// assert_eq!(q.pop(), Err(PopError));
/// ```
pub struct SegQueue<T> {
    /// The head of the queue.
    head: CachePadded<Position<T>>,
    /// The tail of the queue.
    tail: CachePadded<Position<T>>,
    /// Indicates that dropping a `SegQueue<T>` may drop values of type `T`.
    _marker: PhantomData<T>,
}
unsafe impl<T: Send> Send for SegQueue<T> {}
unsafe impl<T: Send> Sync for SegQueue<T> {}
impl<T> SegQueue<T> {
    /// Creates a new unbounded queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::SegQueue;
    ///
    /// let q = SegQueue::<i32>::new();
    /// ```
    pub fn new() -> SegQueue<T> {
        SegQueue {
            head: CachePadded::new(Position {
                block: AtomicPtr::new(ptr::null_mut()),
                index: AtomicUsize::new(0),
            }),
            tail: CachePadded::new(Position {
                block: AtomicPtr::new(ptr::null_mut()),
                index: AtomicUsize::new(0),
            }),
            _marker: PhantomData,
        }
    }
    /// Pushes an element into the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::SegQueue;
    ///
    /// let q = SegQueue::new();
    ///
    /// q.push(10);
    /// q.push(20);
    /// ```
    pub fn push(&self, value: T) {
        let backoff = Backoff::new();
        let mut tail = self.tail.index.load(Ordering::Acquire);
        let mut block = self.tail.block.load(Ordering::Acquire);
        let mut next_block = None;
        loop {
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
                            let next_index = new_tail.wrapping_add(1 << SHIFT);
                            self.tail.block.store(next_block, Ordering::Release);
                            self.tail.index.store(next_index, Ordering::Release);
                            (*block).next.store(next_block, Ordering::Release);
                        }
                        let slot = (*block).slots.get_unchecked(offset);
                        slot.value.get().write(MaybeUninit::new(value));
                        slot.state.fetch_or(WRITE, Ordering::Release);
                        return;
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
    /// Pops an element from the queue.
    ///
    /// If the queue is empty, an error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::{PopError, SegQueue};
    ///
    /// let q = SegQueue::new();
    ///
    /// q.push(10);
    /// assert_eq!(q.pop(), Ok(10));
    /// assert_eq!(q.pop(), Err(PopError));
    /// ```
    pub fn pop(&self) -> Result<T, PopError> {
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
            if new_head & HAS_NEXT == 0 {
                atomic::fence(Ordering::SeqCst);
                let tail = self.tail.index.load(Ordering::Relaxed);
                if head >> SHIFT == tail >> SHIFT {
                    return Err(PopError);
                }
                if (head >> SHIFT) / LAP != (tail >> SHIFT) / LAP {
                    new_head |= HAS_NEXT;
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
                            let mut next_index = (new_head & !HAS_NEXT)
                                .wrapping_add(1 << SHIFT);
                            if !(*next).next.load(Ordering::Relaxed).is_null() {
                                next_index |= HAS_NEXT;
                            }
                            self.head.block.store(next, Ordering::Release);
                            self.head.index.store(next_index, Ordering::Release);
                        }
                        let slot = (*block).slots.get_unchecked(offset);
                        slot.wait_write();
                        let value = slot.value.get().read().assume_init();
                        if offset + 1 == BLOCK_CAP {
                            Block::destroy(block, 0);
                        } else if slot.state.fetch_or(READ, Ordering::AcqRel) & DESTROY
                            != 0
                        {
                            Block::destroy(block, offset + 1);
                        }
                        return Ok(value);
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
    /// Returns `true` if the queue is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::SegQueue;
    ///
    /// let q = SegQueue::new();
    ///
    /// assert!(q.is_empty());
    /// q.push(1);
    /// assert!(!q.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let head = self.head.index.load(Ordering::SeqCst);
        let tail = self.tail.index.load(Ordering::SeqCst);
        head >> SHIFT == tail >> SHIFT
    }
    /// Returns the number of elements in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::SegQueue;
    ///
    /// let q = SegQueue::new();
    /// assert_eq!(q.len(), 0);
    ///
    /// q.push(10);
    /// assert_eq!(q.len(), 1);
    ///
    /// q.push(20);
    /// assert_eq!(q.len(), 2);
    /// ```
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
}
impl<T> Drop for SegQueue<T> {
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
                    let p = &mut *slot.value.get();
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
impl<T> fmt::Debug for SegQueue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("SegQueue { .. }")
    }
}
impl<T> Default for SegQueue<T> {
    fn default() -> SegQueue<T> {
        SegQueue::new()
    }
}
#[cfg(test)]
mod tests_rug_507 {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use crate::seg_queue::{Block, BLOCK_CAP, DESTROY, READ};
    #[test]
    fn test_seg_queue_destroy() {
        let _rug_st_tests_rug_507_rrrruuuugggg_test_seg_queue_destroy = 0;
        let rug_fuzz_0 = 0;
        let mut block = Box::into_raw(Box::new(Block::<usize>::new()));
        let start = rug_fuzz_0;
        unsafe {
            Block::<usize>::destroy(block, start);
        }
        let _rug_ed_tests_rug_507_rrrruuuugggg_test_seg_queue_destroy = 0;
    }
}
#[cfg(test)]
mod tests_rug_508 {
    use super::*;
    use crate::SegQueue;
    use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
    use std::ptr;
    use std::marker::PhantomData;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_508_rrrruuuugggg_test_rug = 0;
        let q: SegQueue<i32> = SegQueue::new();
        let _rug_ed_tests_rug_508_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_509 {
    use super::*;
    use crate::SegQueue;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_509_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = SegQueue::new();
        let mut p1 = rug_fuzz_0;
        p0.push(p1);
        let _rug_ed_tests_rug_509_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_510 {
    use super::*;
    use crate::{PopError, SegQueue};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_510_rrrruuuugggg_test_rug = 0;
        let mut p0: SegQueue<i32> = SegQueue::new();
        p0.pop();
        let _rug_ed_tests_rug_510_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_511 {
    use super::*;
    use crate::SegQueue;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_rug_511_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = 1;
        let q = SegQueue::<i32>::new();
        debug_assert!(q.is_empty());
        q.push(rug_fuzz_0);
        debug_assert!(! q.is_empty());
        let _rug_ed_tests_rug_511_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_512 {
    use super::*;
    use crate::SegQueue;
    #[test]
    fn test_len() {
        let _rug_st_tests_rug_512_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let q = SegQueue::<i32>::new();
        debug_assert_eq!(q.len(), 0);
        q.push(rug_fuzz_0);
        debug_assert_eq!(q.len(), 1);
        q.push(rug_fuzz_1);
        debug_assert_eq!(q.len(), 2);
        let _rug_ed_tests_rug_512_rrrruuuugggg_test_len = 0;
    }
}
#[cfg(test)]
mod tests_rug_514 {
    use super::*;
    use crate::seg_queue::SegQueue;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_514_rrrruuuugggg_test_default = 0;
        let queue: SegQueue<i32> = <SegQueue<i32>>::default();
        let _rug_ed_tests_rug_514_rrrruuuugggg_test_default = 0;
    }
}
