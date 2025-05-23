//! Conversion range error
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
mod tests_rug_287 {
    use super::*;
    use crate::error::{ConversionRange, Error};
    use std::convert::From;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_287_rrrruuuugggg_test_rug = 0;
        let mut p0 = ConversionRange;
        Error::from(p0);
        let _rug_ed_tests_rug_287_rrrruuuugggg_test_rug = 0;
    }
}
