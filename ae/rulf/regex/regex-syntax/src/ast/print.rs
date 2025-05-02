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
            Ast::Class(ast::Class::Bracketed(ref x)) => {
                self.fmt_class_bracketed_pre(x)
            }
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
            Ast::Class(Class::Bracketed(ref x)) => {
                self.fmt_class_bracketed_post(x)
            }
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
            ast::ClassSetItem::Bracketed(ref x) => {
                self.fmt_class_bracketed_pre(x)
            }
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

    fn fmt_repetition_range(
        &mut self,
        ast: &ast::RepetitionRange,
    ) -> fmt::Result {
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
            Special(ast::SpecialLiteralKind::Bell) => {
                self.wtr.write_str(r"\a")
            }
            Special(ast::SpecialLiteralKind::FormFeed) => {
                self.wtr.write_str(r"\f")
            }
            Special(ast::SpecialLiteralKind::Tab) => self.wtr.write_str(r"\t"),
            Special(ast::SpecialLiteralKind::LineFeed) => {
                self.wtr.write_str(r"\n")
            }
            Special(ast::SpecialLiteralKind::CarriageReturn) => {
                self.wtr.write_str(r"\r")
            }
            Special(ast::SpecialLiteralKind::VerticalTab) => {
                self.wtr.write_str(r"\v")
            }
            Special(ast::SpecialLiteralKind::Space) => {
                self.wtr.write_str(r"\ ")
            }
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
                FlagsItemKind::Flag(ref flag) => match *flag {
                    Flag::CaseInsensitive => self.wtr.write_str("i"),
                    Flag::MultiLine => self.wtr.write_str("m"),
                    Flag::DotMatchesNewLine => self.wtr.write_str("s"),
                    Flag::SwapGreed => self.wtr.write_str("U"),
                    Flag::Unicode => self.wtr.write_str("u"),
                    Flag::IgnoreWhitespace => self.wtr.write_str("x"),
                },
            }?;
        }
        Ok(())
    }

    fn fmt_class_bracketed_pre(
        &mut self,
        ast: &ast::ClassBracketed,
    ) -> fmt::Result {
        if ast.negated {
            self.wtr.write_str("[^")
        } else {
            self.wtr.write_str("[")
        }
    }

    fn fmt_class_bracketed_post(
        &mut self,
        _ast: &ast::ClassBracketed,
    ) -> fmt::Result {
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
#[test]
fn test_visit_alternation_in() {
    use ast::Ast;
    use ast::print::Printer;
    use ast::print::Writer;
    use ast::visitor::Visitor;
    use std::fmt::Write;
    use std::fmt;

    struct FakeWriter {
        count: usize
    }

    impl Write for FakeWriter {
        fn write_str(&mut self, _: &str) -> fmt::Result { 
            self.count += 1;
            Ok(())
        }
    }

    impl fmt::Debug for FakeWriter {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "FakeWriter({})", self.count)
        }
    }

    let ast = Ast::Alternation;
    let mut writer = Writer {
        printer: &mut Printer::new(),
        wtr: FakeWriter { count: 0 },
    };

    writer.visit_alternation_in().unwrap();

    assert_eq!(writer.wtr.count, 1);
}#[cfg(test)]
mod tests_llm_16_23 {
    use super::*;

use crate::*;

    struct MockWriter;
    impl fmt::Write for MockWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            Ok(())
        }
    }

    #[test]
    fn test_visit_class_set_item_post() {
        let ast = ast::ClassSetItem::Literal(ast::Literal {
            span: ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0, 0)),
            kind: ast::LiteralKind::Verbatim,
            c: 'a',
        });

        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: MockWriter,
        };

        let result = writer.visit_class_set_item_post(&ast);

        assert_eq!(result, Ok(()));
    }
}#[cfg(test)]
mod tests_llm_16_231 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let printer = Printer::new();
        // Verify the printer is properly constructed
        // Add assertions here
        
    }
}#[cfg(test)]
mod tests_llm_16_249 {
    use super::*;

use crate::*;
    use ast::{ClassUnicode, ClassUnicodeKind, ClassUnicodeOpKind, Span, Position};

    #[test]
    fn test_fmt_class_unicode() {
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        let ast = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: false,
            kind: ClassUnicodeKind::OneLetter('c'),
        };
        writer.fmt_class_unicode(&ast).unwrap();
        assert_eq!(writer.wtr, r"\pc");

        writer.wtr.clear();

