use crate::{
    buffer::Buffer, layout::{Alignment, Rect},
    style::Style, text::{StyledGrapheme, Text},
    widgets::{
        reflow::{LineComposer, LineTruncator, WordWrapper},
        Block, Widget,
    },
};
use std::iter;
use unicode_width::UnicodeWidthStr;
fn get_line_offset(line_width: u16, text_area_width: u16, alignment: Alignment) -> u16 {
    match alignment {
        Alignment::Center => (text_area_width / 2).saturating_sub(line_width / 2),
        Alignment::Right => text_area_width.saturating_sub(line_width),
        Alignment::Left => 0,
    }
}
/// A widget to display some text.
///
/// # Examples
///
/// ```
/// # use tui::text::{Text, Spans, Span};
/// # use tui::widgets::{Block, Borders, Paragraph, Wrap};
/// # use tui::style::{Style, Color, Modifier};
/// # use tui::layout::{Alignment};
/// let text = vec![
///     Spans::from(vec![
///         Span::raw("First"),
///         Span::styled("line",Style::default().add_modifier(Modifier::ITALIC)),
///         Span::raw("."),
///     ]),
///     Spans::from(Span::styled("Second line", Style::default().fg(Color::Red))),
/// ];
/// Paragraph::new(text)
///     .block(Block::default().title("Paragraph").borders(Borders::ALL))
///     .style(Style::default().fg(Color::White).bg(Color::Black))
///     .alignment(Alignment::Center)
///     .wrap(Wrap { trim: true });
/// ```
#[derive(Debug, Clone)]
pub struct Paragraph<'a> {
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
    /// How to wrap the text
    wrap: Option<Wrap>,
    /// The text to display
    text: Text<'a>,
    /// Scroll
    scroll: (u16, u16),
    /// Alignment of the text
    alignment: Alignment,
}
/// Describes how to wrap text across lines.
///
/// ## Examples
///
/// ```
/// # use tui::widgets::{Paragraph, Wrap};
/// # use tui::text::Text;
/// let bullet_points = Text::from(r#"Some indented points:
///     - First thing goes here and is long so that it wraps
///     - Here is another point that is long enough to wrap"#);
///
/// // With leading spaces trimmed (window width of 30 chars):
/// Paragraph::new(bullet_points.clone()).wrap(Wrap { trim: true });
/// // Some indented points:
/// // - First thing goes here and is
/// // long so that it wraps
/// // - Here is another point that
/// // is long enough to wrap
///
/// // But without trimming, indentation is preserved:
/// Paragraph::new(bullet_points).wrap(Wrap { trim: false });
/// // Some indented points:
/// //     - First thing goes here
/// // and is long so that it wraps
/// //     - Here is another point
/// // that is long enough to wrap
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Wrap {
    /// Should leading whitespace be trimmed
    pub trim: bool,
}
impl<'a> Paragraph<'a> {
    pub fn new<T>(text: T) -> Paragraph<'a>
    where
        T: Into<Text<'a>>,
    {
        Paragraph {
            block: None,
            style: Default::default(),
            wrap: None,
            text: text.into(),
            scroll: (0, 0),
            alignment: Alignment::Left,
        }
    }
    pub fn block(mut self, block: Block<'a>) -> Paragraph<'a> {
        self.block = Some(block);
        self
    }
    pub fn style(mut self, style: Style) -> Paragraph<'a> {
        self.style = style;
        self
    }
    pub fn wrap(mut self, wrap: Wrap) -> Paragraph<'a> {
        self.wrap = Some(wrap);
        self
    }
    pub fn scroll(mut self, offset: (u16, u16)) -> Paragraph<'a> {
        self.scroll = offset;
        self
    }
    pub fn alignment(mut self, alignment: Alignment) -> Paragraph<'a> {
        self.alignment = alignment;
        self
    }
}
impl<'a> Widget for Paragraph<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let text_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        if text_area.height < 1 {
            return;
        }
        let style = self.style;
        let mut styled = self
            .text
            .lines
            .iter()
            .flat_map(|spans| {
                spans
                    .0
                    .iter()
                    .flat_map(|span| span.styled_graphemes(style))
                    .chain(
                        iter::once(StyledGrapheme {
                            symbol: "\n",
                            style: self.style,
                        }),
                    )
            });
        let mut line_composer: Box<dyn LineComposer> = if let Some(Wrap { trim })
            = self.wrap
        {
            Box::new(WordWrapper::new(&mut styled, text_area.width, trim))
        } else {
            let mut line_composer = Box::new(
                LineTruncator::new(&mut styled, text_area.width),
            );
            if let Alignment::Left = self.alignment {
                line_composer.set_horizontal_offset(self.scroll.1);
            }
            line_composer
        };
        let mut y = 0;
        while let Some((current_line, current_line_width)) = line_composer.next_line() {
            if y >= self.scroll.0 {
                let mut x = get_line_offset(
                    current_line_width,
                    text_area.width,
                    self.alignment,
                );
                for StyledGrapheme { symbol, style } in current_line {
                    buf.get_mut(
                            text_area.left() + x,
                            text_area.top() + y - self.scroll.0,
                        )
                        .set_symbol(if symbol.is_empty() { " " } else { symbol })
                        .set_style(*style);
                    x += symbol.width() as u16;
                }
            }
            y += 1;
            if y >= text_area.height + self.scroll.0 {
                break;
            }
        }
    }
}
#[cfg(test)]
mod tests_llm_16_113 {
    use super::*;
    use crate::*;
    use crate::style::*;
    use crate::layout::*;
    use crate::buffer::*;
    use unicode_segmentation::UnicodeSegmentation;
    #[test]
    fn test_render() {
        let _rug_st_tests_llm_16_113_rrrruuuugggg_test_render = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = "Test paragraph";
        let rug_fuzz_5 = false;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 10;
        let rug_fuzz_11 = 5;
        let mut buf = Buffer::empty(
            Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3),
        );
        let mut paragraph = Paragraph::new(rug_fuzz_4)
            .block(Block::default())
            .style(Style::default())
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: rug_fuzz_5 })
            .scroll((rug_fuzz_6, rug_fuzz_7));
        let area = Rect::new(rug_fuzz_8, rug_fuzz_9, rug_fuzz_10, rug_fuzz_11);
        paragraph.render(area, &mut buf);
        debug_assert_eq!(
            buf.content, vec![Cell { symbol : 'T'.to_string(), fg : Color::Reset, bg :
            Color::Reset, modifier : Modifier::empty() }, Cell { symbol : 'e'
            .to_string(), fg : Color::Reset, bg : Color::Reset, modifier :
            Modifier::empty() }, Cell { symbol : 's'.to_string(), fg : Color::Reset, bg :
            Color::Reset, modifier : Modifier::empty() }, Cell { symbol : 't'
            .to_string(), fg : Color::Reset, bg : Color::Reset, modifier :
            Modifier::empty() }, Cell { symbol : ' '.to_string(), fg : Color::Reset, bg :
            Color::Reset, modifier : Modifier::empty() }, Cell { symbol : 'p'
            .to_string(), fg : Color::Reset, bg : Color::Reset, modifier :
            Modifier::empty() }, Cell { symbol : 'a'.to_string(), fg : Color::Reset, bg :
            Color::Reset, modifier : Modifier::empty() }, Cell { symbol : 'r'
            .to_string(), fg : Color::Reset, bg : Color::Reset, modifier :
            Modifier::empty() }, Cell { symbol : 'a'.to_string(), fg : Color::Reset, bg :
            Color::Reset, modifier : Modifier::empty() }, Cell { symbol : 'g'
            .to_string(), fg : Color::Reset, bg : Color::Reset, modifier :
            Modifier::empty() },]
        );
        let _rug_ed_tests_llm_16_113_rrrruuuugggg_test_render = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_359 {
    use super::*;
    use crate::*;
    use std::borrow::Cow;
    #[test]
    fn test_alignment() {
        let _rug_st_tests_llm_16_359_rrrruuuugggg_test_alignment = 0;
        let rug_fuzz_0 = "Hello, World!";
        let text = Text::raw(rug_fuzz_0);
        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        debug_assert_eq!(Alignment::Center, paragraph.alignment);
        let _rug_ed_tests_llm_16_359_rrrruuuugggg_test_alignment = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_366 {
    use crate::widgets::paragraph::get_line_offset;
    use crate::layout::Alignment;
    #[test]
    fn test_get_line_offset_left_alignment() {
        let _rug_st_tests_llm_16_366_rrrruuuugggg_test_get_line_offset_left_alignment = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let line_width: u16 = rug_fuzz_0;
        let text_area_width: u16 = rug_fuzz_1;
        let alignment = Alignment::Left;
        debug_assert_eq!(get_line_offset(line_width, text_area_width, alignment), 0);
        let _rug_ed_tests_llm_16_366_rrrruuuugggg_test_get_line_offset_left_alignment = 0;
    }
    #[test]
    fn test_get_line_offset_center_alignment() {
        let _rug_st_tests_llm_16_366_rrrruuuugggg_test_get_line_offset_center_alignment = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let line_width: u16 = rug_fuzz_0;
        let text_area_width: u16 = rug_fuzz_1;
        let alignment = Alignment::Center;
        debug_assert_eq!(get_line_offset(line_width, text_area_width, alignment), 5);
        let _rug_ed_tests_llm_16_366_rrrruuuugggg_test_get_line_offset_center_alignment = 0;
    }
    #[test]
    fn test_get_line_offset_right_alignment() {
        let _rug_st_tests_llm_16_366_rrrruuuugggg_test_get_line_offset_right_alignment = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let line_width: u16 = rug_fuzz_0;
        let text_area_width: u16 = rug_fuzz_1;
        let alignment = Alignment::Right;
        debug_assert_eq!(get_line_offset(line_width, text_area_width, alignment), 10);
        let _rug_ed_tests_llm_16_366_rrrruuuugggg_test_get_line_offset_right_alignment = 0;
    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::widgets::paragraph::{Paragraph, Text, Alignment};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_11_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello, world!";
        let mut p0: Text = rug_fuzz_0.into();
        let p1: Paragraph = Paragraph::new(p0);
        let _rug_ed_tests_rug_11_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use crate::widgets::{Block, Paragraph};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_12_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_12_prepare {
            use crate::widgets::Paragraph;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_12_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "Hello World";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_12_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v12: Paragraph<'static> = Paragraph::new(rug_fuzz_0);
                let _rug_ed_tests_rug_12_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_12_prepare_rrrruuuugggg_sample = 0;
            }
        }
        #[cfg(test)]
        mod tests_rug_12_prepare2 {
            use crate::widgets::Block;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_12_prepare2_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 0;
                let _rug_st_tests_rug_12_rrrruuuugggg_sample = rug_fuzz_0;
                let mut v13: Block<'static> = Block::default();
                let _rug_ed_tests_rug_12_rrrruuuugggg_sample = rug_fuzz_1;
                let _rug_ed_tests_rug_12_prepare2_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: Paragraph<'static> = Paragraph::new("Hello World");
        let mut p1: Block<'static> = Block::default();
        crate::widgets::paragraph::Paragraph::<'static>::block(p0, p1);
        let _rug_ed_tests_rug_12_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    use crate::style::{Color, Modifier, Style};
    use crate::widgets::Paragraph;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_13_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello World";
        let mut p0: Paragraph<'static> = Paragraph::new(rug_fuzz_0);
        let mut p1 = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        p0.style(p1);
        let _rug_ed_tests_rug_13_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use crate::widgets::{Paragraph, Wrap};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_14_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello World";
        let rug_fuzz_1 = true;
        let mut p0: Paragraph<'static> = Paragraph::new(rug_fuzz_0);
        let mut p1 = Wrap { trim: rug_fuzz_1 };
        p0.wrap(p1);
        let _rug_ed_tests_rug_14_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::widgets::Paragraph;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_15_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Hello World";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let mut p0: Paragraph<'static> = Paragraph::new(rug_fuzz_0);
        let p1: (u16, u16) = (rug_fuzz_1, rug_fuzz_2);
        p0.scroll(p1);
        let _rug_ed_tests_rug_15_rrrruuuugggg_test_rug = 0;
    }
}
