// This is a part of Chrono.
// See README.md and LICENSE.txt for details.

/*!
 * Various scanning routines for the parser.
 */

#![allow(deprecated)]

use super::{ParseResult, INVALID, OUT_OF_RANGE, TOO_SHORT};
use crate::Weekday;

/// Returns true when two slices are equal case-insensitively (in ASCII).
/// Assumes that the `pattern` is already converted to lower case.
fn equals(s: &[u8], pattern: &str) -> bool {
    let mut xs = s.iter().map(|&c| match c {
        b'A'..=b'Z' => c + 32,
        _ => c,
    });
    let mut ys = pattern.as_bytes().iter().cloned();
    loop {
        match (xs.next(), ys.next()) {
            (None, None) => return true,
            (None, _) | (_, None) => return false,
            (Some(x), Some(y)) if x != y => return false,
            _ => (),
        }
    }
}

/// Tries to parse the non-negative number from `min` to `max` digits.
///
/// The absence of digits at all is an unconditional error.
/// More than `max` digits are consumed up to the first `max` digits.
/// Any number that does not fit in `i64` is an error.
#[inline]
pub(super) fn number(s: &str, min: usize, max: usize) -> ParseResult<(&str, i64)> {
    assert!(min <= max);

    // We are only interested in ascii numbers, so we can work with the `str` as bytes. We stop on
    // the first non-numeric byte, which may be another ascii character or beginning of multi-byte
    // UTF-8 character.
    let bytes = s.as_bytes();
    if bytes.len() < min {
        return Err(TOO_SHORT);
    }

    let mut n = 0i64;
    for (i, c) in bytes.iter().take(max).cloned().enumerate() {
        // cloned() = copied()
        if !c.is_ascii_digit() {
            if i < min {
                return Err(INVALID);
            } else {
                return Ok((&s[i..], n));
            }
        }

        n = match n.checked_mul(10).and_then(|n| n.checked_add((c - b'0') as i64)) {
            Some(n) => n,
            None => return Err(OUT_OF_RANGE),
        };
    }

    Ok((&s[core::cmp::min(max, bytes.len())..], n))
}

/// Tries to consume at least one digits as a fractional second.
/// Returns the number of whole nanoseconds (0--999,999,999).
pub(super) fn nanosecond(s: &str) -> ParseResult<(&str, i64)> {
    // record the number of digits consumed for later scaling.
    let origlen = s.len();
    let (s, v) = number(s, 1, 9)?;
    let consumed = origlen - s.len();

    // scale the number accordingly.
    static SCALE: [i64; 10] =
        [0, 100_000_000, 10_000_000, 1_000_000, 100_000, 10_000, 1_000, 100, 10, 1];
    let v = v.checked_mul(SCALE[consumed]).ok_or(OUT_OF_RANGE)?;

    // if there are more than 9 digits, skip next digits.
    let s = s.trim_left_matches(|c: char| c.is_ascii_digit());

    Ok((s, v))
}

/// Tries to consume a fixed number of digits as a fractional second.
/// Returns the number of whole nanoseconds (0--999,999,999).
pub(super) fn nanosecond_fixed(s: &str, digits: usize) -> ParseResult<(&str, i64)> {
    // record the number of digits consumed for later scaling.
    let (s, v) = number(s, digits, digits)?;

    // scale the number accordingly.
    static SCALE: [i64; 10] =
        [0, 100_000_000, 10_000_000, 1_000_000, 100_000, 10_000, 1_000, 100, 10, 1];
    let v = v.checked_mul(SCALE[digits]).ok_or(OUT_OF_RANGE)?;

    Ok((s, v))
}

/// Tries to parse the month index (0 through 11) with the first three ASCII letters.
pub(super) fn short_month0(s: &str) -> ParseResult<(&str, u8)> {
    if s.len() < 3 {
        return Err(TOO_SHORT);
    }
    let buf = s.as_bytes();
    let month0 = match (buf[0] | 32, buf[1] | 32, buf[2] | 32) {
        (b'j', b'a', b'n') => 0,
        (b'f', b'e', b'b') => 1,
        (b'm', b'a', b'r') => 2,
        (b'a', b'p', b'r') => 3,
        (b'm', b'a', b'y') => 4,
        (b'j', b'u', b'n') => 5,
        (b'j', b'u', b'l') => 6,
        (b'a', b'u', b'g') => 7,
        (b's', b'e', b'p') => 8,
        (b'o', b'c', b't') => 9,
        (b'n', b'o', b'v') => 10,
        (b'd', b'e', b'c') => 11,
        _ => return Err(INVALID),
    };
    Ok((&s[3..], month0))
}

