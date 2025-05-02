//! The default garbage collector.
//!
//! For each thread, a participant is lazily initialized on its first use, when the current thread
//! is registered in the default collector.  If initialized, the thread's participant will get
//! destructed on thread exit, which in turn unregisters the thread.

use crate::collector::{Collector, LocalHandle};
use crate::guard::Guard;
use lazy_static::lazy_static;

lazy_static! {
    /// The global data for the default garbage collector.
    static ref COLLECTOR: Collector = Collector::new();
}

thread_local! {
    /// The per-thread participant for the default garbage collector.
    static HANDLE: LocalHandle = COLLECTOR.register();
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
    HANDLE
        .try_with(|h| f(h))
        .unwrap_or_else(|_| f(&COLLECTOR.register()))
}

#[cfg(test)]
mod tests {
    use crossbeam_utils::thread;

    #[test]
    fn pin_while_exiting() {
        struct Foo;

        impl Drop for Foo {
            fn drop(&mut self) {
                // Pin after `HANDLE` has been dropped. This must not panic.
                super::pin();
            }
        }

        thread_local! {
            static FOO: Foo = Foo;
        }

        thread::scope(|scope| {
            scope.spawn(|_| {
                // Initialize `FOO` and then `HANDLE`.
                FOO.with(|_| ());
                super::pin();
                // At thread exit, `HANDLE` gets dropped first and `FOO` second.
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
        pin();
    }
}#[cfg(test)]
mod tests_rug_459 {
    use super::*;
    use crate::default::with_handle;

    #[test]
    fn test_is_pinned() {
        let result = is_pinned();
        assert_eq!(result, false); // Assuming false is the default value if not pinned
    }
}#[cfg(test)]
mod tests_rug_460 {
    use super::*;
    use crate::Collector;
    use crate::default::COLLECTOR;

    #[test]
    fn test_rug() {
        let collector: &'static Collector = default_collector();
        // Add your assertions here
    }
}