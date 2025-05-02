//! Methods for two-dimensional arrays.
use crate::imp_prelude::*;
/// # Methods For 2-D Arrays
impl<A, S> ArrayBase<S, Ix2>
where
    S: RawData<Elem = A>,
{
    /// Return an array view of row `index`.
    ///
    /// **Panics** if `index` is out of bounds.
    pub fn row(&self, index: Ix) -> ArrayView1<'_, A>
    where
        S: Data,
    {
        self.index_axis(Axis(0), index)
    }
    /// Return a mutable array view of row `index`.
    ///
    /// **Panics** if `index` is out of bounds.
    pub fn row_mut(&mut self, index: Ix) -> ArrayViewMut1<'_, A>
    where
        S: DataMut,
    {
        self.index_axis_mut(Axis(0), index)
    }
    /// Return the number of rows (length of `Axis(0)`) in the two-dimensional array.
    pub fn nrows(&self) -> usize {
        self.len_of(Axis(0))
    }
    /// Return the number of rows (length of `Axis(0)`) in the two-dimensional array.
    #[deprecated(note = "Renamed to .nrows(), please use the new name")]
    pub fn rows(&self) -> usize {
        self.nrows()
    }
    /// Return an array view of column `index`.
    ///
    /// **Panics** if `index` is out of bounds.
    pub fn column(&self, index: Ix) -> ArrayView1<'_, A>
    where
        S: Data,
    {
        self.index_axis(Axis(1), index)
    }
    /// Return a mutable array view of column `index`.
    ///
    /// **Panics** if `index` is out of bounds.
    pub fn column_mut(&mut self, index: Ix) -> ArrayViewMut1<'_, A>
    where
        S: DataMut,
    {
        self.index_axis_mut(Axis(1), index)
    }
    /// Return the number of columns (length of `Axis(1)`) in the two-dimensional array.
    pub fn ncols(&self) -> usize {
        self.len_of(Axis(1))
    }
    /// Return the number of columns (length of `Axis(1)`) in the two-dimensional array.
    #[deprecated(note = "Renamed to .ncols(), please use the new name")]
    pub fn cols(&self) -> usize {
        self.ncols()
    }
    /// Return true if the array is square, false otherwise.
    pub fn is_square(&self) -> bool {
        self.nrows() == self.ncols()
    }
}
#[cfg(test)]
mod tests_rug_1125 {
    use super::*;
    use crate::{Array, ArrayBase, ArrayViewMut1, Axis, DataMut, Ix};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(i32, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = Array::from(vec![rug_fuzz_0, 2, 3, 4])
            .into_shape((rug_fuzz_1, rug_fuzz_2))
            .unwrap();
        let mut index = rug_fuzz_3;
        let _ = <ArrayBase<_, _>>::row_mut(&mut data, index);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1127 {
    use super::*;
    use crate::{ArrayBase, OwnedRepr, Dimension, Ix2};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(f64, usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data: Vec<f64> = vec![rug_fuzz_0, 2.0, 3.0, 4.0];
        let array = ArrayBase::<
            OwnedRepr<f64>,
            Dim<[usize; 2]>,
        >::from_shape_vec(
                (rug_fuzz_1, rug_fuzz_2).strides((rug_fuzz_3, rug_fuzz_4)),
                data,
            )
            .unwrap();
        array.rows();
             }
});    }
}
#[cfg(test)]
mod tests_rug_1128 {
    use super::*;
    use crate::{ArrayView1, ArrayBase, Data, Ix, Axis, Array};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data: Array<f64, _> = Array::zeros((rug_fuzz_0, rug_fuzz_1));
        let index = rug_fuzz_2;
        let result = data.view().column(index);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1131 {
    use super::*;
    use crate::{ArrayBase, Data, Ix2, OwnedRepr};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(f64, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: Vec<f64> = vec![rug_fuzz_0, 2.0, 3.0, 4.0];
        let p0: ArrayBase<OwnedRepr<f64>, Ix2> = ArrayBase::from_shape_vec(
                (rug_fuzz_1, rug_fuzz_2),
                data,
            )
            .unwrap();
        p0.cols();
             }
});    }
}
