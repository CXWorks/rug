// Copyright 2016 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Closed-open intervals, and operations on them.

//NOTE: intervals used to be more fancy, and could be open or closed on either
//end. It may now be worth considering replacing intervals with Range<usize> or similar.

use std::cmp::{max, min};
use std::fmt;
use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

/// A fancy version of Range<usize>, representing a closed-open range;
/// the interval [5, 7) is the set {5, 6}.
///
/// It is an invariant that `start <= end`. An interval where `end < start` is
/// considered empty.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub start: usize,
    pub end: usize,
}

impl Interval {
    /// Construct a new `Interval` representing the range [start..end).
    /// It is an invariant that `start <= end`.
    pub fn new(start: usize, end: usize) -> Interval {
        debug_assert!(start <= end);
        Interval { start, end }
    }

    #[deprecated(since = "0.3.0", note = "all intervals are now closed_open, use Interval::new")]
    pub fn new_closed_open(start: usize, end: usize) -> Interval {
        Self::new(start, end)
    }

    #[deprecated(since = "0.3.0", note = "all intervals are now closed_open")]
    pub fn new_open_closed(start: usize, end: usize) -> Interval {
        Self::new(start, end)
    }

    #[deprecated(since = "0.3.0", note = "all intervals are now closed_open")]
    pub fn new_closed_closed(start: usize, end: usize) -> Interval {
        Self::new(start, end)
    }

    #[deprecated(since = "0.3.0", note = "all intervals are now closed_open")]
    pub fn new_open_open(start: usize, end: usize) -> Interval {
        Self::new(start, end)
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn start_end(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    // The following 3 methods define a trisection, exactly one is true.
    // (similar to std::cmp::Ordering, but "Equal" is not the same as "contains")

    /// the interval is before the point (the point is after the interval)
    pub fn is_before(&self, val: usize) -> bool {
        self.end <= val
    }

    /// the point is inside the interval
    pub fn contains(&self, val: usize) -> bool {
        self.start <= val && val < self.end
    }

    /// the interval is after the point (the point is before the interval)
    pub fn is_after(&self, val: usize) -> bool {
        self.start > val
    }

    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    // impl BitAnd would be completely valid for this
    pub fn intersect(&self, other: Interval) -> Interval {
        let start = max(self.start, other.start);
        let end = min(self.end, other.end);
        Interval { start, end: max(start, end) }
    }

    // smallest interval that encloses both inputs; if the inputs are
    // disjoint, then it fills in the hole.
    pub fn union(&self, other: Interval) -> Interval {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return *self;
        }
        let start = min(self.start, other.start);
        let end = max(self.end, other.end);
        Interval { start, end }
    }

    // the first half of self - other
    pub fn prefix(&self, other: Interval) -> Interval {
        Interval { start: min(self.start, other.start), end: min(self.end, other.start) }
    }

    // the second half of self - other
    pub fn suffix(&self, other: Interval) -> Interval {
        Interval { start: max(self.start, other.end), end: max(self.end, other.end) }
    }

    // could impl Add trait, but that's probably too cute
    pub fn translate(&self, amount: usize) -> Interval {
        Interval { start: self.start + amount, end: self.end + amount }
    }

    // as above for Sub trait
    pub fn translate_neg(&self, amount: usize) -> Interval {
        debug_assert!(self.start >= amount);
        Interval { start: self.start - amount, end: self.end - amount }
    }

    // insensitive to open or closed ends, just the size of the interior
    pub fn size(&self) -> usize {
        self.end - self.start
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}, {})", self.start(), self.end())
    }
}

impl fmt::Debug for Interval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<Range<usize>> for Interval {
    fn from(src: Range<usize>) -> Interval {
        let Range { start, end } = src;
        Interval { start, end }
    }
}

impl From<RangeTo<usize>> for Interval {
    fn from(src: RangeTo<usize>) -> Interval {
        Interval::new(0, src.end)
    }
}

