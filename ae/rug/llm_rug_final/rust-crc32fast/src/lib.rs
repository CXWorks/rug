//! Fast, SIMD-accelerated CRC32 (IEEE) checksum computation.
//!
//! ## Usage
//!
//! ### Simple usage
//!
//! For simple use-cases, you can call the [`hash()`] convenience function to
//! directly compute the CRC32 checksum for a given byte slice:
//!
//! ```rust
//! let checksum = crc32fast::hash(b"foo bar baz");
//! ```
//!
//! ### Advanced usage
//!
//! For use-cases that require more flexibility or performance, for example when
//! processing large amounts of data, you can create and manipulate a [`Hasher`]:
//!
//! ```rust
//! use crc32fast::Hasher;
//!
//! let mut hasher = Hasher::new();
//! hasher.update(b"foo bar baz");
//! let checksum = hasher.finalize();
//! ```
//!
//! ## Performance
//!
//! This crate contains multiple CRC32 implementations:
//!
//! - A fast baseline implementation which processes up to 16 bytes per iteration
//! - An optimized implementation for modern `x86` using `sse` and `pclmulqdq` instructions
//!
//! Calling the [`Hasher::new`] constructor at runtime will perform a feature detection to select the most
//! optimal implementation for the current CPU feature set.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    all(feature = "nightly", target_arch = "aarch64"),
    feature(stdsimd, aarch64_target_feature)
)]

//#[deny(missing_docs)]
#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[macro_use]
extern crate cfg_if;

#[cfg(feature = "std")]
use std as core;

use core::fmt;
use core::hash;

mod baseline;
mod combine;
mod specialized;
mod table;

/// Computes the CRC32 hash of a byte slice.
///
/// Check out [`Hasher`] for more advanced use-cases.
pub fn hash(buf: &[u8]) -> u32 {
    let mut h = Hasher::new();
    h.update(buf);
    h.finalize()
}

#[derive(Clone)]
enum State {
    Baseline(baseline::State),
    Specialized(specialized::State),
}

#[derive(Clone)]
/// Represents an in-progress CRC32 computation.
pub struct Hasher {
    amount: u64,
    state: State,
}

const DEFAULT_INIT_STATE: u32 = 0;

impl Hasher {
    /// Create a new `Hasher`.
    ///
    /// This will perform a CPU feature detection at runtime to select the most
    /// optimal implementation for the current processor architecture.
    pub fn new() -> Self {
        Self::new_with_initial(DEFAULT_INIT_STATE)
    }

    /// Create a new `Hasher` with an initial CRC32 state.
    ///
    /// This works just like `Hasher::new`, except that it allows for an initial
    /// CRC32 state to be passed in.
    pub fn new_with_initial(init: u32) -> Self {
        Self::new_with_initial_len(init, 0)
    }

    /// Create a new `Hasher` with an initial CRC32 state.
    ///
    /// As `new_with_initial`, but also accepts a length (in bytes). The
    /// resulting object can then be used with `combine` to compute `crc(a ||
    /// b)` from `crc(a)`, `crc(b)`, and `len(b)`.
    pub fn new_with_initial_len(init: u32, amount: u64) -> Self {
        Self::internal_new_specialized(init, amount)
            .unwrap_or_else(|| Self::internal_new_baseline(init, amount))
    }

    #[doc(hidden)]
    // Internal-only API. Don't use.
    pub fn internal_new_baseline(init: u32, amount: u64) -> Self {
        Hasher {
            amount,
            state: State::Baseline(baseline::State::new(init)),
        }
    }

    #[doc(hidden)]
    // Internal-only API. Don't use.
    pub fn internal_new_specialized(init: u32, amount: u64) -> Option<Self> {
        {
            if let Some(state) = specialized::State::new(init) {
                return Some(Hasher {
                    amount,
                    state: State::Specialized(state),
                });
            }
        }
        None
    }

    /// Process the given byte slice and update the hash state.
    pub fn update(&mut self, buf: &[u8]) {
        self.amount += buf.len() as u64;
        match self.state {
            State::Baseline(ref mut state) => state.update(buf),
            State::Specialized(ref mut state) => state.update(buf),
        }
    }

    /// Finalize the hash state and return the computed CRC32 value.
    pub fn finalize(self) -> u32 {
        match self.state {
            State::Baseline(state) => state.finalize(),
            State::Specialized(state) => state.finalize(),
        }
    }

    /// Reset the hash state.
    pub fn reset(&mut self) {
        self.amount = 0;
        match self.state {
            State::Baseline(ref mut state) => state.reset(),
            State::Specialized(ref mut state) => state.reset(),
        }
    }

