// Copyright 2014-2016 bluss and ndarray developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Constructor methods for ndarray
//!
//!

#![allow(clippy::match_wild_err_arm)]

use num_traits::{Float, One, Zero};
use std::mem::MaybeUninit;

use crate::dimension;
use crate::error::{self, ShapeError};
use crate::extension::nonnull::nonnull_from_vec_data;
use crate::imp_prelude::*;
use crate::indexes;
use crate::indices;
use crate::iterators::{to_vec, to_vec_mapped};
use crate::StrideShape;
use crate::{geomspace, linspace, logspace};

/// # Constructor Methods for Owned Arrays
///
/// Note that the constructor methods apply to `Array` and `ArcArray`,
/// the two array types that have owned storage.
///
/// ## Constructor methods for one-dimensional arrays.
impl<S, A> ArrayBase<S, Ix1>
where
    S: DataOwned<Elem = A>,
{
    /// Create a one-dimensional array from a vector (no copying needed).
    ///
    /// **Panics** if the length is greater than `isize::MAX`.
    ///
    /// ```rust
    /// use ndarray::Array;
    ///
    /// let array = Array::from(vec![1., 2., 3., 4.]);
    /// ```
    #[deprecated(note = "use standard `from`", since = "0.13.0")]
    pub fn from_vec(v: Vec<A>) -> Self {
        Self::from(v)
    }

    /// Create a one-dimensional array with `n` evenly spaced elements from
    /// `start` to `end` (inclusive). `A` must be a floating point type.
    ///
    /// Note that if `start > end`, the first element will still be `start`,
    /// and the following elements will be decreasing. This is different from
    /// the behavior of `std::ops::RangeInclusive`, which interprets `start >
    /// end` to mean that the range is empty.
    ///
    /// **Panics** if `n` is greater than `isize::MAX` or if converting `n - 1`
    /// to type `A` fails.
    ///
    /// ```rust
    /// use ndarray::{Array, arr1};
    ///
    /// let array = Array::linspace(0., 1., 5);
    /// assert!(array == arr1(&[0.0, 0.25, 0.5, 0.75, 1.0]))
    /// ```
    pub fn linspace(start: A, end: A, n: usize) -> Self
    where
        A: Float,
    {
        Self::from(to_vec(linspace::linspace(start, end, n)))
    }

    /// Create a one-dimensional array with elements from `start` to `end`
    /// (exclusive), incrementing by `step`. `A` must be a floating point type.
    ///
    /// **Panics** if the length is greater than `isize::MAX`.
    ///
    /// ```rust
    /// use ndarray::{Array, arr1};
    ///
    /// let array = Array::range(0., 5., 1.);
    /// assert!(array == arr1(&[0., 1., 2., 3., 4.]))
    /// ```
    pub fn range(start: A, end: A, step: A) -> Self
    where
        A: Float,
    {
        Self::from(to_vec(linspace::range(start, end, step)))
    }

    /// Create a one-dimensional array with `n` logarithmically spaced
    /// elements, with the starting value being `base.powf(start)` and the
    /// final one being `base.powf(end)`. `A` must be a floating point type.
    ///
    /// If `base` is negative, all values will be negative.
    ///
    /// **Panics** if `n` is greater than `isize::MAX` or if converting `n - 1`
    /// to type `A` fails.
    ///
    /// ```rust
    /// use approx::assert_abs_diff_eq;
    /// use ndarray::{Array, arr1};
    ///
    /// # #[cfg(feature = "approx")] {
    /// let array = Array::logspace(10.0, 0.0, 3.0, 4);
    /// assert_abs_diff_eq!(array, arr1(&[1e0, 1e1, 1e2, 1e3]));
    ///
    /// let array = Array::logspace(-10.0, 3.0, 0.0, 4);
    /// assert_abs_diff_eq!(array, arr1(&[-1e3, -1e2, -1e1, -1e0]));
    /// # }
    /// ```
    pub fn logspace(base: A, start: A, end: A, n: usize) -> Self
    where
        A: Float,
    {
        Self::from(to_vec(logspace::logspace(base, start, end, n)))
    }

    /// Create a one-dimensional array with `n` geometrically spaced elements
    /// from `start` to `end` (inclusive). `A` must be a floating point type.
    ///
    /// Returns `None` if `start` and `end` have different signs or if either
    /// one is zero. Conceptually, this means that in order to obtain a `Some`
    /// result, `end / start` must be positive.
    ///
    /// **Panics** if `n` is greater than `isize::MAX` or if converting `n - 1`
    /// to type `A` fails.
    ///
    /// ```rust
    /// use approx::assert_abs_diff_eq;
    /// use ndarray::{Array, arr1};
    ///
    /// # fn example() -> Option<()> {
    /// # #[cfg(feature = "approx")] {
    /// let array = Array::geomspace(1e0, 1e3, 4)?;
    /// assert_abs_diff_eq!(array, arr1(&[1e0, 1e1, 1e2, 1e3]), epsilon = 1e-12);
    ///
    /// let array = Array::geomspace(-1e3, -1e0, 4)?;
    /// assert_abs_diff_eq!(array, arr1(&[-1e3, -1e2, -1e1, -1e0]), epsilon = 1e-12);
    /// # }
    /// # Some(())
    /// # }
    /// #
    /// # example().unwrap();
    /// ```
    pub fn geomspace(start: A, end: A, n: usize) -> Option<Self>
    where
        A: Float,
    {
        Some(Self::from(to_vec(geomspace::geomspace(start, end, n)?)))
    }
}