impl From<RangeInclusive<usize>> for Interval {
    fn from(src: RangeInclusive<usize>) -> Interval {
        Interval::new(*src.start(), src.end().saturating_add(1))
    }
}

impl From<RangeToInclusive<usize>> for Interval {
    fn from(src: RangeToInclusive<usize>) -> Interval {
        Interval::new(0, src.end.saturating_add(1))
    }
}

/// A trait for types that represent unbounded ranges; they need an explicit
/// upper bound in order to be converted to `Interval`s.
///
/// This exists so that some methods that use `Interval` under the hood can
/// accept arguments like `..` or `10..`.
///
/// This trait should only be used when the idea of taking all of something
/// makes sense.
pub trait IntervalBounds {
    fn into_interval(self, upper_bound: usize) -> Interval;
}

impl<T: Into<Interval>> IntervalBounds for T {
    fn into_interval(self, _upper_bound: usize) -> Interval {
        self.into()
    }
}

impl IntervalBounds for RangeFrom<usize> {
    fn into_interval(self, upper_bound: usize) -> Interval {
        Interval::new(self.start, upper_bound)
    }
}

impl IntervalBounds for RangeFull {
    fn into_interval(self, upper_bound: usize) -> Interval {
        Interval::new(0, upper_bound)
    }
}

#[cfg(test)]
mod tests {
    use crate::interval::Interval;

    #[test]
    fn contains() {
        let i = Interval::new(2, 42);
        assert!(!i.contains(1));
        assert!(i.contains(2));
        assert!(i.contains(3));
        assert!(i.contains(41));
        assert!(!i.contains(42));
        assert!(!i.contains(43));
    }

    #[test]
    fn before() {
        let i = Interval::new(2, 42);
        assert!(!i.is_before(1));
        assert!(!i.is_before(2));
        assert!(!i.is_before(3));
        assert!(!i.is_before(41));
        assert!(i.is_before(42));
        assert!(i.is_before(43));
    }

    #[test]
    fn after() {
        let i = Interval::new(2, 42);
        assert!(i.is_after(1));
        assert!(!i.is_after(2));
        assert!(!i.is_after(3));
        assert!(!i.is_after(41));
        assert!(!i.is_after(42));
        assert!(!i.is_after(43));
    }

    #[test]
    fn translate() {
        let i = Interval::new(2, 42);
        assert_eq!(Interval::new(5, 45), i.translate(3));
        assert_eq!(Interval::new(1, 41), i.translate_neg(1));
    }

    #[test]
    fn empty() {
        assert!(Interval::new(0, 0).is_empty());
        assert!(Interval::new(1, 1).is_empty());
        assert!(!Interval::new(1, 2).is_empty());
    }

    #[test]
    fn intersect() {
        assert_eq!(Interval::new(2, 3), Interval::new(1, 3).intersect(Interval::new(2, 4)));
        assert!(Interval::new(1, 2).intersect(Interval::new(2, 43)).is_empty());
    }

    #[test]
    fn prefix() {
        assert_eq!(Interval::new(1, 2), Interval::new(1, 4).prefix(Interval::new(2, 3)));
    }

    #[test]
    fn suffix() {
        assert_eq!(Interval::new(3, 4), Interval::new(1, 4).suffix(Interval::new(2, 3)));
    }

    #[test]
    fn size() {
        assert_eq!(40, Interval::new(2, 42).size());
        assert_eq!(0, Interval::new(1, 1).size());
        assert_eq!(1, Interval::new(1, 2).size());
    }
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;

use crate::*;
    use std::ops::Range;

    #[test]
    fn test_into_interval() {
        let upper_bound = 10;
        let range: Range<usize> = 1..5;
        let interval: Interval = range.into_interval(upper_bound);
        let expected_interval = Interval::new(1, 5);
        assert_eq!(interval, expected_interval);
    }
}#[cfg(test)]
mod tests_llm_16_49 {
    use super::*;

use crate::*;
    use std::ops::Range;
    
