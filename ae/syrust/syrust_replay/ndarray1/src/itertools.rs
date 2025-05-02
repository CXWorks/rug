//! A few iterator-related utilities and tools
use std::iter;
/// Iterate `iterable` with a running index.
///
/// `IntoIterator` enabled version of `.enumerate()`.
///
/// ```
/// use itertools::enumerate;
///
/// for (i, elt) in enumerate(&[1, 2, 3]) {
///     /* loop body */
/// }
/// ```
pub(crate) fn enumerate<I>(iterable: I) -> iter::Enumerate<I::IntoIter>
where
    I: IntoIterator,
{
    iterable.into_iter().enumerate()
}
/// Iterate `i` and `j` in lock step.
///
/// `IntoIterator` enabled version of `i.zip(j)`.
///
/// ```
/// use itertools::zip;
///
/// let data = [1, 2, 3, 4, 5];
/// for (a, b) in zip(&data, &data[1..]) {
///     /* loop body */
/// }
/// ```
pub(crate) fn zip<I, J>(i: I, j: J) -> iter::Zip<I::IntoIter, J::IntoIter>
where
    I: IntoIterator,
    J: IntoIterator,
{
    i.into_iter().zip(j)
}
/// Create an iterator running multiple iterators in lockstep.
///
/// The `izip!` iterator yields elements until any subiterator
/// returns `None`.
///
/// This is a version of the standard ``.zip()`` that's supporting more than
/// two iterators. The iterator element type is a tuple with one element
/// from each of the input iterators. Just like ``.zip()``, the iteration stops
/// when the shortest of the inputs reaches its end.
///
/// **Note:** The result of this macro is in the general case an iterator
/// composed of repeated `.zip()` and a `.map()`; it has an anonymous type.
/// The special cases of one and two arguments produce the equivalent of
/// `$a.into_iter()` and `$a.into_iter().zip($b)` respectively.
///
/// Prefer this macro `izip!()` over [`multizip`] for the performance benefits
/// of using the standard library `.zip()`.
///
/// [`multizip`]: fn.multizip.html
///
/// ```
/// #[macro_use] extern crate itertools;
/// # fn main() {
///
/// // iterate over three sequences side-by-side
/// let mut results = [0, 0, 0, 0];
/// let inputs = [3, 7, 9, 6];
///
/// for (r, index, input) in izip!(&mut results, 0..10, &inputs) {
///     *r = index * 10 + input;
/// }
///
/// assert_eq!(results, [0 + 3, 10 + 7, 29, 36]);
/// # }
/// ```
///
/// **Note:** To enable the macros in this crate, use the `#[macro_use]`
/// attribute when importing the crate:
///
/// ```
/// #[macro_use] extern crate itertools;
/// # fn main() { }
/// ```
macro_rules! izip {
    (@ closure $p:pat => $tup:expr) => {
        |$p | $tup
    };
    (@ closure $p:pat => ($($tup:tt)*), $_iter:expr $(, $tail:expr)*) => {
        izip!(@ closure($p, b) => ($($tup)*, b) $(, $tail)*)
    };
    ($first:expr $(,)*) => {
        IntoIterator::into_iter($first)
    };
    ($first:expr, $second:expr $(,)*) => {
        izip!($first) .zip($second)
    };
    ($first:expr $(, $rest:expr)* $(,)*) => {
        izip!($first) $(.zip($rest))* .map(izip!(@ closure a => (a) $(, $rest)*))
    };
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(i32, i32, i32, i32, i32, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: [i32; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let shape = (rug_fuzz_5,);
        let p0 = ArrayView::<i32, _>::from_shape(shape, &data).unwrap();
        crate::itertools::enumerate(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7, mut rug_fuzz_8, mut rug_fuzz_9, mut rug_fuzz_10)) = <(i32, i32, i32, i32, i32, usize, i32, i32, i32, i32, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data1: [i32; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        let shape1 = (rug_fuzz_5,);
        let data2: [i32; 4] = [rug_fuzz_6, rug_fuzz_7, rug_fuzz_8, rug_fuzz_9];
        let shape2 = (rug_fuzz_10,);
        let p0 = ArrayView::<i32, _>::from_shape(shape1, &data1).unwrap();
        let p1 = ArrayView::<i32, _>::from_shape(shape2, &data2).unwrap();
        crate::itertools::zip(p0.into_owned().into_iter(), p1.into_owned().into_iter());
             }
}
}
}    }
}
