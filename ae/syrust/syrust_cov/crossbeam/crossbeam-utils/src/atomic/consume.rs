#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
use core::sync::atomic::compiler_fence;
use core::sync::atomic::Ordering;

/// Trait which allows reading from primitive atomic types with "consume" ordering.
pub trait AtomicConsume {
    /// Type returned by `load_consume`.
    type Val;

    /// Loads a value from the atomic using a "consume" memory ordering.
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
    /// This is currently only implemented on ARM and AArch64, where a fence
    /// can be avoided. On other architectures this will fall back to a simple
    /// `load(Ordering::Acquire)`.
    fn load_consume(&self) -> Self::Val;
}

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
macro_rules! impl_consume {
    () => {
        #[inline]
        fn load_consume(&self) -> Self::Val {
            let result = self.load(Ordering::Relaxed);
            compiler_fence(Ordering::Acquire);
            result
        }
    };
}

#[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
macro_rules! impl_consume {
    () => {
        #[inline]
        fn load_consume(&self) -> Self::Val {
            self.load(Ordering::Acquire)
        }
    };
}

macro_rules! impl_atomic {
    ($atomic:ident, $val:ty) => {
        impl AtomicConsume for ::core::sync::atomic::$atomic {
            type Val = $val;
            impl_consume!();
        }
    };
}

impl_atomic!(AtomicBool, bool);
impl_atomic!(AtomicUsize, usize);
impl_atomic!(AtomicIsize, isize);
#[cfg(all(feature = "nightly", target_has_atomic = "8"))]
impl_atomic!(AtomicU8, u8);
#[cfg(all(feature = "nightly", target_has_atomic = "8"))]
impl_atomic!(AtomicI8, i8);
#[cfg(all(feature = "nightly", target_has_atomic = "16"))]
impl_atomic!(AtomicU16, u16);
#[cfg(all(feature = "nightly", target_has_atomic = "16"))]
impl_atomic!(AtomicI16, i16);
#[cfg(all(feature = "nightly", target_has_atomic = "32"))]
impl_atomic!(AtomicU32, u32);
#[cfg(all(feature = "nightly", target_has_atomic = "32"))]
impl_atomic!(AtomicI32, i32);
#[cfg(all(feature = "nightly", target_has_atomic = "64"))]
impl_atomic!(AtomicU64, u64);
#[cfg(all(feature = "nightly", target_has_atomic = "64"))]
impl_atomic!(AtomicI64, i64);

impl<T> AtomicConsume for ::core::sync::atomic::AtomicPtr<T> {
    type Val = *mut T;
    impl_consume!();
}
#[cfg(test)]
mod tests_rug_766 {
    use crate::atomic::consume::AtomicConsume;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_rug() {
        let p0 = AtomicBool::new(true);
        <AtomicBool as AtomicConsume>::load_consume(&p0);
    }
}#[cfg(test)]
mod tests_rug_767 {
    use super::*;
    use crate::atomic::AtomicConsume;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    
    #[test]
    fn test_rug() {
        let mut p0 = AtomicUsize::new(10);            
        p0.load_consume();
    }
}
#[cfg(test)]
mod tests_rug_768 {
    use super::*;
    use crate::atomic::AtomicConsume;
    use std::sync::atomic::AtomicIsize;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_rug() {
        let mut p0: AtomicIsize = AtomicIsize::new(42);
        
        <AtomicIsize as AtomicConsume>::load_consume(&p0);
    }
}#[cfg(test)]
mod tests_rug_769 {
    use super::*;
    use crate::atomic::AtomicConsume;
    use std::sync::atomic::{AtomicPtr, Ordering};

    #[test]
    fn test_load_consume() {
        // Create a pointer to atomic data
        let data: i32 = 42;
        let p0 = AtomicPtr::new(&data as *const i32 as *mut i32); // Example data, it can be any data type

        <AtomicPtr<i32> as AtomicConsume>::load_consume(&p0);
    }
}