/// ## Constructor methods for two-dimensional arrays.
impl<S, A> ArrayBase<S, Ix2>
where
    S: DataOwned<Elem = A>,
{
    /// Create an identity matrix of size `n` (square 2D array).
    ///
    /// **Panics** if `n * n` would overflow `isize`.
    pub fn eye(n: Ix) -> Self
    where
        S: DataMut,
        A: Clone + Zero + One,
    {
        let mut eye = Self::zeros((n, n));
        for a_ii in eye.diag_mut() {
            *a_ii = A::one();
        }
        eye
    }

    /// Create a 2D matrix from its diagonal
    ///
    /// **Panics** if `diag.len() * diag.len()` would overflow `isize`.
    ///
    /// ```rust
    /// use ndarray::{Array2, arr1, arr2};
    ///
    /// let diag = arr1(&[1, 2]);
    /// let array = Array2::from_diag(&diag);
    /// assert_eq!(array, arr2(&[[1, 0], [0, 2]]));
    /// ```
    pub fn from_diag<S2>(diag: &ArrayBase<S2, Ix1>) -> Self
    where
        A: Clone + Zero,
        S: DataMut,
        S2: Data<Elem = A>,
    {
        let n = diag.len();
        let mut arr = Self::zeros((n, n));
        arr.diag_mut().assign(&diag);
        arr
    }
}

#[cfg(not(debug_assertions))]
#[allow(clippy::match_wild_err_arm)]
macro_rules! size_of_shape_checked_unwrap {
    ($dim:expr) => {
        match dimension::size_of_shape_checked($dim) {
            Ok(sz) => sz,
            Err(_) => {
                panic!("ndarray: Shape too large, product of non-zero axis lengths overflows isize")
            }
        }
    };
}

#[cfg(debug_assertions)]
macro_rules! size_of_shape_checked_unwrap {
    ($dim:expr) => {
        match dimension::size_of_shape_checked($dim) {
            Ok(sz) => sz,
            Err(_) => panic!(
                "ndarray: Shape too large, product of non-zero axis lengths \
                 overflows isize in shape {:?}",
                $dim
            ),
        }
    };
}

