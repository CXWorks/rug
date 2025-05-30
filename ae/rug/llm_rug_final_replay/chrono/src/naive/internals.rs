//! The internal implementation of the calendar and ordinal date.
//!
//! The current implementation is optimized for determining year, month, day and day of week.
//! 4-bit `YearFlags` map to one of 14 possible classes of year in the Gregorian calendar,
//! which are included in every packed `NaiveDate` instance.
//! The conversion between the packed calendar date (`Mdf`) and the ordinal date (`Of`) is
//! based on the moderately-sized lookup table (~1.5KB)
//! and the packed representation is chosen for the efficient lookup.
//! Every internal data structure does not validate its input,
//! but the conversion keeps the valid value valid and the invalid value invalid
//! so that the user-facing `NaiveDate` can validate the input as late as possible.
#![cfg_attr(feature = "__internal_bench", allow(missing_docs))]
use crate::Weekday;
use core::convert::TryFrom;
use core::{fmt, i32};
/// The internal date representation. This also includes the packed `Mdf` value.
pub(super) type DateImpl = i32;
pub(super) const MAX_YEAR: DateImpl = i32::MAX >> 13;
pub(super) const MIN_YEAR: DateImpl = i32::MIN >> 13;
/// The year flags (aka the dominical letter).
///
/// There are 14 possible classes of year in the Gregorian calendar:
/// common and leap years starting with Monday through Sunday.
/// The `YearFlags` stores this information into 4 bits `abbb`,
/// where `a` is `1` for the common year (simplifies the `Of` validation)
/// and `bbb` is a non-zero `Weekday` (mapping `Mon` to 7) of the last day in the past year
/// (simplifies the day of week calculation from the 1-based ordinal).
#[allow(unreachable_pub)]
#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub struct YearFlags(pub(super) u8);
pub(super) const A: YearFlags = YearFlags(0o15);
pub(super) const AG: YearFlags = YearFlags(0o05);
pub(super) const B: YearFlags = YearFlags(0o14);
pub(super) const BA: YearFlags = YearFlags(0o04);
pub(super) const C: YearFlags = YearFlags(0o13);
pub(super) const CB: YearFlags = YearFlags(0o03);
pub(super) const D: YearFlags = YearFlags(0o12);
pub(super) const DC: YearFlags = YearFlags(0o02);
pub(super) const E: YearFlags = YearFlags(0o11);
pub(super) const ED: YearFlags = YearFlags(0o01);
pub(super) const F: YearFlags = YearFlags(0o17);
pub(super) const FE: YearFlags = YearFlags(0o07);
pub(super) const G: YearFlags = YearFlags(0o16);
pub(super) const GF: YearFlags = YearFlags(0o06);
static YEAR_TO_FLAGS: [YearFlags; 400] = [
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    C,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    E,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    G,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
    BA,
    G,
    F,
    E,
    DC,
    B,
    A,
    G,
    FE,
    D,
    C,
    B,
    AG,
    F,
    E,
    D,
    CB,
    A,
    G,
    F,
    ED,
    C,
    B,
    A,
    GF,
    E,
    D,
    C,
];
static YEAR_DELTAS: [u8; 401] = [
    0,
    1,
    1,
    1,
    1,
    2,
    2,
    2,
    2,
    3,
    3,
    3,
    3,
    4,
    4,
    4,
    4,
    5,
    5,
    5,
    5,
    6,
    6,
    6,
    6,
    7,
    7,
    7,
    7,
    8,
    8,
    8,
    8,
    9,
    9,
    9,
    9,
    10,
    10,
    10,
    10,
    11,
    11,
    11,
    11,
    12,
    12,
    12,
    12,
    13,
    13,
    13,
    13,
    14,
    14,
    14,
    14,
    15,
    15,
    15,
    15,
    16,
    16,
    16,
    16,
    17,
    17,
    17,
    17,
    18,
    18,
    18,
    18,
    19,
    19,
    19,
    19,
    20,
    20,
    20,
    20,
    21,
    21,
    21,
    21,
    22,
    22,
    22,
    22,
    23,
    23,
    23,
    23,
    24,
    24,
    24,
    24,
    25,
    25,
    25,
    25,
    25,
    25,
    25,
    25,
    26,
    26,
    26,
    26,
    27,
    27,
    27,
    27,
    28,
    28,
    28,
    28,
    29,
    29,
    29,
    29,
    30,
    30,
    30,
    30,
    31,
    31,
    31,
    31,
    32,
    32,
    32,
    32,
    33,
    33,
    33,
    33,
    34,
    34,
    34,
    34,
    35,
    35,
    35,
    35,
    36,
    36,
    36,
    36,
    37,
    37,
    37,
    37,
    38,
    38,
    38,
    38,
    39,
    39,
    39,
    39,
    40,
    40,
    40,
    40,
    41,
    41,
    41,
    41,
    42,
    42,
    42,
    42,
    43,
    43,
    43,
    43,
    44,
    44,
    44,
    44,
    45,
    45,
    45,
    45,
    46,
    46,
    46,
    46,
    47,
    47,
    47,
    47,
    48,
    48,
    48,
    48,
    49,
    49,
    49,
    49,
    49,
    49,
    49,
    49,
    50,
    50,
    50,
    50,
    51,
    51,
    51,
    51,
    52,
    52,
    52,
    52,
    53,
    53,
    53,
    53,
    54,
    54,
    54,
    54,
    55,
    55,
    55,
    55,
    56,
    56,
    56,
    56,
    57,
    57,
    57,
    57,
    58,
    58,
    58,
    58,
    59,
    59,
    59,
    59,
    60,
    60,
    60,
    60,
    61,
    61,
    61,
    61,
    62,
    62,
    62,
    62,
    63,
    63,
    63,
    63,
    64,
    64,
    64,
    64,
    65,
    65,
    65,
    65,
    66,
    66,
    66,
    66,
    67,
    67,
    67,
    67,
    68,
    68,
    68,
    68,
    69,
    69,
    69,
    69,
    70,
    70,
    70,
    70,
    71,
    71,
    71,
    71,
    72,
    72,
    72,
    72,
    73,
    73,
    73,
    73,
    73,
    73,
    73,
    73,
    74,
    74,
    74,
    74,
    75,
    75,
    75,
    75,
    76,
    76,
    76,
    76,
    77,
    77,
    77,
    77,
    78,
    78,
    78,
    78,
    79,
    79,
    79,
    79,
    80,
    80,
    80,
    80,
    81,
    81,
    81,
    81,
    82,
    82,
    82,
    82,
    83,
    83,
    83,
    83,
    84,
    84,
    84,
    84,
    85,
    85,
    85,
    85,
    86,
    86,
    86,
    86,
    87,
    87,
    87,
    87,
    88,
    88,
    88,
    88,
    89,
    89,
    89,
    89,
    90,
    90,
    90,
    90,
    91,
    91,
    91,
    91,
    92,
    92,
    92,
    92,
    93,
    93,
    93,
    93,
    94,
    94,
    94,
    94,
    95,
    95,
    95,
    95,
    96,
    96,
    96,
    96,
    97,
    97,
    97,
    97,
];
pub(super) fn cycle_to_yo(cycle: u32) -> (u32, u32) {
    let mut year_mod_400 = cycle / 365;
    let mut ordinal0 = cycle % 365;
    let delta = u32::from(YEAR_DELTAS[year_mod_400 as usize]);
    if ordinal0 < delta {
        year_mod_400 -= 1;
        ordinal0 += 365 - u32::from(YEAR_DELTAS[year_mod_400 as usize]);
    } else {
        ordinal0 -= delta;
    }
    (year_mod_400, ordinal0 + 1)
}
pub(super) fn yo_to_cycle(year_mod_400: u32, ordinal: u32) -> u32 {
    year_mod_400 * 365 + u32::from(YEAR_DELTAS[year_mod_400 as usize]) + ordinal - 1
}
impl YearFlags {
    #[allow(unreachable_pub)]
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub fn from_year(year: i32) -> YearFlags {
        let year = year.rem_euclid(400);
        YearFlags::from_year_mod_400(year)
    }
    #[inline]
    pub(super) fn from_year_mod_400(year: i32) -> YearFlags {
        YEAR_TO_FLAGS[year as usize]
    }
    #[inline]
    pub(super) fn ndays(&self) -> u32 {
        let YearFlags(flags) = *self;
        366 - u32::from(flags >> 3)
    }
    #[inline]
    pub(super) fn isoweek_delta(&self) -> u32 {
        let YearFlags(flags) = *self;
        let mut delta = u32::from(flags) & 0b0111;
        if delta < 3 {
            delta += 7;
        }
        delta
    }
    #[inline]
    pub(super) const fn nisoweeks(&self) -> u32 {
        let YearFlags(flags) = *self;
        52 + ((0b0000_0100_0000_0110 >> flags as usize) & 1)
    }
}
impl fmt::Debug for YearFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let YearFlags(flags) = *self;
        match flags {
            0o15 => "A".fmt(f),
            0o05 => "AG".fmt(f),
            0o14 => "B".fmt(f),
            0o04 => "BA".fmt(f),
            0o13 => "C".fmt(f),
            0o03 => "CB".fmt(f),
            0o12 => "D".fmt(f),
            0o02 => "DC".fmt(f),
            0o11 => "E".fmt(f),
            0o01 => "ED".fmt(f),
            0o10 => "F?".fmt(f),
            0o00 => "FE?".fmt(f),
            0o17 => "F".fmt(f),
            0o07 => "FE".fmt(f),
            0o16 => "G".fmt(f),
            0o06 => "GF".fmt(f),
            _ => write!(f, "YearFlags({})", flags),
        }
    }
}
pub(super) const MIN_OL: u32 = 1 << 1;
pub(super) const MAX_OL: u32 = 366 << 1;
pub(super) const MAX_MDL: u32 = (12 << 6) | (31 << 1) | 1;
const XX: i8 = -128;
static MDL_TO_OL: [i8; MAX_MDL as usize + 1] = [
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    XX,
    XX,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    XX,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    XX,
    XX,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    XX,
    XX,
    XX,
    XX,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    XX,
    XX,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    XX,
    XX,
    XX,
    XX,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    XX,
    XX,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    XX,
    XX,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    XX,
    XX,
    XX,
    XX,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    XX,
    XX,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    XX,
    XX,
    XX,
    XX,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
];
static OL_TO_MDL: [u8; MAX_OL as usize + 1] = [
    0,
    0,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    64,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    66,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    74,
    72,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    76,
    74,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    80,
    78,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    82,
    80,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    86,
    84,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    88,
    86,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    90,
    88,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    94,
    92,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    96,
    94,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
    100,
    98,
];
/// Ordinal (day of year) and year flags: `(ordinal << 4) | flags`.
///
/// The whole bits except for the least 3 bits are referred as `Ol` (ordinal and leap flag),
/// which is an index to the `OL_TO_MDL` lookup table.
#[derive(PartialEq, PartialOrd, Copy, Clone)]
pub(super) struct Of(pub(crate) u32);
impl Of {
    #[inline]
    pub(super) fn new(ordinal: u32, YearFlags(flags): YearFlags) -> Option<Of> {
        match ordinal <= 366 {
            true => Some(Of((ordinal << 4) | u32::from(flags))),
            false => None,
        }
    }
    #[inline]
    pub(super) fn from_mdf(Mdf(mdf): Mdf) -> Of {
        let mdl = mdf >> 3;
        match MDL_TO_OL.get(mdl as usize) {
            Some(&v) => Of(mdf.wrapping_sub((i32::from(v) as u32 & 0x3ff) << 3)),
            None => Of(0),
        }
    }
    #[inline]
    pub(super) fn valid(&self) -> bool {
        let Of(of) = *self;
        let ol = of >> 3;
        (MIN_OL..=MAX_OL).contains(&ol)
    }
    #[inline]
    pub(super) const fn ordinal(&self) -> u32 {
        let Of(of) = *self;
        of >> 4
    }
    #[inline]
    pub(super) const fn with_ordinal(&self, ordinal: u32) -> Option<Of> {
        if ordinal > 366 {
            return None;
        }
        let Of(of) = *self;
        Some(Of((of & 0b1111) | (ordinal << 4)))
    }
    #[inline]
    pub(super) const fn flags(&self) -> YearFlags {
        let Of(of) = *self;
        YearFlags((of & 0b1111) as u8)
    }
    #[inline]
    pub(super) fn weekday(&self) -> Weekday {
        let Of(of) = *self;
        Weekday::try_from((((of >> 4) + (of & 0b111)) % 7) as u8).unwrap()
    }
    #[inline]
    pub(super) fn isoweekdate_raw(&self) -> (u32, Weekday) {
        let Of(of) = *self;
        let weekord = (of >> 4).wrapping_add(self.flags().isoweek_delta());
        (weekord / 7, Weekday::try_from((weekord % 7) as u8).unwrap())
    }
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::wrong_self_convention))]
    #[inline]
    pub(super) fn to_mdf(&self) -> Mdf {
        Mdf::from_of(*self)
    }
    #[inline]
    pub(super) const fn succ(&self) -> Of {
        let Of(of) = *self;
        Of(of + (1 << 4))
    }
    #[inline]
    pub(super) const fn pred(&self) -> Of {
        let Of(of) = *self;
        Of(of - (1 << 4))
    }
}
impl fmt::Debug for Of {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Of(of) = *self;
        write!(
            f, "Of(({} << 4) | {:#04o} /*{:?}*/)", of >> 4, of & 0b1111, YearFlags((of &
            0b1111) as u8)
        )
    }
}
/// Month, day of month and year flags: `(month << 9) | (day << 4) | flags`
///
/// The whole bits except for the least 3 bits are referred as `Mdl`
/// (month, day of month and leap flag),
/// which is an index to the `MDL_TO_OL` lookup table.
#[derive(PartialEq, PartialOrd, Copy, Clone)]
pub(super) struct Mdf(pub(super) u32);
impl Mdf {
    #[inline]
    pub(super) fn new(month: u32, day: u32, YearFlags(flags): YearFlags) -> Option<Mdf> {
        match month <= 12 && day <= 31 {
            true => Some(Mdf((month << 9) | (day << 4) | u32::from(flags))),
            false => None,
        }
    }
    #[inline]
    pub(super) fn from_of(Of(of): Of) -> Mdf {
        let ol = of >> 3;
        match OL_TO_MDL.get(ol as usize) {
            Some(&v) => Mdf(of + (u32::from(v) << 3)),
            None => Mdf(0),
        }
    }
    #[cfg(test)]
    pub(super) fn valid(&self) -> bool {
        let Mdf(mdf) = *self;
        let mdl = mdf >> 3;
        match MDL_TO_OL.get(mdl as usize) {
            Some(&v) => v >= 0,
            None => false,
        }
    }
    #[inline]
    pub(super) const fn month(&self) -> u32 {
        let Mdf(mdf) = *self;
        mdf >> 9
    }
    #[inline]
    pub(super) const fn with_month(&self, month: u32) -> Option<Mdf> {
        if month > 12 {
            return None;
        }
        let Mdf(mdf) = *self;
        Some(Mdf((mdf & 0b1_1111_1111) | (month << 9)))
    }
    #[inline]
    pub(super) const fn day(&self) -> u32 {
        let Mdf(mdf) = *self;
        (mdf >> 4) & 0b1_1111
    }
    #[inline]
    pub(super) const fn with_day(&self, day: u32) -> Option<Mdf> {
        if day > 31 {
            return None;
        }
        let Mdf(mdf) = *self;
        Some(Mdf((mdf & !0b1_1111_0000) | (day << 4)))
    }
    #[inline]
    pub(super) fn with_flags(&self, YearFlags(flags): YearFlags) -> Mdf {
        let Mdf(mdf) = *self;
        Mdf((mdf & !0b1111) | u32::from(flags))
    }
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::wrong_self_convention))]
    #[inline]
    pub(super) fn to_of(&self) -> Of {
        Of::from_mdf(*self)
    }
}
impl fmt::Debug for Mdf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Mdf(mdf) = *self;
        write!(
            f, "Mdf(({} << 9) | ({} << 4) | {:#04o} /*{:?}*/)", mdf >> 9, (mdf >> 4) &
            0b1_1111, mdf & 0b1111, YearFlags((mdf & 0b1111) as u8)
        )
    }
}
#[cfg(test)]
mod tests {
    use num_iter::range_inclusive;
    use std::u32;
    use super::{Mdf, Of};
    use super::{YearFlags, A, AG, B, BA, C, CB, D, DC, E, ED, F, FE, G, GF};
    use crate::Weekday;
    const NONLEAP_FLAGS: [YearFlags; 7] = [A, B, C, D, E, F, G];
    const LEAP_FLAGS: [YearFlags; 7] = [AG, BA, CB, DC, ED, FE, GF];
    const FLAGS: [YearFlags; 14] = [A, B, C, D, E, F, G, AG, BA, CB, DC, ED, FE, GF];
    #[test]
    fn test_year_flags_ndays_from_year() {
        assert_eq!(YearFlags::from_year(2014).ndays(), 365);
        assert_eq!(YearFlags::from_year(2012).ndays(), 366);
        assert_eq!(YearFlags::from_year(2000).ndays(), 366);
        assert_eq!(YearFlags::from_year(1900).ndays(), 365);
        assert_eq!(YearFlags::from_year(1600).ndays(), 366);
        assert_eq!(YearFlags::from_year(1).ndays(), 365);
        assert_eq!(YearFlags::from_year(0).ndays(), 366);
        assert_eq!(YearFlags::from_year(- 1).ndays(), 365);
        assert_eq!(YearFlags::from_year(- 4).ndays(), 366);
        assert_eq!(YearFlags::from_year(- 99).ndays(), 365);
        assert_eq!(YearFlags::from_year(- 100).ndays(), 365);
        assert_eq!(YearFlags::from_year(- 399).ndays(), 365);
        assert_eq!(YearFlags::from_year(- 400).ndays(), 366);
    }
    #[test]
    fn test_year_flags_nisoweeks() {
        assert_eq!(A.nisoweeks(), 52);
        assert_eq!(B.nisoweeks(), 52);
        assert_eq!(C.nisoweeks(), 52);
        assert_eq!(D.nisoweeks(), 53);
        assert_eq!(E.nisoweeks(), 52);
        assert_eq!(F.nisoweeks(), 52);
        assert_eq!(G.nisoweeks(), 52);
        assert_eq!(AG.nisoweeks(), 52);
        assert_eq!(BA.nisoweeks(), 52);
        assert_eq!(CB.nisoweeks(), 52);
        assert_eq!(DC.nisoweeks(), 53);
        assert_eq!(ED.nisoweeks(), 53);
        assert_eq!(FE.nisoweeks(), 52);
        assert_eq!(GF.nisoweeks(), 52);
    }
    #[test]
    fn test_of() {
        fn check(expected: bool, flags: YearFlags, ordinal1: u32, ordinal2: u32) {
            for ordinal in range_inclusive(ordinal1, ordinal2) {
                let of = match Of::new(ordinal, flags) {
                    Some(of) => of,
                    None if !expected => continue,
                    None => panic!("Of::new({}, {:?}) returned None", ordinal, flags),
                };
                assert!(
                    of.valid() == expected,
                    "ordinal {} = {:?} should be {} for dominical year {:?}", ordinal,
                    of, if expected { "valid" } else { "invalid" }, flags
                );
            }
        }
        for &flags in NONLEAP_FLAGS.iter() {
            check(false, flags, 0, 0);
            check(true, flags, 1, 365);
            check(false, flags, 366, 1024);
            check(false, flags, u32::MAX, u32::MAX);
        }
        for &flags in LEAP_FLAGS.iter() {
            check(false, flags, 0, 0);
            check(true, flags, 1, 366);
            check(false, flags, 367, 1024);
            check(false, flags, u32::MAX, u32::MAX);
        }
    }
    #[test]
    fn test_mdf_valid() {
        fn check(
            expected: bool,
            flags: YearFlags,
            month1: u32,
            day1: u32,
            month2: u32,
            day2: u32,
        ) {
            for month in range_inclusive(month1, month2) {
                for day in range_inclusive(day1, day2) {
                    let mdf = match Mdf::new(month, day, flags) {
                        Some(mdf) => mdf,
                        None if !expected => continue,
                        None => {
                            panic!(
                                "Mdf::new({}, {}, {:?}) returned None", month, day, flags
                            )
                        }
                    };
                    assert!(
                        mdf.valid() == expected,
                        "month {} day {} = {:?} should be {} for dominical year {:?}",
                        month, day, mdf, if expected { "valid" } else { "invalid" },
                        flags
                    );
                }
            }
        }
        for &flags in NONLEAP_FLAGS.iter() {
            check(false, flags, 0, 0, 0, 1024);
            check(false, flags, 0, 0, 16, 0);
            check(true, flags, 1, 1, 1, 31);
            check(false, flags, 1, 32, 1, 1024);
            check(true, flags, 2, 1, 2, 28);
            check(false, flags, 2, 29, 2, 1024);
            check(true, flags, 3, 1, 3, 31);
            check(false, flags, 3, 32, 3, 1024);
            check(true, flags, 4, 1, 4, 30);
            check(false, flags, 4, 31, 4, 1024);
            check(true, flags, 5, 1, 5, 31);
            check(false, flags, 5, 32, 5, 1024);
            check(true, flags, 6, 1, 6, 30);
            check(false, flags, 6, 31, 6, 1024);
            check(true, flags, 7, 1, 7, 31);
            check(false, flags, 7, 32, 7, 1024);
            check(true, flags, 8, 1, 8, 31);
            check(false, flags, 8, 32, 8, 1024);
            check(true, flags, 9, 1, 9, 30);
            check(false, flags, 9, 31, 9, 1024);
            check(true, flags, 10, 1, 10, 31);
            check(false, flags, 10, 32, 10, 1024);
            check(true, flags, 11, 1, 11, 30);
            check(false, flags, 11, 31, 11, 1024);
            check(true, flags, 12, 1, 12, 31);
            check(false, flags, 12, 32, 12, 1024);
            check(false, flags, 13, 0, 16, 1024);
            check(false, flags, u32::MAX, 0, u32::MAX, 1024);
            check(false, flags, 0, u32::MAX, 16, u32::MAX);
            check(false, flags, u32::MAX, u32::MAX, u32::MAX, u32::MAX);
        }
        for &flags in LEAP_FLAGS.iter() {
            check(false, flags, 0, 0, 0, 1024);
            check(false, flags, 0, 0, 16, 0);
            check(true, flags, 1, 1, 1, 31);
            check(false, flags, 1, 32, 1, 1024);
            check(true, flags, 2, 1, 2, 29);
            check(false, flags, 2, 30, 2, 1024);
            check(true, flags, 3, 1, 3, 31);
            check(false, flags, 3, 32, 3, 1024);
            check(true, flags, 4, 1, 4, 30);
            check(false, flags, 4, 31, 4, 1024);
            check(true, flags, 5, 1, 5, 31);
            check(false, flags, 5, 32, 5, 1024);
            check(true, flags, 6, 1, 6, 30);
            check(false, flags, 6, 31, 6, 1024);
            check(true, flags, 7, 1, 7, 31);
            check(false, flags, 7, 32, 7, 1024);
            check(true, flags, 8, 1, 8, 31);
            check(false, flags, 8, 32, 8, 1024);
            check(true, flags, 9, 1, 9, 30);
            check(false, flags, 9, 31, 9, 1024);
            check(true, flags, 10, 1, 10, 31);
            check(false, flags, 10, 32, 10, 1024);
            check(true, flags, 11, 1, 11, 30);
            check(false, flags, 11, 31, 11, 1024);
            check(true, flags, 12, 1, 12, 31);
            check(false, flags, 12, 32, 12, 1024);
            check(false, flags, 13, 0, 16, 1024);
            check(false, flags, u32::MAX, 0, u32::MAX, 1024);
            check(false, flags, 0, u32::MAX, 16, u32::MAX);
            check(false, flags, u32::MAX, u32::MAX, u32::MAX, u32::MAX);
        }
    }
    #[test]
    fn test_of_fields() {
        for &flags in FLAGS.iter() {
            for ordinal in range_inclusive(1u32, 366) {
                let of = Of::new(ordinal, flags).unwrap();
                if of.valid() {
                    assert_eq!(of.ordinal(), ordinal);
                }
            }
        }
    }
    #[test]
    fn test_of_with_fields() {
        fn check(flags: YearFlags, ordinal: u32) {
            let of = Of::new(ordinal, flags).unwrap();
            for ordinal in range_inclusive(0u32, 1024) {
                let of = match of.with_ordinal(ordinal) {
                    Some(of) => of,
                    None if ordinal > 366 => continue,
                    None => panic!("failed to create Of with ordinal {}", ordinal),
                };
                assert_eq!(of.valid(), Of::new(ordinal, flags).unwrap().valid());
                if of.valid() {
                    assert_eq!(of.ordinal(), ordinal);
                }
            }
        }
        for &flags in NONLEAP_FLAGS.iter() {
            check(flags, 1);
            check(flags, 365);
        }
        for &flags in LEAP_FLAGS.iter() {
            check(flags, 1);
            check(flags, 366);
        }
    }
    #[test]
    fn test_of_weekday() {
        assert_eq!(Of::new(1, A).unwrap().weekday(), Weekday::Sun);
        assert_eq!(Of::new(1, B).unwrap().weekday(), Weekday::Sat);
        assert_eq!(Of::new(1, C).unwrap().weekday(), Weekday::Fri);
        assert_eq!(Of::new(1, D).unwrap().weekday(), Weekday::Thu);
        assert_eq!(Of::new(1, E).unwrap().weekday(), Weekday::Wed);
        assert_eq!(Of::new(1, F).unwrap().weekday(), Weekday::Tue);
        assert_eq!(Of::new(1, G).unwrap().weekday(), Weekday::Mon);
        assert_eq!(Of::new(1, AG).unwrap().weekday(), Weekday::Sun);
        assert_eq!(Of::new(1, BA).unwrap().weekday(), Weekday::Sat);
        assert_eq!(Of::new(1, CB).unwrap().weekday(), Weekday::Fri);
        assert_eq!(Of::new(1, DC).unwrap().weekday(), Weekday::Thu);
        assert_eq!(Of::new(1, ED).unwrap().weekday(), Weekday::Wed);
        assert_eq!(Of::new(1, FE).unwrap().weekday(), Weekday::Tue);
        assert_eq!(Of::new(1, GF).unwrap().weekday(), Weekday::Mon);
        for &flags in FLAGS.iter() {
            let mut prev = Of::new(1, flags).unwrap().weekday();
            for ordinal in range_inclusive(2u32, flags.ndays()) {
                let of = Of::new(ordinal, flags).unwrap();
                let expected = prev.succ();
                assert_eq!(of.weekday(), expected);
                prev = expected;
            }
        }
    }
    #[test]
    fn test_mdf_fields() {
        for &flags in FLAGS.iter() {
            for month in range_inclusive(1u32, 12) {
                for day in range_inclusive(1u32, 31) {
                    let mdf = match Mdf::new(month, day, flags) {
                        Some(mdf) => mdf,
                        None => continue,
                    };
                    if mdf.valid() {
                        assert_eq!(mdf.month(), month);
                        assert_eq!(mdf.day(), day);
                    }
                }
            }
        }
    }
    #[test]
    fn test_mdf_with_fields() {
        fn check(flags: YearFlags, month: u32, day: u32) {
            let mdf = Mdf::new(month, day, flags).unwrap();
            for month in range_inclusive(0u32, 16) {
                let mdf = match mdf.with_month(month) {
                    Some(mdf) => mdf,
                    None if month > 12 => continue,
                    None => panic!("failed to create Mdf with month {}", month),
                };
                if mdf.valid() {
                    assert_eq!(mdf.month(), month);
                    assert_eq!(mdf.day(), day);
                }
            }
            for day in range_inclusive(0u32, 1024) {
                let mdf = match mdf.with_day(day) {
                    Some(mdf) => mdf,
                    None if day > 31 => continue,
                    None => panic!("failed to create Mdf with month {}", month),
                };
                if mdf.valid() {
                    assert_eq!(mdf.month(), month);
                    assert_eq!(mdf.day(), day);
                }
            }
        }
        for &flags in NONLEAP_FLAGS.iter() {
            check(flags, 1, 1);
            check(flags, 1, 31);
            check(flags, 2, 1);
            check(flags, 2, 28);
            check(flags, 2, 29);
            check(flags, 12, 31);
        }
        for &flags in LEAP_FLAGS.iter() {
            check(flags, 1, 1);
            check(flags, 1, 31);
            check(flags, 2, 1);
            check(flags, 2, 29);
            check(flags, 2, 30);
            check(flags, 12, 31);
        }
    }
    #[test]
    fn test_of_isoweekdate_raw() {
        for &flags in FLAGS.iter() {
            let (week, _) = Of::new(4, flags).unwrap().isoweekdate_raw();
            assert_eq!(week, 1);
        }
    }
    #[test]
    fn test_of_to_mdf() {
        for i in range_inclusive(0u32, 8192) {
            let of = Of(i);
            assert_eq!(of.valid(), of.to_mdf().valid());
        }
    }
    #[test]
    fn test_mdf_to_of() {
        for i in range_inclusive(0u32, 8192) {
            let mdf = Mdf(i);
            assert_eq!(mdf.valid(), mdf.to_of().valid());
        }
    }
    #[test]
    fn test_of_to_mdf_to_of() {
        for i in range_inclusive(0u32, 8192) {
            let of = Of(i);
            if of.valid() {
                assert_eq!(of, of.to_mdf().to_of());
            }
        }
    }
    #[test]
    fn test_mdf_to_of_to_mdf() {
        for i in range_inclusive(0u32, 8192) {
            let mdf = Mdf(i);
            if mdf.valid() {
                assert_eq!(mdf, mdf.to_of().to_mdf());
            }
        }
    }
}
#[cfg(test)]
mod tests_rug_334 {
    use super::*;
    use crate::naive::internals::cycle_to_yo;
    #[test]
    fn test_cycle_to_yo() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let cycle: u32 = rug_fuzz_0;
        cycle_to_yo(cycle);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_335 {
    use super::*;
    use crate::naive::internals::yo_to_cycle;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        yo_to_cycle(p0, p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_336 {
    use super::*;
    use crate::naive::internals::YearFlags;
    #[test]
    fn test_from_year() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i32 = rug_fuzz_0;
        YearFlags::from_year(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_337 {
    use super::*;
    use crate::naive::internals::YearFlags;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: i32 = rug_fuzz_0;
        YearFlags::from_year_mod_400(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_338 {
    use super::*;
    use crate::naive::internals::YearFlags;
    #[test]
    fn test_ndays() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u8, u8, u8, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let flags = YearFlags(rug_fuzz_0);
        debug_assert_eq!(flags.ndays(), 366);
        let flags = YearFlags(rug_fuzz_1);
        debug_assert_eq!(flags.ndays(), 365);
        let flags = YearFlags(rug_fuzz_2);
        debug_assert_eq!(flags.ndays(), 365);
        let flags = YearFlags(rug_fuzz_3);
        debug_assert_eq!(flags.ndays(), 364);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_339 {
    use super::*;
    use crate::naive::internals::YearFlags;
    #[test]
    fn test_isoweek_delta() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = YearFlags::from_year(rug_fuzz_0);
        debug_assert_eq!(p0.isoweek_delta(), 2);
        let p1 = YearFlags::from_year_mod_400(rug_fuzz_1);
        debug_assert_eq!(p1.isoweek_delta(), 2);
        let p2 = YearFlags::from_year_mod_400(rug_fuzz_2);
        debug_assert_eq!(p2.isoweek_delta(), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_340 {
    use super::*;
    use crate::naive::internals::YearFlags;
    #[test]
    fn test_nisoweeks() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = YearFlags::from_year(rug_fuzz_0);
        debug_assert_eq!(p0.nisoweeks(), 53);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_341 {
    use super::*;
    use crate::naive::internals::{YearFlags, Of};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let ordinal: u32 = rug_fuzz_0;
        let flags: YearFlags = YearFlags::from_year(rug_fuzz_1);
        Of::new(ordinal, flags);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_342 {
    use super::*;
    use crate::naive::internals::{Mdf, Of};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = Mdf::from_of(Of(rug_fuzz_0));
        Of::from_mdf(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_345 {
    use super::*;
    use crate::naive::internals::Of;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Of(rug_fuzz_0);
        let mut p1 = rug_fuzz_1;
        p0.with_ordinal(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_350 {
    use super::*;
    use crate::naive::internals::Of;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = Of(rug_fuzz_0);
        debug_assert_eq!(Of::succ(& p0), Of(16));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_351 {
    use super::*;
    use crate::naive::internals::Of;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Of(rug_fuzz_0);
        Of::pred(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_352 {
    use super::*;
    use crate::naive::internals::{YearFlags, Mdf};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u32, u32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u32 = rug_fuzz_0;
        let mut p1: u32 = rug_fuzz_1;
        let mut p2 = YearFlags::from_year(rug_fuzz_2);
        Mdf::new(p0, p1, p2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_358 {
    use super::*;
    use crate::naive::internals::Mdf;
    use crate::naive::internals::YearFlags;
    #[test]
    fn test_with_flags() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, u8) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Mdf(rug_fuzz_0);
        let mut p1 = YearFlags(rug_fuzz_1);
        Mdf::with_flags(&mut p0, p1);
             }
}
}
}    }
}
