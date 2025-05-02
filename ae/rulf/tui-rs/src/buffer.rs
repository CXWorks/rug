use crate::{
    layout::Rect, style::{Color, Modifier, Style},
    text::{Span, Spans},
};
use std::cmp::min;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
/// A buffer cell
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
}
impl Cell {
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        self
    }
    pub fn set_char(&mut self, ch: char) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push(ch);
        self
    }
    pub fn set_fg(&mut self, color: Color) -> &mut Cell {
        self.fg = color;
        self
    }
    pub fn set_bg(&mut self, color: Color) -> &mut Cell {
        self.bg = color;
        self
    }
    pub fn set_style(&mut self, style: Style) -> &mut Cell {
        if let Some(c) = style.fg {
            self.fg = c;
        }
        if let Some(c) = style.bg {
            self.bg = c;
        }
        self.modifier.insert(style.add_modifier);
        self.modifier.remove(style.sub_modifier);
        self
    }
    pub fn style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg).add_modifier(self.modifier)
    }
    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.fg = Color::Reset;
        self.bg = Color::Reset;
        self.modifier = Modifier::empty();
    }
}
impl Default for Cell {
    fn default() -> Cell {
        Cell {
            symbol: " ".into(),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
        }
    }
}
/// A buffer that maps to the desired content of the terminal after the draw call
///
/// No widget in the library interacts directly with the terminal. Instead each of them is required
/// to draw their state to an intermediate buffer. It is basically a grid where each cell contains
/// a grapheme, a foreground color and a background color. This grid will then be used to output
/// the appropriate escape sequences and characters to draw the UI as the user has defined it.
///
/// # Examples:
///
/// ```
/// use tui::buffer::{Buffer, Cell};
/// use tui::layout::Rect;
/// use tui::style::{Color, Style, Modifier};
///
/// let mut buf = Buffer::empty(Rect{x: 0, y: 0, width: 10, height: 5});
/// buf.get_mut(0, 2).set_symbol("x");
/// assert_eq!(buf.get(0, 2).symbol, "x");
/// buf.set_string(3, 0, "string", Style::default().fg(Color::Red).bg(Color::White));
/// assert_eq!(buf.get(5, 0), &Cell{
///     symbol: String::from("r"),
///     fg: Color::Red,
///     bg: Color::White,
///     modifier: Modifier::empty()
/// });
/// buf.get_mut(5, 0).set_char('x');
/// assert_eq!(buf.get(5, 0).symbol, "x");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Buffer {
    /// The area represented by this buffer
    pub area: Rect,
    /// The content of the buffer. The length of this Vec should always be equal to area.width *
    /// area.height
    pub content: Vec<Cell>,
}
impl Default for Buffer {
    fn default() -> Buffer {
        Buffer {
            area: Default::default(),
            content: Vec::new(),
        }
    }
}
impl Buffer {
    /// Returns a Buffer with all cells set to the default one
    pub fn empty(area: Rect) -> Buffer {
        let cell: Cell = Default::default();
        Buffer::filled(area, &cell)
    }
    /// Returns a Buffer with all cells initialized with the attributes of the given Cell
    pub fn filled(area: Rect, cell: &Cell) -> Buffer {
        let size = area.area() as usize;
        let mut content = Vec::with_capacity(size);
        for _ in 0..size {
            content.push(cell.clone());
        }
        Buffer { area, content }
    }
    /// Returns a Buffer containing the given lines
    pub fn with_lines<S>(lines: Vec<S>) -> Buffer
    where
        S: AsRef<str>,
    {
        let height = lines.len() as u16;
        let width = lines
            .iter()
            .map(|i| i.as_ref().width() as u16)
            .max()
            .unwrap_or_default();
        let mut buffer = Buffer::empty(Rect { x: 0, y: 0, width, height });
        for (y, line) in lines.iter().enumerate() {
            buffer.set_string(0, y as u16, line, Style::default());
        }
        buffer
    }
    /// Returns the content of the buffer as a slice
    pub fn content(&self) -> &[Cell] {
        &self.content
    }
    /// Returns the area covered by this buffer
    pub fn area(&self) -> &Rect {
        &self.area
    }
    /// Returns a reference to Cell at the given coordinates
    pub fn get(&self, x: u16, y: u16) -> &Cell {
        let i = self.index_of(x, y);
        &self.content[i]
    }
    /// Returns a mutable reference to Cell at the given coordinates
    pub fn get_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        let i = self.index_of(x, y);
        &mut self.content[i]
    }
    /// Returns the index in the Vec<Cell> for the given global (x, y) coordinates.
    ///
    /// Global coordinates are offset by the Buffer's area offset (`x`/`y`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use tui::buffer::Buffer;
    /// # use tui::layout::Rect;
    /// let rect = Rect::new(200, 100, 10, 10);
    /// let buffer = Buffer::empty(rect);
    /// // Global coordinates to the top corner of this buffer's area
    /// assert_eq!(buffer.index_of(200, 100), 0);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when given an coordinate that is outside of this Buffer's area.
    ///
    /// ```should_panic
    /// # use tui::buffer::Buffer;
    /// # use tui::layout::Rect;
    /// let rect = Rect::new(200, 100, 10, 10);
    /// let buffer = Buffer::empty(rect);
    /// // Top coordinate is outside of the buffer in global coordinate space, as the Buffer's area
    /// // starts at (200, 100).
    /// buffer.index_of(0, 0); // Panics
    /// ```
    pub fn index_of(&self, x: u16, y: u16) -> usize {
        debug_assert!(
            x >= self.area.left() && x < self.area.right() && y >= self.area.top() && y <
            self.area.bottom(),
            "Trying to access position outside the buffer: x={}, y={}, area={:?}", x, y,
            self.area
        );
        ((y - self.area.y) * self.area.width + (x - self.area.x)) as usize
    }
    /// Returns the (global) coordinates of a cell given its index
    ///
    /// Global coordinates are offset by the Buffer's area offset (`x`/`y`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use tui::buffer::Buffer;
    /// # use tui::layout::Rect;
    /// let rect = Rect::new(200, 100, 10, 10);
    /// let buffer = Buffer::empty(rect);
    /// assert_eq!(buffer.pos_of(0), (200, 100));
    /// assert_eq!(buffer.pos_of(14), (204, 101));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when given an index that is outside the Buffer's content.
    ///
    /// ```should_panic
    /// # use tui::buffer::Buffer;
    /// # use tui::layout::Rect;
    /// let rect = Rect::new(0, 0, 10, 10); // 100 cells in total
    /// let buffer = Buffer::empty(rect);
    /// // Index 100 is the 101th cell, which lies outside of the area of this Buffer.
    /// buffer.pos_of(100); // Panics
    /// ```
    pub fn pos_of(&self, i: usize) -> (u16, u16) {
        debug_assert!(
            i < self.content.len(),
            "Trying to get the coords of a cell outside the buffer: i={} len={}", i, self
            .content.len()
        );
        (
            self.area.x + i as u16 % self.area.width,
            self.area.y + i as u16 / self.area.width,
        )
    }
    /// Print a string, starting at the position (x, y)
    pub fn set_string<S>(&mut self, x: u16, y: u16, string: S, style: Style)
    where
        S: AsRef<str>,
    {
        self.set_stringn(x, y, string, usize::MAX, style);
    }
    /// Print at most the first n characters of a string if enough space is available
    /// until the end of the line
    pub fn set_stringn<S>(
        &mut self,
        x: u16,
        y: u16,
        string: S,
        width: usize,
        style: Style,
    ) -> (u16, u16)
    where
        S: AsRef<str>,
    {
        let mut index = self.index_of(x, y);
        let mut x_offset = x as usize;
        let graphemes = UnicodeSegmentation::graphemes(string.as_ref(), true);
        let max_offset = min(
            self.area.right() as usize,
            width.saturating_add(x as usize),
        );
        for s in graphemes {
            let width = s.width();
            if width == 0 {
                continue;
            }
            if width > max_offset.saturating_sub(x_offset) {
                break;
            }
            self.content[index].set_symbol(s);
            self.content[index].set_style(style);
            for i in index + 1..index + width {
                self.content[i].reset();
            }
            index += width;
            x_offset += width;
        }
        (x_offset as u16, y)
    }
    pub fn set_spans<'a>(
        &mut self,
        x: u16,
        y: u16,
        spans: &Spans<'a>,
        width: u16,
    ) -> (u16, u16) {
        let mut remaining_width = width;
        let mut x = x;
        for span in &spans.0 {
            if remaining_width == 0 {
                break;
            }
            let pos = self
                .set_stringn(
                    x,
                    y,
                    span.content.as_ref(),
                    remaining_width as usize,
                    span.style,
                );
            let w = pos.0.saturating_sub(x);
            x = pos.0;
            remaining_width = remaining_width.saturating_sub(w);
        }
        (x, y)
    }
    pub fn set_span<'a>(
        &mut self,
        x: u16,
        y: u16,
        span: &Span<'a>,
        width: u16,
    ) -> (u16, u16) {
        self.set_stringn(x, y, span.content.as_ref(), width as usize, span.style)
    }
    #[deprecated(
        since = "0.10.0",
        note = "You should use styling capabilities of `Buffer::set_style`"
    )]
    pub fn set_background(&mut self, area: Rect, color: Color) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                self.get_mut(x, y).set_bg(color);
            }
        }
    }
    pub fn set_style(&mut self, area: Rect, style: Style) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                self.get_mut(x, y).set_style(style);
            }
        }
    }
    /// Resize the buffer so that the mapped area matches the given area and that the buffer
    /// length is equal to area.width * area.height
    pub fn resize(&mut self, area: Rect) {
        let length = area.area() as usize;
        if self.content.len() > length {
            self.content.truncate(length);
        } else {
            self.content.resize(length, Default::default());
        }
        self.area = area;
    }
    /// Reset all cells in the buffer
    pub fn reset(&mut self) {
        for c in &mut self.content {
            c.reset();
        }
    }
    /// Merge an other buffer into this one
    pub fn merge(&mut self, other: &Buffer) {
        let area = self.area.union(other.area);
        let cell: Cell = Default::default();
        self.content.resize(area.area() as usize, cell.clone());
        let size = self.area.area() as usize;
        for i in (0..size).rev() {
            let (x, y) = self.pos_of(i);
            let k = ((y - area.y) * area.width + x - area.x) as usize;
            if i != k {
                self.content[k] = self.content[i].clone();
                self.content[i] = cell.clone();
            }
        }
        let size = other.area.area() as usize;
        for i in 0..size {
            let (x, y) = other.pos_of(i);
            let k = ((y - area.y) * area.width + x - area.x) as usize;
            self.content[k] = other.content[i].clone();
        }
        self.area = area;
    }
    /// Builds a minimal sequence of coordinates and Cells necessary to update the UI from
    /// self to other.
    ///
    /// We're assuming that buffers are well-formed, that is no double-width cell is followed by
    /// a non-blank cell.
    ///
    /// # Multi-width characters handling:
    ///
    /// ```text
    /// (Index:) `01`
    /// Prev:    `コ`
    /// Next:    `aa`
    /// Updates: `0: a, 1: a'
    /// ```
    ///
    /// ```text
    /// (Index:) `01`
    /// Prev:    `a `
    /// Next:    `コ`
    /// Updates: `0: コ` (double width symbol at index 0 - skip index 1)
    /// ```
    ///
    /// ```text
    /// (Index:) `012`
    /// Prev:    `aaa`
    /// Next:    `aコ`
    /// Updates: `0: a, 1: コ` (double width symbol at index 1 - skip index 2)
    /// ```
    pub fn diff<'a>(&self, other: &'a Buffer) -> Vec<(u16, u16, &'a Cell)> {
        let previous_buffer = &self.content;
        let next_buffer = &other.content;
        let width = self.area.width;
        let mut updates: Vec<(u16, u16, &Cell)> = vec![];
        let mut invalidated: usize = 0;
        let mut to_skip: usize = 0;
        for (i, (current, previous)) in next_buffer
            .iter()
            .zip(previous_buffer.iter())
            .enumerate()
        {
            if (current != previous || invalidated > 0) && to_skip == 0 {
                let x = i as u16 % width;
                let y = i as u16 / width;
                updates.push((x, y, &next_buffer[i]));
            }
            to_skip = current.symbol.width().saturating_sub(1);
            let affected_width = std::cmp::max(
                current.symbol.width(),
                previous.symbol.width(),
            );
            invalidated = std::cmp::max(affected_width, invalidated).saturating_sub(1);
        }
        updates
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cell(s: &str) -> Cell {
        let mut cell = Cell::default();
        cell.set_symbol(s);
        cell
    }
    #[test]
    fn it_translates_to_and_from_coordinates() {
        let rect = Rect::new(200, 100, 50, 80);
        let buf = Buffer::empty(rect);
        assert_eq!(buf.pos_of(0), (200, 100));
        assert_eq!(buf.index_of(200, 100), 0);
        assert_eq!(buf.pos_of(buf.content.len() - 1), (249, 179));
        assert_eq!(buf.index_of(249, 179), buf.content.len() - 1);
    }
    #[test]
    #[should_panic(expected = "outside the buffer")]
    fn pos_of_panics_on_out_of_bounds() {
        let rect = Rect::new(0, 0, 10, 10);
        let buf = Buffer::empty(rect);
        buf.pos_of(100);
    }
    #[test]
    #[should_panic(expected = "outside the buffer")]
    fn index_of_panics_on_out_of_bounds() {
        let rect = Rect::new(0, 0, 10, 10);
        let buf = Buffer::empty(rect);
        buf.index_of(10, 0);
    }
    #[test]
    fn buffer_set_string() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buffer = Buffer::empty(area);
        buffer.set_stringn(0, 0, "aaa", 0, Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["     "]));
        buffer.set_string(0, 0, "aaa", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["aaa  "]));
        buffer.set_stringn(0, 0, "bbbbbbbbbbbbbb", 4, Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["bbbb "]));
        buffer.set_string(0, 0, "12345", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["12345"]));
        buffer.set_string(0, 0, "123456", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["12345"]));
    }
    #[test]
    fn buffer_set_string_zero_width() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buffer = Buffer::empty(area);
        let s = "\u{1}a";
        buffer.set_stringn(0, 0, s, 1, Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["a"]));
        let s = "a\u{1}";
        buffer.set_stringn(0, 0, s, 1, Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["a"]));
    }
    #[test]
    fn buffer_set_string_double_width() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buffer = Buffer::empty(area);
        buffer.set_string(0, 0, "コン", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["コン "]));
        buffer.set_string(0, 0, "コンピ", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["コン "]));
    }
    #[test]
    fn buffer_with_lines() {
        let buffer = Buffer::with_lines(
            vec![
                "┌────────┐", "│コンピュ│",
                "│ーa 上で│", "└────────┘"
            ],
        );
        assert_eq!(buffer.area.x, 0);
        assert_eq!(buffer.area.y, 0);
        assert_eq!(buffer.area.width, 10);
        assert_eq!(buffer.area.height, 4);
    }
    #[test]
    fn buffer_diffing_empty_empty() {
        let area = Rect::new(0, 0, 40, 40);
        let prev = Buffer::empty(area);
        let next = Buffer::empty(area);
        let diff = prev.diff(&next);
        assert_eq!(diff, vec![]);
    }
    #[test]
    fn buffer_diffing_empty_filled() {
        let area = Rect::new(0, 0, 40, 40);
        let prev = Buffer::empty(area);
        let next = Buffer::filled(area, Cell::default().set_symbol("a"));
        let diff = prev.diff(&next);
        assert_eq!(diff.len(), 40 * 40);
    }
    #[test]
    fn buffer_diffing_filled_filled() {
        let area = Rect::new(0, 0, 40, 40);
        let prev = Buffer::filled(area, Cell::default().set_symbol("a"));
        let next = Buffer::filled(area, Cell::default().set_symbol("a"));
        let diff = prev.diff(&next);
        assert_eq!(diff, vec![]);
    }
    #[test]
    fn buffer_diffing_single_width() {
        let prev = Buffer::with_lines(
            vec![
                "          ", "┌Title─┐  ", "│      │  ", "│      │  ",
                "└──────┘  ",
            ],
        );
        let next = Buffer::with_lines(
            vec![
                "          ", "┌TITLE─┐  ", "│      │  ", "│      │  ",
                "└──────┘  ",
            ],
        );
        let diff = prev.diff(&next);
        assert_eq!(
            diff, vec![(2, 1, & cell("I")), (3, 1, & cell("T")), (4, 1, & cell("L")), (5,
            1, & cell("E")),]
        );
    }
    #[test]
    #[rustfmt::skip]
    fn buffer_diffing_multi_width() {
        let prev = Buffer::with_lines(
            vec!["┌Title─┐  ", "└──────┘  ",],
        );
        let next = Buffer::with_lines(
            vec!["┌称号──┐  ", "└──────┘  ",],
        );
        let diff = prev.diff(&next);
        assert_eq!(
            diff, vec![(1, 0, & cell("称")), (3, 0, & cell("号")), (5, 0, &
            cell("─")),]
        );
    }
    #[test]
    fn buffer_diffing_multi_width_offset() {
        let prev = Buffer::with_lines(vec!["┌称号──┐"]);
        let next = Buffer::with_lines(vec!["┌─称号─┐"]);
        let diff = prev.diff(&next);
        assert_eq!(
            diff, vec![(1, 0, & cell("─")), (2, 0, & cell("称")), (4, 0, &
            cell("号")),]
        );
    }
    #[test]
    fn buffer_merge() {
        let mut one = Buffer::filled(
            Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("1"),
        );
        let two = Buffer::filled(
            Rect {
                x: 0,
                y: 2,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("2"),
        );
        one.merge(&two);
        assert_eq!(one, Buffer::with_lines(vec!["11", "11", "22", "22"]));
    }
    #[test]
    fn buffer_merge2() {
        let mut one = Buffer::filled(
            Rect {
                x: 2,
                y: 2,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("1"),
        );
        let two = Buffer::filled(
            Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("2"),
        );
        one.merge(&two);
        assert_eq!(one, Buffer::with_lines(vec!["22  ", "22  ", "  11", "  11"]));
    }
    #[test]
    fn buffer_merge3() {
        let mut one = Buffer::filled(
            Rect {
                x: 3,
                y: 3,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("1"),
        );
        let two = Buffer::filled(
            Rect {
                x: 1,
                y: 1,
                width: 3,
                height: 4,
            },
            Cell::default().set_symbol("2"),
        );
        one.merge(&two);
        let mut merged = Buffer::with_lines(vec!["222 ", "222 ", "2221", "2221"]);
        merged
            .area = Rect {
            x: 1,
            y: 1,
            width: 4,
            height: 4,
        };
        assert_eq!(one, merged);
    }
}
#[cfg(test)]
mod tests_llm_16_27 {
    use super::*;
    use crate::*;
    use crate::buffer::{Buffer, Cell};
    use crate::layout::Rect;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_27_rrrruuuugggg_test_default = 0;
        let buf = Buffer::default();
        debug_assert_eq!(buf.area, Rect::default());
        debug_assert_eq!(buf.content, Vec:: < Cell > ::new());
        let _rug_ed_tests_llm_16_27_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_28 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_28_rrrruuuugggg_test_default = 0;
        let cell = Cell::default();
        debug_assert_eq!(cell.symbol, " ");
        debug_assert_eq!(cell.fg, Color::Reset);
        debug_assert_eq!(cell.bg, Color::Reset);
        debug_assert_eq!(cell.modifier, Modifier::empty());
        let _rug_ed_tests_llm_16_28_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_135 {
    use crate::buffer::{Buffer, Cell};
    use crate::layout::Rect;
    use crate::style::{Color, Style, Modifier};
    #[test]
    fn test_area() {
        let _rug_st_tests_llm_16_135_rrrruuuugggg_test_area = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 5;
        let buffer = Buffer {
            area: Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3),
            content: Vec::new(),
        };
        let area = buffer.area();
        let expected_area = &Rect::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7);
        debug_assert_eq!(area, expected_area);
        let _rug_ed_tests_llm_16_135_rrrruuuugggg_test_area = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_137 {
    use super::*;
    use crate::*;
    use crate::style::Style;
    #[test]
    fn test_diff() {
        let _rug_st_tests_llm_16_137_rrrruuuugggg_test_diff = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "Hello";
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = "World";
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 1;
        let rug_fuzz_12 = "World";
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 2;
        let rug_fuzz_15 = "World";
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 3;
        let rug_fuzz_18 = "World";
        let rug_fuzz_19 = 0;
        let rug_fuzz_20 = 0;
        let rug_fuzz_21 = 0;
        let rect = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let mut buffer1 = Buffer::empty(rect);
        let mut buffer2 = Buffer::empty(rect);
        buffer1.set_string(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, Style::default());
        buffer2.set_string(rug_fuzz_7, rug_fuzz_8, rug_fuzz_9, Style::default());
        buffer2.set_string(rug_fuzz_10, rug_fuzz_11, rug_fuzz_12, Style::default());
        buffer2.set_string(rug_fuzz_13, rug_fuzz_14, rug_fuzz_15, Style::default());
        buffer2.set_string(rug_fuzz_16, rug_fuzz_17, rug_fuzz_18, Style::default());
        let updates = buffer1.diff(&buffer2);
        let expected_result = vec![
            (rug_fuzz_19, rug_fuzz_20, & buffer2.content[rug_fuzz_21]), (0, 1, & buffer2
            .content[10]), (0, 2, & buffer2.content[20]), (0, 3, & buffer2.content[30])
        ];
        debug_assert_eq!(updates, expected_result);
        let _rug_ed_tests_llm_16_137_rrrruuuugggg_test_diff = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_139 {
    use super::*;
    use crate::*;
    use crate::style::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_139_rrrruuuugggg_test_empty = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let area = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let expected = Buffer {
            area: area.clone(),
            content: vec![
                Cell { symbol : String::from(" "), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }; 50
            ],
        };
        debug_assert_eq!(Buffer::empty(area), expected);
        let _rug_ed_tests_llm_16_139_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_142 {
    use super::*;
    use crate::*;
    #[test]
    fn test_buffer_get() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_get = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = "x";
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 2;
        let rug_fuzz_9 = 3;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = "string";
        let rug_fuzz_12 = 5;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 5;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 'x';
        let rug_fuzz_17 = 5;
        let rug_fuzz_18 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        buf.get_mut(rug_fuzz_4, rug_fuzz_5).set_symbol(rug_fuzz_6);
        debug_assert_eq!(buf.get(rug_fuzz_7, rug_fuzz_8).symbol, "x");
        buf.set_string(
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            Style::default().fg(Color::Red).bg(Color::White),
        );
        debug_assert_eq!(
            buf.get(rug_fuzz_12, rug_fuzz_13), & Cell { symbol : String::from("r"), fg :
            Color::Red, bg : Color::White, modifier : Modifier::empty() }
        );
        buf.get_mut(rug_fuzz_14, rug_fuzz_15).set_char(rug_fuzz_16);
        debug_assert_eq!(buf.get(rug_fuzz_17, rug_fuzz_18).symbol, "x");
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_get = 0;
    }
    #[test]
    fn test_buffer_index_of() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_index_of = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = 1;
        let rug_fuzz_11 = 1;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 2;
        let rug_fuzz_14 = 1;
        let rug_fuzz_15 = 2;
        let buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            height: rug_fuzz_2,
            width: rug_fuzz_3,
        });
        debug_assert_eq!(buf.index_of(rug_fuzz_4, rug_fuzz_5), 0);
        debug_assert_eq!(buf.index_of(rug_fuzz_6, rug_fuzz_7), 1);
        debug_assert_eq!(buf.index_of(rug_fuzz_8, rug_fuzz_9), 2);
        debug_assert_eq!(buf.index_of(rug_fuzz_10, rug_fuzz_11), 3);
        debug_assert_eq!(buf.index_of(rug_fuzz_12, rug_fuzz_13), 4);
        debug_assert_eq!(buf.index_of(rug_fuzz_14, rug_fuzz_15), 5);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_index_of = 0;
    }
    #[test]
    #[should_panic]
    fn test_buffer_index_of_panic() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_index_of_panic = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 0;
        let buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            height: rug_fuzz_2,
            width: rug_fuzz_3,
        });
        buf.index_of(rug_fuzz_4, rug_fuzz_5);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_index_of_panic = 0;
    }
    #[test]
    fn test_buffer_pos_of() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_pos_of = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 3;
        let rug_fuzz_8 = 4;
        let rug_fuzz_9 = 5;
        let buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            height: rug_fuzz_2,
            width: rug_fuzz_3,
        });
        debug_assert_eq!(buf.pos_of(rug_fuzz_4), (0, 0));
        debug_assert_eq!(buf.pos_of(rug_fuzz_5), (1, 0));
        debug_assert_eq!(buf.pos_of(rug_fuzz_6), (0, 1));
        debug_assert_eq!(buf.pos_of(rug_fuzz_7), (1, 1));
        debug_assert_eq!(buf.pos_of(rug_fuzz_8), (0, 2));
        debug_assert_eq!(buf.pos_of(rug_fuzz_9), (1, 2));
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_pos_of = 0;
    }
    #[test]
    #[should_panic]
    fn test_buffer_pos_of_panic() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_pos_of_panic = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 6;
        let buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            height: rug_fuzz_2,
            width: rug_fuzz_3,
        });
        buf.pos_of(rug_fuzz_4);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_pos_of_panic = 0;
    }
    #[test]
    fn test_buffer_set_string() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_set_string = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "hello";
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        buf.set_string(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, Style::default());
        debug_assert_eq!(buf.get(rug_fuzz_7, rug_fuzz_8).symbol, "hello");
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_set_string = 0;
    }
    #[test]
    fn test_buffer_set_stringn() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_set_stringn = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "hello world";
        let rug_fuzz_7 = 5;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        buf.set_stringn(
            rug_fuzz_4,
            rug_fuzz_5,
            rug_fuzz_6,
            rug_fuzz_7,
            Style::default(),
        );
        debug_assert_eq!(buf.get(rug_fuzz_8, rug_fuzz_9).symbol, "hello");
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_set_stringn = 0;
    }
    #[test]
    fn test_buffer_set_spans() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_set_spans = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = "hello";
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 5;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 5;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 5;
        let rug_fuzz_15 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        let spans = Spans::from(
            vec![
                Span::styled(rug_fuzz_4, Style::default().fg(Color::Red)),
                Span::styled(" world", Style::default().fg(Color::Blue))
            ],
        );
        buf.set_spans(rug_fuzz_5, rug_fuzz_6, &spans, rug_fuzz_7);
        debug_assert_eq!(buf.get(rug_fuzz_8, rug_fuzz_9).symbol, "hello");
        debug_assert_eq!(buf.get(rug_fuzz_10, rug_fuzz_11).symbol, "world");
        debug_assert_eq!(buf.get(rug_fuzz_12, rug_fuzz_13).fg, Color::Red);
        debug_assert_eq!(buf.get(rug_fuzz_14, rug_fuzz_15).fg, Color::Blue);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_set_spans = 0;
    }
    #[test]
    fn test_buffer_set_span() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_set_span = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = "hello";
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 5;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        let span = Span::styled(rug_fuzz_4, Style::default().fg(Color::Red));
        buf.set_span(rug_fuzz_5, rug_fuzz_6, &span, rug_fuzz_7);
        debug_assert_eq!(buf.get(rug_fuzz_8, rug_fuzz_9).symbol, "hello");
        debug_assert_eq!(buf.get(rug_fuzz_10, rug_fuzz_11).fg, Color::Red);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_set_span = 0;
    }
    #[test]
    #[deprecated]
    fn test_buffer_set_background() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_set_background = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        buf.set_background(
            Rect {
                x: rug_fuzz_4,
                y: rug_fuzz_5,
                width: rug_fuzz_6,
                height: rug_fuzz_7,
            },
            Color::Red,
        );
        debug_assert_eq!(buf.get(rug_fuzz_8, rug_fuzz_9).bg, Color::Red);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_set_background = 0;
    }
    #[test]
    fn test_buffer_set_style() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_set_style = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        buf.set_style(
            Rect {
                x: rug_fuzz_4,
                y: rug_fuzz_5,
                width: rug_fuzz_6,
                height: rug_fuzz_7,
            },
            Style::default().fg(Color::Red),
        );
        debug_assert_eq!(buf.get(rug_fuzz_8, rug_fuzz_9).fg, Color::Red);
        debug_assert_eq!(buf.get(rug_fuzz_10, rug_fuzz_11).bg, Color::Reset);
        debug_assert_eq!(buf.get(rug_fuzz_12, rug_fuzz_13).modifier, Modifier::empty());
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_set_style = 0;
    }
    #[test]
    fn test_buffer_resize() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_resize = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 5;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        buf.resize(Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        });
        debug_assert_eq!(buf.area.height, 5);
        debug_assert_eq!(buf.area.width, 10);
        debug_assert_eq!(buf.content.len(), 50);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_resize = 0;
    }
    #[test]
    fn test_buffer_reset() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_reset = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        buf.set_style(
            Rect {
                x: rug_fuzz_4,
                y: rug_fuzz_5,
                width: rug_fuzz_6,
                height: rug_fuzz_7,
            },
            Style::default().fg(Color::Red),
        );
        buf.reset();
        debug_assert_eq!(buf.content[rug_fuzz_8].symbol, " ");
        debug_assert_eq!(buf.content[rug_fuzz_9].fg, Color::Reset);
        debug_assert_eq!(buf.content[rug_fuzz_10].bg, Color::Reset);
        debug_assert_eq!(buf.content[rug_fuzz_11].modifier, Modifier::empty());
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_reset = 0;
    }
    #[test]
    fn test_buffer_merge() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_merge = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 5;
        let rug_fuzz_11 = 1;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 5;
        let rug_fuzz_15 = 1;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 0;
        let mut buf1 = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        let mut buf2 = Buffer::empty(Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        });
        buf1.set_style(
            Rect {
                x: rug_fuzz_8,
                y: rug_fuzz_9,
                width: rug_fuzz_10,
                height: rug_fuzz_11,
            },
            Style::default().fg(Color::Red),
        );
        buf2.set_style(
            Rect {
                x: rug_fuzz_12,
                y: rug_fuzz_13,
                width: rug_fuzz_14,
                height: rug_fuzz_15,
            },
            Style::default().fg(Color::Blue),
        );
        buf1.merge(&buf2);
        debug_assert_eq!(buf1.get(rug_fuzz_16, rug_fuzz_17).fg, Color::Blue);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_merge = 0;
    }
    #[test]
    fn test_buffer_diff() {
        let _rug_st_tests_llm_16_142_rrrruuuugggg_test_buffer_diff = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 1;
        let buf1 = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        let mut buf2 = buf1.clone();
        buf2.set_style(
            Rect {
                x: rug_fuzz_4,
                y: rug_fuzz_5,
                width: rug_fuzz_6,
                height: rug_fuzz_7,
            },
            Style::default().fg(Color::Red),
        );
        let updates = buf1.diff(&buf2);
        debug_assert_eq!(updates.len(), 5);
        let _rug_ed_tests_llm_16_142_rrrruuuugggg_test_buffer_diff = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_143 {
    use super::*;
    use crate::*;
    use crate::layout::Rect;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_llm_16_143_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = "x";
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 2;
        let rug_fuzz_9 = 3;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = "string";
        let rug_fuzz_12 = 5;
        let rug_fuzz_13 = 0;
        let rug_fuzz_14 = 5;
        let rug_fuzz_15 = 0;
        let rug_fuzz_16 = 'x';
        let rug_fuzz_17 = 5;
        let rug_fuzz_18 = 0;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        buf.get_mut(rug_fuzz_4, rug_fuzz_5).set_symbol(rug_fuzz_6);
        debug_assert_eq!(buf.get(rug_fuzz_7, rug_fuzz_8).symbol, "x");
        buf.set_string(
            rug_fuzz_9,
            rug_fuzz_10,
            rug_fuzz_11,
            Style::default().fg(Color::Red).bg(Color::White),
        );
        debug_assert_eq!(
            buf.get(rug_fuzz_12, rug_fuzz_13), & Cell { symbol : "r".to_string(), fg :
            Color::Red, bg : Color::White, modifier : Modifier::empty() }
        );
        buf.get_mut(rug_fuzz_14, rug_fuzz_15).set_char(rug_fuzz_16);
        debug_assert_eq!(buf.get(rug_fuzz_17, rug_fuzz_18).symbol, "x");
        let _rug_ed_tests_llm_16_143_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_144 {
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    #[test]
    fn test_index_of() {
        let _rug_st_tests_llm_16_144_rrrruuuugggg_test_index_of = 0;
        let rug_fuzz_0 = 200;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 200;
        let rug_fuzz_5 = 100;
        let rect = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let buffer = Buffer::empty(rect);
        debug_assert_eq!(buffer.index_of(rug_fuzz_4, rug_fuzz_5), 0);
        let _rug_ed_tests_llm_16_144_rrrruuuugggg_test_index_of = 0;
    }
    #[test]
    #[should_panic]
    fn test_index_of_panic() {
        let _rug_st_tests_llm_16_144_rrrruuuugggg_test_index_of_panic = 0;
        let rug_fuzz_0 = 200;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rect = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let buffer = Buffer::empty(rect);
        buffer.index_of(rug_fuzz_4, rug_fuzz_5);
        let _rug_ed_tests_llm_16_144_rrrruuuugggg_test_index_of_panic = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_147 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Style};
    #[test]
    fn test_pos_of() {
        let _rug_st_tests_llm_16_147_rrrruuuugggg_test_pos_of = 0;
        let rug_fuzz_0 = 200;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 14;
        let rect = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let buffer = Buffer::empty(rect);
        debug_assert_eq!(buffer.pos_of(rug_fuzz_4), (200, 100));
        debug_assert_eq!(buffer.pos_of(rug_fuzz_5), (204, 101));
        let _rug_ed_tests_llm_16_147_rrrruuuugggg_test_pos_of = 0;
    }
    #[test]
    #[should_panic]
    fn test_pos_of_panic() {
        let _rug_st_tests_llm_16_147_rrrruuuugggg_test_pos_of_panic = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 100;
        let rect = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let buffer = Buffer::empty(rect);
        buffer.pos_of(rug_fuzz_4);
        let _rug_ed_tests_llm_16_147_rrrruuuugggg_test_pos_of_panic = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_148 {
    use super::*;
    use crate::*;
    use crate::style::Style;
    use crate::style::Color;
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_148_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = "A";
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 2;
        let rug_fuzz_9 = "";
        let mut buffer = Buffer {
            area: Rect {
                x: rug_fuzz_0,
                y: rug_fuzz_1,
                width: rug_fuzz_2,
                height: rug_fuzz_3,
            },
            content: vec![
                Cell { symbol : String::from(rug_fuzz_4), fg : Color::Reset, bg :
                Color::Reset, modifier : Modifier::empty(), }, Cell { symbol :
                String::from("B"), fg : Color::Reset, bg : Color::Reset, modifier :
                Modifier::empty(), }, Cell { symbol : String::from("C"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("D"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }
            ],
        };
        buffer.reset();
        let expected_buffer = Buffer {
            area: Rect {
                x: rug_fuzz_5,
                y: rug_fuzz_6,
                width: rug_fuzz_7,
                height: rug_fuzz_8,
            },
            content: vec![
                Cell { symbol : String::from(rug_fuzz_9), fg : Color::Reset, bg :
                Color::Reset, modifier : Modifier::empty(), }, Cell { symbol :
                String::from(""), fg : Color::Reset, bg : Color::Reset, modifier :
                Modifier::empty(), }, Cell { symbol : String::from(""), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from(""), fg : Color::Reset, bg : Color::Reset, modifier
                : Modifier::empty(), }
            ],
        };
        debug_assert_eq!(buffer, expected_buffer);
        let _rug_ed_tests_llm_16_148_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_149 {
    use super::*;
    use crate::*;
    #[test]
    fn test_buffer_resize() {
        let _rug_st_tests_llm_16_149_rrrruuuugggg_test_buffer_resize = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 3;
        let rug_fuzz_7 = 3;
        let mut buffer = Buffer {
            area: Rect {
                x: rug_fuzz_0,
                y: rug_fuzz_1,
                width: rug_fuzz_2,
                height: rug_fuzz_3,
            },
            content: vec![
                Cell::default(), Cell::default(), Cell::default(), Cell::default(),
                Cell::default(), Cell::default(), Cell::default(), Cell::default(),
                Cell::default(), Cell::default(), Cell::default(), Cell::default(),
                Cell::default(), Cell::default(), Cell::default(), Cell::default(),
                Cell::default(), Cell::default(), Cell::default(), Cell::default(),
                Cell::default(), Cell::default(), Cell::default(), Cell::default(),
                Cell::default()
            ],
        };
        let area = Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        };
        buffer.resize(area);
        let expected_buffer = Buffer {
            area: area,
            content: vec![
                Cell::default(), Cell::default(), Cell::default(), Cell::default(),
                Cell::default(), Cell::default(), Cell::default(), Cell::default(),
                Cell::default()
            ],
        };
        debug_assert_eq!(buffer, expected_buffer);
        let _rug_ed_tests_llm_16_149_rrrruuuugggg_test_buffer_resize = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_153 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    #[should_panic]
    fn test_set_span_panics_on_invalid_coordinates() {
        let _rug_st_tests_llm_16_153_rrrruuuugggg_test_set_span_panics_on_invalid_coordinates = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 11;
        let rug_fuzz_5 = 11;
        let rug_fuzz_6 = "Test";
        let rug_fuzz_7 = 10;
        let mut buffer = Buffer::empty(
            Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3),
        );
        buffer.set_span(rug_fuzz_4, rug_fuzz_5, &Span::raw(rug_fuzz_6), rug_fuzz_7);
        let _rug_ed_tests_llm_16_153_rrrruuuugggg_test_set_span_panics_on_invalid_coordinates = 0;
    }
    #[test]
    fn test_set_span() {
        let _rug_st_tests_llm_16_153_rrrruuuugggg_test_set_span = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = "Test";
        let rug_fuzz_5 = "Test";
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 10;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let rug_fuzz_11 = 0;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 1;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 1;
        let rug_fuzz_16 = 0;
        let rug_fuzz_17 = 2;
        let rug_fuzz_18 = 0;
        let rug_fuzz_19 = 2;
        let rug_fuzz_20 = 0;
        let rug_fuzz_21 = 3;
        let rug_fuzz_22 = 0;
        let rug_fuzz_23 = 3;
        let rug_fuzz_24 = 0;
        let mut buffer = Buffer::empty(
            Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3),
        );
        let span = Span::styled(rug_fuzz_4, Style::default().fg(Color::Yellow));
        let expected_span = Span::styled(
            rug_fuzz_5,
            Style::default().fg(Color::Yellow).bg(Color::Black),
        );
        let (x, y) = buffer.set_span(rug_fuzz_6, rug_fuzz_7, &span, rug_fuzz_8);
        debug_assert_eq!(x, 4);
        debug_assert_eq!(y, 0);
        debug_assert_eq!(buffer.get(rug_fuzz_9, rug_fuzz_10).symbol, "T");
        debug_assert_eq!(
            buffer.get(rug_fuzz_11, rug_fuzz_12).style(), Style::default()
            .fg(Color::Yellow).bg(Color::Black)
        );
        debug_assert_eq!(buffer.get(rug_fuzz_13, rug_fuzz_14).symbol, "e");
        debug_assert_eq!(
            buffer.get(rug_fuzz_15, rug_fuzz_16).style(), Style::default()
            .fg(Color::Yellow).bg(Color::Black)
        );
        debug_assert_eq!(buffer.get(rug_fuzz_17, rug_fuzz_18).symbol, "s");
        debug_assert_eq!(
            buffer.get(rug_fuzz_19, rug_fuzz_20).style(), Style::default()
            .fg(Color::Yellow).bg(Color::Black)
        );
        debug_assert_eq!(buffer.get(rug_fuzz_21, rug_fuzz_22).symbol, "t");
        debug_assert_eq!(
            buffer.get(rug_fuzz_23, rug_fuzz_24).style(), Style::default()
            .fg(Color::Yellow).bg(Color::Black)
        );
        let _rug_ed_tests_llm_16_153_rrrruuuugggg_test_set_span = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_157 {
    use super::*;
    use crate::*;
    use crate::style::*;
    use crate::layout::Rect;
    #[test]
    fn test_set_stringn() {
        let _rug_st_tests_llm_16_157_rrrruuuugggg_test_set_stringn = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "Hello world";
        let rug_fuzz_7 = 10;
        let mut buffer = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        let style = Style::default();
        let (x_offset, y) = buffer
            .set_stringn(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7, style);
        debug_assert_eq!(x_offset, 10);
        debug_assert_eq!(y, 0);
        let _rug_ed_tests_llm_16_157_rrrruuuugggg_test_set_stringn = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_159_llm_16_158 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_style() {
        let _rug_st_tests_llm_16_159_llm_16_158_rrrruuuugggg_test_set_style = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 6;
        let rug_fuzz_7 = 6;
        let mut buffer = Buffer::empty(
            Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3),
        );
        let area = Rect::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7);
        let style = Style::default().fg(Color::Red).bg(Color::White);
        buffer.set_style(area, style);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = buffer.get(x, y);
                debug_assert_eq!(cell.style(), style);
            }
        }
        let _rug_ed_tests_llm_16_159_llm_16_158_rrrruuuugggg_test_set_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_160 {
    use super::*;
    use crate::*;
    use crate::style::Style;
    #[test]
    fn test_with_lines() {
        let _rug_st_tests_llm_16_160_rrrruuuugggg_test_with_lines = 0;
        let rug_fuzz_0 = "Hello";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 6;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = "H";
        let lines = vec![rug_fuzz_0, "World!"];
        let buffer = Buffer::with_lines(lines);
        let expected_buffer = Buffer {
            area: Rect {
                x: rug_fuzz_1,
                y: rug_fuzz_2,
                width: rug_fuzz_3,
                height: rug_fuzz_4,
            },
            content: vec![
                Cell { symbol : String::from(rug_fuzz_5), fg : Color::Reset, bg :
                Color::Reset, modifier : Modifier::empty(), }, Cell { symbol :
                String::from("e"), fg : Color::Reset, bg : Color::Reset, modifier :
                Modifier::empty(), }, Cell { symbol : String::from("l"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("l"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("o"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("\n"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("W"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("o"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("r"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("l"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("d"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("!"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("\n"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }
            ],
        };
        debug_assert_eq!(buffer, expected_buffer);
        let _rug_ed_tests_llm_16_160_rrrruuuugggg_test_with_lines = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_161 {
    use crate::buffer::Cell;
    use crate::style::{Color, Modifier};
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_161_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = "A";
        let mut cell = Cell {
            symbol: rug_fuzz_0.into(),
            fg: Color::Red,
            bg: Color::Blue,
            modifier: Modifier::BOLD,
        };
        cell.reset();
        debug_assert_eq!(cell.symbol, " ");
        debug_assert_eq!(cell.fg, Color::Reset);
        debug_assert_eq!(cell.bg, Color::Reset);
        debug_assert_eq!(cell.modifier, Modifier::empty());
        let _rug_ed_tests_llm_16_161_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_162 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_bg() {
        let _rug_st_tests_llm_16_162_rrrruuuugggg_test_set_bg = 0;
        let mut cell = Cell::default();
        let color = Color::Red;
        let result = cell.set_bg(color);
        debug_assert_eq!(result.bg, color);
        let _rug_ed_tests_llm_16_162_rrrruuuugggg_test_set_bg = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_166 {
    use super::*;
    use crate::*;
    use crate::style::Color;
    #[test]
    fn test_set_fg() {
        let _rug_st_tests_llm_16_166_rrrruuuugggg_test_set_fg = 0;
        let mut cell = Cell::default();
        let color = Color::Blue;
        let result = cell.set_fg(color);
        debug_assert_eq!(result.fg, color);
        let _rug_ed_tests_llm_16_166_rrrruuuugggg_test_set_fg = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_167 {
    use super::*;
    use crate::*;
    use crate::style::{Color, Modifier, Style};
    #[test]
    fn test_set_style() {
        let _rug_st_tests_llm_16_167_rrrruuuugggg_test_set_style = 0;
        let mut cell = Cell::default();
        let style = Style::default().fg(Color::Red);
        let expected = Cell {
            fg: Color::Red,
            ..Cell::default()
        };
        debug_assert_eq!(cell.set_style(style), & expected);
        let style = Style::default().bg(Color::Green);
        let expected = Cell {
            bg: Color::Green,
            ..expected
        };
        debug_assert_eq!(cell.set_style(style), & expected);
        let style = Style::default().add_modifier(Modifier::BOLD);
        let expected = Cell {
            modifier: Modifier::BOLD,
            ..expected
        };
        debug_assert_eq!(cell.set_style(style), & expected);
        let style = Style::default().remove_modifier(Modifier::BOLD);
        let expected = Cell {
            modifier: Modifier::empty(),
            ..expected
        };
        debug_assert_eq!(cell.set_style(style), & expected);
        let _rug_ed_tests_llm_16_167_rrrruuuugggg_test_set_style = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_168 {
    use crate::buffer::{Cell, Color, Modifier};
    #[test]
    fn test_set_symbol() {
        let _rug_st_tests_llm_16_168_rrrruuuugggg_test_set_symbol = 0;
        let rug_fuzz_0 = "A";
        let mut cell = Cell::default();
        cell.set_symbol(rug_fuzz_0);
        debug_assert_eq!(cell.symbol, "A");
        let _rug_ed_tests_llm_16_168_rrrruuuugggg_test_set_symbol = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_169 {
    use super::*;
    use crate::*;
    #[test]
    fn test_style() {
        let _rug_st_tests_llm_16_169_rrrruuuugggg_test_style = 0;
        let rug_fuzz_0 = "A";
        let cell = Cell {
            symbol: rug_fuzz_0.into(),
            fg: Color::Blue,
            bg: Color::Red,
            modifier: Modifier::BOLD,
        };
        let expected = Style {
            fg: Some(Color::Blue),
            bg: Some(Color::Red),
            add_modifier: Modifier::BOLD,
            sub_modifier: Modifier::empty(),
        };
        debug_assert_eq!(cell.style(), expected);
        let _rug_ed_tests_llm_16_169_rrrruuuugggg_test_style = 0;
    }
}
#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::buffer::Cell;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_37_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'A';
        let mut p0: Cell = Cell::default();
        let p1: char = rug_fuzz_0;
        p0.set_char(p1);
        let _rug_ed_tests_rug_37_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_38 {
    use super::*;
    use crate::layout::Rect;
    use crate::buffer::{Cell, Buffer};
    #[test]
    fn test_filled() {
        let _rug_st_tests_rug_38_rrrruuuugggg_test_filled = 0;
        let mut p0: Rect = Rect::default();
        let mut p1: Cell = Cell::default();
        Buffer::filled(p0, &p1);
        let _rug_ed_tests_rug_38_rrrruuuugggg_test_filled = 0;
    }
}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    use crate::buffer::{Buffer, Cell};
    use crate::layout::Rect;
    use crate::style::{Style, Color};
    #[test]
    fn test_content() {
        let _rug_st_tests_rug_39_rrrruuuugggg_test_content = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let area = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let mut buffer = Buffer::empty(area);
        let content = buffer.content();
        debug_assert_eq!(content, & buffer.content);
        let _rug_ed_tests_rug_39_rrrruuuugggg_test_content = 0;
    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::*;
    use crate::buffer::{Buffer, Cell};
    use crate::layout::Rect;
    use crate::style::{Style, Color};
    use std::ffi::CString;
    #[test]
    fn test_set_string() {
        let _rug_st_tests_rug_40_rrrruuuugggg_test_set_string = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = "Hello World!";
        let area = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let mut buffer = Buffer::empty(area);
        let x = rug_fuzz_4;
        let y = rug_fuzz_5;
        let string = rug_fuzz_6;
        let style = Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::ITALIC | Modifier::BOLD);
        buffer.set_string(x, y, string, style);
        let _rug_ed_tests_rug_40_rrrruuuugggg_test_set_string = 0;
    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::buffer::{Buffer, Cell};
    use crate::layout::Rect;
    use crate::style::{Style, Color};
    use crate::text::Spans;
    #[test]
    fn test_set_spans() {
        let _rug_st_tests_rug_41_rrrruuuugggg_test_set_spans = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "";
        let rug_fuzz_7 = 10;
        let area = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let mut buffer = Buffer::empty(area);
        let x: u16 = rug_fuzz_4;
        let y: u16 = rug_fuzz_5;
        let spans: Spans<'static> = Spans::from(rug_fuzz_6);
        let width: u16 = rug_fuzz_7;
        buffer.set_spans(x, y, &spans, width);
        let _rug_ed_tests_rug_41_rrrruuuugggg_test_set_spans = 0;
    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use crate::buffer::{Buffer, Cell};
    use crate::layout::Rect;
    use crate::style::{Color, Style};
    #[test]
    fn test_set_background() {
        let _rug_st_tests_rug_42_rrrruuuugggg_test_set_background = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let mut p0 = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        let mut p1: Rect = Rect::default();
        let mut p2 = Color::Reset;
        p0.set_background(p1, p2);
        let _rug_ed_tests_rug_42_rrrruuuugggg_test_set_background = 0;
    }
}
#[cfg(test)]
mod tests_rug_43 {
    use super::*;
    use crate::buffer::{Buffer, Cell};
    use crate::layout::Rect;
    #[test]
    fn test_merge() {
        let _rug_st_tests_rug_43_rrrruuuugggg_test_merge = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 5;
        let mut p0 = {
            let area = Rect {
                x: rug_fuzz_0,
                y: rug_fuzz_1,
                width: rug_fuzz_2,
                height: rug_fuzz_3,
            };
            let mut p0 = Buffer::empty(area);
            p0
        };
        let mut p1 = {
            let area = Rect {
                x: rug_fuzz_4,
                y: rug_fuzz_5,
                width: rug_fuzz_6,
                height: rug_fuzz_7,
            };
            let mut p1 = Buffer::empty(area);
            p1
        };
        p0.merge(&p1);
        let _rug_ed_tests_rug_43_rrrruuuugggg_test_merge = 0;
    }
}