/// ## Constructor methods for n-dimensional arrays.
///
/// The `shape` argument can be an integer or a tuple of integers to specify
/// a static size. For example `10` makes a length 10 one-dimensional array
/// (dimension type `Ix1`) and `(5, 6)` a 5 × 6 array (dimension type `Ix2`).
///
/// With the trait `ShapeBuilder` in scope, there is the method `.f()` to select
/// column major (“f” order) memory layout instead of the default row major.
/// For example `Array::zeros((5, 6).f())` makes a column major 5 × 6 array.
///
/// Use [`IxDyn`](type.IxDyn.html) for the shape to create an array with dynamic
/// number of axes.
///
/// Finally, the few constructors that take a completely general
/// `Into<StrideShape>` argument *optionally* support custom strides, for
/// example a shape given like `(10, 2, 2).strides((1, 10, 20))` is valid.
impl<S, A, D> ArrayBase<S, D>
where
    S: DataOwned<Elem = A>,
    D: Dimension,
{
    /// Create an array with copies of `elem`, shape `shape`.
    ///
    /// **Panics** if the product of non-zero axis lengths overflows `isize`.
    ///
    /// ```
    /// use ndarray::{Array, arr3, ShapeBuilder};
    ///
    /// let a = Array::from_elem((2, 2, 2), 1.);
    ///
    /// assert!(
    ///     a == arr3(&[[[1., 1.],
    ///                  [1., 1.]],
    ///                 [[1., 1.],
    ///                  [1., 1.]]])
    /// );
    /// assert!(a.strides() == &[4, 2, 1]);
    ///
    /// let b = Array::from_elem((2, 2, 2).f(), 1.);
    /// assert!(b.strides() == &[1, 2, 4]);
    /// ```
    pub fn from_elem<Sh>(shape: Sh, elem: A) -> Self
    where
        A: Clone,
        Sh: ShapeBuilder<Dim = D>,
    {
        let shape = shape.into_shape();
        let size = size_of_shape_checked_unwrap!(&shape.dim);
        let v = vec![elem; size];
        unsafe { Self::from_shape_vec_unchecked(shape, v) }
    }

    /// Create an array with zeros, shape `shape`.
    ///
    /// **Panics** if the product of non-zero axis lengths overflows `isize`.
    pub fn zeros<Sh>(shape: Sh) -> Self
    where
        A: Clone + Zero,
        Sh: ShapeBuilder<Dim = D>,
    {
        Self::from_elem(shape, A::zero())
    }

    /// Create an array with ones, shape `shape`.
    ///
    /// **Panics** if the product of non-zero axis lengths overflows `isize`.
    pub fn ones<Sh>(shape: Sh) -> Self
    where
        A: Clone + One,
        Sh: ShapeBuilder<Dim = D>,
    {
        Self::from_elem(shape, A::one())
    }

    /// Create an array with default values, shape `shape`
    ///
    /// **Panics** if the product of non-zero axis lengths overflows `isize`.
    pub fn default<Sh>(shape: Sh) -> Self
    where
        A: Default,
        Sh: ShapeBuilder<Dim = D>,
    {
        Self::from_shape_simple_fn(shape, A::default)
    }

    /// Create an array with values created by the function `f`.
    ///
    /// `f` is called with no argument, and it should return the element to
    /// create. If the precise index of the element to create is needed,
    /// use [`from_shape_fn`](ArrayBase::from_shape_fn) instead.
    ///
    /// This constructor can be useful if the element order is not important,
    /// for example if they are identical or random.
    ///
    /// **Panics** if the product of non-zero axis lengths overflows `isize`.
    pub fn from_shape_simple_fn<Sh, F>(shape: Sh, mut f: F) -> Self
    where
        Sh: ShapeBuilder<Dim = D>,
        F: FnMut() -> A,
    {
        let shape = shape.into_shape();
        let len = size_of_shape_checked_unwrap!(&shape.dim);
        let v = to_vec_mapped(0..len, move |_| f());
        unsafe { Self::from_shape_vec_unchecked(shape, v) }
    }

    /// Create an array with values created by the function `f`.
    ///
    /// `f` is called with the index of the element to create; the elements are
    /// visited in arbitrary order.
    ///
    /// **Panics** if the product of non-zero axis lengths overflows `isize`.
    ///
    /// ```
    /// use ndarray::{Array, arr2};
    ///
    /// // Create a table of i × j (with i and j from 1 to 3)
    /// let ij_table = Array::from_shape_fn((3, 3), |(i, j)| (1 + i) * (1 + j));
    ///
    /// assert_eq!(
    ///     ij_table,
    ///     arr2(&[[1, 2, 3],
    ///            [2, 4, 6],
    ///            [3, 6, 9]])
    /// );
    /// ```
    pub fn from_shape_fn<Sh, F>(shape: Sh, f: F) -> Self
    where
        Sh: ShapeBuilder<Dim = D>,
        F: FnMut(D::Pattern) -> A,
    {
        let shape = shape.into_shape();
        let _ = size_of_shape_checked_unwrap!(&shape.dim);
        if shape.is_c {
            let v = to_vec_mapped(indices(shape.dim.clone()).into_iter(), f);
            unsafe { Self::from_shape_vec_unchecked(shape, v) }
        } else {
            let dim = shape.dim.clone();
            let v = to_vec_mapped(indexes::indices_iter_f(dim), f);
            unsafe { Self::from_shape_vec_unchecked(shape, v) }
        }
    }

    /// Create an array with the given shape from a vector. (No cloning of
    /// elements needed.)
    ///
    /// ----
    ///
    /// For a contiguous c- or f-order shape, the following applies:
    ///
    /// **Errors** if `shape` does not correspond to the number of elements in
    /// `v` or if the shape/strides would result in overflowing `isize`.
    ///
    /// ----
    ///
    /// For custom strides, the following applies:
    ///
    /// **Errors** if strides and dimensions can point out of bounds of `v`, if
    /// strides allow multiple indices to point to the same element, or if the
    /// shape/strides would result in overflowing `isize`.
    ///
    /// ```
    /// use ndarray::Array;
    /// use ndarray::ShapeBuilder; // Needed for .strides() method
    /// use ndarray::arr2;
    ///
    /// let a = Array::from_shape_vec((2, 2), vec![1., 2., 3., 4.]);
    /// assert!(a.is_ok());
    ///
    /// let b = Array::from_shape_vec((2, 2).strides((1, 2)),
    ///                               vec![1., 2., 3., 4.]).unwrap();
    /// assert!(
    ///     b == arr2(&[[1., 3.],
    ///                 [2., 4.]])
    /// );
    /// ```
    pub fn from_shape_vec<Sh>(shape: Sh, v: Vec<A>) -> Result<Self, ShapeError>
    where
        Sh: Into<StrideShape<D>>,
    {
        // eliminate the type parameter Sh as soon as possible
        Self::from_shape_vec_impl(shape.into(), v)
    }

    fn from_shape_vec_impl(shape: StrideShape<D>, v: Vec<A>) -> Result<Self, ShapeError> {
        let dim = shape.dim;
        let strides = shape.strides;
        if shape.custom {
            dimension::can_index_slice(&v, &dim, &strides)?;
        } else {
            dimension::can_index_slice_not_custom::<A, _>(&v, &dim)?;
            if dim.size() != v.len() {
                return Err(error::incompatible_shapes(&Ix1(v.len()), &dim));
            }
        }
        unsafe { Ok(Self::from_vec_dim_stride_unchecked(dim, strides, v)) }
    }

    /// Creates an array from a vector and interpret it according to the
    /// provided shape and strides. (No cloning of elements needed.)
    ///
    /// # Safety
    ///
    /// The caller must ensure that the following conditions are met:
    ///
    /// 1. The ndim of `dim` and `strides` must be the same.
    ///
    /// 2. The product of non-zero axis lengths must not exceed `isize::MAX`.
    ///
    /// 3. For axes with length > 1, the stride must be nonnegative.
    ///
    /// 4. If the array will be empty (any axes are zero-length), the
    ///    difference between the least address and greatest address accessible
    ///    by moving along all axes must be ≤ `v.len()`.
    ///
    ///    If the array will not be empty, the difference between the least
    ///    address and greatest address accessible by moving along all axes
    ///    must be < `v.len()`.
    ///
    /// 5. The strides must not allow any element to be referenced by two different
    ///    indices.
    pub unsafe fn from_shape_vec_unchecked<Sh>(shape: Sh, v: Vec<A>) -> Self
    where
        Sh: Into<StrideShape<D>>,
    {
        let shape = shape.into();
        Self::from_vec_dim_stride_unchecked(shape.dim, shape.strides, v)
    }

    unsafe fn from_vec_dim_stride_unchecked(dim: D, strides: D, mut v: Vec<A>) -> Self {
        // debug check for issues that indicates wrong use of this constructor
        debug_assert!(dimension::can_index_slice(&v, &dim, &strides).is_ok());
        ArrayBase {
            ptr: nonnull_from_vec_data(&mut v),
            data: DataOwned::new(v),
            strides,
            dim,
        }
    }

    /// Create an array with uninitalized elements, shape `shape`.
    ///
    /// Prefer to use [`maybe_uninit()`](ArrayBase::maybe_uninit) if possible, because it is
    /// easier to use correctly.
    ///
    /// **Panics** if the number of elements in `shape` would overflow isize.
    ///
    /// ### Safety
    ///
    /// Accessing uninitalized values is undefined behaviour. You must overwrite *all* the elements
    /// in the array after it is created; for example using
    /// [`raw_view_mut`](ArrayBase::raw_view_mut) or other low-level element access.
    ///
    /// The contents of the array is indeterminate before initialization and it
    /// is an error to perform operations that use the previous values. For
    /// example it would not be legal to use `a += 1.;` on such an array.
    ///
    /// This constructor is limited to elements where `A: Copy` (no destructors)
    /// to avoid users shooting themselves too hard in the foot.
    /// 
    /// (Also note that the constructors `from_shape_vec` and
    /// `from_shape_vec_unchecked` allow the user yet more control, in the sense
    /// that Arrays can be created from arbitrary vectors.)
    pub unsafe fn uninitialized<Sh>(shape: Sh) -> Self
    where
        A: Copy,
        Sh: ShapeBuilder<Dim = D>,
    {
        let shape = shape.into_shape();
        let size = size_of_shape_checked_unwrap!(&shape.dim);
        let mut v = Vec::with_capacity(size);
        v.set_len(size);
        Self::from_shape_vec_unchecked(shape, v)
    }
}

