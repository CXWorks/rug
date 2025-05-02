use crate::{
    buffer::Buffer, layout::Rect, style::Style, symbols, widgets::{Block, Widget},
};
use std::cmp::min;
/// Widget to render a sparkline over one or more lines.
///
/// # Examples
///
/// ```
/// # use tui::widgets::{Block, Borders, Sparkline};
/// # use tui::style::{Style, Color};
/// Sparkline::default()
///     .block(Block::default().title("Sparkline").borders(Borders::ALL))
///     .data(&[0, 2, 3, 4, 1, 4, 10])
///     .max(5)
///     .style(Style::default().fg(Color::Red).bg(Color::White));
/// ```
#[derive(Debug, Clone)]
pub struct Sparkline<'a> {
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
    /// A slice of the data to display
    data: &'a [u64],
    /// The maximum value to take to compute the maximum bar height (if nothing is specified, the
    /// widget uses the max of the dataset)
    max: Option<u64>,
    /// A set of bar symbols used to represent the give data
    bar_set: symbols::bar::Set,
}
impl<'a> Default for Sparkline<'a> {
    fn default() -> Sparkline<'a> {
        Sparkline {
            block: None,
            style: Default::default(),
            data: &[],
            max: None,
            bar_set: symbols::bar::NINE_LEVELS,
        }
    }
}
impl<'a> Sparkline<'a> {
    pub fn block(mut self, block: Block<'a>) -> Sparkline<'a> {
        self.block = Some(block);
        self
    }
    pub fn style(mut self, style: Style) -> Sparkline<'a> {
        self.style = style;
        self
    }
    pub fn data(mut self, data: &'a [u64]) -> Sparkline<'a> {
        self.data = data;
        self
    }
    pub fn max(mut self, max: u64) -> Sparkline<'a> {
        self.max = Some(max);
        self
    }
    pub fn bar_set(mut self, bar_set: symbols::bar::Set) -> Sparkline<'a> {
        self.bar_set = bar_set;
        self
    }
}
impl<'a> Widget for Sparkline<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let spark_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        if spark_area.height < 1 {
            return;
        }
        let max = match self.max {
            Some(v) => v,
            None => *self.data.iter().max().unwrap_or(&1u64),
        };
        let max_index = min(spark_area.width as usize, self.data.len());
        let mut data = self
            .data
            .iter()
            .take(max_index)
            .map(|e| {
                if max != 0 { e * u64::from(spark_area.height) * 8 / max } else { 0 }
            })
            .collect::<Vec<u64>>();
        for j in (0..spark_area.height).rev() {
            for (i, d) in data.iter_mut().enumerate() {
                let symbol = match *d {
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
                buf.get_mut(spark_area.left() + i as u16, spark_area.top() + j)
                    .set_symbol(symbol)
                    .set_style(self.style);
                if *d > 8 {
                    *d -= 8;
                } else {
                    *d = 0;
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_does_not_panic_if_max_is_zero() {
        let widget = Sparkline::default().data(&[0, 0, 0]);
        let area = Rect::new(0, 0, 3, 1);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
    }
    #[test]
    fn it_does_not_panic_if_max_is_set_to_zero() {
        let widget = Sparkline::default().data(&[0, 1, 2]).max(0);
        let area = Rect::new(0, 0, 3, 1);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
    }
}
#[cfg(test)]
mod tests_llm_16_121 {
    use super::*;
    use crate::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crate::style::{Color, Style};
    #[test]
    fn test_sparkline_render() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        let sparkline = Sparkline::default()
            .data(&[0, 2, 3, 4, 1, 4, 10])
            .max(5)
            .style(Style::default().fg(Color::Red).bg(Color::White));
        let area = Rect::new(0, 0, 10, 5);
        sparkline.render(area, &mut buf);
    }
}
#[cfg(test)]
mod tests_llm_16_375 {
    use super::*;
    use crate::*;
    #[test]
    fn test_bar_set() {
        let _rug_st_tests_llm_16_375_rrrruuuugggg_test_bar_set = 0;
        let rug_fuzz_0 = "█";
        let rug_fuzz_1 = "▇";
        let rug_fuzz_2 = "▆";
        let rug_fuzz_3 = "▅";
        let rug_fuzz_4 = "▄";
        let rug_fuzz_5 = "▃";
        let rug_fuzz_6 = "▂";
        let rug_fuzz_7 = "▁";
        let rug_fuzz_8 = " ";
        let bar_set = symbols::bar::Set {
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
        let sparkline = Sparkline::default().bar_set(bar_set);
        debug_assert_eq!(sparkline.bar_set.full, "█");
        debug_assert_eq!(sparkline.bar_set.seven_eighths, "▇");
        debug_assert_eq!(sparkline.bar_set.three_quarters, "▆");
        debug_assert_eq!(sparkline.bar_set.five_eighths, "▅");
        debug_assert_eq!(sparkline.bar_set.half, "▄");
        debug_assert_eq!(sparkline.bar_set.three_eighths, "▃");
        debug_assert_eq!(sparkline.bar_set.one_quarter, "▂");
        debug_assert_eq!(sparkline.bar_set.one_eighth, "▁");
        debug_assert_eq!(sparkline.bar_set.empty, " ");
        let _rug_ed_tests_llm_16_375_rrrruuuugggg_test_bar_set = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_378 {
    use super::*;
    use crate::*;
    use crate::style::Modifier;
    #[test]
    fn test_max() {
        let mut sparkline = Sparkline::default()
            .data(&[0, 2, 3, 4, 1, 4, 10])
            .style(Style::default().add_modifier(Modifier::BOLD));
        sparkline = sparkline.max(5);
        assert_eq!(sparkline.max, Some(5));
    }
}
#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use crate::widgets::sparkline::{Sparkline, symbols};
    use crate::style::Style;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_118_rrrruuuugggg_test_default = 0;
        Sparkline::<'static>::default();
        let _rug_ed_tests_rug_118_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_119 {
    use super::*;
    use crate::widgets::sparkline::Sparkline;
    use crate::widgets::block::Block;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_119_rrrruuuugggg_test_rug = 0;
        let mut p0: Sparkline<'static> = Sparkline::default();
        let mut p1: Block<'static> = Block::default();
        crate::widgets::sparkline::Sparkline::<'static>::block(p0, p1);
        let _rug_ed_tests_rug_119_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    use crate::widgets::sparkline::Sparkline;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_120_rrrruuuugggg_test_rug = 0;
        let mut p0: Sparkline<'static> = Sparkline::default();
        let mut p1 = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        p0 = p0.style(p1);
        let _rug_ed_tests_rug_120_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::widgets::sparkline::Sparkline;
    #[test]
    fn test_data() {
        let _rug_st_tests_rug_121_rrrruuuugggg_test_data = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let mut p0: Sparkline<'static> = Sparkline::default();
        let p1: &[u64] = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        p0.data(p1);
        let _rug_ed_tests_rug_121_rrrruuuugggg_test_data = 0;
    }
}