/// Tries to parse the weekday with the first three ASCII letters.
pub(super) fn short_weekday(s: &str) -> ParseResult<(&str, Weekday)> {
    if s.len() < 3 {
        return Err(TOO_SHORT);
    }
    let buf = s.as_bytes();
    let weekday = match (buf[0] | 32, buf[1] | 32, buf[2] | 32) {
        (b'm', b'o', b'n') => Weekday::Mon,
        (b't', b'u', b'e') => Weekday::Tue,
        (b'w', b'e', b'd') => Weekday::Wed,
        (b't', b'h', b'u') => Weekday::Thu,
        (b'f', b'r', b'i') => Weekday::Fri,
        (b's', b'a', b't') => Weekday::Sat,
        (b's', b'u', b'n') => Weekday::Sun,
        _ => return Err(INVALID),
    };
    Ok((&s[3..], weekday))
}

/// Tries to parse the month index (0 through 11) with short or long month names.
/// It prefers long month names to short month names when both are possible.
pub(super) fn short_or_long_month0(s: &str) -> ParseResult<(&str, u8)> {
    // lowercased month names, minus first three chars
    static LONG_MONTH_SUFFIXES: [&str; 12] =
        ["uary", "ruary", "ch", "il", "", "e", "y", "ust", "tember", "ober", "ember", "ember"];

    let (mut s, month0) = short_month0(s)?;

    // tries to consume the suffix if possible
    let suffix = LONG_MONTH_SUFFIXES[month0 as usize];
    if s.len() >= suffix.len() && equals(&s.as_bytes()[..suffix.len()], suffix) {
        s = &s[suffix.len()..];
    }

    Ok((s, month0))
}

/// Tries to parse the weekday with short or long weekday names.
/// It prefers long weekday names to short weekday names when both are possible.
pub(super) fn short_or_long_weekday(s: &str) -> ParseResult<(&str, Weekday)> {
    // lowercased weekday names, minus first three chars
    static LONG_WEEKDAY_SUFFIXES: [&str; 7] =
        ["day", "sday", "nesday", "rsday", "day", "urday", "day"];

    let (mut s, weekday) = short_weekday(s)?;

    // tries to consume the suffix if possible
    let suffix = LONG_WEEKDAY_SUFFIXES[weekday.num_days_from_monday() as usize];
    if s.len() >= suffix.len() && equals(&s.as_bytes()[..suffix.len()], suffix) {
        s = &s[suffix.len()..];
    }

    Ok((s, weekday))
}

/// Tries to consume exactly one given character.
pub(super) fn char(s: &str, c1: u8) -> ParseResult<&str> {
    match s.as_bytes().first() {
        Some(&c) if c == c1 => Ok(&s[1..]),
        Some(_) => Err(INVALID),
        None => Err(TOO_SHORT),
    }
}

/// Tries to consume one or more whitespace.
pub(super) fn space(s: &str) -> ParseResult<&str> {
    let s_ = s.trim_left();
    if s_.len() < s.len() {
        Ok(s_)
    } else if s.is_empty() {
        Err(TOO_SHORT)
    } else {
        Err(INVALID)
    }
}

/// Returns slice remaining after first char.
/// If <=1 chars in `s` then return an empty slice
pub(super) fn s_next(s: &str) -> &str {
    match s.char_indices().nth(1) {
        Some((offset, _)) => &s[offset..],
        None => {
            // one or zero chars in `s`, return empty string
            &s[s.len()..]
        }
    }
}

/// If the first `char` is whitespace then consume it and return `s`.
/// Else return `s`.
pub(super) fn trim1(s: &str) -> &str {
    match s.chars().next() {
        Some(c) if c.is_whitespace() => s_next(s),
        Some(_) | None => s,
    }
}

