//! `style` contains the primitives used to control how your user interface will look.
use bitflags::bitflags;
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}
bitflags! {
    #[doc = " Modifier changes the way a piece of text is displayed."] #[doc = ""] #[doc
    = " They are bitflags so they can easily be composed."] #[doc = ""] #[doc =
    " ## Examples"] #[doc = ""] #[doc = " ```rust"] #[doc =
    " # use tui::style::Modifier;"] #[doc = ""] #[doc =
    " let m = Modifier::BOLD | Modifier::ITALIC;"] #[doc = " ```"] #[cfg_attr(feature =
    "serde", derive(serde::Serialize, serde::Deserialize))] pub struct Modifier : u16 {
    const BOLD = 0b0000_0000_0001; const DIM = 0b0000_0000_0010; const ITALIC =
    0b0000_0000_0100; const UNDERLINED = 0b0000_0000_1000; const SLOW_BLINK =
    0b0000_0001_0000; const RAPID_BLINK = 0b0000_0010_0000; const REVERSED =
    0b0000_0100_0000; const HIDDEN = 0b0000_1000_0000; const CROSSED_OUT =
    0b0001_0000_0000; }
}
/// Style let you control the main characteristics of the displayed elements.
///
/// ## Examples
///
/// ```rust
/// # use tui::style::{Color, Modifier, Style};
/// Style::default()
///     .fg(Color::Black)
///     .bg(Color::Green)
///     .add_modifier(Modifier::ITALIC | Modifier::BOLD);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub add_modifier: Modifier,
    pub sub_modifier: Modifier,
}
impl Default for Style {
    fn default() -> Style {
        Style {
            fg: None,
            bg: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
        }
    }
}
impl Style {
    /// Changes the foreground color.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::style::{Color, Style};
    /// let style = Style::default().fg(Color::Blue);
    /// let diff = Style::default().fg(Color::Red);
    /// assert_eq!(style.patch(diff), Style::default().fg(Color::Red));
    /// ```
    pub fn fg(mut self, color: Color) -> Style {
        self.fg = Some(color);
        self
    }
    /// Changes the background color.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::style::{Color, Style};
    /// let style = Style::default().bg(Color::Blue);
    /// let diff = Style::default().bg(Color::Red);
    /// assert_eq!(style.patch(diff), Style::default().bg(Color::Red));
    /// ```
    pub fn bg(mut self, color: Color) -> Style {
        self.bg = Some(color);
        self
    }
    /// Changes the text emphasis.
    ///
    /// When applied, it adds the given modifier to the `Style` modifiers.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::style::{Color, Modifier, Style};
    /// let style = Style::default().add_modifier(Modifier::BOLD);
    /// let diff = Style::default().add_modifier(Modifier::ITALIC);
    /// let patched = style.patch(diff);
    /// assert_eq!(patched.add_modifier, Modifier::BOLD | Modifier::ITALIC);
    /// assert_eq!(patched.sub_modifier, Modifier::empty());
    /// ```
    pub fn add_modifier(mut self, modifier: Modifier) -> Style {
        self.sub_modifier.remove(modifier);
        self.add_modifier.insert(modifier);
        self
    }
    /// Changes the text emphasis.
    ///
    /// When applied, it removes the given modifier from the `Style` modifiers.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::style::{Color, Modifier, Style};
    /// let style = Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC);
    /// let diff = Style::default().remove_modifier(Modifier::ITALIC);
    /// let patched = style.patch(diff);
    /// assert_eq!(patched.add_modifier, Modifier::BOLD);
    /// assert_eq!(patched.sub_modifier, Modifier::ITALIC);
    /// ```
    pub fn remove_modifier(mut self, modifier: Modifier) -> Style {
        self.add_modifier.remove(modifier);
        self.sub_modifier.insert(modifier);
        self
    }
    /// Results in a combined style that is equivalent to applying the two individual styles to
    /// a style one after the other.
    ///
    /// ## Examples
    /// ```
    /// # use tui::style::{Color, Modifier, Style};
    /// let style_1 = Style::default().fg(Color::Yellow);
    /// let style_2 = Style::default().bg(Color::Red);
    /// let combined = style_1.patch(style_2);
    /// assert_eq!(
    ///     Style::default().patch(style_1).patch(style_2),
    ///     Style::default().patch(combined));
    /// ```
    pub fn patch(mut self, other: Style) -> Style {
        self.fg = other.fg.or(self.fg);
        self.bg = other.bg.or(self.bg);
        self.add_modifier.remove(other.sub_modifier);
        self.add_modifier.insert(other.add_modifier);
        self.sub_modifier.remove(other.add_modifier);
        self.sub_modifier.insert(other.sub_modifier);
        self
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn styles() -> Vec<Style> {
        vec![
            Style::default(), Style::default().fg(Color::Yellow), Style::default()
            .bg(Color::Yellow), Style::default().add_modifier(Modifier::BOLD),
            Style::default().remove_modifier(Modifier::BOLD), Style::default()
            .add_modifier(Modifier::ITALIC), Style::default()
            .remove_modifier(Modifier::ITALIC), Style::default()
            .add_modifier(Modifier::ITALIC | Modifier::BOLD), Style::default()
            .remove_modifier(Modifier::ITALIC | Modifier::BOLD),
        ]
    }
    #[test]
    fn combined_patch_gives_same_result_as_individual_patch() {
        let styles = styles();
        for &a in &styles {
            for &b in &styles {
                for &c in &styles {
                    for &d in &styles {
                        let combined = a.patch(b.patch(c.patch(d)));
                        assert_eq!(
                            Style::default().patch(a).patch(b).patch(c).patch(d),
                            Style::default().patch(combined)
                        );
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests_llm_16_31 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default_style() {
        let _rug_st_tests_llm_16_31_rrrruuuugggg_test_default_style = 0;
        let default_style = Style::default();
        debug_assert_eq!(default_style.fg, None);
        debug_assert_eq!(default_style.bg, None);
        debug_assert_eq!(default_style.add_modifier, Modifier::empty());
        debug_assert_eq!(default_style.sub_modifier, Modifier::empty());
        let _rug_ed_tests_llm_16_31_rrrruuuugggg_test_default_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_197 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_add_modifier() {
        let _rug_st_tests_llm_16_197_rrrruuuugggg_test_add_modifier = 0;
        let style = Style::default().add_modifier(Modifier::BOLD);
        let diff = Style::default().add_modifier(Modifier::ITALIC);
        let patched = style.patch(diff);
        debug_assert_eq!(patched.add_modifier, Modifier::BOLD | Modifier::ITALIC);
        debug_assert_eq!(patched.sub_modifier, Modifier::empty());
        let _rug_ed_tests_llm_16_197_rrrruuuugggg_test_add_modifier = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_198 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier};
    #[test]
    fn test_bg() {
        let _rug_st_tests_llm_16_198_rrrruuuugggg_test_bg = 0;
        let style = Style::default().bg(Color::Blue);
        let diff = Style::default().bg(Color::Red);
        debug_assert_eq!(style.patch(diff), Style::default().bg(Color::Red));
        let _rug_ed_tests_llm_16_198_rrrruuuugggg_test_bg = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_201 {
    use super::*;
    use crate::*;
    use crate::style::{Modifier, Color};
    #[test]
    fn test_patch() {
        let _rug_st_tests_llm_16_201_rrrruuuugggg_test_patch = 0;
        let style_1 = Style::default().fg(Color::Yellow);
        let style_2 = Style::default().bg(Color::Red);
        let combined = style_1.patch(style_2);
        debug_assert_eq!(
            Style::default().patch(style_1).patch(style_2), Style::default()
            .patch(combined)
        );
        let _rug_ed_tests_llm_16_201_rrrruuuugggg_test_patch = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_202 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_remove_modifier() {
        let _rug_st_tests_llm_16_202_rrrruuuugggg_test_remove_modifier = 0;
        let style = Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC);
        let diff = Style::default().remove_modifier(Modifier::ITALIC);
        let patched = style.patch(diff);
        debug_assert_eq!(patched.add_modifier, Modifier::BOLD);
        debug_assert_eq!(patched.sub_modifier, Modifier::ITALIC);
        let _rug_ed_tests_llm_16_202_rrrruuuugggg_test_remove_modifier = 0;
    }
}
#[cfg(test)]
mod tests_rug_44 {
    use super::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_44_rrrruuuugggg_test_rug = 0;
        let mut p0 = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        let mut p1 = Color::Reset;
        crate::style::Style::fg(p0, p1);
        let _rug_ed_tests_rug_44_rrrruuuugggg_test_rug = 0;
    }
}
