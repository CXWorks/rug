//! Primitives for styled text.
//!
//! A terminal UI is at its root a lot of strings. In order to make it accessible and stylish,
//! those strings may be associated to a set of styles. `tui` has three ways to represent them:
//! - A single line string where all graphemes have the same style is represented by a [`Span`].
//! - A single line string where each grapheme may have its own style is represented by [`Spans`].
//! - A multiple line string where each grapheme may have its own style is represented by a
//! [`Text`].
//!
//! These types form a hierarchy: [`Spans`] is a collection of [`Span`] and each line of [`Text`]
//! is a [`Spans`].
//!
//! Keep it mind that a lot of widgets will use those types to advertise what kind of string is
//! supported for their properties. Moreover, `tui` provides convenient `From` implementations so
//! that you can start by using simple `String` or `&str` and then promote them to the previous
//! primitives when you need additional styling capabilities.
//!
//! For example, for the [`crate::widgets::Block`] widget, all the following calls are valid to set
//! its `title` property (which is a [`Spans`] under the hood):
//!
//! ```rust
//! # use tui::widgets::Block;
//! # use tui::text::{Span, Spans};
//! # use tui::style::{Color, Style};
//! // A simple string with no styling.
//! // Converted to Spans(vec![
//! //   Span { content: Cow::Borrowed("My title"), style: Style { .. } }
//! // ])
//! let block = Block::default().title("My title");
//!
//! // A simple string with a unique style.
//! // Converted to Spans(vec![
//! //   Span { content: Cow::Borrowed("My title"), style: Style { fg: Some(Color::Yellow), .. }
//! // ])
//! let block = Block::default().title(
//!     Span::styled("My title", Style::default().fg(Color::Yellow))
//! );
//!
//! // A string with multiple styles.
//! // Converted to Spans(vec![
//! //   Span { content: Cow::Borrowed("My"), style: Style { fg: Some(Color::Yellow), .. } },
//! //   Span { content: Cow::Borrowed(" title"), .. }
//! // ])
//! let block = Block::default().title(vec![
//!     Span::styled("My", Style::default().fg(Color::Yellow)),
//!     Span::raw(" title"),
//! ]);
//! ```
use crate::style::Style;
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
/// A grapheme associated to a style.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledGrapheme<'a> {
    pub symbol: &'a str,
    pub style: Style,
}
/// A string where all graphemes have the same style.
#[derive(Debug, Clone, PartialEq)]
pub struct Span<'a> {
    pub content: Cow<'a, str>,
    pub style: Style,
}
impl<'a> Span<'a> {
    /// Create a span with no style.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::text::Span;
    /// Span::raw("My text");
    /// Span::raw(String::from("My text"));
    /// ```
    pub fn raw<T>(content: T) -> Span<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Span {
            content: content.into(),
            style: Style::default(),
        }
    }
    /// Create a span with a style.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tui::text::Span;
    /// # use tui::style::{Color, Modifier, Style};
    /// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
    /// Span::styled("My text", style);
    /// Span::styled(String::from("My text"), style);
    /// ```
    pub fn styled<T>(content: T, style: Style) -> Span<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Span {
            content: content.into(),
            style,
        }
    }
    /// Returns the width of the content held by this span.
    pub fn width(&self) -> usize {
        self.content.width()
    }
    /// Returns an iterator over the graphemes held by this span.
    ///
    /// `base_style` is the [`Style`] that will be patched with each grapheme [`Style`] to get
    /// the resulting [`Style`].
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::text::{Span, StyledGrapheme};
    /// # use tui::style::{Color, Modifier, Style};
    /// # use std::iter::Iterator;
    /// let style = Style::default().fg(Color::Yellow);
    /// let span = Span::styled("Text", style);
    /// let style = Style::default().fg(Color::Green).bg(Color::Black);
    /// let styled_graphemes = span.styled_graphemes(style);
    /// assert_eq!(
    ///     vec![
    ///         StyledGrapheme {
    ///             symbol: "T",
    ///             style: Style {
    ///                 fg: Some(Color::Yellow),
    ///                 bg: Some(Color::Black),
    ///                 add_modifier: Modifier::empty(),
    ///                 sub_modifier: Modifier::empty(),
    ///             },
    ///         },
    ///         StyledGrapheme {
    ///             symbol: "e",
    ///             style: Style {
    ///                 fg: Some(Color::Yellow),
    ///                 bg: Some(Color::Black),
    ///                 add_modifier: Modifier::empty(),
    ///                 sub_modifier: Modifier::empty(),
    ///             },
    ///         },
    ///         StyledGrapheme {
    ///             symbol: "x",
    ///             style: Style {
    ///                 fg: Some(Color::Yellow),
    ///                 bg: Some(Color::Black),
    ///                 add_modifier: Modifier::empty(),
    ///                 sub_modifier: Modifier::empty(),
    ///             },
    ///         },
    ///         StyledGrapheme {
    ///             symbol: "t",
    ///             style: Style {
    ///                 fg: Some(Color::Yellow),
    ///                 bg: Some(Color::Black),
    ///                 add_modifier: Modifier::empty(),
    ///                 sub_modifier: Modifier::empty(),
    ///             },
    ///         },
    ///     ],
    ///     styled_graphemes.collect::<Vec<StyledGrapheme>>()
    /// );
    /// ```
    pub fn styled_graphemes(
        &'a self,
        base_style: Style,
    ) -> impl Iterator<Item = StyledGrapheme<'a>> {
        UnicodeSegmentation::graphemes(self.content.as_ref(), true)
            .map(move |g| StyledGrapheme {
                symbol: g,
                style: base_style.patch(self.style),
            })
            .filter(|s| s.symbol != "\n")
    }
}
impl<'a> From<String> for Span<'a> {
    fn from(s: String) -> Span<'a> {
        Span::raw(s)
    }
}
impl<'a> From<&'a str> for Span<'a> {
    fn from(s: &'a str) -> Span<'a> {
        Span::raw(s)
    }
}
/// A string composed of clusters of graphemes, each with their own style.
#[derive(Debug, Clone, PartialEq)]
pub struct Spans<'a>(pub Vec<Span<'a>>);
impl<'a> Default for Spans<'a> {
    fn default() -> Spans<'a> {
        Spans(Vec::new())
    }
}
impl<'a> Spans<'a> {
    /// Returns the width of the underlying string.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::text::{Span, Spans};
    /// # use tui::style::{Color, Style};
    /// let spans = Spans::from(vec![
    ///     Span::styled("My", Style::default().fg(Color::Yellow)),
    ///     Span::raw(" text"),
    /// ]);
    /// assert_eq!(7, spans.width());
    /// ```
    pub fn width(&self) -> usize {
        self.0.iter().map(Span::width).sum()
    }
}
impl<'a> From<String> for Spans<'a> {
    fn from(s: String) -> Spans<'a> {
        Spans(vec![Span::from(s)])
    }
}
impl<'a> From<&'a str> for Spans<'a> {
    fn from(s: &'a str) -> Spans<'a> {
        Spans(vec![Span::from(s)])
    }
}
impl<'a> From<Vec<Span<'a>>> for Spans<'a> {
    fn from(spans: Vec<Span<'a>>) -> Spans<'a> {
        Spans(spans)
    }
}
impl<'a> From<Span<'a>> for Spans<'a> {
    fn from(span: Span<'a>) -> Spans<'a> {
        Spans(vec![span])
    }
}
impl<'a> From<Spans<'a>> for String {
    fn from(line: Spans<'a>) -> String {
        line.0
            .iter()
            .fold(
                String::new(),
                |mut acc, s| {
                    acc.push_str(s.content.as_ref());
                    acc
                },
            )
    }
}
/// A string split over multiple lines where each line is composed of several clusters, each with
/// their own style.
///
/// A [`Text`], like a [`Span`], can be constructed using one of the many `From` implementations
/// or via the [`Text::raw`] and [`Text::styled`] methods. Helpfully, [`Text`] also implements
/// [`core::iter::Extend`] which enables the concatenation of several [`Text`] blocks.
///
/// ```rust
/// # use tui::text::Text;
/// # use tui::style::{Color, Modifier, Style};
/// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
///
/// // An initial two lines of `Text` built from a `&str`
/// let mut text = Text::from("The first line\nThe second line");
/// assert_eq!(2, text.height());
///
/// // Adding two more unstyled lines
/// text.extend(Text::raw("These are two\nmore lines!"));
/// assert_eq!(4, text.height());
///
/// // Adding a final two styled lines
/// text.extend(Text::styled("Some more lines\nnow with more style!", style));
/// assert_eq!(6, text.height());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Text<'a> {
    pub lines: Vec<Spans<'a>>,
}
impl<'a> Default for Text<'a> {
    fn default() -> Text<'a> {
        Text { lines: Vec::new() }
    }
}
impl<'a> Text<'a> {
    /// Create some text (potentially multiple lines) with no style.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::text::Text;
    /// Text::raw("The first line\nThe second line");
    /// Text::raw(String::from("The first line\nThe second line"));
    /// ```
    pub fn raw<T>(content: T) -> Text<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Text {
            lines: match content.into() {
                Cow::Borrowed(s) => s.lines().map(Spans::from).collect(),
                Cow::Owned(s) => s.lines().map(|l| Spans::from(l.to_owned())).collect(),
            },
        }
    }
    /// Create some text (potentially multiple lines) with a style.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tui::text::Text;
    /// # use tui::style::{Color, Modifier, Style};
    /// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
    /// Text::styled("The first line\nThe second line", style);
    /// Text::styled(String::from("The first line\nThe second line"), style);
    /// ```
    pub fn styled<T>(content: T, style: Style) -> Text<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let mut text = Text::raw(content);
        text.patch_style(style);
        text
    }
    /// Returns the max width of all the lines.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use tui::text::Text;
    /// let text = Text::from("The first line\nThe second line");
    /// assert_eq!(15, text.width());
    /// ```
    pub fn width(&self) -> usize {
        self.lines.iter().map(Spans::width).max().unwrap_or_default()
    }
    /// Returns the height.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use tui::text::Text;
    /// let text = Text::from("The first line\nThe second line");
    /// assert_eq!(2, text.height());
    /// ```
    pub fn height(&self) -> usize {
        self.lines.len()
    }
    /// Apply a new style to existing text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tui::text::Text;
    /// # use tui::style::{Color, Modifier, Style};
    /// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
    /// let mut raw_text = Text::raw("The first line\nThe second line");
    /// let styled_text = Text::styled(String::from("The first line\nThe second line"), style);
    /// assert_ne!(raw_text, styled_text);
    ///
    /// raw_text.patch_style(style);
    /// assert_eq!(raw_text, styled_text);
    /// ```
    pub fn patch_style(&mut self, style: Style) {
        for line in &mut self.lines {
            for span in &mut line.0 {
                span.style = span.style.patch(style);
            }
        }
    }
}
impl<'a> From<String> for Text<'a> {
    fn from(s: String) -> Text<'a> {
        Text::raw(s)
    }
}
impl<'a> From<&'a str> for Text<'a> {
    fn from(s: &'a str) -> Text<'a> {
        Text::raw(s)
    }
}
impl<'a> From<Span<'a>> for Text<'a> {
    fn from(span: Span<'a>) -> Text<'a> {
        Text {
            lines: vec![Spans::from(span)],
        }
    }
}
impl<'a> From<Spans<'a>> for Text<'a> {
    fn from(spans: Spans<'a>) -> Text<'a> {
        Text { lines: vec![spans] }
    }
}
impl<'a> From<Vec<Spans<'a>>> for Text<'a> {
    fn from(lines: Vec<Spans<'a>>) -> Text<'a> {
        Text { lines }
    }
}
impl<'a> IntoIterator for Text<'a> {
    type Item = Spans<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}
