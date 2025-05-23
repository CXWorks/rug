/*! Memory access control.

`bitvec` allows a program to produce handles over memory that do not logically
alias, but may alias in hardware. This module provides a unified interface for
memory accesses that can be specialized to handle aliased and unaliased access
events.

The `BitAccess` trait provides capabilities to access bits in memory elements
through shared references, and its implementations are responsible for
coördinating synchronization and contention as needed.
!*/

use crate::{
	index::{
		BitIdx,
		BitMask,
	},
	mem::BitMemory,
	order::BitOrder,
};

use core::{
	fmt::Debug,
	sync::atomic::Ordering,
};

use radium::Radium;

/** Access interface to memory locations.

This trait abstracts over the specific instructions used to perform accesses to
memory locations, so that use sites elsewhere in the crate can select their
required behavior without changing the interface.

This is automatically implemented for all types that permit shared/mutable
memory access to register types through the `radium` crate. Its use is
constrained in the `store` module.
**/
pub trait BitAccess<M>: Debug + Radium<M> + Sized
where M: BitMemory
{
	/// Sets one bit in a memory element to `0`.
	///
	/// # Type Parameters
	///
	/// - `O`: A bit ordering.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `index`: The semantic index of the bit in `*self` to be erased.
	///
	/// # Effects
	///
	/// The memory element at `*self` has the bit corresponding to `index` set
	/// to `0`, and all other bits are unchanged.
	#[inline]
	fn clear_bit<O>(&self, index: BitIdx<M>)
	where O: BitOrder {
		self.fetch_and(!index.select::<O>().value(), Ordering::Relaxed);
	}

	/// Set any number of bits in a memory element to `0`.
	///
	/// The mask provided to this method must be constructed from indices that
	/// are valid in the caller’s context. As the mask is already computed by
	/// the caller, this does not take an ordering type parameter.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `mask`: A mask of any number of bits. This is a selection mask of bits
	///   to modify.
	///
	/// # Effects
	///
	/// All bits in `*self` that are selected (set to `1`) in `mask` will be set
	/// to `0`, and all bits in `*self` that are not selected (set to `0`) in
	/// `mask` will be unchanged.
	///
	/// Do not invert the mask prior to calling this function in order to save
	/// the unselected bits and erase the selected bits. `BitMask` is a
	/// selection type, not a bitwise-operation argument.
	#[inline]
	fn clear_bits(&self, mask: BitMask<M>) {
		self.fetch_and(!mask.value(), Ordering::Relaxed);
	}

	/// Sets one bit in a memory element to `1`.
	///
	/// # Type Parameters
	///
	/// - `O`: A bit ordering.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `index`: The semantic index of the bit in `*self` to be written.
	///
	/// # Effects
	///
	/// The memory element at `*self` has the bit corresponding to `index` set
	/// to `1`, and all other bits are unchanged.
	#[inline]
	fn set_bit<O>(&self, index: BitIdx<M>)
	where O: BitOrder {
		self.fetch_or(index.select::<O>().value(), Ordering::Relaxed);
	}

	/// Sets any number of bits in a memory element to `1`.
	///
	/// The mask provided to this method must be constructed from indices that
	/// are valid in the caller’s context. As the mask is already computed by
	/// the caller, this does not need to take an ordering type parameter.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `mask`: A mask of any number of bits. This is a selection mask of bits
	///   to modify.
	///
	/// # Effects
	///
	/// All bits in `*self` that are selected (set to `1`) in `mask` will be set
	/// to `1`, and all bits in `*self` that are not selected (set to `0`) in
	/// `mask` will be unchanged.
	#[inline]
	fn set_bits(&self, mask: BitMask<M>) {
		self.fetch_or(mask.value(), Ordering::Relaxed);
	}

	/// Inverts the value of one bit in a memory element.
	///
	/// # Type Parameters
	///
	/// - `O`: A bit ordering.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `index`: The semantic index of the bit in `*self` to be inverted.
	///
	/// # Effects
	///
	/// The memory element at `*self` has the bit corresponding to `index` set
	/// to the opposite of its current value. All other bits are unchanged.
	#[inline]
	fn invert_bit<O>(&self, index: BitIdx<M>)
	where O: BitOrder {
		self.fetch_xor(index.select::<O>().value(), Ordering::Relaxed);
	}

	/// Invert any number of bits in a memory element.
	///
	/// The mask provided to this method must be constructed from indices that
	/// are valid in the caller’s context. As the mask is already computed by
	/// the caller, this does not take an ordering type parameter.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `mask`: A mask of any number of bits. This is a selection mask of bits
	///   to modify.
	///
	/// # Effects
	///
	/// All bits in `*self` that are selected (set to `1`) in `mask` will be set
	/// to the opposite of their current value, and all bits in `*self` that are
	/// not selected (set to `0`) in `mask` will be unchanged.
	#[inline]
	fn invert_bits(&self, mask: BitMask<M>) {
		self.fetch_xor(mask.value(), Ordering::Relaxed);
	}

	/// Fetches the value of one bit in a memory element.
	///
	/// # Type Parameters
	///
	/// - `O`: A bit ordering.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `index`: The semantic index of the bit in `*self` to read.
	///
	/// # Returns
	///
	/// The value of the bit in `*self` corresponding to `index`.
	#[inline]
	fn get_bit<O>(&self, index: BitIdx<M>) -> bool
	where O: BitOrder {
		unsafe { BitMask::new(self.load_value()) }.test(index.select::<O>())
	}

	/// Fetches any number of bits from a memory element.
	///
	/// The mask provided to this method must be constructed from indices that
	/// are valid in the caller’s context. As the mask is already computed by
	/// the caller, this does not take an ordering type parameter.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `mask`: A mask of any number of bits. This is a selection mask of bits
	///   to read.
	///
	/// # Returns
	///
	/// A copy of the memory element at `*self`, with all bits not selected (set
	/// to `0`) in `mask` erased and all bits selected (set to `1`) in `mask`
	/// preserved.
	#[inline]
	fn get_bits(&self, mask: BitMask<M>) -> M {
		self.load_value() & mask.value()
	}

	/// Writes a bit to an index within the `self` element.
	///
	/// # Type Parameters
	///
	/// - `O`: A bit ordering.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `index`: The semantic index of the bit in `*self` to be written.
	/// - `value`: The bit value to write into `*self` at `index`.
	///
	/// # Effects
	///
	/// The bit in `*self` at `index` is set to the `value` bit.
	#[inline]
	fn write_bit<O>(&self, index: BitIdx<M>, value: bool)
	where O: BitOrder {
		if value {
			self.set_bit::<O>(index);
		}
		else {
			self.clear_bit::<O>(index);
		}
	}

	/// Writes a bit value to any number of bits within the `self` element.
	///
	/// The mask provided to this method must be constructed from indices that
	/// are valid in the caller’s context. As the mask is already computed by
	/// the caller, this does not need to take an ordering type parameter.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `mask`: A mask of any number of bits. This is a selection mask of bits
	///   to modify.
	///
	/// # Effects
	///
	/// All bits in `*self` that are selected (set to `1`) in `mask` will be set
	/// to `value`, and all bits in `*self` that are not selected (set to `0`)
	/// in `mask` will be unchanged.
	#[inline]
	fn write_bits(&self, mask: BitMask<M>, value: bool) {
		if value {
			self.set_bits(mask);
		}
		else {
			self.clear_bits(mask);
		}
	}

	/// Gets the function that writes `value` to an index.
	///
	/// # Type Parameters
	///
	/// - `O`: A bit ordering.
	///
	/// # Parameters
	///
	/// - `value`: The bit that will be directly written by the returned
	///   function.
	///
	/// # Returns
	///
	/// A function which, when applied to a reference and an index, will write
	/// `value` into memory. If `value` is `false`, then this produces
	/// [`clear_bit`]; if it is `true`, then this produces [`set_bit`].
	///
	/// [`clear_bit`]: #method.clear_bit
	/// [`set_bit`]: #method.set_bit
	#[inline]
	fn get_writer<O>(value: bool) -> for<'a> fn(&'a Self, BitIdx<M>)
	where O: BitOrder {
		[Self::clear_bit::<O>, Self::set_bit::<O>][value as usize]
	}

	/// Gets the function that writes `value` into all bits under a mask.
	///
	/// # Parameters
	///
	/// - `value`: The bit that will be directly written by the returned
	///   function.
	///
	/// # Returns
	///
	/// A function which, when applied to a reference and a mask, will write
	/// `value` into memory. If `value` is `false`, then this produces
	/// [`clear_bits`]; if it is `true`, then this produces [`set_bits`].
	///
	/// [`clear_bits`]: #method.clear_bits
	/// [`set_bits`]: #method.set_bits
	#[inline]
	fn get_writers(value: bool) -> for<'a> fn(&'a Self, BitMask<M>) {
		[Self::clear_bits, Self::set_bits][value as usize]
	}

	/// Copies a memory element into the caller’s local context.
	///
	/// # Parameters
	///
	/// - `&self`
	///
	/// # Returns
	///
	/// A copy of the value at `*self`.
	#[inline]
	fn load_value(&self) -> M {
		self.load(Ordering::Relaxed)
	}

	/// Unconditionally writes a value into a memory location.
	///
	/// # Parameters
	///
	/// - `&self`
	/// - `value`: The new value to write into `*self`.
	///
	/// # Effects
	///
	/// The current value at `*self` is replaced with `value`.
	///
	/// # Safety
	///
	/// The calling context must either have write permissions to the entire
	/// memory element at `*self`, or construct a `value` that does not modify
	/// the bits of `*self` that the caller does not currently own.
	///
	/// As this directly permits the modification of memory outside the logical
	/// ownership of the caller, this method risks behavior that violates the
	/// Rust memory model, even if it may not be technically undefined.
	#[inline]
	unsafe fn store_value(&self, value: M) {
		self.store(value, Ordering::Relaxed)
	}
}

