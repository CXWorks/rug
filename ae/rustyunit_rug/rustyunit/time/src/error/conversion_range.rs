//! Conversion range error

use core::convert::TryFrom;
use core::fmt;

use crate::error;

/// An error type indicating that a conversion failed because the target type could not store the
/// initial value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionRange;

impl fmt::Display for ConversionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Source value is out of range for the target type")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConversionRange {}

impl From<ConversionRange> for crate::Error {
    fn from(err: ConversionRange) -> Self {
        Self::ConversionRange(err)
    }
}

impl TryFrom<crate::Error> for ConversionRange {
    type Error = error::DifferentVariant;

    fn try_from(err: crate::Error) -> Result<Self, Self::Error> {
        match err {
            crate::Error::ConversionRange(err) => Ok(err),
            _ => Err(error::DifferentVariant),
        }
    }
}
#[cfg(test)]
mod tests_llm_16_97 {
    use std::convert::TryFrom;
    use crate::error::{Error, ConversionRange, DifferentVariant};

    #[test]
    fn test_try_from_conversion_range() {
        let err = Error::ConversionRange(ConversionRange);
        let result = ConversionRange::try_from(err);
        assert!(result.is_ok());
        assert_eq!(result.unwrap_err(), DifferentVariant);
    }
}#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::error::conversion_range::ConversionRange;
    use crate::error::Error;
    
    #[test]
    fn test_conversion_range() {
        let mut p0 = ConversionRange;
        Error::from(p0);
    }
}