use crate::{
    buffer::Buffer, layout::Rect, style::Style, symbols, text::{Span, Spans},
    widgets::{Block, Widget},
};
/// A widget to display available tabs in a multiple panels context.
///
/// # Examples
///
/// ```
/// # use tui::widgets::{Block, Borders, Tabs};
/// # use tui::style::{Style, Color};
/// # use tui::text::{Spans};
/// # use tui::symbols::{DOT};
/// let titles = ["Tab1", "Tab2", "Tab3", "Tab4"].iter().cloned().map(Spans::from).collect();
/// Tabs::new(titles)
///     .block(Block::default().title("Tabs").borders(Borders::ALL))
///     .style(Style::default().fg(Color::White))
///     .highlight_style(Style::default().fg(Color::Yellow))
///     .divider(DOT);
/// ```
#[derive(Debug, Clone)]
pub struct Tabs<'a> {
    /// A block to wrap this widget in if necessary
    block: Option<Block<'a>>,
    /// One title for each tab
    titles: Vec<Spans<'a>>,
    /// The index of the selected tabs
    selected: usize,
    /// The style used to draw the text
    style: Style,
    /// Style to apply to the selected item
    highlight_style: Style,
    /// Tab divider
    divider: Span<'a>,
}
impl<'a> Tabs<'a> {
    pub fn new(titles: Vec<Spans<'a>>) -> Tabs<'a> {
        Tabs {
            block: None,
            titles,
            selected: 0,
            style: Default::default(),
            highlight_style: Default::default(),
            divider: Span::raw(symbols::line::VERTICAL),
        }
    }
    pub fn block(mut self, block: Block<'a>) -> Tabs<'a> {
        self.block = Some(block);
        self
    }
    pub fn select(mut self, selected: usize) -> Tabs<'a> {
        self.selected = selected;
        self
    }
    pub fn style(mut self, style: Style) -> Tabs<'a> {
        self.style = style;
        self
    }
    pub fn highlight_style(mut self, style: Style) -> Tabs<'a> {
        self.highlight_style = style;
        self
    }
    pub fn divider<T>(mut self, divider: T) -> Tabs<'a>
    where
        T: Into<Span<'a>>,
    {
        self.divider = divider.into();
        self
    }
}
impl<'a> Widget for Tabs<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let tabs_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        if tabs_area.height < 1 {
            return;
        }
        let mut x = tabs_area.left();
        let titles_length = self.titles.len();
        for (i, title) in self.titles.into_iter().enumerate() {
            let last_title = titles_length - 1 == i;
            x = x.saturating_add(1);
            let remaining_width = tabs_area.right().saturating_sub(x);
            if remaining_width == 0 {
                break;
            }
            let pos = buf.set_spans(x, tabs_area.top(), &title, remaining_width);
            if i == self.selected {
                buf.set_style(
                    Rect {
                        x,
                        y: tabs_area.top(),
                        width: pos.0.saturating_sub(x),
                        height: 1,
                    },
                    self.highlight_style,
                );
            }
            x = pos.0.saturating_add(1);
            let remaining_width = tabs_area.right().saturating_sub(x);
            if remaining_width == 0 || last_title {
                break;
            }
            let pos = buf.set_span(x, tabs_area.top(), &self.divider, remaining_width);
            x = pos.0;
        }
    }
}
#[cfg(test)]
mod tests_llm_16_127 {
    use super::*;
    use crate::*;
    use crate::backend::TestBackend;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crate::style::Style;
    use crate::symbols::line::VERTICAL;
    use crate::symbols::DOT;
    use crate::text::Spans;
    use crate::widgets::{Block, Borders, Widget};
    #[test]
    fn test_tabs_render() {
        let _rug_st_tests_llm_16_127_rrrruuuugggg_test_tabs_render = 0;
        let rug_fuzz_0 = "Tab1";
        let rug_fuzz_1 = "Tabs";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 10;
        let rug_fuzz_6 = 4;
        let rug_fuzz_7 = "┌Tabs┐     ";
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 1;
        let titles = vec![
            Spans::from(rug_fuzz_0), Spans::from("Tab2"), Spans::from("Tab3"),
            Spans::from("Tab4")
        ];
        let tabs = Tabs::new(titles)
            .block(Block::default().title(rug_fuzz_1).borders(Borders::ALL))
            .style(Style::default())
            .highlight_style(Style::default())
            .divider(DOT)
            .select(rug_fuzz_2);
        let area = Rect::new(rug_fuzz_3, rug_fuzz_4, rug_fuzz_5, rug_fuzz_6);
        let mut buf = Buffer::empty(area);
        tabs.render(area, &mut buf);
        let expected = vec![
            rug_fuzz_7, "│Tab1│     ", "│·--·│     ", "└────┘     "
        ];
        let mut i = rug_fuzz_8;
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                debug_assert_eq!(
                    buf.get(x, y).symbol, expected[i].chars().nth(x as usize).unwrap()
                    .to_string()
                );
            }
            i += rug_fuzz_9;
        }
        let _rug_ed_tests_llm_16_127_rrrruuuugggg_test_tabs_render = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_398 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    use crate::symbols::line::VERTICAL;
    #[test]
    fn test_style() {
        let _rug_st_tests_llm_16_398_rrrruuuugggg_test_style = 0;
        let mut tabs = Tabs::new(vec![]);
        let style = Style::default().fg(Color::Blue);
        tabs = tabs.style(style.clone());
        debug_assert_eq!(tabs.style, style);
        let _rug_ed_tests_llm_16_398_rrrruuuugggg_test_style = 0;
    }
    #[test]
    fn test_highlight_style() {
        let _rug_st_tests_llm_16_398_rrrruuuugggg_test_highlight_style = 0;
        let mut tabs = Tabs::new(vec![]);
        let highlight_style = Style::default().fg(Color::Yellow);
        tabs = tabs.highlight_style(highlight_style.clone());
        debug_assert_eq!(tabs.highlight_style, highlight_style);
        let _rug_ed_tests_llm_16_398_rrrruuuugggg_test_highlight_style = 0;
    }
    #[test]
    fn test_divider() {
        let _rug_st_tests_llm_16_398_rrrruuuugggg_test_divider = 0;
        let mut tabs = Tabs::new(vec![]);
        let divider = Span::raw(VERTICAL);
        tabs = tabs.divider(divider.clone());
        debug_assert_eq!(tabs.divider, divider);
        let _rug_ed_tests_llm_16_398_rrrruuuugggg_test_divider = 0;
    }
}
#[cfg(test)]
mod tests_rug_136 {
    use super::*;
    use crate::symbols;
    use crate::text::{Span, Spans};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_136_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "Sample Text";
        #[cfg(test)]
        mod tests_rug_136_prepare {
            use crate::text::{Span, Spans};
            #[test]
            fn sample() {
                let _rug_st_tests_rug_136_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "Sample Text";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_136_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v98: Vec<Spans<'static>> = Vec::new();
                let spans: Spans<'static> = Spans::from(vec![Span::raw(rug_fuzz_0)]);
                v98.push(spans);
                let _rug_ed_tests_rug_136_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_136_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let p0: Vec<Spans<'static>> = vec![Spans::from(vec![Span::raw("Sample Text")])];
        Tabs::new(p0);
        let _rug_ed_tests_rug_136_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_137 {
    use super::*;
    use crate::widgets::tabs::Tabs;
    use crate::widgets::block::Block;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_137_rrrruuuugggg_test_rug = 0;
        let mut p0: Tabs<'static> = Tabs::new(vec![]);
        let mut p1: Block<'static> = Block::default();
        Tabs::<'static>::block(p0, p1);
        let _rug_ed_tests_rug_137_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_138 {
    use super::*;
    use crate::widgets::tabs::Tabs;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_138_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: Tabs<'static> = Tabs::new(vec![]);
        let p1: usize = rug_fuzz_0;
        let result = p0.select(p1);
        let _rug_ed_tests_rug_138_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::widgets::tabs::Tabs;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_rug = 0;
        let mut p0: Tabs<'static> = Tabs::new(vec![]);
        let mut p1 = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        p0.highlight_style(p1);
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use crate::widgets::tabs::Tabs;
    use crate::text::Span;
    #[test]
    fn test_divider() {
        let _rug_st_tests_rug_140_rrrruuuugggg_test_divider = 0;
        let rug_fuzz_0 = "some_span_text";
        let mut p0: Tabs<'static> = Tabs::new(vec![]);
        let p1: Span<'static> = Span::from(rug_fuzz_0);
        p0.divider(p1);
        let _rug_ed_tests_rug_140_rrrruuuugggg_test_divider = 0;
    }
}
