// This is a part of Chrono.
// Portions copyright (c) 2015, John Nagle.
// See README.md and LICENSE.txt for details.

//! Date and time parsing routines.

#![allow(deprecated)]

use core::borrow::Borrow;
use core::str;
use core::usize;

use super::scan;
use super::{Fixed, InternalFixed, InternalInternal, Item, Numeric, Pad, Parsed};
use super::{ParseError, ParseErrorKind, ParseResult};
use super::{BAD_FORMAT, INVALID, NOT_ENOUGH, OUT_OF_RANGE, TOO_LONG, TOO_SHORT};
use crate::{DateTime, FixedOffset, Weekday};

fn set_weekday_with_num_days_from_sunday(p: &mut Parsed, v: i64) -> ParseResult<()> {
    p.set_weekday(match v {
        0 => Weekday::Sun,
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        6 => Weekday::Sat,
        _ => return Err(OUT_OF_RANGE),
    })
}

fn set_weekday_with_number_from_monday(p: &mut Parsed, v: i64) -> ParseResult<()> {
    p.set_weekday(match v {
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        6 => Weekday::Sat,
        7 => Weekday::Sun,
        _ => return Err(OUT_OF_RANGE),
    })
}

/// Parse an RFC 2822 format datetime
/// e.g. `Fri, 21 Nov 1997 09:55:06 -0600`
///
/// This function allows arbitrary intermixed whitespace per RFC 2822 appendix A.5
fn parse_rfc2822<'a>(parsed: &mut Parsed, mut s: &'a str) -> ParseResult<(&'a str, ())> {
    macro_rules! try_consume {
        ($e:expr) => {{
            let (s_, v) = $e?;
            s = s_;
            v
        }};
    }

    // an adapted RFC 2822 syntax from Section 3.3 and 4.3:
    //
    // c-char      = <any char except '(', ')' and '\\'>
    // c-escape    = "\" <any char>
    // comment     = "(" *(comment / c-char / c-escape) ")" *S
    // date-time   = [ day-of-week "," ] date 1*S time *S *comment
    // day-of-week = *S day-name *S
    // day-name    = "Mon" / "Tue" / "Wed" / "Thu" / "Fri" / "Sat" / "Sun"
    // date        = day month year
    // day         = *S 1*2DIGIT *S
    // month       = 1*S month-name 1*S
    // month-name  = "Jan" / "Feb" / "Mar" / "Apr" / "May" / "Jun" /
    //               "Jul" / "Aug" / "Sep" / "Oct" / "Nov" / "Dec"
    // year        = *S 2*DIGIT *S
    // time        = time-of-day 1*S zone
    // time-of-day = hour ":" minute [ ":" second ]
    // hour        = *S 2DIGIT *S
    // minute      = *S 2DIGIT *S
    // second      = *S 2DIGIT *S
    // zone        = ( "+" / "-" ) 4DIGIT /
    //               "UT" / "GMT" /                  ; same as +0000
    //               "EST" / "CST" / "MST" / "PST" / ; same as -0500 to -0800
    //               "EDT" / "CDT" / "MDT" / "PDT" / ; same as -0400 to -0700
    //               1*(%d65-90 / %d97-122)          ; same as -0000
    //
    // some notes:
    //
    // - quoted characters can be in any mixture of lower and upper cases.
    //
    // - we do not recognize a folding white space (FWS) or comment (CFWS).
    //   for our purposes, instead, we accept any sequence of Unicode
    //   white space characters (denoted here to `S`). For comments, we accept
    //   any text within parentheses while respecting escaped parentheses.
    //   Any actual RFC 2822 parser is expected to parse FWS and/or CFWS themselves
    //   and replace it with a single SP (`%x20`); this is legitimate.
    //
    // - two-digit year < 50 should be interpreted by adding 2000.
    //   two-digit year >= 50 or three-digit year should be interpreted
    //   by adding 1900. note that four-or-more-digit years less than 1000
    //   are *never* affected by this rule.
    //
    // - mismatching day-of-week is always an error, which is consistent to
    //   Chrono's own rules.
    //
    // - zones can range from `-9959` to `+9959`, but `FixedOffset` does not
    //   support offsets larger than 24 hours. this is not *that* problematic
    //   since we do not directly go to a `DateTime` so one can recover
    //   the offset information from `Parsed` anyway.

    s = s.trim_left();

    if let Ok((s_, weekday)) = scan::short_weekday(s) {
        if !s_.starts_with(',') {
            return Err(INVALID);
        }
        s = &s_[1..];
        parsed.set_weekday(weekday)?;
    }

    s = s.trim_left();
    parsed.set_day(try_consume!(scan::number(s, 1, 2)))?;
    s = scan::space(s)?; // mandatory
    parsed.set_month(1 + i64::from(try_consume!(scan::short_month0(s))))?;
    s = scan::space(s)?; // mandatory

    // distinguish two- and three-digit years from four-digit years
    let prevlen = s.len();
    let mut year = try_consume!(scan::number(s, 2, usize::MAX));
    let yearlen = prevlen - s.len();
    match (yearlen, year) {
        (2, 0..=49) => {
            year += 2000;
        } //   47 -> 2047,   05 -> 2005
        (2, 50..=99) => {
            year += 1900;
        } //   79 -> 1979
        (3, _) => {
            year += 1900;
        } //  112 -> 2012,  009 -> 1909
        (_, _) => {} // 1987 -> 1987, 0654 -> 0654
    }
    parsed.set_year(year)?;

    s = scan::space(s)?; // mandatory
    parsed.set_hour(try_consume!(scan::number(s, 2, 2)))?;
    s = scan::char(s.trim_left(), b':')?.trim_left(); // *S ":" *S
    parsed.set_minute(try_consume!(scan::number(s, 2, 2)))?;
    if let Ok(s_) = scan::char(s.trim_left(), b':') {
        // [ ":" *S 2DIGIT ]
        parsed.set_second(try_consume!(scan::number(s_, 2, 2)))?;
    }

    s = scan::space(s)?; // mandatory
    if let Some(offset) = try_consume!(scan::timezone_offset_2822(s)) {
        // only set the offset when it is definitely known (i.e. not `-0000`)
        parsed.set_offset(i64::from(offset))?;
    }

    // optional comments
    while let Ok((s_out, ())) = scan::comment_2822(s) {
        s = s_out;
    }

    Ok((s, ()))
}

fn parse_rfc3339<'a>(parsed: &mut Parsed, mut s: &'a str) -> ParseResult<(&'a str, ())> {
    macro_rules! try_consume {
        ($e:expr) => {{
            let (s_, v) = $e?;
            s = s_;
            v
        }};
    }

    // an adapted RFC 3339 syntax from Section 5.6:
    //
    // date-fullyear  = 4DIGIT
    // date-month     = 2DIGIT ; 01-12
    // date-mday      = 2DIGIT ; 01-28, 01-29, 01-30, 01-31 based on month/year
    // time-hour      = 2DIGIT ; 00-23
    // time-minute    = 2DIGIT ; 00-59
    // time-second    = 2DIGIT ; 00-58, 00-59, 00-60 based on leap second rules
    // time-secfrac   = "." 1*DIGIT
    // time-numoffset = ("+" / "-") time-hour ":" time-minute
    // time-offset    = "Z" / time-numoffset
    // partial-time   = time-hour ":" time-minute ":" time-second [time-secfrac]
    // full-date      = date-fullyear "-" date-month "-" date-mday
    // full-time      = partial-time time-offset
    // date-time      = full-date "T" full-time
    //
    // some notes:
    //
    // - quoted characters can be in any mixture of lower and upper cases.
    //
    // - it may accept any number of fractional digits for seconds.
    //   for Chrono, this means that we should skip digits past first 9 digits.
    //
    // - unlike RFC 2822, the valid offset ranges from -23:59 to +23:59.
    //   note that this restriction is unique to RFC 3339 and not ISO 8601.
    //   since this is not a typical Chrono behavior, we check it earlier.

    parsed.set_year(try_consume!(scan::number(s, 4, 4)))?;
    s = scan::char(s, b'-')?;
    parsed.set_month(try_consume!(scan::number(s, 2, 2)))?;
    s = scan::char(s, b'-')?;
    parsed.set_day(try_consume!(scan::number(s, 2, 2)))?;

    s = match s.as_bytes().first() {
        Some(&b't') | Some(&b'T') => &s[1..],
        Some(_) => return Err(INVALID),
        None => return Err(TOO_SHORT),
    };

    parsed.set_hour(try_consume!(scan::number(s, 2, 2)))?;
    s = scan::char(s, b':')?;
    parsed.set_minute(try_consume!(scan::number(s, 2, 2)))?;
    s = scan::char(s, b':')?;
    parsed.set_second(try_consume!(scan::number(s, 2, 2)))?;
    if s.starts_with('.') {
        let nanosecond = try_consume!(scan::nanosecond(&s[1..]));
        parsed.set_nanosecond(nanosecond)?;
    }

    let offset = try_consume!(scan::timezone_offset_zulu(s, |s| scan::char(s, b':')));
    if offset <= -86_400 || offset >= 86_400 {
        return Err(OUT_OF_RANGE);
    }
    parsed.set_offset(i64::from(offset))?;

    Ok((s, ()))
}

/// Tries to parse given string into `parsed` with given formatting items.
/// Returns `Ok` when the entire string has been parsed (otherwise `parsed` should not be used).
/// There should be no trailing string after parsing;
/// use a stray [`Item::Space`](./enum.Item.html#variant.Space) to trim whitespaces.
///
/// This particular date and time parser is:
///
/// - Greedy. It will consume the longest possible prefix.
///   For example, `April` is always consumed entirely when the long month name is requested;
///   it equally accepts `Apr`, but prefers the longer prefix in this case.
///
/// - Padding-agnostic (for numeric items).
///   The [`Pad`](./enum.Pad.html) field is completely ignored,
///   so one can prepend any number of zeroes before numbers.
///
/// - (Still) obeying the intrinsic parsing width. This allows, for example, parsing `HHMMSS`.
pub fn parse<'a, I, B>(parsed: &mut Parsed, s: &str, items: I) -> ParseResult<()>
where
    I: Iterator<Item = B>,
    B: Borrow<Item<'a>>,
{
    parse_internal(parsed, s, items).map(|_| ()).map_err(|(_s, e)| e)
}

