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

        // Head is initialized to `{ lap: 0, index: 0 }`.
        // Tail is initialized to `{ lap: 0, index: 0 }`.
        let head = 0;
        let tail = 0;

        // Allocate a buffer of `cap` slots initialized
        // with stamps.
        let buffer = {
            let mut v: Vec<Slot<T>> = (0..cap)
                .map(|i| {
                    // Set the stamp to `{ lap: 0, index: i }`.
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

        // One lap is the smallest power of two greater than `cap`.
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
            // Deconstruct the tail.
            let index = tail & (self.one_lap - 1);
            let lap = tail & !(self.one_lap - 1);

            // Inspect the corresponding slot.
            let slot = unsafe { &*self.buffer.add(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);

            // If the tail and the stamp match, we may attempt to push.
            if tail == stamp {
                let new_tail = if index + 1 < self.cap {
                    // Same lap, incremented index.
                    // Set to `{ lap: lap, index: index + 1 }`.
                    tail + 1
                } else {
                    // One lap forward, index wraps around to zero.
                    // Set to `{ lap: lap.wrapping_add(1), index: 0 }`.
                    lap.wrapping_add(self.one_lap)
                };

                // Try moving the tail.
                match self.tail.compare_exchange_weak(
                    tail,
                    new_tail,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Write the value into the slot and update the stamp.
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

                // If the head lags one lap behind the tail as well...
                if head.wrapping_add(self.one_lap) == tail {
                    // ...then the queue is full.
                    return Err(PushError(value));
                }

                backoff.spin();
                tail = self.tail.load(Ordering::Relaxed);
            } else {
                // Snooze because we need to wait for the stamp to get updated.
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
            // Deconstruct the head.
            let index = head & (self.one_lap - 1);
            let lap = head & !(self.one_lap - 1);

            // Inspect the corresponding slot.
            let slot = unsafe { &*self.buffer.add(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);

            // If the the stamp is ahead of the head by 1, we may attempt to pop.
            if head + 1 == stamp {
                let new = if index + 1 < self.cap {
                    // Same lap, incremented index.
                    // Set to `{ lap: lap, index: index + 1 }`.
                    head + 1
                } else {
                    // One lap forward, index wraps around to zero.
                    // Set to `{ lap: lap.wrapping_add(1), index: 0 }`.
                    lap.wrapping_add(self.one_lap)
                };

                // Try moving the head.
                match self.head.compare_exchange_weak(
                    head,
                    new,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Read the value from the slot and update the stamp.
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

                // If the tail equals the head, that means the channel is empty.
                if tail == head {
                    return Err(PopError);
                }

                backoff.spin();
                head = self.head.load(Ordering::Relaxed);
            } else {
                // Snooze because we need to wait for the stamp to get updated.
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

        // Is the tail lagging one lap behind head?
        // Is the tail equal to the head?
        //
        // Note: If the head changes just before we load the tail, that means there was a moment
        // when the channel was not empty, so it is safe to just return `false`.
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

        // Is the head lagging one lap behind tail?
        //
        // Note: If the tail changes just before we load the head, that means there was a moment
        // when the queue was not full, so it is safe to just return `false`.
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
            // Load the tail, then load the head.
            let tail = self.tail.load(Ordering::SeqCst);
            let head = self.head.load(Ordering::SeqCst);

            // If the tail didn't change, we've got consistent values to work with.
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
        // Get the index of the head.
        let hix = self.head.load(Ordering::Relaxed) & (self.one_lap - 1);

        // Loop over all slots that hold a message and drop them.
        for i in 0..self.len() {
            // Compute the index of the next slot holding a message.
            let index = if hix + i < self.cap {
                hix + i
            } else {
                hix + i - self.cap
            };

            unsafe {
                let p = {
                    let slot = &mut *self.buffer.add(index);
                    let value = &mut *slot.value.get();
                    value.as_mut_ptr()
                };
                p.drop_in_place();
            }
        }

        // Finally, deallocate the buffer, but don't run any destructors.
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
        let cap: usize = 100;

        ArrayQueue::<i32>::new(cap);
    }
}#[cfg(test)]
mod tests_rug_497 {
    use super::*;
    use crate::{ArrayQueue, PushError};

    #[test]
    fn test_array_queue_push() {
        let q = ArrayQueue::<i32>::new(1);

        let value_to_push = 10;

        assert_eq!(q.push(value_to_push), Ok(()));
    }
}#[cfg(test)]
mod tests_rug_498 {
    use super::*;
    use crate::{ArrayQueue, PopError};

    #[test]
    fn test_pop() {
        let q = ArrayQueue::<i32>::new(1);
        assert_eq!(q.push(10), Ok(()));

        assert_eq!(ArrayQueue::<i32>::pop(&q), Ok(10));
        assert_eq!(ArrayQueue::<i32>::pop(&q), Err(PopError));
    }
}#[cfg(test)]
mod tests_rug_499 {
    use super::*;
    use crate::ArrayQueue;

    #[test]
    fn test_capacity() {
        let mut p0: ArrayQueue<i32> = ArrayQueue::new(100);

        ArrayQueue::<i32>::capacity(&p0);
    }
}#[cfg(test)]
mod tests_rug_500 {
    use super::*;
    use crate::ArrayQueue;

    #[test]
    fn test_is_empty() {
        let mut q: ArrayQueue<i32> = ArrayQueue::new(100);

        assert!(q.is_empty());
        q.push(1).unwrap();
        assert!(!q.is_empty());
    }
}#[cfg(test)]
mod tests_rug_501 {
    use super::*;
    use crate::ArrayQueue;

    #[test]
    fn test_is_full() {
        let q = ArrayQueue::<i32>::new(1);
        assert!(!q.is_full());
        q.push(1).unwrap();
        assert!(q.is_full());
    }
}#[cfg(test)]
mod tests_rug_502 {
    use super::*;
    use crate::ArrayQueue;

    #[test]
    fn test_array_queue_len() {
        let mut p0: ArrayQueue<i32> = ArrayQueue::new(100);

        assert_eq!(<ArrayQueue<i32>>::len(&p0), 0);

        p0.push(10).unwrap();
        assert_eq!(<ArrayQueue<i32>>::len(&p0), 1);

        p0.push(20).unwrap();
        assert_eq!(<ArrayQueue<i32>>::len(&p0), 2);
    }
}