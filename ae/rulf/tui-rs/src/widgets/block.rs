use crate::{
    buffer::Buffer, layout::Rect, style::Style, symbols::line, text::{Span, Spans},
    widgets::{Borders, Widget},
};
#[derive(Debug, Clone, Copy)]
pub enum BorderType {
    Plain,
    Rounded,
    Double,
    Thick,
}
impl BorderType {
    pub fn line_symbols(border_type: BorderType) -> line::Set {
        match border_type {
            BorderType::Plain => line::NORMAL,
            BorderType::Rounded => line::ROUNDED,
            BorderType::Double => line::DOUBLE,
            BorderType::Thick => line::THICK,
        }
    }
}
/// Base widget to be used with all upper level ones. It may be used to display a box border around
/// the widget and/or add a title.
///
/// # Examples
///
/// ```
/// # use tui::widgets::{Block, BorderType, Borders};
/// # use tui::style::{Style, Color};
/// Block::default()
///     .title("Block")
///     .borders(Borders::LEFT | Borders::RIGHT)
///     .border_style(Style::default().fg(Color::White))
///     .border_type(BorderType::Rounded)
///     .style(Style::default().bg(Color::Black));
/// ```
#[derive(Debug, Clone)]
pub struct Block<'a> {
    /// Optional title place on the upper left of the block
    title: Option<Spans<'a>>,
    /// Visible borders
    borders: Borders,
    /// Border style
    border_style: Style,
    /// Type of the border. The default is plain lines but one can choose to have rounded corners
    /// or doubled lines instead.
    border_type: BorderType,
    /// Widget style
    style: Style,
}
impl<'a> Default for Block<'a> {
    fn default() -> Block<'a> {
        Block {
            title: None,
            borders: Borders::NONE,
            border_style: Default::default(),
            border_type: BorderType::Plain,
            style: Default::default(),
        }
    }
}
impl<'a> Block<'a> {
    pub fn title<T>(mut self, title: T) -> Block<'a>
    where
        T: Into<Spans<'a>>,
    {
        self.title = Some(title.into());
        self
    }
    #[deprecated(
        since = "0.10.0",
        note = "You should use styling capabilities of `text::Spans` given as argument of the `title` method to apply styling to the title."
    )]
    pub fn title_style(mut self, style: Style) -> Block<'a> {
        if let Some(t) = self.title {
            let title = String::from(t);
            self.title = Some(Spans::from(Span::styled(title, style)));
        }
        self
    }
    pub fn border_style(mut self, style: Style) -> Block<'a> {
        self.border_style = style;
        self
    }
    pub fn style(mut self, style: Style) -> Block<'a> {
        self.style = style;
        self
    }
    pub fn borders(mut self, flag: Borders) -> Block<'a> {
        self.borders = flag;
        self
    }
    pub fn border_type(mut self, border_type: BorderType) -> Block<'a> {
        self.border_type = border_type;
        self
    }
    /// Compute the inner area of a block based on its border visibility rules.
    pub fn inner(&self, area: Rect) -> Rect {
        if area.width < 2 || area.height < 2 {
            return Rect::default();
        }
        let mut inner = area;
        if self.borders.intersects(Borders::LEFT) {
            inner.x += 1;
            inner.width -= 1;
        }
        if self.borders.intersects(Borders::TOP) || self.title.is_some() {
            inner.y += 1;
            inner.height -= 1;
        }
        if self.borders.intersects(Borders::RIGHT) {
            inner.width -= 1;
        }
        if self.borders.intersects(Borders::BOTTOM) {
            inner.height -= 1;
        }
        inner
    }
}
impl<'a> Widget for Block<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        if area.width < 2 || area.height < 2 {
            return;
        }
        let symbols = BorderType::line_symbols(self.border_type);
        if self.borders.intersects(Borders::LEFT) {
            for y in area.top()..area.bottom() {
                buf.get_mut(area.left(), y)
                    .set_symbol(symbols.vertical)
                    .set_style(self.border_style);
            }
        }
        if self.borders.intersects(Borders::TOP) {
            for x in area.left()..area.right() {
                buf.get_mut(x, area.top())
                    .set_symbol(symbols.horizontal)
                    .set_style(self.border_style);
            }
        }
        if self.borders.intersects(Borders::RIGHT) {
            let x = area.right() - 1;
            for y in area.top()..area.bottom() {
                buf.get_mut(x, y)
                    .set_symbol(symbols.vertical)
                    .set_style(self.border_style);
            }
        }
        if self.borders.intersects(Borders::BOTTOM) {
            let y = area.bottom() - 1;
            for x in area.left()..area.right() {
                buf.get_mut(x, y)
                    .set_symbol(symbols.horizontal)
                    .set_style(self.border_style);
            }
        }
        if self.borders.contains(Borders::LEFT | Borders::TOP) {
            buf.get_mut(area.left(), area.top())
                .set_symbol(symbols.top_left)
                .set_style(self.border_style);
        }
        if self.borders.contains(Borders::RIGHT | Borders::TOP) {
            buf.get_mut(area.right() - 1, area.top())
                .set_symbol(symbols.top_right)
                .set_style(self.border_style);
        }
        if self.borders.contains(Borders::LEFT | Borders::BOTTOM) {
            buf.get_mut(area.left(), area.bottom() - 1)
                .set_symbol(symbols.bottom_left)
                .set_style(self.border_style);
        }
        if self.borders.contains(Borders::RIGHT | Borders::BOTTOM) {
            buf.get_mut(area.right() - 1, area.bottom() - 1)
                .set_symbol(symbols.bottom_right)
                .set_style(self.border_style);
        }
        if let Some(title) = self.title {
            let lx = if self.borders.intersects(Borders::LEFT) { 1 } else { 0 };
            let rx = if self.borders.intersects(Borders::RIGHT) { 1 } else { 0 };
            let width = area.width - lx - rx;
            buf.set_spans(area.left() + lx, area.top(), &title, width);
        }
    }
}
#[cfg(test)]
mod tests_llm_16_61 {
    use super::*;
    use crate::*;
    use crate::layout::Rect;
    use crate::style::{Color, Modifier, Style};
    use crate::symbols::line::{DOUBLE, NORMAL, ROUNDED, THICK};
    use crate::symbols::line::Set;
    #[test]
    fn test_render() {
        let _rug_st_tests_llm_16_61_rrrruuuugggg_test_render = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = "Title";
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 10;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 1;
        let rug_fuzz_12 = 1;
        let rug_fuzz_13 = 1;
        let rug_fuzz_14 = 1;
        let rug_fuzz_15 = 1;
        let rug_fuzz_16 = "Title";
        let rug_fuzz_17 = 8;
        let mut buf = Buffer::default();
        let area = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let mut block = Block::default()
            .title(Spans::from(vec![Span::raw(rug_fuzz_4)]))
            .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(Color::White))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));
        block.render(area, &mut buf);
        let mut expected_buf = Buffer::empty(area);
        expected_buf.set_style(area, Style::default().bg(Color::Black));
        for y in rug_fuzz_5..rug_fuzz_6 {
            expected_buf
                .get_mut(area.left(), y)
                .set_symbol(ROUNDED.vertical)
                .set_style(Style::default().fg(Color::White));
            expected_buf
                .get_mut(area.right() - rug_fuzz_7, y)
                .set_symbol(ROUNDED.vertical)
                .set_style(Style::default().fg(Color::White));
        }
        for x in rug_fuzz_8..rug_fuzz_9 {
            expected_buf
                .get_mut(x, area.top())
                .set_symbol(ROUNDED.horizontal)
                .set_style(Style::default().fg(Color::White));
            expected_buf
                .get_mut(x, area.bottom() - rug_fuzz_10)
                .set_symbol(ROUNDED.horizontal)
                .set_style(Style::default().fg(Color::White));
        }
        expected_buf
            .get_mut(area.left(), area.top())
            .set_symbol(ROUNDED.top_left)
            .set_style(Style::default().fg(Color::White));
        expected_buf
            .get_mut(area.right() - rug_fuzz_11, area.top())
            .set_symbol(ROUNDED.top_right)
            .set_style(Style::default().fg(Color::White));
        expected_buf
            .get_mut(area.left(), area.bottom() - rug_fuzz_12)
            .set_symbol(ROUNDED.bottom_left)
            .set_style(Style::default().fg(Color::White));
        expected_buf
            .get_mut(area.right() - rug_fuzz_13, area.bottom() - rug_fuzz_14)
            .set_symbol(ROUNDED.bottom_right)
            .set_style(Style::default().fg(Color::White));
        expected_buf
            .set_spans(
                area.left() + rug_fuzz_15,
                area.top(),
                &Spans::from(vec![Span::raw(rug_fuzz_16)]),
                rug_fuzz_17,
            );
        debug_assert_eq!(buf, expected_buf);
        let _rug_ed_tests_llm_16_61_rrrruuuugggg_test_render = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_269 {
    use super::*;
    use crate::*;
    use crate::style::{Style, Color, Modifier};
    #[test]
    fn test_border_style() {
        let _rug_st_tests_llm_16_269_rrrruuuugggg_test_border_style = 0;
        let style = Style::default().fg(Color::Blue);
        let block = Block::default().border_style(style.clone());
        debug_assert_eq!(block.border_style, style);
        let _rug_ed_tests_llm_16_269_rrrruuuugggg_test_border_style = 0;
    }
}
#[cfg(test)]
mod tests_rug_79 {
    use super::*;
    use crate::widgets::block::{Block, Borders, BorderType};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_79_rrrruuuugggg_test_rug = 0;
        Block::<'static>::default();
        let _rug_ed_tests_rug_79_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::widgets::block::Block;
    use crate::text::Spans;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Sample title";
        let mut p0: Block<'static> = Block::default();
        let p1: Spans<'static> = Spans::from(vec![rug_fuzz_0.into()]);
        p0.title(p1);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::widgets::block::Block;
    use crate::style::{Color, Modifier, Style};
    use crate::text::{Spans, Span};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_81_rrrruuuugggg_test_rug = 0;
        let mut p0: Block<'static> = Block::default();
        let mut p1 = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        <Block<'static>>::title_style(p0, p1);
        let _rug_ed_tests_rug_81_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::widgets::block::Block;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_82_rrrruuuugggg_test_rug = 0;
        let mut p0: Block<'static> = Block::default();
        let mut p1 = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        <Block<'static>>::style(p0, p1);
        let _rug_ed_tests_rug_82_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::widgets::block::Block;
    use crate::widgets::Borders;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_rug = 0;
        let mut p0: Block<'static> = Block::default();
        let mut p1 = Borders::all();
        p0.borders(p1);
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use crate::widgets::block::{Block, BorderType};
    #[test]
    fn test_border_type() {
        let _rug_st_tests_rug_84_rrrruuuugggg_test_border_type = 0;
        let mut p0: Block<'static> = Block::default();
        let mut p1: BorderType = BorderType::Plain;
        p0.border_type(p1);
        let _rug_ed_tests_rug_84_rrrruuuugggg_test_border_type = 0;
    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::widgets::block::Block;
    use crate::layout::Rect;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_85_rrrruuuugggg_test_rug = 0;
        let mut p0: Block<'static> = Block::default();
        let mut p1: Rect = Rect::default();
        Block::inner(&p0, p1);
        let _rug_ed_tests_rug_85_rrrruuuugggg_test_rug = 0;
    }
}
