/*! Representation of the `BitSlice` region memory model

This module allows any `BitSlice` region to be decomposed into domains with
more detailed aliasing information.

Specifically, any particular `BitSlice` region is one of:

- touches only interior indices of one element
- touches at least one edge index of any number of elements (including zero)

In the latter case, any elements *completely* spanned by the slice handle are
known to not have any other write-capable views to them, and in the case of an
`&mut BitSlice` handle specifically, no other views at all. As such, the domain
view of this memory is able to remove the aliasing marker type and permit direct
memory access for the duration of its existence.
!*/
use crate::{
    devel as dvl, index::{BitIdx, BitTail},
    mem::BitMemory, order::BitOrder, slice::BitSlice, store::BitStore,
};
use core::{
    fmt::{self, Binary, Debug, Formatter, LowerHex, Octal, UpperHex},
    slice,
};
use wyz::{fmt::FmtForward, pipe::Pipe, tap::Tap};
macro_rules! bit_domain {
    ($t:ident $(=> $m:ident)? $(@ $a:ident)?) => {
        #[doc = " Granular representation of the memory region containing a"] #[doc =
        " `BitSlice`."] #[doc = ""] #[doc =
        " `BitSlice` regions can be described in terms of edge and center"] #[doc =
        " elements, where the edge elements retain the aliasing status of the"] #[doc =
        " source `BitSlice` handle, and the center elements are known to be"] #[doc =
        " completely unaliased by any other view. This property allows any"] #[doc =
        " `BitSlice` handle to be decomposed into smaller regions, and safely"] #[doc =
        " remove any aliasing markers from the subregion of memory that no"] #[doc =
        " longer requires them for correct access."] #[doc = ""] #[doc =
        " This enum acts like the `.split*` methods in that it only subdivides"] #[doc =
        " the source `BitSlice` into smaller `BitSlices`, and makes"] #[doc =
        " appropriate modifications to the aliasing markers. It does not"] #[doc =
        " provide references to the underlying memory elements. If you need"] #[doc =
        " such access directly, use the [`Domain`] or [`DomainMut`] enums."] #[doc = ""]
        #[doc = " # Lifetimes"] #[doc = ""] #[doc =
        " - `'a`: The lifetime of the referent storage region."] #[doc = ""] #[doc =
        " # Type Parameters"] #[doc = ""] #[doc =
        " - `O`: The ordering type of the source `BitSlice` handle."] #[doc =
        " - `T`: The element type of the source `BitSlice` handle, including"] #[doc =
        "   aliasing markers."] #[doc = ""] #[doc = " [`Domain`]: enum.Domain.html"]
        #[doc = " [`DomainMut`]: enum.DomainMut.html"] #[derive(Debug)] pub enum $t <'a,
        O, T > where O : BitOrder, T : 'a + BitStore { #[doc =
        " Indicates that a `BitSlice` is contained entirely in the"] #[doc =
        " interior indices of a single memory element."] Enclave { #[doc =
        " The start index of the `BitSlice`."] #[doc = ""] #[doc =
        " This is not likely to be useful information, but is retained"] #[doc =
        " for structural similarity with the rest of the module."] head : BitIdx < T::Mem
        >, #[doc = " The original `BitSlice` used to create this bit-domain view."] body
        : &'a $($m)? BitSlice < O, T >, #[doc = " The end index of the `BitSlice`."]
        #[doc = ""] #[doc =
        " This is not likely to be useful information, but is retained"] #[doc =
        " for structural similarity with the rest of the module."] tail : BitTail <
        T::Mem >, }, #[doc =
        " Indicates that a `BitSlice` region touches at least one edge"] #[doc =
        " index of any number of elements."] #[doc = ""] #[doc =
        " This contains two bitslices representing the partially-occupied"] #[doc =
        " edge elements, with their original aliasing marker, and one"] #[doc =
        " bitslice representing the fully-occupied interior elements,"] #[doc =
        " marked as unaliased."] Region { #[doc =
        " Any bits that partially-fill the base element of the slice"] #[doc =
        " region."] #[doc = ""] #[doc =
        " This does not modify its aliasing status, as it will already"] #[doc =
        " be appropriately marked before constructing this view."] head : &'a $($m)?
        BitSlice < O, T >, #[doc =
        " Any bits inside elements that the source bitslice completely"] #[doc =
        " covers."] #[doc = ""] #[doc =
        " This is marked as unaliased, because it is statically"] #[doc =
        " impossible for any other handle to have write access to the"] #[doc =
        " region it covers. As such, a bitslice that was marked as"] #[doc =
        " entirely aliased, but contains interior unaliased elements,"] #[doc =
        " can safely remove its aliasing protections."] #[doc = ""] #[doc =
        " # Safety Exception"] #[doc = ""] #[doc =
        " `&BitSlice<O, T::Alias>` references have access to a"] #[doc =
        " `.set_aliased` method, which represents the only means in"] #[doc =
        " `bitvec` of writing to memory without an exclusive `&mut `"] #[doc =
        " reference."] #[doc = ""] #[doc =
        " Construction of two such shared, aliasing, references over"] #[doc =
        " the same data, then construction of a bit-domain view over"] #[doc =
        " one of them and simultaneous writing through the other to"] #[doc =
        " interior elements marked as unaliased, will cause the"] #[doc =
        " bit-domain view to be undefined behavior. Do not combine"] #[doc =
        " bit-domain views and `.set_aliased` calls."] body : &'a $($m)? BitSlice < O,
        T::Mem >, #[doc = " Any bits that partially fill the last element of the slice"]
        #[doc = " region."] #[doc = ""] #[doc =
        " This does not modify its aliasing status, as it will already"] #[doc =
        " be appropriately marked before constructing this view."] tail : &'a $($m)?
        BitSlice < O, T >, }, } impl <'a, O, T > $t <'a, O, T > where O : BitOrder, T :
        'a + BitStore, { #[doc = " Attempts to view the domain as an enclave variant."]
        #[doc = ""] #[doc = " # Parameters"] #[doc = ""] #[doc = " - `self`"] #[doc = ""]
        #[doc = " # Returns"] #[doc = ""] #[doc =
        " If `self` is the [`Enclave`] variant, this returns `Some` of the"] #[doc =
        " enclave fields, as a tuple. Otherwise, it returns `None`."] #[doc = ""] #[doc =
        " [`Enclave`]: #variant.Enclave"] #[inline] pub fn enclave(self) -> Option <
        (BitIdx < T::Mem >, &'a $($m)? BitSlice < O, T >, BitTail < T::Mem >,) > { if let
        Self::Enclave { head, body, tail } = self { Some((head, body, tail)) } else {
        None } } #[doc = " Attempts to view the domain as a region variant."] #[doc = ""]
        #[doc = " # Parameters"] #[doc = ""] #[doc = " - `self`"] #[doc = ""] #[doc =
        " # Returns"] #[doc = ""] #[doc =
        " If `self` is the [`Region`] variant, this returns `Some` of the"] #[doc =
        " region fields, as a tuple. Otherwise, it returns `None`."] #[doc = ""] #[doc =
        " [`Region`]: #variant.Region"] #[inline] pub fn region(self) -> Option < (&'a
        $($m)? BitSlice < O, T >, &'a $($m)? BitSlice < O, T::Mem >, &'a $($m)? BitSlice
        < O, T >,) > { if let Self::Region { head, body, tail } = self { Some((head,
        body, tail)) } else { None } } #[doc =
        " Constructs a bit-domain view from a bitslice."] #[doc = ""] #[doc =
        " # Parameters"] #[doc = ""] #[doc =
        " - `slice`: The source bitslice for which the view is constructed"] #[doc = ""]
        #[doc = " # Returns"] #[doc = ""] #[doc =
        " A bit-domain view over the source slice."] #[inline] pub (crate) fn new(slice :
        &'a $($m)? BitSlice < O, T >) -> Self { let bitptr = slice.bitptr(); let h =
        bitptr.head(); let (e, t) = h.span(bitptr.len()); let w = T::Mem::BITS; match (h
        .value(), e, t.value()) { (_, 0, _) => Self::empty(), (0, _, t) if t == w =>
        Self::spanning(slice), (_, _, t) if t == w => Self::partial_head(slice, h), (0,
        ..) => Self::partial_tail(slice, h, t), (_, 1, _) => Self::minor(slice, h, t), _
        => Self::major(slice, h, t), } } #[inline] fn empty() -> Self { Self::Region {
        head : Default::default(), body : Default::default(), tail : Default::default(),
        } } #[inline] fn major(slice : &'a $($m)? BitSlice < O, T >, head : BitIdx <
        T::Mem >, tail : BitTail < T::Mem >,) -> Self { let (head, rest) =
        bit_domain!(split $($m)? slice, (T::Mem::BITS - head.value()) as usize,); let
        (body, tail) = bit_domain!(split $($m)? rest, rest.len() - (tail.value() as
        usize),); Self::Region { head : bit_domain!(retype $($m)? head), body :
        bit_domain!(retype $($m)? body), tail : bit_domain!(retype $($m)? tail), } }
        #[inline] fn minor(slice : &'a $($m)? BitSlice < O, T >, head : BitIdx < T::Mem
        >, tail : BitTail < T::Mem >,) -> Self { Self::Enclave { head, body : slice,
        tail, } } #[inline] fn partial_head(slice : &'a $($m)? BitSlice < O, T >, head :
        BitIdx < T::Mem >,) -> Self { let (head, rest) = bit_domain!(split $($m)? slice,
        (T::Mem::BITS - head.value()) as usize,); let (head, body) = (bit_domain!(retype
        $($m)? head), bit_domain!(retype $($m)? rest),); Self::Region { head, body, tail
        : Default::default(), } } #[inline] fn partial_tail(slice : &'a $($m)? BitSlice <
        O, T >, _head : BitIdx < T::Mem >, tail : BitTail < T::Mem >,) -> Self { let
        (rest, tail) = bit_domain!(split $($m)? slice, slice.len() - (tail.value() as
        usize),); let (body, tail) = (bit_domain!(retype $($m)? rest), bit_domain!(retype
        $($m)? tail),); Self::Region { head : Default::default(), body, tail, } }
        #[inline] fn spanning(slice : &'a $($m)? BitSlice < O, T >) -> Self {
        Self::Region { head : Default::default(), body : bit_domain!(retype $($m)?
        slice), tail : Default::default(), } } }
    };
    (retype $slice:ident $(,)?) => {
        unsafe { &* ($slice as * const _ as * const _) }
    };
    (retype mut $slice:ident $(,)?) => {
        unsafe { & mut * ($slice as * mut _ as * mut _) }
    };
    (split $slice:ident, $at:expr $(,)?) => {
        unsafe { $slice .split_at_unchecked($at) }
    };
    (split mut $slice:ident, $at:expr $(,)?) => {
        unsafe { $slice .split_at_unchecked_mut($at) }
    };
}
bit_domain!(BitDomain);
bit_domain!(BitDomainMut => mut @ Alias);
impl<O, T> Clone for BitDomain<'_, O, T>
where
    O: BitOrder,
    T: BitStore,
{
    #[inline(always)]
    #[cfg(not(tarpaulin_include))]
    fn clone(&self) -> Self {
        *self
    }
}
impl<O, T> Copy for BitDomain<'_, O, T>
where
    O: BitOrder,
    T: BitStore,
{}
macro_rules! domain {
    ($t:ident $(=> $m:ident)?) => {
        #[doc = " Granular representation of the memory region containing a"] #[doc =
        " `BitSlice`."] #[doc = ""] #[doc =
        " `BitSlice` regions can be described in terms of edge and center"] #[doc =
        " elements, where the edge elements retain the aliasing status of the"] #[doc =
        " source `BitSlice` handle, and the center elements are known to be"] #[doc =
        " completely unaliased by any other view. This property allows any"] #[doc =
        " `BitSlice` handle to be decomposed into smaller regions, and safely"] #[doc =
        " remove any aliasing markers from the subregion of memory that no"] #[doc =
        " longer requires them for correct access."] #[doc = ""] #[doc =
        " This enum splits the element region backing a `BitSlice` into"] #[doc =
        " maybe-aliased and known-unaliased subslices. If you do not need to"] #[doc =
        " work directly with the memory elements, and only need to firmly"] #[doc =
        " specify the aliasing status of a `BitSlice`, see the [`BitDomain`]"] #[doc =
        " and [`BitDomainMut`] enums."] #[doc = ""] #[doc = " # Lifetimes"] #[doc = ""]
        #[doc = " - `'a`: The lifetime of the referent storage region."] #[doc = ""]
        #[doc = " # Type Parameters"] #[doc = ""] #[doc =
        " - `T`: The element type of the source `BitSlice` handle, including"] #[doc =
        "   aliasing markers."] #[doc = ""] #[doc =
        " [`BitDomain`]: enum.BitDomain.html"] #[doc =
        " [`BitDomainMut`]: enum.BitDomainMut.html"] #[derive(Debug)] pub enum $t <'a, T
        > where T : 'a + BitStore, { #[doc =
        " Indicates that a `BitSlice` is contained entirely in the"] #[doc =
        " interior indices of a single memory element."] Enclave { #[doc =
        " The start index of the `BitSlice`."] head : BitIdx < T::Mem >, #[doc =
        " An aliased view of the element containing the `BitSlice`."] #[doc = ""] #[doc =
        " This is necessary even on immutable views, because other"] #[doc =
        " views to the referent element may be permitted to modify it."] elem : &'a
        T::Alias, #[doc = " The end index of the `BitSlice`."] tail : BitTail < T::Mem >,
        }, #[doc = " Indicates that a `BitSlice` region touches at least one edge"] #[doc
        = " index of any number of elements."] #[doc = ""] #[doc =
        " This contains two optional references to the aliased edges, and"] #[doc =
        " one reference to the unaliased middle. Each can be queried and"] #[doc =
        " used individually."] Region { #[doc =
        " If the `BitSlice` started in the interior of its first"] #[doc =
        " element, this contains the starting index and the base"] #[doc = " address."]
        head : Option < (BitIdx < T::Mem >, &'a T::Alias) >, #[doc =
        " All fully-spanned, unaliased, elements."] #[doc = ""] #[doc =
        " This is marked as bare memory without any access"] #[doc =
        " protections, because it is statically impossible for any"] #[doc =
        " other handle to have write access to the region it covers."] #[doc =
        " As such, a bitslice that was marked as entirely aliased, but"] #[doc =
        " contains interior unaliased elements, can safely remove its"] #[doc =
        " aliasing protections."] #[doc = ""] #[doc = " # Safety Exception"] #[doc = ""]
        #[doc = " `&BitSlice<O, T::Alias>` references have access to a"] #[doc =
        " `.set_aliased` method, which represents the only means in"] #[doc =
        " `bitvec` of writing to memory without an exclusive `&mut `"] #[doc =
        " reference."] #[doc = ""] #[doc =
        " Construction of two such shared, aliasing, references over"] #[doc =
        " the same data, then construction of a domain view over one"] #[doc =
        " of them and simultaneous writing through the other to"] #[doc =
        " interior elements marked as unaliased, will cause the domain"] #[doc =
        " view to be undefined behavior. Do not combine domain views"] #[doc =
        " and `.set_aliased` calls."] body : &'a $($m)? [T::Mem], #[doc =
        " If the `BitSlice` ended in the interior of its last element,"] #[doc =
        " this contains the ending index and the last address."] tail : Option < (&'a
        T::Alias, BitTail < T::Mem >) >, } } impl <'a, T > $t <'a, T > where T : 'a +
        BitStore, { #[doc = " Attempts to view the domain as an enclave variant."] #[doc
        = ""] #[doc = " # Parameters"] #[doc = ""] #[doc = " - `self`"] #[doc = ""] #[doc
        = " # Returns"] #[doc = ""] #[doc =
        " If `self` is the [`Enclave`] variant, this returns `Some` of the"] #[doc =
        " enclave fields, as a tuple. Otherwise, it returns `None`."] #[doc = ""] #[doc =
        " [`Enclave`]: #variant.Enclave"] #[inline] pub fn enclave(self) -> Option <
        (BitIdx < T::Mem >, &'a T::Alias, BitTail < T::Mem >,) > { if let Self::Enclave {
        head, elem, tail } = self { Some((head, elem, tail)) } else { None } } #[doc =
        " Attempts to view the domain as the region variant."] #[doc = ""] #[doc =
        " # Parameters"] #[doc = ""] #[doc = " - `self`"] #[doc = ""] #[doc =
        " # Returns"] #[doc = ""] #[doc =
        " If `self` is the [`Region`] variant, this returns `Some` of the"] #[doc =
        " region fields, as a tuple. Otherwise, it returns `None`."] #[doc = ""] #[doc =
        " [`Region`]: #variant.Region"] #[inline] pub fn region(self) -> Option < (Option
        < (BitIdx < T::Mem >, &'a T::Alias) >, &'a $($m)? [T::Mem], Option < (&'a
        T::Alias, BitTail < T::Mem >) >,) > { if let Self::Region { head, body, tail } =
        self { Some((head, body, tail)) } else { None } } #[inline] pub (crate) fn new <
        O > (slice : &'a $($m)? BitSlice < O, T >) -> Self where O : BitOrder { let
        bitptr = slice.bitptr(); let head = bitptr.head(); let elts = bitptr.elements();
        let tail = bitptr.tail(); let bits = T::Mem::BITS; let base = bitptr.pointer()
        .to_alias(); match (head.value(), elts, tail.value()) { (_, 0, _) =>
        Self::empty(), (0, _, t) if t == bits => Self::spanning(base, elts), (_, _, t) if
        t == bits => Self::partial_head(base, elts, head), (0, ..) =>
        Self::partial_tail(base, elts, tail), (_, 1, _) => Self::minor(base, head, tail),
        _ => Self::major(base, elts, head, tail), } } #[inline] fn empty() -> Self {
        Self::Region { head : None, body : & $($m)? [], tail : None, } } #[inline] fn
        major(base : * const T::Alias, elts : usize, head : BitIdx < T::Mem >, tail :
        BitTail < T::Mem >,) -> Self { let h = unsafe { &* base }; let t = unsafe { &*
        base.add(elts - 1) }; let body = domain!(slice $($m)? base.add(1), elts - 2);
        Self::Region { head : Some((head, h)), body, tail : Some((t, tail)), } }
        #[inline] fn minor(addr : * const T::Alias, head : BitIdx < T::Mem >, tail :
        BitTail < T::Mem >,) -> Self { Self::Enclave { head, elem : unsafe { &* addr },
        tail, } } #[inline] fn partial_head(base : * const T::Alias, elts : usize, head :
        BitIdx < T::Mem >,) -> Self { let h = unsafe { &* base }; let body =
        domain!(slice $($m)? base.add(1), elts - 1); Self::Region { head : Some((head,
        h)), body, tail : None, } } #[inline] fn partial_tail(base : * const T::Alias,
        elts : usize, tail : BitTail < T::Mem >,) -> Self { let t = unsafe { &* base
        .add(elts - 1) }; let body = domain!(slice $($m)? base, elts - 1); Self::Region {
        head : None, body, tail : Some((t, tail)), } } #[inline] fn spanning(base : *
        const T::Alias, elts : usize) -> Self { Self::Region { head : None, body :
        domain!(slice $($m)? base, elts), tail : None, } } }
    };
    (slice $base:expr, $elts:expr) => {
        unsafe { slice::from_raw_parts($base as * const _, $elts) }
    };
    (slice mut $base:expr, $elts:expr) => {
        unsafe { slice::from_raw_parts_mut($base as * const _ as * mut _, $elts) }
    };
}
domain!(Domain);
domain!(DomainMut => mut);
impl<T> Clone for Domain<'_, T>
where
    T: BitStore,
{
    #[inline(always)]
    #[cfg(not(tarpaulin_include))]
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, T> Iterator for Domain<'a, T>
where
    T: 'a + BitStore,
{
    type Item = T::Mem;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Enclave { elem, .. } => {
                (*elem)
                    .pipe(dvl::load_aliased_local::<T>)
                    .pipe(Some)
                    .tap(|_| *self = Self::empty())
            }
            Self::Region { head, body, tail } => {
                if let Some((_, elem)) = *head {
                    return elem
                        .pipe(dvl::load_aliased_local::<T>)
                        .pipe(Some)
                        .tap(|_| *head = None);
                }
                if let Some((elem, rest)) = body.split_first() {
                    *body = rest;
                    return Some(*elem);
                }
                if let Some((elem, _)) = *tail {
                    return elem
                        .pipe(dvl::load_aliased_local::<T>)
                        .pipe(Some)
                        .tap(|_| *tail = None);
                }
                None
            }
        }
    }
}
impl<'a, T> DoubleEndedIterator for Domain<'a, T>
where
    T: 'a + BitStore,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Self::Enclave { elem, .. } => {
                (*elem)
                    .pipe(dvl::load_aliased_local::<T>)
                    .pipe(Some)
                    .tap(|_| *self = Self::empty())
            }
            Self::Region { head, body, tail } => {
                if let Some((elem, _)) = *tail {
                    return elem
                        .pipe(dvl::load_aliased_local::<T>)
                        .pipe(Some)
                        .tap(|_| *tail = None);
                }
                if let Some((elem, rest)) = body.split_last() {
                    *body = rest;
                    return Some(*elem);
                }
                if let Some((_, elem)) = *head {
                    return elem
                        .pipe(dvl::load_aliased_local::<T>)
                        .pipe(Some)
                        .tap(|_| *head = None);
                }
                None
            }
        }
    }
}
impl<T> ExactSizeIterator for Domain<'_, T>
where
    T: BitStore,
{
    #[inline]
    fn len(&self) -> usize {
        match self {
            Self::Enclave { .. } => 1,
            Self::Region { head, body, tail } => {
                head.is_some() as usize + body.len() + tail.is_some() as usize
            }
        }
    }
}
impl<T> core::iter::FusedIterator for Domain<'_, T>
where
    T: BitStore,
{}
impl<T> Copy for Domain<'_, T>
where
    T: BitStore,
{}
macro_rules! fmt {
    ($($f:ty => $fwd:ident),+ $(,)?) => {
        $(impl < T > $f for Domain <'_, T > where T : BitStore { #[inline] fn fmt(& self,
        fmt : & mut Formatter) -> fmt::Result { fmt.debug_list().entries(self.into_iter()
        .map(FmtForward::$fwd)).finish() } })+
    };
}
fmt!(
    Binary => fmt_binary, LowerHex => fmt_lower_hex, Octal => fmt_octal, UpperHex =>
    fmt_upper_hex,
);
#[cfg(test)]
mod tests {
    use crate::prelude::*;
    #[test]
    fn domain_iter() {
        let data = [1u32, 2, 3];
        let bits = &data.view_bits::<Local>()[4..92];
        for (iter, elem) in bits.domain().rev().zip([3, 2, 1].iter().copied()) {
            assert_eq!(iter, elem);
        }
    }
}
#[cfg(test)]
mod tests_rug_623 {
    use super::*;
    use crate::domain::Domain;
    #[test]
    fn test_empty() {
        let _rug_st_tests_rug_623_rrrruuuugggg_test_empty = 0;
        let empty_domain: Domain<'static, u8> = Domain::<'static, u8>::empty();
        let _rug_ed_tests_rug_623_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_632 {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_rug_632_rrrruuuugggg_test_empty = 0;
        let empty_domain: DomainMut<'_, u8> = DomainMut::<'_, u8>::empty();
        let _rug_ed_tests_rug_632_rrrruuuugggg_test_empty = 0;
    }
}
