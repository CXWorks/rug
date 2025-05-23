//! Miscellaneous utilities.
use std::cell::{Cell, UnsafeCell};
use std::num::Wrapping;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use crossbeam_utils::Backoff;
/// Randomly shuffles a slice.
pub fn shuffle<T>(v: &mut [T]) {
    let len = v.len();
    if len <= 1 {
        return;
    }
    thread_local! {
        static RNG : Cell < Wrapping < u32 >> = Cell::new(Wrapping(1_406_868_647));
    }
    let _ = RNG
        .try_with(|rng| {
            for i in 1..len {
                let mut x = rng.get();
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                rng.set(x);
                let x = x.0;
                let n = i + 1;
                let j = ((x as u64).wrapping_mul(n as u64) >> 32) as u32 as usize;
                v.swap(i, j);
            }
        });
}
/// Sleeps until the deadline, or forever if the deadline isn't specified.
pub fn sleep_until(deadline: Option<Instant>) {
    loop {
        match deadline {
            None => thread::sleep(Duration::from_secs(1000)),
            Some(d) => {
                let now = Instant::now();
                if now >= d {
                    break;
                }
                thread::sleep(d - now);
            }
        }
    }
}
/// A simple spinlock.
pub struct Spinlock<T> {
    flag: AtomicBool,
    value: UnsafeCell<T>,
}
impl<T> Spinlock<T> {
    /// Returns a new spinlock initialized with `value`.
    pub fn new(value: T) -> Spinlock<T> {
        Spinlock {
            flag: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }
    /// Locks the spinlock.
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        let backoff = Backoff::new();
        while self.flag.swap(true, Ordering::Acquire) {
            backoff.snooze();
        }
        SpinlockGuard { parent: self }
    }
}
/// A guard holding a spinlock locked.
pub struct SpinlockGuard<'a, T> {
    parent: &'a Spinlock<T>,
}
impl<T> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.parent.flag.store(false, Ordering::Release);
    }
}
impl<T> Deref for SpinlockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.parent.value.get() }
    }
}
impl<T> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.parent.value.get() }
    }
}
#[cfg(test)]
mod tests_rug_94 {
    use super::*;
    use crate::utils;
    use std::cell::{Cell, RefCell};
    use std::num::Wrapping;
    use crate::utils::shuffle;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_94_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 7;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 2;
        let mut p0: &mut [i32] = &mut [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        utils::shuffle(p0);
        let _rug_ed_tests_rug_94_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_95 {
    use super::*;
    use std::time::{Duration, Instant};
    use std::thread;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_95_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: Option<Instant> = Some(
            Instant::now() + Duration::from_secs(rug_fuzz_0),
        );
        crate::utils::sleep_until(p0);
        let _rug_ed_tests_rug_95_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use crate::utils::Spinlock;
    use std::sync::atomic::AtomicBool;
    use std::cell::UnsafeCell;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_96_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let value: i32 = rug_fuzz_0;
        let p0 = value;
        let _ = Spinlock::<i32>::new(p0);
        let _rug_ed_tests_rug_96_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::utils::Spinlock;
    #[test]
    fn test_lock() {
        let _rug_st_tests_rug_97_rrrruuuugggg_test_lock = 0;
        let rug_fuzz_0 = 42;
        let spinlock = Spinlock::<i32>::new(rug_fuzz_0);
        let guard = spinlock.lock();
        let _rug_ed_tests_rug_97_rrrruuuugggg_test_lock = 0;
    }
}
