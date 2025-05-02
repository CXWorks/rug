use crate::Duration;
use core::{
    cmp::{Ord, Ordering, PartialEq, PartialOrd},
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration as StdDuration,
};
use standback::convert::{TryFrom, TryInto};
use std::time::Instant as StdInstant;
/// A measurement of a monotonically non-decreasing clock. Opaque and useful
/// only with [`Duration`].
///
/// Instants are always guaranteed to be no less than any previously measured
/// instant when created, and are often useful for tasks such as measuring
/// benchmarks or timing how long an operation takes.
///
/// Note, however, that instants are not guaranteed to be **steady**. In other
/// words, each tick of the underlying clock may not be the same length (e.g.
/// some seconds may be longer than others). An instant may jump forwards or
/// experience time dilation (slow down or speed up), but it will never go
/// backwards.
///
/// Instants are opaque types that can only be compared to one another. There is
/// no method to get "the number of seconds" from an instant. Instead, it only
/// allows measuring the duration between two instants (or comparing two
/// instants).
///
/// This implementation allows for operations with signed [`Duration`]s, but is
/// otherwise identical to [`std::time::Instant`].
#[cfg_attr(__time_02_docs, doc(cfg(feature = "std")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    /// Inner representation, using `std::time::Instant`.
    inner: StdInstant,
}
impl Instant {
    /// Returns an `Instant` corresponding to "now".
    ///
    /// ```rust
    /// # use time::Instant;
    /// println!("{:?}", Instant::now());
    /// ```
    pub fn now() -> Self {
        Self { inner: StdInstant::now() }
    }
    /// Returns the amount of time elapsed since this instant was created. The
    /// duration will always be nonnegative if the instant is not synthetically
    /// created.
    ///
    /// ```rust
    /// # use time::{Instant, prelude::*};
    /// # use std::thread;
    /// let instant = Instant::now();
    /// thread::sleep(1.std_milliseconds());
    /// assert!(instant.elapsed() >= 1.milliseconds());
    /// ```
    pub fn elapsed(self) -> Duration {
        Self::now() - self
    }
    /// Returns `Some(t)` where `t` is the time `self + duration` if `t` can be
    /// represented as `Instant` (which means it's inside the bounds of the
    /// underlying data structure), `None` otherwise.
    ///
    /// ```rust
    /// # use time::{Instant, prelude::*};
    /// let now = Instant::now();
    /// assert_eq!(
    ///     now.checked_add(5.seconds()),
    ///     Some(now + 5.seconds())
    /// );
    /// assert_eq!(
    ///     now.checked_add((-5).seconds()),
    ///     Some(now + (-5).seconds())
    /// );
    /// ```
    ///
    /// This function is only present when using rustc >= 1.34.0.
    #[cfg(__time_02_instant_checked_ops)]
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        if duration.is_zero() {
            Some(self)
        } else if duration.is_positive() {
            self.inner.checked_add(duration.abs_std()).map(From::from)
        } else {
            self.inner.checked_sub(duration.abs_std()).map(From::from)
        }
    }
    /// Returns `Some(t)` where `t` is the time `self - duration` if `t` can be
    /// represented as `Instant` (which means it's inside the bounds of the
    /// underlying data structure), `None` otherwise.
    ///
    /// ```rust
    /// # use time::{Instant, prelude::*};
    /// let now = Instant::now();
    /// assert_eq!(
    ///     now.checked_sub(5.seconds()),
    ///     Some(now - 5.seconds())
    /// );
    /// assert_eq!(
    ///     now.checked_sub((-5).seconds()),
    ///     Some(now - (-5).seconds())
    /// );
    /// ```
    ///
    /// This function is only present when using rustc >= 1.34.0.
    #[cfg(__time_02_instant_checked_ops)]
    pub fn checked_sub(self, duration: Duration) -> Option<Self> {
        self.checked_add(-duration)
    }
}
#[allow(clippy::missing_docs_in_private_items)]
impl Instant {
    #[cfg(feature = "deprecated")]
    #[deprecated(since = "0.2.0", note = "Use `rhs - lhs`")]
    pub fn to(&self, later: Self) -> Duration {
        later - *self
    }
}
impl From<StdInstant> for Instant {
    fn from(instant: StdInstant) -> Self {
        Self { inner: instant }
    }
}
impl From<Instant> for StdInstant {
    fn from(instant: Instant) -> Self {
        instant.inner
    }
}
impl Sub for Instant {
    type Output = Duration;
    fn sub(self, other: Self) -> Self::Output {
        match self.inner.cmp(&other.inner) {
            Ordering::Equal => Duration::zero(),
            Ordering::Greater => {
                (self.inner - other.inner)
                    .try_into()
                    .expect(
                        "overflow converting `std::time::Duration` to `time::Duration`",
                    )
            }
            Ordering::Less => {
                -Duration::try_from(other.inner - self.inner)
                    .expect(
                        "overflow converting `std::time::Duration` to `time::Duration`",
                    )
            }
        }
    }
}
impl Sub<StdInstant> for Instant {
    type Output = Duration;
    fn sub(self, other: StdInstant) -> Self::Output {
        self - Self::from(other)
    }
}
impl Sub<Instant> for StdInstant {
    type Output = Duration;
    fn sub(self, other: Instant) -> Self::Output {
        Instant::from(self) - other
    }
}
impl Add<Duration> for Instant {
    type Output = Self;
    fn add(self, duration: Duration) -> Self::Output {
        if duration.is_positive() {
            (self.inner + duration.abs_std()).into()
        } else if duration.is_negative() {
            (self.inner - duration.abs_std()).into()
        } else {
            self
        }
    }
}
impl Add<Duration> for StdInstant {
    type Output = Self;
    fn add(self, duration: Duration) -> Self::Output {
        (Instant::from(self) + duration).into()
    }
}
impl Add<StdDuration> for Instant {
    type Output = Self;
    fn add(self, duration: StdDuration) -> Self::Output {
        Self {
            inner: self.inner + duration,
        }
    }
}
impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}
impl AddAssign<Duration> for StdInstant {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}
impl AddAssign<StdDuration> for Instant {
    fn add_assign(&mut self, duration: StdDuration) {
        *self = *self + duration;
    }
}
impl Sub<Duration> for Instant {
    type Output = Self;
    fn sub(self, duration: Duration) -> Self::Output {
        self + -duration
    }
}
impl Sub<Duration> for StdInstant {
    type Output = Self;
    fn sub(self, duration: Duration) -> Self::Output {
        (Instant::from(self) - duration).into()
    }
}
impl Sub<StdDuration> for Instant {
    type Output = Self;
    fn sub(self, duration: StdDuration) -> Self::Output {
        Self {
            inner: self.inner - duration,
        }
    }
}
impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}
impl SubAssign<Duration> for StdInstant {
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}
impl SubAssign<StdDuration> for Instant {
    fn sub_assign(&mut self, duration: StdDuration) {
        *self = *self - duration;
    }
}
impl PartialEq<StdInstant> for Instant {
    fn eq(&self, rhs: &StdInstant) -> bool {
        self.inner.eq(rhs)
    }
}
impl PartialEq<Instant> for StdInstant {
    fn eq(&self, rhs: &Instant) -> bool {
        self.eq(&rhs.inner)
    }
}
impl PartialOrd<StdInstant> for Instant {
    fn partial_cmp(&self, rhs: &StdInstant) -> Option<Ordering> {
        self.inner.partial_cmp(rhs)
    }
}
impl PartialOrd<Instant> for StdInstant {
    fn partial_cmp(&self, rhs: &Instant) -> Option<Ordering> {
        self.partial_cmp(&rhs.inner)
    }
}
#[cfg(test)]
mod tests_llm_16_890 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_890_rrrruuuugggg_test_eq = 0;
        let instant1 = Instant {
            inner: StdInstant::now(),
        };
        let instant2 = Instant {
            inner: StdInstant::now(),
        };
        let instant3 = Instant {
            inner: StdInstant::now(),
        };
        debug_assert_eq!(instant1.eq(& instant2), instant1.inner.eq(& instant2.inner));
        debug_assert_eq!(instant2.eq(& instant3), instant2.inner.eq(& instant3.inner));
        debug_assert_eq!(instant3.eq(& instant1), instant3.inner.eq(& instant1.inner));
        let _rug_ed_tests_llm_16_890_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_892_llm_16_891 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    use crate::ext::NumericalDuration;
    use crate::ext::NumericalStdDurationShort;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_892_llm_16_891_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let instant1 = Instant::now();
        let instant2 = Instant::now() + NumericalStdDurationShort::seconds(rug_fuzz_0);
        let instant3 = Instant::now() + NumericalStdDurationShort::seconds(rug_fuzz_1);
        debug_assert_eq!(instant1.partial_cmp(& instant2), Some(Ordering::Less));
        debug_assert_eq!(instant2.partial_cmp(& instant1), Some(Ordering::Greater));
        debug_assert_eq!(instant2.partial_cmp(& instant2), Some(Ordering::Equal));
        debug_assert_eq!(instant2.partial_cmp(& instant3), Some(Ordering::Less));
        let _rug_ed_tests_llm_16_892_llm_16_891_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_895 {
    use super::*;
    use crate::*;
    #[test]
    fn test_add() {
        let _rug_st_tests_llm_16_895_rrrruuuugggg_test_add = 0;
        let rug_fuzz_0 = 10;
        let instant = StdInstant::now();
        let duration = Duration::seconds(rug_fuzz_0);
        let result = instant.add(duration);
        debug_assert_eq!(result, instant + duration);
        let _rug_ed_tests_llm_16_895_rrrruuuugggg_test_add = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_900 {
    use super::*;
    use crate::*;
    #[test]
    fn test_sub() {
        let _rug_st_tests_llm_16_900_rrrruuuugggg_test_sub = 0;
        let instant_1 = Instant::now();
        let instant_2 = Instant::now();
        let duration = instant_1 - instant_2;
        debug_assert_eq!(duration, Duration::zero());
        let _rug_ed_tests_llm_16_900_rrrruuuugggg_test_sub = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_912_llm_16_911 {
    use super::*;
    use crate::*;
    use crate::*;
    use crate::ext::NumericalDuration;
    use crate::ext::NumericalStdDurationShort;
    use crate::Duration as StdDuration;
    #[test]
    fn test_to() {
        let _rug_st_tests_llm_16_912_llm_16_911_rrrruuuugggg_test_to = 0;
        let rug_fuzz_0 = 5;
        let instant = Instant::now();
        let later = instant + NumericalStdDurationShort::seconds(rug_fuzz_0);
        let duration = instant.to(later);
        let expected = later - instant;
        debug_assert_eq!(duration, expected);
        let _rug_ed_tests_llm_16_912_llm_16_911_rrrruuuugggg_test_to = 0;
    }
}
#[cfg(test)]
mod tests_rug_347 {
    use super::*;
    use crate::Instant;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_347_rrrruuuugggg_test_rug = 0;
        let now = Instant::now();
        let _rug_ed_tests_rug_347_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_348 {
    use super::*;
    use crate::{Instant, prelude::*};
    #[test]
    fn test_elapsed() {
        let _rug_st_tests_rug_348_rrrruuuugggg_test_elapsed = 0;
        let mut p0: Instant = Instant::now();
        p0.elapsed();
        let _rug_ed_tests_rug_348_rrrruuuugggg_test_elapsed = 0;
    }
}
#[cfg(test)]
mod tests_rug_349 {
    use super::*;
    use crate::{Instant, Duration};
    #[test]
    fn test_checked_add() {
        let _rug_st_tests_rug_349_rrrruuuugggg_test_checked_add = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 0;
        let now: Instant = Instant::now();
        let duration: Duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(now.checked_add(duration), Some(now + duration));
        let negative_duration: Duration = Duration::new(-rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(
            now.checked_add(negative_duration), Some(now + negative_duration)
        );
        let _rug_ed_tests_rug_349_rrrruuuugggg_test_checked_add = 0;
    }
}
#[cfg(test)]
mod tests_rug_350 {
    use super::*;
    use crate::instant::Instant;
    use crate::duration::Duration;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_350_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0: Instant = Instant::now();
        let mut p1: Duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        p0.checked_sub(p1);
        let _rug_ed_tests_rug_350_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_354 {
    use super::*;
    use crate::{Instant, Duration};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_354_rrrruuuugggg_test_rug = 0;
        let mut p0: Instant = Instant::now();
        let mut p1 = std::time::Instant::now();
        let result = p0.sub(p1);
        let _rug_ed_tests_rug_354_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_357 {
    use super::*;
    use crate::{Instant, Duration};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_357_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0: Instant = Instant::now();
        let mut p1: Duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        <Instant as std::ops::AddAssign<Duration>>::add_assign(&mut p0, p1);
        let _rug_ed_tests_rug_357_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_363 {
    use super::*;
    use crate::{Instant, Date, Duration};
    #[test]
    fn test_sub_assign() {
        let _rug_st_tests_rug_363_rrrruuuugggg_test_sub_assign = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 0;
        let mut p0: Instant = Instant::now();
        let mut p1: Duration = Duration::new(rug_fuzz_0, rug_fuzz_1);
        p0.sub_assign(p1);
        let _rug_ed_tests_rug_363_rrrruuuugggg_test_sub_assign = 0;
    }
}