/// Consumes one colon char `:` if it is at the front of `s`.
/// Always returns `Ok(s)`.
pub(super) fn consume_colon_maybe(mut s: &str) -> ParseResult<&str> {
    if s.is_empty() {
        // nothing consumed
        return Ok(s);
    }

    if s.starts_with(':') {
        s = s_next(s);
        // consumed `':'`
    }

    Ok(s)
}

/// Tries to parse `[-+]\d\d` continued by `\d\d`. Return an offset in seconds if possible.
///
/// The additional `colon` may be used to parse a mandatory or optional `:`
/// between hours and minutes, and should return either a new suffix or `Err` when parsing fails.
pub(super) fn timezone_offset<F>(s: &str, consume_colon: F) -> ParseResult<(&str, i32)>
where
    F: FnMut(&str) -> ParseResult<&str>,
{
    timezone_offset_internal(s, consume_colon, false)
}

fn timezone_offset_internal<F>(
    mut s: &str,
    mut consume_colon: F,
    allow_missing_minutes: bool,
) -> ParseResult<(&str, i32)>
where
    F: FnMut(&str) -> ParseResult<&str>,
{
    const fn digits(s: &str) -> ParseResult<(u8, u8)> {
        let b = s.as_bytes();
        if b.len() < 2 {
            Err(TOO_SHORT)
        } else {
            Ok((b[0], b[1]))
        }
    }
    let negative = match s.as_bytes().first() {
        Some(&b'+') => false,
        Some(&b'-') => true,
        Some(_) => return Err(INVALID),
        None => return Err(TOO_SHORT),
    };
    s = &s[1..];

    // hours (00--99)
    let hours = match digits(s)? {
        (h1 @ b'0'..=b'9', h2 @ b'0'..=b'9') => i32::from((h1 - b'0') * 10 + (h2 - b'0')),
        _ => return Err(INVALID),
    };
    s = &s[2..];

    // colons (and possibly other separators)
    s = consume_colon(s)?;

    // minutes (00--59)
    // if the next two items are digits then we have to add minutes
    let minutes = if let Ok(ds) = digits(s) {
        match ds {
            (m1 @ b'0'..=b'5', m2 @ b'0'..=b'9') => i32::from((m1 - b'0') * 10 + (m2 - b'0')),
            (b'6'..=b'9', b'0'..=b'9') => return Err(OUT_OF_RANGE),
            _ => return Err(INVALID),
        }
    } else if allow_missing_minutes {
        0
    } else {
        return Err(TOO_SHORT);
    };
    s = match s.len() {
        len if len >= 2 => &s[2..],
        len if len == 0 => s,
        _ => return Err(TOO_SHORT),
    };

    let seconds = hours * 3600 + minutes * 60;
    Ok((s, if negative { -seconds } else { seconds }))
}

/// Same as `timezone_offset` but also allows for `z`/`Z` which is the same as `+00:00`.
pub(super) fn timezone_offset_zulu<F>(s: &str, colon: F) -> ParseResult<(&str, i32)>
where
    F: FnMut(&str) -> ParseResult<&str>,
{
    let bytes = s.as_bytes();
    match bytes.first() {
        Some(&b'z') | Some(&b'Z') => Ok((&s[1..], 0)),
        Some(&b'u') | Some(&b'U') => {
            if bytes.len() >= 3 {
                let (b, c) = (bytes[1], bytes[2]);
                match (b | 32, c | 32) {
                    (b't', b'c') => Ok((&s[3..], 0)),
                    _ => Err(INVALID),
                }
            } else {
                Err(INVALID)
            }
        }
        _ => timezone_offset(s, colon),
    }
}

/// Same as `timezone_offset` but also allows for `z`/`Z` which is the same as
/// `+00:00`, and allows missing minutes entirely.
pub(super) fn timezone_offset_permissive<F>(s: &str, colon: F) -> ParseResult<(&str, i32)>
where
    F: FnMut(&str) -> ParseResult<&str>,
{
    match s.as_bytes().first() {
        Some(&b'z') | Some(&b'Z') => Ok((&s[1..], 0)),
        _ => timezone_offset_internal(s, colon, true),
    }
}