impl<M, R> BitAccess<M> for R
where
	M: BitMemory,
	R: Debug + Radium<M>,
{
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;

	#[test]
	fn touch_memory() {
		let mut data = 0u8;
		let bits = data.view_bits_mut::<Local>();
		let accessor = unsafe { &*(bits.bitptr().pointer().to_access()) };

		BitAccess::set_bit::<Lsb0>(accessor, BitIdx::ZERO);
		assert_eq!(accessor.get(), 1);

		BitAccess::set_bits(accessor, BitMask::ALL);
		assert_eq!(accessor.get(), !0);

		BitAccess::clear_bit::<Lsb0>(accessor, BitIdx::ZERO);
		assert_eq!(accessor.get(), !1);

		BitAccess::clear_bits(accessor, BitMask::ALL);
		assert_eq!(accessor.get(), 0);

		BitAccess::invert_bit::<Lsb0>(accessor, BitIdx::ZERO);
		assert_eq!(accessor.get(), 1);
		BitAccess::invert_bits(accessor, BitMask::ALL);
		assert_eq!(accessor.get(), !1);

		assert!(!BitAccess::get_bit::<Lsb0>(accessor, BitIdx::ZERO));
		assert_eq!(accessor.get(), !1);

		BitAccess::write_bit::<Lsb0>(accessor, BitIdx::new(1).unwrap(), false);
		assert_eq!(accessor.get(), !3);

		BitAccess::write_bits(accessor, BitMask::ALL, true);
		assert_eq!(accessor.get(), !0);
		BitAccess::write_bits(accessor, Lsb0::mask(BitIdx::new(2), None), false);
		assert_eq!(
			BitAccess::get_bits(accessor, Lsb0::mask(BitIdx::new(2), None)),
			0
		);
		assert_eq!(accessor.get(), 3);

		BitAccess::get_writer::<Lsb0>(false)(accessor, BitIdx::ZERO);
		assert_eq!(accessor.get(), 2);

		unsafe {
			BitAccess::store_value(accessor, !1);
		}
		assert_eq!(accessor.get(), !1);
	}

	#[test]
	fn sanity_check_prefetch() {
		use core::cell::Cell;

		assert_eq!(
			<Cell<u8> as BitAccess::<u8>>::get_writer::<Msb0>(false)
				as *const (),
			<Cell<u8> as BitAccess::<u8>>::clear_bit::<Msb0> as *const ()
		);

		assert_eq!(
			<Cell<u8> as BitAccess::<u8>>::get_writer::<Msb0>(true) as *const (),
			<Cell<u8> as BitAccess::<u8>>::set_bit::<Msb0> as *const ()
		);

		assert_eq!(
			<Cell<u8> as BitAccess::<u8>>::get_writers(false) as *const (),
			<Cell<u8> as BitAccess::<u8>>::clear_bits as *const ()
		);

		assert_eq!(
			<Cell<u8> as BitAccess::<u8>>::get_writers(true) as *const (),
			<Cell<u8> as BitAccess::<u8>>::set_bits as *const ()
		);
	}
}
#[cfg(test)]
mod tests_rug_209 {
    use super::*;
    use crate::prelude::traits::BitMemory;
    use crate::access::{BitAccess, BitMask, BitIdx};
    use std::sync::atomic::{AtomicU8, Ordering};
    
    #[test]
    fn test_rug() {
        let mut p0 = AtomicU8::new(0);
        let p1: u8 = 5;
        
        unsafe {
            BitAccess::store_value(&p0, p1);
        }
    }
}