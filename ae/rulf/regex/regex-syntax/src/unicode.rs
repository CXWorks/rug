use std::error;
use std::fmt;
use std::result;

use hir;

/// A type alias for errors specific to Unicode handling of classes.
pub type Result<T> = result::Result<T, Error>;

/// An inclusive range of codepoints from a generated file (hence the static
/// lifetime).
type Range = &'static [(char, char)];

/// An error that occurs when dealing with Unicode.
///
/// We don't impl the Error trait here because these always get converted
/// into other public errors. (This error type isn't exported.)
#[derive(Debug)]
pub enum Error {
    PropertyNotFound,
    PropertyValueNotFound,
    // Not used when unicode-perl is enabled.
    #[allow(dead_code)]
    PerlClassNotFound,
}

/// A type alias for errors specific to Unicode case folding.
pub type FoldResult<T> = result::Result<T, CaseFoldError>;

/// An error that occurs when Unicode-aware simple case folding fails.
///
/// This error can occur when the case mapping tables necessary for Unicode
/// aware case folding are unavailable. This only occurs when the
/// `unicode-case` feature is disabled. (The feature is enabled by default.)
#[derive(Debug)]
pub struct CaseFoldError(());

impl error::Error for CaseFoldError {}

impl fmt::Display for CaseFoldError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Unicode-aware case folding is not available \
             (probably because the unicode-case feature is not enabled)"
        )
    }
}

/// An error that occurs when the Unicode-aware `\w` class is unavailable.
///
/// This error can occur when the data tables necessary for the Unicode aware
/// Perl character class `\w` are unavailable. This only occurs when the
/// `unicode-perl` feature is disabled. (The feature is enabled by default.)
#[derive(Debug)]
pub struct UnicodeWordError(());

impl error::Error for UnicodeWordError {}

impl fmt::Display for UnicodeWordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Unicode-aware \\w class is not available \
             (probably because the unicode-perl feature is not enabled)"
        )
    }
}

/// Return an iterator over the equivalence class of simple case mappings
/// for the given codepoint. The equivalence class does not include the
/// given codepoint.
///
/// If the equivalence class is empty, then this returns the next scalar
/// value that has a non-empty equivalence class, if it exists. If no such
/// scalar value exists, then `None` is returned. The point of this behavior
/// is to permit callers to avoid calling `simple_fold` more than they need
/// to, since there is some cost to fetching the equivalence class.
///
/// This returns an error if the Unicode case folding tables are not available.
pub fn simple_fold(
    c: char,
) -> FoldResult<result::Result<impl Iterator<Item = char>, Option<char>>> {
    #[cfg(not(feature = "unicode-case"))]
    fn imp(
        _: char,
    ) -> FoldResult<result::Result<impl Iterator<Item = char>, Option<char>>>
    {
        use std::option::IntoIter;
        Err::<result::Result<IntoIter<char>, _>, _>(CaseFoldError(()))
    }

    #[cfg(feature = "unicode-case")]
    fn imp(
        c: char,
    ) -> FoldResult<result::Result<impl Iterator<Item = char>, Option<char>>>
    {
        use unicode_tables::case_folding_simple::CASE_FOLDING_SIMPLE;

        Ok(CASE_FOLDING_SIMPLE
            .binary_search_by_key(&c, |&(c1, _)| c1)
            .map(|i| CASE_FOLDING_SIMPLE[i].1.iter().map(|&c| c))
            .map_err(|i| {
                if i >= CASE_FOLDING_SIMPLE.len() {
                    None
                } else {
                    Some(CASE_FOLDING_SIMPLE[i].0)
                }
            }))
    }

    imp(c)
}

