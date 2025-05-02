// Copyright 2014-2016 bluss and ndarray developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::{Axis, Dim, Dimension, Ix, Ix0, Ix1};

/// Array shape with a next smaller dimension.
///
/// `RemoveAxis` defines a larger-than relation for array shapes:
/// removing one axis from *Self* gives smaller dimension *Smaller*.
pub trait RemoveAxis: Dimension {
    fn remove_axis(&self, axis: Axis) -> Self::Smaller;
}

impl RemoveAxis for Dim<[Ix; 1]> {
    #[inline]
    fn remove_axis(&self, axis: Axis) -> Ix0 {
        debug_assert!(axis.index() < self.ndim());
        Ix0()
    }
}

impl RemoveAxis for Dim<[Ix; 2]> {
    #[inline]
    fn remove_axis(&self, axis: Axis) -> Ix1 {
        let axis = axis.index();
        debug_assert!(axis < self.ndim());
        if axis == 0 {
            Ix1(get!(self, 1))
        } else {
            Ix1(get!(self, 0))
        }
    }
}

macro_rules! impl_remove_axis_array(
    ($($n:expr),*) => (
    $(
        impl RemoveAxis for Dim<[Ix; $n]>
        {
            #[inline]
            fn remove_axis(&self, axis: Axis) -> Self::Smaller {
                debug_assert!(axis.index() < self.ndim());
                let mut result = Dim([0; $n - 1]);
                {
                    let src = self.slice();
                    let dst = result.slice_mut();
                    dst[..axis.index()].copy_from_slice(&src[..axis.index()]);
                    dst[axis.index()..].copy_from_slice(&src[axis.index() + 1..]);
                }
                result
            }
        }
    )*
    );
);

impl_remove_axis_array!(3, 4, 5, 6);
#[cfg(test)]
mod tests_rug_978 {
    use super::*;
    use crate::imp_prelude::RemoveAxis;
    use crate::dimension::{Dim, remove_axis};
    use crate::prelude::*;

    #[test]
    fn test_rug() {
        let mut p0: Dim<[usize; 2]> = Dim([2, 3]);
        let mut p1 = Axis(2);

        <Dim<[usize; 2]> as remove_axis::RemoveAxis>::remove_axis(&p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_979 {
    use super::*;
    use crate::imp_prelude::RemoveAxis;
    use crate::{Dimension, Dim, Axis};

    #[test]
    fn test_rug() {
        let mut p0: Dim<[usize; 3]> = Dim([3, 4, 5]);
        let mut p1 = Axis(2);

        p0.remove_axis(p1);
    }
}
#[cfg(test)]
mod tests_rug_981 {
    use super::*;
    use crate::imp_prelude::RemoveAxis;
    use crate::{Dim, Dimension, Axis};

    #[test]
    fn test_remove_axis() {
        let mut p0: Dim<[usize; 5]> = Dim([1, 2, 3, 4, 5]);
        let mut p1 = Axis(2);

        p0.remove_axis(p1);

        // Add assertions here
    }
}