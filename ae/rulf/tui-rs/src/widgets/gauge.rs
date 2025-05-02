use crate::{
    buffer::Buffer, layout::Rect, style::{Color, Style},
    symbols, text::{Span, Spans},
    widgets::{Block, Widget},
};
/// A widget to display a task progress.
///
/// # Examples:
///
/// ```
/// # use tui::widgets::{Widget, Gauge, Block, Borders};
/// # use tui::style::{Style, Color, Modifier};
/// Gauge::default()
///     .block(Block::default().borders(Borders::ALL).title("Progress"))
///     .gauge_style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::ITALIC))
///     .percent(20);
/// ```
#[derive(Debug, Clone)]
pub struct Gauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    label: Option<Span<'a>>,
    style: Style,
    gauge_style: Style,
}
impl<'a> Default for Gauge<'a> {
    fn default() -> Gauge<'a> {
        Gauge {
            block: None,
            ratio: 0.0,
            label: None,
            style: Style::default(),
            gauge_style: Style::default(),
        }
    }
}
impl<'a> Gauge<'a> {
    pub fn block(mut self, block: Block<'a>) -> Gauge<'a> {
        self.block = Some(block);
        self
    }
    pub fn percent(mut self, percent: u16) -> Gauge<'a> {
        assert!(percent <= 100, "Percentage should be between 0 and 100 inclusively.");
        self.ratio = f64::from(percent) / 100.0;
        self
    }
    /// Sets ratio ([0.0, 1.0]) directly.
    pub fn ratio(mut self, ratio: f64) -> Gauge<'a> {
        assert!(
            ratio <= 1.0 && ratio >= 0.0, "Ratio should be between 0 and 1 inclusively."
        );
        self.ratio = ratio;
        self
    }
    pub fn label<T>(mut self, label: T) -> Gauge<'a>
    where
        T: Into<Span<'a>>,
    {
        self.label = Some(label.into());
        self
    }
    pub fn style(mut self, style: Style) -> Gauge<'a> {
        self.style = style;
        self
    }
    pub fn gauge_style(mut self, style: Style) -> Gauge<'a> {
        self.gauge_style = style;
        self
    }
}
impl<'a> Widget for Gauge<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let gauge_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        buf.set_style(gauge_area, self.gauge_style);
        if gauge_area.height < 1 {
            return;
        }
        let center = gauge_area.height / 2 + gauge_area.top();
        let width = (f64::from(gauge_area.width) * self.ratio).round() as u16;
        let end = gauge_area.left() + width;
        let ratio = self.ratio;
        let label = self
            .label
            .unwrap_or_else(|| Span::from(format!("{}%", (ratio * 100.0).round())));
        for y in gauge_area.top()..gauge_area.bottom() {
            for x in gauge_area.left()..end {
                buf.get_mut(x, y).set_symbol(" ");
            }
            if y == center {
                let label_width = label.width() as u16;
                let middle = (gauge_area.width - label_width) / 2 + gauge_area.left();
                buf.set_span(middle, y, &label, gauge_area.right() - middle);
            }
            for x in gauge_area.left()..end {
                buf.get_mut(x, y)
                    .set_fg(self.gauge_style.bg.unwrap_or(Color::Reset))
                    .set_bg(self.gauge_style.fg.unwrap_or(Color::Reset));
            }
        }
    }
}
/// A compact widget to display a task progress over a single line.
///
/// # Examples:
///
/// ```
/// # use tui::widgets::{Widget, LineGauge, Block, Borders};
/// # use tui::style::{Style, Color, Modifier};
/// # use tui::symbols;
/// LineGauge::default()
///     .block(Block::default().borders(Borders::ALL).title("Progress"))
///     .gauge_style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD))
///     .line_set(symbols::line::THICK)
///     .ratio(0.4);
/// ```
pub struct LineGauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    label: Option<Spans<'a>>,
    line_set: symbols::line::Set,
    style: Style,
    gauge_style: Style,
}
impl<'a> Default for LineGauge<'a> {
    fn default() -> Self {
        Self {
            block: None,
            ratio: 0.0,
            label: None,
            style: Style::default(),
            line_set: symbols::line::NORMAL,
            gauge_style: Style::default(),
        }
    }
}
impl<'a> LineGauge<'a> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
    pub fn ratio(mut self, ratio: f64) -> Self {
        assert!(
            ratio <= 1.0 && ratio >= 0.0, "Ratio should be between 0 and 1 inclusively."
        );
        self.ratio = ratio;
        self
    }
    pub fn line_set(mut self, set: symbols::line::Set) -> Self {
        self.line_set = set;
        self
    }
    pub fn label<T>(mut self, label: T) -> Self
    where
        T: Into<Spans<'a>>,
    {
        self.label = Some(label.into());
        self
    }
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn gauge_style(mut self, style: Style) -> Self {
        self.gauge_style = style;
        self
    }
}
impl<'a> Widget for LineGauge<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let gauge_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        if gauge_area.height < 1 {
            return;
        }
        let ratio = self.ratio;
        let label = self
            .label
            .unwrap_or_else(move || Spans::from(format!("{:.0}%", ratio * 100.0)));
        let (col, row) = buf
            .set_spans(gauge_area.left(), gauge_area.top(), &label, gauge_area.width);
        let start = col + 1;
        if start >= gauge_area.right() {
            return;
        }
        let end = start
            + (f64::from(gauge_area.right().saturating_sub(start)) * self.ratio).floor()
                as u16;
        for col in start..end {
            buf.get_mut(col, row)
                .set_symbol(self.line_set.horizontal)
                .set_style(Style {
                    fg: self.gauge_style.fg,
                    bg: None,
                    add_modifier: self.gauge_style.add_modifier,
                    sub_modifier: self.gauge_style.sub_modifier,
                });
        }
        for col in end..gauge_area.right() {
            buf.get_mut(col, row)
                .set_symbol(self.line_set.horizontal)
                .set_style(Style {
                    fg: self.gauge_style.bg,
                    bg: None,
                    add_modifier: self.gauge_style.add_modifier,
                    sub_modifier: self.gauge_style.sub_modifier,
                });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[should_panic]
    fn gauge_invalid_percentage() {
        Gauge::default().percent(110);
    }
    #[test]
    #[should_panic]
    fn gauge_invalid_ratio_upper_bound() {
        Gauge::default().ratio(1.1);
    }
    #[test]
    #[should_panic]
    fn gauge_invalid_ratio_lower_bound() {
        Gauge::default().ratio(-0.5);
    }
}
#[cfg(test)]
mod tests_llm_16_330 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    use crate::widgets::{Block, Borders, Widget};
    #[test]
    fn test_gauge_style() {
        let _rug_st_tests_llm_16_330_rrrruuuugggg_test_gauge_style = 0;
        let style = Style::default()
            .fg(Color::White)
            .bg(Color::Black)
            .add_modifier(Modifier::ITALIC);
        let gauge = Gauge::default().gauge_style(style);
        debug_assert_eq!(gauge.gauge_style, style);
        let _rug_ed_tests_llm_16_330_rrrruuuugggg_test_gauge_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_333 {
    use super::*;
    use crate::*;
    use crate::style::{Style, Modifier, Color};
    use crate::widgets::{Block, Borders};
    #[test]
    fn test_percent() {
        let _rug_st_tests_llm_16_333_rrrruuuugggg_test_percent = 0;
        let rug_fuzz_0 = "Progress";
        let rug_fuzz_1 = 20;
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(rug_fuzz_0))
            .gauge_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::ITALIC),
            )
            .percent(rug_fuzz_1);
        debug_assert_eq!(gauge.ratio, 0.2);
        let _rug_ed_tests_llm_16_333_rrrruuuugggg_test_percent = 0;
    }
    #[test]
    #[should_panic(expected = "Percentage should be between 0 and 100 inclusively.")]
    fn test_percent_panic() {
        let _rug_st_tests_llm_16_333_rrrruuuugggg_test_percent_panic = 0;
        let rug_fuzz_0 = 110;
        Gauge::default().percent(rug_fuzz_0);
        let _rug_ed_tests_llm_16_333_rrrruuuugggg_test_percent_panic = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_336 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier};
    #[test]
    fn test_style() {
        let _rug_st_tests_llm_16_336_rrrruuuugggg_test_style = 0;
        let mut gauge = Gauge::default();
        let style = Style::default().fg(Color::Blue);
        gauge = gauge.style(style.clone());
        debug_assert_eq!(gauge.style, style);
        let _rug_ed_tests_llm_16_336_rrrruuuugggg_test_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_338 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    use crate::symbols::line::Set;
    #[test]
    fn test_gauge_style() {
        let _rug_st_tests_llm_16_338_rrrruuuugggg_test_gauge_style = 0;
        let style = Style::default().fg(Color::Red);
        let gauge = LineGauge::default().gauge_style(style);
        debug_assert_eq!(gauge.gauge_style.fg, Some(Color::Red));
        let _rug_ed_tests_llm_16_338_rrrruuuugggg_test_gauge_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_339 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    use crate::symbols;
    #[test]
    fn test_line_gauge_label() {
        let _rug_st_tests_llm_16_339_rrrruuuugggg_test_line_gauge_label = 0;
        let rug_fuzz_0 = "test label";
        let rug_fuzz_1 = "test label";
        let mut gauge = LineGauge::default();
        gauge = gauge.label(rug_fuzz_0);
        let expected_label = Some(Spans::from(rug_fuzz_1));
        debug_assert_eq!(gauge.label, expected_label);
        let _rug_ed_tests_llm_16_339_rrrruuuugggg_test_line_gauge_label = 0;
    }
    #[test]
    fn test_line_gauge_label_into_spans() {
        let _rug_st_tests_llm_16_339_rrrruuuugggg_test_line_gauge_label_into_spans = 0;
        let rug_fuzz_0 = "test label";
        let rug_fuzz_1 = "test label";
        let mut gauge = LineGauge::default();
        gauge = gauge.label(Spans::from(rug_fuzz_0));
        let expected_label = Some(Spans::from(rug_fuzz_1));
        debug_assert_eq!(gauge.label, expected_label);
        let _rug_ed_tests_llm_16_339_rrrruuuugggg_test_line_gauge_label_into_spans = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_343 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_style() {
        let _rug_st_tests_llm_16_343_rrrruuuugggg_test_style = 0;
        let style = Style::default().fg(Color::Blue);
        let expected = Style {
            fg: Some(Color::Blue),
            bg: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
        };
        debug_assert_eq!(style.patch(Style::default()), expected);
        debug_assert_eq!(style.patch(Style::default().fg(Color::Red)), expected);
        let _rug_ed_tests_llm_16_343_rrrruuuugggg_test_style = 0;
    }
    #[test]
    fn test_line_gauge_style() {
        let _rug_st_tests_llm_16_343_rrrruuuugggg_test_line_gauge_style = 0;
        let mut gauge = LineGauge::default();
        let style = Style::default();
        gauge = gauge.style(style);
        debug_assert_eq!(gauge.style, style);
        let gauge_style = Style::default();
        gauge = gauge.gauge_style(gauge_style);
        debug_assert_eq!(gauge.gauge_style, gauge_style);
        let _rug_ed_tests_llm_16_343_rrrruuuugggg_test_line_gauge_style = 0;
    }
}
#[cfg(test)]
mod tests_rug_101 {
    use super::*;
    use crate::widgets::gauge::Gauge;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_101_rrrruuuugggg_test_rug = 0;
        let gauge: Gauge<'static> = Default::default();
        let _rug_ed_tests_rug_101_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_102 {
    use super::*;
    use crate::widgets::{Gauge, Block};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_102_rrrruuuugggg_test_rug = 0;
        let mut p0: Gauge<'static> = Gauge::default();
        let mut p1: Block<'static> = Block::default();
        Gauge::block(p0, p1);
        let _rug_ed_tests_rug_102_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_103 {
    use super::*;
    use crate::widgets::Gauge;
    #[test]
    fn test_ratio() {
        let _rug_st_tests_rug_103_rrrruuuugggg_test_ratio = 0;
        let rug_fuzz_0 = 0.5;
        let mut p0: Gauge<'static> = Gauge::default();
        let p1: f64 = rug_fuzz_0;
        p0.ratio(p1);
        let _rug_ed_tests_rug_103_rrrruuuugggg_test_ratio = 0;
    }
}
#[cfg(test)]
mod tests_rug_104 {
    use super::*;
    use crate::widgets::Gauge;
    use crate::style::Color;
    use crate::text::Span;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_104_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample label";
        let mut p0: Gauge<'static> = Gauge::default();
        let p1: Span<'static> = Span::from(rug_fuzz_0).into();
        p0.label(p1);
        let _rug_ed_tests_rug_104_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_105 {
    use super::*;
    use crate::widgets::Widget;
    use crate::widgets::Gauge;
    use crate::layout::Rect;
    use crate::buffer::{Buffer, Cell};
    use crate::style::{Style, Color};
    #[test]
    fn test_render() {
        let _rug_st_tests_rug_105_rrrruuuugggg_test_render = 0;
        let mut gauge: Gauge<'static> = Gauge::default();
        let area: Rect = Rect::default();
        let mut buf: Buffer = Buffer::empty(area);
        gauge.render(area, &mut buf);
        let _rug_ed_tests_rug_105_rrrruuuugggg_test_render = 0;
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::widgets::gauge::LineGauge;
    use crate::widgets::block::Block;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_107_rrrruuuugggg_test_rug = 0;
        let mut p0: LineGauge<'static> = LineGauge::default();
        let mut p1: Block<'static> = Block::default();
        p0.block(p1);
        let _rug_ed_tests_rug_107_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_108 {
    use super::*;
    use crate::widgets::gauge::LineGauge;
    #[test]
    fn test_ratio() {
        let _rug_st_tests_rug_108_rrrruuuugggg_test_ratio = 0;
        let rug_fuzz_0 = 0.5;
        let mut p0 = LineGauge::default();
        let p1 = rug_fuzz_0;
        p0.ratio(p1);
        let _rug_ed_tests_rug_108_rrrruuuugggg_test_ratio = 0;
    }
}
#[cfg(test)]
mod tests_rug_109 {
    use super::*;
    use crate::widgets::gauge::LineGauge;
    use crate::widgets::gauge::symbols::line;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_109_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "|";
        let rug_fuzz_1 = "-";
        let rug_fuzz_2 = "-";
        let rug_fuzz_3 = "-";
        let rug_fuzz_4 = "-";
        let rug_fuzz_5 = "-";
        let rug_fuzz_6 = "|";
        let rug_fuzz_7 = "|";
        let rug_fuzz_8 = "-";
        let rug_fuzz_9 = "-";
        let rug_fuzz_10 = "+";
        let mut p0: LineGauge<'static> = LineGauge::default();
        let mut p1 = line::Set {
            vertical: rug_fuzz_0,
            horizontal: rug_fuzz_1,
            top_right: rug_fuzz_2,
            top_left: rug_fuzz_3,
            bottom_right: rug_fuzz_4,
            bottom_left: rug_fuzz_5,
            vertical_left: rug_fuzz_6,
            vertical_right: rug_fuzz_7,
            horizontal_down: rug_fuzz_8,
            horizontal_up: rug_fuzz_9,
            cross: rug_fuzz_10,
        };
        p0 = p0.line_set(p1);
        let _rug_ed_tests_rug_109_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use crate::widgets::{Widget, gauge::LineGauge};
    use crate::layout::Rect;
    use crate::buffer::{Buffer, Cell};
    use crate::style::{Style, Color};
    #[test]
    fn test_render() {
        let _rug_st_tests_rug_110_rrrruuuugggg_test_render = 0;
        let mut p0: LineGauge<'static> = LineGauge::default();
        let mut p1: Rect = Rect::default();
        let mut p2: &mut Buffer = &mut Buffer::empty(p1);
        p0.render(p1, p2);
        let _rug_ed_tests_rug_110_rrrruuuugggg_test_render = 0;
    }
}
