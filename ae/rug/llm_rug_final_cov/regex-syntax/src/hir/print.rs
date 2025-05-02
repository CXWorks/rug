/*!
This module provides a regular expression printer for `Hir`.
*/
use core::fmt;
use crate::{
    hir::{
        self, visitor::{self, Visitor},
        Hir, HirKind,
    },
    is_meta_character,
};
/// A builder for constructing a printer.
///
/// Note that since a printer doesn't have any configuration knobs, this type
/// remains unexported.
#[derive(Clone, Debug)]
struct PrinterBuilder {
    _priv: (),
}
impl Default for PrinterBuilder {
    fn default() -> PrinterBuilder {
        PrinterBuilder::new()
    }
}
impl PrinterBuilder {
    fn new() -> PrinterBuilder {
        PrinterBuilder { _priv: () }
    }
    fn build(&self) -> Printer {
        Printer { _priv: () }
    }
}
/// A printer for a regular expression's high-level intermediate
/// representation.
///
/// A printer converts a high-level intermediate representation (HIR) to a
/// regular expression pattern string. This particular printer uses constant
/// stack space and heap space proportional to the size of the HIR.
///
/// Since this printer is only using the HIR, the pattern it prints will likely
/// not resemble the original pattern at all. For example, a pattern like
/// `\pL` will have its entire class written out.
///
/// The purpose of this printer is to provide a means to mutate an HIR and then
/// build a regular expression from the result of that mutation. (A regex
/// library could provide a constructor from this HIR explicitly, but that
/// creates an unnecessary public coupling between the regex library and this
/// specific HIR representation.)
#[derive(Debug)]
pub struct Printer {
    _priv: (),
}
impl Printer {
    /// Create a new printer.
    pub fn new() -> Printer {
        PrinterBuilder::new().build()
    }
    /// Print the given `Ast` to the given writer. The writer must implement
    /// `fmt::Write`. Typical implementations of `fmt::Write` that can be used
    /// here are a `fmt::Formatter` (which is available in `fmt::Display`
    /// implementations) or a `&mut String`.
    pub fn print<W: fmt::Write>(&mut self, hir: &Hir, wtr: W) -> fmt::Result {
        visitor::visit(hir, Writer { wtr })
    }
}
#[derive(Debug)]
struct Writer<W> {
    wtr: W,
}
impl<W: fmt::Write> Visitor for Writer<W> {
    type Output = ();
    type Err = fmt::Error;
    fn finish(self) -> fmt::Result {
        Ok(())
    }
    fn visit_pre(&mut self, hir: &Hir) -> fmt::Result {
        match *hir.kind() {
            HirKind::Empty | HirKind::Repetition(_) => {}
            HirKind::Literal(hir::Literal(ref bytes)) => {
                let result = core::str::from_utf8(bytes);
                let len = result.map_or(bytes.len(), |s| s.chars().count());
                if len > 1 {
                    self.wtr.write_str(r"(?:")?;
                }
                match result {
                    Ok(string) => {
                        for c in string.chars() {
                            self.write_literal_char(c)?;
                        }
                    }
                    Err(_) => {
                        for &b in bytes.iter() {
                            self.write_literal_byte(b)?;
                        }
                    }
                }
                if len > 1 {
                    self.wtr.write_str(r")")?;
                }
            }
            HirKind::Class(hir::Class::Unicode(ref cls)) => {
                if cls.ranges().is_empty() {
                    return self.wtr.write_str("[a&&b]");
                }
                self.wtr.write_str("[")?;
                for range in cls.iter() {
                    if range.start() == range.end() {
                        self.write_literal_char(range.start())?;
                    } else if u32::from(range.start()) + 1 == u32::from(range.end()) {
                        self.write_literal_char(range.start())?;
                        self.write_literal_char(range.end())?;
                    } else {
                        self.write_literal_char(range.start())?;
                        self.wtr.write_str("-")?;
                        self.write_literal_char(range.end())?;
                    }
                }
                self.wtr.write_str("]")?;
            }
            HirKind::Class(hir::Class::Bytes(ref cls)) => {
                if cls.ranges().is_empty() {
                    return self.wtr.write_str("[a&&b]");
                }
                self.wtr.write_str("(?-u:[")?;
                for range in cls.iter() {
                    if range.start() == range.end() {
                        self.write_literal_class_byte(range.start())?;
                    } else if range.start() + 1 == range.end() {
                        self.write_literal_class_byte(range.start())?;
                        self.write_literal_class_byte(range.end())?;
                    } else {
                        self.write_literal_class_byte(range.start())?;
                        self.wtr.write_str("-")?;
                        self.write_literal_class_byte(range.end())?;
                    }
                }
                self.wtr.write_str("])")?;
            }
            HirKind::Look(ref look) => {
                match *look {
                    hir::Look::Start => {
                        self.wtr.write_str(r"\A")?;
                    }
                    hir::Look::End => {
                        self.wtr.write_str(r"\z")?;
                    }
                    hir::Look::StartLF => {
                        self.wtr.write_str("(?m:^)")?;
                    }
                    hir::Look::EndLF => {
                        self.wtr.write_str("(?m:$)")?;
                    }
                    hir::Look::StartCRLF => {
                        self.wtr.write_str("(?mR:^)")?;
                    }
                    hir::Look::EndCRLF => {
                        self.wtr.write_str("(?mR:$)")?;
                    }
                    hir::Look::WordAscii => {
                        self.wtr.write_str(r"(?-u:\b)")?;
                    }
                    hir::Look::WordAsciiNegate => {
                        self.wtr.write_str(r"(?-u:\B)")?;
                    }
                    hir::Look::WordUnicode => {
                        self.wtr.write_str(r"\b")?;
                    }
                    hir::Look::WordUnicodeNegate => {
                        self.wtr.write_str(r"\B")?;
                    }
                }
            }
            HirKind::Capture(hir::Capture { ref name, .. }) => {
                self.wtr.write_str("(")?;
                if let Some(ref name) = *name {
                    write!(self.wtr, "?P<{}>", name)?;
                }
            }
            HirKind::Concat(_) | HirKind::Alternation(_) => {
                self.wtr.write_str(r"(?:")?;
            }
        }
        Ok(())
    }
    fn visit_post(&mut self, hir: &Hir) -> fmt::Result {
        match *hir.kind() {
            HirKind::Empty
            | HirKind::Literal(_)
            | HirKind::Class(_)
            | HirKind::Look(_) => {}
            HirKind::Repetition(ref x) => {
                match (x.min, x.max) {
                    (0, Some(1)) => {
                        self.wtr.write_str("?")?;
                    }
                    (0, None) => {
                        self.wtr.write_str("*")?;
                    }
                    (1, None) => {
                        self.wtr.write_str("+")?;
                    }
                    (1, Some(1)) => {
                        return Ok(());
                    }
                    (m, None) => {
                        write!(self.wtr, "{{{},}}", m)?;
                    }
                    (m, Some(n)) if m == n => {
                        write!(self.wtr, "{{{}}}", m)?;
                        return Ok(());
                    }
                    (m, Some(n)) => {
                        write!(self.wtr, "{{{},{}}}", m, n)?;
                    }
                }
                if !x.greedy {
                    self.wtr.write_str("?")?;
                }
            }
            HirKind::Capture(_) | HirKind::Concat(_) | HirKind::Alternation(_) => {
                self.wtr.write_str(r")")?;
            }
        }
        Ok(())
    }
    fn visit_alternation_in(&mut self) -> fmt::Result {
        self.wtr.write_str("|")
    }
}
impl<W: fmt::Write> Writer<W> {
    fn write_literal_char(&mut self, c: char) -> fmt::Result {
        if is_meta_character(c) {
            self.wtr.write_str("\\")?;
        }
        self.wtr.write_char(c)
    }
    fn write_literal_byte(&mut self, b: u8) -> fmt::Result {
        if b <= 0x7F && !b.is_ascii_control() && !b.is_ascii_whitespace() {
            self.write_literal_char(char::try_from(b).unwrap())
        } else {
            write!(self.wtr, "(?-u:\\x{:02X})", b)
        }
    }
    fn write_literal_class_byte(&mut self, b: u8) -> fmt::Result {
        if b <= 0x7F && !b.is_ascii_control() && !b.is_ascii_whitespace() {
            self.write_literal_char(char::try_from(b).unwrap())
        } else {
            write!(self.wtr, "\\x{:02X}", b)
        }
    }
}
#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, string::{String, ToString}};
    use crate::ParserBuilder;
    use super::*;
    fn roundtrip(given: &str, expected: &str) {
        roundtrip_with(|b| b, given, expected);
    }
    fn roundtrip_bytes(given: &str, expected: &str) {
        roundtrip_with(|b| b.utf8(false), given, expected);
    }
    fn roundtrip_with<F>(mut f: F, given: &str, expected: &str)
    where
        F: FnMut(&mut ParserBuilder) -> &mut ParserBuilder,
    {
        let mut builder = ParserBuilder::new();
        f(&mut builder);
        let hir = builder.build().parse(given).unwrap();
        let mut printer = Printer::new();
        let mut dst = String::new();
        printer.print(&hir, &mut dst).unwrap();
        builder.build().parse(&dst).unwrap();
        assert_eq!(expected, dst);
    }
    #[test]
    fn print_literal() {
        roundtrip("a", "a");
        roundtrip(r"\xff", "\u{FF}");
        roundtrip_bytes(r"\xff", "\u{FF}");
        roundtrip_bytes(r"(?-u)\xff", r"(?-u:\xFF)");
        roundtrip("☃", "☃");
    }
    #[test]
    fn print_class() {
        roundtrip(r"[a]", r"a");
        roundtrip(r"[ab]", r"[ab]");
        roundtrip(r"[a-z]", r"[a-z]");
        roundtrip(r"[a-z--b-c--x-y]", r"[ad-wz]");
        roundtrip(r"[^\x01-\u{10FFFF}]", "\u{0}");
        roundtrip(r"[-]", r"\-");
        roundtrip(r"[☃-⛄]", r"[☃-⛄]");
        roundtrip(r"(?-u)[a]", r"a");
        roundtrip(r"(?-u)[ab]", r"(?-u:[ab])");
        roundtrip(r"(?-u)[a-z]", r"(?-u:[a-z])");
        roundtrip_bytes(r"(?-u)[a-\xFF]", r"(?-u:[a-\xFF])");
        roundtrip(r"[\[]", r"\[");
        roundtrip(r"[Z-_]", r"[Z-_]");
        roundtrip(r"[Z-_--Z]", r"[\[-_]");
        roundtrip_bytes(r"(?-u)[\[]", r"\[");
        roundtrip_bytes(r"(?-u)[Z-_]", r"(?-u:[Z-_])");
        roundtrip_bytes(r"(?-u)[Z-_--Z]", r"(?-u:[\[-_])");
        #[cfg(feature = "unicode-gencat")] roundtrip(r"\P{any}", r"[a&&b]");
        roundtrip_bytes(r"(?-u)[^\x00-\xFF]", r"[a&&b]");
    }
    #[test]
    fn print_anchor() {
        roundtrip(r"^", r"\A");
        roundtrip(r"$", r"\z");
        roundtrip(r"(?m)^", r"(?m:^)");
        roundtrip(r"(?m)$", r"(?m:$)");
    }
    #[test]
    fn print_word_boundary() {
        roundtrip(r"\b", r"\b");
        roundtrip(r"\B", r"\B");
        roundtrip(r"(?-u)\b", r"(?-u:\b)");
        roundtrip_bytes(r"(?-u)\B", r"(?-u:\B)");
    }
    #[test]
    fn print_repetition() {
        roundtrip("a?", "a?");
        roundtrip("a??", "a??");
        roundtrip("(?U)a?", "a??");
        roundtrip("a*", "a*");
        roundtrip("a*?", "a*?");
        roundtrip("(?U)a*", "a*?");
        roundtrip("a+", "a+");
        roundtrip("a+?", "a+?");
        roundtrip("(?U)a+", "a+?");
        roundtrip("a{1}", "a");
        roundtrip("a{2}", "a{2}");
        roundtrip("a{1,}", "a+");
        roundtrip("a{1,5}", "a{1,5}");
        roundtrip("a{1}?", "a");
        roundtrip("a{2}?", "a{2}");
        roundtrip("a{1,}?", "a+?");
        roundtrip("a{1,5}?", "a{1,5}?");
        roundtrip("(?U)a{1}", "a");
        roundtrip("(?U)a{2}", "a{2}");
        roundtrip("(?U)a{1,}", "a+?");
        roundtrip("(?U)a{1,5}", "a{1,5}?");
        roundtrip("a{0}", "");
        roundtrip("(?:ab){0}", "");
        #[cfg(feature = "unicode-gencat")]
        {
            roundtrip(r"\p{any}{0}", "");
            roundtrip(r"\P{any}{0}", "");
        }
    }
    #[test]
    fn print_group() {
        roundtrip("()", "()");
        roundtrip("(?P<foo>)", "(?P<foo>)");
        roundtrip("(?:)", "");
        roundtrip("(a)", "(a)");
        roundtrip("(?P<foo>a)", "(?P<foo>a)");
        roundtrip("(?:a)", "a");
        roundtrip("((((a))))", "((((a))))");
    }
    #[test]
    fn print_alternation() {
        roundtrip("|", "(?:|)");
        roundtrip("||", "(?:||)");
        roundtrip("a|b", "[ab]");
        roundtrip("ab|cd", "(?:(?:ab)|(?:cd))");
        roundtrip("a|b|c", "[a-c]");
        roundtrip("ab|cd|ef", "(?:(?:ab)|(?:cd)|(?:ef))");
        roundtrip("foo|bar|quux", "(?:(?:foo)|(?:bar)|(?:quux))");
    }
    #[test]
    fn regression_repetition_concat() {
        let expr = Hir::concat(
            alloc::vec![
                Hir::literal("x".as_bytes()), Hir::repetition(hir::Repetition { min : 1,
                max : None, greedy : true, sub : Box::new(Hir::literal("ab".as_bytes())),
                }), Hir::literal("y".as_bytes()),
            ],
        );
        assert_eq!(r"(?:x(?:ab)+y)", expr.to_string());
        let expr = Hir::concat(
            alloc::vec![
                Hir::look(hir::Look::Start), Hir::repetition(hir::Repetition { min : 1,
                max : None, greedy : true, sub :
                Box::new(Hir::concat(alloc::vec![Hir::look(hir::Look::Start),
                Hir::look(hir::Look::End),])), }), Hir::look(hir::Look::End),
            ],
        );
        assert_eq!(r"(?:\A(?:\A\z)+\z)", expr.to_string());
    }
    #[test]
    fn regression_repetition_alternation() {
        let expr = Hir::concat(
            alloc::vec![
                Hir::literal("ab".as_bytes()), Hir::repetition(hir::Repetition { min : 1,
                max : None, greedy : true, sub :
                Box::new(Hir::alternation(alloc::vec![Hir::literal("cd".as_bytes()),
                Hir::literal("ef".as_bytes()),])), }), Hir::literal("gh".as_bytes()),
            ],
        );
        assert_eq!(r"(?:(?:ab)(?:(?:cd)|(?:ef))+(?:gh))", expr.to_string());
        let expr = Hir::concat(
            alloc::vec![
                Hir::look(hir::Look::Start), Hir::repetition(hir::Repetition { min : 1,
                max : None, greedy : true, sub :
                Box::new(Hir::alternation(alloc::vec![Hir::look(hir::Look::Start),
                Hir::look(hir::Look::End),])), }), Hir::look(hir::Look::End),
            ],
        );
        assert_eq!(r"(?:\A(?:\A|\z)+\z)", expr.to_string());
    }
    #[test]
    fn regression_alternation_concat() {
        let expr = Hir::concat(
            alloc::vec![
                Hir::literal("ab".as_bytes()),
                Hir::alternation(alloc::vec![Hir::literal("mn".as_bytes()),
                Hir::literal("xy".as_bytes()),]),
            ],
        );
        assert_eq!(r"(?:(?:ab)(?:(?:mn)|(?:xy)))", expr.to_string());
        let expr = Hir::concat(
            alloc::vec![
                Hir::look(hir::Look::Start),
                Hir::alternation(alloc::vec![Hir::look(hir::Look::Start),
                Hir::look(hir::Look::End),]),
            ],
        );
        assert_eq!(r"(?:\A(?:\A|\z))", expr.to_string());
    }
}
#[cfg(test)]
mod tests_rug_593 {
    use super::*;
    use crate::hir::print::PrinterBuilder;
    use std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_593_rrrruuuugggg_test_rug = 0;
        let default_printer_builder: PrinterBuilder = <PrinterBuilder as Default>::default();
        let _rug_ed_tests_rug_593_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_594 {
    use super::*;
    use crate::hir::print::PrinterBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_594_rrrruuuugggg_test_rug = 0;
        PrinterBuilder::new();
        let _rug_ed_tests_rug_594_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_595 {
    use super::*;
    use crate::hir::print::PrinterBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_595_rrrruuuugggg_test_rug = 0;
        let mut p0 = PrinterBuilder::new();
        PrinterBuilder::build(&p0);
        let _rug_ed_tests_rug_595_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_596 {
    use super::*;
    use crate::hir::print::{Printer, PrinterBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_596_rrrruuuugggg_test_rug = 0;
        let printer: Printer = PrinterBuilder::new().build();
        Printer::new();
        let _rug_ed_tests_rug_596_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_604 {
    use super::*;
    use std::fmt;
    struct MockWriter;
    impl fmt::Write for MockWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            Ok(())
        }
    }
    #[test]
    fn test_write_literal_class_byte() {
        let _rug_st_tests_rug_604_rrrruuuugggg_test_write_literal_class_byte = 0;
        let rug_fuzz_0 = 65;
        let mut writer = Writer { wtr: MockWriter };
        let b: u8 = rug_fuzz_0;
        writer.write_literal_class_byte(b).unwrap();
        let _rug_ed_tests_rug_604_rrrruuuugggg_test_write_literal_class_byte = 0;
    }
}
