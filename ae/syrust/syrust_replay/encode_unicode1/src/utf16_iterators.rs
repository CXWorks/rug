use traits::CharExt;
use utf16_char::Utf16Char;
use errors::EmptyStrError;
extern crate core;
use self::core::fmt;
use self::core::borrow::Borrow;
const FIRST_USED: u16 = 0x_dc_00;
const SECOND_USED: u16 = 0;
/// Iterate over the units of the UTF-16 representation of a codepoint.
#[derive(Clone)]
pub struct Utf16Iterator {
    first: u16,
    second: u16,
}
impl From<char> for Utf16Iterator {
    fn from(c: char) -> Self {
        let (first, second) = c.to_utf16_tuple();
        Utf16Iterator {
            first: first,
            second: second.unwrap_or(SECOND_USED),
        }
    }
}
impl From<Utf16Char> for Utf16Iterator {
    fn from(uc: Utf16Char) -> Self {
        let (first, second) = uc.to_tuple();
        Utf16Iterator {
            first: first,
            second: second.unwrap_or(SECOND_USED),
        }
    }
}
impl Iterator for Utf16Iterator {
    type Item = u16;
    fn next(&mut self) -> Option<u16> {
        match (self.first, self.second) {
            (FIRST_USED, SECOND_USED) => None,
            (FIRST_USED, second) => {
                self.second = SECOND_USED;
                Some(second)
            }
            (first, _) => {
                self.first = FIRST_USED;
                Some(first)
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}
impl ExactSizeIterator for Utf16Iterator {
    fn len(&self) -> usize {
        (if self.first == FIRST_USED { 0 } else { 1 })
            + (if self.second == SECOND_USED { 0 } else { 1 })
    }
}
impl fmt::Debug for Utf16Iterator {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        let mut clone = self.clone();
        match (clone.next(), clone.next()) {
            (Some(one), None) => write!(fmtr, "[{}]", one),
            (Some(a), Some(b)) => write!(fmtr, "[{}, {}]", a, b),
            (None, _) => write!(fmtr, "[]"),
        }
    }
}
/// Converts an iterator of `Utf16Char` (or `&Utf16Char`)
/// to an iterator of `u16`s.
/// Is equivalent to calling `.flat_map()` on the original iterator,
/// but the returned iterator is about twice as fast.
///
/// The exact number of units cannot be known in advance, but `size_hint()`
/// gives the possible range.
///
/// # Examples
///
/// From iterator of values:
///
/// ```
/// use encode_unicode::{iter_units, CharExt};
///
/// let iterator = "foo".chars().map(|c| c.to_utf16() );
/// let mut units = [0; 4];
/// for (u,dst) in iter_units(iterator).zip(&mut units) {*dst=u;}
/// assert_eq!(units, ['f' as u16, 'o' as u16, 'o' as u16, 0]);
/// ```
///
/// From iterator of references:
///
#[cfg_attr(feature = "std", doc = " ```")]
#[cfg_attr(not(feature = "std"), doc = " ```no_compile")]
/// use encode_unicode::{iter_units, CharExt, Utf16Char};
///
/// // (💣 takes two units)
/// let chars: Vec<Utf16Char> = "💣 bomb 💣".chars().map(|c| c.to_utf16() ).collect();
/// let units: Vec<u16> = iter_units(&chars).collect();
/// let flat_map: Vec<u16> = chars.iter().flat_map(|u16c| *u16c ).collect();
/// assert_eq!(units, flat_map);
/// ```
pub fn iter_units<U: Borrow<Utf16Char>, I: IntoIterator<Item = U>>(
    iterable: I,
) -> Utf16CharSplitter<U, I::IntoIter> {
    Utf16CharSplitter {
        inner: iterable.into_iter(),
        prev_second: 0,
    }
}
/// The iterator type returned by `iter_units()`
#[derive(Clone)]
pub struct Utf16CharSplitter<U: Borrow<Utf16Char>, I: Iterator<Item = U>> {
    inner: I,
    prev_second: u16,
}
impl<I: Iterator<Item = Utf16Char>> From<I> for Utf16CharSplitter<Utf16Char, I> {
    /// A less generic constructor than `iter_units()`
    fn from(iter: I) -> Self {
        iter_units(iter)
    }
}
impl<U: Borrow<Utf16Char>, I: Iterator<Item = U>> Utf16CharSplitter<U, I> {
    /// Extracts the source iterator.
    ///
    /// Note that `iter_units(iter.into_inner())` is not a no-op:
    /// If the last returned unit from `next()` was a leading surrogate,
    /// the trailing surrogate is lost.
    pub fn into_inner(self) -> I {
        self.inner
    }
}
impl<U: Borrow<Utf16Char>, I: Iterator<Item = U>> Iterator for Utf16CharSplitter<U, I> {
    type Item = u16;
    fn next(&mut self) -> Option<Self::Item> {
        if self.prev_second == 0 {
            self.inner
                .next()
                .map(|u16c| {
                    let units = u16c.borrow().to_array();
                    self.prev_second = units[1];
                    units[0]
                })
        } else {
            let prev_second = self.prev_second;
            self.prev_second = 0;
            Some(prev_second)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.inner.size_hint();
        let add = if self.prev_second == 0 { 0 } else { 1 };
        (min.wrapping_add(add), max.map(|max| max.wrapping_mul(2).wrapping_add(add)))
    }
}
/// An iterator over the codepoints in a `str` represented as `Utf16Char`.
#[derive(Clone)]
pub struct Utf16CharIndices<'a> {
    str: &'a str,
    index: usize,
}
impl<'a> From<&'a str> for Utf16CharIndices<'a> {
    fn from(s: &str) -> Utf16CharIndices {
        Utf16CharIndices {
            str: s,
            index: 0,
        }
    }
}
impl<'a> Utf16CharIndices<'a> {
    /// Extract the remainder of the source `str`.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::{StrExt, Utf16Char};
    /// let mut iter = "abc".utf16char_indices();
    /// assert_eq!(iter.next_back(), Some((2, Utf16Char::from('c'))));
    /// assert_eq!(iter.next(), Some((0, Utf16Char::from('a'))));
    /// assert_eq!(iter.as_str(), "b");
    /// ```
    pub fn as_str(&self) -> &'a str {
        &self.str[self.index..]
    }
}
impl<'a> Iterator for Utf16CharIndices<'a> {
    type Item = (usize, Utf16Char);
    fn next(&mut self) -> Option<(usize, Utf16Char)> {
        match Utf16Char::from_str_start(&self.str[self.index..]) {
            Ok((u16c, bytes)) => {
                let item = (self.index, u16c);
                self.index += bytes;
                Some(item)
            }
            Err(EmptyStrError) => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.str.len() - self.index;
        (len.wrapping_add(3) / 4, Some(len))
    }
}
impl<'a> DoubleEndedIterator for Utf16CharIndices<'a> {
    fn next_back(&mut self) -> Option<(usize, Utf16Char)> {
        if self.index < self.str.len() {
            let rev = self.str.bytes().rev();
            let len = 1 + rev.take_while(|b| b & 0b1100_0000 == 0b1000_0000).count();
            let starts = self.str.len() - len;
            let (u16c, _) = Utf16Char::from_str_start(&self.str[starts..]).unwrap();
            self.str = &self.str[..starts];
            Some((starts, u16c))
        } else {
            None
        }
    }
}
impl<'a> fmt::Debug for Utf16CharIndices<'a> {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.debug_tuple("Utf16CharIndices")
            .field(&self.index)
            .field(&self.as_str())
            .finish()
    }
}
/// An iterator over the codepoints in a `str` represented as `Utf16Char`.
#[derive(Clone)]
pub struct Utf16Chars<'a>(Utf16CharIndices<'a>);
impl<'a> From<&'a str> for Utf16Chars<'a> {
    fn from(s: &str) -> Utf16Chars {
        Utf16Chars(Utf16CharIndices::from(s))
    }
}
impl<'a> Utf16Chars<'a> {
    /// Extract the remainder of the source `str`.
    ///
    /// # Examples
    ///
    /// ```
    /// use encode_unicode::{StrExt, Utf16Char};
    /// let mut iter = "abc".utf16chars();
    /// assert_eq!(iter.next(), Some(Utf16Char::from('a')));
    /// assert_eq!(iter.next_back(), Some(Utf16Char::from('c')));
    /// assert_eq!(iter.as_str(), "b");
    /// ```
    pub fn as_str(&self) -> &'a str {
        self.0.as_str()
    }
}
impl<'a> Iterator for Utf16Chars<'a> {
    type Item = Utf16Char;
    fn next(&mut self) -> Option<Utf16Char> {
        self.0.next().map(|(_, u16c)| u16c)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<'a> DoubleEndedIterator for Utf16Chars<'a> {
    fn next_back(&mut self) -> Option<Utf16Char> {
        self.0.next_back().map(|(_, u16c)| u16c)
    }
}
impl<'a> fmt::Debug for Utf16Chars<'a> {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.debug_tuple("Utf16Chars").field(&self.as_str()).finish()
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use crate::{iter_units, CharExt, Utf16Char};
    #[test]
    fn test_iter_units() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, u16, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let iterator = rug_fuzz_0.chars().map(|c| c.to_utf16());
        let mut units = [rug_fuzz_1; 4];
        for (u, dst) in iter_units(iterator).zip(&mut units) {
            *dst = u;
        }
        debug_assert_eq!(units, ['f' as u16, 'o' as u16, 'o' as u16, 0]);
        let chars: Vec<Utf16Char> = rug_fuzz_2.chars().map(|c| c.to_utf16()).collect();
        let units: Vec<u16> = iter_units(&chars).collect();
        let flat_map: Vec<u16> = chars.iter().flat_map(|u16c| *u16c).collect();
        debug_assert_eq!(units, flat_map);
             }
}
}
}    }
}
use super::*;
#[cfg(test)]
mod tests_rug_57 {
    use crate::std::convert::From;
    use crate::utf16_iterators::{Utf16Iterator, SECOND_USED};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: char = rug_fuzz_0;
        <Utf16Iterator as From<char>>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::{Utf16Char, utf16_char};
    use crate::std::convert::From;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_58_rrrruuuugggg_test_rug = 0;
        let mut p0 = utf16_char::Utf16Char::default();
        <utf16_iterators::Utf16Iterator as std::convert::From<
            utf16_char::Utf16Char,
        >>::from(p0);
        let _rug_ed_tests_rug_58_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::std::iter::Iterator;
    use crate::iterator::Utf16Iterator;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Utf16Iterator::from(rug_fuzz_0);
        debug_assert_eq!(
            < utf16_iterators::Utf16Iterator as std::iter::Iterator > ::next(& mut p0),
            Some(65)
        );
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::iterator::Utf16Iterator;
    use crate::std::iter::Iterator;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Utf16Iterator::from(rug_fuzz_0);
        <utf16_iterators::Utf16Iterator as std::iter::Iterator>::size_hint(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use crate::std::iter::ExactSizeIterator;
    use crate::iterator::Utf16Iterator;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Utf16Iterator::from(rug_fuzz_0);
        <utf16_iterators::Utf16Iterator as std::iter::ExactSizeIterator>::len(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_66 {
    use super::*;
    use crate::std::convert::From;
    use utf16_iterators::Utf16CharIndices;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        <Utf16CharIndices<'_> as From<&str>>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_69 {
    use super::*;
    use crate::std::iter::Iterator;
    use utf16_iterators::Utf16CharIndices;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let string = String::from(rug_fuzz_0);
        let iterator = Utf16CharIndices {
            str: &string,
            index: rug_fuzz_1,
        };
        let mut p0 = &iterator;
        p0.size_hint();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use crate::utf16_iterators::{Utf16Chars, Utf16CharIndices};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        <Utf16Chars>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use crate::{StrExt, Utf16Char};
    #[test]
    fn test_as_str() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let s = rug_fuzz_0;
        let mut iter = s.utf16chars();
        debug_assert_eq!(iter.next(), Some(Utf16Char::from('a')));
        debug_assert_eq!(iter.next_back(), Some(Utf16Char::from('c')));
        debug_assert_eq!(iter.as_str(), "b");
             }
}
}
}    }
}