/// Returns true if and only if the given (inclusive) range contains at least
/// one Unicode scalar value that has a non-empty non-trivial simple case
/// mapping.
///
/// This function panics if `end < start`.
///
/// This returns an error if the Unicode case folding tables are not available.
pub fn contains_simple_case_mapping(
    start: char,
    end: char,
) -> FoldResult<bool> {
    #[cfg(not(feature = "unicode-case"))]
    fn imp(_: char, _: char) -> FoldResult<bool> {
        Err(CaseFoldError(()))
    }

    #[cfg(feature = "unicode-case")]
    fn imp(start: char, end: char) -> FoldResult<bool> {
        use std::cmp::Ordering;
        use unicode_tables::case_folding_simple::CASE_FOLDING_SIMPLE;

        assert!(start <= end);
        Ok(CASE_FOLDING_SIMPLE
            .binary_search_by(|&(c, _)| {
                if start <= c && c <= end {
                    Ordering::Equal
                } else if c > end {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            })
            .is_ok())
    }

    imp(start, end)
}

/// A query for finding a character class defined by Unicode. This supports
/// either use of a property name directly, or lookup by property value. The
/// former generally refers to Binary properties (see UTS#44, Table 8), but
/// as a special exception (see UTS#18, Section 1.2) both general categories
/// (an enumeration) and scripts (a catalog) are supported as if each of their
/// possible values were a binary property.
///
/// In all circumstances, property names and values are normalized and
/// canonicalized. That is, `GC == gc == GeneralCategory == general_category`.
///
/// The lifetime `'a` refers to the shorter of the lifetimes of property name
/// and property value.
#[derive(Debug)]
pub enum ClassQuery<'a> {
    /// Return a class corresponding to a Unicode binary property, named by
    /// a single letter.
    OneLetter(char),
    /// Return a class corresponding to a Unicode binary property.
    ///
    /// Note that, by special exception (see UTS#18, Section 1.2), both
    /// general category values and script values are permitted here as if
    /// they were a binary property.
    Binary(&'a str),
    /// Return a class corresponding to all codepoints whose property
    /// (identified by `property_name`) corresponds to the given value
    /// (identified by `property_value`).
    ByValue {
        /// A property name.
        property_name: &'a str,
        /// A property value.
        property_value: &'a str,
    },
}

impl<'a> ClassQuery<'a> {
    fn canonicalize(&self) -> Result<CanonicalClassQuery> {
        match *self {
            ClassQuery::OneLetter(c) => self.canonical_binary(&c.to_string()),
            ClassQuery::Binary(name) => self.canonical_binary(name),
            ClassQuery::ByValue { property_name, property_value } => {
                let property_name = symbolic_name_normalize(property_name);
                let property_value = symbolic_name_normalize(property_value);

                let canon_name = match canonical_prop(&property_name)? {
                    None => return Err(Error::PropertyNotFound),
                    Some(canon_name) => canon_name,
                };
                Ok(match canon_name {
                    "General_Category" => {
                        let canon = match canonical_gencat(&property_value)? {
                            None => return Err(Error::PropertyValueNotFound),
                            Some(canon) => canon,
                        };
                        CanonicalClassQuery::GeneralCategory(canon)
                    }
                    "Script" => {
                        let canon = match canonical_script(&property_value)? {
                            None => return Err(Error::PropertyValueNotFound),
                            Some(canon) => canon,
                        };
                        CanonicalClassQuery::Script(canon)
                    }
                    _ => {
                        let vals = match property_values(canon_name)? {
                            None => return Err(Error::PropertyValueNotFound),
                            Some(vals) => vals,
                        };
                        let canon_val =
                            match canonical_value(vals, &property_value) {
                                None => {
                                    return Err(Error::PropertyValueNotFound)
                                }
                                Some(canon_val) => canon_val,
                            };
                        CanonicalClassQuery::ByValue {
                            property_name: canon_name,
                            property_value: canon_val,
                        }
                    }
                })
            }
        }
    }

    fn canonical_binary(&self, name: &str) -> Result<CanonicalClassQuery> {
        let norm = symbolic_name_normalize(name);

        // This is a special case where 'cf' refers to the 'Format' general
        // category, but where the 'cf' abbreviation is also an abbreviation
        // for the 'Case_Folding' property. But we want to treat it as
        // a general category. (Currently, we don't even support the
        // 'Case_Folding' property. But if we do in the future, users will be
        // required to spell it out.)
        if norm != "cf" {
            if let Some(canon) = canonical_prop(&norm)? {
                return Ok(CanonicalClassQuery::Binary(canon));
            }
        }
        if let Some(canon) = canonical_gencat(&norm)? {
            return Ok(CanonicalClassQuery::GeneralCategory(canon));
        }
        if let Some(canon) = canonical_script(&norm)? {
            return Ok(CanonicalClassQuery::Script(canon));
        }
        Err(Error::PropertyNotFound)
    }
}

/// Like ClassQuery, but its parameters have been canonicalized. This also
/// differentiates binary properties from flattened general categories and
/// scripts.
#[derive(Debug, Eq, PartialEq)]
enum CanonicalClassQuery {
    /// The canonical binary property name.
    Binary(&'static str),
    /// The canonical general category name.
    GeneralCategory(&'static str),
    /// The canonical script name.
    Script(&'static str),
    /// An arbitrary association between property and value, both of which
    /// have been canonicalized.
    ///
    /// Note that by construction, the property name of ByValue will never
    /// be General_Category or Script. Those two cases are subsumed by the
    /// eponymous variants.
    ByValue {
        /// The canonical property name.
        property_name: &'static str,
        /// The canonical property value.
        property_value: &'static str,
    },
}

/// Looks up a Unicode class given a query. If one doesn't exist, then
/// `None` is returned.
pub fn class(query: ClassQuery) -> Result<hir::ClassUnicode> {
    use self::CanonicalClassQuery::*;

    match query.canonicalize()? {
        Binary(name) => bool_property(name),
        GeneralCategory(name) => gencat(name),
        Script(name) => script(name),
        ByValue { property_name: "Age", property_value } => {
            let mut class = hir::ClassUnicode::empty();
            for set in ages(property_value)? {
                class.union(&hir_class(set));
            }
            Ok(class)
        }
        ByValue { property_name: "Script_Extensions", property_value } => {
            script_extension(property_value)
        }
        ByValue {
            property_name: "Grapheme_Cluster_Break",
            property_value,
        } => gcb(property_value),
        ByValue { property_name: "Sentence_Break", property_value } => {
            sb(property_value)
        }
        ByValue { property_name: "Word_Break", property_value } => {
            wb(property_value)
        }
        _ => {
            // What else should we support?
            Err(Error::PropertyNotFound)
        }
    }
}

/// Returns a Unicode aware class for \w.
///
/// This returns an error if the data is not available for \w.
pub fn perl_word() -> Result<hir::ClassUnicode> {
    #[cfg(not(feature = "unicode-perl"))]
    fn imp() -> Result<hir::ClassUnicode> {
        Err(Error::PerlClassNotFound)
    }

    #[cfg(feature = "unicode-perl")]
    fn imp() -> Result<hir::ClassUnicode> {
        use unicode_tables::perl_word::PERL_WORD;
        Ok(hir_class(PERL_WORD))
    }

    imp()
}

/// Returns a Unicode aware class for \s.
///
/// This returns an error if the data is not available for \s.
pub fn perl_space() -> Result<hir::ClassUnicode> {
    #[cfg(not(any(feature = "unicode-perl", feature = "unicode-bool")))]
    fn imp() -> Result<hir::ClassUnicode> {
        Err(Error::PerlClassNotFound)
    }

    #[cfg(all(feature = "unicode-perl", not(feature = "unicode-bool")))]
    fn imp() -> Result<hir::ClassUnicode> {
        use unicode_tables::perl_space::WHITE_SPACE;
        Ok(hir_class(WHITE_SPACE))
    }

    #[cfg(feature = "unicode-bool")]
    fn imp() -> Result<hir::ClassUnicode> {
        use unicode_tables::property_bool::WHITE_SPACE;
        Ok(hir_class(WHITE_SPACE))
    }

    imp()
}

/// Returns a Unicode aware class for \d.
///
/// This returns an error if the data is not available for \d.
pub fn perl_digit() -> Result<hir::ClassUnicode> {
    #[cfg(not(any(feature = "unicode-perl", feature = "unicode-gencat")))]
    fn imp() -> Result<hir::ClassUnicode> {
        Err(Error::PerlClassNotFound)
    }

    #[cfg(all(feature = "unicode-perl", not(feature = "unicode-gencat")))]
    fn imp() -> Result<hir::ClassUnicode> {
        use unicode_tables::perl_decimal::DECIMAL_NUMBER;
        Ok(hir_class(DECIMAL_NUMBER))
    }

    #[cfg(feature = "unicode-gencat")]
    fn imp() -> Result<hir::ClassUnicode> {
        use unicode_tables::general_category::DECIMAL_NUMBER;
        Ok(hir_class(DECIMAL_NUMBER))
    }

    imp()
}

/// Build a Unicode HIR class from a sequence of Unicode scalar value ranges.
pub fn hir_class(ranges: &[(char, char)]) -> hir::ClassUnicode {
    let hir_ranges: Vec<hir::ClassUnicodeRange> = ranges
        .iter()
        .map(|&(s, e)| hir::ClassUnicodeRange::new(s, e))
        .collect();
    hir::ClassUnicode::new(hir_ranges)
}

/// Returns true only if the given codepoint is in the `\w` character class.
///
/// If the `unicode-perl` feature is not enabled, then this returns an error.
pub fn is_word_character(c: char) -> result::Result<bool, UnicodeWordError> {
    #[cfg(not(feature = "unicode-perl"))]
    fn imp(_: char) -> result::Result<bool, UnicodeWordError> {
        Err(UnicodeWordError(()))
    }

    #[cfg(feature = "unicode-perl")]
    fn imp(c: char) -> result::Result<bool, UnicodeWordError> {
        use is_word_byte;
        use std::cmp::Ordering;
        use unicode_tables::perl_word::PERL_WORD;

        if c <= 0x7F as char && is_word_byte(c as u8) {
            return Ok(true);
        }
        Ok(PERL_WORD
            .binary_search_by(|&(start, end)| {
                if start <= c && c <= end {
                    Ordering::Equal
                } else if start > c {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            })
            .is_ok())
    }

    imp(c)
}

/// A mapping of property values for a specific property.
///
/// The first element of each tuple is a normalized property value while the
/// second element of each tuple is the corresponding canonical property
/// value.
type PropertyValues = &'static [(&'static str, &'static str)];

fn canonical_gencat(normalized_value: &str) -> Result<Option<&'static str>> {
    Ok(match normalized_value {
        "any" => Some("Any"),
        "assigned" => Some("Assigned"),
        "ascii" => Some("ASCII"),
        _ => {
            let gencats = property_values("General_Category")?.unwrap();
            canonical_value(gencats, normalized_value)
        }
    })
}

fn canonical_script(normalized_value: &str) -> Result<Option<&'static str>> {
    let scripts = property_values("Script")?.unwrap();
    Ok(canonical_value(scripts, normalized_value))
}

/// Find the canonical property name for the given normalized property name.
///
/// If no such property exists, then `None` is returned.
///
/// The normalized property name must have been normalized according to
/// UAX44 LM3, which can be done using `symbolic_name_normalize`.
///
/// If the property names data is not available, then an error is returned.
fn canonical_prop(normalized_name: &str) -> Result<Option<&'static str>> {
    #[cfg(not(any(
        feature = "unicode-age",
        feature = "unicode-bool",
        feature = "unicode-gencat",
        feature = "unicode-perl",
        feature = "unicode-script",
        feature = "unicode-segment",
    )))]
    fn imp(_: &str) -> Result<Option<&'static str>> {
        Err(Error::PropertyNotFound)
    }

    #[cfg(any(
        feature = "unicode-age",
        feature = "unicode-bool",
        feature = "unicode-gencat",
        feature = "unicode-perl",
        feature = "unicode-script",
        feature = "unicode-segment",
    ))]
    fn imp(name: &str) -> Result<Option<&'static str>> {
        use unicode_tables::property_names::PROPERTY_NAMES;

        Ok(PROPERTY_NAMES
            .binary_search_by_key(&name, |&(n, _)| n)
            .ok()
            .map(|i| PROPERTY_NAMES[i].1))
    }

    imp(normalized_name)
}

