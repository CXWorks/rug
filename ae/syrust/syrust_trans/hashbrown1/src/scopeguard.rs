use core::ops::{Deref, DerefMut};
pub struct ScopeGuard<T, F>
where
    F: FnMut(&mut T),
{
    dropfn: F,
    value: T,
}
#[cfg_attr(feature = "inline-more", inline)]
pub fn guard<T, F>(value: T, dropfn: F) -> ScopeGuard<T, F>
where
    F: FnMut(&mut T),
{
    ScopeGuard { dropfn, value }
}
impl<T, F> Deref for ScopeGuard<T, F>
where
    F: FnMut(&mut T),
{
    type Target = T;
    #[cfg_attr(feature = "inline-more", inline)]
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T, F> DerefMut for ScopeGuard<T, F>
where
    F: FnMut(&mut T),
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
impl<T, F> Drop for ScopeGuard<T, F>
where
    F: FnMut(&mut T),
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn drop(&mut self) {
        (self.dropfn)(&mut self.value)
    }
}
#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::scopeguard::guard;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_233_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 1;
        let mut p0: i32 = rug_fuzz_0;
        let mut p1 = |value: &mut i32| {
            *value += rug_fuzz_1;
        };
        guard(p0, p1);
        let _rug_ed_tests_rug_233_rrrruuuugggg_test_rug = 0;
    }
}
