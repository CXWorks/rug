use core::borrow::{Borrow, BorrowMut};
use core::cmp;
use core::fmt;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::slice;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::alloc::alloc;
use crate::alloc::boxed::Box;
use crate::guard::Guard;
use crossbeam_utils::atomic::AtomicConsume;
/// Given ordering for the success case in a compare-exchange operation, returns the strongest
/// appropriate ordering for the failure case.
#[inline]
fn strongest_failure_ordering(ord: Ordering) -> Ordering {
    use self::Ordering::*;
    match ord {
        Relaxed | Release => Relaxed,
        Acquire | AcqRel => Acquire,
        _ => SeqCst,
    }
}
/// The error returned on failed compare-and-set operation.
pub struct CompareAndSetError<'g, T: ?Sized + Pointable, P: Pointer<T>> {
    /// The value in the atomic pointer at the time of the failed operation.
    pub current: Shared<'g, T>,
    /// The new value, which the operation failed to store.
    pub new: P,
}
impl<'g, T: 'g, P: Pointer<T> + fmt::Debug> fmt::Debug for CompareAndSetError<'g, T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompareAndSetError")
            .field("current", &self.current)
            .field("new", &self.new)
            .finish()
    }
}
/// Memory orderings for compare-and-set operations.
///
/// A compare-and-set operation can have different memory orderings depending on whether it
/// succeeds or fails. This trait generalizes different ways of specifying memory orderings.
///
/// The two ways of specifying orderings for compare-and-set are:
///
/// 1. Just one `Ordering` for the success case. In case of failure, the strongest appropriate
///    ordering is chosen.
/// 2. A pair of `Ordering`s. The first one is for the success case, while the second one is
///    for the failure case.
pub trait CompareAndSetOrdering {
    /// The ordering of the operation when it succeeds.
    fn success(&self) -> Ordering;
    /// The ordering of the operation when it fails.
    ///
    /// The failure ordering can't be `Release` or `AcqRel` and must be equivalent or weaker than
    /// the success ordering.
    fn failure(&self) -> Ordering;
}
impl CompareAndSetOrdering for Ordering {
    #[inline]
    fn success(&self) -> Ordering {
        *self
    }
    #[inline]
    fn failure(&self) -> Ordering {
        strongest_failure_ordering(*self)
    }
}
impl CompareAndSetOrdering for (Ordering, Ordering) {
    #[inline]
    fn success(&self) -> Ordering {
        self.0
    }
    #[inline]
    fn failure(&self) -> Ordering {
        self.1
    }
}
/// Returns a bitmask containing the unused least significant bits of an aligned pointer to `T`.
#[inline]
fn low_bits<T: ?Sized + Pointable>() -> usize {
    (1 << T::ALIGN.trailing_zeros()) - 1
}
/// Panics if the pointer is not properly unaligned.
#[inline]
fn ensure_aligned<T: ?Sized + Pointable>(raw: usize) {
    assert_eq!(raw & low_bits::< T > (), 0, "unaligned pointer");
}
/// Given a tagged pointer `data`, returns the same pointer, but tagged with `tag`.
///
/// `tag` is truncated to fit into the unused bits of the pointer to `T`.
#[inline]
fn compose_tag<T: ?Sized + Pointable>(data: usize, tag: usize) -> usize {
    (data & !low_bits::<T>()) | (tag & low_bits::<T>())
}
/// Decomposes a tagged pointer `data` into the pointer and the tag.
#[inline]
fn decompose_tag<T: ?Sized + Pointable>(data: usize) -> (usize, usize) {
    (data & !low_bits::<T>(), data & low_bits::<T>())
}
/// Types that are pointed to by a single word.
///
/// In concurrent programming, it is necessary to represent an object within a word because atomic
/// operations (e.g., reads, writes, read-modify-writes) support only single words.  This trait
/// qualifies such types that are pointed to by a single word.
///
/// The trait generalizes `Box<T>` for a sized type `T`.  In a box, an object of type `T` is
/// allocated in heap and it is owned by a single-word pointer.  This trait is also implemented for
/// `[MaybeUninit<T>]` by storing its size along with its elements and pointing to the pair of array
/// size and elements.
///
/// Pointers to `Pointable` types can be stored in [`Atomic`], [`Owned`], and [`Shared`].  In
/// particular, Crossbeam supports dynamically sized slices as follows.
///
/// ```
/// use std::mem::MaybeUninit;
/// use crossbeam_epoch::Owned;
///
/// let o = Owned::<[MaybeUninit<i32>]>::init(10); // allocating [i32; 10]
/// ```
///
/// [`Atomic`]: struct.Atomic.html
/// [`Owned`]: struct.Owned.html
/// [`Shared`]: struct.Shared.html
pub trait Pointable {
    /// The alignment of pointer.
    const ALIGN: usize;
    /// The type for initializers.
    type Init;
    /// Initializes a with the given initializer.
    ///
    /// # Safety
    ///
    /// The result should be a multiple of `ALIGN`.
    unsafe fn init(init: Self::Init) -> usize;
    /// Dereferences the given pointer.
    ///
    /// # Safety
    ///
    /// - The given `ptr` should have been initialized with [`Pointable::init`].
    /// - `ptr` should not have yet been dropped by [`Pointable::drop`].
    /// - `ptr` should not be mutably dereferenced by [`Pointable::deref_mut`] concurrently.
    ///
    /// [`Pointable::init`]: trait.Pointable.html#method.init
    /// [`Pointable::drop`]: trait.Pointable.html#method.drop
    /// [`Pointable::deref`]: trait.Pointable.html#method.deref
    unsafe fn deref<'a>(ptr: usize) -> &'a Self;
    /// Mutably dereferences the given pointer.
    ///
    /// # Safety
    ///
    /// - The given `ptr` should have been initialized with [`Pointable::init`].
    /// - `ptr` should not have yet been dropped by [`Pointable::drop`].
    /// - `ptr` should not be dereferenced by [`Pointable::deref`] or [`Pointable::deref_mut`]
    ///   concurrently.
    ///
    /// [`Pointable::init`]: trait.Pointable.html#method.init
    /// [`Pointable::drop`]: trait.Pointable.html#method.drop
    /// [`Pointable::deref`]: trait.Pointable.html#method.deref
    /// [`Pointable::deref_mut`]: trait.Pointable.html#method.deref_mut
    unsafe fn deref_mut<'a>(ptr: usize) -> &'a mut Self;
    /// Drops the object pointed to by the given pointer.
    ///
    /// # Safety
    ///
    /// - The given `ptr` should have been initialized with [`Pointable::init`].
    /// - `ptr` should not have yet been dropped by [`Pointable::drop`].
    /// - `ptr` should not be dereferenced by [`Pointable::deref`] or [`Pointable::deref_mut`]
    ///   concurrently.
    ///
    /// [`Pointable::init`]: trait.Pointable.html#method.init
    /// [`Pointable::drop`]: trait.Pointable.html#method.drop
    /// [`Pointable::deref`]: trait.Pointable.html#method.deref
    /// [`Pointable::deref_mut`]: trait.Pointable.html#method.deref_mut
    unsafe fn drop(ptr: usize);
}
impl<T> Pointable for T {
    const ALIGN: usize = mem::align_of::<T>();
    type Init = T;
    unsafe fn init(init: Self::Init) -> usize {
        Box::into_raw(Box::new(init)) as usize
    }
    unsafe fn deref<'a>(ptr: usize) -> &'a Self {
        &*(ptr as *const T)
    }
    unsafe fn deref_mut<'a>(ptr: usize) -> &'a mut Self {
        &mut *(ptr as *mut T)
    }
    unsafe fn drop(ptr: usize) {
        drop(Box::from_raw(ptr as *mut T));
    }
}
/// Array with size.
///
/// # Memory layout
///
/// An array consisting of size and elements:
///
/// ```ignore
///          elements
///          |
///          |
/// ------------------------------------
/// | size | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
/// ------------------------------------
/// ```
///
/// Its memory layout is different from that of `Box<[T]>` in that size is in the allocation (not
/// along with pointer as in `Box<[T]>`).
///
/// Elements are not present in the type, but they will be in the allocation.
/// ```
///
#[repr(C)]
struct Array<T> {
    size: usize,
    elements: [MaybeUninit<T>; 0],
}
impl<T> Pointable for [MaybeUninit<T>] {
    const ALIGN: usize = mem::align_of::<Array<T>>();
    type Init = usize;
    unsafe fn init(size: Self::Init) -> usize {
        let size = mem::size_of::<Array<T>>() + mem::size_of::<MaybeUninit<T>>() * size;
        let align = mem::align_of::<Array<T>>();
        let layout = alloc::Layout::from_size_align(size, align).unwrap();
        let ptr = alloc::alloc(layout) as *mut Array<T>;
        (*ptr).size = size;
        ptr as usize
    }
    unsafe fn deref<'a>(ptr: usize) -> &'a Self {
        let array = &*(ptr as *const Array<T>);
        slice::from_raw_parts(array.elements.as_ptr() as *const _, array.size)
    }
    unsafe fn deref_mut<'a>(ptr: usize) -> &'a mut Self {
        let array = &*(ptr as *mut Array<T>);
        slice::from_raw_parts_mut(array.elements.as_ptr() as *mut _, array.size)
    }
    unsafe fn drop(ptr: usize) {
        let array = &*(ptr as *mut Array<T>);
        let size = mem::size_of::<Array<T>>()
            + mem::size_of::<MaybeUninit<T>>() * array.size;
        let align = mem::align_of::<Array<T>>();
        let layout = alloc::Layout::from_size_align(size, align).unwrap();
        alloc::dealloc(ptr as *mut u8, layout);
    }
}
/// An atomic pointer that can be safely shared between threads.
///
/// The pointer must be properly aligned. Since it is aligned, a tag can be stored into the unused
/// least significant bits of the address. For example, the tag for a pointer to a sized type `T`
/// should be less than `(1 << mem::align_of::<T>().trailing_zeros())`.
///
/// Any method that loads the pointer must be passed a reference to a [`Guard`].
///
/// Crossbeam supports dynamically sized types.  See [`Pointable`] for details.
///
/// [`Guard`]: struct.Guard.html
/// [`Pointable`]: trait.Pointable.html
pub struct Atomic<T: ?Sized + Pointable> {
    data: AtomicUsize,
    _marker: PhantomData<*mut T>,
}
unsafe impl<T: ?Sized + Pointable + Send + Sync> Send for Atomic<T> {}
unsafe impl<T: ?Sized + Pointable + Send + Sync> Sync for Atomic<T> {}
impl<T> Atomic<T> {
    /// Allocates `value` on the heap and returns a new atomic pointer pointing to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Atomic;
    ///
    /// let a = Atomic::new(1234);
    /// ```
    pub fn new(init: T) -> Atomic<T> {
        Self::init(init)
    }
}
impl<T: ?Sized + Pointable> Atomic<T> {
    /// Allocates `value` on the heap and returns a new atomic pointer pointing to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Atomic;
    ///
    /// let a = Atomic::<i32>::init(1234);
    /// ```
    pub fn init(init: T::Init) -> Atomic<T> {
        Self::from(Owned::init(init))
    }
    /// Returns a new atomic pointer pointing to the tagged pointer `data`.
    fn from_usize(data: usize) -> Self {
        Self {
            data: AtomicUsize::new(data),
            _marker: PhantomData,
        }
    }
    /// Returns a new null atomic pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Atomic;
    ///
    /// let a = Atomic::<i32>::null();
    /// ```
    #[cfg(feature = "nightly")]
    pub const fn null() -> Atomic<T> {
        Self {
            data: AtomicUsize::new(0),
            _marker: PhantomData,
        }
    }
    /// Returns a new null atomic pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Atomic;
    ///
    /// let a = Atomic::<i32>::null();
    /// ```
    #[cfg(not(feature = "nightly"))]
    pub fn null() -> Atomic<T> {
        Self {
            data: AtomicUsize::new(0),
            _marker: PhantomData,
        }
    }
    /// Loads a `Shared` from the atomic pointer.
    ///
    /// This method takes an [`Ordering`] argument which describes the memory ordering of this
    /// operation.
    ///
    /// [`Ordering`]: https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(1234);
    /// let guard = &epoch::pin();
    /// let p = a.load(SeqCst, guard);
    /// ```
    pub fn load<'g>(&self, ord: Ordering, _: &'g Guard) -> Shared<'g, T> {
        unsafe { Shared::from_usize(self.data.load(ord)) }
    }
    /// Loads a `Shared` from the atomic pointer using a "consume" memory ordering.
    ///
    /// This is similar to the "acquire" ordering, except that an ordering is
    /// only guaranteed with operations that "depend on" the result of the load.
    /// However consume loads are usually much faster than acquire loads on
    /// architectures with a weak memory model since they don't require memory
    /// fence instructions.
    ///
    /// The exact definition of "depend on" is a bit vague, but it works as you
    /// would expect in practice since a lot of software, especially the Linux
    /// kernel, rely on this behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic};
    ///
    /// let a = Atomic::new(1234);
    /// let guard = &epoch::pin();
    /// let p = a.load_consume(guard);
    /// ```
    pub fn load_consume<'g>(&self, _: &'g Guard) -> Shared<'g, T> {
        unsafe { Shared::from_usize(self.data.load_consume()) }
    }
    /// Stores a `Shared` or `Owned` pointer into the atomic pointer.
    ///
    /// This method takes an [`Ordering`] argument which describes the memory ordering of this
    /// operation.
    ///
    /// [`Ordering`]: https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{Atomic, Owned, Shared};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(1234);
    /// a.store(Shared::null(), SeqCst);
    /// a.store(Owned::new(1234), SeqCst);
    /// ```
    pub fn store<P: Pointer<T>>(&self, new: P, ord: Ordering) {
        self.data.store(new.into_usize(), ord);
    }
    /// Stores a `Shared` or `Owned` pointer into the atomic pointer, returning the previous
    /// `Shared`.
    ///
    /// This method takes an [`Ordering`] argument which describes the memory ordering of this
    /// operation.
    ///
    /// [`Ordering`]: https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Shared};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(1234);
    /// let guard = &epoch::pin();
    /// let p = a.swap(Shared::null(), SeqCst, guard);
    /// ```
    pub fn swap<'g, P: Pointer<T>>(
        &self,
        new: P,
        ord: Ordering,
        _: &'g Guard,
    ) -> Shared<'g, T> {
        unsafe { Shared::from_usize(self.data.swap(new.into_usize(), ord)) }
    }
    /// Stores the pointer `new` (either `Shared` or `Owned`) into the atomic pointer if the current
    /// value is the same as `current`. The tag is also taken into account, so two pointers to the
    /// same object, but with different tags, will not be considered equal.
    ///
    /// The return value is a result indicating whether the new pointer was written. On success the
    /// pointer that was written is returned. On failure the actual current value and `new` are
    /// returned.
    ///
    /// This method takes a [`CompareAndSetOrdering`] argument which describes the memory
    /// ordering of this operation.
    ///
    /// [`CompareAndSetOrdering`]: trait.CompareAndSetOrdering.html
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Owned, Shared};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(1234);
    ///
    /// let guard = &epoch::pin();
    /// let curr = a.load(SeqCst, guard);
    /// let res1 = a.compare_and_set(curr, Shared::null(), SeqCst, guard);
    /// let res2 = a.compare_and_set(curr, Owned::new(5678), SeqCst, guard);
    /// ```
    pub fn compare_and_set<'g, O, P>(
        &self,
        current: Shared<'_, T>,
        new: P,
        ord: O,
        _: &'g Guard,
    ) -> Result<Shared<'g, T>, CompareAndSetError<'g, T, P>>
    where
        O: CompareAndSetOrdering,
        P: Pointer<T>,
    {
        let new = new.into_usize();
        self.data
            .compare_exchange(current.into_usize(), new, ord.success(), ord.failure())
            .map(|_| unsafe { Shared::from_usize(new) })
            .map_err(|current| unsafe {
                CompareAndSetError {
                    current: Shared::from_usize(current),
                    new: P::from_usize(new),
                }
            })
    }
    /// Stores the pointer `new` (either `Shared` or `Owned`) into the atomic pointer if the current
    /// value is the same as `current`. The tag is also taken into account, so two pointers to the
    /// same object, but with different tags, will not be considered equal.
    ///
    /// Unlike [`compare_and_set`], this method is allowed to spuriously fail even when comparison
    /// succeeds, which can result in more efficient code on some platforms.  The return value is a
    /// result indicating whether the new pointer was written. On success the pointer that was
    /// written is returned. On failure the actual current value and `new` are returned.
    ///
    /// This method takes a [`CompareAndSetOrdering`] argument which describes the memory
    /// ordering of this operation.
    ///
    /// [`compare_and_set`]: struct.Atomic.html#method.compare_and_set
    /// [`CompareAndSetOrdering`]: trait.CompareAndSetOrdering.html
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Owned, Shared};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(1234);
    /// let guard = &epoch::pin();
    ///
    /// let mut new = Owned::new(5678);
    /// let mut ptr = a.load(SeqCst, guard);
    /// loop {
    ///     match a.compare_and_set_weak(ptr, new, SeqCst, guard) {
    ///         Ok(p) => {
    ///             ptr = p;
    ///             break;
    ///         }
    ///         Err(err) => {
    ///             ptr = err.current;
    ///             new = err.new;
    ///         }
    ///     }
    /// }
    ///
    /// let mut curr = a.load(SeqCst, guard);
    /// loop {
    ///     match a.compare_and_set_weak(curr, Shared::null(), SeqCst, guard) {
    ///         Ok(_) => break,
    ///         Err(err) => curr = err.current,
    ///     }
    /// }
    /// ```
    pub fn compare_and_set_weak<'g, O, P>(
        &self,
        current: Shared<'_, T>,
        new: P,
        ord: O,
        _: &'g Guard,
    ) -> Result<Shared<'g, T>, CompareAndSetError<'g, T, P>>
    where
        O: CompareAndSetOrdering,
        P: Pointer<T>,
    {
        let new = new.into_usize();
        self.data
            .compare_exchange_weak(
                current.into_usize(),
                new,
                ord.success(),
                ord.failure(),
            )
            .map(|_| unsafe { Shared::from_usize(new) })
            .map_err(|current| unsafe {
                CompareAndSetError {
                    current: Shared::from_usize(current),
                    new: P::from_usize(new),
                }
            })
    }
    /// Bitwise "and" with the current tag.
    ///
    /// Performs a bitwise "and" operation on the current tag and the argument `val`, and sets the
    /// new tag to the result. Returns the previous pointer.
    ///
    /// This method takes an [`Ordering`] argument which describes the memory ordering of this
    /// operation.
    ///
    /// [`Ordering`]: https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Shared};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::<i32>::from(Shared::null().with_tag(3));
    /// let guard = &epoch::pin();
    /// assert_eq!(a.fetch_and(2, SeqCst, guard).tag(), 3);
    /// assert_eq!(a.load(SeqCst, guard).tag(), 2);
    /// ```
    pub fn fetch_and<'g>(
        &self,
        val: usize,
        ord: Ordering,
        _: &'g Guard,
    ) -> Shared<'g, T> {
        unsafe { Shared::from_usize(self.data.fetch_and(val | !low_bits::<T>(), ord)) }
    }
    /// Bitwise "or" with the current tag.
    ///
    /// Performs a bitwise "or" operation on the current tag and the argument `val`, and sets the
    /// new tag to the result. Returns the previous pointer.
    ///
    /// This method takes an [`Ordering`] argument which describes the memory ordering of this
    /// operation.
    ///
    /// [`Ordering`]: https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Shared};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::<i32>::from(Shared::null().with_tag(1));
    /// let guard = &epoch::pin();
    /// assert_eq!(a.fetch_or(2, SeqCst, guard).tag(), 1);
    /// assert_eq!(a.load(SeqCst, guard).tag(), 3);
    /// ```
    pub fn fetch_or<'g>(
        &self,
        val: usize,
        ord: Ordering,
        _: &'g Guard,
    ) -> Shared<'g, T> {
        unsafe { Shared::from_usize(self.data.fetch_or(val & low_bits::<T>(), ord)) }
    }
    /// Bitwise "xor" with the current tag.
    ///
    /// Performs a bitwise "xor" operation on the current tag and the argument `val`, and sets the
    /// new tag to the result. Returns the previous pointer.
    ///
    /// This method takes an [`Ordering`] argument which describes the memory ordering of this
    /// operation.
    ///
    /// [`Ordering`]: https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Shared};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::<i32>::from(Shared::null().with_tag(1));
    /// let guard = &epoch::pin();
    /// assert_eq!(a.fetch_xor(3, SeqCst, guard).tag(), 1);
    /// assert_eq!(a.load(SeqCst, guard).tag(), 2);
    /// ```
    pub fn fetch_xor<'g>(
        &self,
        val: usize,
        ord: Ordering,
        _: &'g Guard,
    ) -> Shared<'g, T> {
        unsafe { Shared::from_usize(self.data.fetch_xor(val & low_bits::<T>(), ord)) }
    }
    /// Takes ownership of the pointee.
    ///
    /// This consumes the atomic and converts it into [`Owned`]. As [`Atomic`] doesn't have a
    /// destructor and doesn't drop the pointee while [`Owned`] does, this is suitable for
    /// destructors of data structures.
    ///
    /// # Panics
    ///
    /// Panics if this pointer is null, but only in debug mode.
    ///
    /// # Safety
    ///
    /// This method may be called only if the pointer is valid and nobody else is holding a
    /// reference to the same object.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::mem;
    /// # use crossbeam_epoch::Atomic;
    /// struct DataStructure {
    ///     ptr: Atomic<usize>,
    /// }
    ///
    /// impl Drop for DataStructure {
    ///     fn drop(&mut self) {
    ///         // By now the DataStructure lives only in our thread and we are sure we don't hold
    ///         // any Shared or & to it ourselves.
    ///         unsafe {
    ///             drop(mem::replace(&mut self.ptr, Atomic::null()).into_owned());
    ///         }
    ///     }
    /// }
    /// ```
    pub unsafe fn into_owned(self) -> Owned<T> {
        Owned::from_usize(self.data.into_inner())
    }
}
impl<T: ?Sized + Pointable> fmt::Debug for Atomic<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = self.data.load(Ordering::SeqCst);
        let (raw, tag) = decompose_tag::<T>(data);
        f.debug_struct("Atomic").field("raw", &raw).field("tag", &tag).finish()
    }
}
impl<T: ?Sized + Pointable> fmt::Pointer for Atomic<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = self.data.load(Ordering::SeqCst);
        let (raw, _) = decompose_tag::<T>(data);
        fmt::Pointer::fmt(&(unsafe { T::deref(raw) as *const _ }), f)
    }
}
impl<T: ?Sized + Pointable> Clone for Atomic<T> {
    /// Returns a copy of the atomic value.
    ///
    /// Note that a `Relaxed` load is used here. If you need synchronization, use it with other
    /// atomics or fences.
    fn clone(&self) -> Self {
        let data = self.data.load(Ordering::Relaxed);
        Atomic::from_usize(data)
    }
}
impl<T: ?Sized + Pointable> Default for Atomic<T> {
    fn default() -> Self {
        Atomic::null()
    }
}
impl<T: ?Sized + Pointable> From<Owned<T>> for Atomic<T> {
    /// Returns a new atomic pointer pointing to `owned`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{Atomic, Owned};
    ///
    /// let a = Atomic::<i32>::from(Owned::new(1234));
    /// ```
    fn from(owned: Owned<T>) -> Self {
        let data = owned.data;
        mem::forget(owned);
        Self::from_usize(data)
    }
}
impl<T> From<Box<T>> for Atomic<T> {
    fn from(b: Box<T>) -> Self {
        Self::from(Owned::from(b))
    }
}
impl<T> From<T> for Atomic<T> {
    fn from(t: T) -> Self {
        Self::new(t)
    }
}
impl<'g, T: ?Sized + Pointable> From<Shared<'g, T>> for Atomic<T> {
    /// Returns a new atomic pointer pointing to `ptr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{Atomic, Shared};
    ///
    /// let a = Atomic::<i32>::from(Shared::<i32>::null());
    /// ```
    fn from(ptr: Shared<'g, T>) -> Self {
        Self::from_usize(ptr.data)
    }
}
impl<T> From<*const T> for Atomic<T> {
    /// Returns a new atomic pointer pointing to `raw`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ptr;
    /// use crossbeam_epoch::Atomic;
    ///
    /// let a = Atomic::<i32>::from(ptr::null::<i32>());
    /// ```
    fn from(raw: *const T) -> Self {
        Self::from_usize(raw as usize)
    }
}
/// A trait for either `Owned` or `Shared` pointers.
pub trait Pointer<T: ?Sized + Pointable> {
    /// Returns the machine representation of the pointer.
    fn into_usize(self) -> usize;
    /// Returns a new pointer pointing to the tagged pointer `data`.
    ///
    /// # Safety
    ///
    /// The given `data` should have been created by `Pointer::into_usize()`, and one `data` should
    /// not be converted back by `Pointer::from_usize()` mutliple times.
    unsafe fn from_usize(data: usize) -> Self;
}
/// An owned heap-allocated object.
///
/// This type is very similar to `Box<T>`.
///
/// The pointer must be properly aligned. Since it is aligned, a tag can be stored into the unused
/// least significant bits of the address.
pub struct Owned<T: ?Sized + Pointable> {
    data: usize,
    _marker: PhantomData<Box<T>>,
}
impl<T: ?Sized + Pointable> Pointer<T> for Owned<T> {
    #[inline]
    fn into_usize(self) -> usize {
        let data = self.data;
        mem::forget(self);
        data
    }
    /// Returns a new pointer pointing to the tagged pointer `data`.
    ///
    /// # Panics
    ///
    /// Panics if the data is zero in debug mode.
    #[inline]
    unsafe fn from_usize(data: usize) -> Self {
        debug_assert!(data != 0, "converting zero into `Owned`");
        Owned {
            data,
            _marker: PhantomData,
        }
    }
}
impl<T> Owned<T> {
    /// Returns a new owned pointer pointing to `raw`.
    ///
    /// This function is unsafe because improper use may lead to memory problems. Argument `raw`
    /// must be a valid pointer. Also, a double-free may occur if the function is called twice on
    /// the same raw pointer.
    ///
    /// # Panics
    ///
    /// Panics if `raw` is not properly aligned.
    ///
    /// # Safety
    ///
    /// The given `raw` should have been derived from `Owned`, and one `raw` should not be converted
    /// back by `Owned::from_raw()` mutliple times.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Owned;
    ///
    /// let o = unsafe { Owned::from_raw(Box::into_raw(Box::new(1234))) };
    /// ```
    pub unsafe fn from_raw(raw: *mut T) -> Owned<T> {
        let raw = raw as usize;
        ensure_aligned::<T>(raw);
        Self::from_usize(raw)
    }
    /// Converts the owned pointer into a `Box`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Owned;
    ///
    /// let o = Owned::new(1234);
    /// let b: Box<i32> = o.into_box();
    /// assert_eq!(*b, 1234);
    /// ```
    pub fn into_box(self) -> Box<T> {
        let (raw, _) = decompose_tag::<T>(self.data);
        mem::forget(self);
        unsafe { Box::from_raw(raw as *mut _) }
    }
    /// Allocates `value` on the heap and returns a new owned pointer pointing to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Owned;
    ///
    /// let o = Owned::new(1234);
    /// ```
    pub fn new(init: T) -> Owned<T> {
        Self::init(init)
    }
}
impl<T: ?Sized + Pointable> Owned<T> {
    /// Allocates `value` on the heap and returns a new owned pointer pointing to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Owned;
    ///
    /// let o = Owned::<i32>::init(1234);
    /// ```
    pub fn init(init: T::Init) -> Owned<T> {
        unsafe { Self::from_usize(T::init(init)) }
    }
    /// Converts the owned pointer into a [`Shared`].
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Owned};
    ///
    /// let o = Owned::new(1234);
    /// let guard = &epoch::pin();
    /// let p = o.into_shared(guard);
    /// ```
    ///
    /// [`Shared`]: struct.Shared.html
    #[allow(clippy::needless_lifetimes)]
    pub fn into_shared<'g>(self, _: &'g Guard) -> Shared<'g, T> {
        unsafe { Shared::from_usize(self.into_usize()) }
    }
    /// Returns the tag stored within the pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Owned;
    ///
    /// assert_eq!(Owned::new(1234).tag(), 0);
    /// ```
    pub fn tag(&self) -> usize {
        let (_, tag) = decompose_tag::<T>(self.data);
        tag
    }
    /// Returns the same pointer, but tagged with `tag`. `tag` is truncated to be fit into the
    /// unused bits of the pointer to `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Owned;
    ///
    /// let o = Owned::new(0u64);
    /// assert_eq!(o.tag(), 0);
    /// let o = o.with_tag(2);
    /// assert_eq!(o.tag(), 2);
    /// ```
    pub fn with_tag(self, tag: usize) -> Owned<T> {
        let data = self.into_usize();
        unsafe { Self::from_usize(compose_tag::<T>(data, tag)) }
    }
}
impl<T: ?Sized + Pointable> Drop for Owned<T> {
    fn drop(&mut self) {
        let (raw, _) = decompose_tag::<T>(self.data);
        unsafe {
            T::drop(raw);
        }
    }
}
impl<T: ?Sized + Pointable> fmt::Debug for Owned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (raw, tag) = decompose_tag::<T>(self.data);
        f.debug_struct("Owned").field("raw", &raw).field("tag", &tag).finish()
    }
}
impl<T: Clone> Clone for Owned<T> {
    fn clone(&self) -> Self {
        Owned::new((**self).clone()).with_tag(self.tag())
    }
}
impl<T: ?Sized + Pointable> Deref for Owned<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let (raw, _) = decompose_tag::<T>(self.data);
        unsafe { T::deref(raw) }
    }
}
impl<T: ?Sized + Pointable> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut T {
        let (raw, _) = decompose_tag::<T>(self.data);
        unsafe { T::deref_mut(raw) }
    }
}
impl<T> From<T> for Owned<T> {
    fn from(t: T) -> Self {
        Owned::new(t)
    }
}
impl<T> From<Box<T>> for Owned<T> {
    /// Returns a new owned pointer pointing to `b`.
    ///
    /// # Panics
    ///
    /// Panics if the pointer (the `Box`) is not properly aligned.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Owned;
    ///
    /// let o = unsafe { Owned::from_raw(Box::into_raw(Box::new(1234))) };
    /// ```
    fn from(b: Box<T>) -> Self {
        unsafe { Self::from_raw(Box::into_raw(b)) }
    }
}
impl<T: ?Sized + Pointable> Borrow<T> for Owned<T> {
    fn borrow(&self) -> &T {
        self.deref()
    }
}
impl<T: ?Sized + Pointable> BorrowMut<T> for Owned<T> {
    fn borrow_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}
