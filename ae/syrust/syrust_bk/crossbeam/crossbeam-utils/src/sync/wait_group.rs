// Necessary for using `Mutex<usize>` for conditional variables
#![allow(clippy::mutex_atomic)]

use std::fmt;
use std::sync::{Arc, Condvar, Mutex};

/// Enables threads to synchronize the beginning or end of some computation.
///
/// # Wait groups vs barriers
///
/// `WaitGroup` is very similar to [`Barrier`], but there are a few differences:
///
/// * `Barrier` needs to know the number of threads at construction, while `WaitGroup` is cloned to
///   register more threads.
///
/// * A `Barrier` can be reused even after all threads have synchronized, while a `WaitGroup`
///   synchronizes threads only once.
///
/// * All threads wait for others to reach the `Barrier`. With `WaitGroup`, each thread can choose
///   to either wait for other threads or to continue without blocking.
///
/// # Examples
///
/// ```
/// use crossbeam_utils::sync::WaitGroup;
/// use std::thread;
///
/// // Create a new wait group.
/// let wg = WaitGroup::new();
///
/// for _ in 0..4 {
///     // Create another reference to the wait group.
///     let wg = wg.clone();
///
///     thread::spawn(move || {
///         // Do some work.
///
///         // Drop the reference to the wait group.
///         drop(wg);
///     });
/// }
///
/// // Block until all threads have finished their work.
/// wg.wait();
/// ```
///
/// [`Barrier`]: https://doc.rust-lang.org/std/sync/struct.Barrier.html
pub struct WaitGroup {
    inner: Arc<Inner>,
}

/// Inner state of a `WaitGroup`.
struct Inner {
    cvar: Condvar,
    count: Mutex<usize>,
}

impl Default for WaitGroup {
    fn default() -> Self {
        Self {
            inner: Arc::new(Inner {
                cvar: Condvar::new(),
                count: Mutex::new(1),
            }),
        }
    }
}

impl WaitGroup {
    /// Creates a new wait group and returns the single reference to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::WaitGroup;
    ///
    /// let wg = WaitGroup::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops this reference and waits until all other references are dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::WaitGroup;
    /// use std::thread;
    ///
    /// let wg = WaitGroup::new();
    ///
    /// thread::spawn({
    ///     let wg = wg.clone();
    ///     move || {
    ///         // Block until both threads have reached `wait()`.
    ///         wg.wait();
    ///     }
    /// });
    ///
    /// // Block until both threads have reached `wait()`.
    /// wg.wait();
    /// ```
    pub fn wait(self) {
        if *self.inner.count.lock().unwrap() == 1 {
            return;
        }

        let inner = self.inner.clone();
        drop(self);

        let mut count = inner.count.lock().unwrap();
        while *count > 0 {
            count = inner.cvar.wait(count).unwrap();
        }
    }
}

impl Drop for WaitGroup {
    fn drop(&mut self) {
        let mut count = self.inner.count.lock().unwrap();
        *count -= 1;

        if *count == 0 {
            self.inner.cvar.notify_all();
        }
    }
}

impl Clone for WaitGroup {
    fn clone(&self) -> WaitGroup {
        let mut count = self.inner.count.lock().unwrap();
        *count += 1;

        WaitGroup {
            inner: self.inner.clone(),
        }
    }
}

impl fmt::Debug for WaitGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count: &usize = &*self.inner.count.lock().unwrap();
        f.debug_struct("WaitGroup").field("count", count).finish()
    }
}
#[cfg(test)]
mod tests_rug_800 {
    use super::*;
    use crate::sync::wait_group::WaitGroup;
    use std::sync::{Arc, Condvar, Mutex};
    
    #[test]
    fn test_default() {
        let _wg: WaitGroup = WaitGroup::default();
    }
}#[cfg(test)]
mod tests_rug_801 {
    use super::*;
    use crate::sync::WaitGroup;

    #[test]
    fn test_wait_group_new() {
        let wg = WaitGroup::new();
        // Add assertions or checks if needed
    }

}#[cfg(test)]
mod tests_rug_802 {
    use super::*;
    use crate::sync::WaitGroup;
    use std::thread;

    #[test]
    fn test_rug() {
        let mut wg = WaitGroup::new();

        thread::spawn({
            let wg_clone = wg.clone();
            move || {
                wg_clone.wait();
            }
        });

        wg.wait();
    }
}#[cfg(test)]
mod tests_rug_804 {
    use super::*;
    use crate::sync::WaitGroup;

    use std::clone::Clone;

    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_804_prepare {
            use crate::sync::WaitGroup;

            #[test]
            fn sample() {
                let mut v230 = WaitGroup::new();
            }
        }

        let mut p0 = WaitGroup::new();

        p0.clone();
    }
}