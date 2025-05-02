//! Methods for dynamic-dimensional arrays.
use crate::imp_prelude::*;
/// # Methods for Dynamic-Dimensional Arrays
impl<A, S> ArrayBase<S, IxDyn>
where
    S: Data<Elem = A>,
{
    /// Insert new array axis of length 1 at `axis`, modifying the shape and
    /// strides in-place.
    ///
    /// **Panics** if the axis is out of bounds.
    ///
    /// ```
    /// use ndarray::{Axis, arr2, arr3};
    ///
    /// let mut a = arr2(&[[1, 2, 3], [4, 5, 6]]).into_dyn();
    /// assert_eq!(a.shape(), &[2, 3]);
    ///
    /// a.insert_axis_inplace(Axis(1));
    /// assert_eq!(a, arr3(&[[[1, 2, 3]], [[4, 5, 6]]]).into_dyn());
    /// assert_eq!(a.shape(), &[2, 1, 3]);
    /// ```
    pub fn insert_axis_inplace(&mut self, axis: Axis) {
        assert!(axis.index() <= self.ndim());
        self.dim = self.dim.insert_axis(axis);
        self.strides = self.strides.insert_axis(axis);
    }
    /// Collapses the array to `index` along the axis and removes the axis,
    /// modifying the shape and strides in-place.
    ///
    /// **Panics** if `axis` or `index` is out of bounds.
    ///
    /// ```
    /// use ndarray::{Axis, arr1, arr2};
    ///
    /// let mut a = arr2(&[[1, 2, 3], [4, 5, 6]]).into_dyn();
    /// assert_eq!(a.shape(), &[2, 3]);
    ///
    /// a.index_axis_inplace(Axis(1), 1);
    /// assert_eq!(a, arr1(&[2, 5]).into_dyn());
    /// assert_eq!(a.shape(), &[2]);
    /// ```
    pub fn index_axis_inplace(&mut self, axis: Axis, index: usize) {
        self.collapse_axis(axis, index);
        self.dim = self.dim.remove_axis(axis);
        self.strides = self.strides.remove_axis(axis);
    }
}
#[cfg(test)]
mod tests_rug_1134 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(i32, i32, i32, i32, i32, i32, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = arr2(
                &[
                    [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2],
                    [rug_fuzz_3, rug_fuzz_4, rug_fuzz_5],
                ],
            )
            .into_dyn();
        let p1 = Axis(rug_fuzz_6);
        let p2: usize = rug_fuzz_7;
        <ArrayBase<_, _>>::index_axis_inplace(&mut p0, p1, p2);
        debug_assert_eq!(p0, arr1(& [2, 5]).into_dyn());
        debug_assert_eq!(p0.shape(), & [2]);
             }
});    }
}
