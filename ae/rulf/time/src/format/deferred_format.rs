//! The `DeferredFormat` struct, acting as an intermediary between a request to
//! format and the final output.
use crate::{
    format::{format_specifier, parse_fmt_string, well_known, Format, FormatItem},
    Date, Time, UtcOffset,
};
use core::fmt::{self, Display, Formatter};
/// A struct containing all the necessary information to display the inner type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DeferredFormat {
    /// The `Date` to use for formatting.
    date: Option<Date>,
    /// The `Time` to use for formatting.
    time: Option<Time>,
    /// The `UtcOffset` to use for formatting.
    offset: Option<UtcOffset>,
    /// The list of items used to display the item.
    format: Format,
}
impl DeferredFormat {
    /// Create a new `DeferredFormat` with the provided formatting string.
    pub(crate) fn new(format: impl Into<Format>) -> Self {
        Self {
            date: None,
            time: None,
            offset: None,
            format: format.into(),
        }
    }
    /// Provide the `Date` component.
    pub(crate) fn with_date(&mut self, date: Date) -> &mut Self {
        self.date = Some(date);
        self
    }
    /// Provide the `Time` component.
    pub(crate) fn with_time(&mut self, time: Time) -> &mut Self {
        self.time = Some(time);
        self
    }
    /// Provide the `UtcOffset` component.
    pub(crate) fn with_offset(&mut self, offset: UtcOffset) -> &mut Self {
        self.offset = Some(offset);
        self
    }
    /// Obtain the `Date` component.
    pub(crate) const fn date(&self) -> Option<Date> {
        self.date
    }
    /// Obtain the `Time` component.
    pub(crate) const fn time(&self) -> Option<Time> {
        self.time
    }
    /// Obtain the `UtcOffset` component.
    pub(crate) const fn offset(&self) -> Option<UtcOffset> {
        self.offset
    }
}
impl Display for DeferredFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.format {
            Format::Custom(s) => {
                for item in parse_fmt_string(s) {
                    match item {
                        FormatItem::Literal(value) => f.write_str(value)?,
                        FormatItem::Specifier(specifier) => {
                            format_specifier(
                                    f,
                                    self.date,
                                    self.time,
                                    self.offset,
                                    specifier,
                                )
                                .map_err(|_| fmt::Error)?
                        }
                    }
                }
                Ok(())
            }
            Format::Rfc3339 => well_known::rfc3339::fmt(self, f).map_err(|_| fmt::Error),
            #[cfg(not(__time_02_supports_non_exhaustive))]
            Format::__NonExhaustive => unreachable!(),
        }
    }
}
#[cfg(test)]
mod tests_llm_16_829 {
    use crate::format::deferred_format::{DeferredFormat, Format};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_829_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "yyyy-MM-dd";
        let format: Format = rug_fuzz_0.into();
        let deferred_format = DeferredFormat::new(format);
        debug_assert_eq!(deferred_format.date(), None);
        debug_assert_eq!(deferred_format.time(), None);
        debug_assert_eq!(deferred_format.offset(), None);
        debug_assert_eq!(
            deferred_format.format, Format::Custom("yyyy-MM-dd".to_owned())
        );
        let _rug_ed_tests_llm_16_829_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_831_llm_16_830 {
    use crate::format::deferred_format::{DeferredFormat, Format};
    use crate::format::{FormatItem, well_known};
    use crate::{Date, Time, UtcOffset};
    use std::fmt::{self, Formatter};
    use crate::ext::NumericalDuration;
    fn parse_fmt_string(_s: &str) -> Vec<FormatItem> {
        let _rug_st_tests_llm_16_831_llm_16_830_rrrruuuugggg_parse_fmt_string = 0;
        unimplemented!();
        let _rug_ed_tests_llm_16_831_llm_16_830_rrrruuuugggg_parse_fmt_string = 0;
    }
    #[test]
    fn test_offset() {
        let _rug_st_tests_llm_16_831_llm_16_830_rrrruuuugggg_test_offset = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = 0;
        let mut deferred_format = DeferredFormat::new(
            Format::Custom(String::from(rug_fuzz_0)),
        );
        let offset = UtcOffset::seconds(rug_fuzz_1);
        deferred_format.with_offset(offset);
        debug_assert_eq!(deferred_format.offset(), Some(offset));
        let _rug_ed_tests_llm_16_831_llm_16_830_rrrruuuugggg_test_offset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_833 {
    use super::*;
    use crate::*;
    use format::{Date, Time, UtcOffset};
    #[test]
    fn test_time() {
        let _rug_st_tests_llm_16_833_rrrruuuugggg_test_time = 0;
        let rug_fuzz_0 = "%H:%M:%S";
        let rug_fuzz_1 = 12;
        let rug_fuzz_2 = 30;
        let rug_fuzz_3 = 45;
        let mut deferred_format = DeferredFormat::new(
            format::Format::Custom(rug_fuzz_0.to_owned()),
        );
        deferred_format
            .with_time(Time::try_from_hms(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3).unwrap());
        debug_assert_eq!(
            deferred_format.time(), Some(Time::try_from_hms(12, 30, 45).unwrap())
        );
        deferred_format = DeferredFormat::new(format::Format::Rfc3339);
        debug_assert_eq!(deferred_format.time(), None);
        let _rug_ed_tests_llm_16_833_rrrruuuugggg_test_time = 0;
    }
}
#[cfg(test)]
mod tests_rug_343 {
    use super::*;
    use crate::format::deferred_format::DeferredFormat;
    use crate::date::Date;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_343_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "format string";
        let mut p0 = DeferredFormat::new(rug_fuzz_0);
        let p1 = Date::today();
        p0.with_date(p1);
        let _rug_ed_tests_rug_343_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_344 {
    use super::*;
    use crate::format::deferred_format::DeferredFormat;
    use crate::time_mod::Time;
    #[test]
    fn test_with_time() {
        let _rug_st_tests_rug_344_rrrruuuugggg_test_with_time = 0;
        let rug_fuzz_0 = "format string";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let mut p0 = DeferredFormat::new(rug_fuzz_0);
        let mut p1 = Time::try_from_hms(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3).unwrap();
        p0.with_time(p1);
        let _rug_ed_tests_rug_344_rrrruuuugggg_test_with_time = 0;
    }
}
#[cfg(test)]
mod tests_rug_345 {
    use super::*;
    use crate::format::deferred_format::DeferredFormat;
    use crate::UtcOffset;
    #[test]
    fn test_with_offset() {
        let _rug_st_tests_rug_345_rrrruuuugggg_test_with_offset = 0;
        let rug_fuzz_0 = "format string";
        let rug_fuzz_1 = 15;
        let mut p0 = DeferredFormat::new(rug_fuzz_0);
        let mut p1 = UtcOffset::east_minutes(rug_fuzz_1);
        p0.with_offset(p1);
        let _rug_ed_tests_rug_345_rrrruuuugggg_test_with_offset = 0;
    }
}
#[cfg(test)]
mod tests_rug_346 {
    use super::*;
    use crate::format::deferred_format::DeferredFormat;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_346_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "format string";
        let mut p0 = DeferredFormat::new(rug_fuzz_0);
        crate::format::deferred_format::DeferredFormat::date(&p0);
        let _rug_ed_tests_rug_346_rrrruuuugggg_test_rug = 0;
    }
}