/// Same as `timezone_offset` but also allows for RFC 2822 legacy timezones.
/// May return `None` which indicates an insufficient offset data (i.e. `-0000`).
/// See [RFC 2822 Section 4.3].
///
/// [RFC 2822 Section 4.3]: https://tools.ietf.org/html/rfc2822#section-4.3
pub(super) fn timezone_offset_2822(s: &str) -> ParseResult<(&str, Option<i32>)> {
    // tries to parse legacy time zone names
    let upto = s.as_bytes().iter().position(|&c| !c.is_ascii_alphabetic()).unwrap_or(s.len());
    if upto > 0 {
        let name = &s.as_bytes()[..upto];
        let s = &s[upto..];
        let offset_hours = |o| Ok((s, Some(o * 3600)));
        if equals(name, "gmt") || equals(name, "ut") {
            offset_hours(0)
        } else if equals(name, "edt") {
            offset_hours(-4)
        } else if equals(name, "est") || equals(name, "cdt") {
            offset_hours(-5)
        } else if equals(name, "cst") || equals(name, "mdt") {
            offset_hours(-6)
        } else if equals(name, "mst") || equals(name, "pdt") {
            offset_hours(-7)
        } else if equals(name, "pst") {
            offset_hours(-8)
        } else if name.len() == 1 {
            match name[0] {
                // recommended by RFC 2822: consume but treat it as -0000
                b'a'..=b'i' | b'k'..=b'z' | b'A'..=b'I' | b'K'..=b'Z' => offset_hours(0),
                _ => Ok((s, None)),
            }
        } else {
            Ok((s, None))
        }
    } else {
        let (s_, offset) = timezone_offset(s, |s| Ok(s))?;
        Ok((s_, Some(offset)))
    }
}

/// Tries to consume everything until next whitespace-like symbol.
/// Does not provide any offset information from the consumed data.
pub(super) fn timezone_name_skip(s: &str) -> ParseResult<(&str, ())> {
    Ok((s.trim_left_matches(|c: char| !c.is_whitespace()), ()))
}

/// Tries to consume an RFC2822 comment including preceding ` `.
///
/// Returns the remaining string after the closing parenthesis.
pub(super) fn comment_2822(s: &str) -> ParseResult<(&str, ())> {
    use CommentState::*;

    let s = s.trim_start();

    let mut state = Start;
    for (i, c) in s.bytes().enumerate() {
        state = match (state, c) {
            (Start, b'(') => Next(1),
            (Next(1), b')') => return Ok((&s[i + 1..], ())),
            (Next(depth), b'\\') => Escape(depth),
            (Next(depth), b'(') => Next(depth + 1),
            (Next(depth), b')') => Next(depth - 1),
            (Next(depth), _) | (Escape(depth), _) => Next(depth),
            _ => return Err(INVALID),
        };
    }

    Err(TOO_SHORT)
}

enum CommentState {
    Start,
    Next(usize),
    Escape(usize),
}

#[cfg(test)]
#[test]
fn test_rfc2822_comments() {
    let testdata = [
        ("", Err(TOO_SHORT)),
        (" ", Err(TOO_SHORT)),
        ("x", Err(INVALID)),
        ("(", Err(TOO_SHORT)),
        ("()", Ok("")),
        (" \r\n\t()", Ok("")),
        ("() ", Ok(" ")),
        ("()z", Ok("z")),
        ("(x)", Ok("")),
        ("(())", Ok("")),
        ("((()))", Ok("")),
        ("(x(x(x)x)x)", Ok("")),
        ("( x ( x ( x ) x ) x )", Ok("")),
        (r"(\)", Err(TOO_SHORT)),
        (r"(\()", Ok("")),
        (r"(\))", Ok("")),
        (r"(\\)", Ok("")),
        ("(()())", Ok("")),
        ("( x ( x ) x ( x ) x )", Ok("")),
    ];

    for (test_in, expected) in testdata.iter() {
        let actual = comment_2822(test_in).map(|(s, _)| s);
        assert_eq!(
            *expected, actual,
            "{:?} expected to produce {:?}, but produced {:?}.",
            test_in, expected, actual
        );
    }
}

#[test]
fn test_space() {
    assert_eq!(space(""), Err(TOO_SHORT));
    assert_eq!(space(" "), Ok(""));
    assert_eq!(space(" \t"), Ok(""));
    assert_eq!(space(" \ta"), Ok("a"));
    assert_eq!(space(" \ta "), Ok("a "));
    assert_eq!(space("a"), Err(INVALID));
    assert_eq!(space("a "), Err(INVALID));
}

