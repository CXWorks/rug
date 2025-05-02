use crate::error::{from_kind, ErrorKind, ShapeError};
use crate::imp_prelude::*;
/// Stack arrays along the given axis.
///
/// ***Errors*** if the arrays have mismatching shapes, apart from along `axis`.
/// (may be made more flexible in the future).<br>
/// ***Errors*** if `arrays` is empty, if `axis` is out of bounds,
/// if the result is larger than is possible to represent.
///
/// ```
/// use ndarray::{arr2, Axis, stack};
///
/// let a = arr2(&[[2., 2.],
///                [3., 3.]]);
/// assert!(
///     stack(Axis(0), &[a.view(), a.view()])
///     == Ok(arr2(&[[2., 2.],
///                  [3., 3.],
///                  [2., 2.],
///                  [3., 3.]]))
/// );
/// ```
pub fn stack<'a, A, D>(
    axis: Axis,
    arrays: &[ArrayView<'a, A, D>],
) -> Result<Array<A, D>, ShapeError>
where
    A: Copy,
    D: RemoveAxis,
{
    if arrays.is_empty() {
        return Err(from_kind(ErrorKind::Unsupported));
    }
    let mut res_dim = arrays[0].raw_dim();
    if axis.index() >= res_dim.ndim() {
        return Err(from_kind(ErrorKind::OutOfBounds));
    }
    let common_dim = res_dim.remove_axis(axis);
    if arrays.iter().any(|a| a.raw_dim().remove_axis(axis) != common_dim) {
        return Err(from_kind(ErrorKind::IncompatibleShape));
    }
    let stacked_dim = arrays.iter().fold(0, |acc, a| acc + a.len_of(axis));
    res_dim.set_axis(axis, stacked_dim);
    let size = res_dim.size();
    let mut v = Vec::with_capacity(size);
    unsafe {
        v.set_len(size);
    }
    let mut res = Array::from_shape_vec(res_dim, v)?;
    {
        let mut assign_view = res.view_mut();
        for array in arrays {
            let len = array.len_of(axis);
            let (mut front, rest) = assign_view.split_at(axis, len);
            front.assign(array);
            assign_view = rest;
        }
    }
    Ok(res)
}
/// Stack arrays along the given axis.
///
/// Uses the [`stack`][1] function, calling `ArrayView::from(&a)` on each
/// argument `a`.
///
/// [1]: fn.stack.html
///
/// ***Panics*** if the `stack` function would return an error.
///
/// ```
/// extern crate ndarray;
///
/// use ndarray::{arr2, stack, Axis};
///
/// # fn main() {
///
/// let a = arr2(&[[2., 2.],
///                [3., 3.]]);
/// assert!(
///     stack![Axis(0), a, a]
///     == arr2(&[[2., 2.],
///               [3., 3.],
///               [2., 2.],
///               [3., 3.]])
/// );
/// # }
/// ```
#[macro_export]
macro_rules! stack {
    ($axis:expr, $($array:expr),+) => {
        $crate ::stack($axis, & [$($crate ::ArrayView::from(&$array)),*]).unwrap()
    };
}
use crate::prelude::*;
use crate::{Axis, ArrayView, Array};
#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, f64, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Axis(rug_fuzz_0);
        let data = vec![rug_fuzz_1, 2., 3., 3.];
        let a = Array::from_shape_vec((rug_fuzz_2, rug_fuzz_3), data).unwrap();
        let p1 = vec![a.view(), a.view()];
        crate::stacking::stack(p0, &p1).unwrap();
             }
}
}
}    }
}
