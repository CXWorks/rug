use std::char;
use std::cmp;
use std::fmt::Debug;
use std::slice;
use std::u8;
use unicode;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntervalSet<I> {
    ranges: Vec<I>,
}
impl<I: Interval> IntervalSet<I> {
    /// Create a new set from a sequence of intervals. Each interval is
    /// specified as a pair of bounds, where both bounds are inclusive.
    ///
    /// The given ranges do not need to be in any specific order, and ranges
    /// may overlap.
    pub fn new<T: IntoIterator<Item = I>>(intervals: T) -> IntervalSet<I> {
        let mut set = IntervalSet {
            ranges: intervals.into_iter().collect(),
        };
        set.canonicalize();
        set
    }
    /// Add a new interval to this set.
    pub fn push(&mut self, interval: I) {
        self.ranges.push(interval);
        self.canonicalize();
    }
    /// Return an iterator over all intervals in this set.
    ///
    /// The iterator yields intervals in ascending order.
    pub fn iter(&self) -> IntervalSetIter<I> {
        IntervalSetIter(self.ranges.iter())
    }
    /// Return an immutable slice of intervals in this set.
    ///
    /// The sequence returned is in canonical ordering.
    pub fn intervals(&self) -> &[I] {
        &self.ranges
    }
    /// Expand this interval set such that it contains all case folded
    /// characters. For example, if this class consists of the range `a-z`,
    /// then applying case folding will result in the class containing both the
    /// ranges `a-z` and `A-Z`.
    ///
    /// This returns an error if the necessary case mapping data is not
    /// available.
    pub fn case_fold_simple(&mut self) -> Result<(), unicode::CaseFoldError> {
        let len = self.ranges.len();
        for i in 0..len {
            let range = self.ranges[i];
            if let Err(err) = range.case_fold_simple(&mut self.ranges) {
                self.canonicalize();
                return Err(err);
            }
        }
        self.canonicalize();
        Ok(())
    }
    /// Union this set with the given set, in place.
    pub fn union(&mut self, other: &IntervalSet<I>) {
        self.ranges.extend(&other.ranges);
        self.canonicalize();
    }
    /// Intersect this set with the given set, in place.
    pub fn intersect(&mut self, other: &IntervalSet<I>) {
        if self.ranges.is_empty() {
            return;
        }
        if other.ranges.is_empty() {
            self.ranges.clear();
            return;
        }
        let drain_end = self.ranges.len();
        let mut ita = (0..drain_end).into_iter();
        let mut itb = (0..other.ranges.len()).into_iter();
        let mut a = ita.next().unwrap();
        let mut b = itb.next().unwrap();
        loop {
            if let Some(ab) = self.ranges[a].intersect(&other.ranges[b]) {
                self.ranges.push(ab);
            }
            let (it, aorb) = if self.ranges[a].upper() < other.ranges[b].upper() {
                (&mut ita, &mut a)
            } else {
                (&mut itb, &mut b)
            };
            match it.next() {
                Some(v) => *aorb = v,
                None => break,
            }
        }
        self.ranges.drain(..drain_end);
    }
    /// Subtract the given set from this set, in place.
    pub fn difference(&mut self, other: &IntervalSet<I>) {
        if self.ranges.is_empty() || other.ranges.is_empty() {
            return;
        }
        let drain_end = self.ranges.len();
        let (mut a, mut b) = (0, 0);
        'LOOP: while a < drain_end && b < other.ranges.len() {
            if other.ranges[b].upper() < self.ranges[a].lower() {
                b += 1;
                continue;
            }
            if self.ranges[a].upper() < other.ranges[b].lower() {
                let range = self.ranges[a];
                self.ranges.push(range);
                a += 1;
                continue;
            }
            assert!(! self.ranges[a].is_intersection_empty(& other.ranges[b]));
            let mut range = self.ranges[a];
            while b < other.ranges.len()
                && !range.is_intersection_empty(&other.ranges[b])
            {
                let old_range = range;
                range = match range.difference(&other.ranges[b]) {
                    (None, None) => {
                        a += 1;
                        continue 'LOOP;
                    }
                    (Some(range1), None) | (None, Some(range1)) => range1,
                    (Some(range1), Some(range2)) => {
                        self.ranges.push(range1);
                        range2
                    }
                };
                if other.ranges[b].upper() > old_range.upper() {
                    break;
                }
                b += 1;
            }
            self.ranges.push(range);
            a += 1;
        }
        while a < drain_end {
            let range = self.ranges[a];
            self.ranges.push(range);
            a += 1;
        }
        self.ranges.drain(..drain_end);
    }
    /// Compute the symmetric difference of the two sets, in place.
    ///
    /// This computes the symmetric difference of two interval sets. This
    /// removes all elements in this set that are also in the given set,
    /// but also adds all elements from the given set that aren't in this
    /// set. That is, the set will contain all elements in either set,
    /// but will not contain any elements that are in both sets.
    pub fn symmetric_difference(&mut self, other: &IntervalSet<I>) {
        let mut intersection = self.clone();
        intersection.intersect(other);
        self.union(other);
        self.difference(&intersection);
    }
    /// Negate this interval set.
    ///
    /// For all `x` where `x` is any element, if `x` was in this set, then it
    /// will not be in this set after negation.
    pub fn negate(&mut self) {
        if self.ranges.is_empty() {
            let (min, max) = (I::Bound::min_value(), I::Bound::max_value());
            self.ranges.push(I::create(min, max));
            return;
        }
        let drain_end = self.ranges.len();
        if self.ranges[0].lower() > I::Bound::min_value() {
            let upper = self.ranges[0].lower().decrement();
            self.ranges.push(I::create(I::Bound::min_value(), upper));
        }
        for i in 1..drain_end {
            let lower = self.ranges[i - 1].upper().increment();
            let upper = self.ranges[i].lower().decrement();
            self.ranges.push(I::create(lower, upper));
        }
        if self.ranges[drain_end - 1].upper() < I::Bound::max_value() {
            let lower = self.ranges[drain_end - 1].upper().increment();
            self.ranges.push(I::create(lower, I::Bound::max_value()));
        }
        self.ranges.drain(..drain_end);
    }
    /// Converts this set into a canonical ordering.
    fn canonicalize(&mut self) {
        if self.is_canonical() {
            return;
        }
        self.ranges.sort();
        assert!(! self.ranges.is_empty());
        let drain_end = self.ranges.len();
        for oldi in 0..drain_end {
            if self.ranges.len() > drain_end {
                let (last, rest) = self.ranges.split_last_mut().unwrap();
                if let Some(union) = last.union(&rest[oldi]) {
                    *last = union;
                    continue;
                }
            }
            let range = self.ranges[oldi];
            self.ranges.push(range);
        }
        self.ranges.drain(..drain_end);
    }
    /// Returns true if and only if this class is in a canonical ordering.
    fn is_canonical(&self) -> bool {
        for pair in self.ranges.windows(2) {
            if pair[0] >= pair[1] {
                return false;
            }
            if pair[0].is_contiguous(&pair[1]) {
                return false;
            }
        }
        true
    }
}
/// An iterator over intervals.
#[derive(Debug)]
pub struct IntervalSetIter<'a, I: 'a>(slice::Iter<'a, I>);
impl<'a, I> Iterator for IntervalSetIter<'a, I> {
    type Item = &'a I;
    fn next(&mut self) -> Option<&'a I> {
        self.0.next()
    }
}
pub trait Interval: Clone + Copy + Debug + Default + Eq + PartialEq + PartialOrd + Ord {
    type Bound: Bound;
    fn lower(&self) -> Self::Bound;
    fn upper(&self) -> Self::Bound;
    fn set_lower(&mut self, bound: Self::Bound);
    fn set_upper(&mut self, bound: Self::Bound);
    fn case_fold_simple(
        &self,
        intervals: &mut Vec<Self>,
    ) -> Result<(), unicode::CaseFoldError>;
    /// Create a new interval.
    fn create(lower: Self::Bound, upper: Self::Bound) -> Self {
        let mut int = Self::default();
        if lower <= upper {
            int.set_lower(lower);
            int.set_upper(upper);
        } else {
            int.set_lower(upper);
            int.set_upper(lower);
        }
        int
    }
    /// Union the given overlapping range into this range.
    ///
    /// If the two ranges aren't contiguous, then this returns `None`.
    fn union(&self, other: &Self) -> Option<Self> {
        if !self.is_contiguous(other) {
            return None;
        }
        let lower = cmp::min(self.lower(), other.lower());
        let upper = cmp::max(self.upper(), other.upper());
        Some(Self::create(lower, upper))
    }
    /// Intersect this range with the given range and return the result.
    ///
    /// If the intersection is empty, then this returns `None`.
    fn intersect(&self, other: &Self) -> Option<Self> {
        let lower = cmp::max(self.lower(), other.lower());
        let upper = cmp::min(self.upper(), other.upper());
        if lower <= upper { Some(Self::create(lower, upper)) } else { None }
    }
    /// Subtract the given range from this range and return the resulting
    /// ranges.
    ///
    /// If subtraction would result in an empty range, then no ranges are
    /// returned.
    fn difference(&self, other: &Self) -> (Option<Self>, Option<Self>) {
        if self.is_subset(other) {
            return (None, None);
        }
        if self.is_intersection_empty(other) {
            return (Some(self.clone()), None);
        }
        let add_lower = other.lower() > self.lower();
        let add_upper = other.upper() < self.upper();
        assert!(add_lower || add_upper);
        let mut ret = (None, None);
        if add_lower {
            let upper = other.lower().decrement();
            ret.0 = Some(Self::create(self.lower(), upper));
        }
        if add_upper {
            let lower = other.upper().increment();
            let range = Self::create(lower, self.upper());
            if ret.0.is_none() {
                ret.0 = Some(range);
            } else {
                ret.1 = Some(range);
            }
        }
        ret
    }
    /// Compute the symmetric difference the given range from this range. This
    /// returns the union of the two ranges minus its intersection.
    fn symmetric_difference(&self, other: &Self) -> (Option<Self>, Option<Self>) {
        let union = match self.union(other) {
            None => return (Some(self.clone()), Some(other.clone())),
            Some(union) => union,
        };
        let intersection = match self.intersect(other) {
            None => return (Some(self.clone()), Some(other.clone())),
            Some(intersection) => intersection,
        };
        union.difference(&intersection)
    }
    /// Returns true if and only if the two ranges are contiguous. Two ranges
    /// are contiguous if and only if the ranges are either overlapping or
    /// adjacent.
    fn is_contiguous(&self, other: &Self) -> bool {
        let lower1 = self.lower().as_u32();
        let upper1 = self.upper().as_u32();
        let lower2 = other.lower().as_u32();
        let upper2 = other.upper().as_u32();
        cmp::max(lower1, lower2) <= cmp::min(upper1, upper2).saturating_add(1)
    }
    /// Returns true if and only if the intersection of this range and the
    /// other range is empty.
    fn is_intersection_empty(&self, other: &Self) -> bool {
        let (lower1, upper1) = (self.lower(), self.upper());
        let (lower2, upper2) = (other.lower(), other.upper());
        cmp::max(lower1, lower2) > cmp::min(upper1, upper2)
    }
    /// Returns true if and only if this range is a subset of the other range.
    fn is_subset(&self, other: &Self) -> bool {
        let (lower1, upper1) = (self.lower(), self.upper());
        let (lower2, upper2) = (other.lower(), other.upper());
        (lower2 <= lower1 && lower1 <= upper2) && (lower2 <= upper1 && upper1 <= upper2)
    }
}
pub trait Bound: Copy + Clone + Debug + Eq + PartialEq + PartialOrd + Ord {
    fn min_value() -> Self;
    fn max_value() -> Self;
    fn as_u32(self) -> u32;
    fn increment(self) -> Self;
    fn decrement(self) -> Self;
}
impl Bound for u8 {
    fn min_value() -> Self {
        u8::MIN
    }
    fn max_value() -> Self {
        u8::MAX
    }
    fn as_u32(self) -> u32 {
        self as u32
    }
    fn increment(self) -> Self {
        self.checked_add(1).unwrap()
    }
    fn decrement(self) -> Self {
        self.checked_sub(1).unwrap()
    }
}
impl Bound for char {
    fn min_value() -> Self {
        '\x00'
    }
    fn max_value() -> Self {
        '\u{10FFFF}'
    }
    fn as_u32(self) -> u32 {
        self as u32
    }
    fn increment(self) -> Self {
        match self {
            '\u{D7FF}' => '\u{E000}',
            c => char::from_u32((c as u32).checked_add(1).unwrap()).unwrap(),
        }
    }
    fn decrement(self) -> Self {
        match self {
            '\u{E000}' => '\u{D7FF}',
            c => char::from_u32((c as u32).checked_sub(1).unwrap()).unwrap(),
        }
    }
}
#[cfg(test)]
mod tests_llm_16_29 {
    use crate::hir::interval::Bound;
    #[test]
    fn test_as_u32() {
        let _rug_st_tests_llm_16_29_rrrruuuugggg_test_as_u32 = 0;
        let rug_fuzz_0 = 'a';
        let bound: char = rug_fuzz_0;
        let result = bound.as_u32();
        debug_assert_eq!(result, 97);
        let _rug_ed_tests_llm_16_29_rrrruuuugggg_test_as_u32 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_30 {
    use crate::hir::interval::Bound;
    #[test]
    fn test_decrement() {
        let _rug_st_tests_llm_16_30_rrrruuuugggg_test_decrement = 0;
        let rug_fuzz_0 = '\u{E000}';
        let rug_fuzz_1 = 'a';
        let rug_fuzz_2 = 'A';
        let rug_fuzz_3 = '0';
        let rug_fuzz_4 = '\u{D7FF}';
        let rug_fuzz_5 = '`';
        let rug_fuzz_6 = '@';
        let rug_fuzz_7 = '/';
        let rug_fuzz_8 = '\u{0000}';
        debug_assert_eq!(rug_fuzz_0.decrement(), '\u{D7FF}');
        debug_assert_eq!(rug_fuzz_1.decrement(), '`');
        debug_assert_eq!(rug_fuzz_2.decrement(), '@');
        debug_assert_eq!(rug_fuzz_3.decrement(), '/');
        debug_assert_eq!(rug_fuzz_4.decrement(), '\u{D7FE}');
        debug_assert_eq!(rug_fuzz_5.decrement(), '_');
        debug_assert_eq!(rug_fuzz_6.decrement(), '?');
        debug_assert_eq!(rug_fuzz_7.decrement(), '.');
        debug_assert_eq!(rug_fuzz_8.decrement(), '\u{0000}');
        let _rug_ed_tests_llm_16_30_rrrruuuugggg_test_decrement = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_32_llm_16_31 {
    use super::*;
    use crate::*;
    use crate::hir::interval::Bound;
    #[test]
    fn test_increment() {
        let _rug_st_tests_llm_16_32_llm_16_31_rrrruuuugggg_test_increment = 0;
        let rug_fuzz_0 = 'A';
        let rug_fuzz_1 = '\u{D7FF}';
        let rug_fuzz_2 = 'z';
        let rug_fuzz_3 = '9';
        debug_assert_eq!(rug_fuzz_0.increment(), 'B');
        debug_assert_eq!(rug_fuzz_1.increment(), '\u{E000}');
        debug_assert_eq!(rug_fuzz_2.increment(), '{');
        debug_assert_eq!(rug_fuzz_3.increment(), ':');
        let _rug_ed_tests_llm_16_32_llm_16_31_rrrruuuugggg_test_increment = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_33 {
    use super::*;
    use crate::*;
    use crate::hir::interval::Bound;
    #[test]
    fn test_max_value() {
        let _rug_st_tests_llm_16_33_rrrruuuugggg_test_max_value = 0;
        let result = <char as Bound>::max_value();
        debug_assert_eq!(result, '\u{10FFFF}');
        let _rug_ed_tests_llm_16_33_rrrruuuugggg_test_max_value = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_35_llm_16_34 {
    #[test]
    fn test_min_value() {
        let _rug_st_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_min_value = 0;
        let rug_fuzz_0 = '\x00';
        let min_value = rug_fuzz_0;
        debug_assert_eq!(min_value, '\x00');
        let _rug_ed_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_min_value = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_67 {
    use super::*;
    use crate::*;
    use std::slice;
    fn create_iter<'a, I>(slice: &'a [I]) -> IntervalSetIter<'a, I> {
        IntervalSetIter(slice.iter())
    }
    #[test]
    fn test_next() {
        let mut iter = create_iter::<i32>(&[]);
        assert_eq!(iter.next(), None);
        let mut iter = create_iter(&[10]);
        assert_eq!(iter.next(), Some(& 10));
        assert_eq!(iter.next(), None);
        let mut iter = create_iter(&[1, 2, 3]);
        assert_eq!(iter.next(), Some(& 1));
        assert_eq!(iter.next(), Some(& 2));
        assert_eq!(iter.next(), Some(& 3));
        assert_eq!(iter.next(), None);
    }
}
#[cfg(test)]
mod tests_llm_16_95 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_u32() {
        let _rug_st_tests_llm_16_95_rrrruuuugggg_test_as_u32 = 0;
        let rug_fuzz_0 = 42;
        let bound: u8 = rug_fuzz_0;
        let result = bound.as_u32();
        debug_assert_eq!(result, 42u32);
        let _rug_ed_tests_llm_16_95_rrrruuuugggg_test_as_u32 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_96 {
    use super::*;
    use crate::*;
    #[test]
    fn test_decrement() {
        let _rug_st_tests_llm_16_96_rrrruuuugggg_test_decrement = 0;
        let rug_fuzz_0 = 5;
        let bound: u8 = rug_fuzz_0;
        let result = bound.decrement();
        debug_assert_eq!(result, 4);
        let _rug_ed_tests_llm_16_96_rrrruuuugggg_test_decrement = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_97 {
    use super::*;
    use crate::*;
    #[test]
    fn test_increment() {
        let _rug_st_tests_llm_16_97_rrrruuuugggg_test_increment = 0;
        let rug_fuzz_0 = 100;
        let value: u8 = rug_fuzz_0;
        debug_assert_eq!(value.increment(), 101);
        let _rug_ed_tests_llm_16_97_rrrruuuugggg_test_increment = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_441 {
    use super::*;
    use crate::*;
    #[test]
    fn test_iter() {
        let _rug_st_tests_llm_16_441_rrrruuuugggg_test_iter = 0;
        let _rug_ed_tests_llm_16_441_rrrruuuugggg_test_iter = 0;
    }
}
#[cfg(test)]
mod tests_rug_229 {
    use super::*;
    use crate::hir::ClassBytesRange;
    #[test]
    fn test_union() {
        let _rug_st_tests_rug_229_rrrruuuugggg_test_union = 0;
        let rug_fuzz_0 = 0x41;
        let rug_fuzz_1 = 0x7A;
        let rug_fuzz_2 = 0x41;
        let rug_fuzz_3 = 0x7A;
        let mut p0 = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3);
        p0.union(&p1);
        let _rug_ed_tests_rug_229_rrrruuuugggg_test_union = 0;
    }
}
#[cfg(test)]
mod tests_rug_230 {
    use super::*;
    use crate::hir::ClassUnicodeRange;
    #[test]
    fn test_intersect() {
        let _rug_st_tests_rug_230_rrrruuuugggg_test_intersect = 0;
        let rug_fuzz_0 = '\u{80}';
        let rug_fuzz_1 = '\u{80}';
        let rug_fuzz_2 = '\u{80}';
        let rug_fuzz_3 = '\u{80}';
        let mut p0 = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = ClassUnicodeRange::new(rug_fuzz_2, rug_fuzz_3);
        let result = p0.intersect(&p1);
        debug_assert_eq!(result, Some(ClassUnicodeRange::new('\u{80}', '\u{80}')));
        let _rug_ed_tests_rug_230_rrrruuuugggg_test_intersect = 0;
    }
}
#[cfg(test)]
mod tests_rug_231 {
    use super::*;
    use crate::hir::ClassBytesRange;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_231_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x41;
        let rug_fuzz_1 = 0x7A;
        let rug_fuzz_2 = 0x41;
        let rug_fuzz_3 = 0x7A;
        let mut p0 = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3);
        crate::hir::interval::Interval::difference(&p0, &p1);
        let _rug_ed_tests_rug_231_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_232 {
    use super::*;
    use crate::hir::interval::Interval;
    use crate::hir::ClassUnicodeRange;
    #[test]
    fn test_symmetric_difference() {
        let _rug_st_tests_rug_232_rrrruuuugggg_test_symmetric_difference = 0;
        let rug_fuzz_0 = '\u{80}';
        let rug_fuzz_1 = '\u{80}';
        let rug_fuzz_2 = '\u{80}';
        let rug_fuzz_3 = '\u{80}';
        let mut p0 = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = ClassUnicodeRange::new(rug_fuzz_2, rug_fuzz_3);
        Interval::symmetric_difference(&p0, &p1);
        let _rug_ed_tests_rug_232_rrrruuuugggg_test_symmetric_difference = 0;
    }
}
#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::hir::ClassBytesRange;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_233_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0x41;
        let rug_fuzz_1 = 0x7A;
        let rug_fuzz_2 = 0x41;
        let rug_fuzz_3 = 0x7A;
        let mut v0 = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut v1 = ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3);
        crate::hir::interval::Interval::is_contiguous(&v0, &v1);
        let _rug_ed_tests_rug_233_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_234 {
    use super::*;
    use crate::hir::ClassUnicodeRange;
    #[test]
    fn test_is_intersection_empty() {
        let _rug_st_tests_rug_234_rrrruuuugggg_test_is_intersection_empty = 0;
        let rug_fuzz_0 = '\u{80}';
        let rug_fuzz_1 = '\u{80}';
        let rug_fuzz_2 = '\u{80}';
        let rug_fuzz_3 = '\u{80}';
        let p0 = ClassUnicodeRange::new(rug_fuzz_0, rug_fuzz_1);
        let p1 = ClassUnicodeRange::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(
            crate ::hir::interval::Interval::is_intersection_empty(& p0, & p1), false
        );
        let _rug_ed_tests_rug_234_rrrruuuugggg_test_is_intersection_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    use crate::hir::ClassBytesRange;
    #[test]
    fn test_is_subset() {
        let _rug_st_tests_rug_235_rrrruuuugggg_test_is_subset = 0;
        let rug_fuzz_0 = 0x41;
        let rug_fuzz_1 = 0x7A;
        let rug_fuzz_2 = 0x41;
        let rug_fuzz_3 = 0x7A;
        let mut p0 = ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1);
        let mut p1 = ClassBytesRange::new(rug_fuzz_2, rug_fuzz_3);
        debug_assert!(p0.is_subset(& p1));
        let _rug_ed_tests_rug_235_rrrruuuugggg_test_is_subset = 0;
    }
}
#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    use crate::hir::interval::Bound;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_247_rrrruuuugggg_test_rug = 0;
        <u8 as Bound>::min_value();
        let _rug_ed_tests_rug_247_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_248 {
    use super::*;
    use crate::hir::interval::Bound;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_248_rrrruuuugggg_test_rug = 0;
        <u8 as Bound>::max_value();
        let _rug_ed_tests_rug_248_rrrruuuugggg_test_rug = 0;
    }
}
