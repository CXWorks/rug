//! Port of the `Box<[T]>` function API.
use crate::{
    boxed::BitBox, order::BitOrder, pointer::BitPtr, slice::BitSlice, store::BitStore,
    vec::BitVec,
};
use core::{marker::Unpin, mem::ManuallyDrop, pin::Pin};
use wyz::pipe::Pipe;
impl<O, T> BitBox<O, T>
where
    O: BitOrder,
    T: BitStore,
{
    /// Allocates memory on the heap and copies `x` into it.
    ///
    /// This doesn’t actually allocate if `x` is zero-length.
    ///
    /// # Original
    ///
    /// [`Box::new`](https://doc.rust-lang.org/alloc/boxed/struct.Box.html#method.new)
    ///
    /// # API Differences
    ///
    /// `Box::<[T]>::new` does not exist, because `new` cannot take unsized
    /// types by value. Instead, this takes a slice reference, and boxes the
    /// referent slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let boxed = BitBox::new(bits![0; 5]);
    /// ```
    #[cfg_attr(not(tarpaulin), inline(always))]
    #[deprecated(since = "0.18.0", note = "Prefer `::from_bitslice`")]
    pub fn new(x: &BitSlice<O, T>) -> Self {
        Self::from_bitslice(x)
    }
    /// Constructs a new `Pin<BitBox<O, T>>`.
    ///
    /// `BitSlice` is always `Unpin`, so this has no actual immobility effect.
    ///
    /// # Original
    ///
    /// [`Box::pin`](https://doc.rust-lang.org/alloc/boxed/struct.Box.html#method.pin)
    ///
    /// # API Differences
    ///
    /// As with `::new`, this only exists on `Box` when `T` is not unsized. This
    /// takes a slice reference, and pins the referent slice.
    #[inline]
    pub fn pin(x: &BitSlice<O, T>) -> Pin<Self>
    where
        O: Unpin,
        T: Unpin,
    {
        x.pipe(Self::from_bitslice).pipe(Pin::new)
    }
    /// Constructs a box from a raw pointer.
    ///
    /// After calling this function, the raw pointer is owned by the
    /// resulting `BitBox`. Specifically, the `Box` destructor will free the
    /// allocated memory. For this to be safe, the memory must have been
    /// allocated in accordance with the [memory layout] used by `Box` .
    ///
    /// # Original
    ///
    /// [`Box::from_raw`](https://doc.rust-lang.org/alloc/boxed/struct.Box.html#method.from_raw)
    ///
    /// # Safety
    ///
    /// This function is unsafe because improper use may lead to
    /// memory problems. For example, a double-free may occur if the
    /// function is called twice on the same raw pointer.
    ///
    /// # Examples
    ///
    /// Recreate a `BitBox` which was previously converted to a raw pointer
    /// using [`BitBox::into_raw`]:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let x = bitbox![0; 10];
    /// let ptr = BitBox::into_raw(x);
    /// let x = unsafe { BitBox::from_raw(ptr) };
    /// ```
    ///
    /// [memory layout]: https://doc.rust-lang.org/alloc/boxed/index.html#memory-layout
    /// [`Layout`]: https://doc.rust-lang.org/alloc/struct.Layout.html
    /// [`BitBox::into_raw`]: #method.into_raw
    #[inline]
    pub unsafe fn from_raw(raw: *mut BitSlice<O, T>) -> Self {
        raw.pipe(BitPtr::from_bitslice_ptr_mut)
            .to_nonnull()
            .pipe(|pointer| Self { pointer })
    }
    /// Consumes the `BitBox`, returning a wrapped raw pointer.
    ///
    /// The pointer will be properly aligned and non-null.
    ///
    /// After calling this function, the caller is responsible for the memory
    /// previously managed by the `BitBox`. In particular, the caller should
    /// properly release the memory by converting the pointer back into a
    /// `BitBox` with the [`BitBox::from_raw`] function, allowing the `BitBox`
    /// destructor to perform the cleanup.
    ///
    /// Note: this is an associated function, which means that you have to call
    /// it as `BitBox::into_raw(b)` instead of `b.into_raw()`. This is to match
    /// layout with the standard library’s `Box` API; there will never be a name
    /// conflict with `BitSlice`.
    ///
    /// # Original
    ///
    /// [`Box::into_raw`](https://doc.rust-lang.org/alloc/boxed/struct.Box.html#method.into_raw)
    ///
    /// # Examples
    ///
    /// Converting the raw pointer back into a `BitBox` with
    /// [`BitBox::from_raw`] for automatic cleanup:
    ///
    /// ```rust
    /// # use bitvec::prelude::*;
    /// let b = BitBox::new(bits![Msb0, u32; 0; 32]);
    /// let ptr = BitBox::into_raw(b);
    /// let b = unsafe { BitBox::<Msb0, _>::from_raw(ptr) };
    /// ```
    ///
    /// [`BitBox::from_raw`]: #method.from_raw
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn into_raw(b: Self) -> *mut BitSlice<O, T> {
        Self::leak(b)
    }
    /// Consumes and leaks the `BitBox`, returning a mutable reference,
    /// `&'a mut BitSlice<O, T>`. Note that the memory region `[T]` must outlive
    /// the chosen lifetime `'a`.
    ///
    /// This function is mainly useful for bit regions that live for the
    /// remainder of the program’s life. Dropping the returned reference will
    /// cause a memory leak. If this is not acceptable, the reference should
    /// first be wrapped with the [`BitBox::from_raw`] function, producing a
    /// `BitBox`. This `BitBox` can then be dropped which will properly
    /// deallocate the memory.
    ///
    /// Note: this is an associated function, which means that you have to call
    /// it as `BitBox::leak(b)` instead of `b.leak()`. This is to match layout
    /// with the standard library’s `Box` API; there will never be a name
    /// conflict with `BitSlice`.
    ///
    /// # Original
    ///
    /// [`Box::leak`](https://doc.rust-lang.org/alloc/boxed/struct.Box.html#method.leak)
    ///
    /// # Examples
    ///
    /// Simple usage:
    ///
    /// ```rust
    /// # use bitvec::prelude::*;
    /// let b = BitBox::new(bits![Local, u32; 0; 32]);
    /// let static_ref: &'static mut BitSlice<Local, u32> = BitBox::leak(b);
    /// static_ref.set(0, true);
    /// assert_eq!(static_ref.count_ones(), 1);
    /// ```
    ///
    /// [`BitBox::from_raw`]: #method.from_raw
    #[inline]
    pub fn leak<'a>(b: Self) -> &'a mut BitSlice<O, T>
    where
        T: 'a,
    {
        b.pipe(ManuallyDrop::new).bitptr().to_bitslice_mut()
    }
    /// Converts `self` into a vector without clones or allocation.
    ///
    /// The resulting vector can be converted back into a box via `BitVec<O,
    /// T>`’s `into_boxed_bitslice` method.
    ///
    /// # Original
    ///
    /// [`slice::into_vec`](https://doc.rust-lang.org/std/primitive.slice.html#method.into_vec)
    ///
    /// Despite taking a `Box<[T]>` receiver, this function is written in an
    /// `impl<T> [T]` block.
    ///
    /// Rust does not allow the text
    ///
    /// ```rust,ignore
    /// impl<O, T> BitSlice<O, T> {
    ///   fn into_bitvec(self: BitBox<O, T>);
    /// }
    /// ```
    ///
    /// to be written, so this function must be implemented directly on `BitBox`
    /// rather than on `BitSlice` with a boxed receiver.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let bb = bitbox![0, 1, 0, 1];
    /// let bv = bb.into_bitvec();
    ///
    /// assert_eq!(bv, bitvec![0, 1, 0, 1]);
    /// ```
    #[inline]
    pub fn into_bitvec(self) -> BitVec<O, T> {
        let bitptr = self.bitptr();
        let raw = self
            .pipe(ManuallyDrop::new)
            .with_box(|b| unsafe { ManuallyDrop::take(b) })
            .into_vec()
            .pipe(ManuallyDrop::new);
        unsafe { BitVec::from_raw_parts(bitptr.to_bitslice_ptr_mut(), raw.capacity()) }
    }
}
#[cfg(test)]
mod tests_rug_775 {
    use super::*;
    use crate::{prelude::*, boxed::BitBox, field::BitField};
    #[test]
    fn test_leak() {
        let _rug_st_tests_rug_775_rrrruuuugggg_test_leak = 0;
        let mut p0: BitBox<Local, u32> = BitBox::new(bits![Local, u32; 0; 32]);
        <BitBox<Local, u32>>::leak(p0);
        let _rug_ed_tests_rug_775_rrrruuuugggg_test_leak = 0;
    }
}
#[cfg(test)]
mod tests_rug_776 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_into_bitvec() {
        let mut p0 = bitbox![0, 1, 0, 1];
        let result = p0.into_bitvec();
        assert_eq!(result, bitvec![0, 1, 0, 1]);
    }
}
