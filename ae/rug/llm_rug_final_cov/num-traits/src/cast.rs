use core::mem::size_of;
use core::num::Wrapping;
use core::{f32, f64};
use core::{i128, i16, i32, i64, i8, isize};
use core::{u128, u16, u32, u64, u8, usize};
/// A generic trait for converting a value to a number.
///
/// A value can be represented by the target type when it lies within
/// the range of scalars supported by the target type.
/// For example, a negative integer cannot be represented by an unsigned
/// integer type, and an `i64` with a very high magnitude might not be
/// convertible to an `i32`.
/// On the other hand, conversions with possible precision loss or truncation
/// are admitted, like an `f32` with a decimal part to an integer type, or
/// even a large `f64` saturating to `f32` infinity.
pub trait ToPrimitive {
    /// Converts the value of `self` to an `isize`. If the value cannot be
    /// represented by an `isize`, then `None` is returned.
    #[inline]
    fn to_isize(&self) -> Option<isize> {
        self.to_i64().as_ref().and_then(ToPrimitive::to_isize)
    }
    /// Converts the value of `self` to an `i8`. If the value cannot be
    /// represented by an `i8`, then `None` is returned.
    #[inline]
    fn to_i8(&self) -> Option<i8> {
        self.to_i64().as_ref().and_then(ToPrimitive::to_i8)
    }
    /// Converts the value of `self` to an `i16`. If the value cannot be
    /// represented by an `i16`, then `None` is returned.
    #[inline]
    fn to_i16(&self) -> Option<i16> {
        self.to_i64().as_ref().and_then(ToPrimitive::to_i16)
    }
    /// Converts the value of `self` to an `i32`. If the value cannot be
    /// represented by an `i32`, then `None` is returned.
    #[inline]
    fn to_i32(&self) -> Option<i32> {
        self.to_i64().as_ref().and_then(ToPrimitive::to_i32)
    }
    /// Converts the value of `self` to an `i64`. If the value cannot be
    /// represented by an `i64`, then `None` is returned.
    fn to_i64(&self) -> Option<i64>;
    /// Converts the value of `self` to an `i128`. If the value cannot be
    /// represented by an `i128` (`i64` under the default implementation), then
    /// `None` is returned.
    ///
    /// The default implementation converts through `to_i64()`. Types implementing
    /// this trait should override this method if they can represent a greater range.
    #[inline]
    fn to_i128(&self) -> Option<i128> {
        self.to_i64().map(From::from)
    }
    /// Converts the value of `self` to a `usize`. If the value cannot be
    /// represented by a `usize`, then `None` is returned.
    #[inline]
    fn to_usize(&self) -> Option<usize> {
        self.to_u64().as_ref().and_then(ToPrimitive::to_usize)
    }
    /// Converts the value of `self` to a `u8`. If the value cannot be
    /// represented by a `u8`, then `None` is returned.
    #[inline]
    fn to_u8(&self) -> Option<u8> {
        self.to_u64().as_ref().and_then(ToPrimitive::to_u8)
    }
    /// Converts the value of `self` to a `u16`. If the value cannot be
    /// represented by a `u16`, then `None` is returned.
    #[inline]
    fn to_u16(&self) -> Option<u16> {
        self.to_u64().as_ref().and_then(ToPrimitive::to_u16)
    }
    /// Converts the value of `self` to a `u32`. If the value cannot be
    /// represented by a `u32`, then `None` is returned.
    #[inline]
    fn to_u32(&self) -> Option<u32> {
        self.to_u64().as_ref().and_then(ToPrimitive::to_u32)
    }
    /// Converts the value of `self` to a `u64`. If the value cannot be
    /// represented by a `u64`, then `None` is returned.
    fn to_u64(&self) -> Option<u64>;
    /// Converts the value of `self` to a `u128`. If the value cannot be
    /// represented by a `u128` (`u64` under the default implementation), then
    /// `None` is returned.
    ///
    /// The default implementation converts through `to_u64()`. Types implementing
    /// this trait should override this method if they can represent a greater range.
    #[inline]
    fn to_u128(&self) -> Option<u128> {
        self.to_u64().map(From::from)
    }
    /// Converts the value of `self` to an `f32`. Overflows may map to positive
    /// or negative inifinity, otherwise `None` is returned if the value cannot
    /// be represented by an `f32`.
    #[inline]
    fn to_f32(&self) -> Option<f32> {
        self.to_f64().as_ref().and_then(ToPrimitive::to_f32)
    }
    /// Converts the value of `self` to an `f64`. Overflows may map to positive
    /// or negative inifinity, otherwise `None` is returned if the value cannot
    /// be represented by an `f64`.
    ///
    /// The default implementation tries to convert through `to_i64()`, and
    /// failing that through `to_u64()`. Types implementing this trait should
    /// override this method if they can represent a greater range.
    #[inline]
    fn to_f64(&self) -> Option<f64> {
        match self.to_i64() {
            Some(i) => i.to_f64(),
            None => self.to_u64().as_ref().and_then(ToPrimitive::to_f64),
        }
    }
}
macro_rules! impl_to_primitive_int_to_int {
    ($SrcT:ident : $($(#[$cfg:meta])* fn $method:ident -> $DstT:ident;)*) => {
        $(#[inline] $(#[$cfg])* fn $method (& self) -> Option <$DstT > { let min = $DstT
        ::MIN as $SrcT; let max = $DstT ::MAX as $SrcT; if size_of::<$SrcT > () <=
        size_of::<$DstT > () || (min <= * self && * self <= max) { Some(* self as $DstT)
        } else { None } })*
    };
}
macro_rules! impl_to_primitive_int_to_uint {
    ($SrcT:ident : $($(#[$cfg:meta])* fn $method:ident -> $DstT:ident;)*) => {
        $(#[inline] $(#[$cfg])* fn $method (& self) -> Option <$DstT > { let max = $DstT
        ::MAX as $SrcT; if 0 <= * self && (size_of::<$SrcT > () <= size_of::<$DstT > ()
        || * self <= max) { Some(* self as $DstT) } else { None } })*
    };
}
macro_rules! impl_to_primitive_int {
    ($T:ident) => {
        impl ToPrimitive for $T { impl_to_primitive_int_to_int! { $T : fn to_isize ->
        isize; fn to_i8 -> i8; fn to_i16 -> i16; fn to_i32 -> i32; fn to_i64 -> i64; fn
        to_i128 -> i128; } impl_to_primitive_int_to_uint! { $T : fn to_usize -> usize; fn
        to_u8 -> u8; fn to_u16 -> u16; fn to_u32 -> u32; fn to_u64 -> u64; fn to_u128 ->
        u128; } #[inline] fn to_f32(& self) -> Option < f32 > { Some(* self as f32) }
        #[inline] fn to_f64(& self) -> Option < f64 > { Some(* self as f64) } }
    };
}
impl_to_primitive_int!(isize);
impl_to_primitive_int!(i8);
impl_to_primitive_int!(i16);
impl_to_primitive_int!(i32);
impl_to_primitive_int!(i64);
impl_to_primitive_int!(i128);
macro_rules! impl_to_primitive_uint_to_int {
    ($SrcT:ident : $($(#[$cfg:meta])* fn $method:ident -> $DstT:ident;)*) => {
        $(#[inline] $(#[$cfg])* fn $method (& self) -> Option <$DstT > { let max = $DstT
        ::MAX as $SrcT; if size_of::<$SrcT > () < size_of::<$DstT > () || * self <= max {
        Some(* self as $DstT) } else { None } })*
    };
}
macro_rules! impl_to_primitive_uint_to_uint {
    ($SrcT:ident : $($(#[$cfg:meta])* fn $method:ident -> $DstT:ident;)*) => {
        $(#[inline] $(#[$cfg])* fn $method (& self) -> Option <$DstT > { let max = $DstT
        ::MAX as $SrcT; if size_of::<$SrcT > () <= size_of::<$DstT > () || * self <= max
        { Some(* self as $DstT) } else { None } })*
    };
}
macro_rules! impl_to_primitive_uint {
    ($T:ident) => {
        impl ToPrimitive for $T { impl_to_primitive_uint_to_int! { $T : fn to_isize ->
        isize; fn to_i8 -> i8; fn to_i16 -> i16; fn to_i32 -> i32; fn to_i64 -> i64; fn
        to_i128 -> i128; } impl_to_primitive_uint_to_uint! { $T : fn to_usize -> usize;
        fn to_u8 -> u8; fn to_u16 -> u16; fn to_u32 -> u32; fn to_u64 -> u64; fn to_u128
        -> u128; } #[inline] fn to_f32(& self) -> Option < f32 > { Some(* self as f32) }
        #[inline] fn to_f64(& self) -> Option < f64 > { Some(* self as f64) } }
    };
}
impl_to_primitive_uint!(usize);
impl_to_primitive_uint!(u8);
impl_to_primitive_uint!(u16);
impl_to_primitive_uint!(u32);
impl_to_primitive_uint!(u64);
impl_to_primitive_uint!(u128);
macro_rules! impl_to_primitive_float_to_float {
    ($SrcT:ident : $(fn $method:ident -> $DstT:ident;)*) => {
        $(#[inline] fn $method (& self) -> Option <$DstT > { Some(* self as $DstT) })*
    };
}
#[cfg(has_to_int_unchecked)]
macro_rules! float_to_int_unchecked {
    ($float:expr => $int:ty) => {
        unsafe { $float .to_int_unchecked::<$int > () }
    };
}
#[cfg(not(has_to_int_unchecked))]
macro_rules! float_to_int_unchecked {
    ($float:expr => $int:ty) => {
        $float as $int
    };
}
macro_rules! impl_to_primitive_float_to_signed_int {
    ($f:ident : $($(#[$cfg:meta])* fn $method:ident -> $i:ident;)*) => {
        $(#[inline] $(#[$cfg])* fn $method (& self) -> Option <$i > { if size_of::<$f >
        () > size_of::<$i > () { const MIN_M1 : $f = $i ::MIN as $f - 1.0; const MAX_P1 :
        $f = $i ::MAX as $f + 1.0; if * self > MIN_M1 && * self < MAX_P1 { return
        Some(float_to_int_unchecked!(* self => $i)); } } else { const MIN : $f = $i ::MIN
        as $f; const MAX_P1 : $f = $i ::MAX as $f; if * self >= MIN && * self < MAX_P1 {
        return Some(float_to_int_unchecked!(* self => $i)); } } None })*
    };
}
macro_rules! impl_to_primitive_float_to_unsigned_int {
    ($f:ident : $($(#[$cfg:meta])* fn $method:ident -> $u:ident;)*) => {
        $(#[inline] $(#[$cfg])* fn $method (& self) -> Option <$u > { if size_of::<$f >
        () > size_of::<$u > () { const MAX_P1 : $f = $u ::MAX as $f + 1.0; if * self > -
        1.0 && * self < MAX_P1 { return Some(float_to_int_unchecked!(* self => $u)); } }
        else { const MAX_P1 : $f = $u ::MAX as $f; if * self > - 1.0 && * self < MAX_P1 {
        return Some(float_to_int_unchecked!(* self => $u)); } } None })*
    };
}
macro_rules! impl_to_primitive_float {
    ($T:ident) => {
        impl ToPrimitive for $T { impl_to_primitive_float_to_signed_int! { $T : fn
        to_isize -> isize; fn to_i8 -> i8; fn to_i16 -> i16; fn to_i32 -> i32; fn to_i64
        -> i64; fn to_i128 -> i128; } impl_to_primitive_float_to_unsigned_int! { $T : fn
        to_usize -> usize; fn to_u8 -> u8; fn to_u16 -> u16; fn to_u32 -> u32; fn to_u64
        -> u64; fn to_u128 -> u128; } impl_to_primitive_float_to_float! { $T : fn to_f32
        -> f32; fn to_f64 -> f64; } }
    };
}
impl_to_primitive_float!(f32);
impl_to_primitive_float!(f64);
/// A generic trait for converting a number to a value.
///
/// A value can be represented by the target type when it lies within
/// the range of scalars supported by the target type.
/// For example, a negative integer cannot be represented by an unsigned
/// integer type, and an `i64` with a very high magnitude might not be
/// convertible to an `i32`.
/// On the other hand, conversions with possible precision loss or truncation
/// are admitted, like an `f32` with a decimal part to an integer type, or
/// even a large `f64` saturating to `f32` infinity.
pub trait FromPrimitive: Sized {
    /// Converts an `isize` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_isize(n: isize) -> Option<Self> {
        n.to_i64().and_then(FromPrimitive::from_i64)
    }
    /// Converts an `i8` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_i8(n: i8) -> Option<Self> {
        FromPrimitive::from_i64(From::from(n))
    }
    /// Converts an `i16` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_i16(n: i16) -> Option<Self> {
        FromPrimitive::from_i64(From::from(n))
    }
    /// Converts an `i32` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_i32(n: i32) -> Option<Self> {
        FromPrimitive::from_i64(From::from(n))
    }
    /// Converts an `i64` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    fn from_i64(n: i64) -> Option<Self>;
    /// Converts an `i128` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    ///
    /// The default implementation converts through `from_i64()`. Types implementing
    /// this trait should override this method if they can represent a greater range.
    #[inline]
    fn from_i128(n: i128) -> Option<Self> {
        n.to_i64().and_then(FromPrimitive::from_i64)
    }
    /// Converts a `usize` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_usize(n: usize) -> Option<Self> {
        n.to_u64().and_then(FromPrimitive::from_u64)
    }
    /// Converts an `u8` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_u8(n: u8) -> Option<Self> {
        FromPrimitive::from_u64(From::from(n))
    }
    /// Converts an `u16` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_u16(n: u16) -> Option<Self> {
        FromPrimitive::from_u64(From::from(n))
    }
    /// Converts an `u32` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_u32(n: u32) -> Option<Self> {
        FromPrimitive::from_u64(From::from(n))
    }
    /// Converts an `u64` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    fn from_u64(n: u64) -> Option<Self>;
    /// Converts an `u128` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    ///
    /// The default implementation converts through `from_u64()`. Types implementing
    /// this trait should override this method if they can represent a greater range.
    #[inline]
    fn from_u128(n: u128) -> Option<Self> {
        n.to_u64().and_then(FromPrimitive::from_u64)
    }
    /// Converts a `f32` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    #[inline]
    fn from_f32(n: f32) -> Option<Self> {
        FromPrimitive::from_f64(From::from(n))
    }
    /// Converts a `f64` to return an optional value of this type. If the
    /// value cannot be represented by this type, then `None` is returned.
    ///
    /// The default implementation tries to convert through `from_i64()`, and
    /// failing that through `from_u64()`. Types implementing this trait should
    /// override this method if they can represent a greater range.
    #[inline]
    fn from_f64(n: f64) -> Option<Self> {
        match n.to_i64() {
            Some(i) => FromPrimitive::from_i64(i),
            None => n.to_u64().and_then(FromPrimitive::from_u64),
        }
    }
}
macro_rules! impl_from_primitive {
    ($T:ty, $to_ty:ident) => {
        #[allow(deprecated)] impl FromPrimitive for $T { #[inline] fn from_isize(n :
        isize) -> Option <$T > { n.$to_ty () } #[inline] fn from_i8(n : i8) -> Option <$T
        > { n.$to_ty () } #[inline] fn from_i16(n : i16) -> Option <$T > { n.$to_ty () }
        #[inline] fn from_i32(n : i32) -> Option <$T > { n.$to_ty () } #[inline] fn
        from_i64(n : i64) -> Option <$T > { n.$to_ty () } #[inline] fn from_i128(n :
        i128) -> Option <$T > { n.$to_ty () } #[inline] fn from_usize(n : usize) ->
        Option <$T > { n.$to_ty () } #[inline] fn from_u8(n : u8) -> Option <$T > { n
        .$to_ty () } #[inline] fn from_u16(n : u16) -> Option <$T > { n.$to_ty () }
        #[inline] fn from_u32(n : u32) -> Option <$T > { n.$to_ty () } #[inline] fn
        from_u64(n : u64) -> Option <$T > { n.$to_ty () } #[inline] fn from_u128(n :
        u128) -> Option <$T > { n.$to_ty () } #[inline] fn from_f32(n : f32) -> Option
        <$T > { n.$to_ty () } #[inline] fn from_f64(n : f64) -> Option <$T > { n.$to_ty
        () } }
    };
}
impl_from_primitive!(isize, to_isize);
impl_from_primitive!(i8, to_i8);
impl_from_primitive!(i16, to_i16);
impl_from_primitive!(i32, to_i32);
impl_from_primitive!(i64, to_i64);
impl_from_primitive!(i128, to_i128);
impl_from_primitive!(usize, to_usize);
impl_from_primitive!(u8, to_u8);
impl_from_primitive!(u16, to_u16);
impl_from_primitive!(u32, to_u32);
impl_from_primitive!(u64, to_u64);
impl_from_primitive!(u128, to_u128);
impl_from_primitive!(f32, to_f32);
impl_from_primitive!(f64, to_f64);
macro_rules! impl_to_primitive_wrapping {
    ($($(#[$cfg:meta])* fn $method:ident -> $i:ident;)*) => {
        $(#[inline] $(#[$cfg])* fn $method (& self) -> Option <$i > { (self.0).$method ()
        })*
    };
}
impl<T: ToPrimitive> ToPrimitive for Wrapping<T> {
    impl_to_primitive_wrapping! {
        fn to_isize -> isize; fn to_i8 -> i8; fn to_i16 -> i16; fn to_i32 -> i32; fn
        to_i64 -> i64; fn to_i128 -> i128; fn to_usize -> usize; fn to_u8 -> u8; fn
        to_u16 -> u16; fn to_u32 -> u32; fn to_u64 -> u64; fn to_u128 -> u128; fn to_f32
        -> f32; fn to_f64 -> f64;
    }
}
macro_rules! impl_from_primitive_wrapping {
    ($($(#[$cfg:meta])* fn $method:ident ($i:ident);)*) => {
        $(#[inline] $(#[$cfg])* fn $method (n : $i) -> Option < Self > { T::$method (n)
        .map(Wrapping) })*
    };
}
impl<T: FromPrimitive> FromPrimitive for Wrapping<T> {
    impl_from_primitive_wrapping! {
        fn from_isize(isize); fn from_i8(i8); fn from_i16(i16); fn from_i32(i32); fn
        from_i64(i64); fn from_i128(i128); fn from_usize(usize); fn from_u8(u8); fn
        from_u16(u16); fn from_u32(u32); fn from_u64(u64); fn from_u128(u128); fn
        from_f32(f32); fn from_f64(f64);
    }
}
/// Cast from one machine scalar to another.
///
/// # Examples
///
/// ```
/// # use num_traits as num;
/// let twenty: f32 = num::cast(0x14).unwrap();
/// assert_eq!(twenty, 20f32);
/// ```
///
#[inline]
pub fn cast<T: NumCast, U: NumCast>(n: T) -> Option<U> {
    NumCast::from(n)
}
/// An interface for casting between machine scalars.
pub trait NumCast: Sized + ToPrimitive {
    /// Creates a number from another value that can be converted into
    /// a primitive via the `ToPrimitive` trait. If the source value cannot be
    /// represented by the target type, then `None` is returned.
    ///
    /// A value can be represented by the target type when it lies within
    /// the range of scalars supported by the target type.
    /// For example, a negative integer cannot be represented by an unsigned
    /// integer type, and an `i64` with a very high magnitude might not be
    /// convertible to an `i32`.
    /// On the other hand, conversions with possible precision loss or truncation
    /// are admitted, like an `f32` with a decimal part to an integer type, or
    /// even a large `f64` saturating to `f32` infinity.
    fn from<T: ToPrimitive>(n: T) -> Option<Self>;
}
macro_rules! impl_num_cast {
    ($T:ty, $conv:ident) => {
        impl NumCast for $T { #[inline] #[allow(deprecated)] fn from < N : ToPrimitive >
        (n : N) -> Option <$T > { n.$conv () } }
    };
}
impl_num_cast!(u8, to_u8);
impl_num_cast!(u16, to_u16);
impl_num_cast!(u32, to_u32);
impl_num_cast!(u64, to_u64);
impl_num_cast!(u128, to_u128);
impl_num_cast!(usize, to_usize);
impl_num_cast!(i8, to_i8);
impl_num_cast!(i16, to_i16);
impl_num_cast!(i32, to_i32);
impl_num_cast!(i64, to_i64);
impl_num_cast!(i128, to_i128);
impl_num_cast!(isize, to_isize);
impl_num_cast!(f32, to_f32);
impl_num_cast!(f64, to_f64);
impl<T: NumCast> NumCast for Wrapping<T> {
    fn from<U: ToPrimitive>(n: U) -> Option<Self> {
        T::from(n).map(Wrapping)
    }
}
/// A generic interface for casting between machine scalars with the
/// `as` operator, which admits narrowing and precision loss.
/// Implementers of this trait `AsPrimitive` should behave like a primitive
/// numeric type (e.g. a newtype around another primitive), and the
/// intended conversion must never fail.
///
/// # Examples
///
/// ```
/// # use num_traits::AsPrimitive;
/// let three: i32 = (3.14159265f32).as_();
/// assert_eq!(three, 3);
/// ```
///
/// # Safety
///
/// **In Rust versions before 1.45.0**, some uses of the `as` operator were not entirely safe.
/// In particular, it was undefined behavior if
/// a truncated floating point value could not fit in the target integer
/// type ([#10184](https://github.com/rust-lang/rust/issues/10184)).
///
/// ```ignore
/// # use num_traits::AsPrimitive;
/// let x: u8 = (1.04E+17).as_(); // UB
/// ```
///
pub trait AsPrimitive<T>: 'static + Copy
where
    T: 'static + Copy,
{
    /// Convert a value to another, using the `as` operator.
    fn as_(self) -> T;
}
macro_rules! impl_as_primitive {
    (@ $T:ty => $(#[$cfg:meta])* impl $U:ty) => {
        $(#[$cfg])* impl AsPrimitive <$U > for $T { #[inline] fn as_(self) -> $U { self
        as $U } }
    };
    (@ $T:ty => { $($U:ty),* }) => {
        $(impl_as_primitive!(@ $T => impl $U);)*
    };
    ($T:ty => { $($U:ty),* }) => {
        impl_as_primitive!(@ $T => { $($U),* }); impl_as_primitive!(@ $T => { u8, u16,
        u32, u64, u128, usize }); impl_as_primitive!(@ $T => { i8, i16, i32, i64, i128,
        isize });
    };
}
impl_as_primitive!(u8 => { char, f32, f64 });
impl_as_primitive!(i8 => { f32, f64 });
impl_as_primitive!(u16 => { f32, f64 });
impl_as_primitive!(i16 => { f32, f64 });
impl_as_primitive!(u32 => { f32, f64 });
impl_as_primitive!(i32 => { f32, f64 });
impl_as_primitive!(u64 => { f32, f64 });
impl_as_primitive!(i64 => { f32, f64 });
impl_as_primitive!(u128 => { f32, f64 });
impl_as_primitive!(i128 => { f32, f64 });
impl_as_primitive!(usize => { f32, f64 });
impl_as_primitive!(isize => { f32, f64 });
impl_as_primitive!(f32 => { f32, f64 });
impl_as_primitive!(f64 => { f32, f64 });
impl_as_primitive!(char => { char });
impl_as_primitive!(bool => {});
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        cast::<Wrapping<i32>, i32>(p0);
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_isize(&p0);
        let _rug_ed_tests_rug_2_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_3_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i64> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_i8(&p0);
        let _rug_ed_tests_rug_3_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_4_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_i16(&p0);
        let _rug_ed_tests_rug_4_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_5_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_i32(&p0);
        let _rug_ed_tests_rug_5_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_6_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_i128(&p0);
        let _rug_ed_tests_rug_6_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_7_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_usize(&p0);
        let _rug_ed_tests_rug_7_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_8_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_u8(&p0);
        let _rug_ed_tests_rug_8_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_num_traits() {
        let _rug_st_tests_rug_9_rrrruuuugggg_test_num_traits = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_u16(&p0);
        let _rug_ed_tests_rug_9_rrrruuuugggg_test_num_traits = 0;
    }
}
#[cfg(test)]
mod tests_rug_10 {
    use crate::cast::ToPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_10_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        ToPrimitive::to_u32(&p0);
        let _rug_ed_tests_rug_10_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_11_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_u128(&p0);
        let _rug_ed_tests_rug_11_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_12_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_f32(&p0);
        let _rug_ed_tests_rug_12_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_13_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        crate::cast::ToPrimitive::to_f64(&p0);
        let _rug_ed_tests_rug_13_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_27 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_to_i8() {
        let _rug_st_tests_rug_27_rrrruuuugggg_test_to_i8 = 0;
        let rug_fuzz_0 = 10;
        let p0: isize = rug_fuzz_0;
        <isize as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_27_rrrruuuugggg_test_to_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_28 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i16() {
        let _rug_st_tests_rug_28_rrrruuuugggg_test_to_i16 = 0;
        let rug_fuzz_0 = 42;
        let p0: isize = rug_fuzz_0;
        <isize as ToPrimitive>::to_i16(&p0);
        let _rug_ed_tests_rug_28_rrrruuuugggg_test_to_i16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_29_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        <isize as ToPrimitive>::to_i32(&p0);
        let _rug_ed_tests_rug_29_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_31 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_31_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0: isize = rug_fuzz_0;
        p0.to_i128();
        let _rug_ed_tests_rug_31_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_to_usize() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_to_usize = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        <isize as ToPrimitive>::to_usize(&p0);
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_to_usize = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_33_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: isize = rug_fuzz_0;
        <isize as ToPrimitive>::to_u8(&p0);
        let _rug_ed_tests_rug_33_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_34 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_34_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = rug_fuzz_0;
        <isize as ToPrimitive>::to_u16(&p0);
        let _rug_ed_tests_rug_34_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_35_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        p0.to_u32();
        let _rug_ed_tests_rug_35_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_36_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        p0.to_u64();
        let _rug_ed_tests_rug_36_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_39_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        p0.to_f64();
        let _rug_ed_tests_rug_39_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_40_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        <i8 as ToPrimitive>::to_isize(&p0);
        let _rug_ed_tests_rug_40_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_to_i8() {
        let _rug_st_tests_rug_41_rrrruuuugggg_test_to_i8 = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i8 as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_41_rrrruuuugggg_test_to_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i16() {
        let _rug_st_tests_rug_42_rrrruuuugggg_test_to_i16 = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i8 as crate::ToPrimitive>::to_i16(&p0);
        let _rug_ed_tests_rug_42_rrrruuuugggg_test_to_i16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_43 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_43_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        p0.to_i32();
        let _rug_ed_tests_rug_43_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_44 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_to_i64() {
        let _rug_st_tests_rug_44_rrrruuuugggg_test_to_i64 = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i8 as ToPrimitive>::to_i64(&p0);
        let _rug_ed_tests_rug_44_rrrruuuugggg_test_to_i64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = -rug_fuzz_0;
        <i8 as ToPrimitive>::to_i128(&p0);
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_46 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_46_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        p0.to_usize();
        let _rug_ed_tests_rug_46_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_47 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_47_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i8 as ToPrimitive>::to_u8(&p0);
        let _rug_ed_tests_rug_47_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_48_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0: i8 = rug_fuzz_0;
        <i8>::to_u16(&p0);
        let _rug_ed_tests_rug_48_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_49_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i8 = rug_fuzz_0;
        p0.to_u32();
        let _rug_ed_tests_rug_49_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_50_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i8 = rug_fuzz_0;
        <i8 as ToPrimitive>::to_u64(&p0);
        let _rug_ed_tests_rug_50_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u128() {
        let _rug_st_tests_rug_51_rrrruuuugggg_test_to_u128 = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        p0.to_u128();
        let _rug_ed_tests_rug_51_rrrruuuugggg_test_to_u128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 127;
        let mut p0: i8 = -rug_fuzz_0;
        p0.to_f32();
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_54_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        p0.to_isize();
        let _rug_ed_tests_rug_54_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_55_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16 as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_55_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_to_i16() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_to_i16 = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        p0.to_i16();
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_to_i16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i32() {
        let _rug_st_tests_rug_57_rrrruuuugggg_test_to_i32 = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        p0.to_i32();
        let _rug_ed_tests_rug_57_rrrruuuugggg_test_to_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i64() {
        let _rug_st_tests_rug_58_rrrruuuugggg_test_to_i64 = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <i16>::to_i64(&p0);
        let _rug_ed_tests_rug_58_rrrruuuugggg_test_to_i64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_59_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        p0.to_i128();
        let _rug_ed_tests_rug_59_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_61_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        p0.to_u8();
        let _rug_ed_tests_rug_61_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_62 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_62_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16 as ToPrimitive>::to_u16(&p0);
        let _rug_ed_tests_rug_62_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_64_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        p0.to_u64();
        let _rug_ed_tests_rug_64_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_65 {
    use super::*;
    use crate::ToPrimitive;
    use std::mem::size_of;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_65_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        p0.to_u128();
        let _rug_ed_tests_rug_65_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_66 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_66_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        p0.to_f32();
        let _rug_ed_tests_rug_66_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_f64() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_to_f64 = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16 as ToPrimitive>::to_f64(&p0);
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_to_f64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_68_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i32 = rug_fuzz_0;
        p0.to_isize();
        let _rug_ed_tests_rug_68_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_69 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_69_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i32 = rug_fuzz_0;
        <i32 as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_69_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_70_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        p0.to_i16();
        let _rug_ed_tests_rug_70_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i32() {
        let _rug_st_tests_rug_71_rrrruuuugggg_test_to_i32 = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        p0.to_i32();
        let _rug_ed_tests_rug_71_rrrruuuugggg_test_to_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_73_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i32 = rug_fuzz_0;
        <i32 as ToPrimitive>::to_i128(&p0);
        let _rug_ed_tests_rug_73_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_usize() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_to_usize = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <i32 as ToPrimitive>::to_usize(&p0);
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_to_usize = 0;
    }
}
#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_75_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i32 = rug_fuzz_0;
        <i32 as ToPrimitive>::to_u8(&p0);
        let _rug_ed_tests_rug_75_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_78_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = rug_fuzz_0;
        <i32 as ToPrimitive>::to_u64(&p0);
        let _rug_ed_tests_rug_78_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_79 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_79_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i32 = rug_fuzz_0;
        p0.to_u128();
        let _rug_ed_tests_rug_79_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_f64() {
        let _rug_st_tests_rug_81_rrrruuuugggg_test_to_f64 = 0;
        let rug_fuzz_0 = 10;
        let p0: i32 = rug_fuzz_0;
        p0.to_f64();
        let _rug_ed_tests_rug_81_rrrruuuugggg_test_to_f64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_82_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i64 = rug_fuzz_0;
        <i64 as ToPrimitive>::to_isize(&p0);
        let _rug_ed_tests_rug_82_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i8() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_to_i8 = 0;
        let rug_fuzz_0 = 10;
        let p0: i64 = rug_fuzz_0;
        p0.to_i8();
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_to_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_84_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        p0.to_i16();
        let _rug_ed_tests_rug_84_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i32() {
        let _rug_st_tests_rug_85_rrrruuuugggg_test_to_i32 = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <i64 as ToPrimitive>::to_i32(&p0);
        let _rug_ed_tests_rug_85_rrrruuuugggg_test_to_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_86_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        <i64>::to_i64(&p0);
        let _rug_ed_tests_rug_86_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_87_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <i64>::to_i128(&p0);
        let _rug_ed_tests_rug_87_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_88_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        p0.to_usize();
        let _rug_ed_tests_rug_88_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_89_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        <i64>::to_u8(&p0);
        let _rug_ed_tests_rug_89_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_90_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <i64 as ToPrimitive>::to_u16(&p0);
        let _rug_ed_tests_rug_90_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_91_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        <i64 as ToPrimitive>::to_u32(&p0);
        let _rug_ed_tests_rug_91_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_92_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        <i64 as ToPrimitive>::to_u64(&p0);
        let _rug_ed_tests_rug_92_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_93_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i64 = -rug_fuzz_0;
        <i64>::to_u128(&p0);
        let _rug_ed_tests_rug_93_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_95 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_95_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i64 = -rug_fuzz_0;
        <i64 as ToPrimitive>::to_f64(&p0);
        let _rug_ed_tests_rug_95_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_97_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_97_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_98_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as ToPrimitive>::to_i16(&p0);
        let _rug_ed_tests_rug_98_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i32() {
        let _rug_st_tests_rug_99_rrrruuuugggg_test_to_i32 = 0;
        let rug_fuzz_0 = 12345;
        let p0: i128 = rug_fuzz_0;
        p0.to_i32();
        let _rug_ed_tests_rug_99_rrrruuuugggg_test_to_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_101_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let mut p0: i128 = -rug_fuzz_0;
        p0.to_i128();
        let _rug_ed_tests_rug_101_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_102 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_102_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as ToPrimitive>::to_usize(&p0);
        let _rug_ed_tests_rug_102_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_103_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let p0: i128 = rug_fuzz_0;
        p0.to_u8();
        let _rug_ed_tests_rug_103_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u16() {
        let _rug_st_tests_rug_104_rrrruuuugggg_test_to_u16 = 0;
        let rug_fuzz_0 = 123_456_789_123_456_789;
        let p0: i128 = rug_fuzz_0;
        p0.to_u16();
        let _rug_ed_tests_rug_104_rrrruuuugggg_test_to_u16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_105_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as ToPrimitive>::to_u32(&p0);
        let _rug_ed_tests_rug_105_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_106_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as ToPrimitive>::to_u64(&p0);
        let _rug_ed_tests_rug_106_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_107_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: i128 = rug_fuzz_0;
        <i128>::to_u128(&p0);
        let _rug_ed_tests_rug_107_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_108 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_108_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as ToPrimitive>::to_f32(&p0);
        let _rug_ed_tests_rug_108_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_109 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_109_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789012345678901234567890;
        let mut p0: i128 = rug_fuzz_0;
        <i128>::to_f64(&p0);
        let _rug_ed_tests_rug_109_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i8() {
        let _rug_st_tests_rug_111_rrrruuuugggg_test_to_i8 = 0;
        let rug_fuzz_0 = 100;
        let p0: usize = rug_fuzz_0;
        p0.to_i8();
        let _rug_ed_tests_rug_111_rrrruuuugggg_test_to_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_113_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = rug_fuzz_0;
        <usize as ToPrimitive>::to_i32(&p0);
        let _rug_ed_tests_rug_113_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_114_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        <usize as ToPrimitive>::to_i64(&p0);
        let _rug_ed_tests_rug_114_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_116_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: usize = rug_fuzz_0;
        <usize as crate::cast::ToPrimitive>::to_usize(&p0);
        let _rug_ed_tests_rug_116_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_117_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: usize = rug_fuzz_0;
        p0.to_u8();
        let _rug_ed_tests_rug_117_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_118_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        p0.to_u16();
        let _rug_ed_tests_rug_118_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_119 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_119_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: usize = rug_fuzz_0;
        p0.to_u32();
        let _rug_ed_tests_rug_119_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u64() {
        let _rug_st_tests_rug_120_rrrruuuugggg_test_to_u64 = 0;
        let rug_fuzz_0 = 42;
        let p0: usize = rug_fuzz_0;
        p0.to_u64();
        let _rug_ed_tests_rug_120_rrrruuuugggg_test_to_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_121_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        p0.to_u128();
        let _rug_ed_tests_rug_121_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_124_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        p0.to_isize();
        let _rug_ed_tests_rug_124_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_125_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: u8 = rug_fuzz_0;
        p0.to_i8();
        let _rug_ed_tests_rug_125_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_126_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        <u8>::to_i16(&p0);
        let _rug_ed_tests_rug_126_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_127_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        <u8 as ToPrimitive>::to_i32(&p0);
        let _rug_ed_tests_rug_127_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_129_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: u8 = rug_fuzz_0;
        <u8 as ToPrimitive>::to_i128(&p0);
        let _rug_ed_tests_rug_129_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_130_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: u8 = rug_fuzz_0;
        p0.to_usize();
        let _rug_ed_tests_rug_130_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u8() {
        let _rug_st_tests_rug_131_rrrruuuugggg_test_to_u8 = 0;
        let rug_fuzz_0 = 42;
        let p0: u8 = rug_fuzz_0;
        p0.to_u8();
        let _rug_ed_tests_rug_131_rrrruuuugggg_test_to_u8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_132 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_132_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: u8 = rug_fuzz_0;
        <u8 as ToPrimitive>::to_u16(&p0);
        let _rug_ed_tests_rug_132_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_133 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_133_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        <u8 as ToPrimitive>::to_u32(&p0);
        let _rug_ed_tests_rug_133_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use crate::cast::ToPrimitive;
    use std::mem::size_of;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_134_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u8 = rug_fuzz_0;
        p0.to_u64();
        let _rug_ed_tests_rug_134_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_135_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        p0.to_u128();
        let _rug_ed_tests_rug_135_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_f32() {
        let _rug_st_tests_rug_136_rrrruuuugggg_test_to_f32 = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        p0.to_f32();
        let _rug_ed_tests_rug_136_rrrruuuugggg_test_to_f32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_137_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        <u8>::to_f64(&p0);
        let _rug_ed_tests_rug_137_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_isize() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_to_isize = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        <u16 as ToPrimitive>::to_isize(&p0);
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_to_isize = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i8() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_to_i8 = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        <u16 as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_to_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_140_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        p0.to_i16();
        let _rug_ed_tests_rug_140_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_142_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        p0.to_i64();
        let _rug_ed_tests_rug_142_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_143_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        p0.to_i128();
        let _rug_ed_tests_rug_143_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_144_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16>::to_usize(&p0);
        let _rug_ed_tests_rug_144_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::ToPrimitive;
    use std::mem::size_of;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_145_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: u16 = rug_fuzz_0;
        p0.to_u8();
        let _rug_ed_tests_rug_145_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_146_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        p0.to_u16();
        let _rug_ed_tests_rug_146_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_147_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        p0.to_u32();
        let _rug_ed_tests_rug_147_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_148_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as ToPrimitive>::to_u64(&p0);
        let _rug_ed_tests_rug_148_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u128() {
        let _rug_st_tests_rug_149_rrrruuuugggg_test_to_u128 = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        <u16 as ToPrimitive>::to_u128(&p0);
        let _rug_ed_tests_rug_149_rrrruuuugggg_test_to_u128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_151 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_151_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: u16 = rug_fuzz_0;
        <u16>::to_f64(&p0);
        let _rug_ed_tests_rug_151_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_153_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0: u32 = rug_fuzz_0;
        p0.to_i8();
        let _rug_ed_tests_rug_153_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_154_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: u32 = rug_fuzz_0;
        p0.to_i16();
        let _rug_ed_tests_rug_154_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_155_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        <u32 as ToPrimitive>::to_i32(&p0);
        let _rug_ed_tests_rug_155_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i64() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_to_i64 = 0;
        let rug_fuzz_0 = 42;
        let p0: u32 = rug_fuzz_0;
        <u32 as ToPrimitive>::to_i64(&p0);
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_to_i64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_157 {
    use super::*;
    use crate::*;
    #[test]
    fn test_to_i128() {
        let _rug_st_tests_rug_157_rrrruuuugggg_test_to_i128 = 0;
        let rug_fuzz_0 = 42;
        let p0: u32 = rug_fuzz_0;
        <u32 as cast::ToPrimitive>::to_i128(&p0);
        let _rug_ed_tests_rug_157_rrrruuuugggg_test_to_i128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_159_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        <u32 as ToPrimitive>::to_u8(&p0);
        let _rug_ed_tests_rug_159_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_161_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        <u32>::to_u32(&p0);
        let _rug_ed_tests_rug_161_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_162_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u32 = rug_fuzz_0;
        <u32>::to_u64(&p0);
        let _rug_ed_tests_rug_162_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_163_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        <u32 as ToPrimitive>::to_u128(&p0);
        let _rug_ed_tests_rug_163_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_164_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        <u32 as ToPrimitive>::to_f32(&p0);
        let _rug_ed_tests_rug_164_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_165 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_f64() {
        let _rug_st_tests_rug_165_rrrruuuugggg_test_to_f64 = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        <u32 as ToPrimitive>::to_f64(&p0);
        let _rug_ed_tests_rug_165_rrrruuuugggg_test_to_f64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_166_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: u64 = rug_fuzz_0;
        p0.to_isize();
        let _rug_ed_tests_rug_166_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_167_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_167_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_168 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_168_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as ToPrimitive>::to_i16(&p0);
        let _rug_ed_tests_rug_168_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_169 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_169_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let mut p0: u64 = rug_fuzz_0;
        p0.to_i32();
        let _rug_ed_tests_rug_169_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_170 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_170_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as ToPrimitive>::to_i64(&p0);
        let _rug_ed_tests_rug_170_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_173_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: u64 = rug_fuzz_0;
        p0.to_u8();
        let _rug_ed_tests_rug_173_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_174 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u16() {
        let _rug_st_tests_rug_174_rrrruuuugggg_test_to_u16 = 0;
        let rug_fuzz_0 = 12345;
        let p0: u64 = rug_fuzz_0;
        p0.to_u16();
        let _rug_ed_tests_rug_174_rrrruuuugggg_test_to_u16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_175 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_175_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: u64 = rug_fuzz_0;
        <u64>::to_u32(&p0);
        let _rug_ed_tests_rug_175_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_176_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        p0.to_u64();
        let _rug_ed_tests_rug_176_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_179_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as ToPrimitive>::to_f64(&p0);
        let _rug_ed_tests_rug_179_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_180_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: u128 = rug_fuzz_0;
        p0.to_isize();
        let _rug_ed_tests_rug_180_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_181 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i8() {
        let _rug_st_tests_rug_181_rrrruuuugggg_test_to_i8 = 0;
        let rug_fuzz_0 = 12345;
        let p0: u128 = rug_fuzz_0;
        <u128 as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_181_rrrruuuugggg_test_to_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_183_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: u128 = rug_fuzz_0;
        <u128 as ToPrimitive>::to_i32(&p0);
        let _rug_ed_tests_rug_183_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i64() {
        let _rug_st_tests_rug_184_rrrruuuugggg_test_to_i64 = 0;
        let rug_fuzz_0 = 123456789012345678901234567890;
        let p0: u128 = rug_fuzz_0;
        <u128 as ToPrimitive>::to_i64(&p0);
        let _rug_ed_tests_rug_184_rrrruuuugggg_test_to_i64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_185_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345678901234567890;
        let mut p0: u128 = rug_fuzz_0;
        <u128 as ToPrimitive>::to_i128(&p0);
        let _rug_ed_tests_rug_185_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_187_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let mut p0: u128 = rug_fuzz_0;
        p0.to_u8();
        let _rug_ed_tests_rug_187_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_188_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: u128 = rug_fuzz_0;
        <u128 as ToPrimitive>::to_u16(&p0);
        let _rug_ed_tests_rug_188_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_190_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: u128 = rug_fuzz_0;
        p0.to_u64();
        let _rug_ed_tests_rug_190_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_191_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u128 = rug_fuzz_0;
        <u128 as ToPrimitive>::to_u128(&p0);
        let _rug_ed_tests_rug_191_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_192_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345678901234567890;
        let mut p0: u128 = rug_fuzz_0;
        p0.to_f32();
        let _rug_ed_tests_rug_192_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_194_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        p0.to_isize();
        let _rug_ed_tests_rug_194_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_195 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_195_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42.42;
        let p0: f32 = rug_fuzz_0;
        <f32 as ToPrimitive>::to_i8(&p0);
        let _rug_ed_tests_rug_195_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_196 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i16() {
        let _rug_st_tests_rug_196_rrrruuuugggg_test_to_i16 = 0;
        let rug_fuzz_0 = 3.14159;
        let mut p0: f32 = rug_fuzz_0;
        p0.to_i16();
        let _rug_ed_tests_rug_196_rrrruuuugggg_test_to_i16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_197 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i32() {
        let _rug_st_tests_rug_197_rrrruuuugggg_test_to_i32 = 0;
        let rug_fuzz_0 = 10.5;
        let mut p0: f32 = rug_fuzz_0;
        p0.to_i32();
        let _rug_ed_tests_rug_197_rrrruuuugggg_test_to_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::{ToPrimitive, cast};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_198_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42.0;
        let p0: f32 = rug_fuzz_0;
        <f32 as cast::ToPrimitive>::to_i64(&p0);
        let _rug_ed_tests_rug_198_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i128() {
        let _rug_st_tests_rug_199_rrrruuuugggg_test_to_i128 = 0;
        let rug_fuzz_0 = 8.9;
        let mut p0: f32 = rug_fuzz_0;
        p0.to_i128();
        let _rug_ed_tests_rug_199_rrrruuuugggg_test_to_i128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u16() {
        let _rug_st_tests_rug_202_rrrruuuugggg_test_to_u16 = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        p0.to_u16();
        let _rug_ed_tests_rug_202_rrrruuuugggg_test_to_u16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_204_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f32 = rug_fuzz_0;
        p0.to_u64();
        let _rug_ed_tests_rug_204_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_205 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u128() {
        let _rug_st_tests_rug_205_rrrruuuugggg_test_to_u128 = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f32 = rug_fuzz_0;
        p0.to_u128();
        let _rug_ed_tests_rug_205_rrrruuuugggg_test_to_u128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_206 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_206_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f32 = rug_fuzz_0;
        p0.to_f32();
        let _rug_ed_tests_rug_206_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_208 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_208_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        p0.to_isize();
        let _rug_ed_tests_rug_208_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_210 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_210_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64>::to_i16(&p0);
        let _rug_ed_tests_rug_210_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_212 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_212_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_i64(&p0);
        let _rug_ed_tests_rug_212_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_213 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_i128() {
        let _rug_st_tests_rug_213_rrrruuuugggg_test_to_i128 = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f64 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_i128(&p0);
        let _rug_ed_tests_rug_213_rrrruuuugggg_test_to_i128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_214 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_usize() {
        let _rug_st_tests_rug_214_rrrruuuugggg_test_to_usize = 0;
        let rug_fuzz_0 = 10.5;
        let p0: f64 = rug_fuzz_0;
        p0.to_usize();
        let _rug_ed_tests_rug_214_rrrruuuugggg_test_to_usize = 0;
    }
}
#[cfg(test)]
mod tests_rug_215 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_215_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14159_f64;
        let mut p0 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_u8(&p0);
        let _rug_ed_tests_rug_215_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_216 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_216_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42.5;
        let p0: f64 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_u16(&p0);
        let _rug_ed_tests_rug_216_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_217 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_217_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_u32(&p0);
        let _rug_ed_tests_rug_217_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_to_u64() {
        let _rug_st_tests_rug_218_rrrruuuugggg_test_to_u64 = 0;
        let rug_fuzz_0 = 42.5;
        let p0: f64 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_u64(&p0);
        let _rug_ed_tests_rug_218_rrrruuuugggg_test_to_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_219_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1.234;
        let p0: f64 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_u128(&p0);
        let _rug_ed_tests_rug_219_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_220 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_220_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14159;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_f32(&p0);
        let _rug_ed_tests_rug_220_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_221 {
    use super::*;
    use crate::cast::ToPrimitive;
    #[test]
    fn test_to_f64() {
        let _rug_st_tests_rug_221_rrrruuuugggg_test_to_f64 = 0;
        let rug_fuzz_0 = 3.14159;
        let p0: f64 = rug_fuzz_0;
        <f64 as ToPrimitive>::to_f64(&p0);
        let _rug_ed_tests_rug_221_rrrruuuugggg_test_to_f64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_222 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_isize() {
        let _rug_st_tests_rug_222_rrrruuuugggg_test_from_isize = 0;
        let rug_fuzz_0 = 42;
        let p0: isize = rug_fuzz_0;
        <isize as FromPrimitive>::from_isize(p0);
        let _rug_ed_tests_rug_222_rrrruuuugggg_test_from_isize = 0;
    }
}
#[cfg(test)]
mod tests_rug_224 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_224_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <isize as crate::cast::FromPrimitive>::from_i16(p0);
        let _rug_ed_tests_rug_224_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_227 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_227_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: i128 = rug_fuzz_0;
        <isize>::from_i128(p0);
        let _rug_ed_tests_rug_227_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_230 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_230_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <isize>::from_u16(p0);
        let _rug_ed_tests_rug_230_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u128() {
        let _rug_st_tests_rug_233_rrrruuuugggg_test_from_u128 = 0;
        let rug_fuzz_0 = 12345678901234567890;
        let p0: u128 = rug_fuzz_0;
        <isize as FromPrimitive>::from_u128(p0);
        let _rug_ed_tests_rug_233_rrrruuuugggg_test_from_u128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_236_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        <i8 as FromPrimitive>::from_isize(p0);
        let _rug_ed_tests_rug_236_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_239 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_239_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i32 = rug_fuzz_0;
        <i8 as crate::cast::FromPrimitive>::from_i32(p0);
        let _rug_ed_tests_rug_239_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_241_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i128 = rug_fuzz_0;
        <i8 as FromPrimitive>::from_i128(p0);
        let _rug_ed_tests_rug_241_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_247_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let p0: u128 = rug_fuzz_0;
        <i8 as crate::cast::FromPrimitive>::from_u128(p0);
        let _rug_ed_tests_rug_247_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_248 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_f32() {
        let _rug_st_tests_rug_248_rrrruuuugggg_test_from_f32 = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        <i8 as FromPrimitive>::from_f32(p0);
        let _rug_ed_tests_rug_248_rrrruuuugggg_test_from_f32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_253 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i32() {
        let _rug_st_tests_rug_253_rrrruuuugggg_test_from_i32 = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <i16 as FromPrimitive>::from_i32(p0);
        let _rug_ed_tests_rug_253_rrrruuuugggg_test_from_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_254 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i64() {
        let _rug_st_tests_rug_254_rrrruuuugggg_test_from_i64 = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <i16>::from_i64(p0);
        let _rug_ed_tests_rug_254_rrrruuuugggg_test_from_i64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_255 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i128() {
        let _rug_st_tests_rug_255_rrrruuuugggg_test_from_i128 = 0;
        let rug_fuzz_0 = 100;
        let p0: i128 = rug_fuzz_0;
        <i16 as FromPrimitive>::from_i128(p0);
        let _rug_ed_tests_rug_255_rrrruuuugggg_test_from_i128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_256 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_usize() {
        let _rug_st_tests_rug_256_rrrruuuugggg_test_from_usize = 0;
        let rug_fuzz_0 = 10;
        let p0: usize = rug_fuzz_0;
        <i16 as FromPrimitive>::from_usize(p0);
        let _rug_ed_tests_rug_256_rrrruuuugggg_test_from_usize = 0;
    }
}
#[cfg(test)]
mod tests_rug_257 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u8() {
        let _rug_st_tests_rug_257_rrrruuuugggg_test_from_u8 = 0;
        let rug_fuzz_0 = 42;
        let p0: u8 = rug_fuzz_0;
        <i16 as FromPrimitive>::from_u8(p0);
        let _rug_ed_tests_rug_257_rrrruuuugggg_test_from_u8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_259 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u32() {
        let _rug_st_tests_rug_259_rrrruuuugggg_test_from_u32 = 0;
        let rug_fuzz_0 = 12345;
        let p0: u32 = rug_fuzz_0;
        <i16 as FromPrimitive>::from_u32(p0);
        let _rug_ed_tests_rug_259_rrrruuuugggg_test_from_u32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_260 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u64() {
        let _rug_st_tests_rug_260_rrrruuuugggg_test_from_u64 = 0;
        let rug_fuzz_0 = 42;
        let p0: u64 = rug_fuzz_0;
        <i16 as FromPrimitive>::from_u64(p0);
        let _rug_ed_tests_rug_260_rrrruuuugggg_test_from_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_261 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_261_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let p0: u128 = rug_fuzz_0;
        <i16>::from_u128(p0);
        let _rug_ed_tests_rug_261_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_263 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_263_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 2.5;
        let mut p0: f64 = rug_fuzz_0;
        <i16>::from_f64(p0);
        let _rug_ed_tests_rug_263_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_isize() {
        let _rug_st_tests_rug_264_rrrruuuugggg_test_from_isize = 0;
        let rug_fuzz_0 = 42;
        let p0: isize = rug_fuzz_0;
        <i32 as crate::cast::FromPrimitive>::from_isize(p0);
        let _rug_ed_tests_rug_264_rrrruuuugggg_test_from_isize = 0;
    }
}
#[cfg(test)]
mod tests_rug_265 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i8() {
        let _rug_st_tests_rug_265_rrrruuuugggg_test_from_i8 = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i32 as FromPrimitive>::from_i8(p0);
        let _rug_ed_tests_rug_265_rrrruuuugggg_test_from_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_267 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i32() {
        let _rug_st_tests_rug_267_rrrruuuugggg_test_from_i32 = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <i32 as FromPrimitive>::from_i32(p0);
        let _rug_ed_tests_rug_267_rrrruuuugggg_test_from_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_268 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_i64() {
        let _rug_st_tests_rug_268_rrrruuuugggg_test_from_i64 = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <i32 as FromPrimitive>::from_i64(p0);
        let _rug_ed_tests_rug_268_rrrruuuugggg_test_from_i64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_274 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_274_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let p0: u64 = rug_fuzz_0;
        <i32>::from_u64(p0);
        let _rug_ed_tests_rug_274_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_278 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_isize() {
        let _rug_st_tests_rug_278_rrrruuuugggg_test_from_isize = 0;
        let rug_fuzz_0 = 10;
        let p0: isize = rug_fuzz_0;
        <i64 as FromPrimitive>::from_isize(p0);
        let _rug_ed_tests_rug_278_rrrruuuugggg_test_from_isize = 0;
    }
}
#[cfg(test)]
mod tests_rug_279 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_279_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i8 = rug_fuzz_0;
        <i64>::from_i8(p0);
        let _rug_ed_tests_rug_279_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_281 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_281_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5;
        let mut p0: i32 = rug_fuzz_0;
        <i64>::from_i32(p0);
        let _rug_ed_tests_rug_281_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_293 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_i8() {
        let _rug_st_tests_rug_293_rrrruuuugggg_test_from_i8 = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i128 as FromPrimitive>::from_i8(p0);
        let _rug_ed_tests_rug_293_rrrruuuugggg_test_from_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_300 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u16() {
        let _rug_st_tests_rug_300_rrrruuuugggg_test_from_u16 = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        <i128 as crate::cast::FromPrimitive>::from_u16(p0);
        let _rug_ed_tests_rug_300_rrrruuuugggg_test_from_u16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_302 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u64() {
        let _rug_st_tests_rug_302_rrrruuuugggg_test_from_u64 = 0;
        let rug_fuzz_0 = 42;
        let p0: u64 = rug_fuzz_0;
        <i128 as FromPrimitive>::from_u64(p0);
        let _rug_ed_tests_rug_302_rrrruuuugggg_test_from_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_304 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_304_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        <i128 as FromPrimitive>::from_f32(p0);
        let _rug_ed_tests_rug_304_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_311 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_311_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i128 = rug_fuzz_0;
        <usize as FromPrimitive>::from_i128(p0);
        let _rug_ed_tests_rug_311_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_313 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u8() {
        let _rug_st_tests_rug_313_rrrruuuugggg_test_from_u8 = 0;
        let rug_fuzz_0 = 42;
        let p0: u8 = rug_fuzz_0;
        <usize>::from_u8(p0);
        let _rug_ed_tests_rug_313_rrrruuuugggg_test_from_u8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_318_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f32 = rug_fuzz_0;
        <usize>::from_f32(p0);
        let _rug_ed_tests_rug_318_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_322 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i16() {
        let _rug_st_tests_rug_322_rrrruuuugggg_test_from_i16 = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <u8 as FromPrimitive>::from_i16(p0);
        let _rug_ed_tests_rug_322_rrrruuuugggg_test_from_i16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_323 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_323_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <u8 as FromPrimitive>::from_i32(p0);
        let _rug_ed_tests_rug_323_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_325 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i128() {
        let _rug_st_tests_rug_325_rrrruuuugggg_test_from_i128 = 0;
        let rug_fuzz_0 = 123;
        let p0: i128 = rug_fuzz_0;
        <u8 as FromPrimitive>::from_i128(p0);
        let _rug_ed_tests_rug_325_rrrruuuugggg_test_from_i128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_330 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u64() {
        let _rug_st_tests_rug_330_rrrruuuugggg_test_from_u64 = 0;
        let rug_fuzz_0 = 42;
        let p0: u64 = rug_fuzz_0;
        <u8 as crate::cast::FromPrimitive>::from_u64(p0);
        let _rug_ed_tests_rug_330_rrrruuuugggg_test_from_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_334 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_isize() {
        let _rug_st_tests_rug_334_rrrruuuugggg_test_from_isize = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        <u16 as FromPrimitive>::from_isize(p0);
        let _rug_ed_tests_rug_334_rrrruuuugggg_test_from_isize = 0;
    }
}
#[cfg(test)]
mod tests_rug_335 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_i8() {
        let _rug_st_tests_rug_335_rrrruuuugggg_test_from_i8 = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <u16 as FromPrimitive>::from_i8(p0);
        let _rug_ed_tests_rug_335_rrrruuuugggg_test_from_i8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_339 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_339_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i128 = rug_fuzz_0;
        <u16 as FromPrimitive>::from_i128(p0);
        let _rug_ed_tests_rug_339_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_340 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_340_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: usize = rug_fuzz_0;
        <u16 as FromPrimitive>::from_usize(p0);
        let _rug_ed_tests_rug_340_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_345 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u128() {
        let _rug_st_tests_rug_345_rrrruuuugggg_test_from_u128 = 0;
        let rug_fuzz_0 = 12345;
        let p0: u128 = rug_fuzz_0;
        let result = <u16>::from_u128(p0);
        let _rug_ed_tests_rug_345_rrrruuuugggg_test_from_u128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_346 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_f32() {
        let _rug_st_tests_rug_346_rrrruuuugggg_test_from_f32 = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        <u16 as FromPrimitive>::from_f32(p0);
        let _rug_ed_tests_rug_346_rrrruuuugggg_test_from_f32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_359 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_359_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: u128 = rug_fuzz_0;
        <u32 as crate::cast::FromPrimitive>::from_u128(p0);
        let _rug_ed_tests_rug_359_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_360 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_360_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14159;
        let mut p0: f32 = rug_fuzz_0;
        <u32>::from_f32(p0);
        let _rug_ed_tests_rug_360_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_363 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_363_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        <u64 as FromPrimitive>::from_i8(p0);
        let _rug_ed_tests_rug_363_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_367 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_367_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789012345678901234567890;
        let mut p0: i128 = rug_fuzz_0;
        <u64>::from_i128(p0);
        let _rug_ed_tests_rug_367_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_371 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u32() {
        let _rug_st_tests_rug_371_rrrruuuugggg_test_from_u32 = 0;
        let rug_fuzz_0 = 25;
        let p0: u32 = rug_fuzz_0;
        <u64 as FromPrimitive>::from_u32(p0);
        let _rug_ed_tests_rug_371_rrrruuuugggg_test_from_u32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_372 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_u64() {
        let _rug_st_tests_rug_372_rrrruuuugggg_test_from_u64 = 0;
        let rug_fuzz_0 = 42;
        let p0: u64 = rug_fuzz_0;
        <u64 as FromPrimitive>::from_u64(p0);
        let _rug_ed_tests_rug_372_rrrruuuugggg_test_from_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_376 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_376_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: isize = rug_fuzz_0;
        <u128>::from_isize(p0);
        let _rug_ed_tests_rug_376_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_377 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_i8() {
        let _rug_st_tests_rug_377_rrrruuuugggg_test_from_i8 = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <u128 as crate::cast::FromPrimitive>::from_i8(p0);
        let _rug_ed_tests_rug_377_rrrruuuugggg_test_from_i8 = 0;
    }
}
#[test]
fn test_rug() {
    let p0: u64 = 1234;
    <u128>::from_u64(p0);
}
#[cfg(test)]
mod tests_rug_387 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u128() {
        let _rug_st_tests_rug_387_rrrruuuugggg_test_from_u128 = 0;
        let rug_fuzz_0 = 123456789;
        let p0: u128 = rug_fuzz_0;
        <u128 as crate::cast::FromPrimitive>::from_u128(p0);
        let _rug_ed_tests_rug_387_rrrruuuugggg_test_from_u128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_388 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_f32() {
        let _rug_st_tests_rug_388_rrrruuuugggg_test_from_f32 = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        <u128>::from_f32(p0);
        let _rug_ed_tests_rug_388_rrrruuuugggg_test_from_f32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_390 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_isize() {
        let _rug_st_tests_rug_390_rrrruuuugggg_test_from_isize = 0;
        let rug_fuzz_0 = 10;
        let p0: isize = rug_fuzz_0;
        <f32 as FromPrimitive>::from_isize(p0);
        let _rug_ed_tests_rug_390_rrrruuuugggg_test_from_isize = 0;
    }
}
#[cfg(test)]
mod tests_rug_392 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_i16() {
        let _rug_st_tests_rug_392_rrrruuuugggg_test_from_i16 = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <f32 as FromPrimitive>::from_i16(p0);
        let _rug_ed_tests_rug_392_rrrruuuugggg_test_from_i16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_393 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i32() {
        let _rug_st_tests_rug_393_rrrruuuugggg_test_from_i32 = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <f32 as FromPrimitive>::from_i32(p0);
        let _rug_ed_tests_rug_393_rrrruuuugggg_test_from_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_399 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_u32() {
        let _rug_st_tests_rug_399_rrrruuuugggg_test_from_u32 = 0;
        let rug_fuzz_0 = 123;
        let p0: u32 = rug_fuzz_0;
        <f32>::from_u32(p0);
        let _rug_ed_tests_rug_399_rrrruuuugggg_test_from_u32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_400 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_u64() {
        let _rug_st_tests_rug_400_rrrruuuugggg_test_from_u64 = 0;
        let rug_fuzz_0 = 123;
        let p0: u64 = rug_fuzz_0;
        <f32>::from_u64(p0);
        let _rug_ed_tests_rug_400_rrrruuuugggg_test_from_u64 = 0;
    }
}
#[cfg(test)]
mod tests_rug_402 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_f32() {
        let _rug_st_tests_rug_402_rrrruuuugggg_test_from_f32 = 0;
        let rug_fuzz_0 = 5.6;
        let mut p0: f32 = rug_fuzz_0;
        <f32 as FromPrimitive>::from_f32(p0);
        let _rug_ed_tests_rug_402_rrrruuuugggg_test_from_f32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_406 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_from_i16() {
        let _rug_st_tests_rug_406_rrrruuuugggg_test_from_i16 = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <f64 as FromPrimitive>::from_i16(p0);
        let _rug_ed_tests_rug_406_rrrruuuugggg_test_from_i16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_407 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_from_i32() {
        let _rug_st_tests_rug_407_rrrruuuugggg_test_from_i32 = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <f64 as FromPrimitive>::from_i32(p0);
        let _rug_ed_tests_rug_407_rrrruuuugggg_test_from_i32 = 0;
    }
}
#[cfg(test)]
mod tests_rug_409 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_409_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: i128 = rug_fuzz_0;
        <f64>::from_i128(p0);
        let _rug_ed_tests_rug_409_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_416 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_416_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        <f64>::from_f32(p0);
        let _rug_ed_tests_rug_416_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_418 {
    use super::*;
    use crate::ToPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_418_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        p0.to_isize();
        let _rug_ed_tests_rug_418_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_419 {
    use super::*;
    use crate::ToPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_419_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        p0.to_i8();
        let _rug_ed_tests_rug_419_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_420 {
    use crate::ToPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_420_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <std::num::Wrapping<i32> as crate::ToPrimitive>::to_i16(&p0);
        let _rug_ed_tests_rug_420_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_421 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_421_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <std::num::Wrapping<i32> as ToPrimitive>::to_i32(&p0);
        let _rug_ed_tests_rug_421_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_422 {
    use super::*;
    use std::num::Wrapping;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_422_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        p0.to_i64();
        let _rug_ed_tests_rug_422_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_423 {
    use super::*;
    use crate::ToPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_to_i128() {
        let _rug_st_tests_rug_423_rrrruuuugggg_test_to_i128 = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        p0.to_i128();
        let _rug_ed_tests_rug_423_rrrruuuugggg_test_to_i128 = 0;
    }
}
#[cfg(test)]
mod tests_rug_424 {
    use std::num::Wrapping;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_424_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        p0.to_usize();
        let _rug_ed_tests_rug_424_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_425 {
    use super::*;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_425_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <std::num::Wrapping<i32> as crate::ToPrimitive>::to_u8(&p0);
        let _rug_ed_tests_rug_425_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_426 {
    use super::*;
    use crate::ToPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_426_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: std::num::Wrapping<i32> = std::num::Wrapping(rug_fuzz_0);
        <std::num::Wrapping<i32> as ToPrimitive>::to_u16(&p0);
        let _rug_ed_tests_rug_426_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_427 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_427_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: std::num::Wrapping<i32> = std::num::Wrapping(rug_fuzz_0);
        <std::num::Wrapping<i32> as ToPrimitive>::to_u32(&p0);
        let _rug_ed_tests_rug_427_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_428 {
    use super::*;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_428_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: std::num::Wrapping<i32> = std::num::Wrapping(rug_fuzz_0);
        <std::num::Wrapping<i32> as crate::ToPrimitive>::to_u64(&p0);
        let _rug_ed_tests_rug_428_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_429 {
    use super::*;
    use crate::ToPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_429_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        p0.to_u128();
        let _rug_ed_tests_rug_429_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_430 {
    use super::*;
    use crate::ToPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_430_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        p0.to_f32();
        let _rug_ed_tests_rug_430_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_431 {
    use super::*;
    use std::num::Wrapping;
    use crate::ToPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_431_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <std::num::Wrapping<i32> as ToPrimitive>::to_f64(&p0);
        let _rug_ed_tests_rug_431_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_432 {
    use super::*;
    use crate::FromPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_432_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        <Wrapping<isize> as FromPrimitive>::from_isize(p0);
        let _rug_ed_tests_rug_432_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_443 {
    use super::*;
    use crate::FromPrimitive;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_443_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let mut p0: u128 = rug_fuzz_0;
        <Wrapping<u128>>::from_u128(p0);
        let _rug_ed_tests_rug_443_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_444 {
    use super::*;
    use crate::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_444_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14159;
        let p0: f32 = rug_fuzz_0;
        <std::num::Wrapping<f32>>::from_f32(p0);
        let _rug_ed_tests_rug_444_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_445 {
    use super::*;
    use crate::cast::FromPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_445_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let p0 = rug_fuzz_0;
        <std::num::Wrapping<f64>>::from_f64(p0);
        let _rug_ed_tests_rug_445_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_447 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_447_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <u16 as NumCast>::from(p0);
        let _rug_ed_tests_rug_447_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_448 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_448_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <u32 as NumCast>::from(p0);
        let _rug_ed_tests_rug_448_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_450 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_450_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <u128 as crate::cast::NumCast>::from(p0);
        let _rug_ed_tests_rug_450_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_451 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_451_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <usize as NumCast>::from(p0);
        let _rug_ed_tests_rug_451_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_452 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        #[allow(deprecated)]
        fn from<N: ToPrimitive>(n: N) -> Option<i8> {
            n.to_i8()
        }
        let mut p0: Wrapping<i32> = Wrapping(42);
        <i8 as crate::cast::NumCast>::from(p0);
    }
}
#[cfg(test)]
mod tests_rug_453 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_453_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <i16 as NumCast>::from(p0);
        let _rug_ed_tests_rug_453_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_454 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_454_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <i32 as NumCast>::from(p0);
        let _rug_ed_tests_rug_454_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_456 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_456_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <i128 as NumCast>::from::<Wrapping<i32>>(p0);
        let _rug_ed_tests_rug_456_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_457 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_457_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <isize as crate::cast::NumCast>::from(p0);
        let _rug_ed_tests_rug_457_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_459 {
    use super::*;
    use crate::NumCast;
    use std::num::Wrapping;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_459_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: Wrapping<i32> = Wrapping(rug_fuzz_0);
        <f64 as crate::cast::NumCast>::from(p0);
        let _rug_ed_tests_rug_459_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_460 {
    use super::*;
    use crate::NumCast;
    #[cfg(test)]
    mod tests_rug_460_prepare {
        use std::num::Wrapping;
        #[test]
        fn sample() {
            let _rug_st_tests_rug_460_prepare_rrrruuuugggg_sample = 0;
            let rug_fuzz_0 = 0;
            let rug_fuzz_1 = 42;
            let rug_fuzz_2 = 0;
            let _rug_st_tests_rug_460_rrrruuuugggg_sample = rug_fuzz_0;
            let rug_fuzz_0 = rug_fuzz_1;
            let mut v1: Wrapping<i32> = Wrapping(rug_fuzz_0);
            let _rug_ed_tests_rug_460_rrrruuuugggg_sample = rug_fuzz_2;
            let _rug_ed_tests_rug_460_prepare_rrrruuuugggg_sample = 0;
        }
    }
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_460_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: std::num::Wrapping<i32> = std::num::Wrapping(rug_fuzz_0);
        <std::num::Wrapping<i32> as crate::cast::NumCast>::from(p0);
        let _rug_ed_tests_rug_460_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_464 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_464_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u8 = rug_fuzz_0;
        <u8 as AsPrimitive<u8>>::as_(p0);
        let _rug_ed_tests_rug_464_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_476 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_476_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        <i8 as AsPrimitive<f32>>::as_(p0);
        let _rug_ed_tests_rug_476_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_478 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_478_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        <i8 as AsPrimitive<u8>>::as_(p0);
        let _rug_ed_tests_rug_478_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_482_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        <i8 as AsPrimitive<u128>>::as_(p0);
        let _rug_ed_tests_rug_482_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_486 {
    use crate::AsPrimitive;
    use super::*;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_486_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i8 as AsPrimitive<i32>>::as_(p0);
        let _rug_ed_tests_rug_486_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_487 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_487_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        let result: i64 = p0.as_();
        let _rug_ed_tests_rug_487_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_489 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_489_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        <i8 as AsPrimitive<isize>>::as_(p0);
        let _rug_ed_tests_rug_489_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_493 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_493_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as AsPrimitive<u16>>::as_(p0);
        let _rug_ed_tests_rug_493_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_496 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_496_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as AsPrimitive<u128>>::as_(p0);
        let _rug_ed_tests_rug_496_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_497 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_497_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        <u16 as crate::AsPrimitive<usize>>::as_(p0);
        let _rug_ed_tests_rug_497_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_498 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_498_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        <u16 as AsPrimitive<i8>>::as_(p0);
        let _rug_ed_tests_rug_498_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_499 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_499_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as AsPrimitive<i16>>::as_(p0);
        let _rug_ed_tests_rug_499_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_500 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_500_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        <u16 as AsPrimitive<i32>>::as_(p0);
        let _rug_ed_tests_rug_500_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_501 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_501_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as AsPrimitive<i64>>::as_(p0);
        let _rug_ed_tests_rug_501_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_502 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_502_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        <u16 as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_502_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_504 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_504_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16 as AsPrimitive<f32>>::as_(p0);
        let _rug_ed_tests_rug_504_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_505 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_505_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <i16 as AsPrimitive<f64>>::as_(p0);
        let _rug_ed_tests_rug_505_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_507 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_507_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16 as AsPrimitive<u16>>::as_(p0);
        let _rug_ed_tests_rug_507_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_508 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_508_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16 as AsPrimitive<u32>>::as_(p0);
        let _rug_ed_tests_rug_508_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_509 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_509_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16 as AsPrimitive<u64>>::as_(p0);
        let _rug_ed_tests_rug_509_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_511 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_511_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <i16 as AsPrimitive<usize>>::as_(p0);
        let _rug_ed_tests_rug_511_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_513 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_513_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i16 = rug_fuzz_0;
        <i16 as AsPrimitive<i16>>::as_(p0);
        let _rug_ed_tests_rug_513_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_516 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_516_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <i16 as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_516_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_518 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_518_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        <u32 as AsPrimitive<f32>>::as_(p0);
        let _rug_ed_tests_rug_518_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_524 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_524_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u32 = rug_fuzz_0;
        <u32 as AsPrimitive<u128>>::as_(p0);
        let _rug_ed_tests_rug_524_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_530 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_530_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        <u32 as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_530_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_535 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_535_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i32 = rug_fuzz_0;
        <i32 as AsPrimitive<u16>>::as_(p0);
        let _rug_ed_tests_rug_535_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_536 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_536_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <i32 as AsPrimitive<u32>>::as_(p0);
        let _rug_ed_tests_rug_536_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_543 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_543_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <i32 as AsPrimitive<i64>>::as_(p0);
        let _rug_ed_tests_rug_543_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_544 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_544_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let mut p0: i32 = rug_fuzz_0;
        <i32 as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_544_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_545 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_545_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        <i32 as AsPrimitive<isize>>::as_(p0);
        let _rug_ed_tests_rug_545_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_548 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_548_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as AsPrimitive<u8>>::as_(p0);
        let _rug_ed_tests_rug_548_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_550 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_550_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 55;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as AsPrimitive<u32>>::as_(p0);
        let _rug_ed_tests_rug_550_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_551 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_551_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as AsPrimitive<u64>>::as_(p0);
        let _rug_ed_tests_rug_551_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_552 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_552_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u64 = rug_fuzz_0;
        <u64 as AsPrimitive<u128>>::as_(p0);
        let _rug_ed_tests_rug_552_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_553 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_553_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let p0: u64 = rug_fuzz_0;
        <u64 as AsPrimitive<usize>>::as_(p0);
        let _rug_ed_tests_rug_553_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_554 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_554_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as AsPrimitive<i8>>::as_(p0);
        let _rug_ed_tests_rug_554_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_555 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_555_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as AsPrimitive<i16>>::as_(p0);
        let _rug_ed_tests_rug_555_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_558 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_558_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        <u64 as crate::cast::AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_558_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_561 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_561_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <i64 as AsPrimitive<f64>>::as_(p0);
        let _rug_ed_tests_rug_561_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_563 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_563_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        <i64 as AsPrimitive<u16>>::as_(p0);
        let _rug_ed_tests_rug_563_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_566 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_566_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        <i64 as AsPrimitive<u128>>::as_(p0);
        let _rug_ed_tests_rug_566_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_567 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_567_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i64 = rug_fuzz_0;
        <i64 as AsPrimitive<usize>>::as_(p0);
        let _rug_ed_tests_rug_567_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_572 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_572_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0: i64 = rug_fuzz_0;
        <i64 as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_572_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_573 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_573_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        <i64 as AsPrimitive<isize>>::as_(p0);
        let _rug_ed_tests_rug_573_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_574 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_574_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let p0: u128 = rug_fuzz_0;
        <u128 as AsPrimitive<f32>>::as_(p0);
        let _rug_ed_tests_rug_574_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_575 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_575_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: u128 = rug_fuzz_0;
        <u128 as crate::cast::AsPrimitive<f64>>::as_(p0);
        let _rug_ed_tests_rug_575_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_576 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_576_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let p0: u128 = rug_fuzz_0;
        <u128 as AsPrimitive<u8>>::as_(p0);
        let _rug_ed_tests_rug_576_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_578 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_578_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: u128 = rug_fuzz_0;
        <u128 as AsPrimitive<u32>>::as_(p0);
        let _rug_ed_tests_rug_578_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_582 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_582_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 12345;
        let mut p0: u128 = rug_fuzz_0;
        <u128 as crate::cast::AsPrimitive<i8>>::as_(p0);
        let _rug_ed_tests_rug_582_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_586 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_586_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let p0: u128 = rug_fuzz_0;
        <u128 as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_586_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_591 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_591_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as AsPrimitive<u16>>::as_(p0);
        let _rug_ed_tests_rug_591_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_595 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_595_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 100;
        let p0: i128 = rug_fuzz_0;
        <i128 as AsPrimitive<usize>>::as_(p0);
        let _rug_ed_tests_rug_595_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_596 {
    use super::*;
    use crate::cast::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_596_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 123456789;
        let p0: i128 = rug_fuzz_0;
        <i128 as AsPrimitive<i8>>::as_(p0);
        let _rug_ed_tests_rug_596_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_598 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_598_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: i128 = rug_fuzz_0;
        <i128 as crate::cast::AsPrimitive<i32>>::as_(p0);
        let _rug_ed_tests_rug_598_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_603 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_603_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: usize = rug_fuzz_0;
        <usize as AsPrimitive<f64>>::as_(p0);
        let _rug_ed_tests_rug_603_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_612 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_612_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: usize = rug_fuzz_0;
        <usize as AsPrimitive<i32>>::as_(p0);
        let _rug_ed_tests_rug_612_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_613 {
    use super::*;
    use crate::{AsPrimitive, cast};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_613_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0: usize = rug_fuzz_0;
        <usize as cast::AsPrimitive<i64>>::as_(p0);
        let _rug_ed_tests_rug_613_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_615 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_615_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: usize = rug_fuzz_0;
        <usize as AsPrimitive<isize>>::as_(p0);
        let _rug_ed_tests_rug_615_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_616 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_616_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 100;
        let p0: isize = rug_fuzz_0;
        <isize as crate::cast::AsPrimitive<f32>>::as_(p0);
        let _rug_ed_tests_rug_616_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_618 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_618_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: isize = rug_fuzz_0;
        <isize as AsPrimitive<u8>>::as_(p0);
        let _rug_ed_tests_rug_618_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_621 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_621_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        <isize as AsPrimitive<u64>>::as_(p0);
        let _rug_ed_tests_rug_621_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_625 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_625_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0: isize = rug_fuzz_0;
        <isize as AsPrimitive<i16>>::as_(p0);
        let _rug_ed_tests_rug_625_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_632 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_632_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5.5;
        let mut p0: f32 = rug_fuzz_0;
        <f32 as AsPrimitive<u8>>::as_(p0);
        let _rug_ed_tests_rug_632_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_634 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_634_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        <f32 as AsPrimitive<u32>>::as_(p0);
        let _rug_ed_tests_rug_634_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_636 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_636_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        <f32 as AsPrimitive<u128>>::as_(p0);
        let _rug_ed_tests_rug_636_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_640 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_640_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f32 = rug_fuzz_0;
        <f32 as AsPrimitive<i32>>::as_(p0);
        let _rug_ed_tests_rug_640_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_642 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_642_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f32 = rug_fuzz_0;
        <f32 as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_642_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_647 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_647_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<u16>>::as_(p0);
        let _rug_ed_tests_rug_647_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_648 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_648_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<u32>>::as_(p0);
        let _rug_ed_tests_rug_648_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_649 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_649_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<u64>>::as_(p0);
        let _rug_ed_tests_rug_649_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_650 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_650_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<u128>>::as_(p0);
        let _rug_ed_tests_rug_650_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_651 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_651_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14159;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<usize>>::as_(p0);
        let _rug_ed_tests_rug_651_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_653 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_653_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<i16>>::as_(p0);
        let _rug_ed_tests_rug_653_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_655 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_655_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<i64>>::as_(p0);
        let _rug_ed_tests_rug_655_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_656 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_656_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_656_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_657 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_657_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        <f64 as AsPrimitive<isize>>::as_(p0);
        let _rug_ed_tests_rug_657_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_660 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_660_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'A';
        let mut p0: char = rug_fuzz_0;
        <char as AsPrimitive<u16>>::as_(p0);
        let _rug_ed_tests_rug_660_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_661 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_661_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 'a';
        let p0: char = rug_fuzz_0;
        <char as AsPrimitive<u32>>::as_(p0);
        let _rug_ed_tests_rug_661_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_667 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_667_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0: char = rug_fuzz_0;
        <char as AsPrimitive<i32>>::as_(p0);
        let _rug_ed_tests_rug_667_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_669 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_669_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = 'a';
        let p0: char = rug_fuzz_0;
        <char as AsPrimitive<i128>>::as_(p0);
        let _rug_ed_tests_rug_669_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_670 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_670_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'A';
        let p0: char = rug_fuzz_0;
        <char as AsPrimitive<isize>>::as_(p0);
        let _rug_ed_tests_rug_670_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_671 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_671_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: bool = rug_fuzz_0;
        <bool as AsPrimitive<u8>>::as_(p0);
        let _rug_ed_tests_rug_671_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_672 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_672_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: bool = rug_fuzz_0;
        <bool as AsPrimitive<u16>>::as_(p0);
        let _rug_ed_tests_rug_672_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_676 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_as_() {
        let _rug_st_tests_rug_676_rrrruuuugggg_test_as_ = 0;
        let rug_fuzz_0 = true;
        let p0: bool = rug_fuzz_0;
        <bool as AsPrimitive<usize>>::as_(p0);
        let _rug_ed_tests_rug_676_rrrruuuugggg_test_as_ = 0;
    }
}
#[cfg(test)]
mod tests_rug_678 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_678_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let p0: bool = rug_fuzz_0;
        <bool as AsPrimitive<i16>>::as_(p0);
        let _rug_ed_tests_rug_678_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_682 {
    use super::*;
    use crate::AsPrimitive;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_682_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let p0: bool = rug_fuzz_0;
        <bool as AsPrimitive<isize>>::as_(p0);
        let _rug_ed_tests_rug_682_rrrruuuugggg_test_rug = 0;
    }
}
