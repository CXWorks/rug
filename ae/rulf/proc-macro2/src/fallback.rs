use crate::parse::{token_stream, Cursor};
use crate::{Delimiter, Spacing, TokenTree};
#[cfg(span_locations)]
use std::cell::RefCell;
#[cfg(span_locations)]
use std::cmp;
use std::fmt::{self, Debug, Display};
use std::iter::FromIterator;
use std::mem;
use std::ops::RangeBounds;
#[cfg(procmacro2_semver_exempt)]
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::vec;
use unicode_xid::UnicodeXID;
/// Force use of proc-macro2's fallback implementation of the API for now, even
/// if the compiler's implementation is available.
pub fn force() {
    #[cfg(wrap_proc_macro)] crate::detection::force_fallback();
}
/// Resume using the compiler's implementation of the proc macro API if it is
/// available.
pub fn unforce() {
    #[cfg(wrap_proc_macro)] crate::detection::unforce_fallback();
}
#[derive(Clone)]
pub(crate) struct TokenStream {
    pub(crate) inner: Vec<TokenTree>,
}
#[derive(Debug)]
pub(crate) struct LexError;
impl TokenStream {
    pub fn new() -> TokenStream {
        TokenStream { inner: Vec::new() }
    }
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }
    fn take_inner(&mut self) -> Vec<TokenTree> {
        mem::replace(&mut self.inner, Vec::new())
    }
    fn push_token(&mut self, token: TokenTree) {
        match token {
            #[cfg(not(no_bind_by_move_pattern_guard))]
            TokenTree::Literal(
                crate::Literal {
                    #[cfg(wrap_proc_macro)]
                    inner: crate::imp::Literal::Fallback(literal),
                    #[cfg(not(wrap_proc_macro))]
                    inner: literal,
                    ..
                },
            ) if literal.text.starts_with('-') => {
                push_negative_literal(self, literal);
            }
            #[cfg(no_bind_by_move_pattern_guard)]
            TokenTree::Literal(
                crate::Literal {
                    #[cfg(wrap_proc_macro)]
                    inner: crate::imp::Literal::Fallback(literal),
                    #[cfg(not(wrap_proc_macro))]
                    inner: literal,
                    ..
                },
            ) => {
                if literal.text.starts_with('-') {
                    push_negative_literal(self, literal);
                } else {
                    self.inner
                        .push(TokenTree::Literal(crate::Literal::_new_stable(literal)));
                }
            }
            _ => self.inner.push(token),
        }
        #[cold]
        fn push_negative_literal(stream: &mut TokenStream, mut literal: Literal) {
            literal.text.remove(0);
            let mut punct = crate::Punct::new('-', Spacing::Alone);
            punct.set_span(crate::Span::_new_stable(literal.span));
            stream.inner.push(TokenTree::Punct(punct));
            stream.inner.push(TokenTree::Literal(crate::Literal::_new_stable(literal)));
        }
    }
}
impl Drop for TokenStream {
    fn drop(&mut self) {
        while let Some(token) = self.inner.pop() {
            let group = match token {
                TokenTree::Group(group) => group.inner,
                _ => continue,
            };
            #[cfg(wrap_proc_macro)]
            let group = match group {
                crate::imp::Group::Fallback(group) => group,
                _ => continue,
            };
            let mut group = group;
            self.inner.extend(group.stream.take_inner());
        }
    }
}
#[cfg(span_locations)]
fn get_cursor(src: &str) -> Cursor {
    SOURCE_MAP
        .with(|cm| {
            let mut cm = cm.borrow_mut();
            let name = format!("<parsed string {}>", cm.files.len());
            let span = cm.add_file(&name, src);
            Cursor { rest: src, off: span.lo }
        })
}
#[cfg(not(span_locations))]
fn get_cursor(src: &str) -> Cursor {
    Cursor { rest: src }
}
impl FromStr for TokenStream {
    type Err = LexError;
    fn from_str(src: &str) -> Result<TokenStream, LexError> {
        let cursor = get_cursor(src);
        let (rest, tokens) = token_stream(cursor)?;
        if rest.is_empty() { Ok(tokens) } else { Err(LexError) }
    }
}
impl Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("cannot parse string into token stream")
    }
}
impl Display for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut joint = false;
        for (i, tt) in self.inner.iter().enumerate() {
            if i != 0 && !joint {
                write!(f, " ")?;
            }
            joint = false;
            match tt {
                TokenTree::Group(tt) => Display::fmt(tt, f),
                TokenTree::Ident(tt) => Display::fmt(tt, f),
                TokenTree::Punct(tt) => {
                    joint = tt.spacing() == Spacing::Joint;
                    Display::fmt(tt, f)
                }
                TokenTree::Literal(tt) => Display::fmt(tt, f),
            }?
        }
        Ok(())
    }
}
impl Debug for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("TokenStream ")?;
        f.debug_list().entries(self.clone()).finish()
    }
}
#[cfg(use_proc_macro)]
impl From<proc_macro::TokenStream> for TokenStream {
    fn from(inner: proc_macro::TokenStream) -> TokenStream {
        inner.to_string().parse().expect("compiler token stream parse failed")
    }
}
#[cfg(use_proc_macro)]
impl From<TokenStream> for proc_macro::TokenStream {
    fn from(inner: TokenStream) -> proc_macro::TokenStream {
        inner.to_string().parse().expect("failed to parse to compiler tokens")
    }
}
impl From<TokenTree> for TokenStream {
    fn from(tree: TokenTree) -> TokenStream {
        let mut stream = TokenStream::new();
        stream.push_token(tree);
        stream
    }
}
impl FromIterator<TokenTree> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenTree>>(tokens: I) -> Self {
        let mut stream = TokenStream::new();
        stream.extend(tokens);
        stream
    }
}
impl FromIterator<TokenStream> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenStream>>(streams: I) -> Self {
        let mut v = Vec::new();
        for mut stream in streams {
            v.extend(stream.take_inner());
        }
        TokenStream { inner: v }
    }
}
impl Extend<TokenTree> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenTree>>(&mut self, tokens: I) {
        tokens.into_iter().for_each(|token| self.push_token(token));
    }
}
impl Extend<TokenStream> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenStream>>(&mut self, streams: I) {
        self.inner.extend(streams.into_iter().flatten());
    }
}
pub(crate) type TokenTreeIter = vec::IntoIter<TokenTree>;
impl IntoIterator for TokenStream {
    type Item = TokenTree;
    type IntoIter = TokenTreeIter;
    fn into_iter(mut self) -> TokenTreeIter {
        self.take_inner().into_iter()
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SourceFile {
    path: PathBuf,
}
impl SourceFile {
    /// Get the path to this source file as a string.
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn is_real(&self) -> bool {
        false
    }
}
impl Debug for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SourceFile")
            .field("path", &self.path())
            .field("is_real", &self.is_real())
            .finish()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineColumn {
    pub line: usize,
    pub column: usize,
}
#[cfg(span_locations)]
thread_local! {
    static SOURCE_MAP : RefCell < SourceMap > = RefCell::new(SourceMap { files :
    vec![FileInfo { #[cfg(procmacro2_semver_exempt)] name : "<unspecified>".to_owned(),
    span : Span { lo : 0, hi : 0 }, lines : vec![0], }], });
}
#[cfg(span_locations)]
struct FileInfo {
    #[cfg(procmacro2_semver_exempt)]
    name: String,
    span: Span,
    lines: Vec<usize>,
}
#[cfg(span_locations)]
impl FileInfo {
    fn offset_line_column(&self, offset: usize) -> LineColumn {
        assert!(self.span_within(Span { lo : offset as u32, hi : offset as u32 }));
        let offset = offset - self.span.lo as usize;
        match self.lines.binary_search(&offset) {
            Ok(found) => {
                LineColumn {
                    line: found + 1,
                    column: 0,
                }
            }
            Err(idx) => {
                LineColumn {
                    line: idx,
                    column: offset - self.lines[idx - 1],
                }
            }
        }
    }
    fn span_within(&self, span: Span) -> bool {
        span.lo >= self.span.lo && span.hi <= self.span.hi
    }
}
/// Computes the offsets of each line in the given source string
/// and the total number of characters
#[cfg(span_locations)]
fn lines_offsets(s: &str) -> (usize, Vec<usize>) {
    let mut lines = vec![0];
    let mut total = 0;
    for ch in s.chars() {
        total += 1;
        if ch == '\n' {
            lines.push(total);
        }
    }
    (total, lines)
}
#[cfg(span_locations)]
struct SourceMap {
    files: Vec<FileInfo>,
}
#[cfg(span_locations)]
impl SourceMap {
    fn next_start_pos(&self) -> u32 {
        self.files.last().unwrap().span.hi + 1
    }
    fn add_file(&mut self, name: &str, src: &str) -> Span {
        let (len, lines) = lines_offsets(src);
        let lo = self.next_start_pos();
        let span = Span { lo, hi: lo + (len as u32) };
        self.files
            .push(FileInfo {
                #[cfg(procmacro2_semver_exempt)]
                name: name.to_owned(),
                span,
                lines,
            });
        #[cfg(not(procmacro2_semver_exempt))]
        let _ = name;
        span
    }
    fn fileinfo(&self, span: Span) -> &FileInfo {
        for file in &self.files {
            if file.span_within(span) {
                return file;
            }
        }
        panic!("Invalid span with no related FileInfo!");
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    #[cfg(span_locations)]
    pub(crate) lo: u32,
    #[cfg(span_locations)]
    pub(crate) hi: u32,
}
impl Span {
    #[cfg(not(span_locations))]
    pub fn call_site() -> Span {
        Span {}
    }
    #[cfg(span_locations)]
    pub fn call_site() -> Span {
        Span { lo: 0, hi: 0 }
    }
    #[cfg(hygiene)]
    pub fn mixed_site() -> Span {
        Span::call_site()
    }
    #[cfg(procmacro2_semver_exempt)]
    pub fn def_site() -> Span {
        Span::call_site()
    }
    pub fn resolved_at(&self, _other: Span) -> Span {
        *self
    }
    pub fn located_at(&self, other: Span) -> Span {
        other
    }
    #[cfg(procmacro2_semver_exempt)]
    pub fn source_file(&self) -> SourceFile {
        SOURCE_MAP
            .with(|cm| {
                let cm = cm.borrow();
                let fi = cm.fileinfo(*self);
                SourceFile {
                    path: Path::new(&fi.name).to_owned(),
                }
            })
    }
    #[cfg(span_locations)]
    pub fn start(&self) -> LineColumn {
        SOURCE_MAP
            .with(|cm| {
                let cm = cm.borrow();
                let fi = cm.fileinfo(*self);
                fi.offset_line_column(self.lo as usize)
            })
    }
    #[cfg(span_locations)]
    pub fn end(&self) -> LineColumn {
        SOURCE_MAP
            .with(|cm| {
                let cm = cm.borrow();
                let fi = cm.fileinfo(*self);
                fi.offset_line_column(self.hi as usize)
            })
    }
    #[cfg(not(span_locations))]
    pub fn join(&self, _other: Span) -> Option<Span> {
        Some(Span {})
    }
    #[cfg(span_locations)]
    pub fn join(&self, other: Span) -> Option<Span> {
        SOURCE_MAP
            .with(|cm| {
                let cm = cm.borrow();
                if !cm.fileinfo(*self).span_within(other) {
                    return None;
                }
                Some(Span {
                    lo: cmp::min(self.lo, other.lo),
                    hi: cmp::max(self.hi, other.hi),
                })
            })
    }
    #[cfg(not(span_locations))]
    fn first_byte(self) -> Self {
        self
    }
    #[cfg(span_locations)]
    fn first_byte(self) -> Self {
        Span {
            lo: self.lo,
            hi: cmp::min(self.lo.saturating_add(1), self.hi),
        }
    }
    #[cfg(not(span_locations))]
    fn last_byte(self) -> Self {
        self
    }
    #[cfg(span_locations)]
    fn last_byte(self) -> Self {
        Span {
            lo: cmp::max(self.hi.saturating_sub(1), self.lo),
            hi: self.hi,
        }
    }
}
impl Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(span_locations)] return write!(f, "bytes({}..{})", self.lo, self.hi);
        #[cfg(not(span_locations))] write!(f, "Span")
    }
}
pub(crate) fn debug_span_field_if_nontrivial(debug: &mut fmt::DebugStruct, span: Span) {
    #[cfg(span_locations)]
    {
        if span.lo == 0 && span.hi == 0 {
            return;
        }
    }
    if cfg!(span_locations) {
        debug.field("span", &span);
    }
}
#[derive(Clone)]
pub(crate) struct Group {
    delimiter: Delimiter,
    stream: TokenStream,
    span: Span,
}
impl Group {
    pub fn new(delimiter: Delimiter, stream: TokenStream) -> Group {
        Group {
            delimiter,
            stream,
            span: Span::call_site(),
        }
    }
    pub fn delimiter(&self) -> Delimiter {
        self.delimiter
    }
    pub fn stream(&self) -> TokenStream {
        self.stream.clone()
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn span_open(&self) -> Span {
        self.span.first_byte()
    }
    pub fn span_close(&self) -> Span {
        self.span.last_byte()
    }
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}
impl Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (open, close) = match self.delimiter {
            Delimiter::Parenthesis => ("(", ")"),
            Delimiter::Brace => ("{ ", "}"),
            Delimiter::Bracket => ("[", "]"),
            Delimiter::None => ("", ""),
        };
        f.write_str(open)?;
        Display::fmt(&self.stream, f)?;
        if self.delimiter == Delimiter::Brace && !self.stream.inner.is_empty() {
            f.write_str(" ")?;
        }
        f.write_str(close)?;
        Ok(())
    }
}
impl Debug for Group {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = fmt.debug_struct("Group");
        debug.field("delimiter", &self.delimiter);
        debug.field("stream", &self.stream);
        debug_span_field_if_nontrivial(&mut debug, self.span);
        debug.finish()
    }
}
#[derive(Clone)]
pub(crate) struct Ident {
    sym: String,
    span: Span,
    raw: bool,
}
impl Ident {
    fn _new(string: &str, raw: bool, span: Span) -> Ident {
        validate_ident(string);
        Ident {
            sym: string.to_owned(),
            span,
            raw,
        }
    }
    pub fn new(string: &str, span: Span) -> Ident {
        Ident::_new(string, false, span)
    }
    pub fn new_raw(string: &str, span: Span) -> Ident {
        Ident::_new(string, true, span)
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}
pub(crate) fn is_ident_start(c: char) -> bool {
    ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z') || c == '_'
        || (c > '\x7f' && UnicodeXID::is_xid_start(c))
}
pub(crate) fn is_ident_continue(c: char) -> bool {
    ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z') || c == '_'
        || ('0' <= c && c <= '9') || (c > '\x7f' && UnicodeXID::is_xid_continue(c))
}
fn validate_ident(string: &str) {
    let validate = string;
    if validate.is_empty() {
        panic!("Ident is not allowed to be empty; use Option<Ident>");
    }
    if validate.bytes().all(|digit| digit >= b'0' && digit <= b'9') {
        panic!("Ident cannot be a number; use Literal instead");
    }
    fn ident_ok(string: &str) -> bool {
        let mut chars = string.chars();
        let first = chars.next().unwrap();
        if !is_ident_start(first) {
            return false;
        }
        for ch in chars {
            if !is_ident_continue(ch) {
                return false;
            }
        }
        true
    }
    if !ident_ok(validate) {
        panic!("{:?} is not a valid Ident", string);
    }
}
impl PartialEq for Ident {
    fn eq(&self, other: &Ident) -> bool {
        self.sym == other.sym && self.raw == other.raw
    }
}
impl<T> PartialEq<T> for Ident
where
    T: ?Sized + AsRef<str>,
{
    fn eq(&self, other: &T) -> bool {
        let other = other.as_ref();
        if self.raw {
            other.starts_with("r#") && self.sym == other[2..]
        } else {
            self.sym == other
        }
    }
}
impl Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.raw {
            f.write_str("r#")?;
        }
        Display::fmt(&self.sym, f)
    }
}
impl Debug for Ident {
    #[cfg(not(span_locations))]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = f.debug_tuple("Ident");
        debug.field(&format_args!("{}", self));
        debug.finish()
    }
    #[cfg(span_locations)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = f.debug_struct("Ident");
        debug.field("sym", &format_args!("{}", self));
        debug_span_field_if_nontrivial(&mut debug, self.span);
        debug.finish()
    }
}
#[derive(Clone)]
pub(crate) struct Literal {
    text: String,
    span: Span,
}
macro_rules! suffixed_numbers {
    ($($name:ident => $kind:ident,)*) => {
        $(pub fn $name (n : $kind) -> Literal { Literal::_new(format!(concat!("{}",
        stringify!($kind)), n)) })*
    };
}
macro_rules! unsuffixed_numbers {
    ($($name:ident => $kind:ident,)*) => {
        $(pub fn $name (n : $kind) -> Literal { Literal::_new(n.to_string()) })*
    };
}
impl Literal {
    pub(crate) fn _new(text: String) -> Literal {
        Literal {
            text,
            span: Span::call_site(),
        }
    }
    suffixed_numbers! {
        u8_suffixed => u8, u16_suffixed => u16, u32_suffixed => u32, u64_suffixed => u64,
        u128_suffixed => u128, usize_suffixed => usize, i8_suffixed => i8, i16_suffixed
        => i16, i32_suffixed => i32, i64_suffixed => i64, i128_suffixed => i128,
        isize_suffixed => isize, f32_suffixed => f32, f64_suffixed => f64,
    }
    unsuffixed_numbers! {
        u8_unsuffixed => u8, u16_unsuffixed => u16, u32_unsuffixed => u32, u64_unsuffixed
        => u64, u128_unsuffixed => u128, usize_unsuffixed => usize, i8_unsuffixed => i8,
        i16_unsuffixed => i16, i32_unsuffixed => i32, i64_unsuffixed => i64,
        i128_unsuffixed => i128, isize_unsuffixed => isize,
    }
    pub fn f32_unsuffixed(f: f32) -> Literal {
        let mut s = f.to_string();
        if !s.contains('.') {
            s.push_str(".0");
        }
        Literal::_new(s)
    }
    pub fn f64_unsuffixed(f: f64) -> Literal {
        let mut s = f.to_string();
        if !s.contains('.') {
            s.push_str(".0");
        }
        Literal::_new(s)
    }
    pub fn string(t: &str) -> Literal {
        let mut text = String::with_capacity(t.len() + 2);
        text.push('"');
        for c in t.chars() {
            if c == '\'' {
                text.push(c);
            } else {
                text.extend(c.escape_debug());
            }
        }
        text.push('"');
        Literal::_new(text)
    }
    pub fn character(t: char) -> Literal {
        let mut text = String::new();
        text.push('\'');
        if t == '"' {
            text.push(t);
        } else {
            text.extend(t.escape_debug());
        }
        text.push('\'');
        Literal::_new(text)
    }
    pub fn byte_string(bytes: &[u8]) -> Literal {
        let mut escaped = "b\"".to_string();
        for b in bytes {
            #[allow(clippy::match_overlapping_arm)]
            match *b {
                b'\0' => escaped.push_str(r"\0"),
                b'\t' => escaped.push_str(r"\t"),
                b'\n' => escaped.push_str(r"\n"),
                b'\r' => escaped.push_str(r"\r"),
                b'"' => escaped.push_str("\\\""),
                b'\\' => escaped.push_str("\\\\"),
                b'\x20'..=b'\x7E' => escaped.push(*b as char),
                _ => escaped.push_str(&format!("\\x{:02X}", b)),
            }
        }
        escaped.push('"');
        Literal::_new(escaped)
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
    pub fn subspan<R: RangeBounds<usize>>(&self, _range: R) -> Option<Span> {
        None
    }
}
impl Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.text, f)
    }
}
impl Debug for Literal {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = fmt.debug_struct("Literal");
        debug.field("lit", &format_args!("{}", self.text));
        debug_span_field_if_nontrivial(&mut debug, self.span);
        debug.finish()
    }
}
#[cfg(test)]
mod tests_llm_16_33 {
    use crate::fallback::{Ident, Span};
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_33_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = "proc_macro";
        let rug_fuzz_1 = "proc_macro";
        let rug_fuzz_2 = "r#union";
        let rug_fuzz_3 = "r#union";
        let rug_fuzz_4 = "proc_macro";
        let rug_fuzz_5 = "r#proc_macro";
        let rug_fuzz_6 = "proc_macro";
        let rug_fuzz_7 = "r#proc_macro";
        let ident1 = Ident::new(rug_fuzz_0, Span::call_site());
        let ident2 = Ident::new(rug_fuzz_1, Span::call_site());
        debug_assert_eq!(ident1.eq(& ident2), true);
        let ident3 = Ident::new_raw(rug_fuzz_2, Span::call_site());
        let ident4 = Ident::new_raw(rug_fuzz_3, Span::call_site());
        debug_assert_eq!(ident3.eq(& ident4), true);
        let ident5 = Ident::new(rug_fuzz_4, Span::call_site());
        let ident6 = Ident::new_raw(rug_fuzz_5, Span::call_site());
        debug_assert_eq!(ident5.eq(& ident6), true);
        let ident7 = Ident::new(rug_fuzz_6, Span::call_site());
        let ident8 = Ident::new(rug_fuzz_7, Span::call_site());
        debug_assert_eq!(ident7.eq(& ident8), false);
        let _rug_ed_tests_llm_16_33_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_42 {
    use super::*;
    use crate::*;
    use crate::{TokenStream, TokenTree};
    #[test]
    fn test_extend() {
        let _rug_st_tests_llm_16_42_rrrruuuugggg_test_extend = 0;
        let rug_fuzz_0 = "foo";
        let rug_fuzz_1 = "bar";
        let rug_fuzz_2 = "foo bar";
        let mut token_stream = TokenStream::new();
        let token1 = TokenStream::from_str(rug_fuzz_0).unwrap();
        let token2 = TokenStream::from_str(rug_fuzz_1).unwrap();
        token_stream.extend(vec![token1, token2]);
        let expected = TokenStream::from_str(rug_fuzz_2).unwrap();
        debug_assert_eq!(token_stream.to_string(), expected.to_string());
        let _rug_ed_tests_llm_16_42_rrrruuuugggg_test_extend = 0;
    }
    #[test]
    fn test_extend_empty() {
        let _rug_st_tests_llm_16_42_rrrruuuugggg_test_extend_empty = 0;
        let rug_fuzz_0 = "foo";
        let mut token_stream = TokenStream::from_str(rug_fuzz_0).unwrap();
        let empty: Vec<TokenTree> = Vec::new();
        token_stream.extend(empty);
        debug_assert_eq!(token_stream.to_string(), "foo".to_string());
        let _rug_ed_tests_llm_16_42_rrrruuuugggg_test_extend_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_228_llm_16_227 {
    use crate::Group;
    use crate::Delimiter;
    use crate::TokenStream;
    #[test]
    fn test_delimiter() {
        let _rug_st_tests_llm_16_228_llm_16_227_rrrruuuugggg_test_delimiter = 0;
        let delimiter = Delimiter::Parenthesis;
        let group = Group::new(delimiter, TokenStream::new());
        debug_assert_eq!(group.delimiter(), delimiter);
        let _rug_ed_tests_llm_16_228_llm_16_227_rrrruuuugggg_test_delimiter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_232 {
    use super::*;
    use crate::*;
    use crate::fallback::{Group, Span, TokenStream};
    #[test]
    fn test_set_span() {
        let _rug_st_tests_llm_16_232_rrrruuuugggg_test_set_span = 0;
        let mut group = Group::new(Delimiter::Parenthesis, TokenStream::new());
        let span = Span::call_site().join(Span::call_site()).unwrap();
        group.set_span(span);
        debug_assert_eq!(group.span(), span);
        let _rug_ed_tests_llm_16_232_rrrruuuugggg_test_set_span = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_234 {
    use super::*;
    use crate::*;
    use crate::Group;
    use crate::Span;
    use crate::TokenStream;
    use crate::Delimiter;
    #[test]
    fn test_span() {
        let _rug_st_tests_llm_16_234_rrrruuuugggg_test_span = 0;
        let delimiter = Delimiter::Parenthesis;
        let stream = TokenStream::new();
        let group = Group::new(delimiter, stream);
        let result = group.span();
        let _rug_ed_tests_llm_16_234_rrrruuuugggg_test_span = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_242 {
    use super::*;
    use crate::*;
    use crate::fallback::{Ident, Span};
    #[test]
    fn test__new() {
        let _rug_st_tests_llm_16_242_rrrruuuugggg_test__new = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = true;
        let string = rug_fuzz_0;
        let raw = rug_fuzz_1;
        let span = Span::call_site();
        let ident = Ident::_new(string, raw, span);
        debug_assert_eq!(ident.sym, "test".to_owned());
        debug_assert_eq!(ident.span, span);
        debug_assert_eq!(ident.raw, true);
        let _rug_ed_tests_llm_16_242_rrrruuuugggg_test__new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_244 {
    use super::*;
    use crate::*;
    use crate::fallback::{Ident, Span};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_244_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "test";
        let string = rug_fuzz_0;
        let span = Span::call_site();
        let ident = Ident::new(string, span);
        debug_assert_eq!(ident.sym, string);
        debug_assert_eq!(ident.raw, false);
        debug_assert_eq!(ident.span, span);
        let _rug_ed_tests_llm_16_244_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_248 {
    use super::*;
    use crate::*;
    use crate::fallback::{Ident, Span};
    #[test]
    fn test_set_span() {
        let _rug_st_tests_llm_16_248_rrrruuuugggg_test_set_span = 0;
        let rug_fuzz_0 = "test";
        let mut ident = Ident::new(rug_fuzz_0, Span::call_site());
        let new_span = Span::call_site();
        ident.set_span(new_span);
        debug_assert_eq!(ident.span(), new_span);
        let _rug_ed_tests_llm_16_248_rrrruuuugggg_test_set_span = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_256 {
    use super::*;
    use crate::*;
    use crate::fallback::*;
    #[test]
    fn test_character() {
        let _rug_st_tests_llm_16_256_rrrruuuugggg_test_character = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = '"';
        let rug_fuzz_2 = '\'';
        let t1: char = rug_fuzz_0;
        let t2: char = rug_fuzz_1;
        let t3: char = rug_fuzz_2;
        let literal1 = fallback::Literal::character(t1);
        let literal2 = fallback::Literal::character(t2);
        let literal3 = fallback::Literal::character(t3);
        debug_assert_eq!(literal1.text, "'a'");
        debug_assert_eq!(literal1.span, fallback::Span::call_site());
        debug_assert_eq!(literal2.text, "'\"'");
        debug_assert_eq!(literal2.span, fallback::Span::call_site());
        debug_assert_eq!(literal3.text, "'''");
        debug_assert_eq!(literal3.span, fallback::Span::call_site());
        let _rug_ed_tests_llm_16_256_rrrruuuugggg_test_character = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_263 {
    use crate::fallback::{Literal, Span};
    use std::fmt::{Debug, Display};
    #[test]
    fn test_f64_unsuffixed() {
        let _rug_st_tests_llm_16_263_rrrruuuugggg_test_f64_unsuffixed = 0;
        let rug_fuzz_0 = 3.14;
        let f = rug_fuzz_0;
        let literal = Literal::f64_unsuffixed(f);
        debug_assert_eq!(format!("{:?}", literal), "Literal { lit: \"3.14\" }");
        let _rug_ed_tests_llm_16_263_rrrruuuugggg_test_f64_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_265 {
    use crate::fallback::Literal;
    use std::fmt::{Debug, Formatter};
    #[test]
    fn test_i128_suffixed() {
        let _rug_st_tests_llm_16_265_rrrruuuugggg_test_i128_suffixed = 0;
        let rug_fuzz_0 = "100";
        let rug_fuzz_1 = 100;
        let literal = Literal::_new(rug_fuzz_0.to_string());
        let result = Literal::i128_suffixed(rug_fuzz_1);
        debug_assert_eq!(format!("{:?}", literal), format!("{:?}", result));
        debug_assert_eq!(literal.span, result.span);
        let _rug_ed_tests_llm_16_265_rrrruuuugggg_test_i128_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_269_llm_16_268 {
    use super::*;
    use crate::*;
    use crate::fallback::Span;
    #[test]
    fn test_i16_suffixed() {
        let _rug_st_tests_llm_16_269_llm_16_268_rrrruuuugggg_test_i16_suffixed = 0;
        let rug_fuzz_0 = 42;
        let result = fallback::Literal::i16_suffixed(rug_fuzz_0);
        debug_assert_eq!(result.text, "42");
        debug_assert_eq!(result.span, fallback::Span::call_site());
        let _rug_ed_tests_llm_16_269_llm_16_268_rrrruuuugggg_test_i16_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_273 {
    use super::*;
    use crate::*;
    use crate::fallback::Literal;
    #[test]
    fn test_i32_suffixed() {
        let _rug_st_tests_llm_16_273_rrrruuuugggg_test_i32_suffixed = 0;
        let rug_fuzz_0 = 42;
        let n: i32 = rug_fuzz_0;
        let literal = Literal::i32_suffixed(n);
        debug_assert_eq!(literal.to_string(), "42i32");
        let _rug_ed_tests_llm_16_273_rrrruuuugggg_test_i32_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_284 {
    use super::*;
    use crate::*;
    use crate::fallback::{Literal, Span};
    #[test]
    fn test_isize_suffixed() {
        let _rug_st_tests_llm_16_284_rrrruuuugggg_test_isize_suffixed = 0;
        let rug_fuzz_0 = 10;
        let literal = Literal::isize_suffixed(rug_fuzz_0);
        debug_assert_eq!(literal.text, "10isize");
        debug_assert_eq!(literal.span, Span::call_site());
        let _rug_ed_tests_llm_16_284_rrrruuuugggg_test_isize_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_288_llm_16_287 {
    use super::*;
    use crate::*;
    use crate::fallback::Literal;
    use crate::fallback::Span;
    #[test]
    fn test_set_span() {
        let _rug_st_tests_llm_16_288_llm_16_287_rrrruuuugggg_test_set_span = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 2;
        let mut literal = Literal::_new(rug_fuzz_0.to_string());
        let span = Span {
            #[cfg(span_locations)]
            lo: rug_fuzz_1,
            #[cfg(span_locations)]
            hi: rug_fuzz_2,
        };
        literal.set_span(span);
        let expected_span = Span {
            #[cfg(span_locations)]
            lo: rug_fuzz_3,
            #[cfg(span_locations)]
            hi: rug_fuzz_4,
        };
        debug_assert_eq!(literal.span, expected_span);
        let _rug_ed_tests_llm_16_288_llm_16_287_rrrruuuugggg_test_set_span = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_294 {
    use super::*;
    use crate::*;
    use fallback::{Span, Literal};
    use std::ops::RangeBounds;
    #[test]
    fn test_subspan() {
        let _rug_st_tests_llm_16_294_rrrruuuugggg_test_subspan = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 4;
        let literal = Literal {
            text: String::from(rug_fuzz_0),
            span: Span {
                #[cfg(span_locations)]
                lo: rug_fuzz_1,
                #[cfg(span_locations)]
                hi: rug_fuzz_2,
            },
        };
        let range: std::ops::Range<usize> = rug_fuzz_3..rug_fuzz_4;
        let subspan = literal.subspan(range);
        debug_assert_eq!(subspan, None);
        let _rug_ed_tests_llm_16_294_rrrruuuugggg_test_subspan = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_295 {
    use crate::fallback::Literal;
    #[test]
    fn test_u128_suffixed() {
        let _rug_st_tests_llm_16_295_rrrruuuugggg_test_u128_suffixed = 0;
        let rug_fuzz_0 = 42;
        let n: u128 = rug_fuzz_0;
        let literal = Literal::u128_suffixed(n);
        debug_assert_eq!(literal.to_string(), "42u128");
        let _rug_ed_tests_llm_16_295_rrrruuuugggg_test_u128_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_303_llm_16_302 {
    use crate::fallback::{Literal, Span};
    #[test]
    fn test_u32_suffixed() {
        let _rug_st_tests_llm_16_303_llm_16_302_rrrruuuugggg_test_u32_suffixed = 0;
        let rug_fuzz_0 = 42;
        let n: u32 = rug_fuzz_0;
        let result = Literal::u32_suffixed(n);
        let expected = Literal {
            text: format!("{}u32", n),
            span: Span::call_site(),
        };
        debug_assert_eq!(result.text, expected.text);
        debug_assert_eq!(result.span, expected.span);
        let _rug_ed_tests_llm_16_303_llm_16_302_rrrruuuugggg_test_u32_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_305 {
    use crate::fallback::{Literal, Span};
    #[test]
    fn test_u32_unsuffixed() {
        let _rug_st_tests_llm_16_305_rrrruuuugggg_test_u32_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let n: u32 = rug_fuzz_0;
        let result = Literal::u32_unsuffixed(n);
        let expected_text = n.to_string();
        let expected_span = Span::call_site();
        debug_assert_eq!(result.text, expected_text);
        debug_assert_eq!(result.span, expected_span);
        let _rug_ed_tests_llm_16_305_rrrruuuugggg_test_u32_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_306 {
    use crate::fallback::Literal;
    use crate::fallback::Span;
    #[test]
    fn test_u64_suffixed() {
        let _rug_st_tests_llm_16_306_rrrruuuugggg_test_u64_suffixed = 0;
        let rug_fuzz_0 = 10;
        let n: u64 = rug_fuzz_0;
        let literal = Literal::u64_suffixed(n);
        debug_assert_eq!(literal.text, "10u64");
        debug_assert_eq!(literal.span, Span::call_site());
        let _rug_ed_tests_llm_16_306_rrrruuuugggg_test_u64_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_307 {
    use crate::fallback::Literal;
    #[test]
    fn test_u64_unsuffixed() {
        let _rug_st_tests_llm_16_307_rrrruuuugggg_test_u64_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let n: u64 = rug_fuzz_0;
        let result = Literal::u64_unsuffixed(n);
        debug_assert_eq!(format!("{:?}", result), "Literal { lit: \"42\" }");
        debug_assert_eq!(format!("{}", result), "42");
        let _rug_ed_tests_llm_16_307_rrrruuuugggg_test_u64_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_308 {
    use crate::fallback::Literal;
    #[test]
    fn test_u8_suffixed() {
        let _rug_st_tests_llm_16_308_rrrruuuugggg_test_u8_suffixed = 0;
        let rug_fuzz_0 = 5;
        let literal = Literal::u8_suffixed(rug_fuzz_0);
        debug_assert_eq!(literal.to_string(), "5u8");
        let _rug_ed_tests_llm_16_308_rrrruuuugggg_test_u8_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_311 {
    use super::*;
    use crate::*;
    #[test]
    fn test_usize_suffixed() {
        let _rug_st_tests_llm_16_311_rrrruuuugggg_test_usize_suffixed = 0;
        let rug_fuzz_0 = 42;
        let n = rug_fuzz_0;
        let result = fallback::Literal::usize_suffixed(n);
        debug_assert_eq!(result.text, "42usize");
        debug_assert_eq!(result.span, fallback::Span::call_site());
        let _rug_ed_tests_llm_16_311_rrrruuuugggg_test_usize_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_316 {
    use super::*;
    use crate::*;
    use std::path::PathBuf;
    #[test]
    fn test_path() {
        let _rug_st_tests_llm_16_316_rrrruuuugggg_test_path = 0;
        let rug_fuzz_0 = "/path/to/source_file.rs";
        let source_file = SourceFile {
            path: PathBuf::from(rug_fuzz_0),
        };
        debug_assert_eq!(source_file.path(), PathBuf::from("/path/to/source_file.rs"));
        let _rug_ed_tests_llm_16_316_rrrruuuugggg_test_path = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_317 {
    use crate::fallback::Span;
    #[test]
    fn test_call_site() {
        let _rug_st_tests_llm_16_317_rrrruuuugggg_test_call_site = 0;
        let result = Span::call_site();
        let _rug_ed_tests_llm_16_317_rrrruuuugggg_test_call_site = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_319 {
    use crate::fallback::Span;
    #[test]
    fn test_first_byte_not_span_locations() {
        let _rug_st_tests_llm_16_319_rrrruuuugggg_test_first_byte_not_span_locations = 0;
        let span = Span::call_site();
        let result = span.first_byte();
        debug_assert_eq!(result, span);
        let _rug_ed_tests_llm_16_319_rrrruuuugggg_test_first_byte_not_span_locations = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_321 {
    use crate::fallback::Span;
    #[test]
    fn test_join_not_span_locations() {
        let _rug_st_tests_llm_16_321_rrrruuuugggg_test_join_not_span_locations = 0;
        let span = Span {};
        let other = Span {};
        let result = span.join(other);
        debug_assert_eq!(result, Some(Span {}));
        let _rug_ed_tests_llm_16_321_rrrruuuugggg_test_join_not_span_locations = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_323 {
    use super::*;
    use crate::*;
    use crate::fallback::Span;
    #[test]
    fn test_last_byte() {
        let _rug_st_tests_llm_16_323_rrrruuuugggg_test_last_byte = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 10;
        #[cfg(span_locations)]
        {
            let span = Span {
                lo: rug_fuzz_0,
                hi: rug_fuzz_1,
            };
            debug_assert_eq!(span.last_byte().lo, 9);
            debug_assert_eq!(span.last_byte().hi, 10);
        }
        #[cfg(not(span_locations))]
        {
            let span = Span {};
            debug_assert_eq!(span.last_byte(), span);
        }
        let _rug_ed_tests_llm_16_323_rrrruuuugggg_test_last_byte = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_325 {
    use super::*;
    use crate::*;
    use crate::fallback::Span;
    #[test]
    fn test_located_at() {
        let _rug_st_tests_llm_16_325_rrrruuuugggg_test_located_at = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let span1 = Span {
            #[cfg(span_locations)]
            lo: rug_fuzz_0,
            #[cfg(span_locations)]
            hi: rug_fuzz_1,
        };
        let span2 = Span {
            #[cfg(span_locations)]
            lo: rug_fuzz_2,
            #[cfg(span_locations)]
            hi: rug_fuzz_3,
        };
        let result = span1.located_at(span2);
        debug_assert_eq!(result, span2);
        let _rug_ed_tests_llm_16_325_rrrruuuugggg_test_located_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_327 {
    use super::*;
    use crate::*;
    use fallback::Span;
    #[test]
    fn test_mixed_site() {
        let _rug_st_tests_llm_16_327_rrrruuuugggg_test_mixed_site = 0;
        let span = Span::mixed_site();
        let _rug_ed_tests_llm_16_327_rrrruuuugggg_test_mixed_site = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_333_llm_16_332 {
    use crate::TokenStream;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_333_llm_16_332_rrrruuuugggg_test_new = 0;
        let token_stream = TokenStream::new();
        debug_assert!(token_stream.inner.is_empty());
        let _rug_ed_tests_llm_16_333_llm_16_332_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_341 {
    use crate::fallback::force;
    #[test]
    fn test_force() {
        let _rug_st_tests_llm_16_341_rrrruuuugggg_test_force = 0;
        force();
        let _rug_ed_tests_llm_16_341_rrrruuuugggg_test_force = 0;
    }
}
#[cfg(test)]
mod test {
    use unicode_xid::UnicodeXID;
    use crate::fallback::is_ident_start;
    #[test]
    fn test_is_ident_start() {
        assert_eq!(is_ident_start('a'), true);
        assert_eq!(is_ident_start('z'), true);
        assert_eq!(is_ident_start('A'), true);
        assert_eq!(is_ident_start('Z'), true);
        assert_eq!(is_ident_start('_'), true);
        assert_eq!(is_ident_start('\u{80}'), false);
        assert_eq!(is_ident_start('\u{200}'), true);
    }
}
#[cfg(test)]
mod tests_llm_16_348 {
    use super::*;
    use crate::*;
    #[test]
    fn test_unforce() {
        let _rug_st_tests_llm_16_348_rrrruuuugggg_test_unforce = 0;
        unforce();
        let _rug_ed_tests_llm_16_348_rrrruuuugggg_test_unforce = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_350 {
    use super::*;
    use crate::*;
    use proc_macro::Literal;
    #[test]
    #[should_panic]
    fn test_validate_ident_empty_string() {
        let _rug_st_tests_llm_16_350_rrrruuuugggg_test_validate_ident_empty_string = 0;
        let rug_fuzz_0 = "";
        validate_ident(rug_fuzz_0);
        let _rug_ed_tests_llm_16_350_rrrruuuugggg_test_validate_ident_empty_string = 0;
    }
    #[test]
    #[should_panic]
    fn test_validate_ident_numbers() {
        let _rug_st_tests_llm_16_350_rrrruuuugggg_test_validate_ident_numbers = 0;
        let rug_fuzz_0 = "123";
        validate_ident(rug_fuzz_0);
        let _rug_ed_tests_llm_16_350_rrrruuuugggg_test_validate_ident_numbers = 0;
    }
    #[test]
    #[should_panic]
    fn test_validate_ident_invalid_ident() {
        let _rug_st_tests_llm_16_350_rrrruuuugggg_test_validate_ident_invalid_ident = 0;
        let rug_fuzz_0 = "123ab";
        validate_ident(rug_fuzz_0);
        let _rug_ed_tests_llm_16_350_rrrruuuugggg_test_validate_ident_invalid_ident = 0;
    }
    #[test]
    fn test_validate_ident_valid_ident() {
        let _rug_st_tests_llm_16_350_rrrruuuugggg_test_validate_ident_valid_ident = 0;
        let rug_fuzz_0 = "valid_ident";
        let rug_fuzz_1 = "a123";
        let rug_fuzz_2 = "_validIdent";
        validate_ident(rug_fuzz_0);
        validate_ident(rug_fuzz_1);
        validate_ident(rug_fuzz_2);
        let _rug_ed_tests_llm_16_350_rrrruuuugggg_test_validate_ident_valid_ident = 0;
    }
    #[test]
    fn test_validate_ident_valid_literal() {
        let _rug_st_tests_llm_16_350_rrrruuuugggg_test_validate_ident_valid_literal = 0;
        let rug_fuzz_0 = "123ABC";
        validate_ident(rug_fuzz_0);
        let _rug_ed_tests_llm_16_350_rrrruuuugggg_test_validate_ident_valid_literal = 0;
    }
}
#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::fallback::Cursor;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_37_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_str";
        let p0: &str = rug_fuzz_0;
        get_cursor(p0);
        let _rug_ed_tests_rug_37_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_39_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0: char = rug_fuzz_0;
        crate::fallback::is_ident_continue(p0);
        let _rug_ed_tests_rug_39_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::fallback::{TokenStream, TokenTree};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_41_rrrruuuugggg_test_rug = 0;
        let mut v5 = TokenStream::new();
        TokenStream::is_empty(&v5);
        let _rug_ed_tests_rug_41_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use crate::fallback;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_42_rrrruuuugggg_test_rug = 0;
        let mut p0 = fallback::TokenStream::new();
        <fallback::TokenStream>::take_inner(&mut p0);
        let _rug_ed_tests_rug_42_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_43 {
    use super::*;
    use crate::fallback;
    use crate::{Literal, TokenTree, Spacing};
    #[test]
    fn test_push_token() {
        let _rug_st_tests_rug_43_rrrruuuugggg_test_push_token = 0;
        let rug_fuzz_0 = "-Hello, World!";
        let mut p0 = fallback::TokenStream::new();
        let mut p1 = TokenTree::Literal(Literal::string(rug_fuzz_0));
        <fallback::TokenStream>::push_token(&mut p0, p1);
        let _rug_ed_tests_rug_43_rrrruuuugggg_test_push_token = 0;
    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use crate::fallback::TokenStream;
    #[test]
    fn test_extend() {
        let _rug_st_tests_rug_51_rrrruuuugggg_test_extend = 0;
        let mut p0 = TokenStream::new();
        let p1: TokenStream = TokenStream::new();
        p0.extend(p1);
        let _rug_ed_tests_rug_51_rrrruuuugggg_test_extend = 0;
    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::fallback::SourceFile;
    use std::path::PathBuf;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "path/to/source/file.rs";
        let mut p0 = SourceFile {
            path: PathBuf::from(rug_fuzz_0),
        };
        debug_assert_eq!(p0.is_real(), false);
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use crate::fallback::Span;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_54_rrrruuuugggg_test_rug = 0;
        let mut p0 = Span::call_site();
        let mut p1 = Span::call_site();
        p0.resolved_at(p1);
        let _rug_ed_tests_rug_54_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    use crate::Span;
    use crate::TokenStream;
    use crate::Delimiter;
    use crate::fallback;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_55_rrrruuuugggg_test_rug = 0;
        let mut p0 = Delimiter::Parenthesis;
        let mut p1 = fallback::TokenStream::new();
        crate::fallback::Group::new(p0, p1);
        let _rug_ed_tests_rug_55_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::{Delimiter, Group, Span, TokenStream};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_rug = 0;
        let mut p0 = Group::new(Delimiter::Parenthesis, TokenStream::new());
        p0.stream();
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::{Group, Delimiter, Span, TokenStream};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_57_rrrruuuugggg_test_rug = 0;
        let mut v15 = Group::new(Delimiter::Parenthesis, TokenStream::new());
        v15.span_open();
        let _rug_ed_tests_rug_57_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::{Delimiter, Group, Span, TokenStream};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_58_rrrruuuugggg_test_rug = 0;
        let mut p0 = Group::new(Delimiter::Parenthesis, TokenStream::new());
        p0.span_close();
        let _rug_ed_tests_rug_58_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::fallback::Span;
    #[test]
    fn test_new_raw() {
        let _rug_st_tests_rug_59_rrrruuuugggg_test_new_raw = 0;
        let rug_fuzz_0 = "some_string";
        let p0: &'static str = rug_fuzz_0;
        let p1 = Span::call_site();
        crate::fallback::Ident::new_raw(p0, p1);
        let _rug_ed_tests_rug_59_rrrruuuugggg_test_new_raw = 0;
    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::fallback::Ident;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_60_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "proc_macro";
        let rug_fuzz_1 = false;
        let mut p0 = Ident::_new(rug_fuzz_0, rug_fuzz_1, Span::call_site());
        Ident::span(&p0);
        let _rug_ed_tests_rug_60_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_62 {
    use super::*;
    use crate::Span;
    use crate::fallback::Literal;
    use std::string::String;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_62_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample text";
        let mut p0: String = rug_fuzz_0.to_string();
        Literal::_new(p0);
        let _rug_ed_tests_rug_62_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_u16_suffixed() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_u16_suffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        Literal::u16_suffixed(p0);
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_u16_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_64_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        Literal::i8_suffixed(p0);
        let _rug_ed_tests_rug_64_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_65 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_65_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        Literal::i64_suffixed(p0);
        let _rug_ed_tests_rug_65_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_66 {
    use super::*;
    use crate::fallback::Literal;
    #[test]
    fn test_f32_suffixed() {
        let _rug_st_tests_rug_66_rrrruuuugggg_test_f32_suffixed = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        Literal::f32_suffixed(p0);
        let _rug_ed_tests_rug_66_rrrruuuugggg_test_f32_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        Literal::f64_suffixed(p0);
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::fallback::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_68_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        Literal::u8_unsuffixed(p0);
        let _rug_ed_tests_rug_68_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_69 {
    use super::*;
    use crate::fallback::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_69_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        Literal::u16_unsuffixed(p0);
        let _rug_ed_tests_rug_69_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_u128_unsuffixed() {
        let _rug_st_tests_rug_70_rrrruuuugggg_test_u128_unsuffixed = 0;
        let rug_fuzz_0 = 123456789;
        let p0: u128 = rug_fuzz_0;
        Literal::u128_unsuffixed(p0);
        let _rug_ed_tests_rug_70_rrrruuuugggg_test_u128_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_71_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: usize = rug_fuzz_0;
        Literal::usize_unsuffixed(p0);
        let _rug_ed_tests_rug_71_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use crate::fallback::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_72_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        Literal::i8_unsuffixed(p0);
        let _rug_ed_tests_rug_72_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_i16_unsuffixed() {
        let _rug_st_tests_rug_73_rrrruuuugggg_test_i16_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        Literal::i16_unsuffixed(p0);
        let _rug_ed_tests_rug_73_rrrruuuugggg_test_i16_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i64 = rug_fuzz_0;
        Literal::i64_unsuffixed(p0);
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_i128_unsuffixed() {
        let _rug_st_tests_rug_75_rrrruuuugggg_test_i128_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: i128 = rug_fuzz_0;
        let result = Literal::i128_unsuffixed(p0);
        let _rug_ed_tests_rug_75_rrrruuuugggg_test_i128_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_76 {
    use super::*;
    use crate::fallback::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_76_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        Literal::isize_unsuffixed(p0);
        let _rug_ed_tests_rug_76_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_77 {
    use super::*;
    use crate::fallback::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_77_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1.23;
        let p0: f32 = rug_fuzz_0;
        Literal::f32_unsuffixed(p0);
        let _rug_ed_tests_rug_77_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_string() {
        let _rug_st_tests_rug_78_rrrruuuugggg_test_string = 0;
        let rug_fuzz_0 = "Hello, World!";
        let p0: &str = rug_fuzz_0;
        Literal::string(&p0);
        let _rug_ed_tests_rug_78_rrrruuuugggg_test_string = 0;
    }
}
#[cfg(test)]
mod tests_rug_79 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_byte_string() {
        let _rug_st_tests_rug_79_rrrruuuugggg_test_byte_string = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let p0: &[u8] = rug_fuzz_0;
        Literal::byte_string(p0);
        let _rug_ed_tests_rug_79_rrrruuuugggg_test_byte_string = 0;
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::Span;
    use crate::fallback::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0 = Literal::_new(rug_fuzz_0.to_string());
        Literal::span(&p0);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_rug = 0;
    }
}
