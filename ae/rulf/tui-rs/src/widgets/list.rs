use crate::{
    buffer::Buffer, layout::{Corner, Rect},
    style::Style, text::Text, widgets::{Block, StatefulWidget, Widget},
};
use std::iter::{self, Iterator};
use unicode_width::UnicodeWidthStr;
#[derive(Debug, Clone)]
pub struct ListState {
    offset: usize,
    selected: Option<usize>,
}
impl Default for ListState {
    fn default() -> ListState {
        ListState {
            offset: 0,
            selected: None,
        }
    }
}
impl ListState {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if index.is_none() {
            self.offset = 0;
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem<'a> {
    content: Text<'a>,
    style: Style,
}
impl<'a> ListItem<'a> {
    pub fn new<T>(content: T) -> ListItem<'a>
    where
        T: Into<Text<'a>>,
    {
        ListItem {
            content: content.into(),
            style: Style::default(),
        }
    }
    pub fn style(mut self, style: Style) -> ListItem<'a> {
        self.style = style;
        self
    }
    pub fn height(&self) -> usize {
        self.content.height()
    }
}
/// A widget to display several items among which one can be selected (optional)
///
/// # Examples
///
/// ```
/// # use tui::widgets::{Block, Borders, List, ListItem};
/// # use tui::style::{Style, Color, Modifier};
/// let items = [ListItem::new("Item 1"), ListItem::new("Item 2"), ListItem::new("Item 3")];
/// List::new(items)
///     .block(Block::default().title("List").borders(Borders::ALL))
///     .style(Style::default().fg(Color::White))
///     .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
///     .highlight_symbol(">>");
/// ```
#[derive(Debug, Clone)]
pub struct List<'a> {
    block: Option<Block<'a>>,
    items: Vec<ListItem<'a>>,
    /// Style used as a base style for the widget
    style: Style,
    start_corner: Corner,
    /// Style used to render selected item
    highlight_style: Style,
    /// Symbol in front of the selected item (Shift all items to the right)
    highlight_symbol: Option<&'a str>,
}
impl<'a> List<'a> {
    pub fn new<T>(items: T) -> List<'a>
    where
        T: Into<Vec<ListItem<'a>>>,
    {
        List {
            block: None,
            style: Style::default(),
            items: items.into(),
            start_corner: Corner::TopLeft,
            highlight_style: Style::default(),
            highlight_symbol: None,
        }
    }
    pub fn block(mut self, block: Block<'a>) -> List<'a> {
        self.block = Some(block);
        self
    }
    pub fn style(mut self, style: Style) -> List<'a> {
        self.style = style;
        self
    }
    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> List<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }
    pub fn highlight_style(mut self, style: Style) -> List<'a> {
        self.highlight_style = style;
        self
    }
    pub fn start_corner(mut self, corner: Corner) -> List<'a> {
        self.start_corner = corner;
        self
    }
}
impl<'a> StatefulWidget for List<'a> {
    type State = ListState;
    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        if list_area.width < 1 || list_area.height < 1 {
            return;
        }
        if self.items.is_empty() {
            return;
        }
        let list_height = list_area.height as usize;
        let mut start = state.offset;
        let mut end = state.offset;
        let mut height = 0;
        for item in self.items.iter().skip(state.offset) {
            if height + item.height() > list_height {
                break;
            }
            height += item.height();
            end += 1;
        }
        let selected = state.selected.unwrap_or(0).min(self.items.len() - 1);
        while selected >= end {
            height = height.saturating_add(self.items[end].height());
            end += 1;
            while height > list_height {
                height = height.saturating_sub(self.items[start].height());
                start += 1;
            }
        }
        while selected < start {
            start -= 1;
            height = height.saturating_add(self.items[start].height());
            while height > list_height {
                end -= 1;
                height = height.saturating_sub(self.items[end].height());
            }
        }
        state.offset = start;
        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let blank_symbol = iter::repeat(" ")
            .take(highlight_symbol.width())
            .collect::<String>();
        let mut current_height = 0;
        let has_selection = state.selected.is_some();
        for (i, item) in self
            .items
            .iter_mut()
            .enumerate()
            .skip(state.offset)
            .take(end - start)
        {
            let (x, y) = match self.start_corner {
                Corner::BottomLeft => {
                    current_height += item.height() as u16;
                    (list_area.left(), list_area.bottom() - current_height)
                }
                _ => {
                    let pos = (list_area.left(), list_area.top() + current_height);
                    current_height += item.height() as u16;
                    pos
                }
            };
            let area = Rect {
                x,
                y,
                width: list_area.width,
                height: item.height() as u16,
            };
            let item_style = self.style.patch(item.style);
            buf.set_style(area, item_style);
            let is_selected = state.selected.map(|s| s == i).unwrap_or(false);
            let elem_x = if has_selection {
                let symbol = if is_selected { highlight_symbol } else { &blank_symbol };
                let (x, _) = buf
                    .set_stringn(x, y, symbol, list_area.width as usize, item_style);
                x
            } else {
                x
            };
            let max_element_width = (list_area.width - (elem_x - x)) as usize;
            for (j, line) in item.content.lines.iter().enumerate() {
                buf.set_spans(elem_x, y + j as u16, line, max_element_width as u16);
            }
            if is_selected {
                buf.set_style(area, self.highlight_style);
            }
        }
    }
}
impl<'a> Widget for List<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = ListState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
#[cfg(test)]
mod tests_llm_16_112 {
    use crate::widgets::ListState;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_112_rrrruuuugggg_test_default = 0;
        let default_state: ListState = ListState::default();
        debug_assert_eq!(default_state.offset, 0);
        debug_assert_eq!(default_state.selected, None);
        let _rug_ed_tests_llm_16_112_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_346_llm_16_345 {
    use crate::widgets::list::{List, ListItem};
    use crate::style::{Color, Modifier, Style};
    use crate::layout::Corner;
    #[test]
    fn test_highlight_style() {
        let _rug_st_tests_llm_16_346_llm_16_345_rrrruuuugggg_test_highlight_style = 0;
        let rug_fuzz_0 = "Item 1";
        let style1 = Style::default().bg(Color::Red);
        let style2 = Style::default().bg(Color::Blue);
        let list = List::new(
                vec![
                    ListItem::new(rug_fuzz_0).style(style1.clone()),
                    ListItem::new("Item 2").style(style2.clone())
                ],
            )
            .highlight_style(style1.clone());
        debug_assert_eq!(list.highlight_style, style1);
        let _rug_ed_tests_llm_16_346_llm_16_345_rrrruuuugggg_test_highlight_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_353 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier};
    #[test]
    fn test_height() {
        let _rug_st_tests_llm_16_353_rrrruuuugggg_test_height = 0;
        let rug_fuzz_0 = "The first line\nThe second line";
        let rug_fuzz_1 = 2;
        let text = Text::from(rug_fuzz_0);
        let item = ListItem::new(text);
        debug_assert_eq!(rug_fuzz_1, item.height());
        let _rug_ed_tests_llm_16_353_rrrruuuugggg_test_height = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_354 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier};
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_354_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "Example content";
        let content = rug_fuzz_0;
        let item = ListItem::new(content);
        debug_assert_eq!(item.content, Text::from(content));
        debug_assert_eq!(item.style, Style::default());
        let _rug_ed_tests_llm_16_354_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_357 {
    use crate::widgets::list::ListState;
    #[test]
    fn test_select_with_index() {
        let _rug_st_tests_llm_16_357_rrrruuuugggg_test_select_with_index = 0;
        let rug_fuzz_0 = 2;
        let mut list_state = ListState::default();
        list_state.select(Some(rug_fuzz_0));
        debug_assert_eq!(list_state.selected(), Some(2));
        debug_assert_eq!(list_state.offset, 0);
        let _rug_ed_tests_llm_16_357_rrrruuuugggg_test_select_with_index = 0;
    }
    #[test]
    fn test_select_without_index() {
        let _rug_st_tests_llm_16_357_rrrruuuugggg_test_select_without_index = 0;
        let mut list_state = ListState::default();
        list_state.select(None);
        debug_assert_eq!(list_state.selected(), None);
        debug_assert_eq!(list_state.offset, 0);
        let _rug_ed_tests_llm_16_357_rrrruuuugggg_test_select_without_index = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_358 {
    use super::*;
    use crate::*;
    #[test]
    fn test_selected() {
        let _rug_st_tests_llm_16_358_rrrruuuugggg_test_selected = 0;
        let rug_fuzz_0 = 0;
        let mut list_state = ListState::default();
        debug_assert_eq!(list_state.selected(), None);
        list_state.select(Some(rug_fuzz_0));
        debug_assert_eq!(list_state.selected(), Some(0));
        let _rug_ed_tests_llm_16_358_rrrruuuugggg_test_selected = 0;
    }
}
#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use crate::widgets::list::{List, ListItem};
    use crate::style::Style;
    use crate::layout::Corner;
    #[test]
    fn test_list_new() {
        let _rug_st_tests_rug_111_rrrruuuugggg_test_list_new = 0;
        let rug_fuzz_0 = "Item 1";
        let items: Vec<ListItem<'static>> = vec![
            ListItem::new(rug_fuzz_0), ListItem::new("Item 2"), ListItem::new("Item 3")
        ];
        let list = List::new(items);
        let _rug_ed_tests_rug_111_rrrruuuugggg_test_list_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_112 {
    use super::*;
    use crate::widgets::{List, Block};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_112_rrrruuuugggg_test_rug = 0;
        let mut p0: List<'static> = List::new(Vec::new());
        let mut p1: Block<'static> = Block::default();
        p0.block(p1);
        let _rug_ed_tests_rug_112_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::style::{Color, Modifier, Style};
    use crate::widgets::List;
    #[test]
    fn test_style() {
        let _rug_st_tests_rug_113_rrrruuuugggg_test_style = 0;
        let mut p0: List<'static> = List::new(Vec::new());
        let mut p1 = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        p0.style(p1);
        let _rug_ed_tests_rug_113_rrrruuuugggg_test_style = 0;
    }
}
#[cfg(test)]
mod tests_rug_114 {
    use super::*;
    use crate::widgets::List;
    #[test]
    fn test_highlight_symbol() {
        let _rug_st_tests_rug_114_rrrruuuugggg_test_highlight_symbol = 0;
        let rug_fuzz_0 = "sample_highlight_symbol";
        let mut p0: List<'static> = List::new(Vec::new());
        let p1: &str = rug_fuzz_0;
        p0.highlight_symbol(&p1);
        let _rug_ed_tests_rug_114_rrrruuuugggg_test_highlight_symbol = 0;
    }
}
#[cfg(test)]
mod tests_rug_115 {
    use super::*;
    use crate::widgets::List;
    use crate::layout::Corner;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_115_rrrruuuugggg_test_rug = 0;
        let mut p0: List<'static> = List::new(Vec::new());
        let mut p1: Corner = Corner::TopLeft;
        p0.start_corner(p1);
        let _rug_ed_tests_rug_115_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use crate::{
        layout::Rect, buffer::{Buffer, Cell},
        widgets::Widget,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_117_rrrruuuugggg_test_rug = 0;
        let mut p0: crate::widgets::list::List<'static> = crate::widgets::List::new(
            Vec::new(),
        );
        let mut p1: Rect = Rect::default();
        let mut p2: Buffer = Buffer::empty(p1);
        <crate::widgets::list::List<
            'static,
        > as crate::widgets::Widget>::render(p0, p1, &mut p2);
        let _rug_ed_tests_rug_117_rrrruuuugggg_test_rug = 0;
    }
}
