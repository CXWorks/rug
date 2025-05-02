//! Different variant error

use core::convert::TryFrom;
use core::fmt;

/// An error type indicating that a [`TryFrom`](core::convert::TryFrom) call failed because the
/// original value was of a different variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentVariant;

impl fmt::Display for DifferentVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "value was of a different variant than required")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DifferentVariant {}

impl From<DifferentVariant> for crate::Error {
    fn from(err: DifferentVariant) -> Self {
        Self::DifferentVariant(err)
    }
}

impl TryFrom<crate::Error> for DifferentVariant {
    type Error = Self;

    fn try_from(err: crate::Error) -> Result<Self, Self::Error> {
        match err {
            crate::Error::DifferentVariant(err) => Ok(err),
            _ => Err(Self),
        }
    }
}
#[cfg(test)]
mod tests_llm_16_340 {
    use super::*;

use crate::*;
    use crate::error::conversion_range::ConversionRange;
    use crate::error::different_variant::DifferentVariant;
    use crate::error::component_range::ComponentRange;
    
    #[test]
    fn test_from() {
        let err = DifferentVariant;
        let result = Error::from(err);
        match result {
            Error::DifferentVariant(err) => assert_eq!(err, DifferentVariant),
            _ => panic!("Unexpected error variant"),
        }
    }
}