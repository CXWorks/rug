// This Source Code Form is subject to the terms of
// the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You
// can obtain one at http://mozilla.org/MPL/2.0/.

//! `Skip` trait to allow efficient skipping of consecutive bytes.

use std::i64;
use std::io::{Result, Error, ErrorKind, Seek, SeekFrom};

/// Type which supports skipping a number of bytes.
///
/// Similar in spirit to `std::io::Seek` but only allows
/// uni-directional movement.
pub trait Skip {
    /// Skip over `n` consecutive bytes.
    fn skip(&mut self, n: u64) -> Result<()>;
}

impl<A: Seek> Skip for A {
    /// `n` must be in range `[0, i64::MAX]`.
    fn skip(&mut self, n: u64) -> Result<()> {
        if n > i64::MAX as u64 {
            return Err(Error::new(ErrorKind::Other, "n too large"))
        }
        self.seek(SeekFrom::Current(n as i64)).and(Ok(()))
    }
}

#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::skip::Skip;
    use std::io::{Error, ErrorKind, Cursor};
    use std::io::{Seek, SeekFrom};

    #[test]
    fn test_skip() {
        let data: Vec<u8> = vec![195, 164, 110, 97, 109, 101, 164, 65, 108, 105, 99, 101];
        let mut cursor = Cursor::new(data);
        let mut n: u64 = 10;

        <std::io::Cursor<_> as Skip>::skip(&mut cursor, n).unwrap();
    }
}