#[test]
fn test_s_next() {
    assert_eq!(s_next(""), "");
    assert_eq!(s_next(" "), "");
    assert_eq!(s_next("a"), "");
    assert_eq!(s_next("ab"), "b");
    assert_eq!(s_next("abc"), "bc");
    assert_eq!(s_next("😾b"), "b");
    assert_eq!(s_next("a😾"), "😾");
    assert_eq!(s_next("😾bc"), "bc");
    assert_eq!(s_next("a😾c"), "😾c");
}

#[test]
fn test_trim1() {
    assert_eq!(trim1(""), "");
    assert_eq!(trim1(" "), "");
    assert_eq!(trim1("\t"), "");
    assert_eq!(trim1("\t\t"), "\t");
    assert_eq!(trim1("  "), " ");
    assert_eq!(trim1("a"), "a");
    assert_eq!(trim1("a "), "a ");
    assert_eq!(trim1("ab"), "ab");
    assert_eq!(trim1("😼"), "😼");
    assert_eq!(trim1("😼b"), "😼b");
}

#[test]
fn test_consume_colon_maybe() {
    assert_eq!(consume_colon_maybe(""), Ok(""));
    assert_eq!(consume_colon_maybe(" "), Ok(" "));
    assert_eq!(consume_colon_maybe("\n"), Ok("\n"));
    assert_eq!(consume_colon_maybe("  "), Ok("  "));
    assert_eq!(consume_colon_maybe(":"), Ok(""));
    assert_eq!(consume_colon_maybe(" :"), Ok(" :"));
    assert_eq!(consume_colon_maybe(": "), Ok(" "));
    assert_eq!(consume_colon_maybe(" : "), Ok(" : "));
    assert_eq!(consume_colon_maybe(":  "), Ok("  "));
    assert_eq!(consume_colon_maybe("  :"), Ok("  :"));
    assert_eq!(consume_colon_maybe(":: "), Ok(": "));
    assert_eq!(consume_colon_maybe("😸"), Ok("😸"));
    assert_eq!(consume_colon_maybe("😸😸"), Ok("😸😸"));
    assert_eq!(consume_colon_maybe("😸:"), Ok("😸:"));
    assert_eq!(consume_colon_maybe("😸 "), Ok("😸 "));
    assert_eq!(consume_colon_maybe(":😸"), Ok("😸"));
    assert_eq!(consume_colon_maybe(":😸 "), Ok("😸 "));
    assert_eq!(consume_colon_maybe(": 😸"), Ok(" 😸"));
    assert_eq!(consume_colon_maybe(":  😸"), Ok("  😸"));
    assert_eq!(consume_colon_maybe(": :😸"), Ok(" :😸"));
}
     
#[cfg(test)]
mod tests_rug_228 {
    use super::*;
    use crate::format::scan::equals;

    #[test]
    fn test_equals() {
        let p0: &[u8] = b"hello";  // Sample data for the first argument
        let p1: &str = "Hello";  // Sample data for the second argument

        assert_eq!(equals(p0, p1), false);
    }
}
            #[cfg(test)]
mod tests_rug_229 {
    use super::*;
    use crate::format::scan::ParseResult;

    #[test]
    fn test_number() {
        let p0: &str = "12345"; // Sample data for the first argument
        let p1: usize = 2; // Sample data for the second argument
        let p2: usize = 5; // Sample data for the third argument

        let result: ParseResult<(&str, i64)> = crate::format::scan::number(p0, p1, p2);

        // Add assertions here to validate the result
    }
}                        
#[cfg(test)]
mod tests_rug_230 {
    use super::*;
    use crate::format::ParseResult;
    
    #[test]
    fn test_nanosecond() {
        let p0: &str = "1234567890";
        
        crate::format::scan::nanosecond(&p0);
    }
}
                            #[cfg(test)]
mod tests_rug_231 {
    use super::*;
    use crate::format::scan::ParseResult;
    use crate::format::scan::{number, OUT_OF_RANGE};