impl<S, A, D> ArrayBase<S, D>
where
    S: DataOwned<Elem = MaybeUninit<A>>,
    D: Dimension,
{
    /// Create an array with uninitalized elements, shape `shape`.
    ///
    /// The uninitialized elements of type `A` are represented by the type `MaybeUninit<A>`,
    /// an easier way to handle uninit values correctly.
    ///
    /// Only *when* the array is completely initialized with valid elements, can it be
    /// converted to an array of `A` elements using [`.assume_init()`].
    ///
    /// **Panics** if the number of elements in `shape` would overflow isize.
    ///
    /// ### Safety
    ///
    /// The whole of the array must be initialized before it is converted
    /// using [`.assume_init()`] or otherwise traversed.
    ///
    /// ### Examples
    ///
    /// It is possible to assign individual values through `*elt = MaybeUninit::new(value)`
    /// and so on.
    ///
    /// [`.assume_init()`]: ArrayBase::assume_init
    ///
    /// ```
    /// use ndarray::{s, Array2};
    /// use ndarray::Zip;
    /// use ndarray::Axis;
    ///
    /// // Example Task: Let's create a column shifted copy of the input
    ///
    /// fn shift_by_two(a: &Array2<f32>) -> Array2<f32> {
    ///     // create an uninitialized array
    ///     let mut b = Array2::maybe_uninit(a.dim());
    ///
    ///     // two first columns in b are two last in a
    ///     // rest of columns in b are the initial columns in a
    ///
    ///     assign_to(a.slice(s![.., -2..]), b.slice_mut(s![.., ..2]));
    ///     assign_to(a.slice(s![.., 2..]), b.slice_mut(s![.., ..-2]));
    ///
    ///     // Now we can promise that `b` is safe to use with all operations
    ///     unsafe {
    ///         b.assume_init()
    ///     }
    /// }
    ///
    /// use ndarray::{IntoNdProducer, AssignElem};
    ///
    /// // This function clones elements from the first input to the second;
    /// // the two producers must have the same shape
    /// fn assign_to<'a, P1, P2, A>(from: P1, to: P2)
    ///     where P1: IntoNdProducer<Item = &'a A>,
    ///           P2: IntoNdProducer<Dim = P1::Dim>,
    ///           P2::Item: AssignElem<A>,
    ///           A: Clone + 'a
    /// {
    ///     Zip::from(from)
    ///         .apply_assign_into(to, A::clone);
    /// }
    ///
    /// # shift_by_two(&Array2::zeros((8, 8)));
    /// ```
    pub fn maybe_uninit<Sh>(shape: Sh) -> Self
    where
        Sh: ShapeBuilder<Dim = D>,
    {
        unsafe {
            let shape = shape.into_shape();
            let size = size_of_shape_checked_unwrap!(&shape.dim);
            let mut v = Vec::with_capacity(size);
            v.set_len(size);
            Self::from_shape_vec_unchecked(shape, v)
        }
    }
}
#[cfg(test)]
mod tests_rug_1102 {
    use super::*;
    use crate::{Array, ArrayBase, Data, OwnedRepr, DataOwned};

