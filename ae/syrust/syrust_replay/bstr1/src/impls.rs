macro_rules! impl_partial_eq {
    ($lhs:ty, $rhs:ty) => {
        impl <'a, 'b > PartialEq <$rhs > for $lhs { #[inline] fn eq(& self, other :
        &$rhs) -> bool { let other : & [u8] = other.as_ref(); PartialEq::eq(self
        .as_bytes(), other) } } impl <'a, 'b > PartialEq <$lhs > for $rhs { #[inline] fn
        eq(& self, other : &$lhs) -> bool { let this : & [u8] = self.as_ref();
        PartialEq::eq(this, other.as_bytes()) } }
    };
}
#[cfg(feature = "std")]
macro_rules! impl_partial_eq_cow {
    ($lhs:ty, $rhs:ty) => {
        impl <'a, 'b > PartialEq <$rhs > for $lhs { #[inline] fn eq(& self, other :
        &$rhs) -> bool { let other : & [u8] = (&** other).as_ref(); PartialEq::eq(self
        .as_bytes(), other) } } impl <'a, 'b > PartialEq <$lhs > for $rhs { #[inline] fn
        eq(& self, other : &$lhs) -> bool { let this : & [u8] = (&** other).as_ref();
        PartialEq::eq(this, other.as_bytes()) } }
    };
}
macro_rules! impl_partial_ord {
    ($lhs:ty, $rhs:ty) => {
        impl <'a, 'b > PartialOrd <$rhs > for $lhs { #[inline] fn partial_cmp(& self,
        other : &$rhs) -> Option < Ordering > { let other : & [u8] = other.as_ref();
        PartialOrd::partial_cmp(self.as_bytes(), other) } } impl <'a, 'b > PartialOrd
        <$lhs > for $rhs { #[inline] fn partial_cmp(& self, other : &$lhs) -> Option <
        Ordering > { let this : & [u8] = self.as_ref(); PartialOrd::partial_cmp(this,
        other.as_bytes()) } }
    };
}
#[cfg(feature = "std")]
mod bstring {
    use std::borrow::{Borrow, Cow, ToOwned};
    use std::cmp::Ordering;
    use std::fmt;
    use std::iter::FromIterator;
    use std::ops;
    use bstr::BStr;
    use bstring::BString;
    use ext_vec::ByteVec;
    impl fmt::Display for BString {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self.as_bstr(), f)
        }
    }
    impl fmt::Debug for BString {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Debug::fmt(self.as_bstr(), f)
        }
    }
    impl ops::Deref for BString {
        type Target = Vec<u8>;
        #[inline]
        fn deref(&self) -> &Vec<u8> {
            &self.bytes
        }
    }
    impl ops::DerefMut for BString {
        #[inline]
        fn deref_mut(&mut self) -> &mut Vec<u8> {
            &mut self.bytes
        }
    }
    impl AsRef<[u8]> for BString {
        #[inline]
        fn as_ref(&self) -> &[u8] {
            &self.bytes
        }
    }
    impl AsRef<BStr> for BString {
        #[inline]
        fn as_ref(&self) -> &BStr {
            self.as_bstr()
        }
    }
    impl AsMut<[u8]> for BString {
        #[inline]
        fn as_mut(&mut self) -> &mut [u8] {
            &mut self.bytes
        }
    }
    impl AsMut<BStr> for BString {
        #[inline]
        fn as_mut(&mut self) -> &mut BStr {
            self.as_mut_bstr()
        }
    }
    impl Borrow<BStr> for BString {
        #[inline]
        fn borrow(&self) -> &BStr {
            self.as_bstr()
        }
    }
    impl ToOwned for BStr {
        type Owned = BString;
        #[inline]
        fn to_owned(&self) -> BString {
            BString::from(self)
        }
    }
    impl Default for BString {
        fn default() -> BString {
            BString::from(vec![])
        }
    }
    impl<'a> From<&'a [u8]> for BString {
        #[inline]
        fn from(s: &'a [u8]) -> BString {
            BString::from(s.to_vec())
        }
    }
    impl From<Vec<u8>> for BString {
        #[inline]
        fn from(s: Vec<u8>) -> BString {
            BString { bytes: s }
        }
    }
    impl From<BString> for Vec<u8> {
        #[inline]
        fn from(s: BString) -> Vec<u8> {
            s.bytes
        }
    }
    impl<'a> From<&'a str> for BString {
        #[inline]
        fn from(s: &'a str) -> BString {
            BString::from(s.as_bytes().to_vec())
        }
    }
    impl From<String> for BString {
        #[inline]
        fn from(s: String) -> BString {
            BString::from(s.into_bytes())
        }
    }
    impl<'a> From<&'a BStr> for BString {
        #[inline]
        fn from(s: &'a BStr) -> BString {
            BString::from(s.bytes.to_vec())
        }
    }
    impl<'a> From<BString> for Cow<'a, BStr> {
        #[inline]
        fn from(s: BString) -> Cow<'a, BStr> {
            Cow::Owned(s)
        }
    }
    impl FromIterator<char> for BString {
        #[inline]
        fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> BString {
            BString::from(iter.into_iter().collect::<String>())
        }
    }
    impl FromIterator<u8> for BString {
        #[inline]
        fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> BString {
            BString::from(iter.into_iter().collect::<Vec<u8>>())
        }
    }
    impl<'a> FromIterator<&'a str> for BString {
        #[inline]
        fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> BString {
            let mut buf = vec![];
            for b in iter {
                buf.push_str(b);
            }
            BString::from(buf)
        }
    }
    impl<'a> FromIterator<&'a [u8]> for BString {
        #[inline]
        fn from_iter<T: IntoIterator<Item = &'a [u8]>>(iter: T) -> BString {
            let mut buf = vec![];
            for b in iter {
                buf.push_str(b);
            }
            BString::from(buf)
        }
    }
    impl<'a> FromIterator<&'a BStr> for BString {
        #[inline]
        fn from_iter<T: IntoIterator<Item = &'a BStr>>(iter: T) -> BString {
            let mut buf = vec![];
            for b in iter {
                buf.push_str(b);
            }
            BString::from(buf)
        }
    }
    impl FromIterator<BString> for BString {
        #[inline]
        fn from_iter<T: IntoIterator<Item = BString>>(iter: T) -> BString {
            let mut buf = vec![];
            for b in iter {
                buf.push_str(b);
            }
            BString::from(buf)
        }
    }
    impl Eq for BString {}
    impl PartialEq for BString {
        #[inline]
        fn eq(&self, other: &BString) -> bool {
            &self[..] == &other[..]
        }
    }
    impl_partial_eq!(BString, Vec < u8 >);
    impl_partial_eq!(BString, [u8]);
    impl_partial_eq!(BString, &'a[u8]);
    impl_partial_eq!(BString, String);
    impl_partial_eq!(BString, str);
    impl_partial_eq!(BString, &'a str);
    impl_partial_eq!(BString, BStr);
    impl_partial_eq!(BString, &'a BStr);
    impl PartialOrd for BString {
        #[inline]
        fn partial_cmp(&self, other: &BString) -> Option<Ordering> {
            PartialOrd::partial_cmp(&self.bytes, &other.bytes)
        }
    }
    impl Ord for BString {
        #[inline]
        fn cmp(&self, other: &BString) -> Ordering {
            self.partial_cmp(other).unwrap()
        }
    }
    impl_partial_ord!(BString, Vec < u8 >);
    impl_partial_ord!(BString, [u8]);
    impl_partial_ord!(BString, &'a[u8]);
    impl_partial_ord!(BString, String);
    impl_partial_ord!(BString, str);
    impl_partial_ord!(BString, &'a str);
    impl_partial_ord!(BString, BStr);
    impl_partial_ord!(BString, &'a BStr);
}
mod bstr {
    #[cfg(feature = "std")]
    use std::borrow::Cow;
    use core::cmp::Ordering;
    use core::fmt;
    use core::ops;
    use bstr::BStr;
    use ext_slice::ByteSlice;
    impl fmt::Display for BStr {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            /// Write the given bstr (lossily) to the given formatter.
            fn write_bstr(
                f: &mut fmt::Formatter,
                bstr: &BStr,
            ) -> Result<(), fmt::Error> {
                for chunk in bstr.utf8_chunks() {
                    f.write_str(chunk.valid())?;
                    if !chunk.invalid().is_empty() {
                        f.write_str("\u{FFFD}")?;
                    }
                }
                Ok(())
            }
            /// Write 'num' fill characters to the given formatter.
            fn write_pads(f: &mut fmt::Formatter, num: usize) -> fmt::Result {
                let fill = f.fill();
                for _ in 0..num {
                    f.write_fmt(format_args!("{}", fill))?;
                }
                Ok(())
            }
            if let Some(align) = f.align() {
                let width = f.width().unwrap_or(0);
                let nchars = self.chars().count();
                let remaining_pads = width.saturating_sub(nchars);
                match align {
                    fmt::Alignment::Left => {
                        write_bstr(f, self)?;
                        write_pads(f, remaining_pads)?;
                    }
                    fmt::Alignment::Right => {
                        write_pads(f, remaining_pads)?;
                        write_bstr(f, self)?;
                    }
                    fmt::Alignment::Center => {
                        let half = remaining_pads / 2;
                        let second_half = if remaining_pads % 2 == 0 {
                            half
                        } else {
                            half + 1
                        };
                        write_pads(f, half)?;
                        write_bstr(f, self)?;
                        write_pads(f, second_half)?;
                    }
                }
                Ok(())
            } else {
                write_bstr(f, self)?;
                Ok(())
            }
        }
    }
    impl fmt::Debug for BStr {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "\"")?;
            for (s, e, ch) in self.char_indices() {
                match ch {
                    '\0' => write!(f, "\\0")?,
                    '\u{FFFD}' => {
                        let bytes = self[s..e].as_bytes();
                        if bytes == b"\xEF\xBF\xBD" {
                            write!(f, "{}", ch.escape_debug())?;
                        } else {
                            for &b in self[s..e].as_bytes() {
                                write!(f, r"\x{:02X}", b)?;
                            }
                        }
                    }
                    '\x01'..='\x08' | '\x0b' | '\x0c' | '\x0e'..='\x19' | '\x7f' => {
                        write!(f, "\\x{:02x}", ch as u32)?;
                    }
                    '\n' | '\r' | '\t' | _ => {
                        write!(f, "{}", ch.escape_debug())?;
                    }
                }
            }
            write!(f, "\"")?;
            Ok(())
        }
    }
    impl ops::Deref for BStr {
        type Target = [u8];
        #[inline]
        fn deref(&self) -> &[u8] {
            &self.bytes
        }
    }
    impl ops::DerefMut for BStr {
        #[inline]
        fn deref_mut(&mut self) -> &mut [u8] {
            &mut self.bytes
        }
    }
    impl ops::Index<usize> for BStr {
        type Output = u8;
        #[inline]
        fn index(&self, idx: usize) -> &u8 {
            &self.as_bytes()[idx]
        }
    }
    impl ops::Index<ops::RangeFull> for BStr {
        type Output = BStr;
        #[inline]
        fn index(&self, _: ops::RangeFull) -> &BStr {
            self
        }
    }
    impl ops::Index<ops::Range<usize>> for BStr {
        type Output = BStr;
        #[inline]
        fn index(&self, r: ops::Range<usize>) -> &BStr {
            BStr::new(&self.as_bytes()[r.start..r.end])
        }
    }
    impl ops::Index<ops::RangeInclusive<usize>> for BStr {
        type Output = BStr;
        #[inline]
        fn index(&self, r: ops::RangeInclusive<usize>) -> &BStr {
            BStr::new(&self.as_bytes()[*r.start()..=*r.end()])
        }
    }
    impl ops::Index<ops::RangeFrom<usize>> for BStr {
        type Output = BStr;
        #[inline]
        fn index(&self, r: ops::RangeFrom<usize>) -> &BStr {
            BStr::new(&self.as_bytes()[r.start..])
        }
    }
    impl ops::Index<ops::RangeTo<usize>> for BStr {
        type Output = BStr;
        #[inline]
        fn index(&self, r: ops::RangeTo<usize>) -> &BStr {
            BStr::new(&self.as_bytes()[..r.end])
        }
    }
    impl ops::Index<ops::RangeToInclusive<usize>> for BStr {
        type Output = BStr;
        #[inline]
        fn index(&self, r: ops::RangeToInclusive<usize>) -> &BStr {
            BStr::new(&self.as_bytes()[..=r.end])
        }
    }
    impl ops::IndexMut<usize> for BStr {
        #[inline]
        fn index_mut(&mut self, idx: usize) -> &mut u8 {
            &mut self.bytes[idx]
        }
    }
    impl ops::IndexMut<ops::RangeFull> for BStr {
        #[inline]
        fn index_mut(&mut self, _: ops::RangeFull) -> &mut BStr {
            self
        }
    }
    impl ops::IndexMut<ops::Range<usize>> for BStr {
        #[inline]
        fn index_mut(&mut self, r: ops::Range<usize>) -> &mut BStr {
            BStr::from_bytes_mut(&mut self.bytes[r.start..r.end])
        }
    }
    impl ops::IndexMut<ops::RangeInclusive<usize>> for BStr {
        #[inline]
        fn index_mut(&mut self, r: ops::RangeInclusive<usize>) -> &mut BStr {
            BStr::from_bytes_mut(&mut self.bytes[*r.start()..=*r.end()])
        }
    }
    impl ops::IndexMut<ops::RangeFrom<usize>> for BStr {
        #[inline]
        fn index_mut(&mut self, r: ops::RangeFrom<usize>) -> &mut BStr {
            BStr::from_bytes_mut(&mut self.bytes[r.start..])
        }
    }
    impl ops::IndexMut<ops::RangeTo<usize>> for BStr {
        #[inline]
        fn index_mut(&mut self, r: ops::RangeTo<usize>) -> &mut BStr {
            BStr::from_bytes_mut(&mut self.bytes[..r.end])
        }
    }
    impl ops::IndexMut<ops::RangeToInclusive<usize>> for BStr {
        #[inline]
        fn index_mut(&mut self, r: ops::RangeToInclusive<usize>) -> &mut BStr {
            BStr::from_bytes_mut(&mut self.bytes[..=r.end])
        }
    }
    impl AsRef<[u8]> for BStr {
        #[inline]
        fn as_ref(&self) -> &[u8] {
            self.as_bytes()
        }
    }
    impl AsRef<BStr> for [u8] {
        #[inline]
        fn as_ref(&self) -> &BStr {
            BStr::new(self)
        }
    }
    impl AsRef<BStr> for str {
        #[inline]
        fn as_ref(&self) -> &BStr {
            BStr::new(self)
        }
    }
    impl AsMut<[u8]> for BStr {
        #[inline]
        fn as_mut(&mut self) -> &mut [u8] {
            &mut self.bytes
        }
    }
    impl AsMut<BStr> for [u8] {
        #[inline]
        fn as_mut(&mut self) -> &mut BStr {
            BStr::new_mut(self)
        }
    }
    impl<'a> Default for &'a BStr {
        fn default() -> &'a BStr {
            BStr::from_bytes(b"")
        }
    }
    impl<'a> Default for &'a mut BStr {
        fn default() -> &'a mut BStr {
            BStr::from_bytes_mut(&mut [])
        }
    }
    impl<'a> From<&'a [u8]> for &'a BStr {
        #[inline]
        fn from(s: &'a [u8]) -> &'a BStr {
            BStr::from_bytes(s)
        }
    }
    impl<'a> From<&'a str> for &'a BStr {
        #[inline]
        fn from(s: &'a str) -> &'a BStr {
            BStr::from_bytes(s.as_bytes())
        }
    }
    #[cfg(feature = "std")]
    impl<'a> From<&'a BStr> for Cow<'a, BStr> {
        #[inline]
        fn from(s: &'a BStr) -> Cow<'a, BStr> {
            Cow::Borrowed(s)
        }
    }
    #[cfg(feature = "std")]
    impl From<Box<[u8]>> for Box<BStr> {
        #[inline]
        fn from(s: Box<[u8]>) -> Box<BStr> {
            BStr::from_boxed_bytes(s)
        }
    }
    #[cfg(feature = "std")]
    impl From<Box<BStr>> for Box<[u8]> {
        #[inline]
        fn from(s: Box<BStr>) -> Box<[u8]> {
            BStr::into_boxed_bytes(s)
        }
    }
    impl Eq for BStr {}
    impl PartialEq<BStr> for BStr {
        #[inline]
        fn eq(&self, other: &BStr) -> bool {
            self.as_bytes() == other.as_bytes()
        }
    }
    impl_partial_eq!(BStr, [u8]);
    impl_partial_eq!(BStr, &'a[u8]);
    impl_partial_eq!(BStr, str);
    impl_partial_eq!(BStr, &'a str);
    #[cfg(feature = "std")]
    impl_partial_eq!(BStr, Vec < u8 >);
    #[cfg(feature = "std")]
    impl_partial_eq!(&'a BStr, Vec < u8 >);
    #[cfg(feature = "std")]
    impl_partial_eq!(BStr, String);
    #[cfg(feature = "std")]
    impl_partial_eq!(&'a BStr, String);
    #[cfg(feature = "std")]
    impl_partial_eq_cow!(&'a BStr, Cow <'a, BStr >);
    #[cfg(feature = "std")]
    impl_partial_eq_cow!(&'a BStr, Cow <'a, str >);
    #[cfg(feature = "std")]
    impl_partial_eq_cow!(&'a BStr, Cow <'a, [u8] >);
    impl PartialOrd for BStr {
        #[inline]
        fn partial_cmp(&self, other: &BStr) -> Option<Ordering> {
            PartialOrd::partial_cmp(self.as_bytes(), other.as_bytes())
        }
    }
    impl Ord for BStr {
        #[inline]
        fn cmp(&self, other: &BStr) -> Ordering {
            self.partial_cmp(other).unwrap()
        }
    }
    impl_partial_ord!(BStr, [u8]);
    impl_partial_ord!(BStr, &'a[u8]);
    impl_partial_ord!(BStr, str);
    impl_partial_ord!(BStr, &'a str);
    #[cfg(feature = "std")]
    impl_partial_ord!(BStr, Vec < u8 >);
    #[cfg(feature = "std")]
    impl_partial_ord!(&'a BStr, Vec < u8 >);
    #[cfg(feature = "std")]
    impl_partial_ord!(BStr, String);
    #[cfg(feature = "std")]
    impl_partial_ord!(&'a BStr, String);
}
#[cfg(feature = "serde1-nostd")]
mod bstr_serde {
    use core::fmt;
    use serde::{
        de::Error, de::Visitor, Deserialize, Deserializer, Serialize, Serializer,
    };
    use bstr::BStr;
    impl Serialize for BStr {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_bytes(self.as_bytes())
        }
    }
    impl<'a, 'de: 'a> Deserialize<'de> for &'a BStr {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<&'a BStr, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct BStrVisitor;
            impl<'de> Visitor<'de> for BStrVisitor {
                type Value = &'de BStr;
                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("a borrowed byte string")
                }
                #[inline]
                fn visit_borrowed_bytes<E: Error>(
                    self,
                    value: &'de [u8],
                ) -> Result<&'de BStr, E> {
                    Ok(BStr::new(value))
                }
                #[inline]
                fn visit_borrowed_str<E: Error>(
                    self,
                    value: &'de str,
                ) -> Result<&'de BStr, E> {
                    Ok(BStr::new(value))
                }
            }
            deserializer.deserialize_bytes(BStrVisitor)
        }
    }
}
#[cfg(feature = "serde1")]
mod bstring_serde {
    use std::cmp;
    use std::fmt;
    use serde::{
        de::Error, de::SeqAccess, de::Visitor, Deserialize, Deserializer, Serialize,
        Serializer,
    };
    use bstring::BString;
    impl Serialize for BString {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_bytes(self.as_bytes())
        }
    }
    impl<'de> Deserialize<'de> for BString {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<BString, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct BStringVisitor;
            impl<'de> Visitor<'de> for BStringVisitor {
                type Value = BString;
                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("a byte string")
                }
                #[inline]
                fn visit_seq<V: SeqAccess<'de>>(
                    self,
                    mut visitor: V,
                ) -> Result<BString, V::Error> {
                    let len = cmp::min(visitor.size_hint().unwrap_or(0), 256);
                    let mut bytes = Vec::with_capacity(len);
                    while let Some(v) = visitor.next_element()? {
                        bytes.push(v);
                    }
                    Ok(BString::from(bytes))
                }
                #[inline]
                fn visit_bytes<E: Error>(self, value: &[u8]) -> Result<BString, E> {
                    Ok(BString::from(value))
                }
                #[inline]
                fn visit_byte_buf<E: Error>(self, value: Vec<u8>) -> Result<BString, E> {
                    Ok(BString::from(value))
                }
                #[inline]
                fn visit_str<E: Error>(self, value: &str) -> Result<BString, E> {
                    Ok(BString::from(value))
                }
                #[inline]
                fn visit_string<E: Error>(self, value: String) -> Result<BString, E> {
                    Ok(BString::from(value))
                }
            }
            deserializer.deserialize_byte_buf(BStringVisitor)
        }
    }
}
#[cfg(test)]
mod display {
    use crate::ByteSlice;
    use bstring::BString;
    #[test]
    fn clean() {
        assert_eq!(& format!("{}", & b"abc".as_bstr()), "abc");
        assert_eq!(& format!("{}", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��");
    }
    #[test]
    fn width_bigger_than_bstr() {
        assert_eq!(& format!("{:<7}!", & b"abc".as_bstr()), "abc    !");
        assert_eq!(& format!("{:>7}!", & b"abc".as_bstr()), "    abc!");
        assert_eq!(& format!("{:^7}!", & b"abc".as_bstr()), "  abc  !");
        assert_eq!(& format!("{:^6}!", & b"abc".as_bstr()), " abc  !");
        assert_eq!(& format!("{:-<7}!", & b"abc".as_bstr()), "abc----!");
        assert_eq!(& format!("{:->7}!", & b"abc".as_bstr()), "----abc!");
        assert_eq!(& format!("{:-^7}!", & b"abc".as_bstr()), "--abc--!");
        assert_eq!(& format!("{:-^6}!", & b"abc".as_bstr()), "-abc--!");
        assert_eq!(
            & format!("{:<7}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��   !"
        );
        assert_eq!(
            & format!("{:>7}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "   �(��!"
        );
        assert_eq!(
            & format!("{:^7}!", & b"\xf0\x28\x8c\xbc".as_bstr()), " �(��  !"
        );
        assert_eq!(
            & format!("{:^6}!", & b"\xf0\x28\x8c\xbc".as_bstr()), " �(�� !"
        );
        assert_eq!(
            & format!("{:-<7}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��---!"
        );
        assert_eq!(
            & format!("{:->7}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "---�(��!"
        );
        assert_eq!(
            & format!("{:-^7}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "-�(��--!"
        );
        assert_eq!(
            & format!("{:-^6}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "-�(��-!"
        );
    }
    #[test]
    fn width_lesser_than_bstr() {
        assert_eq!(& format!("{:<2}!", & b"abc".as_bstr()), "abc!");
        assert_eq!(& format!("{:>2}!", & b"abc".as_bstr()), "abc!");
        assert_eq!(& format!("{:^2}!", & b"abc".as_bstr()), "abc!");
        assert_eq!(& format!("{:-<2}!", & b"abc".as_bstr()), "abc!");
        assert_eq!(& format!("{:->2}!", & b"abc".as_bstr()), "abc!");
        assert_eq!(& format!("{:-^2}!", & b"abc".as_bstr()), "abc!");
        assert_eq!(& format!("{:<3}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��!");
        assert_eq!(& format!("{:>3}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��!");
        assert_eq!(& format!("{:^3}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��!");
        assert_eq!(& format!("{:^2}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��!");
        assert_eq!(& format!("{:-<3}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��!");
        assert_eq!(& format!("{:->3}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��!");
        assert_eq!(& format!("{:-^3}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��!");
        assert_eq!(& format!("{:-^2}!", & b"\xf0\x28\x8c\xbc".as_bstr()), "�(��!");
    }
    quickcheck! {
        fn total_length(bstr : BString) -> bool { let size = bstr.chars().count();
        format!("{:<1$}", bstr.as_bstr(), size) .chars().count() >= size }
    }
}
#[cfg(test)]
mod bstring_arbitrary {
    use bstring::BString;
    use quickcheck::{Arbitrary, Gen};
    impl Arbitrary for BString {
        fn arbitrary<G: Gen>(g: &mut G) -> BString {
            BString::from(Vec::<u8>::arbitrary(g))
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = BString>> {
            Box::new(self.bytes.shrink().map(BString::from))
        }
    }
}
#[test]
fn test_debug() {
    use crate::{ByteSlice, B};
    assert_eq!(
        r#""\0\0\0 ftypisom\0\0\x02\0isomiso2avc1mp""#, format!("{:?}",
        b"\0\0\0 ftypisom\0\0\x02\0isomiso2avc1mp".as_bstr()),
    );
    assert_eq!(
        b"\"\\xFF\xEF\xBF\xBD\\xFF\"".as_bstr(), B(& format!("{:?}",
        b"\xFF\xEF\xBF\xBD\xFF".as_bstr())).as_bstr(),
    );
}
#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::lazy_static::__Deref;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_176_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        <BString as __Deref>::deref(&p0);
        let _rug_ed_tests_rug_176_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_177 {
    use super::*;
    use crate::std::ops::DerefMut;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_177_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        <BString as DerefMut>::deref_mut(&mut p0);
        let _rug_ed_tests_rug_177_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::std::convert::AsRef;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_178_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        <BString as AsRef<[u8]>>::as_ref(&p0);
        let _rug_ed_tests_rug_178_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use crate::std::convert::AsRef;
    use crate::{BString, BStr};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_179_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        <BString as AsRef<BStr>>::as_ref(&p0);
        let _rug_ed_tests_rug_179_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use crate::BString;
    use crate::std::convert::AsMut;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_180_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        <BString as AsMut<[u8]>>::as_mut(&mut p0);
        let _rug_ed_tests_rug_180_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use crate::{BStr, BString};
    use crate::std::borrow::ToOwned;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_183_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        <BStr as ToOwned>::to_owned(&p0);
        let _rug_ed_tests_rug_183_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::bstring::BString;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_185_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = b"hello";
        let p0: &[u8] = rug_fuzz_0;
        <BString>::from(p0);
        let _rug_ed_tests_rug_185_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use crate::std::convert::From;
    use crate::bstring::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v13 = vec![rug_fuzz_0, 3, 5, 7, 9];
        let p0: Vec<u8> = v13;
        let _result = <BString>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::std::convert::From;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_187_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        <std::vec::Vec<u8>>::from(p0);
        let _rug_ed_tests_rug_187_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::bstring::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &str = rug_fuzz_0;
        <BString>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::bstring::BString;
    use std::convert::From;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: String = rug_fuzz_0.to_string();
        <BString>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::std::borrow::Cow;
    use crate::{BStr, BString};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_191_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        <Cow<'_, BStr>>::from(p0);
        let _rug_ed_tests_rug_191_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::std::iter::FromIterator;
    use bstring::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(char, char, char) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v47: Vec<char> = Vec::new();
        v47.push(rug_fuzz_0);
        v47.push(rug_fuzz_1);
        v47.push(rug_fuzz_2);
        let result = <BString>::from_iter(v47);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::bstring::BString;
    use std::iter::FromIterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_193_rrrruuuugggg_test_rug = 0;
        let p0: std::sync::mpsc::Receiver<u8> = unimplemented!();
        <BString>::from_iter(p0);
        let _rug_ed_tests_rug_193_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::std::iter::FromIterator;
    use bstring::BString;
    use std::collections::VecDeque;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v48: VecDeque<&str> = VecDeque::new();
        v48.push_back(rug_fuzz_0);
        v48.push_back(rug_fuzz_1);
        <BString>::from_iter(v48);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::bstring::BString;
    use crate::std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_198_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        let mut p1 = BString::default();
        p0.eq(&p1);
        let _rug_ed_tests_rug_198_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = BString::default();
        let mut p1 = vec![rug_fuzz_0, 3, 5, 7, 9];
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_200 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::std::vec::Vec;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v13 = vec![rug_fuzz_0, 3, 5, 7, 9];
        let mut v46 = BString::default();
        v13.eq(&v46);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_202_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example_input";
        let mut p0: &[u8] = rug_fuzz_0;
        let mut p1 = BString::default();
        <[u8]>::eq(p0, p1.as_bytes());
        let _rug_ed_tests_rug_202_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_203 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_203_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let mut p0 = BString::default();
        let mut p1: &[u8] = rug_fuzz_0;
        p0.eq(&p1);
        let _rug_ed_tests_rug_203_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let mut p0: &[u8] = rug_fuzz_0;
        let mut p1 = BString::from(rug_fuzz_1);
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_205 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v46 = BString::default();
        let mut v47 = String::from(rug_fuzz_0);
        v46.eq(&v47);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_206 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use std::string::String;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: String = String::from(rug_fuzz_0);
        let mut p1: BString = BString::default();
        <std::string::String>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v46 = BString::default();
        let mut other = rug_fuzz_0;
        v46.eq(&other);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_208 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &str = rug_fuzz_0;
        let mut p1 = BString::default();
        <str>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_209 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = BString::default();
        let p1: &str = rug_fuzz_0;
        p0.eq(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_210 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = rug_fuzz_0;
        let mut p1 = BString::from(rug_fuzz_1);
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_211 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BString;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_211_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let mut p0 = BString::default();
        let bytes: &[u8] = rug_fuzz_0;
        let p1 = BStr::new(bytes);
        p0.eq(&p1);
        let _rug_ed_tests_rug_211_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_212 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::{BStr, BString};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_212_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let mut p1 = BString::default();
        p0.eq(&p1);
        let _rug_ed_tests_rug_212_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_214 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::{BStr, BString};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_214_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let mut v46 = BString::default();
        let p1 = &v46;
        p0.eq(p1);
        let _rug_ed_tests_rug_214_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_215 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_215_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        let mut p1 = BString::default();
        <BString as PartialOrd>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_215_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_216 {
    use super::*;
    use crate::std::cmp::Ord;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_216_rrrruuuugggg_test_rug = 0;
        let mut p0 = BString::default();
        let mut p1 = BString::default();
        <BString as Ord>::cmp(&p0, &p1);
        let _rug_ed_tests_rug_216_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_217 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = BString::default();
        let mut p1 = vec![rug_fuzz_0, 3, 5, 7, 9];
        <BString as PartialOrd<Vec<u8>>>::partial_cmp(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v13 = vec![rug_fuzz_0, 3, 5, 7, 9];
        let mut v46 = BString::default();
        <std::vec::Vec<u8>>::partial_cmp(&v13, &v46);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::BString;
    use std::cmp::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_219_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example";
        let mut p0 = BString::default();
        let p1: &[u8] = rug_fuzz_0;
        p0.partial_cmp(p1);
        let _rug_ed_tests_rug_219_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_220 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_220_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example data";
        let mut p0: &[u8] = rug_fuzz_0;
        let mut p1: BString = BString::default();
        <[u8]>::partial_cmp(p0, p1.as_bytes());
        let _rug_ed_tests_rug_220_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_221 {
    use super::*;
    use crate::std::cmp::{Ordering, PartialOrd};
    use crate::bstring::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_221_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"test_data";
        let mut p0 = BString::default();
        let sample_data: &[u8] = rug_fuzz_0;
        let p1: &[u8] = sample_data;
        <BString as PartialOrd<&[u8]>>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_221_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_222 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_222_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"test_data";
        let rug_fuzz_1 = "test_data";
        let data1: &'static [u8] = rug_fuzz_0;
        let data2 = BString::from(rug_fuzz_1);
        let mut p0: &[_] = data1;
        let mut p1: &BString = &data2;
        p0.partial_cmp(p1);
        let _rug_ed_tests_rug_222_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_223 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = BString::default();
        let mut p1: &std::string::String = &rug_fuzz_0.to_string();
        p0.partial_cmp(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_224 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: std::string::String = rug_fuzz_0.to_string();
        let mut p1 = BString::default();
        <std::string::String>::partial_cmp(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_225 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = BString::default();
        let p1: &str = rug_fuzz_0;
        p0.partial_cmp(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_226 {
    use super::*;
    use crate::std::cmp::{Ordering, PartialOrd};
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &str = rug_fuzz_0;
        let mut p1 = BString::default();
        p0.partial_cmp(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_227 {
    use super::*;
    use crate::std::cmp::{Ordering, PartialOrd};
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = BString::default();
        let mut p1: &str = rug_fuzz_0;
        p0.partial_cmp(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_228 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BString;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: &str = rug_fuzz_0;
        let mut p1 = BString::default();
        p0.partial_cmp(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_229 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::{BString, BStr};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_229_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let mut p0 = BString::default();
        let bytes: &[u8] = rug_fuzz_0;
        let p1 = BStr::new(bytes);
        p0.partial_cmp(&p1);
        let _rug_ed_tests_rug_229_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_230 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    use bstring::BString;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_230_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1 = BString::default();
        p0.partial_cmp(&p1);
        let _rug_ed_tests_rug_230_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_231 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::{BString, BStr};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_231_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let mut p0 = BString::default();
        let bytes: &[u8] = rug_fuzz_0;
        let p1 = BStr::new(bytes);
        p0.partial_cmp(&p1);
        let _rug_ed_tests_rug_231_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_232 {
    use super::*;
    use crate::std::cmp::{PartialOrd, Ordering};
    use crate::{BString, BStr};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_232_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bytes0: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes0);
        let mut p1 = BString::default();
        p0.partial_cmp(&p1);
        let _rug_ed_tests_rug_232_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::lazy_static::__Deref;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_233_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        <BStr as lazy_static::__Deref>::deref(&p0);
        let _rug_ed_tests_rug_233_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        use crate::bstr;
        use crate::std::ops::Index;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = bstr::BStr::new(bytes);
        let p1: usize = rug_fuzz_1;
        <bstr::BStr>::index(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    use crate::std::ops::Index;
    use crate::BStr;
    use std::ops::RangeFull;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_236_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let bstr = BStr::new(bytes);
        let mut range: RangeFull = ..;
        <BStr>::index(&bstr, range);
        let _rug_ed_tests_rug_236_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_237 {
    use super::*;
    use crate::std::ops::Index;
    use crate::BStr;
    use std::ops::Range;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1, mut rug_fuzz_2)) = <([u8; 5], usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: Range<usize> = Range {
            start: rug_fuzz_1,
            end: rug_fuzz_2,
        };
        <BStr as Index<Range<usize>>>::index(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    use crate::std::ops::Index;
    use std::ops::RangeInclusive;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1, mut rug_fuzz_2)) = <([u8; 5], usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: RangeInclusive<usize> = rug_fuzz_1..=rug_fuzz_2;
        <BStr as Index<RangeInclusive<usize>>>::index(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_239 {
    use super::*;
    use crate::bstr::BStr;
    use crate::std::ops::Index;
    use std::ops::RangeFrom;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: RangeFrom<usize> = rug_fuzz_1..;
        <BStr>::index(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    use crate::std::ops::Index;
    use crate::BStr;
    use std::ops::RangeTo;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: RangeTo<usize> = ..rug_fuzz_1;
        <BStr>::index(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::std::ops::Index;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1, mut rug_fuzz_2)) = <([u8; 5], usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1 = rug_fuzz_1..=rug_fuzz_2;
        p0.index(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::std::convert::AsRef;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_249_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        <BStr as AsRef<[u8]>>::as_ref(&p0);
        let _rug_ed_tests_rug_249_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_250 {
    use super::*;
    use crate::bstr::BStr;
    use crate::std::convert::AsRef;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_250_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"example_data";
        let input_data: &[u8] = rug_fuzz_0;
        let p0 = input_data;
        <[u8] as AsRef<BStr>>::as_ref(&p0);
        let _rug_ed_tests_rug_250_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_251 {
    use super::*;
    use crate::std::convert::AsRef;
    use crate::{BStr, B};
    #[test]
    fn test_as_ref() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        <str as AsRef<BStr>>::as_ref(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_253 {
    use super::*;
    use crate::bstr::BStr;
    use crate::impls::bstr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4)) = <(u8, u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: [u8; 5] = [
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        p0.as_mut();
             }
}
}
}    }
}
use crate::BStr;
#[cfg(test)]
mod tests_rug_254 {
    use super::*;
    use crate::std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_254_rrrruuuugggg_test_rug = 0;
        let default_bstr: &BStr = <&'_ BStr>::default();
        let _rug_ed_tests_rug_254_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_255 {
    use super::*;
    use crate::BStr;
    use std::default::Default;
    #[test]
    fn test_default_impl() {
        let _rug_st_tests_rug_255_rrrruuuugggg_test_default_impl = 0;
        let mut data: [u8; 0] = [];
        let bstr = BStr::from_bytes_mut(&mut data);
        let result = <&mut BStr>::default();
        debug_assert_eq!(result, bstr);
        let _rug_ed_tests_rug_255_rrrruuuugggg_test_default_impl = 0;
    }
}
#[cfg(test)]
mod tests_rug_256 {
    use super::*;
    use crate::std::convert::From;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_256_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let mut p0: &[u8] = rug_fuzz_0;
        <&'static BStr>::from(p0);
        let _rug_ed_tests_rug_256_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_259 {
    use super::*;
    use crate::std::{convert::From, boxed::Box};
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Box<[u8]> = Box::new([rug_fuzz_0, rug_fuzz_1, rug_fuzz_2]);
        <Box<BStr>>::from(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_261 {
    use super::*;
    use crate::bstr::BStr;
    use crate::std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_261_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        let bytes1: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes1);
        let bytes2: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes2);
        <BStr as PartialEq>::eq(&p0, &p1);
        let _rug_ed_tests_rug_261_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_262 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_262_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        use crate::BStr;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let other_bytes: &[u8] = rug_fuzz_1;
        let p1 = other_bytes;
        p0.eq(&p1);
        let _rug_ed_tests_rug_262_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_263 {
    use super::*;
    use crate::std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_263_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"world";
        let rug_fuzz_1 = b"hello";
        use crate::BStr;
        let mut p0: &[u8] = rug_fuzz_0;
        let bytes: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes);
        <[u8]>::eq(p0, p1.as_bytes());
        let _rug_ed_tests_rug_263_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use crate::BStr;
    use crate::std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_264_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        let bytes_p0: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes_p0);
        let bytes_p1: &[u8] = rug_fuzz_1;
        let p1: &[u8] = bytes_p1;
        <BStr as PartialEq<&[u8]>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_264_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_265 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::bstr::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_265_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"world";
        let rug_fuzz_1 = b"world";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let bytes2: &[u8] = rug_fuzz_1;
        let p1: &'static [u8] = bytes2;
        p0.eq(p1);
        let _rug_ed_tests_rug_265_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_266 {
    use super::*;
    use crate::bstr::BStr;
    use crate::std::cmp::PartialEq;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: &str = rug_fuzz_1;
        <BStr as PartialEq<str>>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_267 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::bstr::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let p0_bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(p0_bytes);
        let p1_str: &str = rug_fuzz_1;
        p0.eq(p1_str);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_268 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: &str = rug_fuzz_1;
        <BStr>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_269 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::bstr::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let v8 = BStr::new(bytes);
        let this_str: &str = rug_fuzz_1;
        let p1: &BStr = &v8;
        this_str.eq(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_270 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let mut v13 = vec![rug_fuzz_1, 3, 5, 7, 9];
        let p1 = &v13;
        <BStr>::eq(&p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_271_prepare {
    #[test]
    fn sample() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v13 = vec![rug_fuzz_0, 3, 5, 7, 9];
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_271 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_271_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = b"hello";
        let mut p0 = vec![rug_fuzz_0, 3, 5, 7, 9];
        let bytes: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes);
        <std::vec::Vec<u8>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_271_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_272 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes1: &[u8] = rug_fuzz_0;
        let v8_1 = BStr::new(bytes1);
        let mut v13 = vec![rug_fuzz_1, 3, 5, 7, 9];
        let v8_2 = &v13;
        v8_1.eq(v8_2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_273_prepare {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::bstr::BStr;
    #[test]
    fn sample1() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v13 = vec![rug_fuzz_0, 3, 5, 7, 9];
             }
}
}
}    }
    #[test]
    fn sample2() {
        let _rug_st_tests_rug_273_prepare_rrrruuuugggg_sample2 = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let v8 = BStr::new(bytes);
        let _rug_ed_tests_rug_273_prepare_rrrruuuugggg_sample2 = 0;
    }
}
#[cfg(test)]
mod tests_rug_273 {
    use super::*;
    use crate::std::cmp::PartialEq;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_273_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = b"hello";
        let mut p0 = vec![rug_fuzz_0, 3, 5, 7, 9];
        let bytes: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes);
        <std::vec::Vec<u8>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_273_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_274 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        use std::cmp::PartialEq;
        let bytes: &[u8] = rug_fuzz_0;
        let v8 = BStr::new(bytes);
        let mut string_arg = String::from(rug_fuzz_1);
        <BStr>::eq(&v8, &string_arg);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_275 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::bstr::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_275_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example";
        let rug_fuzz_1 = b"hello";
        let mut p0 = String::from(rug_fuzz_0);
        let bytes: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes);
        <std::string::String>::eq(&p0, &p1);
        let _rug_ed_tests_rug_275_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_276 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::bstr::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let mut p1 = String::from(rug_fuzz_1);
        p0.eq(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_277 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = String::from(rug_fuzz_1);
        let p1 = BStr::new(bytes);
        <std::string::String>::eq(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_279 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use std::borrow::Cow;
    use crate::{B, BStr};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_279_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample_data";
        let rug_fuzz_1 = b"hello";
        let data = rug_fuzz_0;
        let v60 = Cow::Borrowed(B(data));
        let bytes: &[u8] = rug_fuzz_1;
        let v8 = BStr::new(bytes);
        v60.eq(&v8);
        let _rug_ed_tests_rug_279_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_281 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use std::borrow::Cow;
    use crate::{BStr, ByteSlice};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_281_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = b"hello";
        let mut p0: Cow<str> = Cow::Borrowed(rug_fuzz_0);
        let bytes: &[u8] = rug_fuzz_1;
        let mut p1 = BStr::new(bytes);
        p0.eq(&p1);
        let _rug_ed_tests_rug_281_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_283 {
    use super::*;
    use crate::std::cmp::PartialEq;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_283_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample data";
        let rug_fuzz_1 = b"hello";
        use crate::ByteSlice;
        use crate::BStr;
        let data: Cow<'static, [u8]> = Cow::Borrowed(rug_fuzz_0);
        let mut p0: Cow<'_, [u8]> = data;
        let bytes: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes);
        <std::borrow::Cow<'_, [u8]>>::eq(&p0, &p1);
        let _rug_ed_tests_rug_283_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_284 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_284_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        let bytes0: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes0);
        let bytes1: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes1);
        <BStr as std::cmp::PartialOrd>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_284_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_285 {
    use super::*;
    use crate::std::cmp::Ordering;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_285_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        let bytes1: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes1);
        let bytes2: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes2);
        <BStr as Ord>::cmp(&p0, &p1);
        let _rug_ed_tests_rug_285_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_286 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_286_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        let bytes1: &[u8] = rug_fuzz_0;
        let bs1 = BStr::new(bytes1);
        let bytes2: &[u8] = rug_fuzz_1;
        let bs2 = BStr::new(bytes2);
        <BStr as PartialOrd<[u8]>>::partial_cmp(&bs1, bs2.as_ref());
        let _rug_ed_tests_rug_286_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_287 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_287_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"rust is fun!";
        let rug_fuzz_1 = b"hello";
        let mut p0: &[u8] = rug_fuzz_0;
        let bytes: &[u8] = rug_fuzz_1;
        let mut p1 = BStr::new(bytes);
        <[u8]>::partial_cmp(p0, p1.as_bytes());
        let _rug_ed_tests_rug_287_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_288 {
    use super::*;
    use crate::bstr::BStr;
    use std::cmp::PartialOrd;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_rug_288_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = b"world";
        let bytes1: &[u8] = rug_fuzz_0;
        let bytes2: &[u8] = rug_fuzz_1;
        let bstr1 = BStr::new(bytes1);
        let bstr2 = BStr::new(bytes2);
        let result = <BStr as PartialOrd>::partial_cmp(&bstr1, &bstr2);
        debug_assert_eq!(result, Some(Ordering::Less));
        let _rug_ed_tests_rug_288_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_rug_290 {
    use super::*;
    use crate::bstr::BStr;
    use std::cmp::PartialOrd;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: &str = rug_fuzz_1;
        <BStr as PartialOrd<str>>::partial_cmp(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_291 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let mut bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: &str = rug_fuzz_1;
        p0.partial_cmp(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_292 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    use std::cmp::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_292_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = "world";
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: &'static str = rug_fuzz_1;
        <BStr as PartialOrd<&str>>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_292_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_294 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes_p0: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes_p0);
        let v13_p1 = vec![rug_fuzz_1, 3, 5, 7, 9];
        let p1 = &v13_p1;
        p0.partial_cmp(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_295 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use std::cmp::Ordering;
    use std::vec::Vec;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_295_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = b"hello";
        let mut p0 = vec![rug_fuzz_0, 3, 5, 7, 9];
        let bytes: &[u8] = rug_fuzz_1;
        let mut p1 = BStr::new(bytes);
        <Vec<u8>>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_295_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_296 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    use std::cmp::Ordering;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes0: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes0);
        let mut v13 = vec![rug_fuzz_1, 3, 5, 7, 9];
        let p1 = &v13;
        p0.partial_cmp(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_297_prepare {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use std::cmp::Ordering;
    use std::vec::Vec;
    use crate::BStr;
    #[test]
    fn sample_vec() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v13 = vec![rug_fuzz_0, 3, 5, 7, 9];
        let p0 = v13;
             }
}
}
}    }
    #[test]
    fn sample_bstr() {
        let _rug_st_tests_rug_297_prepare_rrrruuuugggg_sample_bstr = 0;
        let rug_fuzz_0 = b"hello";
        let bytes: &[u8] = rug_fuzz_0;
        let v8 = BStr::new(bytes);
        let p1 = &v8;
        let _rug_ed_tests_rug_297_prepare_rrrruuuugggg_sample_bstr = 0;
    }
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_rug_297_prepare_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = b"hello";
        let mut p0 = vec![rug_fuzz_0, 3, 5, 7, 9];
        let bytes: &[u8] = rug_fuzz_1;
        let v8 = BStr::new(bytes);
        let p1 = &v8;
        <Vec<u8>>::partial_cmp(&p0, p1);
        let _rug_ed_tests_rug_297_prepare_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_rug_298 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1: std::string::String = String::from(rug_fuzz_1);
        <BStr as PartialOrd<std::string::String>>::partial_cmp(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_299 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_299_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example";
        let rug_fuzz_1 = b"hello";
        let mut p0: std::string::String = rug_fuzz_0.to_string();
        let bytes: &[u8] = rug_fuzz_1;
        let mut p1 = BStr::new(bytes);
        <std::string::String>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_299_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_300 {
    use super::*;
    use crate::std::cmp::PartialOrd;
    use crate::bstr::BStr;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 5], &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let bytes: &[u8] = rug_fuzz_0;
        let p0 = BStr::new(bytes);
        let p1 = String::from(rug_fuzz_1);
        p0.partial_cmp(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_301 {
    use super::*;
    use std::cmp::PartialOrd;
    use crate::BStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_301_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "world";
        let rug_fuzz_1 = b"hello";
        let mut p0 = String::from(rug_fuzz_0);
        let bytes: &[u8] = rug_fuzz_1;
        let p1 = BStr::new(bytes);
        <std::string::String>::partial_cmp(&p0, &p1);
        let _rug_ed_tests_rug_301_rrrruuuugggg_test_rug = 0;
    }
}
