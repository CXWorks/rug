use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
/// A thread parking primitive.
///
/// Conceptually, each `Parker` has an associated token which is initially not present:
///
/// * The [`park`] method blocks the current thread unless or until the token is available, at
///   which point it automatically consumes the token. It may also return *spuriously*, without
///   consuming the token.
///
/// * The [`park_timeout`] method works the same as [`park`], but blocks for a specified maximum
///   time.
///
/// * The [`unpark`] method atomically makes the token available if it wasn't already. Because the
///   token is initially absent, [`unpark`] followed by [`park`] will result in the second call
///   returning immediately.
///
/// In other words, each `Parker` acts a bit like a spinlock that can be locked and unlocked using
/// [`park`] and [`unpark`].
///
/// # Examples
///
/// ```
/// use std::thread;
/// use std::time::Duration;
/// use crossbeam_utils::sync::Parker;
///
/// let p = Parker::new();
/// let u = p.unparker().clone();
///
/// // Make the token available.
/// u.unpark();
/// // Wakes up immediately and consumes the token.
/// p.park();
///
/// thread::spawn(move || {
///     thread::sleep(Duration::from_millis(500));
///     u.unpark();
/// });
///
/// // Wakes up when `u.unpark()` provides the token, but may also wake up
/// // spuriously before that without consuming the token.
/// p.park();
/// ```
///
/// [`park`]: struct.Parker.html#method.park
/// [`park_timeout`]: struct.Parker.html#method.park_timeout
/// [`unpark`]: struct.Unparker.html#method.unpark
pub struct Parker {
    unparker: Unparker,
    _marker: PhantomData<*const ()>,
}
unsafe impl Send for Parker {}
impl Default for Parker {
    fn default() -> Self {
        Self {
            unparker: Unparker {
                inner: Arc::new(Inner {
                    state: AtomicUsize::new(EMPTY),
                    lock: Mutex::new(()),
                    cvar: Condvar::new(),
                }),
            },
            _marker: PhantomData,
        }
    }
}
impl Parker {
    /// Creates a new `Parker`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::Parker;
    ///
    /// let p = Parker::new();
    /// ```
    ///
    pub fn new() -> Parker {
        Self::default()
    }
    /// Blocks the current thread until the token is made available.
    ///
    /// A call to `park` may wake up spuriously without consuming the token, and callers should be
    /// prepared for this possibility.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::Parker;
    ///
    /// let p = Parker::new();
    /// let u = p.unparker().clone();
    ///
    /// // Make the token available.
    /// u.unpark();
    ///
    /// // Wakes up immediately and consumes the token.
    /// p.park();
    /// ```
    pub fn park(&self) {
        self.unparker.inner.park(None);
    }
    /// Blocks the current thread until the token is made available, but only for a limited time.
    ///
    /// A call to `park_timeout` may wake up spuriously without consuming the token, and callers
    /// should be prepared for this possibility.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use crossbeam_utils::sync::Parker;
    ///
    /// let p = Parker::new();
    ///
    /// // Waits for the token to become available, but will not wait longer than 500 ms.
    /// p.park_timeout(Duration::from_millis(500));
    /// ```
    pub fn park_timeout(&self, timeout: Duration) {
        self.unparker.inner.park(Some(timeout));
    }
    /// Returns a reference to an associated [`Unparker`].
    ///
    /// The returned [`Unparker`] doesn't have to be used by reference - it can also be cloned.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::Parker;
    ///
    /// let p = Parker::new();
    /// let u = p.unparker().clone();
    ///
    /// // Make the token available.
    /// u.unpark();
    /// // Wakes up immediately and consumes the token.
    /// p.park();
    /// ```
    ///
    /// [`park`]: struct.Parker.html#method.park
    /// [`park_timeout`]: struct.Parker.html#method.park_timeout
    ///
    /// [`Unparker`]: struct.Unparker.html
    pub fn unparker(&self) -> &Unparker {
        &self.unparker
    }
    /// Converts a `Parker` into a raw pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::Parker;
    ///
    /// let p = Parker::new();
    /// let raw = Parker::into_raw(p);
    /// ```
    pub fn into_raw(this: Parker) -> *const () {
        Unparker::into_raw(this.unparker)
    }
    /// Converts a raw pointer into a `Parker`.
    ///
    /// # Safety
    ///
    /// This method is safe to use only with pointers returned by [`Parker::into_raw`].
    ///
    /// [`Parker::into_raw`]: struct.Parker.html#method.into_raw
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::Parker;
    ///
    /// let p = Parker::new();
    /// let raw = Parker::into_raw(p);
    /// let p = unsafe { Parker::from_raw(raw) };
    /// ```
    pub unsafe fn from_raw(ptr: *const ()) -> Parker {
        Parker {
            unparker: Unparker::from_raw(ptr),
            _marker: PhantomData,
        }
    }
}
impl fmt::Debug for Parker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Parker { .. }")
    }
}
/// Unparks a thread parked by the associated [`Parker`].
///
/// [`Parker`]: struct.Parker.html
pub struct Unparker {
    inner: Arc<Inner>,
}
unsafe impl Send for Unparker {}
unsafe impl Sync for Unparker {}
impl Unparker {
    /// Atomically makes the token available if it is not already.
    ///
    /// This method will wake up the thread blocked on [`park`] or [`park_timeout`], if there is
    /// any.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::thread;
    /// use std::time::Duration;
    /// use crossbeam_utils::sync::Parker;
    ///
    /// let p = Parker::new();
    /// let u = p.unparker().clone();
    ///
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_millis(500));
    ///     u.unpark();
    /// });
    ///
    /// // Wakes up when `u.unpark()` provides the token, but may also wake up
    /// // spuriously before that without consuming the token.
    /// p.park();
    /// ```
    ///
    /// [`park`]: struct.Parker.html#method.park
    /// [`park_timeout`]: struct.Parker.html#method.park_timeout
    pub fn unpark(&self) {
        self.inner.unpark()
    }
    /// Converts an `Unparker` into a raw pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::{Parker, Unparker};
    ///
    /// let p = Parker::new();
    /// let u = p.unparker().clone();
    /// let raw = Unparker::into_raw(u);
    /// ```
    pub fn into_raw(this: Unparker) -> *const () {
        Arc::into_raw(this.inner) as *const ()
    }
    /// Converts a raw pointer into an `Unparker`.
    ///
    /// # Safety
    ///
    /// This method is safe to use only with pointers returned by [`Unparker::into_raw`].
    ///
    /// [`Unparker::into_raw`]: struct.Unparker.html#method.into_raw
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::sync::{Parker, Unparker};
    ///
    /// let p = Parker::new();
    /// let u = p.unparker().clone();
    ///
    /// let raw = Unparker::into_raw(u);
    /// let u = unsafe { Unparker::from_raw(raw) };
    /// ```
    pub unsafe fn from_raw(ptr: *const ()) -> Unparker {
        Unparker {
            inner: Arc::from_raw(ptr as *const Inner),
        }
    }
}
impl fmt::Debug for Unparker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Unparker { .. }")
    }
}
impl Clone for Unparker {
    fn clone(&self) -> Unparker {
        Unparker {
            inner: self.inner.clone(),
        }
    }
}
const EMPTY: usize = 0;
const PARKED: usize = 1;
const NOTIFIED: usize = 2;
struct Inner {
    state: AtomicUsize,
    lock: Mutex<()>,
    cvar: Condvar,
}
impl Inner {
    fn park(&self, timeout: Option<Duration>) {
        if self.state.compare_exchange(NOTIFIED, EMPTY, SeqCst, SeqCst).is_ok() {
            return;
        }
        if let Some(ref dur) = timeout {
            if *dur == Duration::from_millis(0) {
                return;
            }
        }
        let mut m = self.lock.lock().unwrap();
        match self.state.compare_exchange(EMPTY, PARKED, SeqCst, SeqCst) {
            Ok(_) => {}
            Err(NOTIFIED) => {
                let old = self.state.swap(EMPTY, SeqCst);
                assert_eq!(old, NOTIFIED, "park state changed unexpectedly");
                return;
            }
            Err(n) => panic!("inconsistent park_timeout state: {}", n),
        }
        match timeout {
            None => {
                loop {
                    m = self.cvar.wait(m).unwrap();
                    if self
                        .state
                        .compare_exchange(NOTIFIED, EMPTY, SeqCst, SeqCst)
                        .is_ok()
                    {
                        return;
                    }
                }
            }
            Some(timeout) => {
                let (_m, _result) = self.cvar.wait_timeout(m, timeout).unwrap();
                match self.state.swap(EMPTY, SeqCst) {
                    NOTIFIED => {}
                    PARKED => {}
                    n => panic!("inconsistent park_timeout state: {}", n),
                }
            }
        }
    }
    pub fn unpark(&self) {
        match self.state.swap(NOTIFIED, SeqCst) {
            EMPTY => return,
            NOTIFIED => return,
            PARKED => {}
            _ => panic!("inconsistent state in unpark"),
        }
        drop(self.lock.lock().unwrap());
        self.cvar.notify_one();
    }
}
#[cfg(test)]
mod tests_rug_787 {
    use super::*;
    use crate::sync::parker::{Parker, Unparker, Inner};
    use std::sync::{Arc, Mutex, Condvar};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::marker::PhantomData;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_787_rrrruuuugggg_test_rug = 0;
        let parker_default = <Parker as Default>::default();
        let _rug_ed_tests_rug_787_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_788 {
    use super::*;
    use crate::sync::Parker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_788_rrrruuuugggg_test_rug = 0;
        Parker::new();
        let _rug_ed_tests_rug_788_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_789 {
    use super::*;
    use crate::sync::Parker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_789_rrrruuuugggg_test_rug = 0;
        let p0 = Parker::new();
        Parker::park(&p0);
        let _rug_ed_tests_rug_789_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_790 {
    use super::*;
    use crate::sync::Parker;
    use std::time::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_790_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let p0 = Parker::new();
        let p1 = Duration::from_secs(rug_fuzz_0);
        p0.park_timeout(p1);
        let _rug_ed_tests_rug_790_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_791 {
    use super::*;
    use crate::sync::{Parker, Unparker};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_791_rrrruuuugggg_test_rug = 0;
        let p0 = Parker::new();
        Parker::unparker(&p0);
        let _rug_ed_tests_rug_791_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_792 {
    use super::*;
    use crate::sync::Parker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_792_rrrruuuugggg_test_rug = 0;
        let p0 = Parker::new();
        Parker::into_raw(p0);
        let _rug_ed_tests_rug_792_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_793 {
    use super::*;
    use crate::sync::{Parker, Unparker};
    use std::marker::PhantomData;
    #[test]
    fn test_from_raw() {
        let _rug_st_tests_rug_793_rrrruuuugggg_test_from_raw = 0;
        let rug_fuzz_0 = 0;
        let ptr: *const () = rug_fuzz_0 as *const ();
        let p0 = ptr;
        unsafe {
            let _ = Parker::from_raw(p0);
        }
        let _rug_ed_tests_rug_793_rrrruuuugggg_test_from_raw = 0;
    }
}
#[cfg(test)]
mod tests_rug_794 {
    use super::*;
    use crate::sync::Parker;
    use crate::sync::Unparker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_794_rrrruuuugggg_test_rug = 0;
        let p = Parker::new();
        let u = p.unparker().clone();
        Unparker::unpark(&u);
        let _rug_ed_tests_rug_794_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_795 {
    use super::*;
    use crate::sync::{Parker, Unparker};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_795_rrrruuuugggg_test_rug = 0;
        let p = Parker::new();
        let u = p.unparker().clone();
        crate::sync::parker::Unparker::into_raw(u);
        let _rug_ed_tests_rug_795_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_796 {
    use super::*;
    use crate::sync::{Parker, Unparker};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_796_rrrruuuugggg_test_rug = 0;
        let mut p0: *const () = &();
        unsafe {
            let p = Parker::new();
            let u = p.unparker().clone();
            let raw = Unparker::into_raw(u);
            let u = Unparker::from_raw(raw);
        }
        let _rug_ed_tests_rug_796_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_798 {
    use super::*;
    use crate::sync::parker::Inner;
    use std::sync::{Arc, Mutex, Condvar};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_798_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 5;
        let v228 = Inner {
            state: AtomicUsize::new(rug_fuzz_0),
            lock: Mutex::new(()),
            cvar: Condvar::new(),
        };
        let v229 = Some(Duration::from_secs(rug_fuzz_1));
        v228.park(v229);
        let _rug_ed_tests_rug_798_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_799 {
    use super::*;
    use crate::sync::parker::Inner;
    use std::sync::{Arc, Mutex, Condvar};
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_799_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let v228 = Inner {
            state: AtomicUsize::new(rug_fuzz_0),
            lock: Mutex::new(()),
            cvar: Condvar::new(),
        };
        Inner::unpark(&v228);
        let _rug_ed_tests_rug_799_rrrruuuugggg_test_rug = 0;
    }
}