    /// Combine the hash state with the hash state for the subsequent block of bytes.
    pub fn combine(&mut self, other: &Self) {
        self.amount += other.amount;
        let other_crc = other.clone().finalize();
        match self.state {
            State::Baseline(ref mut state) => state.combine(other_crc, other.amount),
            State::Specialized(ref mut state) => state.combine(other_crc, other.amount),
        }
    }
}

impl fmt::Debug for Hasher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("crc32fast::Hasher").finish()
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl hash::Hasher for Hasher {
    fn write(&mut self, bytes: &[u8]) {
        self.update(bytes)
    }

    fn finish(&self) -> u64 {
        u64::from(self.clone().finalize())
    }
}

#[cfg(test)]
mod test {
    use super::Hasher;

    quickcheck! {
        fn combine(bytes_1: Vec<u8>, bytes_2: Vec<u8>) -> bool {
            let mut hash_a = Hasher::new();
            hash_a.update(&bytes_1);
            hash_a.update(&bytes_2);
            let mut hash_b = Hasher::new();
            hash_b.update(&bytes_2);
            let mut hash_c = Hasher::new();
            hash_c.update(&bytes_1);
            hash_c.combine(&hash_b);

            hash_a.finalize() == hash_c.finalize()
        }

        fn combine_from_len(bytes_1: Vec<u8>, bytes_2: Vec<u8>) -> bool {
            let mut hash_a = Hasher::new();
            hash_a.update(&bytes_1);
            let a = hash_a.finalize();

            let mut hash_b = Hasher::new();
            hash_b.update(&bytes_2);
            let b = hash_b.finalize();

            let mut hash_ab = Hasher::new();
            hash_ab.update(&bytes_1);
            hash_ab.update(&bytes_2);
            let ab = hash_ab.finalize();

            let mut reconstructed = Hasher::new_with_initial_len(a, bytes_1.len() as u64);
            let hash_b_reconstructed = Hasher::new_with_initial_len(b, bytes_2.len() as u64);

            reconstructed.combine(&hash_b_reconstructed);

            reconstructed.finalize() == ab
        }
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let p0: &[u8] = b"Hello, World!"; // sample data
        
        crate::hash(p0);
    }
}#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::Hasher;
    
    #[test]
    fn test_rug() {
        Hasher::new();
    }
}#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    
    #[test]
    fn test_hasher() {
        let p0: u32 = 0; // Sample data
        
        <Hasher>::new_with_initial(p0);

    }
}#[cfg(test)]
mod tests_rug_23 {
    use super::*;

    #[test]
    fn test_rug() {
        let p0: u32 = 12345;
        let p1: u64 = 9876543210;

        <Hasher>::new_with_initial_len(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_24 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let p0: u32 = 123; // Sample data for the first argument
        let p1: u64 = 456; // Sample data for the second argument

        <Hasher>::internal_new_baseline(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0 = 0;
        let mut p1 = 10;
        
        Hasher::internal_new_specialized(p0, p1);
    }
}#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::Hasher;
    
    #[test]
    fn test_rug() {
        let mut p0 = Hasher::new();
        let p1: &[u8] = b"sample_data";
        
        p0.update(p1);
    }
}#[cfg(test)]
mod tests_rug_27 {
    use super::*;
    use crate::Hasher;

    #[test]
    fn test_rug() {
        let mut p0 = Hasher::new();

        Hasher::finalize(p0);
    }
}#[cfg(test)]
mod tests_rug_28 {
    use super::*;
    use crate::Hasher;

    #[test]
    fn test_reset() {
        let mut p0 = {
            let mut v4 = Hasher::new();
            v4
        };

        <Hasher>::reset(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::Hasher;

    #[test]
    fn test_combine() {
        let mut p0 = Hasher::new();
        let mut p1 = Hasher::new();
        p0.combine(&p1);
    }
}
#[cfg(test)]
mod tests_rug_30 {
    use super::*;
    use crate::Hasher;
    use std::default::Default;

    #[test]
    fn test_rug() {
        <Hasher as Default>::default();
    }
}
#[cfg(test)]
mod tests_rug_31 {
    use super::*;
    use crate::Hasher;

    #[test]
    fn test_rug() {
        let mut p0 = Hasher::new();
        let p1: &[u8] = b"test";

        <Hasher as std::hash::Hasher>::write(&mut p0, p1);
    }
}#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::Hasher;
    use std::hash::Hasher as StdHasher;

    #[test]
    fn test_rug() {
        let mut p0 = Hasher::new();
        StdHasher::finish(&p0);
    }
}