#![allow(clippy::unit_arg)]
#![allow(clippy::let_unit_value)]
use core::cell::UnsafeCell;
use core::fmt;
use core::mem;
use core::ptr;
use core::sync::atomic::{self, AtomicBool, Ordering};
#[cfg(feature = "std")]
use std::panic::{RefUnwindSafe, UnwindSafe};
use super::seq_lock::SeqLock;
/// A thread-safe mutable memory location.
///
/// This type is equivalent to [`Cell`], except it can also be shared among multiple threads.
///
/// Operations on `AtomicCell`s use atomic instructions whenever possible, and synchronize using
/// global locks otherwise. You can call [`AtomicCell::<T>::is_lock_free()`] to check whether
/// atomic instructions or locks will be used.
///
/// Atomic loads use the [`Acquire`] ordering and atomic stores use the [`Release`] ordering.
///
/// [`Cell`]: https://doc.rust-lang.org/std/cell/struct.Cell.html
/// [`AtomicCell::<T>::is_lock_free()`]: struct.AtomicCell.html#method.is_lock_free
/// [`Acquire`]: https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html#variant.Acquire
/// [`Release`]: https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html#variant.Release
#[repr(transparent)]
pub struct AtomicCell<T: ?Sized> {
    /// The inner value.
    ///
    /// If this value can be transmuted into a primitive atomic type, it will be treated as such.
    /// Otherwise, all potentially concurrent operations on this data will be protected by a global
    /// lock.
    value: UnsafeCell<T>,
}
unsafe impl<T: Send> Send for AtomicCell<T> {}
unsafe impl<T: Send> Sync for AtomicCell<T> {}
#[cfg(feature = "std")]
impl<T> UnwindSafe for AtomicCell<T> {}
#[cfg(feature = "std")]
impl<T> RefUnwindSafe for AtomicCell<T> {}
impl<T> AtomicCell<T> {
    /// Creates a new atomic cell initialized with `val`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(7);
    /// ```
    pub const fn new(val: T) -> AtomicCell<T> {
        AtomicCell {
            value: UnsafeCell::new(val),
        }
    }
    /// Unwraps the atomic cell and returns its inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(7);
    /// let v = a.into_inner();
    ///
    /// assert_eq!(v, 7);
    /// ```
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
    /// Returns `true` if operations on values of this type are lock-free.
    ///
    /// If the compiler or the platform doesn't support the necessary atomic instructions,
    /// `AtomicCell<T>` will use global locks for every potentially concurrent atomic operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// // This type is internally represented as `AtomicUsize` so we can just use atomic
    /// // operations provided by it.
    /// assert_eq!(AtomicCell::<usize>::is_lock_free(), true);
    ///
    /// // A wrapper struct around `isize`.
    /// struct Foo {
    ///     bar: isize,
    /// }
    /// // `AtomicCell<Foo>` will be internally represented as `AtomicIsize`.
    /// assert_eq!(AtomicCell::<Foo>::is_lock_free(), true);
    ///
    /// // Operations on zero-sized types are always lock-free.
    /// assert_eq!(AtomicCell::<()>::is_lock_free(), true);
    ///
    /// // Very large types cannot be represented as any of the standard atomic types, so atomic
    /// // operations on them will have to use global locks for synchronization.
    /// assert_eq!(AtomicCell::<[u8; 1000]>::is_lock_free(), false);
    /// ```
    pub fn is_lock_free() -> bool {
        atomic_is_lock_free::<T>()
    }
    /// Stores `val` into the atomic cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(7);
    ///
    /// assert_eq!(a.load(), 7);
    /// a.store(8);
    /// assert_eq!(a.load(), 8);
    /// ```
    pub fn store(&self, val: T) {
        if mem::needs_drop::<T>() {
            drop(self.swap(val));
        } else {
            unsafe {
                atomic_store(self.value.get(), val);
            }
        }
    }
    /// Stores `val` into the atomic cell and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(7);
    ///
    /// assert_eq!(a.load(), 7);
    /// assert_eq!(a.swap(8), 7);
    /// assert_eq!(a.load(), 8);
    /// ```
    pub fn swap(&self, val: T) -> T {
        unsafe { atomic_swap(self.value.get(), val) }
    }
}
impl<T: ?Sized> AtomicCell<T> {
    /// Returns a raw pointer to the underlying data in this atomic cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(5);
    ///
    /// let ptr = a.as_ptr();
    /// ```
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}
impl<T: Default> AtomicCell<T> {
    /// Takes the value of the atomic cell, leaving `Default::default()` in its place.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(5);
    /// let five = a.take();
    ///
    /// assert_eq!(five, 5);
    /// assert_eq!(a.into_inner(), 0);
    /// ```
    pub fn take(&self) -> T {
        self.swap(Default::default())
    }
}
impl<T: Copy> AtomicCell<T> {
    /// Loads a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(7);
    ///
    /// assert_eq!(a.load(), 7);
    /// ```
    pub fn load(&self) -> T {
        unsafe { atomic_load(self.value.get()) }
    }
}
impl<T: Copy + Eq> AtomicCell<T> {
    /// If the current value equals `current`, stores `new` into the atomic cell.
    ///
    /// The return value is always the previous value. If it is equal to `current`, then the value
    /// was updated.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(1);
    ///
    /// assert_eq!(a.compare_and_swap(2, 3), 1);
    /// assert_eq!(a.load(), 1);
    ///
    /// assert_eq!(a.compare_and_swap(1, 2), 1);
    /// assert_eq!(a.load(), 2);
    /// ```
    pub fn compare_and_swap(&self, current: T, new: T) -> T {
        match self.compare_exchange(current, new) {
            Ok(v) => v,
            Err(v) => v,
        }
    }
    /// If the current value equals `current`, stores `new` into the atomic cell.
    ///
    /// The return value is a result indicating whether the new value was written and containing
    /// the previous value. On success this value is guaranteed to be equal to `current`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(1);
    ///
    /// assert_eq!(a.compare_exchange(2, 3), Err(1));
    /// assert_eq!(a.load(), 1);
    ///
    /// assert_eq!(a.compare_exchange(1, 2), Ok(1));
    /// assert_eq!(a.load(), 2);
    /// ```
    pub fn compare_exchange(&self, current: T, new: T) -> Result<T, T> {
        unsafe { atomic_compare_exchange_weak(self.value.get(), current, new) }
    }
}
macro_rules! impl_arithmetic {
    ($t:ty, $example:tt) => {
        impl AtomicCell <$t > { #[doc =
        " Increments the current value by `val` and returns the previous value."] #[doc =
        ""] #[doc = " The addition wraps on overflow."] #[doc = ""] #[doc =
        " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_add(3), 7);"] #[doc =
        " assert_eq!(a.load(), 10);"] #[doc = " ```"] #[inline] pub fn fetch_add(& self,
        val : $t) -> $t { if can_transmute::<$t, atomic::AtomicUsize > () { let a =
        unsafe { &* (self.value.get() as * const atomic::AtomicUsize) }; a.fetch_add(val
        as usize, Ordering::AcqRel) as $t } else { let _guard = lock(self.value.get() as
        usize).write(); let value = unsafe { & mut * (self.value.get()) }; let old = *
        value; * value = value.wrapping_add(val); old } } #[doc =
        " Decrements the current value by `val` and returns the previous value."] #[doc =
        ""] #[doc = " The subtraction wraps on overflow."] #[doc = ""] #[doc =
        " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_sub(3), 7);"] #[doc =
        " assert_eq!(a.load(), 4);"] #[doc = " ```"] #[inline] pub fn fetch_sub(& self,
        val : $t) -> $t { if can_transmute::<$t, atomic::AtomicUsize > () { let a =
        unsafe { &* (self.value.get() as * const atomic::AtomicUsize) }; a.fetch_sub(val
        as usize, Ordering::AcqRel) as $t } else { let _guard = lock(self.value.get() as
        usize).write(); let value = unsafe { & mut * (self.value.get()) }; let old = *
        value; * value = value.wrapping_sub(val); old } } #[doc =
        " Applies bitwise \"and\" to the current value and returns the previous value."]
        #[doc = ""] #[doc = " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_and(3), 7);"] #[doc =
        " assert_eq!(a.load(), 3);"] #[doc = " ```"] #[inline] pub fn fetch_and(& self,
        val : $t) -> $t { if can_transmute::<$t, atomic::AtomicUsize > () { let a =
        unsafe { &* (self.value.get() as * const atomic::AtomicUsize) }; a.fetch_and(val
        as usize, Ordering::AcqRel) as $t } else { let _guard = lock(self.value.get() as
        usize).write(); let value = unsafe { & mut * (self.value.get()) }; let old = *
        value; * value &= val; old } } #[doc =
        " Applies bitwise \"or\" to the current value and returns the previous value."]
        #[doc = ""] #[doc = " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_or(16), 7);"] #[doc =
        " assert_eq!(a.load(), 23);"] #[doc = " ```"] #[inline] pub fn fetch_or(& self,
        val : $t) -> $t { if can_transmute::<$t, atomic::AtomicUsize > () { let a =
        unsafe { &* (self.value.get() as * const atomic::AtomicUsize) }; a.fetch_or(val
        as usize, Ordering::AcqRel) as $t } else { let _guard = lock(self.value.get() as
        usize).write(); let value = unsafe { & mut * (self.value.get()) }; let old = *
        value; * value |= val; old } } #[doc =
        " Applies bitwise \"xor\" to the current value and returns the previous value."]
        #[doc = ""] #[doc = " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_xor(2), 7);"] #[doc =
        " assert_eq!(a.load(), 5);"] #[doc = " ```"] #[inline] pub fn fetch_xor(& self,
        val : $t) -> $t { if can_transmute::<$t, atomic::AtomicUsize > () { let a =
        unsafe { &* (self.value.get() as * const atomic::AtomicUsize) }; a.fetch_xor(val
        as usize, Ordering::AcqRel) as $t } else { let _guard = lock(self.value.get() as
        usize).write(); let value = unsafe { & mut * (self.value.get()) }; let old = *
        value; * value ^= val; old } } }
    };
    ($t:ty, $atomic:ty, $example:tt) => {
        impl AtomicCell <$t > { #[doc =
        " Increments the current value by `val` and returns the previous value."] #[doc =
        ""] #[doc = " The addition wraps on overflow."] #[doc = ""] #[doc =
        " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_add(3), 7);"] #[doc =
        " assert_eq!(a.load(), 10);"] #[doc = " ```"] #[inline] pub fn fetch_add(& self,
        val : $t) -> $t { let a = unsafe { &* (self.value.get() as * const $atomic) }; a
        .fetch_add(val, Ordering::AcqRel) } #[doc =
        " Decrements the current value by `val` and returns the previous value."] #[doc =
        ""] #[doc = " The subtraction wraps on overflow."] #[doc = ""] #[doc =
        " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_sub(3), 7);"] #[doc =
        " assert_eq!(a.load(), 4);"] #[doc = " ```"] #[inline] pub fn fetch_sub(& self,
        val : $t) -> $t { let a = unsafe { &* (self.value.get() as * const $atomic) }; a
        .fetch_sub(val, Ordering::AcqRel) } #[doc =
        " Applies bitwise \"and\" to the current value and returns the previous value."]
        #[doc = ""] #[doc = " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_and(3), 7);"] #[doc =
        " assert_eq!(a.load(), 3);"] #[doc = " ```"] #[inline] pub fn fetch_and(& self,
        val : $t) -> $t { let a = unsafe { &* (self.value.get() as * const $atomic) }; a
        .fetch_and(val, Ordering::AcqRel) } #[doc =
        " Applies bitwise \"or\" to the current value and returns the previous value."]
        #[doc = ""] #[doc = " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_or(16), 7);"] #[doc =
        " assert_eq!(a.load(), 23);"] #[doc = " ```"] #[inline] pub fn fetch_or(& self,
        val : $t) -> $t { let a = unsafe { &* (self.value.get() as * const $atomic) }; a
        .fetch_or(val, Ordering::AcqRel) } #[doc =
        " Applies bitwise \"xor\" to the current value and returns the previous value."]
        #[doc = ""] #[doc = " # Examples"] #[doc = ""] #[doc = " ```"] #[doc =
        " use crossbeam_utils::atomic::AtomicCell;"] #[doc = ""] #[doc = $example] #[doc
        = ""] #[doc = " assert_eq!(a.fetch_xor(2), 7);"] #[doc =
        " assert_eq!(a.load(), 5);"] #[doc = " ```"] #[inline] pub fn fetch_xor(& self,
        val : $t) -> $t { let a = unsafe { &* (self.value.get() as * const $atomic) }; a
        .fetch_xor(val, Ordering::AcqRel) } }
    };
}
#[cfg(has_atomic_u8)]
impl_arithmetic!(u8, atomic::AtomicU8, "let a = AtomicCell::new(7u8);");
#[cfg(has_atomic_u8)]
impl_arithmetic!(i8, atomic::AtomicI8, "let a = AtomicCell::new(7i8);");
#[cfg(has_atomic_u16)]
impl_arithmetic!(u16, atomic::AtomicU16, "let a = AtomicCell::new(7u16);");
#[cfg(has_atomic_u16)]
impl_arithmetic!(i16, atomic::AtomicI16, "let a = AtomicCell::new(7i16);");
#[cfg(has_atomic_u32)]
impl_arithmetic!(u32, atomic::AtomicU32, "let a = AtomicCell::new(7u32);");
#[cfg(has_atomic_u32)]
impl_arithmetic!(i32, atomic::AtomicI32, "let a = AtomicCell::new(7i32);");
#[cfg(has_atomic_u64)]
impl_arithmetic!(u64, atomic::AtomicU64, "let a = AtomicCell::new(7u64);");
#[cfg(has_atomic_u64)]
impl_arithmetic!(i64, atomic::AtomicI64, "let a = AtomicCell::new(7i64);");
#[cfg(has_atomic_u128)]
impl_arithmetic!(u128, atomic::AtomicU128, "let a = AtomicCell::new(7u128);");
#[cfg(has_atomic_u128)]
impl_arithmetic!(i128, atomic::AtomicI128, "let  a = AtomicCell::new(7i128);");
impl_arithmetic!(usize, atomic::AtomicUsize, "let a = AtomicCell::new(7usize);");
impl_arithmetic!(isize, atomic::AtomicIsize, "let a = AtomicCell::new(7isize);");
impl AtomicCell<bool> {
    /// Applies logical "and" to the current value and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(true);
    ///
    /// assert_eq!(a.fetch_and(true), true);
    /// assert_eq!(a.load(), true);
    ///
    /// assert_eq!(a.fetch_and(false), true);
    /// assert_eq!(a.load(), false);
    /// ```
    #[inline]
    pub fn fetch_and(&self, val: bool) -> bool {
        let a = unsafe { &*(self.value.get() as *const AtomicBool) };
        a.fetch_and(val, Ordering::AcqRel)
    }
    /// Applies logical "or" to the current value and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(false);
    ///
    /// assert_eq!(a.fetch_or(false), false);
    /// assert_eq!(a.load(), false);
    ///
    /// assert_eq!(a.fetch_or(true), false);
    /// assert_eq!(a.load(), true);
    /// ```
    #[inline]
    pub fn fetch_or(&self, val: bool) -> bool {
        let a = unsafe { &*(self.value.get() as *const AtomicBool) };
        a.fetch_or(val, Ordering::AcqRel)
    }
    /// Applies logical "xor" to the current value and returns the previous value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::atomic::AtomicCell;
    ///
    /// let a = AtomicCell::new(true);
    ///
    /// assert_eq!(a.fetch_xor(false), true);
    /// assert_eq!(a.load(), true);
    ///
    /// assert_eq!(a.fetch_xor(true), true);
    /// assert_eq!(a.load(), false);
    /// ```
    #[inline]
    pub fn fetch_xor(&self, val: bool) -> bool {
        let a = unsafe { &*(self.value.get() as *const AtomicBool) };
        a.fetch_xor(val, Ordering::AcqRel)
    }
}
impl<T: Default> Default for AtomicCell<T> {
    fn default() -> AtomicCell<T> {
        AtomicCell::new(T::default())
    }
}
impl<T: Copy + fmt::Debug> fmt::Debug for AtomicCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AtomicCell").field("value", &self.load()).finish()
    }
}
/// Returns `true` if values of type `A` can be transmuted into values of type `B`.
fn can_transmute<A, B>() -> bool {
    mem::size_of::<A>() == mem::size_of::<B>()
        && mem::align_of::<A>() >= mem::align_of::<B>()
}
/// Returns a reference to the global lock associated with the `AtomicCell` at address `addr`.
///
/// This function is used to protect atomic data which doesn't fit into any of the primitive atomic
/// types in `std::sync::atomic`. Operations on such atomics must therefore use a global lock.
///
/// However, there is not only one global lock but an array of many locks, and one of them is
/// picked based on the given address. Having many locks reduces contention and improves
/// scalability.
#[inline]
#[must_use]
fn lock(addr: usize) -> &'static SeqLock {
    const LEN: usize = 97;
    static LOCKS: [SeqLock; LEN] = [
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
        SeqLock::new(),
    ];
    &LOCKS[addr % LEN]
}
/// An atomic `()`.
///
/// All operations are noops.
struct AtomicUnit;
impl AtomicUnit {
    #[inline]
    fn load(&self, _order: Ordering) {}
    #[inline]
    fn store(&self, _val: (), _order: Ordering) {}
    #[inline]
    fn swap(&self, _val: (), _order: Ordering) {}
    #[inline]
    fn compare_exchange_weak(
        &self,
        _current: (),
        _new: (),
        _success: Ordering,
        _failure: Ordering,
    ) -> Result<(), ()> {
        Ok(())
    }
}
macro_rules! atomic {
    (@ check, $t:ty, $atomic:ty, $a:ident, $atomic_op:expr) => {
        if can_transmute::<$t, $atomic > () { let $a : &$atomic; break $atomic_op; }
    };
    ($t:ty, $a:ident, $atomic_op:expr, $fallback_op:expr) => {
        loop { atomic!(@ check, $t, AtomicUnit, $a, $atomic_op); atomic!(@ check, $t,
        atomic::AtomicUsize, $a, $atomic_op); #[cfg(has_atomic_u8)] atomic!(@ check, $t,
        atomic::AtomicU8, $a, $atomic_op); #[cfg(has_atomic_u16)] atomic!(@ check, $t,
        atomic::AtomicU16, $a, $atomic_op); #[cfg(has_atomic_u32)] atomic!(@ check, $t,
        atomic::AtomicU32, $a, $atomic_op); #[cfg(has_atomic_u64)] atomic!(@ check, $t,
        atomic::AtomicU64, $a, $atomic_op); break $fallback_op; }
    };
}
/// Returns `true` if operations on `AtomicCell<T>` are lock-free.
fn atomic_is_lock_free<T>() -> bool {
    atomic! {
        T, _a, true, false
    }
}
/// Atomically reads data from `src`.
///
/// This operation uses the `Acquire` ordering. If possible, an atomic instructions is used, and a
/// global lock otherwise.
unsafe fn atomic_load<T>(src: *mut T) -> T
where
    T: Copy,
{
    atomic! {
        T, a, { a = &* (src as * const _ as * const _); mem::transmute_copy(& a
        .load(Ordering::Acquire)) }, { let lock = lock(src as usize); if let Some(stamp)
        = lock.optimistic_read() { let val = ptr::read_volatile(src); if lock
        .validate_read(stamp) { return val; } } let guard = lock.write(); let val =
        ptr::read(src); guard.abort(); val }
    }
}
/// Atomically writes `val` to `dst`.
///
/// This operation uses the `Release` ordering. If possible, an atomic instructions is used, and a
/// global lock otherwise.
unsafe fn atomic_store<T>(dst: *mut T, val: T) {
    atomic! {
        T, a, { a = &* (dst as * const _ as * const _); a.store(mem::transmute_copy(&
        val), Ordering::Release); mem::forget(val); }, { let _guard = lock(dst as usize)
        .write(); ptr::write(dst, val); }
    }
}
/// Atomically swaps data at `dst` with `val`.
///
/// This operation uses the `AcqRel` ordering. If possible, an atomic instructions is used, and a
/// global lock otherwise.
unsafe fn atomic_swap<T>(dst: *mut T, val: T) -> T {
    atomic! {
        T, a, { a = &* (dst as * const _ as * const _); let res = mem::transmute_copy(& a
        .swap(mem::transmute_copy(& val), Ordering::AcqRel)); mem::forget(val); res }, {
        let _guard = lock(dst as usize).write(); ptr::replace(dst, val) }
    }
}
/// Atomically compares data at `dst` to `current` and, if equal byte-for-byte, exchanges data at
/// `dst` with `new`.
///
/// Returns the old value on success, or the current value at `dst` on failure.
///
/// This operation uses the `AcqRel` ordering. If possible, an atomic instructions is used, and a
/// global lock otherwise.
unsafe fn atomic_compare_exchange_weak<T>(
    dst: *mut T,
    mut current: T,
    new: T,
) -> Result<T, T>
where
    T: Copy + Eq,
{
    atomic! {
        T, a, { a = &* (dst as * const _ as * const _); let mut current_raw =
        mem::transmute_copy(& current); let new_raw = mem::transmute_copy(& new); loop {
        match a.compare_exchange_weak(current_raw, new_raw, Ordering::AcqRel,
        Ordering::Acquire,) { Ok(_) => break Ok(current), Err(previous_raw) => { let
        previous = mem::transmute_copy(& previous_raw); if ! T::eq(& previous, & current)
        { break Err(previous); } current = previous; current_raw = previous_raw; } } } },
        { let guard = lock(dst as usize).write(); if T::eq(&* dst, & current) {
        Ok(ptr::replace(dst, new)) } else { let val = ptr::read(dst); guard.abort();
        Err(val) } }
    }
}
#[cfg(test)]
mod tests_rug_665 {
    use super::*;
    use std::mem;
    #[test]
    fn test_can_transmute() {
        let _rug_st_tests_rug_665_rrrruuuugggg_test_can_transmute = 0;
        let result = can_transmute::<i32, i32>();
        debug_assert_eq!(result, true);
        let _rug_ed_tests_rug_665_rrrruuuugggg_test_can_transmute = 0;
    }
}
#[cfg(test)]
mod tests_rug_666 {
    use super::*;
    use crate::atomic::atomic_cell::SeqLock;
    #[test]
    fn test_lock() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        crate::atomic::atomic_cell::lock(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_667 {
    use super::*;
    use crate::atomic::atomic_cell::atomic_is_lock_free;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_667_rrrruuuugggg_test_rug = 0;
        let _ = atomic_is_lock_free::<usize>();
        let _rug_ed_tests_rug_667_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_669 {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: *mut AtomicI32 = &mut AtomicI32::new(rug_fuzz_0) as *mut AtomicI32;
        let p1: AtomicI32 = AtomicI32::new(rug_fuzz_1);
        unsafe {
            crate::atomic::atomic_cell::atomic_store(p0, p1);
            debug_assert_eq!((* p0).load(Ordering::Acquire), 42);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_670 {
    use super::*;
    use std::sync::{Arc, Mutex};
    use crate::atomic::atomic_cell::atomic_swap;
    #[test]
    fn test_rug() {
        let mut p0: *mut _ = &mut 0 as *mut _;
        let mut p1: _ = 42;
        unsafe {
            atomic_swap(p0, p1);
        }
    }
}
#[cfg(test)]
mod tests_rug_672 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i32 = rug_fuzz_0;
        AtomicCell::<i32>::new(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_673 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        <AtomicCell<i32>>::into_inner(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_674 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_is_lock_free() {
        let _rug_st_tests_rug_674_rrrruuuugggg_test_is_lock_free = 0;
        debug_assert_eq!(AtomicCell:: < usize > ::is_lock_free(), true);
        struct Foo {
            bar: isize,
        }
        debug_assert_eq!(AtomicCell:: < Foo > ::is_lock_free(), true);
        debug_assert_eq!(AtomicCell:: < () > ::is_lock_free(), true);
        debug_assert_eq!(AtomicCell:: < [u8; 1000] > ::is_lock_free(), false);
        let _rug_ed_tests_rug_674_rrrruuuugggg_test_is_lock_free = 0;
    }
}
#[cfg(test)]
mod tests_rug_675 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let val = rug_fuzz_0;
        let a = AtomicCell::new(val);
        let new_val = rug_fuzz_1;
        AtomicCell::<i32>::store(&a, new_val);
        debug_assert_eq!(a.load(), new_val);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_676 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_swap() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::new(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        debug_assert_eq!(p0.load(), 7);
        debug_assert_eq!(p0.swap(p1), 7);
        debug_assert_eq!(p0.load(), 8);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_677 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        p0.as_ptr();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_678 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        AtomicCell::<i32>::take(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_679 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = AtomicCell::<i32>::new(rug_fuzz_0);
        debug_assert_eq!(< AtomicCell < i32 > > ::load(& p0), 42);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_680 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        let mut p1: i32 = rug_fuzz_1;
        let mut p2: i32 = rug_fuzz_2;
        p0.compare_and_swap(p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_681 {
    use super::*;
    use crate::atomic::AtomicCell;
    use crate::cache_padded::CachePadded;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        let mut p1: i32 = rug_fuzz_1;
        let mut p2: i32 = rug_fuzz_2;
        p0.compare_exchange(p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_682 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u8>::new(rug_fuzz_0);
        let mut p1: u8 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_add(p1), 5);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_683 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = AtomicCell::<u8>::new(rug_fuzz_0);
        let val_to_subtract: u8 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < u8 > ::fetch_sub(& a, val_to_subtract), 10);
        debug_assert_eq!(a.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_684 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_and() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = AtomicCell::<u8>::new(rug_fuzz_0);
        let val: u8 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < u8 > ::fetch_and(& a, val), 10);
        debug_assert_eq!(a.load(), 2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_685 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u8>::new(rug_fuzz_0);
        let mut p1: u8 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_or(p1), 10);
        debug_assert_eq!(p0.load(), 11);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_686 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_xor() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u8>::new(rug_fuzz_0);
        let p1: u8 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 5);
        debug_assert_eq!(p0.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_687 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i8, i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<i8>::new(rug_fuzz_0);
        let mut p1: i8 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_add(p1), 5);
        debug_assert_eq!(p0.load(), 8);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_688 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i8, i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<i8>::new(rug_fuzz_0);
        let p1: i8 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_sub(p1), 10);
        debug_assert_eq!(p0.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_689 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i8, i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<i8>::new(rug_fuzz_0);
        let mut p1: i8 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_and(p1), 5);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_690 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_fetch_or() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i8, i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = AtomicCell::<i8>::new(rug_fuzz_0);
        let val = rug_fuzz_1;
        debug_assert_eq!(< AtomicCell < i8 > > ::fetch_or(& a, val), 7);
        debug_assert_eq!(a.load(), 23);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_691 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_xor() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i8, i8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<i8>::new(rug_fuzz_0);
        let p1: i8 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 5);
        debug_assert_eq!(p0.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_692 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u16>::new(rug_fuzz_0);
        let mut p1: u16 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < u16 > ::fetch_add(& p0, p1), 5);
        debug_assert_eq!(p0.load(), 8);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_693 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u16>::new(rug_fuzz_0);
        let p1: u16 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_sub(p1), 10);
        debug_assert_eq!(p0.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_694 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_and() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u16>::new(rug_fuzz_0);
        let p1: u16 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_and(p1), 10);
        debug_assert_eq!(p0.load(), 2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_695 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_or() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u16>::new(rug_fuzz_0);
        let p1: u16 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_or(p1), 5);
        debug_assert_eq!(p0.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_696 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u16, u16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u16>::new(rug_fuzz_0);
        let p1: u16 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_697 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i16, i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<i16>::new(rug_fuzz_0);
        let p1: i16 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_add(p1), 5);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_698 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i16, i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut atomic_cell_value = AtomicCell::<i16>::new(rug_fuzz_0);
        let val: i16 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < i16 > ::fetch_sub(& atomic_cell_value, val), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_699 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_and() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i16, i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<i16>::new(rug_fuzz_0);
        let p1: i16 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_and(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_700 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i16, i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<i16>::new(rug_fuzz_0);
        let mut p1: i16 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_or(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_701 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_xor() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i16, i16) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<i16>::new(rug_fuzz_0);
        let p1: i16 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 5);
        debug_assert_eq!(p0.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_702 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u32>::new(rug_fuzz_0);
        let p1: u32 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_add(p1), 5);
        debug_assert_eq!(p0.load(), 8);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_703 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u32>::new(rug_fuzz_0);
        let p1: u32 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_sub(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_704 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_fetch_and() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u32>::new(rug_fuzz_0);
        let p1: u32 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_and(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_705 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_fetch_or() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u32>::new(rug_fuzz_0);
        let mut p1: u32 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < u32 > ::fetch_or(& p0, p1), 5);
        debug_assert_eq!(p0.load(), 21);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_706 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u32>::new(rug_fuzz_0);
        let p1: u32 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 5);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_707 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        let p1: i32 = rug_fuzz_1;
        p0.fetch_add(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_708 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        let p1: i32 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_sub(p1), 10);
        debug_assert_eq!(p0.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_709 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        let p1: i32 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_and(p1), 5);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_710 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        let p1: i32 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_or(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_711 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: AtomicCell<i32> = AtomicCell::new(rug_fuzz_0);
        let p1: i32 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_712 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u64>::new(rug_fuzz_0);
        let p1: u64 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_add(p1), 5);
        debug_assert_eq!(p0.load(), 8);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_713 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_fetch_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = AtomicCell::new(rug_fuzz_0);
        let val = rug_fuzz_1;
        debug_assert_eq!(a.fetch_sub(val), 10);
        debug_assert_eq!(a.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_714 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_and() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u64>::new(rug_fuzz_0);
        let p1: u64 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_and(p1), 5);
        debug_assert_eq!(p0.load(), 1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_715 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_or() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u64>::new(rug_fuzz_0);
        let p1: u64 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_or(p1), 7);
        debug_assert_eq!(p0.load(), 23);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_716 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_xor() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<u64>::new(rug_fuzz_0);
        let mut p1: u64 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 5);
        debug_assert_eq!(p0.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_717 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i64, i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::new(rug_fuzz_0);
        let mut p1: i64 = rug_fuzz_1;
        crate::atomic::atomic_cell::AtomicCell::<i64>::fetch_add(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_718 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i64, i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v205 = AtomicCell::new(rug_fuzz_0);
        let p0 = &v205;
        let p1: i64 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < i64 > ::fetch_sub(p0, p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_719 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_and() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i64, i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::new(rug_fuzz_0);
        let p1: i64 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < i64 > ::fetch_and(& p0, p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_721 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i64, i64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v205 = AtomicCell::new(rug_fuzz_0);
        let p0 = &v205;
        let p1: i64 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < i64 > ::fetch_xor(p0, p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_722 {
    use super::*;
    use crate::atomic::atomic_cell::AtomicCell;
    #[test]
    fn test_fetch_add() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v206 = AtomicCell::new(rug_fuzz_0);
        let p0 = &v206;
        let p1: usize = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < usize > ::fetch_add(p0, p1), 42);
        debug_assert_eq!(v206.load(), 52);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_723 {
    use super::*;
    use crate::atomic::atomic_cell::AtomicCell;
    #[test]
    fn test_fetch_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_sub(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_724 {
    use super::*;
    use crate::atomic::atomic_cell::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<usize>::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_and(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_725 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<usize>::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < usize > ::fetch_or(& p0, p1), 7);
        debug_assert_eq!(p0.load(), 23);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_726 {
    use super::*;
    use crate::atomic::atomic_cell::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<usize>::new(rug_fuzz_0);
        let p1: usize = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_727 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let value = AtomicCell::<isize>::new(rug_fuzz_0);
        let p0 = value;
        let p1: isize = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < isize > ::fetch_add(& p0, p1), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_728 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_sub() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = AtomicCell::<isize>::new(rug_fuzz_0);
        let val = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < isize > ::fetch_sub(& a, val), 10);
        debug_assert_eq!(a.load(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_729 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<isize>::new(rug_fuzz_0);
        let p1: isize = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_and(p1), 5);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_730 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_fetch_or() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = AtomicCell::<isize>::new(rug_fuzz_0);
        let p1: isize = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_or(p1), 10);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_731 {
    use super::*;
    use crate::atomic::AtomicCell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AtomicCell::<isize>::new(rug_fuzz_0);
        let mut p1 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), 5);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_732 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::{AtomicBool, Ordering};
    #[test]
    fn test_fetch_and() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(bool, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = AtomicCell::new(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < bool > ::fetch_and(& p0, p1), true);
        debug_assert_eq!(p0.load(), true);
        debug_assert_eq!(AtomicCell:: < bool > ::fetch_and(& p0, p1), true);
        debug_assert_eq!(p0.load(), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_733 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::{AtomicBool, Ordering};
    #[test]
    fn test_fetch_or() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(bool, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let cell = AtomicCell::<bool>::new(rug_fuzz_0);
        let val = rug_fuzz_1;
        debug_assert_eq!(AtomicCell:: < bool > ::fetch_or(& cell, val), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_734 {
    use super::*;
    use crate::atomic::AtomicCell;
    use std::sync::atomic::{AtomicBool, Ordering};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(bool, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = AtomicCell::<bool>::new(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        debug_assert_eq!(p0.fetch_xor(p1), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_735 {
    use super::*;
    use crate::atomic::atomic_cell::AtomicCell;
    use std::default::Default;
    #[test]
    fn test_default() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let default_value: i32 = rug_fuzz_0;
        let atomic_cell_default_value = AtomicCell::<i32>::new(default_value);
        let result = <AtomicCell<i32> as Default>::default();
        debug_assert_eq!(result.load(), atomic_cell_default_value.load());
             }
}
}
}    }
}