/// Find the canonical property value for the given normalized property
/// value.
///
/// The given property values should correspond to the values for the property
/// under question, which can be found using `property_values`.
///
/// If no such property value exists, then `None` is returned.
///
/// The normalized property value must have been normalized according to
/// UAX44 LM3, which can be done using `symbolic_name_normalize`.
fn canonical_value(
    vals: PropertyValues,
    normalized_value: &str,
) -> Option<&'static str> {
    vals.binary_search_by_key(&normalized_value, |&(n, _)| n)
        .ok()
        .map(|i| vals[i].1)
}

/// Return the table of property values for the given property name.
///
/// If the property values data is not available, then an error is returned.
fn property_values(
    canonical_property_name: &'static str,
) -> Result<Option<PropertyValues>> {
    #[cfg(not(any(
        feature = "unicode-age",
        feature = "unicode-bool",
        feature = "unicode-gencat",
        feature = "unicode-perl",
        feature = "unicode-script",
        feature = "unicode-segment",
    )))]
    fn imp(_: &'static str) -> Result<Option<PropertyValues>> {
        Err(Error::PropertyValueNotFound)
    }

    #[cfg(any(
        feature = "unicode-age",
        feature = "unicode-bool",
        feature = "unicode-gencat",
        feature = "unicode-perl",
        feature = "unicode-script",
        feature = "unicode-segment",
    ))]
    fn imp(name: &'static str) -> Result<Option<PropertyValues>> {
        use unicode_tables::property_values::PROPERTY_VALUES;

        Ok(PROPERTY_VALUES
            .binary_search_by_key(&name, |&(n, _)| n)
            .ok()
            .map(|i| PROPERTY_VALUES[i].1))
    }

    imp(canonical_property_name)
}

// This is only used in some cases, but small enough to just let it be dead
// instead of figuring out (and maintaining) the right set of features.
#[allow(dead_code)]
fn property_set(
    name_map: &'static [(&'static str, Range)],
    canonical: &'static str,
) -> Option<Range> {
    name_map
        .binary_search_by_key(&canonical, |x| x.0)
        .ok()
        .map(|i| name_map[i].1)
}

