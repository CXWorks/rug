use crate::{
    buffer::Buffer, layout::Rect, style::Style, symbols, widgets::{Block, Widget},
};
use std::cmp::min;
use unicode_width::UnicodeWidthStr;
/// Display multiple bars in a single widgets
///
/// # Examples
///
/// ```
/// # use tui::widgets::{Block, Borders, BarChart};
/// # use tui::style::{Style, Color, Modifier};
/// BarChart::default()
///     .block(Block::default().title("BarChart").borders(Borders::ALL))
///     .bar_width(3)
///     .bar_gap(1)
///     .bar_style(Style::default().fg(Color::Yellow).bg(Color::Red))
///     .value_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
///     .label_style(Style::default().fg(Color::White))
///     .data(&[("B0", 0), ("B1", 2), ("B2", 4), ("B3", 3)])
///     .max(4);
/// ```
#[derive(Debug, Clone)]
pub struct BarChart<'a> {
    /// Block to wrap the widget in
    block: Option<Block<'a>>,
    /// The width of each bar
    bar_width: u16,
    /// The gap between each bar
    bar_gap: u16,
    /// Set of symbols used to display the data
    bar_set: symbols::bar::Set,
    /// Style of the bars
    bar_style: Style,
    /// Style of the values printed at the bottom of each bar
    value_style: Style,
    /// Style of the labels printed under each bar
    label_style: Style,
    /// Style for the widget
    style: Style,
    /// Slice of (label, value) pair to plot on the chart
    data: &'a [(&'a str, u64)],
    /// Value necessary for a bar to reach the maximum height (if no value is specified,
    /// the maximum value in the data is taken as reference)
    max: Option<u64>,
    /// Values to display on the bar (computed when the data is passed to the widget)
    values: Vec<String>,
}
impl<'a> Default for BarChart<'a> {
    fn default() -> BarChart<'a> {
        BarChart {
            block: None,
            max: None,
            data: &[],
            values: Vec::new(),
            bar_style: Style::default(),
            bar_width: 1,
            bar_gap: 1,
            bar_set: symbols::bar::NINE_LEVELS,
            value_style: Default::default(),
            label_style: Default::default(),
            style: Default::default(),
        }
    }
}
impl<'a> BarChart<'a> {
    pub fn data(mut self, data: &'a [(&'a str, u64)]) -> BarChart<'a> {
        self.data = data;
        self.values = Vec::with_capacity(self.data.len());
        for &(_, v) in self.data {
            self.values.push(format!("{}", v));
        }
        self
    }
    pub fn block(mut self, block: Block<'a>) -> BarChart<'a> {
        self.block = Some(block);
        self
    }
    pub fn max(mut self, max: u64) -> BarChart<'a> {
        self.max = Some(max);
        self
    }
    pub fn bar_style(mut self, style: Style) -> BarChart<'a> {
        self.bar_style = style;
        self
    }
    pub fn bar_width(mut self, width: u16) -> BarChart<'a> {
        self.bar_width = width;
        self
    }
    pub fn bar_gap(mut self, gap: u16) -> BarChart<'a> {
        self.bar_gap = gap;
        self
    }
    pub fn bar_set(mut self, bar_set: symbols::bar::Set) -> BarChart<'a> {
        self.bar_set = bar_set;
        self
    }
    pub fn value_style(mut self, style: Style) -> BarChart<'a> {
        self.value_style = style;
        self
    }
    pub fn label_style(mut self, style: Style) -> BarChart<'a> {
        self.label_style = style;
        self
    }
    pub fn style(mut self, style: Style) -> BarChart<'a> {
        self.style = style;
        self
    }
}
impl<'a> Widget for BarChart<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let chart_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        if chart_area.height < 2 {
            return;
        }
        let max = self
            .max
            .unwrap_or_else(|| self.data.iter().map(|t| t.1).max().unwrap_or_default());
        let max_index = min(
            (chart_area.width / (self.bar_width + self.bar_gap)) as usize,
            self.data.len(),
        );
        let mut data = self
            .data
            .iter()
            .take(max_index)
            .map(|&(l, v)| {
                (l, v * u64::from(chart_area.height - 1) * 8 / std::cmp::max(max, 1))
            })
            .collect::<Vec<(&str, u64)>>();
        for j in (0..chart_area.height - 1).rev() {
            for (i, d) in data.iter_mut().enumerate() {
                let symbol = match d.1 {
                    0 => self.bar_set.empty,
                    1 => self.bar_set.one_eighth,
                    2 => self.bar_set.one_quarter,
                    3 => self.bar_set.three_eighths,
                    4 => self.bar_set.half,
                    5 => self.bar_set.five_eighths,
                    6 => self.bar_set.three_quarters,
                    7 => self.bar_set.seven_eighths,
                    _ => self.bar_set.full,
                };
                for x in 0..self.bar_width {
                    buf.get_mut(
                            chart_area.left()
                                + i as u16 * (self.bar_width + self.bar_gap) + x,
                            chart_area.top() + j,
                        )
                        .set_symbol(symbol)
                        .set_style(self.bar_style);
                }
                if d.1 > 8 {
                    d.1 -= 8;
                } else {
                    d.1 = 0;
                }
            }
        }
        for (i, &(label, value)) in self.data.iter().take(max_index).enumerate() {
            if value != 0 {
                let value_label = &self.values[i];
                let width = value_label.width() as u16;
                if width < self.bar_width {
                    buf.set_string(
                        chart_area.left() + i as u16 * (self.bar_width + self.bar_gap)
                            + (self.bar_width - width) / 2,
                        chart_area.bottom() - 2,
                        value_label,
                        self.value_style,
                    );
                }
            }
            buf.set_stringn(
                chart_area.left() + i as u16 * (self.bar_width + self.bar_gap),
                chart_area.bottom() - 1,
                label,
                self.bar_width as usize,
                self.label_style,
            );
        }
    }
}
#[cfg(test)]
mod tests_llm_16_58_llm_16_57 {
    use super::*;
    use crate::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crate::style::{Color, Modifier, Style};
    use crate::symbols::bar::Set;
    use crate::widgets::{BarChart, Block, Borders, Widget};
    #[test]
    fn test_render() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        let area = Rect::new(0, 0, 10, 10);
        let mut chart = BarChart::default()
            .block(Block::default().title("BarChart").borders(Borders::ALL))
            .bar_width(3)
            .bar_gap(1)
            .bar_set(Set {
                full: "@",
                seven_eighths: "&",
                three_quarters: "3",
                five_eighths: "5",
                half: "%",
                three_eighths: "3",
                one_quarter: "1",
                one_eighth: "1",
                empty: " ",
            })
            .bar_style(Style::default().fg(Color::Yellow).bg(Color::Red))
            .value_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .label_style(Style::default().fg(Color::White))
            .data(&[("B0", 0), ("B1", 2), ("B2", 4), ("B3", 3)])
            .max(4);
        chart.render(area, &mut buf);
    }
}
#[cfg(test)]
mod tests_llm_16_256 {
    use super::*;
    use crate::*;
    use crate::style::Modifier;
    use crate::symbols::bar::Set;
    #[test]
    fn test_bar_gap() {
        let _rug_st_tests_llm_16_256_rrrruuuugggg_test_bar_gap = 0;
        let rug_fuzz_0 = 2;
        let mut barchart = BarChart::default();
        let gap = rug_fuzz_0;
        barchart = barchart.bar_gap(gap);
        debug_assert_eq!(barchart.bar_gap, gap);
        let _rug_ed_tests_llm_16_256_rrrruuuugggg_test_bar_gap = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_264 {
    use crate::widgets::barchart::BarChart;
    use crate::style::{Style, Color, Modifier};
    #[test]
    fn test_label_style() {
        let _rug_st_tests_llm_16_264_rrrruuuugggg_test_label_style = 0;
        let expected = BarChart::default().label_style(Style::default().fg(Color::Blue));
        let actual = BarChart::default().label_style(Style::default().fg(Color::Blue));
        debug_assert_eq!(expected.label_style, actual.label_style);
        let _rug_ed_tests_llm_16_264_rrrruuuugggg_test_label_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_268 {
    use super::*;
    use crate::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crate::style::{Color, Modifier, Style};
    use crate::symbols::bar::Set;
    use crate::widgets::{Block, Borders, Widget};
    #[test]
    fn test_value_style() {
        let _rug_st_tests_llm_16_268_rrrruuuugggg_test_value_style = 0;
        let mut bar_chart = BarChart::default();
        let style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        bar_chart = bar_chart.value_style(style.clone());
        let expected_value_style = style;
        debug_assert_eq!(bar_chart.value_style, expected_value_style);
        let _rug_ed_tests_llm_16_268_rrrruuuugggg_test_value_style = 0;
    }
    #[test]
    fn test_value_style_render() {
        let _rug_st_tests_llm_16_268_rrrruuuugggg_test_value_style_render = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let bar_chart = BarChart::default().value_style(Style::default().fg(Color::Red));
        let area = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let mut buffer = Buffer::empty(area);
        bar_chart.render(area, &mut buffer);
        let test_style = Style::default().fg(Color::Red);
        let mut style_count = rug_fuzz_4;
        for cell in buffer.content().iter() {
            if cell.style() == test_style {
                style_count += rug_fuzz_5;
            }
        }
        debug_assert_eq!(style_count, buffer.content().len());
        let _rug_ed_tests_llm_16_268_rrrruuuugggg_test_value_style_render = 0;
    }
}
#[cfg(test)]
mod tests_rug_70 {
    use super::*;
    use crate::widgets::barchart::BarChart;
    use crate::symbols;
    use crate::style::Style;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_70_rrrruuuugggg_test_default = 0;
        let chart: BarChart<'static> = <BarChart<'static> as Default>::default();
        let _rug_ed_tests_rug_70_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_71 {
    use super::*;
    use crate::widgets::barchart::BarChart;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_71_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "A";
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = "B";
        let rug_fuzz_3 = 15;
        let rug_fuzz_4 = "C";
        let rug_fuzz_5 = 20;
        let mut data: [(&'static str, u64); 3] = [
            (rug_fuzz_0, rug_fuzz_1),
            (rug_fuzz_2, rug_fuzz_3),
            (rug_fuzz_4, rug_fuzz_5),
        ];
        let mut bar_chart: BarChart<'static> = BarChart::default();
        bar_chart.data(&data);
        let _rug_ed_tests_rug_71_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_72 {
    use super::*;
    use crate::widgets::{BarChart, Block};
    #[test]
    fn test_block() {
        let _rug_st_tests_rug_72_rrrruuuugggg_test_block = 0;
        let mut p0: BarChart<'static> = BarChart::default();
        let mut p1: Block<'static> = Block::default();
        BarChart::block(p0, p1);
        let _rug_ed_tests_rug_72_rrrruuuugggg_test_block = 0;
    }
}
#[cfg(test)]
mod tests_rug_73 {
    use super::*;
    use crate::widgets::BarChart;
    #[test]
    fn test_max() {
        let _rug_st_tests_rug_73_rrrruuuugggg_test_max = 0;
        let rug_fuzz_0 = 100;
        let mut p0: BarChart<'static> = BarChart::default();
        let p1: u64 = rug_fuzz_0;
        p0.max(p1);
        let _rug_ed_tests_rug_73_rrrruuuugggg_test_max = 0;
    }
}
#[cfg(test)]
mod tests_rug_74 {
    use super::*;
    use crate::widgets::barchart::BarChart;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_bar_style() {
        let _rug_st_tests_rug_74_rrrruuuugggg_test_bar_style = 0;
        let mut p0: BarChart<'static> = BarChart::default();
        let mut p1: Style = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        p0.bar_style(p1);
        let _rug_ed_tests_rug_74_rrrruuuugggg_test_bar_style = 0;
    }
}
#[cfg(test)]
mod tests_rug_75 {
    use super::*;
    use crate::widgets::BarChart;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_75_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0: BarChart<'static> = BarChart::default();
        let mut p1: u16 = rug_fuzz_0;
        p0.bar_width(p1);
        let _rug_ed_tests_rug_75_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_76 {
    use super::*;
    use crate::widgets::BarChart;
    use crate::symbols::bar::Set;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_76_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "█";
        let rug_fuzz_1 = "▉";
        let rug_fuzz_2 = "▊";
        let rug_fuzz_3 = "▋";
        let rug_fuzz_4 = "▌";
        let rug_fuzz_5 = "▍";
        let rug_fuzz_6 = "▎";
        let rug_fuzz_7 = "▏";
        let rug_fuzz_8 = " ";
        let mut v72: BarChart<'static> = BarChart::default();
        let mut v74 = Set {
            full: rug_fuzz_0,
            seven_eighths: rug_fuzz_1,
            three_quarters: rug_fuzz_2,
            five_eighths: rug_fuzz_3,
            half: rug_fuzz_4,
            three_eighths: rug_fuzz_5,
            one_quarter: rug_fuzz_6,
            one_eighth: rug_fuzz_7,
            empty: rug_fuzz_8,
        };
        v72.bar_set(v74);
        let _rug_ed_tests_rug_76_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_77 {
    use super::*;
    use crate::widgets::BarChart;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_77_rrrruuuugggg_test_rug = 0;
        let mut p0: BarChart<'static> = BarChart::default();
        let mut p1 = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        crate::widgets::barchart::BarChart::<'static>::style(p0, p1);
        let _rug_ed_tests_rug_77_rrrruuuugggg_test_rug = 0;
    }
}
