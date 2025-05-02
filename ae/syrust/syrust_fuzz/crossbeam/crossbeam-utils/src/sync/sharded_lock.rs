use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::{LockResult, PoisonError, TryLockError, TryLockResult};
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::{self, ThreadId};
use crate::CachePadded;
use lazy_static::lazy_static;
/// The number of shards per sharded lock. Must be a power of two.
const NUM_SHARDS: usize = 8;
/// A shard containing a single reader-writer lock.
struct Shard {
    /// The inner reader-writer lock.
    lock: RwLock<()>,
    /// The write-guard keeping this shard locked.
    ///
    /// Write operations will lock each shard and store the guard here. These guards get dropped at
    /// the same time the big guard is dropped.
    write_guard: UnsafeCell<Option<RwLockWriteGuard<'static, ()>>>,
}
/// A sharded reader-writer lock.
///
/// This lock is equivalent to [`RwLock`], except read operations are faster and write operations
/// are slower.
///
/// A `ShardedLock` is internally made of a list of *shards*, each being a [`RwLock`] occupying a
/// single cache line. Read operations will pick one of the shards depending on the current thread
/// and lock it. Write operations need to lock all shards in succession.
///
/// By splitting the lock into shards, concurrent read operations will in most cases choose
/// different shards and thus update different cache lines, which is good for scalability. However,
/// write operations need to do more work and are therefore slower than usual.
///
/// The priority policy of the lock is dependent on the underlying operating system's
/// implementation, and this type does not guarantee that any particular policy will be used.
///
/// # Poisoning
///
/// A `ShardedLock`, like [`RwLock`], will become poisoned on a panic. Note that it may only be
/// poisoned if a panic occurs while a write operation is in progress. If a panic occurs in any
/// read operation, the lock will not be poisoned.
///
/// # Examples
///
/// ```
/// use crossbeam_utils::sync::ShardedLock;
///
/// let lock = ShardedLock::new(5);
///
/// // Any number of read locks can be held at once.
/// {
///     let r1 = lock.read().unwrap();
///     let r2 = lock.read().unwrap();
///     assert_eq!(*r1, 5);
///     assert_eq!(*r2, 5);
/// } // Read locks are dropped at this point.
///
/// // However, only one write lock may be held.
/// {
///     let mut w = lock.write().unwrap();
///     *w += 1;
///     assert_eq!(*w, 6);
/// } // Write lock is dropped here.
/// ```
///
/// [`RwLock`]: https://doc.rust-lang.org/std/sync/struct.RwLock.html
pub struct ShardedLock<T: ?Sized> {
    /// A list of locks protecting the internal data.
    shards: Box<[CachePadded<Shard>]>,
    /// The internal data.
    value: UnsafeCell<T>,
}
unsafe impl<T: ?Sized + Send> Send for ShardedLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for ShardedLock<T> {}
impl<T: ?Sized> UnwindSafe for ShardedLock<T> {}
impl<T: ?Sized> RefUnwindSafe for ShardedLock<T> {}
impl<T> ShardedLock<T> {
    /// Creates a new sharded reader-writer lock.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::ShardedLock;
    ///
    /// let lock = ShardedLock::new(5);
    /// ```
    pub fn new(value: T) -> ShardedLock<T> {
        ShardedLock {
            shards: (0..NUM_SHARDS)
                .map(|_| {
                    CachePadded::new(Shard {
                        lock: RwLock::new(()),
                        write_guard: UnsafeCell::new(None),
                    })
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            value: UnsafeCell::new(value),
        }
    }
    /// Consumes this lock, returning the underlying data.
    ///
    /// This method will return an error if the lock is poisoned. A lock gets poisoned when a write
    /// operation panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::ShardedLock;
    ///
    /// let lock = ShardedLock::new(String::new());
    /// {
    ///     let mut s = lock.write().unwrap();
    ///     *s = "modified".to_owned();
    /// }
    /// assert_eq!(lock.into_inner().unwrap(), "modified");
    /// ```
    pub fn into_inner(self) -> LockResult<T> {
        let is_poisoned = self.is_poisoned();
        let inner = self.value.into_inner();
        if is_poisoned { Err(PoisonError::new(inner)) } else { Ok(inner) }
    }
}
impl<T: ?Sized> ShardedLock<T> {
    /// Returns `true` if the lock is poisoned.
    ///
    /// If another thread can still access the lock, it may become poisoned at any time. A `false`
    /// result should not be trusted without additional synchronization.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::ShardedLock;
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// let lock = Arc::new(ShardedLock::new(0));
    /// let c_lock = lock.clone();
    ///
    /// let _ = thread::spawn(move || {
    ///     let _lock = c_lock.write().unwrap();
    ///     panic!(); // the lock gets poisoned
    /// }).join();
    /// assert_eq!(lock.is_poisoned(), true);
    /// ```
    pub fn is_poisoned(&self) -> bool {
        self.shards[0].lock.is_poisoned()
    }
    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the lock mutably, no actual locking needs to take place.
    ///
    /// This method will return an error if the lock is poisoned. A lock gets poisoned when a write
    /// operation panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::ShardedLock;
    ///
    /// let mut lock = ShardedLock::new(0);
    /// *lock.get_mut().unwrap() = 10;
    /// assert_eq!(*lock.read().unwrap(), 10);
    /// ```
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        let is_poisoned = self.is_poisoned();
        let inner = unsafe { &mut *self.value.get() };
        if is_poisoned { Err(PoisonError::new(inner)) } else { Ok(inner) }
    }
    /// Attempts to acquire this lock with shared read access.
    ///
    /// If the access could not be granted at this time, an error is returned. Otherwise, a guard
    /// is returned which will release the shared access when it is dropped. This method does not
    /// provide any guarantees with respect to the ordering of whether contentious readers or
    /// writers will acquire the lock first.
    ///
    /// This method will return an error if the lock is poisoned. A lock gets poisoned when a write
    /// operation panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::ShardedLock;
    ///
    /// let lock = ShardedLock::new(1);
    ///
    /// match lock.try_read() {
    ///     Ok(n) => assert_eq!(*n, 1),
    ///     Err(_) => unreachable!(),
    /// };
    /// ```
    pub fn try_read(&self) -> TryLockResult<ShardedLockReadGuard<'_, T>> {
        let current_index = current_index().unwrap_or(0);
        let shard_index = current_index & (self.shards.len() - 1);
        match self.shards[shard_index].lock.try_read() {
            Ok(guard) => {
                Ok(ShardedLockReadGuard {
                    lock: self,
                    _guard: guard,
                    _marker: PhantomData,
                })
            }
            Err(TryLockError::Poisoned(err)) => {
                let guard = ShardedLockReadGuard {
                    lock: self,
                    _guard: err.into_inner(),
                    _marker: PhantomData,
                };
                Err(TryLockError::Poisoned(PoisonError::new(guard)))
            }
            Err(TryLockError::WouldBlock) => Err(TryLockError::WouldBlock),
        }
    }
    /// Locks with shared read access, blocking the current thread until it can be acquired.
    ///
    /// The calling thread will be blocked until there are no more writers which hold the lock.
    /// There may be other readers currently inside the lock when this method returns. This method
    /// does not provide any guarantees with respect to the ordering of whether contentious readers
    /// or writers will acquire the lock first.
    ///
    /// Returns a guard which will release the shared access when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::ShardedLock;
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// let lock = Arc::new(ShardedLock::new(1));
    /// let c_lock = lock.clone();
    ///
    /// let n = lock.read().unwrap();
    /// assert_eq!(*n, 1);
    ///
    /// thread::spawn(move || {
    ///     let r = c_lock.read();
    ///     assert!(r.is_ok());
    /// }).join().unwrap();
    /// ```
    pub fn read(&self) -> LockResult<ShardedLockReadGuard<'_, T>> {
        let current_index = current_index().unwrap_or(0);
        let shard_index = current_index & (self.shards.len() - 1);
        match self.shards[shard_index].lock.read() {
            Ok(guard) => {
                Ok(ShardedLockReadGuard {
                    lock: self,
                    _guard: guard,
                    _marker: PhantomData,
                })
            }
            Err(err) => {
                Err(
                    PoisonError::new(ShardedLockReadGuard {
                        lock: self,
                        _guard: err.into_inner(),
                        _marker: PhantomData,
                    }),
                )
            }
        }
    }
    /// Attempts to acquire this lock with exclusive write access.
    ///
    /// If the access could not be granted at this time, an error is returned. Otherwise, a guard
    /// is returned which will release the exclusive access when it is dropped. This method does
    /// not provide any guarantees with respect to the ordering of whether contentious readers or
    /// writers will acquire the lock first.
    ///
    /// This method will return an error if the lock is poisoned. A lock gets poisoned when a write
    /// operation panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::ShardedLock;
    ///
    /// let lock = ShardedLock::new(1);
    ///
    /// let n = lock.read().unwrap();
    /// assert_eq!(*n, 1);
    ///
    /// assert!(lock.try_write().is_err());
    /// ```
    pub fn try_write(&self) -> TryLockResult<ShardedLockWriteGuard<'_, T>> {
        let mut poisoned = false;
        let mut blocked = None;
        for (i, shard) in self.shards.iter().enumerate() {
            let guard = match shard.lock.try_write() {
                Ok(guard) => guard,
                Err(TryLockError::Poisoned(err)) => {
                    poisoned = true;
                    err.into_inner()
                }
                Err(TryLockError::WouldBlock) => {
                    blocked = Some(i);
                    break;
                }
            };
            unsafe {
                let guard: RwLockWriteGuard<'static, ()> = mem::transmute(guard);
                let dest: *mut _ = shard.write_guard.get();
                *dest = Some(guard);
            }
        }
        if let Some(i) = blocked {
            for shard in self.shards[0..i].iter().rev() {
                unsafe {
                    let dest: *mut _ = shard.write_guard.get();
                    let guard = mem::replace(&mut *dest, None);
                    drop(guard);
                }
            }
            Err(TryLockError::WouldBlock)
        } else if poisoned {
            let guard = ShardedLockWriteGuard {
                lock: self,
                _marker: PhantomData,
            };
            Err(TryLockError::Poisoned(PoisonError::new(guard)))
        } else {
            Ok(ShardedLockWriteGuard {
                lock: self,
                _marker: PhantomData,
            })
        }
    }
    /// Locks with exclusive write access, blocking the current thread until it can be acquired.
    ///
    /// The calling thread will be blocked until there are no more writers which hold the lock.
    /// There may be other readers currently inside the lock when this method returns. This method
    /// does not provide any guarantees with respect to the ordering of whether contentious readers
    /// or writers will acquire the lock first.
    ///
    /// Returns a guard which will release the exclusive access when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::ShardedLock;
    ///
    /// let lock = ShardedLock::new(1);
    ///
    /// let mut n = lock.write().unwrap();
    /// *n = 2;
    ///
    /// assert!(lock.try_read().is_err());
    /// ```
    pub fn write(&self) -> LockResult<ShardedLockWriteGuard<'_, T>> {
        let mut poisoned = false;
        for shard in self.shards.iter() {
            let guard = match shard.lock.write() {
                Ok(guard) => guard,
                Err(err) => {
                    poisoned = true;
                    err.into_inner()
                }
            };
            unsafe {
                let guard: RwLockWriteGuard<'_, ()> = guard;
                let guard: RwLockWriteGuard<'static, ()> = mem::transmute(guard);
                let dest: *mut _ = shard.write_guard.get();
                *dest = Some(guard);
            }
        }
        if poisoned {
            Err(
                PoisonError::new(ShardedLockWriteGuard {
                    lock: self,
                    _marker: PhantomData,
                }),
            )
        } else {
            Ok(ShardedLockWriteGuard {
                lock: self,
                _marker: PhantomData,
            })
        }
    }
}
impl<T: ?Sized + fmt::Debug> fmt::Debug for ShardedLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.try_read() {
            Ok(guard) => f.debug_struct("ShardedLock").field("data", &&*guard).finish(),
            Err(TryLockError::Poisoned(err)) => {
                f.debug_struct("ShardedLock").field("data", &&**err.get_ref()).finish()
            }
            Err(TryLockError::WouldBlock) => {
                struct LockedPlaceholder;
                impl fmt::Debug for LockedPlaceholder {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str("<locked>")
                    }
                }
                f.debug_struct("ShardedLock").field("data", &LockedPlaceholder).finish()
            }
        }
    }
}
impl<T: Default> Default for ShardedLock<T> {
    fn default() -> ShardedLock<T> {
        ShardedLock::new(Default::default())
    }
}
impl<T> From<T> for ShardedLock<T> {
    fn from(t: T) -> Self {
        ShardedLock::new(t)
    }
}
/// A guard used to release the shared read access of a [`ShardedLock`] when dropped.
///
/// [`ShardedLock`]: struct.ShardedLock.html
pub struct ShardedLockReadGuard<'a, T: ?Sized> {
    lock: &'a ShardedLock<T>,
    _guard: RwLockReadGuard<'a, ()>,
    _marker: PhantomData<RwLockReadGuard<'a, T>>,
}
unsafe impl<T: ?Sized + Sync> Sync for ShardedLockReadGuard<'_, T> {}
impl<T: ?Sized> Deref for ShardedLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}
impl<T: fmt::Debug> fmt::Debug for ShardedLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShardedLockReadGuard").field("lock", &self.lock).finish()
    }
}
impl<T: ?Sized + fmt::Display> fmt::Display for ShardedLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
/// A guard used to release the exclusive write access of a [`ShardedLock`] when dropped.
///
/// [`ShardedLock`]: struct.ShardedLock.html
pub struct ShardedLockWriteGuard<'a, T: ?Sized> {
    lock: &'a ShardedLock<T>,
    _marker: PhantomData<RwLockWriteGuard<'a, T>>,
}
unsafe impl<T: ?Sized + Sync> Sync for ShardedLockWriteGuard<'_, T> {}
impl<T: ?Sized> Drop for ShardedLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        for shard in self.lock.shards.iter().rev() {
            unsafe {
                let dest: *mut _ = shard.write_guard.get();
                let guard = mem::replace(&mut *dest, None);
                drop(guard);
            }
        }
    }
}
impl<T: fmt::Debug> fmt::Debug for ShardedLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShardedLockWriteGuard").field("lock", &self.lock).finish()
    }
}
impl<T: ?Sized + fmt::Display> fmt::Display for ShardedLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
impl<T: ?Sized> Deref for ShardedLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}
impl<T: ?Sized> DerefMut for ShardedLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}
/// Returns a `usize` that identifies the current thread.
///
/// Each thread is associated with an 'index'. While there are no particular guarantees, indices
/// usually tend to be consecutive numbers between 0 and the number of running threads.
///
/// Since this function accesses TLS, `None` might be returned if the current thread's TLS is
/// tearing down.
#[inline]
fn current_index() -> Option<usize> {
    REGISTRATION.try_with(|reg| reg.index).ok()
}
/// The global registry keeping track of registered threads and indices.
struct ThreadIndices {
    /// Mapping from `ThreadId` to thread index.
    mapping: HashMap<ThreadId, usize>,
    /// A list of free indices.
    free_list: Vec<usize>,
    /// The next index to allocate if the free list is empty.
    next_index: usize,
}
lazy_static! {
    static ref THREAD_INDICES : Mutex < ThreadIndices > = Mutex::new(ThreadIndices {
    mapping : HashMap::new(), free_list : Vec::new(), next_index : 0, });
}
/// A registration of a thread with an index.
///
/// When dropped, unregisters the thread and frees the reserved index.
struct Registration {
    index: usize,
    thread_id: ThreadId,
}
impl Drop for Registration {
    fn drop(&mut self) {
        let mut indices = THREAD_INDICES.lock().unwrap();
        indices.mapping.remove(&self.thread_id);
        indices.free_list.push(self.index);
    }
}
thread_local! {
    static REGISTRATION : Registration = { let thread_id = thread::current().id(); let
    mut indices = THREAD_INDICES.lock().unwrap(); let index = match indices.free_list
    .pop() { Some(i) => i, None => { let i = indices.next_index; indices.next_index += 1;
    i } }; indices.mapping.insert(thread_id, index); Registration { index, thread_id, }
    };
}
#[cfg(test)]
mod tests_rug_740 {
    use super::*;
    use crate::sync::sharded_lock::current_index;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_740_rrrruuuugggg_test_rug = 0;
        current_index();
        let _rug_ed_tests_rug_740_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_741 {
    use super::*;
    use crate::sync::ShardedLock;
    use std::sync::{RwLock, Mutex};
    use std::cell::UnsafeCell;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let value: i32 = rug_fuzz_0;
        let p0: i32 = value;
        ShardedLock::<i32>::new(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_742 {
    use super::*;
    use crate::sync::ShardedLock;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let lock = ShardedLock::new(String::new());
        {
            let mut s = lock.write().unwrap();
            *s = rug_fuzz_0.to_owned();
        }
        let result = lock.into_inner().unwrap();
        debug_assert_eq!(result, "modified");
             }
});    }
}
#[cfg(test)]
mod tests_rug_743 {
    use super::*;
    use crate::sync::ShardedLock;
    use std::sync::Arc;
    use std::thread;
    #[test]
    fn test_is_poisoned() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let lock = Arc::new(ShardedLock::new(rug_fuzz_0));
        let c_lock = lock.clone();
        let _ = thread::spawn(move || {
                let _lock = c_lock.write().unwrap();
                panic!();
            })
            .join();
        debug_assert_eq!(lock.is_poisoned(), true);
             }
});    }
}
#[cfg(test)]
mod tests_rug_744 {
    use super::*;
    use crate::sync::ShardedLock;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = ShardedLock::new(rug_fuzz_0);
        p0.get_mut();
             }
});    }
}
#[cfg(test)]
mod tests_rug_745 {
    use super::*;
    use crate::sync::ShardedLock;
    #[test]
    fn test_try_read() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let sharded_lock = ShardedLock::new(data);
        let result = sharded_lock.try_read();
        match result {
            Ok(guard) => {
                debug_assert_eq!(* guard, data);
            }
            Err(_) => unreachable!(),
        }
             }
});    }
}
#[cfg(test)]
mod tests_rug_746 {
    use super::*;
    use crate::sync::ShardedLock;
    use crate::sync::ShardedLockReadGuard;
    use std::marker::PhantomData;
    use std::sync::{Arc, PoisonError};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let value = rug_fuzz_0;
        let lock = Arc::new(ShardedLock::new(value));
        let c_lock = lock.clone();
        let guard = lock.read().unwrap();
        thread::spawn(move || {
                let _ = c_lock.read();
            })
            .join()
            .unwrap();
             }
});    }
}
#[cfg(test)]
mod tests_rug_747 {
    use super::*;
    use crate::sync::{ShardedLock, ShardedLockWriteGuard};
    #[test]
    fn test_try_write() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let sharded_lock = ShardedLock::new(data);
        let guard = sharded_lock.read().unwrap();
        debug_assert_eq!(* guard, data);
        debug_assert!(sharded_lock.try_write().is_err());
             }
});    }
}
#[cfg(test)]
mod tests_rug_748 {
    use super::*;
    use crate::sync::ShardedLock;
    #[test]
    fn test_write() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let lock = ShardedLock::<i32>::new(rug_fuzz_0);
        let p0 = &lock;
        p0.write().unwrap();
             }
});    }
}
#[cfg(test)]
mod tests_rug_749 {
    use super::*;
    use crate::sync::sharded_lock::ShardedLock;
    #[test]
    fn test_default_sharded_lock() {
        let _rug_st_tests_rug_749_rrrruuuugggg_test_default_sharded_lock = 0;
        let default_sharded_lock: ShardedLock<i32> = <ShardedLock<
            i32,
        > as Default>::default();
        let _rug_ed_tests_rug_749_rrrruuuugggg_test_default_sharded_lock = 0;
    }
}