/// Returns an iterator over Unicode Age sets. Each item corresponds to a set
/// of codepoints that were added in a particular revision of Unicode. The
/// iterator yields items in chronological order.
///
/// If the given age value isn't valid or if the data isn't available, then an
/// error is returned instead.
fn ages(canonical_age: &str) -> Result<impl Iterator<Item = Range>> {
    #[cfg(not(feature = "unicode-age"))]
    fn imp(_: &str) -> Result<impl Iterator<Item = Range>> {
        use std::option::IntoIter;
        Err::<IntoIter<Range>, _>(Error::PropertyNotFound)
    }

    #[cfg(feature = "unicode-age")]
    fn imp(canonical_age: &str) -> Result<impl Iterator<Item = Range>> {
        use unicode_tables::age;

        const AGES: &'static [(&'static str, Range)] = &[
            ("V1_1", age::V1_1),
            ("V2_0", age::V2_0),
            ("V2_1", age::V2_1),
            ("V3_0", age::V3_0),
            ("V3_1", age::V3_1),
            ("V3_2", age::V3_2),
            ("V4_0", age::V4_0),
            ("V4_1", age::V4_1),
            ("V5_0", age::V5_0),
            ("V5_1", age::V5_1),
            ("V5_2", age::V5_2),
            ("V6_0", age::V6_0),
            ("V6_1", age::V6_1),
            ("V6_2", age::V6_2),
            ("V6_3", age::V6_3),
            ("V7_0", age::V7_0),
            ("V8_0", age::V8_0),
            ("V9_0", age::V9_0),
            ("V10_0", age::V10_0),
            ("V11_0", age::V11_0),
            ("V12_0", age::V12_0),
            ("V12_1", age::V12_1),
            ("V13_0", age::V13_0),
        ];
        assert_eq!(AGES.len(), age::BY_NAME.len(), "ages are out of sync");

        let pos = AGES.iter().position(|&(age, _)| canonical_age == age);
        match pos {
            None => Err(Error::PropertyValueNotFound),
            Some(i) => Ok(AGES[..i + 1].iter().map(|&(_, classes)| classes)),
        }
    }

    imp(canonical_age)
}

/// Returns the Unicode HIR class corresponding to the given general category.
///
/// Name canonicalization is assumed to be performed by the caller.
///
/// If the given general category could not be found, or if the general
/// category data is not available, then an error is returned.
fn gencat(canonical_name: &'static str) -> Result<hir::ClassUnicode> {
    #[cfg(not(feature = "unicode-gencat"))]
    fn imp(_: &'static str) -> Result<hir::ClassUnicode> {
        Err(Error::PropertyNotFound)
    }

    #[cfg(feature = "unicode-gencat")]
    fn imp(name: &'static str) -> Result<hir::ClassUnicode> {
        use unicode_tables::general_category::BY_NAME;
        match name {
            "ASCII" => Ok(hir_class(&[('\0', '\x7F')])),
            "Any" => Ok(hir_class(&[('\0', '\u{10FFFF}')])),
            "Assigned" => {
                let mut cls = gencat("Unassigned")?;
                cls.negate();
                Ok(cls)
            }
            name => property_set(BY_NAME, name)
                .map(hir_class)
                .ok_or(Error::PropertyValueNotFound),
        }
    }

    match canonical_name {
        "Decimal_Number" => perl_digit(),
        name => imp(name),
    }
}

/// Returns the Unicode HIR class corresponding to the given script.
///
/// Name canonicalization is assumed to be performed by the caller.
///
/// If the given script could not be found, or if the script data is not
/// available, then an error is returned.
fn script(canonical_name: &'static str) -> Result<hir::ClassUnicode> {
    #[cfg(not(feature = "unicode-script"))]
    fn imp(_: &'static str) -> Result<hir::ClassUnicode> {
        Err(Error::PropertyNotFound)
    }

    #[cfg(feature = "unicode-script")]
    fn imp(name: &'static str) -> Result<hir::ClassUnicode> {
        use unicode_tables::script::BY_NAME;
        property_set(BY_NAME, name)
            .map(hir_class)
            .ok_or(Error::PropertyValueNotFound)
    }

    imp(canonical_name)
}

/// Returns the Unicode HIR class corresponding to the given script extension.
///
/// Name canonicalization is assumed to be performed by the caller.
///
/// If the given script extension could not be found, or if the script data is
/// not available, then an error is returned.
fn script_extension(
    canonical_name: &'static str,
) -> Result<hir::ClassUnicode> {
    #[cfg(not(feature = "unicode-script"))]
    fn imp(_: &'static str) -> Result<hir::ClassUnicode> {
        Err(Error::PropertyNotFound)
    }

    #[cfg(feature = "unicode-script")]
    fn imp(name: &'static str) -> Result<hir::ClassUnicode> {
        use unicode_tables::script_extension::BY_NAME;
        property_set(BY_NAME, name)
            .map(hir_class)
            .ok_or(Error::PropertyValueNotFound)
    }

    imp(canonical_name)
}

/// Returns the Unicode HIR class corresponding to the given Unicode boolean
/// property.
///
/// Name canonicalization is assumed to be performed by the caller.
///
/// If the given boolean property could not be found, or if the boolean
/// property data is not available, then an error is returned.
fn bool_property(canonical_name: &'static str) -> Result<hir::ClassUnicode> {
    #[cfg(not(feature = "unicode-bool"))]
    fn imp(_: &'static str) -> Result<hir::ClassUnicode> {
        Err(Error::PropertyNotFound)
    }

    #[cfg(feature = "unicode-bool")]
    fn imp(name: &'static str) -> Result<hir::ClassUnicode> {
        use unicode_tables::property_bool::BY_NAME;
        property_set(BY_NAME, name)
            .map(hir_class)
            .ok_or(Error::PropertyNotFound)
    }

    match canonical_name {
        "Decimal_Number" => perl_digit(),
        "White_Space" => perl_space(),
        name => imp(name),
    }
}

