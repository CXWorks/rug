use std::fmt;
use std::io;
use std::iter::FromIterator;
use std::ops::{self, Range};
use std::result;
use std::str;
use serde::de::Deserialize;
use crate::byte_record::{ByteRecord, ByteRecordIter, Position};
use crate::deserializer::deserialize_string_record;
use crate::error::{Error, ErrorKind, FromUtf8Error, Result};
use crate::reader::Reader;
/// A single CSV record stored as valid UTF-8 bytes.
///
/// A string record permits reading or writing CSV rows that are valid UTF-8.
/// If string records are used to read CSV data that is not valid UTF-8, then
/// the CSV reader will return an invalid UTF-8 error. If you do need to read
/// possibly invalid UTF-8 data, then you should prefer using a
/// [`ByteRecord`](struct.ByteRecord.html),
/// since it makes no assumptions about UTF-8.
///
/// If you are using the Serde (de)serialization APIs, then you probably never
/// need to interact with a `ByteRecord` or a `StringRecord`. However, there
/// are some circumstances in which you might need to use a raw record type
/// while still using Serde. For example, if you need to deserialize possibly
/// invalid UTF-8 fields, then you'll need to first read your record into a
/// `ByteRecord`, and then use `ByteRecord::deserialize` to run Serde. Another
/// reason for using the raw record deserialization APIs is if you're using
/// Serde to read into borrowed data such as a `&'a str` or a `&'a [u8]`.
///
/// Two `StringRecord`s are compared on the basis of their field data. Any
/// position information associated with the records is ignored.
#[derive(Clone, Eq)]
pub struct StringRecord(ByteRecord);
impl PartialEq for StringRecord {
    fn eq(&self, other: &StringRecord) -> bool {
        self.0.iter_eq(&other.0)
    }
}
impl<T: AsRef<[u8]>> PartialEq<Vec<T>> for StringRecord {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.0.iter_eq(other)
    }
}
impl<'a, T: AsRef<[u8]>> PartialEq<Vec<T>> for &'a StringRecord {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.0.iter_eq(other)
    }
}
impl<T: AsRef<[u8]>> PartialEq<[T]> for StringRecord {
    fn eq(&self, other: &[T]) -> bool {
        self.0.iter_eq(other)
    }
}
impl<'a, T: AsRef<[u8]>> PartialEq<[T]> for &'a StringRecord {
    fn eq(&self, other: &[T]) -> bool {
        self.0.iter_eq(other)
    }
}
impl fmt::Debug for StringRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fields: Vec<&str> = self.iter().collect();
        write!(f, "StringRecord({:?})", fields)
    }
}
impl Default for StringRecord {
    #[inline]
    fn default() -> StringRecord {
        StringRecord::new()
    }
}
impl StringRecord {
    /// Create a new empty `StringRecord`.
    ///
    /// Note that you may find the `StringRecord::from` constructor more
    /// convenient, which is provided by an impl on the `From` trait.
    ///
    /// # Example: create an empty record
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let record = StringRecord::new();
    /// assert_eq!(record.len(), 0);
    /// ```
    ///
    /// # Example: initialize a record from a `Vec`
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let record = StringRecord::from(vec!["a", "b", "c"]);
    /// assert_eq!(record.len(), 3);
    /// ```
    #[inline]
    pub fn new() -> StringRecord {
        StringRecord(ByteRecord::new())
    }
    /// Create a new empty `StringRecord` with the given capacity.
    ///
    /// `buffer` refers to the capacity of the buffer used to store the
    /// actual row contents. `fields` refers to the number of fields one
    /// might expect to store.
    #[inline]
    pub fn with_capacity(buffer: usize, fields: usize) -> StringRecord {
        StringRecord(ByteRecord::with_capacity(buffer, fields))
    }
    /// Create a new `StringRecord` from a `ByteRecord`.
    ///
    /// Note that this does UTF-8 validation. If the given `ByteRecord` does
    /// not contain valid UTF-8, then this returns an error. The error includes
    /// the UTF-8 error and the original `ByteRecord`.
    ///
    /// # Example: valid UTF-8
    ///
    /// ```
    /// use std::error::Error;
    /// use csv::{ByteRecord, StringRecord};
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<dyn Error>> {
    ///     let byte_record = ByteRecord::from(vec!["a", "b", "c"]);
    ///     let str_record = StringRecord::from_byte_record(byte_record)?;
    ///     assert_eq!(str_record.len(), 3);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Example: invalid UTF-8
    ///
    /// ```
    /// use csv::{ByteRecord, StringRecord};
    ///
    /// let byte_record = ByteRecord::from(vec![
    ///     &b"quux"[..], &b"foo\xFFbar"[..], &b"c"[..],
    /// ]);
    /// let err = StringRecord::from_byte_record(byte_record).unwrap_err();
    /// assert_eq!(err.utf8_error().field(), 1);
    /// assert_eq!(err.utf8_error().valid_up_to(), 3);
    /// ```
    #[inline]
    pub fn from_byte_record(
        record: ByteRecord,
    ) -> result::Result<StringRecord, FromUtf8Error> {
        match record.validate() {
            Ok(()) => Ok(StringRecord(record)),
            Err(err) => Err(FromUtf8Error::new(record, err)),
        }
    }
    /// Lossily create a new `StringRecord` from a `ByteRecord`.
    ///
    /// This is like `StringRecord::from_byte_record`, except all invalid UTF-8
    /// sequences are replaced with the `U+FFFD REPLACEMENT CHARACTER`, which
    /// looks like this: �.
    ///
    /// # Example: valid UTF-8
    ///
    /// ```
    /// use csv::{ByteRecord, StringRecord};
    ///
    /// let byte_record = ByteRecord::from(vec!["a", "b", "c"]);
    /// let str_record = StringRecord::from_byte_record_lossy(byte_record);
    /// assert_eq!(str_record.len(), 3);
    /// ```
    ///
    /// # Example: invalid UTF-8
    ///
    /// ```
    /// use csv::{ByteRecord, StringRecord};
    ///
    /// let byte_record = ByteRecord::from(vec![
    ///     &b"quux"[..], &b"foo\xFFbar"[..], &b"c"[..],
    /// ]);
    /// let str_record = StringRecord::from_byte_record_lossy(byte_record);
    /// assert_eq!(&str_record[0], "quux");
    /// assert_eq!(&str_record[1], "foo�bar");
    /// assert_eq!(&str_record[2], "c");
    /// ```
    #[inline]
    pub fn from_byte_record_lossy(record: ByteRecord) -> StringRecord {
        if let Ok(()) = record.validate() {
            return StringRecord(record);
        }
        let mut str_record = StringRecord::with_capacity(
            record.as_slice().len(),
            record.len(),
        );
        for field in &record {
            str_record.push_field(&String::from_utf8_lossy(field));
        }
        str_record
    }
    /// Deserialize this record.
    ///
    /// The `D` type parameter refers to the type that this record should be
    /// deserialized into. The `'de` lifetime refers to the lifetime of the
    /// `StringRecord`. The `'de` lifetime permits deserializing into structs
    /// that borrow field data from this record.
    ///
    /// An optional `headers` parameter permits deserializing into a struct
    /// based on its field names (corresponding to header values) rather than
    /// the order in which the fields are defined.
    ///
    /// # Example: without headers
    ///
    /// This shows how to deserialize a single row into a struct based on the
    /// order in which fields occur. This example also shows how to borrow
    /// fields from the `StringRecord`, which results in zero allocation
    /// deserialization.
    ///
    /// ```
    /// use std::error::Error;
    ///
    /// use csv::StringRecord;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Row<'a> {
    ///     city: &'a str,
    ///     country: &'a str,
    ///     population: u64,
    /// }
    ///
    /// # fn main() { example().unwrap() }
    /// fn example() -> Result<(), Box<dyn Error>> {
    ///     let record = StringRecord::from(vec![
    ///         "Boston", "United States", "4628910",
    ///     ]);
    ///
    ///     let row: Row = record.deserialize(None)?;
    ///     assert_eq!(row.city, "Boston");
    ///     assert_eq!(row.country, "United States");
    ///     assert_eq!(row.population, 4628910);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Example: with headers
    ///
    /// This example is like the previous one, but shows how to deserialize
    /// into a struct based on the struct's field names. For this to work,
    /// you must provide a header row.
    ///
    /// This example also shows that you can deserialize into owned data
    /// types (e.g., `String`) instead of borrowed data types (e.g., `&str`).
    ///
    /// ```
    /// use std::error::Error;
    ///
    /// use csv::StringRecord;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Row {
    ///     city: String,
    ///     country: String,
    ///     population: u64,
    /// }
    ///
    /// # fn main() { example().unwrap() }
    /// fn example() -> Result<(), Box<dyn Error>> {
    ///     // Notice that the fields are not in the same order
    ///     // as the fields in the struct!
    ///     let header = StringRecord::from(vec![
    ///         "country", "city", "population",
    ///     ]);
    ///     let record = StringRecord::from(vec![
    ///         "United States", "Boston", "4628910",
    ///     ]);
    ///
    ///     let row: Row = record.deserialize(Some(&header))?;
    ///     assert_eq!(row.city, "Boston");
    ///     assert_eq!(row.country, "United States");
    ///     assert_eq!(row.population, 4628910);
    ///     Ok(())
    /// }
    /// ```
    pub fn deserialize<'de, D: Deserialize<'de>>(
        &'de self,
        headers: Option<&'de StringRecord>,
    ) -> Result<D> {
        deserialize_string_record(self, headers)
    }
    /// Returns an iterator over all fields in this record.
    ///
    /// # Example
    ///
    /// This example shows how to iterate over each field in a `StringRecord`.
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let record = StringRecord::from(vec!["a", "b", "c"]);
    /// for field in record.iter() {
    ///     assert!(field == "a" || field == "b" || field == "c");
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> StringRecordIter {
        self.into_iter()
    }
    /// Return the field at index `i`.
    ///
    /// If no field at index `i` exists, then this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let record = StringRecord::from(vec!["a", "b", "c"]);
    /// assert_eq!(record.get(1), Some("b"));
    /// assert_eq!(record.get(3), None);
    /// ```
    #[inline]
    pub fn get(&self, i: usize) -> Option<&str> {
        self.0
            .get(i)
            .map(|bytes| {
                debug_assert!(str::from_utf8(bytes).is_ok());
                unsafe { str::from_utf8_unchecked(bytes) }
            })
    }
    /// Returns true if and only if this record is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// assert!(StringRecord::new().is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns the number of fields in this record.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let record = StringRecord::from(vec!["a", "b", "c"]);
    /// assert_eq!(record.len(), 3);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Truncate this record to `n` fields.
    ///
    /// If `n` is greater than the number of fields in this record, then this
    /// has no effect.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let mut record = StringRecord::from(vec!["a", "b", "c"]);
    /// assert_eq!(record.len(), 3);
    /// record.truncate(1);
    /// assert_eq!(record.len(), 1);
    /// assert_eq!(record, vec!["a"]);
    /// ```
    #[inline]
    pub fn truncate(&mut self, n: usize) {
        self.0.truncate(n);
    }
    /// Clear this record so that it has zero fields.
    ///
    /// Note that it is not necessary to clear the record to reuse it with
    /// the CSV reader.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let mut record = StringRecord::from(vec!["a", "b", "c"]);
    /// assert_eq!(record.len(), 3);
    /// record.clear();
    /// assert_eq!(record.len(), 0);
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }
    /// Trim the fields of this record so that leading and trailing whitespace
    /// is removed.
    ///
    /// This method uses the Unicode definition of whitespace.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let mut record = StringRecord::from(vec![
    ///     "  ", "\u{3000}\tfoo ", "bar  ", "b a z",
    /// ]);
    /// record.trim();
    /// assert_eq!(record, vec!["", "foo", "bar", "b a z"]);
    /// ```
    pub fn trim(&mut self) {
        let length = self.len();
        if length == 0 {
            return;
        }
        let mut trimmed = StringRecord::with_capacity(self.as_slice().len(), self.len());
        trimmed.set_position(self.position().cloned());
        for field in &*self {
            trimmed.push_field(field.trim());
        }
        *self = trimmed;
    }
    /// Add a new field to this record.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let mut record = StringRecord::new();
    /// record.push_field("foo");
    /// assert_eq!(&record[0], "foo");
    /// ```
    #[inline]
    pub fn push_field(&mut self, field: &str) {
        self.0.push_field(field.as_bytes());
    }
    /// Return the position of this record, if available.
    ///
    /// # Example
    ///
    /// ```
    /// use std::error::Error;
    /// use csv::{StringRecord, ReaderBuilder};
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> Result<(), Box<dyn Error>> {
    ///     let mut record = StringRecord::new();
    ///     let mut rdr = ReaderBuilder::new()
    ///         .has_headers(false)
    ///         .from_reader("a,b,c\nx,y,z".as_bytes());
    ///
    ///     assert!(rdr.read_record(&mut record)?);
    ///     {
    ///         let pos = record.position().expect("a record position");
    ///         assert_eq!(pos.byte(), 0);
    ///         assert_eq!(pos.line(), 1);
    ///         assert_eq!(pos.record(), 0);
    ///     }
    ///
    ///     assert!(rdr.read_record(&mut record)?);
    ///     {
    ///         let pos = record.position().expect("a record position");
    ///         assert_eq!(pos.byte(), 6);
    ///         assert_eq!(pos.line(), 2);
    ///         assert_eq!(pos.record(), 1);
    ///     }
    ///
    ///     // Finish the CSV reader for good measure.
    ///     assert!(!rdr.read_record(&mut record)?);
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn position(&self) -> Option<&Position> {
        self.0.position()
    }
    /// Set the position of this record.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::{StringRecord, Position};
    ///
    /// let mut record = StringRecord::from(vec!["a", "b", "c"]);
    /// let mut pos = Position::new();
    /// pos.set_byte(100);
    /// pos.set_line(4);
    /// pos.set_record(2);
    ///
    /// record.set_position(Some(pos.clone()));
    /// assert_eq!(record.position(), Some(&pos));
    /// ```
    #[inline]
    pub fn set_position(&mut self, pos: Option<Position>) {
        self.0.set_position(pos);
    }
    /// Return the start and end position of a field in this record.
    ///
    /// If no such field exists at the given index, then return `None`.
    ///
    /// The range returned can be used with the slice returned by `as_slice`.
    /// Namely, the range returned is guaranteed to start and end at valid
    /// UTF-8 sequence boundaries.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let record = StringRecord::from(vec!["foo", "quux", "z"]);
    /// let range = record.range(1).expect("a record range");
    /// assert_eq!(&record.as_slice()[range], "quux");
    /// ```
    #[inline]
    pub fn range(&self, i: usize) -> Option<Range<usize>> {
        self.0.range(i)
    }
    /// Return the entire row as a single string slice. The slice returned
    /// stores all fields contiguously. The boundaries of each field can be
    /// determined via the `range` method.
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let record = StringRecord::from(vec!["foo", "quux", "z"]);
    /// assert_eq!(record.as_slice(), "fooquuxz");
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &str {
        debug_assert!(str::from_utf8(self.0.as_slice()).is_ok());
        unsafe { str::from_utf8_unchecked(self.0.as_slice()) }
    }
    /// Return a reference to this record's raw
    /// [`ByteRecord`](struct.ByteRecord.html).
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let str_record = StringRecord::from(vec!["a", "b", "c"]);
    /// let byte_record = str_record.as_byte_record();
    /// assert_eq!(&byte_record[2], b"c");
    /// ```
    #[inline]
    pub fn as_byte_record(&self) -> &ByteRecord {
        &self.0
    }
    /// Convert this `StringRecord` into a
    /// [`ByteRecord`](struct.ByteRecord.html).
    ///
    /// # Example
    ///
    /// ```
    /// use csv::StringRecord;
    ///
    /// let str_record = StringRecord::from(vec!["a", "b", "c"]);
    /// let byte_record = str_record.into_byte_record();
    /// assert_eq!(&byte_record[2], b"c");
    /// ```
    ///
    /// Note that this can also be achieved using the `From` impl:
    ///
    /// ```
    /// use csv::{ByteRecord, StringRecord};
    ///
    /// // Using ByteRecord::from...
    /// let str_record = StringRecord::from(vec!["a", "b", "c"]);
    /// assert_eq!(ByteRecord::from(str_record).len(), 3);
    ///
    /// // Using StringRecord::into...
    /// let str_record = StringRecord::from(vec!["a", "b", "c"]);
    /// let byte_record: ByteRecord = str_record.into();
    /// assert_eq!(byte_record.len(), 3);
    /// ```
    #[inline]
    pub fn into_byte_record(self) -> ByteRecord {
        self.0
    }
    /// A safe function for reading CSV data into a `StringRecord`.
    ///
    /// This relies on the internal representation of `StringRecord`.
    #[inline(always)]
    pub(crate) fn read<R: io::Read>(&mut self, rdr: &mut Reader<R>) -> Result<bool> {
        let pos = rdr.position().clone();
        let read_res = rdr.read_byte_record(&mut self.0);
        let utf8_res = match self.0.validate() {
            Ok(()) => Ok(()),
            Err(err) => {
                self.0.clear();
                Err(err)
            }
        };
        match (read_res, utf8_res) {
            (Err(err), _) => Err(err),
            (Ok(_), Err(err)) => {
                Err(
                    Error::new(ErrorKind::Utf8 {
                        pos: Some(pos),
                        err: err,
                    }),
                )
            }
            (Ok(eof), Ok(())) => Ok(eof),
        }
    }
}
impl ops::Index<usize> for StringRecord {
    type Output = str;
    #[inline]
    fn index(&self, i: usize) -> &str {
        self.get(i).unwrap()
    }
}
impl<T: AsRef<str>> From<Vec<T>> for StringRecord {
    #[inline]
    fn from(xs: Vec<T>) -> StringRecord {
        StringRecord::from_iter(xs.into_iter())
    }
}
impl<'a, T: AsRef<str>> From<&'a [T]> for StringRecord {
    #[inline]
    fn from(xs: &'a [T]) -> StringRecord {
        StringRecord::from_iter(xs)
    }
}
impl<T: AsRef<str>> FromIterator<T> for StringRecord {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> StringRecord {
        let mut record = StringRecord::new();
        record.extend(iter);
        record
    }
}
impl<T: AsRef<str>> Extend<T> for StringRecord {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for x in iter {
            self.push_field(x.as_ref());
        }
    }
}
impl<'a> IntoIterator for &'a StringRecord {
    type IntoIter = StringRecordIter<'a>;
    type Item = &'a str;
    #[inline]
    fn into_iter(self) -> StringRecordIter<'a> {
        StringRecordIter(self.0.iter())
    }
}
/// An iterator over the fields in a string record.
///
/// The `'r` lifetime variable refers to the lifetime of the `StringRecord`
/// that is being iterated over.
pub struct StringRecordIter<'r>(ByteRecordIter<'r>);
impl<'r> Iterator for StringRecordIter<'r> {
    type Item = &'r str;
    #[inline]
    fn next(&mut self) -> Option<&'r str> {
        self.0
            .next()
            .map(|bytes| {
                debug_assert!(str::from_utf8(bytes).is_ok());
                unsafe { str::from_utf8_unchecked(bytes) }
            })
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    #[inline]
    fn count(self) -> usize {
        self.0.len()
    }
}
impl<'r> DoubleEndedIterator for StringRecordIter<'r> {
    #[inline]
    fn next_back(&mut self) -> Option<&'r str> {
        self.0
            .next_back()
            .map(|bytes| {
                debug_assert!(str::from_utf8(bytes).is_ok());
                unsafe { str::from_utf8_unchecked(bytes) }
            })
    }
}
#[cfg(test)]
mod tests {
    use crate::string_record::StringRecord;
    #[test]
    fn trim_front() {
        let mut rec = StringRecord::from(vec![" abc"]);
        rec.trim();
        assert_eq!(rec.get(0), Some("abc"));
        let mut rec = StringRecord::from(vec![" abc", "  xyz"]);
        rec.trim();
        assert_eq!(rec.get(0), Some("abc"));
        assert_eq!(rec.get(1), Some("xyz"));
    }
    #[test]
    fn trim_back() {
        let mut rec = StringRecord::from(vec!["abc "]);
        rec.trim();
        assert_eq!(rec.get(0), Some("abc"));
        let mut rec = StringRecord::from(vec!["abc ", "xyz  "]);
        rec.trim();
        assert_eq!(rec.get(0), Some("abc"));
        assert_eq!(rec.get(1), Some("xyz"));
    }
    #[test]
    fn trim_both() {
        let mut rec = StringRecord::from(vec![" abc "]);
        rec.trim();
        assert_eq!(rec.get(0), Some("abc"));
        let mut rec = StringRecord::from(vec![" abc ", "  xyz  "]);
        rec.trim();
        assert_eq!(rec.get(0), Some("abc"));
        assert_eq!(rec.get(1), Some("xyz"));
    }
    #[test]
    fn trim_does_not_panic_on_empty_records_1() {
        let mut rec = StringRecord::from(vec![""]);
        rec.trim();
        assert_eq!(rec.get(0), Some(""));
    }
    #[test]
    fn trim_does_not_panic_on_empty_records_2() {
        let mut rec = StringRecord::from(vec!["", ""]);
        rec.trim();
        assert_eq!(rec.get(0), Some(""));
        assert_eq!(rec.get(1), Some(""));
    }
    #[test]
    fn trim_does_not_panic_on_empty_records_3() {
        let mut rec = StringRecord::new();
        rec.trim();
        assert_eq!(rec.as_slice().len(), 0);
    }
    #[test]
    fn trim_whitespace_only() {
        let mut rec = StringRecord::from(
            vec![
                "\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0020}\u{0085}\u{00A0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}",
            ],
        );
        rec.trim();
        assert_eq!(rec.get(0), Some(""));
    }
    #[test]
    fn eq_field_boundaries() {
        let test1 = StringRecord::from(vec!["12", "34"]);
        let test2 = StringRecord::from(vec!["123", "4"]);
        assert_ne!(test1, test2);
    }
    #[test]
    fn eq_record_len() {
        let test1 = StringRecord::from(vec!["12", "34", "56"]);
        let test2 = StringRecord::from(vec!["12", "34"]);
        assert_ne!(test1, test2);
    }
}
#[cfg(test)]
mod tests_rug_353 {
    use super::*;
    use crate::StringRecord;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_353_rrrruuuugggg_test_default = 0;
        let _default_record: StringRecord = Default::default();
        let _rug_ed_tests_rug_353_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_354 {
    use super::*;
    use crate::{StringRecord, ByteRecord};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_354_rrrruuuugggg_test_rug = 0;
        let record: StringRecord = StringRecord::new();
        let _rug_ed_tests_rug_354_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_355 {
    use super::*;
    use crate::string_record::StringRecord;
    use crate::byte_record::ByteRecord;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        let mut p1: usize = rug_fuzz_1;
        StringRecord::with_capacity(p0, p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_356 {
    use super::*;
    use crate::{ByteRecord, StringRecord, FromUtf8Error};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_356_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"quux";
        let mut p0 = ByteRecord::from(
            vec![& rug_fuzz_0[..], & b"foo\xFFbar"[..], & b"c"[..]],
        );
        let err = StringRecord::from_byte_record(p0).unwrap_err();
        debug_assert_eq!(err.utf8_error().field(), 1);
        debug_assert_eq!(err.utf8_error().valid_up_to(), 3);
        let _rug_ed_tests_rug_356_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_357 {
    use crate::{ByteRecord, StringRecord};
    use std::iter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_357_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"quux";
        let p0 = ByteRecord::from(
            vec![& rug_fuzz_0[..], & b"foo\xFFbar"[..], & b"c"[..]],
        );
        StringRecord::from_byte_record_lossy(p0);
        let _rug_ed_tests_rug_357_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_359 {
    use super::*;
    use crate::StringRecord;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = StringRecord::from(vec![rug_fuzz_0, "b", "c"]);
        p0.iter();
             }
});    }
}
#[cfg(test)]
mod tests_rug_360 {
    use super::*;
    use crate::StringRecord;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = StringRecord::from(vec![rug_fuzz_0, "b", "c"]);
        let mut p1: usize = rug_fuzz_1;
        p0.get(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_361 {
    use super::*;
    use crate::StringRecord;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_361_rrrruuuugggg_test_rug = 0;
        let mut p0 = StringRecord::new();
        debug_assert_eq!(p0.is_empty(), true);
        let _rug_ed_tests_rug_361_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_362 {
    use super::*;
    use crate::StringRecord;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = StringRecord::from(vec![rug_fuzz_0, "b", "c"]);
        debug_assert_eq!(p0.len(), 3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_363 {
    use super::*;
    use crate::StringRecord;
    #[test]
    fn test_truncate() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut record = StringRecord::from(vec![rug_fuzz_0, "b", "c"]);
        let n = rug_fuzz_1;
        record.truncate(n);
        debug_assert_eq!(record.len(), 1);
        debug_assert_eq!(record, vec!["a"]);
             }
});    }
}
#[cfg(test)]
mod tests_rug_364 {
    use super::*;
    use crate::StringRecord;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = StringRecord::from(vec![rug_fuzz_0, "b", "c"]);
        p0.clear();
        debug_assert_eq!(p0.len(), 0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_365 {
    use super::*;
    use crate::StringRecord;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = StringRecord::from(
            vec![rug_fuzz_0, "\u{3000}\tfoo ", "bar  ", "b a z"],
        );
        p0.trim();
        debug_assert_eq!(p0, vec!["", "foo", "bar", "b a z"]);
             }
});    }
}
#[cfg(test)]
mod tests_rug_366 {
    use super::*;
    use crate::StringRecord;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = StringRecord::new();
        let p1 = rug_fuzz_0;
        p0.push_field(&p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_367 {
    use super::*;
    use crate::{StringRecord, ReaderBuilder};
    use std::error::Error;
    #[test]
    fn test_position() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(bool, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut record = StringRecord::new();
        let mut rdr = ReaderBuilder::new()
            .has_headers(rug_fuzz_0)
            .from_reader(rug_fuzz_1.as_bytes());
        debug_assert!(rdr.read_record(& mut record).is_ok());
        {
            let pos = record.position().expect(rug_fuzz_2);
            debug_assert_eq!(pos.byte(), 0);
            debug_assert_eq!(pos.line(), 1);
            debug_assert_eq!(pos.record(), 0);
        }
        debug_assert!(rdr.read_record(& mut record).is_ok());
        {
            let pos = record.position().expect(rug_fuzz_3);
            debug_assert_eq!(pos.byte(), 6);
            debug_assert_eq!(pos.line(), 2);
            debug_assert_eq!(pos.record(), 1);
        }
        debug_assert!(! rdr.read_record(& mut record).is_ok());
             }
});    }
}
#[cfg(test)]
mod tests_rug_368 {
    use super::*;
    use crate::{StringRecord, Position};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(&str, u64, u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut record = StringRecord::from(vec![rug_fuzz_0, "b", "c"]);
        let mut pos = Position::new();
        pos.set_byte(rug_fuzz_1);
        pos.set_line(rug_fuzz_2);
        pos.set_record(rug_fuzz_3);
        StringRecord::set_position(&mut record, Some(pos.clone()));
             }
});    }
}
#[cfg(test)]
mod tests_rug_369 {
    use super::*;
    use crate::StringRecord;
    use std::ops::Range;
    #[test]
    fn test_range() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = StringRecord::default();
        let mut p1: usize = rug_fuzz_0;
        p0.range(p1);
             }
});    }
}
#[cfg(test)]
mod tests_rug_370 {
    use crate::StringRecord;
    #[test]
    fn test_as_slice() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = StringRecord::from(vec![rug_fuzz_0, "bar", "baz"]);
        debug_assert_eq!(p0.as_slice(), "foobarbaz");
             }
});    }
}
#[cfg(test)]
mod tests_rug_371 {
    use super::*;
    use crate::{StringRecord, ByteRecord};
    #[test]
    fn test_as_byte_record() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = StringRecord::from(vec![rug_fuzz_0, "b", "c"]);
        p0.as_byte_record();
             }
});    }
}
#[cfg(test)]
mod tests_rug_372 {
    use super::*;
    use crate::{StringRecord, ByteRecord};
    #[test]
    fn test_into_byte_record() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = StringRecord::from(vec![rug_fuzz_0, "b", "c"]);
        let byte_record = StringRecord::into_byte_record(p0);
        debug_assert_eq!(& byte_record[rug_fuzz_1], b"c");
             }
});    }
}
