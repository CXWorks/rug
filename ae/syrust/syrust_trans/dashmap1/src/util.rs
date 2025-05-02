//! This module is full of hackery and dark magic.
//! Either spend a day fixing it and quietly submit a PR or don't mention it to anybody.
use core::cell::UnsafeCell;
use core::{mem, ptr};
pub const fn ptr_size_bits() -> usize {
    mem::size_of::<usize>() * 8
}
pub fn map_in_place_2<T, U, F: FnOnce(U, T) -> T>((k, v): (U, &mut T), f: F) {
    unsafe {
        let _promote_panic_to_abort = AbortOnPanic;
        ptr::write(v, f(k, ptr::read(v)));
    }
}
/// # Safety
///
/// Requires that you ensure the reference does not become invalid.
/// The object has to outlive the reference.
pub unsafe fn change_lifetime_const<'a, 'b, T>(x: &'a T) -> &'b T {
    &*(x as *const T)
}
/// # Safety
///
/// Requires that you ensure the reference does not become invalid.
/// The object has to outlive the reference.
pub unsafe fn change_lifetime_mut<'a, 'b, T>(x: &'a mut T) -> &'b mut T {
    &mut *(x as *mut T)
}
/// A simple wrapper around `T`
///
/// This is to prevent UB when using `HashMap::get_key_value`, because
/// `HashMap` doesn't expose an api to get the key and value, where
/// the value is a `&mut T`.
///
/// See [#10](https://github.com/xacrimon/dashmap/issues/10) for details
///
/// This type is meant to be an implementation detail, but must be exposed due to the `Dashmap::shards`
#[repr(transparent)]
pub struct SharedValue<T> {
    value: UnsafeCell<T>,
}
impl<T: Clone> Clone for SharedValue<T> {
    fn clone(&self) -> Self {
        let inner = self.get().clone();
        Self {
            value: UnsafeCell::new(inner),
        }
    }
}
unsafe impl<T: Send> Send for SharedValue<T> {}
unsafe impl<T: Sync> Sync for SharedValue<T> {}
impl<T> SharedValue<T> {
    /// Create a new `SharedValue<T>`
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }
    /// Get a shared reference to `T`
    pub fn get(&self) -> &T {
        unsafe { &*self.value.get() }
    }
    /// Get an unique reference to `T`
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }
    /// Unwraps the value
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
    /// Get a mutable raw pointer to the underlying value
    pub(crate) fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}
struct AbortOnPanic;
impl Drop for AbortOnPanic {
    fn drop(&mut self) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "no_std")] {} else { if std::thread::panicking() {
            std::process::abort() } }
        }
    }
}
#[cfg(test)]
mod tests_rug_28 {
    use super::*;
    use std::mem;
    #[test]
    fn test_ptr_size_bits() {
        let _rug_st_tests_rug_28_rrrruuuugggg_test_ptr_size_bits = 0;
        debug_assert_eq!(ptr_size_bits(), mem::size_of:: < usize > () * 8);
        let _rug_ed_tests_rug_28_rrrruuuugggg_test_ptr_size_bits = 0;
    }
}
#[cfg(test)]
mod tests_rug_30 {
    use super::*;
    #[test]
    fn test_change_lifetime_const() {
        let _rug_st_tests_rug_30_rrrruuuugggg_test_change_lifetime_const = 0;
        let rug_fuzz_0 = 42;
        let num = rug_fuzz_0;
        unsafe {
            crate::util::change_lifetime_const(&num);
        };
        let _rug_ed_tests_rug_30_rrrruuuugggg_test_change_lifetime_const = 0;
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::util::SharedValue;
    use std::clone::Clone;
    use std::cell::UnsafeCell;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let value = rug_fuzz_0;
        let inner_cell = UnsafeCell::new(value);
        let shared_value = SharedValue { value: inner_cell };
        shared_value.clone();
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::util::SharedValue;
    use std::cell::UnsafeCell;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_33_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: i32 = rug_fuzz_0;
        SharedValue::<i32>::new(p0);
        let _rug_ed_tests_rug_33_rrrruuuugggg_test_rug = 0;
    }
}
use super::*;
#[cfg(test)]
mod tests_rug_34 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_34_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let value: i32 = rug_fuzz_0;
        let shared_value = util::SharedValue::<i32>::new(value);
        debug_assert_eq!(* util::SharedValue:: < i32 > ::get(& shared_value), 42);
        let _rug_ed_tests_rug_34_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use crate::util::SharedValue;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_35_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: SharedValue<i32> = SharedValue::new(rug_fuzz_0);
        SharedValue::<i32>::get_mut(&mut p0);
        let _rug_ed_tests_rug_35_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    use crate::util::SharedValue;
    #[test]
    fn test_into_inner() {
        let _rug_st_tests_rug_36_rrrruuuugggg_test_into_inner = 0;
        let rug_fuzz_0 = 42;
        let value = rug_fuzz_0;
        let shared_value = SharedValue::new(value);
        let inner_value = shared_value.into_inner();
        debug_assert_eq!(inner_value, 42);
        let _rug_ed_tests_rug_36_rrrruuuugggg_test_into_inner = 0;
    }
}
#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::util::SharedValue;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_37_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: SharedValue<u32> = SharedValue::new(rug_fuzz_0);
        p0.as_ptr();
        let _rug_ed_tests_rug_37_rrrruuuugggg_test_rug = 0;
    }
}