/// Returns the Unicode HIR class corresponding to the given grapheme cluster
/// break property.
///
/// Name canonicalization is assumed to be performed by the caller.
///
/// If the given property could not be found, or if the corresponding data is
/// not available, then an error is returned.
fn gcb(canonical_name: &'static str) -> Result<hir::ClassUnicode> {
    #[cfg(not(feature = "unicode-segment"))]
    fn imp(_: &'static str) -> Result<hir::ClassUnicode> {
        Err(Error::PropertyNotFound)
    }

    #[cfg(feature = "unicode-segment")]
    fn imp(name: &'static str) -> Result<hir::ClassUnicode> {
        use unicode_tables::grapheme_cluster_break::BY_NAME;
        property_set(BY_NAME, name)
            .map(hir_class)
            .ok_or(Error::PropertyValueNotFound)
    }

    imp(canonical_name)
}

/// Returns the Unicode HIR class corresponding to the given word break
/// property.
///
/// Name canonicalization is assumed to be performed by the caller.
///
/// If the given property could not be found, or if the corresponding data is
/// not available, then an error is returned.
fn wb(canonical_name: &'static str) -> Result<hir::ClassUnicode> {
    #[cfg(not(feature = "unicode-segment"))]
    fn imp(_: &'static str) -> Result<hir::ClassUnicode> {
        Err(Error::PropertyNotFound)
    }

    #[cfg(feature = "unicode-segment")]
    fn imp(name: &'static str) -> Result<hir::ClassUnicode> {
        use unicode_tables::word_break::BY_NAME;
        property_set(BY_NAME, name)
            .map(hir_class)
            .ok_or(Error::PropertyValueNotFound)
    }

    imp(canonical_name)
}

/// Returns the Unicode HIR class corresponding to the given sentence
/// break property.
///
/// Name canonicalization is assumed to be performed by the caller.
///
/// If the given property could not be found, or if the corresponding data is
/// not available, then an error is returned.
fn sb(canonical_name: &'static str) -> Result<hir::ClassUnicode> {
    #[cfg(not(feature = "unicode-segment"))]
    fn imp(_: &'static str) -> Result<hir::ClassUnicode> {
        Err(Error::PropertyNotFound)
    }

    #[cfg(feature = "unicode-segment")]
    fn imp(name: &'static str) -> Result<hir::ClassUnicode> {
        use unicode_tables::sentence_break::BY_NAME;
        property_set(BY_NAME, name)
            .map(hir_class)
            .ok_or(Error::PropertyValueNotFound)
    }

    imp(canonical_name)
}

/// Like symbolic_name_normalize_bytes, but operates on a string.
fn symbolic_name_normalize(x: &str) -> String {
    let mut tmp = x.as_bytes().to_vec();
    let len = symbolic_name_normalize_bytes(&mut tmp).len();
    tmp.truncate(len);
    // This should always succeed because `symbolic_name_normalize_bytes`
    // guarantees that `&tmp[..len]` is always valid UTF-8.
    //
    // N.B. We could avoid the additional UTF-8 check here, but it's unlikely
    // to be worth skipping the additional safety check. A benchmark must
    // justify it first.
    String::from_utf8(tmp).unwrap()
}

/// Normalize the given symbolic name in place according to UAX44-LM3.
///
/// A "symbolic name" typically corresponds to property names and property
/// value aliases. Note, though, that it should not be applied to property
/// string values.
///
/// The slice returned is guaranteed to be valid UTF-8 for all possible values
/// of `slice`.
///
/// See: http://unicode.org/reports/tr44/#UAX44-LM3
fn symbolic_name_normalize_bytes(slice: &mut [u8]) -> &mut [u8] {
    // I couldn't find a place in the standard that specified that property
    // names/aliases had a particular structure (unlike character names), but
    // we assume that it's ASCII only and drop anything that isn't ASCII.
    let mut start = 0;
    let mut starts_with_is = false;
    if slice.len() >= 2 {
        // Ignore any "is" prefix.
        starts_with_is = slice[0..2] == b"is"[..]
            || slice[0..2] == b"IS"[..]
            || slice[0..2] == b"iS"[..]
            || slice[0..2] == b"Is"[..];
        if starts_with_is {
            start = 2;
        }
    }
    let mut next_write = 0;
    for i in start..slice.len() {
        // VALIDITY ARGUMENT: To guarantee that the resulting slice is valid
        // UTF-8, we ensure that the slice contains only ASCII bytes. In
        // particular, we drop every non-ASCII byte from the normalized string.
        let b = slice[i];
        if b == b' ' || b == b'_' || b == b'-' {
            continue;
        } else if b'A' <= b && b <= b'Z' {
            slice[next_write] = b + (b'a' - b'A');
            next_write += 1;
        } else if b <= 0x7F {
            slice[next_write] = b;
            next_write += 1;
        }
    }
    // Special case: ISO_Comment has a 'isc' abbreviation. Since we generally
    // ignore 'is' prefixes, the 'isc' abbreviation gets caught in the cross
    // fire and ends up creating an alias for 'c' to 'ISO_Comment', but it
    // is actually an alias for the 'Other' general category.
    if starts_with_is && next_write == 1 && slice[0] == b'c' {
        slice[0] = b'i';
        slice[1] = b's';
        slice[2] = b'c';
        next_write = 3;
    }
    &mut slice[..next_write]
}

#[cfg(test)]
mod tests {
    use super::{
        contains_simple_case_mapping, simple_fold, symbolic_name_normalize,
        symbolic_name_normalize_bytes,
    };

    #[cfg(feature = "unicode-case")]
    fn simple_fold_ok(c: char) -> impl Iterator<Item = char> {
        simple_fold(c).unwrap().unwrap()
    }

    #[cfg(feature = "unicode-case")]
    fn simple_fold_err(c: char) -> Option<char> {
        match simple_fold(c).unwrap() {
            Ok(_) => unreachable!("simple_fold returned Ok iterator"),
            Err(next) => next,
        }
    }

