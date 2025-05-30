//! Port of the `[T]` function API.
use crate::{
    devel as dvl, mem::BitMemory, order::BitOrder, pointer::BitPtr,
    slice::{
        iter::{
            Chunks, ChunksExact, ChunksExactMut, ChunksMut, Iter, IterMut, RChunks,
            RChunksExact, RChunksExactMut, RChunksMut, RSplit, RSplitMut, RSplitN,
            RSplitNMut, Split, SplitMut, SplitN, SplitNMut, Windows,
        },
        BitMut, BitSlice,
    },
    store::BitStore,
};
use core::{
    cmp,
    ops::{
        Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo,
        RangeToInclusive,
    },
};
use wyz::pipe::Pipe;
#[cfg(feature = "alloc")]
use crate::vec::BitVec;
/// Port of the `[T]` inherent API.
impl<O, T> BitSlice<O, T>
where
    O: BitOrder,
    T: BitStore,
{
    /// Returns the number of bits in the slice.
    ///
    /// # Original
    ///
    /// [`slice::len`](https://doc.rust-lang.org/std/primitive.slice.html#method.len)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0u32;
    /// let bits = data.view_bits::<Local>();
    /// assert_eq!(bits.len(), 32);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.bitptr().len()
    }
    /// Returns `true` if the slice has a length of 0.
    ///
    /// # Original
    ///
    /// [`slice::is_empty`](https://doc.rust-lang.org/std/primitive.slice.html#method.is_empty)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// assert!(BitSlice::<Local, u8>::empty().is_empty());
    /// assert!(!(0u32.view_bits::<Local>()).is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bitptr().len() == 0
    }
    /// Returns the first bit of the slice, or `None` if it is empty.
    ///
    /// # Original
    ///
    /// [`slice::first`](https://doc.rust-lang.org/std/primitive.slice.html#method.first)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 1u8;
    /// let bits = data.view_bits::<Lsb0>();
    /// assert_eq!(Some(&true), bits.first());
    ///
    /// let empty = BitSlice::<Local, usize>::empty();
    /// assert_eq!(None, empty.first());
    /// ```
    #[inline]
    pub fn first(&self) -> Option<&bool> {
        self.get(0)
    }
    /// Returns a mutable pointer to the first bit of the slice, or `None` if it
    /// is empty.
    ///
    /// # Original
    ///
    /// [`slice::first_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.first_mut)
    ///
    /// # API Differences
    ///
    /// This crate cannot manifest `&mut bool` references, and must use the
    /// `BitMut` proxy type where `&mut bool` exists in the standard library
    /// API. The proxy value must be bound as `mut` in order to write through
    /// it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Lsb0>();
    ///
    /// if let Some(mut first) = bits.first_mut() {
    ///   *first = true;
    /// }
    /// assert_eq!(data, 1);
    /// ```
    #[inline]
    pub fn first_mut(&mut self) -> Option<BitMut<O, T>> {
        self.get_mut(0)
    }
    /// Returns the first and all the rest of the bits of the slice, or `None`
    /// if it is empty.
    ///
    /// # Original
    ///
    /// [`slice::split_first`](https://doc.rust-lang.org/std/primitive.slice.html#split_first)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 1u8;
    /// let bits = data.view_bits::<Lsb0>();
    /// if let Some((first, rest)) = bits.split_first() {
    ///   assert!(*first);
    /// }
    /// ```
    #[inline]
    pub fn split_first(&self) -> Option<(&bool, &Self)> {
        match self.len() {
            0 => None,
            _ => {
                unsafe {
                    let (head, rest) = self.split_at_unchecked(1);
                    Some((head.get_unchecked(0), rest))
                }
            }
        }
    }
    /// Returns the first and all the rest of the bits of the slice, or `None`
    /// if it is empty.
    ///
    /// # Original
    ///
    /// [`slice::split_first_mut`](https://doc.rust-lang.org/std/primitive.slice.html#split_first_mut)
    ///
    /// # API Differences
    ///
    /// This crate cannot manifest `&mut bool` references, and must use the
    /// `BitMut` proxy type where `&mut bool` exists in the standard library
    /// API. The proxy value must be bound as `mut` in order to write through
    /// it.
    ///
    /// Because the references are permitted to use the same memory address,
    /// they are marked as aliasing in order to satisfy Rust’s requirements
    /// about freedom from data races.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0usize;
    /// let bits = data.view_bits_mut::<Lsb0>();
    ///
    /// if let Some((mut first, rest)) = bits.split_first_mut() {
    ///   *first = true;
    ///   *rest.get_mut(1).unwrap() = true;
    /// }
    /// assert_eq!(data, 5);
    ///
    /// assert!(BitSlice::<Local, usize>::empty_mut().split_first_mut().is_none());
    /// ```
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn split_first_mut(
        &mut self,
    ) -> Option<(BitMut<O, T::Alias>, &mut BitSlice<O, T::Alias>)> {
        match self.len() {
            0 => None,
            _ => {
                unsafe {
                    let (head, rest) = self.split_at_unchecked_mut(1);
                    Some((head.get_unchecked_mut(0), rest))
                }
            }
        }
    }
    /// Returns the last and all the rest of the bits of the slice, or `None` if
    /// it is empty.
    ///
    /// # Original
    ///
    /// [`slice::split_last`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_last)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 1u8;
    /// let bits = data.view_bits::<Msb0>();
    ///
    /// if let Some((last, rest)) = bits.split_last() {
    ///   assert!(*last);
    /// }
    /// ```
    #[inline]
    pub fn split_last(&self) -> Option<(&bool, &Self)> {
        match self.len() {
            0 => None,
            len => {
                unsafe {
                    let (rest, tail) = self.split_at_unchecked(len.wrapping_sub(1));
                    Some((tail.get_unchecked(0), rest))
                }
            }
        }
    }
    /// Returns the last and all the rest of the bits of the slice, or `None` if
    /// it is empty.
    ///
    /// # Original
    ///
    /// [`slice::split_last_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_last_mut)
    ///
    /// # API Differences
    ///
    /// This crate cannot manifest `&mut bool` references, and must use the
    /// `BitMut` proxy type where `&mut bool` exists in the standard library
    /// API. The proxy value must be bound as `mut` in order to write through
    /// it.
    ///
    /// Because the references are permitted to use the same memory address,
    /// they are marked as aliasing in order to satisfy Rust’s requirements
    /// about freedom from data races.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    ///
    /// if let Some((mut last, rest)) = bits.split_last_mut() {
    ///   *last = true;
    ///   *rest.get_mut(5).unwrap() = true;
    /// }
    /// assert_eq!(data, 5);
    ///
    /// assert!(BitSlice::<Local, usize>::empty_mut().split_last_mut().is_none());
    /// ```
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn split_last_mut(
        &mut self,
    ) -> Option<(BitMut<O, T::Alias>, &mut BitSlice<O, T::Alias>)> {
        match self.len() {
            0 => None,
            len => {
                unsafe {
                    let (rest, tail) = self.split_at_unchecked_mut(len - 1);
                    Some((tail.get_unchecked_mut(0), rest))
                }
            }
        }
    }
    /// Returns the last bit of the slice, or `None` if it is empty.
    ///
    /// # Original
    ///
    /// [`slice::last`](https://doc.rust-lang.org/std/primitive.slice.html#method.last)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 1u8;
    /// let bits = data.view_bits::<Msb0>();
    /// assert_eq!(Some(&true), bits.last());
    ///
    /// let empty = BitSlice::<Local, usize>::empty();
    /// assert_eq!(None, empty.last());
    /// ```
    #[inline]
    pub fn last(&self) -> Option<&bool> {
        match self.len() {
            0 => None,
            len => Some(unsafe { self.get_unchecked(len - 1) }),
        }
    }
    /// Returns a mutable pointer to the last bit of the slice, or `None` if it
    /// is empty.
    ///
    /// # Original
    ///
    /// [`slice::last_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.last_mut)
    ///
    /// # API Differences
    ///
    /// This crate cannot manifest `&mut bool` references, and must use the
    /// `BitMut` proxy type where `&mut bool` exists in the standard library
    /// API. The proxy value must be bound as `mut` in order to write through
    /// it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    ///
    /// if let Some(mut last) = bits.last_mut() {
    ///   *last = true;
    /// }
    /// assert_eq!(data, 1);
    /// ```
    #[inline]
    pub fn last_mut(&mut self) -> Option<BitMut<O, T>> {
        match self.len() {
            0 => None,
            len => Some(unsafe { self.get_unchecked_mut(len - 1) }),
        }
    }
    /// Returns a reference to an element or subslice depending on the type of
    /// index.
    ///
    /// - If given a position, returns a reference to the element at that
    ///   position or `None` if out of bounds.
    /// - If given a range, returns the subslice corresponding to that range, or
    ///   `None` if out of bounds.
    ///
    /// # Original
    ///
    /// [`slice::get`](https://doc.rust-lang.org/std/primitive.slice.html#method.get)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 2u8;
    /// let bits = data.view_bits::<Lsb0>();
    ///
    /// assert_eq!(Some(&true), bits.get(1));
    /// assert_eq!(Some(&bits[1 .. 3]), bits.get(1 .. 3));
    /// assert_eq!(None, bits.get(9));
    /// assert_eq!(None, bits.get(8 .. 10));
    /// ```
    #[inline]
    pub fn get<'a, I>(&'a self, index: I) -> Option<I::Immut>
    where
        I: BitSliceIndex<'a, O, T>,
    {
        index.get(self)
    }
    /// Returns a mutable reference to an element or subslice depending on the
    /// type of index (see [`get`]) or `None` if the index is out of bounds.
    ///
    /// # Original
    ///
    /// [`slice::get_mut`](https://doc.rust-lang.org/core/slice/trait.SliceIndex.html#method.get_mut)
    ///
    /// # API Differences
    ///
    /// When `I` is `usize`, this returns `BitMut` instead of `&mut bool`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u16;
    /// let bits = data.view_bits_mut::<Lsb0>();
    ///
    /// assert!(!bits.get(1).unwrap());
    /// *bits.get_mut(1).unwrap() = true;
    /// assert!(bits.get(1).unwrap());
    /// ```
    ///
    /// [`get`]: #method.get
    #[inline]
    pub fn get_mut<'a, I>(&'a mut self, index: I) -> Option<I::Mut>
    where
        I: BitSliceIndex<'a, O, T>,
    {
        index.get_mut(self)
    }
    /// Returns a reference to an element or subslice, without doing bounds
    /// checking.
    ///
    /// This is generally not recommended; use with caution!
    ///
    /// Unlike the original slice function, calling this with an out-of-bounds
    /// index is not *technically* compile-time [undefined behavior], as the
    /// references produced do not actually describe local memory. However, the
    /// use of an out-of-bounds index will eventually cause an out-of-bounds
    /// memory read, which is a runtime safety violation. For a safe alternative
    /// see [`get`].
    ///
    /// # Original
    ///
    /// [`slice::get_unchecked`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_unchecked)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 2u16;
    /// let bits = data.view_bits::<Lsb0>();
    ///
    /// unsafe{
    ///   assert_eq!(bits.get_unchecked(1), &true);
    /// }
    /// ```
    ///
    /// [`get`]: #method.get
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    #[inline]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unchecked<'a, I>(&'a self, index: I) -> I::Immut
    where
        I: BitSliceIndex<'a, O, T>,
    {
        index.get_unchecked(self)
    }
    /// Returns a mutable reference to the output at this location, without
    /// doing bounds checking.
    ///
    /// This is generally not recommended; use with caution!
    ///
    /// Unlike the original slice function, calling this with an out-of-bounds
    /// index is not *technically* compile-time [undefined behavior], as the
    /// references produced do not actually describe local memory. However, the
    /// use of an out-of-bounds index will eventually cause an out-of-bounds
    /// memory write, which is a runtime safety violation. For a safe
    /// alternative see [`get_mut`].
    ///
    /// # Original
    ///
    /// [`slice::get_unchecked_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_unchecked_mut)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u16;
    /// let bits = data.view_bits_mut::<Lsb0>();
    ///
    /// unsafe {
    ///   let mut bit = bits.get_unchecked_mut(1);
    ///   *bit = true;
    /// }
    /// assert_eq!(data, 2);
    /// ```
    ///
    /// [`get_mut`]: #method.get_mut
    /// [undefined behavior]: ../../reference/behavior-considered-undefined.html
    #[inline]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unchecked_mut<'a, I>(&'a mut self, index: I) -> I::Mut
    where
        I: BitSliceIndex<'a, O, T>,
    {
        index.get_unchecked_mut(self)
    }
    /// Returns a raw bit-slice pointer to the region.
    ///
    /// The caller must ensure that the slice outlives the pointer this function
    /// returns, or else it will end up pointing to garbage.
    ///
    /// The caller must also ensure that the memory the pointer
    /// (non-transitively) points to is only written to if `T` allows shared
    /// mutation, using this pointer or any pointer derived from it. If you need
    /// to mutate the contents of the slice, use [`as_mut_ptr`].
    ///
    /// Modifying the container (such as `BitVec`) referenced by this slice may
    /// cause its buffer to be reällocated, which would also make any pointers
    /// to it invalid.
    ///
    /// # Original
    ///
    /// [`slice::as_ptr`](https://doc.rust-lang.org/std/primitive.slice.html#method.as_ptr)
    ///
    /// # API Differences
    ///
    /// This returns `*const BitSlice`, which is the equivalent of `*const [T]`
    /// instead of `*const T`. The pointer encoding used requires more than one
    /// CPU word of space to address a single bit, so there is no advantage to
    /// removing the length information from the encoded pointer value.
    ///
    /// # Notes
    ///
    /// You **cannot** use any of the methods in the `pointer` fundamental type
    /// or the `core::ptr` module on the `*_ BitSlice` type. This pointer
    /// retains the `bitvec`-specific value encoding, and is incomprehensible by
    /// the Rust standard library.
    ///
    /// The only thing you can do with this pointer is dereference it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 2u16;
    /// let bits = data.view_bits::<Lsb0>();
    /// let bits_ptr = bits.as_ptr();
    ///
    /// for i in 0 .. bits.len() {
    ///   assert_eq!(bits[i], unsafe {
    ///     (&*bits_ptr)[i]
    ///   });
    /// }
    /// ```
    ///
    /// [`as_mut_ptr`]: #method.as_mut_ptr
    #[inline(always)]
    #[cfg(not(tarpaulin_include))]
    pub fn as_ptr(&self) -> *const Self {
        self as *const Self
    }
    /// Returns an unsafe mutable bit-slice pointer to the region.
    ///
    /// The caller must ensure that the slice outlives the pointer this function
    /// returns, or else it will end up pointing to garbage.
    ///
    /// Modifying the container (such as `BitVec`) referenced by this slice may
    /// cause its buffer to be reällocated, which would also make any pointers
    /// to it invalid.
    ///
    /// # Original
    ///
    /// [`slice::as_mut_ptr`](https://doc.rust-lang.org/std/primitive.slice.html#method.as_mut_ptr)
    ///
    /// # API Differences
    ///
    /// This returns `*mut BitSlice`, which is the equivalont of `*mut [T]`
    /// instead of `*mut T`. The pointer encoding used requires more than one
    /// CPU word of space to address a single bit, so there is no advantage to
    /// removing the length information from the encoded pointer value.
    ///
    /// # Notes
    ///
    /// You **cannot** use any of the methods in the `pointer` fundamental type
    /// or the `core::ptr` module on the `*_ BitSlice` type. This pointer
    /// retains the `bitvec`-specific value encoding, and is incomprehensible by
    /// the Rust standard library.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u16;
    /// let bits = data.view_bits_mut::<Lsb0>();
    /// let bits_ptr = bits.as_mut_ptr();
    ///
    /// for i in 0 .. bits.len() {
    ///   unsafe { &mut *bits_ptr }.set(i, i % 2 == 0);
    /// }
    /// assert_eq!(data, 0b0101_0101_0101_0101);
    /// ```
    #[inline(always)]
    #[cfg(not(tarpaulin_include))]
    pub fn as_mut_ptr(&mut self) -> *mut Self {
        self as *mut Self
    }
    /// Swaps two bits in the slice.
    ///
    /// # Original
    ///
    /// [`slice::swap`](https://doc.rust-lang.org/std/primitive.slice.html#method.swap)
    ///
    /// # Arguments
    ///
    /// - `a`: The index of the first bit
    /// - `b`: The index of the second bit
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 2u8;
    /// let bits = data.view_bits_mut::<Lsb0>();
    /// bits.swap(1, 3);
    /// assert_eq!(data, 8);
    /// ```
    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) {
        let len = self.len();
        assert!(a < len, "Index {} out of bounds: {}", a, len);
        assert!(b < len, "Index {} out of bounds: {}", b, len);
        unsafe {
            self.swap_unchecked(a, b);
        }
    }
    /// Reverses the order of bits in the slice, in place.
    ///
    /// # Original
    ///
    /// [`slice::reverse`](https://doc.rust-lang.org/std/primitive.slice.html#method.reverse)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0b1_1001100u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// bits[1 ..].reverse();
    /// assert_eq!(data, 0b1_0011001);
    /// ```
    #[inline]
    pub fn reverse(&mut self) {
        let mut bitptr = self.bitptr();
        loop {
            let len = bitptr.len();
            if len < 2 {
                return;
            }
            unsafe {
                let back = len - 1;
                bitptr.to_bitslice_mut::<O>().swap_unchecked(0, back);
                bitptr.incr_head();
                bitptr.set_len(len - 2);
            }
        }
    }
    /// Returns an iterator over the slice.
    ///
    /// # Original
    ///
    /// [`slice::iter`](https://doc.rust-lang.org/std/primitive.slice.html#method.iter)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 130u8;
    /// let bits = data.view_bits::<Lsb0>();
    /// let mut iterator = bits.iter();
    ///
    /// assert_eq!(iterator.next(), Some(&false));
    /// assert_eq!(iterator.next(), Some(&true));
    /// assert_eq!(iterator.nth(5), Some(&true));
    /// assert_eq!(iterator.next(), None);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<O, T> {
        self.into_iter()
    }
    /// Returns an iterator that allows modifying each bit.
    ///
    /// # Original
    ///
    /// [`slice::iter_mut`](https://doc.rust-lang.org/std/primitive.slice.html#Method.iter_mut)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// for (idx, mut elem) in bits.iter_mut().enumerate() {
    ///   *elem = idx % 3 == 0;
    /// }
    /// assert_eq!(data, 0b100_100_10);
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<O, T> {
        self.into_iter()
    }
    /// Returns an iterator over all contiguous windows of length `size`. The
    /// windows overlap. If the slice is shorter than `size`, the iterator
    /// returns no values.
    ///
    /// # Original
    ///
    /// [`slice::windows`](https://doc.rust-lang.org/std/primitive.slice.html#method.windows)
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0xA5u8;
    /// let bits = data.view_bits::<Msb0>();
    /// let mut iter = bits.windows(6);
    /// assert_eq!(iter.next().unwrap(), &bits[.. 6]);
    /// assert_eq!(iter.next().unwrap(), &bits[1 .. 7]);
    /// assert_eq!(iter.next().unwrap(), &bits[2 ..]);
    /// assert!(iter.next().is_none());
    /// ```
    ///
    /// If the slice is shorter than `size`:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let bits = BitSlice::<Local, usize>::empty();
    /// let mut iter = bits.windows(1);
    /// assert!(iter.next().is_none());
    /// ```
    #[inline]
    pub fn windows(&self, size: usize) -> Windows<O, T> {
        assert_ne!(size, 0, "Window width cannot be 0");
        Windows::new(self, size)
    }
    /// Returns an iterator over `chunk_size` bits of the slice at a time,
    /// starting at the beginning of the slice.
    ///
    /// The chunks are slices and do not overlap. If `chunk_size` does not
    /// divide the length of the slice, then the last chunk will not have length
    /// `chunk_size`.
    ///
    /// See [`chunks_exact`] for a variant of this iterator that returns chunks
    /// of always exactly `chunk_size` bits, and [`rchunks`] for the same
    /// iterator but starting at the end of the slice.
    ///
    /// # Original
    ///
    /// [`slice::chunks`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks)
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0xA5u8;
    /// let bits = data.view_bits::<Lsb0>();
    /// let mut iter = bits.chunks(3);
    /// assert_eq!(iter.next().unwrap(), &bits[.. 3]);
    /// assert_eq!(iter.next().unwrap(), &bits[3 .. 6]);
    /// assert_eq!(iter.next().unwrap(), &bits[6 ..]);
    /// assert!(iter.next().is_none());
    /// ```
    ///
    /// [`chunks_exact`]: #method.chunks_exact
    /// [`rchunks`]: #method.rchunks
    #[inline]
    pub fn chunks(&self, chunk_size: usize) -> Chunks<O, T> {
        assert_ne!(chunk_size, 0, "Chunk width cannot be 0");
        Chunks::new(self, chunk_size)
    }
    /// Returns an iterator over `chunk_size` bits of the slice at a time,
    /// starting at the beginning of the slice.
    ///
    /// The chunks are mutable slices, and do not overlap. If `chunk_size` does
    /// not divide the length of the slice, then the last chunk will not have
    /// length `chunk_size`.
    ///
    /// See [`chunks_exact_mut`] for a variant of this iterator that returns
    /// chunks of always exactly `chunk_size` bits, and [`rchunks_mut`] for the
    /// same iterator but starting at the end of the slice.
    ///
    /// # Original
    ///
    /// [`slice::chunks_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks_mut)
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Lsb0>();
    ///
    /// for (idx, chunk) in bits.chunks_mut(3).enumerate() {
    ///   chunk.set(2 - idx, true);
    /// }
    /// assert_eq!(data, 0b01_010_100);
    /// ```
    ///
    /// [`chunks_exact_mut`]: #method.chunks_exact_mut
    /// [`rchunks_mut`]: #method.rchunks_mut
    #[inline]
    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<O, T> {
        assert_ne!(chunk_size, 0, "Chunk width cannot be 0");
        ChunksMut::new(self, chunk_size)
    }
    /// Returns an iterator over `chunk_size` bits of the slice at a time,
    /// starting at the beginning of the slice.
    ///
    /// The chunks are slices and do not overlap. If `chunk_size` does not
    /// divide the length of the slice, then the last up to `chunk_size-1` bits
    /// will be omitted and can be retrieved from the `remainder` function of
    /// the iterator.
    ///
    /// Due to each chunk having exactly `chunk_size` bits, the compiler may
    /// optimize the resulting code better than in the case of [`chunks`].
    ///
    /// See [`chunks`] for a variant of this iterator that also returns the
    /// remainder as a smaller chunk, and [`rchunks_exact`] for the same
    /// iterator but starting at the end of the slice.
    ///
    /// # Original
    ///
    /// [`slice::chunks_exact`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks_exact)
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0xA5u8;
    /// let bits = data.view_bits::<Lsb0>();
    /// let mut iter = bits.chunks_exact(3);
    /// assert_eq!(iter.next().unwrap(), &bits[.. 3]);
    /// assert_eq!(iter.next().unwrap(), &bits[3 .. 6]);
    /// assert!(iter.next().is_none());
    /// assert_eq!(iter.remainder(), &bits[6 ..]);
    /// ```
    ///
    /// [`chunks`]: #method.chunks
    /// [`rchunks_exact`]: #method.rchunks_exact
    #[inline]
    pub fn chunks_exact(&self, chunk_size: usize) -> ChunksExact<O, T> {
        assert_ne!(chunk_size, 0, "Chunk width cannot be 0");
        ChunksExact::new(self, chunk_size)
    }
    /// Returns an iterator over `chunk_size` bits of the slice at a time,
    /// starting at the beginning of the slice.
    ///
    /// The chunks are mutable slices, and do not overlap. If `chunk_size` does
    /// not divide the beginning length of the slice, then the last up to
    /// `chunk_size-1` bits will be omitted and can be retrieved from the
    /// `into_remainder` function of the iterator.
    ///
    /// Due to each chunk having exactly `chunk_size` bits, the compiler may
    /// optimize the resulting code better than in the case of [`chunks_mut`].
    ///
    /// See [`chunks_mut`] for a variant of this iterator that also returns the
    /// remainder as a smaller chunk, and [`rchunks_exact_mut`] for the same
    /// iterator but starting at the end of the slice.
    ///
    /// # Original
    ///
    /// [`slice::chunks_exact_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks_exact_mut)
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Lsb0>();
    ///
    /// for (idx, chunk) in bits.chunks_exact_mut(3).enumerate() {
    ///   chunk.set(idx, true);
    /// }
    /// assert_eq!(data, 0b00_010_001);
    /// ```
    ///
    /// [`chunks_mut`]: #method.chunks_mut
    /// [`rchunks_exact_mut`]: #method.rchunks_exact_mut
    #[inline]
    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> ChunksExactMut<O, T> {
        assert_ne!(chunk_size, 0, "Chunk width cannot be 0");
        ChunksExactMut::new(self, chunk_size)
    }
    /// Returns an iterator over `chunk_size` bits of the slice at a time,
    /// starting at the end of the slice.
    ///
    /// The chunks are slices and do not overlap. If `chunk_size` does not
    /// divide the length of the slice, then the last chunk will not have length
    /// `chunk_size`.
    ///
    /// See [`rchunks_exact`] for a variant of this iterator that returns chunks
    /// of always exactly `chunk_size` bits, and [`chunks`] for the same
    /// iterator but starting at the beginning of the slice.
    ///
    /// # Original
    ///
    /// [`slice::rchunks`](https://doc.rust-lang.org/std/primitive.slice.html#method.rchunks)
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0xA5u8;
    /// let bits = data.view_bits::<Lsb0>();
    /// let mut iter = bits.rchunks(3);
    /// assert_eq!(iter.next().unwrap(), &bits[5 ..]);
    /// assert_eq!(iter.next().unwrap(), &bits[2 .. 5]);
    /// assert_eq!(iter.next().unwrap(), &bits[.. 2]);
    /// assert!(iter.next().is_none());
    /// ```
    ///
    /// [`chunks`]: #method.chunks
    /// [`rchunks_exact`]: #method.rchunks_exact
    #[inline]
    pub fn rchunks(&self, chunk_size: usize) -> RChunks<O, T> {
        assert_ne!(chunk_size, 0, "Chunk width cannot be 0");
        RChunks::new(self, chunk_size)
    }
    /// Returns an iterator over `chunk_size` bits of the slice at a time,
    /// starting at the end of the slice.
    ///
    /// The chunks are mutable slices, and do not overlap. If `chunk_size` does
    /// not divide the length of the slice, then the last chunk will not have
    /// length `chunk_size`.
    ///
    /// See [`rchunks_exact_mut`] for a variant of this iterator that returns
    /// chunks of always exactly `chunk_size` bits, and [`chunks_mut`] for the
    /// same iterator but starting at the beginning of the slice.
    ///
    /// # Original
    ///
    /// [`slice::rchunks_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.rchunks_mut)
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Lsb0>();
    ///
    /// for (idx, chunk) in bits.rchunks_mut(3).enumerate() {
    ///   chunk.set(2 - idx, true);
    /// }
    /// assert_eq!(data, 0b100_010_01);
    /// ```
    ///
    /// [`chunks_mut`]: #method.chunks_mut
    /// [`rchunks_exact_mut`]: #method.rchunks_exact_mut
    #[inline]
    pub fn rchunks_mut(&mut self, chunk_size: usize) -> RChunksMut<O, T> {
        assert_ne!(chunk_size, 0, "Chunk width cannot be 0");
        RChunksMut::new(self, chunk_size)
    }
    /// Returns an iterator over `chunk_size` bits of the slice at a time,
    /// starting at the end of the slice.
    ///
    /// The chunks are slices and do not overlap. If `chunk_size` does not
    /// divide the length of the slice, then the last up to `chunk_size-1` bits
    /// will be omitted and can be retrieved from the `remainder` function of
    /// the iterator.
    ///
    /// Due to each chunk having exactly `chunk_size` bits, the compiler can
    /// often optimize the resulting code better than in the case of [`chunks`].
    ///
    /// See [`rchunks`] for a variant of this iterator that also returns the
    /// remainder as a smaller chunk, and [`chunks_exact`] for the same iterator
    /// but starting at the beginning of the slice.
    ///
    /// # Original
    ///
    /// [`slice::rchunks_exact`](https://doc.rust-lang.org/std/primitive.slice.html#method.rchunks_exact)
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0xA5u8;
    /// let bits = data.view_bits::<Lsb0>();
    /// let mut iter = bits.rchunks_exact(3);
    /// assert_eq!(iter.next().unwrap(), &bits[5 ..]);
    /// assert_eq!(iter.next().unwrap(), &bits[2 .. 5]);
    /// assert!(iter.next().is_none());
    /// assert_eq!(iter.remainder(), &bits[.. 2]);
    /// ```
    ///
    /// [`chunks`]: #method.chunks
    /// [`rchunks`]: #method.rchunks
    /// [`chunks_exact`]: #method.chunks_exact
    #[inline]
    pub fn rchunks_exact(&self, chunk_size: usize) -> RChunksExact<O, T> {
        assert_ne!(chunk_size, 0, "Chunk width cannot be 0");
        RChunksExact::new(self, chunk_size)
    }
    /// Returns an iterator over `chunk_size` bits of the slice at a time,
    /// starting at the end of the slice.
    ///
    /// The chunks are mutable slices, and do not overlap. If `chunk_size` does
    /// not divide the length of the slice, then the last up to `chunk_size-1`
    /// bits will be omitted and can be retrieved from the `into_remainder`
    /// function of the iterator.
    ///
    /// Due to each chunk having exactly `chunk_size` bits, the compiler can
    /// often optimize the resulting code better than in the case of
    /// [`chunks_mut`].
    ///
    /// See [`rchunks_mut`] for a variant of this iterator that also returns the
    /// remainder as a smaller chunk, and [`chunks_exact_mut`] for the same
    /// iterator but starting at the beginning of the slice.
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Lsb0>();
    ///
    /// for (idx, chunk) in bits.rchunks_exact_mut(3).enumerate() {
    ///   chunk.set(idx, true);
    /// }
    /// assert_eq!(data, 0b001_010_00);
    /// ```
    ///
    /// [`chunks_mut`]: #method.chunks_mut
    /// [`rchunks_mut`]: #method.rchunks_mut
    /// [`chunks_exact_mut`]: #method.chunks_exact_mut
    #[inline]
    pub fn rchunks_exact_mut(&mut self, chunk_size: usize) -> RChunksExactMut<O, T> {
        assert_ne!(chunk_size, 0, "Chunk width cannot be 0");
        RChunksExactMut::new(self, chunk_size)
    }
    /// Divides one slice into two at an index.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding the index
    /// `mid` itself) and the second will contain all indices from `[mid, len)`
    /// (excluding the index `len` itself).
    ///
    /// # Original
    ///
    /// [`slice::split_at`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at)
    ///
    /// # Panics
    ///
    /// Panics if `mid > len`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0xC3u8;
    /// let bits = data.view_bits::<Local>();
    ///
    /// let (left, right) = bits.split_at(0);
    /// assert!(left.is_empty());
    /// assert_eq!(right, bits);
    ///
    /// let (left, right) = bits.split_at(2);
    /// assert_eq!(left, &bits[.. 2]);
    /// assert_eq!(right, &bits[2 ..]);
    ///
    /// let (left, right) = bits.split_at(8);
    /// assert_eq!(left, bits);
    /// assert!(right.is_empty());
    /// ```
    #[inline]
    pub fn split_at(&self, mid: usize) -> (&Self, &Self) {
        let len = self.len();
        assert!(mid <= len, "Index {} out of bounds: {}", mid, len);
        unsafe { self.split_at_unchecked(mid) }
    }
    /// Divides one mutable slice into two at an index.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding the index
    /// `mid` itself) and the second will contain all indices from `[mid, len)`
    /// (excluding the index `len` itself).
    ///
    /// # Original
    ///
    /// [`slice::split_at_mut`](https://doc.rust-lang.org/std/primitive.html#method.split_at_mut)
    ///
    /// # API Differences
    ///
    /// Because the partition point `mid` is permitted to occur in the interior
    /// of a memory element `T`, this method is required to mark the returned
    /// slices as being to aliased memory. This marking ensures that writes to
    /// the covered memory use the appropriate synchronization behavior of your
    /// build to avoid data races – by default, this makes all writes atomic; on
    /// builds with the `atomic` feature disabled, this uses `Cell`s and
    /// forbids the produced subslices from leaving the current thread.
    ///
    /// See the [`BitStore`] documentation for more information.
    ///
    /// # Panics
    ///
    /// Panics if `mid > len`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// // scoped to restrict the lifetime of the borrows
    /// {
    ///   let (left, right) = bits.split_at_mut(3);
    ///   *left.get_mut(1).unwrap() = true;
    ///   *right.get_mut(2).unwrap() = true;
    /// }
    /// assert_eq!(data, 0b010_00100);
    /// ```
    ///
    /// [`BitStore`]: ../store/trait.BitStore.html
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn split_at_mut(
        &mut self,
        mid: usize,
    ) -> (&mut BitSlice<O, T::Alias>, &mut BitSlice<O, T::Alias>) {
        let len = self.len();
        assert!(mid <= len, "Index {} out of bounds: {}", mid, len);
        unsafe { self.split_at_unchecked_mut(mid) }
    }
    /// Returns an iterator over subslices separated by bits that match `pred`.
    /// The matched bit is not contained in the subslices.
    ///
    /// # Original
    ///
    /// [`slice::split`](https://doc.rust-lang.org/std/primitive.slice.html#method.split)
    ///
    /// # API Differences
    ///
    /// In order to allow more than one bit of information for the split
    /// decision, the predicate receives the index of each bit, as well as its
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0b01_001_000u8;
    /// let bits = data.view_bits::<Msb0>();
    /// let mut iter = bits.split(|_pos, bit| *bit);
    ///
    /// assert_eq!(iter.next().unwrap(), &bits[.. 1]);
    /// assert_eq!(iter.next().unwrap(), &bits[2 .. 4]);
    /// assert_eq!(iter.next().unwrap(), &bits[5 ..]);
    /// assert!(iter.next().is_none());
    /// ```
    ///
    /// If the first bit is matched, an empty slice will be the first item
    /// returned by the iterator. Similarly, if the last element in the slice is
    /// matched, an empty slice will be the last item returned by the iterator:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 1u8;
    /// let bits = data.view_bits::<Msb0>();
    /// let mut iter = bits.split(|_pos, bit| *bit);
    ///
    /// assert_eq!(iter.next().unwrap(), &bits[.. 7]);
    /// assert!(iter.next().unwrap().is_empty());
    /// assert!(iter.next().is_none());
    /// ```
    ///
    /// If two matched bits are directly adjacent, an empty slice will be
    /// present between them:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0b001_100_00u8;
    /// let bits = data.view_bits::<Msb0>();
    /// let mut iter = bits.split(|pos, bit| *bit);
    ///
    /// assert_eq!(iter.next().unwrap(), &bits[0 .. 2]);
    /// assert!(iter.next().unwrap().is_empty());
    /// assert_eq!(iter.next().unwrap(), &bits[4 .. 8]);
    /// assert!(iter.next().is_none());
    /// ```
    #[inline]
    pub fn split<F>(&self, pred: F) -> Split<O, T, F>
    where
        F: FnMut(usize, &bool) -> bool,
    {
        Split::new(self, pred)
    }
    /// Returns an iterator over mutable subslices separated by bits that match
    /// `pred`. The matched bit is not contained in the subslices.
    ///
    /// # Original
    ///
    /// [`slice::split_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_mut)
    ///
    /// # API Differences
    ///
    /// In order to allow more than one bit of information for the split
    /// decision, the predicate receives the index of each bit, as well as its
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0b001_000_10u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    ///
    /// for group in bits.split_mut(|_pos, bit| *bit) {
    ///   *group.get_mut(0).unwrap() = true;
    /// }
    /// assert_eq!(data, 0b101_100_11);
    /// ```
    #[inline]
    pub fn split_mut<F>(&mut self, pred: F) -> SplitMut<O, T, F>
    where
        F: FnMut(usize, &bool) -> bool,
    {
        SplitMut::new(self.alias_mut(), pred)
    }
    /// Returns an iterator over subslices separated by bits that match `pred`,
    /// starting at the end of the slice and working backwards. The matched bit
    /// is not contained in the subslices.
    ///
    /// # Original
    ///
    /// [`slice::rsplit`](https://doc.rust-lang.org/std/primitive.slice.html#method.rsplit)
    ///
    /// # API Differences
    ///
    /// In order to allow more than one bit of information for the split
    /// decision, the predicate receives the index of each bit, as well as its
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0b0001_0000u8;
    /// let bits = data.view_bits::<Msb0>();
    /// let mut iter = bits.rsplit(|_pos, bit| *bit);
    ///
    /// assert_eq!(iter.next().unwrap(), &bits[4 ..]);
    /// assert_eq!(iter.next().unwrap(), &bits[.. 3]);
    /// assert!(iter.next().is_none());
    /// ```
    ///
    /// As with `split()`, if the first or last bit is matched, an empty slice
    /// will be the first (or last) item returned by the iterator.
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0b1001_0001u8;
    /// let bits = data.view_bits::<Msb0>();
    /// let mut iter = bits.rsplit(|_pos, bit| *bit);
    /// assert!(iter.next().unwrap().is_empty());
    /// assert_eq!(iter.next().unwrap(), &bits[4 .. 7]);
    /// assert_eq!(iter.next().unwrap(), &bits[1 .. 3]);
    /// assert!(iter.next().unwrap().is_empty());
    /// assert!(iter.next().is_none());
    /// ```
    #[inline]
    pub fn rsplit<F>(&self, pred: F) -> RSplit<O, T, F>
    where
        F: FnMut(usize, &bool) -> bool,
    {
        RSplit::new(self, pred)
    }
    /// Returns an iterator over mutable subslices separated by bits that match
    /// `pred`, starting at the end of the slice and working backwards. The
    /// matched bit is not contained in the subslices.
    ///
    /// # Original
    ///
    /// [`slice::rsplit_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.rsplit_mut)
    ///
    /// # API Differences
    ///
    /// In order to allow more than one bit of information for the split
    /// decision, the predicate receives the index of each bit, as well as its
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0b001_000_10u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    ///
    /// for group in bits.rsplit_mut(|_pos, bit| *bit) {
    ///   *group.get_mut(0).unwrap() = true;
    /// }
    /// assert_eq!(data, 0b101_100_11);
    /// ```
    #[inline]
    pub fn rsplit_mut<F>(&mut self, pred: F) -> RSplitMut<O, T, F>
    where
        F: FnMut(usize, &bool) -> bool,
    {
        RSplitMut::new(self.alias_mut(), pred)
    }
    /// Returns an iterator over subslices separated by bits that match `pred`,
    /// limited to returning at most `n` items. The matched bit is not contained
    /// in the subslices.
    ///
    /// The last item returned, if any, will contain the remainder of the slice.
    ///
    /// # Original
    ///
    /// [`slice::splitn`](https://doc.rust-lang.org/std/primitive.slice.html#method.splitn)
    ///
    /// # API Differences
    ///
    /// In order to allow more than one bit of information for the split
    /// decision, the predicate receives the index of each bit, as well as its
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0xA5u8;
    /// let bits = data.view_bits::<Msb0>();
    ///
    /// for group in bits.splitn(2, |pos, _bit| pos % 3 == 2) {
    /// # #[cfg(feature = "std")] {
    ///   println!("{}", group.len());
    /// # }
    /// }
    /// //  2
    /// //  5
    /// # //  [10]
    /// # //  [00101]
    /// ```
    #[inline]
    pub fn splitn<F>(&self, n: usize, pred: F) -> SplitN<O, T, F>
    where
        F: FnMut(usize, &bool) -> bool,
    {
        SplitN::new(self, pred, n)
    }
    /// Returns an iterator over subslices separated by bits that match `pred`,
    /// limited to returning at most `n` items. The matched element is not
    /// contained in the subslices.
    ///
    /// The last item returned, if any, will contain the remainder of the slice.
    ///
    /// # Original
    ///
    /// [`slice::splitn_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.splitn_mut)
    ///
    /// # API Differences
    ///
    /// In order to allow more than one bit of information for the split
    /// decision, the predicate receives the index of each bit, as well as its
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0b001_000_10u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    ///
    /// for group in bits.splitn_mut(2, |_pos, bit| *bit) {
    ///   *group.get_mut(0).unwrap() = true;
    /// }
    /// assert_eq!(data, 0b101_100_10);
    /// ```
    #[inline]
    pub fn splitn_mut<F>(&mut self, n: usize, pred: F) -> SplitNMut<O, T, F>
    where
        F: FnMut(usize, &bool) -> bool,
    {
        SplitNMut::new(self.alias_mut(), pred, n)
    }
    /// Returns an iterator over subslices separated by bits that match `pred`
    /// limited to returining at most `n` items. This starts at the end of the
    /// slice and works backwards. The matched bit is not contained in the
    /// subslices.
    ///
    /// The last item returned, if any, will contain the remainder of the slice.
    ///
    /// # Original
    ///
    /// [`slice::rsplitn`](https://doc.rust-lang.org/std/primitive.slice.html#method.rsplitn)
    ///
    /// # API Differences
    ///
    /// In order to allow more than one bit of information for the split
    /// decision, the predicate receives the index of each bit, as well as its
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0xA5u8;
    /// let bits = data.view_bits::<Msb0>();
    ///
    /// for group in bits.rsplitn(2, |pos, _bit| pos % 3 == 2) {
    /// # #[cfg(feature = "std")] {
    ///   println!("{}", group.len());
    /// # }
    /// }
    /// //  2
    /// //  5
    /// # //  [10]
    /// # //  [00101]
    /// ```
    #[inline]
    pub fn rsplitn<F>(&self, n: usize, pred: F) -> RSplitN<O, T, F>
    where
        F: FnMut(usize, &bool) -> bool,
    {
        RSplitN::new(self, pred, n)
    }
    /// Returns an iterator over subslices separated by bits that match `pred`
    /// limited to returning at most `n` items. This starts at the end of the
    /// slice and works backwards. The matched bit is not contained in the
    /// subslices.
    ///
    /// The last item returned, if any, will contain the remainder of the slice.
    ///
    /// # Original
    ///
    /// [`slice::rsplitn_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.rsplitn_mut)
    ///
    /// # API Differences
    ///
    /// In order to allow more than one bit of information for the split
    /// decision, the predicate receives the index of each bit, as well as its
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0b001_000_10u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    ///
    /// for group in bits.rsplitn_mut(2, |_pos, bit| *bit) {
    ///   *group.get_mut(0).unwrap() = true;
    /// }
    /// assert_eq!(data, 0b101_000_11);
    /// ```
    #[inline]
    pub fn rsplitn_mut<F>(&mut self, n: usize, pred: F) -> RSplitNMut<O, T, F>
    where
        F: FnMut(usize, &bool) -> bool,
    {
        RSplitNMut::new(self.alias_mut(), pred, n)
    }
    /// Returns `true` if the slice contains a subslice that matches the given
    /// span.
    ///
    /// # Original
    ///
    /// [`slice::contains`](https://doc.rust-lang.org/std/primitive.slice.html#method.contains)
    ///
    /// # API Differences
    ///
    /// This searches for a matching subslice (allowing different type
    /// parameters) rather than for a specific bit. Searching for a contained
    /// element with a given value is not as useful on a collection of `bool`.
    ///
    /// Furthermore, `BitSlice` defines [`any`] and [`not_all`], which are
    /// optimized searchers for any `true` or `false` bit, respectively, in a
    /// sequence.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0b0101_1010u8;
    /// let bits_msb = data.view_bits::<Msb0>();
    /// let bits_lsb = data.view_bits::<Lsb0>();
    /// assert!(bits_msb.contains(&bits_lsb[1 .. 5]));
    /// ```
    ///
    /// This example uses a palindrome pattern to demonstrate that the slice
    /// being searched for does not need to have the same type parameters as the
    /// slice being searched.
    ///
    /// [`any`]: #method.any
    /// [`not_all`]: #method.not_all
    #[inline]
    pub fn contains<O2, T2>(&self, x: &BitSlice<O2, T2>) -> bool
    where
        O2: BitOrder,
        T2: BitStore,
    {
        let len = x.len();
        if len > self.len() {
            return false;
        }
        self.windows(len).any(|s| s == x)
    }
    /// Returns `true` if `needle` is a prefix of the slice.
    ///
    /// # Original
    ///
    /// [`slice::starts_with`](https://doc.rust-lang.org/std/primitive.slice.html#method.starts_with)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0b0100_1011u8;
    /// let haystack = data.view_bits::<Msb0>();
    /// let needle = &data.view_bits::<Lsb0>()[2 .. 5];
    /// assert!(haystack.starts_with(&needle[.. 2]));
    /// assert!(haystack.starts_with(needle));
    /// assert!(!haystack.starts_with(&haystack[2 .. 4]));
    /// ```
    ///
    /// Always returns `true` if `needle` is an empty slice:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let empty = BitSlice::<Local, usize>::empty();
    /// assert!(0u8.view_bits::<Local>().starts_with(empty));
    /// assert!(empty.starts_with(empty));
    /// ```
    #[inline]
    pub fn starts_with<O2, T2>(&self, needle: &BitSlice<O2, T2>) -> bool
    where
        O2: BitOrder,
        T2: BitStore,
    {
        let len = needle.len();
        self.len() >= len && needle == unsafe { self.get_unchecked(..len) }
    }
    /// Returns `true` if `needle` is a suffix of the slice.
    ///
    /// # Original
    ///
    /// [`slice::ends_with`](https://doc.rust-lang.org/std/primitive.slice.html#method.ends_with)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let data = 0b0100_1011u8;
    /// let haystack = data.view_bits::<Lsb0>();
    /// let needle = &data.view_bits::<Msb0>()[3 .. 6];
    /// assert!(haystack.ends_with(&needle[1 ..]));
    /// assert!(haystack.ends_with(needle));
    /// assert!(!haystack.ends_with(&haystack[2 .. 4]));
    /// ```
    ///
    /// Always returns `true` if `needle` is an empty slice:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let empty = BitSlice::<Local, usize>::empty();
    /// assert!(0u8.view_bits::<Local>().ends_with(empty));
    /// assert!(empty.ends_with(empty));
    /// ```
    #[inline]
    pub fn ends_with<O2, T2>(&self, needle: &BitSlice<O2, T2>) -> bool
    where
        O2: BitOrder,
        T2: BitStore,
    {
        let nlen = needle.len();
        let len = self.len();
        len >= nlen && needle == unsafe { self.get_unchecked(len - nlen..) }
    }
    /// Rotates the slice in-place such that the first `by` bits of the slice
    /// move to the end while the last `self.len() - by` bits move to the front.
    /// After calling `rotate_left`, the bit previously at index `by` will
    /// become the first bit in the slice.
    ///
    /// # Original
    ///
    /// [`slice::rotate_left`](https://doc.rust-lang.org/std/primitive.slice.html#rotate_left)
    ///
    /// # Panics
    ///
    /// This function will panic if `by` is greater than the length of the
    /// slice. Note that `by == self.len()` does *not* panic and is a no-op
    /// rotation.
    ///
    /// # Complexity
    ///
    /// Takes linear (in `self.len()`) time.
    ///
    /// # Performance
    ///
    /// While this is faster than the equivalent rotation on `[bool]`, it is
    /// slower than a handcrafted partial-element rotation on `[T]`. Because of
    /// the support for custom orderings, and the lack of specialization, this
    /// method can only accelerate by reducing the number of loop iterations
    /// performed on the slice body, and cannot accelerate by using shift-mask
    /// instructions to move multiple bits in one operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    /// let mut data = 0xF0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// bits.rotate_left(2);
    /// assert_eq!(data, 0xC3);
    /// ```
    ///
    /// Rotating a subslice:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0xF0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// bits[1 .. 5].rotate_left(1);
    /// assert_eq!(data, 0b1_1101_000);
    /// ```
    #[inline]
    pub fn rotate_left(&mut self, mut by: usize) {
        let len = self.len();
        assert!(by <= len, "Slices cannot be rotated by more than their length");
        if by == 0 || by == len {
            return;
        }
        let mut tmp = 0usize;
        let tmp_bits = BitSlice::<O, _>::from_element_mut(&mut tmp);
        while by > 0 {
            let shamt = cmp::min(usize::BITS as usize, by);
            unsafe {
                let tmp_bits = tmp_bits.get_unchecked_mut(..shamt);
                tmp_bits.clone_from_bitslice(self.get_unchecked(..shamt));
                self.copy_within_unchecked(shamt.., 0);
                self.get_unchecked_mut(len - shamt..).clone_from_bitslice(tmp_bits);
            }
            by -= shamt;
        }
    }
    /// Rotates the slice in-place such that the first `self.len() - by` bits of
    /// the slice move to the end while the last `by` bits move to the front.
    /// After calling `rotate_right`, the bit previously at index `self.len() -
    /// by` will become the first bit in the slice.
    ///
    /// # Original
    ///
    /// [`slice::rotate_right`](https://doc.rust-lang.org/std/primitive.slice.html#rotate_right)
    ///
    /// # Panics
    ///
    /// This function will panic if `by` is greater than the length of the
    /// slice. Note that `by == self.len()` does *not* panic and is a no-op
    /// rotation.
    ///
    /// # Complexity
    ///
    /// Takes linear (in `self.len()`) time.
    ///
    /// # Performance
    ///
    /// While this is faster than the equivalent rotation on `[bool]`, it is
    /// slower than a handcrafted partial-element rotation on `[T]`. Because of
    /// the support for custom orderings, and the lack of specialization, this
    /// method can only accelerate by reducing the number of loop iterations
    /// performed on the slice body, and cannot accelerate by using shift-mask
    /// instructions to move multiple bits in one operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0xF0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// bits.rotate_right(2);
    /// assert_eq!(data, 0x3C);
    /// ```
    ///
    /// Rotate a subslice:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0xF0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// bits[1 .. 5].rotate_right(1);
    /// assert_eq!(data, 0b1_0111_000);
    /// ```
    #[inline]
    pub fn rotate_right(&mut self, mut by: usize) {
        let len = self.len();
        assert!(by <= len, "Slices cannot be rotated by more than their length");
        if by == 0 || by == len {
            return;
        }
        let mut tmp = 0usize;
        let tmp_bits = BitSlice::<O, _>::from_element_mut(&mut tmp);
        while by > 0 {
            let shamt = cmp::min(usize::BITS as usize, by);
            let mid = len - shamt;
            unsafe {
                let tmp_bits = tmp_bits.get_unchecked_mut(..shamt);
                tmp_bits.clone_from_bitslice(self.get_unchecked(mid..));
                self.copy_within_unchecked(..mid, shamt);
                self.get_unchecked_mut(..shamt).clone_from_bitslice(tmp_bits);
            }
            by -= shamt;
        }
    }
    /// Copies the bits from `src` into `self`.
    ///
    /// The length of `src` must be the same as `self`.
    ///
    /// # Original
    ///
    /// [`slice::clone_from_slice`](https://doc.rust-lang.org/std/primitive.slice.html#method.clone_from_slice)
    ///
    /// # API Differences
    ///
    /// This method is renamed, as it takes a bit slice rather than an element
    /// slice.
    ///
    /// # Panics
    ///
    /// This function will panic if the two slices have different lengths.
    ///
    /// # Examples
    ///
    /// Cloning two bits from a slice into another:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// let src = 0x0Fu16.view_bits::<Lsb0>();
    /// bits[.. 2].clone_from_bitslice(&src[2 .. 4]);
    /// assert_eq!(data, 0xC0);
    /// ```
    ///
    /// Rust enforces that there can only be one mutable reference with no
    /// immutable references to a particular piece of data in a particular
    /// scope. Because of this, attempting to use `clone_from_bitslice` on a
    /// single slice will result in a compile failure:
    ///
    /// ```rust,compile_fail
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 3u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// bits[.. 2].clone_from_bitslice(&bits[6 ..]);
    /// ```
    ///
    /// To work around this, we can use [`split_at_mut`] to create two distinct
    /// sub-slices from a slice:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 3u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    /// let (head, tail) = bits.split_at_mut(4);
    /// head.clone_from_bitslice(tail);
    /// assert_eq!(data, 0x33);
    /// ```
    ///
    /// [`split_at_mut`]: #method.split_at_mut
    #[inline]
    pub fn clone_from_bitslice<O2, T2>(&mut self, src: &BitSlice<O2, T2>)
    where
        O2: BitOrder,
        T2: BitStore,
    {
        let len = self.len();
        assert_eq!(len, src.len(), "Cloning from slice requires equal lengths",);
        for idx in 0..len {
            unsafe {
                self.set_unchecked(idx, *src.get_unchecked(idx));
            }
        }
    }
    #[inline]
    #[doc(hidden)]
    #[deprecated(note = "Use `.clone_from_bitslice` to copy between bitslices")]
    #[cfg(not(tarpaulin_include))]
    pub fn clone_from_slice<O2, T2>(&mut self, src: &BitSlice<O2, T2>)
    where
        O2: BitOrder,
        T2: BitStore,
    {
        self.clone_from_bitslice(src)
    }
    /// Copies all bits from `src` into `self`.
    ///
    /// The length of `src` must be the same as `self`.
    ///
    /// # Original
    ///
    /// [`slice::copy_from_slice`](https://doc.rust-lang.org/std/primitive.std.html#method.copy_from_slice)
    ///
    /// # API Differences
    ///
    /// This method is renamed, as it takes a bit slice rather than an element
    /// slice.
    ///
    /// This is unable to guarantee a strictly faster copy behavior than
    /// [`clone_from_bitslice`]. In the future, the implementation *may*
    /// specialize, as the language allows.
    ///
    /// # Panics
    ///
    /// This function will panic if the two slices have different lengths.
    ///
    /// # Examples
    ///
    /// Copying two bits from a slice into another:
    ///
    /// [`clone_from_bitslice`]: #method.clone_from_bitslice
    #[inline(always)]
    #[cfg(not(tarpaulin_include))]
    pub fn copy_from_bitslice(&mut self, src: &Self) {
        self.clone_from_bitslice(src);
    }
    #[inline]
    #[doc(hidden)]
    #[deprecated(note = "Use `.copy_from_bitslice` to copy between bitslices")]
    #[cfg(not(tarpaulin_include))]
    pub fn copy_from_slice(&mut self, src: &Self) {
        self.copy_from_bitslice(src)
    }
    /// Copies bits from one part of the slice to another part of itself.
    ///
    /// `src` is the range within `self` to copy from. `dest` is the starting
    /// index of the range within `self` to copy to, which will have the same
    /// length as `src`. The two ranges may overlap. The ends of the two ranges
    /// must be less than or equal to `self.len()`.
    ///
    /// # Original
    ///
    /// [`slice::copy_within`](https://doc.rust-lang.org/std/primitive.slice.html#method.copy_within)
    ///
    /// # Panics
    ///
    /// This function will panic if either range exceeds the end of the slice,
    /// or if the end of `src` is before the start.
    ///
    /// # Examples
    ///
    /// Copying four bytes within a slice:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut data = 0x07u8;
    /// let bits = data.view_bits_mut::<Msb0>();
    ///
    /// bits.copy_within(5 .., 0);
    ///
    /// assert_eq!(data, 0xE7);
    /// ```
    #[inline]
    pub fn copy_within<R>(&mut self, src: R, dest: usize)
    where
        R: RangeBounds<usize>,
    {
        let len = self.len();
        let src = dvl::normalize_range(src, len);
        dvl::assert_range(src.clone(), len);
        dvl::assert_range(dest..dest + (src.end - src.start), len);
        unsafe {
            self.copy_within_unchecked(src, dest);
        }
    }
    /// Swaps all bits in `self` with those in `other`.
    ///
    /// The length of `other` must be the same as `self`.
    ///
    /// # Original
    ///
    /// [`slice::swap_with_slice`](https://doc.rust-lang.org/std/primitive.slice.html#method.swap_with_slice)
    ///
    /// # API Differences
    ///
    /// This method is renamed, as it takes a bit slice rather than an element
    /// slice.
    ///
    /// # Panics
    ///
    /// This function will panic if the two slices have different lengths.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// let mut one = [0xA5u8, 0x69];
    /// let mut two = 0x1234u16;
    /// let one_bits = one.view_bits_mut::<Msb0>();
    /// let two_bits = two.view_bits_mut::<Lsb0>();
    ///
    /// one_bits.swap_with_bitslice(two_bits);
    ///
    /// assert_eq!(one, [0x2C, 0x48]);
    /// # #[cfg(target_endian = "little")] {
    /// assert_eq!(two, 0x96A5);
    /// # }
    /// ```
    #[inline]
    pub fn swap_with_bitslice<O2, T2>(&mut self, other: &mut BitSlice<O2, T2>)
    where
        O2: BitOrder,
        T2: BitStore,
    {
        let len = self.len();
        assert_eq!(len, other.len());
        for n in 0..len {
            unsafe {
                let (this, that) = (*self.get_unchecked(n), *other.get_unchecked(n));
                self.set_unchecked(n, that);
                other.set_unchecked(n, this);
            }
        }
    }
    #[inline]
    #[doc(hidden)]
    #[deprecated(note = "Use `.swap_with_bitslice` to swap between bitslices")]
    #[cfg(not(tarpaulin_include))]
    pub fn swap_with_slice<O2, T2>(&mut self, other: &mut BitSlice<O2, T2>)
    where
        O2: BitOrder,
        T2: BitStore,
    {
        self.swap_with_bitslice(other);
    }
    /// Transmute the bitslice to a bitslice of another type, ensuring alignment
    /// of the types is maintained.
    ///
    /// This method splits the bitslice into three distinct bitslices: prefix,
    /// correctly aligned middle bitslice of a new type, and the suffix
    /// bitslice. The method may make the middle bitslice the greatest
    /// length possible for a given type and input bitslice, but only your
    /// algorithm's performance should depend on that, not its correctness. It
    /// is permissible for all of the input data to be returned as the prefix or
    /// suffix bitslice.
    ///
    /// # Original
    ///
    /// [`slice::align_to`](https://doc.rust-lang.org/std/primitive.slice.html#method.align_to)
    ///
    /// # API Differences
    ///
    /// Type `U` is **required** to have the same type family as type `T`.
    /// Whatever `T` is of the fundamental integers, atomics, or `Cell`
    /// wrappers, `U` must be a different width in the same family. Changing the
    /// type family with this method is **unsound** and strictly forbidden.
    /// Unfortunately, it cannot be guaranteed by this function, so you are
    /// required to abide by this limitation.
    ///
    /// # Safety
    ///
    /// This method is essentially a `transmute` with respect to the elements in
    /// the returned middle bitslice, so all the usual caveats pertaining to
    /// `transmute::<T, U>` also apply here.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// unsafe {
    ///   let bytes: [u8; 7] = [1, 2, 3, 4, 5, 6, 7];
    ///   let bits = bytes.view_bits::<Local>();
    ///   let (prefix, shorts, suffix) = bits.align_to::<u16>();
    ///   match prefix.len() {
    ///     0 => {
    ///       assert_eq!(shorts, bits[.. 48]);
    ///       assert_eq!(suffix, bits[48 ..]);
    ///     },
    ///     8 => {
    ///       assert_eq!(prefix, bits[.. 8]);
    ///       assert_eq!(shorts, bits[8 ..]);
    ///     },
    ///     _ => unreachable!("This case will not occur")
    ///   }
    /// }
    /// ```
    #[inline]
    pub unsafe fn align_to<U>(&self) -> (&Self, &BitSlice<O, U>, &Self)
    where
        U: BitStore,
    {
        let bitptr = self.bitptr();
        let bp_len = bitptr.len();
        let (l, c, r) = bitptr.as_aliased_slice().align_to::<U::Alias>();
        let l_start = bitptr.head().value() as usize;
        let mut l = BitSlice::<O, T::Alias>::from_aliased_slice_unchecked(l);
        if l.len() > l_start {
            l = l.get_unchecked(l_start..);
        }
        let mut c = BitSlice::<O, U::Alias>::from_aliased_slice_unchecked(c);
        let c_len = cmp::min(c.len(), bp_len - l.len());
        c = c.get_unchecked(..c_len);
        let mut r = BitSlice::<O, T::Alias>::from_aliased_slice_unchecked(r);
        let r_len = bp_len - l.len() - c.len();
        if r.len() > r_len {
            r = r.get_unchecked(..r_len);
        }
        (
            l.bitptr().pipe(dvl::remove_bitptr_alias::<T>).to_bitslice_ref(),
            c.bitptr().pipe(dvl::remove_bitptr_alias::<U>).to_bitslice_ref(),
            r.bitptr().pipe(dvl::remove_bitptr_alias::<T>).to_bitslice_ref(),
        )
    }
    /// Transmute the bitslice to a bitslice of another type, ensuring alignment
    /// of the types is maintained.
    ///
    /// This method splits the bitslice into three distinct bitslices: prefix,
    /// correctly aligned middle bitslice of a new type, and the suffix
    /// bitslice. The method may make the middle bitslice the greatest
    /// length possible for a given type and input bitslice, but only your
    /// algorithm's performance should depend on that, not its correctness. It
    /// is permissible for all of the input data to be returned as the prefix or
    /// suffix bitslice.
    ///
    /// # Original
    ///
    /// [`slice::align_to`](https://doc.rust-lang.org/std/primitive.slice.html#method.align_to)
    ///
    /// # API Differences
    ///
    /// Type `U` is **required** to have the same type family as type `T`.
    /// Whatever `T` is of the fundamental integers, atomics, or `Cell`
    /// wrappers, `U` must be a different width in the same family. Changing the
    /// type family with this method is **unsound** and strictly forbidden.
    /// Unfortunately, it cannot be guaranteed by this function, so you are
    /// required to abide by this limitation.
    ///
    /// # Safety
    ///
    /// This method is essentially a `transmute` with respect to the elements in
    /// the returned middle bitslice, so all the usual caveats pertaining to
    /// `transmute::<T, U>` also apply here.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// unsafe {
    ///   let mut bytes: [u8; 7] = [1, 2, 3, 4, 5, 6, 7];
    ///   let bits = bytes.view_bits_mut::<Local>();
    ///   let (prefix, shorts, suffix) = bits.align_to_mut::<u16>();
    ///   //  same access and behavior as in `align_to`
    /// }
    /// ```
    #[inline]
    pub unsafe fn align_to_mut<U>(
        &mut self,
    ) -> (&mut Self, &mut BitSlice<O, U>, &mut Self)
    where
        U: BitStore,
    {
        let (l, c, r) = self.align_to::<U>();
        (
            l.bitptr().to_bitslice_mut(),
            c.bitptr().to_bitslice_mut(),
            r.bitptr().to_bitslice_mut(),
        )
    }
}
/// These functions only exist when `BitVec` does.
#[cfg(feature = "alloc")]
impl<O, T> BitSlice<O, T>
where
    O: BitOrder,
    T: BitStore,
{
    /// Copies `self` into a new `BitVec`.
    ///
    /// # Original
    ///
    /// [`slice::to_vec`](https://doc.rust-lang.org/std.primitive.html#method.to_vec)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "stde")] {
    /// use bitvec::prelude::*;
    ///
    /// let bits = bits![0, 1, 0, 1];
    /// let bv = bits.to_bitvec();
    /// assert_eq!(bits, bv);
    /// # }
    /// ```
    #[inline]
    pub fn to_bitvec(&self) -> BitVec<O, T> {
        self.pipe(BitVec::from_bitslice)
    }
    #[doc(hidden)]
    #[deprecated(note = "Use `.to_bitvec` to convert a bit slice into a vector")]
    pub fn to_vec(&self) -> BitVec<O, T> {
        self.to_bitvec()
    }
    /// Creates a vector by repeating a slice `n` times.
    ///
    /// # Original
    ///
    /// [`slice::repeat`](https://doc.rust-lang.org/std/primitive.slice.html#method.repeat)
    ///
    /// # Panics
    ///
    /// This function will panic if the capacity would overflow.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// use bitvec::prelude::*;
    ///
    /// assert_eq!(bits![0, 1].repeat(3), bits![0, 1, 0, 1, 0, 1]);
    /// ```
    ///
    /// A panic upon overflow:
    ///
    /// ```rust,should_panic
    /// use bitvec::prelude::*;
    ///
    /// // this will panic at runtime
    /// bits![0, 1].repeat(BitSlice::<Local, usize>::MAX_BITS);
    /// ```
    #[inline]
    pub fn repeat(&self, n: usize) -> BitVec<O, T>
    where
        O: BitOrder,
        T: BitStore,
    {
        let len = self.len();
        let total = len.checked_mul(n).expect("capacity overflow");
        let mut out = BitVec::with_capacity(total);
        for span in (0..n).map(|rep| rep * len..(rep + 1) * len) {
            unsafe { out.get_unchecked_mut(span) }.clone_from_bitslice(self);
        }
        unsafe {
            out.set_len(total);
        }
        out
    }
}
/** Converts a reference to `T` into a bitslice over one element.

# Original

[`slice::from_ref`](https://doc.rust-lang.org/core/slice/fn.from_ref.html)
**/
#[inline(always)]
pub fn from_ref<O, T>(elem: &T) -> &BitSlice<O, T>
where
    O: BitOrder,
    T: BitStore + BitMemory,
{
    BitSlice::from_element(elem)
}
/** Converts a reference to `T` into a bitslice over one element.

# Original

[`slice::from_mut`](https://doc.rust-lang.org/core/slice/fn.from_mut.html)
**/
#[inline(always)]
pub fn from_mut<O, T>(elem: &mut T) -> &mut BitSlice<O, T>
where
    O: BitOrder,
    T: BitStore + BitMemory,
{
    BitSlice::from_element_mut(elem)
}
/// Forms a bitslice from a pointer and a length.
///
/// The `len` argument is the number of **elements**, not the number of bits.
///
/// # Original
///
/// [`slice::from_raw_parts`](https://doc.rust-lang.org/core/slice/fn.from_raw_parts.html)
///
/// # Safety
///
/// Behavior is undefined if any of the following conditions are violated:
///
/// - `data` must be [valid] for `len * mem::size_of::<T>()` many bytes, and it
///   must be properly aligned. This means in particular:
///   - The entire memory range of this slice must be contained within a single
///     allocated object! Slices can never span across multiple allocated
///     objects.
///   - `data` must be non-null and aligned even for zero-length slices. The
///     `&BitSlice` pointer encoding requires this porperty to hold. You can
///     obtain a pointer that is usable as `data` for zero-length slices using
///     [`NonNull::dangling()`].
/// - The memory referenced by the returned bitslice must not be mutated for the
///   duration of the lifetime `'a`, except inside an `UnsafeCell`.
/// - The total size `len * T::Mem::BITS` of the slice must be no larger than
///   [`BitSlice::<_, T>::MAX_BITS`]
///
/// # Caveat
///
/// The lifetime for the returned slice is inferred from its usage. To prevent
/// accidental misuse, it's suggested to tie the lifetime to whichever source
/// lifetime is safe in the context, such as by providing a helper function
/// taking the lifetime of a host value for the slice, or by explicit
/// annotation.
///
/// # Examples
///
/// ```rust
/// use bitvec::prelude::*;
/// use bitvec::slice as bv_slice;
///
/// let x = 42u8;
/// let ptr = &x as *const _;
/// let bits = unsafe {
///   bv_slice::from_raw_parts::<Local, u8>(ptr, 1)
/// };
/// assert_eq!(bits.count_ones(), 3);
/// ```
///
/// [valid]: https://doc.rust-lang.org/core/ptr/index.html#safety
/// [`BitSlice::<_, T>::MAX_BITS`]:
/// struct.BitSlice.html#associatedconstant.MAX_BITS [`NonNull::dangling()`]: https://doc.rust-lang.org/core/ptr/struct.NonNull.html#method.dangling
#[inline]
pub unsafe fn from_raw_parts<'a, O, T>(data: *const T, len: usize) -> &'a BitSlice<O, T>
where
    O: BitOrder,
    T: 'a + BitStore + BitMemory,
{
    super::bits_from_raw_parts(data, 0, len * T::Mem::BITS as usize)
        .unwrap_or_else(|| {
            panic!(
                "Failed to construct `&{}BitSlice` from invalid pointer {:p} \
				 or element count {}",
                "", data, len
            )
        })
}
/// Performs the same functionality as [`from_raw_parts`], except that a mutable
/// bitslice is returned.
///
/// # Original
///
/// [`slice::from_raw_parts_mut`](https://doc.rust-lang.org/core/slice/fn.from_raw_parts_mut.html)
///
/// # Safety
///
/// Behavior is undefined if any of the following conditions are violated:
///
/// - `data` must be [valid] for `len * mem::size_of::<T>()` many bytes, and it
///   must be properly aligned. This means in particular:
///   - The entire memory range of this slice must be contained within a single
///     allocated object! Slices can never span across multiple allocated
///     objects.
///   - `data` must be non-null and aligned even for zero-length slices. The
///     `&BitSlice` pointer encoding requires this porperty to hold. You can
///     obtain a pointer that is usable as `data` for zero-length slices using
///     [`NonNull::dangling()`].
/// - The memory referenced by the returned bitslice must not be accessed
///   through other pointer (not derived from the return value) for the duration
///   of the lifetime `'a`. Both read and write accesses are forbidden.
/// - The total size `len * T::Mem::BITS` of the slice must be no larger than
///   [`BitSlice::<_, T>::MAX_BITS`]
///
/// [valid]: https://doc.rust-lang.org/core/ptr/index.html#safety
/// [`BitSlice::<_, T>::MAX_BITS`]:
/// struct.BitSlice.html#associatedconstant.MAX_BITS
/// [`NonNull::dangling()`]: https://doc.rust-lang.org/core/ptr/struct.NonNull.html#method.dangling
#[inline]
pub unsafe fn from_raw_parts_mut<'a, O, T>(
    data: *mut T,
    len: usize,
) -> &'a mut BitSlice<O, T>
where
    O: BitOrder,
    T: 'a + BitStore + BitMemory,
{
    super::bits_from_raw_parts_mut(data, 0, len * T::Mem::BITS as usize)
        .unwrap_or_else(|| {
            panic!(
                "Failed to construct `&{}BitSlice` from invalid pointer {:p} \
				 or element count {}",
                "mut ", data, len
            )
        })
}
/** A helper trait used for indexing operations.

This trait has its definition stabilized, but has not stabilized its associated
functions. This means it cannot be implemented outside of the distribution
libraries. *Furthermore*, since `bitvec` cannot create `&mut bool` references,
it is insufficient for `bitvec`’s uses.

There is no tracking issue for `feature(slice_index_methods)`.

# Original

[`slice::SliceIndex`](https://doc.rust-lang.org/stable/core/slice/trait.SliceIndex.html)

# API Differences

`SliceIndex::Output` is not usable here, because the `usize` implementation
cannot produce `&mut bool`. Instead, two output types `Immut` and `Mut` are
defined. The range implementations define these to be the appropriately mutable
`BitSlice` reference; the `usize` implementation defines them to be `&bool` and
the proxy type.
**/
pub trait BitSliceIndex<'a, O, T>
where
    O: 'a + BitOrder,
    T: 'a + BitStore,
{
    /// The output type for immutable functions.
    type Immut;
    /// The output type for mutable functions.
    type Mut;
    /// Returns a shared reference to the output at this location, if in bounds.
    ///
    /// # Original
    ///
    /// [`SliceIndex::get`](https://doc.rust-lang.org/core/slice/trait.SliceIndex.html#method.get)
    fn get(self, slice: &'a BitSlice<O, T>) -> Option<Self::Immut>;
    /// Returns a mutable reference to the output at this location, if in
    /// bounds.
    ///
    /// # Original
    ///
    /// [`SliceIndex::get_mut`](https://doc.rust-lang.org/core/slice/trait.SliceIndex.html#method.get_mut)
    fn get_mut(self, slice: &'a mut BitSlice<O, T>) -> Option<Self::Mut>;
    /// Returns a shared reference to the output at this location, without
    /// performing any bounds checking. Calling this method with an
    /// out-of-bounds index is [undefined behavior] even if the resulting
    /// reference is not used.
    ///
    /// # Original
    ///
    /// [`SliceIndex::get_unchecked`](https://doc.rust-lang.org/core/slice/trait.SliceIndex.html#method.get_unchecked)
    ///
    /// # Safety
    ///
    /// As this function does not perform boundary checking, the caller must
    /// ensure that `self` is an index within the boundaries of `slice` before
    /// calling in order to prevent boundary escapes and the ensuing safety
    /// violations.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    unsafe fn get_unchecked(self, slice: &'a BitSlice<O, T>) -> Self::Immut;
    /// Returns a mutable reference to the output at this location, without
    /// performing any bounds checking. Calling this method with an
    /// out-of-bounds index is [undefined behavior] even if the resulting
    /// reference is not used.
    ///
    /// # Original
    ///
    /// [`SliceIndex::get_unchecked_mut`](https://doc.rust-lang.org/core/slice/trait.SliceIndex.html#method.get_unchecked_mut)
    ///
    /// # Safety
    ///
    /// As this function does not perform boundary checking, the caller must
    /// ensure that `self` is an index within the boundaries of `slice` before
    /// calling in order to prevent boundary escapes and the ensuing safety
    /// violations.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    unsafe fn get_unchecked_mut(self, slice: &'a mut BitSlice<O, T>) -> Self::Mut;
    /// Returns a shared reference to the output at this location, panicking if
    /// out of bounds.
    ///
    /// # Original
    ///
    /// [`SliceIndex::index`](https://doc.rust-lang.org/core/slice/trait.SliceIndex.html#method.index)
    fn index(self, slice: &'a BitSlice<O, T>) -> Self::Immut;
    /// Returns a mutable reference to the output at this location, panicking if
    /// out of bounds.
    ///
    /// # Original
    ///
    /// [`SliceIndex::index_mut`](https://doc.rust-lang.org/core/slice/trait.SliceIndex.html#method.index_mut)
    fn index_mut(self, slice: &'a mut BitSlice<O, T>) -> Self::Mut;
}
impl<'a, O, T> BitSliceIndex<'a, O, T> for usize
where
    O: 'a + BitOrder,
    T: 'a + BitStore,
{
    type Immut = &'a bool;
    type Mut = BitMut<'a, O, T>;
    #[inline]
    fn get(self, slice: &'a BitSlice<O, T>) -> Option<Self::Immut> {
        if self < slice.len() {
            Some(unsafe { self.get_unchecked(slice) })
        } else {
            None
        }
    }
    #[inline]
    fn get_mut(self, slice: &'a mut BitSlice<O, T>) -> Option<Self::Mut> {
        if self < slice.len() {
            Some(unsafe { self.get_unchecked_mut(slice) })
        } else {
            None
        }
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: &'a BitSlice<O, T>) -> Self::Immut {
        if slice.bitptr().read::<O>(self) { &true } else { &false }
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: &'a mut BitSlice<O, T>) -> Self::Mut {
        let bitptr = slice.bitptr();
        let (elt, bit) = bitptr.head().offset(self as isize);
        let addr = bitptr.pointer().to_access().offset(elt);
        BitMut::new_unchecked(addr, bit)
    }
    #[inline]
    fn index(self, slice: &'a BitSlice<O, T>) -> Self::Immut {
        self.get(slice)
            .unwrap_or_else(|| {
                panic!("Index {} out of bounds: {}", self, slice.len())
            })
    }
    #[inline]
    fn index_mut(self, slice: &'a mut BitSlice<O, T>) -> Self::Mut {
        let len = slice.len();
        self.get_mut(slice)
            .unwrap_or_else(|| panic!("Index {} out of bounds: {}", self, len))
    }
}
/// Implement indexing for the different range types.
macro_rules! range_impl {
    ($r:ty { $get:item $unchecked:item }) => {
        impl <'a, O, T > BitSliceIndex <'a, O, T > for $r where O : 'a + BitOrder, T : 'a
        + BitStore { type Immut = &'a BitSlice < O, T >; type Mut = &'a mut BitSlice < O,
        T >; #[inline] $get #[inline] fn get_mut(self, slice : Self::Mut) -> Option <
        Self::Mut > { self.get(slice).map(| s | s.bitptr().to_bitslice_mut()) } #[inline]
        $unchecked #[inline] unsafe fn get_unchecked_mut(self, slice : Self::Mut) ->
        Self::Mut { self.get_unchecked(slice).bitptr().to_bitslice_mut() } fn index(self,
        slice : Self::Immut) -> Self::Immut { let r = self.clone(); let l = slice.len();
        self.get(slice).unwrap_or_else(|| { panic!("Range {:?} out of bounds: {}", r, l)
        }) } #[inline] fn index_mut(self, slice : Self::Mut) -> Self::Mut { self
        .index(slice).bitptr().to_bitslice_mut() } }
    };
    ($($r:ty => map $func:expr;)*) => {
        $(impl <'a, O, T > BitSliceIndex <'a, O, T > for $r where O : 'a + BitOrder, T :
        'a + BitStore { type Immut = &'a BitSlice < O, T >; type Mut = &'a mut BitSlice <
        O, T >; #[inline] fn get(self, slice : Self::Immut) -> Option < Self::Immut > {
        $func (self).get(slice) } #[inline] fn get_mut(self, slice : Self::Mut) -> Option
        < Self::Mut > { $func (self).get_mut(slice) } #[inline] unsafe fn
        get_unchecked(self, slice : Self::Immut) -> Self::Immut { $func (self)
        .get_unchecked(slice) } #[inline] unsafe fn get_unchecked_mut(self, slice :
        Self::Mut) -> Self::Mut { $func (self).get_unchecked_mut(slice) } #[inline] fn
        index(self, slice : Self::Immut) -> Self::Immut { $func (self).index(slice) }
        #[inline] fn index_mut(self, slice : Self::Mut) -> Self::Mut { $func (self)
        .index_mut(slice) } })*
    };
}
range_impl!(
    Range < usize > { fn get(self, slice : Self::Immut) -> Option < Self::Immut > { let
    len = slice.len(); if self.start > len || self.end > len || self.start > self.end {
    return None; } Some(unsafe { (self.start..self.end).get_unchecked(slice) }) } unsafe
    fn get_unchecked(self, slice : Self::Immut) -> Self::Immut { let (addr, head, _) =
    slice.bitptr().raw_parts(); let (skip, new_head) = head.offset(self.start as isize);
    BitPtr::new_unchecked(addr.to_const().offset(skip), new_head, self.end - self.start,)
    .to_bitslice_ref() } }
);
range_impl!(
    RangeFrom < usize > { fn get(self, slice : Self::Immut) -> Option < Self::Immut > {
    let len = slice.len(); if self.start <= len { Some(unsafe { (self.start..)
    .get_unchecked(slice) }) } else { None } } unsafe fn get_unchecked(self, slice :
    Self::Immut) -> Self::Immut { let (addr, head, bits) = slice.bitptr().raw_parts();
    let (skip, new_head) = head.offset(self.start as isize); BitPtr::new_unchecked(addr
    .to_const().offset(skip), new_head, bits - self.start,).to_bitslice_ref() } }
);
range_impl!(
    RangeTo < usize > { fn get(self, slice : Self::Immut) -> Option < Self::Immut > { let
    len = slice.len(); if self.end <= len { Some(unsafe { (..self.end)
    .get_unchecked(slice) }) } else { None } } unsafe fn get_unchecked(self, slice :
    Self::Immut) -> Self::Immut { let mut bp = slice.bitptr(); bp.set_len(self.end); bp
    .to_bitslice_ref() } }
);
range_impl! {
    RangeInclusive < usize > => map | this : Self | { #[allow(clippy::range_plus_one)] (*
    this.start().. * this.end() + 1) }; RangeToInclusive < usize > => map |
    RangeToInclusive { end } | { #[allow(clippy::range_plus_one)] (..end + 1) };
}
/// `RangeFull` is the identity function.
#[cfg(not(tarpaulin_include))]
impl<'a, O, T> BitSliceIndex<'a, O, T> for RangeFull
where
    O: 'a + BitOrder,
    T: 'a + BitStore,
{
    type Immut = &'a BitSlice<O, T>;
    type Mut = &'a mut BitSlice<O, T>;
    #[inline]
    fn get(self, slice: Self::Immut) -> Option<Self::Immut> {
        Some(slice)
    }
    #[inline]
    fn get_mut(self, slice: Self::Mut) -> Option<Self::Mut> {
        Some(slice)
    }
    #[inline]
    unsafe fn get_unchecked(self, slice: Self::Immut) -> Self::Immut {
        slice
    }
    #[inline]
    unsafe fn get_unchecked_mut(self, slice: Self::Mut) -> Self::Mut {
        slice
    }
    #[inline]
    fn index(self, slice: Self::Immut) -> Self::Immut {
        slice
    }
    #[inline]
    fn index_mut(self, slice: Self::Mut) -> Self::Mut {
        slice
    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::prelude::*;
    use crate::slice as bv_slice;
    #[test]
    fn test_from_raw_parts() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = &rug_fuzz_0 as *const _;
        let len = rug_fuzz_1;
        unsafe {
            bv_slice::from_raw_parts::<Local, u8>(data, len);
        }
             }
});    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_len() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Local>();
        let p0 = bits;
        p0.len();
             }
});    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_rug_88_rrrruuuugggg_test_is_empty = 0;
        let mut p0 = BitSlice::<Local, u8>::empty();
        debug_assert!(p0.is_empty());
        let _rug_ed_tests_rug_88_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_first() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: u8 = rug_fuzz_0;
        let bits = data.view_bits::<Lsb0>();
        debug_assert_eq!(Some(& rug_fuzz_1), bits.first());
        let empty = BitSlice::<Local, usize>::empty();
        debug_assert_eq!(None, empty.first());
             }
});    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_first_mut() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Lsb0>();
        if let Some(mut first) = bits.first_mut() {
            *first = rug_fuzz_1;
        }
        debug_assert_eq!(data, 1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_split_first() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: u8 = rug_fuzz_0;
        let bits = data.view_bits::<Lsb0>();
        if let Some((first, _rest)) = bits.split_first() {
            debug_assert!(* first);
        }
             }
});    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_split_first_mut() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, bool, usize, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Lsb0>();
        if let Some((mut first, rest)) = bits.split_first_mut() {
            *first = rug_fuzz_1;
            *rest.get_mut(rug_fuzz_2).unwrap() = rug_fuzz_3;
        }
        debug_assert_eq!(data, 5);
        debug_assert!(
            BitSlice:: < Local, usize > ::empty_mut().split_first_mut().is_none()
        );
             }
});    }
}
#[cfg(test)]
mod tests_rug_93 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_split_last() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Msb0>();
        if let Some((last, _rest)) = bits.split_last() {
            debug_assert!(* last);
        }
             }
});    }
}
#[cfg(test)]
mod tests_rug_94 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u8, bool, usize, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        if let Some((mut last, rest)) = bits.split_last_mut() {
            *last = rug_fuzz_1;
            *rest.get_mut(rug_fuzz_2).unwrap() = rug_fuzz_3;
        }
        debug_assert_eq!(data, 5);
        debug_assert!(
            BitSlice:: < Local, usize > ::empty_mut().split_last_mut().is_none()
        );
             }
});    }
}
#[cfg(test)]
mod tests_rug_95 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_last() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data: u8 = rug_fuzz_0;
        let bits = data.view_bits::<Msb0>();
        debug_assert_eq!(Some(& rug_fuzz_1), bits.last());
        let empty = BitSlice::<Local, usize>::empty();
        debug_assert_eq!(None, empty.last());
             }
});    }
}
#[cfg(test)]
mod tests_rug_96 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        if let Some(mut last) = bits.last_mut() {
            *last = rug_fuzz_1;
        }
        debug_assert_eq!(data, 1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::prelude::*;
    use crate::slice::BitSlice;
    use crate::slice::BitSliceIndex;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u8, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Lsb0>();
        let mut p0 = bits;
        let mut p1 = rug_fuzz_1..rug_fuzz_2;
        BitSlice::<Lsb0, u8>::get(&p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use crate::prelude::*;
    use crate::slice::{BitSliceIndex, BitSlice};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = vec![0u16; 1];
        let bits = data.view_bits_mut::<Lsb0>();
        let p0: &mut BitSlice<Lsb0, u16> = bits;
        let p1 = rug_fuzz_0..=rug_fuzz_1;
        p0.get_mut(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::prelude::*;
    use crate::slice::BitSlice;
    #[test]
    fn test_get_unchecked() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u16, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Lsb0>();
        let index = rug_fuzz_1;
        unsafe {
            debug_assert_eq!(bits.get_unchecked(index), & true);
        }
             }
});    }
}
#[cfg(test)]
mod tests_rug_102 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u16, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Lsb0>();
        let bits_ptr = bits.as_mut_ptr();
        for i in rug_fuzz_1..bits.len() {
            unsafe { &mut *bits_ptr }.set(i, i % rug_fuzz_2 == rug_fuzz_3);
        }
        debug_assert_eq!(data, 0b0101_0101_0101_0101);
             }
});    }
}
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_swap() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u8, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Lsb0>();
        let p0 = bits;
        let p1 = rug_fuzz_1;
        let p2 = rug_fuzz_2;
        p0.swap(p1, p2);
        debug_assert_eq!(data, 8);
             }
});    }
}
#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_reverse() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let mut bits = data.view_bits_mut::<Msb0>();
        bits.reverse();
        debug_assert_eq!(data, 0b1_0011001);
             }
});    }
}
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_iter_mut() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u8, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        for (idx, mut elem) in bits.iter_mut().enumerate() {
            *elem = idx % rug_fuzz_1 == rug_fuzz_2;
        }
        debug_assert_eq!(data, 0b100_100_10);
             }
});    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Msb0>();
        let mut p0 = bits;
        let p1 = rug_fuzz_1;
        p0.windows(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_108 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_chunks() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Lsb0>();
        let mut p0 = bits;
        let p1: usize = rug_fuzz_1;
        p0.chunks(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_109 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_chunks_mut() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Lsb0>();
        let chunk_size = rug_fuzz_1;
        bits.chunks_mut(chunk_size);
             }
});    }
}
#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use crate::{prelude::*, slice::{BitSlice, api::ChunksExact}};
    #[test]
    fn test_chunks_exact() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Lsb0>();
        let mut chunk_size = rug_fuzz_1;
        BitSlice::<Lsb0, u8>::chunks_exact(&bits, chunk_size);
             }
});    }
}
#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_chunks_exact_mut() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Lsb0>();
        let mut chunk_size = rug_fuzz_1;
        bits.chunks_exact_mut(chunk_size);
             }
});    }
}
#[cfg(test)]
mod tests_rug_112 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Lsb0>();
        let mut p0 = bits;
        let mut p1: usize = rug_fuzz_1;
        p0.rchunks(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Lsb0>();
        let mut chunk_size = rug_fuzz_1;
        bits.rchunks_mut(chunk_size);
             }
});    }
}
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Lsb0>();
        let chunk_size = rug_fuzz_1;
        let p0 = bits;
        let p1 = chunk_size;
        p0.rchunks_exact(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_115 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Lsb0>();
        let chunk_size = rug_fuzz_1;
        bits.rchunks_exact_mut(chunk_size);
             }
});    }
}
#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_split_at() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Local>();
        let mid = rug_fuzz_1;
        let (left, right) = bits.split_at(mid);
        debug_assert_eq!(left, & bits[..2]);
        debug_assert_eq!(right, & bits[2..]);
             }
});    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        let mid = rug_fuzz_1;
        let (mut p0, mut p1) = bits.split_at_mut(mid);
             }
});    }
}
#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Msb0>();
        let mut pred = |pos: usize, bit: &bool| *bit;
        bits.split(pred).for_each(|slice| {});
             }
});    }
}
#[cfg(test)]
mod tests_rug_119 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        bits.split_mut(|_pos, bit| *bit);
             }
});    }
}
#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Msb0>();
        let mut p0 = bits;
        let mut p1 = |pos: usize, bit: &bool| bit.clone();
        p0.rsplit(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rsplit_mut() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        let mut p0 = bits;
        let p1 = |_: usize, bit: &bool| *bit;
        p0.rsplit_mut(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::prelude::*;
    use crate::slice;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u8, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Msb0>();
        let mut p0 = bits;
        let p1 = rug_fuzz_1;
        let mut p2 = |pos: usize, bit: &bool| pos % rug_fuzz_2 == rug_fuzz_3;
        p0.splitn(p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use crate::prelude::*;
    use crate::slice;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        let n = rug_fuzz_1;
        <slice::BitSlice<Msb0, u8>>::splitn_mut(bits, n, |_, bit| *bit);
             }
});    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::prelude::*;
    use crate::slice::BitSlice;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u8, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits = data.view_bits::<Msb0>();
        let mut p0: &BitSlice<Msb0, u8> = bits;
        let p1: usize = rug_fuzz_1;
        let mut p2 = |pos: usize, bit: &bool| -> bool { pos % rug_fuzz_2 == rug_fuzz_3 };
        p0.rsplitn(p1, p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        let mut p0 = bits;
        let p1: usize = rug_fuzz_1;
        let mut p2 = |pos: usize, bit: &bool| *bit;
        p0.rsplitn_mut(p1, &mut p2);
             }
});    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u8, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let bits_msb = data.view_bits::<Msb0>();
        let bits_lsb = data.view_bits::<Lsb0>();
        bits_msb.contains(&bits_lsb[rug_fuzz_1..rug_fuzz_2]);
             }
});    }
}
#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    use crate::prelude::*;
    use crate::slice::{BitSlice, BitOrder, BitStore};
    #[test]
    fn test_starts_with() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u8, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let haystack = data.view_bits::<Msb0>();
        let needle = &data.view_bits::<Lsb0>()[rug_fuzz_1..rug_fuzz_2];
        haystack.starts_with(needle);
             }
});    }
}
#[cfg(test)]
mod tests_rug_128 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u8, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let data = rug_fuzz_0;
        let haystack = data.view_bits::<Lsb0>();
        let needle = &data.view_bits::<Msb0>()[rug_fuzz_1..rug_fuzz_2];
        let p0: &BitSlice<Lsb0, u8> = haystack;
        let p1: &BitSlice<Msb0, u8> = needle;
        p0.ends_with(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rotate_left() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u8, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let mut bits = data.view_bits_mut::<Msb0>();
        let mut by = rug_fuzz_1;
        bits.rotate_left(by);
        debug_assert_eq!(data, 0xC3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rotate_right() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = bitvec![Msb0, u8; 0xF0];
        let mut p1 = rug_fuzz_0;
        p0.rotate_right(p1);
        debug_assert_eq!(p0.as_slice() [rug_fuzz_1], 0x3C);
             }
});    }
}
#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u8, u16, usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits = data.view_bits_mut::<Msb0>();
        let mut src_data = rug_fuzz_1;
        let src_bits = src_data.view_bits::<Lsb0>();
        bits[..rug_fuzz_2].clone_from_bitslice(&src_bits[rug_fuzz_3..rug_fuzz_4]);
        debug_assert_eq!(data, 0xC0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use crate::prelude::*;
    use std::ops::RangeTo;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u8, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut data = rug_fuzz_0;
        let bits: &mut BitSlice<Msb0, u8> = data.view_bits_mut::<Msb0>();
        let src: RangeTo<usize> = ..rug_fuzz_1;
        let dest: usize = rug_fuzz_2;
        bits.copy_within(src, dest);
        debug_assert_eq!(data, 0x1E);
             }
});    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_136_rrrruuuugggg_test_rug = 0;
        let mut p0 = bitvec![Msb0, u8; 0xA5, 0x69];
        let mut p1 = bitvec![Lsb0, u16; 0x1234];
        p0.swap_with_bitslice(&mut p1);
        let _rug_ed_tests_rug_136_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_align_to() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_align_to = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 6;
        let rug_fuzz_6 = 7;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 8;
        let rug_fuzz_9 = "This case will not occur";
        unsafe {
            let bytes: [u8; 7] = [
                rug_fuzz_0,
                rug_fuzz_1,
                rug_fuzz_2,
                rug_fuzz_3,
                rug_fuzz_4,
                rug_fuzz_5,
                rug_fuzz_6,
            ];
            let bits = bytes.view_bits::<Local>();
            let (prefix, shorts, suffix) = bits.align_to::<u16>();
            match prefix.len() {
                rug_fuzz_7 => {
                    debug_assert_eq!(shorts, bits[..48]);
                    debug_assert_eq!(suffix, bits[48..]);
                }
                rug_fuzz_8 => {
                    debug_assert_eq!(prefix, bits[..8]);
                    debug_assert_eq!(shorts, bits[8..]);
                }
                _ => unreachable!(rug_fuzz_9),
            }
        }
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_align_to = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_bitvec_to_bitvec() {
        let bits = bits![0, 1, 0, 1];
        let bv = bits.to_bitvec();
        assert_eq!(bits, bv);
    }
}
#[cfg(test)]
mod tests_rug_141 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_141_rrrruuuugggg_test_rug = 0;
        let mut p0 = &BitSlice::<Local, u32>::empty();
        p0.to_vec();
        let _rug_ed_tests_rug_141_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_bitvec_repeat() {
        let bitslice_data = bits![0, 1];
        let n = 3;
        let result = bitslice_data.repeat(n);
        assert_eq!(result, bits![0, 1, 0, 1, 0, 1]);
    }
}
