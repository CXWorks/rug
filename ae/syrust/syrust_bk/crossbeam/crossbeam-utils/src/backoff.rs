use core::cell::Cell;
use core::fmt;
use core::sync::atomic;

const SPIN_LIMIT: u32 = 6;
const YIELD_LIMIT: u32 = 10;

/// Performs exponential backoff in spin loops.
///
/// Backing off in spin loops reduces contention and improves overall performance.
///
/// This primitive can execute *YIELD* and *PAUSE* instructions, yield the current thread to the OS
/// scheduler, and tell when is a good time to block the thread using a different synchronization
/// mechanism. Each step of the back off procedure takes roughly twice as long as the previous
/// step.
///
/// # Examples
///
/// Backing off in a lock-free loop:
///
/// ```
/// use crossbeam_utils::Backoff;
/// use std::sync::atomic::AtomicUsize;
/// use std::sync::atomic::Ordering::SeqCst;
///
/// fn fetch_mul(a: &AtomicUsize, b: usize) -> usize {
///     let backoff = Backoff::new();
///     loop {
///         let val = a.load(SeqCst);
///         if a.compare_and_swap(val, val.wrapping_mul(b), SeqCst) == val {
///             return val;
///         }
///         backoff.spin();
///     }
/// }
/// ```
///
/// Waiting for an [`AtomicBool`] to become `true`:
///
/// ```
/// use crossbeam_utils::Backoff;
/// use std::sync::atomic::AtomicBool;
/// use std::sync::atomic::Ordering::SeqCst;
///
/// fn spin_wait(ready: &AtomicBool) {
///     let backoff = Backoff::new();
///     while !ready.load(SeqCst) {
///         backoff.snooze();
///     }
/// }
/// ```
///
/// Waiting for an [`AtomicBool`] to become `true` and parking the thread after a long wait.
/// Note that whoever sets the atomic variable to `true` must notify the parked thread by calling
/// [`unpark()`]:
///
/// ```
/// use crossbeam_utils::Backoff;
/// use std::sync::atomic::AtomicBool;
/// use std::sync::atomic::Ordering::SeqCst;
/// use std::thread;
///
/// fn blocking_wait(ready: &AtomicBool) {
///     let backoff = Backoff::new();
///     while !ready.load(SeqCst) {
///         if backoff.is_completed() {
///             thread::park();
///         } else {
///             backoff.snooze();
///         }
///     }
/// }
/// ```
///
/// [`is_completed`]: struct.Backoff.html#method.is_completed
/// [`std::thread::park()`]: https://doc.rust-lang.org/std/thread/fn.park.html
/// [`Condvar`]: https://doc.rust-lang.org/std/sync/struct.Condvar.html
/// [`AtomicBool`]: https://doc.rust-lang.org/std/sync/atomic/struct.AtomicBool.html
/// [`unpark()`]: https://doc.rust-lang.org/std/thread/struct.Thread.html#method.unpark
pub struct Backoff {
    step: Cell<u32>,
}