fn parse_internal<'a, 'b, I, B>(
    parsed: &mut Parsed,
    mut s: &'b str,
    items: I,
) -> Result<&'b str, (&'b str, ParseError)>
where
    I: Iterator<Item = B>,
    B: Borrow<Item<'a>>,
{
    macro_rules! try_consume {
        ($e:expr) => {{
            match $e {
                Ok((s_, v)) => {
                    s = s_;
                    v
                }
                Err(e) => return Err((s, e)),
            }
        }};
    }

    for item in items {
        match *item.borrow() {
            Item::Literal(prefix) => {
                if s.len() < prefix.len() {
                    return Err((s, TOO_SHORT));
                }
                if !s.starts_with(prefix) {
                    return Err((s, INVALID));
                }
                s = &s[prefix.len()..];
            }

            #[cfg(any(feature = "alloc", feature = "std", test))]
            Item::OwnedLiteral(ref prefix) => {
                if s.len() < prefix.len() {
                    return Err((s, TOO_SHORT));
                }
                if !s.starts_with(&prefix[..]) {
                    return Err((s, INVALID));
                }
                s = &s[prefix.len()..];
            }

            Item::Space(item_space) => {
                for expect in item_space.chars() {
                    let actual = match s.chars().next() {
                        Some(c) => c,
                        None => {
                            return Err((s, TOO_SHORT));
                        }
                    };
                    if expect != actual {
                        return Err((s, INVALID));
                    }
                    // advance `s` forward 1 char
                    s = scan::s_next(s);
                }
            }

            #[cfg(any(feature = "alloc", feature = "std", test))]
            Item::OwnedSpace(ref item_space) => {
                for expect in item_space.chars() {
                    let actual = match s.chars().next() {
                        Some(c) => c,
                        None => {
                            return Err((s, TOO_SHORT));
                        }
                    };
                    if expect != actual {
                        return Err((s, INVALID));
                    }
                    // advance `s` forward 1 char
                    s = scan::s_next(s);
                }
            }

            Item::Numeric(ref spec, ref _pad) => {
                use super::Numeric::*;
                type Setter = fn(&mut Parsed, i64) -> ParseResult<()>;

                let (width, signed, set): (usize, bool, Setter) = match *spec {
                    Year => (4, true, Parsed::set_year),
                    YearDiv100 => (2, false, Parsed::set_year_div_100),
                    YearMod100 => (2, false, Parsed::set_year_mod_100),
                    IsoYear => (4, true, Parsed::set_isoyear),
                    IsoYearDiv100 => (2, false, Parsed::set_isoyear_div_100),
                    IsoYearMod100 => (2, false, Parsed::set_isoyear_mod_100),
                    Month => (2, false, Parsed::set_month),
                    Day => (2, false, Parsed::set_day),
                    WeekFromSun => (2, false, Parsed::set_week_from_sun),
                    WeekFromMon => (2, false, Parsed::set_week_from_mon),
                    IsoWeek => (2, false, Parsed::set_isoweek),
                    NumDaysFromSun => (1, false, set_weekday_with_num_days_from_sunday),
                    WeekdayFromMon => (1, false, set_weekday_with_number_from_monday),
                    Ordinal => (3, false, Parsed::set_ordinal),
                    Hour => (2, false, Parsed::set_hour),
                    Hour12 => (2, false, Parsed::set_hour12),
                    Minute => (2, false, Parsed::set_minute),
                    Second => (2, false, Parsed::set_second),
                    Nanosecond => (9, false, Parsed::set_nanosecond),
                    Timestamp => (usize::MAX, false, Parsed::set_timestamp),

                    // for the future expansion
                    Internal(ref int) => match int._dummy {},
                };

                let v = if signed {
                    if s.starts_with('-') {
                        let v = try_consume!(scan::number(&s[1..], 1, usize::MAX));
                        0i64.checked_sub(v).ok_or((s, OUT_OF_RANGE))?
                    } else if s.starts_with('+') {
                        try_consume!(scan::number(&s[1..], 1, usize::MAX))
                    } else {
                        // if there is no explicit sign, we respect the original `width`
                        try_consume!(scan::number(s, 1, width))
                    }
                } else {
                    try_consume!(scan::number(s, 1, width))
                };
                set(parsed, v).map_err(|e| (s, e))?;
            }

            Item::Fixed(ref spec) => {
                use super::Fixed::*;

                match spec {
                    &ShortMonthName => {
                        let month0 = try_consume!(scan::short_month0(s));
                        parsed.set_month(i64::from(month0) + 1).map_err(|e| (s, e))?;
                    }

                    &LongMonthName => {
                        let month0 = try_consume!(scan::short_or_long_month0(s));
                        parsed.set_month(i64::from(month0) + 1).map_err(|e| (s, e))?;
                    }

                    &ShortWeekdayName => {
                        let weekday = try_consume!(scan::short_weekday(s));
                        parsed.set_weekday(weekday).map_err(|e| (s, e))?;
                    }

                    &LongWeekdayName => {
                        let weekday = try_consume!(scan::short_or_long_weekday(s));
                        parsed.set_weekday(weekday).map_err(|e| (s, e))?;
                    }

                    &LowerAmPm | &UpperAmPm => {
                        if s.len() < 2 {
                            return Err((s, TOO_SHORT));
                        }
                        let ampm = match (s.as_bytes()[0] | 32, s.as_bytes()[1] | 32) {
                            (b'a', b'm') => false,
                            (b'p', b'm') => true,
                            _ => return Err((s, INVALID)),
                        };
                        parsed.set_ampm(ampm).map_err(|e| (s, e))?;
                        s = &s[2..];
                    }

                    &Nanosecond | &Nanosecond3 | &Nanosecond6 | &Nanosecond9 => {
                        if s.starts_with('.') {
                            let nano = try_consume!(scan::nanosecond(&s[1..]));
                            parsed.set_nanosecond(nano).map_err(|e| (s, e))?;
                        }
                    }

                    &Internal(InternalFixed { val: InternalInternal::Nanosecond3NoDot }) => {
                        if s.len() < 3 {
                            return Err((s, TOO_SHORT));
                        }
                        let nano = try_consume!(scan::nanosecond_fixed(s, 3));
                        parsed.set_nanosecond(nano).map_err(|e| (s, e))?;
                    }

                    &Internal(InternalFixed { val: InternalInternal::Nanosecond6NoDot }) => {
                        if s.len() < 6 {
                            return Err((s, TOO_SHORT));
                        }
                        let nano = try_consume!(scan::nanosecond_fixed(s, 6));
                        parsed.set_nanosecond(nano).map_err(|e| (s, e))?;
                    }

                    &Internal(InternalFixed { val: InternalInternal::Nanosecond9NoDot }) => {
                        if s.len() < 9 {
                            return Err((s, TOO_SHORT));
                        }
                        let nano = try_consume!(scan::nanosecond_fixed(s, 9));
                        parsed.set_nanosecond(nano).map_err(|e| (s, e))?;
                    }

                    &TimezoneName => {
                        try_consume!(scan::timezone_name_skip(s));
                    }

                    &TimezoneOffsetColon
                    | &TimezoneOffsetDoubleColon
                    | &TimezoneOffsetTripleColon
                    | &TimezoneOffset => {
                        s = scan::trim1(s);
                        let offset =
                            try_consume!(scan::timezone_offset(s, scan::consume_colon_maybe));
                        parsed.set_offset(i64::from(offset)).map_err(|e| (s, e))?;
                    }

                    &TimezoneOffsetColonZ | &TimezoneOffsetZ => {
                        s = scan::trim1(s);
                        let offset =
                            try_consume!(scan::timezone_offset_zulu(s, scan::consume_colon_maybe));
                        parsed.set_offset(i64::from(offset)).map_err(|e| (s, e))?;
                    }

                    &Internal(InternalFixed {
                        val: InternalInternal::TimezoneOffsetPermissive,
                    }) => {
                        s = scan::trim1(s);
                        let offset = try_consume!(scan::timezone_offset_permissive(
                            s,
                            scan::consume_colon_maybe
                        ));
                        parsed.set_offset(i64::from(offset)).map_err(|e| (s, e))?;
                    }

                    &RFC2822 => try_consume!(parse_rfc2822(parsed, s)),
                    &RFC3339 => try_consume!(parse_rfc3339(parsed, s)),
                }
            }

            Item::Error => {
                return Err((s, BAD_FORMAT));
            }
        }
    }

    // if there are trailling chars, it is an error
    if !s.is_empty() {
        Err((s, TOO_LONG))
    } else {
        Ok(s)
    }
}

/// Accepts a relaxed form of RFC3339.
/// A space or a 'T' are accepted as the separator between the date and time
/// parts.
///
/// ```
/// # use chrono::{DateTime, offset::FixedOffset};
/// "2000-01-02T03:04:05Z".parse::<DateTime<FixedOffset>>();
/// "2000-01-02 03:04:05Z".parse::<DateTime<FixedOffset>>();
/// ```
impl str::FromStr for DateTime<FixedOffset> {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<DateTime<FixedOffset>> {
        const DATE_ITEMS: &[Item<'static>] = &[
            Item::Numeric(Numeric::Year, Pad::Zero),
            Item::Literal("-"),
            Item::Numeric(Numeric::Month, Pad::Zero),
            Item::Literal("-"),
            Item::Numeric(Numeric::Day, Pad::Zero),
        ];
        const TIME_ITEMS: &[Item<'static>] = &[
            Item::Numeric(Numeric::Hour, Pad::Zero),
            Item::Literal(":"),
            Item::Numeric(Numeric::Minute, Pad::Zero),
            Item::Literal(":"),
            Item::Numeric(Numeric::Second, Pad::Zero),
            Item::Fixed(Fixed::Nanosecond),
            Item::Fixed(Fixed::TimezoneOffsetZ),
        ];

        let mut parsed = Parsed::new();
        match parse_internal(&mut parsed, s, DATE_ITEMS.iter()) {
            Err((remainder, e)) if e.0 == ParseErrorKind::TooLong => {
                if remainder.starts_with('T') || remainder.starts_with(' ') {
                    parse(&mut parsed, &remainder[1..], TIME_ITEMS.iter())?;
                } else {
                    return Err(INVALID);
                }
            }
            Err((_s, e)) => return Err(e),
            Ok(_) => return Err(NOT_ENOUGH),
        };
        parsed.to_datetime()
    }
}