    #[test]
    fn test_rug() {
        #[derive(Debug)]
        struct A {
            data: i32,
        }

        let v21: Vec<A> = vec![A { data: 5 }, A { data: 10 }, A { data: 15 }];

        let p0 = v21;

        <ArrayBase<OwnedRepr<A>, dimension::dim::Dim<[usize; 1]>>>::from_vec(p0);
    }
}#[cfg(test)]
mod tests_rug_1103 {
    use super::*;
    use crate::{Array, arr1};
    use crate::linspace;

    #[test]
    fn test_rug() {
        let start = 0.0;
        let end = 1.0;
        let n = 5;

        let array = Array::linspace(start, end, n);
        assert!(array == arr1(&[0.0, 0.25, 0.5, 0.75, 1.0]));
    }
}
#[cfg(test)]
mod tests_rug_1105 {
    use super::*;
    use crate::Array;
    use crate::arr1;
    
    #[test]
    fn test_rug() {
        let mut p0 = arr1(&[10.0]);
        let mut p1 = arr1(&[0.0]);
        let mut p2 = arr1(&[3.0]);
        let p3: usize = 4;

        Array::logspace(p0[0], p1[0], p2[0], p3);
    }
}
#[cfg(test)]
mod tests_rug_1106 {
    use super::*;
    use crate::Array;