    #[test]
    fn test_from() {
        let range: Range<usize> = 5..10;
        let interval: Interval = <Interval as std::convert::From<std::ops::Range<usize>>>::from(range);
        assert_eq!(interval.start, 5);
        assert_eq!(interval.end, 10);
    }
}#[cfg(test)]
mod tests_llm_16_51_llm_16_50 {
    use super::*;

use crate::*;
    use crate::breaks::*;
    use crate::interval::Interval;
    use std::ops::RangeInclusive;
    
    #[test]
    fn test_from_range_inclusive() {
        let src = 0..=5;
        let expected = Interval::new(0, 6);
        let result: Interval = <Interval as std::convert::From<RangeInclusive<usize>>>::from(src);
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_52 {
    use crate::Interval;
    use std::ops::RangeTo;

    #[test]
    fn test_from() {
        let src: RangeTo<usize> = ..10;
        let result = Interval::from(src);
        assert_eq!(result.start, 0);
        assert_eq!(result.end, 10);
    }
}#[cfg(test)]
mod tests_llm_16_53 {
    use super::*;

use crate::*;
    use std::ops::RangeToInclusive;
    
    #[test]
    fn test_from() {
        let src: RangeToInclusive<usize> = ..=10;
        let interval: Interval = Interval::from(src);
        assert_eq!(interval.start, 0);
        assert_eq!(interval.end, 11);
    }
}#[cfg(test)]
mod tests_llm_16_123 {
    use crate::interval::Interval;
    use std::ops::{RangeFrom, Range};

    #[test]
    fn test_into_interval() {
        let range_from: RangeFrom<usize> = 10..;
        let upper_bound: usize = 20;

        let expected_interval = Interval::new(range_from.start, upper_bound);
        let result = <RangeFrom<usize> as crate::interval::IntervalBounds>::into_interval(range_from, upper_bound);
        assert_eq!(result, expected_interval);
    }
}#[cfg(test)]
mod tests_llm_16_125 {
    use super::*;

use crate::*;
    use crate::interval::Interval;
    use std::ops::RangeFull;
    
    #[test]
    fn test_into_interval() {
        let upper_bound = 10;
        let result = Interval::new(0, upper_bound);
        assert_eq!(RangeFull.into_interval(upper_bound), result);
    }
}#[cfg(test)]
mod tests_llm_16_282 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_contains() {
        let interval = Interval::new(2, 5);
        
        assert_eq!(interval.contains(3), true);
        assert_eq!(interval.contains(5), false);
        assert_eq!(interval.contains(2), true);
        assert_eq!(interval.contains(7), false);
        assert_eq!(interval.contains(0), false);
        assert_eq!(interval.contains(4), true);
    }
}#[cfg(test)]
mod tests_llm_16_283 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_end() {
        let interval = Interval {
            start: 3,
            end: 8
        };
        assert_eq!(interval.end(), 8);
    }
}#[cfg(test)]
mod tests_llm_16_284 {
    use super::*;

use crate::*;
    use std::ops::Range;

