use crate::detection::inside_proc_macro;
use crate::{fallback, Delimiter, Punct, Spacing, TokenTree};
use std::fmt::{self, Debug, Display};
use std::iter::FromIterator;
use std::ops::RangeBounds;
use std::panic;
#[cfg(super_unstable)]
use std::path::PathBuf;
use std::str::FromStr;
#[derive(Clone)]
pub(crate) enum TokenStream {
    Compiler(DeferredTokenStream),
    Fallback(fallback::TokenStream),
}
#[derive(Clone)]
pub(crate) struct DeferredTokenStream {
    stream: proc_macro::TokenStream,
    extra: Vec<proc_macro::TokenTree>,
}
pub(crate) enum LexError {
    Compiler(proc_macro::LexError),
    Fallback(fallback::LexError),
}
fn mismatch() -> ! {
    panic!("stable/nightly mismatch")
}
impl DeferredTokenStream {
    fn new(stream: proc_macro::TokenStream) -> Self {
        DeferredTokenStream {
            stream,
            extra: Vec::new(),
        }
    }
    fn is_empty(&self) -> bool {
        self.stream.is_empty() && self.extra.is_empty()
    }
    fn evaluate_now(&mut self) {
        if !self.extra.is_empty() {
            self.stream.extend(self.extra.drain(..));
        }
    }
    fn into_token_stream(mut self) -> proc_macro::TokenStream {
        self.evaluate_now();
        self.stream
    }
}
impl TokenStream {
    pub fn new() -> TokenStream {
        if inside_proc_macro() {
            TokenStream::Compiler(
                DeferredTokenStream::new(proc_macro::TokenStream::new()),
            )
        } else {
            TokenStream::Fallback(fallback::TokenStream::new())
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            TokenStream::Compiler(tts) => tts.is_empty(),
            TokenStream::Fallback(tts) => tts.is_empty(),
        }
    }
    fn unwrap_nightly(self) -> proc_macro::TokenStream {
        match self {
            TokenStream::Compiler(s) => s.into_token_stream(),
            TokenStream::Fallback(_) => mismatch(),
        }
    }
    fn unwrap_stable(self) -> fallback::TokenStream {
        match self {
            TokenStream::Compiler(_) => mismatch(),
            TokenStream::Fallback(s) => s,
        }
    }
}
impl FromStr for TokenStream {
    type Err = LexError;
    fn from_str(src: &str) -> Result<TokenStream, LexError> {
        if inside_proc_macro() {
            Ok(TokenStream::Compiler(DeferredTokenStream::new(proc_macro_parse(src)?)))
        } else {
            Ok(TokenStream::Fallback(src.parse()?))
        }
    }
}
fn proc_macro_parse(src: &str) -> Result<proc_macro::TokenStream, LexError> {
    panic::catch_unwind(|| src.parse().map_err(LexError::Compiler))
        .unwrap_or(Err(LexError::Fallback(fallback::LexError)))
}
impl Display for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenStream::Compiler(tts) => {
                Display::fmt(&tts.clone().into_token_stream(), f)
            }
            TokenStream::Fallback(tts) => Display::fmt(tts, f),
        }
    }
}
impl From<proc_macro::TokenStream> for TokenStream {
    fn from(inner: proc_macro::TokenStream) -> TokenStream {
        TokenStream::Compiler(DeferredTokenStream::new(inner))
    }
}
impl From<TokenStream> for proc_macro::TokenStream {
    fn from(inner: TokenStream) -> proc_macro::TokenStream {
        match inner {
            TokenStream::Compiler(inner) => inner.into_token_stream(),
            TokenStream::Fallback(inner) => inner.to_string().parse().unwrap(),
        }
    }
}
impl From<fallback::TokenStream> for TokenStream {
    fn from(inner: fallback::TokenStream) -> TokenStream {
        TokenStream::Fallback(inner)
    }
}
fn into_compiler_token(token: TokenTree) -> proc_macro::TokenTree {
    match token {
        TokenTree::Group(tt) => tt.inner.unwrap_nightly().into(),
        TokenTree::Punct(tt) => {
            let spacing = match tt.spacing() {
                Spacing::Joint => proc_macro::Spacing::Joint,
                Spacing::Alone => proc_macro::Spacing::Alone,
            };
            let mut punct = proc_macro::Punct::new(tt.as_char(), spacing);
            punct.set_span(tt.span().inner.unwrap_nightly());
            punct.into()
        }
        TokenTree::Ident(tt) => tt.inner.unwrap_nightly().into(),
        TokenTree::Literal(tt) => tt.inner.unwrap_nightly().into(),
    }
}
impl From<TokenTree> for TokenStream {
    fn from(token: TokenTree) -> TokenStream {
        if inside_proc_macro() {
            TokenStream::Compiler(
                DeferredTokenStream::new(into_compiler_token(token).into()),
            )
        } else {
            TokenStream::Fallback(token.into())
        }
    }
}
impl FromIterator<TokenTree> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenTree>>(trees: I) -> Self {
        if inside_proc_macro() {
            TokenStream::Compiler(
                DeferredTokenStream::new(
                    trees.into_iter().map(into_compiler_token).collect(),
                ),
            )
        } else {
            TokenStream::Fallback(trees.into_iter().collect())
        }
    }
}
impl FromIterator<TokenStream> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenStream>>(streams: I) -> Self {
        let mut streams = streams.into_iter();
        match streams.next() {
            Some(TokenStream::Compiler(mut first)) => {
                first.evaluate_now();
                first
                    .stream
                    .extend(
                        streams
                            .map(|s| match s {
                                TokenStream::Compiler(s) => s.into_token_stream(),
                                TokenStream::Fallback(_) => mismatch(),
                            }),
                    );
                TokenStream::Compiler(first)
            }
            Some(TokenStream::Fallback(mut first)) => {
                first
                    .extend(
                        streams
                            .map(|s| match s {
                                TokenStream::Fallback(s) => s,
                                TokenStream::Compiler(_) => mismatch(),
                            }),
                    );
                TokenStream::Fallback(first)
            }
            None => TokenStream::new(),
        }
    }
}
impl Extend<TokenTree> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenTree>>(&mut self, stream: I) {
        match self {
            TokenStream::Compiler(tts) => {
                for token in stream {
                    tts.extra.push(into_compiler_token(token));
                }
            }
            TokenStream::Fallback(tts) => tts.extend(stream),
        }
    }
}
impl Extend<TokenStream> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenStream>>(&mut self, streams: I) {
        match self {
            TokenStream::Compiler(tts) => {
                tts.evaluate_now();
                tts.stream.extend(streams.into_iter().map(TokenStream::unwrap_nightly));
            }
            TokenStream::Fallback(tts) => {
                tts.extend(streams.into_iter().map(TokenStream::unwrap_stable));
            }
        }
    }
}
impl Debug for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenStream::Compiler(tts) => Debug::fmt(&tts.clone().into_token_stream(), f),
            TokenStream::Fallback(tts) => Debug::fmt(tts, f),
        }
    }
}
impl From<proc_macro::LexError> for LexError {
    fn from(e: proc_macro::LexError) -> LexError {
        LexError::Compiler(e)
    }
}
impl From<fallback::LexError> for LexError {
    fn from(e: fallback::LexError) -> LexError {
        LexError::Fallback(e)
    }
}
impl Debug for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexError::Compiler(e) => Debug::fmt(e, f),
            LexError::Fallback(e) => Debug::fmt(e, f),
        }
    }
}
impl Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(lexerror_display)]
            LexError::Compiler(e) => Display::fmt(e, f),
            #[cfg(not(lexerror_display))]
            LexError::Compiler(_e) => Display::fmt(&fallback::LexError, f),
            LexError::Fallback(e) => Display::fmt(e, f),
        }
    }
}
#[derive(Clone)]
pub(crate) enum TokenTreeIter {
    Compiler(proc_macro::token_stream::IntoIter),
    Fallback(fallback::TokenTreeIter),
}
impl IntoIterator for TokenStream {
    type Item = TokenTree;
    type IntoIter = TokenTreeIter;
    fn into_iter(self) -> TokenTreeIter {
        match self {
            TokenStream::Compiler(tts) => {
                TokenTreeIter::Compiler(tts.into_token_stream().into_iter())
            }
            TokenStream::Fallback(tts) => TokenTreeIter::Fallback(tts.into_iter()),
        }
    }
}
impl Iterator for TokenTreeIter {
    type Item = TokenTree;
    fn next(&mut self) -> Option<TokenTree> {
        let token = match self {
            TokenTreeIter::Compiler(iter) => iter.next()?,
            TokenTreeIter::Fallback(iter) => return iter.next(),
        };
        Some(
            match token {
                proc_macro::TokenTree::Group(tt) => {
                    crate::Group::_new(Group::Compiler(tt)).into()
                }
                proc_macro::TokenTree::Punct(tt) => {
                    let spacing = match tt.spacing() {
                        proc_macro::Spacing::Joint => Spacing::Joint,
                        proc_macro::Spacing::Alone => Spacing::Alone,
                    };
                    let mut o = Punct::new(tt.as_char(), spacing);
                    o.set_span(crate::Span::_new(Span::Compiler(tt.span())));
                    o.into()
                }
                proc_macro::TokenTree::Ident(s) => {
                    crate::Ident::_new(Ident::Compiler(s)).into()
                }
                proc_macro::TokenTree::Literal(l) => {
                    crate::Literal::_new(Literal::Compiler(l)).into()
                }
            },
        )
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            TokenTreeIter::Compiler(tts) => tts.size_hint(),
            TokenTreeIter::Fallback(tts) => tts.size_hint(),
        }
    }
}
impl Debug for TokenTreeIter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TokenTreeIter").finish()
    }
}
#[derive(Clone, PartialEq, Eq)]
#[cfg(super_unstable)]
pub(crate) enum SourceFile {
    Compiler(proc_macro::SourceFile),
    Fallback(fallback::SourceFile),
}
#[cfg(super_unstable)]
impl SourceFile {
    fn nightly(sf: proc_macro::SourceFile) -> Self {
        SourceFile::Compiler(sf)
    }
    /// Get the path to this source file as a string.
    pub fn path(&self) -> PathBuf {
        match self {
            SourceFile::Compiler(a) => a.path(),
            SourceFile::Fallback(a) => a.path(),
        }
    }
    pub fn is_real(&self) -> bool {
        match self {
            SourceFile::Compiler(a) => a.is_real(),
            SourceFile::Fallback(a) => a.is_real(),
        }
    }
}
#[cfg(super_unstable)]
impl Debug for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SourceFile::Compiler(a) => Debug::fmt(a, f),
            SourceFile::Fallback(a) => Debug::fmt(a, f),
        }
    }
}
#[cfg(any(super_unstable, feature = "span-locations"))]
pub(crate) struct LineColumn {
    pub line: usize,
    pub column: usize,
}
#[derive(Copy, Clone)]
pub(crate) enum Span {
    Compiler(proc_macro::Span),
    Fallback(fallback::Span),
}
impl Span {
    pub fn call_site() -> Span {
        if inside_proc_macro() {
            Span::Compiler(proc_macro::Span::call_site())
        } else {
            Span::Fallback(fallback::Span::call_site())
        }
    }
    #[cfg(hygiene)]
    pub fn mixed_site() -> Span {
        if inside_proc_macro() {
            Span::Compiler(proc_macro::Span::mixed_site())
        } else {
            Span::Fallback(fallback::Span::mixed_site())
        }
    }
    #[cfg(super_unstable)]
    pub fn def_site() -> Span {
        if inside_proc_macro() {
            Span::Compiler(proc_macro::Span::def_site())
        } else {
            Span::Fallback(fallback::Span::def_site())
        }
    }
    pub fn resolved_at(&self, other: Span) -> Span {
        match (self, other) {
            #[cfg(hygiene)]
            (Span::Compiler(a), Span::Compiler(b)) => Span::Compiler(a.resolved_at(b)),
            #[cfg(not(hygiene))]
            (Span::Compiler(_), Span::Compiler(_)) => other,
            (Span::Fallback(a), Span::Fallback(b)) => Span::Fallback(a.resolved_at(b)),
            _ => mismatch(),
        }
    }
    pub fn located_at(&self, other: Span) -> Span {
        match (self, other) {
            #[cfg(hygiene)]
            (Span::Compiler(a), Span::Compiler(b)) => Span::Compiler(a.located_at(b)),
            #[cfg(not(hygiene))]
            (Span::Compiler(_), Span::Compiler(_)) => *self,
            (Span::Fallback(a), Span::Fallback(b)) => Span::Fallback(a.located_at(b)),
            _ => mismatch(),
        }
    }
    pub fn unwrap(self) -> proc_macro::Span {
        match self {
            Span::Compiler(s) => s,
            Span::Fallback(_) => {
                panic!("proc_macro::Span is only available in procedural macros")
            }
        }
    }
    #[cfg(super_unstable)]
    pub fn source_file(&self) -> SourceFile {
        match self {
            Span::Compiler(s) => SourceFile::nightly(s.source_file()),
            Span::Fallback(s) => SourceFile::Fallback(s.source_file()),
        }
    }
    #[cfg(any(super_unstable, feature = "span-locations"))]
    pub fn start(&self) -> LineColumn {
        match self {
            #[cfg(proc_macro_span)]
            Span::Compiler(s) => {
                let proc_macro::LineColumn { line, column } = s.start();
                LineColumn { line, column }
            }
            #[cfg(not(proc_macro_span))]
            Span::Compiler(_) => LineColumn { line: 0, column: 0 },
            Span::Fallback(s) => {
                let fallback::LineColumn { line, column } = s.start();
                LineColumn { line, column }
            }
        }
    }
    #[cfg(any(super_unstable, feature = "span-locations"))]
    pub fn end(&self) -> LineColumn {
        match self {
            #[cfg(proc_macro_span)]
            Span::Compiler(s) => {
                let proc_macro::LineColumn { line, column } = s.end();
                LineColumn { line, column }
            }
            #[cfg(not(proc_macro_span))]
            Span::Compiler(_) => LineColumn { line: 0, column: 0 },
            Span::Fallback(s) => {
                let fallback::LineColumn { line, column } = s.end();
                LineColumn { line, column }
            }
        }
    }
    pub fn join(&self, other: Span) -> Option<Span> {
        let ret = match (self, other) {
            #[cfg(proc_macro_span)]
            (Span::Compiler(a), Span::Compiler(b)) => Span::Compiler(a.join(b)?),
            (Span::Fallback(a), Span::Fallback(b)) => Span::Fallback(a.join(b)?),
            _ => return None,
        };
        Some(ret)
    }
    #[cfg(super_unstable)]
    pub fn eq(&self, other: &Span) -> bool {
        match (self, other) {
            (Span::Compiler(a), Span::Compiler(b)) => a.eq(b),
            (Span::Fallback(a), Span::Fallback(b)) => a.eq(b),
            _ => false,
        }
    }
    fn unwrap_nightly(self) -> proc_macro::Span {
        match self {
            Span::Compiler(s) => s,
            Span::Fallback(_) => mismatch(),
        }
    }
}
impl From<proc_macro::Span> for crate::Span {
    fn from(proc_span: proc_macro::Span) -> crate::Span {
        crate::Span::_new(Span::Compiler(proc_span))
    }
}
impl From<fallback::Span> for Span {
    fn from(inner: fallback::Span) -> Span {
        Span::Fallback(inner)
    }
}
impl Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Span::Compiler(s) => Debug::fmt(s, f),
            Span::Fallback(s) => Debug::fmt(s, f),
        }
    }
}
pub(crate) fn debug_span_field_if_nontrivial(debug: &mut fmt::DebugStruct, span: Span) {
    match span {
        Span::Compiler(s) => {
            debug.field("span", &s);
        }
        Span::Fallback(s) => fallback::debug_span_field_if_nontrivial(debug, s),
    }
}
#[derive(Clone)]
pub(crate) enum Group {
    Compiler(proc_macro::Group),
    Fallback(fallback::Group),
}
impl Group {
    pub fn new(delimiter: Delimiter, stream: TokenStream) -> Group {
        match stream {
            TokenStream::Compiler(tts) => {
                let delimiter = match delimiter {
                    Delimiter::Parenthesis => proc_macro::Delimiter::Parenthesis,
                    Delimiter::Bracket => proc_macro::Delimiter::Bracket,
                    Delimiter::Brace => proc_macro::Delimiter::Brace,
                    Delimiter::None => proc_macro::Delimiter::None,
                };
                Group::Compiler(
                    proc_macro::Group::new(delimiter, tts.into_token_stream()),
                )
            }
            TokenStream::Fallback(stream) => {
                Group::Fallback(fallback::Group::new(delimiter, stream))
            }
        }
    }
    pub fn delimiter(&self) -> Delimiter {
        match self {
            Group::Compiler(g) => {
                match g.delimiter() {
                    proc_macro::Delimiter::Parenthesis => Delimiter::Parenthesis,
                    proc_macro::Delimiter::Bracket => Delimiter::Bracket,
                    proc_macro::Delimiter::Brace => Delimiter::Brace,
                    proc_macro::Delimiter::None => Delimiter::None,
                }
            }
            Group::Fallback(g) => g.delimiter(),
        }
    }
    pub fn stream(&self) -> TokenStream {
        match self {
            Group::Compiler(g) => {
                TokenStream::Compiler(DeferredTokenStream::new(g.stream()))
            }
            Group::Fallback(g) => TokenStream::Fallback(g.stream()),
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Group::Compiler(g) => Span::Compiler(g.span()),
            Group::Fallback(g) => Span::Fallback(g.span()),
        }
    }
    pub fn span_open(&self) -> Span {
        match self {
            #[cfg(proc_macro_span)]
            Group::Compiler(g) => Span::Compiler(g.span_open()),
            #[cfg(not(proc_macro_span))]
            Group::Compiler(g) => Span::Compiler(g.span()),
            Group::Fallback(g) => Span::Fallback(g.span_open()),
        }
    }
    pub fn span_close(&self) -> Span {
        match self {
            #[cfg(proc_macro_span)]
            Group::Compiler(g) => Span::Compiler(g.span_close()),
            #[cfg(not(proc_macro_span))]
            Group::Compiler(g) => Span::Compiler(g.span()),
            Group::Fallback(g) => Span::Fallback(g.span_close()),
        }
    }
    pub fn set_span(&mut self, span: Span) {
        match (self, span) {
            (Group::Compiler(g), Span::Compiler(s)) => g.set_span(s),
            (Group::Fallback(g), Span::Fallback(s)) => g.set_span(s),
            _ => mismatch(),
        }
    }
    fn unwrap_nightly(self) -> proc_macro::Group {
        match self {
            Group::Compiler(g) => g,
            Group::Fallback(_) => mismatch(),
        }
    }
}
impl From<fallback::Group> for Group {
    fn from(g: fallback::Group) -> Self {
        Group::Fallback(g)
    }
}
impl Display for Group {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Group::Compiler(group) => Display::fmt(group, formatter),
            Group::Fallback(group) => Display::fmt(group, formatter),
        }
    }
}
impl Debug for Group {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Group::Compiler(group) => Debug::fmt(group, formatter),
            Group::Fallback(group) => Debug::fmt(group, formatter),
        }
    }
}
#[derive(Clone)]
pub(crate) enum Ident {
    Compiler(proc_macro::Ident),
    Fallback(fallback::Ident),
}
impl Ident {
    pub fn new(string: &str, span: Span) -> Ident {
        match span {
            Span::Compiler(s) => Ident::Compiler(proc_macro::Ident::new(string, s)),
            Span::Fallback(s) => Ident::Fallback(fallback::Ident::new(string, s)),
        }
    }
    pub fn new_raw(string: &str, span: Span) -> Ident {
        match span {
            Span::Compiler(s) => {
                let p: proc_macro::TokenStream = string.parse().unwrap();
                let ident = match p.into_iter().next() {
                    Some(proc_macro::TokenTree::Ident(mut i)) => {
                        i.set_span(s);
                        i
                    }
                    _ => panic!(),
                };
                Ident::Compiler(ident)
            }
            Span::Fallback(s) => Ident::Fallback(fallback::Ident::new_raw(string, s)),
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Ident::Compiler(t) => Span::Compiler(t.span()),
            Ident::Fallback(t) => Span::Fallback(t.span()),
        }
    }
    pub fn set_span(&mut self, span: Span) {
        match (self, span) {
            (Ident::Compiler(t), Span::Compiler(s)) => t.set_span(s),
            (Ident::Fallback(t), Span::Fallback(s)) => t.set_span(s),
            _ => mismatch(),
        }
    }
    fn unwrap_nightly(self) -> proc_macro::Ident {
        match self {
            Ident::Compiler(s) => s,
            Ident::Fallback(_) => mismatch(),
        }
    }
}
impl PartialEq for Ident {
    fn eq(&self, other: &Ident) -> bool {
        match (self, other) {
            (Ident::Compiler(t), Ident::Compiler(o)) => t.to_string() == o.to_string(),
            (Ident::Fallback(t), Ident::Fallback(o)) => t == o,
            _ => mismatch(),
        }
    }
}
impl<T> PartialEq<T> for Ident
where
    T: ?Sized + AsRef<str>,
{
    fn eq(&self, other: &T) -> bool {
        let other = other.as_ref();
        match self {
            Ident::Compiler(t) => t.to_string() == other,
            Ident::Fallback(t) => t == other,
        }
    }
}
impl Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ident::Compiler(t) => Display::fmt(t, f),
            Ident::Fallback(t) => Display::fmt(t, f),
        }
    }
}
impl Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ident::Compiler(t) => Debug::fmt(t, f),
            Ident::Fallback(t) => Debug::fmt(t, f),
        }
    }
}
#[derive(Clone)]
pub(crate) enum Literal {
    Compiler(proc_macro::Literal),
    Fallback(fallback::Literal),
}
macro_rules! suffixed_numbers {
    ($($name:ident => $kind:ident,)*) => {
        $(pub fn $name (n : $kind) -> Literal { if inside_proc_macro() {
        Literal::Compiler(proc_macro::Literal::$name (n)) } else {
        Literal::Fallback(fallback::Literal::$name (n)) } })*
    };
}
macro_rules! unsuffixed_integers {
    ($($name:ident => $kind:ident,)*) => {
        $(pub fn $name (n : $kind) -> Literal { if inside_proc_macro() {
        Literal::Compiler(proc_macro::Literal::$name (n)) } else {
        Literal::Fallback(fallback::Literal::$name (n)) } })*
    };
}
impl Literal {
    suffixed_numbers! {
        u8_suffixed => u8, u16_suffixed => u16, u32_suffixed => u32, u64_suffixed => u64,
        u128_suffixed => u128, usize_suffixed => usize, i8_suffixed => i8, i16_suffixed
        => i16, i32_suffixed => i32, i64_suffixed => i64, i128_suffixed => i128,
        isize_suffixed => isize, f32_suffixed => f32, f64_suffixed => f64,
    }
    unsuffixed_integers! {
        u8_unsuffixed => u8, u16_unsuffixed => u16, u32_unsuffixed => u32, u64_unsuffixed
        => u64, u128_unsuffixed => u128, usize_unsuffixed => usize, i8_unsuffixed => i8,
        i16_unsuffixed => i16, i32_unsuffixed => i32, i64_unsuffixed => i64,
        i128_unsuffixed => i128, isize_unsuffixed => isize,
    }
    pub fn f32_unsuffixed(f: f32) -> Literal {
        if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::f32_unsuffixed(f))
        } else {
            Literal::Fallback(fallback::Literal::f32_unsuffixed(f))
        }
    }
    pub fn f64_unsuffixed(f: f64) -> Literal {
        if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::f64_unsuffixed(f))
        } else {
            Literal::Fallback(fallback::Literal::f64_unsuffixed(f))
        }
    }
    pub fn string(t: &str) -> Literal {
        if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::string(t))
        } else {
            Literal::Fallback(fallback::Literal::string(t))
        }
    }
    pub fn character(t: char) -> Literal {
        if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::character(t))
        } else {
            Literal::Fallback(fallback::Literal::character(t))
        }
    }
    pub fn byte_string(bytes: &[u8]) -> Literal {
        if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::byte_string(bytes))
        } else {
            Literal::Fallback(fallback::Literal::byte_string(bytes))
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Literal::Compiler(lit) => Span::Compiler(lit.span()),
            Literal::Fallback(lit) => Span::Fallback(lit.span()),
        }
    }
    pub fn set_span(&mut self, span: Span) {
        match (self, span) {
            (Literal::Compiler(lit), Span::Compiler(s)) => lit.set_span(s),
            (Literal::Fallback(lit), Span::Fallback(s)) => lit.set_span(s),
            _ => mismatch(),
        }
    }
    pub fn subspan<R: RangeBounds<usize>>(&self, range: R) -> Option<Span> {
        match self {
            #[cfg(proc_macro_span)]
            Literal::Compiler(lit) => lit.subspan(range).map(Span::Compiler),
            #[cfg(not(proc_macro_span))]
            Literal::Compiler(_lit) => None,
            Literal::Fallback(lit) => lit.subspan(range).map(Span::Fallback),
        }
    }
    fn unwrap_nightly(self) -> proc_macro::Literal {
        match self {
            Literal::Compiler(s) => s,
            Literal::Fallback(_) => mismatch(),
        }
    }
}
impl From<fallback::Literal> for Literal {
    fn from(s: fallback::Literal) -> Literal {
        Literal::Fallback(s)
    }
}
impl Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Compiler(t) => Display::fmt(t, f),
            Literal::Fallback(t) => Display::fmt(t, f),
        }
    }
}
impl Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Compiler(t) => Debug::fmt(t, f),
            Literal::Fallback(t) => Debug::fmt(t, f),
        }
    }
}
#[cfg(test)]
mod tests_llm_16_68_llm_16_67 {
    use super::*;
    use crate::*;
    use std::str::FromStr;
    use fallback::TokenStream as FallbackTokenStream;
    use imp::TokenStream as CompilerTokenStream;
    use proc_macro::TokenStream as ProcMacroTokenStream;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_68_llm_16_67_rrrruuuugggg_test_from = 0;
        let inner = FallbackTokenStream::new();
        let result = CompilerTokenStream::from(inner);
        debug_assert!(matches!(result, CompilerTokenStream::Fallback(_)));
        let _rug_ed_tests_llm_16_68_llm_16_67_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_359 {
    use super::*;
    use crate::*;
    use proc_macro::TokenStream;
    #[test]
    fn test_into_token_stream() {
        let _rug_st_tests_llm_16_359_rrrruuuugggg_test_into_token_stream = 0;
        let token_stream = TokenStream::new();
        let deferred_token_stream = imp::DeferredTokenStream::new(token_stream);
        let result: proc_macro::TokenStream = deferred_token_stream.into_token_stream();
        let _rug_ed_tests_llm_16_359_rrrruuuugggg_test_into_token_stream = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_360 {
    use proc_macro::TokenStream;
    use crate::imp::DeferredTokenStream;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_360_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "";
        let rug_fuzz_2 = "";
        let rug_fuzz_3 = "";
        let empty_stream1: TokenStream = rug_fuzz_0.parse().unwrap();
        let empty_stream2: TokenStream = rug_fuzz_1.parse().unwrap();
        let empty_stream3: TokenStream = rug_fuzz_2.parse().unwrap();
        let empty_stream4: TokenStream = rug_fuzz_3.parse().unwrap();
        let deferred_token_stream1 = DeferredTokenStream::new(empty_stream1);
        let deferred_token_stream2 = DeferredTokenStream::new(empty_stream2);
        let deferred_token_stream3 = DeferredTokenStream::new(empty_stream3);
        let deferred_token_stream4 = DeferredTokenStream::new(empty_stream4);
        debug_assert_eq!(deferred_token_stream1.is_empty(), true);
        debug_assert_eq!(deferred_token_stream2.is_empty(), true);
        debug_assert_eq!(deferred_token_stream3.is_empty(), true);
        debug_assert_eq!(deferred_token_stream4.is_empty(), true);
        let _rug_ed_tests_llm_16_360_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_364 {
    use super::*;
    use crate::*;
    use crate::imp::Group;
    use crate::imp::TokenStream as ImpTokenStream;
    #[test]
    fn test_delimiter_compiler_parenthesis() {
        let _rug_st_tests_llm_16_364_rrrruuuugggg_test_delimiter_compiler_parenthesis = 0;
        let group = Group::Compiler(
            proc_macro::Group::new(
                proc_macro::Delimiter::Parenthesis,
                ImpTokenStream::new().into(),
            ),
        );
        debug_assert_eq!(group.delimiter(), Delimiter::Parenthesis);
        let _rug_ed_tests_llm_16_364_rrrruuuugggg_test_delimiter_compiler_parenthesis = 0;
    }
    #[test]
    fn test_delimiter_compiler_bracket() {
        let _rug_st_tests_llm_16_364_rrrruuuugggg_test_delimiter_compiler_bracket = 0;
        let group = Group::Compiler(
            proc_macro::Group::new(
                proc_macro::Delimiter::Bracket,
                ImpTokenStream::new().into(),
            ),
        );
        debug_assert_eq!(group.delimiter(), Delimiter::Bracket);
        let _rug_ed_tests_llm_16_364_rrrruuuugggg_test_delimiter_compiler_bracket = 0;
    }
    #[test]
    fn test_delimiter_compiler_brace() {
        let _rug_st_tests_llm_16_364_rrrruuuugggg_test_delimiter_compiler_brace = 0;
        let group = Group::Compiler(
            proc_macro::Group::new(
                proc_macro::Delimiter::Brace,
                ImpTokenStream::new().into(),
            ),
        );
        debug_assert_eq!(group.delimiter(), Delimiter::Brace);
        let _rug_ed_tests_llm_16_364_rrrruuuugggg_test_delimiter_compiler_brace = 0;
    }
    #[test]
    fn test_delimiter_compiler_none() {
        let _rug_st_tests_llm_16_364_rrrruuuugggg_test_delimiter_compiler_none = 0;
        let group = Group::Compiler(
            proc_macro::Group::new(
                proc_macro::Delimiter::None,
                ImpTokenStream::new().into(),
            ),
        );
        debug_assert_eq!(group.delimiter(), Delimiter::None);
        let _rug_ed_tests_llm_16_364_rrrruuuugggg_test_delimiter_compiler_none = 0;
    }
    #[test]
    fn test_delimiter_fallback() {
        let _rug_st_tests_llm_16_364_rrrruuuugggg_test_delimiter_fallback = 0;
        let group = Group::Fallback(
            fallback::Group::new(Delimiter::Parenthesis, fallback::TokenStream::new()),
        );
        debug_assert_eq!(group.delimiter(), Delimiter::Parenthesis);
        let _rug_ed_tests_llm_16_364_rrrruuuugggg_test_delimiter_fallback = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_368 {
    use super::*;
    use crate::*;
    use crate::*;
    #[test]
    #[cfg(proc_macro2_semver_exempt)]
    fn test_set_span_compiler_group_compiler_span() {
        let _rug_st_tests_llm_16_368_rrrruuuugggg_test_set_span_compiler_group_compiler_span = 0;
        let mut group = imp::Group::Compiler(
            proc_macro::Group::new(
                proc_macro::Delimiter::Parenthesis,
                imp::TokenStream::new(),
            ),
        );
        let span = imp::Span::Compiler(proc_macro::Span::call_site());
        group.set_span(span);
        let _rug_ed_tests_llm_16_368_rrrruuuugggg_test_set_span_compiler_group_compiler_span = 0;
    }
    #[test]
    #[cfg(proc_macro2_semver_exempt)]
    fn test_set_span_compiler_group_fallback_span() {
        let _rug_st_tests_llm_16_368_rrrruuuugggg_test_set_span_compiler_group_fallback_span = 0;
        let mut group = imp::Group::Compiler(
            proc_macro::Group::new(
                proc_macro::Delimiter::Parenthesis,
                imp::TokenStream::new(),
            ),
        );
        let span = imp::Span::Fallback(fallback::Span::call_site());
        group.set_span(span);
        let _rug_ed_tests_llm_16_368_rrrruuuugggg_test_set_span_compiler_group_fallback_span = 0;
    }
    #[test]
    fn test_set_span_fallback_group_fallback_span() {
        let _rug_st_tests_llm_16_368_rrrruuuugggg_test_set_span_fallback_group_fallback_span = 0;
        let mut group = imp::Group::Fallback(
            fallback::Group::new(Delimiter::Parenthesis, fallback::TokenStream::new()),
        );
        let span = imp::Span::Fallback(fallback::Span::call_site());
        group.set_span(span);
        let _rug_ed_tests_llm_16_368_rrrruuuugggg_test_set_span_fallback_group_fallback_span = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_404 {
    use crate::imp::Literal;
    use crate::fallback::{Literal as FallbackLiteral, Span};
    use std::fmt::{Debug, Display};
    fn inside_proc_macro() -> bool {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_inside_proc_macro = 0;
        unimplemented!();
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_inside_proc_macro = 0;
    }
    #[test]
    fn test_i16_suffixed() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_i16_suffixed = 0;
        let rug_fuzz_0 = 123;
        let n: i16 = rug_fuzz_0;
        let result = if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::i16_suffixed(n))
        } else {
            Literal::Fallback(FallbackLiteral::i16_suffixed(n))
        };
        unimplemented!();
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_i16_suffixed = 0;
    }
    #[test]
    fn test_u16_suffixed() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_u16_suffixed = 0;
        let rug_fuzz_0 = 123;
        let n: u16 = rug_fuzz_0;
        let result = if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::u16_suffixed(n))
        } else {
            Literal::Fallback(FallbackLiteral::u16_suffixed(n))
        };
        unimplemented!();
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_u16_suffixed = 0;
    }
    #[test]
    fn test_f32_unsuffixed() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_f32_unsuffixed = 0;
        let rug_fuzz_0 = 1.23;
        let f: f32 = rug_fuzz_0;
        let result = if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::f32_unsuffixed(f))
        } else {
            Literal::Fallback(FallbackLiteral::f32_unsuffixed(f))
        };
        unimplemented!();
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_f32_unsuffixed = 0;
    }
    #[test]
    fn test_f64_unsuffixed() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_f64_unsuffixed = 0;
        let rug_fuzz_0 = 1.23;
        let f: f64 = rug_fuzz_0;
        let result = if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::f64_unsuffixed(f))
        } else {
            Literal::Fallback(FallbackLiteral::f64_unsuffixed(f))
        };
        unimplemented!();
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_f64_unsuffixed = 0;
    }
    #[test]
    fn test_string() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_string = 0;
        let rug_fuzz_0 = "hello";
        let t: &str = rug_fuzz_0;
        let result = if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::string(t))
        } else {
            Literal::Fallback(FallbackLiteral::string(t))
        };
        unimplemented!();
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_string = 0;
    }
    #[test]
    fn test_character() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_character = 0;
        let rug_fuzz_0 = 'a';
        let t: char = rug_fuzz_0;
        let result = if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::character(t))
        } else {
            Literal::Fallback(FallbackLiteral::character(t))
        };
        unimplemented!();
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_character = 0;
    }
    #[test]
    fn test_byte_string() {
        let _rug_st_tests_llm_16_404_rrrruuuugggg_test_byte_string = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let bytes: &[u8] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let result = if inside_proc_macro() {
            Literal::Compiler(proc_macro::Literal::byte_string(bytes))
        } else {
            Literal::Fallback(FallbackLiteral::byte_string(bytes))
        };
        unimplemented!();
        let _rug_ed_tests_llm_16_404_rrrruuuugggg_test_byte_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_430 {
    use super::*;
    use crate::*;
    use crate::fallback::Literal as FallbackLiteral;
    use crate::fallback::Span as FallbackSpan;
    #[test]
    #[cfg(proc_macro_span)]
    fn test_subspan_compiler() {
        let _rug_st_tests_llm_16_430_rrrruuuugggg_test_subspan_compiler = 0;
        let rug_fuzz_0 = "hello";
        let literal = FallbackLiteral::string(rug_fuzz_0);
        let result = literal.subspan(..);
        debug_assert_eq!(result, Some(FallbackSpan {}));
        let _rug_ed_tests_llm_16_430_rrrruuuugggg_test_subspan_compiler = 0;
    }
    #[test]
    fn test_subspan_fallback() {
        let _rug_st_tests_llm_16_430_rrrruuuugggg_test_subspan_fallback = 0;
        let rug_fuzz_0 = "hello";
        let literal = FallbackLiteral::string(rug_fuzz_0);
        let result = literal.subspan(..);
        debug_assert_eq!(result, Some(FallbackSpan {}));
        let _rug_ed_tests_llm_16_430_rrrruuuugggg_test_subspan_fallback = 0;
    }
    #[test]
    #[cfg(not(proc_macro_span))]
    fn test_subspan_compiler_none() {
        let _rug_st_tests_llm_16_430_rrrruuuugggg_test_subspan_compiler_none = 0;
        let rug_fuzz_0 = "hello";
        let literal = FallbackLiteral::string(rug_fuzz_0);
        let result = literal.subspan(..);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_430_rrrruuuugggg_test_subspan_compiler_none = 0;
    }
    #[test]
    fn test_subspan_invalid() {
        let _rug_st_tests_llm_16_430_rrrruuuugggg_test_subspan_invalid = 0;
        let rug_fuzz_0 = "hello";
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 10;
        let literal = FallbackLiteral::string(rug_fuzz_0);
        let result = literal.subspan(rug_fuzz_1..rug_fuzz_2);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_430_rrrruuuugggg_test_subspan_invalid = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_482 {
    use crate::imp::mismatch;
    #[test]
    #[should_panic(expected = "stable/nightly mismatch")]
    fn test_mismatch() {
        let _rug_st_tests_llm_16_482_rrrruuuugggg_test_mismatch = 0;
        mismatch();
        let _rug_ed_tests_llm_16_482_rrrruuuugggg_test_mismatch = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_483 {
    use super::*;
    use crate::*;
    use crate::imp::LexError;
    use std::panic;
    #[test]
    fn test_proc_macro_parse() {
        let _rug_st_tests_llm_16_483_rrrruuuugggg_test_proc_macro_parse = 0;
        let rug_fuzz_0 = "";
        let result = panic::catch_unwind(|| {
            let src = rug_fuzz_0;
            let token_stream = proc_macro_parse(src).unwrap();
        });
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_483_rrrruuuugggg_test_proc_macro_parse = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::{Literal, TokenTree, Spacing};
    #[test]
    fn test_into_compiler_token() {
        let _rug_st_tests_rug_81_rrrruuuugggg_test_into_compiler_token = 0;
        let rug_fuzz_0 = "Hello, World!";
        let p0 = TokenTree::Literal(Literal::string(rug_fuzz_0));
        crate::imp::into_compiler_token(p0);
        let _rug_ed_tests_rug_81_rrrruuuugggg_test_into_compiler_token = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use proc_macro::TokenStream;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_rug = 0;
        let mut p0: TokenStream = TokenStream::new();
        crate::imp::DeferredTokenStream::new(p0);
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::imp::DeferredTokenStream;
    use proc_macro::TokenStream;
    #[test]
    fn test_evaluate_now() {
        let _rug_st_tests_rug_84_rrrruuuugggg_test_evaluate_now = 0;
        let mut v18 = DeferredTokenStream::new(TokenStream::new());
        v18.evaluate_now();
        let _rug_ed_tests_rug_84_rrrruuuugggg_test_evaluate_now = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_86_rrrruuuugggg_test_rug = 0;
        use crate::imp::TokenStream;
        let mut p0: TokenStream = TokenStream::new();
        crate::imp::TokenStream::is_empty(&p0);
        let _rug_ed_tests_rug_86_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::imp::TokenStream;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_87_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_87_prepare {
            #[test]
            fn sample() {
                let _rug_st_tests_rug_87_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 0;
                let _rug_st_tests_rug_87_rrrruuuugggg_sample = rug_fuzz_0;
                use crate::imp::TokenStream;
                let mut v11: TokenStream = TokenStream::new();
                let _rug_ed_tests_rug_87_rrrruuuugggg_sample = rug_fuzz_1;
                let _rug_ed_tests_rug_87_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: TokenStream = TokenStream::new();
        crate::imp::TokenStream::unwrap_nightly(p0);
        let _rug_ed_tests_rug_87_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use crate::imp::Span;
    #[cfg(hygiene)]
    use proc_macro::Span as CompilerSpan;
    #[cfg(hygiene)]
    use fallback::Span as FallbackSpan;
    #[test]
    fn test_rug() {
        mixed_site();
    }
    #[cfg(hygiene)]
    fn inside_proc_macro() -> bool {
        unimplemented!()
    }
    fn mixed_site() {
        if inside_proc_macro() {
            Span::Compiler(CompilerSpan::mixed_site());
        } else {
            Span::Fallback(FallbackSpan::mixed_site());
        }
    }
}
#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::imp::Span;
    #[test]
    fn test_resolved_at() {
        let _rug_st_tests_rug_104_rrrruuuugggg_test_resolved_at = 0;
        let p0 = Span::call_site();
        let p1 = Span::from(Span::mixed_site());
        let result = Span::resolved_at(&p0, p1);
        let _rug_ed_tests_rug_104_rrrruuuugggg_test_resolved_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    #[cfg(hygiene)]
    use crate::Span;
    #[cfg(not(hygiene))]
    use proc_macro::Span;
    #[test]
    fn test_located_at() {
        let _rug_st_tests_rug_105_rrrruuuugggg_test_located_at = 0;
        let p0 = Span::call_site();
        let p1 = Span::call_site();
        p0.located_at(p1);
        let _rug_ed_tests_rug_105_rrrruuuugggg_test_located_at = 0;
    }
}
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use crate::imp::Span;
    use proc_macro::Span as ProcMacroSpan;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_106_rrrruuuugggg_test_rug = 0;
        let p0 = Span::Compiler(ProcMacroSpan::call_site());
        Span::unwrap(p0);
        let _rug_ed_tests_rug_106_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::{Span, TokenStream};
    use quote::quote;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_107_rrrruuuugggg_test_rug = 0;
        let p0 = Span::call_site();
        let p1 = Span::mixed_site();
        p0.join(p1);
        let _rug_ed_tests_rug_107_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use crate::imp::{Group, TokenStream, Delimiter};
    use crate::imp::fallback;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_111_rrrruuuugggg_test_rug = 0;
        let mut p0 = Delimiter::Parenthesis;
        let mut p1: TokenStream = TokenStream::new();
        Group::new(p0, p1);
        let _rug_ed_tests_rug_111_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_112 {
    use super::*;
    use proc_macro::Delimiter;
    use proc_macro::Span;
    use proc_macro::TokenStream;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_112_rrrruuuugggg_test_rug = 0;
        let mut p0 = Group::Compiler(
            proc_macro::Group::new(Delimiter::Parenthesis, TokenStream::new()),
        );
        crate::imp::Group::stream(&p0);
        let _rug_ed_tests_rug_112_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::Ident;
    use crate::Span;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_121_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example_ident";
        let mut p0 = Ident::new(rug_fuzz_0, Span::call_site());
        let p1 = Span::call_site();
        p0.set_span(p1);
        let _rug_ed_tests_rug_121_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_125_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u8 = rug_fuzz_0;
        Literal::u8_suffixed(p0);
        let _rug_ed_tests_rug_125_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::Literal;
    use proc_macro::Literal as CompilerLiteral;
    use fallback::Literal as FallbackLiteral;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_126_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: u16 = rug_fuzz_0;
        Literal::u16_suffixed(p0);
        let _rug_ed_tests_rug_126_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    use crate::{Literal, TokenStream};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_127_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        Literal::u32_suffixed(p0);
        let _rug_ed_tests_rug_127_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_128 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_128_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        Literal::u64_suffixed(p0);
        let _rug_ed_tests_rug_128_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::{Literal, TokenStream};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_129_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let p0: u128 = rug_fuzz_0;
        Literal::u128_suffixed(p0);
        let _rug_ed_tests_rug_129_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_130_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: usize = rug_fuzz_0;
        crate::imp::Literal::usize_suffixed(p0);
        let _rug_ed_tests_rug_130_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use crate::Literal;
    use crate::TokenStream;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_131_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        Literal::i8_suffixed(p0);
        let _rug_ed_tests_rug_131_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_132 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_i32_suffixed() {
        let _rug_st_tests_rug_132_rrrruuuugggg_test_i32_suffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        Literal::i32_suffixed(p0);
        let _rug_ed_tests_rug_132_rrrruuuugggg_test_i32_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_133 {
    use super::*;
    use crate::imp::Literal;
    use crate::fallback;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_133_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        Literal::i64_suffixed(p0);
        let _rug_ed_tests_rug_133_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_134_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123456789;
        let mut p0: i128 = rug_fuzz_0;
        crate::imp::Literal::i128_suffixed(p0);
        let _rug_ed_tests_rug_134_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_135 {
    use super::*;
    use crate::imp::Literal;
    use proc_macro::Literal as CompilerLiteral;
    use crate::fallback::Literal as FallbackLiteral;
    #[test]
    fn test_isize_suffixed() {
        let _rug_st_tests_rug_135_rrrruuuugggg_test_isize_suffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: isize = rug_fuzz_0;
        let result = Literal::isize_suffixed(p0);
        let _rug_ed_tests_rug_135_rrrruuuugggg_test_isize_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_136_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f32 = rug_fuzz_0;
        crate::imp::Literal::f32_suffixed(p0);
        let _rug_ed_tests_rug_136_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_137_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f64 = rug_fuzz_0;
        crate::imp::Literal::f64_suffixed(p0);
        let _rug_ed_tests_rug_137_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::Literal;
    use proc_macro::Literal as CompilerLiteral;
    use fallback::Literal as FallbackLiteral;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        Literal::u8_unsuffixed(p0);
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        Literal::u16_unsuffixed(p0);
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_140_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let p0: u32 = rug_fuzz_0;
        Literal::u32_unsuffixed(p0);
        let _rug_ed_tests_rug_140_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_141 {
    use super::*;
    use crate::Literal;
    use crate::fallback;
    use proc_macro::Literal as CompilerLiteral;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_141_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        let _ = crate::imp::Literal::u64_unsuffixed(p0);
        let _rug_ed_tests_rug_141_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_142 {
    use super::*;
    use crate::imp::Literal;
    use crate::fallback::Literal as FallbackLiteral;
    use proc_macro::Literal as ProcMacroLiteral;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_142_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let p0: u128 = rug_fuzz_0;
        Literal::u128_unsuffixed(p0);
        let _rug_ed_tests_rug_142_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_usize_unsuffixed() {
        let _rug_st_tests_rug_143_rrrruuuugggg_test_usize_unsuffixed = 0;
        let rug_fuzz_0 = 10;
        let p0: usize = rug_fuzz_0;
        Literal::usize_unsuffixed(p0);
        let _rug_ed_tests_rug_143_rrrruuuugggg_test_usize_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::imp::Literal;
    use crate::imp::fallback;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_144_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i8 = rug_fuzz_0;
        Literal::i8_unsuffixed(p0);
        let _rug_ed_tests_rug_144_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_i16_unsuffixed() {
        let _rug_st_tests_rug_145_rrrruuuugggg_test_i16_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        Literal::i16_unsuffixed(p0);
        let _rug_ed_tests_rug_145_rrrruuuugggg_test_i16_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use crate::{Literal, TokenStream};
    use crate::fallback::Literal as FallbackLiteral;
    #[test]
    fn test_i32_unsuffixed() {
        let _rug_st_tests_rug_146_rrrruuuugggg_test_i32_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: i32 = rug_fuzz_0;
        Literal::i32_unsuffixed(p0);
        let _rug_ed_tests_rug_146_rrrruuuugggg_test_i32_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    use crate::imp::Literal;
    use proc_macro::Literal as CompilerLiteral;
    use crate::fallback::Literal as FallbackLiteral;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_147_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        Literal::i64_unsuffixed(p0);
        let _rug_ed_tests_rug_147_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::{Literal, TokenStream};
    use proc_macro::Literal as CompilerLiteral;
    use crate::fallback::Literal as FallbackLiteral;
    use crate::imp;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_148_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0: i128 = rug_fuzz_0;
        imp::Literal::i128_unsuffixed(p0);
        let _rug_ed_tests_rug_148_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::Literal;
    use proc_macro::Literal as CompilerLiteral;
    use fallback::Literal as FallbackLiteral;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_149_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: isize = rug_fuzz_0;
        Literal::isize_unsuffixed(p0);
        let _rug_ed_tests_rug_149_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::imp::Literal;
    use crate::fallback;
    #[test]
    fn test_f32_unsuffixed() {
        let _rug_st_tests_rug_150_rrrruuuugggg_test_f32_unsuffixed = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f32 = rug_fuzz_0;
        Literal::f32_unsuffixed(p0);
        let _rug_ed_tests_rug_150_rrrruuuugggg_test_f32_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_151 {
    use super::*;
    use crate::{Literal, TokenStream, Group as TokenGroup, Delimiter};
    #[test]
    fn test_f64_unsuffixed() {
        let _rug_st_tests_rug_151_rrrruuuugggg_test_f64_unsuffixed = 0;
        let rug_fuzz_0 = 3.14159;
        let p0: f64 = rug_fuzz_0;
        Literal::f64_unsuffixed(p0);
        let _rug_ed_tests_rug_151_rrrruuuugggg_test_f64_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_152 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_string() {
        let _rug_st_tests_rug_152_rrrruuuugggg_test_string = 0;
        let rug_fuzz_0 = "Sample string";
        let p0: &str = rug_fuzz_0;
        Literal::string(&p0);
        let _rug_ed_tests_rug_152_rrrruuuugggg_test_string = 0;
    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_character() {
        let _rug_st_tests_rug_153_rrrruuuugggg_test_character = 0;
        let rug_fuzz_0 = 'a';
        let p0: char = rug_fuzz_0;
        Literal::character(p0);
        let _rug_ed_tests_rug_153_rrrruuuugggg_test_character = 0;
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::imp::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_154_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"sample_bytes";
        let p0: &[u8] = rug_fuzz_0;
        Literal::byte_string(p0);
        let _rug_ed_tests_rug_154_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::{Span, Literal};
    #[test]
    fn test_span() {
        let _rug_st_tests_rug_155_rrrruuuugggg_test_span = 0;
        let rug_fuzz_0 = 10;
        let p0 = Literal::u8_suffixed(rug_fuzz_0);
        p0.span();
        let _rug_ed_tests_rug_155_rrrruuuugggg_test_span = 0;
    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use crate::Span;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_156_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = Literal::u8_suffixed(rug_fuzz_0);
        let p1 = Span::call_site();
        Literal::set_span(&mut p0, p1);
        let _rug_ed_tests_rug_156_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::Span;
    use crate::fallback::Literal;
    use crate::imp;
    use std::convert::From;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_158_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0 = Literal::_new(rug_fuzz_0.to_string());
        <imp::Literal as std::convert::From<fallback::Literal>>::from(p0);
        let _rug_ed_tests_rug_158_rrrruuuugggg_test_rug = 0;
    }
}
