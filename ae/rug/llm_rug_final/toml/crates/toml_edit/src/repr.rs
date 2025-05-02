use std::borrow::Cow;

use crate::RawString;

/// A value together with its `to_string` representation,
/// including surrounding it whitespaces and comments.
#[derive(Eq, PartialEq, Clone, Hash)]
pub struct Formatted<T> {
    value: T,
    repr: Option<Repr>,
    decor: Decor,
}

impl<T> Formatted<T>
where
    T: ValueRepr,
{
    /// Default-formatted value
    pub fn new(value: T) -> Self {
        Self {
            value,
            repr: None,
            decor: Default::default(),
        }
    }

    pub(crate) fn set_repr_unchecked(&mut self, repr: Repr) {
        self.repr = Some(repr);
    }

    /// The wrapped value
    pub fn value(&self) -> &T {
        &self.value
    }

    /// The wrapped value
    pub fn into_value(self) -> T {
        self.value
    }

    /// Returns the raw representation, if available.
    pub fn as_repr(&self) -> Option<&Repr> {
        self.repr.as_ref()
    }

    /// Returns the default raw representation.
    pub fn default_repr(&self) -> Repr {
        self.value.to_repr()
    }

    /// Returns a raw representation.
    pub fn display_repr(&self) -> Cow<str> {
        self.as_repr()
            .and_then(|r| r.as_raw().as_str())
            .map(Cow::Borrowed)
            .unwrap_or_else(|| {
                Cow::Owned(self.default_repr().as_raw().as_str().unwrap().to_owned())
            })
    }

    /// Returns the location within the original document
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        self.repr.as_ref().and_then(|r| r.span())
    }

    pub(crate) fn despan(&mut self, input: &str) {
        self.decor.despan(input);
        if let Some(repr) = &mut self.repr {
            repr.despan(input);
        }
    }

    /// Returns the surrounding whitespace
    pub fn decor_mut(&mut self) -> &mut Decor {
        &mut self.decor
    }

    /// Returns the surrounding whitespace
    pub fn decor(&self) -> &Decor {
        &self.decor
    }

    /// Auto formats the value.
    pub fn fmt(&mut self) {
        self.repr = Some(self.value.to_repr());
    }
}

impl<T> std::fmt::Debug for Formatted<T>
where
    T: std::fmt::Debug,
{
    #[inline]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut d = formatter.debug_struct("Formatted");
        d.field("value", &self.value);
        match &self.repr {
            Some(r) => d.field("repr", r),
            None => d.field("repr", &"default"),
        };
        d.field("decor", &self.decor);
        d.finish()
    }
}

impl<T> std::fmt::Display for Formatted<T>
where
    T: ValueRepr,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::encode::Encode::encode(self, f, None, ("", ""))
    }
}

pub trait ValueRepr: crate::private::Sealed {
    /// The TOML representation of the value
    fn to_repr(&self) -> Repr;
}

/// TOML-encoded value
#[derive(Eq, PartialEq, Clone, Hash)]
pub struct Repr {
    raw_value: RawString,
}

impl Repr {
    pub(crate) fn new_unchecked(raw: impl Into<RawString>) -> Self {
        Repr {
            raw_value: raw.into(),
        }
    }

    /// Access the underlying value
    pub fn as_raw(&self) -> &RawString {
        &self.raw_value
    }

    /// Returns the location within the original document
    pub(crate) fn span(&self) -> Option<std::ops::Range<usize>> {
        self.raw_value.span()
    }

    pub(crate) fn despan(&mut self, input: &str) {
        self.raw_value.despan(input)
    }

    pub(crate) fn encode(&self, buf: &mut dyn std::fmt::Write, input: &str) -> std::fmt::Result {
        self.as_raw().encode(buf, input)
    }
}

impl std::fmt::Debug for Repr {
    #[inline]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.raw_value.fmt(formatter)
    }
}

/// A prefix and suffix,
///
/// Including comments, whitespaces and newlines.
#[derive(Eq, PartialEq, Clone, Default, Hash)]
pub struct Decor {
    prefix: Option<RawString>,
    suffix: Option<RawString>,
}

impl Decor {
    /// Creates a new decor from the given prefix and suffix.
    pub fn new(prefix: impl Into<RawString>, suffix: impl Into<RawString>) -> Self {
        Self {
            prefix: Some(prefix.into()),
            suffix: Some(suffix.into()),
        }
    }

    /// Go back to default decor
    pub fn clear(&mut self) {
        self.prefix = None;
        self.suffix = None;
    }

    /// Get the prefix.
    pub fn prefix(&self) -> Option<&RawString> {
        self.prefix.as_ref()
    }