impl<'a> Extend<Spans<'a>> for Text<'a> {
    fn extend<T: IntoIterator<Item = Spans<'a>>>(&mut self, iter: T) {
        self.lines.extend(iter);
    }
}
#[cfg(test)]
mod tests_llm_16_35_llm_16_34 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier};
    #[test]
    fn test_span_from_str() {
        let _rug_st_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_from_str = 0;
        let rug_fuzz_0 = "test";
        let s: &'static str = rug_fuzz_0;
        let span: Span<'static> = s.into();
        debug_assert_eq!(span.content, "test");
        debug_assert_eq!(span.style, Style::default());
        let _rug_ed_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_from_str = 0;
    }
    #[test]
    fn test_span_from_string() {
        let _rug_st_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_from_string = 0;
        let rug_fuzz_0 = "test";
        let s: String = String::from(rug_fuzz_0);
        let span: Span = s.into();
        debug_assert_eq!(span.content, "test");
        debug_assert_eq!(span.style, Style::default());
        let _rug_ed_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_from_string = 0;
    }
    #[test]
    fn test_span_raw() {
        let _rug_st_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_raw = 0;
        let rug_fuzz_0 = "test";
        let span: Span<'static> = Span::raw(rug_fuzz_0);
        debug_assert_eq!(span.content, "test");
        debug_assert_eq!(span.style, Style::default());
        let _rug_ed_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_raw = 0;
    }
    #[test]
    fn test_span_styled() {
        let _rug_st_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_styled = 0;
        let rug_fuzz_0 = "test";
        let style = Style::default().fg(Color::Blue);
        let span: Span<'static> = Span::styled(rug_fuzz_0, style);
        debug_assert_eq!(span.content, "test");
        debug_assert_eq!(span.style, style);
        let _rug_ed_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_styled = 0;
    }
    #[test]
    fn test_span_width() {
        let _rug_st_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_width = 0;
        let rug_fuzz_0 = "test";
        let span: Span<'static> = Span::raw(rug_fuzz_0);
        debug_assert_eq!(span.width(), 4);
        let _rug_ed_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_width = 0;
    }
    #[test]
    fn test_span_styled_graphemes() {
        let _rug_st_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_styled_graphemes = 0;
        let rug_fuzz_0 = "Text";
        let rug_fuzz_1 = "T";
        let style = Style::default().fg(Color::Yellow);
        let span: Span<'static> = Span::styled(rug_fuzz_0, style);
        let styled_graphemes = span
            .styled_graphemes(Style::default().fg(Color::Green).bg(Color::Black));
        let expected = vec![
            StyledGrapheme { symbol : rug_fuzz_1, style : Style { fg :
            Some(Color::Yellow), bg : Some(Color::Black), add_modifier :
            Modifier::empty(), sub_modifier : Modifier::empty(), }, }, StyledGrapheme {
            symbol : "e", style : Style { fg : Some(Color::Yellow), bg :
            Some(Color::Black), add_modifier : Modifier::empty(), sub_modifier :
            Modifier::empty(), }, }, StyledGrapheme { symbol : "x", style : Style { fg :
            Some(Color::Yellow), bg : Some(Color::Black), add_modifier :
            Modifier::empty(), sub_modifier : Modifier::empty(), }, }, StyledGrapheme {
            symbol : "t", style : Style { fg : Some(Color::Yellow), bg :
            Some(Color::Black), add_modifier : Modifier::empty(), sub_modifier :
            Modifier::empty(), }, }
        ];
        debug_assert_eq!(
            styled_graphemes.collect:: < Vec < StyledGrapheme > > (), expected
        );
        let _rug_ed_tests_llm_16_35_llm_16_34_rrrruuuugggg_test_span_styled_graphemes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_36 {
    use super::*;
    use crate::*;
    use crate::text::Span;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_36_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "Hello World";
        let s = String::from(rug_fuzz_0);
        let span = Span::from(s);
        debug_assert_eq!(span.content, "Hello World");
        debug_assert_eq!(span.style, Style::default());
        let _rug_ed_tests_llm_16_36_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_39 {
    use super::*;
    use crate::*;
    use crate::text::{Span, Spans};
    use crate::style::{Color, Style};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_39_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = "test";
        let expected_spans = Spans(vec![Span::from(String::from(rug_fuzz_0))]);
        let actual_spans: Spans = From::from(String::from(rug_fuzz_1));
        debug_assert_eq!(expected_spans, actual_spans);
        let _rug_ed_tests_llm_16_39_rrrruuuugggg_test_from = 0;
    }
    #[test]
    fn test_from_spans() {
        let _rug_st_tests_llm_16_39_rrrruuuugggg_test_from_spans = 0;
        let rug_fuzz_0 = "test";
        let expected_spans = Spans(vec![Span::from(String::from(rug_fuzz_0))]);
        let actual_spans: Spans = From::from(expected_spans.clone());
        debug_assert_eq!(expected_spans, actual_spans);
        let _rug_ed_tests_llm_16_39_rrrruuuugggg_test_from_spans = 0;
    }
    #[test]
    fn test_from_string() {
        let _rug_st_tests_llm_16_39_rrrruuuugggg_test_from_string = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = "test";
        let expected_spans = Spans(vec![Span::from(String::from(rug_fuzz_0))]);
        let actual_spans: Spans = From::from(rug_fuzz_1.to_string());
        debug_assert_eq!(expected_spans, actual_spans);
        let _rug_ed_tests_llm_16_39_rrrruuuugggg_test_from_string = 0;
    }
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_39_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = "test";
        let expected_spans = Spans(vec![Span::from(String::from(rug_fuzz_0))]);
        let actual_spans: Spans = From::from(rug_fuzz_1);
        debug_assert_eq!(expected_spans, actual_spans);
        let _rug_ed_tests_llm_16_39_rrrruuuugggg_test_from_str = 0;
    }
    #[test]
    fn test_from_span() {
        let _rug_st_tests_llm_16_39_rrrruuuugggg_test_from_span = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = "test";
        let expected_spans = Spans(vec![Span::from(String::from(rug_fuzz_0))]);
        let actual_spans: Spans = From::from(Span::from(rug_fuzz_1));
        debug_assert_eq!(expected_spans, actual_spans);
        let _rug_ed_tests_llm_16_39_rrrruuuugggg_test_from_span = 0;
    }
    #[test]
    fn test_from_vec_span() {
        let _rug_st_tests_llm_16_39_rrrruuuugggg_test_from_vec_span = 0;
        let rug_fuzz_0 = "test";
        let rug_fuzz_1 = "test";
        let expected_spans = Spans(vec![Span::from(String::from(rug_fuzz_0))]);
        let actual_spans: Spans = From::from(vec![Span::from(rug_fuzz_1)]);
        debug_assert_eq!(expected_spans, actual_spans);
        let _rug_ed_tests_llm_16_39_rrrruuuugggg_test_from_vec_span = 0;
    }
    #[test]
    fn test_width() {
        let _rug_st_tests_llm_16_39_rrrruuuugggg_test_width = 0;
        let rug_fuzz_0 = "My";
        let rug_fuzz_1 = 7;
        let spans = Spans::from(
            vec![
                Span::styled(rug_fuzz_0, Style::default().fg(Color::Yellow)),
                Span::raw(" text")
            ],
        );
        let expected_width = rug_fuzz_1;
        let actual_width = spans.width();
        debug_assert_eq!(expected_width, actual_width);
        let _rug_ed_tests_llm_16_39_rrrruuuugggg_test_width = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_43 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_43_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "test";
        let span = Span::raw(rug_fuzz_0);
        let result = Spans::from(span.clone());
        let expected = Spans(vec![span]);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_43_rrrruuuugggg_test_from = 0;
    }
    #[test]
    fn test_width() {
        let _rug_st_tests_llm_16_43_rrrruuuugggg_test_width = 0;
        let rug_fuzz_0 = "Hello";
        let rug_fuzz_1 = "World";
        let rug_fuzz_2 = 10;
        let span1 = Span::raw(rug_fuzz_0);
        let span2 = Span::raw(rug_fuzz_1);
        let spans = Spans(vec![span1, span2]);
        let result = spans.width();
        let expected = rug_fuzz_2;
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_43_rrrruuuugggg_test_width = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_44 {
    use super::*;
    use crate::*;
    use crate::text::{Span, Spans};
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_44_rrrruuuugggg_test_default = 0;
        let expected = Spans(Vec::new());
        let result = Spans::default();
        debug_assert_eq!(expected, result);
        let _rug_ed_tests_llm_16_44_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_47 {
    use std::convert::From;
    use std::string::String;
    use crate::text::{Text, Spans};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_47_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "Hello, world!";
        let s: String = rug_fuzz_0.to_string();
        let expected_text: Text = Text::raw(s.clone());
        let converted_text: Text = From::<String>::from(s);
        debug_assert_eq!(expected_text, converted_text);
        let _rug_ed_tests_llm_16_47_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_48 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_48_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "Hello";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let spans = vec![
            Spans::from(Span::from(rug_fuzz_0)), Spans::from(Span::from("world"))
        ];
        let text = Text::from(spans);
        debug_assert_eq!(text.lines.len(), 2);
        debug_assert_eq!(text.lines[rug_fuzz_1].0[rug_fuzz_2].content, "Hello");
        debug_assert_eq!(text.lines[rug_fuzz_3].0[rug_fuzz_4].content, "world");
        let _rug_ed_tests_llm_16_48_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_49 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier};
    #[test]
    fn test_from_function() {
        let _rug_st_tests_llm_16_49_rrrruuuugggg_test_from_function = 0;
        let rug_fuzz_0 = "Hello, World";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let span = Span::raw(rug_fuzz_0);
        let text = Text::from(span);
        debug_assert_eq!(text.lines.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_1].0.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_2].0[rug_fuzz_3].content, "Hello, World");
        let _rug_ed_tests_llm_16_49_rrrruuuugggg_test_from_function = 0;
    }
    #[test]
    fn test_raw_method() {
        let _rug_st_tests_llm_16_49_rrrruuuugggg_test_raw_method = 0;
        let rug_fuzz_0 = "Hello, World";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let text = Text::raw(rug_fuzz_0);
        debug_assert_eq!(text.lines.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_1].0.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_2].0[rug_fuzz_3].content, "Hello, World");
        let _rug_ed_tests_llm_16_49_rrrruuuugggg_test_raw_method = 0;
    }
    #[test]
    fn test_styled_method() {
        let _rug_st_tests_llm_16_49_rrrruuuugggg_test_styled_method = 0;
        let rug_fuzz_0 = "Hello, World";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let style = Style::default().fg(Color::Blue);
        let text = Text::styled(rug_fuzz_0, style);
        debug_assert_eq!(text.lines.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_1].0.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_2].0[rug_fuzz_3].content, "Hello, World");
        debug_assert_eq!(
            text.lines[rug_fuzz_4].0[rug_fuzz_5].style.fg, Some(Color::Blue)
        );
        let _rug_ed_tests_llm_16_49_rrrruuuugggg_test_styled_method = 0;
    }
    #[test]
    fn test_width_method() {
        let _rug_st_tests_llm_16_49_rrrruuuugggg_test_width_method = 0;
        let rug_fuzz_0 = "Hello, World";
        let text = Text::raw(rug_fuzz_0);
        debug_assert_eq!(text.width(), 12);
        let _rug_ed_tests_llm_16_49_rrrruuuugggg_test_width_method = 0;
    }
    #[test]
    fn test_height_method() {
        let _rug_st_tests_llm_16_49_rrrruuuugggg_test_height_method = 0;
        let rug_fuzz_0 = "Hello, World";
        let text = Text::raw(rug_fuzz_0);
        debug_assert_eq!(text.height(), 1);
        let _rug_ed_tests_llm_16_49_rrrruuuugggg_test_height_method = 0;
    }
    #[test]
    fn test_patch_style_method() {
        let _rug_st_tests_llm_16_49_rrrruuuugggg_test_patch_style_method = 0;
        let rug_fuzz_0 = "Hello, World";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let mut text = Text::raw(rug_fuzz_0);
        let style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        text.patch_style(style);
        debug_assert_eq!(text.lines.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_1].0.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_2].0[rug_fuzz_3].content, "Hello, World");
        debug_assert_eq!(
            text.lines[rug_fuzz_4].0[rug_fuzz_5].style.fg, Some(Color::Red)
        );
        debug_assert_eq!(
            text.lines[rug_fuzz_6].0[rug_fuzz_7].style.add_modifier, Modifier::BOLD
        );
        let _rug_ed_tests_llm_16_49_rrrruuuugggg_test_patch_style_method = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_50 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "My";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 1;
        let spans = Spans::from(
            vec![
                Span::styled(rug_fuzz_0, Style::default().fg(Color::Yellow)),
                Span::raw(" text")
            ],
        );
        let text: Text = From::from(spans);
        debug_assert_eq!(text.lines.len(), 1);
        debug_assert_eq!(text.lines[rug_fuzz_1].0[rug_fuzz_2].content, "My");
        debug_assert_eq!(
            text.lines[rug_fuzz_3].0[rug_fuzz_4].style.fg, Some(Color::Yellow)
        );
        debug_assert_eq!(text.lines[rug_fuzz_5].0[rug_fuzz_6].content, " text");
        debug_assert_eq!(text.lines[rug_fuzz_7].0[rug_fuzz_8].style.fg, None);
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_51 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_51_rrrruuuugggg_test_default = 0;
        let default_text: Text = Default::default();
        let expected_text = Text { lines: Vec::new() };
        debug_assert_eq!(default_text, expected_text);
        let _rug_ed_tests_llm_16_51_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_52 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_extend() {
        let _rug_st_tests_llm_16_52_rrrruuuugggg_test_extend = 0;
        let rug_fuzz_0 = "The first line\nThe second line";
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = "These are two\nmore lines!";
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = "Some more lines\nnow with more style!";
        let rug_fuzz_5 = 6;
        let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
        let mut text = Text::from(rug_fuzz_0);
        debug_assert_eq!(rug_fuzz_1, text.height());
        text.extend(Text::raw(rug_fuzz_2));
        debug_assert_eq!(rug_fuzz_3, text.height());
        text.extend(Text::styled(rug_fuzz_4, style));
        debug_assert_eq!(rug_fuzz_5, text.height());
        let _rug_ed_tests_llm_16_52_rrrruuuugggg_test_extend = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_247 {
    use super::*;
    use crate::*;
    use crate::text::Span;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_styled_graphemes() {
        let _rug_st_tests_llm_16_247_rrrruuuugggg_test_styled_graphemes = 0;
        let rug_fuzz_0 = "Text";
        let rug_fuzz_1 = "T";
        let style = Style::default().fg(Color::Yellow);
        let span = Span::styled(rug_fuzz_0, style);
        let style = Style::default().fg(Color::Green).bg(Color::Black);
        let styled_graphemes = span.styled_graphemes(style);
        let expected = vec![
            StyledGrapheme { symbol : rug_fuzz_1, style : Style { fg :
            Some(Color::Yellow), bg : Some(Color::Black), add_modifier :
            Modifier::empty(), sub_modifier : Modifier::empty(), }, }, StyledGrapheme {
            symbol : "e", style : Style { fg : Some(Color::Yellow), bg :
            Some(Color::Black), add_modifier : Modifier::empty(), sub_modifier :
            Modifier::empty(), }, }, StyledGrapheme { symbol : "x", style : Style { fg :
            Some(Color::Yellow), bg : Some(Color::Black), add_modifier :
            Modifier::empty(), sub_modifier : Modifier::empty(), }, }, StyledGrapheme {
            symbol : "t", style : Style { fg : Some(Color::Yellow), bg :
            Some(Color::Black), add_modifier : Modifier::empty(), sub_modifier :
            Modifier::empty(), }, }
        ];
        let result: Vec<StyledGrapheme> = styled_graphemes.collect();
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_247_rrrruuuugggg_test_styled_graphemes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_248 {
    use crate::text::{Span, StyledGrapheme};
    use crate::style::{Color, Modifier, Style};
    use std::iter::Iterator;
    #[test]
    fn test_span_width() {
        let _rug_st_tests_llm_16_248_rrrruuuugggg_test_span_width = 0;
        let rug_fuzz_0 = "Hello, World!";
        let span = Span::raw(rug_fuzz_0);
        debug_assert_eq!(span.width(), 13);
        let _rug_ed_tests_llm_16_248_rrrruuuugggg_test_span_width = 0;
    }
    #[test]
    fn test_span_styled_graphemes() {
        let _rug_st_tests_llm_16_248_rrrruuuugggg_test_span_styled_graphemes = 0;
        let rug_fuzz_0 = "Text";
        let rug_fuzz_1 = "T";
        let style = Style::default().fg(Color::Yellow);
        let span = Span::styled(rug_fuzz_0, style);
        let style = Style::default().fg(Color::Green).bg(Color::Black);
        let styled_graphemes = span.styled_graphemes(style);
        let expected_graphemes = vec![
            StyledGrapheme { symbol : rug_fuzz_1, style : Style { fg :
            Some(Color::Yellow), bg : Some(Color::Black), add_modifier :
            Modifier::empty(), sub_modifier : Modifier::empty(), }, }, StyledGrapheme {
            symbol : "e", style : Style { fg : Some(Color::Yellow), bg :
            Some(Color::Black), add_modifier : Modifier::empty(), sub_modifier :
            Modifier::empty(), }, }, StyledGrapheme { symbol : "x", style : Style { fg :
            Some(Color::Yellow), bg : Some(Color::Black), add_modifier :
            Modifier::empty(), sub_modifier : Modifier::empty(), }, }, StyledGrapheme {
            symbol : "t", style : Style { fg : Some(Color::Yellow), bg :
            Some(Color::Black), add_modifier : Modifier::empty(), sub_modifier :
            Modifier::empty(), }, }
        ];
        debug_assert_eq!(
            styled_graphemes.collect:: < Vec < StyledGrapheme > > (), expected_graphemes
        );
        let _rug_ed_tests_llm_16_248_rrrruuuugggg_test_span_styled_graphemes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_249 {
    use crate::text::{Span, Spans};
    use crate::style::{Color, Style};
    #[test]
    fn test_width() {
        let _rug_st_tests_llm_16_249_rrrruuuugggg_test_width = 0;
        let rug_fuzz_0 = "My";
        let rug_fuzz_1 = 7;
        let spans = Spans::from(
            vec![
                Span::styled(rug_fuzz_0, Style::default().fg(Color::Yellow)),
                Span::raw(" text")
            ],
        );
        debug_assert_eq!(rug_fuzz_1, spans.width());
        let _rug_ed_tests_llm_16_249_rrrruuuugggg_test_width = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_250 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_height() {
        let _rug_st_tests_llm_16_250_rrrruuuugggg_test_height = 0;
        let rug_fuzz_0 = "The first line\nThe second line";
        let rug_fuzz_1 = 2;
        let text = Text::from(rug_fuzz_0);
        debug_assert_eq!(rug_fuzz_1, text.height());
        let _rug_ed_tests_llm_16_250_rrrruuuugggg_test_height = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_251 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_patch_style() {
        let _rug_st_tests_llm_16_251_rrrruuuugggg_test_patch_style = 0;
        let rug_fuzz_0 = "The first line\nThe second line";
        let rug_fuzz_1 = "The first line\nThe second line";
        let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
        let mut raw_text = Text::raw(rug_fuzz_0);
        let styled_text = Text::styled(String::from(rug_fuzz_1), style);
        debug_assert_ne!(raw_text, styled_text);
        raw_text.patch_style(style);
        debug_assert_eq!(raw_text, styled_text);
        let _rug_ed_tests_llm_16_251_rrrruuuugggg_test_patch_style = 0;
    }
}
mod tests_llm_16_252 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_raw() {
        let _rug_st_tests_llm_16_252_rrrruuuugggg_test_raw = 0;
        let rug_fuzz_0 = "The first line";
        let rug_fuzz_1 = "The first line\nThe second line";
        let rug_fuzz_2 = "The first line";
        let rug_fuzz_3 = "The first line\nThe second line";
        let expected = Text {
            lines: vec![Spans::from(rug_fuzz_0), Spans::from("The second line")],
        };
        debug_assert_eq!(Text::raw(rug_fuzz_1), expected);
        let expected = Text {
            lines: vec![Spans::from(rug_fuzz_2), Spans::from("The second line")],
        };
        debug_assert_eq!(Text::raw(String::from(rug_fuzz_3)), expected);
        let _rug_ed_tests_llm_16_252_rrrruuuugggg_test_raw = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_254 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_styled() {
        let _rug_st_tests_llm_16_254_rrrruuuugggg_test_styled = 0;
        let rug_fuzz_0 = "The first line\nThe second line";
        let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
        let raw_content = rug_fuzz_0;
        let result: Text = Text::styled(raw_content, style);
        let expected: Text = Text::styled(raw_content, style);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_254_rrrruuuugggg_test_styled = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_255 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_width() {
        let _rug_st_tests_llm_16_255_rrrruuuugggg_test_width = 0;
        let rug_fuzz_0 = "The first line\nThe second line";
        let rug_fuzz_1 = 15;
        let text = Text::from(rug_fuzz_0);
        debug_assert_eq!(rug_fuzz_1, text.width());
        let _rug_ed_tests_llm_16_255_rrrruuuugggg_test_width = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use std::borrow::Cow;
    use crate::text::{Span, Style};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "My text";
        let mut p0: Cow<str> = Cow::Borrowed(rug_fuzz_0);
        Span::raw::<Cow<str>>(p0);
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_64 {
    use super::*;
    use crate::text::Span;
    use crate::style::{Color, Modifier, Style};
    use std::borrow::Cow;
    #[test]
    fn test_styled() {
        let _rug_st_tests_rug_64_rrrruuuugggg_test_styled = 0;
        let rug_fuzz_0 = "My text";
        let mut p0: Cow<'static, str> = rug_fuzz_0.into();
        let mut p1 = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
        Span::styled(p0, p1);
        let _rug_ed_tests_rug_64_rrrruuuugggg_test_styled = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use crate::text::Spans;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_67_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "";
        let p0: Spans<'static> = Spans::from(rug_fuzz_0);
        <std::string::String>::from(p0);
        let _rug_ed_tests_rug_67_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::text::Text;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_68_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello World";
        let p0: &str = rug_fuzz_0;
        <Text<'static>>::from(p0);
        let _rug_ed_tests_rug_68_rrrruuuugggg_test_rug = 0;
    }
}
