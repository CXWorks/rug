// Copyright 2014-2016 bluss and ndarray developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use num_complex::Complex;

/// Elements that can be used as direct operands in arithmetic with arrays.
///
/// For example, `f64` is a `ScalarOperand` which means that for an array `a`,
/// arithmetic like `a + 1.0`, and, `a * 2.`, and `a += 3.` are allowed.
///
/// In the description below, let `A` be an array or array view,
/// let `B` be an array with owned data,
/// and let `C` be an array with mutable data.
///
/// `ScalarOperand` determines for which scalars `K` operations `&A @ K`, and `B @ K`,
/// and `C @= K` are defined, as ***right hand side operands***, for applicable
/// arithmetic operators (denoted `@`).
///
/// ***Left hand side*** scalar operands are not related to this trait
/// (they need one `impl` per concrete scalar type); but they are still
/// implemented for the same types, allowing operations
/// `K @ &A`, and `K @ B` for primitive numeric types `K`.
///
/// This trait ***does not*** limit which elements can be stored in an array in general.
/// Non-`ScalarOperand` types can still participate in arithmetic as array elements in
/// in array-array operations.
pub trait ScalarOperand: 'static + Clone {}
impl ScalarOperand for bool {}
impl ScalarOperand for i8 {}
impl ScalarOperand for u8 {}
impl ScalarOperand for i16 {}
impl ScalarOperand for u16 {}
impl ScalarOperand for i32 {}
impl ScalarOperand for u32 {}
impl ScalarOperand for i64 {}
impl ScalarOperand for u64 {}
impl ScalarOperand for i128 {}
impl ScalarOperand for u128 {}
impl ScalarOperand for isize {}
impl ScalarOperand for usize {}
impl ScalarOperand for f32 {}
impl ScalarOperand for f64 {}
impl ScalarOperand for Complex<f32> {}
impl ScalarOperand for Complex<f64> {}

macro_rules! impl_binary_op(
    ($trt:ident, $operator:tt, $mth:ident, $iop:tt, $doc:expr) => (
/// Perform elementwise
#[doc=$doc]
/// between `self` and `rhs`,
/// and return the result (based on `self`).
///
/// `self` must be an `Array` or `ArcArray`.
///
/// If their shapes disagree, `rhs` is broadcast to the shape of `self`.
///
/// **Panics** if broadcasting isn’t possible.
impl<A, B, S, S2, D, E> $trt<ArrayBase<S2, E>> for ArrayBase<S, D>
where
    A: Clone + $trt<B, Output=A>,
    B: Clone,
    S: DataOwned<Elem=A> + DataMut,
    S2: Data<Elem=B>,
    D: Dimension,
    E: Dimension,
{
    type Output = ArrayBase<S, D>;
    fn $mth(self, rhs: ArrayBase<S2, E>) -> ArrayBase<S, D>
    {
        self.$mth(&rhs)
    }
}

/// Perform elementwise
#[doc=$doc]
/// between `self` and reference `rhs`,
/// and return the result (based on `self`).
///
/// If their shapes disagree, `rhs` is broadcast to the shape of `self`.
///
/// **Panics** if broadcasting isn’t possible.
impl<'a, A, B, S, S2, D, E> $trt<&'a ArrayBase<S2, E>> for ArrayBase<S, D>
where
    A: Clone + $trt<B, Output=A>,
    B: Clone,
    S: DataOwned<Elem=A> + DataMut,
    S2: Data<Elem=B>,
    D: Dimension,
    E: Dimension,
{
    type Output = ArrayBase<S, D>;
    fn $mth(mut self, rhs: &ArrayBase<S2, E>) -> ArrayBase<S, D>
    {
        self.zip_mut_with(rhs, |x, y| {
            *x = x.clone() $operator y.clone();
        });
        self
    }
}

/// Perform elementwise
#[doc=$doc]
/// between references `self` and `rhs`,
/// and return the result as a new `Array`.
///
/// If their shapes disagree, `rhs` is broadcast to the shape of `self`.
///
/// **Panics** if broadcasting isn’t possible.
impl<'a, A, B, S, S2, D, E> $trt<&'a ArrayBase<S2, E>> for &'a ArrayBase<S, D>
where
    A: Clone + $trt<B, Output=A>,
    B: Clone,
    S: Data<Elem=A>,
    S2: Data<Elem=B>,
    D: Dimension,
    E: Dimension,
{
    type Output = Array<A, D>;
    fn $mth(self, rhs: &'a ArrayBase<S2, E>) -> Array<A, D> {
        // FIXME: Can we co-broadcast arrays here? And how?
        self.to_owned().$mth(rhs)
    }
}

/// Perform elementwise
#[doc=$doc]
/// between `self` and the scalar `x`,
/// and return the result (based on `self`).
///
/// `self` must be an `Array` or `ArcArray`.
impl<A, S, D, B> $trt<B> for ArrayBase<S, D>
    where A: Clone + $trt<B, Output=A>,
          S: DataOwned<Elem=A> + DataMut,
          D: Dimension,
          B: ScalarOperand,
{
    type Output = ArrayBase<S, D>;
    fn $mth(mut self, x: B) -> ArrayBase<S, D> {
        self.unordered_foreach_mut(move |elt| {
            *elt = elt.clone() $operator x.clone();
        });
        self
    }
}

/// Perform elementwise
#[doc=$doc]
/// between the reference `self` and the scalar `x`,
/// and return the result as a new `Array`.
impl<'a, A, S, D, B> $trt<B> for &'a ArrayBase<S, D>
    where A: Clone + $trt<B, Output=A>,
          S: Data<Elem=A>,
          D: Dimension,
          B: ScalarOperand,
{
    type Output = Array<A, D>;
    fn $mth(self, x: B) -> Array<A, D> {
        self.to_owned().$mth(x)
    }
}
    );
);