    #[cfg(feature = "unicode-case")]
    fn contains_case_map(start: char, end: char) -> bool {
        contains_simple_case_mapping(start, end).unwrap()
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn simple_fold_k() {
        let xs: Vec<char> = simple_fold_ok('k').collect();
        assert_eq!(xs, vec!['K', 'K']);

        let xs: Vec<char> = simple_fold_ok('K').collect();
        assert_eq!(xs, vec!['k', 'K']);

        let xs: Vec<char> = simple_fold_ok('K').collect();
        assert_eq!(xs, vec!['K', 'k']);
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn simple_fold_a() {
        let xs: Vec<char> = simple_fold_ok('a').collect();
        assert_eq!(xs, vec!['A']);

        let xs: Vec<char> = simple_fold_ok('A').collect();
        assert_eq!(xs, vec!['a']);
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn simple_fold_empty() {
        assert_eq!(Some('A'), simple_fold_err('?'));
        assert_eq!(Some('A'), simple_fold_err('@'));
        assert_eq!(Some('a'), simple_fold_err('['));
        assert_eq!(Some('Ⰰ'), simple_fold_err('☃'));
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn simple_fold_max() {
        assert_eq!(None, simple_fold_err('\u{10FFFE}'));
        assert_eq!(None, simple_fold_err('\u{10FFFF}'));
    }

    #[test]
    #[cfg(not(feature = "unicode-case"))]
    fn simple_fold_disabled() {
        assert!(simple_fold('a').is_err());
    }

    #[test]
    #[cfg(feature = "unicode-case")]
    fn range_contains() {
        assert!(contains_case_map('A', 'A'));
        assert!(contains_case_map('Z', 'Z'));
        assert!(contains_case_map('A', 'Z'));
        assert!(contains_case_map('@', 'A'));
        assert!(contains_case_map('Z', '['));
        assert!(contains_case_map('☃', 'Ⰰ'));

        assert!(!contains_case_map('[', '['));
        assert!(!contains_case_map('[', '`'));

        assert!(!contains_case_map('☃', '☃'));
    }

    #[test]
    #[cfg(not(feature = "unicode-case"))]
    fn range_contains_disabled() {
        assert!(contains_simple_case_mapping('a', 'a').is_err());
    }

    #[test]
    #[cfg(feature = "unicode-gencat")]
    fn regression_466() {
        use super::{CanonicalClassQuery, ClassQuery};

        let q = ClassQuery::OneLetter('C');
        assert_eq!(
            q.canonicalize().unwrap(),
            CanonicalClassQuery::GeneralCategory("Other")
        );
    }

    #[test]
    fn sym_normalize() {
        let sym_norm = symbolic_name_normalize;

        assert_eq!(sym_norm("Line_Break"), "linebreak");
        assert_eq!(sym_norm("Line-break"), "linebreak");
        assert_eq!(sym_norm("linebreak"), "linebreak");
        assert_eq!(sym_norm("BA"), "ba");
        assert_eq!(sym_norm("ba"), "ba");
        assert_eq!(sym_norm("Greek"), "greek");
        assert_eq!(sym_norm("isGreek"), "greek");
        assert_eq!(sym_norm("IS_Greek"), "greek");
        assert_eq!(sym_norm("isc"), "isc");
        assert_eq!(sym_norm("is c"), "isc");
        assert_eq!(sym_norm("is_c"), "isc");
    }

    #[test]
    fn valid_utf8_symbolic() {
        let mut x = b"abc\xFFxyz".to_vec();
        let y = symbolic_name_normalize_bytes(&mut x);
        assert_eq!(y, b"abcxyz");
    }
}
#[cfg(test)]
mod tests_llm_16_616 {
    use super::*;

use crate::*;
    use crate::unicode::ClassQuery;

    #[test]
    fn test_canonical_binary() {
        let query = ClassQuery::Binary("cf");
        let result = query.canonical_binary("cf").unwrap();
        assert_eq!(result, CanonicalClassQuery::GeneralCategory("Format"));

        let query = ClassQuery::Binary("some_binary_property");
        let result = query.canonical_binary("some_binary_property").unwrap();
        assert_eq!(result, CanonicalClassQuery::Binary("some_binary_property"));
    }

    #[test]
    fn test_canonicalize() {
        let query = ClassQuery::OneLetter('c');
        let result = query.canonicalize().unwrap();
        assert_eq!(result, CanonicalClassQuery::Binary("c"));

        let query = ClassQuery::Binary("some_binary_property");
        let result = query.canonicalize().unwrap();
        assert_eq!(result, CanonicalClassQuery::Binary("some_binary_property"));

        let query = ClassQuery::ByValue {
            property_name: "General_Category",
            property_value: "Letter",
        };
        let result = query.canonicalize().unwrap();
        assert_eq!(result, CanonicalClassQuery::GeneralCategory("Letter"));

        let query = ClassQuery::ByValue {
            property_name: "Script",
            property_value: "Latin",
        };
        let result = query.canonicalize().unwrap();
        assert_eq!(result, CanonicalClassQuery::Script("Latin"));

        let query = ClassQuery::ByValue {
            property_name: "some_property",
            property_value: "some_value",
        };
        let result = query.canonicalize().unwrap();
        assert_eq!(
            result,
            CanonicalClassQuery::ByValue {
                property_name: "some_property",
                property_value: "some_value",
            }
        );
    }
}#[cfg(test)]
mod tests_llm_16_636_llm_16_635 {
    use super::*;

use crate::*;
    use crate::unicode::PropertyValues;

    #[cfg(test)]
    fn test_canonical_value() {
        let property_values: &'static [(&'static str, &'static str)] = &[
            ("value1", "normalized_value1"),
            ("value2", "normalized_value2"),
            // Add more property values as necessary
        ];
        assert_eq!(
            canonical_value(property_values, "normalized_value1"),
            Some("value1")
        );
        // Add more test cases as necessary
    }
}#[cfg(test)]
mod tests_llm_16_687 {
    use super::*;

use crate::*;
    
    #[test]
    #[cfg(not(feature = "unicode-case"))]
    fn test_simple_fold() {
        let result = simple_fold('a');
        assert!(matches!(result, Err(_)));
    }
    
    #[test]
    #[cfg(feature = "unicode-case")]
    fn test_simple_fold() {
        let result = simple_fold('a');
        assert!(matches!(result, Ok(Ok(_))));
    }
}#[cfg(test)]
mod tests_llm_16_690 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_symbolic_name_normalize() {
        let input = "test_input";
        let output = symbolic_name_normalize(input);
        let expected = "test_input";
        assert_eq!(output, expected);
    }
}#[cfg(test)]
mod tests_llm_16_692 {
    use crate::unicode::symbolic_name_normalize_bytes;

    #[test]
    fn test_symbolic_name_normalize_bytes() {
        let mut input: &mut [u8] = &mut [b'I', b's', b'A', b'l', b'n', b'u', b'm', b'\0'];
        let output: &[u8] = b"isalnum";
        assert_eq!(symbolic_name_normalize_bytes(&mut input), output);

        let mut input: &mut [u8] = &mut [0, 0, 0, 0];
        let output: &[u8] = &[];
        assert_eq!(symbolic_name_normalize_bytes(&mut input), output);

        let mut input: &mut [u8] = &mut [b'P', b'r', b'o', b'p', b'_', b'N', b'F', b'C'];
        let output: &[u8] = b"prop_nfc";
        assert_eq!(symbolic_name_normalize_bytes(&mut input), output);

        let mut input: &mut [u8] = &mut [0, 0, 0, b'D', b'U', b'M', b'M', b'Y', b'\0'];
        let output: &[u8] = b"dummy";
        assert_eq!(symbolic_name_normalize_bytes(&mut input), output);
    }
}#[cfg(test)]
mod tests_rug_536 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0: char = 'a';
        
        crate::unicode::simple_fold::imp(p0);
    }
}        
#[cfg(test)]
mod tests_rug_537 {
    use super::*;
    
