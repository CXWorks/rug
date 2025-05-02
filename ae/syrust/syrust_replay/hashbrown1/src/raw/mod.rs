use crate::alloc::alloc::{alloc, dealloc, handle_alloc_error};
use crate::scopeguard::guard;
use crate::TryReserveError;
use core::alloc::Layout;
use core::hint;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem;
use core::mem::ManuallyDrop;
use core::ptr::NonNull;
cfg_if! {
    if #[cfg(all(target_feature = "sse2", any(target_arch = "x86", target_arch =
    "x86_64"), not(miri)))] { mod sse2; use sse2 as imp; } else { #[path = "generic.rs"]
    mod generic; use generic as imp; }
}
mod bitmask;
use self::bitmask::{BitMask, BitMaskIter};
use self::imp::Group;
#[cfg(feature = "nightly")]
use core::intrinsics::{likely, unlikely};
#[cfg(not(feature = "nightly"))]
#[inline]
fn likely(b: bool) -> bool {
    b
}
#[cfg(not(feature = "nightly"))]
#[inline]
fn unlikely(b: bool) -> bool {
    b
}
#[cfg(feature = "nightly")]
#[cfg_attr(feature = "inline-more", inline)]
unsafe fn offset_from<T>(to: *const T, from: *const T) -> usize {
    to.offset_from(from) as usize
}
#[cfg(not(feature = "nightly"))]
#[cfg_attr(feature = "inline-more", inline)]
unsafe fn offset_from<T>(to: *const T, from: *const T) -> usize {
    (to as usize - from as usize) / mem::size_of::<T>()
}
/// Whether memory allocation errors should return an error or abort.
#[derive(Copy, Clone)]
enum Fallibility {
    Fallible,
    Infallible,
}
impl Fallibility {
    /// Error to return on capacity overflow.
    #[cfg_attr(feature = "inline-more", inline)]
    fn capacity_overflow(self) -> TryReserveError {
        match self {
            Fallibility::Fallible => TryReserveError::CapacityOverflow,
            Fallibility::Infallible => panic!("Hash table capacity overflow"),
        }
    }
    /// Error to return on allocation error.
    #[cfg_attr(feature = "inline-more", inline)]
    fn alloc_err(self, layout: Layout) -> TryReserveError {
        match self {
            Fallibility::Fallible => {
                TryReserveError::AllocError {
                    layout,
                }
            }
            Fallibility::Infallible => handle_alloc_error(layout),
        }
    }
}
/// Control byte value for an empty bucket.
const EMPTY: u8 = 0b1111_1111;
/// Control byte value for a deleted bucket.
const DELETED: u8 = 0b1000_0000;
/// Checks whether a control byte represents a full bucket (top bit is clear).
#[inline]
fn is_full(ctrl: u8) -> bool {
    ctrl & 0x80 == 0
}
/// Checks whether a control byte represents a special value (top bit is set).
#[inline]
fn is_special(ctrl: u8) -> bool {
    ctrl & 0x80 != 0
}
/// Checks whether a special control value is EMPTY (just check 1 bit).
#[inline]
fn special_is_empty(ctrl: u8) -> bool {
    debug_assert!(is_special(ctrl));
    ctrl & 0x01 != 0
}
/// Primary hash function, used to select the initial bucket to probe from.
#[inline]
#[allow(clippy::cast_possible_truncation)]
fn h1(hash: u64) -> usize {
    hash as usize
}
/// Secondary hash function, saved in the low 7 bits of the control byte.
#[inline]
#[allow(clippy::cast_possible_truncation)]
fn h2(hash: u64) -> u8 {
    let hash_len = usize::min(mem::size_of::<usize>(), mem::size_of::<u64>());
    let top7 = hash >> (hash_len * 8 - 7);
    (top7 & 0x7f) as u8
}
/// Probe sequence based on triangular numbers, which is guaranteed (since our
/// table size is a power of two) to visit every group of elements exactly once.
///
/// A triangular probe has us jump by 1 more group every time. So first we
/// jump by 1 group (meaning we just continue our linear scan), then 2 groups
/// (skipping over 1 group), then 3 groups (skipping over 2 groups), and so on.
///
/// Proof that the probe will visit every group in the table:
/// <https://fgiesen.wordpress.com/2015/02/22/triangular-numbers-mod-2n/>
struct ProbeSeq {
    bucket_mask: usize,
    pos: usize,
    stride: usize,
}
impl Iterator for ProbeSeq {
    type Item = usize;
    #[inline]
    fn next(&mut self) -> Option<usize> {
        debug_assert!(
            self.stride <= self.bucket_mask, "Went past end of probe sequence"
        );
        let result = self.pos;
        self.stride += Group::WIDTH;
        self.pos += self.stride;
        self.pos &= self.bucket_mask;
        Some(result)
    }
}
/// Returns the number of buckets needed to hold the given number of items,
/// taking the maximum load factor into account.
///
/// Returns `None` if an overflow occurs.
#[cfg_attr(target_os = "emscripten", inline(never))]
#[cfg_attr(not(target_os = "emscripten"), inline)]
fn capacity_to_buckets(cap: usize) -> Option<usize> {
    debug_assert_ne!(cap, 0);
    if cap < 8 {
        return Some(if cap < 4 { 4 } else { 8 });
    }
    let adjusted_cap = cap.checked_mul(8)? / 7;
    Some(adjusted_cap.next_power_of_two())
}
/// Returns the maximum effective capacity for the given bucket mask, taking
/// the maximum load factor into account.
#[inline]
fn bucket_mask_to_capacity(bucket_mask: usize) -> usize {
    if bucket_mask < 8 { bucket_mask } else { ((bucket_mask + 1) / 8) * 7 }
}
/// Returns a Layout which describes the allocation required for a hash table,
/// and the offset of the control bytes in the allocation.
/// (the offset is also one past last element of buckets)
///
/// Returns `None` if an overflow occurs.
#[cfg_attr(feature = "inline-more", inline)]
#[cfg(feature = "nightly")]
fn calculate_layout<T>(buckets: usize) -> Option<(Layout, usize)> {
    debug_assert!(buckets.is_power_of_two());
    let data = Layout::array::<T>(buckets).ok()?;
    let ctrl = unsafe {
        Layout::from_size_align_unchecked(buckets + Group::WIDTH, Group::WIDTH)
    };
    data.extend(ctrl).ok()
}
/// Returns a Layout which describes the allocation required for a hash table,
/// and the offset of the control bytes in the allocation.
/// (the offset is also one past last element of buckets)
///
/// Returns `None` if an overflow occurs.
#[cfg_attr(feature = "inline-more", inline)]
#[cfg(not(feature = "nightly"))]
fn calculate_layout<T>(buckets: usize) -> Option<(Layout, usize)> {
    debug_assert!(buckets.is_power_of_two());
    let ctrl_align = usize::max(mem::align_of::<T>(), Group::WIDTH);
    let ctrl_offset = mem::size_of::<T>()
        .checked_mul(buckets)?
        .checked_add(ctrl_align - 1)? & !(ctrl_align - 1);
    let len = ctrl_offset.checked_add(buckets + Group::WIDTH)?;
    Some((unsafe { Layout::from_size_align_unchecked(len, ctrl_align) }, ctrl_offset))
}
/// A reference to a hash table bucket containing a `T`.
///
/// This is usually just a pointer to the element itself. However if the element
/// is a ZST, then we instead track the index of the element in the table so
/// that `erase` works properly.
pub struct Bucket<T> {
    ptr: NonNull<T>,
}
unsafe impl<T> Send for Bucket<T> {}
impl<T> Clone for Bucket<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}
impl<T> Bucket<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn from_base_index(base: NonNull<T>, index: usize) -> Self {
        let ptr = if mem::size_of::<T>() == 0 {
            (index + 1) as *mut T
        } else {
            base.as_ptr().sub(index)
        };
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn to_base_index(&self, base: NonNull<T>) -> usize {
        if mem::size_of::<T>() == 0 {
            self.ptr.as_ptr() as usize - 1
        } else {
            offset_from(base.as_ptr(), self.ptr.as_ptr())
        }
    }
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn as_ptr(&self) -> *mut T {
        if mem::size_of::<T>() == 0 {
            mem::align_of::<T>() as *mut T
        } else {
            self.ptr.as_ptr().sub(1)
        }
    }
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn next_n(&self, offset: usize) -> Self {
        let ptr = if mem::size_of::<T>() == 0 {
            (self.ptr.as_ptr() as usize + offset) as *mut T
        } else {
            self.ptr.as_ptr().sub(offset)
        };
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn drop(&self) {
        self.as_ptr().drop_in_place();
    }
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn read(&self) -> T {
        self.as_ptr().read()
    }
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn write(&self, val: T) {
        self.as_ptr().write(val);
    }
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        &*self.as_ptr()
    }
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn as_mut<'a>(&self) -> &'a mut T {
        &mut *self.as_ptr()
    }
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn copy_from_nonoverlapping(&self, other: &Self) {
        self.as_ptr().copy_from_nonoverlapping(other.as_ptr(), 1);
    }
}
/// A raw hash table with an unsafe API.
pub struct RawTable<T> {
    bucket_mask: usize,
    ctrl: NonNull<u8>,
    growth_left: usize,
    items: usize,
    marker: PhantomData<T>,
}
impl<T> RawTable<T> {
    /// Creates a new empty hash table without allocating any memory.
    ///
    /// In effect this returns a table with exactly 1 bucket. However we can
    /// leave the data pointer dangling since that bucket is never written to
    /// due to our load factor forcing us to always have at least 1 free bucket.
    #[cfg_attr(feature = "inline-more", inline)]
    pub const fn new() -> Self {
        Self {
            ctrl: unsafe {
                NonNull::new_unchecked(Group::static_empty() as *const _ as *mut u8)
            },
            bucket_mask: 0,
            items: 0,
            growth_left: 0,
            marker: PhantomData,
        }
    }
    /// Allocates a new hash table with the given number of buckets.
    ///
    /// The control bytes are left uninitialized.
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn new_uninitialized(
        buckets: usize,
        fallability: Fallibility,
    ) -> Result<Self, TryReserveError> {
        debug_assert!(buckets.is_power_of_two());
        let (layout, ctrl_offset) = match calculate_layout::<T>(buckets) {
            Some(lco) => lco,
            None => return Err(fallability.capacity_overflow()),
        };
        let ptr = match NonNull::new(alloc(layout)) {
            Some(ptr) => ptr,
            None => return Err(fallability.alloc_err(layout)),
        };
        let ctrl = NonNull::new_unchecked(ptr.as_ptr().add(ctrl_offset));
        Ok(Self {
            ctrl,
            bucket_mask: buckets - 1,
            items: 0,
            growth_left: bucket_mask_to_capacity(buckets - 1),
            marker: PhantomData,
        })
    }
    /// Attempts to allocate a new hash table with at least enough capacity
    /// for inserting the given number of elements without reallocating.
    fn fallible_with_capacity(
        capacity: usize,
        fallability: Fallibility,
    ) -> Result<Self, TryReserveError> {
        if capacity == 0 {
            Ok(Self::new())
        } else {
            unsafe {
                let buckets = match capacity_to_buckets(capacity) {
                    Some(buckets) => buckets,
                    None => return Err(fallability.capacity_overflow()),
                };
                let result = Self::new_uninitialized(buckets, fallability)?;
                result.ctrl(0).write_bytes(EMPTY, result.num_ctrl_bytes());
                Ok(result)
            }
        }
    }
    /// Attempts to allocate a new hash table with at least enough capacity
    /// for inserting the given number of elements without reallocating.
    #[cfg(feature = "raw")]
    pub fn try_with_capacity(capacity: usize) -> Result<Self, TryReserveError> {
        Self::fallible_with_capacity(capacity, Fallibility::Fallible)
    }
    /// Allocates a new hash table with at least enough capacity for inserting
    /// the given number of elements without reallocating.
    pub fn with_capacity(capacity: usize) -> Self {
        match Self::fallible_with_capacity(capacity, Fallibility::Infallible) {
            Ok(capacity) => capacity,
            Err(_) => unsafe { hint::unreachable_unchecked() }
        }
    }
    /// Deallocates the table without dropping any entries.
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn free_buckets(&mut self) {
        let (layout, ctrl_offset) = match calculate_layout::<T>(self.buckets()) {
            Some(lco) => lco,
            None => hint::unreachable_unchecked(),
        };
        dealloc(self.ctrl.as_ptr().sub(ctrl_offset), layout);
    }
    /// Returns pointer to one past last element of data table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn data_end(&self) -> NonNull<T> {
        NonNull::new_unchecked(self.ctrl.as_ptr() as *mut T)
    }
    /// Returns pointer to start of data table.
    #[cfg_attr(feature = "inline-more", inline)]
    #[cfg(feature = "nightly")]
    pub unsafe fn data_start(&self) -> *mut T {
        self.data_end().as_ptr().wrapping_sub(self.buckets())
    }
    /// Returns the index of a bucket from a `Bucket`.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn bucket_index(&self, bucket: &Bucket<T>) -> usize {
        bucket.to_base_index(self.data_end())
    }
    /// Returns a pointer to a control byte.
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn ctrl(&self, index: usize) -> *mut u8 {
        debug_assert!(index < self.num_ctrl_bytes());
        self.ctrl.as_ptr().add(index)
    }
    /// Returns a pointer to an element in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn bucket(&self, index: usize) -> Bucket<T> {
        debug_assert_ne!(self.bucket_mask, 0);
        debug_assert!(index < self.buckets());
        Bucket::from_base_index(self.data_end(), index)
    }
    /// Erases an element from the table without dropping it.
    #[cfg_attr(feature = "inline-more", inline)]
    #[deprecated(since = "0.8.1", note = "use erase or remove instead")]
    pub unsafe fn erase_no_drop(&mut self, item: &Bucket<T>) {
        let index = self.bucket_index(item);
        debug_assert!(is_full(* self.ctrl(index)));
        let index_before = index.wrapping_sub(Group::WIDTH) & self.bucket_mask;
        let empty_before = Group::load(self.ctrl(index_before)).match_empty();
        let empty_after = Group::load(self.ctrl(index)).match_empty();
        let ctrl = if empty_before.leading_zeros() + empty_after.trailing_zeros()
            >= Group::WIDTH
        {
            DELETED
        } else {
            self.growth_left += 1;
            EMPTY
        };
        self.set_ctrl(index, ctrl);
        self.items -= 1;
    }
    /// Erases an element from the table, dropping it in place.
    #[cfg_attr(feature = "inline-more", inline)]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(deprecated)]
    pub unsafe fn erase(&mut self, item: Bucket<T>) {
        self.erase_no_drop(&item);
        item.drop();
    }
    /// Finds and erases an element from the table, dropping it in place.
    /// Returns true if an element was found.
    #[cfg(feature = "raw")]
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn erase_entry(&mut self, hash: u64, eq: impl FnMut(&T) -> bool) -> bool {
        if let Some(bucket) = self.find(hash, eq) {
            unsafe { self.erase(bucket) };
            true
        } else {
            false
        }
    }
    /// Removes an element from the table, returning it.
    #[cfg_attr(feature = "inline-more", inline)]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(deprecated)]
    pub unsafe fn remove(&mut self, item: Bucket<T>) -> T {
        self.erase_no_drop(&item);
        item.read()
    }
    /// Finds and removes an element from the table, returning it.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn remove_entry(&mut self, hash: u64, eq: impl FnMut(&T) -> bool) -> Option<T> {
        match self.find(hash, eq) {
            Some(bucket) => Some(unsafe { self.remove(bucket) }),
            None => None,
        }
    }
    /// Returns an iterator for a probe sequence on the table.
    ///
    /// This iterator never terminates, but is guaranteed to visit each bucket
    /// group exactly once. The loop using `probe_seq` must terminate upon
    /// reaching a group containing an empty bucket.
    #[cfg_attr(feature = "inline-more", inline)]
    fn probe_seq(&self, hash: u64) -> ProbeSeq {
        ProbeSeq {
            bucket_mask: self.bucket_mask,
            pos: h1(hash) & self.bucket_mask,
            stride: 0,
        }
    }
    /// Sets a control byte, and possibly also the replicated control byte at
    /// the end of the array.
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn set_ctrl(&self, index: usize, ctrl: u8) {
        let index2 = ((index.wrapping_sub(Group::WIDTH)) & self.bucket_mask)
            + Group::WIDTH;
        *self.ctrl(index) = ctrl;
        *self.ctrl(index2) = ctrl;
    }
    /// Searches for an empty or deleted bucket which is suitable for inserting
    /// a new element.
    ///
    /// There must be at least 1 empty bucket in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    fn find_insert_slot(&self, hash: u64) -> usize {
        for pos in self.probe_seq(hash) {
            unsafe {
                let group = Group::load(self.ctrl(pos));
                if let Some(bit) = group.match_empty_or_deleted().lowest_set_bit() {
                    let result = (pos + bit) & self.bucket_mask;
                    if unlikely(is_full(*self.ctrl(result))) {
                        debug_assert!(self.bucket_mask < Group::WIDTH);
                        debug_assert_ne!(pos, 0);
                        return Group::load_aligned(self.ctrl(0))
                            .match_empty_or_deleted()
                            .lowest_set_bit_nonzero();
                    } else {
                        return result;
                    }
                }
            }
        }
        unreachable!();
    }
    /// Marks all table buckets as empty without dropping their contents.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn clear_no_drop(&mut self) {
        if !self.is_empty_singleton() {
            unsafe {
                self.ctrl(0).write_bytes(EMPTY, self.num_ctrl_bytes());
            }
        }
        self.items = 0;
        self.growth_left = bucket_mask_to_capacity(self.bucket_mask);
    }
    /// Removes all elements from the table without freeing the backing memory.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn clear(&mut self) {
        let self_ = guard(self, |self_| self_.clear_no_drop());
        if mem::needs_drop::<T>() && self_.len() != 0 {
            unsafe {
                for item in self_.iter() {
                    item.drop();
                }
            }
        }
    }
    /// Shrinks the table to fit `max(self.len(), min_size)` elements.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn shrink_to(&mut self, min_size: usize, hasher: impl Fn(&T) -> u64) {
        let min_size = usize::max(self.items, min_size);
        if min_size == 0 {
            *self = Self::new();
            return;
        }
        let min_buckets = match capacity_to_buckets(min_size) {
            Some(buckets) => buckets,
            None => return,
        };
        if min_buckets < self.buckets() {
            if self.items == 0 {
                *self = Self::with_capacity(min_size)
            } else {
                if self.resize(min_size, hasher, Fallibility::Infallible).is_err() {
                    unsafe { hint::unreachable_unchecked() }
                }
            }
        }
    }
    /// Ensures that at least `additional` items can be inserted into the table
    /// without reallocation.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn reserve(&mut self, additional: usize, hasher: impl Fn(&T) -> u64) {
        if additional > self.growth_left {
            if self.reserve_rehash(additional, hasher, Fallibility::Infallible).is_err()
            {
                unsafe { hint::unreachable_unchecked() }
            }
        }
    }
    /// Tries to ensure that at least `additional` items can be inserted into
    /// the table without reallocation.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn try_reserve(
        &mut self,
        additional: usize,
        hasher: impl Fn(&T) -> u64,
    ) -> Result<(), TryReserveError> {
        if additional > self.growth_left {
            self.reserve_rehash(additional, hasher, Fallibility::Fallible)
        } else {
            Ok(())
        }
    }
    /// Out-of-line slow path for `reserve` and `try_reserve`.
    #[cold]
    #[inline(never)]
    fn reserve_rehash(
        &mut self,
        additional: usize,
        hasher: impl Fn(&T) -> u64,
        fallability: Fallibility,
    ) -> Result<(), TryReserveError> {
        let new_items = match self.items.checked_add(additional) {
            Some(new_items) => new_items,
            None => return Err(fallability.capacity_overflow()),
        };
        let full_capacity = bucket_mask_to_capacity(self.bucket_mask);
        if new_items <= full_capacity / 2 {
            self.rehash_in_place(hasher);
            Ok(())
        } else {
            self.resize(usize::max(new_items, full_capacity + 1), hasher, fallability)
        }
    }
    /// Rehashes the contents of the table in place (i.e. without changing the
    /// allocation).
    ///
    /// If `hasher` panics then some the table's contents may be lost.
    fn rehash_in_place(&mut self, hasher: impl Fn(&T) -> u64) {
        unsafe {
            for i in (0..self.buckets()).step_by(Group::WIDTH) {
                let group = Group::load_aligned(self.ctrl(i));
                let group = group.convert_special_to_empty_and_full_to_deleted();
                group.store_aligned(self.ctrl(i));
            }
            if self.buckets() < Group::WIDTH {
                self.ctrl(0).copy_to(self.ctrl(Group::WIDTH), self.buckets());
            } else {
                self.ctrl(0).copy_to(self.ctrl(self.buckets()), Group::WIDTH);
            }
            let mut guard = guard(
                self,
                |self_| {
                    if mem::needs_drop::<T>() {
                        for i in 0..self_.buckets() {
                            if *self_.ctrl(i) == DELETED {
                                self_.set_ctrl(i, EMPTY);
                                self_.bucket(i).drop();
                                self_.items -= 1;
                            }
                        }
                    }
                    self_
                        .growth_left = bucket_mask_to_capacity(self_.bucket_mask)
                        - self_.items;
                },
            );
            'outer: for i in 0..guard.buckets() {
                if *guard.ctrl(i) != DELETED {
                    continue;
                }
                'inner: loop {
                    let item = guard.bucket(i);
                    let hash = hasher(item.as_ref());
                    let new_i = guard.find_insert_slot(hash);
                    let probe_index = |pos: usize| {
                        (pos.wrapping_sub(guard.probe_seq(hash).pos) & guard.bucket_mask)
                            / Group::WIDTH
                    };
                    if likely(probe_index(i) == probe_index(new_i)) {
                        guard.set_ctrl(i, h2(hash));
                        continue 'outer;
                    }
                    let prev_ctrl = *guard.ctrl(new_i);
                    guard.set_ctrl(new_i, h2(hash));
                    if prev_ctrl == EMPTY {
                        guard.set_ctrl(i, EMPTY);
                        guard.bucket(new_i).copy_from_nonoverlapping(&item);
                        continue 'outer;
                    } else {
                        debug_assert_eq!(prev_ctrl, DELETED);
                        mem::swap(guard.bucket(new_i).as_mut(), item.as_mut());
                        continue 'inner;
                    }
                }
            }
            guard.growth_left = bucket_mask_to_capacity(guard.bucket_mask) - guard.items;
            mem::forget(guard);
        }
    }
    /// Allocates a new table of a different size and moves the contents of the
    /// current table into it.
    fn resize(
        &mut self,
        capacity: usize,
        hasher: impl Fn(&T) -> u64,
        fallability: Fallibility,
    ) -> Result<(), TryReserveError> {
        unsafe {
            debug_assert!(self.items <= capacity);
            let mut new_table = Self::fallible_with_capacity(capacity, fallability)?;
            new_table.growth_left -= self.items;
            new_table.items = self.items;
            let mut new_table = guard(
                ManuallyDrop::new(new_table),
                |new_table| {
                    if !new_table.is_empty_singleton() {
                        new_table.free_buckets();
                    }
                },
            );
            for item in self.iter() {
                let hash = hasher(item.as_ref());
                let index = new_table.find_insert_slot(hash);
                new_table.set_ctrl(index, h2(hash));
                new_table.bucket(index).copy_from_nonoverlapping(&item);
            }
            mem::swap(self, &mut new_table);
            Ok(())
        }
    }
    /// Inserts a new element into the table, and returns its raw bucket.
    ///
    /// This does not check if the given element already exists in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn insert(
        &mut self,
        hash: u64,
        value: T,
        hasher: impl Fn(&T) -> u64,
    ) -> Bucket<T> {
        unsafe {
            let mut index = self.find_insert_slot(hash);
            let old_ctrl = *self.ctrl(index);
            if unlikely(self.growth_left == 0 && special_is_empty(old_ctrl)) {
                self.reserve(1, hasher);
                index = self.find_insert_slot(hash);
            }
            let bucket = self.bucket(index);
            self.growth_left -= special_is_empty(old_ctrl) as usize;
            self.set_ctrl(index, h2(hash));
            bucket.write(value);
            self.items += 1;
            bucket
        }
    }
    /// Inserts a new element into the table, and returns a mutable reference to it.
    ///
    /// This does not check if the given element already exists in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn insert_entry(
        &mut self,
        hash: u64,
        value: T,
        hasher: impl Fn(&T) -> u64,
    ) -> &mut T {
        unsafe { self.insert(hash, value, hasher).as_mut() }
    }
    /// Inserts a new element into the table, without growing the table.
    ///
    /// There must be enough space in the table to insert the new element.
    ///
    /// This does not check if the given element already exists in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    #[cfg(any(feature = "raw", feature = "rustc-internal-api"))]
    pub fn insert_no_grow(&mut self, hash: u64, value: T) -> Bucket<T> {
        unsafe {
            let index = self.find_insert_slot(hash);
            let bucket = self.bucket(index);
            let old_ctrl = *self.ctrl(index);
            self.growth_left -= special_is_empty(old_ctrl) as usize;
            self.set_ctrl(index, h2(hash));
            bucket.write(value);
            self.items += 1;
            bucket
        }
    }
    /// Temporary removes a bucket, applying the given function to the removed
    /// element and optionally put back the returned value in the same bucket.
    ///
    /// Returns `true` if the bucket still contains an element
    ///
    /// This does not check if the given bucket is actually occupied.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn replace_bucket_with<F>(&mut self, bucket: Bucket<T>, f: F) -> bool
    where
        F: FnOnce(T) -> Option<T>,
    {
        let index = self.bucket_index(&bucket);
        let old_ctrl = *self.ctrl(index);
        debug_assert!(is_full(old_ctrl));
        let old_growth_left = self.growth_left;
        let item = self.remove(bucket);
        if let Some(new_item) = f(item) {
            self.growth_left = old_growth_left;
            self.set_ctrl(index, old_ctrl);
            self.items += 1;
            self.bucket(index).write(new_item);
            true
        } else {
            false
        }
    }
    /// Searches for an element in the table.
    #[inline]
    pub fn find(&self, hash: u64, mut eq: impl FnMut(&T) -> bool) -> Option<Bucket<T>> {
        unsafe {
            for bucket in self.iter_hash(hash) {
                let elm = bucket.as_ref();
                if likely(eq(elm)) {
                    return Some(bucket);
                }
            }
            None
        }
    }
    /// Gets a reference to an element in the table.
    #[inline]
    pub fn get(&self, hash: u64, eq: impl FnMut(&T) -> bool) -> Option<&T> {
        match self.find(hash, eq) {
            Some(bucket) => Some(unsafe { bucket.as_ref() }),
            None => None,
        }
    }
    /// Gets a mutable reference to an element in the table.
    #[inline]
    pub fn get_mut(&mut self, hash: u64, eq: impl FnMut(&T) -> bool) -> Option<&mut T> {
        match self.find(hash, eq) {
            Some(bucket) => Some(unsafe { bucket.as_mut() }),
            None => None,
        }
    }
    /// Returns the number of elements the map can hold without reallocating.
    ///
    /// This number is a lower bound; the table might be able to hold
    /// more, but is guaranteed to be able to hold at least this many.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn capacity(&self) -> usize {
        self.items + self.growth_left
    }
    /// Returns the number of elements in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn len(&self) -> usize {
        self.items
    }
    /// Returns the number of buckets in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn buckets(&self) -> usize {
        self.bucket_mask + 1
    }
    /// Returns the number of control bytes in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    fn num_ctrl_bytes(&self) -> usize {
        self.bucket_mask + 1 + Group::WIDTH
    }
    /// Returns whether this table points to the empty singleton with a capacity
    /// of 0.
    #[cfg_attr(feature = "inline-more", inline)]
    fn is_empty_singleton(&self) -> bool {
        self.bucket_mask == 0
    }
    /// Returns an iterator over every element in the table. It is up to
    /// the caller to ensure that the `RawTable` outlives the `RawIter`.
    /// Because we cannot make the `next` method unsafe on the `RawIter`
    /// struct, we have to make the `iter` method unsafe.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn iter(&self) -> RawIter<T> {
        let data = Bucket::from_base_index(self.data_end(), 0);
        RawIter {
            iter: RawIterRange::new(self.ctrl.as_ptr(), data, self.buckets()),
            items: self.items,
        }
    }
    /// Returns an iterator over occupied buckets that could match a given hash.
    ///
    /// In rare cases, the iterator may return a bucket with a different hash.
    ///
    /// It is up to the caller to ensure that the `RawTable` outlives the
    /// `RawIterHash`. Because we cannot make the `next` method unsafe on the
    /// `RawIterHash` struct, we have to make the `iter_hash` method unsafe.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn iter_hash(&self, hash: u64) -> RawIterHash<'_, T> {
        RawIterHash::new(self, hash)
    }
    /// Returns an iterator which removes all elements from the table without
    /// freeing the memory.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn drain(&mut self) -> RawDrain<'_, T> {
        unsafe {
            let iter = self.iter();
            self.drain_iter_from(iter)
        }
    }
    /// Returns an iterator which removes all elements from the table without
    /// freeing the memory.
    ///
    /// Iteration starts at the provided iterator's current location.
    ///
    /// It is up to the caller to ensure that the iterator is valid for this
    /// `RawTable` and covers all items that remain in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn drain_iter_from(&mut self, iter: RawIter<T>) -> RawDrain<'_, T> {
        debug_assert_eq!(iter.len(), self.len());
        RawDrain {
            iter,
            table: ManuallyDrop::new(mem::replace(self, Self::new())),
            orig_table: NonNull::from(self),
            marker: PhantomData,
        }
    }
    /// Returns an iterator which consumes all elements from the table.
    ///
    /// Iteration starts at the provided iterator's current location.
    ///
    /// It is up to the caller to ensure that the iterator is valid for this
    /// `RawTable` and covers all items that remain in the table.
    pub unsafe fn into_iter_from(self, iter: RawIter<T>) -> RawIntoIter<T> {
        debug_assert_eq!(iter.len(), self.len());
        let alloc = self.into_alloc();
        RawIntoIter {
            iter,
            alloc,
            marker: PhantomData,
        }
    }
    /// Converts the table into a raw allocation. The contents of the table
    /// should be dropped using a `RawIter` before freeing the allocation.
    #[cfg_attr(feature = "inline-more", inline)]
    pub(crate) fn into_alloc(self) -> Option<(NonNull<u8>, Layout)> {
        let alloc = if self.is_empty_singleton() {
            None
        } else {
            let (layout, ctrl_offset) = match calculate_layout::<T>(self.buckets()) {
                Some(lco) => lco,
                None => unsafe { hint::unreachable_unchecked() }
            };
            Some((
                unsafe { NonNull::new_unchecked(self.ctrl.as_ptr().sub(ctrl_offset)) },
                layout,
            ))
        };
        mem::forget(self);
        alloc
    }
}
unsafe impl<T> Send for RawTable<T>
where
    T: Send,
{}
unsafe impl<T> Sync for RawTable<T>
where
    T: Sync,
{}
impl<T: Clone> Clone for RawTable<T> {
    fn clone(&self) -> Self {
        if self.is_empty_singleton() {
            Self::new()
        } else {
            unsafe {
                let mut new_table = ManuallyDrop::new(
                    match Self::new_uninitialized(
                        self.buckets(),
                        Fallibility::Infallible,
                    ) {
                        Ok(table) => table,
                        Err(_) => hint::unreachable_unchecked(),
                    },
                );
                new_table
                    .clone_from_spec(
                        self,
                        |new_table| {
                            new_table.free_buckets();
                        },
                    );
                ManuallyDrop::into_inner(new_table)
            }
        }
    }
    fn clone_from(&mut self, source: &Self) {
        if source.is_empty_singleton() {
            *self = Self::new();
        } else {
            unsafe {
                if mem::needs_drop::<T>() && self.len() != 0 {
                    for item in self.iter() {
                        item.drop();
                    }
                }
                if self.buckets() != source.buckets() {
                    if !self.is_empty_singleton() {
                        self.free_buckets();
                    }
                    (self as *mut Self)
                        .write(
                            match Self::new_uninitialized(
                                source.buckets(),
                                Fallibility::Infallible,
                            ) {
                                Ok(table) => table,
                                Err(_) => hint::unreachable_unchecked(),
                            },
                        );
                }
                self.clone_from_spec(source, |self_| { self_.clear_no_drop() });
            }
        }
    }
}
/// Specialization of `clone_from` for `Copy` types
trait RawTableClone {
    unsafe fn clone_from_spec(&mut self, source: &Self, on_panic: impl FnMut(&mut Self));
}
impl<T: Clone> RawTableClone for RawTable<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    default_fn! {
        unsafe fn clone_from_spec(& mut self, source : & Self, on_panic : impl FnMut(&
        mut Self)) { self.clone_from_impl(source, on_panic); }
    }
}
#[cfg(feature = "nightly")]
impl<T: Copy> RawTableClone for RawTable<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn clone_from_spec(
        &mut self,
        source: &Self,
        _on_panic: impl FnMut(&mut Self),
    ) {
        source.ctrl(0).copy_to_nonoverlapping(self.ctrl(0), self.num_ctrl_bytes());
        source.data_start().copy_to_nonoverlapping(self.data_start(), self.buckets());
        self.items = source.items;
        self.growth_left = source.growth_left;
    }
}
impl<T: Clone> RawTable<T> {
    /// Common code for clone and clone_from. Assumes `self.buckets() == source.buckets()`.
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn clone_from_impl(
        &mut self,
        source: &Self,
        mut on_panic: impl FnMut(&mut Self),
    ) {
        source.ctrl(0).copy_to_nonoverlapping(self.ctrl(0), self.num_ctrl_bytes());
        let mut guard = guard(
            (0, &mut *self),
            |(index, self_)| {
                if mem::needs_drop::<T>() && self_.len() != 0 {
                    for i in 0..=*index {
                        if is_full(*self_.ctrl(i)) {
                            self_.bucket(i).drop();
                        }
                    }
                }
                on_panic(self_);
            },
        );
        for from in source.iter() {
            let index = source.bucket_index(&from);
            let to = guard.1.bucket(index);
            to.write(from.as_ref().clone());
            guard.0 = index;
        }
        mem::forget(guard);
        self.items = source.items;
        self.growth_left = source.growth_left;
    }
    /// Variant of `clone_from` to use when a hasher is available.
    #[cfg(feature = "raw")]
    pub fn clone_from_with_hasher(&mut self, source: &Self, hasher: impl Fn(&T) -> u64) {
        if self.buckets() != source.buckets()
            && bucket_mask_to_capacity(self.bucket_mask) >= source.len()
        {
            self.clear();
            let guard_self = guard(
                &mut *self,
                |self_| {
                    self_.clear();
                },
            );
            unsafe {
                for item in source.iter() {
                    let item = item.as_ref().clone();
                    let hash = hasher(&item);
                    let index = guard_self.find_insert_slot(hash);
                    guard_self.set_ctrl(index, h2(hash));
                    guard_self.bucket(index).write(item);
                }
            }
            mem::forget(guard_self);
            self.items = source.items;
            self.growth_left -= source.items;
        } else {
            self.clone_from(source);
        }
    }
}
#[cfg(feature = "nightly")]
unsafe impl<#[may_dangle] T> Drop for RawTable<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn drop(&mut self) {
        if !self.is_empty_singleton() {
            unsafe {
                if mem::needs_drop::<T>() && self.len() != 0 {
                    for item in self.iter() {
                        item.drop();
                    }
                }
                self.free_buckets();
            }
        }
    }
}
#[cfg(not(feature = "nightly"))]
impl<T> Drop for RawTable<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn drop(&mut self) {
        if !self.is_empty_singleton() {
            unsafe {
                if mem::needs_drop::<T>() && self.len() != 0 {
                    for item in self.iter() {
                        item.drop();
                    }
                }
                self.free_buckets();
            }
        }
    }
}
impl<T> IntoIterator for RawTable<T> {
    type Item = T;
    type IntoIter = RawIntoIter<T>;
    #[cfg_attr(feature = "inline-more", inline)]
    fn into_iter(self) -> RawIntoIter<T> {
        unsafe {
            let iter = self.iter();
            self.into_iter_from(iter)
        }
    }
}
/// Iterator over a sub-range of a table. Unlike `RawIter` this iterator does
/// not track an item count.
pub(crate) struct RawIterRange<T> {
    current_group: BitMask,
    data: Bucket<T>,
    next_ctrl: *const u8,
    end: *const u8,
}
impl<T> RawIterRange<T> {
    /// Returns a `RawIterRange` covering a subset of a table.
    ///
    /// The control byte address must be aligned to the group size.
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn new(ctrl: *const u8, data: Bucket<T>, len: usize) -> Self {
        debug_assert_ne!(len, 0);
        debug_assert_eq!(ctrl as usize % Group::WIDTH, 0);
        let end = ctrl.add(len);
        let current_group = Group::load_aligned(ctrl).match_full();
        let next_ctrl = ctrl.add(Group::WIDTH);
        Self {
            current_group,
            data,
            next_ctrl,
            end,
        }
    }
    /// Splits a `RawIterRange` into two halves.
    ///
    /// Returns `None` if the remaining range is smaller than or equal to the
    /// group width.
    #[cfg_attr(feature = "inline-more", inline)]
    #[cfg(feature = "rayon")]
    pub(crate) fn split(mut self) -> (Self, Option<RawIterRange<T>>) {
        unsafe {
            if self.end <= self.next_ctrl {
                (self, None)
            } else {
                let len = offset_from(self.end, self.next_ctrl);
                debug_assert_eq!(len % Group::WIDTH, 0);
                let mid = (len / 2) & !(Group::WIDTH - 1);
                let tail = Self::new(
                    self.next_ctrl.add(mid),
                    self.data.next_n(Group::WIDTH).next_n(mid),
                    len - mid,
                );
                debug_assert_eq!(
                    self.data.next_n(Group::WIDTH).next_n(mid).ptr, tail.data.ptr
                );
                debug_assert_eq!(self.end, tail.end);
                self.end = self.next_ctrl.add(mid);
                debug_assert_eq!(self.end.add(Group::WIDTH), tail.next_ctrl);
                (self, Some(tail))
            }
        }
    }
}
unsafe impl<T> Send for RawIterRange<T> {}
unsafe impl<T> Sync for RawIterRange<T> {}
impl<T> Clone for RawIterRange<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            next_ctrl: self.next_ctrl,
            current_group: self.current_group,
            end: self.end,
        }
    }
}
impl<T> Iterator for RawIterRange<T> {
    type Item = Bucket<T>;
    #[cfg_attr(feature = "inline-more", inline)]
    fn next(&mut self) -> Option<Bucket<T>> {
        unsafe {
            loop {
                if let Some(index) = self.current_group.lowest_set_bit() {
                    self.current_group = self.current_group.remove_lowest_bit();
                    return Some(self.data.next_n(index));
                }
                if self.next_ctrl >= self.end {
                    return None;
                }
                self.current_group = Group::load_aligned(self.next_ctrl).match_full();
                self.data = self.data.next_n(Group::WIDTH);
                self.next_ctrl = self.next_ctrl.add(Group::WIDTH);
            }
        }
    }
    #[cfg_attr(feature = "inline-more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(unsafe { offset_from(self.end, self.next_ctrl) + Group::WIDTH }))
    }
}
impl<T> FusedIterator for RawIterRange<T> {}
/// Iterator which returns a raw pointer to every full bucket in the table.
///
/// For maximum flexibility this iterator is not bound by a lifetime, but you
/// must observe several rules when using it:
/// - You must not free the hash table while iterating (including via growing/shrinking).
/// - It is fine to erase a bucket that has been yielded by the iterator.
/// - Erasing a bucket that has not yet been yielded by the iterator may still
///   result in the iterator yielding that bucket (unless `reflect_remove` is called).
/// - It is unspecified whether an element inserted after the iterator was
///   created will be yielded by that iterator (unless `reflect_insert` is called).
/// - The order in which the iterator yields bucket is unspecified and may
///   change in the future.
pub struct RawIter<T> {
    pub(crate) iter: RawIterRange<T>,
    items: usize,
}
impl<T> RawIter<T> {
    /// Refresh the iterator so that it reflects a removal from the given bucket.
    ///
    /// For the iterator to remain valid, this method must be called once
    /// for each removed bucket before `next` is called again.
    ///
    /// This method should be called _before_ the removal is made. It is not necessary to call this
    /// method if you are removing an item that this iterator yielded in the past.
    #[cfg(feature = "raw")]
    pub fn reflect_remove(&mut self, b: &Bucket<T>) {
        self.reflect_toggle_full(b, false);
    }
    /// Refresh the iterator so that it reflects an insertion into the given bucket.
    ///
    /// For the iterator to remain valid, this method must be called once
    /// for each insert before `next` is called again.
    ///
    /// This method does not guarantee that an insertion of a bucket witha greater
    /// index than the last one yielded will be reflected in the iterator.
    ///
    /// This method should be called _after_ the given insert is made.
    #[cfg(feature = "raw")]
    pub fn reflect_insert(&mut self, b: &Bucket<T>) {
        self.reflect_toggle_full(b, true);
    }
    /// Refresh the iterator so that it reflects a change to the state of the given bucket.
    #[cfg(feature = "raw")]
    fn reflect_toggle_full(&mut self, b: &Bucket<T>, is_insert: bool) {
        unsafe {
            if b.as_ptr() > self.iter.data.as_ptr() {
                return;
            }
            if self.iter.next_ctrl < self.iter.end
                && b.as_ptr() <= self.iter.data.next_n(Group::WIDTH).as_ptr()
            {
                if cfg!(debug_assertions) {
                    let offset = offset_from(self.iter.data.as_ptr(), b.as_ptr());
                    let ctrl = self.iter.next_ctrl.sub(Group::WIDTH).add(offset);
                    assert!(is_full(* ctrl));
                }
                if is_insert {
                    self.items += 1;
                } else {
                    self.items -= 1;
                }
                return;
            }
            if let Some(index) = self.iter.current_group.lowest_set_bit() {
                let next_bucket = self.iter.data.next_n(index);
                if b.as_ptr() > next_bucket.as_ptr() {} else {
                    let our_bit = offset_from(self.iter.data.as_ptr(), b.as_ptr());
                    let was_full = self.iter.current_group.flip(our_bit);
                    debug_assert_ne!(was_full, is_insert);
                    if is_insert {
                        self.items += 1;
                    } else {
                        self.items -= 1;
                    }
                    if cfg!(debug_assertions) {
                        if b.as_ptr() == next_bucket.as_ptr() {
                            debug_assert_ne!(
                                self.iter.current_group.lowest_set_bit(), Some(index)
                            );
                        } else {
                            debug_assert_eq!(
                                self.iter.current_group.lowest_set_bit(), Some(index)
                            );
                        }
                    }
                }
            } else {}
        }
    }
}
impl<T> Clone for RawIter<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            items: self.items,
        }
    }
}
impl<T> Iterator for RawIter<T> {
    type Item = Bucket<T>;
    #[cfg_attr(feature = "inline-more", inline)]
    fn next(&mut self) -> Option<Bucket<T>> {
        if let Some(b) = self.iter.next() {
            self.items -= 1;
            Some(b)
        } else {
            debug_assert_eq!(self.items, 0);
            None
        }
    }
    #[cfg_attr(feature = "inline-more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.items, Some(self.items))
    }
}
impl<T> ExactSizeIterator for RawIter<T> {}
impl<T> FusedIterator for RawIter<T> {}
/// Iterator which consumes a table and returns elements.
pub struct RawIntoIter<T> {
    iter: RawIter<T>,
    alloc: Option<(NonNull<u8>, Layout)>,
    marker: PhantomData<T>,
}
impl<T> RawIntoIter<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn iter(&self) -> RawIter<T> {
        self.iter.clone()
    }
}
unsafe impl<T> Send for RawIntoIter<T>
where
    T: Send,
{}
unsafe impl<T> Sync for RawIntoIter<T>
where
    T: Sync,
{}
#[cfg(feature = "nightly")]
unsafe impl<#[may_dangle] T> Drop for RawIntoIter<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn drop(&mut self) {
        unsafe {
            if mem::needs_drop::<T>() && self.iter.len() != 0 {
                while let Some(item) = self.iter.next() {
                    item.drop();
                }
            }
            if let Some((ptr, layout)) = self.alloc {
                dealloc(ptr.as_ptr(), layout);
            }
        }
    }
}
#[cfg(not(feature = "nightly"))]
impl<T> Drop for RawIntoIter<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn drop(&mut self) {
        unsafe {
            if mem::needs_drop::<T>() && self.iter.len() != 0 {
                while let Some(item) = self.iter.next() {
                    item.drop();
                }
            }
            if let Some((ptr, layout)) = self.alloc {
                dealloc(ptr.as_ptr(), layout);
            }
        }
    }
}
impl<T> Iterator for RawIntoIter<T> {
    type Item = T;
    #[cfg_attr(feature = "inline-more", inline)]
    fn next(&mut self) -> Option<T> {
        unsafe { Some(self.iter.next()?.read()) }
    }
    #[cfg_attr(feature = "inline-more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<T> ExactSizeIterator for RawIntoIter<T> {}
impl<T> FusedIterator for RawIntoIter<T> {}
/// Iterator which consumes elements without freeing the table storage.
pub struct RawDrain<'a, T> {
    iter: RawIter<T>,
    table: ManuallyDrop<RawTable<T>>,
    orig_table: NonNull<RawTable<T>>,
    marker: PhantomData<&'a RawTable<T>>,
}
impl<T> RawDrain<'_, T> {
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn iter(&self) -> RawIter<T> {
        self.iter.clone()
    }
}
unsafe impl<T> Send for RawDrain<'_, T>
where
    T: Send,
{}
unsafe impl<T> Sync for RawDrain<'_, T>
where
    T: Sync,
{}
impl<T> Drop for RawDrain<'_, T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn drop(&mut self) {
        unsafe {
            if mem::needs_drop::<T>() && self.iter.len() != 0 {
                while let Some(item) = self.iter.next() {
                    item.drop();
                }
            }
            self.table.clear_no_drop();
            self.orig_table.as_ptr().copy_from_nonoverlapping(&*self.table, 1);
        }
    }
}
impl<T> Iterator for RawDrain<'_, T> {
    type Item = T;
    #[cfg_attr(feature = "inline-more", inline)]
    fn next(&mut self) -> Option<T> {
        unsafe {
            let item = self.iter.next()?;
            Some(item.read())
        }
    }
    #[cfg_attr(feature = "inline-more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<T> ExactSizeIterator for RawDrain<'_, T> {}
