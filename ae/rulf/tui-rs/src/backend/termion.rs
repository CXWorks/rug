use super::Backend;
use crate::{buffer::Cell, layout::Rect, style::{Color, Modifier}};
use std::{fmt, io::{self, Write}};
pub struct TermionBackend<W>
where
    W: Write,
{
    stdout: W,
}
impl<W> TermionBackend<W>
where
    W: Write,
{
    pub fn new(stdout: W) -> TermionBackend<W> {
        TermionBackend { stdout }
    }
}
impl<W> Write for TermionBackend<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}
impl<W> Backend for TermionBackend<W>
where
    W: Write,
{
    /// Clears the entire screen and move the cursor to the top left of the screen
    fn clear(&mut self) -> io::Result<()> {
        write!(self.stdout, "{}", termion::clear::All)?;
        write!(self.stdout, "{}", termion::cursor::Goto(1, 1))?;
        self.stdout.flush()
    }
    /// Hides cursor
    fn hide_cursor(&mut self) -> io::Result<()> {
        write!(self.stdout, "{}", termion::cursor::Hide)?;
        self.stdout.flush()
    }
    /// Shows cursor
    fn show_cursor(&mut self) -> io::Result<()> {
        write!(self.stdout, "{}", termion::cursor::Show)?;
        self.stdout.flush()
    }
    /// Gets cursor position (0-based index)
    fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        termion::cursor::DetectCursorPos::cursor_pos(&mut self.stdout)
            .map(|(x, y)| (x - 1, y - 1))
    }
    /// Sets cursor position (0-based index)
    fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        write!(self.stdout, "{}", termion::cursor::Goto(x + 1, y + 1))?;
        self.stdout.flush()
    }
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        use std::fmt::Write;
        let mut string = String::with_capacity(content.size_hint().0 * 3);
        let mut fg = Color::Reset;
        let mut bg = Color::Reset;
        let mut modifier = Modifier::empty();
        let mut last_pos: Option<(u16, u16)> = None;
        for (x, y, cell) in content {
            if !matches!(last_pos, Some(p) if x == p.0 + 1 && y == p.1) {
                write!(string, "{}", termion::cursor::Goto(x + 1, y + 1)).unwrap();
            }
            last_pos = Some((x, y));
            if cell.modifier != modifier {
                write!(
                    string, "{}", ModifierDiff { from : modifier, to : cell.modifier }
                )
                    .unwrap();
                modifier = cell.modifier;
            }
            if cell.fg != fg {
                write!(string, "{}", Fg(cell.fg)).unwrap();
                fg = cell.fg;
            }
            if cell.bg != bg {
                write!(string, "{}", Bg(cell.bg)).unwrap();
                bg = cell.bg;
            }
            string.push_str(&cell.symbol);
        }
        write!(
            self.stdout, "{}{}{}{}", string, Fg(Color::Reset), Bg(Color::Reset),
            termion::style::Reset,
        )
    }
    /// Return the size of the terminal
    fn size(&self) -> io::Result<Rect> {
        let terminal = termion::terminal_size()?;
        Ok(Rect::new(0, 0, terminal.0, terminal.1))
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}
struct Fg(Color);
struct Bg(Color);
struct ModifierDiff {
    from: Modifier,
    to: Modifier,
}
impl fmt::Display for Fg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use termion::color::Color as TermionColor;
        match self.0 {
            Color::Reset => termion::color::Reset.write_fg(f),
            Color::Black => termion::color::Black.write_fg(f),
            Color::Red => termion::color::Red.write_fg(f),
            Color::Green => termion::color::Green.write_fg(f),
            Color::Yellow => termion::color::Yellow.write_fg(f),
            Color::Blue => termion::color::Blue.write_fg(f),
            Color::Magenta => termion::color::Magenta.write_fg(f),
            Color::Cyan => termion::color::Cyan.write_fg(f),
            Color::Gray => termion::color::White.write_fg(f),
            Color::DarkGray => termion::color::LightBlack.write_fg(f),
            Color::LightRed => termion::color::LightRed.write_fg(f),
            Color::LightGreen => termion::color::LightGreen.write_fg(f),
            Color::LightBlue => termion::color::LightBlue.write_fg(f),
            Color::LightYellow => termion::color::LightYellow.write_fg(f),
            Color::LightMagenta => termion::color::LightMagenta.write_fg(f),
            Color::LightCyan => termion::color::LightCyan.write_fg(f),
            Color::White => termion::color::LightWhite.write_fg(f),
            Color::Indexed(i) => termion::color::AnsiValue(i).write_fg(f),
            Color::Rgb(r, g, b) => termion::color::Rgb(r, g, b).write_fg(f),
        }
    }
}
impl fmt::Display for Bg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use termion::color::Color as TermionColor;
        match self.0 {
            Color::Reset => termion::color::Reset.write_bg(f),
            Color::Black => termion::color::Black.write_bg(f),
            Color::Red => termion::color::Red.write_bg(f),
            Color::Green => termion::color::Green.write_bg(f),
            Color::Yellow => termion::color::Yellow.write_bg(f),
            Color::Blue => termion::color::Blue.write_bg(f),
            Color::Magenta => termion::color::Magenta.write_bg(f),
            Color::Cyan => termion::color::Cyan.write_bg(f),
            Color::Gray => termion::color::White.write_bg(f),
            Color::DarkGray => termion::color::LightBlack.write_bg(f),
            Color::LightRed => termion::color::LightRed.write_bg(f),
            Color::LightGreen => termion::color::LightGreen.write_bg(f),
            Color::LightBlue => termion::color::LightBlue.write_bg(f),
            Color::LightYellow => termion::color::LightYellow.write_bg(f),
            Color::LightMagenta => termion::color::LightMagenta.write_bg(f),
            Color::LightCyan => termion::color::LightCyan.write_bg(f),
            Color::White => termion::color::LightWhite.write_bg(f),
            Color::Indexed(i) => termion::color::AnsiValue(i).write_bg(f),
            Color::Rgb(r, g, b) => termion::color::Rgb(r, g, b).write_bg(f),
        }
    }
}
impl fmt::Display for ModifierDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let remove = self.from - self.to;
        if remove.contains(Modifier::REVERSED) {
            write!(f, "{}", termion::style::NoInvert)?;
        }
        if remove.contains(Modifier::BOLD) {
            write!(f, "{}", termion::style::NoFaint)?;
            if self.to.contains(Modifier::DIM) {
                write!(f, "{}", termion::style::Faint)?;
            }
        }
        if remove.contains(Modifier::ITALIC) {
            write!(f, "{}", termion::style::NoItalic)?;
        }
        if remove.contains(Modifier::UNDERLINED) {
            write!(f, "{}", termion::style::NoUnderline)?;
        }
        if remove.contains(Modifier::DIM) {
            write!(f, "{}", termion::style::NoFaint)?;
            if self.to.contains(Modifier::BOLD) {
                write!(f, "{}", termion::style::Bold)?;
            }
        }
        if remove.contains(Modifier::CROSSED_OUT) {
            write!(f, "{}", termion::style::NoCrossedOut)?;
        }
        if remove.contains(Modifier::SLOW_BLINK)
            || remove.contains(Modifier::RAPID_BLINK)
        {
            write!(f, "{}", termion::style::NoBlink)?;
        }
        let add = self.to - self.from;
        if add.contains(Modifier::REVERSED) {
            write!(f, "{}", termion::style::Invert)?;
        }
        if add.contains(Modifier::BOLD) {
            write!(f, "{}", termion::style::Bold)?;
        }
        if add.contains(Modifier::ITALIC) {
            write!(f, "{}", termion::style::Italic)?;
        }
        if add.contains(Modifier::UNDERLINED) {
            write!(f, "{}", termion::style::Underline)?;
        }
        if add.contains(Modifier::DIM) {
            write!(f, "{}", termion::style::Faint)?;
        }
        if add.contains(Modifier::CROSSED_OUT) {
            write!(f, "{}", termion::style::CrossedOut)?;
        }
        if add.contains(Modifier::SLOW_BLINK) || add.contains(Modifier::RAPID_BLINK) {
            write!(f, "{}", termion::style::Blink)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;
    use crate::*;
    use std::io::Write;
    use termion::clear::All;
    use termion::cursor::Goto;
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let mut stdout = Vec::new();
        let mut backend = TermionBackend::new(&mut stdout);
        backend.clear().unwrap();
        let expected_output = format!("{}{}", All, Goto(rug_fuzz_0, rug_fuzz_1));
        let actual_output = String::from_utf8(stdout).unwrap();
        debug_assert_eq!(actual_output, expected_output);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_8 {
    use super::*;
    use crate::*;
    use std::io::{self, Write};
    struct MockWriter;
    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn test_hide_cursor() {
        let _rug_st_tests_llm_16_8_rrrruuuugggg_test_hide_cursor = 0;
        let mut backend = TermionBackend {
            stdout: MockWriter,
        };
        let result = backend.hide_cursor();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_8_rrrruuuugggg_test_hide_cursor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9 {
    use crate::backend::termion::TermionBackend;
    use crate::backend::Backend;
    use std::io;
    use std::io::Write;
    use termion::cursor::Goto;
    #[test]
    fn test_set_cursor() {
        let _rug_st_tests_llm_16_9_rrrruuuugggg_test_set_cursor = 0;
        let rug_fuzz_0 = 4;
        let rug_fuzz_1 = 6;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 1;
        let mut stdout = Vec::new();
        let mut backend = TermionBackend::new(&mut stdout);
        let x = rug_fuzz_0;
        let y = rug_fuzz_1;
        let result = backend.set_cursor(x, y);
        debug_assert!(result.is_ok());
        let expected_output = format!("{}", Goto(x + rug_fuzz_2, y + rug_fuzz_3));
        debug_assert_eq!(stdout, expected_output.as_bytes());
        let _rug_ed_tests_llm_16_9_rrrruuuugggg_test_set_cursor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_10 {
    use super::*;
    use crate::*;
    struct MockWriter;
    impl Write for MockWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Ok(0)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn test_show_cursor() {
        let _rug_st_tests_llm_16_10_rrrruuuugggg_test_show_cursor = 0;
        let mut backend = TermionBackend::new(MockWriter);
        let result = backend.show_cursor();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_10_rrrruuuugggg_test_show_cursor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_13 {
    use crate::backend::termion::TermionBackend;
    use std::io::Write;
    #[test]
    fn test_flush() {
        struct DummyWriter;
        impl Write for DummyWriter {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Ok(0)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut backend = TermionBackend::new(DummyWriter);
        let result = backend.flush();
        assert!(result.is_ok());
    }
}
#[cfg(test)]
mod tests_llm_16_15 {
    use super::*;
    use crate::*;
    use crate::backend::termion::TermionBackend;
    use crate::backend::Backend;
    use crate::style::{Color, Modifier};
    use crate::symbols::Marker;
    use crate::text::Span;
    use std::io::{self, Write};
    #[test]
    fn test_write() {
        let _rug_st_tests_llm_16_15_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let mut stdout = Vec::new();
        let mut backend = TermionBackend::new(&mut stdout);
        let buf = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let result = backend.write(&buf);
        debug_assert_eq!(result.is_ok(), true);
        debug_assert_eq!(result.unwrap(), buf.len());
        let _rug_ed_tests_llm_16_15_rrrruuuugggg_test_write = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_128 {
    use super::*;
    use crate::*;
    use std::io::Write;
    struct MockWriter;
    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_128_rrrruuuugggg_test_new = 0;
        let stdout = MockWriter;
        let backend = TermionBackend::new(stdout);
        let _rug_ed_tests_llm_16_128_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::backend::Backend;
    use crate::backend::termion::TermionBackend;
    use termion::raw::IntoRawMode;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_33_rrrruuuugggg_test_rug = 0;
        let stdout = std::io::stdout().into_raw_mode().unwrap();
        let backend = TermionBackend::new(stdout);
        let mut p0: TermionBackend<_> = backend;
        <TermionBackend<_> as Backend>::get_cursor(&mut p0);
        let _rug_ed_tests_rug_33_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use crate::backend::Backend;
    use crate::backend::termion::TermionBackend;
    use termion::raw::IntoRawMode;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_35_rrrruuuugggg_test_rug = 0;
        let stdout = std::io::stdout().into_raw_mode().unwrap();
        let backend = TermionBackend::new(stdout);
        let mut p0: TermionBackend<_> = backend;
        p0.size().unwrap();
        let _rug_ed_tests_rug_35_rrrruuuugggg_test_rug = 0;
    }
}
