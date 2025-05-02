//! The implementation is based on Dmitry Vyukov's bounded MPMC queue.
//!
//! Source:
//!   - http://www.1024cores.net/home/lock-free-algorithms/queues/bounded-mpmc-queue
//!
//! Copyright & License:
//!   - Copyright (c) 2010-2011 Dmitry Vyukov
//!   - Simplified BSD License and Apache License, Version 2.0
//!   - http://www.1024cores.net/home/code-license
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::fmt;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::sync::atomic::{self, AtomicUsize, Ordering};
use crossbeam_utils::{Backoff, CachePadded};
use crate::err::{PopError, PushError};
/// A slot in a queue.
struct Slot<T> {
    /// The current stamp.
    ///
    /// If the stamp equals the tail, this node will be next written to. If it equals head + 1,
    /// this node will be next read from.
    stamp: AtomicUsize,
    /// The value in this slot.
    value: UnsafeCell<MaybeUninit<T>>,
}
/// A bounded multi-producer multi-consumer queue.
///
/// This queue allocates a fixed-capacity buffer on construction, which is used to store pushed
/// elements. The queue cannot hold more elements than the buffer allows. Attempting to push an
/// element into a full queue will fail. Having a buffer allocated upfront makes this queue a bit
/// faster than [`SegQueue`].
///
/// [`SegQueue`]: struct.SegQueue.html
///
/// # Examples
///
/// ```
/// use crossbeam_queue::{ArrayQueue, PushError};
///
/// let q = ArrayQueue::new(2);
///
/// assert_eq!(q.push('a'), Ok(()));
/// assert_eq!(q.push('b'), Ok(()));
/// assert_eq!(q.push('c'), Err(PushError('c')));
/// assert_eq!(q.pop(), Ok('a'));
/// ```
pub struct ArrayQueue<T> {
    /// The head of the queue.
    ///
    /// This value is a "stamp" consisting of an index into the buffer and a lap, but packed into a
    /// single `usize`. The lower bits represent the index, while the upper bits represent the lap.
    ///
    /// Elements are popped from the head of the queue.
    head: CachePadded<AtomicUsize>,
    /// The tail of the queue.
    ///
    /// This value is a "stamp" consisting of an index into the buffer and a lap, but packed into a
    /// single `usize`. The lower bits represent the index, while the upper bits represent the lap.
    ///
    /// Elements are pushed into the tail of the queue.
    tail: CachePadded<AtomicUsize>,
    /// The buffer holding slots.
    buffer: *mut Slot<T>,
    /// The queue capacity.
    cap: usize,
    /// A stamp with the value of `{ lap: 1, index: 0 }`.
    one_lap: usize,
    /// Indicates that dropping an `ArrayQueue<T>` may drop elements of type `T`.
    _marker: PhantomData<T>,
}
unsafe impl<T: Send> Sync for ArrayQueue<T> {}
unsafe impl<T: Send> Send for ArrayQueue<T> {}
impl<T> ArrayQueue<T> {
    /// Creates a new bounded queue with the given capacity.
    ///
    /// # Panics
    ///
    /// Panics if the capacity is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::ArrayQueue;
    ///
    /// let q = ArrayQueue::<i32>::new(100);
    /// ```
    pub fn new(cap: usize) -> ArrayQueue<T> {
        assert!(cap > 0, "capacity must be non-zero");
        let head = 0;
        let tail = 0;
        let buffer = {
            let mut v: Vec<Slot<T>> = (0..cap)
                .map(|i| {
                    Slot {
                        stamp: AtomicUsize::new(i),
                        value: UnsafeCell::new(MaybeUninit::uninit()),
                    }
                })
                .collect();
            let ptr = v.as_mut_ptr();
            mem::forget(v);
            ptr
        };
        let one_lap = (cap + 1).next_power_of_two();
        ArrayQueue {
            buffer,
            cap,
            one_lap,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
            _marker: PhantomData,
        }
    }
    /// Attempts to push an element into the queue.
    ///
    /// If the queue is full, the element is returned back as an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::{ArrayQueue, PushError};
    ///
    /// let q = ArrayQueue::new(1);
    ///
    /// assert_eq!(q.push(10), Ok(()));
    /// assert_eq!(q.push(20), Err(PushError(20)));
    /// ```
    pub fn push(&self, value: T) -> Result<(), PushError<T>> {
        let backoff = Backoff::new();
        let mut tail = self.tail.load(Ordering::Relaxed);
        loop {
            let index = tail & (self.one_lap - 1);
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
                        unsafe {
                            slot.value.get().write(MaybeUninit::new(value));
                        }
                        slot.stamp.store(tail + 1, Ordering::Release);
                        return Ok(());
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
                    return Err(PushError(value));
                }
                backoff.spin();
                tail = self.tail.load(Ordering::Relaxed);
            } else {
                backoff.snooze();
                tail = self.tail.load(Ordering::Relaxed);
            }
        }
    }
    /// Attempts to pop an element from the queue.
    ///
    /// If the queue is empty, an error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::{ArrayQueue, PopError};
    ///
    /// let q = ArrayQueue::new(1);
    /// assert_eq!(q.push(10), Ok(()));
    ///
    /// assert_eq!(q.pop(), Ok(10));
    /// assert_eq!(q.pop(), Err(PopError));
    /// ```
    pub fn pop(&self) -> Result<T, PopError> {
        let backoff = Backoff::new();
        let mut head = self.head.load(Ordering::Relaxed);
        loop {
            let index = head & (self.one_lap - 1);
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
                        let msg = unsafe { slot.value.get().read().assume_init() };
                        slot.stamp
                            .store(head.wrapping_add(self.one_lap), Ordering::Release);
                        return Ok(msg);
                    }
                    Err(h) => {
                        head = h;
                        backoff.spin();
                    }
                }
            } else if stamp == head {
                atomic::fence(Ordering::SeqCst);
                let tail = self.tail.load(Ordering::Relaxed);
                if tail == head {
                    return Err(PopError);
                }
                backoff.spin();
                head = self.head.load(Ordering::Relaxed);
            } else {
                backoff.snooze();
                head = self.head.load(Ordering::Relaxed);
            }
        }
    }
    /// Returns the capacity of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::ArrayQueue;
    ///
    /// let q = ArrayQueue::<i32>::new(100);
    ///
    /// assert_eq!(q.capacity(), 100);
    /// ```
    pub fn capacity(&self) -> usize {
        self.cap
    }
    /// Returns `true` if the queue is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::ArrayQueue;
    ///
    /// let q = ArrayQueue::new(100);
    ///
    /// assert!(q.is_empty());
    /// q.push(1).unwrap();
    /// assert!(!q.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        tail == head
    }
    /// Returns `true` if the queue is full.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::ArrayQueue;
    ///
    /// let q = ArrayQueue::new(1);
    ///
    /// assert!(!q.is_full());
    /// q.push(1).unwrap();
    /// assert!(q.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);
        head.wrapping_add(self.one_lap) == tail
    }
    /// Returns the number of elements in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_queue::ArrayQueue;
    ///
    /// let q = ArrayQueue::new(100);
    /// assert_eq!(q.len(), 0);
    ///
    /// q.push(10).unwrap();
    /// assert_eq!(q.len(), 1);
    ///
    /// q.push(20).unwrap();
    /// assert_eq!(q.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        loop {
            let tail = self.tail.load(Ordering::SeqCst);
            let head = self.head.load(Ordering::SeqCst);
            if self.tail.load(Ordering::SeqCst) == tail {
                let hix = head & (self.one_lap - 1);
                let tix = tail & (self.one_lap - 1);
                return if hix < tix {
                    tix - hix
                } else if hix > tix {
                    self.cap - hix + tix
                } else if tail == head {
                    0
                } else {
                    self.cap
                };
            }
        }
    }
}
impl<T> Drop for ArrayQueue<T> {
    fn drop(&mut self) {
        let hix = self.head.load(Ordering::Relaxed) & (self.one_lap - 1);
        for i in 0..self.len() {
            let index = if hix + i < self.cap { hix + i } else { hix + i - self.cap };
            unsafe {
                let p = {
                    let slot = &mut *self.buffer.add(index);
                    let value = &mut *slot.value.get();
                    value.as_mut_ptr()
                };
                p.drop_in_place();
            }
        }
        unsafe {
            Vec::from_raw_parts(self.buffer, 0, self.cap);
        }
    }
}
impl<T> fmt::Debug for ArrayQueue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("ArrayQueue { .. }")
    }
}
#[cfg(test)]
mod tests_rug_496 {
    use super::*;
    use crate::ArrayQueue;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::cell::UnsafeCell;
    use std::mem::{self, MaybeUninit};
    use std::ptr;
    use std::marker::PhantomData;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_496_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let cap: usize = rug_fuzz_0;
        ArrayQueue::<i32>::new(cap);
        let _rug_ed_tests_rug_496_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_497 {
    use super::*;
    use crate::{ArrayQueue, PushError};
    #[test]
    fn test_array_queue_push() {
        let _rug_st_tests_rug_497_rrrruuuugggg_test_array_queue_push = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 10;
        let q = ArrayQueue::<i32>::new(rug_fuzz_0);
        let value_to_push = rug_fuzz_1;
        debug_assert_eq!(q.push(value_to_push), Ok(()));
        let _rug_ed_tests_rug_497_rrrruuuugggg_test_array_queue_push = 0;
    }
}
#[cfg(test)]
mod tests_rug_498 {
    use super::*;
    use crate::{ArrayQueue, PopError};
    #[test]
    fn test_pop() {
        let _rug_st_tests_rug_498_rrrruuuugggg_test_pop = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 10;
        let q = ArrayQueue::<i32>::new(rug_fuzz_0);
        debug_assert_eq!(q.push(rug_fuzz_1), Ok(()));
        debug_assert_eq!(ArrayQueue:: < i32 > ::pop(& q), Ok(10));
        debug_assert_eq!(ArrayQueue:: < i32 > ::pop(& q), Err(PopError));
        let _rug_ed_tests_rug_498_rrrruuuugggg_test_pop = 0;
    }
}
#[cfg(test)]
mod tests_rug_499 {
    use super::*;
    use crate::ArrayQueue;
    #[test]
    fn test_capacity() {
        let _rug_st_tests_rug_499_rrrruuuugggg_test_capacity = 0;
        let rug_fuzz_0 = 100;
        let mut p0: ArrayQueue<i32> = ArrayQueue::new(rug_fuzz_0);
        ArrayQueue::<i32>::capacity(&p0);
        let _rug_ed_tests_rug_499_rrrruuuugggg_test_capacity = 0;
    }
}
#[cfg(test)]
mod tests_rug_500 {
    use super::*;
    use crate::ArrayQueue;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_rug_500_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 1;
        let mut q: ArrayQueue<i32> = ArrayQueue::new(rug_fuzz_0);
        debug_assert!(q.is_empty());
        q.push(rug_fuzz_1).unwrap();
        debug_assert!(! q.is_empty());
        let _rug_ed_tests_rug_500_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_501 {
    use super::*;
    use crate::ArrayQueue;
    #[test]
    fn test_is_full() {
        let _rug_st_tests_rug_501_rrrruuuugggg_test_is_full = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let q = ArrayQueue::<i32>::new(rug_fuzz_0);
        debug_assert!(! q.is_full());
        q.push(rug_fuzz_1).unwrap();
        debug_assert!(q.is_full());
        let _rug_ed_tests_rug_501_rrrruuuugggg_test_is_full = 0;
    }
}
#[cfg(test)]
mod tests_rug_502 {
    use super::*;
    use crate::ArrayQueue;
    #[test]
    fn test_array_queue_len() {
        let _rug_st_tests_rug_502_rrrruuuugggg_test_array_queue_len = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 20;
        let mut p0: ArrayQueue<i32> = ArrayQueue::new(rug_fuzz_0);
        debug_assert_eq!(< ArrayQueue < i32 > > ::len(& p0), 0);
        p0.push(rug_fuzz_1).unwrap();
        debug_assert_eq!(< ArrayQueue < i32 > > ::len(& p0), 1);
        p0.push(rug_fuzz_2).unwrap();
        debug_assert_eq!(< ArrayQueue < i32 > > ::len(& p0), 2);
        let _rug_ed_tests_rug_502_rrrruuuugggg_test_array_queue_len = 0;
    }
}
