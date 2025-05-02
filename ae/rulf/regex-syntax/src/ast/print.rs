/*!
This module provides a regular expression printer for `Ast`.
*/
use std::fmt;
use ast::visitor::{self, Visitor};
use ast::{self, Ast};
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
/// A printer for a regular expression abstract syntax tree.
///
/// A printer converts an abstract syntax tree (AST) to a regular expression
/// pattern string. This particular printer uses constant stack space and heap
/// space proportional to the size of the AST.
///
/// This printer will not necessarily preserve the original formatting of the
/// regular expression pattern string. For example, all whitespace and comments
/// are ignored.
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
    pub fn print<W: fmt::Write>(&mut self, ast: &Ast, wtr: W) -> fmt::Result {
        visitor::visit(ast, Writer { printer: self, wtr: wtr })
    }
}
#[derive(Debug)]
struct Writer<'p, W> {
    printer: &'p mut Printer,
    wtr: W,
}
impl<'p, W: fmt::Write> Visitor for Writer<'p, W> {
    type Output = ();
    type Err = fmt::Error;
    fn finish(self) -> fmt::Result {
        Ok(())
    }
    fn visit_pre(&mut self, ast: &Ast) -> fmt::Result {
        match *ast {
            Ast::Group(ref x) => self.fmt_group_pre(x),
            Ast::Class(ast::Class::Bracketed(ref x)) => self.fmt_class_bracketed_pre(x),
            _ => Ok(()),
        }
    }
    fn visit_post(&mut self, ast: &Ast) -> fmt::Result {
        use ast::Class;
        match *ast {
            Ast::Empty(_) => Ok(()),
            Ast::Flags(ref x) => self.fmt_set_flags(x),
            Ast::Literal(ref x) => self.fmt_literal(x),
            Ast::Dot(_) => self.wtr.write_str("."),
            Ast::Assertion(ref x) => self.fmt_assertion(x),
            Ast::Class(Class::Perl(ref x)) => self.fmt_class_perl(x),
            Ast::Class(Class::Unicode(ref x)) => self.fmt_class_unicode(x),
            Ast::Class(Class::Bracketed(ref x)) => self.fmt_class_bracketed_post(x),
            Ast::Repetition(ref x) => self.fmt_repetition(x),
            Ast::Group(ref x) => self.fmt_group_post(x),
            Ast::Alternation(_) => Ok(()),
            Ast::Concat(_) => Ok(()),
        }
    }
    fn visit_alternation_in(&mut self) -> fmt::Result {
        self.wtr.write_str("|")
    }
    fn visit_class_set_item_pre(
        &mut self,
        ast: &ast::ClassSetItem,
    ) -> Result<(), Self::Err> {
        match *ast {
            ast::ClassSetItem::Bracketed(ref x) => self.fmt_class_bracketed_pre(x),
            _ => Ok(()),
        }
    }
    fn visit_class_set_item_post(
        &mut self,
        ast: &ast::ClassSetItem,
    ) -> Result<(), Self::Err> {
        use ast::ClassSetItem::*;
        match *ast {
            Empty(_) => Ok(()),
            Literal(ref x) => self.fmt_literal(x),
            Range(ref x) => {
                self.fmt_literal(&x.start)?;
                self.wtr.write_str("-")?;
                self.fmt_literal(&x.end)?;
                Ok(())
            }
            Ascii(ref x) => self.fmt_class_ascii(x),
            Unicode(ref x) => self.fmt_class_unicode(x),
            Perl(ref x) => self.fmt_class_perl(x),
            Bracketed(ref x) => self.fmt_class_bracketed_post(x),
            Union(_) => Ok(()),
        }
    }
    fn visit_class_set_binary_op_in(
        &mut self,
        ast: &ast::ClassSetBinaryOp,
    ) -> Result<(), Self::Err> {
        self.fmt_class_set_binary_op_kind(&ast.kind)
    }
}
impl<'p, W: fmt::Write> Writer<'p, W> {
    fn fmt_group_pre(&mut self, ast: &ast::Group) -> fmt::Result {
        use ast::GroupKind::*;
        match ast.kind {
            CaptureIndex(_) => self.wtr.write_str("("),
            CaptureName(ref x) => {
                self.wtr.write_str("(?P<")?;
                self.wtr.write_str(&x.name)?;
                self.wtr.write_str(">")?;
                Ok(())
            }
            NonCapturing(ref flags) => {
                self.wtr.write_str("(?")?;
                self.fmt_flags(flags)?;
                self.wtr.write_str(":")?;
                Ok(())
            }
        }
    }
    fn fmt_group_post(&mut self, _ast: &ast::Group) -> fmt::Result {
        self.wtr.write_str(")")
    }
    fn fmt_repetition(&mut self, ast: &ast::Repetition) -> fmt::Result {
        use ast::RepetitionKind::*;
        match ast.op.kind {
            ZeroOrOne if ast.greedy => self.wtr.write_str("?"),
            ZeroOrOne => self.wtr.write_str("??"),
            ZeroOrMore if ast.greedy => self.wtr.write_str("*"),
            ZeroOrMore => self.wtr.write_str("*?"),
            OneOrMore if ast.greedy => self.wtr.write_str("+"),
            OneOrMore => self.wtr.write_str("+?"),
            Range(ref x) => {
                self.fmt_repetition_range(x)?;
                if !ast.greedy {
                    self.wtr.write_str("?")?;
                }
                Ok(())
            }
        }
    }
    fn fmt_repetition_range(&mut self, ast: &ast::RepetitionRange) -> fmt::Result {
        use ast::RepetitionRange::*;
        match *ast {
            Exactly(x) => write!(self.wtr, "{{{}}}", x),
            AtLeast(x) => write!(self.wtr, "{{{},}}", x),
            Bounded(x, y) => write!(self.wtr, "{{{},{}}}", x, y),
        }
    }
    fn fmt_literal(&mut self, ast: &ast::Literal) -> fmt::Result {
        use ast::LiteralKind::*;
        match ast.kind {
            Verbatim => self.wtr.write_char(ast.c),
            Punctuation => write!(self.wtr, r"\{}", ast.c),
            Octal => write!(self.wtr, r"\{:o}", ast.c as u32),
            HexFixed(ast::HexLiteralKind::X) => {
                write!(self.wtr, r"\x{:02X}", ast.c as u32)
            }
            HexFixed(ast::HexLiteralKind::UnicodeShort) => {
                write!(self.wtr, r"\u{:04X}", ast.c as u32)
            }
            HexFixed(ast::HexLiteralKind::UnicodeLong) => {
                write!(self.wtr, r"\U{:08X}", ast.c as u32)
            }
            HexBrace(ast::HexLiteralKind::X) => {
                write!(self.wtr, r"\x{{{:X}}}", ast.c as u32)
            }
            HexBrace(ast::HexLiteralKind::UnicodeShort) => {
                write!(self.wtr, r"\u{{{:X}}}", ast.c as u32)
            }
            HexBrace(ast::HexLiteralKind::UnicodeLong) => {
                write!(self.wtr, r"\U{{{:X}}}", ast.c as u32)
            }
            Special(ast::SpecialLiteralKind::Bell) => self.wtr.write_str(r"\a"),
            Special(ast::SpecialLiteralKind::FormFeed) => self.wtr.write_str(r"\f"),
            Special(ast::SpecialLiteralKind::Tab) => self.wtr.write_str(r"\t"),
            Special(ast::SpecialLiteralKind::LineFeed) => self.wtr.write_str(r"\n"),
            Special(ast::SpecialLiteralKind::CarriageReturn) => self.wtr.write_str(r"\r"),
            Special(ast::SpecialLiteralKind::VerticalTab) => self.wtr.write_str(r"\v"),
            Special(ast::SpecialLiteralKind::Space) => self.wtr.write_str(r"\ "),
        }
    }
    fn fmt_assertion(&mut self, ast: &ast::Assertion) -> fmt::Result {
        use ast::AssertionKind::*;
        match ast.kind {
            StartLine => self.wtr.write_str("^"),
            EndLine => self.wtr.write_str("$"),
            StartText => self.wtr.write_str(r"\A"),
            EndText => self.wtr.write_str(r"\z"),
            WordBoundary => self.wtr.write_str(r"\b"),
            NotWordBoundary => self.wtr.write_str(r"\B"),
        }
    }
    fn fmt_set_flags(&mut self, ast: &ast::SetFlags) -> fmt::Result {
        self.wtr.write_str("(?")?;
        self.fmt_flags(&ast.flags)?;
        self.wtr.write_str(")")?;
        Ok(())
    }
    fn fmt_flags(&mut self, ast: &ast::Flags) -> fmt::Result {
        use ast::{Flag, FlagsItemKind};
        for item in &ast.items {
            match item.kind {
                FlagsItemKind::Negation => self.wtr.write_str("-"),
                FlagsItemKind::Flag(ref flag) => {
                    match *flag {
                        Flag::CaseInsensitive => self.wtr.write_str("i"),
                        Flag::MultiLine => self.wtr.write_str("m"),
                        Flag::DotMatchesNewLine => self.wtr.write_str("s"),
                        Flag::SwapGreed => self.wtr.write_str("U"),
                        Flag::Unicode => self.wtr.write_str("u"),
                        Flag::IgnoreWhitespace => self.wtr.write_str("x"),
                    }
                }
            }?;
        }
        Ok(())
    }
    fn fmt_class_bracketed_pre(&mut self, ast: &ast::ClassBracketed) -> fmt::Result {
        if ast.negated { self.wtr.write_str("[^") } else { self.wtr.write_str("[") }
    }
    fn fmt_class_bracketed_post(&mut self, _ast: &ast::ClassBracketed) -> fmt::Result {
        self.wtr.write_str("]")
    }
    fn fmt_class_set_binary_op_kind(
        &mut self,
        ast: &ast::ClassSetBinaryOpKind,
    ) -> fmt::Result {
        use ast::ClassSetBinaryOpKind::*;
        match *ast {
            Intersection => self.wtr.write_str("&&"),
            Difference => self.wtr.write_str("--"),
            SymmetricDifference => self.wtr.write_str("~~"),
        }
    }
    fn fmt_class_perl(&mut self, ast: &ast::ClassPerl) -> fmt::Result {
        use ast::ClassPerlKind::*;
        match ast.kind {
            Digit if ast.negated => self.wtr.write_str(r"\D"),
            Digit => self.wtr.write_str(r"\d"),
            Space if ast.negated => self.wtr.write_str(r"\S"),
            Space => self.wtr.write_str(r"\s"),
            Word if ast.negated => self.wtr.write_str(r"\W"),
            Word => self.wtr.write_str(r"\w"),
        }
    }
    fn fmt_class_ascii(&mut self, ast: &ast::ClassAscii) -> fmt::Result {
        use ast::ClassAsciiKind::*;
        match ast.kind {
            Alnum if ast.negated => self.wtr.write_str("[:^alnum:]"),
            Alnum => self.wtr.write_str("[:alnum:]"),
            Alpha if ast.negated => self.wtr.write_str("[:^alpha:]"),
            Alpha => self.wtr.write_str("[:alpha:]"),
            Ascii if ast.negated => self.wtr.write_str("[:^ascii:]"),
            Ascii => self.wtr.write_str("[:ascii:]"),
            Blank if ast.negated => self.wtr.write_str("[:^blank:]"),
            Blank => self.wtr.write_str("[:blank:]"),
            Cntrl if ast.negated => self.wtr.write_str("[:^cntrl:]"),
            Cntrl => self.wtr.write_str("[:cntrl:]"),
            Digit if ast.negated => self.wtr.write_str("[:^digit:]"),
            Digit => self.wtr.write_str("[:digit:]"),
            Graph if ast.negated => self.wtr.write_str("[:^graph:]"),
            Graph => self.wtr.write_str("[:graph:]"),
            Lower if ast.negated => self.wtr.write_str("[:^lower:]"),
            Lower => self.wtr.write_str("[:lower:]"),
            Print if ast.negated => self.wtr.write_str("[:^print:]"),
            Print => self.wtr.write_str("[:print:]"),
            Punct if ast.negated => self.wtr.write_str("[:^punct:]"),
            Punct => self.wtr.write_str("[:punct:]"),
            Space if ast.negated => self.wtr.write_str("[:^space:]"),
            Space => self.wtr.write_str("[:space:]"),
            Upper if ast.negated => self.wtr.write_str("[:^upper:]"),
            Upper => self.wtr.write_str("[:upper:]"),
            Word if ast.negated => self.wtr.write_str("[:^word:]"),
            Word => self.wtr.write_str("[:word:]"),
            Xdigit if ast.negated => self.wtr.write_str("[:^xdigit:]"),
            Xdigit => self.wtr.write_str("[:xdigit:]"),
        }
    }
    fn fmt_class_unicode(&mut self, ast: &ast::ClassUnicode) -> fmt::Result {
        use ast::ClassUnicodeKind::*;
        use ast::ClassUnicodeOpKind::*;
        if ast.negated {
            self.wtr.write_str(r"\P")?;
        } else {
            self.wtr.write_str(r"\p")?;
        }
        match ast.kind {
            OneLetter(c) => self.wtr.write_char(c),
            Named(ref x) => write!(self.wtr, "{{{}}}", x),
            NamedValue { op: Equal, ref name, ref value } => {
                write!(self.wtr, "{{{}={}}}", name, value)
            }
            NamedValue { op: Colon, ref name, ref value } => {
                write!(self.wtr, "{{{}:{}}}", name, value)
            }
            NamedValue { op: NotEqual, ref name, ref value } => {
                write!(self.wtr, "{{{}!={}}}", name, value)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::Printer;
    use ast::parse::ParserBuilder;
    fn roundtrip(given: &str) {
        roundtrip_with(|b| b, given);
    }
    fn roundtrip_with<F>(mut f: F, given: &str)
    where
        F: FnMut(&mut ParserBuilder) -> &mut ParserBuilder,
    {
        let mut builder = ParserBuilder::new();
        f(&mut builder);
        let ast = builder.build().parse(given).unwrap();
        let mut printer = Printer::new();
        let mut dst = String::new();
        printer.print(&ast, &mut dst).unwrap();
        assert_eq!(given, dst);
    }
    #[test]
    fn print_literal() {
        roundtrip("a");
        roundtrip(r"\[");
        roundtrip_with(|b| b.octal(true), r"\141");
        roundtrip(r"\x61");
        roundtrip(r"\x7F");
        roundtrip(r"\u0061");
        roundtrip(r"\U00000061");
        roundtrip(r"\x{61}");
        roundtrip(r"\x{7F}");
        roundtrip(r"\u{61}");
        roundtrip(r"\U{61}");
        roundtrip(r"\a");
        roundtrip(r"\f");
        roundtrip(r"\t");
        roundtrip(r"\n");
        roundtrip(r"\r");
        roundtrip(r"\v");
        roundtrip(r"(?x)\ ");
    }
    #[test]
    fn print_dot() {
        roundtrip(".");
    }
    #[test]
    fn print_concat() {
        roundtrip("ab");
        roundtrip("abcde");
        roundtrip("a(bcd)ef");
    }
    #[test]
    fn print_alternation() {
        roundtrip("a|b");
        roundtrip("a|b|c|d|e");
        roundtrip("|a|b|c|d|e");
        roundtrip("|a|b|c|d|e|");
        roundtrip("a(b|c|d)|e|f");
    }
    #[test]
    fn print_assertion() {
        roundtrip(r"^");
        roundtrip(r"$");
        roundtrip(r"\A");
        roundtrip(r"\z");
        roundtrip(r"\b");
        roundtrip(r"\B");
    }
    #[test]
    fn print_repetition() {
        roundtrip("a?");
        roundtrip("a??");
        roundtrip("a*");
        roundtrip("a*?");
        roundtrip("a+");
        roundtrip("a+?");
        roundtrip("a{5}");
        roundtrip("a{5}?");
        roundtrip("a{5,}");
        roundtrip("a{5,}?");
        roundtrip("a{5,10}");
        roundtrip("a{5,10}?");
    }
    #[test]
    fn print_flags() {
        roundtrip("(?i)");
        roundtrip("(?-i)");
        roundtrip("(?s-i)");
        roundtrip("(?-si)");
        roundtrip("(?siUmux)");
    }
    #[test]
    fn print_group() {
        roundtrip("(?i:a)");
        roundtrip("(?P<foo>a)");
        roundtrip("(a)");
    }
    #[test]
    fn print_class() {
        roundtrip(r"[abc]");
        roundtrip(r"[a-z]");
        roundtrip(r"[^a-z]");
        roundtrip(r"[a-z0-9]");
        roundtrip(r"[-a-z0-9]");
        roundtrip(r"[-a-z0-9]");
        roundtrip(r"[a-z0-9---]");
        roundtrip(r"[a-z&&m-n]");
        roundtrip(r"[[a-z&&m-n]]");
        roundtrip(r"[a-z--m-n]");
        roundtrip(r"[a-z~~m-n]");
        roundtrip(r"[a-z[0-9]]");
        roundtrip(r"[a-z[^0-9]]");
        roundtrip(r"\d");
        roundtrip(r"\D");
        roundtrip(r"\s");
        roundtrip(r"\S");
        roundtrip(r"\w");
        roundtrip(r"\W");
        roundtrip(r"[[:alnum:]]");
        roundtrip(r"[[:^alnum:]]");
        roundtrip(r"[[:alpha:]]");
        roundtrip(r"[[:^alpha:]]");
        roundtrip(r"[[:ascii:]]");
        roundtrip(r"[[:^ascii:]]");
        roundtrip(r"[[:blank:]]");
        roundtrip(r"[[:^blank:]]");
        roundtrip(r"[[:cntrl:]]");
        roundtrip(r"[[:^cntrl:]]");
        roundtrip(r"[[:digit:]]");
        roundtrip(r"[[:^digit:]]");
        roundtrip(r"[[:graph:]]");
        roundtrip(r"[[:^graph:]]");
        roundtrip(r"[[:lower:]]");
        roundtrip(r"[[:^lower:]]");
        roundtrip(r"[[:print:]]");
        roundtrip(r"[[:^print:]]");
        roundtrip(r"[[:punct:]]");
        roundtrip(r"[[:^punct:]]");
        roundtrip(r"[[:space:]]");
        roundtrip(r"[[:^space:]]");
        roundtrip(r"[[:upper:]]");
        roundtrip(r"[[:^upper:]]");
        roundtrip(r"[[:word:]]");
        roundtrip(r"[[:^word:]]");
        roundtrip(r"[[:xdigit:]]");
        roundtrip(r"[[:^xdigit:]]");
        roundtrip(r"\pL");
        roundtrip(r"\PL");
        roundtrip(r"\p{L}");
        roundtrip(r"\P{L}");
        roundtrip(r"\p{X=Y}");
        roundtrip(r"\P{X=Y}");
        roundtrip(r"\p{X:Y}");
        roundtrip(r"\P{X:Y}");
        roundtrip(r"\p{X!=Y}");
        roundtrip(r"\P{X!=Y}");
    }
}
#[cfg(test)]
mod tests_llm_16_16 {
    use crate::ast::print::PrinterBuilder;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_16_rrrruuuugggg_test_default = 0;
        let builder: PrinterBuilder = Default::default();
        let _rug_ed_tests_llm_16_16_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_17 {
    use super::*;
    use crate::*;
    use std::fmt;
    #[derive(Debug)]
    struct DummyWriter;
    impl fmt::Write for DummyWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            Ok(())
        }
    }
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_17_rrrruuuugggg_test_finish = 0;
        let writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let result = writer.finish();
        debug_assert_eq!(result, Ok(()));
        let _rug_ed_tests_llm_16_17_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_232 {
    use super::*;
    use crate::*;
    use ast::*;
    use ast::AssertionKind::*;
    use std::fmt::Write;
    #[test]
    fn test_fmt_assertion_start_line() {
        let _rug_st_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_start_line = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "fmt_assertion failed";
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let ast = Assertion {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: StartLine,
        };
        writer.fmt_assertion(&ast).expect(rug_fuzz_6);
        debug_assert_eq!(writer.wtr, "^");
        let _rug_ed_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_start_line = 0;
    }
    #[test]
    fn test_fmt_assertion_end_line() {
        let _rug_st_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_end_line = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "fmt_assertion failed";
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let ast = Assertion {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: EndLine,
        };
        writer.fmt_assertion(&ast).expect(rug_fuzz_6);
        debug_assert_eq!(writer.wtr, "$");
        let _rug_ed_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_end_line = 0;
    }
    #[test]
    fn test_fmt_assertion_start_text() {
        let _rug_st_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_start_text = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "fmt_assertion failed";
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let ast = Assertion {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: StartText,
        };
        writer.fmt_assertion(&ast).expect(rug_fuzz_6);
        debug_assert_eq!(writer.wtr, r"\A");
        let _rug_ed_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_start_text = 0;
    }
    #[test]
    fn test_fmt_assertion_end_text() {
        let _rug_st_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_end_text = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "fmt_assertion failed";
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let ast = Assertion {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: EndText,
        };
        writer.fmt_assertion(&ast).expect(rug_fuzz_6);
        debug_assert_eq!(writer.wtr, r"\z");
        let _rug_ed_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_end_text = 0;
    }
    #[test]
    fn test_fmt_assertion_word_boundary() {
        let _rug_st_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_word_boundary = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "fmt_assertion failed";
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let ast = Assertion {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: WordBoundary,
        };
        writer.fmt_assertion(&ast).expect(rug_fuzz_6);
        debug_assert_eq!(writer.wtr, r"\b");
        let _rug_ed_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_word_boundary = 0;
    }
    #[test]
    fn test_fmt_assertion_not_word_boundary() {
        let _rug_st_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_not_word_boundary = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "fmt_assertion failed";
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let ast = Assertion {
            span: Span::new(
                Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: NotWordBoundary,
        };
        writer.fmt_assertion(&ast).expect(rug_fuzz_6);
        debug_assert_eq!(writer.wtr, r"\B");
        let _rug_ed_tests_llm_16_232_rrrruuuugggg_test_fmt_assertion_not_word_boundary = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_234 {
    use super::*;
    use crate::*;
    use ast::{ClassAscii, ClassAsciiKind, Span, Position};
    use ast::ClassAsciiKind::*;
    use ast::print::{Writer, Printer};
    use std::fmt;
    struct MockWriter;
    impl fmt::Write for MockWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            Ok(())
        }
    }
    #[test]
    fn test_fmt_class_ascii() {
        let _rug_st_tests_llm_16_234_rrrruuuugggg_test_fmt_class_ascii = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = false;
        let mut printer = Printer::new();
        let mock_writer = MockWriter;
        let mut writer = Writer::<'_, MockWriter> {
            printer: &mut printer,
            wtr: mock_writer,
        };
        let ast = ClassAscii {
            span: Span {
                start: Position {
                    offset: rug_fuzz_0,
                    line: rug_fuzz_1,
                    column: rug_fuzz_2,
                },
                end: Position {
                    offset: rug_fuzz_3,
                    line: rug_fuzz_4,
                    column: rug_fuzz_5,
                },
            },
            kind: Alnum,
            negated: rug_fuzz_6,
        };
        writer.fmt_class_ascii(&ast).unwrap();
        let _rug_ed_tests_llm_16_234_rrrruuuugggg_test_fmt_class_ascii = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_236 {
    use super::*;
    use crate::*;
    use crate::ast::*;
    #[test]
    fn test_fmt_class_bracketed_pre_negated() {
        let _rug_st_tests_llm_16_236_rrrruuuugggg_test_fmt_class_bracketed_pre_negated = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = true;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = false;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 'a';
        let mut printer = Printer::new();
        let ast = Ast::Class(
            Class::Bracketed(ClassBracketed {
                span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
                negated: rug_fuzz_3,
                kind: ClassSet::Item(
                    ClassSetItem::Bracketed(
                        Box::new(ClassBracketed {
                            span: Span::splat(
                                Position::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6),
                            ),
                            negated: rug_fuzz_7,
                            kind: ClassSet::Item(
                                ClassSetItem::Literal(Literal {
                                    span: Span::splat(
                                        Position::new(rug_fuzz_8, rug_fuzz_9, rug_fuzz_10),
                                    ),
                                    kind: LiteralKind::Verbatim,
                                    c: rug_fuzz_11,
                                }),
                            ),
                        }),
                    ),
                ),
            }),
        );
        let mut buf = String::new();
        printer.print(&ast, &mut buf).unwrap();
        debug_assert_eq!(buf, "[^a]");
        let _rug_ed_tests_llm_16_236_rrrruuuugggg_test_fmt_class_bracketed_pre_negated = 0;
    }
    #[test]
    fn test_fmt_class_bracketed_pre_not_negated() {
        let _rug_st_tests_llm_16_236_rrrruuuugggg_test_fmt_class_bracketed_pre_not_negated = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = false;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 'a';
        let mut printer = Printer::new();
        let ast = Ast::Class(
            Class::Bracketed(ClassBracketed {
                span: Span::splat(Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
                negated: rug_fuzz_3,
                kind: ClassSet::Item(
                    ClassSetItem::Bracketed(
                        Box::new(ClassBracketed {
                            span: Span::splat(
                                Position::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6),
                            ),
                            negated: rug_fuzz_7,
                            kind: ClassSet::Item(
                                ClassSetItem::Literal(Literal {
                                    span: Span::splat(
                                        Position::new(rug_fuzz_8, rug_fuzz_9, rug_fuzz_10),
                                    ),
                                    kind: LiteralKind::Verbatim,
                                    c: rug_fuzz_11,
                                }),
                            ),
                        }),
                    ),
                ),
            }),
        );
        let mut buf = String::new();
        printer.print(&ast, &mut buf).unwrap();
        debug_assert_eq!(buf, "[a]");
        let _rug_ed_tests_llm_16_236_rrrruuuugggg_test_fmt_class_bracketed_pre_not_negated = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_237 {
    use super::*;
    use crate::*;
    use std::fmt::Write;
    #[test]
    fn test_fmt_class_perl() {
        let _rug_st_tests_llm_16_237_rrrruuuugggg_test_fmt_class_perl = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = false;
        let rug_fuzz_7 = "Failed to format class perl";
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let ast = ast::ClassPerl {
            span: ast::Span::new(
                ast::Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                ast::Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            kind: ast::ClassPerlKind::Digit,
            negated: rug_fuzz_6,
        };
        writer.fmt_class_perl(&ast).expect(rug_fuzz_7);
        let result = writer.wtr;
        debug_assert_eq!(result, r"\d");
        let _rug_ed_tests_llm_16_237_rrrruuuugggg_test_fmt_class_perl = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_242 {
    use super::*;
    use crate::*;
    #[test]
    fn test_fmt_flags() {
        let _rug_st_tests_llm_16_242_rrrruuuugggg_test_fmt_flags = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let flags = ast::Flags {
            span: ast::Span::new(
                ast::Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2),
                ast::Position::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5),
            ),
            items: vec![
                ast::FlagsItem { kind : ast::FlagsItemKind::Negation, span :
                ast::Span::new(ast::Position::new(rug_fuzz_6, rug_fuzz_7, rug_fuzz_8),
                ast::Position::new(rug_fuzz_9, rug_fuzz_10, rug_fuzz_11),), },
                ast::FlagsItem { kind :
                ast::FlagsItemKind::Flag(ast::Flag::CaseInsensitive), span :
                ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0,
                0),), }, ast::FlagsItem { kind :
                ast::FlagsItemKind::Flag(ast::Flag::MultiLine), span :
                ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0,
                0),), }, ast::FlagsItem { kind :
                ast::FlagsItemKind::Flag(ast::Flag::DotMatchesNewLine), span :
                ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0,
                0),), }, ast::FlagsItem { kind :
                ast::FlagsItemKind::Flag(ast::Flag::SwapGreed), span :
                ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0,
                0),), }, ast::FlagsItem { kind :
                ast::FlagsItemKind::Flag(ast::Flag::Unicode), span :
                ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0,
                0),), }, ast::FlagsItem { kind :
                ast::FlagsItemKind::Flag(ast::Flag::IgnoreWhitespace), span :
                ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0,
                0),), }
            ],
        };
        writer.fmt_flags(&flags).unwrap();
        debug_assert_eq!(writer.wtr, "-imsuUx");
        let _rug_ed_tests_llm_16_242_rrrruuuugggg_test_fmt_flags = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_250 {
    use super::*;
    use crate::*;
    #[test]
    fn test_fmt_repetition_range_exactly() {
        let _rug_st_tests_llm_16_250_rrrruuuugggg_test_fmt_repetition_range_exactly = 0;
        let rug_fuzz_0 = 5;
        let mut printer = Printer::new();
        let ast = ast::RepetitionRange::Exactly(rug_fuzz_0);
        let mut writer = Writer {
            printer: &mut printer,
            wtr: String::new(),
        };
        writer.fmt_repetition_range(&ast);
        debug_assert_eq!(writer.wtr, "{5}");
        let _rug_ed_tests_llm_16_250_rrrruuuugggg_test_fmt_repetition_range_exactly = 0;
    }
    #[test]
    fn test_fmt_repetition_range_at_least() {
        let _rug_st_tests_llm_16_250_rrrruuuugggg_test_fmt_repetition_range_at_least = 0;
        let rug_fuzz_0 = 3;
        let mut printer = Printer::new();
        let ast = ast::RepetitionRange::AtLeast(rug_fuzz_0);
        let mut writer = Writer {
            printer: &mut printer,
            wtr: String::new(),
        };
        writer.fmt_repetition_range(&ast);
        debug_assert_eq!(writer.wtr, "{3,}");
        let _rug_ed_tests_llm_16_250_rrrruuuugggg_test_fmt_repetition_range_at_least = 0;
    }
    #[test]
    fn test_fmt_repetition_range_bounded() {
        let _rug_st_tests_llm_16_250_rrrruuuugggg_test_fmt_repetition_range_bounded = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 5;
        let mut printer = Printer::new();
        let ast = ast::RepetitionRange::Bounded(rug_fuzz_0, rug_fuzz_1);
        let mut writer = Writer {
            printer: &mut printer,
            wtr: String::new(),
        };
        writer.fmt_repetition_range(&ast);
        debug_assert_eq!(writer.wtr, "{2,5}");
        let _rug_ed_tests_llm_16_250_rrrruuuugggg_test_fmt_repetition_range_bounded = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_251 {
    use super::*;
    use crate::*;
    use ast::*;
    #[test]
    fn test_fmt_set_flags() {
        let _rug_st_tests_llm_16_251_rrrruuuugggg_test_fmt_set_flags = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 2;
        let rug_fuzz_9 = 3;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 4;
        let rug_fuzz_12 = 1;
        let rug_fuzz_13 = 1;
        let rug_fuzz_14 = 2;
        let rug_fuzz_15 = 2;
        let rug_fuzz_16 = 1;
        let rug_fuzz_17 = 3;
        let rug_fuzz_18 = "Failed to format flags";
        let rug_fuzz_19 = "(?im)";
        let ast = SetFlags {
            span: Span {
                start: Position {
                    offset: rug_fuzz_0,
                    line: rug_fuzz_1,
                    column: rug_fuzz_2,
                },
                end: Position {
                    offset: rug_fuzz_3,
                    line: rug_fuzz_4,
                    column: rug_fuzz_5,
                },
            },
            flags: Flags {
                span: Span {
                    start: Position {
                        offset: rug_fuzz_6,
                        line: rug_fuzz_7,
                        column: rug_fuzz_8,
                    },
                    end: Position {
                        offset: rug_fuzz_9,
                        line: rug_fuzz_10,
                        column: rug_fuzz_11,
                    },
                },
                items: vec![
                    FlagsItem { kind : FlagsItemKind::Flag(Flag::CaseInsensitive), span :
                    Span { start : Position { offset : rug_fuzz_12, line : rug_fuzz_13,
                    column : rug_fuzz_14, }, end : Position { offset : rug_fuzz_15, line
                    : rug_fuzz_16, column : rug_fuzz_17, }, }, }, FlagsItem { kind :
                    FlagsItemKind::Flag(Flag::MultiLine), span : Span { start : Position
                    { offset : 2, line : 1, column : 3, }, end : Position { offset : 3,
                    line : 1, column : 4, }, }, }
                ],
            },
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_set_flags(&ast).expect(rug_fuzz_18);
        let expected = rug_fuzz_19;
        debug_assert_eq!(writer.wtr, expected);
        let _rug_ed_tests_llm_16_251_rrrruuuugggg_test_fmt_set_flags = 0;
    }
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::ast::print::PrinterBuilder;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_249_rrrruuuugggg_test_rug = 0;
        let printer_builder: PrinterBuilder = PrinterBuilder::new();
        let _rug_ed_tests_rug_249_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_250 {
    use super::*;
    use crate::ast::print::PrinterBuilder;
    #[test]
    fn test_build() {
        let _rug_st_tests_rug_250_rrrruuuugggg_test_build = 0;
        let mut p0: PrinterBuilder = PrinterBuilder {
            ..Default::default()
        };
        let _ = <ast::print::PrinterBuilder>::build(&p0);
        let _rug_ed_tests_rug_250_rrrruuuugggg_test_build = 0;
    }
}
#[cfg(test)]
mod tests_rug_251 {
    use super::*;
    use crate::ast::print::{Printer, PrinterBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_251_rrrruuuugggg_test_rug = 0;
        let result: Printer = <ast::print::Printer>::new();
        let _rug_ed_tests_rug_251_rrrruuuugggg_test_rug = 0;
    }
}
