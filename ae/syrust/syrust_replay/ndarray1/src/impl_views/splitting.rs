use crate::imp_prelude::*;
use crate::slice::MultiSlice;
/// Methods for read-only array views.
impl<'a, A, D> ArrayView<'a, A, D>
where
    D: Dimension,
{
    /// Split the array view along `axis` and return one view strictly before the
    /// split and one view after the split.
    ///
    /// **Panics** if `axis` or `index` is out of bounds.
    ///
    /// **Examples:**
    /// ```rust
    /// # use ndarray::prelude::*;
    /// let a = aview2(&[[0, 1, 2, 3],
    ///                  [4, 5, 6, 7],
    ///                  [8, 9, 0, 1]]);
    ///
    /// ```
    /// The array view `a` has two axes and shape 3 × 4:
    /// ```text
    ///          ──▶ Axis(1)
    ///         ┌─────┬─────┬─────┬─────┐ 0
    ///       │ │ a₀₀ │ a₀₁ │ a₀₂ │ a₀₃ │
    ///       ▼ ├─────┼─────┼─────┼─────┤ 1
    ///  Axis(0)│ a₁₀ │ a₁₁ │ a₁₂ │ a₁₃ │
    ///         ├─────┼─────┼─────┼─────┤ 2
    ///         │ a₂₀ │ a₂₁ │ a₂₂ │ a₂₃ │
    ///         └─────┴─────┴─────┴─────┘ 3 ↑
    ///         0     1     2     3     4 ← possible split_at indices.
    /// ```
    ///
    /// Row indices increase along `Axis(0)`, and column indices increase along
    /// `Axis(1)`. Note that we split “before” an element index, and that
    /// both 0 and the endpoint are valid split indices.
    ///
    /// **Example 1**: Split `a` along the first axis, in this case the rows, at
    /// index 2.<br>
    /// This produces views v1 and v2 of shapes 2 × 4 and 1 × 4:
    ///
    /// ```rust
    /// # use ndarray::prelude::*;
    /// # let a = aview2(&[[0; 4]; 3]);
    /// let (v1, v2) = a.split_at(Axis(0), 1);
    /// ```
    /// ```text
    ///         ┌─────┬─────┬─────┬─────┐       0  ↓ indices
    ///         │ a₀₀ │ a₀₁ │ a₀₂ │ a₀₃ │            along Axis(0)
    ///         ├─────┼─────┼─────┼─────┤ v1    1
    ///         │ a₁₀ │ a₁₁ │ a₁₂ │ a₁₃ │
    ///         └─────┴─────┴─────┴─────┘
    ///         ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄       2
    ///         ┌─────┬─────┬─────┬─────┐
    ///         │ a₂₀ │ a₂₁ │ a₂₂ │ a₂₃ │ v2
    ///         └─────┴─────┴─────┴─────┘       3
    /// ```
    ///
    /// **Example 2**: Split `a` along the second axis, in this case the
    /// columns, at index 2.<br>
    /// This produces views u1 and u2 of shapes 3 × 2 and 3 × 2:
    ///
    /// ```rust
    /// # use ndarray::prelude::*;
    /// # let a = aview2(&[[0; 4]; 3]);
    /// let (u1, u2) = a.split_at(Axis(1), 2);
    ///
    /// ```
    /// ```text
    ///              u1             u2
    ///         ┌─────┬─────┐┊┌─────┬─────┐
    ///         │ a₀₀ │ a₀₁ │┊│ a₀₂ │ a₀₃ │
    ///         ├─────┼─────┤┊├─────┼─────┤
    ///         │ a₁₀ │ a₁₁ │┊│ a₁₂ │ a₁₃ │
    ///         ├─────┼─────┤┊├─────┼─────┤
    ///         │ a₂₀ │ a₂₁ │┊│ a₂₂ │ a₂₃ │
    ///         └─────┴─────┘┊└─────┴─────┘
    ///         0     1      2      3     4  indices →
    ///                                      along Axis(1)
    /// ```
    pub fn split_at(self, axis: Axis, index: Ix) -> (Self, Self) {
        unsafe {
            let (left, right) = self.into_raw_view().split_at(axis, index);
            (left.deref_into_view(), right.deref_into_view())
        }
    }
}
/// Methods for read-write array views.
impl<'a, A, D> ArrayViewMut<'a, A, D>
where
    D: Dimension,
{
    /// Split the array view along `axis` and return one mutable view strictly
    /// before the split and one mutable view after the split.
    ///
    /// **Panics** if `axis` or `index` is out of bounds.
    pub fn split_at(self, axis: Axis, index: Ix) -> (Self, Self) {
        unsafe {
            let (left, right) = self.into_raw_view_mut().split_at(axis, index);
            (left.deref_into_view_mut(), right.deref_into_view_mut())
        }
    }
    /// Split the view into multiple disjoint slices.
    ///
    /// This is similar to [`.multi_slice_mut()`], but `.multi_slice_move()`
    /// consumes `self` and produces views with lifetimes matching that of
    /// `self`.
    ///
    /// See [*Slicing*](#slicing) for full documentation.
    /// See also [`SliceInfo`] and [`D::SliceArg`].
    ///
    /// [`.multi_slice_mut()`]: struct.ArrayBase.html#method.multi_slice_mut
    /// [`SliceInfo`]: struct.SliceInfo.html
    /// [`D::SliceArg`]: trait.Dimension.html#associatedtype.SliceArg
    ///
    /// **Panics** if any of the following occur:
    ///
    /// * if any of the views would intersect (i.e. if any element would appear in multiple slices)
    /// * if an index is out of bounds or step size is zero
    /// * if `D` is `IxDyn` and `info` does not match the number of array axes
    pub fn multi_slice_move<M>(self, info: M) -> M::Output
    where
        M: MultiSlice<'a, A, D>,
    {
        info.multi_slice_move(self)
    }
}
#[cfg(test)]
mod tests_rug_1489 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_split_at() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = vec![rug_fuzz_0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1];
        let shape = (rug_fuzz_1, rug_fuzz_2);
        let a = ArrayView2::from_shape(shape, &data).unwrap();
        let p0 = a.view();
        let p1 = Axis(rug_fuzz_3);
        let p2: usize = rug_fuzz_4;
        p0.split_at(p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_1490 {
    use super::*;
    use crate::{ArrayBase, ViewRepr};
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut array_data = vec![rug_fuzz_0, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((rug_fuzz_1, rug_fuzz_2), array_data)
            .unwrap();
        let mut p0 = array.view_mut();
        let mut p1 = Axis(rug_fuzz_3);
        let p2: usize = rug_fuzz_4;
        p0.split_at(p1, p2);
             }
}
}
}    }
}
