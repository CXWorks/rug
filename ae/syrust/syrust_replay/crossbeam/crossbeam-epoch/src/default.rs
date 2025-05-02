//! The default garbage collector.
//!
//! For each thread, a participant is lazily initialized on its first use, when the current thread
//! is registered in the default collector.  If initialized, the thread's participant will get
//! destructed on thread exit, which in turn unregisters the thread.
use crate::collector::{Collector, LocalHandle};
use crate::guard::Guard;
use lazy_static::lazy_static;
lazy_static! {
    #[doc = " The global data for the default garbage collector."] static ref COLLECTOR :
    Collector = Collector::new();
}
thread_local! {
    #[doc = " The per-thread participant for the default garbage collector."] static
    HANDLE : LocalHandle = COLLECTOR.register();
}
/// Pins the current thread.
#[inline]
pub fn pin() -> Guard {
    with_handle(|handle| handle.pin())
}
/// Returns `true` if the current thread is pinned.
#[inline]
pub fn is_pinned() -> bool {
    with_handle(|handle| handle.is_pinned())
}
/// Returns the default global collector.
pub fn default_collector() -> &'static Collector {
    &COLLECTOR
}
#[inline]
fn with_handle<F, R>(mut f: F) -> R
where
    F: FnMut(&LocalHandle) -> R,
{
    HANDLE.try_with(|h| f(h)).unwrap_or_else(|_| f(&COLLECTOR.register()))
}
#[cfg(test)]
mod tests {
    use crossbeam_utils::thread;
    #[test]
    fn pin_while_exiting() {
        struct Foo;
        impl Drop for Foo {
            fn drop(&mut self) {
                super::pin();
            }
        }
        thread_local! {
            static FOO : Foo = Foo;
        }
        thread::scope(|scope| {
                scope
                    .spawn(|_| {
                        FOO.with(|_| ());
                        super::pin();
                    });
            })
            .unwrap();
    }
}
#[cfg(test)]
mod tests_rug_458 {
    use super::*;
    use crate::default::pin;
    use crate::Guard;
    use crate::unprotected;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_458_rrrruuuugggg_test_rug = 0;
        pin();
        let _rug_ed_tests_rug_458_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_459 {
    use super::*;
    use crate::default::with_handle;
    #[test]
    fn test_is_pinned() {
        let _rug_st_tests_rug_459_rrrruuuugggg_test_is_pinned = 0;
        let result = is_pinned();
        debug_assert_eq!(result, false);
        let _rug_ed_tests_rug_459_rrrruuuugggg_test_is_pinned = 0;
    }
}
#[cfg(test)]
mod tests_rug_460 {
    use super::*;
    use crate::Collector;
    use crate::default::COLLECTOR;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_460_rrrruuuugggg_test_rug = 0;
        let collector: &'static Collector = default_collector();
        let _rug_ed_tests_rug_460_rrrruuuugggg_test_rug = 0;
    }
}
