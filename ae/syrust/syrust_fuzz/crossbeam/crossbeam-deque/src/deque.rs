#![allow(clippy::mut_range_bound)]
use std::cell::{Cell, UnsafeCell};
use std::cmp;
use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::ptr;
use std::sync::atomic::{self, AtomicIsize, AtomicPtr, AtomicUsize, Ordering};
use std::sync::Arc;
use crate::epoch::{self, Atomic, Owned};
use crate::utils::{Backoff, CachePadded};
const MIN_CAP: usize = 64;
const MAX_BATCH: usize = 32;
const FLUSH_THRESHOLD_BYTES: usize = 1 << 10;
/// A buffer that holds tasks in a worker queue.
///
/// This is just a pointer to the buffer and its length - dropping an instance of this struct will
/// *not* deallocate the buffer.
struct Buffer<T> {
    /// Pointer to the allocated memory.
    ptr: *mut T,
    /// Capacity of the buffer. Always a power of two.
    cap: usize,
}
unsafe impl<T> Send for Buffer<T> {}
impl<T> Buffer<T> {
    /// Allocates a new buffer with the specified capacity.
    fn alloc(cap: usize) -> Buffer<T> {
        debug_assert_eq!(cap, cap.next_power_of_two());
        let mut v = Vec::with_capacity(cap);
        let ptr = v.as_mut_ptr();
        mem::forget(v);
        Buffer { ptr, cap }
    }
    /// Deallocates the buffer.
    unsafe fn dealloc(self) {
        drop(Vec::from_raw_parts(self.ptr, 0, self.cap));
    }
    /// Returns a pointer to the task at the specified `index`.
    unsafe fn at(&self, index: isize) -> *mut T {
        self.ptr.offset(index & (self.cap - 1) as isize)
    }
    /// Writes `task` into the specified `index`.
    ///
    /// This method might be concurrently called with another `read` at the same index, which is
    /// technically speaking a data race and therefore UB. We should use an atomic store here, but
    /// that would be more expensive and difficult to implement generically for all types `T`.
    /// Hence, as a hack, we use a volatile write instead.
    unsafe fn write(&self, index: isize, task: T) {
        ptr::write_volatile(self.at(index), task)
    }
    /// Reads a task from the specified `index`.
    ///
    /// This method might be concurrently called with another `write` at the same index, which is
    /// technically speaking a data race and therefore UB. We should use an atomic load here, but
    /// that would be more expensive and difficult to implement generically for all types `T`.
    /// Hence, as a hack, we use a volatile write instead.
    unsafe fn read(&self, index: isize) -> T {
        ptr::read_volatile(self.at(index))
    }
}
impl<T> Clone for Buffer<T> {
    fn clone(&self) -> Buffer<T> {
        Buffer {
            ptr: self.ptr,
            cap: self.cap,
        }
    }
}
impl<T> Copy for Buffer<T> {}
/// Internal queue data shared between the worker and stealers.
///
/// The implementation is based on the following work:
///
/// 1. [Chase and Lev. Dynamic circular work-stealing deque. SPAA 2005.][chase-lev]
/// 2. [Le, Pop, Cohen, and Nardelli. Correct and efficient work-stealing for weak memory models.
///    PPoPP 2013.][weak-mem]
/// 3. [Norris and Demsky. CDSchecker: checking concurrent data structures written with C/C++
///    atomics. OOPSLA 2013.][checker]
///
/// [chase-lev]: https://dl.acm.org/citation.cfm?id=1073974
/// [weak-mem]: https://dl.acm.org/citation.cfm?id=2442524
/// [checker]: https://dl.acm.org/citation.cfm?id=2509514
struct Inner<T> {
    /// The front index.
    front: AtomicIsize,
    /// The back index.
    back: AtomicIsize,
    /// The underlying buffer.
    buffer: CachePadded<Atomic<Buffer<T>>>,
}
impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        let b = self.back.load(Ordering::Relaxed);
        let f = self.front.load(Ordering::Relaxed);
        unsafe {
            let buffer = self.buffer.load(Ordering::Relaxed, epoch::unprotected());
            let mut i = f;
            while i != b {
                buffer.deref().at(i).drop_in_place();
                i = i.wrapping_add(1);
            }
            buffer.into_owned().into_box().dealloc();
        }
    }
}
/// Worker queue flavor: FIFO or LIFO.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Flavor {
    /// The first-in first-out flavor.
    Fifo,
    /// The last-in first-out flavor.
    Lifo,
}
/// A worker queue.
///
/// This is a FIFO or LIFO queue that is owned by a single thread, but other threads may steal
/// tasks from it. Task schedulers typically create a single worker queue per thread.
///
/// # Examples
///
/// A FIFO worker:
///
/// ```
/// use crossbeam_deque::{Steal, Worker};
///
/// let w = Worker::new_fifo();
/// let s = w.stealer();
///
/// w.push(1);
/// w.push(2);
/// w.push(3);
///
/// assert_eq!(s.steal(), Steal::Success(1));
/// assert_eq!(w.pop(), Some(2));
/// assert_eq!(w.pop(), Some(3));
/// ```
///
/// A LIFO worker:
///
/// ```
/// use crossbeam_deque::{Steal, Worker};
///
/// let w = Worker::new_lifo();
/// let s = w.stealer();
///
/// w.push(1);
/// w.push(2);
/// w.push(3);
///
/// assert_eq!(s.steal(), Steal::Success(1));
/// assert_eq!(w.pop(), Some(3));
/// assert_eq!(w.pop(), Some(2));
/// ```
pub struct Worker<T> {
    /// A reference to the inner representation of the queue.
    inner: Arc<CachePadded<Inner<T>>>,
    /// A copy of `inner.buffer` for quick access.
    buffer: Cell<Buffer<T>>,
    /// The flavor of the queue.
    flavor: Flavor,
    /// Indicates that the worker cannot be shared among threads.
    _marker: PhantomData<*mut ()>,
}
unsafe impl<T: Send> Send for Worker<T> {}
impl<T> Worker<T> {
    /// Creates a FIFO worker queue.
    ///
    /// Tasks are pushed and popped from opposite ends.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w = Worker::<i32>::new_fifo();
    /// ```
    pub fn new_fifo() -> Worker<T> {
        let buffer = Buffer::alloc(MIN_CAP);
        let inner = Arc::new(
            CachePadded::new(Inner {
                front: AtomicIsize::new(0),
                back: AtomicIsize::new(0),
                buffer: CachePadded::new(Atomic::new(buffer)),
            }),
        );
        Worker {
            inner,
            buffer: Cell::new(buffer),
            flavor: Flavor::Fifo,
            _marker: PhantomData,
        }
    }
    /// Creates a LIFO worker queue.
    ///
    /// Tasks are pushed and popped from the same end.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w = Worker::<i32>::new_lifo();
    /// ```
    pub fn new_lifo() -> Worker<T> {
        let buffer = Buffer::alloc(MIN_CAP);
        let inner = Arc::new(
            CachePadded::new(Inner {
                front: AtomicIsize::new(0),
                back: AtomicIsize::new(0),
                buffer: CachePadded::new(Atomic::new(buffer)),
            }),
        );
        Worker {
            inner,
            buffer: Cell::new(buffer),
            flavor: Flavor::Lifo,
            _marker: PhantomData,
        }
    }
    /// Creates a stealer for this queue.
    ///
    /// The returned stealer can be shared among threads and cloned.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w = Worker::<i32>::new_lifo();
    /// let s = w.stealer();
    /// ```
    pub fn stealer(&self) -> Stealer<T> {
        Stealer {
            inner: self.inner.clone(),
            flavor: self.flavor,
        }
    }
    /// Resizes the internal buffer to the new capacity of `new_cap`.
    #[cold]
    unsafe fn resize(&self, new_cap: usize) {
        let b = self.inner.back.load(Ordering::Relaxed);
        let f = self.inner.front.load(Ordering::Relaxed);
        let buffer = self.buffer.get();
        let new = Buffer::alloc(new_cap);
        let mut i = f;
        while i != b {
            ptr::copy_nonoverlapping(buffer.at(i), new.at(i), 1);
            i = i.wrapping_add(1);
        }
        let guard = &epoch::pin();
        self.buffer.replace(new);
        let old = self
            .inner
            .buffer
            .swap(Owned::new(new).into_shared(guard), Ordering::Release, guard);
        guard.defer_unchecked(move || old.into_owned().into_box().dealloc());
        if mem::size_of::<T>() * new_cap >= FLUSH_THRESHOLD_BYTES {
            guard.flush();
        }
    }
    /// Reserves enough capacity so that `reserve_cap` tasks can be pushed without growing the
    /// buffer.
    fn reserve(&self, reserve_cap: usize) {
        if reserve_cap > 0 {
            let b = self.inner.back.load(Ordering::Relaxed);
            let f = self.inner.front.load(Ordering::SeqCst);
            let len = b.wrapping_sub(f) as usize;
            let cap = self.buffer.get().cap;
            if cap - len < reserve_cap {
                let mut new_cap = cap * 2;
                while new_cap - len < reserve_cap {
                    new_cap *= 2;
                }
                unsafe {
                    self.resize(new_cap);
                }
            }
        }
    }
    /// Returns `true` if the queue is empty.
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w = Worker::new_lifo();
    ///
    /// assert!(w.is_empty());
    /// w.push(1);
    /// assert!(!w.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let b = self.inner.back.load(Ordering::Relaxed);
        let f = self.inner.front.load(Ordering::SeqCst);
        b.wrapping_sub(f) <= 0
    }
    /// Returns the number of tasks in the deque.
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w = Worker::new_lifo();
    ///
    /// assert_eq!(w.len(), 0);
    /// w.push(1);
    /// assert_eq!(w.len(), 1);
    /// w.push(1);
    /// assert_eq!(w.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        let b = self.inner.back.load(Ordering::Relaxed);
        let f = self.inner.front.load(Ordering::SeqCst);
        b.wrapping_sub(f).max(0) as usize
    }
    /// Pushes a task into the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w = Worker::new_lifo();
    /// w.push(1);
    /// w.push(2);
    /// ```
    pub fn push(&self, task: T) {
        let b = self.inner.back.load(Ordering::Relaxed);
        let f = self.inner.front.load(Ordering::Acquire);
        let mut buffer = self.buffer.get();
        let len = b.wrapping_sub(f);
        if len >= buffer.cap as isize {
            unsafe {
                self.resize(2 * buffer.cap);
            }
            buffer = self.buffer.get();
        }
        unsafe {
            buffer.write(b, task);
        }
        atomic::fence(Ordering::Release);
        self.inner.back.store(b.wrapping_add(1), Ordering::Release);
    }
    /// Pops a task from the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w = Worker::new_fifo();
    /// w.push(1);
    /// w.push(2);
    ///
    /// assert_eq!(w.pop(), Some(1));
    /// assert_eq!(w.pop(), Some(2));
    /// assert_eq!(w.pop(), None);
    /// ```
    pub fn pop(&self) -> Option<T> {
        let b = self.inner.back.load(Ordering::Relaxed);
        let f = self.inner.front.load(Ordering::Relaxed);
        let len = b.wrapping_sub(f);
        if len <= 0 {
            return None;
        }
        match self.flavor {
            Flavor::Fifo => {
                let f = self.inner.front.fetch_add(1, Ordering::SeqCst);
                let new_f = f.wrapping_add(1);
                if b.wrapping_sub(new_f) < 0 {
                    self.inner.front.store(f, Ordering::Relaxed);
                    return None;
                }
                unsafe {
                    let buffer = self.buffer.get();
                    let task = buffer.read(f);
                    if buffer.cap > MIN_CAP && len <= buffer.cap as isize / 4 {
                        self.resize(buffer.cap / 2);
                    }
                    Some(task)
                }
            }
            Flavor::Lifo => {
                let b = b.wrapping_sub(1);
                self.inner.back.store(b, Ordering::Relaxed);
                atomic::fence(Ordering::SeqCst);
                let f = self.inner.front.load(Ordering::Relaxed);
                let len = b.wrapping_sub(f);
                if len < 0 {
                    self.inner.back.store(b.wrapping_add(1), Ordering::Relaxed);
                    None
                } else {
                    let buffer = self.buffer.get();
                    let mut task = unsafe { Some(buffer.read(b)) };
                    if len == 0 {
                        if self
                            .inner
                            .front
                            .compare_exchange(
                                f,
                                f.wrapping_add(1),
                                Ordering::SeqCst,
                                Ordering::Relaxed,
                            )
                            .is_err()
                        {
                            mem::forget(task.take());
                        }
                        self.inner.back.store(b.wrapping_add(1), Ordering::Relaxed);
                    } else {
                        if buffer.cap > MIN_CAP && len < buffer.cap as isize / 4 {
                            unsafe {
                                self.resize(buffer.cap / 2);
                            }
                        }
                    }
                    task
                }
            }
        }
    }
}
impl<T> fmt::Debug for Worker<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Worker { .. }")
    }
}
/// A stealer handle of a worker queue.
///
/// Stealers can be shared among threads.
///
/// Task schedulers typically have a single worker queue per worker thread.
///
/// # Examples
///
/// ```
/// use crossbeam_deque::{Steal, Worker};
///
/// let w = Worker::new_lifo();
/// w.push(1);
/// w.push(2);
///
/// let s = w.stealer();
/// assert_eq!(s.steal(), Steal::Success(1));
/// assert_eq!(s.steal(), Steal::Success(2));
/// assert_eq!(s.steal(), Steal::Empty);
/// ```
pub struct Stealer<T> {
    /// A reference to the inner representation of the queue.
    inner: Arc<CachePadded<Inner<T>>>,
    /// The flavor of the queue.
    flavor: Flavor,
}
unsafe impl<T: Send> Send for Stealer<T> {}
unsafe impl<T: Send> Sync for Stealer<T> {}
impl<T> Stealer<T> {
    /// Returns `true` if the queue is empty.
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w = Worker::new_lifo();
    /// let s = w.stealer();
    ///
    /// assert!(s.is_empty());
    /// w.push(1);
    /// assert!(!s.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let f = self.inner.front.load(Ordering::Acquire);
        atomic::fence(Ordering::SeqCst);
        let b = self.inner.back.load(Ordering::Acquire);
        b.wrapping_sub(f) <= 0
    }
    /// Steals a task from the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::{Steal, Worker};
    ///
    /// let w = Worker::new_lifo();
    /// w.push(1);
    /// w.push(2);
    ///
    /// let s = w.stealer();
    /// assert_eq!(s.steal(), Steal::Success(1));
    /// assert_eq!(s.steal(), Steal::Success(2));
    /// ```
    pub fn steal(&self) -> Steal<T> {
        let f = self.inner.front.load(Ordering::Acquire);
        if epoch::is_pinned() {
            atomic::fence(Ordering::SeqCst);
        }
        let guard = &epoch::pin();
        let b = self.inner.back.load(Ordering::Acquire);
        if b.wrapping_sub(f) <= 0 {
            return Steal::Empty;
        }
        let buffer = self.inner.buffer.load(Ordering::Acquire, guard);
        let task = unsafe { buffer.deref().read(f) };
        if self
            .inner
            .front
            .compare_exchange(f, f.wrapping_add(1), Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            mem::forget(task);
            return Steal::Retry;
        }
        Steal::Success(task)
    }
    /// Steals a batch of tasks and pushes them into another worker.
    ///
    /// How many tasks exactly will be stolen is not specified. That said, this method will try to
    /// steal around half of the tasks in the queue, but also not more than some constant limit.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Worker;
    ///
    /// let w1 = Worker::new_fifo();
    /// w1.push(1);
    /// w1.push(2);
    /// w1.push(3);
    /// w1.push(4);
    ///
    /// let s = w1.stealer();
    /// let w2 = Worker::new_fifo();
    ///
    /// let _ = s.steal_batch(&w2);
    /// assert_eq!(w2.pop(), Some(1));
    /// assert_eq!(w2.pop(), Some(2));
    /// ```
    pub fn steal_batch(&self, dest: &Worker<T>) -> Steal<()> {
        if Arc::ptr_eq(&self.inner, &dest.inner) {
            if dest.is_empty() {
                return Steal::Empty;
            } else {
                return Steal::Success(());
            }
        }
        let mut f = self.inner.front.load(Ordering::Acquire);
        if epoch::is_pinned() {
            atomic::fence(Ordering::SeqCst);
        }
        let guard = &epoch::pin();
        let b = self.inner.back.load(Ordering::Acquire);
        let len = b.wrapping_sub(f);
        if len <= 0 {
            return Steal::Empty;
        }
        let batch_size = cmp::min((len as usize + 1) / 2, MAX_BATCH);
        dest.reserve(batch_size);
        let mut batch_size = batch_size as isize;
        let dest_buffer = dest.buffer.get();
        let mut dest_b = dest.inner.back.load(Ordering::Relaxed);
        let buffer = self.inner.buffer.load(Ordering::Acquire, guard);
        match self.flavor {
            Flavor::Fifo => {
                match dest.flavor {
                    Flavor::Fifo => {
                        for i in 0..batch_size {
                            unsafe {
                                let task = buffer.deref().read(f.wrapping_add(i));
                                dest_buffer.write(dest_b.wrapping_add(i), task);
                            }
                        }
                    }
                    Flavor::Lifo => {
                        for i in 0..batch_size {
                            unsafe {
                                let task = buffer.deref().read(f.wrapping_add(i));
                                dest_buffer
                                    .write(dest_b.wrapping_add(batch_size - 1 - i), task);
                            }
                        }
                    }
                }
                if self
                    .inner
                    .front
                    .compare_exchange(
                        f,
                        f.wrapping_add(batch_size),
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                    .is_err()
                {
                    return Steal::Retry;
                }
                dest_b = dest_b.wrapping_add(batch_size);
            }
            Flavor::Lifo => {
                for i in 0..batch_size {
                    if i > 0 {
                        atomic::fence(Ordering::SeqCst);
                        let b = self.inner.back.load(Ordering::Acquire);
                        if b.wrapping_sub(f) <= 0 {
                            batch_size = i;
                            break;
                        }
                    }
                    let task = unsafe { buffer.deref().read(f) };
                    if self
                        .inner
                        .front
                        .compare_exchange(
                            f,
                            f.wrapping_add(1),
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        )
                        .is_err()
                    {
                        mem::forget(task);
                        batch_size = i;
                        break;
                    }
                    unsafe {
                        dest_buffer.write(dest_b, task);
                    }
                    f = f.wrapping_add(1);
                    dest_b = dest_b.wrapping_add(1);
                }
                if batch_size == 0 {
                    return Steal::Retry;
                }
                if dest.flavor == Flavor::Fifo {
                    for i in 0..batch_size / 2 {
                        unsafe {
                            let i1 = dest_b.wrapping_sub(batch_size - i);
                            let i2 = dest_b.wrapping_sub(i + 1);
                            let t1 = dest_buffer.read(i1);
                            let t2 = dest_buffer.read(i2);
                            dest_buffer.write(i1, t2);
                            dest_buffer.write(i2, t1);
                        }
                    }
                }
            }
        }
        atomic::fence(Ordering::Release);
        dest.inner.back.store(dest_b, Ordering::Release);
        Steal::Success(())
    }
    /// Steals a batch of tasks, pushes them into another worker, and pops a task from that worker.
    ///
    /// How many tasks exactly will be stolen is not specified. That said, this method will try to
    /// steal around half of the tasks in the queue, but also not more than some constant limit.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::{Steal, Worker};
    ///
    /// let w1 = Worker::new_fifo();
    /// w1.push(1);
    /// w1.push(2);
    /// w1.push(3);
    /// w1.push(4);
    ///
    /// let s = w1.stealer();
    /// let w2 = Worker::new_fifo();
    ///
    /// assert_eq!(s.steal_batch_and_pop(&w2), Steal::Success(1));
    /// assert_eq!(w2.pop(), Some(2));
    /// ```
    pub fn steal_batch_and_pop(&self, dest: &Worker<T>) -> Steal<T> {
        if Arc::ptr_eq(&self.inner, &dest.inner) {
            match dest.pop() {
                None => return Steal::Empty,
                Some(task) => return Steal::Success(task),
            }
        }
        let mut f = self.inner.front.load(Ordering::Acquire);
        if epoch::is_pinned() {
            atomic::fence(Ordering::SeqCst);
        }
        let guard = &epoch::pin();
        let b = self.inner.back.load(Ordering::Acquire);
        let len = b.wrapping_sub(f);
        if len <= 0 {
            return Steal::Empty;
        }
        let batch_size = cmp::min((len as usize - 1) / 2, MAX_BATCH - 1);
        dest.reserve(batch_size);
        let mut batch_size = batch_size as isize;
        let dest_buffer = dest.buffer.get();
        let mut dest_b = dest.inner.back.load(Ordering::Relaxed);
        let buffer = self.inner.buffer.load(Ordering::Acquire, guard);
        let mut task = unsafe { buffer.deref().read(f) };
        match self.flavor {
            Flavor::Fifo => {
                match dest.flavor {
                    Flavor::Fifo => {
                        for i in 0..batch_size {
                            unsafe {
                                let task = buffer.deref().read(f.wrapping_add(i + 1));
                                dest_buffer.write(dest_b.wrapping_add(i), task);
                            }
                        }
                    }
                    Flavor::Lifo => {
                        for i in 0..batch_size {
                            unsafe {
                                let task = buffer.deref().read(f.wrapping_add(i + 1));
                                dest_buffer
                                    .write(dest_b.wrapping_add(batch_size - 1 - i), task);
                            }
                        }
                    }
                }
                if self
                    .inner
                    .front
                    .compare_exchange(
                        f,
                        f.wrapping_add(batch_size + 1),
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                    .is_err()
                {
                    mem::forget(task);
                    return Steal::Retry;
                }
                dest_b = dest_b.wrapping_add(batch_size);
            }
            Flavor::Lifo => {
                if self
                    .inner
                    .front
                    .compare_exchange(
                        f,
                        f.wrapping_add(1),
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                    .is_err()
                {
                    mem::forget(task);
                    return Steal::Retry;
                }
                f = f.wrapping_add(1);
                for i in 0..batch_size {
                    atomic::fence(Ordering::SeqCst);
                    let b = self.inner.back.load(Ordering::Acquire);
                    if b.wrapping_sub(f) <= 0 {
                        batch_size = i;
                        break;
                    }
                    let tmp = unsafe { buffer.deref().read(f) };
                    if self
                        .inner
                        .front
                        .compare_exchange(
                            f,
                            f.wrapping_add(1),
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        )
                        .is_err()
                    {
                        mem::forget(tmp);
                        batch_size = i;
                        break;
                    }
                    unsafe {
                        dest_buffer.write(dest_b, mem::replace(&mut task, tmp));
                    }
                    f = f.wrapping_add(1);
                    dest_b = dest_b.wrapping_add(1);
                }
                if dest.flavor == Flavor::Fifo {
                    for i in 0..batch_size / 2 {
                        unsafe {
                            let i1 = dest_b.wrapping_sub(batch_size - i);
                            let i2 = dest_b.wrapping_sub(i + 1);
                            let t1 = dest_buffer.read(i1);
                            let t2 = dest_buffer.read(i2);
                            dest_buffer.write(i1, t2);
                            dest_buffer.write(i2, t1);
                        }
                    }
                }
            }
        }
        atomic::fence(Ordering::Release);
        dest.inner.back.store(dest_b, Ordering::Release);
        Steal::Success(task)
    }
}
impl<T> Clone for Stealer<T> {
    fn clone(&self) -> Stealer<T> {
        Stealer {
            inner: self.inner.clone(),
            flavor: self.flavor,
        }
    }
}
impl<T> fmt::Debug for Stealer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Stealer { .. }")
    }
}
const WRITE: usize = 1;
const READ: usize = 2;
const DESTROY: usize = 4;
const LAP: usize = 64;
const BLOCK_CAP: usize = LAP - 1;
const SHIFT: usize = 1;
const HAS_NEXT: usize = 1;
/// A slot in a block.
struct Slot<T> {
    /// The task.
    task: UnsafeCell<MaybeUninit<T>>,
    /// The state of the slot.
    state: AtomicUsize,
}
impl<T> Slot<T> {
    /// Waits until a task is written into the slot.
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
    unsafe fn destroy(this: *mut Block<T>, count: usize) {
        for i in (0..count).rev() {
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
/// An injector queue.
///
/// This is a FIFO queue that can be shared among multiple threads. Task schedulers typically have
/// a single injector queue, which is the entry point for new tasks.
///
/// # Examples
///
/// ```
/// use crossbeam_deque::{Injector, Steal};
///
/// let q = Injector::new();
/// q.push(1);
/// q.push(2);
///
/// assert_eq!(q.steal(), Steal::Success(1));
/// assert_eq!(q.steal(), Steal::Success(2));
/// assert_eq!(q.steal(), Steal::Empty);
/// ```
pub struct Injector<T> {
    /// The head of the queue.
    head: CachePadded<Position<T>>,
    /// The tail of the queue.
    tail: CachePadded<Position<T>>,
    /// Indicates that dropping a `Injector<T>` may drop values of type `T`.
    _marker: PhantomData<T>,
}
unsafe impl<T: Send> Send for Injector<T> {}
unsafe impl<T: Send> Sync for Injector<T> {}
impl<T> Default for Injector<T> {
    fn default() -> Self {
        let block = Box::into_raw(Box::new(Block::<T>::new()));
        Self {
            head: CachePadded::new(Position {
                block: AtomicPtr::new(block),
                index: AtomicUsize::new(0),
            }),
            tail: CachePadded::new(Position {
                block: AtomicPtr::new(block),
                index: AtomicUsize::new(0),
            }),
            _marker: PhantomData,
        }
    }
}
impl<T> Injector<T> {
    /// Creates a new injector queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Injector;
    ///
    /// let q = Injector::<i32>::new();
    /// ```
    pub fn new() -> Injector<T> {
        Self::default()
    }
    /// Pushes a task into the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Injector;
    ///
    /// let w = Injector::new();
    /// w.push(1);
    /// w.push(2);
    /// ```
    pub fn push(&self, task: T) {
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
                        slot.task.get().write(MaybeUninit::new(task));
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
    /// Steals a task from the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::{Injector, Steal};
    ///
    /// let q = Injector::new();
    /// q.push(1);
    /// q.push(2);
    ///
    /// assert_eq!(q.steal(), Steal::Success(1));
    /// assert_eq!(q.steal(), Steal::Success(2));
    /// assert_eq!(q.steal(), Steal::Empty);
    /// ```
    pub fn steal(&self) -> Steal<T> {
        let mut head;
        let mut block;
        let mut offset;
        let backoff = Backoff::new();
        loop {
            head = self.head.index.load(Ordering::Acquire);
            block = self.head.block.load(Ordering::Acquire);
            offset = (head >> SHIFT) % LAP;
            if offset == BLOCK_CAP {
                backoff.snooze();
            } else {
                break;
            }
        }
        let mut new_head = head + (1 << SHIFT);
        if new_head & HAS_NEXT == 0 {
            atomic::fence(Ordering::SeqCst);
            let tail = self.tail.index.load(Ordering::Relaxed);
            if head >> SHIFT == tail >> SHIFT {
                return Steal::Empty;
            }
            if (head >> SHIFT) / LAP != (tail >> SHIFT) / LAP {
                new_head |= HAS_NEXT;
            }
        }
        if self
            .head
            .index
            .compare_exchange_weak(head, new_head, Ordering::SeqCst, Ordering::Acquire)
            .is_err()
        {
            return Steal::Retry;
        }
        unsafe {
            if offset + 1 == BLOCK_CAP {
                let next = (*block).wait_next();
                let mut next_index = (new_head & !HAS_NEXT).wrapping_add(1 << SHIFT);
                if !(*next).next.load(Ordering::Relaxed).is_null() {
                    next_index |= HAS_NEXT;
                }
                self.head.block.store(next, Ordering::Release);
                self.head.index.store(next_index, Ordering::Release);
            }
            let slot = (*block).slots.get_unchecked(offset);
            slot.wait_write();
            let task = slot.task.get().read().assume_init();
            if (offset + 1 == BLOCK_CAP)
                || (slot.state.fetch_or(READ, Ordering::AcqRel) & DESTROY != 0)
            {
                Block::destroy(block, offset);
            }
            Steal::Success(task)
        }
    }
    /// Steals a batch of tasks and pushes them into a worker.
    ///
    /// How many tasks exactly will be stolen is not specified. That said, this method will try to
    /// steal around half of the tasks in the queue, but also not more than some constant limit.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::{Injector, Worker};
    ///
    /// let q = Injector::new();
    /// q.push(1);
    /// q.push(2);
    /// q.push(3);
    /// q.push(4);
    ///
    /// let w = Worker::new_fifo();
    /// let _ = q.steal_batch(&w);
    /// assert_eq!(w.pop(), Some(1));
    /// assert_eq!(w.pop(), Some(2));
    /// ```
    pub fn steal_batch(&self, dest: &Worker<T>) -> Steal<()> {
        let mut head;
        let mut block;
        let mut offset;
        let backoff = Backoff::new();
        loop {
            head = self.head.index.load(Ordering::Acquire);
            block = self.head.block.load(Ordering::Acquire);
            offset = (head >> SHIFT) % LAP;
            if offset == BLOCK_CAP {
                backoff.snooze();
            } else {
                break;
            }
        }
        let mut new_head = head;
        let advance;
        if new_head & HAS_NEXT == 0 {
            atomic::fence(Ordering::SeqCst);
            let tail = self.tail.index.load(Ordering::Relaxed);
            if head >> SHIFT == tail >> SHIFT {
                return Steal::Empty;
            }
            if (head >> SHIFT) / LAP != (tail >> SHIFT) / LAP {
                new_head |= HAS_NEXT;
                advance = (BLOCK_CAP - offset).min(MAX_BATCH);
            } else {
                let len = (tail - head) >> SHIFT;
                advance = ((len + 1) / 2).min(MAX_BATCH);
            }
        } else {
            advance = (BLOCK_CAP - offset).min(MAX_BATCH);
        }
        new_head += advance << SHIFT;
        let new_offset = offset + advance;
        if self
            .head
            .index
            .compare_exchange_weak(head, new_head, Ordering::SeqCst, Ordering::Acquire)
            .is_err()
        {
            return Steal::Retry;
        }
        let batch_size = new_offset - offset;
        dest.reserve(batch_size);
        let dest_buffer = dest.buffer.get();
        let dest_b = dest.inner.back.load(Ordering::Relaxed);
        unsafe {
            if new_offset == BLOCK_CAP {
                let next = (*block).wait_next();
                let mut next_index = (new_head & !HAS_NEXT).wrapping_add(1 << SHIFT);
                if !(*next).next.load(Ordering::Relaxed).is_null() {
                    next_index |= HAS_NEXT;
                }
                self.head.block.store(next, Ordering::Release);
                self.head.index.store(next_index, Ordering::Release);
            }
            match dest.flavor {
                Flavor::Fifo => {
                    for i in 0..batch_size {
                        let slot = (*block).slots.get_unchecked(offset + i);
                        slot.wait_write();
                        let task = slot.task.get().read().assume_init();
                        dest_buffer.write(dest_b.wrapping_add(i as isize), task);
                    }
                }
                Flavor::Lifo => {
                    for i in 0..batch_size {
                        let slot = (*block).slots.get_unchecked(offset + i);
                        slot.wait_write();
                        let task = slot.task.get().read().assume_init();
                        dest_buffer
                            .write(
                                dest_b.wrapping_add((batch_size - 1 - i) as isize),
                                task,
                            );
                    }
                }
            }
            atomic::fence(Ordering::Release);
            dest.inner
                .back
                .store(dest_b.wrapping_add(batch_size as isize), Ordering::Release);
            if new_offset == BLOCK_CAP {
                Block::destroy(block, offset);
            } else {
                for i in offset..new_offset {
                    let slot = (*block).slots.get_unchecked(i);
                    if slot.state.fetch_or(READ, Ordering::AcqRel) & DESTROY != 0 {
                        Block::destroy(block, offset);
                        break;
                    }
                }
            }
            Steal::Success(())
        }
    }
    /// Steals a batch of tasks, pushes them into a worker, and pops a task from that worker.
    ///
    /// How many tasks exactly will be stolen is not specified. That said, this method will try to
    /// steal around half of the tasks in the queue, but also not more than some constant limit.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::{Injector, Steal, Worker};
    ///
    /// let q = Injector::new();
    /// q.push(1);
    /// q.push(2);
    /// q.push(3);
    /// q.push(4);
    ///
    /// let w = Worker::new_fifo();
    /// assert_eq!(q.steal_batch_and_pop(&w), Steal::Success(1));
    /// assert_eq!(w.pop(), Some(2));
    /// ```
    pub fn steal_batch_and_pop(&self, dest: &Worker<T>) -> Steal<T> {
        let mut head;
        let mut block;
        let mut offset;
        let backoff = Backoff::new();
        loop {
            head = self.head.index.load(Ordering::Acquire);
            block = self.head.block.load(Ordering::Acquire);
            offset = (head >> SHIFT) % LAP;
            if offset == BLOCK_CAP {
                backoff.snooze();
            } else {
                break;
            }
        }
        let mut new_head = head;
        let advance;
        if new_head & HAS_NEXT == 0 {
            atomic::fence(Ordering::SeqCst);
            let tail = self.tail.index.load(Ordering::Relaxed);
            if head >> SHIFT == tail >> SHIFT {
                return Steal::Empty;
            }
            if (head >> SHIFT) / LAP != (tail >> SHIFT) / LAP {
                new_head |= HAS_NEXT;
                advance = (BLOCK_CAP - offset).min(MAX_BATCH + 1);
            } else {
                let len = (tail - head) >> SHIFT;
                advance = ((len + 1) / 2).min(MAX_BATCH + 1);
            }
        } else {
            advance = (BLOCK_CAP - offset).min(MAX_BATCH + 1);
        }
        new_head += advance << SHIFT;
        let new_offset = offset + advance;
        if self
            .head
            .index
            .compare_exchange_weak(head, new_head, Ordering::SeqCst, Ordering::Acquire)
            .is_err()
        {
            return Steal::Retry;
        }
        let batch_size = new_offset - offset - 1;
        dest.reserve(batch_size);
        let dest_buffer = dest.buffer.get();
        let dest_b = dest.inner.back.load(Ordering::Relaxed);
        unsafe {
            if new_offset == BLOCK_CAP {
                let next = (*block).wait_next();
                let mut next_index = (new_head & !HAS_NEXT).wrapping_add(1 << SHIFT);
                if !(*next).next.load(Ordering::Relaxed).is_null() {
                    next_index |= HAS_NEXT;
                }
                self.head.block.store(next, Ordering::Release);
                self.head.index.store(next_index, Ordering::Release);
            }
            let slot = (*block).slots.get_unchecked(offset);
            slot.wait_write();
            let task = slot.task.get().read().assume_init();
            match dest.flavor {
                Flavor::Fifo => {
                    for i in 0..batch_size {
                        let slot = (*block).slots.get_unchecked(offset + i + 1);
                        slot.wait_write();
                        let task = slot.task.get().read().assume_init();
                        dest_buffer.write(dest_b.wrapping_add(i as isize), task);
                    }
                }
                Flavor::Lifo => {
                    for i in 0..batch_size {
                        let slot = (*block).slots.get_unchecked(offset + i + 1);
                        slot.wait_write();
                        let task = slot.task.get().read().assume_init();
                        dest_buffer
                            .write(
                                dest_b.wrapping_add((batch_size - 1 - i) as isize),
                                task,
                            );
                    }
                }
            }
            atomic::fence(Ordering::Release);
            dest.inner
                .back
                .store(dest_b.wrapping_add(batch_size as isize), Ordering::Release);
            if new_offset == BLOCK_CAP {
                Block::destroy(block, offset);
            } else {
                for i in offset..new_offset {
                    let slot = (*block).slots.get_unchecked(i);
                    if slot.state.fetch_or(READ, Ordering::AcqRel) & DESTROY != 0 {
                        Block::destroy(block, offset);
                        break;
                    }
                }
            }
            Steal::Success(task)
        }
    }
    /// Returns `true` if the queue is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Injector;
    ///
    /// let q = Injector::new();
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
    /// Returns the number of tasks in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Injector;
    ///
    /// let q = Injector::new();
    ///
    /// assert_eq!(q.len(), 0);
    /// q.push(1);
    /// assert_eq!(q.len(), 1);
    /// q.push(1);
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
impl<T> Drop for Injector<T> {
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
                    let p = &mut *slot.task.get();
                    p.as_mut_ptr().drop_in_place();
                } else {
                    let next = (*block).next.load(Ordering::Relaxed);
                    drop(Box::from_raw(block));
                    block = next;
                }
                head = head.wrapping_add(1 << SHIFT);
            }
            drop(Box::from_raw(block));
        }
    }
}
impl<T> fmt::Debug for Injector<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Worker { .. }")
    }
}
/// Possible outcomes of a steal operation.
///
/// # Examples
///
/// There are lots of ways to chain results of steal operations together:
///
/// ```
/// use crossbeam_deque::Steal::{self, Empty, Retry, Success};
///
/// let collect = |v: Vec<Steal<i32>>| v.into_iter().collect::<Steal<i32>>();
///
/// assert_eq!(collect(vec![Empty, Empty, Empty]), Empty);
/// assert_eq!(collect(vec![Empty, Retry, Empty]), Retry);
/// assert_eq!(collect(vec![Retry, Success(1), Empty]), Success(1));
///
/// assert_eq!(collect(vec![Empty, Empty]).or_else(|| Retry), Retry);
/// assert_eq!(collect(vec![Retry, Empty]).or_else(|| Success(1)), Success(1));
/// ```
#[must_use]
#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Steal<T> {
    /// The queue was empty at the time of stealing.
    Empty,
    /// At least one task was successfully stolen.
    Success(T),
    /// The steal operation needs to be retried.
    Retry,
}
impl<T> Steal<T> {
    /// Returns `true` if the queue was empty at the time of stealing.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Steal::{Empty, Retry, Success};
    ///
    /// assert!(!Success(7).is_empty());
    /// assert!(!Retry::<i32>.is_empty());
    ///
    /// assert!(Empty::<i32>.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        match self {
            Steal::Empty => true,
            _ => false,
        }
    }
    /// Returns `true` if at least one task was stolen.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Steal::{Empty, Retry, Success};
    ///
    /// assert!(!Empty::<i32>.is_success());
    /// assert!(!Retry::<i32>.is_success());
    ///
    /// assert!(Success(7).is_success());
    /// ```
    pub fn is_success(&self) -> bool {
        match self {
            Steal::Success(_) => true,
            _ => false,
        }
    }
    /// Returns `true` if the steal operation needs to be retried.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Steal::{Empty, Retry, Success};
    ///
    /// assert!(!Empty::<i32>.is_retry());
    /// assert!(!Success(7).is_retry());
    ///
    /// assert!(Retry::<i32>.is_retry());
    /// ```
    pub fn is_retry(&self) -> bool {
        match self {
            Steal::Retry => true,
            _ => false,
        }
    }
    /// Returns the result of the operation, if successful.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Steal::{Empty, Retry, Success};
    ///
    /// assert_eq!(Empty::<i32>.success(), None);
    /// assert_eq!(Retry::<i32>.success(), None);
    ///
    /// assert_eq!(Success(7).success(), Some(7));
    /// ```
    pub fn success(self) -> Option<T> {
        match self {
            Steal::Success(res) => Some(res),
            _ => None,
        }
    }
    /// If no task was stolen, attempts another steal operation.
    ///
    /// Returns this steal result if it is `Success`. Otherwise, closure `f` is invoked and then:
    ///
    /// * If the second steal resulted in `Success`, it is returned.
    /// * If both steals were unsuccessful but any resulted in `Retry`, then `Retry` is returned.
    /// * If both resulted in `None`, then `None` is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_deque::Steal::{Empty, Retry, Success};
    ///
    /// assert_eq!(Success(1).or_else(|| Success(2)), Success(1));
    /// assert_eq!(Retry.or_else(|| Success(2)), Success(2));
    ///
    /// assert_eq!(Retry.or_else(|| Empty), Retry::<i32>);
    /// assert_eq!(Empty.or_else(|| Retry), Retry::<i32>);
    ///
    /// assert_eq!(Empty.or_else(|| Empty), Empty::<i32>);
    /// ```
    pub fn or_else<F>(self, f: F) -> Steal<T>
    where
        F: FnOnce() -> Steal<T>,
    {
        match self {
            Steal::Empty => f(),
            Steal::Success(_) => self,
            Steal::Retry => {
                if let Steal::Success(res) = f() {
                    Steal::Success(res)
                } else {
                    Steal::Retry
                }
            }
        }
    }
}
impl<T> fmt::Debug for Steal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Steal::Empty => f.pad("Empty"),
            Steal::Success(_) => f.pad("Success(..)"),
            Steal::Retry => f.pad("Retry"),
        }
    }
}
impl<T> FromIterator<Steal<T>> for Steal<T> {
    /// Consumes items until a `Success` is found and returns it.
    ///
    /// If no `Success` was found, but there was at least one `Retry`, then returns `Retry`.
    /// Otherwise, `Empty` is returned.
    fn from_iter<I>(iter: I) -> Steal<T>
    where
        I: IntoIterator<Item = Steal<T>>,
    {
        let mut retry = false;
        for s in iter {
            match &s {
                Steal::Empty => {}
                Steal::Success(_) => return s,
                Steal::Retry => retry = true,
            }
        }
        if retry { Steal::Retry } else { Steal::Empty }
    }
}
#[cfg(test)]
mod tests_rug_304 {
    use super::*;
    use crate::deque::Buffer;
    use std::mem;
    #[test]
    fn test_alloc() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        let buffer = Buffer::<i32>::alloc(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_306 {
    use super::*;
    use crate::deque::Buffer;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut buffer = Buffer::<i32> {
            ptr: std::ptr::null_mut(),
            cap: rug_fuzz_0,
        };
        let index = rug_fuzz_1;
        let result = unsafe { Buffer::<i32>::at(&buffer, index as isize) };
        debug_assert_eq!(result, std::ptr::null_mut());
             }
});    }
}
#[cfg(test)]
mod tests_rug_309 {
    use super::*;
    use crate::deque::Buffer;
    #[test]
    fn test_clone() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let ptr: *mut usize = std::ptr::null_mut();
        let cap: usize = rug_fuzz_0;
        let buffer = Buffer::<usize> { ptr, cap };
        buffer.clone();
             }
});    }
}
#[cfg(test)]
mod tests_rug_311 {
    use super::*;
    use crate::Worker;
    use crate::deque;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_311_rrrruuuugggg_test_rug = 0;
        deque::Worker::<i32>::new_fifo();
        let _rug_ed_tests_rug_311_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_313 {
    use super::*;
    use crate::{Worker, Stealer};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_313_rrrruuuugggg_test_rug = 0;
        let w = Worker::<i32>::new_fifo();
        let s = w.stealer();
        Worker::<i32>::stealer(&w);
        let _rug_ed_tests_rug_313_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_316 {
    use super::*;
    use crate::Worker;
    #[test]
    fn test_is_empty() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let w = Worker::<i32>::new_lifo();
        debug_assert!(< Worker < i32 > > ::is_empty(& w));
        w.push(rug_fuzz_0);
        debug_assert!(! < Worker < i32 > > ::is_empty(& w));
             }
});    }
}
#[cfg(test)]
mod tests_rug_317 {
    use super::*;
    use crate::Worker;
    use crate::Injector;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_len() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let w: Worker<i32> = Worker::new_lifo();
        debug_assert_eq!(w.len(), 0);
        w.push(rug_fuzz_0);
        debug_assert_eq!(w.len(), 1);
        w.push(rug_fuzz_1);
        debug_assert_eq!(w.len(), 2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use crate::Worker;
    #[test]
    fn test_push() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let w = Worker::new_lifo();
        let task = rug_fuzz_0;
        w.push(task);
             }
});    }
}
#[cfg(test)]
mod tests_rug_319 {
    use super::*;
    use crate::Worker;
    #[test]
    fn test_pop() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let w = Worker::<i32>::new_fifo();
        w.push(rug_fuzz_0);
        w.push(rug_fuzz_1);
        debug_assert_eq!(w.pop(), Some(1));
        debug_assert_eq!(w.pop(), Some(2));
        debug_assert_eq!(w.pop(), None);
             }
});    }
}
#[cfg(test)]
mod tests_rug_320 {
    use super::*;
    use crate::{Worker, deque};
    #[test]
    fn test_is_empty() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let w = Worker::new_fifo();
        let s = w.stealer();
        debug_assert!(deque::Stealer:: < i32 > ::is_empty(& s));
        w.push(rug_fuzz_0);
        debug_assert!(! deque::Stealer:: < i32 > ::is_empty(& s));
             }
});    }
}
#[cfg(test)]
mod tests_rug_321 {
    use super::*;
    use crate::{Steal, Worker};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let w = Worker::<i32>::new_lifo();
        w.push(rug_fuzz_0);
        w.push(rug_fuzz_1);
        let s = w.stealer();
        debug_assert_eq!(s.steal(), Steal::Success(1));
        debug_assert_eq!(s.steal(), Steal::Success(2));
             }
});    }
}
#[cfg(test)]
mod tests_rug_322 {
    use super::*;
    use crate::{Worker, Stealer};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(i32, i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let w1 = Worker::<i32>::new_fifo();
        w1.push(rug_fuzz_0);
        w1.push(rug_fuzz_1);
        w1.push(rug_fuzz_2);
        w1.push(rug_fuzz_3);
        let s = w1.stealer();
        let w2 = Worker::<i32>::new_fifo();
        Stealer::<i32>::steal_batch(&s, &w2);
        debug_assert_eq!(w2.pop(), Some(1));
        debug_assert_eq!(w2.pop(), Some(2));
             }
});    }
}
#[cfg(test)]
mod tests_rug_323 {
    use super::*;
    use crate::{Steal, Worker, deque};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let w1 = Worker::<i32>::new_fifo();
        w1.push(rug_fuzz_0);
        w1.push(rug_fuzz_1);
        let s = w1.stealer();
        let w2 = Worker::<i32>::new_fifo();
        deque::Stealer::<i32>::steal_batch_and_pop(&s, &w2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_326 {
    use super::*;
    use crate::deque::Block;
    use std::mem::MaybeUninit;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_326_rrrruuuugggg_test_rug = 0;
        let block: Block<usize> = unsafe { MaybeUninit::zeroed().assume_init() };
        let _rug_ed_tests_rug_326_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_327 {
    use super::*;
    use crate::deque;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_327_rrrruuuugggg_test_rug = 0;
        let mut p0 = deque::Block::<u32>::new();
        p0.wait_next();
        let _rug_ed_tests_rug_327_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_330 {
    use super::*;
    use crate::Injector;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_330_rrrruuuugggg_test_new = 0;
        let q: Injector<i32> = Injector::new();
        let _rug_ed_tests_rug_330_rrrruuuugggg_test_new = 0;
    }
}
use crate::deque;
#[cfg(test)]
mod tests_rug_331_prepare {
    use super::*;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_331_prepare_rrrruuuugggg_sample = 0;
        let mut v69: deque::Injector<u32> = deque::Injector::new();
        let _rug_ed_tests_rug_331_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_331 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: deque::Injector<u32> = deque::Injector::new();
        let p1: u32 = rug_fuzz_0;
        crate::deque::Injector::<u32>::push(&p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_332 {
    use super::*;
    use crate::{Injector, Steal};
    use crate::deque;
    #[test]
    fn test_steal() {
        let _rug_st_tests_rug_332_rrrruuuugggg_test_steal = 0;
        let mut p0: deque::Injector<u32> = deque::Injector::new();
        debug_assert_eq!(p0.steal(), Steal::Empty);
        let _rug_ed_tests_rug_332_rrrruuuugggg_test_steal = 0;
    }
}
#[cfg(test)]
mod tests_rug_333 {
    use super::*;
    use crate::{Injector, Worker};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u32, u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Injector<u32> = Injector::new();
        p0.push(rug_fuzz_0);
        p0.push(rug_fuzz_1);
        p0.push(rug_fuzz_2);
        p0.push(rug_fuzz_3);
        let p1: Worker<u32> = Worker::new_fifo();
        let _ = p0.steal_batch(&p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_334 {
    use super::*;
    use crate::deque::{Injector, Worker, Steal};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u32, u32, u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Injector<u32> = Injector::new();
        p0.push(rug_fuzz_0);
        p0.push(rug_fuzz_1);
        p0.push(rug_fuzz_2);
        p0.push(rug_fuzz_3);
        let p1: Worker<u32> = Worker::new_fifo();
        debug_assert_eq!(p0.steal_batch_and_pop(& p1), Steal::Success(1));
        debug_assert_eq!(p1.pop(), Some(2));
             }
});    }
}
#[cfg(test)]
mod tests_rug_335 {
    use super::*;
    use crate::deque;
    #[test]
    fn test_is_empty() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: deque::Injector<u32> = deque::Injector::new();
        debug_assert!(p0.is_empty());
        p0.push(rug_fuzz_0);
        debug_assert!(! p0.is_empty());
             }
});    }
}
#[cfg(test)]
mod tests_rug_336 {
    use super::*;
    use crate::deque;
    #[test]
    fn test_len() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: deque::Injector<u32> = deque::Injector::new();
        debug_assert_eq!(p0.len(), 0);
        p0.push(rug_fuzz_0);
        debug_assert_eq!(p0.len(), 1);
        p0.push(rug_fuzz_1);
        debug_assert_eq!(p0.len(), 2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_338 {
    use super::*;
    use crate::Steal::{Empty, Retry, Success};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_338_rrrruuuugggg_test_rug = 0;
        let p0 = Empty::<i32>;
        debug_assert!(< deque::Steal < i32 > > ::is_empty(& p0));
        let _rug_ed_tests_rug_338_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_339 {
    use super::*;
    use crate::Steal::{Empty, Retry, Success};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = Empty::<i32>;
        debug_assert!(! < deque::Steal < i32 > > ::is_success(& p0));
        let p1 = Retry::<i32>;
        debug_assert!(! < deque::Steal < i32 > > ::is_success(& p1));
        let p2 = Success(rug_fuzz_0);
        debug_assert!(< deque::Steal < i32 > > ::is_success(& p2));
             }
});    }
}
#[cfg(test)]
mod tests_rug_340 {
    use super::*;
    use crate::Steal::{Empty, Retry, Success};
    #[test]
    fn test_is_retry() {
        let _rug_st_tests_rug_340_rrrruuuugggg_test_is_retry = 0;
        let p0 = Retry::<i32>;
        debug_assert!(< deque::Steal < i32 > > ::is_retry(& p0));
        let _rug_ed_tests_rug_340_rrrruuuugggg_test_is_retry = 0;
    }
}
#[cfg(test)]
mod tests_rug_341 {
    use super::*;
    use crate::deque::Steal::{Empty, Retry, Success};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_341_rrrruuuugggg_test_rug = 0;
        let p0 = Retry::<i32>;
        debug_assert_eq!(< deque::Steal < i32 > > ::success(p0), None);
        let _rug_ed_tests_rug_341_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_342 {
    use super::*;
    use crate::Steal::{Empty, Retry, Success};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Empty::<i32>;
        let mut p1 = || Success(rug_fuzz_0);
        p0.or_else(p1);
             }
});    }
}