impl Backoff {
    /// Creates a new `Backoff`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::Backoff;
    ///
    /// let backoff = Backoff::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Backoff { step: Cell::new(0) }
    }

    /// Resets the `Backoff`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::Backoff;
    ///
    /// let backoff = Backoff::new();
    /// backoff.reset();
    /// ```
    #[inline]
    pub fn reset(&self) {
        self.step.set(0);
    }

    /// Backs off in a lock-free loop.
    ///
    /// This method should be used when we need to retry an operation because another thread made
    /// progress.
    ///
    /// The processor may yield using the *YIELD* or *PAUSE* instruction.
    ///
    /// # Examples
    ///
    /// Backing off in a lock-free loop:
    ///
    /// ```
    /// use crossbeam_utils::Backoff;
    /// use std::sync::atomic::AtomicUsize;
    /// use std::sync::atomic::Ordering::SeqCst;
    ///
    /// fn fetch_mul(a: &AtomicUsize, b: usize) -> usize {
    ///     let backoff = Backoff::new();
    ///     loop {
    ///         let val = a.load(SeqCst);
    ///         if a.compare_and_swap(val, val.wrapping_mul(b), SeqCst) == val {
    ///             return val;
    ///         }
    ///         backoff.spin();
    ///     }
    /// }
    ///
    /// let a = AtomicUsize::new(7);
    /// assert_eq!(fetch_mul(&a, 8), 7);
    /// assert_eq!(a.load(SeqCst), 56);
    /// ```
    #[inline]
    pub fn spin(&self) {
        for _ in 0..1 << self.step.get().min(SPIN_LIMIT) {
            atomic::spin_loop_hint();
        }

        if self.step.get() <= SPIN_LIMIT {
            self.step.set(self.step.get() + 1);
        }
    }

    /// Backs off in a blocking loop.
    ///
    /// This method should be used when we need to wait for another thread to make progress.
    ///
    /// The processor may yield using the *YIELD* or *PAUSE* instruction and the current thread
    /// may yield by giving up a timeslice to the OS scheduler.
    ///
    /// In `#[no_std]` environments, this method is equivalent to [`spin`].
    ///
    /// If possible, use [`is_completed`] to check when it is advised to stop using backoff and
    /// block the current thread using a different synchronization mechanism instead.
    ///
    /// [`spin`]: struct.Backoff.html#method.spin
    /// [`is_completed`]: struct.Backoff.html#method.is_completed
    ///
    /// # Examples
    ///
    /// Waiting for an [`AtomicBool`] to become `true`:
    ///
    /// ```
    /// use crossbeam_utils::Backoff;
    /// use std::sync::Arc;
    /// use std::sync::atomic::AtomicBool;
    /// use std::sync::atomic::Ordering::SeqCst;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// fn spin_wait(ready: &AtomicBool) {
    ///     let backoff = Backoff::new();
    ///     while !ready.load(SeqCst) {
    ///         backoff.snooze();
    ///     }
    /// }
    ///
    /// let ready = Arc::new(AtomicBool::new(false));
    /// let ready2 = ready.clone();
    ///
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_millis(100));
    ///     ready2.store(true, SeqCst);
    /// });
    ///
    /// assert_eq!(ready.load(SeqCst), false);
    /// spin_wait(&ready);
    /// assert_eq!(ready.load(SeqCst), true);
    /// ```
    ///
    /// [`AtomicBool`]: https://doc.rust-lang.org/std/sync/atomic/struct.AtomicBool.html
    #[inline]
    pub fn snooze(&self) {
        if self.step.get() <= SPIN_LIMIT {
            for _ in 0..1 << self.step.get() {
                atomic::spin_loop_hint();
            }
        } else {
            #[cfg(not(feature = "std"))]
            for _ in 0..1 << self.step.get() {
                atomic::spin_loop_hint();
            }

            #[cfg(feature = "std")]
            ::std::thread::yield_now();
        }

        if self.step.get() <= YIELD_LIMIT {
            self.step.set(self.step.get() + 1);
        }
    }

    /// Returns `true` if exponential backoff has completed and blocking the thread is advised.
    ///
    /// # Examples
    ///
    /// Waiting for an [`AtomicBool`] to become `true` and parking the thread after a long wait:
    ///
    /// ```
    /// use crossbeam_utils::Backoff;
    /// use std::sync::Arc;
    /// use std::sync::atomic::AtomicBool;
    /// use std::sync::atomic::Ordering::SeqCst;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// fn blocking_wait(ready: &AtomicBool) {
    ///     let backoff = Backoff::new();
    ///     while !ready.load(SeqCst) {
    ///         if backoff.is_completed() {
    ///             thread::park();
    ///         } else {
    ///             backoff.snooze();
    ///         }
    ///     }
    /// }
    ///
    /// let ready = Arc::new(AtomicBool::new(false));
    /// let ready2 = ready.clone();
    /// let waiter = thread::current();
    ///
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_millis(100));
    ///     ready2.store(true, SeqCst);
    ///     waiter.unpark();
    /// });
    ///
    /// assert_eq!(ready.load(SeqCst), false);
    /// blocking_wait(&ready);
    /// assert_eq!(ready.load(SeqCst), true);
    /// ```
    ///
    /// [`AtomicBool`]: https://doc.rust-lang.org/std/sync/atomic/struct.AtomicBool.html
    #[inline]
    pub fn is_completed(&self) -> bool {
        self.step.get() > YIELD_LIMIT
    }
}

impl fmt::Debug for Backoff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Backoff")
            .field("step", &self.step)
            .field("is_completed", &self.is_completed())
            .finish()
    }
}

impl Default for Backoff {
    fn default() -> Backoff {
        Backoff::new()
    }
}
#[cfg(test)]
mod tests_rug_781 {
    use super::*;
    use crate::Backoff;

    #[test]
    fn test_rug() {
        Backoff::new();
    }
}#[cfg(test)]
mod tests_rug_782 {
    use super::*;
    use crate::Backoff;

    #[test]
    fn test_reset() {
        let backoff = Backoff::default();
        backoff.reset();
        // Assert that step is reset to 0 after calling the reset method
        assert_eq!(backoff.step.get(), 0);
    }
}#[cfg(test)]
mod tests_rug_783 {
    use super::*;
    use crate::Backoff;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering::SeqCst;

    #[test]
    fn test_rug() {
        let a = AtomicUsize::new(7);
        let backoff = Backoff::new();

        loop {
            let val = a.load(SeqCst);
            if a.compare_and_swap(val, val.wrapping_mul(8), SeqCst) == val {
                break;
            }
            backoff.spin();
        }

        assert_eq!(a.load(SeqCst), 56);
    }
}#[cfg(test)]
mod tests_rug_784 {
    use super::*;
    use crate::Backoff;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_rug() {
        let ready = Arc::new(AtomicBool::new(false));
        let ready2 = ready.clone();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            ready2.store(true, Ordering::SeqCst);
        });

        let backoff = Backoff::new();
        while !ready.load(Ordering::SeqCst) {
            backoff.snooze();
          }
      }
}#[cfg(test)]
mod tests_rug_785 {
    use super::*;
    use crate::{Backoff};
    
    #[test]
    fn test_rug() {
        let mut backoff = Backoff::default();

        backoff.is_completed();
    }
}#[cfg(test)]
mod tests_rug_786 {
    use super::*;
    use crate::backoff::Backoff;
    use std::default::Default;

    #[test]
    fn test_default_backoff() {
        let backoff_instance: Backoff = <Backoff as Default>::default();
        // Add assertions or any test logic here
    }
}