impl<T: ?Sized + Pointable> AsRef<T> for Owned<T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}
impl<T: ?Sized + Pointable> AsMut<T> for Owned<T> {
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}
/// A pointer to an object protected by the epoch GC.
///
/// The pointer is valid for use only during the lifetime `'g`.
///
/// The pointer must be properly aligned. Since it is aligned, a tag can be stored into the unused
/// least significant bits of the address.
pub struct Shared<'g, T: 'g + ?Sized + Pointable> {
    data: usize,
    _marker: PhantomData<(&'g (), *const T)>,
}
impl<T: ?Sized + Pointable> Clone for Shared<'_, T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            _marker: PhantomData,
        }
    }
}
impl<T: ?Sized + Pointable> Copy for Shared<'_, T> {}
impl<T: ?Sized + Pointable> Pointer<T> for Shared<'_, T> {
    #[inline]
    fn into_usize(self) -> usize {
        self.data
    }
    #[inline]
    unsafe fn from_usize(data: usize) -> Self {
        Shared {
            data,
            _marker: PhantomData,
        }
    }
}
impl<'g, T> Shared<'g, T> {
    /// Converts the pointer to a raw pointer (without the tag).
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Owned};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let o = Owned::new(1234);
    /// let raw = &*o as *const _;
    /// let a = Atomic::from(o);
    ///
    /// let guard = &epoch::pin();
    /// let p = a.load(SeqCst, guard);
    /// assert_eq!(p.as_raw(), raw);
    /// ```
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn as_raw(&self) -> *const T {
        let (raw, _) = decompose_tag::<T>(self.data);
        raw as *const _
    }
}
impl<'g, T: ?Sized + Pointable> Shared<'g, T> {
    /// Returns a new null pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Shared;
    ///
    /// let p = Shared::<i32>::null();
    /// assert!(p.is_null());
    /// ```
    pub fn null() -> Shared<'g, T> {
        Shared {
            data: 0,
            _marker: PhantomData,
        }
    }
    /// Returns `true` if the pointer is null.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Owned};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::null();
    /// let guard = &epoch::pin();
    /// assert!(a.load(SeqCst, guard).is_null());
    /// a.store(Owned::new(1234), SeqCst);
    /// assert!(!a.load(SeqCst, guard).is_null());
    /// ```
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn is_null(&self) -> bool {
        let (raw, _) = decompose_tag::<T>(self.data);
        raw == 0
    }
    /// Dereferences the pointer.
    ///
    /// Returns a reference to the pointee that is valid during the lifetime `'g`.
    ///
    /// # Safety
    ///
    /// Dereferencing a pointer is unsafe because it could be pointing to invalid memory.
    ///
    /// Another concern is the possiblity of data races due to lack of proper synchronization.
    /// For example, consider the following scenario:
    ///
    /// 1. A thread creates a new object: `a.store(Owned::new(10), Relaxed)`
    /// 2. Another thread reads it: `*a.load(Relaxed, guard).as_ref().unwrap()`
    ///
    /// The problem is that relaxed orderings don't synchronize initialization of the object with
    /// the read from the second thread. This is a data race. A possible solution would be to use
    /// `Release` and `Acquire` orderings.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(1234);
    /// let guard = &epoch::pin();
    /// let p = a.load(SeqCst, guard);
    /// unsafe {
    ///     assert_eq!(p.deref(), &1234);
    /// }
    /// ```
    #[allow(clippy::trivially_copy_pass_by_ref)]
    #[allow(clippy::should_implement_trait)]
    pub unsafe fn deref(&self) -> &'g T {
        let (raw, _) = decompose_tag::<T>(self.data);
        T::deref(raw)
    }
    /// Dereferences the pointer.
    ///
    /// Returns a mutable reference to the pointee that is valid during the lifetime `'g`.
    ///
    /// # Safety
    ///
    /// * There is no guarantee that there are no more threads attempting to read/write from/to the
    ///   actual object at the same time.
    ///
    ///   The user must know that there are no concurrent accesses towards the object itself.
    ///
    /// * Other than the above, all safety concerns of `deref()` applies here.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(vec![1, 2, 3, 4]);
    /// let guard = &epoch::pin();
    ///
    /// let mut p = a.load(SeqCst, guard);
    /// unsafe {
    ///     assert!(!p.is_null());
    ///     let b = p.deref_mut();
    ///     assert_eq!(b, &vec![1, 2, 3, 4]);
    ///     b.push(5);
    ///     assert_eq!(b, &vec![1, 2, 3, 4, 5]);
    /// }
    ///
    /// let p = a.load(SeqCst, guard);
    /// unsafe {
    ///     assert_eq!(p.deref(), &vec![1, 2, 3, 4, 5]);
    /// }
    /// ```
    #[allow(clippy::should_implement_trait)]
    pub unsafe fn deref_mut(&mut self) -> &'g mut T {
        let (raw, _) = decompose_tag::<T>(self.data);
        T::deref_mut(raw)
    }
    /// Converts the pointer to a reference.
    ///
    /// Returns `None` if the pointer is null, or else a reference to the object wrapped in `Some`.
    ///
    /// # Safety
    ///
    /// Dereferencing a pointer is unsafe because it could be pointing to invalid memory.
    ///
    /// Another concern is the possiblity of data races due to lack of proper synchronization.
    /// For example, consider the following scenario:
    ///
    /// 1. A thread creates a new object: `a.store(Owned::new(10), Relaxed)`
    /// 2. Another thread reads it: `*a.load(Relaxed, guard).as_ref().unwrap()`
    ///
    /// The problem is that relaxed orderings don't synchronize initialization of the object with
    /// the read from the second thread. This is a data race. A possible solution would be to use
    /// `Release` and `Acquire` orderings.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(1234);
    /// let guard = &epoch::pin();
    /// let p = a.load(SeqCst, guard);
    /// unsafe {
    ///     assert_eq!(p.as_ref(), Some(&1234));
    /// }
    /// ```
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub unsafe fn as_ref(&self) -> Option<&'g T> {
        let (raw, _) = decompose_tag::<T>(self.data);
        if raw == 0 { None } else { Some(T::deref(raw)) }
    }
    /// Takes ownership of the pointee.
    ///
    /// # Panics
    ///
    /// Panics if this pointer is null, but only in debug mode.
    ///
    /// # Safety
    ///
    /// This method may be called only if the pointer is valid and nobody else is holding a
    /// reference to the same object.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(1234);
    /// unsafe {
    ///     let guard = &epoch::unprotected();
    ///     let p = a.load(SeqCst, guard);
    ///     drop(p.into_owned());
    /// }
    /// ```
    pub unsafe fn into_owned(self) -> Owned<T> {
        debug_assert!(! self.is_null(), "converting a null `Shared` into `Owned`");
        Owned::from_usize(self.data)
    }
    /// Returns the tag stored within the pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic, Owned};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::<u64>::from(Owned::new(0u64).with_tag(2));
    /// let guard = &epoch::pin();
    /// let p = a.load(SeqCst, guard);
    /// assert_eq!(p.tag(), 2);
    /// ```
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn tag(&self) -> usize {
        let (_, tag) = decompose_tag::<T>(self.data);
        tag
    }
    /// Returns the same pointer, but tagged with `tag`. `tag` is truncated to be fit into the
    /// unused bits of the pointer to `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::{self as epoch, Atomic};
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// let a = Atomic::new(0u64);
    /// let guard = &epoch::pin();
    /// let p1 = a.load(SeqCst, guard);
    /// let p2 = p1.with_tag(2);
    ///
    /// assert_eq!(p1.tag(), 0);
    /// assert_eq!(p2.tag(), 2);
    /// assert_eq!(p1.as_raw(), p2.as_raw());
    /// ```
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn with_tag(&self, tag: usize) -> Shared<'g, T> {
        unsafe { Self::from_usize(compose_tag::<T>(self.data, tag)) }
    }
}
impl<T> From<*const T> for Shared<'_, T> {
    /// Returns a new pointer pointing to `raw`.
    ///
    /// # Panics
    ///
    /// Panics if `raw` is not properly aligned.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_epoch::Shared;
    ///
    /// let p = Shared::from(Box::into_raw(Box::new(1234)) as *const _);
    /// assert!(!p.is_null());
    /// ```
    fn from(raw: *const T) -> Self {
        let raw = raw as usize;
        ensure_aligned::<T>(raw);
        unsafe { Self::from_usize(raw) }
    }
}
impl<'g, T: ?Sized + Pointable> PartialEq<Shared<'g, T>> for Shared<'g, T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}
impl<T: ?Sized + Pointable> Eq for Shared<'_, T> {}
impl<'g, T: ?Sized + Pointable> PartialOrd<Shared<'g, T>> for Shared<'g, T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.data.partial_cmp(&other.data)
    }
}
impl<T: ?Sized + Pointable> Ord for Shared<'_, T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.data.cmp(&other.data)
    }
}
impl<T: ?Sized + Pointable> fmt::Debug for Shared<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (raw, tag) = decompose_tag::<T>(self.data);
        f.debug_struct("Shared").field("raw", &raw).field("tag", &tag).finish()
    }
}
impl<T: ?Sized + Pointable> fmt::Pointer for Shared<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(unsafe { self.deref() as *const _ }), f)
    }
}
impl<T: ?Sized + Pointable> Default for Shared<'_, T> {
    fn default() -> Self {
        Shared::null()
    }
}
#[cfg(test)]
mod tests {
    use super::Shared;
    #[test]
    fn valid_tag_i8() {
        Shared::<i8>::null().with_tag(0);
    }
    #[test]
    fn valid_tag_i64() {
        Shared::<i64>::null().with_tag(7);
    }
}
#[cfg(test)]
mod tests_rug_344 {
    use super::*;
    use crate::atomic::Ordering;
    #[test]
    fn test_strongest_failure_ordering() {
        let _rug_st_tests_rug_344_rrrruuuugggg_test_strongest_failure_ordering = 0;
        let p0 = Ordering::Relaxed;
        debug_assert_eq!(
            crate ::atomic::strongest_failure_ordering(p0), Ordering::Relaxed
        );
        let p1 = Ordering::Release;
        debug_assert_eq!(
            crate ::atomic::strongest_failure_ordering(p1), Ordering::Relaxed
        );
        let p2 = Ordering::Acquire;
        debug_assert_eq!(
            crate ::atomic::strongest_failure_ordering(p2), Ordering::Acquire
        );
        let p3 = Ordering::AcqRel;
        debug_assert_eq!(
            crate ::atomic::strongest_failure_ordering(p3), Ordering::Acquire
        );
        let p4 = Ordering::SeqCst;
        debug_assert_eq!(
            crate ::atomic::strongest_failure_ordering(p4), Ordering::SeqCst
        );
        let _rug_ed_tests_rug_344_rrrruuuugggg_test_strongest_failure_ordering = 0;
    }
}
#[cfg(test)]
mod tests_rug_345 {
    use super::*;
    use crate::atomic;
    #[test]
    fn test_low_bits() {
        let _rug_st_tests_rug_345_rrrruuuugggg_test_low_bits = 0;
        let result: usize = atomic::low_bits::<usize>();
        let _rug_ed_tests_rug_345_rrrruuuugggg_test_low_bits = 0;
    }
}
#[cfg(test)]
mod tests_rug_346 {
    use super::*;
    use crate::atomic::Pointable;
    #[test]
    fn test_ensure_aligned() {
        let _rug_st_tests_rug_346_rrrruuuugggg_test_ensure_aligned = 0;
        let rug_fuzz_0 = 123;
        let raw: usize = rug_fuzz_0;
        crate::atomic::ensure_aligned::<usize>(raw);
        let _rug_ed_tests_rug_346_rrrruuuugggg_test_ensure_aligned = 0;
    }
}
#[cfg(test)]
mod tests_rug_347 {
    use super::*;
    use crate::atomic::Pointable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_347_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x12345678;
        let rug_fuzz_1 = 0x9abcdef0;
        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        crate::atomic::compose_tag::<usize>(p0, p1);
        let _rug_ed_tests_rug_347_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_348 {
    use super::*;
    use crate::atomic::Pointable;
    #[test]
    fn test_decompose_tag() {
        let _rug_st_tests_rug_348_rrrruuuugggg_test_decompose_tag = 0;
        let rug_fuzz_0 = 12345;
        let data: usize = rug_fuzz_0;
        crate::atomic::decompose_tag::<usize>(data);
        let _rug_ed_tests_rug_348_rrrruuuugggg_test_decompose_tag = 0;
    }
}
#[cfg(test)]
mod tests_rug_349 {
    use super::*;
    use crate::atomic::Ordering;
    use crate::CompareAndSetOrdering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_349_rrrruuuugggg_test_rug = 0;
        let p0 = Ordering::Relaxed;
        <Ordering as CompareAndSetOrdering>::success(&p0);
        let _rug_ed_tests_rug_349_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_350 {
    use super::*;
    use crate::CompareAndSetOrdering;
    use crate::atomic::Ordering;
    #[test]
    fn test_failure() {
        let _rug_st_tests_rug_350_rrrruuuugggg_test_failure = 0;
        let p0 = Ordering::Relaxed;
        p0.failure();
        let _rug_ed_tests_rug_350_rrrruuuugggg_test_failure = 0;
    }
}
#[cfg(test)]
mod tests_rug_351 {
    use super::*;
    use crate::atomic::CompareAndSetOrdering;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_351_rrrruuuugggg_test_rug = 0;
        let mut p0 = (Ordering::Relaxed, Ordering::Acquire);
        <(Ordering, Ordering) as CompareAndSetOrdering>::success(&p0);
        let _rug_ed_tests_rug_351_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_352 {
    use super::*;
    use crate::atomic::CompareAndSetOrdering;
    #[test]
    fn test_failure() {
        let _rug_st_tests_rug_352_rrrruuuugggg_test_failure = 0;
        let p0 = (
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::AcqRel,
        );
        p0.failure();
        let _rug_ed_tests_rug_352_rrrruuuugggg_test_failure = 0;
    }
}
#[cfg(test)]
mod tests_rug_361 {
    use super::*;
    use crate::Atomic;
    #[test]
    fn test_atomic_new() {
        let _rug_st_tests_rug_361_rrrruuuugggg_test_atomic_new = 0;
        let rug_fuzz_0 = 1234;
        let p0: i32 = rug_fuzz_0;
        Atomic::<i32>::new(p0);
        let _rug_ed_tests_rug_361_rrrruuuugggg_test_atomic_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_363 {
    use super::*;
    use crate::atomic::Atomic;
    use std::marker::PhantomData;
    use std::sync::atomic::AtomicUsize;
    #[test]
    fn test_from_usize() {
        let _rug_st_tests_rug_363_rrrruuuugggg_test_from_usize = 0;
        let rug_fuzz_0 = 123;
        let p0: usize = rug_fuzz_0;
        Atomic::<usize>::from_usize(p0);
        let _rug_ed_tests_rug_363_rrrruuuugggg_test_from_usize = 0;
    }
}
#[cfg(test)]
mod tests_rug_364 {
    use super::*;
    use crate::Atomic;
    use std::marker::PhantomData;
    use std::sync::atomic::AtomicUsize;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_364_rrrruuuugggg_test_rug = 0;
        let a = <Atomic<i32>>::null();
        let _rug_ed_tests_rug_364_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_365 {
    use super::*;
    use crate::{Atomic, Guard};
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_365_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: Atomic<u32> = Atomic::<u32>::new(rug_fuzz_0);
        let p1 = Ordering::SeqCst;
        let guard = Guard { local: std::ptr::null() };
        p0.load(p1, &guard);
        let _rug_ed_tests_rug_365_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_366 {
    use super::*;
    use crate::{Atomic, pin};
    #[test]
    fn test_load_consume() {
        let _rug_st_tests_rug_366_rrrruuuugggg_test_load_consume = 0;
        let rug_fuzz_0 = 1234;
        let a = Atomic::new(rug_fuzz_0);
        let guard = &pin();
        let p0 = &a;
        let p1 = &guard;
        p0.load_consume(p1);
        let _rug_ed_tests_rug_366_rrrruuuugggg_test_load_consume = 0;
    }
}
#[cfg(test)]
mod tests_rug_367 {
    use super::*;
    use crate::{Atomic, Owned, Shared};
    use crate::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_367_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 1234;
        let p0: Atomic<u32> = Atomic::<u32>::new(rug_fuzz_0);
        let p1: Owned<u32> = Owned::new(rug_fuzz_1);
        let p2 = Ordering::SeqCst;
        p0.store(p1, p2);
        let _rug_ed_tests_rug_367_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_368 {
    use crate::{self as epoch, Atomic, Owned, Shared};
    use std::sync::atomic::Ordering::SeqCst;
    #[test]
    fn test_swap() {
        let _rug_st_tests_rug_368_rrrruuuugggg_test_swap = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 10;
        let p0: Atomic<u32> = Atomic::<u32>::new(rug_fuzz_0);
        let p1: Owned<u32> = Owned::new(rug_fuzz_1);
        let guard = &epoch::pin();
        let p2 = p0.swap(p1, SeqCst, guard);
        let _rug_ed_tests_rug_368_rrrruuuugggg_test_swap = 0;
    }
}
#[cfg(test)]
mod tests_rug_369 {
    use crate::{self as epoch, Atomic, Owned, Shared};
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_369_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 5678;
        let v77: Atomic<u32> = Atomic::<u32>::new(rug_fuzz_0);
        let guard = &epoch::pin();
        let curr = v77.load(Ordering::SeqCst, guard);
        let res1 = v77.compare_and_set(curr, Shared::null(), Ordering::SeqCst, guard);
        let res2 = v77
            .compare_and_set(curr, Owned::new(rug_fuzz_1), Ordering::SeqCst, guard);
        let _rug_ed_tests_rug_369_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_371 {
    use super::*;
    use crate::{self as epoch, Atomic, Shared};
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_371_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3;
        let rug_fuzz_1 = 2;
        let p0: Atomic<i32> = Atomic::<i32>::from(Shared::null().with_tag(rug_fuzz_0));
        let p1: usize = rug_fuzz_1;
        let p2: Ordering = Ordering::SeqCst;
        let guard = &epoch::pin();
        p0.fetch_and(p1, p2, guard);
        let _rug_ed_tests_rug_371_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_372 {
    use super::*;
    use crate::{self as epoch, Atomic, Shared};
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_372_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let mut p0: Atomic::<i32> = Atomic::<
            i32,
        >::from(Shared::null().with_tag(rug_fuzz_0));
        let p1: usize = rug_fuzz_1;
        let p2 = Ordering::SeqCst;
        let guard = &epoch::pin();
        debug_assert_eq!(p0.fetch_or(p1, p2, guard).tag(), 1);
        debug_assert_eq!(p0.load(Ordering::SeqCst, guard).tag(), 3);
        let _rug_ed_tests_rug_372_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_373 {
    use super::*;
    use crate::{self as epoch, Atomic, Shared};
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_373_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 3;
        let p0: Atomic<u32> = Atomic::<u32>::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        let p2: Ordering = Ordering::SeqCst;
        let guard = &epoch::pin();
        p0.fetch_xor(p1, p2, guard);
        let _rug_ed_tests_rug_373_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_374 {
    use super::*;
    use crate::{Atomic, Owned};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_374_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let mut p0: Atomic<usize> = Atomic::<usize>::new(rug_fuzz_0);
        unsafe {
            let _owned: Owned<usize> = Atomic::<usize>::into_owned(p0);
        }
        let _rug_ed_tests_rug_374_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_375 {
    use super::*;
    use crate::atomic::Atomic;
    use std::clone::Clone;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_375_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 15;
        let p0: Atomic<i32> = Atomic::<i32>::new(rug_fuzz_0);
        p0.clone();
        let _rug_ed_tests_rug_375_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_380 {
    use super::*;
    use crate::{Atomic, Shared};
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_380_rrrruuuugggg_test_from = 0;
        let p0: Shared<'_, i32> = Shared::<i32>::null();
        Atomic::<i32>::from(p0);
        let _rug_ed_tests_rug_380_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_382 {
    use super::*;
    use crate::atomic;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_382_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = atomic::Owned::new(Box::new(rug_fuzz_0));
        p0.into_usize();
        let _rug_ed_tests_rug_382_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_384 {
    use super::*;
    use crate::{Owned, atomic};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_384_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let mut p0: *mut i32 = Box::into_raw(Box::new(rug_fuzz_0));
        unsafe {
            <atomic::Owned::<i32>>::from_raw(p0);
        }
        let _rug_ed_tests_rug_384_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_385 {
    use super::*;
    use crate::Owned;
    use std::mem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_385_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Owned::new(Box::new(rug_fuzz_0));
        p0.into_box();
        let _rug_ed_tests_rug_385_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_386 {
    use super::*;
    use crate::Owned;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_386_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let mut p0: i32 = rug_fuzz_0;
        Owned::<i32>::new(p0);
        let _rug_ed_tests_rug_386_rrrruuuugggg_test_rug = 0;
    }
}
use crate::{self as epoch, atomic};
#[cfg(test)]
mod tests_rug_388 {
    use super::*;
    use crate::Guard;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_388_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = atomic::Owned::new(Box::new(rug_fuzz_0));
        let guard = &epoch::pin();
        atomic::Owned::<Box<i32>>::into_shared(p0, guard);
        let _rug_ed_tests_rug_388_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_389 {
    use super::*;
    use crate::Owned;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_389_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Owned::new(Box::new(rug_fuzz_0));
        debug_assert_eq!(p0.tag(), 0);
        let _rug_ed_tests_rug_389_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_390 {
    use super::*;
    use crate::Owned;
    #[test]
    fn test_with_tag() {
        let _rug_st_tests_rug_390_rrrruuuugggg_test_with_tag = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 5;
        let mut p0 = Owned::new(Box::new(rug_fuzz_0));
        let p1: usize = rug_fuzz_1;
        p0.with_tag(p1);
        let _rug_ed_tests_rug_390_rrrruuuugggg_test_with_tag = 0;
    }
}
#[cfg(test)]
mod tests_rug_393 {
    use super::*;
    use crate::atomic;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_393_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = atomic::Owned::new(Box::new(rug_fuzz_0));
        p0.deref();
        let _rug_ed_tests_rug_393_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_395 {
    use super::*;
    use crate::atomic::Owned;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_395_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 42;
        let p0: u32 = rug_fuzz_0;
        let _ = <Owned<u32> as std::convert::From<u32>>::from(p0);
        let _rug_ed_tests_rug_395_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_396 {
    use super::*;
    use crate::Owned;
    use std::boxed::Box;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_396_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Box<i32> = Box::new(rug_fuzz_0);
        <Owned<i32> as From<Box<i32>>>::from(p0);
        let _rug_ed_tests_rug_396_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_399 {
    use super::*;
    use crate::atomic;
    use crate::Owned;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_399_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = atomic::Owned::new(Box::new(rug_fuzz_0));
        p0.as_ref();
        let _rug_ed_tests_rug_399_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_403 {
    use super::*;
    use crate::atomic::{Pointer, Shared};
    use std::marker::PhantomData;
    struct MyType;
    #[test]
    fn test_from_usize() {
        let _rug_st_tests_rug_403_rrrruuuugggg_test_from_usize = 0;
        let rug_fuzz_0 = 42;
        let data: usize = rug_fuzz_0;
        let p0: usize = data;
        let shared: Shared<'_, MyType> = unsafe {
            <Shared<'_, MyType> as Pointer<MyType>>::from_usize(p0)
        };
        let _rug_ed_tests_rug_403_rrrruuuugggg_test_from_usize = 0;
    }
}
#[cfg(test)]
mod tests_rug_404 {
    use super::*;
    use crate::{self as epoch, Atomic, Owned};
    use std::sync::atomic::Ordering::SeqCst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_404_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let o = Owned::new(rug_fuzz_0);
        let a = Atomic::from(o);
        let guard = &epoch::pin();
        let p0: atomic::Shared<'_, i32> = a.load(SeqCst, guard);
        p0.as_raw();
        let _rug_ed_tests_rug_404_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_405 {
    use super::*;
    use crate::Shared;
    use std::marker::PhantomData;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_405_rrrruuuugggg_test_rug = 0;
        let p: Shared<i32> = Shared::<i32>::null();
        debug_assert!(p.is_null());
        let _rug_ed_tests_rug_405_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_406 {
    use super::*;
    use crate::{self as epoch, Atomic, Owned};
    use std::sync::atomic::Ordering::SeqCst;
    #[test]
    fn test_is_null() {
        let _rug_st_tests_rug_406_rrrruuuugggg_test_is_null = 0;
        let rug_fuzz_0 = 1234;
        let a: Atomic<i32> = Atomic::null();
        let guard = &epoch::pin();
        debug_assert!(atomic::Shared:: < '_, _ > ::is_null(& a.load(SeqCst, guard)));
        a.store(Owned::new(rug_fuzz_0), SeqCst);
        debug_assert!(! atomic::Shared:: < '_, _ > ::is_null(& a.load(SeqCst, guard)));
        let _rug_ed_tests_rug_406_rrrruuuugggg_test_is_null = 0;
    }
}
#[cfg(test)]
mod tests_rug_407 {
    use super::*;
    use crate::{self as epoch, Atomic};
    use std::sync::atomic::Ordering::SeqCst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_407_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let a = Atomic::new(rug_fuzz_0);
        let guard = &epoch::pin();
        let p0 = a.load(SeqCst, guard);
        unsafe {
            debug_assert_eq!(crate ::atomic::Shared:: < '_, _ > ::deref(& p0), & 1234);
        }
        let _rug_ed_tests_rug_407_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_408 {
    use crate::{self as epoch, Atomic};
    use std::sync::atomic::Ordering::SeqCst;
    #[test]
    fn test_deref_mut() {
        let _rug_st_tests_rug_408_rrrruuuugggg_test_deref_mut = 0;
        let rug_fuzz_0 = 1;
        let a = Atomic::new(vec![rug_fuzz_0, 2, 3, 4]);
        let guard = &epoch::pin();
        let mut p0 = a.load(SeqCst, guard);
        unsafe {
            debug_assert!(! p0.is_null());
            let _b = p0.deref_mut();
        }
        let _rug_ed_tests_rug_408_rrrruuuugggg_test_deref_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_409 {
    use crate::{self as epoch, Atomic, Owned, Shared};
    use std::sync::atomic::Ordering::SeqCst;
    #[test]
    fn test_as_ref() {
        let _rug_st_tests_rug_409_rrrruuuugggg_test_as_ref = 0;
        let rug_fuzz_0 = 1234;
        let a = Atomic::new(rug_fuzz_0);
        let guard = &epoch::pin();
        let p0: Shared<'_, i32> = a.load(SeqCst, guard);
        unsafe {
            debug_assert_eq!(p0.as_ref(), Some(& 1234));
        }
        let _rug_ed_tests_rug_409_rrrruuuugggg_test_as_ref = 0;
    }
}
#[cfg(test)]
mod tests_rug_411 {
    use super::*;
    use crate::{self as epoch, Atomic, Owned};
    use std::sync::atomic::Ordering::SeqCst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_411_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u64;
        let rug_fuzz_1 = 2;
        let guard = &epoch::pin();
        let data = Owned::new(rug_fuzz_0).with_tag(rug_fuzz_1);
        let a = Atomic::<u64>::from(data);
        let p0: atomic::Shared<'_, u64> = a.load(SeqCst, guard);
        p0.tag();
        let _rug_ed_tests_rug_411_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_412 {
    use super::*;
    use crate::{self as epoch, Atomic};
    use std::sync::atomic::Ordering::SeqCst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_412_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0u64;
        let rug_fuzz_1 = 2;
        let a = Atomic::<u64>::new(rug_fuzz_0);
        let guard = &epoch::pin();
        let p0 = a.load(SeqCst, guard);
        let tag = rug_fuzz_1;
        let p1 = p0.with_tag(tag);
        let _rug_ed_tests_rug_412_rrrruuuugggg_test_rug = 0;
    }
}
