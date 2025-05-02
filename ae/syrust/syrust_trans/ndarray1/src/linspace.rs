use num_traits::Float;
/// An iterator of a sequence of evenly spaced floats.
///
/// Iterator element type is `F`.
pub struct Linspace<F> {
    start: F,
    step: F,
    index: usize,
    len: usize,
}
impl<F> Iterator for Linspace<F>
where
    F: Float,
{
    type Item = F;
    #[inline]
    fn next(&mut self) -> Option<F> {
        if self.index >= self.len {
            None
        } else {
            let i = self.index;
            self.index += 1;
            Some(self.start + self.step * F::from(i).unwrap())
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len - self.index;
        (n, Some(n))
    }
}
impl<F> DoubleEndedIterator for Linspace<F>
where
    F: Float,
{
    #[inline]
    fn next_back(&mut self) -> Option<F> {
        if self.index >= self.len {
            None
        } else {
            self.len -= 1;
            let i = self.len;
            Some(self.start + self.step * F::from(i).unwrap())
        }
    }
}
impl<F> ExactSizeIterator for Linspace<F>
where
    Linspace<F>: Iterator,
{}
/// Return an iterator of evenly spaced floats.
///
/// The `Linspace` has `n` elements from `a` to `b` (inclusive).
///
/// The iterator element type is `F`, where `F` must implement `Float`, e.g.
/// `f32` or `f64`.
///
/// **Panics** if converting `n - 1` to type `F` fails.
#[inline]
pub fn linspace<F>(a: F, b: F, n: usize) -> Linspace<F>
where
    F: Float,
{
    let step = if n > 1 {
        let num_steps = F::from(n - 1)
            .expect("Converting number of steps to `A` must not fail.");
        (b - a) / num_steps
    } else {
        F::zero()
    };
    Linspace {
        start: a,
        step,
        index: 0,
        len: n,
    }
}
/// Return an iterator of floats from `start` to `end` (exclusive),
/// incrementing by `step`.
///
/// Numerical reasons can result in `b` being included in the result.
///
/// The iterator element type is `F`, where `F` must implement `Float`, e.g.
/// `f32` or `f64`.
///
/// **Panics** if converting `((b - a) / step).ceil()` to type `F` fails.
#[inline]
pub fn range<F>(a: F, b: F, step: F) -> Linspace<F>
where
    F: Float,
{
    let len = b - a;
    let steps = F::ceil(len / step);
    Linspace {
        start: a,
        step,
        len: steps
            .to_usize()
            .expect(
                "Converting the length to `usize` must not fail. The most likely \
             cause of this failure is if the sign of `end - start` is \
             different from the sign of `step`.",
            ),
        index: 0,
    }
}
#[cfg(test)]
mod tests_rug_237 {
    use super::*;
    use crate::linspace;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_237_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0.0;
        let rug_fuzz_1 = 10.0;
        let rug_fuzz_2 = 5;
        let mut p0: linspace::Linspace<f64> = linspace(
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
        );
        p0.next();
        let _rug_ed_tests_rug_237_rrrruuuugggg_test_rug = 0;
    }
}
