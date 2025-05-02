use num_traits::{self, Float, FromPrimitive, Zero};
use std::ops::{Add, Div, Mul};
use crate::imp_prelude::*;
use crate::itertools::enumerate;
use crate::numeric_util;
use crate::{FoldWhile, Zip};
/// # Numerical Methods for Arrays
impl<A, S, D> ArrayBase<S, D>
where
    S: Data<Elem = A>,
    D: Dimension,
{
    /// Return the sum of all elements in the array.
    ///
    /// ```
    /// use ndarray::arr2;
    ///
    /// let a = arr2(&[[1., 2.],
    ///                [3., 4.]]);
    /// assert_eq!(a.sum(), 10.);
    /// ```
    pub fn sum(&self) -> A
    where
        A: Clone + Add<Output = A> + num_traits::Zero,
    {
        if let Some(slc) = self.as_slice_memory_order() {
            return numeric_util::unrolled_fold(slc, A::zero, A::add);
        }
        let mut sum = A::zero();
        for row in self.inner_rows() {
            if let Some(slc) = row.as_slice() {
                sum = sum + numeric_util::unrolled_fold(slc, A::zero, A::add);
            } else {
                sum = sum + row.iter().fold(A::zero(), |acc, elt| acc + elt.clone());
            }
        }
        sum
    }
    /// Returns the [arithmetic mean] x̅ of all elements in the array:
    ///
    /// ```text
    ///     1   n
    /// x̅ = ―   ∑ xᵢ
    ///     n  i=1
    /// ```
    ///
    /// If the array is empty, `None` is returned.
    ///
    /// **Panics** if `A::from_usize()` fails to convert the number of elements in the array.
    ///
    /// [arithmetic mean]: https://en.wikipedia.org/wiki/Arithmetic_mean
    pub fn mean(&self) -> Option<A>
    where
        A: Clone + FromPrimitive + Add<Output = A> + Div<Output = A> + Zero,
    {
        let n_elements = self.len();
        if n_elements == 0 {
            None
        } else {
            let n_elements = A::from_usize(n_elements)
                .expect("Converting number of elements to `A` must not fail.");
            Some(self.sum() / n_elements)
        }
    }
    /// Return the sum of all elements in the array.
    ///
    /// *This method has been renamed to `.sum()` and will be deprecated in the
    /// next version.*
    pub fn scalar_sum(&self) -> A
    where
        A: Clone + Add<Output = A> + num_traits::Zero,
    {
        self.sum()
    }
    /// Return the product of all elements in the array.
    ///
    /// ```
    /// use ndarray::arr2;
    ///
    /// let a = arr2(&[[1., 2.],
    ///                [3., 4.]]);
    /// assert_eq!(a.product(), 24.);
    /// ```
    pub fn product(&self) -> A
    where
        A: Clone + Mul<Output = A> + num_traits::One,
    {
        if let Some(slc) = self.as_slice_memory_order() {
            return numeric_util::unrolled_fold(slc, A::one, A::mul);
        }
        let mut sum = A::one();
        for row in self.inner_rows() {
            if let Some(slc) = row.as_slice() {
                sum = sum * numeric_util::unrolled_fold(slc, A::one, A::mul);
            } else {
                sum = sum * row.iter().fold(A::one(), |acc, elt| acc * elt.clone());
            }
        }
        sum
    }
    /// Return sum along `axis`.
    ///
    /// ```
    /// use ndarray::{aview0, aview1, arr2, Axis};
    ///
    /// let a = arr2(&[[1., 2., 3.],
    ///                [4., 5., 6.]]);
    /// assert!(
    ///     a.sum_axis(Axis(0)) == aview1(&[5., 7., 9.]) &&
    ///     a.sum_axis(Axis(1)) == aview1(&[6., 15.]) &&
    ///
    ///     a.sum_axis(Axis(0)).sum_axis(Axis(0)) == aview0(&21.)
    /// );
    /// ```
    ///
    /// **Panics** if `axis` is out of bounds.
    pub fn sum_axis(&self, axis: Axis) -> Array<A, D::Smaller>
    where
        A: Clone + Zero + Add<Output = A>,
        D: RemoveAxis,
    {
        let n = self.len_of(axis);
        let mut res = Array::zeros(self.raw_dim().remove_axis(axis));
        let stride = self.strides()[axis.index()];
        if self.ndim() == 2 && stride == 1 {
            let ax = axis.index();
            for (i, elt) in enumerate(&mut res) {
                *elt = self.index_axis(Axis(1 - ax), i).sum();
            }
        } else {
            for i in 0..n {
                let view = self.index_axis(axis, i);
                res = res + &view;
            }
        }
        res
    }
    /// Return mean along `axis`.
    ///
    /// Return `None` if the length of the axis is zero.
    ///
    /// **Panics** if `axis` is out of bounds or if `A::from_usize()`
    /// fails for the axis length.
    ///
    /// ```
    /// use ndarray::{aview0, aview1, arr2, Axis};
    ///
    /// let a = arr2(&[[1., 2., 3.],
    ///                [4., 5., 6.]]);
    /// assert!(
    ///     a.mean_axis(Axis(0)).unwrap() == aview1(&[2.5, 3.5, 4.5]) &&
    ///     a.mean_axis(Axis(1)).unwrap() == aview1(&[2., 5.]) &&
    ///
    ///     a.mean_axis(Axis(0)).unwrap().mean_axis(Axis(0)).unwrap() == aview0(&3.5)
    /// );
    /// ```
    pub fn mean_axis(&self, axis: Axis) -> Option<Array<A, D::Smaller>>
    where
        A: Clone + Zero + FromPrimitive + Add<Output = A> + Div<Output = A>,
        D: RemoveAxis,
    {
        let axis_length = self.len_of(axis);
        if axis_length == 0 {
            None
        } else {
            let axis_length = A::from_usize(axis_length)
                .expect("Converting axis length to `A` must not fail.");
            let sum = self.sum_axis(axis);
            Some(sum / aview0(&axis_length))
        }
    }
    /// Return variance along `axis`.
    ///
    /// The variance is computed using the [Welford one-pass
    /// algorithm](https://www.jstor.org/stable/1266577).
    ///
    /// The parameter `ddof` specifies the "delta degrees of freedom". For
    /// example, to calculate the population variance, use `ddof = 0`, or to
    /// calculate the sample variance, use `ddof = 1`.
    ///
    /// The variance is defined as:
    ///
    /// ```text
    ///               1       n
    /// variance = ――――――――   ∑ (xᵢ - x̅)²
    ///            n - ddof  i=1
    /// ```
    ///
    /// where
    ///
    /// ```text
    ///     1   n
    /// x̅ = ―   ∑ xᵢ
    ///     n  i=1
    /// ```
    ///
    /// and `n` is the length of the axis.
    ///
    /// **Panics** if `ddof` is less than zero or greater than `n`, if `axis`
    /// is out of bounds, or if `A::from_usize()` fails for any any of the
    /// numbers in the range `0..=n`.
    ///
    /// # Example
    ///
    /// ```
    /// use ndarray::{aview1, arr2, Axis};
    ///
    /// let a = arr2(&[[1., 2.],
    ///                [3., 4.],
    ///                [5., 6.]]);
    /// let var = a.var_axis(Axis(0), 1.);
    /// assert_eq!(var, aview1(&[4., 4.]));
    /// ```
    pub fn var_axis(&self, axis: Axis, ddof: A) -> Array<A, D::Smaller>
    where
        A: Float + FromPrimitive,
        D: RemoveAxis,
    {
        let zero = A::from_usize(0).expect("Converting 0 to `A` must not fail.");
        let n = A::from_usize(self.len_of(axis))
            .expect("Converting length to `A` must not fail.");
        assert!(
            ! (ddof < zero || ddof > n),
            "`ddof` must not be less than zero or greater than the length of \
             the axis",
        );
        let dof = n - ddof;
        let mut mean = Array::<A, _>::zeros(self.dim.remove_axis(axis));
        let mut sum_sq = Array::<A, _>::zeros(self.dim.remove_axis(axis));
        for (i, subview) in self.axis_iter(axis).enumerate() {
            let count = A::from_usize(i + 1)
                .expect("Converting index to `A` must not fail.");
            azip!(
                (mean in & mut mean, sum_sq in & mut sum_sq, & x in & subview) { let
                delta = x - * mean; * mean = * mean + delta / count; * sum_sq = (x - *
                mean).mul_add(delta, * sum_sq); }
            );
        }
        sum_sq.mapv_into(|s| s / dof)
    }
    /// Return standard deviation along `axis`.
    ///
    /// The standard deviation is computed from the variance using
    /// the [Welford one-pass algorithm](https://www.jstor.org/stable/1266577).
    ///
    /// The parameter `ddof` specifies the "delta degrees of freedom". For
    /// example, to calculate the population standard deviation, use `ddof = 0`,
    /// or to calculate the sample standard deviation, use `ddof = 1`.
    ///
    /// The standard deviation is defined as:
    ///
    /// ```text
    ///               ⎛    1       n          ⎞
    /// stddev = sqrt ⎜ ――――――――   ∑ (xᵢ - x̅)²⎟
    ///               ⎝ n - ddof  i=1         ⎠
    /// ```
    ///
    /// where
    ///
    /// ```text
    ///     1   n
    /// x̅ = ―   ∑ xᵢ
    ///     n  i=1
    /// ```
    ///
    /// and `n` is the length of the axis.
    ///
    /// **Panics** if `ddof` is less than zero or greater than `n`, if `axis`
    /// is out of bounds, or if `A::from_usize()` fails for any any of the
    /// numbers in the range `0..=n`.
    ///
    /// # Example
    ///
    /// ```
    /// use ndarray::{aview1, arr2, Axis};
    ///
    /// let a = arr2(&[[1., 2.],
    ///                [3., 4.],
    ///                [5., 6.]]);
    /// let stddev = a.std_axis(Axis(0), 1.);
    /// assert_eq!(stddev, aview1(&[2., 2.]));
    /// ```
    pub fn std_axis(&self, axis: Axis, ddof: A) -> Array<A, D::Smaller>
    where
        A: Float + FromPrimitive,
        D: RemoveAxis,
    {
        self.var_axis(axis, ddof).mapv_into(|x| x.sqrt())
    }
    /// Return `true` if the arrays' elementwise differences are all within
    /// the given absolute tolerance, `false` otherwise.
    ///
    /// If their shapes disagree, `rhs` is broadcast to the shape of `self`.
    ///
    /// **Panics** if broadcasting to the same shape isn’t possible.
    #[deprecated(
        note = "Use `abs_diff_eq` - it requires the `approx` crate feature",
        since = "0.13.0"
    )]
    pub fn all_close<S2, E>(&self, rhs: &ArrayBase<S2, E>, tol: A) -> bool
    where
        A: Float,
        S2: Data<Elem = A>,
        E: Dimension,
    {
        !Zip::from(self)
            .and(rhs.broadcast_unwrap(self.raw_dim()))
            .fold_while(
                (),
                |_, x, y| {
                    if (*x - *y).abs() <= tol {
                        FoldWhile::Continue(())
                    } else {
                        FoldWhile::Done(())
                    }
                },
            )
            .is_done()
    }
}
#[cfg(test)]
mod tests_rug_1135 {
    use super::*;
    use crate::{arr2, ArrayBase};
    use crate::prelude::ArrayBase as ArrayBaseTrait;
    #[test]
    fn test_sum() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(f64, f64, f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = arr2(&[[rug_fuzz_0, rug_fuzz_1], [rug_fuzz_2, rug_fuzz_3]]);
        ArrayBase::<_, _>::sum(&a);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1136 {
    use super::*;
    use crate::{Array, ArrayBase, Data, OwnedRepr};
    use crate::prelude::*;
    #[test]
    fn test_mean() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, f32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: ArrayBase<OwnedRepr<f32>, Dim<[usize; 2]>> = ArrayBase::<
            OwnedRepr<f32>,
            Dim<[usize; 2]>,
        >::from_shape_vec((rug_fuzz_0, rug_fuzz_1), vec![rug_fuzz_2, 2.0, 3.0, 4.0])
            .unwrap();
        let result = p0.mean();
        debug_assert_eq!(result, Some(2.5));
             }
});    }
}
#[cfg(test)]
mod tests_rug_1138 {
    use super::*;
    use crate::{ArrayBase, Data, Dimension, RemoveAxis};
    use num_traits::One;
    use std::ops::Mul;
    #[test]
    fn test_product() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(f64, f64, f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = arr2(&[[rug_fuzz_0, rug_fuzz_1], [rug_fuzz_2, rug_fuzz_3]]);
        debug_assert_eq!(ArrayBase::product(& a), 24.);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1139 {
    use super::*;
    use crate::prelude::*;
    use crate::aview0;
    use crate::aview1;
    use crate::arr2;
    use crate::Axis;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9, mut rug_fuzz_10, mut rug_fuzz_11, mut rug_fuzz_12, mut rug_fuzz_13, mut rug_fuzz_14, mut rug_fuzz_15)) = <(f64, f64, f64, f64, f64, f64, usize, f64, f64, f64, usize, f64, f64, usize, usize, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = arr2(
            &[[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2], [rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]],
        );
        debug_assert!(
            ArrayBase:: < _, _ > ::sum_axis(& a, Axis(rug_fuzz_6)) == aview1(&
            [rug_fuzz_7, rug_fuzz_8, rug_fuzz_9]) && ArrayBase:: < _, _ > ::sum_axis(& a,
            Axis(rug_fuzz_10)) == aview1(& [rug_fuzz_11, rug_fuzz_12]) && ArrayBase:: <
            _, _ > ::sum_axis(& ArrayBase:: < _, _ > ::sum_axis(& a, Axis(rug_fuzz_13)),
            Axis(rug_fuzz_14)) == aview0(& rug_fuzz_15)
        );
             }
});    }
}
#[cfg(test)]
mod tests_rug_1140 {
    use super::*;
    use crate::Array;
    use crate::array;
    use crate::aview0;
    use crate::aview1;
    use crate::arr2;
    use crate::Axis;
    #[test]
    fn test_mean_axis() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(f64, f64, f64, f64, f64, f64, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = arr2(
            &[[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2], [rug_fuzz_3, rug_fuzz_4, rug_fuzz_5]],
        );
        let axis_0 = Axis(rug_fuzz_6);
        let axis_1 = Axis(rug_fuzz_7);
        debug_assert_eq!(
            Array::mean_axis(& a, axis_0).unwrap(), aview1(& [2.5, 3.5, 4.5])
        );
        debug_assert_eq!(Array::mean_axis(& a, axis_1).unwrap(), aview1(& [2., 5.]));
        debug_assert_eq!(
            Array::mean_axis(& Array::mean_axis(& a, axis_0).unwrap(), axis_0).unwrap(),
            aview0(& 3.5)
        );
             }
});    }
}
#[cfg(test)]
mod tests_rug_1141 {
    use super::*;
    use crate::{aview1, arr2, Array, Axis};
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(f64, f64, f64, f64, f64, f64, usize, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let a = arr2(
            &[
                [rug_fuzz_0, rug_fuzz_1],
                [rug_fuzz_2, rug_fuzz_3],
                [rug_fuzz_4, rug_fuzz_5],
            ],
        );
        let axis = Axis(rug_fuzz_6);
        let ddof = rug_fuzz_7;
        let result = a.var_axis(axis, ddof);
        debug_assert_eq!(result, aview1(& [4., 4.]));
             }
});    }
}
use crate::{prelude::*, arr2, aview1};
use crate::Axis;
#[cfg(test)]
mod tests_rug_1142 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(f64, f64, f64, f64, f64, f64, usize, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = arr2(
            &[
                [rug_fuzz_0, rug_fuzz_1],
                [rug_fuzz_2, rug_fuzz_3],
                [rug_fuzz_4, rug_fuzz_5],
            ],
        );
        let mut p1 = Axis(rug_fuzz_6);
        let mut p2 = rug_fuzz_7;
        p0.std_axis(p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_1143 {
    use super::*;
    use crate::{ArrayBase, Data, Dimension, OwnedRepr, Zip};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(usize, f64, usize, f64, f64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: ArrayBase<OwnedRepr<f64>, _> = ArrayBase::<
            OwnedRepr<f64>,
            _,
        >::from_elem(rug_fuzz_0, rug_fuzz_1);
        let mut p1: ArrayBase<OwnedRepr<f64>, _> = ArrayBase::<
            OwnedRepr<f64>,
            _,
        >::from_elem(rug_fuzz_2, rug_fuzz_3);
        let mut p2 = rug_fuzz_4;
        p0.all_close(&p1, p2);
             }
});    }
}
