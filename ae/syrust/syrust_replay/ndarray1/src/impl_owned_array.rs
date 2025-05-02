use crate::imp_prelude::*;
/// Methods specific to `Array0`.
///
/// ***See also all methods for [`ArrayBase`]***
///
/// [`ArrayBase`]: struct.ArrayBase.html
impl<A> Array<A, Ix0> {
    /// Returns the single element in the array without cloning it.
    ///
    /// ```
    /// use ndarray::{arr0, Array0};
    ///
    /// // `Foo` doesn't implement `Clone`.
    /// #[derive(Debug, Eq, PartialEq)]
    /// struct Foo;
    ///
    /// let array: Array0<Foo> = arr0(Foo);
    /// let scalar: Foo = array.into_scalar();
    /// assert_eq!(scalar, Foo);
    /// ```
    pub fn into_scalar(self) -> A {
        let size = ::std::mem::size_of::<A>();
        if size == 0 {
            self.data.into_vec().remove(0)
        } else {
            let first = self.ptr.as_ptr() as usize;
            let base = self.data.as_ptr() as usize;
            let index = (first - base) / size;
            debug_assert_eq!((first - base) % size, 0);
            self.data.into_vec().remove(index)
        }
    }
}
/// Methods specific to `Array`.
///
/// ***See also all methods for [`ArrayBase`]***
///
/// [`ArrayBase`]: struct.ArrayBase.html
impl<A, D> Array<A, D>
where
    D: Dimension,
{
    /// Return a vector of the elements in the array, in the way they are
    /// stored internally.
    ///
    /// If the array is in standard memory layout, the logical element order
    /// of the array (`.iter()` order) and of the returned vector will be the same.
    pub fn into_raw_vec(self) -> Vec<A> {
        self.data.into_vec()
    }
}
#[cfg(test)]
mod tests_rug_1121 {
    use super::*;
    use crate::{Array0, ArrayBase, Data, Dimension};
    use crate::data_repr::OwnedRepr;
    #[test]
    fn test_into_scalar() {
        let _rug_st_tests_rug_1121_rrrruuuugggg_test_into_scalar = 0;
        #[derive(Debug, PartialEq)]
        struct Foo;
        let array: Array0<Foo> = Array0::from_shape_vec(().strides(()), vec![Foo])
            .unwrap();
        let scalar: Foo = array.into_scalar();
        debug_assert_eq!(scalar, Foo);
        let _rug_ed_tests_rug_1121_rrrruuuugggg_test_into_scalar = 0;
    }
}
#[cfg(test)]
mod tests_rug_1122 {
    use super::*;
    use crate::{ArrayBase, data_repr, Data, OwnedRepr};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(i32, usize, usize, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = vec![rug_fuzz_0, 2, 3, 4];
        let shape = (rug_fuzz_1, rug_fuzz_2);
        let strides = (rug_fuzz_3, rug_fuzz_4);
        let p0 = ArrayBase::<OwnedRepr<i32>, _>::from_shape_vec(shape, data).unwrap();
        let _result = p0.into_raw_vec();
             }
}
}
}    }
}