// Pick the expression $a for commutative and $b for ordered binop
macro_rules! if_commutative {
    (Commute { $a:expr } or { $b:expr }) => {
        $a
    };
    (Ordered { $a:expr } or { $b:expr }) => {
        $b
    };
}

macro_rules! impl_scalar_lhs_op {
    // $commutative flag. Reuse the self + scalar impl if we can.
    // We can do this safely since these are the primitive numeric types
    ($scalar:ty, $commutative:ident, $operator:tt, $trt:ident, $mth:ident, $doc:expr) => (
// these have no doc -- they are not visible in rustdoc
// Perform elementwise
// between the scalar `self` and array `rhs`,
// and return the result (based on `self`).
impl<S, D> $trt<ArrayBase<S, D>> for $scalar
    where S: DataOwned<Elem=$scalar> + DataMut,
          D: Dimension,
{
    type Output = ArrayBase<S, D>;
    fn $mth(self, rhs: ArrayBase<S, D>) -> ArrayBase<S, D> {
        if_commutative!($commutative {
            rhs.$mth(self)
        } or {{
            let mut rhs = rhs;
            rhs.unordered_foreach_mut(move |elt| {
                *elt = self $operator *elt;
            });
            rhs
        }})
    }
}

// Perform elementwise
// between the scalar `self` and array `rhs`,
// and return the result as a new `Array`.
impl<'a, S, D> $trt<&'a ArrayBase<S, D>> for $scalar
    where S: Data<Elem=$scalar>,
          D: Dimension,
{
    type Output = Array<$scalar, D>;
    fn $mth(self, rhs: &ArrayBase<S, D>) -> Array<$scalar, D> {
        if_commutative!($commutative {
            rhs.$mth(self)
        } or {
            self.$mth(rhs.to_owned())
        })
    }
}
    );
}

mod arithmetic_ops {
    use super::*;
    use crate::imp_prelude::*;

    use num_complex::Complex;
    use std::ops::*;

    impl_binary_op!(Add, +, add, +=, "addition");
    impl_binary_op!(Sub, -, sub, -=, "subtraction");
    impl_binary_op!(Mul, *, mul, *=, "multiplication");
    impl_binary_op!(Div, /, div, /=, "division");
    impl_binary_op!(Rem, %, rem, %=, "remainder");
    impl_binary_op!(BitAnd, &, bitand, &=, "bit and");
    impl_binary_op!(BitOr, |, bitor, |=, "bit or");
    impl_binary_op!(BitXor, ^, bitxor, ^=, "bit xor");
    impl_binary_op!(Shl, <<, shl, <<=, "left shift");
    impl_binary_op!(Shr, >>, shr, >>=, "right shift");

