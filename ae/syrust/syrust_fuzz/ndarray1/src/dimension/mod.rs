use crate::error::{from_kind, ErrorKind, ShapeError};
use crate::{Ix, Ixs, Slice, SliceOrIndex};
use num_integer::div_floor;
pub use self::axes::{axes_of, Axes, AxisDescription};
pub use self::axis::Axis;
pub use self::conversion::IntoDimension;
pub use self::dim::*;
pub use self::dimension_trait::Dimension;
pub use self::dynindeximpl::IxDynImpl;
pub use self::ndindex::NdIndex;
pub use self::remove_axis::RemoveAxis;
use std::isize;
use std::mem;
#[macro_use]
mod macros;
mod axes;
mod axis;
mod conversion;
pub mod dim;
mod dimension_trait;
mod dynindeximpl;
mod ndindex;
mod remove_axis;
/// Calculate offset from `Ix` stride converting sign properly
#[inline(always)]
pub fn stride_offset(n: Ix, stride: Ix) -> isize {
    (n as isize) * ((stride as Ixs) as isize)
}
/// Check whether the given `dim` and `stride` lead to overlapping indices
///
/// There is overlap if, when iterating through the dimensions in order of
/// increasing stride, the current stride is less than or equal to the maximum
/// possible offset along the preceding axes. (Axes of length ≤1 are ignored.)
///
/// The current implementation assumes that strides of axes with length > 1 are
/// nonnegative. Additionally, it does not check for overflow.
pub fn dim_stride_overlap<D: Dimension>(dim: &D, strides: &D) -> bool {
    let order = strides._fastest_varying_stride_order();
    let mut sum_prev_offsets = 0;
    for &index in order.slice() {
        let d = dim[index];
        let s = strides[index] as isize;
        match d {
            0 => return false,
            1 => {}
            _ => {
                if s <= sum_prev_offsets {
                    return true;
                }
                sum_prev_offsets += (d - 1) as isize * s;
            }
        }
    }
    false
}
/// Returns the `size` of the `dim`, checking that the product of non-zero axis
/// lengths does not exceed `isize::MAX`.
///
/// If `size_of_checked_shape(dim)` returns `Ok(size)`, the data buffer is a
/// slice or `Vec` of length `size`, and `strides` are created with
/// `self.default_strides()` or `self.fortran_strides()`, then the invariants
/// are met to construct an array from the data buffer, `dim`, and `strides`.
/// (The data buffer being a slice or `Vec` guarantees that it contains no more
/// than `isize::MAX` bytes.)
pub fn size_of_shape_checked<D: Dimension>(dim: &D) -> Result<usize, ShapeError> {
    let size_nonzero = dim
        .slice()
        .iter()
        .filter(|&&d| d != 0)
        .try_fold(1usize, |acc, &d| acc.checked_mul(d))
        .ok_or_else(|| from_kind(ErrorKind::Overflow))?;
    if size_nonzero > ::std::isize::MAX as usize {
        Err(from_kind(ErrorKind::Overflow))
    } else {
        Ok(dim.size())
    }
}
/// Checks whether the given data and dimension meet the invariants of the
/// `ArrayBase` type, assuming the strides are created using
/// `dim.default_strides()` or `dim.fortran_strides()`.
///
/// To meet the invariants,
///
/// 1. The product of non-zero axis lengths must not exceed `isize::MAX`.
///
/// 2. The result of `dim.size()` (assuming no overflow) must be less than or
///    equal to the length of the slice.
///
///    (Since `dim.default_strides()` and `dim.fortran_strides()` always return
///    contiguous strides for non-empty arrays, this ensures that for non-empty
///    arrays the difference between the least address and greatest address
///    accessible by moving along all axes is < the length of the slice. Since
///    `dim.default_strides()` and `dim.fortran_strides()` always return all
///    zero strides for empty arrays, this ensures that for empty arrays the
///    difference between the least address and greatest address accessible by
///    moving along all axes is ≤ the length of the slice.)
///
/// Note that since slices cannot contain more than `isize::MAX` bytes,
/// conditions 1 and 2 are sufficient to guarantee that the offset in units of
/// `A` and in units of bytes between the least address and greatest address
/// accessible by moving along all axes does not exceed `isize::MAX`.
pub fn can_index_slice_not_custom<A, D: Dimension>(
    data: &[A],
    dim: &D,
) -> Result<(), ShapeError> {
    let len = size_of_shape_checked(dim)?;
    if len > data.len() {
        return Err(from_kind(ErrorKind::OutOfBounds));
    }
    Ok(())
}
/// Returns the absolute difference in units of `A` between least and greatest
/// address accessible by moving along all axes.
///
/// Returns `Ok` only if
///
/// 1. The ndim of `dim` and `strides` is the same.
///
/// 2. The absolute difference in units of `A` and in units of bytes between
///    the least address and greatest address accessible by moving along all axes
///    does not exceed `isize::MAX`.
///
/// 3. The product of non-zero axis lengths does not exceed `isize::MAX`. (This
///    also implies that the length of any individual axis does not exceed
///    `isize::MAX`.)
pub fn max_abs_offset_check_overflow<A, D>(
    dim: &D,
    strides: &D,
) -> Result<usize, ShapeError>
where
    D: Dimension,
{
    if dim.ndim() != strides.ndim() {
        return Err(from_kind(ErrorKind::IncompatibleLayout));
    }
    let _ = size_of_shape_checked(dim)?;
    let max_offset: usize = izip!(dim.slice(), strides.slice())
        .try_fold(
            0usize,
            |acc, (&d, &s)| {
                let s = s as isize;
                let off = d.saturating_sub(1).checked_mul(s.abs() as usize)?;
                acc.checked_add(off)
            },
        )
        .ok_or_else(|| from_kind(ErrorKind::Overflow))?;
    if max_offset > isize::MAX as usize {
        return Err(from_kind(ErrorKind::Overflow));
    }
    let max_offset_bytes = max_offset
        .checked_mul(mem::size_of::<A>())
        .ok_or_else(|| from_kind(ErrorKind::Overflow))?;
    if max_offset_bytes > isize::MAX as usize {
        return Err(from_kind(ErrorKind::Overflow));
    }
    Ok(max_offset)
}
/// Checks whether the given data, dimension, and strides meet the invariants
/// of the `ArrayBase` type (except for checking ownership of the data).
///
/// To meet the invariants,
///
/// 1. The ndim of `dim` and `strides` must be the same.
///
/// 2. The product of non-zero axis lengths must not exceed `isize::MAX`.
///
/// 3. For axes with length > 1, the stride must be nonnegative. This is
///    necessary to make sure the pointer cannot move backwards outside the
///    slice. For axes with length ≤ 1, the stride can be anything.
///
/// 4. If the array will be empty (any axes are zero-length), the difference
///    between the least address and greatest address accessible by moving
///    along all axes must be ≤ `data.len()`. (It's fine in this case to move
///    one byte past the end of the slice since the pointers will be offset but
///    never dereferenced.)
///
///    If the array will not be empty, the difference between the least address
///    and greatest address accessible by moving along all axes must be <
///    `data.len()`. This and #3 ensure that all dereferenceable pointers point
///    to elements within the slice.
///
/// 5. The strides must not allow any element to be referenced by two different
///    indices.
///
/// Note that since slices cannot contain more than `isize::MAX` bytes,
/// condition 4 is sufficient to guarantee that the absolute difference in
/// units of `A` and in units of bytes between the least address and greatest
/// address accessible by moving along all axes does not exceed `isize::MAX`.
pub fn can_index_slice<A, D: Dimension>(
    data: &[A],
    dim: &D,
    strides: &D,
) -> Result<(), ShapeError> {
    let max_offset = max_abs_offset_check_overflow::<A, _>(dim, strides)?;
    let is_empty = dim.slice().iter().any(|&d| d == 0);
    if is_empty && max_offset > data.len() {
        return Err(from_kind(ErrorKind::OutOfBounds));
    }
    if !is_empty && max_offset >= data.len() {
        return Err(from_kind(ErrorKind::OutOfBounds));
    }
    for (&d, &s) in izip!(dim.slice(), strides.slice()) {
        let s = s as isize;
        if d > 1 && s < 0 {
            return Err(from_kind(ErrorKind::Unsupported));
        }
    }
    if !is_empty && dim_stride_overlap(dim, strides) {
        return Err(from_kind(ErrorKind::Unsupported));
    }
    Ok(())
}
/// Stride offset checked general version (slices)
#[inline]
pub fn stride_offset_checked(dim: &[Ix], strides: &[Ix], index: &[Ix]) -> Option<isize> {
    if index.len() != dim.len() {
        return None;
    }
    let mut offset = 0;
    for (&d, &i, &s) in izip!(dim, index, strides) {
        if i >= d {
            return None;
        }
        offset += stride_offset(i, s);
    }
    Some(offset)
}
/// Implementation-specific extensions to `Dimension`
pub trait DimensionExt {
    /// Get the dimension at `axis`.
    ///
    /// *Panics* if `axis` is out of bounds.
    fn axis(&self, axis: Axis) -> Ix;
    /// Set the dimension at `axis`.
    ///
    /// *Panics* if `axis` is out of bounds.
    fn set_axis(&mut self, axis: Axis, value: Ix);
}
impl<D> DimensionExt for D
where
    D: Dimension,
{
    #[inline]
    fn axis(&self, axis: Axis) -> Ix {
        self[axis.index()]
    }
    #[inline]
    fn set_axis(&mut self, axis: Axis, value: Ix) {
        self[axis.index()] = value;
    }
}
impl<'a> DimensionExt for [Ix] {
    #[inline]
    fn axis(&self, axis: Axis) -> Ix {
        self[axis.index()]
    }
    #[inline]
    fn set_axis(&mut self, axis: Axis, value: Ix) {
        self[axis.index()] = value;
    }
}
/// Collapse axis `axis` and shift so that only subarray `index` is
/// available.
///
/// **Panics** if `index` is larger than the size of the axis
pub fn do_collapse_axis<D: Dimension>(
    dims: &mut D,
    strides: &D,
    axis: usize,
    index: usize,
) -> isize {
    let dim = dims.slice()[axis];
    let stride = strides.slice()[axis];
    ndassert!(
        index < dim,
        "collapse_axis: Index {} must be less than axis length {} for \
         array with shape {:?}",
        index, dim, * dims
    );
    dims.slice_mut()[axis] = 1;
    stride_offset(index, stride)
}
/// Compute the equivalent unsigned index given the axis length and signed index.
#[inline]
pub fn abs_index(len: Ix, index: Ixs) -> Ix {
    if index < 0 { len - (-index as Ix) } else { index as Ix }
}
/// Determines nonnegative start and end indices, and performs sanity checks.
///
/// The return value is (start, end, step).
///
/// **Panics** if stride is 0 or if any index is out of bounds.
fn to_abs_slice(axis_len: usize, slice: Slice) -> (usize, usize, isize) {
    let Slice { start, end, step } = slice;
    let start = abs_index(axis_len, start);
    let mut end = abs_index(axis_len, end.unwrap_or(axis_len as isize));
    if end < start {
        end = start;
    }
    ndassert!(
        start <= axis_len, "Slice begin {} is past end of axis of length {}", start,
        axis_len,
    );
    ndassert!(
        end <= axis_len, "Slice end {} is past end of axis of length {}", end, axis_len,
    );
    ndassert!(step != 0, "Slice stride must not be zero");
    (start, end, step)
}
/// Modify dimension, stride and return data pointer offset
///
/// **Panics** if stride is 0 or if any index is out of bounds.
pub fn do_slice(dim: &mut usize, stride: &mut usize, slice: Slice) -> isize {
    let (start, end, step) = to_abs_slice(*dim, slice);
    let m = end - start;
    let s = (*stride) as isize;
    let offset = if m == 0 {
        0
    } else if step < 0 {
        stride_offset(end - 1, *stride)
    } else {
        stride_offset(start, *stride)
    };
    let abs_step = step.abs() as usize;
    *dim = if abs_step == 1 {
        m
    } else {
        let d = m / abs_step;
        let r = m % abs_step;
        d + if r > 0 { 1 } else { 0 }
    };
    *stride = if *dim <= 1 { 0 } else { (s * step) as usize };
    offset
}
/// Solves `a * x + b * y = gcd(a, b)` for `x`, `y`, and `gcd(a, b)`.
///
/// Returns `(g, (x, y))`, where `g` is `gcd(a, b)`, and `g` is always
/// nonnegative.
///
/// See https://en.wikipedia.org/wiki/Extended_Euclidean_algorithm
fn extended_gcd(a: isize, b: isize) -> (isize, (isize, isize)) {
    if a == 0 {
        (b.abs(), (0, b.signum()))
    } else if b == 0 {
        (a.abs(), (a.signum(), 0))
    } else {
        let mut r = (a, b);
        let mut s = (1, 0);
        let mut t = (0, 1);
        while r.1 != 0 {
            let q = r.0 / r.1;
            r = (r.1, r.0 - q * r.1);
            s = (s.1, s.0 - q * s.1);
            t = (t.1, t.0 - q * t.1);
        }
        if r.0 > 0 { (r.0, (s.0, t.0)) } else { (-r.0, (-s.0, -t.0)) }
    }
}
/// Solves `a * x + b * y = c` for `x` where `a`, `b`, `c`, `x`, and `y` are
/// integers.
///
/// If the return value is `Some((x0, xd))`, there is a solution. `xd` is
/// always positive. Solutions `x` are given by `x0 + xd * t` where `t` is any
/// integer. The value of `y` for any `x` is then `y = (c - a * x) / b`.
///
/// If the return value is `None`, no solutions exist.
///
/// **Note** `a` and `b` must be nonzero.
///
/// See https://en.wikipedia.org/wiki/Diophantine_equation#One_equation
/// and https://math.stackexchange.com/questions/1656120#1656138
fn solve_linear_diophantine_eq(a: isize, b: isize, c: isize) -> Option<(isize, isize)> {
    debug_assert_ne!(a, 0);
    debug_assert_ne!(b, 0);
    let (g, (u, _)) = extended_gcd(a, b);
    if c % g == 0 { Some((c / g * u, (b / g).abs())) } else { None }
}
/// Returns `true` if two (finite length) arithmetic sequences intersect.
///
/// `min*` and `max*` are the (inclusive) bounds of the sequences, and they
/// must be elements in the sequences. `step*` are the steps between
/// consecutive elements (the sign is irrelevant).
///
/// **Note** `step1` and `step2` must be nonzero.
fn arith_seq_intersect(
    (min1, max1, step1): (isize, isize, isize),
    (min2, max2, step2): (isize, isize, isize),
) -> bool {
    debug_assert!(max1 >= min1);
    debug_assert!(max2 >= min2);
    debug_assert_eq!((max1 - min1) % step1, 0);
    debug_assert_eq!((max2 - min2) % step2, 0);
    if min1 > max2 || min2 > max1 {
        false
    } else {
        let step1 = step1.abs();
        let step2 = step2.abs();
        if let Some((x0, xd)) = solve_linear_diophantine_eq(-step1, step2, min1 - min2) {
            let min = ::std::cmp::max(min1, min2);
            let max = ::std::cmp::min(max1, max2);
            min1 + step1 * (x0 - xd * div_floor(min - min1 - step1 * x0, -step1 * xd))
                <= max
                || min1
                    + step1 * (x0 + xd * div_floor(max - min1 - step1 * x0, step1 * xd))
                    >= min
        } else {
            false
        }
    }
}
/// Returns the minimum and maximum values of the indices (inclusive).
///
/// If the slice is empty, then returns `None`, otherwise returns `Some((min, max))`.
fn slice_min_max(axis_len: usize, slice: Slice) -> Option<(usize, usize)> {
    let (start, end, step) = to_abs_slice(axis_len, slice);
    if start == end {
        None
    } else if step > 0 {
        Some((start, end - 1 - (end - start - 1) % (step as usize)))
    } else {
        Some((start + (end - start - 1) % (-step as usize), end - 1))
    }
}
/// Returns `true` iff the slices intersect.
pub fn slices_intersect<D: Dimension>(
    dim: &D,
    indices1: &D::SliceArg,
    indices2: &D::SliceArg,
) -> bool {
    debug_assert_eq!(indices1.as_ref().len(), indices2.as_ref().len());
    for (&axis_len, &si1, &si2) in izip!(
        dim.slice(), indices1.as_ref(), indices2.as_ref()
    ) {
        match (si1, si2) {
            (
                SliceOrIndex::Slice { start: start1, end: end1, step: step1 },
                SliceOrIndex::Slice { start: start2, end: end2, step: step2 },
            ) => {
                let (min1, max1) = match slice_min_max(
                    axis_len,
                    Slice::new(start1, end1, step1),
                ) {
                    Some(m) => m,
                    None => return false,
                };
                let (min2, max2) = match slice_min_max(
                    axis_len,
                    Slice::new(start2, end2, step2),
                ) {
                    Some(m) => m,
                    None => return false,
                };
                if !arith_seq_intersect(
                    (min1 as isize, max1 as isize, step1),
                    (min2 as isize, max2 as isize, step2),
                ) {
                    return false;
                }
            }
            (SliceOrIndex::Slice { start, end, step }, SliceOrIndex::Index(ind))
            | (SliceOrIndex::Index(ind), SliceOrIndex::Slice { start, end, step }) => {
                let ind = abs_index(axis_len, ind);
                let (min, max) = match slice_min_max(
                    axis_len,
                    Slice::new(start, end, step),
                ) {
                    Some(m) => m,
                    None => return false,
                };
                if ind < min || ind > max || (ind - min) % step.abs() as usize != 0 {
                    return false;
                }
            }
            (SliceOrIndex::Index(ind1), SliceOrIndex::Index(ind2)) => {
                let ind1 = abs_index(axis_len, ind1);
                let ind2 = abs_index(axis_len, ind2);
                if ind1 != ind2 {
                    return false;
                }
            }
        }
    }
    true
}
pub fn merge_axes<D>(dim: &mut D, strides: &mut D, take: Axis, into: Axis) -> bool
where
    D: Dimension,
{
    let into_len = dim.axis(into);
    let into_stride = strides.axis(into) as isize;
    let take_len = dim.axis(take);
    let take_stride = strides.axis(take) as isize;
    let merged_len = into_len * take_len;
    if take_len <= 1 {
        dim.set_axis(into, merged_len);
        dim.set_axis(take, if merged_len == 0 { 0 } else { 1 });
        true
    } else if into_len <= 1 {
        strides.set_axis(into, take_stride as usize);
        dim.set_axis(into, merged_len);
        dim.set_axis(take, if merged_len == 0 { 0 } else { 1 });
        true
    } else if take_stride == into_len as isize * into_stride {
        dim.set_axis(into, merged_len);
        dim.set_axis(take, 1);
        true
    } else {
        false
    }
}
#[cfg(test)]
mod test {
    use super::{
        arith_seq_intersect, can_index_slice, can_index_slice_not_custom, extended_gcd,
        max_abs_offset_check_overflow, slice_min_max, slices_intersect,
        solve_linear_diophantine_eq, IntoDimension,
    };
    use crate::error::{from_kind, ErrorKind};
    use crate::slice::Slice;
    use crate::{Dim, Dimension, Ix0, Ix1, Ix2, Ix3, IxDyn};
    use num_integer::gcd;
    use quickcheck::{quickcheck, TestResult};
    #[test]
    fn slice_indexing_uncommon_strides() {
        let v: Vec<_> = (0..12).collect();
        let dim = (2, 3, 2).into_dimension();
        let strides = (1, 2, 6).into_dimension();
        assert!(super::can_index_slice(& v, & dim, & strides).is_ok());
        let strides = (2, 4, 12).into_dimension();
        assert_eq!(
            super::can_index_slice(& v, & dim, & strides),
            Err(from_kind(ErrorKind::OutOfBounds))
        );
    }
    #[test]
    fn overlapping_strides_dim() {
        let dim = (2, 3, 2).into_dimension();
        let strides = (5, 2, 1).into_dimension();
        assert!(super::dim_stride_overlap(& dim, & strides));
        let strides = (6, 2, 1).into_dimension();
        assert!(! super::dim_stride_overlap(& dim, & strides));
        let strides = (6, 0, 1).into_dimension();
        assert!(super::dim_stride_overlap(& dim, & strides));
        let dim = (2, 2).into_dimension();
        let strides = (3, 2).into_dimension();
        assert!(! super::dim_stride_overlap(& dim, & strides));
    }
    #[test]
    fn max_abs_offset_check_overflow_examples() {
        let dim = (1, ::std::isize::MAX as usize, 1).into_dimension();
        let strides = (1, 1, 1).into_dimension();
        max_abs_offset_check_overflow::<u8, _>(&dim, &strides).unwrap();
        let dim = (1, ::std::isize::MAX as usize, 2).into_dimension();
        let strides = (1, 1, 1).into_dimension();
        max_abs_offset_check_overflow::<u8, _>(&dim, &strides).unwrap_err();
        let dim = (0, 2, 2).into_dimension();
        let strides = (1, ::std::isize::MAX as usize, 1).into_dimension();
        max_abs_offset_check_overflow::<u8, _>(&dim, &strides).unwrap_err();
        let dim = (0, 2, 2).into_dimension();
        let strides = (1, ::std::isize::MAX as usize / 4, 1).into_dimension();
        max_abs_offset_check_overflow::<i32, _>(&dim, &strides).unwrap_err();
    }
    #[test]
    fn can_index_slice_ix0() {
        can_index_slice::<i32, _>(&[1], &Ix0(), &Ix0()).unwrap();
        can_index_slice::<i32, _>(&[], &Ix0(), &Ix0()).unwrap_err();
    }
    #[test]
    fn can_index_slice_ix1() {
        can_index_slice::<i32, _>(&[], &Ix1(0), &Ix1(0)).unwrap();
        can_index_slice::<i32, _>(&[], &Ix1(0), &Ix1(1)).unwrap();
        can_index_slice::<i32, _>(&[], &Ix1(1), &Ix1(0)).unwrap_err();
        can_index_slice::<i32, _>(&[], &Ix1(1), &Ix1(1)).unwrap_err();
        can_index_slice::<i32, _>(&[1], &Ix1(1), &Ix1(0)).unwrap();
        can_index_slice::<i32, _>(&[1], &Ix1(1), &Ix1(2)).unwrap();
        can_index_slice::<i32, _>(&[1], &Ix1(1), &Ix1(-1isize as usize)).unwrap();
        can_index_slice::<i32, _>(&[1], &Ix1(2), &Ix1(1)).unwrap_err();
        can_index_slice::<i32, _>(&[1, 2], &Ix1(2), &Ix1(0)).unwrap_err();
        can_index_slice::<i32, _>(&[1, 2], &Ix1(2), &Ix1(1)).unwrap();
        can_index_slice::<i32, _>(&[1, 2], &Ix1(2), &Ix1(-1isize as usize)).unwrap_err();
    }
    #[test]
    fn can_index_slice_ix2() {
        can_index_slice::<i32, _>(&[], &Ix2(0, 0), &Ix2(0, 0)).unwrap();
        can_index_slice::<i32, _>(&[], &Ix2(0, 0), &Ix2(2, 1)).unwrap();
        can_index_slice::<i32, _>(&[], &Ix2(0, 1), &Ix2(0, 0)).unwrap();
        can_index_slice::<i32, _>(&[], &Ix2(0, 1), &Ix2(2, 1)).unwrap();
        can_index_slice::<i32, _>(&[], &Ix2(0, 2), &Ix2(0, 0)).unwrap();
        can_index_slice::<i32, _>(&[], &Ix2(0, 2), &Ix2(2, 1)).unwrap_err();
        can_index_slice::<i32, _>(&[1], &Ix2(1, 2), &Ix2(5, 1)).unwrap_err();
        can_index_slice::<i32, _>(&[1, 2], &Ix2(1, 2), &Ix2(5, 1)).unwrap();
        can_index_slice::<i32, _>(&[1, 2], &Ix2(1, 2), &Ix2(5, 2)).unwrap_err();
        can_index_slice::<i32, _>(&[1, 2, 3, 4, 5], &Ix2(2, 2), &Ix2(3, 1)).unwrap();
        can_index_slice::<i32, _>(&[1, 2, 3, 4], &Ix2(2, 2), &Ix2(3, 1)).unwrap_err();
    }
    #[test]
    fn can_index_slice_ix3() {
        can_index_slice::<i32, _>(&[], &Ix3(0, 0, 1), &Ix3(2, 1, 3)).unwrap();
        can_index_slice::<i32, _>(&[], &Ix3(1, 1, 1), &Ix3(2, 1, 3)).unwrap_err();
        can_index_slice::<i32, _>(&[1], &Ix3(1, 1, 1), &Ix3(2, 1, 3)).unwrap();
        can_index_slice::<i32, _>(&[1; 11], &Ix3(2, 2, 3), &Ix3(6, 3, 1)).unwrap_err();
        can_index_slice::<i32, _>(&[1; 12], &Ix3(2, 2, 3), &Ix3(6, 3, 1)).unwrap();
    }
    #[test]
    fn can_index_slice_zero_size_elem() {
        can_index_slice::<(), _>(&[], &Ix1(0), &Ix1(1)).unwrap();
        can_index_slice::<(), _>(&[()], &Ix1(1), &Ix1(1)).unwrap();
        can_index_slice::<(), _>(&[(), ()], &Ix1(2), &Ix1(1)).unwrap();
        can_index_slice::<(), _>(&[], &Ix1(1), &Ix1(1)).unwrap_err();
        can_index_slice::<(), _>(&[()], &Ix1(2), &Ix1(1)).unwrap_err();
        can_index_slice::<(), _>(&[(), ()], &Ix2(2, 1), &Ix2(1, 0)).unwrap();
        can_index_slice::<(), _>(&[], &Ix2(0, 2), &Ix2(0, 0)).unwrap();
        can_index_slice::<(), _>(&[], &Ix2(0, 2), &Ix2(2, 1)).unwrap_err();
    }
    quickcheck! {
        fn can_index_slice_not_custom_same_as_can_index_slice(data : Vec < u8 >, dim :
        Vec < usize >) -> bool { let dim = IxDyn(& dim); let result =
        can_index_slice_not_custom(& data, & dim); if dim.size_checked().is_none() {
        result.is_err() } else { result == can_index_slice(& data, & dim, & dim
        .default_strides()) && result == can_index_slice(& data, & dim, & dim
        .fortran_strides()) } }
    }
    quickcheck! {
        fn extended_gcd_solves_eq(a : isize, b : isize) -> bool { let (g, (x, y)) =
        extended_gcd(a, b); a * x + b * y == g } fn extended_gcd_correct_gcd(a : isize, b
        : isize) -> bool { let (g, _) = extended_gcd(a, b); g == gcd(a, b) }
    }
    #[test]
    fn extended_gcd_zero() {
        assert_eq!(extended_gcd(0, 0), (0, (0, 0)));
        assert_eq!(extended_gcd(0, 5), (5, (0, 1)));
        assert_eq!(extended_gcd(5, 0), (5, (1, 0)));
        assert_eq!(extended_gcd(0, - 5), (5, (0, - 1)));
        assert_eq!(extended_gcd(- 5, 0), (5, (- 1, 0)));
    }
    quickcheck! {
        fn solve_linear_diophantine_eq_solution_existence(a : isize, b : isize, c :
        isize) -> TestResult { if a == 0 || b == 0 { TestResult::discard() } else {
        TestResult::from_bool((c % gcd(a, b) == 0) == solve_linear_diophantine_eq(a, b,
        c).is_some()) } } fn solve_linear_diophantine_eq_correct_solution(a : isize, b :
        isize, c : isize, t : isize) -> TestResult { if a == 0 || b == 0 {
        TestResult::discard() } else { match solve_linear_diophantine_eq(a, b, c) {
        Some((x0, xd)) => { let x = x0 + xd * t; let y = (c - a * x) / b;
        TestResult::from_bool(a * x + b * y == c) } None => TestResult::discard(), } } }
    }
    quickcheck! {
        fn arith_seq_intersect_correct(first1 : isize, len1 : isize, step1 : isize,
        first2 : isize, len2 : isize, step2 : isize) -> TestResult { use std::cmp; if
        len1 == 0 || len2 == 0 { return TestResult::discard(); } let len1 = len1.abs();
        let len2 = len2.abs(); let last1 = first1 + step1 * (len1 - 1); let (min1, max1)
        = (cmp::min(first1, last1), cmp::max(first1, last1)); let last2 = first2 + step2
        * (len2 - 1); let (min2, max2) = (cmp::min(first2, last2), cmp::max(first2,
        last2)); let seq1 : Vec < _ > = (0..len1).map(| n | first1 + step1 * n)
        .collect(); let intersects = (0..len2).map(| n | first2 + step2 * n).any(| elem2
        | seq1.contains(& elem2)); TestResult::from_bool(arith_seq_intersect((min1, max1,
        if step1 == 0 { 1 } else { step1 }), (min2, max2, if step2 == 0 { 1 } else {
        step2 })) == intersects) }
    }
    #[test]
    fn slice_min_max_empty() {
        assert_eq!(slice_min_max(0, Slice::new(0, None, 3)), None);
        assert_eq!(slice_min_max(10, Slice::new(1, Some(1), 3)), None);
        assert_eq!(slice_min_max(10, Slice::new(- 1, Some(- 1), 3)), None);
        assert_eq!(slice_min_max(10, Slice::new(1, Some(1), - 3)), None);
        assert_eq!(slice_min_max(10, Slice::new(- 1, Some(- 1), - 3)), None);
    }
    #[test]
    fn slice_min_max_pos_step() {
        assert_eq!(slice_min_max(10, Slice::new(1, Some(8), 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(1, Some(9), 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 9, Some(8), 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 9, Some(9), 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(1, Some(- 2), 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(1, Some(- 1), 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 9, Some(- 2), 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 9, Some(- 1), 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(1, None, 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 9, None, 3)), Some((1, 7)));
        assert_eq!(slice_min_max(11, Slice::new(1, None, 3)), Some((1, 10)));
        assert_eq!(slice_min_max(11, Slice::new(- 10, None, 3)), Some((1, 10)));
    }
    #[test]
    fn slice_min_max_neg_step() {
        assert_eq!(slice_min_max(10, Slice::new(1, Some(8), - 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(2, Some(8), - 3)), Some((4, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 9, Some(8), - 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 8, Some(8), - 3)), Some((4, 7)));
        assert_eq!(slice_min_max(10, Slice::new(1, Some(- 2), - 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(2, Some(- 2), - 3)), Some((4, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 9, Some(- 2), - 3)), Some((1, 7)));
        assert_eq!(slice_min_max(10, Slice::new(- 8, Some(- 2), - 3)), Some((4, 7)));
        assert_eq!(slice_min_max(9, Slice::new(2, None, - 3)), Some((2, 8)));
        assert_eq!(slice_min_max(9, Slice::new(- 7, None, - 3)), Some((2, 8)));
        assert_eq!(slice_min_max(9, Slice::new(3, None, - 3)), Some((5, 8)));
        assert_eq!(slice_min_max(9, Slice::new(- 6, None, - 3)), Some((5, 8)));
    }
    #[test]
    fn slices_intersect_true() {
        assert!(slices_intersect(& Dim([4, 5]), s![.., ..], s![.., ..]));
        assert!(slices_intersect(& Dim([4, 5]), s![0, ..], s![0, ..]));
        assert!(slices_intersect(& Dim([4, 5]), s![..; 2, ..], s![..; 3, ..]));
        assert!(slices_intersect(& Dim([4, 5]), s![.., ..; 2], s![.., 1..; 3]));
        assert!(slices_intersect(& Dim([4, 10]), s![.., ..; 9], s![.., 3..; 6]));
    }
    #[test]
    fn slices_intersect_false() {
        assert!(! slices_intersect(& Dim([4, 5]), s![..; 2, ..], s![1..; 2, ..]));
        assert!(! slices_intersect(& Dim([4, 5]), s![..; 2, ..], s![1..; 3, ..]));
        assert!(! slices_intersect(& Dim([4, 5]), s![.., ..; 9], s![.., 3..; 6]));
    }
}
#[cfg(test)]
mod tests_rug_279 {
    use super::*;
    use crate::dimension::{Ix, Ixs, stride_offset};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        stride_offset(p0 as Ix, p1 as Ix);
             }
});    }
}
#[cfg(test)]
mod tests_rug_281 {
    use super::*;
    use crate::{Dimension, Array, Ix1};
    #[test]
    fn test_size_of_shape_checked() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let shape: Ix1 = Array::from(vec![rug_fuzz_0, 3, 4]).raw_dim();
        let result = size_of_shape_checked(&shape);
        debug_assert_eq!(result, Ok(24));
             }
});    }
}
#[cfg(test)]
mod tests_rug_285 {
    use super::*;
    use crate::dimension::{stride_offset_checked, stride_offset};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(usize, usize, usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = [rug_fuzz_0, rug_fuzz_1];
        let mut p1 = [rug_fuzz_2, rug_fuzz_3];
        let mut p2 = [rug_fuzz_4, rug_fuzz_5];
        crate::dimension::stride_offset_checked(&p0, &p1, &p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_287 {
    use super::*;
    use crate::dimension::abs_index;
    #[test]
    fn test_abs_index() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: isize = -rug_fuzz_1;
        abs_index(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_288 {
    use super::*;
    use crate::Slice;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, isize, isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1 = Slice::new(rug_fuzz_1, Some(rug_fuzz_2), rug_fuzz_3);
        crate::dimension::to_abs_slice(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_289 {
    use super::*;
    use crate::Slice;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(usize, usize, isize, isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        let mut p2 = Slice::new(rug_fuzz_2, Some(rug_fuzz_3), rug_fuzz_4);
        crate::dimension::do_slice(&mut p0, &mut p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_290 {
    use super::*;
    #[test]
    fn test_extended_gcd() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: isize = rug_fuzz_0;
        let mut p1: isize = rug_fuzz_1;
        crate::dimension::extended_gcd(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_291 {
    use super::*;
    use crate::dimension::solve_linear_diophantine_eq;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(isize, isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: isize = rug_fuzz_0;
        let mut p1: isize = rug_fuzz_1;
        let mut p2: isize = rug_fuzz_2;
        crate::dimension::solve_linear_diophantine_eq(p0, p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_292 {
    use super::*;
    use crate::dimension::arith_seq_intersect;
    use crate::dimension::solve_linear_diophantine_eq;
    use std::cmp::{max, min};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(isize, isize, isize, isize, isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = (rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let mut p1 = (rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        arith_seq_intersect(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_293 {
    use super::*;
    use crate::{Slice, Dimension};
    #[test]
    fn test_slice_min_max() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, isize, isize, isize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1 = Slice::new(rug_fuzz_1, Some(rug_fuzz_2), rug_fuzz_3);
        crate::dimension::slice_min_max(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_295 {
    use super::*;
    use crate::prelude::{IxDyn, Dimension, Axis};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = IxDyn::zeros(rug_fuzz_1);
        let mut p2 = Axis(rug_fuzz_2);
        let mut p3 = Axis(rug_fuzz_3);
        crate::dimension::merge_axes(&mut p0, &mut p1, p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_296 {
    use super::*;
    use crate::imp_prelude::DimensionExt;
    use crate::prelude::{IxDyn, Dimension, Axis};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = Axis(rug_fuzz_1);
        p0.axis(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_297 {
    use super::*;
    use crate::aliases::IxDyn;
    use crate::prelude::{Dimension, Axis};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = IxDyn::zeros(rug_fuzz_0);
        let mut p1 = Axis(rug_fuzz_1);
        let p2: usize = rug_fuzz_2;
        p0.set_axis(p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_298 {
    use super::*;
    use crate::imp_prelude::DimensionExt;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [usize; 3] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut p1 = Axis(rug_fuzz_3);
        <[usize] as DimensionExt>::axis(&p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_299 {
    use super::*;
    use crate::imp_prelude::DimensionExt;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = vec![rug_fuzz_0, 2, 3];
        let mut p1 = Axis(rug_fuzz_1);
        let mut p2 = rug_fuzz_2;
        p0.set_axis(p1, p2);
             }
});    }
}