    #[test]
    fn test_intersect() {
        let interval1 = Interval::new(2, 7);
        let interval2 = Interval::new(4, 9);
        let intersection = interval1.intersect(interval2);
        assert_eq!(intersection.start, 4);
        assert_eq!(intersection.end, 7);
    }
}#[cfg(test)]
mod tests_llm_16_285 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_is_after_returns_true_when_interval_is_after_point() {
        let interval = Interval::new(5, 10);
        let val = 2;
        assert_eq!(interval.is_after(val), true);
    }
    
    #[test]
    fn test_is_after_returns_false_when_interval_is_before_point() {
        let interval = Interval::new(5, 10);
        let val = 8;
        assert_eq!(interval.is_after(val), false);
    }
    
    #[test]
    fn test_is_after_returns_false_when_interval_contains_point() {
        let interval = Interval::new(5, 10);
        let val = 7;
        assert_eq!(interval.is_after(val), false);
    }
    
    #[test]
    fn test_is_after_returns_false_when_interval_is_empty() {
        let interval = Interval::new(10, 10);
        let val = 5;
        assert_eq!(interval.is_after(val), false);
    }
}#[cfg(test)]
mod tests_llm_16_286 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_is_before() {
        let interval = Interval::new(1, 5);
        assert_eq!(interval.is_before(0), false);
        assert_eq!(interval.is_before(1), false);
        assert_eq!(interval.is_before(4), false);
        assert_eq!(interval.is_before(5), true);
        assert_eq!(interval.is_before(6), true);
    }
}#[cfg(test)]
mod tests_llm_16_287 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_is_empty_when_end_is_less_than_start() {
        let interval = Interval::new(3, 2);
        assert!(interval.is_empty());
    }
    
    #[test]
    fn test_is_empty_when_end_is_equal_to_start() {
        let interval = Interval::new(5, 5);
        assert!(interval.is_empty());
    }
    
    #[test]
    fn test_is_empty_when_end_is_greater_than_start() {
        let interval = Interval::new(2, 4);
        assert!(!interval.is_empty());
    }
}#[cfg(test)]
mod tests_llm_16_288 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_interval_new() {
        let interval = Interval::new(0, 5);
        assert_eq!(interval.start, 0);
        assert_eq!(interval.end, 5);
    }
}#[cfg(test)]
mod tests_llm_16_289 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new_closed_closed() {
        let interval = Interval::new_closed_closed(1, 5);
        assert_eq!(interval.start, 1);
        assert_eq!(interval.end, 5);
    }
}#[cfg(test)]
mod tests_llm_16_290 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new_closed_open() {
        let interval = Interval::new_closed_open(2, 5);
        assert_eq!(interval.start, 2);
        assert_eq!(interval.end, 5);
    }
}#[cfg(test)]
mod tests_llm_16_291 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new_open_closed() {
        let interval = Interval::new_open_closed(0, 10);
        assert_eq!(interval.start, 0);
        assert_eq!(interval.end, 10);
    }
}#[cfg(test)]
mod tests_llm_16_292 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new_open_open() {
        let interval = Interval::new_open_open(5, 10);
        assert_eq!(interval.start, 5);
        assert_eq!(interval.end, 10);
    }
}#[cfg(test)]
mod tests_llm_16_293 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_prefix() {
        let interval1 = Interval::new(2, 8);
        let interval2 = Interval::new(5, 12);
        let result = interval1.prefix(interval2);
        assert_eq!(result, Interval::new(2, 5));
    }
    
    // Add more test cases here...
}#[cfg(test)]
mod tests_llm_16_294 {
    use std::ops::Range;

    use crate::interval::Interval;

