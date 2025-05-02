//! The `Month` enum and its associated `impl`s.

use core::convert::TryFrom;
use core::fmt;
use core::num::NonZeroU8;

use self::Month::*;
use crate::error;

/// Months of the year.
#[allow(clippy::missing_docs_in_private_items)] // variants
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

impl Month {
    /// Create a `Month` from its numerical value.
    pub(crate) const fn from_number(n: NonZeroU8) -> Result<Self, error::ComponentRange> {
        match n.get() {
            1 => Ok(January),
            2 => Ok(February),
            3 => Ok(March),
            4 => Ok(April),
            5 => Ok(May),
            6 => Ok(June),
            7 => Ok(July),
            8 => Ok(August),
            9 => Ok(September),
            10 => Ok(October),
            11 => Ok(November),
            12 => Ok(December),
            n => Err(error::ComponentRange {
                name: "month",
                minimum: 1,
                maximum: 12,
                value: n as _,
                conditional_range: false,
            }),
        }
    }

    /// Get the previous month.
    ///
    /// ```rust
    /// # use time::Month;
    /// assert_eq!(Month::January.previous(), Month::December);
    /// ```
    pub const fn previous(self) -> Self {
        match self {
            January => December,
            February => January,
            March => February,
            April => March,
            May => April,
            June => May,
            July => June,
            August => July,
            September => August,
            October => September,
            November => October,
            December => November,
        }
    }

    /// Get the next month.
    ///
    /// ```rust
    /// # use time::Month;
    /// assert_eq!(Month::January.next(), Month::February);
    /// ```
    pub const fn next(self) -> Self {
        match self {
            January => February,
            February => March,
            March => April,
            April => May,
            May => June,
            June => July,
            July => August,
            August => September,
            September => October,
            October => November,
            November => December,
            December => January,
        }
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            January => "January",
            February => "February",
            March => "March",
            April => "April",
            May => "May",
            June => "June",
            July => "July",
            August => "August",
            September => "September",
            October => "October",
            November => "November",
            December => "December",
        })
    }
}

impl From<Month> for u8 {
    fn from(month: Month) -> Self {
        month as _
    }
}

impl TryFrom<u8> for Month {
    type Error = error::ComponentRange;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match NonZeroU8::new(value) {
            Some(value) => Self::from_number(value),
            None => Err(error::ComponentRange {
                name: "month",
                minimum: 1,
                maximum: 12,
                value: 0,
                conditional_range: false,
            }),
        }
    }
}
#[cfg(test)]
mod tests_llm_16_361 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;

    #[test]
    fn test_month_from() {
        assert_eq!(u8::from(Month::January), 1);
        assert_eq!(u8::from(Month::February), 2);
        assert_eq!(u8::from(Month::March), 3);
        assert_eq!(u8::from(Month::April), 4);
        assert_eq!(u8::from(Month::May), 5);
        assert_eq!(u8::from(Month::June), 6);
        assert_eq!(u8::from(Month::July), 7);
        assert_eq!(u8::from(Month::August), 8);
        assert_eq!(u8::from(Month::September), 9);
        assert_eq!(u8::from(Month::October), 10);
        assert_eq!(u8::from(Month::November), 11);
        assert_eq!(u8::from(Month::December), 12);
    }
}#[cfg(test)]
mod tests_llm_16_363 {
    use crate::month::{Month, error::ComponentRange};
    use std::num::NonZeroU8;
    
    #[test]
    fn test_from_number() {
        assert_eq!(Month::from_number(NonZeroU8::new(1).unwrap()), Ok(Month::January));
        assert_eq!(Month::from_number(NonZeroU8::new(2).unwrap()), Ok(Month::February));
        assert_eq!(Month::from_number(NonZeroU8::new(3).unwrap()), Ok(Month::March));
        assert_eq!(Month::from_number(NonZeroU8::new(4).unwrap()), Ok(Month::April));
        assert_eq!(Month::from_number(NonZeroU8::new(5).unwrap()), Ok(Month::May));
        assert_eq!(Month::from_number(NonZeroU8::new(6).unwrap()), Ok(Month::June));
        assert_eq!(Month::from_number(NonZeroU8::new(7).unwrap()), Ok(Month::July));
        assert_eq!(Month::from_number(NonZeroU8::new(8).unwrap()), Ok(Month::August));
        assert_eq!(Month::from_number(NonZeroU8::new(9).unwrap()), Ok(Month::September));
        assert_eq!(Month::from_number(NonZeroU8::new(10).unwrap()), Ok(Month::October));
        assert_eq!(Month::from_number(NonZeroU8::new(11).unwrap()), Ok(Month::November));
        assert_eq!(Month::from_number(NonZeroU8::new(12).unwrap()), Ok(Month::December));
        
        assert_eq!(Month::from_number(NonZeroU8::new(0).unwrap()), Err(ComponentRange {
            name: "month",
            minimum: 1,
            maximum: 12,
            value: 0,
            conditional_range: false,
        }));
        
        assert_eq!(Month::from_number(NonZeroU8::new(13).unwrap()), Err(ComponentRange {
            name: "month",
            minimum: 1,
            maximum: 12,
            value: 13,
            conditional_range: false,
        }));
    }
}#[cfg(test)]
mod tests_llm_16_364 {
    use super::*;

use crate::*;

    #[test]
    fn test_next() {
        assert_eq!(Month::January.next(), Month::February);
        assert_eq!(Month::February.next(), Month::March);
        assert_eq!(Month::March.next(), Month::April);
        assert_eq!(Month::April.next(), Month::May);
        assert_eq!(Month::May.next(), Month::June);
        assert_eq!(Month::June.next(), Month::July);
        assert_eq!(Month::July.next(), Month::August);
        assert_eq!(Month::August.next(), Month::September);
        assert_eq!(Month::September.next(), Month::October);
        assert_eq!(Month::October.next(), Month::November);
        assert_eq!(Month::November.next(), Month::December);
        assert_eq!(Month::December.next(), Month::January);
    }
}mod tests_llm_16_365 {
    use super::*;

use crate::*;
    use std::convert::TryFrom;
    use crate::month::Month::*;

    #[test]
    fn test_previous() {
        assert_eq!(Month::January.previous(), Month::December);
        assert_eq!(Month::February.previous(), Month::January);
        assert_eq!(Month::March.previous(), Month::February);
        assert_eq!(Month::April.previous(), Month::March);
        assert_eq!(Month::May.previous(), Month::April);
        assert_eq!(Month::June.previous(), Month::May);
        assert_eq!(Month::July.previous(), Month::June);
        assert_eq!(Month::August.previous(), Month::July);
        assert_eq!(Month::September.previous(), Month::August);
        assert_eq!(Month::October.previous(), Month::September);
        assert_eq!(Month::November.previous(), Month::October);
        assert_eq!(Month::December.previous(), Month::November);
    }
}