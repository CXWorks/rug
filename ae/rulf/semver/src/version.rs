//! The `version` module gives you tools to create and compare SemVer-compliant
//! versions.
use std::cmp::{self, Ordering};
use std::error::Error;
use std::fmt;
use std::hash;
use std::result;
use std::str;
use semver_parser;
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor};
#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer};
/// An identifier in the pre-release or build metadata.
///
/// See sections 9 and 10 of the spec for more about pre-release identifers and
/// build metadata.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Identifier {
    /// An identifier that's solely numbers.
    Numeric(u64),
    /// An identifier with letters and numbers.
    AlphaNumeric(String),
}
impl From<semver_parser::version::Identifier> for Identifier {
    fn from(other: semver_parser::version::Identifier) -> Identifier {
        match other {
            semver_parser::version::Identifier::Numeric(n) => Identifier::Numeric(n),
            semver_parser::version::Identifier::AlphaNumeric(s) => {
                Identifier::AlphaNumeric(s)
            }
        }
    }
}
impl fmt::Display for Identifier {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Identifier::Numeric(ref n) => fmt::Display::fmt(n, f),
            Identifier::AlphaNumeric(ref s) => fmt::Display::fmt(s, f),
        }
    }
}
#[cfg(feature = "serde")]
impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Identifier::Numeric(n) => serializer.serialize_u64(n),
            Identifier::AlphaNumeric(ref s) => serializer.serialize_str(s),
        }
    }
}
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IdentifierVisitor;
        impl<'de> Visitor<'de> for IdentifierVisitor {
            type Value = Identifier;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a SemVer pre-release or build identifier")
            }
            fn visit_u64<E>(self, numeric: u64) -> result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Identifier::Numeric(numeric))
            }
            fn visit_str<E>(self, alphanumeric: &str) -> result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Identifier::AlphaNumeric(alphanumeric.to_owned()))
            }
        }
        deserializer.deserialize_any(IdentifierVisitor)
    }
}
/// Represents a version number conforming to the semantic versioning scheme.
#[derive(Clone, Eq, Debug)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", sql_type = "diesel::sql_types::Text")]
pub struct Version {
    /// The major version, to be incremented on incompatible changes.
    pub major: u64,
    /// The minor version, to be incremented when functionality is added in a
    /// backwards-compatible manner.
    pub minor: u64,
    /// The patch version, to be incremented when backwards-compatible bug
    /// fixes are made.
    pub patch: u64,
    /// The pre-release version identifier, if one exists.
    pub pre: Vec<Identifier>,
    /// The build metadata, ignored when determining version precedence.
    pub build: Vec<Identifier>,
}
impl From<semver_parser::version::Version> for Version {
    fn from(other: semver_parser::version::Version) -> Version {
        Version {
            major: other.major,
            minor: other.minor,
            patch: other.patch,
            pre: other.pre.into_iter().map(From::from).collect(),
            build: other.build.into_iter().map(From::from).collect(),
        }
    }
}
#[cfg(feature = "serde")]
impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VersionVisitor;
        impl<'de> Visitor<'de> for VersionVisitor {
            type Value = Version;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a SemVer version as a string")
            }
            fn visit_str<E>(self, v: &str) -> result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Version::parse(v).map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_str(VersionVisitor)
    }
}
/// An error type for this crate
///
/// Currently, just a generic error. Will make this nicer later.
#[derive(Clone, PartialEq, Debug, PartialOrd)]
pub enum SemVerError {
    /// An error ocurred while parsing.
    ParseError(String),
}
impl fmt::Display for SemVerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemVerError::ParseError(ref m) => write!(f, "{}", m),
        }
    }
}
impl Error for SemVerError {}
/// A Result type for errors
pub type Result<T> = result::Result<T, SemVerError>;
impl Version {
    /// Contructs the simple case without pre or build.
    pub fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
            pre: Vec::new(),
            build: Vec::new(),
        }
    }
    /// Parse a string into a semver object.
    ///
    /// # Errors
    ///
    /// Returns an error variant if the input could not be parsed as a semver object.
    ///
    /// In general, this means that the provided string does not conform to the
    /// [semver spec][semver].
    ///
    /// An error for overflow is returned if any numeric component is larger than what can be
    /// stored in `u64`.
    ///
    /// The following are examples for other common error causes:
    ///
    /// * `1.0` - too few numeric components are used. Exactly 3 are expected.
    /// * `1.0.01` - a numeric component has a leading zero.
    /// * `1.0.foo` - uses a non-numeric components where one is expected.
    /// * `1.0.0foo` - metadata is not separated using a legal character like, `+` or `-`.
    /// * `1.0.0+foo_123` - contains metadata with an illegal character (`_`).
    ///   Legal characters for metadata include `a-z`, `A-Z`, `0-9`, `-`, and `.` (dot).
    ///
    /// [semver]: https://semver.org
    pub fn parse(version: &str) -> Result<Version> {
        let res = semver_parser::version::parse(version);
        match res {
            Err(e) => Err(SemVerError::ParseError(e.to_string())),
            Ok(v) => Ok(From::from(v)),
        }
    }
    /// Clears the build metadata
    fn clear_metadata(&mut self) {
        self.build = Vec::new();
        self.pre = Vec::new();
    }
    /// Increments the patch number for this Version (Must be mutable)
    pub fn increment_patch(&mut self) {
        self.patch += 1;
        self.clear_metadata();
    }
    /// Increments the minor version number for this Version (Must be mutable)
    ///
    /// As instructed by section 7 of the spec, the patch number is reset to 0.
    pub fn increment_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
        self.clear_metadata();
    }
    /// Increments the major version number for this Version (Must be mutable)
    ///
    /// As instructed by section 8 of the spec, the minor and patch numbers are
    /// reset to 0
    pub fn increment_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
        self.clear_metadata();
    }
    /// Checks to see if the current Version is in pre-release status
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}
impl str::FromStr for Version {
    type Err = SemVerError;
    fn from_str(s: &str) -> Result<Version> {
        Version::parse(s)
    }
}
impl fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if !self.pre.is_empty() {
            result.push_str("-");
            for (i, x) in self.pre.iter().enumerate() {
                if i != 0 {
                    result.push_str(".");
                }
                result.push_str(format!("{}", x).as_ref());
            }
        }
        if !self.build.is_empty() {
            result.push_str("+");
            for (i, x) in self.build.iter().enumerate() {
                if i != 0 {
                    result.push_str(".");
                }
                result.push_str(format!("{}", x).as_ref());
            }
        }
        f.pad(result.as_ref())?;
        Ok(())
    }
}
impl cmp::PartialEq for Version {
    #[inline]
    fn eq(&self, other: &Version) -> bool {
        self.major == other.major && self.minor == other.minor
            && self.patch == other.patch && self.pre == other.pre
    }
}
impl cmp::PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl cmp::Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            r => return r,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            r => return r,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            r => return r,
        }
        match (self.pre.len(), other.pre.len()) {
            (0, 0) => Ordering::Equal,
            (0, _) => Ordering::Greater,
            (_, 0) => Ordering::Less,
            (_, _) => self.pre.cmp(&other.pre),
        }
    }
}
impl hash::Hash for Version {
    fn hash<H: hash::Hasher>(&self, into: &mut H) {
        self.major.hash(into);
        self.minor.hash(into);
        self.patch.hash(into);
        self.pre.hash(into);
    }
}
impl From<(u64, u64, u64)> for Version {
    fn from(tuple: (u64, u64, u64)) -> Version {
        let (major, minor, patch) = tuple;
        Version::new(major, minor, patch)
    }
}
#[cfg(test)]
mod tests {
    use super::Identifier;
    use super::SemVerError;
    use super::Version;
    use std::result;
    #[test]
    fn test_parse() {
        fn parse_error(e: &str) -> result::Result<Version, SemVerError> {
            return Err(SemVerError::ParseError(e.to_string()));
        }
        assert_eq!(Version::parse(""), parse_error("expected more input"));
        assert_eq!(Version::parse("  "), parse_error("expected more input"));
        assert_eq!(Version::parse("1"), parse_error("expected more input"));
        assert_eq!(Version::parse("1.2"), parse_error("expected more input"));
        assert_eq!(Version::parse("1.2.3-"), parse_error("expected more input"));
        assert_eq!(
            Version::parse("a.b.c"),
            parse_error("encountered unexpected token: AlphaNumeric(\"a\")")
        );
        assert_eq!(
            Version::parse("1.2.3 abc"),
            parse_error("expected end of input, but got: [AlphaNumeric(\"abc\")]")
        );
        assert_eq!(
            Version::parse("1.2.3"), Ok(Version { major : 1, minor : 2, patch : 3, pre :
            Vec::new(), build : Vec::new(), })
        );
        assert_eq!(Version::parse("1.2.3"), Ok(Version::new(1, 2, 3)));
        assert_eq!(
            Version::parse("  1.2.3  "), Ok(Version { major : 1, minor : 2, patch : 3,
            pre : Vec::new(), build : Vec::new(), })
        );
        assert_eq!(
            Version::parse("1.2.3-alpha1"), Ok(Version { major : 1, minor : 2, patch : 3,
            pre : vec![Identifier::AlphaNumeric(String::from("alpha1"))], build :
            Vec::new(), })
        );
        assert_eq!(
            Version::parse("  1.2.3-alpha1  "), Ok(Version { major : 1, minor : 2, patch
            : 3, pre : vec![Identifier::AlphaNumeric(String::from("alpha1"))], build :
            Vec::new(), })
        );
        assert_eq!(
            Version::parse("1.2.3+build5"), Ok(Version { major : 1, minor : 2, patch : 3,
            pre : Vec::new(), build :
            vec![Identifier::AlphaNumeric(String::from("build5"))], })
        );
        assert_eq!(
            Version::parse("  1.2.3+build5  "), Ok(Version { major : 1, minor : 2, patch
            : 3, pre : Vec::new(), build :
            vec![Identifier::AlphaNumeric(String::from("build5"))], })
        );
        assert_eq!(
            Version::parse("1.2.3-alpha1+build5"), Ok(Version { major : 1, minor : 2,
            patch : 3, pre : vec![Identifier::AlphaNumeric(String::from("alpha1"))],
            build : vec![Identifier::AlphaNumeric(String::from("build5"))], })
        );
        assert_eq!(
            Version::parse("  1.2.3-alpha1+build5  "), Ok(Version { major : 1, minor : 2,
            patch : 3, pre : vec![Identifier::AlphaNumeric(String::from("alpha1"))],
            build : vec![Identifier::AlphaNumeric(String::from("build5"))], })
        );
        assert_eq!(
            Version::parse("1.2.3-1.alpha1.9+build5.7.3aedf  "), Ok(Version { major : 1,
            minor : 2, patch : 3, pre : vec![Identifier::Numeric(1),
            Identifier::AlphaNumeric(String::from("alpha1")), Identifier::Numeric(9),],
            build : vec![Identifier::AlphaNumeric(String::from("build5")),
            Identifier::Numeric(7), Identifier::AlphaNumeric(String::from("3aedf")),], })
        );
        assert_eq!(
            Version::parse("0.4.0-beta.1+0851523"), Ok(Version { major : 0, minor : 4,
            patch : 0, pre : vec![Identifier::AlphaNumeric(String::from("beta")),
            Identifier::Numeric(1),], build :
            vec![Identifier::AlphaNumeric(String::from("0851523"))], })
        );
        assert_eq!(
            Version::parse("1.1.0-beta-10"), Ok(Version { major : 1, minor : 1, patch :
            0, pre : vec![Identifier::AlphaNumeric(String::from("beta-10")),], build :
            Vec::new(), })
        );
    }
    #[test]
    fn test_increment_patch() {
        let mut buggy_release = Version::parse("0.1.0").unwrap();
        buggy_release.increment_patch();
        assert_eq!(buggy_release, Version::parse("0.1.1").unwrap());
    }
    #[test]
    fn test_increment_minor() {
        let mut feature_release = Version::parse("1.4.6").unwrap();
        feature_release.increment_minor();
        assert_eq!(feature_release, Version::parse("1.5.0").unwrap());
    }
    #[test]
    fn test_increment_major() {
        let mut chrome_release = Version::parse("46.1.246773").unwrap();
        chrome_release.increment_major();
        assert_eq!(chrome_release, Version::parse("47.0.0").unwrap());
    }
    #[test]
    fn test_increment_keep_prerelease() {
        let mut release = Version::parse("1.0.0-alpha").unwrap();
        release.increment_patch();
        assert_eq!(release, Version::parse("1.0.1").unwrap());
        release.increment_minor();
        assert_eq!(release, Version::parse("1.1.0").unwrap());
        release.increment_major();
        assert_eq!(release, Version::parse("2.0.0").unwrap());
    }
    #[test]
    fn test_increment_clear_metadata() {
        let mut release = Version::parse("1.0.0+4442").unwrap();
        release.increment_patch();
        assert_eq!(release, Version::parse("1.0.1").unwrap());
        release = Version::parse("1.0.1+hello").unwrap();
        release.increment_minor();
        assert_eq!(release, Version::parse("1.1.0").unwrap());
        release = Version::parse("1.1.3747+hello").unwrap();
        release.increment_major();
        assert_eq!(release, Version::parse("2.0.0").unwrap());
    }
    #[test]
    fn test_eq() {
        assert_eq!(Version::parse("1.2.3"), Version::parse("1.2.3"));
        assert_eq!(Version::parse("1.2.3-alpha1"), Version::parse("1.2.3-alpha1"));
        assert_eq!(Version::parse("1.2.3+build.42"), Version::parse("1.2.3+build.42"));
        assert_eq!(Version::parse("1.2.3-alpha1+42"), Version::parse("1.2.3-alpha1+42"));
        assert_eq!(Version::parse("1.2.3+23"), Version::parse("1.2.3+42"));
    }
    #[test]
    fn test_ne() {
        assert!(Version::parse("0.0.0") != Version::parse("0.0.1"));
        assert!(Version::parse("0.0.0") != Version::parse("0.1.0"));
        assert!(Version::parse("0.0.0") != Version::parse("1.0.0"));
        assert!(Version::parse("1.2.3-alpha") != Version::parse("1.2.3-beta"));
    }
    #[test]
    fn test_show() {
        assert_eq!(format!("{}", Version::parse("1.2.3").unwrap()), "1.2.3".to_string());
        assert_eq!(
            format!("{}", Version::parse("1.2.3-alpha1").unwrap()), "1.2.3-alpha1"
            .to_string()
        );
        assert_eq!(
            format!("{}", Version::parse("1.2.3+build.42").unwrap()), "1.2.3+build.42"
            .to_string()
        );
        assert_eq!(
            format!("{}", Version::parse("1.2.3-alpha1+42").unwrap()), "1.2.3-alpha1+42"
            .to_string()
        );
    }
    #[test]
    fn test_display() {
        let version = Version::parse("1.2.3-rc1").unwrap();
        assert_eq!(format!("{:20}", version), "1.2.3-rc1           ");
        assert_eq!(format!("{:*^20}", version), "*****1.2.3-rc1******");
        assert_eq!(format!("{:.4}", version), "1.2.");
    }
    #[test]
    fn test_to_string() {
        assert_eq!(Version::parse("1.2.3").unwrap().to_string(), "1.2.3".to_string());
        assert_eq!(
            Version::parse("1.2.3-alpha1").unwrap().to_string(), "1.2.3-alpha1"
            .to_string()
        );
        assert_eq!(
            Version::parse("1.2.3+build.42").unwrap().to_string(), "1.2.3+build.42"
            .to_string()
        );
        assert_eq!(
            Version::parse("1.2.3-alpha1+42").unwrap().to_string(), "1.2.3-alpha1+42"
            .to_string()
        );
    }
    #[test]
    fn test_lt() {
        assert!(Version::parse("0.0.0") < Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.0.0") < Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.2.0") < Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.2.3-alpha1") < Version::parse("1.2.3"));
        assert!(Version::parse("1.2.3-alpha1") < Version::parse("1.2.3-alpha2"));
        assert!(! (Version::parse("1.2.3-alpha2") < Version::parse("1.2.3-alpha2")));
        assert!(! (Version::parse("1.2.3+23") < Version::parse("1.2.3+42")));
    }
    #[test]
    fn test_le() {
        assert!(Version::parse("0.0.0") <= Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.0.0") <= Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.2.0") <= Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.2.3-alpha1") <= Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.2.3-alpha2") <= Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.2.3+23") <= Version::parse("1.2.3+42"));
    }
    #[test]
    fn test_gt() {
        assert!(Version::parse("1.2.3-alpha2") > Version::parse("0.0.0"));
        assert!(Version::parse("1.2.3-alpha2") > Version::parse("1.0.0"));
        assert!(Version::parse("1.2.3-alpha2") > Version::parse("1.2.0"));
        assert!(Version::parse("1.2.3-alpha2") > Version::parse("1.2.3-alpha1"));
        assert!(Version::parse("1.2.3") > Version::parse("1.2.3-alpha2"));
        assert!(! (Version::parse("1.2.3-alpha2") > Version::parse("1.2.3-alpha2")));
        assert!(! (Version::parse("1.2.3+23") > Version::parse("1.2.3+42")));
    }
    #[test]
    fn test_ge() {
        assert!(Version::parse("1.2.3-alpha2") >= Version::parse("0.0.0"));
        assert!(Version::parse("1.2.3-alpha2") >= Version::parse("1.0.0"));
        assert!(Version::parse("1.2.3-alpha2") >= Version::parse("1.2.0"));
        assert!(Version::parse("1.2.3-alpha2") >= Version::parse("1.2.3-alpha1"));
        assert!(Version::parse("1.2.3-alpha2") >= Version::parse("1.2.3-alpha2"));
        assert!(Version::parse("1.2.3+23") >= Version::parse("1.2.3+42"));
    }
    #[test]
    fn test_prerelease_check() {
        assert!(Version::parse("1.0.0").unwrap().is_prerelease() == false);
        assert!(Version::parse("0.0.1").unwrap().is_prerelease() == false);
        assert!(Version::parse("4.1.4-alpha").unwrap().is_prerelease());
        assert!(Version::parse("1.0.0-beta294296").unwrap().is_prerelease());
    }
    #[test]
    fn test_spec_order() {
        let vs = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        let mut i = 1;
        while i < vs.len() {
            let a = Version::parse(vs[i - 1]);
            let b = Version::parse(vs[i]);
            assert!(a < b, "nope {:?} < {:?}", a, b);
            i += 1;
        }
    }
    #[test]
    fn test_from_str() {
        assert_eq!(
            "1.2.3".parse(), Ok(Version { major : 1, minor : 2, patch : 3, pre :
            Vec::new(), build : Vec::new(), })
        );
        assert_eq!(
            "  1.2.3  ".parse(), Ok(Version { major : 1, minor : 2, patch : 3, pre :
            Vec::new(), build : Vec::new(), })
        );
        assert_eq!(
            "1.2.3-alpha1".parse(), Ok(Version { major : 1, minor : 2, patch : 3, pre :
            vec![Identifier::AlphaNumeric(String::from("alpha1"))], build : Vec::new(),
            })
        );
        assert_eq!(
            "  1.2.3-alpha1  ".parse(), Ok(Version { major : 1, minor : 2, patch : 3, pre
            : vec![Identifier::AlphaNumeric(String::from("alpha1"))], build : Vec::new(),
            })
        );
        assert_eq!(
            "1.2.3+build5".parse(), Ok(Version { major : 1, minor : 2, patch : 3, pre :
            Vec::new(), build : vec![Identifier::AlphaNumeric(String::from("build5"))],
            })
        );
        assert_eq!(
            "  1.2.3+build5  ".parse(), Ok(Version { major : 1, minor : 2, patch : 3, pre
            : Vec::new(), build : vec![Identifier::AlphaNumeric(String::from("build5"))],
            })
        );
        assert_eq!(
            "1.2.3-alpha1+build5".parse(), Ok(Version { major : 1, minor : 2, patch : 3,
            pre : vec![Identifier::AlphaNumeric(String::from("alpha1"))], build :
            vec![Identifier::AlphaNumeric(String::from("build5"))], })
        );
        assert_eq!(
            "  1.2.3-alpha1+build5  ".parse(), Ok(Version { major : 1, minor : 2, patch :
            3, pre : vec![Identifier::AlphaNumeric(String::from("alpha1"))], build :
            vec![Identifier::AlphaNumeric(String::from("build5"))], })
        );
        assert_eq!(
            "1.2.3-1.alpha1.9+build5.7.3aedf  ".parse(), Ok(Version { major : 1, minor :
            2, patch : 3, pre : vec![Identifier::Numeric(1),
            Identifier::AlphaNumeric(String::from("alpha1")), Identifier::Numeric(9),],
            build : vec![Identifier::AlphaNumeric(String::from("build5")),
            Identifier::Numeric(7), Identifier::AlphaNumeric(String::from("3aedf")),], })
        );
        assert_eq!(
            "0.4.0-beta.1+0851523".parse(), Ok(Version { major : 0, minor : 4, patch : 0,
            pre : vec![Identifier::AlphaNumeric(String::from("beta")),
            Identifier::Numeric(1),], build :
            vec![Identifier::AlphaNumeric(String::from("0851523"))], })
        );
    }
    #[test]
    fn test_from_str_errors() {
        fn parse_error(e: &str) -> result::Result<Version, SemVerError> {
            return Err(SemVerError::ParseError(e.to_string()));
        }
        assert_eq!("".parse(), parse_error("expected more input"));
        assert_eq!("  ".parse(), parse_error("expected more input"));
        assert_eq!("1".parse(), parse_error("expected more input"));
        assert_eq!("1.2".parse(), parse_error("expected more input"));
        assert_eq!("1.2.3-".parse(), parse_error("expected more input"));
        assert_eq!(
            "a.b.c".parse(),
            parse_error("encountered unexpected token: AlphaNumeric(\"a\")")
        );
        assert_eq!(
            "1.2.3 abc".parse(),
            parse_error("expected end of input, but got: [AlphaNumeric(\"abc\")]")
        );
    }
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;
    use crate::*;
    use semver_parser::version::Identifier;
    #[test]
    fn test_from_numeric_identifier() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_from_numeric_identifier = 0;
        let rug_fuzz_0 = 123;
        let rug_fuzz_1 = 123;
        let other = Identifier::Numeric(rug_fuzz_0);
        let expected = Identifier::Numeric(rug_fuzz_1);
        let result = <Identifier as std::convert::From<Identifier>>::from(other);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_from_numeric_identifier = 0;
    }
    #[test]
    fn test_from_alphanumeric_identifier() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_from_alphanumeric_identifier = 0;
        let rug_fuzz_0 = "abc123";
        let rug_fuzz_1 = "abc123";
        let other = Identifier::AlphaNumeric(String::from(rug_fuzz_0));
        let expected = Identifier::AlphaNumeric(String::from(rug_fuzz_1));
        let result = <Identifier as std::convert::From<Identifier>>::from(other);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_from_alphanumeric_identifier = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_3_llm_16_2 {
    use super::*;
    use crate::*;
    use crate::Version;
    use std::cmp::Ordering;
    use std::str::FromStr;
    #[test]
    fn test_cmp() {
        let _rug_st_tests_llm_16_3_llm_16_2_rrrruuuugggg_test_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let v1 = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let v2 = Version::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(v1.cmp(& v2), Ordering::Equal);
        let _rug_ed_tests_llm_16_3_llm_16_2_rrrruuuugggg_test_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_4 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_4_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 1;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 1;
        let rug_fuzz_15 = 1;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 0;
        let rug_fuzz_18 = 1;
        let rug_fuzz_19 = 0;
        let rug_fuzz_20 = 0;
        let v1 = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let v2 = Version::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let v3 = Version::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        let v4 = Version::new(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11);
        let v5 = Version::new(rug_fuzz_12, rug_fuzz_13, rug_fuzz_14);
        let v6 = Version::new(rug_fuzz_15, rug_fuzz_16, rug_fuzz_17);
        let v7 = Version::new(rug_fuzz_18, rug_fuzz_19, rug_fuzz_20);
        debug_assert_eq!(v1.eq(& v2), true);
        debug_assert_eq!(v1.eq(& v3), false);
        debug_assert_eq!(v2.eq(& v4), false);
        debug_assert_eq!(v2.eq(& v5), false);
        debug_assert_eq!(v1.eq(& v6), true);
        debug_assert_eq!(v6.eq(& v7), true);
        let _rug_ed_tests_llm_16_4_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_5 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_5_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 1;
        let rug_fuzz_13 = 1;
        let rug_fuzz_14 = 1;
        let version1 = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let version2 = Version::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        let version3 = Version::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8);
        let version4 = Version::new(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11);
        let version5 = Version::new(rug_fuzz_12, rug_fuzz_13, rug_fuzz_14);
        debug_assert_eq!(version1.partial_cmp(& version2), Some(Ordering::Less));
        debug_assert_eq!(version2.partial_cmp(& version1), Some(Ordering::Greater));
        debug_assert_eq!(version3.partial_cmp(& version4), Some(Ordering::Equal));
        debug_assert_eq!(version4.partial_cmp(& version3), Some(Ordering::Equal));
        debug_assert_eq!(version4.partial_cmp(& version5), Some(Ordering::Less));
        debug_assert_eq!(version5.partial_cmp(& version4), Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_5_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_6 {
    use super::*;
    use crate::*;
    use std::str::FromStr;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_6_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let version = Version::from((rug_fuzz_0, rug_fuzz_1, rug_fuzz_2));
        debug_assert_eq!(version.major, 1);
        debug_assert_eq!(version.minor, 2);
        debug_assert_eq!(version.patch, 3);
        debug_assert_eq!(version.pre, vec![]);
        debug_assert_eq!(version.build, vec![]);
        let _rug_ed_tests_llm_16_6_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_8_llm_16_7 {
    use semver_parser::version::Version;
    use semver_parser::version::Identifier;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_8_llm_16_7_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = "build";
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 0;
        let other = semver_parser::version::Version {
            major: rug_fuzz_0,
            minor: rug_fuzz_1,
            patch: rug_fuzz_2,
            pre: vec![
                semver_parser::version::Identifier::Numeric(rug_fuzz_3),
                semver_parser::version::Identifier::AlphaNumeric("alpha".to_string())
            ],
            build: vec![
                semver_parser::version::Identifier::AlphaNumeric(rug_fuzz_4.to_string())
            ],
        };
        let result = Version::from(other);
        debug_assert_eq!(result.major, 1);
        debug_assert_eq!(result.minor, 2);
        debug_assert_eq!(result.patch, 3);
        debug_assert_eq!(result.pre.len(), 2);
        debug_assert_eq!(result.pre[rug_fuzz_5], Identifier::Numeric(1));
        debug_assert_eq!(
            result.pre[rug_fuzz_6], Identifier::AlphaNumeric("alpha".to_string())
        );
        debug_assert_eq!(result.build.len(), 1);
        debug_assert_eq!(
            result.build[rug_fuzz_7], Identifier::AlphaNumeric("build".to_string())
        );
        let _rug_ed_tests_llm_16_8_llm_16_7_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9 {
    use std::hash::{Hash, Hasher};
    use version::{Version, Identifier};
    #[test]
    fn test_hash() {
        let _rug_st_tests_llm_16_9_rrrruuuugggg_test_hash = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = "test";
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let version = Version {
            major: rug_fuzz_0,
            minor: rug_fuzz_1,
            patch: rug_fuzz_2,
            pre: vec![Identifier::Numeric(rug_fuzz_3)],
            build: vec![Identifier::AlphaNumeric(rug_fuzz_4.to_string())],
        };
        version.hash(&mut hasher);
        let hash_result = hasher.finish();
        debug_assert_eq!(hash_result, 16619063093792307336);
        let _rug_ed_tests_llm_16_9_rrrruuuugggg_test_hash = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_10 {
    use std::str::FromStr;
    use crate::version::Version;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_10_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "1.2.3";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let input = rug_fuzz_0;
        let expected = Version::new(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let result = <Version as FromStr>::from_str(input);
        debug_assert_eq!(result, Ok(expected));
        let _rug_ed_tests_llm_16_10_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_22 {
    use super::*;
    use crate::*;
    #[test]
    fn test_clear_metadata() {
        let _rug_st_tests_llm_16_22_rrrruuuugggg_test_clear_metadata = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = "pre";
        let rug_fuzz_4 = "build";
        let mut version = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        version.pre = vec![Identifier::AlphaNumeric(String::from(rug_fuzz_3))];
        version.build = vec![Identifier::AlphaNumeric(String::from(rug_fuzz_4))];
        version.clear_metadata();
        debug_assert_eq!(version.pre.len(), 0);
        debug_assert_eq!(version.build.len(), 0);
        let _rug_ed_tests_llm_16_22_rrrruuuugggg_test_clear_metadata = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_23 {
    use super::*;
    use crate::*;
    #[test]
    fn test_increment_major() {
        let _rug_st_tests_llm_16_23_rrrruuuugggg_test_increment_major = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut version = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        version.increment_major();
        debug_assert_eq!(version.major, 2);
        debug_assert_eq!(version.minor, 0);
        debug_assert_eq!(version.patch, 0);
        debug_assert_eq!(version.pre, Vec::new());
        debug_assert_eq!(version.build, Vec::new());
        let _rug_ed_tests_llm_16_23_rrrruuuugggg_test_increment_major = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_24 {
    use super::*;
    use crate::*;
    #[test]
    fn test_increment_minor() {
        let _rug_st_tests_llm_16_24_rrrruuuugggg_test_increment_minor = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut version = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        version.increment_minor();
        debug_assert_eq!(version.major, 1);
        debug_assert_eq!(version.minor, 3);
        debug_assert_eq!(version.patch, 0);
        debug_assert_eq!(version.pre, Vec::new());
        debug_assert_eq!(version.build, Vec::new());
        let _rug_ed_tests_llm_16_24_rrrruuuugggg_test_increment_minor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_25 {
    use super::*;
    use crate::*;
    #[test]
    fn test_increment_patch() {
        let _rug_st_tests_llm_16_25_rrrruuuugggg_test_increment_patch = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut version = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        version.increment_patch();
        debug_assert_eq!(version.patch, 4);
        debug_assert_eq!(version.pre.len(), 0);
        debug_assert_eq!(version.build.len(), 0);
        let _rug_ed_tests_llm_16_25_rrrruuuugggg_test_increment_patch = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_26 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_prerelease_empty_pre_returns_false() {
        let _rug_st_tests_llm_16_26_rrrruuuugggg_test_is_prerelease_empty_pre_returns_false = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let version = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(version.is_prerelease(), false);
        let _rug_ed_tests_llm_16_26_rrrruuuugggg_test_is_prerelease_empty_pre_returns_false = 0;
    }
    #[test]
    fn test_is_prerelease_non_empty_pre_returns_true() {
        let _rug_st_tests_llm_16_26_rrrruuuugggg_test_is_prerelease_non_empty_pre_returns_true = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let mut version = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        version.pre.push(Identifier::Numeric(rug_fuzz_3));
        debug_assert_eq!(version.is_prerelease(), true);
        let _rug_ed_tests_llm_16_26_rrrruuuugggg_test_is_prerelease_non_empty_pre_returns_true = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_28 {
    use super::*;
    use crate::*;
    use crate::Version;
    #[test]
    fn test_new_version() {
        let _rug_st_tests_llm_16_28_rrrruuuugggg_test_new_version = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let version = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(version.major, 1);
        debug_assert_eq!(version.minor, 2);
        debug_assert_eq!(version.patch, 3);
        debug_assert!(version.pre.is_empty());
        debug_assert!(version.build.is_empty());
        let _rug_ed_tests_llm_16_28_rrrruuuugggg_test_new_version = 0;
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use crate::Version;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "1.2.3";
        let mut p0 = rug_fuzz_0;
        <Version>::parse(&p0).unwrap();
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
