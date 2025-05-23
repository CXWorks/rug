// Extracted from the scopeguard crate
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
        let mut p0: i32 = 42;
        let mut p1 = |value: &mut i32| {
            *value += 1;
        };
        
        guard(p0, p1);
    }
}