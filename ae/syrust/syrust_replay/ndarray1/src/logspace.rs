use num_traits::Float;
/// An iterator of a sequence of logarithmically spaced number.
///
/// Iterator element type is `F`.
pub struct Logspace<F> {
    sign: F,
    base: F,
    start: F,
    step: F,
    index: usize,
    len: usize,
}
impl<F> Iterator for Logspace<F>
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
            let exponent = self.start + self.step * F::from(i).unwrap();
            Some(self.sign * self.base.powf(exponent))
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len - self.index;
        (n, Some(n))
    }
}
impl<F> DoubleEndedIterator for Logspace<F>
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
            let exponent = self.start + self.step * F::from(i).unwrap();
            Some(self.sign * self.base.powf(exponent))
        }
    }
}
impl<F> ExactSizeIterator for Logspace<F>
where
    Logspace<F>: Iterator,
{}
/// An iterator of a sequence of logarithmically spaced numbers.
///
/// The `Logspace` has `n` elements, where the first element is `base.powf(a)`
/// and the last element is `base.powf(b)`.  If `base` is negative, this
/// iterator will return all negative values.
///
/// The iterator element type is `F`, where `F` must implement `Float`, e.g.
/// `f32` or `f64`.
///
/// **Panics** if converting `n - 1` to type `F` fails.
#[inline]
pub fn logspace<F>(base: F, a: F, b: F, n: usize) -> Logspace<F>
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
    Logspace {
        sign: base.signum(),
        base: base.abs(),
        start: a,
        step,
        index: 0,
        len: n,
    }
}
#[cfg(test)]
mod tests {
    use super::logspace;
    #[test]
    #[cfg(feature = "approx")]
    fn valid() {
        use crate::{arr1, Array1};
        use approx::assert_abs_diff_eq;
        let array: Array1<_> = logspace(10.0, 0.0, 3.0, 4).collect();
        assert_abs_diff_eq!(array, arr1(& [1e0, 1e1, 1e2, 1e3]));
        let array: Array1<_> = logspace(10.0, 3.0, 0.0, 4).collect();
        assert_abs_diff_eq!(array, arr1(& [1e3, 1e2, 1e1, 1e0]));
        let array: Array1<_> = logspace(-10.0, 3.0, 0.0, 4).collect();
        assert_abs_diff_eq!(array, arr1(& [- 1e3, - 1e2, - 1e1, - 1e0]));
        let array: Array1<_> = logspace(-10.0, 0.0, 3.0, 4).collect();
        assert_abs_diff_eq!(array, arr1(& [- 1e0, - 1e1, - 1e2, - 1e3]));
    }
    #[test]
    fn iter_forward() {
        let mut iter = logspace(10.0f64, 0.0, 3.0, 4);
        assert!(iter.size_hint() == (4, Some(4)));
        assert!((iter.next().unwrap() - 1e0).abs() < 1e-5);
        assert!((iter.next().unwrap() - 1e1).abs() < 1e-5);
        assert!((iter.next().unwrap() - 1e2).abs() < 1e-5);
        assert!((iter.next().unwrap() - 1e3).abs() < 1e-5);
        assert!(iter.next().is_none());
        assert!(iter.size_hint() == (0, Some(0)));
    }
    #[test]
    fn iter_backward() {
        let mut iter = logspace(10.0f64, 0.0, 3.0, 4);
        assert!(iter.size_hint() == (4, Some(4)));
        assert!((iter.next_back().unwrap() - 1e3).abs() < 1e-5);
        assert!((iter.next_back().unwrap() - 1e2).abs() < 1e-5);
        assert!((iter.next_back().unwrap() - 1e1).abs() < 1e-5);
        assert!((iter.next_back().unwrap() - 1e0).abs() < 1e-5);
        assert!(iter.next_back().is_none());
        assert!(iter.size_hint() == (0, Some(0)));
    }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::logspace::Logspace;
    use std::iter::Iterator;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(f64, f64, f64, f64, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let start = rug_fuzz_0;
        let step = rug_fuzz_1;
        let base = rug_fuzz_2;
        let sign = rug_fuzz_3;
        let len = rug_fuzz_4;
        let mut p0 = Logspace::<f64> {
            start,
            step,
            base,
            sign,
            len,
            index: rug_fuzz_5,
        };
        p0.next();
             }
}
}
}    }
}
