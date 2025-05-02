//! The public parts of this private module are used to create traits
//! that cannot be implemented outside of our own crate.  This way we
//! can feel free to extend those traits without worrying about it
//! being a breaking change for other implementations.

/// If this type is pub but not publicly reachable, third parties
/// can't name it and can't implement traits using it.
pub struct PrivateMarker;

macro_rules! private_decl {
    () => {
        /// This trait is private to implement; this method exists to make it
        /// impossible to implement outside the crate.
        fn __private__(&self) -> crate::private::PrivateMarker;
    }
}

macro_rules! private_impl {
    () => {
        #[doc(hidden)]
        fn __private__(&self) -> crate::private::PrivateMarker {
            crate::private::PrivateMarker
        }
    }
}

#[cfg(test)]
mod tests_rug_492 {
    use super::*;
    use crate::{imp_prelude::{RawData, ViewRepr}, private::PrivateMarker};

    #[test]
    fn test_rug() {
        let mut p0: &ViewRepr<&'static i32> = &ViewRepr::new();

        <ViewRepr<&'static i32> as RawData>::__private__(p0);
    }
}
#[cfg(test)]
mod tests_rug_493 {
    use super::*;
    use crate::imp_prelude::RawData;
    use crate::ViewRepr;

    #[test]
    fn test_rug() {
        let mut p0: ViewRepr<&'static mut i32> = ViewRepr::new();

        <ViewRepr<&'static mut i32> as RawData>::__private__(&p0);
    }
}#[cfg(test)]
mod tests_rug_505 {
    use super::*;
    use crate::zip::Offset;
    use crate::Array1;

    #[test]
    fn test_rug() {
        let data = vec![1, 2, 3, 4, 5];
        let array = Array1::from(data);
        let p0 = array.as_ptr();

        p0.__private__();
    }
}#[cfg(test)]
mod tests_rug_506 {
    use super::*;
    use crate::zip::Offset;
    use crate::{ArrayViewMut2, Array2};

    #[test]
    fn test_rug() {
        let mut data: Array2<f64> = Array2::from_shape_vec((2, 2), vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let p0: *mut f64 = data.as_slice_mut().unwrap().as_mut_ptr();

        p0.__private__();
    }
}#[cfg(test)]
mod tests_rug_508 {
    use super::*;
    use crate::{NdProducer, ArrayBase, ViewRepr};
    use crate::Array;

    #[test]
    fn test_rug() {
        let mut array_data = vec![1, 2, 3, 4, 5, 6];
        let mut array = Array::from_shape_vec((2, 3), array_data).unwrap();
        let mut v23 = array.view_mut();

        let p0: ArrayBase<ViewRepr<&mut i32>, _> = v23;

        p0.__private__();
    }
}use crate::dimension::Dim;
use crate::prelude::Dimension;

#[cfg(test)]
mod tests_rug_513 {
    use super::*;
    use crate::dimension::Dim;
    #[test]
    fn test_rug() {
        let mut p0: Dim<[usize; 2]> = Dim([2, 3]);

        <Dim<[usize; 2]> as Dimension>::__private__(&p0);
    }
}