#[cfg(test)]
#[test]
fn test_parse() {
    use super::*;

    // workaround for Rust issue #22255
    fn parse_all(s: &str, items: &[Item]) -> ParseResult<Parsed> {
        let mut parsed = Parsed::new();
        parse(&mut parsed, s, items.iter())?;
        Ok(parsed)
    }

    macro_rules! check {
        ($fmt:expr, $items:expr; $err:tt) => (
            eprintln!("test_parse: format {:?}", $fmt);
            assert_eq!(parse_all($fmt, &$items), Err($err))
        );
        ($fmt:expr, $items:expr; $($k:ident: $v:expr),*) => ({
            eprintln!("test_parse: format {:?}", $fmt);
            let expected = Parsed {
                $($k: Some($v),)*
                ..Default::default()
            };
            assert_eq!(parse_all($fmt, &$items), Ok(expected))
        });
    }

    // empty string
    check!("",  []; );
    check!(" ", []; TOO_LONG);
    check!("a", []; TOO_LONG);
    check!("abc", []; TOO_LONG);
    check!("🤠", []; TOO_LONG);

    // whitespaces
    check!("",          [sp!("")]; );
    check!(" ",         [sp!(" ")]; );
    check!("  ",        [sp!("  ")]; );
    check!("   ",       [sp!("   ")]; );
    check!(" ",         [sp!("")]; TOO_LONG);
    check!("  ",        [sp!(" ")]; TOO_LONG);
    check!("   ",       [sp!("  ")]; TOO_LONG);
    check!("    ",      [sp!("  ")]; TOO_LONG);
    check!("",          [sp!(" ")]; TOO_SHORT);
    check!(" ",         [sp!("  ")]; TOO_SHORT);
    check!("  ",        [sp!("   ")]; TOO_SHORT);
    check!("  ",        [sp!("  "), sp!("  ")]; TOO_SHORT);
    check!("   ",       [sp!("  "), sp!("  ")]; TOO_SHORT);
    check!("  ",        [sp!(" "), sp!(" ")]; );
    check!("   ",       [sp!("  "), sp!(" ")]; );
    check!("   ",       [sp!(" "), sp!("  ")]; );
    check!("   ",       [sp!(" "), sp!(" "), sp!(" ")]; );
    check!("\t",        [sp!("")]; TOO_LONG);
    check!(" \n\r  \n", [sp!("")]; TOO_LONG);
    check!("\t",        [sp!("\t")]; );
    check!("\t",        [sp!(" ")]; INVALID);
    check!(" ",         [sp!("\t")]; INVALID);
    check!("\t\r",      [sp!("\t\r")]; );
    check!("\t\r ",     [sp!("\t\r ")]; );
    check!("\t \r",     [sp!("\t \r")]; );
    check!(" \t\r",     [sp!(" \t\r")]; );
    check!(" \n\r  \n", [sp!(" \n\r  \n")]; );
    check!(" \t\n",     [sp!(" \t")]; TOO_LONG);
    check!(" \n\t",     [sp!(" \t\n")]; INVALID);
    check!("\u{2002}",  [sp!("\u{2002}")]; );
    // most unicode whitespace characters
    check!(
        "\u{00A0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{3000}",
        [sp!("\u{00A0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{3000}")];
    );
    // most unicode whitespace characters
    check!(
        "\u{00A0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{3000}",
        [
            sp!("\u{00A0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}"),
            sp!("\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{3000}")
        ];
    );
    check!("a",         [sp!("")]; TOO_LONG);
    check!("a",         [sp!(" ")]; INVALID);
    // a Space containing a literal can match a literal, but this should not be done
    check!("a",         [sp!("a")]; );
    check!("abc",       [sp!("")]; TOO_LONG);
    check!("abc",       [sp!(" ")]; INVALID);
    check!(" abc",      [sp!("")]; TOO_LONG);
    check!(" abc",      [sp!(" ")]; TOO_LONG);

    // `\u{0363}` is combining diacritic mark "COMBINING LATIN SMALL LETTER A"

    // literal
    check!("",    [lit!("")]; );
    check!("",    [lit!("a")]; TOO_SHORT);
    check!(" ",   [lit!("a")]; INVALID);
    check!("a",   [lit!("a")]; );
    // a Literal may contain whitespace and match whitespace, but this should not be done
    check!(" ",   [lit!(" ")]; );
    check!("aa",  [lit!("a")]; TOO_LONG);
    check!("🤠",  [lit!("a")]; INVALID);
    check!("A",   [lit!("a")]; INVALID);
    check!("a",   [lit!("z")]; INVALID);
    check!("a",   [lit!("🤠")]; TOO_SHORT);
    check!("a",   [lit!("\u{0363}a")]; TOO_SHORT);
    check!("\u{0363}a", [lit!("a")]; INVALID);
    check!("\u{0363}a", [lit!("\u{0363}a")]; );
    check!("a",   [lit!("ab")]; TOO_SHORT);
    check!("xy",  [lit!("xy")]; );
    check!("xy",  [lit!("x"), lit!("y")]; );
    check!("1",   [lit!("1")]; );
    check!("1234", [lit!("1234")]; );
    check!("+1234", [lit!("+1234")]; );
    check!("PST", [lit!("PST")]; );
    check!("🤠",  [lit!("🤠")]; );
    check!("🤠a", [lit!("🤠"), lit!("a")]; );
    check!("🤠a🤠", [lit!("🤠"), lit!("a🤠")]; );
    check!("a🤠b", [lit!("a"), lit!("🤠"), lit!("b")]; );
    // literals can be together
    check!("xy",  [lit!("xy")]; );
    check!("xyz",  [lit!("xyz")]; );
    // or literals can be apart
    check!("xy",  [lit!("x"), lit!("y")]; );
    check!("xyz",  [lit!("x"), lit!("yz")]; );
    check!("xyz",  [lit!("xy"), lit!("z")]; );
    check!("xyz",  [lit!("x"), lit!("y"), lit!("z")]; );
    //
    check!("x y", [lit!("x"), lit!("y")]; INVALID);
    check!("xy",  [lit!("x"), sp!(""), lit!("y")]; );
    check!("x y", [lit!("x"), sp!(""), lit!("y")]; INVALID);
    check!("x y", [lit!("x"), sp!(" "), lit!("y")]; );

    // whitespaces + literals
    check!("a\n",         [lit!("a"), sp!("\n")]; );
    check!("\tab\n",      [sp!("\t"), lit!("ab"), sp!("\n")]; );
    check!("ab\tcd\ne",   [lit!("ab"), sp!("\t"), lit!("cd"), sp!("\n"), lit!("e")]; );
    check!("+1ab\tcd\r\n+,.", [lit!("+1ab"), sp!("\t"), lit!("cd"), sp!("\r\n"), lit!("+,.")]; );
    // whitespace and literals can be intermixed
    check!("a\tb",        [lit!("a\tb")]; );
    check!("a\tb",        [lit!("a"), sp!("\t"), lit!("b")]; );

    // numeric
    check!("1987",        [num!(Year)]; year: 1987);
    check!("1987 ",       [num!(Year)]; TOO_LONG);
    check!("0x12",        [num!(Year)]; TOO_LONG); // `0` is parsed
    check!("x123",        [num!(Year)]; INVALID);
    check!("o123",        [num!(Year)]; INVALID);
    check!("2015",        [num!(Year)]; year: 2015);
    check!("0000",        [num!(Year)]; year:    0);
    check!("9999",        [num!(Year)]; year: 9999);
    check!(" \t987",      [num!(Year)]; INVALID);
    check!(" \t987",      [sp!(" \t"), num!(Year)]; year:  987);
    check!(" \t987🤠",    [sp!(" \t"), num!(Year), lit!("🤠")]; year:  987);
    check!("987🤠",       [num!(Year), lit!("🤠")]; year:  987);
    check!("5",           [num!(Year)]; year:    5);
    check!("5\0",         [num!(Year)]; TOO_LONG);
    check!("\x005",       [num!(Year)]; INVALID);
    check!("",            [num!(Year)]; TOO_SHORT);
    check!("12345",       [num!(Year), lit!("5")]; year: 1234);
    check!("12345",       [nums!(Year), lit!("5")]; year: 1234);
    check!("12345",       [num0!(Year), lit!("5")]; year: 1234);
    check!("12341234",    [num!(Year), num!(Year)]; year: 1234);
    check!("1234 1234",   [num!(Year), num!(Year)]; INVALID);
    check!("1234 1234",   [num!(Year), sp!(" "), num!(Year)]; year: 1234);
    check!("1234 1235",   [num!(Year), num!(Year)]; INVALID);
    check!("1234 1234",   [num!(Year), lit!("x"), num!(Year)]; INVALID);
    check!("1234x1234",   [num!(Year), lit!("x"), num!(Year)]; year: 1234);
    check!("1234 x 1234", [num!(Year), lit!("x"), num!(Year)]; INVALID);
    check!("1234xx1234",  [num!(Year), lit!("x"), num!(Year)]; INVALID);
    check!("1234xx1234",  [num!(Year), lit!("xx"), num!(Year)]; year: 1234);
    check!("1234 x 1234", [num!(Year), sp!(" "), lit!("x"), sp!(" "), num!(Year)]; year: 1234);
    check!("1234 x 1235", [num!(Year), sp!(" "), lit!("x"), sp!(" "), lit!("1235")]; year: 1234);

    // signed numeric
    check!("-42",         [num!(Year)]; year: -42);
    check!("+42",         [num!(Year)]; year: 42);
    check!("-0042",       [num!(Year)]; year: -42);
    check!("+0042",       [num!(Year)]; year: 42);
    check!("-42195",      [num!(Year)]; year: -42195);
    check!("+42195",      [num!(Year)]; year: 42195);
    check!(" -42195",     [num!(Year)]; INVALID);
    check!(" +42195",     [num!(Year)]; INVALID);
    check!("  -42195",    [num!(Year)]; INVALID);
    check!("  +42195",    [num!(Year)]; INVALID);
    check!("-42195 ",     [num!(Year)]; TOO_LONG);
    check!("+42195 ",     [num!(Year)]; TOO_LONG);
    check!("  -   42",    [num!(Year)]; INVALID);
    check!("  +   42",    [num!(Year)]; INVALID);
    check!("  -42195",    [sp!("  "), num!(Year)]; year: -42195);
    check!("  +42195",    [sp!("  "), num!(Year)]; year: 42195);
    check!("  -   42",    [sp!("  "), num!(Year)]; INVALID);
    check!("  +   42",    [sp!("  "), num!(Year)]; INVALID);
    check!("-",           [num!(Year)]; TOO_SHORT);
    check!("+",           [num!(Year)]; TOO_SHORT);

    // unsigned numeric
    check!("345",   [num!(Ordinal)]; ordinal: 345);
    check!("+345",  [num!(Ordinal)]; INVALID);
    check!("-345",  [num!(Ordinal)]; INVALID);
    check!(" 345",  [num!(Ordinal)]; INVALID);
    check!("345 ",  [num!(Ordinal)]; TOO_LONG);
    check!(" 345",  [sp!(" "), num!(Ordinal)]; ordinal: 345);
    check!("345 ",  [num!(Ordinal), sp!(" ")]; ordinal: 345);
    check!("345🤠 ", [num!(Ordinal), lit!("🤠"), sp!(" ")]; ordinal: 345);
    check!("345🤠", [num!(Ordinal)]; TOO_LONG);
    check!("\u{0363}345", [num!(Ordinal)]; INVALID);
    check!(" +345", [num!(Ordinal)]; INVALID);
    check!(" -345", [num!(Ordinal)]; INVALID);
    check!("\t345", [sp!("\t"), num!(Ordinal)]; ordinal: 345);
    check!(" +345", [sp!(" "), num!(Ordinal)]; INVALID);
    check!(" -345", [sp!(" "), num!(Ordinal)]; INVALID);

    // various numeric fields
    check!("1234 5678", [num!(Year), num!(IsoYear)]; INVALID);
    check!("1234 5678",
           [num!(Year), sp!(" "), num!(IsoYear)];
           year: 1234, isoyear: 5678);
    check!("12 34 56 78",
           [num!(YearDiv100), num!(YearMod100), num!(IsoYearDiv100), num!(IsoYearMod100)];
           INVALID);
    check!("12 34🤠56 78",
           [num!(YearDiv100), sp!(" "), num!(YearMod100),
           lit!("🤠"), num!(IsoYearDiv100), sp!(" "), num!(IsoYearMod100)];
           year_div_100: 12, year_mod_100: 34, isoyear_div_100: 56, isoyear_mod_100: 78);
    check!("1 2 3 4 5 6",
           [num!(Month), sp!(" "), num!(Day), sp!(" "), num!(WeekFromSun), sp!(" "),
           num!(WeekFromMon), sp!(" "), num!(IsoWeek), sp!(" "), num!(NumDaysFromSun)];
           month: 1, day: 2, week_from_sun: 3, week_from_mon: 4, isoweek: 5, weekday: Weekday::Sat);
    check!("7 89 01",
           [num!(WeekdayFromMon), sp!(" "), num!(Ordinal), sp!(" "), num!(Hour12)];
           weekday: Weekday::Sun, ordinal: 89, hour_mod_12: 1);
    check!("23 45 6 78901234 567890123",
           [num!(Hour), sp!(" "), num!(Minute), sp!(" "), num!(Second), sp!(" "),
           num!(Nanosecond), sp!(" "), num!(Timestamp)];
           hour_div_12: 1, hour_mod_12: 11, minute: 45, second: 6, nanosecond: 78_901_234,
           timestamp: 567_890_123);

    // fixed: month and weekday names
    check!("apr",       [fix!(ShortMonthName)]; month: 4);
    check!("Apr",       [fix!(ShortMonthName)]; month: 4);
    check!("APR",       [fix!(ShortMonthName)]; month: 4);
    check!("ApR",       [fix!(ShortMonthName)]; month: 4);
    check!("\u{0363}APR", [fix!(ShortMonthName)]; INVALID);
    check!("April",     [fix!(ShortMonthName)]; TOO_LONG); // `Apr` is parsed
    check!("A",         [fix!(ShortMonthName)]; TOO_SHORT);
    check!("Sol",       [fix!(ShortMonthName)]; INVALID);
    check!("Apr",       [fix!(LongMonthName)]; month: 4);
    check!("Apri",      [fix!(LongMonthName)]; TOO_LONG); // `Apr` is parsed
    check!("April",     [fix!(LongMonthName)]; month: 4);
    check!("Aprill",    [fix!(LongMonthName)]; TOO_LONG);
    check!("Aprill",    [fix!(LongMonthName), lit!("l")]; month: 4);
    check!("Aprl",      [fix!(LongMonthName), lit!("l")]; month: 4);
    check!("April",     [fix!(LongMonthName), lit!("il")]; TOO_SHORT); // do not backtrack
    check!("thu",       [fix!(ShortWeekdayName)]; weekday: Weekday::Thu);
    check!("Thu",       [fix!(ShortWeekdayName)]; weekday: Weekday::Thu);
    check!("THU",       [fix!(ShortWeekdayName)]; weekday: Weekday::Thu);
    check!("tHu",       [fix!(ShortWeekdayName)]; weekday: Weekday::Thu);
    check!("Thursday",  [fix!(ShortWeekdayName)]; TOO_LONG); // `Thu` is parsed
    check!("T",         [fix!(ShortWeekdayName)]; TOO_SHORT);
    check!("The",       [fix!(ShortWeekdayName)]; INVALID);
    check!("Nop",       [fix!(ShortWeekdayName)]; INVALID);
    check!("Thu",       [fix!(LongWeekdayName)]; weekday: Weekday::Thu);
    check!("Thur",      [fix!(LongWeekdayName)]; TOO_LONG); // `Thu` is parsed
    check!("Thurs",     [fix!(LongWeekdayName)]; TOO_LONG); // ditto
    check!("Thursday",  [fix!(LongWeekdayName)]; weekday: Weekday::Thu);
    check!("Thursdays", [fix!(LongWeekdayName)]; TOO_LONG);
    check!("Thursdays", [fix!(LongWeekdayName), lit!("s")]; weekday: Weekday::Thu);
    check!("Thus",      [fix!(LongWeekdayName), lit!("s")]; weekday: Weekday::Thu);
    check!("Thursday",  [fix!(LongWeekdayName), lit!("rsday")]; TOO_SHORT); // do not backtrack

    // fixed: am/pm
    check!("am",  [fix!(LowerAmPm)]; hour_div_12: 0);
    check!("pm",  [fix!(LowerAmPm)]; hour_div_12: 1);
    check!("AM",  [fix!(LowerAmPm)]; hour_div_12: 0);
    check!("PM",  [fix!(LowerAmPm)]; hour_div_12: 1);
    check!("am",  [fix!(UpperAmPm)]; hour_div_12: 0);
    check!("pm",  [fix!(UpperAmPm)]; hour_div_12: 1);
    check!("AM",  [fix!(UpperAmPm)]; hour_div_12: 0);
    check!("PM",  [fix!(UpperAmPm)]; hour_div_12: 1);
    check!("Am",  [fix!(LowerAmPm)]; hour_div_12: 0);
    check!(" Am", [sp!(" "), fix!(LowerAmPm)]; hour_div_12: 0);
    check!("Am🤠", [fix!(LowerAmPm), lit!("🤠")]; hour_div_12: 0);
    check!("🤠Am", [lit!("🤠"), fix!(LowerAmPm)]; hour_div_12: 0);
    check!("\u{0363}am", [fix!(LowerAmPm)]; INVALID);
    check!("\u{0360}am", [fix!(LowerAmPm)]; INVALID);
    check!(" Am", [fix!(LowerAmPm)]; INVALID);
    check!("Am ", [fix!(LowerAmPm)]; TOO_LONG);
    check!("a.m.", [fix!(LowerAmPm)]; INVALID);
    check!("A.M.", [fix!(LowerAmPm)]; INVALID);
    check!("ame", [fix!(LowerAmPm)]; TOO_LONG); // `am` is parsed
    check!("a",   [fix!(LowerAmPm)]; TOO_SHORT);
    check!("p",   [fix!(LowerAmPm)]; TOO_SHORT);
    check!("x",   [fix!(LowerAmPm)]; TOO_SHORT);
    check!("xx",  [fix!(LowerAmPm)]; INVALID);
    check!("",    [fix!(LowerAmPm)]; TOO_SHORT);

    // fixed: dot plus nanoseconds
    check!("",              [fix!(Nanosecond)]; ); // no field set, but not an error
    check!("4",             [fix!(Nanosecond)]; TOO_LONG); // never consumes `4`
    check!("4",             [fix!(Nanosecond), num!(Second)]; second: 4);
    check!(".0",            [fix!(Nanosecond)]; nanosecond: 0);
    check!(".4",            [fix!(Nanosecond)]; nanosecond: 400_000_000);
    check!(".42",           [fix!(Nanosecond)]; nanosecond: 420_000_000);
    check!(".421",          [fix!(Nanosecond)]; nanosecond: 421_000_000);
    check!(".42195",        [fix!(Nanosecond)]; nanosecond: 421_950_000);
    check!(".421951",       [fix!(Nanosecond)]; nanosecond: 421_951_000);
    check!(".4219512",      [fix!(Nanosecond)]; nanosecond: 421_951_200);
    check!(".42195123",     [fix!(Nanosecond)]; nanosecond: 421_951_230);
    check!(".421950803",    [fix!(Nanosecond)]; nanosecond: 421_950_803);
    check!(".4219508035",   [fix!(Nanosecond)]; nanosecond: 421_950_803);
    check!(".42195080354",  [fix!(Nanosecond)]; nanosecond: 421_950_803);
    check!(".421950803547", [fix!(Nanosecond)]; nanosecond: 421_950_803);
    check!(".000000003",    [fix!(Nanosecond)]; nanosecond: 3);
    check!(".0000000031",   [fix!(Nanosecond)]; nanosecond: 3);
    check!(".0000000035",   [fix!(Nanosecond)]; nanosecond: 3);
    check!(".000000003547", [fix!(Nanosecond)]; nanosecond: 3);
    check!(".0000000009",   [fix!(Nanosecond)]; nanosecond: 0);
    check!(".000000000547", [fix!(Nanosecond)]; nanosecond: 0);
    check!(".0000000009999999999999999999999999", [fix!(Nanosecond)]; nanosecond: 0);
    check!(".4🤠",          [fix!(Nanosecond), lit!("🤠")]; nanosecond: 400_000_000);
    check!(".",             [fix!(Nanosecond)]; TOO_SHORT);
    check!(".4x",           [fix!(Nanosecond)]; TOO_LONG);
    check!(".  4",          [fix!(Nanosecond)]; INVALID);
    check!("  .4",          [fix!(Nanosecond)]; TOO_LONG); // no automatic trimming

    // fixed: nanoseconds without the dot
    check!("",             [internal_fix!(Nanosecond3NoDot)]; TOO_SHORT);
    check!("0",            [internal_fix!(Nanosecond3NoDot)]; TOO_SHORT);
    check!("4",            [internal_fix!(Nanosecond3NoDot)]; TOO_SHORT);
    check!("42",           [internal_fix!(Nanosecond3NoDot)]; TOO_SHORT);
    check!("421",          [internal_fix!(Nanosecond3NoDot)]; nanosecond: 421_000_000);
    check!("4210",         [internal_fix!(Nanosecond3NoDot)]; TOO_LONG);
    check!("42143",        [internal_fix!(Nanosecond3NoDot), num!(Second)]; nanosecond: 421_000_000, second: 43);
    check!("421🤠",        [internal_fix!(Nanosecond3NoDot), lit!("🤠")]; nanosecond: 421_000_000);
    check!("🤠421",        [lit!("🤠"), internal_fix!(Nanosecond3NoDot)]; nanosecond: 421_000_000);
    check!("42195",        [internal_fix!(Nanosecond3NoDot)]; TOO_LONG);
    check!("123456789",    [internal_fix!(Nanosecond3NoDot)]; TOO_LONG);
    check!("4x",           [internal_fix!(Nanosecond3NoDot)]; TOO_SHORT);
    check!("  4",          [internal_fix!(Nanosecond3NoDot)]; INVALID);
    check!(".421",         [internal_fix!(Nanosecond3NoDot)]; INVALID);

    check!("",             [internal_fix!(Nanosecond6NoDot)]; TOO_SHORT);
    check!("0",            [internal_fix!(Nanosecond6NoDot)]; TOO_SHORT);
    check!("1234",         [internal_fix!(Nanosecond6NoDot)]; TOO_SHORT);
    check!("12345",        [internal_fix!(Nanosecond6NoDot)]; TOO_SHORT);
    check!("421950",       [internal_fix!(Nanosecond6NoDot)]; nanosecond: 421_950_000);
    check!("000003",       [internal_fix!(Nanosecond6NoDot)]; nanosecond: 3000);
    check!("000000",       [internal_fix!(Nanosecond6NoDot)]; nanosecond: 0);
    check!("1234567",      [internal_fix!(Nanosecond6NoDot)]; TOO_LONG);
    check!("123456789",    [internal_fix!(Nanosecond6NoDot)]; TOO_LONG);
    check!("4x",           [internal_fix!(Nanosecond6NoDot)]; TOO_SHORT);
    check!("     4",       [internal_fix!(Nanosecond6NoDot)]; INVALID);
    check!(".42100",       [internal_fix!(Nanosecond6NoDot)]; INVALID);

    check!("",             [internal_fix!(Nanosecond9NoDot)]; TOO_SHORT);
    check!("42195",        [internal_fix!(Nanosecond9NoDot)]; TOO_SHORT);
    check!("12345678",     [internal_fix!(Nanosecond9NoDot)]; TOO_SHORT);
    check!("421950803",    [internal_fix!(Nanosecond9NoDot)]; nanosecond: 421_950_803);
    check!("000000003",    [internal_fix!(Nanosecond9NoDot)]; nanosecond: 3);
    check!("42195080354",  [internal_fix!(Nanosecond9NoDot), num!(Second)]; nanosecond: 421_950_803, second: 54); // don't skip digits that come after the 9
    check!("1234567890",   [internal_fix!(Nanosecond9NoDot)]; TOO_LONG);
    check!("000000000",    [internal_fix!(Nanosecond9NoDot)]; nanosecond: 0);
    check!("00000000x",    [internal_fix!(Nanosecond9NoDot)]; INVALID);
    check!("        4",    [internal_fix!(Nanosecond9NoDot)]; INVALID);
    check!(".42100000",    [internal_fix!(Nanosecond9NoDot)]; INVALID);

    // fixed: timezone offsets

    // TimezoneOffset
    check!("1",            [fix!(TimezoneOffset)]; INVALID);
    check!("12",           [fix!(TimezoneOffset)]; INVALID);
    check!("123",          [fix!(TimezoneOffset)]; INVALID);
    check!("1234",         [fix!(TimezoneOffset)]; INVALID);
    check!("12345",        [fix!(TimezoneOffset)]; INVALID);
    check!("123456",       [fix!(TimezoneOffset)]; INVALID);
    check!("1234567",      [fix!(TimezoneOffset)]; INVALID);
    check!("+1",           [fix!(TimezoneOffset)]; TOO_SHORT);
    check!("+12",          [fix!(TimezoneOffset)]; TOO_SHORT);
    check!("+123",         [fix!(TimezoneOffset)]; TOO_SHORT);
    check!("+1234",        [fix!(TimezoneOffset)]; offset: 45_240);
    check!("+12345",       [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+123456",      [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+1234567",     [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12345678",    [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12:",         [fix!(TimezoneOffset)]; TOO_SHORT);
    check!("+12:3",        [fix!(TimezoneOffset)]; TOO_SHORT);
    check!("+12:34",       [fix!(TimezoneOffset)]; offset: 45_240);
    check!("-12:34",       [fix!(TimezoneOffset)]; offset: -45_240);
    check!("+12:34:",      [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12:34:5",     [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12:34:56",    [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12:34:56:",   [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12 34",       [fix!(TimezoneOffset)]; INVALID);
    check!("+12  34",      [fix!(TimezoneOffset)]; INVALID);
    check!("12:34",        [fix!(TimezoneOffset)]; INVALID);
    check!("12:34:56",     [fix!(TimezoneOffset)]; INVALID);
    check!("+12::34",      [fix!(TimezoneOffset)]; INVALID);
    check!("+12: :34",     [fix!(TimezoneOffset)]; INVALID);
    check!("+12:::34",     [fix!(TimezoneOffset)]; INVALID);
    check!("+12::::34",    [fix!(TimezoneOffset)]; INVALID);
    check!("+12::34",      [fix!(TimezoneOffset)]; INVALID);
    check!("+12:34:56",    [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12:3456",     [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+1234:56",     [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+1234:567",    [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+00:00",       [fix!(TimezoneOffset)]; offset: 0);
    check!("-00:00",       [fix!(TimezoneOffset)]; offset: 0);
    check!("+00:01",       [fix!(TimezoneOffset)]; offset: 60);
    check!("-00:01",       [fix!(TimezoneOffset)]; offset: -60);
    check!("+00:30",       [fix!(TimezoneOffset)]; offset: 1_800);
    check!("-00:30",       [fix!(TimezoneOffset)]; offset: -1_800);
    check!("+24:00",       [fix!(TimezoneOffset)]; offset: 86_400);
    check!("-24:00",       [fix!(TimezoneOffset)]; offset: -86_400);
    check!("+99:59",       [fix!(TimezoneOffset)]; offset: 359_940);
    check!("-99:59",       [fix!(TimezoneOffset)]; offset: -359_940);
    check!("+00:60",       [fix!(TimezoneOffset)]; OUT_OF_RANGE);
    check!("+00:99",       [fix!(TimezoneOffset)]; OUT_OF_RANGE);
    check!("#12:34",       [fix!(TimezoneOffset)]; INVALID);
    check!("+12:34 ",      [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12 34 ",      [fix!(TimezoneOffset)]; INVALID);
    check!(" +12:34",      [fix!(TimezoneOffset)]; offset: 45_240);
    check!(" -12:34",      [fix!(TimezoneOffset)]; offset: -45_240);
    check!("  +12:34",     [fix!(TimezoneOffset)]; INVALID);
    check!("  -12:34",     [fix!(TimezoneOffset)]; INVALID);
    check!("\t -12:34",    [fix!(TimezoneOffset)]; INVALID);
    check!("-12: 34",      [fix!(TimezoneOffset)]; INVALID);
    check!("-12 :34",      [fix!(TimezoneOffset)]; INVALID);
    check!("-12 : 34",     [fix!(TimezoneOffset)]; INVALID);
    check!("-12 :  34",    [fix!(TimezoneOffset)]; INVALID);
    check!("-12  : 34",    [fix!(TimezoneOffset)]; INVALID);
    check!("-12:  34",     [fix!(TimezoneOffset)]; INVALID);
    check!("-12  :34",     [fix!(TimezoneOffset)]; INVALID);
    check!("-12  :  34",   [fix!(TimezoneOffset)]; INVALID);
    check!("12:34 ",       [fix!(TimezoneOffset)]; INVALID);
    check!(" 12:34",       [fix!(TimezoneOffset)]; INVALID);
    check!("",             [fix!(TimezoneOffset)]; TOO_SHORT);
    check!("+",            [fix!(TimezoneOffset)]; TOO_SHORT);
    check!("+12345",       [fix!(TimezoneOffset), num!(Day)]; offset: 45_240, day: 5);
    check!("+12:345",      [fix!(TimezoneOffset), num!(Day)]; offset: 45_240, day: 5);
    check!("+12:34:",      [fix!(TimezoneOffset), lit!(":")]; offset: 45_240);
    check!("Z12:34",       [fix!(TimezoneOffset)]; INVALID);
    check!("X12:34",       [fix!(TimezoneOffset)]; INVALID);
    check!("Z+12:34",      [fix!(TimezoneOffset)]; INVALID);
    check!("X+12:34",      [fix!(TimezoneOffset)]; INVALID);
    check!("🤠+12:34",     [fix!(TimezoneOffset)]; INVALID);
    check!("+12:34🤠",     [fix!(TimezoneOffset)]; TOO_LONG);
    check!("+12:🤠34",     [fix!(TimezoneOffset)]; INVALID);
    check!("+12:34🤠",     [fix!(TimezoneOffset), lit!("🤠")]; offset: 45_240);
    check!("🤠+12:34",     [lit!("🤠"), fix!(TimezoneOffset)]; offset: 45_240);
    check!("Z",            [fix!(TimezoneOffset)]; INVALID);
    check!("A",            [fix!(TimezoneOffset)]; INVALID);
    check!("PST",          [fix!(TimezoneOffset)]; INVALID);
    check!("#Z",           [fix!(TimezoneOffset)]; INVALID);
    check!(":Z",           [fix!(TimezoneOffset)]; INVALID);
    check!("+Z",           [fix!(TimezoneOffset)]; TOO_SHORT);
    check!("+:Z",          [fix!(TimezoneOffset)]; INVALID);
    check!("+Z:",          [fix!(TimezoneOffset)]; INVALID);
    check!("z",            [fix!(TimezoneOffset)]; INVALID);
    check!(" :Z",          [fix!(TimezoneOffset)]; INVALID);
    check!(" Z",           [fix!(TimezoneOffset)]; INVALID);
    check!(" z",           [fix!(TimezoneOffset)]; INVALID);

    // TimezoneOffsetColon
    check!("1",            [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12",           [fix!(TimezoneOffsetColon)]; INVALID);
    check!("123",          [fix!(TimezoneOffsetColon)]; INVALID);
    check!("1234",         [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12345",        [fix!(TimezoneOffsetColon)]; INVALID);
    check!("123456",       [fix!(TimezoneOffsetColon)]; INVALID);
    check!("1234567",      [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12345678",     [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+1",           [fix!(TimezoneOffsetColon)]; TOO_SHORT);
    check!("+12",          [fix!(TimezoneOffsetColon)]; TOO_SHORT);
    check!("+123",         [fix!(TimezoneOffsetColon)]; TOO_SHORT);
    check!("+1234",        [fix!(TimezoneOffsetColon)]; offset: 45_240);
    check!("-1234",        [fix!(TimezoneOffsetColon)]; offset: -45_240);
    check!("+12345",       [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+123456",      [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+1234567",     [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+12345678",    [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("1:",           [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12:",          [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12:3",         [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12:34",        [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12:34:",       [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12:34:5",      [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12:34:56",     [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+1:",          [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12:",         [fix!(TimezoneOffsetColon)]; TOO_SHORT);
    check!("+12:3",        [fix!(TimezoneOffsetColon)]; TOO_SHORT);
    check!("+12:34",       [fix!(TimezoneOffsetColon)]; offset: 45_240);
    check!("-12:34",       [fix!(TimezoneOffsetColon)]; offset: -45_240);
    check!("+12:34:",      [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+12:34:5",     [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+12:34:56",    [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+12:34:56:",   [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+12:34:56:7",  [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+12:34:56:78", [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+12:3456",     [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+1234:56",     [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!("+12 34",       [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12: 34",      [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12 :34",      [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12 : 34",     [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12  : 34",    [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12 :  34",    [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12  :  34",   [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12::34",      [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12: :34",     [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12:::34",     [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12::::34",    [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12::34",      [fix!(TimezoneOffsetColon)]; INVALID);
    check!("#1234",        [fix!(TimezoneOffsetColon)]; INVALID);
    check!("#12:34",       [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12:34 ",      [fix!(TimezoneOffsetColon)]; TOO_LONG);
    check!(" +12:34",      [fix!(TimezoneOffsetColon)]; offset: 45_240);
    check!("\t+12:34",     [fix!(TimezoneOffsetColon)]; offset: 45_240);
    check!("\t\t+12:34",   [fix!(TimezoneOffsetColon)]; INVALID);
    check!("12:34 ",       [fix!(TimezoneOffsetColon)]; INVALID);
    check!(" 12:34",       [fix!(TimezoneOffsetColon)]; INVALID);
    check!("",             [fix!(TimezoneOffsetColon)]; TOO_SHORT);
    check!("+",            [fix!(TimezoneOffsetColon)]; TOO_SHORT);
    check!(":",            [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+12345",       [fix!(TimezoneOffsetColon), num!(Day)]; offset: 45_240, day: 5);
    check!("+12:345",      [fix!(TimezoneOffsetColon), num!(Day)]; offset: 45_240, day: 5);
    check!("+12:34:",      [fix!(TimezoneOffsetColon), lit!(":")]; offset: 45_240);
    check!("Z",            [fix!(TimezoneOffsetColon)]; INVALID);
    check!("A",            [fix!(TimezoneOffsetColon)]; INVALID);
    check!("PST",          [fix!(TimezoneOffsetColon)]; INVALID);
    check!("#Z",           [fix!(TimezoneOffsetColon)]; INVALID);
    check!(":Z",           [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+Z",           [fix!(TimezoneOffsetColon)]; TOO_SHORT);
    check!("+:Z",          [fix!(TimezoneOffsetColon)]; INVALID);
    check!("+Z:",          [fix!(TimezoneOffsetColon)]; INVALID);
    check!("z",            [fix!(TimezoneOffsetColon)]; INVALID);
    check!(" :Z",          [fix!(TimezoneOffsetColon)]; INVALID);
    check!(" Z",           [fix!(TimezoneOffsetColon)]; INVALID);
    check!(" z",           [fix!(TimezoneOffsetColon)]; INVALID);
    // testing `TimezoneOffsetColon` also tests same path as `TimezoneOffsetDoubleColon`
    // and `TimezoneOffsetTripleColon` for function `parse_internal`.
    // No need for separate tests for `TimezoneOffsetDoubleColon` and
    // `TimezoneOffsetTripleColon`.

    // TimezoneOffsetZ
    check!("1",            [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12",           [fix!(TimezoneOffsetZ)]; INVALID);
    check!("123",          [fix!(TimezoneOffsetZ)]; INVALID);
    check!("1234",         [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12345",        [fix!(TimezoneOffsetZ)]; INVALID);
    check!("123456",       [fix!(TimezoneOffsetZ)]; INVALID);
    check!("1234567",      [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12345678",     [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+1",           [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("+12",          [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("+123",         [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("+1234",        [fix!(TimezoneOffsetZ)]; offset: 45_240);
    check!("-1234",        [fix!(TimezoneOffsetZ)]; offset: -45_240);
    check!("+12345",       [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+123456",      [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+1234567",     [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12345678",    [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("1:",           [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12:",          [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12:3",         [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12:34",        [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12:34:",       [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12:34:5",      [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12:34:56",     [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+1:",          [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12:",         [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("+12:3",        [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("+12:34",       [fix!(TimezoneOffsetZ)]; offset: 45_240);
    check!("-12:34",       [fix!(TimezoneOffsetZ)]; offset: -45_240);
    check!("+12:34:",      [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12:34:5",     [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12:34:56",    [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12:34:56:",   [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12:34:56:7",  [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12:34:56:78", [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12::34",      [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12:3456",     [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+1234:56",     [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12 34",       [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12  34",      [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12: 34",      [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12 :34",      [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12 : 34",     [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12  : 34",    [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12 :  34",    [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12  :  34",   [fix!(TimezoneOffsetZ)]; INVALID);
    check!("12:34 ",       [fix!(TimezoneOffsetZ)]; INVALID);
    check!(" 12:34",       [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+12:34 ",      [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("+12 34 ",      [fix!(TimezoneOffsetZ)]; INVALID);
    check!(" +12:34",      [fix!(TimezoneOffsetZ)]; offset: 45_240);
    check!("+12345",       [fix!(TimezoneOffsetZ), num!(Day)]; offset: 45_240, day: 5);
    check!("+12:345",      [fix!(TimezoneOffsetZ), num!(Day)]; offset: 45_240, day: 5);
    check!("+12:34:",      [fix!(TimezoneOffsetZ), lit!(":")]; offset: 45_240);
    check!("Z12:34",       [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("X12:34",       [fix!(TimezoneOffsetZ)]; INVALID);
    check!("Z",            [fix!(TimezoneOffsetZ)]; offset: 0);
    check!("z",            [fix!(TimezoneOffsetZ)]; offset: 0);
    check!(" Z",           [fix!(TimezoneOffsetZ)]; offset: 0);
    check!(" z",           [fix!(TimezoneOffsetZ)]; offset: 0);
    check!("\u{0363}Z",    [fix!(TimezoneOffsetZ)]; INVALID);
    check!("Z ",           [fix!(TimezoneOffsetZ)]; TOO_LONG);
    check!("A",            [fix!(TimezoneOffsetZ)]; INVALID);
    check!("PST",          [fix!(TimezoneOffsetZ)]; INVALID);
    check!("#Z",           [fix!(TimezoneOffsetZ)]; INVALID);
    check!(":Z",           [fix!(TimezoneOffsetZ)]; INVALID);
    check!(":z",           [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+Z",           [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("-Z",           [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("+A",           [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("+🙃",          [fix!(TimezoneOffsetZ)]; INVALID);
    check!("+Z:",          [fix!(TimezoneOffsetZ)]; INVALID);
    check!(" :Z",          [fix!(TimezoneOffsetZ)]; INVALID);
    check!(" +Z",          [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!(" -Z",          [fix!(TimezoneOffsetZ)]; TOO_SHORT);
    check!("+:Z",          [fix!(TimezoneOffsetZ)]; INVALID);
    check!("Y",            [fix!(TimezoneOffsetZ)]; INVALID);
    check!("Zulu",         [fix!(TimezoneOffsetZ), lit!("ulu")]; offset: 0);
    check!("zulu",         [fix!(TimezoneOffsetZ), lit!("ulu")]; offset: 0);
    check!("+1234ulu",     [fix!(TimezoneOffsetZ), lit!("ulu")]; offset: 45_240);
    check!("+12:34ulu",    [fix!(TimezoneOffsetZ), lit!("ulu")]; offset: 45_240);
    // Testing `TimezoneOffsetZ` also tests same path as `TimezoneOffsetColonZ`
    // in function `parse_internal`.
    // No need for separate tests for `TimezoneOffsetColonZ`.

    // TimezoneOffsetPermissive
    check!("1",            [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12",           [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("123",          [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("1234",         [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12345",        [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("123456",       [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("1234567",      [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12345678",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+1",           [internal_fix!(TimezoneOffsetPermissive)]; TOO_SHORT);
    check!("+12",          [internal_fix!(TimezoneOffsetPermissive)]; offset: 43_200);
    check!("+123",         [internal_fix!(TimezoneOffsetPermissive)]; TOO_SHORT);
    check!("+1234",        [internal_fix!(TimezoneOffsetPermissive)]; offset: 45_240);
    check!("-1234",        [internal_fix!(TimezoneOffsetPermissive)]; offset: -45_240);
    check!("+12345",       [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+123456",      [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+1234567",     [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+12345678",    [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("1:",           [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12:",          [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12:3",         [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12:34",        [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12:34:",       [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12:34:5",      [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12:34:56",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+1:",          [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12:",         [internal_fix!(TimezoneOffsetPermissive)]; offset: 43_200);
    check!("+12:3",        [internal_fix!(TimezoneOffsetPermissive)]; TOO_SHORT);
    check!("+12:34",       [internal_fix!(TimezoneOffsetPermissive)]; offset: 45_240);
    check!("-12:34",       [internal_fix!(TimezoneOffsetPermissive)]; offset: -45_240);
    check!("+12:34:",      [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+12:34:5",     [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+12:34:56",    [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+12:34:56:",   [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+12:34:56:7",  [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+12:34:56:78", [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+12 34",       [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12  34",      [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12 :34",      [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12: 34",      [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12 : 34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12  :34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12:  34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12  :  34",   [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12::34",      [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12 ::34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12: :34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12:: 34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12  ::34",    [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12:  :34",    [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12::  34",    [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12:::34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12::::34",    [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("12:34 ",       [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!(" 12:34",       [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12:34 ",      [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!(" +12:34",      [internal_fix!(TimezoneOffsetPermissive)]; offset: 45_240);
    check!("+12345",       [internal_fix!(TimezoneOffsetPermissive), num!(Day)]; offset: 45_240, day: 5);
    check!("+12:345",      [internal_fix!(TimezoneOffsetPermissive), num!(Day)]; offset: 45_240, day: 5);
    check!("+12:34:",      [internal_fix!(TimezoneOffsetPermissive), lit!(":")]; offset: 45_240);
    check!("🤠+12:34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12:34🤠",     [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("+12:🤠34",     [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+12:34🤠",     [internal_fix!(TimezoneOffsetPermissive), lit!("🤠")]; offset: 45_240);
    check!("🤠+12:34",     [lit!("🤠"), internal_fix!(TimezoneOffsetPermissive)]; offset: 45_240);
    check!("Z",            [internal_fix!(TimezoneOffsetPermissive)]; offset: 0);
    check!("A",            [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("PST",          [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("z",            [internal_fix!(TimezoneOffsetPermissive)]; offset: 0);
    check!(" Z",           [internal_fix!(TimezoneOffsetPermissive)]; offset: 0);
    check!(" z",           [internal_fix!(TimezoneOffsetPermissive)]; offset: 0);
    check!("Z ",           [internal_fix!(TimezoneOffsetPermissive)]; TOO_LONG);
    check!("#Z",           [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!(":Z",           [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!(":z",           [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+Z",           [internal_fix!(TimezoneOffsetPermissive)]; TOO_SHORT);
    check!("-Z",           [internal_fix!(TimezoneOffsetPermissive)]; TOO_SHORT);
    check!("+A",           [internal_fix!(TimezoneOffsetPermissive)]; TOO_SHORT);
    check!("+PST",         [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+🙃",          [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("+Z:",          [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!(" :Z",          [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!(" +Z",          [internal_fix!(TimezoneOffsetPermissive)]; TOO_SHORT);
    check!(" -Z",          [internal_fix!(TimezoneOffsetPermissive)]; TOO_SHORT);
    check!("+:Z",          [internal_fix!(TimezoneOffsetPermissive)]; INVALID);
    check!("Y",            [internal_fix!(TimezoneOffsetPermissive)]; INVALID);

    // TimezoneName
    check!("CEST",         [fix!(TimezoneName)]; );
    check!("cest",         [fix!(TimezoneName)]; ); // lowercase
    check!("XXXXXXXX",     [fix!(TimezoneName)]; ); // not a real timezone name
    check!("!!!!",         [fix!(TimezoneName)]; ); // not a real timezone name!
    check!("CEST 5",       [fix!(TimezoneName), lit!(" "), num!(Day)]; day: 5);
    check!("CEST ",        [fix!(TimezoneName)]; TOO_LONG);
    check!(" CEST",        [fix!(TimezoneName)]; TOO_LONG);
    check!("CE ST",        [fix!(TimezoneName)]; TOO_LONG);

    // some practical examples
    check!("2015-02-04T14:37:05+09:00",
           [num!(Year), lit!("-"), num!(Month), lit!("-"), num!(Day), lit!("T"),
            num!(Hour), lit!(":"), num!(Minute), lit!(":"), num!(Second), fix!(TimezoneOffset)];
           year: 2015, month: 2, day: 4, hour_div_12: 1, hour_mod_12: 2,
           minute: 37, second: 5, offset: 32400);
    check!("20150204143705567",
            [num!(Year), num!(Month), num!(Day),
            num!(Hour), num!(Minute), num!(Second), internal_fix!(Nanosecond3NoDot)];
            year: 2015, month: 2, day: 4, hour_div_12: 1, hour_mod_12: 2,
            minute: 37, second: 5, nanosecond: 567000000);
    check!("20150204143705.567",
            [num!(Year), num!(Month), num!(Day),
            num!(Hour), num!(Minute), num!(Second), fix!(Nanosecond)];
            year: 2015, month: 2, day: 4, hour_div_12: 1, hour_mod_12: 2,
            minute: 37, second: 5, nanosecond: 567000000);
    check!("20150204143705.567891",
            [num!(Year), num!(Month), num!(Day),
            num!(Hour), num!(Minute), num!(Second), fix!(Nanosecond)];
            year: 2015, month: 2, day: 4, hour_div_12: 1, hour_mod_12: 2,
            minute: 37, second: 5, nanosecond: 567891000);
    check!("20150204143705.567891023",
            [num!(Year), num!(Month), num!(Day),
            num!(Hour), num!(Minute), num!(Second), fix!(Nanosecond)];
            year: 2015, month: 2, day: 4, hour_div_12: 1, hour_mod_12: 2,
            minute: 37, second: 5, nanosecond: 567891023);
    check!("Mon, 10 Jun 2013 09:32:37  GMT",
           [fix!(ShortWeekdayName), lit!(","), sp!(" "), num!(Day), sp!(" "),
            fix!(ShortMonthName), sp!(" "), num!(Year), sp!(" "), num!(Hour), lit!(":"),
            num!(Minute), lit!(":"), num!(Second), sp!("  "), lit!("GMT")];
           year: 2013, month: 6, day: 10, weekday: Weekday::Mon,
           hour_div_12: 0, hour_mod_12: 9, minute: 32, second: 37);
    check!("🤠Mon, 10 Jun🤠2013 09:32:37  GMT🤠",
           [lit!("🤠"), fix!(ShortWeekdayName), lit!(","), sp!(" "), num!(Day), sp!(" "),
            fix!(ShortMonthName), lit!("🤠"), num!(Year), sp!(" "), num!(Hour), lit!(":"),
            num!(Minute), lit!(":"), num!(Second), sp!("  "), lit!("GMT"), lit!("🤠")];
           year: 2013, month: 6, day: 10, weekday: Weekday::Mon,
           hour_div_12: 0, hour_mod_12: 9, minute: 32, second: 37);
    check!("Sun Aug 02 13:39:15 CEST 2020",
            [fix!(ShortWeekdayName), sp!(" "), fix!(ShortMonthName), sp!(" "),
            num!(Day), sp!(" "), num!(Hour), lit!(":"), num!(Minute), lit!(":"),
            num!(Second), sp!(" "), fix!(TimezoneName), sp!(" "), num!(Year)];
            year: 2020, month: 8, day: 2, weekday: Weekday::Sun,
            hour_div_12: 1, hour_mod_12: 1, minute: 39, second: 15);
    check!("20060102150405",
           [num!(Year), num!(Month), num!(Day), num!(Hour), num!(Minute), num!(Second)];
           year: 2006, month: 1, day: 2, hour_div_12: 1, hour_mod_12: 3, minute: 4, second: 5);
    check!("3:14PM",
           [num!(Hour12), lit!(":"), num!(Minute), fix!(LowerAmPm)];
           hour_div_12: 1, hour_mod_12: 3, minute: 14);
    check!("12345678901234.56789",
           [num!(Timestamp), lit!("."), num!(Nanosecond)];
           nanosecond: 56_789, timestamp: 12_345_678_901_234);
    check!("12345678901234.56789",
           [num!(Timestamp), fix!(Nanosecond)];
           nanosecond: 567_890_000, timestamp: 12_345_678_901_234);

    // docstring examples from `impl str::FromStr`
    check!("2000-01-02T03:04:05Z",
           [num!(Year), lit!("-"), num!(Month), lit!("-"), num!(Day), lit!("T"),
           num!(Hour), lit!(":"), num!(Minute), lit!(":"), num!(Second),
           internal_fix!(TimezoneOffsetPermissive)];
           year: 2000, month: 1, day: 2,
           hour_div_12: 0, hour_mod_12: 3, minute: 4, second: 5,
           offset: 0);
    check!("2000-01-02 03:04:05Z",
           [num!(Year), lit!("-"), num!(Month), lit!("-"), num!(Day), sp!(" "),
           num!(Hour), lit!(":"), num!(Minute), lit!(":"), num!(Second),
           internal_fix!(TimezoneOffsetPermissive)];
           year: 2000, month: 1, day: 2,
           hour_div_12: 0, hour_mod_12: 3, minute: 4, second: 5,
           offset: 0);
}

#[cfg(test)]
#[test]
fn test_rfc2822() {
    use super::NOT_ENOUGH;
    use super::*;
    use crate::offset::FixedOffset;
    use crate::DateTime;

    // Test data - (input, Ok(expected result after parse and format) or Err(error code))
    let testdates = [
        ("Tue, 20 Jan 2015 17:35:20 -0800", Ok("Tue, 20 Jan 2015 17:35:20 -0800")), // normal case
        ("Fri,  2 Jan 2015 17:35:20 -0800", Ok("Fri, 02 Jan 2015 17:35:20 -0800")), // folding whitespace
        ("Fri, 02 Jan 2015 17:35:20 -0800", Ok("Fri, 02 Jan 2015 17:35:20 -0800")), // leading zero
        ("Tue, 20 Jan 2015 17:35:20 -0800 (UTC)", Ok("Tue, 20 Jan 2015 17:35:20 -0800")), // trailing comment
        ("Tue,  20 Jan 2015 17:35:20 -0800 (UTC)", Ok("Tue, 20 Jan 2015 17:35:20 -0800")), // intermixed arbitrary whitespace
        ("Tue, 20     Jan   2015\t17:35:20\t-0800\t\t(UTC)", Ok("Tue, 20 Jan 2015 17:35:20 -0800")), // intermixed arbitrary whitespace
        (
            r"Tue, 20 Jan 2015 17:35:20 -0800 ( (UTC ) (\( (a)\(( \t ) ) \\( \) ))",
            Ok("Tue, 20 Jan 2015 17:35:20 -0800"),
        ), // complex trailing comment
        (r"Tue, 20 Jan 2015 17:35:20 -0800 (UTC\)", Err(TOO_LONG)), // incorrect comment, not enough closing parentheses
        (
            "Tue, 20 Jan 2015 17:35:20 -0800 (UTC)\t \r\n(Anothercomment)",
            Ok("Tue, 20 Jan 2015 17:35:20 -0800"),
        ), // multiple comments
        ("Tue, 20 Jan 2015 17:35:20 -0800 (UTC) ", Err(TOO_LONG)), // trailing whitespace after comment
        ("20 Jan 2015 17:35:20 -0800", Ok("Tue, 20 Jan 2015 17:35:20 -0800")), // no day of week
        ("20 JAN 2015 17:35:20 -0800", Ok("Tue, 20 Jan 2015 17:35:20 -0800")), // upper case month
        ("Tue, 20 Jan 2015 17:35 -0800", Ok("Tue, 20 Jan 2015 17:35:00 -0800")), // no second
        ("11 Sep 2001 09:45:00 EST", Ok("Tue, 11 Sep 2001 09:45:00 -0500")),
        ("30 Feb 2015 17:35:20 -0800", Err(OUT_OF_RANGE)), // bad day of month
        ("Tue, 20 Jan 2015", Err(TOO_SHORT)),              // omitted fields
        ("Tue, 20 Avr 2015 17:35:20 -0800", Err(INVALID)), // bad month name
        ("Tue, 20 Jan 2015 25:35:20 -0800", Err(OUT_OF_RANGE)), // bad hour
        ("Tue, 20 Jan 2015 7:35:20 -0800", Err(INVALID)),  // bad # of digits in hour
        ("Tue, 20 Jan 2015 17:65:20 -0800", Err(OUT_OF_RANGE)), // bad minute
        ("Tue, 20 Jan 2015 17:35:90 -0800", Err(OUT_OF_RANGE)), // bad second
        ("Tue, 20 Jan 2015 17:35:20 -0890", Err(OUT_OF_RANGE)), // bad offset
        ("6 Jun 1944 04:00:00Z", Err(INVALID)),            // bad offset (zulu not allowed)
        ("Tue, 20 Jan 2015 17:35:20 HAS", Err(NOT_ENOUGH)), // bad named time zone
        // named timezones that have specific timezone offsets
        // see https://www.rfc-editor.org/rfc/rfc2822#section-4.3
        ("Tue, 20 Jan 2015 17:35:20 GMT", Ok("Tue, 20 Jan 2015 17:35:20 +0000")),
        ("Tue, 20 Jan 2015 17:35:20 UT", Ok("Tue, 20 Jan 2015 17:35:20 +0000")),
        ("Tue, 20 Jan 2015 17:35:20 ut", Ok("Tue, 20 Jan 2015 17:35:20 +0000")),
        ("Tue, 20 Jan 2015 17:35:20 EDT", Ok("Tue, 20 Jan 2015 17:35:20 -0400")),
        ("Tue, 20 Jan 2015 17:35:20 EST", Ok("Tue, 20 Jan 2015 17:35:20 -0500")),
        ("Tue, 20 Jan 2015 17:35:20 CDT", Ok("Tue, 20 Jan 2015 17:35:20 -0500")),
        ("Tue, 20 Jan 2015 17:35:20 CST", Ok("Tue, 20 Jan 2015 17:35:20 -0600")),
        ("Tue, 20 Jan 2015 17:35:20 MDT", Ok("Tue, 20 Jan 2015 17:35:20 -0600")),
        ("Tue, 20 Jan 2015 17:35:20 MST", Ok("Tue, 20 Jan 2015 17:35:20 -0700")),
        ("Tue, 20 Jan 2015 17:35:20 PDT", Ok("Tue, 20 Jan 2015 17:35:20 -0700")),
        ("Tue, 20 Jan 2015 17:35:20 PST", Ok("Tue, 20 Jan 2015 17:35:20 -0800")),
        ("Tue, 20 Jan 2015 17:35:20 pst", Ok("Tue, 20 Jan 2015 17:35:20 -0800")),
        // named single-letter military timezones must fallback to +0000
        ("Tue, 20 Jan 2015 17:35:20 Z", Ok("Tue, 20 Jan 2015 17:35:20 +0000")),
        ("Tue, 20 Jan 2015 17:35:20 A", Ok("Tue, 20 Jan 2015 17:35:20 +0000")),
        ("Tue, 20 Jan 2015 17:35:20 a", Ok("Tue, 20 Jan 2015 17:35:20 +0000")),
        ("Tue, 20 Jan 2015 17:35:20 K", Ok("Tue, 20 Jan 2015 17:35:20 +0000")),
        ("Tue, 20 Jan 2015 17:35:20 k", Ok("Tue, 20 Jan 2015 17:35:20 +0000")),
        // named single-letter timezone "J" is specifically not valid
        ("Tue, 20 Jan 2015 17:35:20 J", Err(NOT_ENOUGH)),
        ("Tue, 20 Jan 2015😈17:35:20 -0800", Err(INVALID)), // bad character!
    ];

    fn rfc2822_to_datetime(date: &str) -> ParseResult<DateTime<FixedOffset>> {
        let mut parsed = Parsed::new();
        parse(&mut parsed, date, [Item::Fixed(Fixed::RFC2822)].iter())?;
        parsed.to_datetime()
    }

    fn fmt_rfc2822_datetime(dt: DateTime<FixedOffset>) -> String {
        dt.format_with_items([Item::Fixed(Fixed::RFC2822)].iter()).to_string()
    }

    // Test against test data above
    for &(date, checkdate) in testdates.iter() {
        let d = rfc2822_to_datetime(date); // parse a date
        let dt = match d {
            // did we get a value?
            Ok(dt) => Ok(fmt_rfc2822_datetime(dt)), // yes, go on
            Err(e) => Err(e),                       // otherwise keep an error for the comparison
        };
        if dt != checkdate.map(|s| s.to_string()) {
            // check for expected result
            panic!(
                "Date conversion failed for {}\nReceived: {:?}\nExpected: {:?}",
                date, dt, checkdate
            );
        }
    }
}

#[cfg(test)]
#[test]
fn parse_rfc850() {
    use crate::{TimeZone, Utc};

    static RFC850_FMT: &str = "%A, %d-%b-%y %T GMT";

    let dt_str = "Sunday, 06-Nov-94 08:49:37 GMT";
    let dt = Utc.with_ymd_and_hms(1994, 11, 6, 8, 49, 37).unwrap();

    // Check that the format is what we expect
    assert_eq!(dt.format(RFC850_FMT).to_string(), dt_str);

    // Check that it parses correctly
    assert_eq!(Ok(dt), Utc.datetime_from_str("Sunday, 06-Nov-94 08:49:37 GMT", RFC850_FMT));

    // Check that the rest of the weekdays parse correctly (this test originally failed because
    // Sunday parsed incorrectly).
    let testdates = [
        (Utc.with_ymd_and_hms(1994, 11, 7, 8, 49, 37).unwrap(), "Monday, 07-Nov-94 08:49:37 GMT"),
        (Utc.with_ymd_and_hms(1994, 11, 8, 8, 49, 37).unwrap(), "Tuesday, 08-Nov-94 08:49:37 GMT"),
        (
            Utc.with_ymd_and_hms(1994, 11, 9, 8, 49, 37).unwrap(),
            "Wednesday, 09-Nov-94 08:49:37 GMT",
        ),
        (
            Utc.with_ymd_and_hms(1994, 11, 10, 8, 49, 37).unwrap(),
            "Thursday, 10-Nov-94 08:49:37 GMT",
        ),
        (Utc.with_ymd_and_hms(1994, 11, 11, 8, 49, 37).unwrap(), "Friday, 11-Nov-94 08:49:37 GMT"),
        (
            Utc.with_ymd_and_hms(1994, 11, 12, 8, 49, 37).unwrap(),
            "Saturday, 12-Nov-94 08:49:37 GMT",
        ),
    ];

    for val in &testdates {
        assert_eq!(Ok(val.0), Utc.datetime_from_str(val.1, RFC850_FMT));
    }
}

#[cfg(test)]
#[test]
fn test_rfc3339() {
    use super::*;
    use crate::offset::FixedOffset;
    use crate::DateTime;

    // Test data - (input, Ok(expected result after parse and format) or Err(error code))
    let testdates = [
        ("2015-01-20T17:35:20-08:00", Ok("2015-01-20T17:35:20-08:00")), // normal case
        ("1944-06-06T04:04:00Z", Ok("1944-06-06T04:04:00+00:00")),      // D-day
        ("2001-09-11T09:45:00-08:00", Ok("2001-09-11T09:45:00-08:00")),
        ("2015-01-20T17:35:20.001-08:00", Ok("2015-01-20T17:35:20.001-08:00")),
        ("2015-01-20T17:35:20.000031-08:00", Ok("2015-01-20T17:35:20.000031-08:00")),
        ("2015-01-20T17:35:20.000000004-08:00", Ok("2015-01-20T17:35:20.000000004-08:00")),
        ("2015-01-20T17:35:20.000000000452-08:00", Ok("2015-01-20T17:35:20-08:00")), // too small
        ("2015-01-20 17:35:20.001-08:00", Err(INVALID)), // missing separator 'T'
        ("2015/01/20T17:35:20.001-08:00", Err(INVALID)), // wrong separator char YMD
        ("2015-01-20T17-35-20.001-08:00", Err(INVALID)), // wrong separator char HMS
        ("99999-01-20T17:35:20-08:00", Err(INVALID)),    // bad year value
        ("-2000-01-20T17:35:20-08:00", Err(INVALID)),    // bad year value
        ("2015-02-30T17:35:20-08:00", Err(OUT_OF_RANGE)), // bad day of month value
        ("2015-01-20T25:35:20-08:00", Err(OUT_OF_RANGE)), // bad hour value
        ("2015-01-20T17:65:20-08:00", Err(OUT_OF_RANGE)), // bad minute value
        ("2015-01-20T17:35:90-08:00", Err(OUT_OF_RANGE)), // bad second value
        ("2015-01-20T17:35:20-24:00", Err(OUT_OF_RANGE)), // bad offset value
        ("15-01-20T17:35:20-08:00", Err(INVALID)),       // bad year format
        ("15-01-20T17:35:20-08:00:00", Err(INVALID)),    // bad year format, bad offset format
        ("2015-01-20T17:35:20-0800", Err(INVALID)),      // bad offset format
        ("2015-01-20T17:35:20.001-08 : 00", Err(INVALID)), // bad offset format
        ("2015-01-20T17:35:20-08:00:00", Err(TOO_LONG)), // bad offset format
        ("2015-01-20T17:35:20-08:", Err(TOO_SHORT)),     // bad offset format
        ("2015-01-20T17:35:20-08", Err(TOO_SHORT)),      // bad offset format
        ("2015-01-20T", Err(TOO_SHORT)),                 // missing HMS
        ("2015-01-20T00:00:1", Err(TOO_SHORT)),          // missing complete S
        ("2015-01-20T00:00:1-08:00", Err(INVALID)),      // missing complete S
    ];

    fn rfc3339_to_datetime(date: &str) -> ParseResult<DateTime<FixedOffset>> {
        let mut parsed = Parsed::new();
        parse(&mut parsed, date, [Item::Fixed(Fixed::RFC3339)].iter())?;
        parsed.to_datetime()
    }

    fn fmt_rfc3339_datetime(dt: DateTime<FixedOffset>) -> String {
        dt.format_with_items([Item::Fixed(Fixed::RFC3339)].iter()).to_string()
    }

    // Test against test data above
    for &(date, checkdate) in testdates.iter() {
        eprintln!("test_rfc3339: date {:?}, expect {:?}", date, checkdate);
        let d = rfc3339_to_datetime(date); // parse a date
        let dt = match d {
            // did we get a value?
            Ok(dt) => Ok(fmt_rfc3339_datetime(dt)), // yes, go on
            Err(e) => Err(e),                       // otherwise keep an error for the comparison
        };
        if dt != checkdate.map(|s| s.to_string()) {
            // check for expected result
            panic!(
                "Date conversion failed for {}\nReceived: {:?}\nExpected: {:?}",
                date, dt, checkdate
            );
        }
    }
}

#[cfg(test)]
#[test]
fn test_issue_1010() {
    let dt = crate::NaiveDateTime::parse_from_str("\u{c}SUN\u{e}\u{3000}\0m@J\u{3000}\0\u{3000}\0m\u{c}!\u{c}\u{b}\u{c}\u{c}\u{c}\u{c}%A\u{c}\u{b}\0SU\u{c}\u{c}",
    "\u{c}\u{c}%A\u{c}\u{b}\0SUN\u{c}\u{c}\u{c}SUNN\u{c}\u{c}\u{c}SUN\u{c}\u{c}!\u{c}\u{b}\u{c}\u{c}\u{c}\u{c}%A\u{c}\u{b}%a");
    assert_eq!(dt, Err(ParseError(ParseErrorKind::Invalid)));
}

#[cfg(test)]
mod tests_rug_221 {
    use super::*;
    use crate::format::Parsed;
    use crate::Weekday;
    
    #[test]
    fn test_set_weekday_with_num_days_from_sunday() {
        let mut p0: Parsed = Parsed::new();
        let mut p1: i64 = 0;

        set_weekday_with_num_days_from_sunday(&mut p0, p1);
        
        // Add assertions if necessary
        // assert_eq!(...);
    }
}
        
        
#[cfg(test)]
        mod tests_rug_222 {
            use super::*;
            use crate::format::Parsed;
            
            #[test]
            fn test_rug() {
                
                let mut p0: Parsed = Parsed::new();
                let mut p1: i64 = 2;
                
                crate::format::parse::set_weekday_with_number_from_monday(&mut p0, p1);

            }
        }#[cfg(test)]
mod tests_rug_223 {
    use super::*;
    use crate::format::Parsed;

    #[test]
    fn test_parse_rfc2822() {
        let mut parsed: Parsed = Parsed::new();
        let s: &str = "Fri, 21 Nov 1997 09:55:06 -0600";

        parse_rfc2822(&mut parsed, s);
    }
}
#[cfg(test)]
mod tests_rug_224 {
    use super::*;
    use crate::format::Parsed;
    
    #[test]
    fn test_rug() {
        let mut p0: Parsed = Parsed::new();
        let p1: &str = "2021-01-01T12:30:00Z";
               
        crate::format::parse::parse_rfc3339(&mut p0, &p1);

    }
}#[cfg(test)]
mod tests_rug_225 {
    use super::*;
    use crate::format::Parsed;
    use crate::format::StrftimeItems;
    
    #[test]
    fn test_parse() {
        let mut p0: Parsed = Parsed::new();
        let p1: &str = "your input string";
        let mut p2: StrftimeItems = StrftimeItems::new("your strftime format string");
        
        crate::format::parse::parse(&mut p0, p1, &mut p2);
        
        // add your assertions here
    }
} 
#[cfg(test)]
mod tests_rug_226 {
    use super::*;
    use crate::format::Parsed;
    use crate::format::StrftimeItems;

    #[test]
    fn test_rug() {
        let mut p0: Parsed = Parsed::new();
        let p1: &str = "your test string";
        let mut p2: StrftimeItems = StrftimeItems::new("your strftime format string");
        
        crate::format::parse::parse_internal(&mut p0, &p1, &mut p2);
    }
}
                            