    #[test]
    fn test_contains_simple_case_mapping() {
        let p0: char = 'A';
        let p1: char = 'Z';
        
        crate::unicode::contains_simple_case_mapping(p0, p1);
    }
}   
#[cfg(test)]
mod tests_rug_538 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0 = 'A';
        let mut p1 = 'Z';

        crate::unicode::contains_simple_case_mapping::imp(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_539 {
    use super::*;
    use crate::unicode::{ClassQuery, CanonicalClassQuery, Error, hir};

    #[test]
    fn test_rug() {
        let mut p0: ClassQuery<'_> = ClassQuery::new();
        p0.bytes = Some(&[]);

        let result = crate::unicode::class(p0);

        // Add assertions here
    }
}     
#[cfg(test)]
mod tests_rug_540 {
    use super::*;
    use crate::regex_syntax::hir::ClassUnicode;
    use crate::regex_syntax::hir_class;
    use crate::regex_syntax::unicode_tables::perl_word;
    use crate::regex_syntax::Error;
    use crate::regex_syntax::imp;

    #[test]
    fn test_rug() {
        let result: Result<ClassUnicode> = perl_word();
        assert_eq!(result, Err(Error::PerlClassNotFound));
    }
}
                            #[cfg(test)]
mod tests_rug_541 {
    use super::*;
    use crate::unicode::perl_word::imp;
    use crate::unicode_tables::perl_word::PERL_WORD;
    use crate::hir::ClassUnicode;
    use crate::hir_class;

    #[test]
    fn test_rug() {
        let result: Result<ClassUnicode, ()> = imp();
        assert_eq!(result, Ok(hir_class(PERL_WORD)));
    }
}#[cfg(test)]
mod tests_rug_542 {
    use super::*;
    use crate::Error;
    use crate::hir;
    use crate::Provider;
    use crate::hir::*;
    use std::result::Result;

    #[test]
    fn test_rug() {
        let result: Result<hir::ClassUnicode, Error> = perl_space();

        assert!(result.is_ok());
        let class: hir::ClassUnicode = result.unwrap();

        // Add your assertions here

        // For example, to assert that the class should contain a certain codepoint:
        // assert!(class.contains_codepoint(0x1234))
    }
}#[cfg(test)]
mod tests_rug_543 {
    use super::*;
    use crate::unicode::perl_space::imp;
    
    #[test]
    fn test_imp() {
        let result: Result<hir::ClassUnicode, RegexError> = imp();
        assert!(result.is_ok());
    }
}        
#[cfg(test)]
mod tests_rug_544 {
    use super::*;
    use crate::error::Error;
    use crate::hir::ClassUnicode;
    use crate::hir_class;
    #[cfg(feature = "unicode-gencat")]
    use crate::unicode_tables::general_category::DECIMAL_NUMBER;
    #[cfg(all(feature = "unicode-perl", not(feature = "unicode-gencat")))]
    use crate::unicode_tables::perl_decimal::DECIMAL_NUMBER;

    #[test]
    fn test_perl_digit() {
        let result = perl_digit();
        match result {
            Ok(class) => {
                // handle success case
                // add your assertions here
            }
            Err(error) => {
                assert_eq!(error, Error::PerlClassNotFound);
            }
        }
    }
}
#[cfg(test)]
mod tests_rug_545 {
    use super::*;
    use crate::unicode::perl_digit::imp;
    use crate::unicode_tables::general_category::DECIMAL_NUMBER;
    use crate::hir::{hir_class, ClassUnicode};

    #[test]
    fn test_rug() {
        let res: Result<ClassUnicode> = imp();
        assert_eq!(res, Ok(hir_class(DECIMAL_NUMBER)));
    }
}
#[cfg(test)]
mod tests_rug_546 {
    use super::*;
    use crate::hir;

    #[test]
    fn test_hir_class() {
        let p0: &[(char, char)] = &[
            ('a', 'z'),
            ('0', '9'),
            ('A', 'Z'),
        ];

        crate::unicode::hir_class(p0);
    }
}#[cfg(test)]
mod tests_rug_547 {
    use super::*;
    use crate::unicode::UnicodeWordError;

    #[test]
    fn test_is_word_character() {
        let p0: char = 'a';

        crate::regex_syntax::unicode::is_word_character(p0);
    }
}#[cfg(test)]
mod tests_rug_548 {
    use super::*;
    use crate::unicode::is_word_byte;
    use crate::unicode_tables::perl_word::PERL_WORD;

    #[test]
    fn test_rug() {
        let mut p0: char = 'A';

        assert_eq!(imp(p0), Ok(true));
    }
}
#[cfg(test)]
mod tests_rug_549 {
    use super::*;

    #[test]
    fn test_rug() {
        let mut p0: &str = "any";
        assert_eq!(crate::unicode::canonical_gencat(&p0).unwrap(), Some("Any"));

        p0 = "assigned";
        assert_eq!(crate::unicode::canonical_gencat(&p0).unwrap(), Some("Assigned"));

        p0 = "ascii";
        assert_eq!(crate::unicode::canonical_gencat(&p0).unwrap(), Some("ASCII"));

        p0 = "category";
        assert_eq!(crate::unicode::canonical_gencat(&p0).unwrap(), Some("Category"));

        p0 = "letter";
        assert_eq!(crate::unicode::canonical_gencat(&p0).unwrap(), Some("Letter"));

        p0 = "number";
        assert_eq!(crate::unicode::canonical_gencat(&p0).unwrap(), Some("Number"));

        p0 = "other";
        assert_eq!(crate::unicode::canonical_gencat(&p0).unwrap(), Some("Other"));

    }
}#[cfg(test)]
mod tests_rug_550 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0: &str = "L";

        crate::unicode::canonical_script(&p0).unwrap();

    }
}
#[cfg(test)]
mod tests_rug_551 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let p0: &str = "some_property";
        
        crate::unicode::canonical_prop(&p0);
    }
}
   
    #[cfg(test)]
    mod tests_rug_552 {
        use super::*;
        
        #[test]
        fn test_rug() {
            let mut p0 = "Script";

            crate::unicode::canonical_prop::imp(&p0);

        }
    }
                        
