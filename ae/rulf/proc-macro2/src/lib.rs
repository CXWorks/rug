//! A wrapper around the procedural macro API of the compiler's [`proc_macro`]
//! crate. This library serves two purposes:
//!
//! [`proc_macro`]: https://doc.rust-lang.org/proc_macro/
//!
//! - **Bring proc-macro-like functionality to other contexts like build.rs and
//!   main.rs.** Types from `proc_macro` are entirely specific to procedural
//!   macros and cannot ever exist in code outside of a procedural macro.
//!   Meanwhile `proc_macro2` types may exist anywhere including non-macro code.
//!   By developing foundational libraries like [syn] and [quote] against
//!   `proc_macro2` rather than `proc_macro`, the procedural macro ecosystem
//!   becomes easily applicable to many other use cases and we avoid
//!   reimplementing non-macro equivalents of those libraries.
//!
//! - **Make procedural macros unit testable.** As a consequence of being
//!   specific to procedural macros, nothing that uses `proc_macro` can be
//!   executed from a unit test. In order for helper libraries or components of
//!   a macro to be testable in isolation, they must be implemented using
//!   `proc_macro2`.
//!
//! [syn]: https://github.com/dtolnay/syn
//! [quote]: https://github.com/dtolnay/quote
//!
//! # Usage
//!
//! The skeleton of a typical procedural macro typically looks like this:
//!
//! ```
//! extern crate proc_macro;
//!
//! # const IGNORE: &str = stringify! {
//! #[proc_macro_derive(MyDerive)]
//! # };
//! # #[cfg(wrap_proc_macro)]
//! pub fn my_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//!     let input = proc_macro2::TokenStream::from(input);
//!
//!     let output: proc_macro2::TokenStream = {
//!         /* transform input */
//!         # input
//!     };
//!
//!     proc_macro::TokenStream::from(output)
//! }
//! ```
//!
//! If parsing with [Syn], you'll use [`parse_macro_input!`] instead to
//! propagate parse errors correctly back to the compiler when parsing fails.
//!
//! [`parse_macro_input!`]: https://docs.rs/syn/1.0/syn/macro.parse_macro_input.html
//!
//! # Unstable features
//!
//! The default feature set of proc-macro2 tracks the most recent stable
//! compiler API. Functionality in `proc_macro` that is not yet stable is not
//! exposed by proc-macro2 by default.
//!
//! To opt into the additional APIs available in the most recent nightly
//! compiler, the `procmacro2_semver_exempt` config flag must be passed to
//! rustc. We will polyfill those nightly-only APIs back to Rust 1.31.0. As
//! these are unstable APIs that track the nightly compiler, minor versions of
//! proc-macro2 may make breaking changes to them at any time.
//!
//! ```sh
//! RUSTFLAGS='--cfg procmacro2_semver_exempt' cargo build
//! ```
//!
//! Note that this must not only be done for your crate, but for any crate that
//! depends on your crate. This infectious nature is intentional, as it serves
//! as a reminder that you are outside of the normal semver guarantees.
//!
//! Semver exempt methods are marked as such in the proc-macro2 documentation.
//!
//! # Thread-Safety
//!
//! Most types in this crate are `!Sync` because the underlying compiler
//! types make use of thread-local memory, meaning they cannot be accessed from
//! a different thread.
#![doc(html_root_url = "https://docs.rs/proc-macro2/1.0.24")]
#![cfg_attr(any(proc_macro_span, super_unstable), feature(proc_macro_span))]
#![cfg_attr(super_unstable, feature(proc_macro_raw_ident, proc_macro_def_site))]
#![allow(clippy::needless_doctest_main)]
#[cfg(use_proc_macro)]
extern crate proc_macro;
mod marker;
mod parse;
#[cfg(wrap_proc_macro)]
mod detection;
#[doc(hidden)]
pub mod fallback;
#[cfg(not(wrap_proc_macro))]
use crate::fallback as imp;
#[path = "wrapper.rs"]
#[cfg(wrap_proc_macro)]
mod imp;
use crate::marker::Marker;
use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::RangeBounds;
#[cfg(procmacro2_semver_exempt)]
use std::path::PathBuf;
use std::str::FromStr;
/// An abstract stream of tokens, or more concretely a sequence of token trees.
///
/// This type provides interfaces for iterating over token trees and for
/// collecting token trees into one stream.
///
/// Token stream is both the input and output of `#[proc_macro]`,
/// `#[proc_macro_attribute]` and `#[proc_macro_derive]` definitions.
#[derive(Clone)]
pub struct TokenStream {
    inner: imp::TokenStream,
    _marker: Marker,
}
/// Error returned from `TokenStream::from_str`.
pub struct LexError {
    inner: imp::LexError,
    _marker: Marker,
}
impl TokenStream {
    fn _new(inner: imp::TokenStream) -> TokenStream {
        TokenStream {
            inner,
            _marker: Marker,
        }
    }
    fn _new_stable(inner: fallback::TokenStream) -> TokenStream {
        TokenStream {
            inner: inner.into(),
            _marker: Marker,
        }
    }
    /// Returns an empty `TokenStream` containing no token trees.
    pub fn new() -> TokenStream {
        TokenStream::_new(imp::TokenStream::new())
    }
    /// Checks if this `TokenStream` is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
/// `TokenStream::default()` returns an empty stream,
/// i.e. this is equivalent with `TokenStream::new()`.
impl Default for TokenStream {
    fn default() -> Self {
        TokenStream::new()
    }
}
/// Attempts to break the string into tokens and parse those tokens into a token
/// stream.
///
/// May fail for a number of reasons, for example, if the string contains
/// unbalanced delimiters or characters not existing in the language.
///
/// NOTE: Some errors may cause panics instead of returning `LexError`. We
/// reserve the right to change these errors into `LexError`s later.
impl FromStr for TokenStream {
    type Err = LexError;
    fn from_str(src: &str) -> Result<TokenStream, LexError> {
        let e = src
            .parse()
            .map_err(|e| LexError {
                inner: e,
                _marker: Marker,
            })?;
        Ok(TokenStream::_new(e))
    }
}
#[cfg(use_proc_macro)]
impl From<proc_macro::TokenStream> for TokenStream {
    fn from(inner: proc_macro::TokenStream) -> TokenStream {
        TokenStream::_new(inner.into())
    }
}
#[cfg(use_proc_macro)]
impl From<TokenStream> for proc_macro::TokenStream {
    fn from(inner: TokenStream) -> proc_macro::TokenStream {
        inner.inner.into()
    }
}
impl From<TokenTree> for TokenStream {
    fn from(token: TokenTree) -> Self {
        TokenStream::_new(imp::TokenStream::from(token))
    }
}
impl Extend<TokenTree> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenTree>>(&mut self, streams: I) {
        self.inner.extend(streams)
    }
}
impl Extend<TokenStream> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenStream>>(&mut self, streams: I) {
        self.inner.extend(streams.into_iter().map(|stream| stream.inner))
    }
}
/// Collects a number of token trees into a single stream.
impl FromIterator<TokenTree> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenTree>>(streams: I) -> Self {
        TokenStream::_new(streams.into_iter().collect())
    }
}
impl FromIterator<TokenStream> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenStream>>(streams: I) -> Self {
        TokenStream::_new(streams.into_iter().map(|i| i.inner).collect())
    }
}
/// Prints the token stream as a string that is supposed to be losslessly
/// convertible back into the same token stream (modulo spans), except for
/// possibly `TokenTree::Group`s with `Delimiter::None` delimiters and negative
/// numeric literals.
impl Display for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
/// Prints token in a form convenient for debugging.
impl Debug for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
impl Debug for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
impl Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
impl Error for LexError {}
/// The source file of a given `Span`.
///
/// This type is semver exempt and not exposed by default.
#[cfg(procmacro2_semver_exempt)]
#[derive(Clone, PartialEq, Eq)]
pub struct SourceFile {
    inner: imp::SourceFile,
    _marker: Marker,
}
#[cfg(procmacro2_semver_exempt)]
impl SourceFile {
    fn _new(inner: imp::SourceFile) -> Self {
        SourceFile {
            inner,
            _marker: Marker,
        }
    }
    /// Get the path to this source file.
    ///
    /// ### Note
    ///
    /// If the code span associated with this `SourceFile` was generated by an
    /// external macro, this may not be an actual path on the filesystem. Use
    /// [`is_real`] to check.
    ///
    /// Also note that even if `is_real` returns `true`, if
    /// `--remap-path-prefix` was passed on the command line, the path as given
    /// may not actually be valid.
    ///
    /// [`is_real`]: #method.is_real
    pub fn path(&self) -> PathBuf {
        self.inner.path()
    }
    /// Returns `true` if this source file is a real source file, and not
    /// generated by an external macro's expansion.
    pub fn is_real(&self) -> bool {
        self.inner.is_real()
    }
}
#[cfg(procmacro2_semver_exempt)]
impl Debug for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
/// A line-column pair representing the start or end of a `Span`.
///
/// This type is semver exempt and not exposed by default.
#[cfg(span_locations)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LineColumn {
    /// The 1-indexed line in the source file on which the span starts or ends
    /// (inclusive).
    pub line: usize,
    /// The 0-indexed column (in UTF-8 characters) in the source file on which
    /// the span starts or ends (inclusive).
    pub column: usize,
}
#[cfg(span_locations)]
impl Ord for LineColumn {
    fn cmp(&self, other: &Self) -> Ordering {
        self.line.cmp(&other.line).then(self.column.cmp(&other.column))
    }
}
#[cfg(span_locations)]
impl PartialOrd for LineColumn {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
/// A region of source code, along with macro expansion information.
#[derive(Copy, Clone)]
pub struct Span {
    inner: imp::Span,
    _marker: Marker,
}
impl Span {
    fn _new(inner: imp::Span) -> Span {
        Span { inner, _marker: Marker }
    }
    fn _new_stable(inner: fallback::Span) -> Span {
        Span {
            inner: inner.into(),
            _marker: Marker,
        }
    }
    /// The span of the invocation of the current procedural macro.
    ///
    /// Identifiers created with this span will be resolved as if they were
    /// written directly at the macro call location (call-site hygiene) and
    /// other code at the macro call site will be able to refer to them as well.
    pub fn call_site() -> Span {
        Span::_new(imp::Span::call_site())
    }
    /// The span located at the invocation of the procedural macro, but with
    /// local variables, labels, and `$crate` resolved at the definition site
    /// of the macro. This is the same hygiene behavior as `macro_rules`.
    ///
    /// This function requires Rust 1.45 or later.
    #[cfg(hygiene)]
    pub fn mixed_site() -> Span {
        Span::_new(imp::Span::mixed_site())
    }
    /// A span that resolves at the macro definition site.
    ///
    /// This method is semver exempt and not exposed by default.
    #[cfg(procmacro2_semver_exempt)]
    pub fn def_site() -> Span {
        Span::_new(imp::Span::def_site())
    }
    /// Creates a new span with the same line/column information as `self` but
    /// that resolves symbols as though it were at `other`.
    pub fn resolved_at(&self, other: Span) -> Span {
        Span::_new(self.inner.resolved_at(other.inner))
    }
    /// Creates a new span with the same name resolution behavior as `self` but
    /// with the line/column information of `other`.
    pub fn located_at(&self, other: Span) -> Span {
        Span::_new(self.inner.located_at(other.inner))
    }
    /// Convert `proc_macro2::Span` to `proc_macro::Span`.
    ///
    /// This method is available when building with a nightly compiler, or when
    /// building with rustc 1.29+ *without* semver exempt features.
    ///
    /// # Panics
    ///
    /// Panics if called from outside of a procedural macro. Unlike
    /// `proc_macro2::Span`, the `proc_macro::Span` type can only exist within
    /// the context of a procedural macro invocation.
    #[cfg(wrap_proc_macro)]
    pub fn unwrap(self) -> proc_macro::Span {
        self.inner.unwrap()
    }
    #[cfg(wrap_proc_macro)]
    #[doc(hidden)]
    pub fn unstable(self) -> proc_macro::Span {
        self.unwrap()
    }
    /// The original source file into which this span points.
    ///
    /// This method is semver exempt and not exposed by default.
    #[cfg(procmacro2_semver_exempt)]
    pub fn source_file(&self) -> SourceFile {
        SourceFile::_new(self.inner.source_file())
    }
    /// Get the starting line/column in the source file for this span.
    ///
    /// This method requires the `"span-locations"` feature to be enabled.
    #[cfg(span_locations)]
    pub fn start(&self) -> LineColumn {
        let imp::LineColumn { line, column } = self.inner.start();
        LineColumn { line, column }
    }
    /// Get the ending line/column in the source file for this span.
    ///
    /// This method requires the `"span-locations"` feature to be enabled.
    #[cfg(span_locations)]
    pub fn end(&self) -> LineColumn {
        let imp::LineColumn { line, column } = self.inner.end();
        LineColumn { line, column }
    }
    /// Create a new span encompassing `self` and `other`.
    ///
    /// Returns `None` if `self` and `other` are from different files.
    ///
    /// Warning: the underlying [`proc_macro::Span::join`] method is
    /// nightly-only. When called from within a procedural macro not using a
    /// nightly compiler, this method will always return `None`.
    ///
    /// [`proc_macro::Span::join`]: https://doc.rust-lang.org/proc_macro/struct.Span.html#method.join
    pub fn join(&self, other: Span) -> Option<Span> {
        self.inner.join(other.inner).map(Span::_new)
    }
    /// Compares two spans to see if they're equal.
    ///
    /// This method is semver exempt and not exposed by default.
    #[cfg(procmacro2_semver_exempt)]
    pub fn eq(&self, other: &Span) -> bool {
        self.inner.eq(&other.inner)
    }
}
/// Prints a span in a form convenient for debugging.
impl Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
/// A single token or a delimited sequence of token trees (e.g. `[1, (), ..]`).
#[derive(Clone)]
pub enum TokenTree {
    /// A token stream surrounded by bracket delimiters.
    Group(Group),
    /// An identifier.
    Ident(Ident),
    /// A single punctuation character (`+`, `,`, `$`, etc.).
    Punct(Punct),
    /// A literal character (`'a'`), string (`"hello"`), number (`2.3`), etc.
    Literal(Literal),
}
impl TokenTree {
    /// Returns the span of this tree, delegating to the `span` method of
    /// the contained token or a delimited stream.
    pub fn span(&self) -> Span {
        match self {
            TokenTree::Group(t) => t.span(),
            TokenTree::Ident(t) => t.span(),
            TokenTree::Punct(t) => t.span(),
            TokenTree::Literal(t) => t.span(),
        }
    }
    /// Configures the span for *only this token*.
    ///
    /// Note that if this token is a `Group` then this method will not configure
    /// the span of each of the internal tokens, this will simply delegate to
    /// the `set_span` method of each variant.
    pub fn set_span(&mut self, span: Span) {
        match self {
            TokenTree::Group(t) => t.set_span(span),
            TokenTree::Ident(t) => t.set_span(span),
            TokenTree::Punct(t) => t.set_span(span),
            TokenTree::Literal(t) => t.set_span(span),
        }
    }
}
impl From<Group> for TokenTree {
    fn from(g: Group) -> TokenTree {
        TokenTree::Group(g)
    }
}
impl From<Ident> for TokenTree {
    fn from(g: Ident) -> TokenTree {
        TokenTree::Ident(g)
    }
}
impl From<Punct> for TokenTree {
    fn from(g: Punct) -> TokenTree {
        TokenTree::Punct(g)
    }
}
impl From<Literal> for TokenTree {
    fn from(g: Literal) -> TokenTree {
        TokenTree::Literal(g)
    }
}
/// Prints the token tree as a string that is supposed to be losslessly
/// convertible back into the same token tree (modulo spans), except for
/// possibly `TokenTree::Group`s with `Delimiter::None` delimiters and negative
/// numeric literals.
impl Display for TokenTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenTree::Group(t) => Display::fmt(t, f),
            TokenTree::Ident(t) => Display::fmt(t, f),
            TokenTree::Punct(t) => Display::fmt(t, f),
            TokenTree::Literal(t) => Display::fmt(t, f),
        }
    }
}
/// Prints token tree in a form convenient for debugging.
impl Debug for TokenTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenTree::Group(t) => Debug::fmt(t, f),
            TokenTree::Ident(t) => {
                let mut debug = f.debug_struct("Ident");
                debug.field("sym", &format_args!("{}", t));
                imp::debug_span_field_if_nontrivial(&mut debug, t.span().inner);
                debug.finish()
            }
            TokenTree::Punct(t) => Debug::fmt(t, f),
            TokenTree::Literal(t) => Debug::fmt(t, f),
        }
    }
}
/// A delimited token stream.
///
/// A `Group` internally contains a `TokenStream` which is surrounded by
/// `Delimiter`s.
#[derive(Clone)]
pub struct Group {
    inner: imp::Group,
}
/// Describes how a sequence of token trees is delimited.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Delimiter {
    /// `( ... )`
    Parenthesis,
    /// `{ ... }`
    Brace,
    /// `[ ... ]`
    Bracket,
    /// `Ø ... Ø`
    ///
    /// An implicit delimiter, that may, for example, appear around tokens
    /// coming from a "macro variable" `$var`. It is important to preserve
    /// operator priorities in cases like `$var * 3` where `$var` is `1 + 2`.
    /// Implicit delimiters may not survive roundtrip of a token stream through
    /// a string.
    None,
}
impl Group {
    fn _new(inner: imp::Group) -> Self {
        Group { inner }
    }
    fn _new_stable(inner: fallback::Group) -> Self {
        Group { inner: inner.into() }
    }
    /// Creates a new `Group` with the given delimiter and token stream.
    ///
    /// This constructor will set the span for this group to
    /// `Span::call_site()`. To change the span you can use the `set_span`
    /// method below.
    pub fn new(delimiter: Delimiter, stream: TokenStream) -> Group {
        Group {
            inner: imp::Group::new(delimiter, stream.inner),
        }
    }
    /// Returns the delimiter of this `Group`
    pub fn delimiter(&self) -> Delimiter {
        self.inner.delimiter()
    }
    /// Returns the `TokenStream` of tokens that are delimited in this `Group`.
    ///
    /// Note that the returned token stream does not include the delimiter
    /// returned above.
    pub fn stream(&self) -> TokenStream {
        TokenStream::_new(self.inner.stream())
    }
    /// Returns the span for the delimiters of this token stream, spanning the
    /// entire `Group`.
    ///
    /// ```text
    /// pub fn span(&self) -> Span {
    ///            ^^^^^^^
    /// ```
    pub fn span(&self) -> Span {
        Span::_new(self.inner.span())
    }
    /// Returns the span pointing to the opening delimiter of this group.
    ///
    /// ```text
    /// pub fn span_open(&self) -> Span {
    ///                 ^
    /// ```
    pub fn span_open(&self) -> Span {
        Span::_new(self.inner.span_open())
    }
    /// Returns the span pointing to the closing delimiter of this group.
    ///
    /// ```text
    /// pub fn span_close(&self) -> Span {
    ///                        ^
    /// ```
    pub fn span_close(&self) -> Span {
        Span::_new(self.inner.span_close())
    }
    /// Configures the span for this `Group`'s delimiters, but not its internal
    /// tokens.
    ///
    /// This method will **not** set the span of all the internal tokens spanned
    /// by this group, but rather it will only set the span of the delimiter
    /// tokens at the level of the `Group`.
    pub fn set_span(&mut self, span: Span) {
        self.inner.set_span(span.inner)
    }
}
/// Prints the group as a string that should be losslessly convertible back
/// into the same group (modulo spans), except for possibly `TokenTree::Group`s
/// with `Delimiter::None` delimiters.
impl Display for Group {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, formatter)
    }
}
impl Debug for Group {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, formatter)
    }
}
/// An `Punct` is an single punctuation character like `+`, `-` or `#`.
///
/// Multicharacter operators like `+=` are represented as two instances of
/// `Punct` with different forms of `Spacing` returned.
#[derive(Clone)]
pub struct Punct {
    ch: char,
    spacing: Spacing,
    span: Span,
}
/// Whether an `Punct` is followed immediately by another `Punct` or followed by
/// another token or whitespace.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Spacing {
    /// E.g. `+` is `Alone` in `+ =`, `+ident` or `+()`.
    Alone,
    /// E.g. `+` is `Joint` in `+=` or `'` is `Joint` in `'#`.
    ///
    /// Additionally, single quote `'` can join with identifiers to form
    /// lifetimes `'ident`.
    Joint,
}
impl Punct {
    /// Creates a new `Punct` from the given character and spacing.
    ///
    /// The `ch` argument must be a valid punctuation character permitted by the
    /// language, otherwise the function will panic.
    ///
    /// The returned `Punct` will have the default span of `Span::call_site()`
    /// which can be further configured with the `set_span` method below.
    pub fn new(ch: char, spacing: Spacing) -> Punct {
        Punct {
            ch,
            spacing,
            span: Span::call_site(),
        }
    }
    /// Returns the value of this punctuation character as `char`.
    pub fn as_char(&self) -> char {
        self.ch
    }
    /// Returns the spacing of this punctuation character, indicating whether
    /// it's immediately followed by another `Punct` in the token stream, so
    /// they can potentially be combined into a multicharacter operator
    /// (`Joint`), or it's followed by some other token or whitespace (`Alone`)
    /// so the operator has certainly ended.
    pub fn spacing(&self) -> Spacing {
        self.spacing
    }
    /// Returns the span for this punctuation character.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Configure the span for this punctuation character.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}
/// Prints the punctuation character as a string that should be losslessly
/// convertible back into the same character.
impl Display for Punct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.ch, f)
    }
}
impl Debug for Punct {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = fmt.debug_struct("Punct");
        debug.field("char", &self.ch);
        debug.field("spacing", &self.spacing);
        imp::debug_span_field_if_nontrivial(&mut debug, self.span.inner);
        debug.finish()
    }
}
/// A word of Rust code, which may be a keyword or legal variable name.
///
/// An identifier consists of at least one Unicode code point, the first of
/// which has the XID_Start property and the rest of which have the XID_Continue
/// property.
///
/// - The empty string is not an identifier. Use `Option<Ident>`.
/// - A lifetime is not an identifier. Use `syn::Lifetime` instead.
///
/// An identifier constructed with `Ident::new` is permitted to be a Rust
/// keyword, though parsing one through its [`Parse`] implementation rejects
/// Rust keywords. Use `input.call(Ident::parse_any)` when parsing to match the
/// behaviour of `Ident::new`.
///
/// [`Parse`]: https://docs.rs/syn/1.0/syn/parse/trait.Parse.html
///
/// # Examples
///
/// A new ident can be created from a string using the `Ident::new` function.
/// A span must be provided explicitly which governs the name resolution
/// behavior of the resulting identifier.
///
/// ```
/// use proc_macro2::{Ident, Span};
///
/// fn main() {
///     let call_ident = Ident::new("calligraphy", Span::call_site());
///
///     println!("{}", call_ident);
/// }
/// ```
///
/// An ident can be interpolated into a token stream using the `quote!` macro.
///
/// ```
/// use proc_macro2::{Ident, Span};
/// use quote::quote;
///
/// fn main() {
///     let ident = Ident::new("demo", Span::call_site());
///
///     // Create a variable binding whose name is this ident.
///     let expanded = quote! { let #ident = 10; };
///
///     // Create a variable binding with a slightly different name.
///     let temp_ident = Ident::new(&format!("new_{}", ident), Span::call_site());
///     let expanded = quote! { let #temp_ident = 10; };
/// }
/// ```
///
/// A string representation of the ident is available through the `to_string()`
/// method.
///
/// ```
/// # use proc_macro2::{Ident, Span};
/// #
/// # let ident = Ident::new("another_identifier", Span::call_site());
/// #
/// // Examine the ident as a string.
/// let ident_string = ident.to_string();
/// if ident_string.len() > 60 {
///     println!("Very long identifier: {}", ident_string)
/// }
/// ```
#[derive(Clone)]
pub struct Ident {
    inner: imp::Ident,
    _marker: Marker,
}
impl Ident {
    fn _new(inner: imp::Ident) -> Ident {
        Ident { inner, _marker: Marker }
    }
    /// Creates a new `Ident` with the given `string` as well as the specified
    /// `span`.
    ///
    /// The `string` argument must be a valid identifier permitted by the
    /// language, otherwise the function will panic.
    ///
    /// Note that `span`, currently in rustc, configures the hygiene information
    /// for this identifier.
    ///
    /// As of this time `Span::call_site()` explicitly opts-in to "call-site"
    /// hygiene meaning that identifiers created with this span will be resolved
    /// as if they were written directly at the location of the macro call, and
    /// other code at the macro call site will be able to refer to them as well.
    ///
    /// Later spans like `Span::def_site()` will allow to opt-in to
    /// "definition-site" hygiene meaning that identifiers created with this
    /// span will be resolved at the location of the macro definition and other
    /// code at the macro call site will not be able to refer to them.
    ///
    /// Due to the current importance of hygiene this constructor, unlike other
    /// tokens, requires a `Span` to be specified at construction.
    ///
    /// # Panics
    ///
    /// Panics if the input string is neither a keyword nor a legal variable
    /// name. If you are not sure whether the string contains an identifier and
    /// need to handle an error case, use
    /// <a href="https://docs.rs/syn/1.0/syn/fn.parse_str.html"><code
    ///   style="padding-right:0;">syn::parse_str</code></a><code
    ///   style="padding-left:0;">::&lt;Ident&gt;</code>
    /// rather than `Ident::new`.
    pub fn new(string: &str, span: Span) -> Ident {
        Ident::_new(imp::Ident::new(string, span.inner))
    }
    /// Same as `Ident::new`, but creates a raw identifier (`r#ident`).
    ///
    /// This method is semver exempt and not exposed by default.
    #[cfg(procmacro2_semver_exempt)]
    pub fn new_raw(string: &str, span: Span) -> Ident {
        Ident::_new_raw(string, span)
    }
    fn _new_raw(string: &str, span: Span) -> Ident {
        Ident::_new(imp::Ident::new_raw(string, span.inner))
    }
    /// Returns the span of this `Ident`.
    pub fn span(&self) -> Span {
        Span::_new(self.inner.span())
    }
    /// Configures the span of this `Ident`, possibly changing its hygiene
    /// context.
    pub fn set_span(&mut self, span: Span) {
        self.inner.set_span(span.inner);
    }
}
impl PartialEq for Ident {
    fn eq(&self, other: &Ident) -> bool {
        self.inner == other.inner
    }
}
impl<T> PartialEq<T> for Ident
where
    T: ?Sized + AsRef<str>,
{
    fn eq(&self, other: &T) -> bool {
        self.inner == other
    }
}
impl Eq for Ident {}
impl PartialOrd for Ident {
    fn partial_cmp(&self, other: &Ident) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Ident {
    fn cmp(&self, other: &Ident) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}
impl Hash for Ident {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.to_string().hash(hasher)
    }
}
/// Prints the identifier as a string that should be losslessly convertible back
/// into the same identifier.
impl Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
impl Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
/// A literal string (`"hello"`), byte string (`b"hello"`), character (`'a'`),
/// byte character (`b'a'`), an integer or floating point number with or without
/// a suffix (`1`, `1u8`, `2.3`, `2.3f32`).
///
/// Boolean literals like `true` and `false` do not belong here, they are
/// `Ident`s.
#[derive(Clone)]
pub struct Literal {
    inner: imp::Literal,
    _marker: Marker,
}
macro_rules! suffixed_int_literals {
    ($($name:ident => $kind:ident,)*) => {
        $(#[doc = " Creates a new suffixed integer literal with the specified value."]
        #[doc = ""] #[doc =
        " This function will create an integer like `1u32` where the integer"] #[doc =
        " value specified is the first part of the token and the integral is"] #[doc =
        " also suffixed at the end. Literals created from negative numbers may"] #[doc =
        " not survive rountrips through `TokenStream` or strings and may be"] #[doc =
        " broken into two tokens (`-` and positive literal)."] #[doc = ""] #[doc =
        " Literals created through this method have the `Span::call_site()`"] #[doc =
        " span by default, which can be configured with the `set_span` method"] #[doc =
        " below."] pub fn $name (n : $kind) -> Literal {
        Literal::_new(imp::Literal::$name (n)) })*
    };
}
macro_rules! unsuffixed_int_literals {
    ($($name:ident => $kind:ident,)*) => {
        $(#[doc = " Creates a new unsuffixed integer literal with the specified value."]
        #[doc = ""] #[doc =
        " This function will create an integer like `1` where the integer"] #[doc =
        " value specified is the first part of the token. No suffix is"] #[doc =
        " specified on this token, meaning that invocations like"] #[doc =
        " `Literal::i8_unsuffixed(1)` are equivalent to"] #[doc =
        " `Literal::u32_unsuffixed(1)`. Literals created from negative numbers"] #[doc =
        " may not survive rountrips through `TokenStream` or strings and may"] #[doc =
        " be broken into two tokens (`-` and positive literal)."] #[doc = ""] #[doc =
        " Literals created through this method have the `Span::call_site()`"] #[doc =
        " span by default, which can be configured with the `set_span` method"] #[doc =
        " below."] pub fn $name (n : $kind) -> Literal {
        Literal::_new(imp::Literal::$name (n)) })*
    };
}
impl Literal {
    fn _new(inner: imp::Literal) -> Literal {
        Literal { inner, _marker: Marker }
    }
    fn _new_stable(inner: fallback::Literal) -> Literal {
        Literal {
            inner: inner.into(),
            _marker: Marker,
        }
    }
    suffixed_int_literals! {
        u8_suffixed => u8, u16_suffixed => u16, u32_suffixed => u32, u64_suffixed => u64,
        u128_suffixed => u128, usize_suffixed => usize, i8_suffixed => i8, i16_suffixed
        => i16, i32_suffixed => i32, i64_suffixed => i64, i128_suffixed => i128,
        isize_suffixed => isize,
    }
    unsuffixed_int_literals! {
        u8_unsuffixed => u8, u16_unsuffixed => u16, u32_unsuffixed => u32, u64_unsuffixed
        => u64, u128_unsuffixed => u128, usize_unsuffixed => usize, i8_unsuffixed => i8,
        i16_unsuffixed => i16, i32_unsuffixed => i32, i64_unsuffixed => i64,
        i128_unsuffixed => i128, isize_unsuffixed => isize,
    }
    /// Creates a new unsuffixed floating-point literal.
    ///
    /// This constructor is similar to those like `Literal::i8_unsuffixed` where
    /// the float's value is emitted directly into the token but no suffix is
    /// used, so it may be inferred to be a `f64` later in the compiler.
    /// Literals created from negative numbers may not survive rountrips through
    /// `TokenStream` or strings and may be broken into two tokens (`-` and
    /// positive literal).
    ///
    /// # Panics
    ///
    /// This function requires that the specified float is finite, for example
    /// if it is infinity or NaN this function will panic.
    pub fn f64_unsuffixed(f: f64) -> Literal {
        assert!(f.is_finite());
        Literal::_new(imp::Literal::f64_unsuffixed(f))
    }
    /// Creates a new suffixed floating-point literal.
    ///
    /// This constructor will create a literal like `1.0f64` where the value
    /// specified is the preceding part of the token and `f64` is the suffix of
    /// the token. This token will always be inferred to be an `f64` in the
    /// compiler. Literals created from negative numbers may not survive
    /// rountrips through `TokenStream` or strings and may be broken into two
    /// tokens (`-` and positive literal).
    ///
    /// # Panics
    ///
    /// This function requires that the specified float is finite, for example
    /// if it is infinity or NaN this function will panic.
    pub fn f64_suffixed(f: f64) -> Literal {
        assert!(f.is_finite());
        Literal::_new(imp::Literal::f64_suffixed(f))
    }
    /// Creates a new unsuffixed floating-point literal.
    ///
    /// This constructor is similar to those like `Literal::i8_unsuffixed` where
    /// the float's value is emitted directly into the token but no suffix is
    /// used, so it may be inferred to be a `f64` later in the compiler.
    /// Literals created from negative numbers may not survive rountrips through
    /// `TokenStream` or strings and may be broken into two tokens (`-` and
    /// positive literal).
    ///
    /// # Panics
    ///
    /// This function requires that the specified float is finite, for example
    /// if it is infinity or NaN this function will panic.
    pub fn f32_unsuffixed(f: f32) -> Literal {
        assert!(f.is_finite());
        Literal::_new(imp::Literal::f32_unsuffixed(f))
    }
    /// Creates a new suffixed floating-point literal.
    ///
    /// This constructor will create a literal like `1.0f32` where the value
    /// specified is the preceding part of the token and `f32` is the suffix of
    /// the token. This token will always be inferred to be an `f32` in the
    /// compiler. Literals created from negative numbers may not survive
    /// rountrips through `TokenStream` or strings and may be broken into two
    /// tokens (`-` and positive literal).
    ///
    /// # Panics
    ///
    /// This function requires that the specified float is finite, for example
    /// if it is infinity or NaN this function will panic.
    pub fn f32_suffixed(f: f32) -> Literal {
        assert!(f.is_finite());
        Literal::_new(imp::Literal::f32_suffixed(f))
    }
    /// String literal.
    pub fn string(string: &str) -> Literal {
        Literal::_new(imp::Literal::string(string))
    }
    /// Character literal.
    pub fn character(ch: char) -> Literal {
        Literal::_new(imp::Literal::character(ch))
    }
    /// Byte string literal.
    pub fn byte_string(s: &[u8]) -> Literal {
        Literal::_new(imp::Literal::byte_string(s))
    }
    /// Returns the span encompassing this literal.
    pub fn span(&self) -> Span {
        Span::_new(self.inner.span())
    }
    /// Configures the span associated for this literal.
    pub fn set_span(&mut self, span: Span) {
        self.inner.set_span(span.inner);
    }
    /// Returns a `Span` that is a subset of `self.span()` containing only
    /// the source bytes in range `range`. Returns `None` if the would-be
    /// trimmed span is outside the bounds of `self`.
    ///
    /// Warning: the underlying [`proc_macro::Literal::subspan`] method is
    /// nightly-only. When called from within a procedural macro not using a
    /// nightly compiler, this method will always return `None`.
    ///
    /// [`proc_macro::Literal::subspan`]: https://doc.rust-lang.org/proc_macro/struct.Literal.html#method.subspan
    pub fn subspan<R: RangeBounds<usize>>(&self, range: R) -> Option<Span> {
        self.inner.subspan(range).map(Span::_new)
    }
}
impl Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
impl Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
/// Public implementation details for the `TokenStream` type, such as iterators.
pub mod token_stream {
    use crate::marker::Marker;
    use crate::{imp, TokenTree};
    use std::fmt::{self, Debug};
    pub use crate::TokenStream;
    /// An iterator over `TokenStream`'s `TokenTree`s.
    ///
    /// The iteration is "shallow", e.g. the iterator doesn't recurse into
    /// delimited groups, and returns whole groups as token trees.
    #[derive(Clone)]
    pub struct IntoIter {
        inner: imp::TokenTreeIter,
        _marker: Marker,
    }
    impl Iterator for IntoIter {
        type Item = TokenTree;
        fn next(&mut self) -> Option<TokenTree> {
            self.inner.next()
        }
    }
    impl Debug for IntoIter {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            Debug::fmt(&self.inner, f)
        }
    }
    impl IntoIterator for TokenStream {
        type Item = TokenTree;
        type IntoIter = IntoIter;
        fn into_iter(self) -> IntoIter {
            IntoIter {
                inner: self.inner.into_iter(),
                _marker: Marker,
            }
        }
    }
}
#[cfg(test)]
mod tests_llm_16_95 {
    use super::*;
    use crate::*;
    #[test]
    fn test_delimiter() {
        let _rug_st_tests_llm_16_95_rrrruuuugggg_test_delimiter = 0;
        let group = Group::new(Delimiter::Parenthesis, TokenStream::new());
        debug_assert_eq!(group.delimiter(), Delimiter::Parenthesis);
        let group = Group::new(Delimiter::Brace, TokenStream::new());
        debug_assert_eq!(group.delimiter(), Delimiter::Brace);
        let group = Group::new(Delimiter::Bracket, TokenStream::new());
        debug_assert_eq!(group.delimiter(), Delimiter::Bracket);
        let group = Group::new(Delimiter::None, TokenStream::new());
        debug_assert_eq!(group.delimiter(), Delimiter::None);
        let _rug_ed_tests_llm_16_95_rrrruuuugggg_test_delimiter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_102 {
    use crate::Span;
    #[test]
    fn test_span_close() {
        let _rug_st_tests_llm_16_102_rrrruuuugggg_test_span_close = 0;
        let span = Span::call_site();
        let group = crate::Group::new(
            crate::Delimiter::Parenthesis,
            crate::TokenStream::new(),
        );
        let span_close = group.span_close();
        let _rug_ed_tests_llm_16_102_rrrruuuugggg_test_span_close = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_105 {
    use super::*;
    use crate::*;
    #[test]
    fn test_stream() {
        let _rug_st_tests_llm_16_105_rrrruuuugggg_test_stream = 0;
        let group = Group::new(Delimiter::Parenthesis, TokenStream::new());
        let token_stream = group.stream();
        debug_assert!(token_stream.is_empty());
        let _rug_ed_tests_llm_16_105_rrrruuuugggg_test_stream = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_123_llm_16_122 {
    use super::*;
    use crate::*;
    use crate::Literal;
    #[test]
    fn test_character() {
        let _rug_st_tests_llm_16_123_llm_16_122_rrrruuuugggg_test_character = 0;
        let rug_fuzz_0 = 'a';
        let ch = rug_fuzz_0;
        let literal = Literal::character(ch);
        debug_assert_eq!(literal.to_string(), "'a'");
        let _rug_ed_tests_llm_16_123_llm_16_122_rrrruuuugggg_test_character = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_127_llm_16_126 {
    use super::*;
    use crate::*;
    #[test]
    fn test_f32_unsuffixed() {
        let _rug_st_tests_llm_16_127_llm_16_126_rrrruuuugggg_test_f32_unsuffixed = 0;
        let rug_fuzz_0 = 3.14;
        let f: f32 = rug_fuzz_0;
        let result = Literal::f32_unsuffixed(f);
        let _ = result.clone();
        let _rug_ed_tests_llm_16_127_llm_16_126_rrrruuuugggg_test_f32_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_134 {
    use crate::Literal;
    #[test]
    fn test_i128_unsuffixed() {
        let _rug_st_tests_llm_16_134_rrrruuuugggg_test_i128_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let literal = Literal::i128_unsuffixed(rug_fuzz_0);
        debug_assert_eq!(literal.to_string(), "42");
        let _rug_ed_tests_llm_16_134_rrrruuuugggg_test_i128_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_139 {
    use super::*;
    use crate::*;
    #[test]
    fn test_i32_suffixed() {
        let _rug_st_tests_llm_16_139_rrrruuuugggg_test_i32_suffixed = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 42;
        let expected = Literal::_new(imp::Literal::i32_suffixed(rug_fuzz_0));
        let actual = Literal::i32_suffixed(rug_fuzz_1);
        debug_assert_eq!(format!("{:?}", expected), format!("{:?}", actual));
        debug_assert_eq!(format!("{}", expected), format!("{}", actual));
        let _rug_ed_tests_llm_16_139_rrrruuuugggg_test_i32_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_141_llm_16_140 {
    use super::*;
    use crate::*;
    #[test]
    fn test_i32_unsuffixed() {
        let _rug_st_tests_llm_16_141_llm_16_140_rrrruuuugggg_test_i32_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let result = Literal::i32_unsuffixed(rug_fuzz_0);
        debug_assert_eq!(
            format!("{:?}", result), "Literal::i32_unsuffixed(42)".to_string()
        );
        debug_assert_eq!(format!("{}", result), "42".to_string());
        let _rug_ed_tests_llm_16_141_llm_16_140_rrrruuuugggg_test_i32_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_145_llm_16_144 {
    use proc_macro::Literal;
    #[cfg(span_locations)]
    use proc_macro::Span;
    #[cfg(not(span_locations))]
    use proc_macro::LineColumn;
    #[test]
    fn test_i64_unsuffixed() {
        let _rug_st_tests_llm_16_145_llm_16_144_rrrruuuugggg_test_i64_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let result = Literal::i64_unsuffixed(rug_fuzz_0);
        debug_assert_eq!(result.to_string(), "42");
        let _rug_ed_tests_llm_16_145_llm_16_144_rrrruuuugggg_test_i64_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_157 {
    use proc_macro::Literal;
    use proc_macro::Span;
    use proc_macro::TokenStream;
    #[test]
    fn test_literal_span() {
        let _rug_st_tests_llm_16_157_rrrruuuugggg_test_literal_span = 0;
        let rug_fuzz_0 = "hello";
        let lit: Literal = Literal::string(rug_fuzz_0);
        let span: Span = lit.span();
        let _rug_ed_tests_llm_16_157_rrrruuuugggg_test_literal_span = 0;
    }
    #[test]
    fn test_literal_set_span() {
        let _rug_st_tests_llm_16_157_rrrruuuugggg_test_literal_set_span = 0;
        let rug_fuzz_0 = "hello";
        let lit: Literal = Literal::string(rug_fuzz_0);
        let span: Span = lit.span();
        let _rug_ed_tests_llm_16_157_rrrruuuugggg_test_literal_set_span = 0;
    }
    #[test]
    fn test_literal_subspan() {
        let _rug_st_tests_llm_16_157_rrrruuuugggg_test_literal_subspan = 0;
        let rug_fuzz_0 = "hello";
        let lit: Literal = Literal::string(rug_fuzz_0);
        let span: Span = lit.span();
        let _rug_ed_tests_llm_16_157_rrrruuuugggg_test_literal_subspan = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_158 {
    use crate::Literal;
    #[test]
    fn test_literal_string() {
        let _rug_st_tests_llm_16_158_rrrruuuugggg_test_literal_string = 0;
        let rug_fuzz_0 = "Hello, World!";
        let text = rug_fuzz_0;
        let literal = Literal::string(text);
        debug_assert_eq!(literal.to_string(), text);
        let _rug_ed_tests_llm_16_158_rrrruuuugggg_test_literal_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_168_llm_16_167 {
    use super::*;
    use crate::*;
    use crate::Literal;
    #[test]
    fn test_u16_unsuffixed() {
        let _rug_st_tests_llm_16_168_llm_16_167_rrrruuuugggg_test_u16_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let literal = Literal::u16_unsuffixed(rug_fuzz_0);
        debug_assert_eq!(literal.to_string(), "42");
        let _rug_ed_tests_llm_16_168_llm_16_167_rrrruuuugggg_test_u16_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_170 {
    use super::*;
    use crate::*;
    use proc_macro::Literal as PMLiteral;
    #[test]
    fn test_u32_suffixed() {
        let _rug_st_tests_llm_16_170_rrrruuuugggg_test_u32_suffixed = 0;
        let rug_fuzz_0 = 10;
        let n: u32 = rug_fuzz_0;
        let expected = PMLiteral::u32_suffixed(n);
        let actual = Literal::u32_suffixed(n);
        debug_assert_eq!(expected.to_string(), actual.to_string());
        let _rug_ed_tests_llm_16_170_rrrruuuugggg_test_u32_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_182_llm_16_181 {
    use super::*;
    use crate::*;
    use proc_macro::Literal;
    #[test]
    fn test_usize_suffixed() {
        let _rug_st_tests_llm_16_182_llm_16_181_rrrruuuugggg_test_usize_suffixed = 0;
        let rug_fuzz_0 = 42usize;
        let value = rug_fuzz_0;
        let result = Literal::usize_suffixed(value);
        debug_assert_eq!(result.to_string(), "42usize");
        let _rug_ed_tests_llm_16_182_llm_16_181_rrrruuuugggg_test_usize_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_184 {
    use super::*;
    use crate::*;
    #[test]
    fn test_usize_unsuffixed() {
        let _rug_st_tests_llm_16_184_rrrruuuugggg_test_usize_unsuffixed = 0;
        let rug_fuzz_0 = 10;
        let n: usize = rug_fuzz_0;
        let expected = Literal::_new(imp::Literal::usize_unsuffixed(n));
        let actual = Literal::usize_unsuffixed(n);
        debug_assert_eq!(format!("{:?}", actual), format!("{:?}", expected));
        let _rug_ed_tests_llm_16_184_rrrruuuugggg_test_usize_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_186 {
    use proc_macro::Punct;
    #[test]
    fn test_as_char() {
        let _rug_st_tests_llm_16_186_rrrruuuugggg_test_as_char = 0;
        let rug_fuzz_0 = '+';
        let punct = Punct::new(rug_fuzz_0, proc_macro::Spacing::Alone);
        debug_assert_eq!(punct.as_char(), '+');
        let _rug_ed_tests_llm_16_186_rrrruuuugggg_test_as_char = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_191 {
    use super::*;
    use crate::*;
    #[test]
    fn test_spacing() {
        let _rug_st_tests_llm_16_191_rrrruuuugggg_test_spacing = 0;
        let rug_fuzz_0 = '+';
        let punct = Punct::new(rug_fuzz_0, Spacing::Alone);
        debug_assert_eq!(punct.spacing(), Spacing::Alone);
        let _rug_ed_tests_llm_16_191_rrrruuuugggg_test_spacing = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_194 {
    use super::*;
    use crate::*;
    #[test]
    fn test__new() {
        let _rug_st_tests_llm_16_194_rrrruuuugggg_test__new = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let inner = imp::Span::Fallback(fallback::Span {
            #[cfg(span_locations)]
            lo: rug_fuzz_0,
            #[cfg(span_locations)]
            hi: rug_fuzz_1,
        });
        let span = Span::_new(inner);
        let _rug_ed_tests_llm_16_194_rrrruuuugggg_test__new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_196 {
    #[test]
    fn test_new_stable() {
        let _rug_st_tests_llm_16_196_rrrruuuugggg_test_new_stable = 0;
        let inner = crate::fallback::Span {};
        let span = crate::Span::_new_stable(inner);
        let _rug_ed_tests_llm_16_196_rrrruuuugggg_test_new_stable = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_197 {
    use super::*;
    use crate::*;
    #[test]
    fn test_call_site() {
        let _rug_st_tests_llm_16_197_rrrruuuugggg_test_call_site = 0;
        let span = Span::call_site();
        let _rug_ed_tests_llm_16_197_rrrruuuugggg_test_call_site = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_205_llm_16_204 {
    #[test]
    fn test_resolved_at() {
        let _rug_st_tests_llm_16_205_llm_16_204_rrrruuuugggg_test_resolved_at = 0;
        use proc_macro::Span;
        let span1 = Span::call_site();
        let span2 = Span::call_site();
        let resolved_span = span1.resolved_at(span2);
        let _rug_ed_tests_llm_16_205_llm_16_204_rrrruuuugggg_test_resolved_at = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_210 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_210_rrrruuuugggg_test_new = 0;
        let token_stream = TokenStream::new();
        debug_assert_eq!(token_stream.is_empty(), true);
        let _rug_ed_tests_llm_16_210_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_215 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_215_rrrruuuugggg_test_new = 0;
        let token_stream = TokenStream::new();
        debug_assert!(token_stream.is_empty());
        debug_assert_eq!(token_stream.to_string(), "");
        let _rug_ed_tests_llm_16_215_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::fallback;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_159_rrrruuuugggg_test_rug = 0;
        let mut p0 = fallback::TokenStream::new();
        <TokenStream>::_new_stable(p0);
        let _rug_ed_tests_rug_159_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::TokenStream;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_160_rrrruuuugggg_test_rug = 0;
        let mut p0: TokenStream = TokenStream::new();
        <TokenStream>::is_empty(&p0);
        let _rug_ed_tests_rug_160_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::TokenStream;
    use std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_161_rrrruuuugggg_test_rug = 0;
        TokenStream::default();
        let _rug_ed_tests_rug_161_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_rug_162_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "test string";
        let p0: &str = rug_fuzz_0;
        TokenStream::from_str(p0);
        let _rug_ed_tests_rug_162_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_163 {
    use super::*;
    use crate::TokenStream;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_163_rrrruuuugggg_test_from = 0;
        let inner = proc_macro::TokenStream::new();
        let p0: TokenStream = inner.into();
        let _result: TokenStream = TokenStream::from(p0);
        let _rug_ed_tests_rug_163_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_164_prepare {
    use crate::TokenStream;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_164_prepare_rrrruuuugggg_sample = 0;
        let mut v12: TokenStream = TokenStream::new();
        let _rug_ed_tests_rug_164_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_164 {
    use super::*;
    use crate::TokenStream;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_164_rrrruuuugggg_test_from = 0;
        let mut p0: TokenStream = TokenStream::new();
        <proc_macro::TokenStream>::from(p0);
        let _rug_ed_tests_rug_164_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_167 {
    use super::*;
    use crate::TokenStream;
    #[test]
    fn test_extend() {
        let _rug_st_tests_rug_167_rrrruuuugggg_test_extend = 0;
        let mut p0: TokenStream = TokenStream::new();
        let mut p1: TokenStream = TokenStream::new();
        p0.extend(p1);
        let _rug_ed_tests_rug_167_rrrruuuugggg_test_extend = 0;
    }
}
#[cfg(test)]
mod tests_rug_170 {
    use super::*;
    use crate::Span;
    #[test]
    fn test_mixed_site() {
        let _rug_st_tests_rug_170_rrrruuuugggg_test_mixed_site = 0;
        Span::mixed_site();
        let _rug_ed_tests_rug_170_rrrruuuugggg_test_mixed_site = 0;
    }
}
#[cfg(test)]
mod tests_rug_171 {
    use super::*;
    use crate::Span;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_171_rrrruuuugggg_test_rug = 0;
        let mut p0: Span = Span::call_site();
        let mut p1: Span = Span::call_site();
        p0.located_at(p1);
        let _rug_ed_tests_rug_171_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_172 {
    use super::*;
    use crate::Span;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_172_rrrruuuugggg_test_rug = 0;
        let mut p0: Span = Span::call_site();
        <Span>::unwrap(p0);
        let _rug_ed_tests_rug_172_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::Span;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_173_rrrruuuugggg_test_rug = 0;
        let mut p0: Span = Span::call_site();
        Span::unstable(p0);
        let _rug_ed_tests_rug_173_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_174 {
    use super::*;
    use crate::{Span, TokenStream};
    #[test]
    fn test_join() {
        let _rug_st_tests_rug_174_rrrruuuugggg_test_join = 0;
        let mut p0: Span = Span::call_site();
        let mut p1: Span = Span::call_site();
        p0.join(p1);
        let _rug_ed_tests_rug_174_rrrruuuugggg_test_join = 0;
    }
}
#[cfg(test)]
mod tests_rug_175 {
    use super::*;
    use crate::{Span, Literal, TokenTree};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_175_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello, World!";
        let mut p0 = TokenTree::Literal(Literal::string(rug_fuzz_0));
        TokenTree::span(&p0);
        let _rug_ed_tests_rug_175_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::{Literal, Span, TokenTree};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_176_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello, World!";
        let mut p0 = TokenTree::Literal(Literal::string(rug_fuzz_0));
        let mut p1: Span = Span::call_site();
        TokenTree::set_span(&mut p0, p1);
        let _rug_ed_tests_rug_176_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_177 {
    use super::*;
    use crate::TokenStream;
    use crate::Delimiter;
    use crate::TokenTree;
    use crate::Group;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_177_rrrruuuugggg_test_rug = 0;
        let p0 = Group::new(Delimiter::Parenthesis, TokenStream::new());
        let _ = <TokenTree as std::convert::From<Group>>::from(p0);
        let _rug_ed_tests_rug_177_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::{Ident, Span};
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_178_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "sample_ident";
        let p0: Ident = Ident::new(rug_fuzz_0, Span::call_site());
        let result: TokenTree = <TokenTree as std::convert::From<Ident>>::from(p0);
        let _rug_ed_tests_rug_178_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use crate::{Punct, Spacing, TokenTree};
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_179_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = '+';
        let mut p0 = Punct::new(rug_fuzz_0, Spacing::Alone);
        let result = <TokenTree as std::convert::From<Punct>>::from(p0);
        let _rug_ed_tests_rug_179_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use crate::{Delimiter, TokenStream};
    #[test]
    fn test_group_new() {
        let _rug_st_tests_rug_183_rrrruuuugggg_test_group_new = 0;
        let mut p0 = Delimiter::Parenthesis;
        let mut p1: TokenStream = TokenStream::new();
        Group::new(p0, p1);
        let _rug_ed_tests_rug_183_rrrruuuugggg_test_group_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use crate::{Group, Delimiter, TokenStream, TokenTree};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_184_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_184_prepare {
            use super::*;
            use crate::{Group, Delimiter, TokenStream};
            #[test]
            fn sample() {
                let _rug_st_tests_rug_184_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 0;
                let _rug_st_tests_rug_184_rrrruuuugggg_sample = rug_fuzz_0;
                let v28 = Group::new(Delimiter::Parenthesis, TokenStream::new());
                let _rug_ed_tests_rug_184_rrrruuuugggg_sample = rug_fuzz_1;
                let _rug_ed_tests_rug_184_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0 = Group::new(Delimiter::Parenthesis, TokenStream::new());
        Group::span(&p0);
        let _rug_ed_tests_rug_184_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::{Span, Group, Delimiter};
    #[test]
    fn test_span_open() {
        let _rug_st_tests_rug_185_rrrruuuugggg_test_span_open = 0;
        let inner = TokenStream::new();
        let p0 = Group::new(Delimiter::Parenthesis, inner);
        let result = p0.span_open();
        let _rug_ed_tests_rug_185_rrrruuuugggg_test_span_open = 0;
    }
}
#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use crate::Group;
    use crate::Span;
    use crate::Delimiter;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_186_rrrruuuugggg_test_rug = 0;
        let mut p0 = Group::new(Delimiter::Parenthesis, TokenStream::new());
        let mut p1: Span = Span::call_site();
        p0.set_span(p1);
        let _rug_ed_tests_rug_186_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::{Span, Spacing};
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_187_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = '!';
        let mut p0: char = rug_fuzz_0;
        let mut p1: Spacing = Spacing::Joint;
        let _ = Punct::new(p0, p1);
        let _rug_ed_tests_rug_187_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::{Span, Punct, Spacing};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_188_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = '+';
        let mut p0 = Punct::new(rug_fuzz_0, Spacing::Alone);
        <Punct>::span(&p0);
        let _rug_ed_tests_rug_188_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::Span;
    use crate::{Punct, Spacing};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_189_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = '+';
        let mut p0 = Punct::new(rug_fuzz_0, Spacing::Alone);
        let mut p1: Span = Span::call_site();
        Punct::set_span(&mut p0, p1);
        let _rug_ed_tests_rug_189_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_191 {
    use super::*;
    use crate::Span;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_191_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "identifier";
        let p0: &str = rug_fuzz_0;
        let p1: Span = Span::call_site();
        Ident::new(p0, p1);
        let _rug_ed_tests_rug_191_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::Span;
    #[test]
    fn test_new_raw() {
        let _rug_st_tests_rug_192_rrrruuuugggg_test_new_raw = 0;
        let rug_fuzz_0 = "sample_data";
        let p0: &str = rug_fuzz_0;
        let p1: Span = Span::call_site();
        Ident::_new_raw(p0, p1);
        let _rug_ed_tests_rug_192_rrrruuuugggg_test_new_raw = 0;
    }
}
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::{Ident, Span};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_193_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_ident";
        let mut p0 = Ident::new(rug_fuzz_0, Span::call_site());
        <Ident>::span(&p0);
        let _rug_ed_tests_rug_193_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_194 {
    use super::*;
    use crate::{Ident, Span};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_194_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_ident";
        let mut p0 = Ident::new(rug_fuzz_0, Span::call_site());
        let mut p1: Span = Span::call_site();
        p0.set_span(p1);
        let _rug_ed_tests_rug_194_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_195 {
    use super::*;
    use crate::{Ident, Span};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_195_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_ident_1";
        let rug_fuzz_1 = "sample_ident_2";
        let p0 = Ident::new(rug_fuzz_0, Span::call_site());
        let p1 = Ident::new(rug_fuzz_1, Span::call_site());
        <Ident as std::cmp::PartialEq>::eq(&p0, &p1);
        let _rug_ed_tests_rug_195_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_198 {
    use super::*;
    use crate::{Ident, Span};
    use std::cmp::Ord;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_198_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_ident_1";
        let rug_fuzz_1 = "sample_ident_2";
        let p0 = Ident::new(rug_fuzz_0, Span::call_site());
        let p1 = Ident::new(rug_fuzz_1, Span::call_site());
        <Ident as std::cmp::Ord>::cmp(&p0, &p1);
        let _rug_ed_tests_rug_198_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_199 {
    use std::hash::Hash;
    use crate::Ident;
    #[allow(unused_imports)]
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_199_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_ident";
        let mut v29 = Ident::new(rug_fuzz_0, Span::call_site());
        let mut v34: std::hash::SipHasher = std::hash::SipHasher::new();
        v29.hash(&mut v34);
        let _rug_ed_tests_rug_199_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_200 {
    use super::*;
    use crate::Literal;
    use crate::imp;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_200_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = imp::Literal::u8_suffixed(rug_fuzz_0);
        <Literal>::_new(p0);
        let _rug_ed_tests_rug_200_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_201 {
    use super::*;
    use crate::fallback::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_201_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample data";
        let mut p0 = Literal::_new(rug_fuzz_0.to_string());
        crate::Literal::_new_stable(p0);
        let _rug_ed_tests_rug_201_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_202 {
    use super::*;
    use crate::Span;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_202_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        Literal::u8_suffixed(p0);
        let _rug_ed_tests_rug_202_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_203 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_203_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u16 = rug_fuzz_0;
        Literal::u16_suffixed(p0);
        let _rug_ed_tests_rug_203_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_204 {
    use super::*;
    use crate::{Literal, Span};
    #[test]
    fn test_u64_suffixed() {
        let _rug_st_tests_rug_204_rrrruuuugggg_test_u64_suffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: u64 = rug_fuzz_0;
        Literal::u64_suffixed(p0);
        let _rug_ed_tests_rug_204_rrrruuuugggg_test_u64_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_205 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_205_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234567890;
        let p0: u128 = rug_fuzz_0;
        Literal::u128_suffixed(p0);
        let _rug_ed_tests_rug_205_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_206 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_i8_suffixed() {
        let _rug_st_tests_rug_206_rrrruuuugggg_test_i8_suffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        Literal::i8_suffixed(p0);
        let _rug_ed_tests_rug_206_rrrruuuugggg_test_i8_suffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_207_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        <Literal>::i16_suffixed(p0);
        let _rug_ed_tests_rug_207_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_208 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_208_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: i64 = rug_fuzz_0;
        Literal::i64_suffixed(p0);
        let _rug_ed_tests_rug_208_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_209 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_209_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let p0: i128 = rug_fuzz_0;
        Literal::i128_suffixed(p0);
        let _rug_ed_tests_rug_209_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_210 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_210_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: isize = rug_fuzz_0;
        Literal::isize_suffixed(p0);
        let _rug_ed_tests_rug_210_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_211 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_211_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u8 = rug_fuzz_0;
        Literal::u8_unsuffixed(p0);
        let _rug_ed_tests_rug_211_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_212 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_212_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u32 = rug_fuzz_0;
        Literal::u32_unsuffixed(p0);
        let _rug_ed_tests_rug_212_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_213 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_213_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0: u64 = rug_fuzz_0;
        Literal::u64_unsuffixed(p0);
        let _rug_ed_tests_rug_213_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_214 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_u128_unsuffixed() {
        let _rug_st_tests_rug_214_rrrruuuugggg_test_u128_unsuffixed = 0;
        let rug_fuzz_0 = 123456789;
        let p0: u128 = rug_fuzz_0;
        Literal::u128_unsuffixed(p0);
        let _rug_ed_tests_rug_214_rrrruuuugggg_test_u128_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_215 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_i8_unsuffixed() {
        let _rug_st_tests_rug_215_rrrruuuugggg_test_i8_unsuffixed = 0;
        let rug_fuzz_0 = 42;
        let p0: i8 = rug_fuzz_0;
        Literal::i8_unsuffixed(p0);
        let _rug_ed_tests_rug_215_rrrruuuugggg_test_i8_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_216 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_216_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let p0: i16 = rug_fuzz_0;
        Literal::i16_unsuffixed(p0);
        let _rug_ed_tests_rug_216_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_217 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_217_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = rug_fuzz_0;
        Literal::isize_unsuffixed(p0);
        let _rug_ed_tests_rug_217_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_f64_unsuffixed() {
        let _rug_st_tests_rug_218_rrrruuuugggg_test_f64_unsuffixed = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f64 = rug_fuzz_0;
        Literal::f64_unsuffixed(p0);
        let _rug_ed_tests_rug_218_rrrruuuugggg_test_f64_unsuffixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_219_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let p0: f64 = rug_fuzz_0;
        Literal::f64_suffixed(p0);
        let _rug_ed_tests_rug_219_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_220 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_220_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: f32 = rug_fuzz_0;
        Literal::f32_suffixed(p0);
        let _rug_ed_tests_rug_220_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_221 {
    use super::*;
    use crate::Literal;
    #[test]
    fn test_byte_string() {
        let _rug_st_tests_rug_221_rrrruuuugggg_test_byte_string = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let p0: &[u8] = rug_fuzz_0;
        Literal::byte_string(p0);
        let _rug_ed_tests_rug_221_rrrruuuugggg_test_byte_string = 0;
    }
}
#[cfg(test)]
mod tests_rug_222 {
    use super::*;
    use crate::{Literal, Span};
    #[test]
    fn test_set_span() {
        let _rug_st_tests_rug_222_rrrruuuugggg_test_set_span = 0;
        let rug_fuzz_0 = 3.14;
        let mut p0: Literal = Literal::f64_suffixed(rug_fuzz_0);
        let mut p1: Span = Span::call_site();
        p0.set_span(p1);
        let _rug_ed_tests_rug_222_rrrruuuugggg_test_set_span = 0;
    }
}
#[cfg(test)]
mod tests_rug_223 {
    use super::*;
    use crate::Literal;
    use std::ops::RangeToInclusive;
    #[test]
    fn test_subspan() {
        let _rug_st_tests_rug_223_rrrruuuugggg_test_subspan = 0;
        let rug_fuzz_0 = 3.14;
        let rug_fuzz_1 = 5;
        let mut literal: Literal = Literal::f64_suffixed(rug_fuzz_0);
        let mut range: RangeToInclusive<usize> = ..=rug_fuzz_1;
        let _ = literal.subspan(range);
        let _rug_ed_tests_rug_223_rrrruuuugggg_test_subspan = 0;
    }
}
#[cfg(test)]
mod tests_rug_224 {
    use super::*;
    use crate::token_stream::TokenStream;
    use std::iter::Iterator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_224_rrrruuuugggg_test_rug = 0;
        let mut p0 = TokenStream::new().into_iter();
        p0.next();
        let _rug_ed_tests_rug_224_rrrruuuugggg_test_rug = 0;
    }
}