    #[test]
    fn test_geomspace() {
        let p0 = arr1(&[1.0, 1.0, 1.0]);
        let p1 = arr1(&[10.0, 10.0, 10.0]);
        let p2: usize = 4;

        Array::geomspace(p0[0], p1[0], p2);
    }
}#[cfg(test)]
mod tests_rug_1107 {
    use super::*;
    use crate::{ArrayBase, DataMut, DataOwned, Ix, OwnedRepr, Array2};
    use crate::dimension::{Dim, dim};
    use num_traits::{Zero, One};

    #[test]
    fn test_eye() {
        let n: usize = 3;
        let p0: Ix = n;
        
        let result = <ArrayBase<OwnedRepr<f64>, Dim<[usize; 2]>>>::eye(p0);

        assert_eq!(result, Array2::<f64>::eye(n));
    }
}#[cfg(test)]
mod tests_rug_1108 {
    use super::*;
    use crate::{ArrayBase, Data, DataMut, Ix1, Ix2, Array2};

    #[test]
    fn test_from_diag() {
        let diag = array![1, 2];
        let mut diag_array: ArrayBase<_, dimension::dim::Dim<[usize; 1]>> = diag.into_shape((2,)).unwrap();

        let array = Array2::from_diag(&diag_array);

        assert_eq!(array, array![[1, 0], [0, 2]]);
    }
}#[cfg(test)]
mod tests_rug_1114 {
    use super::*;
    use crate::prelude::*;
    use crate::{Array, arr2, Shape, indices};

    #[test]
    fn test_from_shape_fn() {
        let mut p0: Shape<_> = (3, 3).into_shape().f();
        let mut p1 = |(i, j)| (1 + i) * (1 + j);

        let result = Array::from_shape_fn(p0, p1);

        assert_eq!(
            result,
            arr2(&[[1, 2, 3],
                   [2, 4, 6],
                   [3, 6, 9]])
        );
    }
}