use const_fn::const_fn;
use core::fmt::{self, Display};
#[cfg(feature = "serde")]
use standback::convert::TryInto;
use Weekday::*;
/// Days of the week.
///
/// As order is dependent on context (Sunday could be either
/// two days after or five days before Friday), this type does not implement
/// `PartialOrd` or `Ord`.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "crate::serde::Weekday"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weekday {
    #[allow(clippy::missing_docs_in_private_items)]
    Monday,
    #[allow(clippy::missing_docs_in_private_items)]
    Tuesday,
    #[allow(clippy::missing_docs_in_private_items)]
    Wednesday,
    #[allow(clippy::missing_docs_in_private_items)]
    Thursday,
    #[allow(clippy::missing_docs_in_private_items)]
    Friday,
    #[allow(clippy::missing_docs_in_private_items)]
    Saturday,
    #[allow(clippy::missing_docs_in_private_items)]
    Sunday,
}
#[cfg(feature = "serde")]
impl<'a> serde::Deserialize<'a> for Weekday {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        crate::serde::Weekday::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}
impl Weekday {
    /// Get the previous weekday.
    ///
    /// ```rust
    /// # use time::Weekday;
    /// assert_eq!(Weekday::Tuesday.previous(), Weekday::Monday);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn previous(self) -> Self {
        match self {
            Monday => Sunday,
            Tuesday => Monday,
            Wednesday => Tuesday,
            Thursday => Wednesday,
            Friday => Thursday,
            Saturday => Friday,
            Sunday => Saturday,
        }
    }
    /// Get the next weekday.
    ///
    /// ```rust
    /// # use time::Weekday;
    /// assert_eq!(Weekday::Monday.next(), Weekday::Tuesday);
    /// ```
    ///
    /// This function is `const fn` when using rustc >= 1.46.
    #[const_fn("1.46")]
    pub const fn next(self) -> Self {
        match self {
            Monday => Tuesday,
            Tuesday => Wednesday,
            Wednesday => Thursday,
            Thursday => Friday,
            Friday => Saturday,
            Saturday => Sunday,
            Sunday => Monday,
        }
    }
    /// Get the ISO 8601 weekday number. Equivalent to
    /// [`Weekday::number_from_monday`].
    ///
    /// ```rust
    /// # use time::Weekday;
    /// assert_eq!(Weekday::Monday.iso_weekday_number(), 1);
    /// ```
    pub const fn iso_weekday_number(self) -> u8 {
        self.number_from_monday()
    }
    /// Get the one-indexed number of days from Monday.
    ///
    /// ```rust
    /// # use time::Weekday;
    /// assert_eq!(Weekday::Monday.number_from_monday(), 1);
    /// ```
    pub const fn number_from_monday(self) -> u8 {
        self.number_days_from_monday() + 1
    }
    /// Get the one-indexed number of days from Sunday.
    ///
    /// ```rust
    /// # use time::Weekday;
    /// assert_eq!(Weekday::Monday.number_from_sunday(), 2);
    /// ```
    pub const fn number_from_sunday(self) -> u8 {
        self.number_days_from_sunday() + 1
    }
    /// Get the zero-indexed number of days from Monday.
    ///
    /// ```rust
    /// # use time::Weekday;
    /// assert_eq!(Weekday::Monday.number_days_from_monday(), 0);
    /// ```
    pub const fn number_days_from_monday(self) -> u8 {
        self as u8
    }
    /// Get the zero-indexed number of days from Sunday.
    ///
    /// ```rust
    /// # use time::Weekday;
    /// assert_eq!(Weekday::Monday.number_days_from_sunday(), 1);
    /// ```
    pub const fn number_days_from_sunday(self) -> u8 {
        (self as u8 + 1) % 7
    }
}
impl Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            match self {
                Monday => "Monday",
                Tuesday => "Tuesday",
                Wednesday => "Wednesday",
                Thursday => "Thursday",
                Friday => "Friday",
                Saturday => "Saturday",
                Sunday => "Sunday",
            },
        )
    }
}
#[cfg(test)]
mod tests_llm_16_1045 {
    use crate::weekday::Weekday;
    #[test]
    fn test_iso_weekday_number() {
        let _rug_st_tests_llm_16_1045_rrrruuuugggg_test_iso_weekday_number = 0;
        debug_assert_eq!(Weekday::Monday.iso_weekday_number(), 1);
        debug_assert_eq!(Weekday::Tuesday.iso_weekday_number(), 2);
        debug_assert_eq!(Weekday::Wednesday.iso_weekday_number(), 3);
        debug_assert_eq!(Weekday::Thursday.iso_weekday_number(), 4);
        debug_assert_eq!(Weekday::Friday.iso_weekday_number(), 5);
        debug_assert_eq!(Weekday::Saturday.iso_weekday_number(), 6);
        debug_assert_eq!(Weekday::Sunday.iso_weekday_number(), 7);
        let _rug_ed_tests_llm_16_1045_rrrruuuugggg_test_iso_weekday_number = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1046 {
    use super::*;
    use crate::*;
    use std::string::ToString;
    #[test]
    fn test_next_weekday() {
        let _rug_st_tests_llm_16_1046_rrrruuuugggg_test_next_weekday = 0;
        debug_assert_eq!(Weekday::Monday.next(), Weekday::Tuesday);
        debug_assert_eq!(Weekday::Tuesday.next(), Weekday::Wednesday);
        debug_assert_eq!(Weekday::Wednesday.next(), Weekday::Thursday);
        debug_assert_eq!(Weekday::Thursday.next(), Weekday::Friday);
        debug_assert_eq!(Weekday::Friday.next(), Weekday::Saturday);
        debug_assert_eq!(Weekday::Saturday.next(), Weekday::Sunday);
        debug_assert_eq!(Weekday::Sunday.next(), Weekday::Monday);
        let _rug_ed_tests_llm_16_1046_rrrruuuugggg_test_next_weekday = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1047 {
    use super::*;
    use crate::*;
    use crate::Weekday;
    #[test]
    fn test_number_days_from_monday() {
        let _rug_st_tests_llm_16_1047_rrrruuuugggg_test_number_days_from_monday = 0;
        debug_assert_eq!(Weekday::Monday.number_days_from_monday(), 0);
        debug_assert_eq!(Weekday::Tuesday.number_days_from_monday(), 1);
        debug_assert_eq!(Weekday::Wednesday.number_days_from_monday(), 2);
        debug_assert_eq!(Weekday::Thursday.number_days_from_monday(), 3);
        debug_assert_eq!(Weekday::Friday.number_days_from_monday(), 4);
        debug_assert_eq!(Weekday::Saturday.number_days_from_monday(), 5);
        debug_assert_eq!(Weekday::Sunday.number_days_from_monday(), 6);
        let _rug_ed_tests_llm_16_1047_rrrruuuugggg_test_number_days_from_monday = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1049 {
    use crate::weekday::Weekday;
    #[test]
    fn test_number_days_from_sunday() {
        let _rug_st_tests_llm_16_1049_rrrruuuugggg_test_number_days_from_sunday = 0;
        debug_assert_eq!(Weekday::Monday.number_days_from_sunday(), 1);
        debug_assert_eq!(Weekday::Tuesday.number_days_from_sunday(), 2);
        debug_assert_eq!(Weekday::Wednesday.number_days_from_sunday(), 3);
        debug_assert_eq!(Weekday::Thursday.number_days_from_sunday(), 4);
        debug_assert_eq!(Weekday::Friday.number_days_from_sunday(), 5);
        debug_assert_eq!(Weekday::Saturday.number_days_from_sunday(), 6);
        debug_assert_eq!(Weekday::Sunday.number_days_from_sunday(), 0);
        let _rug_ed_tests_llm_16_1049_rrrruuuugggg_test_number_days_from_sunday = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1052 {
    use crate::weekday::Weekday;
    #[test]
    fn test_number_from_sunday() {
        let _rug_st_tests_llm_16_1052_rrrruuuugggg_test_number_from_sunday = 0;
        debug_assert_eq!(Weekday::Monday.number_from_sunday(), 2);
        debug_assert_eq!(Weekday::Tuesday.number_from_sunday(), 3);
        debug_assert_eq!(Weekday::Wednesday.number_from_sunday(), 4);
        debug_assert_eq!(Weekday::Thursday.number_from_sunday(), 5);
        debug_assert_eq!(Weekday::Friday.number_from_sunday(), 6);
        debug_assert_eq!(Weekday::Saturday.number_from_sunday(), 7);
        debug_assert_eq!(Weekday::Sunday.number_from_sunday(), 1);
        let _rug_ed_tests_llm_16_1052_rrrruuuugggg_test_number_from_sunday = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_1053 {
    use super::*;
    use crate::*;
    use weekday::Weekday::{
        Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,
    };
    #[test]
    fn test_previous_weekday() {
        let _rug_st_tests_llm_16_1053_rrrruuuugggg_test_previous_weekday = 0;
        debug_assert_eq!(Monday.previous(), Sunday);
        debug_assert_eq!(Tuesday.previous(), Monday);
        debug_assert_eq!(Wednesday.previous(), Tuesday);
        debug_assert_eq!(Thursday.previous(), Wednesday);
        debug_assert_eq!(Friday.previous(), Thursday);
        debug_assert_eq!(Saturday.previous(), Friday);
        debug_assert_eq!(Sunday.previous(), Saturday);
        let _rug_ed_tests_llm_16_1053_rrrruuuugggg_test_previous_weekday = 0;
    }
}
#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use crate::Weekday;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_482_rrrruuuugggg_test_rug = 0;
        use crate::Weekday;
        let mut p0: Weekday = Weekday::Monday;
        <Weekday>::number_from_monday(p0);
        let _rug_ed_tests_rug_482_rrrruuuugggg_test_rug = 0;
    }
}