        let ast = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: true,
            kind: ClassUnicodeKind::OneLetter('c'),
        };
        writer.fmt_class_unicode(&ast).unwrap();
        assert_eq!(writer.wtr, r"\Pc");

        writer.wtr.clear();

        let ast = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: false,
            kind: ClassUnicodeKind::Named(String::from("digit")),
        };
        writer.fmt_class_unicode(&ast).unwrap();
        assert_eq!(writer.wtr, r"\p{digit}");

        writer.wtr.clear();

        let ast = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: false,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::Equal,
                name: String::from("script"),
                value: String::from("Latin"),
            },
        };
        writer.fmt_class_unicode(&ast).unwrap();
        assert_eq!(writer.wtr, r"\p{script=Latin}");

        writer.wtr.clear();

        let ast = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: false,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::Colon,
                name: String::from("script"),
                value: String::from("Latin"),
            },
        };
        writer.fmt_class_unicode(&ast).unwrap();
        assert_eq!(writer.wtr, r"\p{script:Latin}");

        writer.wtr.clear();

        let ast = ClassUnicode {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            negated: false,
            kind: ClassUnicodeKind::NamedValue {
                op: ClassUnicodeOpKind::NotEqual,
                name: String::from("script"),
                value: String::from("Latin"),
            },
        };
        writer.fmt_class_unicode(&ast).unwrap();
        assert_eq!(writer.wtr, r"\p{script!=Latin}");
    }
}#[cfg(test)]
mod tests_llm_16_251 {
    use crate::ast::print::{Writer, Printer};
    use crate::ast::{Flags, FlagsItem, FlagsItemKind, Flag, Span, Position};
    use std::fmt::Write;

    #[test]
    fn test_fmt_flags() {
        // create test ast::Flags
        let ast = Flags {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            items: vec![
                FlagsItem {
                    kind: FlagsItemKind::Negation,
                    span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::CaseInsensitive),
                    span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::MultiLine),
                    span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::DotMatchesNewLine),
                    span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::SwapGreed),
                    span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::Unicode),
                    span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                },
                FlagsItem {
                    kind: FlagsItemKind::Flag(Flag::IgnoreWhitespace),
                    span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                },
            ],
        };

        // create test ast::print::Writer
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: std::string::String::new(),
        };

        // test fmt_flags
        let result = writer.fmt_flags(&ast);
        assert!(result.is_ok());
    }
}#[cfg(test)]
mod tests_llm_16_252 {
    use super::*;

use crate::*;
    use std::fmt::Write;

    #[test]
    fn test_fmt_group_post() {
        let mut wtr = String::new();
        let _ast = ast::Group {
            span: ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0, 0)),
            kind: ast::GroupKind::NonCapturing(ast::Flags {
                span: ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0, 0)),
                items: vec![],
            }),
            ast: Box::new(ast::Ast::Empty(ast::Span::new(ast::Position::new(0, 0, 0), ast::Position::new(0, 0, 0)))),
        };
        let mut writer = ast::print::Writer {
            printer: &mut Printer { _priv: () },
            wtr: &mut wtr,
        };
        writer.fmt_group_post(&_ast).unwrap();
        assert_eq!(wtr, ")")
    }
}#[cfg(test)]
mod tests_llm_16_255 {
    use super::*;

use crate::*;
    use ast::*;
    use ast::print::*;
    use ast::print::Writer;

