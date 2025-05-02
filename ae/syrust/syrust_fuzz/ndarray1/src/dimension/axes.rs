use crate::{Axis, Dimension, Ix, Ixs};
/// Create a new Axes iterator
pub fn axes_of<'a, D>(d: &'a D, strides: &'a D) -> Axes<'a, D>
where
    D: Dimension,
{
    Axes {
        dim: d,
        strides,
        start: 0,
        end: d.ndim(),
    }
}
/// An iterator over the length and stride of each axis of an array.
///
/// See [`.axes()`](../struct.ArrayBase.html#method.axes) for more information.
///
/// Iterator element type is `AxisDescription`.
///
/// # Examples
///
/// ```
/// use ndarray::Array3;
/// use ndarray::Axis;
///
/// let a = Array3::<f32>::zeros((3, 5, 4));
///
/// let largest_axis = a.axes()
///                     .max_by_key(|ax| ax.len())
///                     .unwrap().axis();
/// assert_eq!(largest_axis, Axis(1));
/// ```
#[derive(Debug)]
pub struct Axes<'a, D> {
    dim: &'a D,
    strides: &'a D,
    start: usize,
    end: usize,
}
/// Description of the axis, its length and its stride.
#[derive(Debug)]
pub struct AxisDescription(pub Axis, pub Ix, pub Ixs);
copy_and_clone!(AxisDescription);
#[allow(clippy::len_without_is_empty)]
impl AxisDescription {
    /// Return axis
    #[inline(always)]
    pub fn axis(self) -> Axis {
        self.0
    }
    /// Return length
    #[inline(always)]
    pub fn len(self) -> Ix {
        self.1
    }
    /// Return stride
    #[inline(always)]
    pub fn stride(self) -> Ixs {
        self.2
    }
}
copy_and_clone!(['a, D] Axes <'a, D >);
impl<'a, D> Iterator for Axes<'a, D>
where
    D: Dimension,
{
    /// Description of the axis, its length and its stride.
    type Item = AxisDescription;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let i = self.start.post_inc();
            Some(AxisDescription(Axis(i), self.dim[i], self.strides[i] as Ixs))
        } else {
            None
        }
    }
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, AxisDescription) -> B,
    {
        (self.start..self.end)
            .map(move |i| AxisDescription(
                Axis(i),
                self.dim[i],
                self.strides[i] as isize,
            ))
            .fold(init, f)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end - self.start;
        (len, Some(len))
    }
}
impl<'a, D> DoubleEndedIterator for Axes<'a, D>
where
    D: Dimension,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let i = self.end.pre_dec();
            Some(AxisDescription(Axis(i), self.dim[i], self.strides[i] as Ixs))
        } else {
            None
        }
    }
}
trait IncOps: Copy {
    fn post_inc(&mut self) -> Self;
    fn post_dec(&mut self) -> Self;
    fn pre_dec(&mut self) -> Self;
}
impl IncOps for usize {
    #[inline(always)]
    fn post_inc(&mut self) -> Self {
        let x = *self;
        *self += 1;
        x
    }
    #[inline(always)]
    fn post_dec(&mut self) -> Self {
        let x = *self;
        *self -= 1;
        x
    }
    #[inline(always)]
    fn pre_dec(&mut self) -> Self {
        *self -= 1;
        *self
    }
}
#[cfg(test)]
mod tests_rug_248 {
    use super::*;
    use crate::prelude::{IxDyn, Dimension};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = IxDyn::zeros(rug_fuzz_1);
        crate::dimension::axes::axes_of(&p0, &p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::dimension::axes::AxisDescription;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = AxisDescription(Axis(rug_fuzz_0), rug_fuzz_1, rug_fuzz_2);
        crate::dimension::axes::AxisDescription::axis(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_250 {
    use super::*;
    use crate::dimension::axes::AxisDescription;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_250_rrrruuuugggg_test_rug = 0;
        let mut p0: AxisDescription = todo!();
        crate::dimension::axes::AxisDescription::len(p0);
        let _rug_ed_tests_rug_250_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_251 {
    use super::*;
    use crate::dimension::axes::AxisDescription;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_251_rrrruuuugggg_test_rug = 0;
        let mut p0: AxisDescription = unimplemented!();
        p0.stride();
        let _rug_ed_tests_rug_251_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_256 {
    use super::*;
    use crate::dimension::axes::IncOps;
    #[test]
    fn test_post_inc() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let result = usize::post_inc(&mut p0);
        debug_assert_eq!(result, 5);
        debug_assert_eq!(p0, 6);
             }
});    }
}
#[cfg(test)]
mod tests_rug_257 {
    use super::*;
    use crate::dimension::axes::IncOps;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        debug_assert_eq!(< usize as IncOps > ::post_dec(& mut p0), 5);
        debug_assert_eq!(p0, 4);
             }
});    }
}
#[cfg(test)]
mod tests_rug_258 {
    use super::*;
    use crate::dimension::axes::IncOps;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        debug_assert_eq!(< usize as IncOps > ::pre_dec(& mut p0), 4);
             }
});    }
}