    macro_rules! all_scalar_ops {
        ($int_scalar:ty) => (
            impl_scalar_lhs_op!($int_scalar, Commute, +, Add, add, "addition");
            impl_scalar_lhs_op!($int_scalar, Ordered, -, Sub, sub, "subtraction");
            impl_scalar_lhs_op!($int_scalar, Commute, *, Mul, mul, "multiplication");
            impl_scalar_lhs_op!($int_scalar, Ordered, /, Div, div, "division");
            impl_scalar_lhs_op!($int_scalar, Ordered, %, Rem, rem, "remainder");
            impl_scalar_lhs_op!($int_scalar, Commute, &, BitAnd, bitand, "bit and");
            impl_scalar_lhs_op!($int_scalar, Commute, |, BitOr, bitor, "bit or");
            impl_scalar_lhs_op!($int_scalar, Commute, ^, BitXor, bitxor, "bit xor");
            impl_scalar_lhs_op!($int_scalar, Ordered, <<, Shl, shl, "left shift");
            impl_scalar_lhs_op!($int_scalar, Ordered, >>, Shr, shr, "right shift");
        );
    }
    all_scalar_ops!(i8);
    all_scalar_ops!(u8);
    all_scalar_ops!(i16);
    all_scalar_ops!(u16);
    all_scalar_ops!(i32);
    all_scalar_ops!(u32);
    all_scalar_ops!(i64);
    all_scalar_ops!(u64);
    all_scalar_ops!(i128);
    all_scalar_ops!(u128);

    impl_scalar_lhs_op!(bool, Commute, &, BitAnd, bitand, "bit and");
    impl_scalar_lhs_op!(bool, Commute, |, BitOr, bitor, "bit or");
    impl_scalar_lhs_op!(bool, Commute, ^, BitXor, bitxor, "bit xor");

    impl_scalar_lhs_op!(f32, Commute, +, Add, add, "addition");
    impl_scalar_lhs_op!(f32, Ordered, -, Sub, sub, "subtraction");
    impl_scalar_lhs_op!(f32, Commute, *, Mul, mul, "multiplication");
    impl_scalar_lhs_op!(f32, Ordered, /, Div, div, "division");
    impl_scalar_lhs_op!(f32, Ordered, %, Rem, rem, "remainder");

    impl_scalar_lhs_op!(f64, Commute, +, Add, add, "addition");
    impl_scalar_lhs_op!(f64, Ordered, -, Sub, sub, "subtraction");
    impl_scalar_lhs_op!(f64, Commute, *, Mul, mul, "multiplication");
    impl_scalar_lhs_op!(f64, Ordered, /, Div, div, "division");
    impl_scalar_lhs_op!(f64, Ordered, %, Rem, rem, "remainder");

    impl_scalar_lhs_op!(Complex<f32>, Commute, +, Add, add, "addition");
    impl_scalar_lhs_op!(Complex<f32>, Ordered, -, Sub, sub, "subtraction");
    impl_scalar_lhs_op!(Complex<f32>, Commute, *, Mul, mul, "multiplication");
    impl_scalar_lhs_op!(Complex<f32>, Ordered, /, Div, div, "division");

    impl_scalar_lhs_op!(Complex<f64>, Commute, +, Add, add, "addition");
    impl_scalar_lhs_op!(Complex<f64>, Ordered, -, Sub, sub, "subtraction");
    impl_scalar_lhs_op!(Complex<f64>, Commute, *, Mul, mul, "multiplication");
    impl_scalar_lhs_op!(Complex<f64>, Ordered, /, Div, div, "division");

    impl<A, S, D> Neg for ArrayBase<S, D>
    where
        A: Clone + Neg<Output = A>,
        S: DataOwned<Elem = A> + DataMut,
        D: Dimension,
    {
        type Output = Self;
        /// Perform an elementwise negation of `self` and return the result.
        fn neg(mut self) -> Self {
            self.unordered_foreach_mut(|elt| {
                *elt = -elt.clone();
            });
            self
        }
    }

    impl<'a, A, S, D> Neg for &'a ArrayBase<S, D>
    where
        &'a A: 'a + Neg<Output = A>,
        S: Data<Elem = A>,
        D: Dimension,
    {
        type Output = Array<A, D>;
        /// Perform an elementwise negation of reference `self` and return the
        /// result as a new `Array`.
        fn neg(self) -> Array<A, D> {
            self.map(Neg::neg)
        }
    }

    impl<A, S, D> Not for ArrayBase<S, D>
    where
        A: Clone + Not<Output = A>,
        S: DataOwned<Elem = A> + DataMut,
        D: Dimension,
    {
        type Output = Self;
        /// Perform an elementwise unary not of `self` and return the result.
        fn not(mut self) -> Self {
            self.unordered_foreach_mut(|elt| {
                *elt = !elt.clone();
            });
            self
        }
    }

    impl<'a, A, S, D> Not for &'a ArrayBase<S, D>
    where
        &'a A: 'a + Not<Output = A>,
        S: Data<Elem = A>,
        D: Dimension,
    {
        type Output = Array<A, D>;
        /// Perform an elementwise unary not of reference `self` and return the
        /// result as a new `Array`.
        fn not(self) -> Array<A, D> {
            self.map(Not::not)
        }
    }
}