    #[test]
    fn test_fmt_literal_verbatim() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Verbatim,
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, "a");
    }

    #[test]
    fn test_fmt_literal_punctuation() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Punctuation,
            c: '*',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\*");
    }

    #[test]
    fn test_fmt_literal_octal() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Octal,
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\141");
    }

    #[test]
    fn test_fmt_literal_hex_fixed_x() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexFixed(HexLiteralKind::X),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\x61");
    }

    #[test]
    fn test_fmt_literal_hex_fixed_unicode_short() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexFixed(HexLiteralKind::UnicodeShort),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\u0061");
    }

    #[test]
    fn test_fmt_literal_hex_fixed_unicode_long() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexFixed(HexLiteralKind::UnicodeLong),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\U00000061");
    }

    #[test]
    fn test_fmt_literal_hex_brace_x() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexBrace(HexLiteralKind::X),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\x{61}");
    }

    #[test]
    fn test_fmt_literal_hex_brace_unicode_short() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexBrace(HexLiteralKind::UnicodeShort),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\u{61}");
    }

    #[test]
    fn test_fmt_literal_hex_brace_unicode_long() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::HexBrace(HexLiteralKind::UnicodeLong),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\U{61}");
    }

    #[test]
    fn test_fmt_literal_special_bell() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Special(SpecialLiteralKind::Bell),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\a");
    }

    #[test]
    fn test_fmt_literal_special_form_feed() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Special(SpecialLiteralKind::FormFeed),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\f");
    }

    #[test]
    fn test_fmt_literal_special_tab() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Special(SpecialLiteralKind::Tab),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\t");
    }

    #[test]
    fn test_fmt_literal_special_line_feed() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Special(SpecialLiteralKind::LineFeed),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\n");
    }

    #[test]
    fn test_fmt_literal_special_carriage_return() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Special(SpecialLiteralKind::CarriageReturn),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\r");
    }

    #[test]
    fn test_fmt_literal_special_vertical_tab() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Special(SpecialLiteralKind::VerticalTab),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\v");
    }

    #[test]
    fn test_fmt_literal_special_space() {
        let ast = Literal {
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            kind: LiteralKind::Special(SpecialLiteralKind::Space),
            c: 'a',
        };
        let mut writer = Writer {
            printer: &mut Printer::new(),
            wtr: String::new(),
        };
        writer.fmt_literal(&ast).unwrap();
        assert_eq!(writer.wtr, r"\ ");
    }
}#[cfg(test)]
mod tests_llm_16_257_llm_16_256 {
    use super::*;

use crate::*;
    use crate::ast::{
        Ast, Assertion, AssertionKind, Class, ClassBracketed, ClassPerl, ClassPerlKind, ClassUnicode, ClassUnicodeKind,
        Flags, FlagsItem, FlagsItemKind, Flag, HexLiteralKind, Literal, LiteralKind, Position, Repetition, RepetitionKind,
        RepetitionOp, RepetitionRange, Span,
    };

    struct DummyWriter;

    impl fmt::Write for DummyWriter {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Ok(())
        }