#[cfg(test)]
mod tests_rug_553 {
    use super::*;
    use crate::regex_syntax::unicode::PropertyValues;
    use crate::regex_syntax::Error;
    
    #[test]
    fn test_property_values() {
        let canonical_property_name = "General_Category";
        
        let result = property_values(&canonical_property_name);
        
        assert_eq!(result, Err(Error::PropertyValueNotFound));
    }
}#[cfg(test)]
mod tests_rug_554 {
    use super::*;

    #[test]
    fn test_rug() {
        let mut p0: &'static str = "Script";
        crate::regex_syntax::unicode::property_values::imp(p0);
    }
}#[cfg(test)]
mod tests_rug_555 {
    use super::*;
    use crate::unicode::UNICODE_VERSION;

    #[test]
    fn test_property_set() {
        let p0: &[(&'static str, &'static [(char, char)])] = &UNICODE_VERSION;
        let p1: &str = "Canonical";

        crate::unicode::property_set(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_556 {
    use super::*;

    #[test]
    fn test_rug() {
        let p0 = "V12_0";

        crate::unicode::ages(&p0).unwrap();
    }
}#[cfg(test)]
mod tests_rug_557 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0: &str = "V3_0";
                
        crate::unicode::ages::imp(p0);
    }
}#[cfg(test)]
mod tests_rug_558 {
    use super::*;
    use crate::hir::ClassUnicode;
    use crate::Error;

    #[test]
    fn test_gencat() {
        let mut p0: &'static str = "Decimal_Number";

        gencat(p0);
    }
}#[cfg(test)]
    mod tests_rug_559 {
        use super::*;
        
        #[test]
        fn test_imp() {
            let mut p0: &'static str = "Any";
                
            crate::unicode::gencat::imp(&p0);

        }
    }#[cfg(test)]
mod tests_rug_560 {
    use super::*;

    #[test]
    fn test_script() {
        let p0: &'static str = "Latin";

        script(p0);
    }
}#[cfg(test)]
mod tests_rug_561 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let p0: &'static str = "Latin";
        
        imp(p0).unwrap();
    }
}#[cfg(test)]
mod tests_rug_562 {
    use super::*;
    
    #[test]
    fn test_script_extension() {
        let p0: &'static str = "Latin";
        
        crate::unicode::script_extension(&p0);

    }
}
#[cfg(test)]
mod tests_rug_563 {
    use super::*;
    use crate::unicode::script_extension::imp;
    use crate::hir::ClassUnicode;
    use crate::Error;

    #[test]
    fn test_imp() {
        let name: &'static str = "sample_name";
        let result: Result<ClassUnicode, Error> = imp(name);
        assert_eq!(result.is_ok(), true);
    }
}
#[cfg(test)]
mod tests_rug_564 {
    use super::*;
    use crate::hir::{ClassUnicode, ClassUnicodeRange};
    use crate::Error;
    
    #[test]
    fn test_regex_syntax() {
        let mut p0: &'static str = "White_Space";

        bool_property(p0);
    }
}
#[cfg(test)]
mod tests_rug_565 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0 = "some_property_name";
        
        unicode::bool_property::imp(&p0);
    }
}
#[cfg(test)]
mod tests_rug_566 {
    use super::*;
    
    #[test]
    fn test_gcb() {
        let p0: &str = "Sample grapheme cluster break property name";

        crate::unicode::gcb(&p0);
    }
}#[cfg(test)]
mod tests_rug_567 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let p0: &str = "SomeSampleName";
        
        crate::unicode::gcb::imp(&p0);
    }
}#[cfg(test)]
mod tests_rug_568 {
    use super::unicode::wb;
    
    #[test]
    fn test_wb() {
        let p0: &str = "Alphabetic";
        
        wb(p0).unwrap();
    }
}#[cfg(test)]
mod tests_rug_569 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0: &'static str = "Name"; 
               
        crate::unicode::wb::imp(&p0);
    }
}#[cfg(test)]
mod tests_rug_570 {
    use super::*;

    #[test]
    fn test_sb() {
        let p0: &str = "Alphabetic";

        crate::unicode::sb(p0);
    }
}
#[cfg(test)]
mod tests_rug_571 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0: &'static str = "...";
        crate::unicode::sb::imp(&p0);
        
    }
}#[cfg(test)]
mod tests_rug_572 {
    use super::*;
    use crate::unicode::ClassQuery;

    #[test]
    fn test_rug() {
        let mut p0: ClassQuery<'static> = ClassQuery::default();

        <unicode::ClassQuery<'static>>::canonicalize(&mut p0).unwrap();
    }
}