use std::ptr::NonNull;
/// Return a NonNull<T> pointer to the vector's data
pub(crate) fn nonnull_from_vec_data<T>(v: &mut Vec<T>) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(v.as_mut_ptr()) }
}
/// Converts `ptr` to `NonNull<T>`
///
/// Safety: `ptr` *must* be non-null.
/// This is checked with a debug assertion, and will panic if this is not true,
/// but treat this as an unconditional conversion.
#[inline]
pub(crate) unsafe fn nonnull_debug_checked_from_ptr<T>(ptr: *mut T) -> NonNull<T> {
    debug_assert!(! ptr.is_null());
    NonNull::new_unchecked(ptr)
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use std::vec::Vec;
    use std::ptr::NonNull;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_88_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let mut p0: Vec<i32> = vec![rug_fuzz_0, 2, 3];
        crate::extension::nonnull::nonnull_from_vec_data(&mut p0);
        let _rug_ed_tests_rug_88_rrrruuuugggg_test_rug = 0;
    }
}
