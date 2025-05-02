use core::mem;

/// A wrapper for `&[u8]` that provides convenient string oriented trait impls.
///
/// If you need ownership or a growable byte string buffer, then use
/// [`BString`](struct.BString.html).
///
/// Using a `&BStr` is just like using a `&[u8]`, since `BStr`
/// implements `Deref` to `[u8]`. So all methods available on `[u8]`
/// are also available on `BStr`.
///
/// # Representation
///
/// A `&BStr` has the same representation as a `&str`. That is, a `&BStr` is
/// a fat pointer which consists of a pointer to some bytes and a length.
///
/// # Trait implementations
///
/// The `BStr` type has a number of trait implementations, and in particular,
/// defines equality and ordinal comparisons between `&BStr`, `&str` and
/// `&[u8]` for convenience.
///
/// The `Debug` implementation for `BStr` shows its bytes as a normal string.
/// For invalid UTF-8, hex escape sequences are used.
///
/// The `Display` implementation behaves as if `BStr` were first lossily
/// converted to a `str`. Invalid UTF-8 bytes are substituted with the Unicode
/// replacement codepoint, which looks like this: �.
#[derive(Hash)]
#[repr(transparent)]
pub struct BStr {
    pub(crate) bytes: [u8],
}

impl BStr {
    #[inline]
    pub(crate) fn new<B: ?Sized + AsRef<[u8]>>(bytes: &B) -> &BStr {
        BStr::from_bytes(bytes.as_ref())
    }

    #[inline]
    pub(crate) fn new_mut<B: ?Sized + AsMut<[u8]>>(
        bytes: &mut B,
    ) -> &mut BStr {
        BStr::from_bytes_mut(bytes.as_mut())
    }

    #[inline]
    pub(crate) fn from_bytes(slice: &[u8]) -> &BStr {
        unsafe { mem::transmute(slice) }
    }

    #[inline]
    pub(crate) fn from_bytes_mut(slice: &mut [u8]) -> &mut BStr {
        unsafe { mem::transmute(slice) }
    }

    #[inline]
    #[cfg(feature = "std")]
    pub(crate) fn from_boxed_bytes(slice: Box<[u8]>) -> Box<BStr> {
        unsafe { Box::from_raw(Box::into_raw(slice) as _) }
    }

    #[inline]
    #[cfg(feature = "std")]
    pub(crate) fn into_boxed_bytes(slice: Box<BStr>) -> Box<[u8]> {
        unsafe { Box::from_raw(Box::into_raw(slice) as _) }
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
         
#[cfg(test)]
mod tests_rug_374 {
    use super::*;
    use crate::BStr;
    
    #[test]
    fn test_rug() {
        let bytes: &[u8] = b"hello";
        let p0 = BStr::new(bytes);

        crate::bstr::BStr::new(p0);
    }
}
#[cfg(test)]
mod tests_rug_375 {
    use super::*;
    use crate::BStr;

    #[test]
    fn test_rug() {
        let mut bytes: &mut Vec<u8> = &mut vec![104, 101, 108, 108, 111];
                
        <BStr>::new_mut(bytes);
    }
}#[cfg(test)]
mod tests_rug_376 {
    use super::*;
    use crate::BStr;
    
    #[test]
    fn test_rug() {
        let mut p0: &[u8] = b"hello";

        let _ = BStr::from_bytes(p0);
    }
}#[cfg(test)]
mod tests_rug_377 {
    use super::*;
    use crate::BStr;

    #[test]
    fn test_from_bytes_mut() {
        let mut p0 = &mut [b'a', b'b', b'c'];

        let result = BStr::from_bytes_mut(p0);
        
        // Add assertions here based on the expected behavior of the function
    }
}#[cfg(test)]
mod tests_rug_378 {
    use super::*;
    use std::boxed::Box;

    #[test]
    fn test_rug() {
        let p0: Box<[u8]> = Box::new([97, 98, 99]);

        crate::bstr::BStr::from_boxed_bytes(p0);
    }
}
#[cfg(test)]
mod tests_rug_380 {
    use super::*;
    use crate::BStr;
    
    #[test]
    fn test_rug() {
        let bytes: &[u8] = b"hello";
        let p0 = BStr::new(bytes);

        p0.as_bytes();
    }
}