    #[test]
    fn test_rug() {
        let mut p0: &str = "0.123456789";
        let mut p1: usize = 9;

        let result: ParseResult<(&str, i64)> = crate::format::scan::nanosecond_fixed(p0, p1);

        // Add assertions here
    }
}#[cfg(test)]
mod tests_rug_232 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let p0: &str = "Mar";
        
        crate::format::scan::short_month0(&p0);

    }
}#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::Weekday;

    #[test]
    fn test_rug() {
        let mut p0: &str = "Mon";

        crate::format::scan::short_weekday(&p0);

    }
}#[cfg(test)]
mod tests_rug_234 {
    use super::*;
    use crate::format::scan::ParseResult;

    #[test]
    fn test_rug() {
        let mut p0 = "Jan";

        let result: ParseResult<(&str, u8)> = crate::format::scan::short_or_long_month0(p0);

        assert_eq!(result.is_ok(), true);
    }
}#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    use crate::Weekday;

    #[test]
    fn test_short_or_long_weekday() {
        let p0 = "Sun";
        crate::format::scan::short_or_long_weekday(&p0);
    }
}#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    use crate::format::ParseResult;

    #[test]
    fn test_char() {
        let mut p0 = "Hello, world!";
        let mut p1 = b'H';

        char(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_237 {
    use super::*;
    
    #[test]
    fn test_rug() {
        // Sample data
        let p0: &str = "   test   ";
        
        crate::format::scan::space(p0);
    }
}#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    
    #[test]
    fn test_rug() {
        // Sample data
        let s: &str = "Hello, World!";
        
        crate::format::scan::s_next(s);
    }
}#[cfg(test)]
mod tests_rug_239 {
    use super::*;

    #[test]
    fn test_trim1() {
        let p0: &str = "  Hello World!   ";
        assert_eq!(trim1(p0), "Hello World!   ");

        let p1: &str = "Hi!";
        assert_eq!(trim1(p1), "Hi!");

        let p2: &str = "";
        assert_eq!(trim1(p2), "");

        let p3: &str = "   ";
        assert_eq!(trim1(p3), "");

        let p4: &str = "\t\n   ";
        assert_eq!(trim1(p4), "\t\n   ");
    }
}#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let mut p0: &str = "sample_input";
        
        crate::format::scan::consume_colon_maybe(&p0);
        
    }
}
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    use crate::format::scan::{ParseResult, timezone_offset, INVALID};

    #[test]
    fn test_timezone_offset_zulu() {
        let mut p0: &str = "zulu";
        let p1 = |s: &str| match s {
            "zulu" => Ok(""),
            _ => Err(INVALID),
        };

        assert_eq!(
            timezone_offset_zulu::<fn(&str) -> ParseResult<&str>>(
                &p0,
                p1 as fn(&str) -> ParseResult<&str>
            ),
            Ok(("", 0))
        );

        let mut p0: &str = "Z";
        let p1 = |s: &str| match s {
            "Z" => Ok(""),
            _ => Err(INVALID),
        };

        assert_eq!(
            timezone_offset_zulu::<fn(&str) -> ParseResult<&str>>(
                &p0,
                p1 as fn(&str) -> ParseResult<&str>
            ),
            Ok(("", 0))
        );

        let mut p0: &str = "utc";
        let p1 = |s: &str| match s {
            "utc" => Ok(""),
            _ => Err(INVALID),
        };

        assert_eq!(
            timezone_offset_zulu::<fn(&str) -> ParseResult<&str>>(
                &p0,
                p1 as fn(&str) -> ParseResult<&str>
            ),
            Ok(("", 0))
        );

    }
}#[cfg(test)]
mod tests_rug_246 {
    use super::*;
    use crate::format::scan::timezone_offset_2822;

    #[test]
    fn test_timezone_offset_2822() {
        let p0: &str = "GMT";

        timezone_offset_2822(p0);
    }
}#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    use crate::ParseResult;

    #[test]
    fn test_rug() {
        let mut p0: &str = "   America/New_York";

        crate::format::scan::timezone_name_skip(&p0);

    }
}#[cfg(test)]
mod tests_rug_248 {
    use super::*;
    
    #[test]
    fn test_rug() {
        let p0 = "This is a test comment (with nested (parentheses))";

        crate::format::scan::comment_2822(&p0);
    }
}