impl<T> FusedIterator for RawDrain<'_, T> {}
/// Iterator over occupied buckets that could match a given hash.
///
/// In rare cases, the iterator may return a bucket with a different hash.
pub struct RawIterHash<'a, T> {
    table: &'a RawTable<T>,
    h2_hash: u8,
    probe_seq: ProbeSeq,
    pos: usize,
    group: Group,
    bitmask: BitMaskIter,
}
impl<'a, T> RawIterHash<'a, T> {
    fn new(table: &'a RawTable<T>, hash: u64) -> Self {
        unsafe {
            let h2_hash = h2(hash);
            let mut probe_seq = table.probe_seq(hash);
            let pos = probe_seq.next().unwrap();
            let group = Group::load(table.ctrl(pos));
            let bitmask = group.match_byte(h2_hash).into_iter();
            RawIterHash {
                table,
                h2_hash,
                probe_seq,
                pos,
                group,
                bitmask,
            }
        }
    }
}
impl<'a, T> Iterator for RawIterHash<'a, T> {
    type Item = Bucket<T>;
    fn next(&mut self) -> Option<Bucket<T>> {
        unsafe {
            loop {
                if let Some(bit) = self.bitmask.next() {
                    let index = (self.pos + bit) & self.table.bucket_mask;
                    let bucket = self.table.bucket(index);
                    return Some(bucket);
                }
                if likely(self.group.match_empty().any_bit_set()) {
                    return None;
                }
                self.pos = self.probe_seq.next().unwrap();
                self.group = Group::load(self.table.ctrl(self.pos));
                self.bitmask = self.group.match_byte(self.h2_hash).into_iter();
            }
        }
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: bool = rug_fuzz_0;
        crate::raw::likely(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: bool = rug_fuzz_0;
        crate::raw::unlikely(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    use std::mem;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: *const i32 = rug_fuzz_0 as *const i32;
        let mut p1: *const i32 = rug_fuzz_1 as *const i32;
        unsafe {
            crate::raw::offset_from(p0, p1);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    #[test]
    fn test_is_full() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(crate ::raw::is_full(p0), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    #[test]
    fn test_raw() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u8 = rug_fuzz_0;
        debug_assert_eq!(is_special(p0), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::raw::{special_is_empty, is_special};
    #[test]
    fn test_special_is_empty() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let ctrl: u8 = rug_fuzz_0;
        debug_assert_eq!(special_is_empty(ctrl), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    use crate::raw;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u64 = rug_fuzz_0;
        raw::h1(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use std::mem;
    use crate::raw;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: u64 = rug_fuzz_0;
        crate::raw::h2(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::raw;
    #[test]
    fn test_capacity_to_buckets() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        crate::raw::capacity_to_buckets(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        crate::raw::bucket_mask_to_capacity(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::raw::Group;
    use std::mem;
    use std::alloc::Layout;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let buckets: usize = rug_fuzz_0;
        let p0 = buckets;
        crate::raw::calculate_layout::<usize>(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use crate::raw::Fallibility;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_12_rrrruuuugggg_test_rug = 0;
        let mut p0 = Fallibility::Fallible;
        p0.capacity_overflow();
        let _rug_ed_tests_rug_12_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    use crate::raw::Fallibility;
    use core::alloc::Layout;
    use core::mem;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_13_rrrruuuugggg_test_rug = 0;
        let mut p0 = Fallibility::Fallible;
        let mut p1 = Layout::from_size_align(
                mem::size_of::<RawTable<u8>>(),
                mem::align_of::<RawTable<u8>>(),
            )
            .unwrap();
        Fallibility::alloc_err(p0, p1);
        let _rug_ed_tests_rug_13_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_ctrl() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<i32> = RawTable::new();
        let p1: usize = rug_fuzz_0;
        unsafe {
            p0.ctrl(p1);
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_34 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<i32> = RawTable::new();
        let mut p1: usize = rug_fuzz_0;
        unsafe { RawTable::<i32>::bucket(&p0, p1) };
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_38 {
    use super::*;
    use crate::raw::RawTable;
    struct SampleType;
    impl SampleType {
        fn sample_impl(&mut self, val: &u64) -> bool {
            true
        }
    }
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<SampleType> = unimplemented!();
        let mut p1: u64 = rug_fuzz_0;
        let mut p2 = |val: &SampleType| SampleType.sample_impl(&p1);
        p0.remove_entry(p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_probe_seq() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<u32> = RawTable::new();
        let p1: u64 = rug_fuzz_0;
        p0.probe_seq(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<_> = RawTable::<u32>::with_capacity(rug_fuzz_0);
        let p1: u64 = rug_fuzz_1;
        p0.find_insert_slot(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_42_rrrruuuugggg_test_rug = 0;
        let mut p0: RawTable<i32> = RawTable::new();
        RawTable::<i32>::clear_no_drop(&mut p0);
        let _rug_ed_tests_rug_42_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_43 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_clear() {
        let _rug_st_tests_rug_43_rrrruuuugggg_test_clear = 0;
        let mut p0: RawTable<i32> = RawTable::new();
        RawTable::<i32>::clear(&mut p0);
        let _rug_ed_tests_rug_43_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::raw::RawTable;
    use crate::raw::Fallibility;
    use std::hint;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<i32> = RawTable::new();
        let p1: usize = rug_fuzz_0;
        let p2 = |val: &i32| -> u64 { (*val as u64) };
        p0.reserve(p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_47 {
    use super::*;
    use crate::raw::{RawTable, Fallibility};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<i32> = RawTable::new();
        let mut p1: usize = rug_fuzz_0;
        let mut hasher = |val: &i32| -> u64 { (*val as u64) };
        let mut p3 = Fallibility::Fallible;
        p0.reserve_rehash(p1, &hasher, p3);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::raw::{RawTable, Fallibility};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<_> = RawTable::new();
        let p1: usize = rug_fuzz_0;
        let p2 = |&_: &()| rug_fuzz_1;
        let mut p3 = Fallibility::Fallible;
        p0.resize(p1, p2, p3);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    struct Foo {
        value: u32,
    }
    impl Foo {
        fn new(value: u32) -> Self {
            Foo { value }
        }
    }
    #[test]
    fn test_raw_table_insert_entry() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut table = crate::raw::RawTable::<Foo>::new();
        let hash: u64 = rug_fuzz_0;
        let value = Foo::new(rug_fuzz_1);
        let hasher = |f: &Foo| -> u64 { f.value as u64 };
        table.insert_entry(hash, value, hasher);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::raw::RawTable;
    use core::marker::Sized;
    use core::ops::FnMut;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: RawTable<u32> = RawTable::<u32>::new();
        let mut p1: u64 = rug_fuzz_0;
        let mut p2 = |x: &u32| -> bool { *x == rug_fuzz_1 };
        p0.find(p1, &mut p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: crate::raw::RawTable<u32> = unimplemented!();
        let p1: u64 = rug_fuzz_0;
        let p2 = |val: &u32| *val == rug_fuzz_1;
        p0.get_mut(p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_capacity() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_capacity = 0;
        let mut p0: RawTable<u32> = RawTable::<u32>::new();
        p0.capacity();
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_capacity = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_57_rrrruuuugggg_test_rug = 0;
        let mut p0: RawTable<i32> = RawTable::new();
        RawTable::<i32>::len(&p0);
        let _rug_ed_tests_rug_57_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_buckets() {
        let _rug_st_tests_rug_58_rrrruuuugggg_test_buckets = 0;
        let mut p0: RawTable<u32> = RawTable::new();
        RawTable::<u32>::buckets(&p0);
        let _rug_ed_tests_rug_58_rrrruuuugggg_test_buckets = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_num_ctrl_bytes() {
        let _rug_st_tests_rug_59_rrrruuuugggg_test_num_ctrl_bytes = 0;
        let mut p0: RawTable<i32> = RawTable::new();
        crate::raw::RawTable::<i32>::num_ctrl_bytes(&p0);
        let _rug_ed_tests_rug_59_rrrruuuugggg_test_num_ctrl_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_60_rrrruuuugggg_test_rug = 0;
        let p0: RawTable<u32> = RawTable::new();
        p0.is_empty_singleton();
        let _rug_ed_tests_rug_60_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_rug = 0;
        let mut p0: RawTable<u32> = RawTable::new();
        p0.drain();
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_66 {
    use super::*;
    use crate::raw::RawTable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_66_rrrruuuugggg_test_rug = 0;
        let mut p0: RawTable<u32> = RawTable::new();
        p0.into_alloc();
        let _rug_ed_tests_rug_66_rrrruuuugggg_test_rug = 0;
    }
}
