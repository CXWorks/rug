// Copyright 2014-2016 bluss and ndarray developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use super::Dimension;
use std::error::Error;
use std::fmt;

/// An error related to array shape or layout.
#[derive(Clone)]
pub struct ShapeError {
    // we want to be able to change this representation later
    repr: ErrorKind,
}

impl ShapeError {
    /// Return the `ErrorKind` of this error.
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.repr
    }

    /// Create a new `ShapeError`
    pub fn from_kind(error: ErrorKind) -> Self {
        from_kind(error)
    }
}

/// Error code for an error related to array shape or layout.
///
/// This enumeration is not exhaustive. The representation of the enum
/// is not guaranteed.
#[derive(Copy, Clone, Debug)]
pub enum ErrorKind {
    /// incompatible shape
    IncompatibleShape = 1,
    /// incompatible memory layout
    IncompatibleLayout,
    /// the shape does not fit inside type limits
    RangeLimited,
    /// out of bounds indexing
    OutOfBounds,
    /// aliasing array elements
    Unsupported,
    /// overflow when computing offset, length, etc.
    Overflow,
    #[doc(hidden)]
    __Incomplete,
}

#[inline(always)]
pub fn from_kind(k: ErrorKind) -> ShapeError {
    ShapeError { repr: k }
}

impl PartialEq for ErrorKind {
    #[inline(always)]
    fn eq(&self, rhs: &Self) -> bool {
        *self as u8 == *rhs as u8
    }
}

impl PartialEq for ShapeError {
    #[inline(always)]
    fn eq(&self, rhs: &Self) -> bool {
        self.repr == rhs.repr
    }
}

impl Error for ShapeError {}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self.kind() {
            ErrorKind::IncompatibleShape => "incompatible shapes",
            ErrorKind::IncompatibleLayout => "incompatible memory layout",
            ErrorKind::RangeLimited => "the shape does not fit in type limits",
            ErrorKind::OutOfBounds => "out of bounds indexing",
            ErrorKind::Unsupported => "unsupported operation",
            ErrorKind::Overflow => "arithmetic overflow",
            ErrorKind::__Incomplete => "this error variant is not in use",
        };
        write!(f, "ShapeError/{:?}: {}", self.kind(), description)
    }
}

impl fmt::Debug for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub fn incompatible_shapes<D, E>(_a: &D, _b: &E) -> ShapeError
where
    D: Dimension,
    E: Dimension,
{
    from_kind(ErrorKind::IncompatibleShape)
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::error::ErrorKind;
    
    #[test]
    fn test_rug() {
        let p0 = ErrorKind::IncompatibleShape;
        
        crate::error::from_kind(p0);
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::error::{ShapeError, ErrorKind};

    #[test]
    fn test_rug() {
        let p0 = ShapeError::from_kind(ErrorKind::IncompatibleShape);

        assert_eq!(ShapeError::kind(&p0), ErrorKind::IncompatibleShape);
    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::error::ShapeError;

    #[test]
    fn test_rug() {
        // Sample data for the first ShapeError parameter
        let mut p0 = ShapeError::from_kind(ErrorKind::IncompatibleShape);

        // Sample data for the second ShapeError parameter
        let mut p1 = ShapeError::from_kind(ErrorKind::IncompatibleShape);

        // Calling the eq function for unit testing
        p0.eq(&p1);
    }
}