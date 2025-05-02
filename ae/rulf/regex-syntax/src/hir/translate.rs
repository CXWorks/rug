/*!
Defines a translator that converts an `Ast` to an `Hir`.
*/
use std::cell::{Cell, RefCell};
use std::result;
use ast::{self, Ast, Span, Visitor};
use hir::{self, Error, ErrorKind, Hir};
use unicode::{self, ClassQuery};
type Result<T> = result::Result<T, Error>;
/// A builder for constructing an AST->HIR translator.
#[derive(Clone, Debug)]
pub struct TranslatorBuilder {
    allow_invalid_utf8: bool,
    flags: Flags,
}
impl Default for TranslatorBuilder {
    fn default() -> TranslatorBuilder {
        TranslatorBuilder::new()
    }
}
impl TranslatorBuilder {
    /// Create a new translator builder with a default c onfiguration.
    pub fn new() -> TranslatorBuilder {
        TranslatorBuilder {
            allow_invalid_utf8: false,
            flags: Flags::default(),
        }
    }
    /// Build a translator using the current configuration.
    pub fn build(&self) -> Translator {
        Translator {
            stack: RefCell::new(vec![]),
            flags: Cell::new(self.flags),
            allow_invalid_utf8: self.allow_invalid_utf8,
        }
    }
    /// When enabled, translation will permit the construction of a regular
    /// expression that may match invalid UTF-8.
    ///
    /// When disabled (the default), the translator is guaranteed to produce
    /// an expression that will only ever match valid UTF-8 (otherwise, the
    /// translator will return an error).
    ///
    /// Perhaps surprisingly, when invalid UTF-8 isn't allowed, a negated ASCII
    /// word boundary (uttered as `(?-u:\B)` in the concrete syntax) will cause
    /// the parser to return an error. Namely, a negated ASCII word boundary
    /// can result in matching positions that aren't valid UTF-8 boundaries.
    pub fn allow_invalid_utf8(&mut self, yes: bool) -> &mut TranslatorBuilder {
        self.allow_invalid_utf8 = yes;
        self
    }
    /// Enable or disable the case insensitive flag (`i`) by default.
    pub fn case_insensitive(&mut self, yes: bool) -> &mut TranslatorBuilder {
        self.flags.case_insensitive = if yes { Some(true) } else { None };
        self
    }
    /// Enable or disable the multi-line matching flag (`m`) by default.
    pub fn multi_line(&mut self, yes: bool) -> &mut TranslatorBuilder {
        self.flags.multi_line = if yes { Some(true) } else { None };
        self
    }
    /// Enable or disable the "dot matches any character" flag (`s`) by
    /// default.
    pub fn dot_matches_new_line(&mut self, yes: bool) -> &mut TranslatorBuilder {
        self.flags.dot_matches_new_line = if yes { Some(true) } else { None };
        self
    }
    /// Enable or disable the "swap greed" flag (`U`) by default.
    pub fn swap_greed(&mut self, yes: bool) -> &mut TranslatorBuilder {
        self.flags.swap_greed = if yes { Some(true) } else { None };
        self
    }
    /// Enable or disable the Unicode flag (`u`) by default.
    pub fn unicode(&mut self, yes: bool) -> &mut TranslatorBuilder {
        self.flags.unicode = if yes { None } else { Some(false) };
        self
    }
}
/// A translator maps abstract syntax to a high level intermediate
/// representation.
///
/// A translator may be benefit from reuse. That is, a translator can translate
/// many abstract syntax trees.
///
/// A `Translator` can be configured in more detail via a
/// [`TranslatorBuilder`](struct.TranslatorBuilder.html).
#[derive(Clone, Debug)]
pub struct Translator {
    /// Our call stack, but on the heap.
    stack: RefCell<Vec<HirFrame>>,
    /// The current flag settings.
    flags: Cell<Flags>,
    /// Whether we're allowed to produce HIR that can match arbitrary bytes.
    allow_invalid_utf8: bool,
}
impl Translator {
    /// Create a new translator using the default configuration.
    pub fn new() -> Translator {
        TranslatorBuilder::new().build()
    }
    /// Translate the given abstract syntax tree (AST) into a high level
    /// intermediate representation (HIR).
    ///
    /// If there was a problem doing the translation, then an HIR-specific
    /// error is returned.
    ///
    /// The original pattern string used to produce the `Ast` *must* also be
    /// provided. The translator does not use the pattern string during any
    /// correct translation, but is used for error reporting.
    pub fn translate(&mut self, pattern: &str, ast: &Ast) -> Result<Hir> {
        ast::visit(ast, TranslatorI::new(self, pattern))
    }
}
/// An HirFrame is a single stack frame, represented explicitly, which is
/// created for each item in the Ast that we traverse.
///
/// Note that technically, this type doesn't represent our entire stack
/// frame. In particular, the Ast visitor represents any state associated with
/// traversing the Ast itself.
#[derive(Clone, Debug)]
enum HirFrame {
    /// An arbitrary HIR expression. These get pushed whenever we hit a base
    /// case in the Ast. They get popped after an inductive (i.e., recursive)
    /// step is complete.
    Expr(Hir),
    /// A Unicode character class. This frame is mutated as we descend into
    /// the Ast of a character class (which is itself its own mini recursive
    /// structure).
    ClassUnicode(hir::ClassUnicode),
    /// A byte-oriented character class. This frame is mutated as we descend
    /// into the Ast of a character class (which is itself its own mini
    /// recursive structure).
    ///
    /// Byte character classes are created when Unicode mode (`u`) is disabled.
    /// If `allow_invalid_utf8` is disabled (the default), then a byte
    /// character is only permitted to match ASCII text.
    ClassBytes(hir::ClassBytes),
    /// This is pushed on to the stack upon first seeing any kind of group,
    /// indicated by parentheses (including non-capturing groups). It is popped
    /// upon leaving a group.
    Group {
        /// The old active flags when this group was opened.
        ///
        /// If this group sets flags, then the new active flags are set to the
        /// result of merging the old flags with the flags introduced by this
        /// group. If the group doesn't set any flags, then this is simply
        /// equivalent to whatever flags were set when the group was opened.
        ///
        /// When this group is popped, the active flags should be restored to
        /// the flags set here.
        ///
        /// The "active" flags correspond to whatever flags are set in the
        /// Translator.
        old_flags: Flags,
    },
    /// This is pushed whenever a concatenation is observed. After visiting
    /// every sub-expression in the concatenation, the translator's stack is
    /// popped until it sees a Concat frame.
    Concat,
    /// This is pushed whenever an alternation is observed. After visiting
    /// every sub-expression in the alternation, the translator's stack is
    /// popped until it sees an Alternation frame.
    Alternation,
}
impl HirFrame {
    /// Assert that the current stack frame is an Hir expression and return it.
    fn unwrap_expr(self) -> Hir {
        match self {
            HirFrame::Expr(expr) => expr,
            _ => panic!("tried to unwrap expr from HirFrame, got: {:?}", self),
        }
    }
    /// Assert that the current stack frame is a Unicode class expression and
    /// return it.
    fn unwrap_class_unicode(self) -> hir::ClassUnicode {
        match self {
            HirFrame::ClassUnicode(cls) => cls,
            _ => {
                panic!(
                    "tried to unwrap Unicode class \
                 from HirFrame, got: {:?}",
                    self
                )
            }
        }
    }
    /// Assert that the current stack frame is a byte class expression and
    /// return it.
    fn unwrap_class_bytes(self) -> hir::ClassBytes {
        match self {
            HirFrame::ClassBytes(cls) => cls,
            _ => {
                panic!(
                    "tried to unwrap byte class \
                 from HirFrame, got: {:?}",
                    self
                )
            }
        }
    }
    /// Assert that the current stack frame is a group indicator and return
    /// its corresponding flags (the flags that were active at the time the
    /// group was entered).
    fn unwrap_group(self) -> Flags {
        match self {
            HirFrame::Group { old_flags } => old_flags,
            _ => panic!("tried to unwrap group from HirFrame, got: {:?}", self),
        }
    }
}
impl<'t, 'p> Visitor for TranslatorI<'t, 'p> {
    type Output = Hir;
    type Err = Error;
    fn finish(self) -> Result<Hir> {
        assert_eq!(self.trans().stack.borrow().len(), 1);
        Ok(self.pop().unwrap().unwrap_expr())
    }
    fn visit_pre(&mut self, ast: &Ast) -> Result<()> {
        match *ast {
            Ast::Class(ast::Class::Bracketed(_)) => {
                if self.flags().unicode() {
                    let cls = hir::ClassUnicode::empty();
                    self.push(HirFrame::ClassUnicode(cls));
                } else {
                    let cls = hir::ClassBytes::empty();
                    self.push(HirFrame::ClassBytes(cls));
                }
            }
            Ast::Group(ref x) => {
                let old_flags = x
                    .flags()
                    .map(|ast| self.set_flags(ast))
                    .unwrap_or_else(|| self.flags());
                self.push(HirFrame::Group { old_flags });
            }
            Ast::Concat(ref x) if x.asts.is_empty() => {}
            Ast::Concat(_) => {
                self.push(HirFrame::Concat);
            }
            Ast::Alternation(ref x) if x.asts.is_empty() => {}
            Ast::Alternation(_) => {
                self.push(HirFrame::Alternation);
            }
            _ => {}
        }
        Ok(())
    }
    fn visit_post(&mut self, ast: &Ast) -> Result<()> {
        match *ast {
            Ast::Empty(_) => {
                self.push(HirFrame::Expr(Hir::empty()));
            }
            Ast::Flags(ref x) => {
                self.set_flags(&x.flags);
                self.push(HirFrame::Expr(Hir::empty()));
            }
            Ast::Literal(ref x) => {
                self.push(HirFrame::Expr(self.hir_literal(x)?));
            }
            Ast::Dot(span) => {
                self.push(HirFrame::Expr(self.hir_dot(span)?));
            }
            Ast::Assertion(ref x) => {
                self.push(HirFrame::Expr(self.hir_assertion(x)?));
            }
            Ast::Class(ast::Class::Perl(ref x)) => {
                if self.flags().unicode() {
                    let cls = self.hir_perl_unicode_class(x)?;
                    let hcls = hir::Class::Unicode(cls);
                    self.push(HirFrame::Expr(Hir::class(hcls)));
                } else {
                    let cls = self.hir_perl_byte_class(x);
                    let hcls = hir::Class::Bytes(cls);
                    self.push(HirFrame::Expr(Hir::class(hcls)));
                }
            }
            Ast::Class(ast::Class::Unicode(ref x)) => {
                let cls = hir::Class::Unicode(self.hir_unicode_class(x)?);
                self.push(HirFrame::Expr(Hir::class(cls)));
            }
            Ast::Class(ast::Class::Bracketed(ref ast)) => {
                if self.flags().unicode() {
                    let mut cls = self.pop().unwrap().unwrap_class_unicode();
                    self.unicode_fold_and_negate(&ast.span, ast.negated, &mut cls)?;
                    if cls.ranges().is_empty() {
                        return Err(
                            self.error(ast.span, ErrorKind::EmptyClassNotAllowed),
                        );
                    }
                    let expr = Hir::class(hir::Class::Unicode(cls));
                    self.push(HirFrame::Expr(expr));
                } else {
                    let mut cls = self.pop().unwrap().unwrap_class_bytes();
                    self.bytes_fold_and_negate(&ast.span, ast.negated, &mut cls)?;
                    if cls.ranges().is_empty() {
                        return Err(
                            self.error(ast.span, ErrorKind::EmptyClassNotAllowed),
                        );
                    }
                    let expr = Hir::class(hir::Class::Bytes(cls));
                    self.push(HirFrame::Expr(expr));
                }
            }
            Ast::Repetition(ref x) => {
                let expr = self.pop().unwrap().unwrap_expr();
                self.push(HirFrame::Expr(self.hir_repetition(x, expr)));
            }
            Ast::Group(ref x) => {
                let expr = self.pop().unwrap().unwrap_expr();
                let old_flags = self.pop().unwrap().unwrap_group();
                self.trans().flags.set(old_flags);
                self.push(HirFrame::Expr(self.hir_group(x, expr)));
            }
            Ast::Concat(_) => {
                let mut exprs = vec![];
                while let Some(HirFrame::Expr(expr)) = self.pop() {
                    if !expr.kind().is_empty() {
                        exprs.push(expr);
                    }
                }
                exprs.reverse();
                self.push(HirFrame::Expr(Hir::concat(exprs)));
            }
            Ast::Alternation(_) => {
                let mut exprs = vec![];
                while let Some(HirFrame::Expr(expr)) = self.pop() {
                    exprs.push(expr);
                }
                exprs.reverse();
                self.push(HirFrame::Expr(Hir::alternation(exprs)));
            }
        }
        Ok(())
    }
    fn visit_class_set_item_pre(&mut self, ast: &ast::ClassSetItem) -> Result<()> {
        match *ast {
            ast::ClassSetItem::Bracketed(_) => {
                if self.flags().unicode() {
                    let cls = hir::ClassUnicode::empty();
                    self.push(HirFrame::ClassUnicode(cls));
                } else {
                    let cls = hir::ClassBytes::empty();
                    self.push(HirFrame::ClassBytes(cls));
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn visit_class_set_item_post(&mut self, ast: &ast::ClassSetItem) -> Result<()> {
        match *ast {
            ast::ClassSetItem::Empty(_) => {}
            ast::ClassSetItem::Literal(ref x) => {
                if self.flags().unicode() {
                    let mut cls = self.pop().unwrap().unwrap_class_unicode();
                    cls.push(hir::ClassUnicodeRange::new(x.c, x.c));
                    self.push(HirFrame::ClassUnicode(cls));
                } else {
                    let mut cls = self.pop().unwrap().unwrap_class_bytes();
                    let byte = self.class_literal_byte(x)?;
                    cls.push(hir::ClassBytesRange::new(byte, byte));
                    self.push(HirFrame::ClassBytes(cls));
                }
            }
            ast::ClassSetItem::Range(ref x) => {
                if self.flags().unicode() {
                    let mut cls = self.pop().unwrap().unwrap_class_unicode();
                    cls.push(hir::ClassUnicodeRange::new(x.start.c, x.end.c));
                    self.push(HirFrame::ClassUnicode(cls));
                } else {
                    let mut cls = self.pop().unwrap().unwrap_class_bytes();
                    let start = self.class_literal_byte(&x.start)?;
                    let end = self.class_literal_byte(&x.end)?;
                    cls.push(hir::ClassBytesRange::new(start, end));
                    self.push(HirFrame::ClassBytes(cls));
                }
            }
            ast::ClassSetItem::Ascii(ref x) => {
                if self.flags().unicode() {
                    let mut cls = self.pop().unwrap().unwrap_class_unicode();
                    for &(s, e) in ascii_class(&x.kind) {
                        cls.push(hir::ClassUnicodeRange::new(s, e));
                    }
                    self.unicode_fold_and_negate(&x.span, x.negated, &mut cls)?;
                    self.push(HirFrame::ClassUnicode(cls));
                } else {
                    let mut cls = self.pop().unwrap().unwrap_class_bytes();
                    for &(s, e) in ascii_class(&x.kind) {
                        cls.push(hir::ClassBytesRange::new(s as u8, e as u8));
                    }
                    self.bytes_fold_and_negate(&x.span, x.negated, &mut cls)?;
                    self.push(HirFrame::ClassBytes(cls));
                }
            }
            ast::ClassSetItem::Unicode(ref x) => {
                let xcls = self.hir_unicode_class(x)?;
                let mut cls = self.pop().unwrap().unwrap_class_unicode();
                cls.union(&xcls);
                self.push(HirFrame::ClassUnicode(cls));
            }
            ast::ClassSetItem::Perl(ref x) => {
                if self.flags().unicode() {
                    let xcls = self.hir_perl_unicode_class(x)?;
                    let mut cls = self.pop().unwrap().unwrap_class_unicode();
                    cls.union(&xcls);
                    self.push(HirFrame::ClassUnicode(cls));
                } else {
                    let xcls = self.hir_perl_byte_class(x);
                    let mut cls = self.pop().unwrap().unwrap_class_bytes();
                    cls.union(&xcls);
                    self.push(HirFrame::ClassBytes(cls));
                }
            }
            ast::ClassSetItem::Bracketed(ref ast) => {
                if self.flags().unicode() {
                    let mut cls1 = self.pop().unwrap().unwrap_class_unicode();
                    self.unicode_fold_and_negate(&ast.span, ast.negated, &mut cls1)?;
                    let mut cls2 = self.pop().unwrap().unwrap_class_unicode();
                    cls2.union(&cls1);
                    self.push(HirFrame::ClassUnicode(cls2));
                } else {
                    let mut cls1 = self.pop().unwrap().unwrap_class_bytes();
                    self.bytes_fold_and_negate(&ast.span, ast.negated, &mut cls1)?;
                    let mut cls2 = self.pop().unwrap().unwrap_class_bytes();
                    cls2.union(&cls1);
                    self.push(HirFrame::ClassBytes(cls2));
                }
            }
            ast::ClassSetItem::Union(_) => {}
        }
        Ok(())
    }
    fn visit_class_set_binary_op_pre(
        &mut self,
        _op: &ast::ClassSetBinaryOp,
    ) -> Result<()> {
        if self.flags().unicode() {
            let cls = hir::ClassUnicode::empty();
            self.push(HirFrame::ClassUnicode(cls));
        } else {
            let cls = hir::ClassBytes::empty();
            self.push(HirFrame::ClassBytes(cls));
        }
        Ok(())
    }
    fn visit_class_set_binary_op_in(
        &mut self,
        _op: &ast::ClassSetBinaryOp,
    ) -> Result<()> {
        if self.flags().unicode() {
            let cls = hir::ClassUnicode::empty();
            self.push(HirFrame::ClassUnicode(cls));
        } else {
            let cls = hir::ClassBytes::empty();
            self.push(HirFrame::ClassBytes(cls));
        }
        Ok(())
    }
    fn visit_class_set_binary_op_post(
        &mut self,
        op: &ast::ClassSetBinaryOp,
    ) -> Result<()> {
        use ast::ClassSetBinaryOpKind::*;
        if self.flags().unicode() {
            let mut rhs = self.pop().unwrap().unwrap_class_unicode();
            let mut lhs = self.pop().unwrap().unwrap_class_unicode();
            let mut cls = self.pop().unwrap().unwrap_class_unicode();
            if self.flags().case_insensitive() {
                rhs.try_case_fold_simple()
                    .map_err(|_| {
                        self.error(
                            op.rhs.span().clone(),
                            ErrorKind::UnicodeCaseUnavailable,
                        )
                    })?;
                lhs.try_case_fold_simple()
                    .map_err(|_| {
                        self.error(
                            op.lhs.span().clone(),
                            ErrorKind::UnicodeCaseUnavailable,
                        )
                    })?;
            }
            match op.kind {
                Intersection => lhs.intersect(&rhs),
                Difference => lhs.difference(&rhs),
                SymmetricDifference => lhs.symmetric_difference(&rhs),
            }
            cls.union(&lhs);
            self.push(HirFrame::ClassUnicode(cls));
        } else {
            let mut rhs = self.pop().unwrap().unwrap_class_bytes();
            let mut lhs = self.pop().unwrap().unwrap_class_bytes();
            let mut cls = self.pop().unwrap().unwrap_class_bytes();
            if self.flags().case_insensitive() {
                rhs.case_fold_simple();
                lhs.case_fold_simple();
            }
            match op.kind {
                Intersection => lhs.intersect(&rhs),
                Difference => lhs.difference(&rhs),
                SymmetricDifference => lhs.symmetric_difference(&rhs),
            }
            cls.union(&lhs);
            self.push(HirFrame::ClassBytes(cls));
        }
        Ok(())
    }
}
/// The internal implementation of a translator.
///
/// This type is responsible for carrying around the original pattern string,
/// which is not tied to the internal state of a translator.
///
/// A TranslatorI exists for the time it takes to translate a single Ast.
#[derive(Clone, Debug)]
struct TranslatorI<'t, 'p> {
    trans: &'t Translator,
    pattern: &'p str,
}
impl<'t, 'p> TranslatorI<'t, 'p> {
    /// Build a new internal translator.
    fn new(trans: &'t Translator, pattern: &'p str) -> TranslatorI<'t, 'p> {
        TranslatorI {
            trans: trans,
            pattern: pattern,
        }
    }
    /// Return a reference to the underlying translator.
    fn trans(&self) -> &Translator {
        &self.trans
    }
    /// Push the given frame on to the call stack.
    fn push(&self, frame: HirFrame) {
        self.trans().stack.borrow_mut().push(frame);
    }
    /// Pop the top of the call stack. If the call stack is empty, return None.
    fn pop(&self) -> Option<HirFrame> {
        self.trans().stack.borrow_mut().pop()
    }
    /// Create a new error with the given span and error type.
    fn error(&self, span: Span, kind: ErrorKind) -> Error {
        Error {
            kind: kind,
            pattern: self.pattern.to_string(),
            span: span,
        }
    }
    /// Return a copy of the active flags.
    fn flags(&self) -> Flags {
        self.trans().flags.get()
    }
    /// Set the flags of this translator from the flags set in the given AST.
    /// Then, return the old flags.
    fn set_flags(&self, ast_flags: &ast::Flags) -> Flags {
        let old_flags = self.flags();
        let mut new_flags = Flags::from_ast(ast_flags);
        new_flags.merge(&old_flags);
        self.trans().flags.set(new_flags);
        old_flags
    }
    fn hir_literal(&self, lit: &ast::Literal) -> Result<Hir> {
        let ch = match self.literal_to_char(lit)? {
            byte @ hir::Literal::Byte(_) => return Ok(Hir::literal(byte)),
            hir::Literal::Unicode(ch) => ch,
        };
        if self.flags().case_insensitive() {
            self.hir_from_char_case_insensitive(lit.span, ch)
        } else {
            self.hir_from_char(lit.span, ch)
        }
    }
    /// Convert an Ast literal to its scalar representation.
    ///
    /// When Unicode mode is enabled, then this always succeeds and returns a
    /// `char` (Unicode scalar value).
    ///
    /// When Unicode mode is disabled, then a raw byte is returned. If that
    /// byte is not ASCII and invalid UTF-8 is not allowed, then this returns
    /// an error.
    fn literal_to_char(&self, lit: &ast::Literal) -> Result<hir::Literal> {
        if self.flags().unicode() {
            return Ok(hir::Literal::Unicode(lit.c));
        }
        let byte = match lit.byte() {
            None => return Ok(hir::Literal::Unicode(lit.c)),
            Some(byte) => byte,
        };
        if byte <= 0x7F {
            return Ok(hir::Literal::Unicode(byte as char));
        }
        if !self.trans().allow_invalid_utf8 {
            return Err(self.error(lit.span, ErrorKind::InvalidUtf8));
        }
        Ok(hir::Literal::Byte(byte))
    }
    fn hir_from_char(&self, span: Span, c: char) -> Result<Hir> {
        if !self.flags().unicode() && c.len_utf8() > 1 {
            return Err(self.error(span, ErrorKind::UnicodeNotAllowed));
        }
        Ok(Hir::literal(hir::Literal::Unicode(c)))
    }
    fn hir_from_char_case_insensitive(&self, span: Span, c: char) -> Result<Hir> {
        if self.flags().unicode() {
            let map = unicode::contains_simple_case_mapping(c, c)
                .map_err(|_| { self.error(span, ErrorKind::UnicodeCaseUnavailable) })?;
            if !map {
                return self.hir_from_char(span, c);
            }
            let mut cls = hir::ClassUnicode::new(
                vec![hir::ClassUnicodeRange::new(c, c,)],
            );
            cls.try_case_fold_simple()
                .map_err(|_| { self.error(span, ErrorKind::UnicodeCaseUnavailable) })?;
            Ok(Hir::class(hir::Class::Unicode(cls)))
        } else {
            if c.len_utf8() > 1 {
                return Err(self.error(span, ErrorKind::UnicodeNotAllowed));
            }
            match c {
                'A'..='Z' | 'a'..='z' => {}
                _ => return self.hir_from_char(span, c),
            }
            let mut cls = hir::ClassBytes::new(
                vec![hir::ClassBytesRange::new(c as u8, c as u8,)],
            );
            cls.case_fold_simple();
            Ok(Hir::class(hir::Class::Bytes(cls)))
        }
    }
    fn hir_dot(&self, span: Span) -> Result<Hir> {
        let unicode = self.flags().unicode();
        if !unicode && !self.trans().allow_invalid_utf8 {
            return Err(self.error(span, ErrorKind::InvalidUtf8));
        }
        Ok(
            if self.flags().dot_matches_new_line() {
                Hir::any(!unicode)
            } else {
                Hir::dot(!unicode)
            },
        )
    }
    fn hir_assertion(&self, asst: &ast::Assertion) -> Result<Hir> {
        let unicode = self.flags().unicode();
        let multi_line = self.flags().multi_line();
        Ok(
            match asst.kind {
                ast::AssertionKind::StartLine => {
                    Hir::anchor(
                        if multi_line {
                            hir::Anchor::StartLine
                        } else {
                            hir::Anchor::StartText
                        },
                    )
                }
                ast::AssertionKind::EndLine => {
                    Hir::anchor(
                        if multi_line {
                            hir::Anchor::EndLine
                        } else {
                            hir::Anchor::EndText
                        },
                    )
                }
                ast::AssertionKind::StartText => Hir::anchor(hir::Anchor::StartText),
                ast::AssertionKind::EndText => Hir::anchor(hir::Anchor::EndText),
                ast::AssertionKind::WordBoundary => {
                    Hir::word_boundary(
                        if unicode {
                            hir::WordBoundary::Unicode
                        } else {
                            hir::WordBoundary::Ascii
                        },
                    )
                }
                ast::AssertionKind::NotWordBoundary => {
                    Hir::word_boundary(
                        if unicode {
                            hir::WordBoundary::UnicodeNegate
                        } else {
                            if !self.trans().allow_invalid_utf8 {
                                return Err(self.error(asst.span, ErrorKind::InvalidUtf8));
                            }
                            hir::WordBoundary::AsciiNegate
                        },
                    )
                }
            },
        )
    }
    fn hir_group(&self, group: &ast::Group, expr: Hir) -> Hir {
        let kind = match group.kind {
            ast::GroupKind::CaptureIndex(idx) => hir::GroupKind::CaptureIndex(idx),
            ast::GroupKind::CaptureName(ref capname) => {
                hir::GroupKind::CaptureName {
                    name: capname.name.clone(),
                    index: capname.index,
                }
            }
            ast::GroupKind::NonCapturing(_) => hir::GroupKind::NonCapturing,
        };
        Hir::group(hir::Group {
            kind: kind,
            hir: Box::new(expr),
        })
    }
    fn hir_repetition(&self, rep: &ast::Repetition, expr: Hir) -> Hir {
        let kind = match rep.op.kind {
            ast::RepetitionKind::ZeroOrOne => hir::RepetitionKind::ZeroOrOne,
            ast::RepetitionKind::ZeroOrMore => hir::RepetitionKind::ZeroOrMore,
            ast::RepetitionKind::OneOrMore => hir::RepetitionKind::OneOrMore,
            ast::RepetitionKind::Range(ast::RepetitionRange::Exactly(m)) => {
                hir::RepetitionKind::Range(hir::RepetitionRange::Exactly(m))
            }
            ast::RepetitionKind::Range(ast::RepetitionRange::AtLeast(m)) => {
                hir::RepetitionKind::Range(hir::RepetitionRange::AtLeast(m))
            }
            ast::RepetitionKind::Range(ast::RepetitionRange::Bounded(m, n)) => {
                hir::RepetitionKind::Range(hir::RepetitionRange::Bounded(m, n))
            }
        };
        let greedy = if self.flags().swap_greed() { !rep.greedy } else { rep.greedy };
        Hir::repetition(hir::Repetition {
            kind: kind,
            greedy: greedy,
            hir: Box::new(expr),
        })
    }
    fn hir_unicode_class(
        &self,
        ast_class: &ast::ClassUnicode,
    ) -> Result<hir::ClassUnicode> {
        use ast::ClassUnicodeKind::*;
        if !self.flags().unicode() {
            return Err(self.error(ast_class.span, ErrorKind::UnicodeNotAllowed));
        }
        let query = match ast_class.kind {
            OneLetter(name) => ClassQuery::OneLetter(name),
            Named(ref name) => ClassQuery::Binary(name),
            NamedValue { ref name, ref value, .. } => {
                ClassQuery::ByValue {
                    property_name: name,
                    property_value: value,
                }
            }
        };
        let mut result = self
            .convert_unicode_class_error(&ast_class.span, unicode::class(query));
        if let Ok(ref mut class) = result {
            self.unicode_fold_and_negate(&ast_class.span, ast_class.negated, class)?;
            if class.ranges().is_empty() {
                let err = self.error(ast_class.span, ErrorKind::EmptyClassNotAllowed);
                return Err(err);
            }
        }
        result
    }
    fn hir_perl_unicode_class(
        &self,
        ast_class: &ast::ClassPerl,
    ) -> Result<hir::ClassUnicode> {
        use ast::ClassPerlKind::*;
        assert!(self.flags().unicode());
        let result = match ast_class.kind {
            Digit => unicode::perl_digit(),
            Space => unicode::perl_space(),
            Word => unicode::perl_word(),
        };
        let mut class = self.convert_unicode_class_error(&ast_class.span, result)?;
        if ast_class.negated {
            class.negate();
        }
        Ok(class)
    }
    fn hir_perl_byte_class(&self, ast_class: &ast::ClassPerl) -> hir::ClassBytes {
        use ast::ClassPerlKind::*;
        assert!(! self.flags().unicode());
        let mut class = match ast_class.kind {
            Digit => hir_ascii_class_bytes(&ast::ClassAsciiKind::Digit),
            Space => hir_ascii_class_bytes(&ast::ClassAsciiKind::Space),
            Word => hir_ascii_class_bytes(&ast::ClassAsciiKind::Word),
        };
        if ast_class.negated {
            class.negate();
        }
        class
    }
    /// Converts the given Unicode specific error to an HIR translation error.
    ///
    /// The span given should approximate the position at which an error would
    /// occur.
    fn convert_unicode_class_error(
        &self,
        span: &Span,
        result: unicode::Result<hir::ClassUnicode>,
    ) -> Result<hir::ClassUnicode> {
        result
            .map_err(|err| {
                let sp = span.clone();
                match err {
                    unicode::Error::PropertyNotFound => {
                        self.error(sp, ErrorKind::UnicodePropertyNotFound)
                    }
                    unicode::Error::PropertyValueNotFound => {
                        self.error(sp, ErrorKind::UnicodePropertyValueNotFound)
                    }
                    unicode::Error::PerlClassNotFound => {
                        self.error(sp, ErrorKind::UnicodePerlClassNotFound)
                    }
                }
            })
    }
    fn unicode_fold_and_negate(
        &self,
        span: &Span,
        negated: bool,
        class: &mut hir::ClassUnicode,
    ) -> Result<()> {
        if self.flags().case_insensitive() {
            class
                .try_case_fold_simple()
                .map_err(|_| {
                    self.error(span.clone(), ErrorKind::UnicodeCaseUnavailable)
                })?;
        }
        if negated {
            class.negate();
        }
        Ok(())
    }
    fn bytes_fold_and_negate(
        &self,
        span: &Span,
        negated: bool,
        class: &mut hir::ClassBytes,
    ) -> Result<()> {
        if self.flags().case_insensitive() {
            class.case_fold_simple();
        }
        if negated {
            class.negate();
        }
        if !self.trans().allow_invalid_utf8 && !class.is_all_ascii() {
            return Err(self.error(span.clone(), ErrorKind::InvalidUtf8));
        }
        Ok(())
    }
    /// Return a scalar byte value suitable for use as a literal in a byte
    /// character class.
    fn class_literal_byte(&self, ast: &ast::Literal) -> Result<u8> {
        match self.literal_to_char(ast)? {
            hir::Literal::Byte(byte) => Ok(byte),
            hir::Literal::Unicode(ch) => {
                if ch <= 0x7F as char {
                    Ok(ch as u8)
                } else {
                    Err(self.error(ast.span, ErrorKind::UnicodeNotAllowed))
                }
            }
        }
    }
}
/// A translator's representation of a regular expression's flags at any given
/// moment in time.
///
/// Each flag can be in one of three states: absent, present but disabled or
/// present but enabled.
#[derive(Clone, Copy, Debug, Default)]
struct Flags {
    case_insensitive: Option<bool>,
    multi_line: Option<bool>,
    dot_matches_new_line: Option<bool>,
    swap_greed: Option<bool>,
    unicode: Option<bool>,
}
impl Flags {
    fn from_ast(ast: &ast::Flags) -> Flags {
        let mut flags = Flags::default();
        let mut enable = true;
        for item in &ast.items {
            match item.kind {
                ast::FlagsItemKind::Negation => {
                    enable = false;
                }
                ast::FlagsItemKind::Flag(ast::Flag::CaseInsensitive) => {
                    flags.case_insensitive = Some(enable);
                }
                ast::FlagsItemKind::Flag(ast::Flag::MultiLine) => {
                    flags.multi_line = Some(enable);
                }
                ast::FlagsItemKind::Flag(ast::Flag::DotMatchesNewLine) => {
                    flags.dot_matches_new_line = Some(enable);
                }
                ast::FlagsItemKind::Flag(ast::Flag::SwapGreed) => {
                    flags.swap_greed = Some(enable);
                }
                ast::FlagsItemKind::Flag(ast::Flag::Unicode) => {
                    flags.unicode = Some(enable);
                }
                ast::FlagsItemKind::Flag(ast::Flag::IgnoreWhitespace) => {}
            }
        }
        flags
    }
    fn merge(&mut self, previous: &Flags) {
        if self.case_insensitive.is_none() {
            self.case_insensitive = previous.case_insensitive;
        }
        if self.multi_line.is_none() {
            self.multi_line = previous.multi_line;
        }
        if self.dot_matches_new_line.is_none() {
            self.dot_matches_new_line = previous.dot_matches_new_line;
        }
        if self.swap_greed.is_none() {
            self.swap_greed = previous.swap_greed;
        }
        if self.unicode.is_none() {
            self.unicode = previous.unicode;
        }
    }
    fn case_insensitive(&self) -> bool {
        self.case_insensitive.unwrap_or(false)
    }
    fn multi_line(&self) -> bool {
        self.multi_line.unwrap_or(false)
    }
    fn dot_matches_new_line(&self) -> bool {
        self.dot_matches_new_line.unwrap_or(false)
    }
    fn swap_greed(&self) -> bool {
        self.swap_greed.unwrap_or(false)
    }
    fn unicode(&self) -> bool {
        self.unicode.unwrap_or(true)
    }
}
fn hir_ascii_class_bytes(kind: &ast::ClassAsciiKind) -> hir::ClassBytes {
    let ranges: Vec<_> = ascii_class(kind)
        .iter()
        .cloned()
        .map(|(s, e)| hir::ClassBytesRange::new(s as u8, e as u8))
        .collect();
    hir::ClassBytes::new(ranges)
}
fn ascii_class(kind: &ast::ClassAsciiKind) -> &'static [(char, char)] {
    use ast::ClassAsciiKind::*;
    match *kind {
        Alnum => &[('0', '9'), ('A', 'Z'), ('a', 'z')],
        Alpha => &[('A', 'Z'), ('a', 'z')],
        Ascii => &[('\x00', '\x7F')],
        Blank => &[('\t', '\t'), (' ', ' ')],
        Cntrl => &[('\x00', '\x1F'), ('\x7F', '\x7F')],
        Digit => &[('0', '9')],
        Graph => &[('!', '~')],
        Lower => &[('a', 'z')],
        Print => &[(' ', '~')],
        Punct => &[('!', '/'), (':', '@'), ('[', '`'), ('{', '~')],
        Space => {
            &[
                ('\t', '\t'),
                ('\n', '\n'),
                ('\x0B', '\x0B'),
                ('\x0C', '\x0C'),
                ('\r', '\r'),
                (' ', ' '),
            ]
        }
        Upper => &[('A', 'Z')],
        Word => &[('0', '9'), ('A', 'Z'), ('_', '_'), ('a', 'z')],
        Xdigit => &[('0', '9'), ('A', 'F'), ('a', 'f')],
    }
}
#[cfg(test)]
mod tests {
    use ast::parse::ParserBuilder;
    use ast::{self, Ast, Position, Span};
    use hir::{self, Hir, HirKind};
    use unicode::{self, ClassQuery};
    use super::{ascii_class, TranslatorBuilder};
    #[derive(Clone, Debug)]
    struct TestError {
        span: Span,
        kind: hir::ErrorKind,
    }
    impl PartialEq<hir::Error> for TestError {
        fn eq(&self, other: &hir::Error) -> bool {
            self.span == other.span && self.kind == other.kind
        }
    }
    impl PartialEq<TestError> for hir::Error {
        fn eq(&self, other: &TestError) -> bool {
            self.span == other.span && self.kind == other.kind
        }
    }
    fn parse(pattern: &str) -> Ast {
        ParserBuilder::new().octal(true).build().parse(pattern).unwrap()
    }
    fn t(pattern: &str) -> Hir {
        TranslatorBuilder::new()
            .allow_invalid_utf8(false)
            .build()
            .translate(pattern, &parse(pattern))
            .unwrap()
    }
    fn t_err(pattern: &str) -> hir::Error {
        TranslatorBuilder::new()
            .allow_invalid_utf8(false)
            .build()
            .translate(pattern, &parse(pattern))
            .unwrap_err()
    }
    fn t_bytes(pattern: &str) -> Hir {
        TranslatorBuilder::new()
            .allow_invalid_utf8(true)
            .build()
            .translate(pattern, &parse(pattern))
            .unwrap()
    }
    fn hir_lit(s: &str) -> Hir {
        match s.len() {
            0 => Hir::empty(),
            _ => {
                let lits = s
                    .chars()
                    .map(hir::Literal::Unicode)
                    .map(Hir::literal)
                    .collect();
                Hir::concat(lits)
            }
        }
    }
    fn hir_blit(s: &[u8]) -> Hir {
        match s.len() {
            0 => Hir::empty(),
            1 => Hir::literal(hir::Literal::Byte(s[0])),
            _ => {
                let lits = s
                    .iter()
                    .cloned()
                    .map(hir::Literal::Byte)
                    .map(Hir::literal)
                    .collect();
                Hir::concat(lits)
            }
        }
    }
    fn hir_group(i: u32, expr: Hir) -> Hir {
        Hir::group(hir::Group {
            kind: hir::GroupKind::CaptureIndex(i),
            hir: Box::new(expr),
        })
    }
    fn hir_group_name(i: u32, name: &str, expr: Hir) -> Hir {
        Hir::group(hir::Group {
            kind: hir::GroupKind::CaptureName {
                name: name.to_string(),
                index: i,
            },
            hir: Box::new(expr),
        })
    }
    fn hir_group_nocap(expr: Hir) -> Hir {
        Hir::group(hir::Group {
            kind: hir::GroupKind::NonCapturing,
            hir: Box::new(expr),
        })
    }
    fn hir_quest(greedy: bool, expr: Hir) -> Hir {
        Hir::repetition(hir::Repetition {
            kind: hir::RepetitionKind::ZeroOrOne,
            greedy: greedy,
            hir: Box::new(expr),
        })
    }
    fn hir_star(greedy: bool, expr: Hir) -> Hir {
        Hir::repetition(hir::Repetition {
            kind: hir::RepetitionKind::ZeroOrMore,
            greedy: greedy,
            hir: Box::new(expr),
        })
    }
    fn hir_plus(greedy: bool, expr: Hir) -> Hir {
        Hir::repetition(hir::Repetition {
            kind: hir::RepetitionKind::OneOrMore,
            greedy: greedy,
            hir: Box::new(expr),
        })
    }
    fn hir_range(greedy: bool, range: hir::RepetitionRange, expr: Hir) -> Hir {
        Hir::repetition(hir::Repetition {
            kind: hir::RepetitionKind::Range(range),
            greedy: greedy,
            hir: Box::new(expr),
        })
    }
    fn hir_alt(alts: Vec<Hir>) -> Hir {
        Hir::alternation(alts)
    }
    fn hir_cat(exprs: Vec<Hir>) -> Hir {
        Hir::concat(exprs)
    }
    #[allow(dead_code)]
    fn hir_uclass_query(query: ClassQuery) -> Hir {
        Hir::class(hir::Class::Unicode(unicode::class(query).unwrap()))
    }
    #[allow(dead_code)]
    fn hir_uclass_perl_word() -> Hir {
        Hir::class(hir::Class::Unicode(unicode::perl_word().unwrap()))
    }
    fn hir_uclass(ranges: &[(char, char)]) -> Hir {
        let ranges: Vec<hir::ClassUnicodeRange> = ranges
            .iter()
            .map(|&(s, e)| hir::ClassUnicodeRange::new(s, e))
            .collect();
        Hir::class(hir::Class::Unicode(hir::ClassUnicode::new(ranges)))
    }
    fn hir_bclass(ranges: &[(u8, u8)]) -> Hir {
        let ranges: Vec<hir::ClassBytesRange> = ranges
            .iter()
            .map(|&(s, e)| hir::ClassBytesRange::new(s, e))
            .collect();
        Hir::class(hir::Class::Bytes(hir::ClassBytes::new(ranges)))
    }
    fn hir_bclass_from_char(ranges: &[(char, char)]) -> Hir {
        let ranges: Vec<hir::ClassBytesRange> = ranges
            .iter()
            .map(|&(s, e)| {
                assert!(s as u32 <= 0x7F);
                assert!(e as u32 <= 0x7F);
                hir::ClassBytesRange::new(s as u8, e as u8)
            })
            .collect();
        Hir::class(hir::Class::Bytes(hir::ClassBytes::new(ranges)))
    }
    fn hir_case_fold(expr: Hir) -> Hir {
        match expr.into_kind() {
            HirKind::Class(mut cls) => {
                cls.case_fold_simple();
                Hir::class(cls)
            }
            _ => panic!("cannot case fold non-class Hir expr"),
        }
    }
    fn hir_negate(expr: Hir) -> Hir {
        match expr.into_kind() {
            HirKind::Class(mut cls) => {
                cls.negate();
                Hir::class(cls)
            }
            _ => panic!("cannot negate non-class Hir expr"),
        }
    }
    #[allow(dead_code)]
    fn hir_union(expr1: Hir, expr2: Hir) -> Hir {
        use hir::Class::{Bytes, Unicode};
        match (expr1.into_kind(), expr2.into_kind()) {
            (HirKind::Class(Unicode(mut c1)), HirKind::Class(Unicode(c2))) => {
                c1.union(&c2);
                Hir::class(hir::Class::Unicode(c1))
            }
            (HirKind::Class(Bytes(mut c1)), HirKind::Class(Bytes(c2))) => {
                c1.union(&c2);
                Hir::class(hir::Class::Bytes(c1))
            }
            _ => panic!("cannot union non-class Hir exprs"),
        }
    }
    #[allow(dead_code)]
    fn hir_difference(expr1: Hir, expr2: Hir) -> Hir {
        use hir::Class::{Bytes, Unicode};
        match (expr1.into_kind(), expr2.into_kind()) {
            (HirKind::Class(Unicode(mut c1)), HirKind::Class(Unicode(c2))) => {
                c1.difference(&c2);
                Hir::class(hir::Class::Unicode(c1))
            }
            (HirKind::Class(Bytes(mut c1)), HirKind::Class(Bytes(c2))) => {
                c1.difference(&c2);
                Hir::class(hir::Class::Bytes(c1))
            }
            _ => panic!("cannot difference non-class Hir exprs"),
        }
    }
    fn hir_anchor(anchor: hir::Anchor) -> Hir {
        Hir::anchor(anchor)
    }
    fn hir_word(wb: hir::WordBoundary) -> Hir {
        Hir::word_boundary(wb)
    }
    #[test]
    fn empty() {
        assert_eq!(t(""), Hir::empty());
        assert_eq!(t("(?i)"), Hir::empty());
        assert_eq!(t("()"), hir_group(1, Hir::empty()));
        assert_eq!(t("(?:)"), hir_group_nocap(Hir::empty()));
        assert_eq!(t("(?P<wat>)"), hir_group_name(1, "wat", Hir::empty()));
        assert_eq!(t("|"), hir_alt(vec![Hir::empty(), Hir::empty()]));
        assert_eq!(
            t("()|()"), hir_alt(vec![hir_group(1, Hir::empty()), hir_group(2,
            Hir::empty()),])
        );
        assert_eq!(t("(|b)"), hir_group(1, hir_alt(vec![Hir::empty(), hir_lit("b"),])));
        assert_eq!(t("(a|)"), hir_group(1, hir_alt(vec![hir_lit("a"), Hir::empty(),])));
        assert_eq!(
            t("(a||c)"), hir_group(1, hir_alt(vec![hir_lit("a"), Hir::empty(),
            hir_lit("c"),]))
        );
        assert_eq!(
            t("(||)"), hir_group(1, hir_alt(vec![Hir::empty(), Hir::empty(),
            Hir::empty(),]))
        );
    }
    #[test]
    fn literal() {
        assert_eq!(t("a"), hir_lit("a"));
        assert_eq!(t("(?-u)a"), hir_lit("a"));
        assert_eq!(t("☃"), hir_lit("☃"));
        assert_eq!(t("abcd"), hir_lit("abcd"));
        assert_eq!(t_bytes("(?-u)a"), hir_lit("a"));
        assert_eq!(t_bytes("(?-u)\x61"), hir_lit("a"));
        assert_eq!(t_bytes(r"(?-u)\x61"), hir_lit("a"));
        assert_eq!(t_bytes(r"(?-u)\xFF"), hir_blit(b"\xFF"));
        assert_eq!(
            t_err("(?-u)☃"), TestError { kind : hir::ErrorKind::UnicodeNotAllowed, span
            : Span::new(Position::new(5, 1, 6), Position::new(8, 1, 7)), }
        );
        assert_eq!(
            t_err(r"(?-u)\xFF"), TestError { kind : hir::ErrorKind::InvalidUtf8, span :
            Span::new(Position::new(5, 1, 6), Position::new(9, 1, 10)), }
        );
    }
    #[test]
    fn literal_case_insensitive() {
        #[cfg(feature = "unicode-case")]
        assert_eq!(t("(?i)a"), hir_uclass(& [('A', 'A'), ('a', 'a'),]));
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i:a)"), hir_group_nocap(hir_uclass(& [('A', 'A'), ('a', 'a')],))
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("a(?i)a(?-i)a"), hir_cat(vec![hir_lit("a"), hir_uclass(& [('A', 'A'), ('a',
            'a')]), hir_lit("a"),])
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i)ab@c"), hir_cat(vec![hir_uclass(& [('A', 'A'), ('a', 'a')]),
            hir_uclass(& [('B', 'B'), ('b', 'b')]), hir_lit("@"), hir_uclass(& [('C',
            'C'), ('c', 'c')]),])
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i)β"), hir_uclass(& [('Β', 'Β'), ('β', 'β'), ('ϐ', 'ϐ'),])
        );
        assert_eq!(t("(?i-u)a"), hir_bclass(& [(b'A', b'A'), (b'a', b'a'),]));
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?-u)a(?i)a(?-i)a"), hir_cat(vec![hir_lit("a"), hir_bclass(& [(b'A',
            b'A'), (b'a', b'a')]), hir_lit("a"),])
        );
        assert_eq!(
            t("(?i-u)ab@c"), hir_cat(vec![hir_bclass(& [(b'A', b'A'), (b'a', b'a')]),
            hir_bclass(& [(b'B', b'B'), (b'b', b'b')]), hir_lit("@"), hir_bclass(&
            [(b'C', b'C'), (b'c', b'c')]),])
        );
        assert_eq!(t_bytes("(?i-u)a"), hir_bclass(& [(b'A', b'A'), (b'a', b'a'),]));
        assert_eq!(t_bytes("(?i-u)\x61"), hir_bclass(& [(b'A', b'A'), (b'a', b'a'),]));
        assert_eq!(t_bytes(r"(?i-u)\x61"), hir_bclass(& [(b'A', b'A'), (b'a', b'a'),]));
        assert_eq!(t_bytes(r"(?i-u)\xFF"), hir_blit(b"\xFF"));
        assert_eq!(
            t_err("(?i-u)β"), TestError { kind : hir::ErrorKind::UnicodeNotAllowed, span
            : Span::new(Position::new(6, 1, 7), Position::new(8, 1, 8),), }
        );
    }
    #[test]
    fn dot() {
        assert_eq!(t("."), hir_uclass(& [('\0', '\t'), ('\x0B', '\u{10FFFF}'),]));
        assert_eq!(t("(?s)."), hir_uclass(& [('\0', '\u{10FFFF}'),]));
        assert_eq!(
            t_bytes("(?-u)."), hir_bclass(& [(b'\0', b'\t'), (b'\x0B', b'\xFF'),])
        );
        assert_eq!(t_bytes("(?s-u)."), hir_bclass(& [(b'\0', b'\xFF'),]));
        assert_eq!(
            t_err("(?-u)."), TestError { kind : hir::ErrorKind::InvalidUtf8, span :
            Span::new(Position::new(5, 1, 6), Position::new(6, 1, 7)), }
        );
        assert_eq!(
            t_err("(?s-u)."), TestError { kind : hir::ErrorKind::InvalidUtf8, span :
            Span::new(Position::new(6, 1, 7), Position::new(7, 1, 8)), }
        );
    }
    #[test]
    fn assertions() {
        assert_eq!(t("^"), hir_anchor(hir::Anchor::StartText));
        assert_eq!(t("$"), hir_anchor(hir::Anchor::EndText));
        assert_eq!(t(r"\A"), hir_anchor(hir::Anchor::StartText));
        assert_eq!(t(r"\z"), hir_anchor(hir::Anchor::EndText));
        assert_eq!(t("(?m)^"), hir_anchor(hir::Anchor::StartLine));
        assert_eq!(t("(?m)$"), hir_anchor(hir::Anchor::EndLine));
        assert_eq!(t(r"(?m)\A"), hir_anchor(hir::Anchor::StartText));
        assert_eq!(t(r"(?m)\z"), hir_anchor(hir::Anchor::EndText));
        assert_eq!(t(r"\b"), hir_word(hir::WordBoundary::Unicode));
        assert_eq!(t(r"\B"), hir_word(hir::WordBoundary::UnicodeNegate));
        assert_eq!(t(r"(?-u)\b"), hir_word(hir::WordBoundary::Ascii));
        assert_eq!(t_bytes(r"(?-u)\B"), hir_word(hir::WordBoundary::AsciiNegate));
        assert_eq!(
            t_err(r"(?-u)\B"), TestError { kind : hir::ErrorKind::InvalidUtf8, span :
            Span::new(Position::new(5, 1, 6), Position::new(7, 1, 8)), }
        );
    }
    #[test]
    fn group() {
        assert_eq!(t("(a)"), hir_group(1, hir_lit("a")));
        assert_eq!(
            t("(a)(b)"), hir_cat(vec![hir_group(1, hir_lit("a")), hir_group(2,
            hir_lit("b")),])
        );
        assert_eq!(
            t("(a)|(b)"), hir_alt(vec![hir_group(1, hir_lit("a")), hir_group(2,
            hir_lit("b")),])
        );
        assert_eq!(t("(?P<foo>)"), hir_group_name(1, "foo", Hir::empty()));
        assert_eq!(t("(?P<foo>a)"), hir_group_name(1, "foo", hir_lit("a")));
        assert_eq!(
            t("(?P<foo>a)(?P<bar>b)"), hir_cat(vec![hir_group_name(1, "foo",
            hir_lit("a")), hir_group_name(2, "bar", hir_lit("b")),])
        );
        assert_eq!(t("(?:)"), hir_group_nocap(Hir::empty()));
        assert_eq!(t("(?:a)"), hir_group_nocap(hir_lit("a")));
        assert_eq!(
            t("(?:a)(b)"), hir_cat(vec![hir_group_nocap(hir_lit("a")), hir_group(1,
            hir_lit("b")),])
        );
        assert_eq!(
            t("(a)(?:b)(c)"), hir_cat(vec![hir_group(1, hir_lit("a")),
            hir_group_nocap(hir_lit("b")), hir_group(2, hir_lit("c")),])
        );
        assert_eq!(
            t("(a)(?P<foo>b)(c)"), hir_cat(vec![hir_group(1, hir_lit("a")),
            hir_group_name(2, "foo", hir_lit("b")), hir_group(3, hir_lit("c")),])
        );
        assert_eq!(t("()"), hir_group(1, Hir::empty()));
        assert_eq!(t("((?i))"), hir_group(1, Hir::empty()));
        assert_eq!(t("((?x))"), hir_group(1, Hir::empty()));
        assert_eq!(t("(((?x)))"), hir_group(1, hir_group(2, Hir::empty())));
    }
    #[test]
    fn flags() {
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i:a)a"), hir_cat(vec![hir_group_nocap(hir_uclass(& [('A', 'A'), ('a',
            'a')])), hir_lit("a"),])
        );
        assert_eq!(
            t("(?i-u:a)β"), hir_cat(vec![hir_group_nocap(hir_bclass(& [(b'A', b'A'),
            (b'a', b'a')])), hir_lit("β"),])
        );
        assert_eq!(
            t("(?:(?i-u)a)b"), hir_cat(vec![hir_group_nocap(hir_bclass(& [(b'A', b'A'),
            (b'a', b'a')])), hir_lit("b"),])
        );
        assert_eq!(
            t("((?i-u)a)b"), hir_cat(vec![hir_group(1, hir_bclass(& [(b'A', b'A'), (b'a',
            b'a')])), hir_lit("b"),])
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i)(?-i:a)a"), hir_cat(vec![hir_group_nocap(hir_lit("a")), hir_uclass(&
            [('A', 'A'), ('a', 'a')]),])
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?im)a^"), hir_cat(vec![hir_uclass(& [('A', 'A'), ('a', 'a')]),
            hir_anchor(hir::Anchor::StartLine),])
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?im)a^(?i-m)a^"), hir_cat(vec![hir_uclass(& [('A', 'A'), ('a', 'a')]),
            hir_anchor(hir::Anchor::StartLine), hir_uclass(& [('A', 'A'), ('a', 'a')]),
            hir_anchor(hir::Anchor::StartText),])
        );
        assert_eq!(
            t("(?U)a*a*?(?-U)a*a*?"), hir_cat(vec![hir_star(false, hir_lit("a")),
            hir_star(true, hir_lit("a")), hir_star(true, hir_lit("a")), hir_star(false,
            hir_lit("a")),])
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?:a(?i)a)a"), hir_cat(vec![hir_group_nocap(hir_cat(vec![hir_lit("a"),
            hir_uclass(& [('A', 'A'), ('a', 'a')]),])), hir_lit("a"),])
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i)(?:a(?-i)a)a"), hir_cat(vec![hir_group_nocap(hir_cat(vec![hir_uclass(&
            [('A', 'A'), ('a', 'a')]), hir_lit("a"),])), hir_uclass(& [('A', 'A'), ('a',
            'a')]),])
        );
    }
    #[test]
    fn escape() {
        assert_eq!(t(r"\\\.\+\*\?\(\)\|\[\]\{\}\^\$\#"), hir_lit(r"\.+*?()|[]{}^$#"));
    }
    #[test]
    fn repetition() {
        assert_eq!(t("a?"), hir_quest(true, hir_lit("a")));
        assert_eq!(t("a*"), hir_star(true, hir_lit("a")));
        assert_eq!(t("a+"), hir_plus(true, hir_lit("a")));
        assert_eq!(t("a??"), hir_quest(false, hir_lit("a")));
        assert_eq!(t("a*?"), hir_star(false, hir_lit("a")));
        assert_eq!(t("a+?"), hir_plus(false, hir_lit("a")));
        assert_eq!(
            t("a{1}"), hir_range(true, hir::RepetitionRange::Exactly(1), hir_lit("a"),)
        );
        assert_eq!(
            t("a{1,}"), hir_range(true, hir::RepetitionRange::AtLeast(1), hir_lit("a"),)
        );
        assert_eq!(
            t("a{1,2}"), hir_range(true, hir::RepetitionRange::Bounded(1, 2),
            hir_lit("a"),)
        );
        assert_eq!(
            t("a{1}?"), hir_range(false, hir::RepetitionRange::Exactly(1), hir_lit("a"),)
        );
        assert_eq!(
            t("a{1,}?"), hir_range(false, hir::RepetitionRange::AtLeast(1),
            hir_lit("a"),)
        );
        assert_eq!(
            t("a{1,2}?"), hir_range(false, hir::RepetitionRange::Bounded(1, 2),
            hir_lit("a"),)
        );
        assert_eq!(
            t("ab?"), hir_cat(vec![hir_lit("a"), hir_quest(true, hir_lit("b")),])
        );
        assert_eq!(
            t("(ab)?"), hir_quest(true, hir_group(1, hir_cat(vec![hir_lit("a"),
            hir_lit("b"),])))
        );
        assert_eq!(
            t("a|b?"), hir_alt(vec![hir_lit("a"), hir_quest(true, hir_lit("b")),])
        );
    }
    #[test]
    fn cat_alt() {
        assert_eq!(t("(ab)"), hir_group(1, hir_cat(vec![hir_lit("a"), hir_lit("b"),])));
        assert_eq!(t("a|b"), hir_alt(vec![hir_lit("a"), hir_lit("b"),]));
        assert_eq!(t("a|b|c"), hir_alt(vec![hir_lit("a"), hir_lit("b"), hir_lit("c"),]));
        assert_eq!(
            t("ab|bc|cd"), hir_alt(vec![hir_lit("ab"), hir_lit("bc"), hir_lit("cd"),])
        );
        assert_eq!(t("(a|b)"), hir_group(1, hir_alt(vec![hir_lit("a"), hir_lit("b"),])));
        assert_eq!(
            t("(a|b|c)"), hir_group(1, hir_alt(vec![hir_lit("a"), hir_lit("b"),
            hir_lit("c"),]))
        );
        assert_eq!(
            t("(ab|bc|cd)"), hir_group(1, hir_alt(vec![hir_lit("ab"), hir_lit("bc"),
            hir_lit("cd"),]))
        );
        assert_eq!(
            t("(ab|(bc|(cd)))"), hir_group(1, hir_alt(vec![hir_lit("ab"), hir_group(2,
            hir_alt(vec![hir_lit("bc"), hir_group(3, hir_lit("cd")),])),]))
        );
    }
    #[test]
    fn class_ascii() {
        assert_eq!(
            t("[[:alnum:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Alnum))
        );
        assert_eq!(
            t("[[:alpha:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Alpha))
        );
        assert_eq!(
            t("[[:ascii:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Ascii))
        );
        assert_eq!(
            t("[[:blank:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Blank))
        );
        assert_eq!(
            t("[[:cntrl:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Cntrl))
        );
        assert_eq!(
            t("[[:digit:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Digit))
        );
        assert_eq!(
            t("[[:graph:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Graph))
        );
        assert_eq!(
            t("[[:lower:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Lower))
        );
        assert_eq!(
            t("[[:print:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Print))
        );
        assert_eq!(
            t("[[:punct:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Punct))
        );
        assert_eq!(
            t("[[:space:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Space))
        );
        assert_eq!(
            t("[[:upper:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Upper))
        );
        assert_eq!(
            t("[[:word:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Word))
        );
        assert_eq!(
            t("[[:xdigit:]]"), hir_uclass(ascii_class(& ast::ClassAsciiKind::Xdigit))
        );
        assert_eq!(
            t("[[:^lower:]]"), hir_negate(hir_uclass(ascii_class(&
            ast::ClassAsciiKind::Lower)))
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i)[[:lower:]]"), hir_uclass(& [('A', 'Z'), ('a', 'z'), ('\u{17F}',
            '\u{17F}'), ('\u{212A}', '\u{212A}'),])
        );
        assert_eq!(
            t("(?-u)[[:lower:]]"), hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Lower))
        );
        assert_eq!(
            t("(?i-u)[[:lower:]]"), hir_case_fold(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Lower)))
        );
        assert_eq!(
            t_err("(?-u)[[:^lower:]]"), TestError { kind : hir::ErrorKind::InvalidUtf8,
            span : Span::new(Position::new(6, 1, 7), Position::new(16, 1, 17)), }
        );
        assert_eq!(
            t_err("(?i-u)[[:^lower:]]"), TestError { kind : hir::ErrorKind::InvalidUtf8,
            span : Span::new(Position::new(7, 1, 8), Position::new(17, 1, 18)), }
        );
    }
    #[test]
    #[cfg(feature = "unicode-perl")]
    fn class_perl() {
        assert_eq!(t(r"\d"), hir_uclass_query(ClassQuery::Binary("digit")));
        assert_eq!(t(r"\s"), hir_uclass_query(ClassQuery::Binary("space")));
        assert_eq!(t(r"\w"), hir_uclass_perl_word());
        #[cfg(feature = "unicode-case")]
        assert_eq!(t(r"(?i)\d"), hir_uclass_query(ClassQuery::Binary("digit")));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t(r"(?i)\s"), hir_uclass_query(ClassQuery::Binary("space")));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t(r"(?i)\w"), hir_uclass_perl_word());
        assert_eq!(t(r"\D"), hir_negate(hir_uclass_query(ClassQuery::Binary("digit"))));
        assert_eq!(t(r"\S"), hir_negate(hir_uclass_query(ClassQuery::Binary("space"))));
        assert_eq!(t(r"\W"), hir_negate(hir_uclass_perl_word()));
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t(r"(?i)\D"), hir_negate(hir_uclass_query(ClassQuery::Binary("digit")))
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t(r"(?i)\S"), hir_negate(hir_uclass_query(ClassQuery::Binary("space")))
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(t(r"(?i)\W"), hir_negate(hir_uclass_perl_word()));
        assert_eq!(
            t(r"(?-u)\d"), hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Digit))
        );
        assert_eq!(
            t(r"(?-u)\s"), hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Space))
        );
        assert_eq!(
            t(r"(?-u)\w"), hir_bclass_from_char(ascii_class(& ast::ClassAsciiKind::Word))
        );
        assert_eq!(
            t(r"(?i-u)\d"), hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Digit))
        );
        assert_eq!(
            t(r"(?i-u)\s"), hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Space))
        );
        assert_eq!(
            t(r"(?i-u)\w"), hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Word))
        );
        assert_eq!(
            t(r"(?-u)\D"), hir_negate(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Digit)))
        );
        assert_eq!(
            t(r"(?-u)\S"), hir_negate(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Space)))
        );
        assert_eq!(
            t(r"(?-u)\W"), hir_negate(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Word)))
        );
        assert_eq!(
            t(r"(?i-u)\D"), hir_negate(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Digit)))
        );
        assert_eq!(
            t(r"(?i-u)\S"), hir_negate(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Space)))
        );
        assert_eq!(
            t(r"(?i-u)\W"), hir_negate(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Word)))
        );
    }
    #[test]
    #[cfg(not(feature = "unicode-perl"))]
    fn class_perl_word_disabled() {
        assert_eq!(
            t_err(r"\w"), TestError { kind : hir::ErrorKind::UnicodePerlClassNotFound,
            span : Span::new(Position::new(0, 1, 1), Position::new(2, 1, 3)), }
        );
    }
    #[test]
    #[cfg(all(not(feature = "unicode-perl"), not(feature = "unicode-bool")))]
    fn class_perl_space_disabled() {
        assert_eq!(
            t_err(r"\s"), TestError { kind : hir::ErrorKind::UnicodePerlClassNotFound,
            span : Span::new(Position::new(0, 1, 1), Position::new(2, 1, 3)), }
        );
    }
    #[test]
    #[cfg(all(not(feature = "unicode-perl"), not(feature = "unicode-gencat")))]
    fn class_perl_digit_disabled() {
        assert_eq!(
            t_err(r"\d"), TestError { kind : hir::ErrorKind::UnicodePerlClassNotFound,
            span : Span::new(Position::new(0, 1, 1), Position::new(2, 1, 3)), }
        );
    }
    #[test]
    #[cfg(feature = "unicode-gencat")]
    fn class_unicode_gencat() {
        assert_eq!(t(r"\pZ"), hir_uclass_query(ClassQuery::Binary("Z")));
        assert_eq!(t(r"\pz"), hir_uclass_query(ClassQuery::Binary("Z")));
        assert_eq!(t(r"\p{Separator}"), hir_uclass_query(ClassQuery::Binary("Z")));
        assert_eq!(
            t(r"\p{se      PaRa ToR}"), hir_uclass_query(ClassQuery::Binary("Z"))
        );
        assert_eq!(t(r"\p{gc:Separator}"), hir_uclass_query(ClassQuery::Binary("Z")));
        assert_eq!(t(r"\p{gc=Separator}"), hir_uclass_query(ClassQuery::Binary("Z")));
        assert_eq!(t(r"\p{Other}"), hir_uclass_query(ClassQuery::Binary("Other")));
        assert_eq!(t(r"\pC"), hir_uclass_query(ClassQuery::Binary("Other")));
        assert_eq!(t(r"\PZ"), hir_negate(hir_uclass_query(ClassQuery::Binary("Z"))));
        assert_eq!(
            t(r"\P{separator}"), hir_negate(hir_uclass_query(ClassQuery::Binary("Z")))
        );
        assert_eq!(
            t(r"\P{gc!=separator}"),
            hir_negate(hir_uclass_query(ClassQuery::Binary("Z")))
        );
        assert_eq!(t(r"\p{any}"), hir_uclass_query(ClassQuery::Binary("Any")));
        assert_eq!(t(r"\p{assigned}"), hir_uclass_query(ClassQuery::Binary("Assigned")));
        assert_eq!(t(r"\p{ascii}"), hir_uclass_query(ClassQuery::Binary("ASCII")));
        assert_eq!(t(r"\p{gc:any}"), hir_uclass_query(ClassQuery::Binary("Any")));
        assert_eq!(
            t(r"\p{gc:assigned}"), hir_uclass_query(ClassQuery::Binary("Assigned"))
        );
        assert_eq!(t(r"\p{gc:ascii}"), hir_uclass_query(ClassQuery::Binary("ASCII")));
        assert_eq!(
            t_err(r"(?-u)\pZ"), TestError { kind : hir::ErrorKind::UnicodeNotAllowed,
            span : Span::new(Position::new(5, 1, 6), Position::new(8, 1, 9)), }
        );
        assert_eq!(
            t_err(r"(?-u)\p{Separator}"), TestError { kind :
            hir::ErrorKind::UnicodeNotAllowed, span : Span::new(Position::new(5, 1, 6),
            Position::new(18, 1, 19)), }
        );
        assert_eq!(
            t_err(r"\pE"), TestError { kind : hir::ErrorKind::UnicodePropertyNotFound,
            span : Span::new(Position::new(0, 1, 1), Position::new(3, 1, 4)), }
        );
        assert_eq!(
            t_err(r"\p{Foo}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyNotFound, span : Span::new(Position::new(0, 1,
            1), Position::new(7, 1, 8)), }
        );
        assert_eq!(
            t_err(r"\p{gc:Foo}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyValueNotFound, span :
            Span::new(Position::new(0, 1, 1), Position::new(10, 1, 11)), }
        );
    }
    #[test]
    #[cfg(not(feature = "unicode-gencat"))]
    fn class_unicode_gencat_disabled() {
        assert_eq!(
            t_err(r"\p{Separator}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyNotFound, span : Span::new(Position::new(0, 1,
            1), Position::new(13, 1, 14)), }
        );
        assert_eq!(
            t_err(r"\p{Any}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyNotFound, span : Span::new(Position::new(0, 1,
            1), Position::new(7, 1, 8)), }
        );
    }
    #[test]
    #[cfg(feature = "unicode-script")]
    fn class_unicode_script() {
        assert_eq!(t(r"\p{Greek}"), hir_uclass_query(ClassQuery::Binary("Greek")));
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t(r"(?i)\p{Greek}"),
            hir_case_fold(hir_uclass_query(ClassQuery::Binary("Greek")))
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t(r"(?i)\P{Greek}"),
            hir_negate(hir_case_fold(hir_uclass_query(ClassQuery::Binary("Greek"))))
        );
        assert_eq!(
            t_err(r"\p{sc:Foo}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyValueNotFound, span :
            Span::new(Position::new(0, 1, 1), Position::new(10, 1, 11)), }
        );
        assert_eq!(
            t_err(r"\p{scx:Foo}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyValueNotFound, span :
            Span::new(Position::new(0, 1, 1), Position::new(11, 1, 12)), }
        );
    }
    #[test]
    #[cfg(not(feature = "unicode-script"))]
    fn class_unicode_script_disabled() {
        assert_eq!(
            t_err(r"\p{Greek}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyNotFound, span : Span::new(Position::new(0, 1,
            1), Position::new(9, 1, 10)), }
        );
        assert_eq!(
            t_err(r"\p{scx:Greek}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyNotFound, span : Span::new(Position::new(0, 1,
            1), Position::new(13, 1, 14)), }
        );
    }
    #[test]
    #[cfg(feature = "unicode-age")]
    fn class_unicode_age() {
        assert_eq!(
            t_err(r"\p{age:Foo}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyValueNotFound, span :
            Span::new(Position::new(0, 1, 1), Position::new(11, 1, 12)), }
        );
    }
    #[test]
    #[cfg(feature = "unicode-gencat")]
    fn class_unicode_any_empty() {
        assert_eq!(
            t_err(r"\P{any}"), TestError { kind : hir::ErrorKind::EmptyClassNotAllowed,
            span : Span::new(Position::new(0, 1, 1), Position::new(7, 1, 8)), }
        );
    }
    #[test]
    #[cfg(not(feature = "unicode-age"))]
    fn class_unicode_age_disabled() {
        assert_eq!(
            t_err(r"\p{age:3.0}"), TestError { kind :
            hir::ErrorKind::UnicodePropertyNotFound, span : Span::new(Position::new(0, 1,
            1), Position::new(11, 1, 12)), }
        );
    }
    #[test]
    fn class_bracketed() {
        assert_eq!(t("[a]"), hir_uclass(& [('a', 'a')]));
        assert_eq!(t("[^[a]]"), hir_negate(hir_uclass(& [('a', 'a')])));
        assert_eq!(t("[a-z]"), hir_uclass(& [('a', 'z')]));
        assert_eq!(t("[a-fd-h]"), hir_uclass(& [('a', 'h')]));
        assert_eq!(t("[a-fg-m]"), hir_uclass(& [('a', 'm')]));
        assert_eq!(t(r"[\x00]"), hir_uclass(& [('\0', '\0')]));
        assert_eq!(t(r"[\n]"), hir_uclass(& [('\n', '\n')]));
        assert_eq!(t("[\n]"), hir_uclass(& [('\n', '\n')]));
        #[cfg(any(feature = "unicode-perl", feature = "unicode-gencat"))]
        assert_eq!(t(r"[\d]"), hir_uclass_query(ClassQuery::Binary("digit")));
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(t(r"[\pZ]"), hir_uclass_query(ClassQuery::Binary("separator")));
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(
            t(r"[\p{separator}]"), hir_uclass_query(ClassQuery::Binary("separator"))
        );
        #[cfg(any(feature = "unicode-perl", feature = "unicode-gencat"))]
        assert_eq!(t(r"[^\D]"), hir_uclass_query(ClassQuery::Binary("digit")));
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(t(r"[^\PZ]"), hir_uclass_query(ClassQuery::Binary("separator")));
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(
            t(r"[^\P{separator}]"), hir_uclass_query(ClassQuery::Binary("separator"))
        );
        #[cfg(
            all(
                feature = "unicode-case",
                any(feature = "unicode-perl", feature = "unicode-gencat")
            )
        )] assert_eq!(t(r"(?i)[^\D]"), hir_uclass_query(ClassQuery::Binary("digit")));
        #[cfg(all(feature = "unicode-case", feature = "unicode-script"))]
        assert_eq!(
            t(r"(?i)[^\P{greek}]"),
            hir_case_fold(hir_uclass_query(ClassQuery::Binary("greek")))
        );
        assert_eq!(t("(?-u)[a]"), hir_bclass(& [(b'a', b'a')]));
        assert_eq!(t(r"(?-u)[\x00]"), hir_bclass(& [(b'\0', b'\0')]));
        assert_eq!(t_bytes(r"(?-u)[\xFF]"), hir_bclass(& [(b'\xFF', b'\xFF')]));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t("(?i)[a]"), hir_uclass(& [('A', 'A'), ('a', 'a')]));
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i)[k]"), hir_uclass(& [('K', 'K'), ('k', 'k'), ('\u{212A}',
            '\u{212A}'),])
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t("(?i)[β]"), hir_uclass(& [('Β', 'Β'), ('β', 'β'), ('ϐ', 'ϐ'),])
        );
        assert_eq!(t("(?i-u)[k]"), hir_bclass(& [(b'K', b'K'), (b'k', b'k'),]));
        assert_eq!(t("[^a]"), hir_negate(hir_uclass(& [('a', 'a')])));
        assert_eq!(t(r"[^\x00]"), hir_negate(hir_uclass(& [('\0', '\0')])));
        assert_eq!(t_bytes("(?-u)[^a]"), hir_negate(hir_bclass(& [(b'a', b'a')])));
        #[cfg(any(feature = "unicode-perl", feature = "unicode-gencat"))]
        assert_eq!(
            t(r"[^\d]"), hir_negate(hir_uclass_query(ClassQuery::Binary("digit")))
        );
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(
            t(r"[^\pZ]"), hir_negate(hir_uclass_query(ClassQuery::Binary("separator")))
        );
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(
            t(r"[^\p{separator}]"),
            hir_negate(hir_uclass_query(ClassQuery::Binary("separator")))
        );
        #[cfg(all(feature = "unicode-case", feature = "unicode-script"))]
        assert_eq!(
            t(r"(?i)[^\p{greek}]"),
            hir_negate(hir_case_fold(hir_uclass_query(ClassQuery::Binary("greek"))))
        );
        #[cfg(all(feature = "unicode-case", feature = "unicode-script"))]
        assert_eq!(
            t(r"(?i)[\P{greek}]"),
            hir_negate(hir_case_fold(hir_uclass_query(ClassQuery::Binary("greek"))))
        );
        assert_eq!(t(r"[\[]"), hir_uclass(& [('[', '[')]));
        assert_eq!(t(r"[&]"), hir_uclass(& [('&', '&')]));
        assert_eq!(t(r"[\&]"), hir_uclass(& [('&', '&')]));
        assert_eq!(t(r"[\&\&]"), hir_uclass(& [('&', '&')]));
        assert_eq!(t(r"[\x00-&]"), hir_uclass(& [('\0', '&')]));
        assert_eq!(t(r"[&-\xFF]"), hir_uclass(& [('&', '\u{FF}')]));
        assert_eq!(t(r"[~]"), hir_uclass(& [('~', '~')]));
        assert_eq!(t(r"[\~]"), hir_uclass(& [('~', '~')]));
        assert_eq!(t(r"[\~\~]"), hir_uclass(& [('~', '~')]));
        assert_eq!(t(r"[\x00-~]"), hir_uclass(& [('\0', '~')]));
        assert_eq!(t(r"[~-\xFF]"), hir_uclass(& [('~', '\u{FF}')]));
        assert_eq!(t(r"[-]"), hir_uclass(& [('-', '-')]));
        assert_eq!(t(r"[\-]"), hir_uclass(& [('-', '-')]));
        assert_eq!(t(r"[\-\-]"), hir_uclass(& [('-', '-')]));
        assert_eq!(t(r"[\x00-\-]"), hir_uclass(& [('\0', '-')]));
        assert_eq!(t(r"[\--\xFF]"), hir_uclass(& [('-', '\u{FF}')]));
        assert_eq!(
            t_err("(?-u)[^a]"), TestError { kind : hir::ErrorKind::InvalidUtf8, span :
            Span::new(Position::new(5, 1, 6), Position::new(9, 1, 10)), }
        );
        #[cfg(any(feature = "unicode-perl", feature = "unicode-bool"))]
        assert_eq!(
            t_err(r"[^\s\S]"), TestError { kind : hir::ErrorKind::EmptyClassNotAllowed,
            span : Span::new(Position::new(0, 1, 1), Position::new(7, 1, 8)), }
        );
        #[cfg(any(feature = "unicode-perl", feature = "unicode-bool"))]
        assert_eq!(
            t_err(r"(?-u)[^\s\S]"), TestError { kind :
            hir::ErrorKind::EmptyClassNotAllowed, span : Span::new(Position::new(5, 1,
            6), Position::new(12, 1, 13)), }
        );
    }
    #[test]
    fn class_bracketed_union() {
        assert_eq!(t("[a-zA-Z]"), hir_uclass(& [('A', 'Z'), ('a', 'z')]));
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(
            t(r"[a\pZb]"), hir_union(hir_uclass(& [('a', 'b')]),
            hir_uclass_query(ClassQuery::Binary("separator")))
        );
        #[cfg(all(feature = "unicode-gencat", feature = "unicode-script"))]
        assert_eq!(
            t(r"[\pZ\p{Greek}]"),
            hir_union(hir_uclass_query(ClassQuery::Binary("greek")),
            hir_uclass_query(ClassQuery::Binary("separator")))
        );
        #[cfg(
            all(
                feature = "unicode-age",
                feature = "unicode-gencat",
                feature = "unicode-script"
            )
        )]
        assert_eq!(
            t(r"[\p{age:3.0}\pZ\p{Greek}]"),
            hir_union(hir_uclass_query(ClassQuery::ByValue { property_name : "age",
            property_value : "3.0", }),
            hir_union(hir_uclass_query(ClassQuery::Binary("greek")),
            hir_uclass_query(ClassQuery::Binary("separator"))))
        );
        #[cfg(
            all(
                feature = "unicode-age",
                feature = "unicode-gencat",
                feature = "unicode-script"
            )
        )]
        assert_eq!(
            t(r"[[[\p{age:3.0}\pZ]\p{Greek}][\p{Cyrillic}]]"),
            hir_union(hir_uclass_query(ClassQuery::ByValue { property_name : "age",
            property_value : "3.0", }),
            hir_union(hir_uclass_query(ClassQuery::Binary("cyrillic")),
            hir_union(hir_uclass_query(ClassQuery::Binary("greek")),
            hir_uclass_query(ClassQuery::Binary("separator")))))
        );
        #[cfg(
            all(
                feature = "unicode-age",
                feature = "unicode-case",
                feature = "unicode-gencat",
                feature = "unicode-script"
            )
        )]
        assert_eq!(
            t(r"(?i)[\p{age:3.0}\pZ\p{Greek}]"),
            hir_case_fold(hir_union(hir_uclass_query(ClassQuery::ByValue { property_name
            : "age", property_value : "3.0", }),
            hir_union(hir_uclass_query(ClassQuery::Binary("greek")),
            hir_uclass_query(ClassQuery::Binary("separator")))))
        );
        #[cfg(
            all(
                feature = "unicode-age",
                feature = "unicode-gencat",
                feature = "unicode-script"
            )
        )]
        assert_eq!(
            t(r"[^\p{age:3.0}\pZ\p{Greek}]"),
            hir_negate(hir_union(hir_uclass_query(ClassQuery::ByValue { property_name :
            "age", property_value : "3.0", }),
            hir_union(hir_uclass_query(ClassQuery::Binary("greek")),
            hir_uclass_query(ClassQuery::Binary("separator")))))
        );
        #[cfg(
            all(
                feature = "unicode-age",
                feature = "unicode-case",
                feature = "unicode-gencat",
                feature = "unicode-script"
            )
        )]
        assert_eq!(
            t(r"(?i)[^\p{age:3.0}\pZ\p{Greek}]"),
            hir_negate(hir_case_fold(hir_union(hir_uclass_query(ClassQuery::ByValue {
            property_name : "age", property_value : "3.0", }),
            hir_union(hir_uclass_query(ClassQuery::Binary("greek")),
            hir_uclass_query(ClassQuery::Binary("separator"))))))
        );
    }
    #[test]
    fn class_bracketed_nested() {
        assert_eq!(t(r"[a[^c]]"), hir_negate(hir_uclass(& [('c', 'c')])));
        assert_eq!(t(r"[a-b[^c]]"), hir_negate(hir_uclass(& [('c', 'c')])));
        assert_eq!(t(r"[a-c[^c]]"), hir_negate(hir_uclass(& [])));
        assert_eq!(t(r"[^a[^c]]"), hir_uclass(& [('c', 'c')]));
        assert_eq!(t(r"[^a-b[^c]]"), hir_uclass(& [('c', 'c')]));
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t(r"(?i)[a[^c]]"), hir_negate(hir_case_fold(hir_uclass(& [('c', 'c')])))
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t(r"(?i)[a-b[^c]]"), hir_negate(hir_case_fold(hir_uclass(& [('c', 'c')])))
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(t(r"(?i)[^a[^c]]"), hir_uclass(& [('C', 'C'), ('c', 'c')]));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t(r"(?i)[^a-b[^c]]"), hir_uclass(& [('C', 'C'), ('c', 'c')]));
        assert_eq!(
            t_err(r"[^a-c[^c]]"), TestError { kind :
            hir::ErrorKind::EmptyClassNotAllowed, span : Span::new(Position::new(0, 1,
            1), Position::new(10, 1, 11)), }
        );
        #[cfg(feature = "unicode-case")]
        assert_eq!(
            t_err(r"(?i)[^a-c[^c]]"), TestError { kind :
            hir::ErrorKind::EmptyClassNotAllowed, span : Span::new(Position::new(4, 1,
            5), Position::new(14, 1, 15)), }
        );
    }
    #[test]
    fn class_bracketed_intersect() {
        assert_eq!(t("[abc&&b-c]"), hir_uclass(& [('b', 'c')]));
        assert_eq!(t("[abc&&[b-c]]"), hir_uclass(& [('b', 'c')]));
        assert_eq!(t("[[abc]&&[b-c]]"), hir_uclass(& [('b', 'c')]));
        assert_eq!(t("[a-z&&b-y&&c-x]"), hir_uclass(& [('c', 'x')]));
        assert_eq!(t("[c-da-b&&a-d]"), hir_uclass(& [('a', 'd')]));
        assert_eq!(t("[a-d&&c-da-b]"), hir_uclass(& [('a', 'd')]));
        assert_eq!(t(r"[a-z&&a-c]"), hir_uclass(& [('a', 'c')]));
        assert_eq!(t(r"[[a-z&&a-c]]"), hir_uclass(& [('a', 'c')]));
        assert_eq!(t(r"[^[a-z&&a-c]]"), hir_negate(hir_uclass(& [('a', 'c')])));
        assert_eq!(t("(?-u)[abc&&b-c]"), hir_bclass(& [(b'b', b'c')]));
        assert_eq!(t("(?-u)[abc&&[b-c]]"), hir_bclass(& [(b'b', b'c')]));
        assert_eq!(t("(?-u)[[abc]&&[b-c]]"), hir_bclass(& [(b'b', b'c')]));
        assert_eq!(t("(?-u)[a-z&&b-y&&c-x]"), hir_bclass(& [(b'c', b'x')]));
        assert_eq!(t("(?-u)[c-da-b&&a-d]"), hir_bclass(& [(b'a', b'd')]));
        assert_eq!(t("(?-u)[a-d&&c-da-b]"), hir_bclass(& [(b'a', b'd')]));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t("(?i)[abc&&b-c]"), hir_case_fold(hir_uclass(& [('b', 'c')])));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t("(?i)[abc&&[b-c]]"), hir_case_fold(hir_uclass(& [('b', 'c')])));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t("(?i)[[abc]&&[b-c]]"), hir_case_fold(hir_uclass(& [('b', 'c')])));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t("(?i)[a-z&&b-y&&c-x]"), hir_case_fold(hir_uclass(& [('c', 'x')])));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t("(?i)[c-da-b&&a-d]"), hir_case_fold(hir_uclass(& [('a', 'd')])));
        #[cfg(feature = "unicode-case")]
        assert_eq!(t("(?i)[a-d&&c-da-b]"), hir_case_fold(hir_uclass(& [('a', 'd')])));
        assert_eq!(t("(?i-u)[abc&&b-c]"), hir_case_fold(hir_bclass(& [(b'b', b'c')])));
        assert_eq!(t("(?i-u)[abc&&[b-c]]"), hir_case_fold(hir_bclass(& [(b'b', b'c')])));
        assert_eq!(
            t("(?i-u)[[abc]&&[b-c]]"), hir_case_fold(hir_bclass(& [(b'b', b'c')]))
        );
        assert_eq!(
            t("(?i-u)[a-z&&b-y&&c-x]"), hir_case_fold(hir_bclass(& [(b'c', b'x')]))
        );
        assert_eq!(
            t("(?i-u)[c-da-b&&a-d]"), hir_case_fold(hir_bclass(& [(b'a', b'd')]))
        );
        assert_eq!(
            t("(?i-u)[a-d&&c-da-b]"), hir_case_fold(hir_bclass(& [(b'a', b'd')]))
        );
        assert_eq!(t(r"[\^&&^]"), hir_uclass(& [('^', '^')]));
        assert_eq!(t(r"[]&&\]]"), hir_uclass(& [(']', ']')]));
        assert_eq!(t(r"[-&&-]"), hir_uclass(& [('-', '-')]));
        assert_eq!(t(r"[\&&&&]"), hir_uclass(& [('&', '&')]));
        assert_eq!(t(r"[\&&&\&]"), hir_uclass(& [('&', '&')]));
        assert_eq!(t(r"[a-w&&[^c-g]z]"), hir_uclass(& [('a', 'b'), ('h', 'w')]));
    }
    #[test]
    fn class_bracketed_intersect_negate() {
        #[cfg(feature = "unicode-perl")]
        assert_eq!(
            t(r"[^\w&&\d]"), hir_negate(hir_uclass_query(ClassQuery::Binary("digit")))
        );
        assert_eq!(t(r"[^[a-z&&a-c]]"), hir_negate(hir_uclass(& [('a', 'c')])));
        #[cfg(feature = "unicode-perl")]
        assert_eq!(
            t(r"[^[\w&&\d]]"), hir_negate(hir_uclass_query(ClassQuery::Binary("digit")))
        );
        #[cfg(feature = "unicode-perl")]
        assert_eq!(t(r"[^[^\w&&\d]]"), hir_uclass_query(ClassQuery::Binary("digit")));
        #[cfg(feature = "unicode-perl")]
        assert_eq!(t(r"[[[^\w]&&[^\d]]]"), hir_negate(hir_uclass_perl_word()));
        #[cfg(feature = "unicode-perl")]
        assert_eq!(
            t_bytes(r"(?-u)[^\w&&\d]"), hir_negate(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Digit)))
        );
        assert_eq!(
            t_bytes(r"(?-u)[^[a-z&&a-c]]"), hir_negate(hir_bclass(& [(b'a', b'c')]))
        );
        assert_eq!(
            t_bytes(r"(?-u)[^[\w&&\d]]"), hir_negate(hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Digit)))
        );
        assert_eq!(
            t_bytes(r"(?-u)[^[^\w&&\d]]"), hir_bclass_from_char(ascii_class(&
            ast::ClassAsciiKind::Digit))
        );
        assert_eq!(
            t_bytes(r"(?-u)[[[^\w]&&[^\d]]]"),
            hir_negate(hir_bclass_from_char(ascii_class(& ast::ClassAsciiKind::Word)))
        );
    }
    #[test]
    fn class_bracketed_difference() {
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(
            t(r"[\pL--[:ascii:]]"),
            hir_difference(hir_uclass_query(ClassQuery::Binary("letter")), hir_uclass(&
            [('\0', '\x7F')]))
        );
        assert_eq!(t(r"(?-u)[[:alpha:]--[:lower:]]"), hir_bclass(& [(b'A', b'Z')]));
    }
    #[test]
    fn class_bracketed_symmetric_difference() {
        #[cfg(feature = "unicode-script")]
        assert_eq!(
            t(r"[\p{sc:Greek}~~\p{scx:Greek}]"), hir_uclass(& [('\u{0342}', '\u{0342}'),
            ('\u{0345}', '\u{0345}'), ('\u{1DC0}', '\u{1DC1}'),])
        );
        assert_eq!(t(r"[a-g~~c-j]"), hir_uclass(& [('a', 'b'), ('h', 'j')]));
        assert_eq!(t(r"(?-u)[a-g~~c-j]"), hir_bclass(& [(b'a', b'b'), (b'h', b'j')]));
    }
    #[test]
    fn ignore_whitespace() {
        assert_eq!(t(r"(?x)\12 3"), hir_lit("\n3"));
        assert_eq!(t(r"(?x)\x { 53 }"), hir_lit("S"));
        assert_eq!(
            t(r"(?x)\x # comment
{ # comment
    53 # comment
} #comment"), hir_lit("S")
        );
        assert_eq!(t(r"(?x)\x 53"), hir_lit("S"));
        assert_eq!(t(r"(?x)\x # comment
        53 # comment"), hir_lit("S"));
        assert_eq!(t(r"(?x)\x5 3"), hir_lit("S"));
        #[cfg(feature = "unicode-gencat")]
        assert_eq!(
            t(r"(?x)\p # comment
{ # comment
    Separator # comment
} # comment"),
            hir_uclass_query(ClassQuery::Binary("separator"))
        );
        assert_eq!(
            t(r"(?x)a # comment
{ # comment
    5 # comment
    , # comment
    10 # comment
} # comment"),
            hir_range(true, hir::RepetitionRange::Bounded(5, 10), hir_lit("a"))
        );
        assert_eq!(t(r"(?x)a\  # hi there"), hir_lit("a "));
    }
    #[test]
    fn analysis_is_always_utf8() {
        assert!(t_bytes(r"a").is_always_utf8());
        assert!(t_bytes(r"ab").is_always_utf8());
        assert!(t_bytes(r"(?-u)a").is_always_utf8());
        assert!(t_bytes(r"(?-u)ab").is_always_utf8());
        assert!(t_bytes(r"\xFF").is_always_utf8());
        assert!(t_bytes(r"\xFF\xFF").is_always_utf8());
        assert!(t_bytes(r"[^a]").is_always_utf8());
        assert!(t_bytes(r"[^a][^a]").is_always_utf8());
        assert!(t_bytes(r"\b").is_always_utf8());
        assert!(t_bytes(r"\B").is_always_utf8());
        assert!(t_bytes(r"(?-u)\b").is_always_utf8());
        assert!(! t_bytes(r"(?-u)\xFF").is_always_utf8());
        assert!(! t_bytes(r"(?-u)\xFF\xFF").is_always_utf8());
        assert!(! t_bytes(r"(?-u)[^a]").is_always_utf8());
        assert!(! t_bytes(r"(?-u)[^a][^a]").is_always_utf8());
        assert!(! t_bytes(r"(?-u)\B").is_always_utf8());
    }
    #[test]
    fn analysis_is_all_assertions() {
        assert!(t(r"\b").is_all_assertions());
        assert!(t(r"\B").is_all_assertions());
        assert!(t(r"^").is_all_assertions());
        assert!(t(r"$").is_all_assertions());
        assert!(t(r"\A").is_all_assertions());
        assert!(t(r"\z").is_all_assertions());
        assert!(t(r"$^\z\A\b\B").is_all_assertions());
        assert!(t(r"$|^|\z|\A|\b|\B").is_all_assertions());
        assert!(t(r"^$|$^").is_all_assertions());
        assert!(t(r"((\b)+())*^").is_all_assertions());
        assert!(! t(r"^a").is_all_assertions());
    }
    #[test]
    fn analysis_is_anchored() {
        assert!(t(r"^").is_anchored_start());
        assert!(t(r"$").is_anchored_end());
        assert!(t(r"^").is_line_anchored_start());
        assert!(t(r"$").is_line_anchored_end());
        assert!(t(r"^^").is_anchored_start());
        assert!(t(r"$$").is_anchored_end());
        assert!(t(r"^^").is_line_anchored_start());
        assert!(t(r"$$").is_line_anchored_end());
        assert!(t(r"^$").is_anchored_start());
        assert!(t(r"^$").is_anchored_end());
        assert!(t(r"^$").is_line_anchored_start());
        assert!(t(r"^$").is_line_anchored_end());
        assert!(t(r"^foo").is_anchored_start());
        assert!(t(r"foo$").is_anchored_end());
        assert!(t(r"^foo").is_line_anchored_start());
        assert!(t(r"foo$").is_line_anchored_end());
        assert!(t(r"^foo|^bar").is_anchored_start());
        assert!(t(r"foo$|bar$").is_anchored_end());
        assert!(t(r"^foo|^bar").is_line_anchored_start());
        assert!(t(r"foo$|bar$").is_line_anchored_end());
        assert!(t(r"^(foo|bar)").is_anchored_start());
        assert!(t(r"(foo|bar)$").is_anchored_end());
        assert!(t(r"^(foo|bar)").is_line_anchored_start());
        assert!(t(r"(foo|bar)$").is_line_anchored_end());
        assert!(t(r"^+").is_anchored_start());
        assert!(t(r"$+").is_anchored_end());
        assert!(t(r"^+").is_line_anchored_start());
        assert!(t(r"$+").is_line_anchored_end());
        assert!(t(r"^++").is_anchored_start());
        assert!(t(r"$++").is_anchored_end());
        assert!(t(r"^++").is_line_anchored_start());
        assert!(t(r"$++").is_line_anchored_end());
        assert!(t(r"(^)+").is_anchored_start());
        assert!(t(r"($)+").is_anchored_end());
        assert!(t(r"(^)+").is_line_anchored_start());
        assert!(t(r"($)+").is_line_anchored_end());
        assert!(t(r"$^").is_anchored_start());
        assert!(t(r"$^").is_anchored_start());
        assert!(t(r"$^").is_line_anchored_end());
        assert!(t(r"$^").is_line_anchored_end());
        assert!(t(r"$^|^$").is_anchored_start());
        assert!(t(r"$^|^$").is_anchored_end());
        assert!(t(r"$^|^$").is_line_anchored_start());
        assert!(t(r"$^|^$").is_line_anchored_end());
        assert!(t(r"\b^").is_anchored_start());
        assert!(t(r"$\b").is_anchored_end());
        assert!(t(r"\b^").is_line_anchored_start());
        assert!(t(r"$\b").is_line_anchored_end());
        assert!(t(r"^(?m:^)").is_anchored_start());
        assert!(t(r"(?m:$)$").is_anchored_end());
        assert!(t(r"^(?m:^)").is_line_anchored_start());
        assert!(t(r"(?m:$)$").is_line_anchored_end());
        assert!(t(r"(?m:^)^").is_anchored_start());
        assert!(t(r"$(?m:$)").is_anchored_end());
        assert!(t(r"(?m:^)^").is_line_anchored_start());
        assert!(t(r"$(?m:$)").is_line_anchored_end());
        assert!(! t(r"(?m)^").is_anchored_start());
        assert!(! t(r"(?m)$").is_anchored_end());
        assert!(! t(r"(?m:^$)|$^").is_anchored_start());
        assert!(! t(r"(?m:^$)|$^").is_anchored_end());
        assert!(! t(r"$^|(?m:^$)").is_anchored_start());
        assert!(! t(r"$^|(?m:^$)").is_anchored_end());
        assert!(! t(r"a^").is_anchored_start());
        assert!(! t(r"$a").is_anchored_start());
        assert!(! t(r"a^").is_line_anchored_start());
        assert!(! t(r"$a").is_line_anchored_start());
        assert!(! t(r"a^").is_anchored_end());
        assert!(! t(r"$a").is_anchored_end());
        assert!(! t(r"a^").is_line_anchored_end());
        assert!(! t(r"$a").is_line_anchored_end());
        assert!(! t(r"^foo|bar").is_anchored_start());
        assert!(! t(r"foo|bar$").is_anchored_end());
        assert!(! t(r"^foo|bar").is_line_anchored_start());
        assert!(! t(r"foo|bar$").is_line_anchored_end());
        assert!(! t(r"^*").is_anchored_start());
        assert!(! t(r"$*").is_anchored_end());
        assert!(! t(r"^*").is_line_anchored_start());
        assert!(! t(r"$*").is_line_anchored_end());
        assert!(! t(r"^*+").is_anchored_start());
        assert!(! t(r"$*+").is_anchored_end());
        assert!(! t(r"^*+").is_line_anchored_start());
        assert!(! t(r"$*+").is_line_anchored_end());
        assert!(! t(r"^+*").is_anchored_start());
        assert!(! t(r"$+*").is_anchored_end());
        assert!(! t(r"^+*").is_line_anchored_start());
        assert!(! t(r"$+*").is_line_anchored_end());
        assert!(! t(r"(^)*").is_anchored_start());
        assert!(! t(r"($)*").is_anchored_end());
        assert!(! t(r"(^)*").is_line_anchored_start());
        assert!(! t(r"($)*").is_line_anchored_end());
    }
    #[test]
    fn analysis_is_line_anchored() {
        assert!(t(r"(?m)^(foo|bar)").is_line_anchored_start());
        assert!(t(r"(?m)(foo|bar)$").is_line_anchored_end());
        assert!(t(r"(?m)^foo|^bar").is_line_anchored_start());
        assert!(t(r"(?m)foo$|bar$").is_line_anchored_end());
        assert!(t(r"(?m)^").is_line_anchored_start());
        assert!(t(r"(?m)$").is_line_anchored_end());
        assert!(t(r"(?m:^$)|$^").is_line_anchored_start());
        assert!(t(r"(?m:^$)|$^").is_line_anchored_end());
        assert!(t(r"$^|(?m:^$)").is_line_anchored_start());
        assert!(t(r"$^|(?m:^$)").is_line_anchored_end());
    }
    #[test]
    fn analysis_is_any_anchored() {
        assert!(t(r"^").is_any_anchored_start());
        assert!(t(r"$").is_any_anchored_end());
        assert!(t(r"\A").is_any_anchored_start());
        assert!(t(r"\z").is_any_anchored_end());
        assert!(! t(r"(?m)^").is_any_anchored_start());
        assert!(! t(r"(?m)$").is_any_anchored_end());
        assert!(! t(r"$").is_any_anchored_start());
        assert!(! t(r"^").is_any_anchored_end());
    }
    #[test]
    fn analysis_is_match_empty() {
        assert!(t(r"").is_match_empty());
        assert!(t(r"()").is_match_empty());
        assert!(t(r"()*").is_match_empty());
        assert!(t(r"()+").is_match_empty());
        assert!(t(r"()?").is_match_empty());
        assert!(t(r"a*").is_match_empty());
        assert!(t(r"a?").is_match_empty());
        assert!(t(r"a{0}").is_match_empty());
        assert!(t(r"a{0,}").is_match_empty());
        assert!(t(r"a{0,1}").is_match_empty());
        assert!(t(r"a{0,10}").is_match_empty());
        #[cfg(feature = "unicode-gencat")] assert!(t(r"\pL*").is_match_empty());
        assert!(t(r"a*|b").is_match_empty());
        assert!(t(r"b|a*").is_match_empty());
        assert!(t(r"a*a?(abcd)*").is_match_empty());
        assert!(t(r"^").is_match_empty());
        assert!(t(r"$").is_match_empty());
        assert!(t(r"(?m)^").is_match_empty());
        assert!(t(r"(?m)$").is_match_empty());
        assert!(t(r"\A").is_match_empty());
        assert!(t(r"\z").is_match_empty());
        assert!(t(r"\B").is_match_empty());
        assert!(t_bytes(r"(?-u)\B").is_match_empty());
        assert!(! t(r"a+").is_match_empty());
        assert!(! t(r"a{1}").is_match_empty());
        assert!(! t(r"a{1,}").is_match_empty());
        assert!(! t(r"a{1,2}").is_match_empty());
        assert!(! t(r"a{1,10}").is_match_empty());
        assert!(! t(r"b|a").is_match_empty());
        assert!(! t(r"a*a+(abcd)*").is_match_empty());
        assert!(! t(r"\b").is_match_empty());
        assert!(! t(r"(?-u)\b").is_match_empty());
    }
    #[test]
    fn analysis_is_literal() {
        assert!(t(r"a").is_literal());
        assert!(t(r"ab").is_literal());
        assert!(t(r"abc").is_literal());
        assert!(t(r"(?m)abc").is_literal());
        assert!(! t(r"").is_literal());
        assert!(! t(r"^").is_literal());
        assert!(! t(r"a|b").is_literal());
        assert!(! t(r"(a)").is_literal());
        assert!(! t(r"a+").is_literal());
        assert!(! t(r"foo(a)").is_literal());
        assert!(! t(r"(a)foo").is_literal());
        assert!(! t(r"[a]").is_literal());
    }
    #[test]
    fn analysis_is_alternation_literal() {
        assert!(t(r"a").is_alternation_literal());
        assert!(t(r"ab").is_alternation_literal());
        assert!(t(r"abc").is_alternation_literal());
        assert!(t(r"(?m)abc").is_alternation_literal());
        assert!(t(r"a|b").is_alternation_literal());
        assert!(t(r"a|b|c").is_alternation_literal());
        assert!(t(r"foo|bar").is_alternation_literal());
        assert!(t(r"foo|bar|baz").is_alternation_literal());
        assert!(! t(r"").is_alternation_literal());
        assert!(! t(r"^").is_alternation_literal());
        assert!(! t(r"(a)").is_alternation_literal());
        assert!(! t(r"a+").is_alternation_literal());
        assert!(! t(r"foo(a)").is_alternation_literal());
        assert!(! t(r"(a)foo").is_alternation_literal());
        assert!(! t(r"[a]").is_alternation_literal());
        assert!(! t(r"[a]|b").is_alternation_literal());
        assert!(! t(r"a|[b]").is_alternation_literal());
        assert!(! t(r"(a)|b").is_alternation_literal());
        assert!(! t(r"a|(b)").is_alternation_literal());
    }
}
#[cfg(test)]
mod tests_llm_16_82 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_82_rrrruuuugggg_test_default = 0;
        let default: TranslatorBuilder = Default::default();
        debug_assert_eq!(default.allow_invalid_utf8, false);
        debug_assert_eq!(default.flags.case_insensitive, None);
        debug_assert_eq!(default.flags.multi_line, None);
        debug_assert_eq!(default.flags.dot_matches_new_line, None);
        debug_assert_eq!(default.flags.swap_greed, None);
        debug_assert_eq!(default.flags.unicode, None);
        let _rug_ed_tests_llm_16_82_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_518 {
    use super::*;
    use crate::*;
    #[test]
    fn test_case_insensitive() {
        let _rug_st_tests_llm_16_518_rrrruuuugggg_test_case_insensitive = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let mut flags = Flags::default();
        debug_assert_eq!(flags.case_insensitive(), false);
        flags.case_insensitive = Some(rug_fuzz_0);
        debug_assert_eq!(flags.case_insensitive(), true);
        flags.case_insensitive = Some(rug_fuzz_1);
        debug_assert_eq!(flags.case_insensitive(), false);
        flags.case_insensitive = None;
        debug_assert_eq!(flags.case_insensitive(), false);
        let previous_flags = Flags {
            case_insensitive: Some(rug_fuzz_2),
            ..Flags::default()
        };
        flags.merge(&previous_flags);
        debug_assert_eq!(flags.case_insensitive(), true);
        let previous_flags = Flags {
            case_insensitive: Some(rug_fuzz_3),
            ..Flags::default()
        };
        flags.merge(&previous_flags);
        debug_assert_eq!(flags.case_insensitive(), true);
        let previous_flags = Flags {
            case_insensitive: None,
            ..Flags::default()
        };
        flags.merge(&previous_flags);
        debug_assert_eq!(flags.case_insensitive(), true);
        let _rug_ed_tests_llm_16_518_rrrruuuugggg_test_case_insensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_519 {
    use super::*;
    use crate::*;
    #[test]
    fn test_dot_matches_new_line() {
        let _rug_st_tests_llm_16_519_rrrruuuugggg_test_dot_matches_new_line = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut flags = Flags::default();
        debug_assert_eq!(flags.dot_matches_new_line(), false);
        flags.dot_matches_new_line = Some(rug_fuzz_0);
        debug_assert_eq!(flags.dot_matches_new_line(), true);
        flags.dot_matches_new_line = Some(rug_fuzz_1);
        debug_assert_eq!(flags.dot_matches_new_line(), false);
        let _rug_ed_tests_llm_16_519_rrrruuuugggg_test_dot_matches_new_line = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_524 {
    use super::*;
    use crate::*;
    #[test]
    fn test_multi_line() {
        let _rug_st_tests_llm_16_524_rrrruuuugggg_test_multi_line = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let flags = Flags {
            case_insensitive: None,
            multi_line: Some(rug_fuzz_0),
            dot_matches_new_line: None,
            swap_greed: None,
            unicode: None,
        };
        debug_assert_eq!(flags.multi_line(), true);
        let flags = Flags {
            case_insensitive: None,
            multi_line: Some(rug_fuzz_1),
            dot_matches_new_line: None,
            swap_greed: None,
            unicode: None,
        };
        debug_assert_eq!(flags.multi_line(), false);
        let flags = Flags {
            case_insensitive: None,
            multi_line: None,
            dot_matches_new_line: None,
            swap_greed: None,
            unicode: None,
        };
        debug_assert_eq!(flags.multi_line(), false);
        let _rug_ed_tests_llm_16_524_rrrruuuugggg_test_multi_line = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_525 {
    use super::*;
    use crate::*;
    #[test]
    fn test_swap_greed() {
        let _rug_st_tests_llm_16_525_rrrruuuugggg_test_swap_greed = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = true;
        let rug_fuzz_5 = true;
        let rug_fuzz_6 = false;
        let rug_fuzz_7 = true;
        let rug_fuzz_8 = true;
        let rug_fuzz_9 = true;
        let rug_fuzz_10 = false;
        let rug_fuzz_11 = true;
        let rug_fuzz_12 = true;
        let rug_fuzz_13 = true;
        let flags = Flags {
            case_insensitive: Some(rug_fuzz_0),
            multi_line: Some(rug_fuzz_1),
            dot_matches_new_line: Some(rug_fuzz_2),
            swap_greed: Some(rug_fuzz_3),
            unicode: Some(rug_fuzz_4),
        };
        debug_assert_eq!(flags.swap_greed(), false);
        let flags = Flags {
            case_insensitive: Some(rug_fuzz_5),
            multi_line: Some(rug_fuzz_6),
            dot_matches_new_line: Some(rug_fuzz_7),
            swap_greed: None,
            unicode: Some(rug_fuzz_8),
        };
        debug_assert_eq!(flags.swap_greed(), false);
        let flags = Flags {
            case_insensitive: Some(rug_fuzz_9),
            multi_line: Some(rug_fuzz_10),
            dot_matches_new_line: Some(rug_fuzz_11),
            swap_greed: Some(rug_fuzz_12),
            unicode: Some(rug_fuzz_13),
        };
        debug_assert_eq!(flags.swap_greed(), true);
        let flags = Flags {
            case_insensitive: None,
            multi_line: None,
            dot_matches_new_line: None,
            swap_greed: None,
            unicode: None,
        };
        debug_assert_eq!(flags.swap_greed(), false);
        let _rug_ed_tests_llm_16_525_rrrruuuugggg_test_swap_greed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_526 {
    use super::*;
    use crate::*;
    #[test]
    fn test_unicode() {
        let _rug_st_tests_llm_16_526_rrrruuuugggg_test_unicode = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let flags = Flags {
            unicode: Some(rug_fuzz_0),
            ..Flags::default()
        };
        debug_assert_eq!(flags.unicode(), true);
        let flags = Flags {
            unicode: Some(rug_fuzz_1),
            ..Flags::default()
        };
        debug_assert_eq!(flags.unicode(), false);
        let flags = Flags {
            unicode: None,
            ..Flags::default()
        };
        debug_assert_eq!(flags.unicode(), true);
        let _rug_ed_tests_llm_16_526_rrrruuuugggg_test_unicode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_527 {
    use super::*;
    use crate::*;
    use hir::{ClassBytes, ClassBytesRange, ClassUnicode, Literal};
    #[test]
    fn test_unwrap_class_bytes() {
        let _rug_st_tests_llm_16_527_rrrruuuugggg_test_unwrap_class_bytes = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let cls = ClassBytes::new(
            vec![
                ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'A',
                b'Z')
            ],
        );
        let frame = HirFrame::ClassBytes(cls);
        let result = frame.unwrap_class_bytes();
        debug_assert_eq!(result.ranges().len(), 2);
        let _rug_ed_tests_llm_16_527_rrrruuuugggg_test_unwrap_class_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_528 {
    use super::*;
    use crate::*;
    use hir::{ClassUnicode, ClassBytes, Class};
    #[test]
    fn test_unwrap_class_unicode() {
        let _rug_st_tests_llm_16_528_rrrruuuugggg_test_unwrap_class_unicode = 0;
        let cls = ClassUnicode::new(vec![]);
        let frame = HirFrame::ClassUnicode(cls.clone());
        debug_assert_eq!(frame.unwrap_class_unicode(), cls);
        let _rug_ed_tests_llm_16_528_rrrruuuugggg_test_unwrap_class_unicode = 0;
    }
    #[test]
    #[should_panic]
    fn test_unwrap_class_unicode_panic() {
        let _rug_st_tests_llm_16_528_rrrruuuugggg_test_unwrap_class_unicode_panic = 0;
        let frame = HirFrame::ClassBytes(ClassBytes::empty());
        frame.unwrap_class_unicode();
        let _rug_ed_tests_llm_16_528_rrrruuuugggg_test_unwrap_class_unicode_panic = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_531 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new_translator() {
        let _rug_st_tests_llm_16_531_rrrruuuugggg_test_new_translator = 0;
        let t = Translator::new();
        debug_assert_eq!(t.allow_invalid_utf8, false);
        let _rug_ed_tests_llm_16_531_rrrruuuugggg_test_new_translator = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_533 {
    use crate::hir::translate::{TranslatorBuilder, Flags};
    #[test]
    fn test_allow_invalid_utf8() {
        let _rug_st_tests_llm_16_533_rrrruuuugggg_test_allow_invalid_utf8 = 0;
        let rug_fuzz_0 = true;
        let mut translator_builder = TranslatorBuilder::new();
        translator_builder.allow_invalid_utf8(rug_fuzz_0);
        let translator = translator_builder.build();
        debug_assert_eq!(translator.allow_invalid_utf8, true);
        let _rug_ed_tests_llm_16_533_rrrruuuugggg_test_allow_invalid_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_536 {
    use super::*;
    use crate::*;
    #[test]
    fn test_case_insensitive() {
        let _rug_st_tests_llm_16_536_rrrruuuugggg_test_case_insensitive = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = false;
        let mut translator_builder = TranslatorBuilder::new();
        translator_builder.case_insensitive(rug_fuzz_0);
        debug_assert_eq!(rug_fuzz_1, translator_builder.flags.case_insensitive.unwrap());
        translator_builder.case_insensitive(rug_fuzz_2);
        debug_assert_eq!(rug_fuzz_3, translator_builder.flags.case_insensitive.unwrap());
        let _rug_ed_tests_llm_16_536_rrrruuuugggg_test_case_insensitive = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_537 {
    use crate::hir::translate::{TranslatorBuilder, Flags};
    #[test]
    fn test_dot_matches_new_line() {
        let _rug_st_tests_llm_16_537_rrrruuuugggg_test_dot_matches_new_line = 0;
        let rug_fuzz_0 = true;
        let mut builder = TranslatorBuilder::new();
        builder.dot_matches_new_line(rug_fuzz_0);
        let flags = Flags::default();
        debug_assert_eq!(builder.flags.dot_matches_new_line, Some(true));
        let _rug_ed_tests_llm_16_537_rrrruuuugggg_test_dot_matches_new_line = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_538 {
    use crate::hir::translate::TranslatorBuilder;
    #[test]
    fn test_multi_line() {
        let _rug_st_tests_llm_16_538_rrrruuuugggg_test_multi_line = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut builder = TranslatorBuilder::new();
        builder.multi_line(rug_fuzz_0);
        let flags = builder.flags;
        debug_assert_eq!(flags.multi_line, Some(true));
        builder.multi_line(rug_fuzz_1);
        let flags = builder.flags;
        debug_assert_eq!(flags.multi_line, Some(false));
        let _rug_ed_tests_llm_16_538_rrrruuuugggg_test_multi_line = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_539 {
    use crate::hir::translate::{TranslatorBuilder, Flags};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_539_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let builder = TranslatorBuilder::new();
        let expected_flags = Flags {
            case_insensitive: None,
            multi_line: None,
            dot_matches_new_line: None,
            swap_greed: None,
            unicode: Some(rug_fuzz_0),
        };
        let expected_builder = TranslatorBuilder {
            allow_invalid_utf8: rug_fuzz_1,
            flags: expected_flags.clone(),
        };
        debug_assert_eq!(
            builder.allow_invalid_utf8, expected_builder.allow_invalid_utf8
        );
        debug_assert_eq!(
            builder.flags.case_insensitive, expected_builder.flags.case_insensitive
        );
        debug_assert_eq!(builder.flags.multi_line, expected_builder.flags.multi_line);
        debug_assert_eq!(
            builder.flags.dot_matches_new_line, expected_builder.flags
            .dot_matches_new_line
        );
        debug_assert_eq!(builder.flags.swap_greed, expected_builder.flags.swap_greed);
        debug_assert_eq!(builder.flags.unicode, expected_builder.flags.unicode);
        let _rug_ed_tests_llm_16_539_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_540 {
    use super::*;
    use crate::*;
    use crate::hir::translate::Flags;
    #[test]
    fn test_swap_greed() {
        let _rug_st_tests_llm_16_540_rrrruuuugggg_test_swap_greed = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = true;
        let mut builder = TranslatorBuilder::new();
        let mut flags = Flags::default();
        flags.swap_greed = Some(rug_fuzz_0);
        builder.flags = flags;
        let result = builder.swap_greed(rug_fuzz_1);
        debug_assert_eq!(result.flags.swap_greed, Some(false));
        let result = builder.swap_greed(rug_fuzz_2);
        debug_assert_eq!(result.flags.swap_greed, Some(true));
        let _rug_ed_tests_llm_16_540_rrrruuuugggg_test_swap_greed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_541 {
    use super::*;
    use crate::*;
    #[test]
    fn test_unicode() {
        let _rug_st_tests_llm_16_541_rrrruuuugggg_test_unicode = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let mut builder = TranslatorBuilder::new();
        builder.unicode(rug_fuzz_0);
        debug_assert_eq!(builder.flags.unicode, None);
        builder.unicode(rug_fuzz_1);
        debug_assert_eq!(builder.flags.unicode, Some(false));
        let _rug_ed_tests_llm_16_541_rrrruuuugggg_test_unicode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_561 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_flags() {
        let _rug_st_tests_llm_16_561_rrrruuuugggg_test_set_flags = 0;
        let _rug_ed_tests_llm_16_561_rrrruuuugggg_test_set_flags = 0;
    }
}
mod tests_llm_16_565 {
    use crate::hir::translate::ascii_class;
    use crate::ast::ClassAsciiKind;
    #[test]
    fn test_ascii_class() {
        let _rug_st_tests_llm_16_565_rrrruuuugggg_test_ascii_class = 0;
        debug_assert_eq!(
            ascii_class(& ClassAsciiKind::Alnum), & [('0', '9'), ('A', 'Z'), ('a', 'z')]
        );
        debug_assert_eq!(
            ascii_class(& ClassAsciiKind::Alpha), & [('A', 'Z'), ('a', 'z')]
        );
        debug_assert_eq!(ascii_class(& ClassAsciiKind::Ascii), & [('\x00', '\x7F')]);
        debug_assert_eq!(
            ascii_class(& ClassAsciiKind::Blank), & [('\t', '\t'), (' ', ' ')]
        );
        debug_assert_eq!(
            ascii_class(& ClassAsciiKind::Cntrl), & [('\x00', '\x1F'), ('\x7F', '\x7F')]
        );
        debug_assert_eq!(ascii_class(& ClassAsciiKind::Digit), & [('0', '9')]);
        debug_assert_eq!(ascii_class(& ClassAsciiKind::Graph), & [('!', '~')]);
        debug_assert_eq!(ascii_class(& ClassAsciiKind::Lower), & [('a', 'z')]);
        debug_assert_eq!(ascii_class(& ClassAsciiKind::Print), & [(' ', '~')]);
        debug_assert_eq!(
            ascii_class(& ClassAsciiKind::Punct), & [('!', '/'), (':', '@'), ('[', '`'),
            ('{', '~')]
        );
        debug_assert_eq!(
            ascii_class(& ClassAsciiKind::Space), & [('\t', '\t'), ('\n', '\n'), ('\x0B',
            '\x0B'), ('\x0C', '\x0C'), ('\r', '\r'), (' ', ' ')]
        );
        debug_assert_eq!(ascii_class(& ClassAsciiKind::Upper), & [('A', 'Z')]);
        debug_assert_eq!(
            ascii_class(& ClassAsciiKind::Word), & [('0', '9'), ('A', 'Z'), ('_', '_'),
            ('a', 'z')]
        );
        debug_assert_eq!(
            ascii_class(& ClassAsciiKind::Xdigit), & [('0', '9'), ('A', 'F'), ('a', 'f')]
        );
        let _rug_ed_tests_llm_16_565_rrrruuuugggg_test_ascii_class = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_566 {
    use super::*;
    use crate::*;
    use crate::ast::ClassAsciiKind;
    #[test]
    fn test_hir_ascii_class_bytes() {
        let _rug_st_tests_llm_16_566_rrrruuuugggg_test_hir_ascii_class_bytes = 0;
        let rug_fuzz_0 = 65;
        let rug_fuzz_1 = 90;
        let kind = ClassAsciiKind::Alpha;
        let expected_ranges = vec![
            hir::ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1),
            hir::ClassBytesRange::new(97, 122)
        ];
        let expected = hir::ClassBytes::new(expected_ranges);
        let result = hir_ascii_class_bytes(&kind);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_566_rrrruuuugggg_test_hir_ascii_class_bytes = 0;
    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use std::cell::Cell;
    use std::cell::RefCell;
    use crate::hir::translate::{Translator, TranslatorBuilder};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_135_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let mut p0: TranslatorBuilder = TranslatorBuilder::new();
        p0.allow_invalid_utf8(rug_fuzz_0);
        <hir::translate::TranslatorBuilder>::build(&p0);
        let _rug_ed_tests_rug_135_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::hir::translate::Translator;
    use crate::ast;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_136_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_pattern";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let mut p0: Translator = Translator::new();
        let p1: &str = rug_fuzz_0;
        let mut p2: ast::Ast = ast::Ast::Empty(
            ast::Span::splat(ast::Position::new(rug_fuzz_1, rug_fuzz_2, rug_fuzz_3)),
        );
        <hir::translate::Translator>::translate(&mut p0, p1, &p2);
        let _rug_ed_tests_rug_136_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::hir::translate::HirFrame;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_rug = 0;
        let mut p0 = HirFrame::Group {
            old_flags: Flags::default(),
        };
        HirFrame::unwrap_group(p0);
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::ast::Visitor;
    use crate::ast::Visitor as AstVisitor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_rug = 0;
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        <hir::translate::TranslatorI<'_, '_> as AstVisitor>::finish(p0).unwrap();
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::ast::Visitor;
    use crate::ast;
    use crate::hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_140_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let mut p1: ast::Ast = ast::Ast::Empty(
            ast::Span::splat(ast::Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
        );
        p0.visit_pre(&p1).unwrap();
        let _rug_ed_tests_rug_140_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_141 {
    use super::*;
    use crate::ast::Visitor;
    #[test]
    fn test_visit_post() {
        let _rug_st_tests_rug_141_rrrruuuugggg_test_visit_post = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let mut p0: hir::translate::TranslatorI<'static, 'static> = todo!();
        let mut p1 = ast::Ast::Empty(
            ast::Span::splat(ast::Position::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2)),
        );
        p0.visit_post(&p1).unwrap();
        let _rug_ed_tests_rug_141_rrrruuuugggg_test_visit_post = 0;
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::ast::Visitor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_144_rrrruuuugggg_test_rug = 0;
        let mut p0: hir::translate::TranslatorI<'static, 'static> = panic!(
            "TODO: construct p0"
        );
        let p1: ast::ClassSetBinaryOp = panic!("TODO: construct p1");
        p0.visit_class_set_binary_op_pre(&p1).unwrap();
        let _rug_ed_tests_rug_144_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    use crate::hir::translate::Translator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_147_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_pattern";
        let mut p0: Translator = Translator::new();
        let p1: &'static str = rug_fuzz_0;
        <hir::translate::TranslatorI<'_, '_>>::new(&p0, &p1);
        let _rug_ed_tests_rug_147_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::hir::translate::Translator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_148_rrrruuuugggg_test_rug = 0;
        let mut p0: hir::translate::TranslatorI<'static, 'static> = unimplemented!();
        p0.trans();
        let _rug_ed_tests_rug_148_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::hir;
    use crate::ast;
    use std::cell::RefCell;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_149_rrrruuuugggg_test_rug = 0;
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let mut p1: hir::translate::HirFrame = unimplemented!();
        p0.push(p1);
        let _rug_ed_tests_rug_149_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::ast;
    use crate::ast::LiteralKind;
    use crate::ast::HexLiteralKind;
    use crate::hir;
    use crate::hir::translate::TranslatorI;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_153_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let lit = ast::Literal {
            span: unimplemented!(),
            kind: LiteralKind::HexFixed(HexLiteralKind::X),
            c: rug_fuzz_0,
        };
        let p1: &ast::Literal = &lit;
        p0.hir_literal(p1).unwrap();
        let _rug_ed_tests_rug_153_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_154_rrrruuuugggg_test_rug = 0;
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let p1: ast::Literal = unimplemented!();
        p0.literal_to_char(&p1);
        let _rug_ed_tests_rug_154_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::ast;
    use crate::hir;
    #[test]
    fn test_hir_from_char() {
        let _rug_st_tests_rug_155_rrrruuuugggg_test_hir_from_char = 0;
        let rug_fuzz_0 = 'a';
        let mut p0: hir::translate::TranslatorI<'_, '_> = todo!();
        let mut p1: ast::Span = todo!();
        let mut p2: char = rug_fuzz_0;
        debug_assert_eq!(
            p0.hir_from_char(p1, p2), Ok(hir::Hir::literal(hir::Literal::Unicode(p2)))
        );
        let _rug_ed_tests_rug_155_rrrruuuugggg_test_hir_from_char = 0;
    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::hir::{Hir, Anchor, WordBoundary};
    #[test]
    fn test_hir_assertion() {
        let _rug_st_tests_rug_158_rrrruuuugggg_test_hir_assertion = 0;
        let mut translator: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let assertion: ast::Assertion = unimplemented!();
        let result = translator.hir_assertion(&assertion);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_158_rrrruuuugggg_test_hir_assertion = 0;
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::ast;
    use crate::hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_160_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let p0: hir::translate::TranslatorI<'static, 'static> = unimplemented!();
        let p1: ast::Repetition = unimplemented!();
        let p2: hir::Hir = {
            use crate::hir::*;
            let v41 = Hir::literal(Literal::Unicode(rug_fuzz_0));
            v41
        };
        hir::translate::TranslatorI::<'static, 'static>::hir_repetition(&p0, &p1, p2);
        let _rug_ed_tests_rug_160_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use std::ops::Not;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_161_rrrruuuugggg_test_rug = 0;
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let p1: ast::ClassUnicode = unimplemented!();
        crate::hir::translate::TranslatorI::hir_unicode_class(&mut p0, &p1);
        let _rug_ed_tests_rug_161_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::{ast, hir, unicode};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_162_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let p1 = ast::ClassPerl {
            span: unimplemented!(),
            kind: ast::ClassPerlKind::Digit,
            negated: rug_fuzz_0,
        };
        <hir::translate::TranslatorI<'_, '_>>::hir_perl_unicode_class(&mut p0, &p1)
            .unwrap();
        let _rug_ed_tests_rug_162_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::{ast, hir};
    #[test]
    fn test_hir_perl_byte_class() {
        let _rug_st_tests_rug_163_rrrruuuugggg_test_hir_perl_byte_class = 0;
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let p1: ast::ClassPerl = unimplemented!();
        p0.hir_perl_byte_class(&p1);
        let _rug_ed_tests_rug_163_rrrruuuugggg_test_hir_perl_byte_class = 0;
    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::ast;
    use crate::hir;
    use std::result;
    #[test]
    fn test_convert_unicode_class_error() {
        let _rug_st_tests_rug_164_rrrruuuugggg_test_convert_unicode_class_error = 0;
        let translator: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let span: ast::Span = unimplemented!();
        let result: result::Result<hir::ClassUnicode, unicode::Error> = unimplemented!();
        translator.convert_unicode_class_error(&span, result);
        let _rug_ed_tests_rug_164_rrrruuuugggg_test_convert_unicode_class_error = 0;
    }
}
#[cfg(test)]
mod tests_rug_165 {
    use super::*;
    use hir::translate::TranslatorI;
    use ast::{self, Span};
    use hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_165_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let mut p1: ast::Span = unimplemented!();
        let p2: bool = rug_fuzz_0;
        let mut p3: hir::ClassUnicode = unimplemented!();
        p0.unicode_fold_and_negate(&p1, p2, &mut p3).unwrap();
        let _rug_ed_tests_rug_165_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::ast;
    use crate::hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_166_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let rug_fuzz_2 = "Failed to apply bytes fold and negate";
        let mut p0: hir::translate::TranslatorI<'_, '_> = unimplemented!();
        let mut p1: ast::Span = unimplemented!();
        let mut p2: bool = unimplemented!();
        let mut p3: hir::ClassBytes = {
            let mut v52 = hir::ClassBytes::new(
                vec![
                    hir::ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1),
                    hir::ClassBytesRange::new(b'A', b'Z')
                ],
            );
            v52
        };
        p0.bytes_fold_and_negate(&p1, p2, &mut p3).expect(rug_fuzz_2);
        let _rug_ed_tests_rug_166_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_169 {
    use super::*;
    #[test]
    fn test_merge() {
        let _rug_st_tests_rug_169_rrrruuuugggg_test_merge = 0;
        let rug_fuzz_0 = true;
        let rug_fuzz_1 = false;
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = false;
        let rug_fuzz_4 = true;
        let rug_fuzz_5 = false;
        let rug_fuzz_6 = true;
        let rug_fuzz_7 = false;
        let mut p0 = <hir::translate::Flags>::default();
        let mut p1 = <hir::translate::Flags>::default();
        p0.case_insensitive = Some(rug_fuzz_0);
        p0.multi_line = Some(rug_fuzz_1);
        p0.dot_matches_new_line = Some(rug_fuzz_2);
        p0.swap_greed = Some(rug_fuzz_3);
        p0.unicode = Some(rug_fuzz_4);
        p1.case_insensitive = Some(rug_fuzz_5);
        p1.multi_line = Some(rug_fuzz_6);
        p1.dot_matches_new_line = Some(rug_fuzz_7);
        <hir::translate::Flags>::merge(&mut p1, &p0);
        debug_assert_eq!(p1.case_insensitive(), true);
        debug_assert_eq!(p1.multi_line(), true);
        debug_assert_eq!(p1.dot_matches_new_line(), false);
        let _rug_ed_tests_rug_169_rrrruuuugggg_test_merge = 0;
    }
}