        fn write_char(&mut self, _: char) -> fmt::Result {
            Ok(())
        }
    }

    fn create_dummy_ast() -> Ast {
        Ast::Class(Class::Perl(ClassPerl {
            kind: ClassPerlKind::Digit,
            negated: false,
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        }))
    }

    #[test]
    fn test_fmt_repetition_with_zero_or_one_greedy() {
        let mut writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let ast = Repetition {
            op: RepetitionOp {
                kind: RepetitionKind::ZeroOrOne,
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            },
            greedy: true,
            ast: Box::new(create_dummy_ast()),
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        };
        let result = writer.fmt_repetition(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fmt_repetition_with_zero_or_one_non_greedy() {
        let mut writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let ast = Repetition {
            op: RepetitionOp {
                kind: RepetitionKind::ZeroOrOne,
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            },
            greedy: false,
            ast: Box::new(create_dummy_ast()),
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        };
        let result = writer.fmt_repetition(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fmt_repetition_with_zero_or_more_greedy() {
        let mut writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let ast = Repetition {
            op: RepetitionOp {
                kind: RepetitionKind::ZeroOrMore,
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            },
            greedy: true,
            ast: Box::new(create_dummy_ast()),
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        };
        let result = writer.fmt_repetition(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fmt_repetition_with_zero_or_more_non_greedy() {
        let mut writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let ast = Repetition {
            op: RepetitionOp {
                kind: RepetitionKind::ZeroOrMore,
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            },
            greedy: false,
            ast: Box::new(create_dummy_ast()),
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        };
        let result = writer.fmt_repetition(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fmt_repetition_with_one_or_more_greedy() {
        let mut writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let ast = Repetition {
            op: RepetitionOp {
                kind: RepetitionKind::OneOrMore,
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            },
            greedy: true,
            ast: Box::new(create_dummy_ast()),
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        };
        let result = writer.fmt_repetition(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fmt_repetition_with_one_or_more_non_greedy() {
        let mut writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let ast = Repetition {
            op: RepetitionOp {
                kind: RepetitionKind::OneOrMore,
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            },
            greedy: false,
            ast: Box::new(create_dummy_ast()),
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        };
        let result = writer.fmt_repetition(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fmt_repetition_with_range_greedy() {
        let mut writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let ast = Repetition {
            op: RepetitionOp {
                kind: RepetitionKind::Range(RepetitionRange::Bounded(0, 1)),
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            },
            greedy: true,
            ast: Box::new(create_dummy_ast()),
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        };
        let result = writer.fmt_repetition(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fmt_repetition_with_range_non_greedy() {
        let mut writer = Writer {
            printer: &mut Printer { _priv: () },
            wtr: DummyWriter,
        };
        let ast = Repetition {
            op: RepetitionOp {
                kind: RepetitionKind::Range(RepetitionRange::Bounded(0, 1)),
                span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
            },
            greedy: false,
            ast: Box::new(create_dummy_ast()),
            span: Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0)),
        };
        let result = writer.fmt_repetition(&ast);
        assert!(result.is_ok());
    }
}        
#[cfg(test)]
mod tests_rug_605 {
    use super::*;
    use crate::ast::print::PrinterBuilder;
    use crate::std::default::Default;
    
    #[test]
    fn test_rug() {
        let _printer_builder: PrinterBuilder = <PrinterBuilder as Default>::default();
    }
}
#[cfg(test)]
mod tests_rug_606 {
    use super::*;
    use crate::ast::print::PrinterBuilder;
    
    #[test]
    fn test_rug() {
        let printer_builder: PrinterBuilder = <PrinterBuilder>::new();
        // assertion or further operations
    }
}
#[cfg(test)]
mod tests_rug_607 {
    use super::*;
    use crate::ast::print::PrinterBuilder;

    #[test]
    fn test_rug() {
        let mut p0: PrinterBuilder = PrinterBuilder::new();

        <ast::print::PrinterBuilder>::build(&mut p0);
    }
}     
#[cfg(test)]
mod tests_rug_608 {
    use super::*;
    use crate::ast::{Ast, Concat};
    use crate::ast::print::Indented;
    
    #[test]
    fn test_rug() {
        // Step 1: Fill in the variables
        let mut p0 = ast::print::Printer::new();
        let p1 = Ast::Concat(Concat {
            asts: vec![],
            span: Span::splat(Position::new(0, 0, 0)),
        });
        let mut p2: Indented<'_, T> = Indented::new();
        
        <ast::print::Printer>::print(&mut p0, &p1, &mut p2);
    }
}   
#[cfg(test)]
mod tests_rug_609 {
    use super::*;
    use crate::ast::Visitor;
    use crate::ast::print::Writer;
    
    #[test]
    fn test_rug() {
        let mut p0: Writer<'static, std::io::Sink> = Writer::new();
        <Writer<'static, std::io::Sink> as Visitor>::finish(p0);
    }
}#[cfg(test)]
mod tests_rug_610 {
    use super::*;
    use crate::ast::Visitor;
    use crate::ast::print::Writer;
    use crate::ast::{Ast, Concat, Class};

    #[test]
    fn test_rug() {
        let mut p0: Writer<'static, &'static mut Vec<u8>> = Writer::new();
        
        let p1: Ast = Ast::Concat(Concat {
            asts: vec![],
            span: Span::splat(Position::new(0, 0, 0)),
        });

        <Writer<'static, &'static mut Vec<u8>> as ast::visitor::Visitor>::visit_pre(&mut p0, &p1).unwrap();
    }
}#[cfg(test)]
mod tests_rug_611 {
    use super::*;
    use crate::ast::Visitor;
    use crate::ast::print::Writer;
    use crate::ast::{Ast, Class, ClassPerl, ClassUnicode, Concat, Span};
    use crate::parse::Position;
    use std::fmt;
    
    #[test]
    fn test_visit_post() {
        let mut p0: Writer<'_, fmt::Result> = Writer::new();
        
        let p1 = Ast::Concat(Concat {
            asts: vec![],
            span: Span::splat(Position::new(0, 0, 0))
        });
        
        p0.visit_post(&p1);
    }
}#[cfg(test)]
mod tests_rug_612 {
    use super::*;
    use crate::ast::{print::Writer, Visitor};
    
    #[test]
    fn test_visit_class_set_item_pre() {
        let mut writer: Writer<'static, W> = Writer::new();
        let class_set_item = ClassSetItem::Range(ClassSetRange {
            start: Literal::Unicode('\u{0041}'),
            end: Literal::Unicode('\u{005A}'),
            span: Span::default(),
        });

        writer.visit_class_set_item_pre(&class_set_item).unwrap();
    }
}
#[cfg(test)]
mod tests_rug_613 {
    use super::*;
    use crate::ast::Visitor;
    // use the necessary use statements from the provided samples
    use crate::ast::print::Writer;
    use crate::ast::{ClassSetBinaryOp, ClassSet, ClassSetBinaryOpKind};

    #[test]
    fn test_rug() {
        // construct the variables based on the sample code
        let mut p0: Writer = Writer::new();
        let mut p1 = ClassSetBinaryOp {
            span: Span::default(),
            kind: ClassSetBinaryOpKind::Union,
            lhs: Box::new(ClassSet {}),
            rhs: Box::new(ClassSet {}),
        };

        <ast::print::Writer<'_, _> as ast::visitor::Visitor>::visit_class_set_binary_op_in(&mut p0, &p1);
    }
}

#[cfg(test)]
mod tests_rug_614 {
    use super::*;
    use crate::ast::print::Writer;
    use crate::ast::{Group, GroupKind, Span};

    #[test]
    fn test_rug() {
        let mut p0: Writer = Writer::new();
        let mut p1 = Group {
            span: Span { start: 0, end: 10 },
            kind: GroupKind::NonCapturing(None),
            ast: Box::new(Ast {}),
        };

        <ast::print::Writer<'p, W>>::fmt_group_pre(p0, &p1);
    }
}
#[cfg(test)]
mod tests_rug_615 {
    use super::*;
    use crate::regex_syntax::ast::print::Writer;
    use crate::regex_syntax::ast::RepetitionRange;
    
    #[test]
    fn test_rug() {
        let mut p0: Writer = Writer::new();
        let p1: RepetitionRange = RepetitionRange::Exactly(5);
        
        <ast::print::Writer<'p, W>>::fmt_repetition_range(&mut p0, &p1);

    }
}        
#[cfg(test)]
mod tests_rug_616 {
    use super::*;
    use crate::ast::{Assertion, AssertionKind};
    use crate::ast::print::Writer;
    
    #[test]
    fn test_rug() {
        let mut p0: Writer<'_, W> = Writer::new();
        let mut p1 = Assertion {
            span: Span::default(),
            kind: AssertionKind::StartLine,
        };
        
        <ast::print::Writer<'_, W>>::fmt_assertion(&mut p0, &p1);

        // add assertions here
    }
}#[cfg(test)]
mod tests_rug_617 {
    use crate::ast::print::Writer;
    use crate::ast::{SetFlags, Span, Flags};
    use std::fmt;
    
    #[test]
    fn test_rug() {
        let mut p0: Writer<'static, std::io::Stdout> = Writer::new();
        let p1 = SetFlags {
            span: Span::default(),
            flags: Flags::default(),
        };
        
        let result: fmt::Result = <ast::print::Writer<'_, _>>::fmt_set_flags(&mut p0, &p1);
        
        assert_eq!(result.is_ok(), true);
    }
}                    
#[cfg(test)]
mod tests_rug_618 {

    use super::*;
    use crate::ast::{print::Writer, ClassBracketed};

    #[test]
    fn test_rug() {
        let mut p0: Writer<'static, Vec<u8>> = Writer::new();
        let p1 = ClassBracketed {
            span: Default::default(),
            negated: false,
            kind: ClassSet::None,
        };

        p0.fmt_class_bracketed_pre(&p1).unwrap();
    }
}#[cfg(test)]
mod tests_rug_619 {
    use super::*;
    use crate::ast::print::Writer;
    use crate::ast::{ClassBracketed, Span, ClassSet};

    #[test]
    fn test_rug() {
        let mut v192: Writer = Writer::new();
        let mut v140 = ClassBracketed {
            span: Span::default(),
            negated: false,
            kind: ClassSet::None,
        };

        <ast::print::Writer<'p, W>>::fmt_class_bracketed_post(&mut v192, &v140);
    }
}#[test]
fn test_rug() {
    let mut p0: ast::print::Writer<'static, std::io::Stdout> = ast::print::Writer::new(std::io::stdout());
    let mut p1 = ast::ClassSetBinaryOpKind::Intersection;

    <ast::print::Writer<'static, std::io::Stdout>>::fmt_class_set_binary_op_kind(&mut p0, &p1);
}#[cfg(test)]
mod tests_rug_621 {
    use super::*;
    use crate::ast::{print::Writer, ClassPerl, ClassPerlKind};

    #[test]
    fn test_fmt_class_perl() {
        let mut writer: Writer<'static, std::string::String> = Writer::new();
        let class_perl = ClassPerl {
            span: regex_syntax::ast::Span::default(),
            kind: ClassPerlKind::Digit,
            negated: true,
        };
        
        writer.fmt_class_perl(&class_perl).unwrap();
        
        // Add assertion here if needed
    }
}#[cfg(test)]
mod tests_rug_622 {
    use super::*;
    use crate::ast::print::Writer;
    use crate::ast::{ast, ClassAscii, ClassAsciiKind};
    
    #[test]
    fn test_rug() {
        let mut p0: Writer<'static, std::io::Stdout> = Writer::new();
        let mut p1 = ClassAscii {
            span: ast::Span::default(),
            kind: ClassAsciiKind::new(),
            negated: false,
        };
        
        <Writer<'_, std::io::Stdout>>::fmt_class_ascii(&mut p0, &p1);
    }
}