mod assign_ops {
    use super::*;
    use crate::imp_prelude::*;

    macro_rules! impl_assign_op {
        ($trt:ident, $method:ident, $doc:expr) => {
            use std::ops::$trt;

            #[doc=$doc]
            /// If their shapes disagree, `rhs` is broadcast to the shape of `self`.
            ///
            /// **Panics** if broadcasting isn’t possible.
            impl<'a, A, S, S2, D, E> $trt<&'a ArrayBase<S2, E>> for ArrayBase<S, D>
            where
                A: Clone + $trt<A>,
                S: DataMut<Elem = A>,
                S2: Data<Elem = A>,
                D: Dimension,
                E: Dimension,
            {
                fn $method(&mut self, rhs: &ArrayBase<S2, E>) {
                    self.zip_mut_with(rhs, |x, y| {
                        x.$method(y.clone());
                    });
                }
            }

            #[doc=$doc]
            impl<A, S, D> $trt<A> for ArrayBase<S, D>
            where
                A: ScalarOperand + $trt<A>,
                S: DataMut<Elem = A>,
                D: Dimension,
            {
                fn $method(&mut self, rhs: A) {
                    self.unordered_foreach_mut(move |elt| {
                        elt.$method(rhs.clone());
                    });
                }
            }
        };
    }

    impl_assign_op!(
        AddAssign,
        add_assign,
        "Perform `self += rhs` as elementwise addition (in place).\n"
    );
    impl_assign_op!(
        SubAssign,
        sub_assign,
        "Perform `self -= rhs` as elementwise subtraction (in place).\n"
    );
    impl_assign_op!(
        MulAssign,
        mul_assign,
        "Perform `self *= rhs` as elementwise multiplication (in place).\n"
    );
    impl_assign_op!(
        DivAssign,
        div_assign,
        "Perform `self /= rhs` as elementwise division (in place).\n"
    );
    impl_assign_op!(
        RemAssign,
        rem_assign,
        "Perform `self %= rhs` as elementwise remainder (in place).\n"
    );
    impl_assign_op!(
        BitAndAssign,
        bitand_assign,
        "Perform `self &= rhs` as elementwise bit and (in place).\n"
    );
    impl_assign_op!(
        BitOrAssign,
        bitor_assign,
        "Perform `self |= rhs` as elementwise bit or (in place).\n"
    );
    impl_assign_op!(
        BitXorAssign,
        bitxor_assign,
        "Perform `self ^= rhs` as elementwise bit xor (in place).\n"
    );
    impl_assign_op!(
        ShlAssign,
        shl_assign,
        "Perform `self <<= rhs` as elementwise left shift (in place).\n"
    );
    impl_assign_op!(
        ShrAssign,
        shr_assign,
        "Perform `self >>= rhs` as elementwise right shift (in place).\n"
    );
}
#[cfg(test)]
mod tests_rug_1207 {
    use super::*;
    use crate::{Array, ArrayBase, Data, DataOwned, Dim, OwnedRepr};
    use crate::prelude::*;
    use std::ops::BitOr;

    #[test]
    fn test_rug() {
        let mut p0: i8 = 3;
        let mut p1: ArrayBase<OwnedRepr<i8>, Dim<[usize; 1]>> = array![1, 2, 3];

        <i8>::bitor(p0, &p1);

    }
}#[cfg(test)]
mod tests_rug_1214 {
    use super::*;
    use crate::{ArrayBase, Data, Dimension, array};
    use std::ops::Add;