    pub(crate) fn prefix_encode(
        &self,
        buf: &mut dyn std::fmt::Write,
        input: Option<&str>,
        default: &str,
    ) -> std::fmt::Result {
        if let Some(prefix) = self.prefix() {
            prefix.encode_with_default(buf, input, default)
        } else {
            write!(buf, "{}", default)
        }
    }

    /// Set the prefix.
    pub fn set_prefix(&mut self, prefix: impl Into<RawString>) {
        self.prefix = Some(prefix.into());
    }

    /// Get the suffix.
    pub fn suffix(&self) -> Option<&RawString> {
        self.suffix.as_ref()
    }

    pub(crate) fn suffix_encode(
        &self,
        buf: &mut dyn std::fmt::Write,
        input: Option<&str>,
        default: &str,
    ) -> std::fmt::Result {
        if let Some(suffix) = self.suffix() {
            suffix.encode_with_default(buf, input, default)
        } else {
            write!(buf, "{}", default)
        }
    }

    /// Set the suffix.
    pub fn set_suffix(&mut self, suffix: impl Into<RawString>) {
        self.suffix = Some(suffix.into());
    }

    pub(crate) fn despan(&mut self, input: &str) {
        if let Some(prefix) = &mut self.prefix {
            prefix.despan(input);
        }
        if let Some(suffix) = &mut self.suffix {
            suffix.despan(input);
        }
    }
}

impl std::fmt::Debug for Decor {
    #[inline]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut d = formatter.debug_struct("Decor");
        match &self.prefix {
            Some(r) => d.field("prefix", r),
            None => d.field("prefix", &"default"),
        };
        match &self.suffix {
            Some(r) => d.field("suffix", r),
            None => d.field("suffix", &"default"),
        };
        d.finish()
    }
}

#[cfg(test)]
mod tests_rug_872 {
    use super::*;
    use crate::repr::Repr;
    use crate::RawString;
    
    #[test]
    fn test_rug() {
        let mut p0: RawString = "test_value".into();
        
        Repr::new_unchecked(p0);

    }
}

#[cfg(test)]
mod tests_rug_877 {
    use super::*;
    use crate::repr::{Decor, RawString};
    
    #[test]
    fn test_rug() {
        let p0: RawString = "sample_prefix".into();
        let p1: RawString = "sample_suffix".into();

        Decor::new(p0, p1);
    }
}

#[cfg(test)]
mod tests_rug_879 {
    use super::*;
    use crate::repr; // Import the necessary module for `repr::Decor`
    
    #[test]
    fn test_rug() {
        let mut p0 = repr::Decor::default(); // Construct the `Decor` variable using the default constructor function

        repr::Decor::prefix(&p0); // Call the `prefix` method on `Decor`
    }
}
                    
#[cfg(test)]
mod tests_rug_880 {
    use super::*;
    use crate::repr;

    #[test]
    fn test_rug() {
        let mut p0 = repr::Decor::default();
        let mut p1: &mut dyn std::fmt::Write = &mut String::new();
        let mut p2: Option<&str> = Some("input_str");
        let mut p3: &str = "default_str";

        repr::Decor::prefix_encode(&mut p0, p1, p2, &p3).unwrap();
    }
}
#[cfg(test)]
mod tests_rug_881 {
    use super::*;
    use crate::repr::{Decor, RawString};
    
    #[test]
    fn test_rug() {
        let mut p0 = Decor::default();
        let mut p1: RawString = "example".into();
        
        Decor::set_prefix(&mut p0, p1);
    }
}#[cfg(test)]
mod tests_rug_882 {
    use super::*;
    use crate::repr::Decor;
    
    #[test]
    fn test_suffix() {
        let p0 = Decor::default();
        
        p0.suffix();
    }
}#[cfg(test)]
mod tests_rug_883 {
    use super::*;
    use crate::repr::Decor;
    use std::fmt::Write;

    #[test]
    fn test_suffix_encode() {
        let p0 = Decor::default();
        let p1 = &mut String::new();
        let p2 = None;
        let p3 = "default value";

        p0.suffix_encode(p1, p2, &p3).unwrap();
    }
}#[cfg(test)]
mod tests_rug_884 {
    use super::*;
    use crate::repr::{Decor, RawString};

    #[test]
    fn test_rug() {
        let mut p0 = Decor::default();
        let mut p1: RawString = "suffix".into();

        p0.set_suffix(p1);
    }
}
#[cfg(test)]
mod tests_rug_885 {
    use super::*;
    use crate::repr::Decor;

    #[test]
    fn test_rug() {
        let mut p0 = Decor::default();
        let p1 = "sample_input";

        p0.despan(p1);
    }
}
