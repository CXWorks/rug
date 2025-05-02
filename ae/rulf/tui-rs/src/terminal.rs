use crate::{
    backend::Backend, buffer::Buffer, layout::Rect, widgets::{StatefulWidget, Widget},
};
use std::io;
#[derive(Debug, Clone, PartialEq)]
/// UNSTABLE
enum ResizeBehavior {
    Fixed,
    Auto,
}
#[derive(Debug, Clone, PartialEq)]
/// UNSTABLE
pub struct Viewport {
    area: Rect,
    resize_behavior: ResizeBehavior,
}
impl Viewport {
    /// UNSTABLE
    pub fn fixed(area: Rect) -> Viewport {
        Viewport {
            area,
            resize_behavior: ResizeBehavior::Fixed,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
/// Options to pass to [`Terminal::with_options`]
pub struct TerminalOptions {
    /// Viewport used to draw to the terminal
    pub viewport: Viewport,
}
/// Interface to the terminal backed by Termion
#[derive(Debug)]
pub struct Terminal<B>
where
    B: Backend,
{
    backend: B,
    /// Holds the results of the current and previous draw calls. The two are compared at the end
    /// of each draw pass to output the necessary updates to the terminal
    buffers: [Buffer; 2],
    /// Index of the current buffer in the previous array
    current: usize,
    /// Whether the cursor is currently hidden
    hidden_cursor: bool,
    /// Viewport
    viewport: Viewport,
}
/// Represents a consistent terminal interface for rendering.
pub struct Frame<'a, B: 'a>
where
    B: Backend,
{
    terminal: &'a mut Terminal<B>,
    /// Where should the cursor be after drawing this frame?
    ///
    /// If `None`, the cursor is hidden and its position is controlled by the backend. If `Some((x,
    /// y))`, the cursor is shown and placed at `(x, y)` after the call to `Terminal::draw()`.
    cursor_position: Option<(u16, u16)>,
}
impl<'a, B> Frame<'a, B>
where
    B: Backend,
{
    /// Terminal size, guaranteed not to change when rendering.
    pub fn size(&self) -> Rect {
        self.terminal.viewport.area
    }
    /// Render a [`Widget`] to the current buffer using [`Widget::render`].
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::io;
    /// # use tui::Terminal;
    /// # use tui::backend::TermionBackend;
    /// # use tui::layout::Rect;
    /// # use tui::widgets::Block;
    /// # let stdout = io::stdout();
    /// # let backend = TermionBackend::new(stdout);
    /// # let mut terminal = Terminal::new(backend).unwrap();
    /// let block = Block::default();
    /// let area = Rect::new(0, 0, 5, 5);
    /// let mut frame = terminal.get_frame();
    /// frame.render_widget(block, area);
    /// ```
    pub fn render_widget<W>(&mut self, widget: W, area: Rect)
    where
        W: Widget,
    {
        widget.render(area, self.terminal.current_buffer_mut());
    }
    /// Render a [`StatefulWidget`] to the current buffer using [`StatefulWidget::render`].
    ///
    /// The last argument should be an instance of the [`StatefulWidget::State`] associated to the
    /// given [`StatefulWidget`].
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::io;
    /// # use tui::Terminal;
    /// # use tui::backend::TermionBackend;
    /// # use tui::layout::Rect;
    /// # use tui::widgets::{List, ListItem, ListState};
    /// # let stdout = io::stdout();
    /// # let backend = TermionBackend::new(stdout);
    /// # let mut terminal = Terminal::new(backend).unwrap();
    /// let mut state = ListState::default();
    /// state.select(Some(1));
    /// let items = vec![
    ///     ListItem::new("Item 1"),
    ///     ListItem::new("Item 2"),
    /// ];
    /// let list = List::new(items);
    /// let area = Rect::new(0, 0, 5, 5);
    /// let mut frame = terminal.get_frame();
    /// frame.render_stateful_widget(list, area, &mut state);
    /// ```
    pub fn render_stateful_widget<W>(
        &mut self,
        widget: W,
        area: Rect,
        state: &mut W::State,
    )
    where
        W: StatefulWidget,
    {
        widget.render(area, self.terminal.current_buffer_mut(), state);
    }
    /// After drawing this frame, make the cursor visible and put it at the specified (x, y)
    /// coordinates. If this method is not called, the cursor will be hidden.
    ///
    /// Note that this will interfere with calls to `Terminal::hide_cursor()`,
    /// `Terminal::show_cursor()`, and `Terminal::set_cursor()`. Pick one of the APIs and stick
    /// with it.
    pub fn set_cursor(&mut self, x: u16, y: u16) {
        self.cursor_position = Some((x, y));
    }
}
impl<B> Drop for Terminal<B>
where
    B: Backend,
{
    fn drop(&mut self) {
        if self.hidden_cursor {
            if let Err(err) = self.show_cursor() {
                eprintln!("Failed to show the cursor: {}", err);
            }
        }
    }
}
impl<B> Terminal<B>
where
    B: Backend,
{
    /// Wrapper around Terminal initialization. Each buffer is initialized with a blank string and
    /// default colors for the foreground and the background
    pub fn new(backend: B) -> io::Result<Terminal<B>> {
        let size = backend.size()?;
        Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport {
                    area: size,
                    resize_behavior: ResizeBehavior::Auto,
                },
            },
        )
    }
    /// UNSTABLE
    pub fn with_options(
        backend: B,
        options: TerminalOptions,
    ) -> io::Result<Terminal<B>> {
        Ok(Terminal {
            backend,
            buffers: [
                Buffer::empty(options.viewport.area),
                Buffer::empty(options.viewport.area),
            ],
            current: 0,
            hidden_cursor: false,
            viewport: options.viewport,
        })
    }
    /// Get a Frame object which provides a consistent view into the terminal state for rendering.
    pub fn get_frame(&mut self) -> Frame<B> {
        Frame {
            terminal: self,
            cursor_position: None,
        }
    }
    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current]
    }
    pub fn backend(&self) -> &B {
        &self.backend
    }
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
    /// Obtains a difference between the previous and the current buffer and passes it to the
    /// current backend for drawing.
    pub fn flush(&mut self) -> io::Result<()> {
        let previous_buffer = &self.buffers[1 - self.current];
        let current_buffer = &self.buffers[self.current];
        let updates = previous_buffer.diff(current_buffer);
        self.backend.draw(updates.into_iter())
    }
    /// Updates the Terminal so that internal buffers match the requested size. Requested size will
    /// be saved so the size can remain consistent when rendering.
    /// This leads to a full clear of the screen.
    pub fn resize(&mut self, area: Rect) -> io::Result<()> {
        self.buffers[self.current].resize(area);
        self.buffers[1 - self.current].resize(area);
        self.viewport.area = area;
        self.clear()
    }
    /// Queries the backend for size and resizes if it doesn't match the previous size.
    pub fn autoresize(&mut self) -> io::Result<()> {
        if self.viewport.resize_behavior == ResizeBehavior::Auto {
            let size = self.size()?;
            if size != self.viewport.area {
                self.resize(size)?;
            }
        }
        Ok(())
    }
    /// Synchronizes terminal size, calls the rendering closure, flushes the current internal state
    /// and prepares for the next draw call.
    pub fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame<B>),
    {
        self.autoresize()?;
        let mut frame = self.get_frame();
        f(&mut frame);
        let cursor_position = frame.cursor_position;
        self.flush()?;
        match cursor_position {
            None => self.hide_cursor()?,
            Some((x, y)) => {
                self.show_cursor()?;
                self.set_cursor(x, y)?;
            }
        }
        self.buffers[1 - self.current].reset();
        self.current = 1 - self.current;
        self.backend.flush()?;
        Ok(())
    }
    pub fn hide_cursor(&mut self) -> io::Result<()> {
        self.backend.hide_cursor()?;
        self.hidden_cursor = true;
        Ok(())
    }
    pub fn show_cursor(&mut self) -> io::Result<()> {
        self.backend.show_cursor()?;
        self.hidden_cursor = false;
        Ok(())
    }
    pub fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        self.backend.get_cursor()
    }
    pub fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.backend.set_cursor(x, y)
    }
    /// Clear the terminal and force a full redraw on the next draw call.
    pub fn clear(&mut self) -> io::Result<()> {
        self.backend.clear()?;
        self.buffers[1 - self.current].reset();
        Ok(())
    }
    /// Queries the real size of the backend.
    pub fn size(&self) -> io::Result<Rect> {
        self.backend.size()
    }
}
#[cfg(test)]
mod tests_llm_16_208 {
    use super::*;
    use crate::*;
    use crate::backend::*;
    use crate::buffer::*;
    use crate::layout::Rect;
    use crate::style::Color;
    use std::io::Write;
    use termion::terminal_size;
    #[test]
    fn test_size() {
        let _rug_st_tests_llm_16_208_rrrruuuugggg_test_size = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let stdout = std::io::stdout();
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        let actual = terminal.size().unwrap();
        let expected = {
            let terminal_size = terminal_size().unwrap();
            let area = Rect::new(
                rug_fuzz_0,
                rug_fuzz_1,
                terminal_size.0,
                terminal_size.1,
            );
            area
        };
        debug_assert_eq!(actual, expected);
        let _rug_ed_tests_llm_16_208_rrrruuuugggg_test_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_233 {
    use super::*;
    use crate::*;
    use crate::style::*;
    use crate::buffer::Cell;
    #[test]
    fn test_set_cursor() {
        struct MockBackend {}
        impl Backend for MockBackend {
            fn draw<'a, I>(&mut self, content: I) -> Result<(), io::Error>
            where
                I: Iterator<Item = (u16, u16, &'a Cell)>,
            {
                Ok(())
            }
            fn hide_cursor(&mut self) -> Result<(), io::Error> {
                Ok(())
            }
            fn show_cursor(&mut self) -> Result<(), io::Error> {
                Ok(())
            }
            fn get_cursor(&mut self) -> Result<(u16, u16), io::Error> {
                Ok((0, 0))
            }
            fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), io::Error> {
                Ok(())
            }
            fn clear(&mut self) -> Result<(), io::Error> {
                Ok(())
            }
            fn size(&self) -> Result<Rect, io::Error> {
                Ok(Rect::new(0, 0, 10, 10))
            }
            fn flush(&mut self) -> Result<(), io::Error> {
                Ok(())
            }
        }
        let mut terminal = Terminal::with_options(
                MockBackend {},
                TerminalOptions {
                    viewport: Viewport::fixed(Rect::new(0, 0, 10, 10)),
                },
            )
            .unwrap();
        let result = terminal.set_cursor(5, 5);
        assert!(result.is_ok());
    }
}
#[cfg(test)]
mod tests_llm_16_235 {
    use super::*;
    use crate::*;
    use std::io;
    use crate::buffer::Cell;
    use crate::backend::Backend;
    use crate::layout::Rect;
    use crate::symbols::line::{DOUBLE, NORMAL, ROUNDED, THICK};
    use crate::buffer::Buffer;
    use crate::style::{Color, Modifier, Style};
    use crate::terminal::{Terminal, TerminalOptions, ResizeBehavior};
    use crate::terminal::Viewport;
    struct MockBackend;
    impl Backend for MockBackend {
        fn draw<'a, I>(&mut self, content: I) -> Result<(), io::Error>
        where
            I: Iterator<Item = (u16, u16, &'a Cell)>,
        {
            Ok(())
        }
        fn hide_cursor(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
        fn show_cursor(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
        fn get_cursor(&mut self) -> Result<(u16, u16), io::Error> {
            Ok((0, 0))
        }
        fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), io::Error> {
            Ok(())
        }
        fn clear(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
        fn size(&self) -> Result<Rect, io::Error> {
            Ok(Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 24,
            })
        }
        fn flush(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
    }
    #[test]
    fn test_show_cursor() {
        let _rug_st_tests_llm_16_235_rrrruuuugggg_test_show_cursor = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 80;
        let rug_fuzz_3 = 24;
        let mut terminal = Terminal::with_options(
                MockBackend {},
                TerminalOptions {
                    viewport: Viewport {
                        area: Rect {
                            x: rug_fuzz_0,
                            y: rug_fuzz_1,
                            width: rug_fuzz_2,
                            height: rug_fuzz_3,
                        },
                        resize_behavior: ResizeBehavior::Fixed,
                    },
                },
            )
            .unwrap();
        let result = terminal.show_cursor();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_235_rrrruuuugggg_test_show_cursor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_240 {
    use super::*;
    use crate::*;
    use crate::terminal::{Viewport, ResizeBehavior};
    use crate::layout::Rect;
    #[test]
    fn test_fixed() {
        let _rug_st_tests_llm_16_240_rrrruuuugggg_test_fixed = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 100;
        let rug_fuzz_3 = 100;
        let area = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        let expected = Viewport {
            area: area.clone(),
            resize_behavior: ResizeBehavior::Fixed,
        };
        let result = Viewport::fixed(area);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_240_rrrruuuugggg_test_fixed = 0;
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::Terminal;
    use crate::backend::TestBackend;
    use crate::widgets::{Block, Borders, Table, Row};
    use crate::style::{Style, Color};
    use crate::layout::Rect;
    #[test]
    fn test_render_widget() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_render_widget = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 5;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();
        let block = Block::default();
        let area = Rect::new(rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5);
        frame.render_widget(block, area);
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_render_widget = 0;
    }
}
#[cfg(test)]
mod tests_rug_46 {
    use super::*;
    use crate::Terminal;
    use crate::backend::TestBackend;
    use crate::layout::Rect;
    use crate::widgets::{List, ListItem, ListState};
    #[test]
    fn test_render_stateful_widget() {
        let _rug_st_tests_rug_46_rrrruuuugggg_test_render_stateful_widget = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = "Item 1";
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 5;
        let rug_fuzz_7 = 5;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = ListState::default();
        state.select(Some(rug_fuzz_2));
        let items = vec![ListItem::new(rug_fuzz_3), ListItem::new("Item 2")];
        let list = List::new(items);
        let area = Rect::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7);
        terminal.get_frame().render_stateful_widget(list, area, &mut state);
        let _rug_ed_tests_rug_46_rrrruuuugggg_test_render_stateful_widget = 0;
    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_51_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut p0 = Terminal::<TestBackend>::new(backend).unwrap();
        p0.get_frame();
        let _rug_ed_tests_rug_51_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        let return_value = terminal.current_buffer_mut();
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        let p0 = &terminal;
        terminal.backend();
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_backend_mut() {
        let _rug_st_tests_rug_54_rrrruuuugggg_test_backend_mut = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut p0: &mut Terminal<TestBackend> = &mut terminal;
        terminal.backend_mut();
        let _rug_ed_tests_rug_54_rrrruuuugggg_test_backend_mut = 0;
    }
}
#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::terminal::Terminal;
    #[test]
    fn test_flush() {
        let _rug_st_tests_rug_55_rrrruuuugggg_test_flush = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.flush().unwrap();
        let _rug_ed_tests_rug_55_rrrruuuugggg_test_flush = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::layout::Rect;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_resize() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_resize = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal: Terminal<TestBackend> = Terminal::new(backend).unwrap();
        let area: Rect = Rect::default();
        terminal.resize(area).unwrap();
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_resize = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_57_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.autoresize().unwrap();
        let _rug_ed_tests_rug_57_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_58 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::terminal::{Frame, Terminal};
    use std::io;
    #[test]
    fn test_draw() {
        let _rug_st_tests_rug_58_rrrruuuugggg_test_draw = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        let f = |frame: &mut Frame<TestBackend>| {};
        terminal.draw(f).unwrap();
        let _rug_ed_tests_rug_58_rrrruuuugggg_test_draw = 0;
    }
}
#[cfg(test)]
mod tests_rug_59 {
    use super::*;
    use crate::terminal::Terminal;
    use crate::backend::TestBackend;
    use std::io;
    #[test]
    fn test_hide_cursor() {
        let _rug_st_tests_rug_59_rrrruuuugggg_test_hide_cursor = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        let result: io::Result<()> = terminal.hide_cursor();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_59_rrrruuuugggg_test_hide_cursor = 0;
    }
}
#[cfg(test)]
mod tests_rug_60 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_get_cursor() {
        let _rug_st_tests_rug_60_rrrruuuugggg_test_get_cursor = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.get_cursor();
        let _rug_ed_tests_rug_60_rrrruuuugggg_test_get_cursor = 0;
    }
}
#[cfg(test)]
mod tests_rug_61 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_61_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.clear().unwrap();
        let _rug_ed_tests_rug_61_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_62 {
    use super::*;
    use crate::backend::TestBackend;
    use crate::Terminal;
    #[test]
    fn test_size() {
        let _rug_st_tests_rug_62_rrrruuuugggg_test_size = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.size().unwrap();
        let _rug_ed_tests_rug_62_rrrruuuugggg_test_size = 0;
    }
}