    #[test]
    fn test_add() {
        let mut p0: u8 = 5;
        let mut p1 = array![1, 2, 3];

        <u8>::add(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_1222 {
    use super::*;
    use crate::prelude::*;
    use crate::ArrayBase;
    use crate::OwnedRepr;
    use crate::Array;
    use std::ops::Rem;

    #[test]
    fn test_rem() {
        let mut p0: u8 = 10;
        let mut p1: ArrayBase<OwnedRepr<u8>, Dim<[usize; 1]>> = ArrayBase::<OwnedRepr<u8>, Dim<[usize; 1]>>::from_shape_vec((3,), vec![5, 2, 3]).unwrap();

        p0.rem(p1);
    }
}#[cfg(test)]
mod tests_rug_1227 {
    use super::*;
    use crate::Array;
    use std::ops::BitOr;
    
    #[test]
    fn test_bitor() {
        // Sample data for the 1st argument
        let mut p0: u8 = 5;

        // Constructing the 2nd argument as an ArrayBase<S, D>
        let shape = (2, 2);
        let data: Vec<u8> = vec![1, 2, 3, 4];
        let array = Array::from_shape_vec(shape, data).unwrap();
        let array_base = array.view();
        let mut p1 = array_base;
        
        p0.bitor(&p1);
    }
}#[cfg(test)]
mod tests_rug_1228 {
    use super::*;
    use crate::prelude::*;
    use crate::ArrayBase;
    use std::ops::BitXor;
    
    #[test]
    fn test_bitxor() {
        let mut p0: u8 = 5;
        let p1 = array![[1, 2], [3, 4]].to_owned();
        
        u8::bitxor(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_1235 {
    use super::*;
    use crate::{Array, ArrayBase, Data, Dim, Ix, OwnedRepr};

    use std::ops::Add;

    #[test]
    fn test_add() {
        let mut p0: i16 = 5;
        let p1: ArrayBase<OwnedRepr<i16>, Dim<[Ix; 2]>> = Array::from_shape_vec((2, 2), vec![1, 2, 3, 4]).unwrap();

        <i16>::add(p0, &p1);
    }
}#[cfg(test)]
mod tests_rug_1240 {
    use super::*;
    
    use crate::{ArrayBase, Data, Dimension, OwnedRepr};
    use crate::prelude::*;
    
    use std::ops::Div;
    
    #[test]
    fn test_rug() {
        let mut p0: i16 = 10;
        let mut p1: ArrayBase<OwnedRepr<i16>, Dim<[usize; 1]>> = ArrayBase::<OwnedRepr<i16>, _>::zeros(5);

        i16::div(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_1277 {
    use super::*;
    use crate::{Array, ArrayBase, Data, Ix1, OwnedRepr};
    use std::ops::Sub;
    
    #[test]
    fn test_subtraction() {
        let p0: i32 = 10;
        let p1: ArrayBase<OwnedRepr<i32>, Ix1> = ArrayBase::<OwnedRepr<i32>, Ix1>::from(vec![1, 2, 3]);
        
        let result = <i32 as Sub<&ArrayBase<OwnedRepr<i32>, Ix1>>>::sub(p0, &p1);
        // Add assertions based on expected result
    }
}#[cfg(test)]
mod tests_rug_1287 {
    use super::*;
    use crate::{Array, ArrayBase, Dim, OwnedRepr};

    use std::ops::BitOr;

    #[test]
    fn test_rug() {
        let mut p0: i32 = 10;

        let data = vec![1, 2, 3, 4];
        let shape = Dim((2, 2));
        let p1: ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>> = ArrayBase::from_shape_vec(shape, data).unwrap();

        p0.bitor(&p1);
    }
}#[cfg(test)]
mod tests_rug_1322 {
    use super::*;
    use crate::ArrayBase;
    use crate::prelude::*;
    use std::ops::Rem;

    #[test]
    fn test_rem() {
        let mut p0: i64 = 10;
        let p1 = array![[1, 2, 3],
                        [4, 5, 6],
                        [7, 8, 9]].into_dyn();

        <i64>::rem(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_1327 {
    use super::*;
    use std::ops::BitOr;
    use crate::{Array, ArrayBase, DataOwned, DataMut, Ix1, OwnedRepr};
    use crate::prelude::*;

    #[test]
    fn test_rug() {
        let mut p0: i64 = 42;
        let p1: ArrayBase<OwnedRepr<i64>, Ix1> = array![1, 2, 3];

        p0.bitor(&p1);
    }
}#[cfg(test)]
mod tests_rug_1354 {
    use super::*;
    use crate::{Array, ArrayBase, DataOwned, Ix1, OwnedRepr};
    use crate::prelude::*;
    use std::ops::Add;

    #[test]
    fn test_rug() {
        let mut p0: i128 = 10;
        let mut p1: ArrayBase<OwnedRepr<i128>, Ix1> = Array::from(vec![1, 2, 3]);

        <i128>::add(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_1380 {
    use super::*;
    use crate::prelude::*;
    use crate::ArrayBase;

    use std::ops::Div;

    #[test]
    fn test_rug() {
        let mut p0: u128 = 10;
        let mut p1 = array![[1, 2], [3, 4]].into_dyn();

        <u128>::div(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_1409 {
    use super::*;
    use crate::{Array, ArrayBase, Axis, Data, Ix1, Ix2};
    use std::ops::Rem;

    #[test]
    fn test_rug() {
        let value: f32 = 10.0;
        let arr_data = vec![1.0, 2.0, 3.0];
        let array = Array::from(arr_data);

        let p0: f32 = value;
        let p1: ArrayBase<_, _> = array.view();

        p0.rem(&p1);
    }
}