    #[test]
    fn test_size() {
        let interval = Interval::new(0, 10);
        assert_eq!(interval.size(), 10);

        let range: Range<usize> = 5..15;
        let interval: Interval = range.into();
        assert_eq!(interval.size(), 10);

        let range_inclusive = 0..=5;
        let interval: Interval = range_inclusive.into();
        assert_eq!(interval.size(), 6);

        let range_to = ..5;
        let interval: Interval = range_to.into();
        assert_eq!(interval.size(), 5);

        let range_to_inclusive = ..=5;
        let interval: Interval = range_to_inclusive.into();
        assert_eq!(interval.size(), 6);

        let interval = Interval::new(5, 5);
        assert_eq!(interval.size(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_295 {
    use super::*;

use crate::*;

    #[test]
    fn test_start() {
        let interval = Interval::new(5, 10);
        assert_eq!(interval.start(), 5);

        let interval = Interval::new(0, 0);
        assert_eq!(interval.start(), 0);

        let interval = Interval::new(100, 200);
        assert_eq!(interval.start(), 100);
    }
}#[cfg(test)]
mod tests_llm_16_296 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_start_end() {
        let interval = Interval::new(5, 10);
        let (start, end) = interval.start_end();
        assert_eq!(start, 5);
        assert_eq!(end, 10);
    }
}#[cfg(test)]
mod tests_llm_16_297 {
    use crate::interval::Interval;
    use std::clone::Clone;
    use std::cmp::{Eq, PartialEq, max};

    #[test]
    fn test_suffix() {
        let interval1 = Interval::new(0, 5);
        let interval2 = Interval::new(2, 7);
        let result = interval1.suffix(interval2);
        assert_eq!(result.start, 2);
        assert_eq!(result.end, 7);
    }
}#[cfg(test)]
mod tests_llm_16_298 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_translate() {
        let interval = Interval::new(0, 5);
        let translated = interval.translate(2);
        assert_eq!(translated, Interval::new(2, 7));
    }
    
    #[test]
    fn test_translate_zero_amount() {
        let interval = Interval::new(3, 8);
        let translated = interval.translate(0);
        assert_eq!(translated, interval);
    }
    
    #[test]
    fn test_translate_negative_amount() {
        let interval = Interval::new(5, 10);
        let translated = interval.translate_neg(3);
        assert_eq!(translated, Interval::new(2, 7));
    }
    
    #[test]
    #[should_panic]
    fn test_translate_negative_panic() {
        let interval = Interval::new(3, 8);
        interval.translate_neg(5);
    }
}#[cfg(test)]
mod tests_llm_16_299 {
    use super::*;

use crate::*;

    #[test]
    fn test_translate_neg() {
        let interval = Interval::new(10, 20);
        let amount = 5;
        let result = interval.translate_neg(amount);
        let expected = Interval::new(5, 15);
        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_300 {
    use crate::interval::Interval;
    use std::ops::Range;
    use std::ops::RangeInclusive;
    use std::ops::RangeTo;
    use std::ops::RangeToInclusive;
    
    #[test]
    fn test_union() {
        let interval1 = Interval::new(1, 5);
        let interval2 = Interval::new(3, 7);
        let result = interval1.union(interval2);
        assert_eq!(result.start, 1);
        assert_eq!(result.end, 7);
    }

    #[test]
    fn test_union_with_empty() {
        let interval1 = Interval::new(1, 5);
        let interval2 = Interval::new(5, 3);
        let result = interval1.union(interval2);
        assert_eq!(result.start, 1);
        assert_eq!(result.end, 5);
    }

    #[test]
    fn test_union_with_empty_result() {
        let interval1 = Interval::new(1, 2);
        let interval2 = Interval::new(3, 4);
        let result = interval1.union(interval2);
        assert_eq!(result.start, 1);
        assert_eq!(result.end, 2);
    }

    #[test]
    fn test_union_with_range() {
        let interval1 = Interval::from(1..5);
        let interval2 = Interval::from(3..7);
        let result = interval1.union(interval2);
        assert_eq!(result.start, 1);
        assert_eq!(result.end, 7);
    }

    #[test]
    fn test_union_with_range_inclusive() {
        let interval1 = Interval::from(1..=5);
        let interval2 = Interval::from(3..=7);
        let result = interval1.union(interval2);
        assert_eq!(result.start, 1);
        assert_eq!(result.end, 8);
    }

    #[test]
    fn test_union_with_range_to() {
        let interval1 = Interval::from(..5);
        let interval2 = Interval::from(..7);
        let result = interval1.union(interval2);
        assert_eq!(result.start, 0);
        assert_eq!(result.end, 7);
    }

    #[test]
    fn test_union_with_range_to_inclusive() {
        let interval1 = Interval::from(..=5);
        let interval2 = Interval::from(..=7);
        let result = interval1.union(interval2);
        assert_eq!(result.start, 0);
        assert_eq!(result.end, 8);
    }
}