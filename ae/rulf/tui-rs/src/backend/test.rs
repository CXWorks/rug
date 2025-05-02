use crate::{
    backend::Backend, buffer::{Buffer, Cell},
    layout::Rect,
};
use std::{fmt::Write, io};
use unicode_width::UnicodeWidthStr;
/// A backend used for the integration tests.
#[derive(Debug)]
pub struct TestBackend {
    width: u16,
    buffer: Buffer,
    height: u16,
    cursor: bool,
    pos: (u16, u16),
}
/// Returns a string representation of the given buffer for debugging purpose.
fn buffer_view(buffer: &Buffer) -> String {
    let mut view = String::with_capacity(
        buffer.content.len() + buffer.area.height as usize * 3,
    );
    for cells in buffer.content.chunks(buffer.area.width as usize) {
        let mut overwritten = vec![];
        let mut skip: usize = 0;
        view.push('"');
        for (x, c) in cells.iter().enumerate() {
            if skip == 0 {
                view.push_str(&c.symbol);
            } else {
                overwritten.push((x, &c.symbol))
            }
            skip = std::cmp::max(skip, c.symbol.width()).saturating_sub(1);
        }
        view.push('"');
        if !overwritten.is_empty() {
            write!(& mut view, " Hidden by multi-width symbols: {:?}", overwritten)
                .unwrap();
        }
        view.push('\n');
    }
    view
}
impl TestBackend {
    pub fn new(width: u16, height: u16) -> TestBackend {
        TestBackend {
            width,
            height,
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            cursor: false,
            pos: (0, 0),
        }
    }
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    pub fn assert_buffer(&self, expected: &Buffer) {
        assert_eq!(expected.area, self.buffer.area);
        let diff = expected.diff(&self.buffer);
        if diff.is_empty() {
            return;
        }
        let mut debug_info = String::from("Buffers are not equal");
        debug_info.push('\n');
        debug_info.push_str("Expected:");
        debug_info.push('\n');
        let expected_view = buffer_view(expected);
        debug_info.push_str(&expected_view);
        debug_info.push('\n');
        debug_info.push_str("Got:");
        debug_info.push('\n');
        let view = buffer_view(&self.buffer);
        debug_info.push_str(&view);
        debug_info.push('\n');
        debug_info.push_str("Diff:");
        debug_info.push('\n');
        let nice_diff = diff
            .iter()
            .enumerate()
            .map(|(i, (x, y, cell))| {
                let expected_cell = expected.get(*x, *y);
                format!(
                    "{}: at ({}, {}) expected {:?} got {:?}", i, x, y, expected_cell,
                    cell
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        debug_info.push_str(&nice_diff);
        panic!(debug_info);
    }
}
impl Backend for TestBackend {
    fn draw<'a, I>(&mut self, content: I) -> Result<(), io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        for (x, y, c) in content {
            let cell = self.buffer.get_mut(x, y);
            *cell = c.clone();
        }
        Ok(())
    }
    fn hide_cursor(&mut self) -> Result<(), io::Error> {
        self.cursor = false;
        Ok(())
    }
    fn show_cursor(&mut self) -> Result<(), io::Error> {
        self.cursor = true;
        Ok(())
    }
    fn get_cursor(&mut self) -> Result<(u16, u16), io::Error> {
        Ok(self.pos)
    }
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), io::Error> {
        self.pos = (x, y);
        Ok(())
    }
    fn clear(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
    fn size(&self) -> Result<Rect, io::Error> {
        Ok(Rect::new(0, 0, self.width, self.height))
    }
    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_20 {
    use super::*;
    use crate::*;
    use crate::backend::Backend;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use std::io;
    #[test]
    fn test_flush() {
        let _rug_st_tests_llm_16_20_rrrruuuugggg_test_flush = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 10;
        let rug_fuzz_5 = 10;
        let mut backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        backend.flush().unwrap();
        backend
            .assert_buffer(
                &Buffer::empty(Rect::new(rug_fuzz_2, rug_fuzz_3, rug_fuzz_4, rug_fuzz_5)),
            );
        let _rug_ed_tests_llm_16_20_rrrruuuugggg_test_flush = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_21 {
    use super::*;
    use crate::*;
    use crate::buffer::Cell;
    use crate::layout::Rect;
    #[test]
    fn test_get_cursor() {
        let _rug_st_tests_llm_16_21_rrrruuuugggg_test_get_cursor = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 5;
        let mut test_backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        test_backend.pos = (rug_fuzz_2, rug_fuzz_3);
        let result = test_backend.get_cursor().unwrap();
        let expected = (rug_fuzz_4, rug_fuzz_5);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_21_rrrruuuugggg_test_get_cursor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_22 {
    use super::*;
    use crate::*;
    use std::io;
    use crate::backend::{Backend, TestBackend};
    #[test]
    fn test_hide_cursor() {
        let _rug_st_tests_llm_16_22_rrrruuuugggg_test_hide_cursor = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let mut backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(backend.cursor, true);
        backend.hide_cursor().unwrap();
        debug_assert_eq!(backend.cursor, false);
        let _rug_ed_tests_llm_16_22_rrrruuuugggg_test_hide_cursor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_23 {
    use std::io;
    use crate::backend::{Backend, test::TestBackend};
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    fn create_backend(width: u16, height: u16) -> TestBackend {
        TestBackend::new(width, height)
    }
    #[test]
    fn test_set_cursor() -> io::Result<()> {
        let mut backend = create_backend(10, 10);
        backend.set_cursor(5, 5)?;
        let cursor_pos = backend.get_cursor()?;
        assert_eq!(cursor_pos, (5, 5));
        Ok(())
    }
}
#[cfg(test)]
mod tests_llm_16_26 {
    use std::io;
    use crate::backend::Backend;
    use crate::buffer::{Buffer, Cell};
    use crate::layout::Rect;
    struct MockBackend {
        width: u16,
        height: u16,
    }
    impl Backend for MockBackend {
        fn draw<'a, I>(&mut self, _content: I) -> Result<(), io::Error>
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
        fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), io::Error> {
            Ok(())
        }
        fn clear(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
        fn size(&self) -> Result<Rect, io::Error> {
            Ok(Rect::new(0, 0, self.width, self.height))
        }
        fn flush(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
    }
    fn size_returns_correct_result() {
        let _rug_st_tests_llm_16_26_rrrruuuugggg_size_returns_correct_result = 0;
        let rug_fuzz_0 = 50;
        let rug_fuzz_1 = 30;
        let mut backend = MockBackend {
            width: rug_fuzz_0,
            height: rug_fuzz_1,
        };
        let result = backend.size().unwrap();
        debug_assert_eq!(result.x, 0);
        debug_assert_eq!(result.y, 0);
        debug_assert_eq!(result.width, 50);
        debug_assert_eq!(result.height, 30);
        let _rug_ed_tests_llm_16_26_rrrruuuugggg_size_returns_correct_result = 0;
    }
    #[test]
    fn test_size() {
        let _rug_st_tests_llm_16_26_rrrruuuugggg_test_size = 0;
        size_returns_correct_result();
        let _rug_ed_tests_llm_16_26_rrrruuuugggg_test_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_129 {
    use super::*;
    use crate::*;
    use crate::buffer::Cell;
    use crate::layout::Rect;
    use crate::style::{Color, Style, Modifier};
    #[test]
    fn test_assert_buffer() {
        let _rug_st_tests_llm_16_129_rrrruuuugggg_test_assert_buffer = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = "..........";
        let rug_fuzz_3 = "..........";
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = "Hello";
        let rug_fuzz_7 = "Hello.....";
        let rug_fuzz_8 = 5;
        let rug_fuzz_9 = 1;
        let rug_fuzz_10 = "World";
        let rug_fuzz_11 = "Hello.....";
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 3;
        let rug_fuzz_14 = "Foo";
        let rug_fuzz_15 = 3;
        let rug_fuzz_16 = 3;
        let rug_fuzz_17 = "Bar";
        let mut backend = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        let expected = Buffer::with_lines(
            vec![rug_fuzz_2, "..........", "..........", "..........", ".........."],
        );
        backend.assert_buffer(&expected);
        let mut expected = Buffer::with_lines(
            vec![rug_fuzz_3, "..........", "..........", "..........", ".........."],
        );
        expected
            .set_string(
                rug_fuzz_4,
                rug_fuzz_5,
                rug_fuzz_6,
                Style::default().fg(Color::Red),
            );
        backend.assert_buffer(&expected);
        let mut expected = Buffer::with_lines(
            vec![rug_fuzz_7, "..........", "..........", "..........", ".........."],
        );
        expected
            .set_string(
                rug_fuzz_8,
                rug_fuzz_9,
                rug_fuzz_10,
                Style::default().fg(Color::Blue),
            );
        backend.assert_buffer(&expected);
        let mut expected = Buffer::with_lines(
            vec![rug_fuzz_11, ".....World", "..........", "..........", ".........."],
        );
        expected
            .set_string(
                rug_fuzz_12,
                rug_fuzz_13,
                rug_fuzz_14,
                Style::default().fg(Color::Green),
            );
        expected
            .set_string(
                rug_fuzz_15,
                rug_fuzz_16,
                rug_fuzz_17,
                Style::default().fg(Color::Yellow),
            );
        backend.assert_buffer(&expected);
        let _rug_ed_tests_llm_16_129_rrrruuuugggg_test_assert_buffer = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_134 {
    use super::*;
    use crate::*;
    use crate::buffer::Cell;
    use crate::layout::Rect;
    use crate::style::{Color, Style, Modifier};
    #[test]
    fn test_buffer_view() {
        let _rug_st_tests_llm_16_134_rrrruuuugggg_test_buffer_view = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 4;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = "a";
        let rug_fuzz_5 = "\"abcd\"\n\"efgh\"\n\"ijkl\"\n\"m\"\n";
        let buffer = Buffer {
            area: Rect {
                x: rug_fuzz_0,
                y: rug_fuzz_1,
                width: rug_fuzz_2,
                height: rug_fuzz_3,
            },
            content: vec![
                Cell { symbol : String::from(rug_fuzz_4), fg : Color::Reset, bg :
                Color::Reset, modifier : Modifier::empty(), }, Cell { symbol :
                String::from("b"), fg : Color::Reset, bg : Color::Reset, modifier :
                Modifier::empty(), }, Cell { symbol : String::from("c"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("d"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("e"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("f"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("g"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("h"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("i"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("j"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("k"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }, Cell {
                symbol : String::from("l"), fg : Color::Reset, bg : Color::Reset,
                modifier : Modifier::empty(), }, Cell { symbol : String::from("m"), fg :
                Color::Reset, bg : Color::Reset, modifier : Modifier::empty(), }
            ],
        };
        let expected = rug_fuzz_5;
        let result = buffer_view(&buffer);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_134_rrrruuuugggg_test_buffer_view = 0;
    }
}
#[cfg(test)]
mod tests_rug_1 {
    use super::*;
    use crate::backend::test::TestBackend;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_1_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let mut p0: u16 = rug_fuzz_0;
        let mut p1: u16 = rug_fuzz_1;
        TestBackend::new(p0, p1);
        let _rug_ed_tests_rug_1_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_2 {
    use super::*;
    use crate::backend::test::{Buffer, TestBackend};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_2_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let mut p0 = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        crate::backend::test::TestBackend::buffer(&p0);
        let _rug_ed_tests_rug_2_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    use crate::backend::Backend;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_4_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        #[cfg(test)]
        mod tests_rug_4_prepare {
            use crate::backend::test::{Buffer, TestBackend};
            #[test]
            fn sample() {
                let _rug_st_tests_rug_4_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 10;
                let rug_fuzz_2 = 10;
                let rug_fuzz_3 = 0;
                let _rug_st_tests_rug_4_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let rug_fuzz_1 = rug_fuzz_2;
                let mut v1 = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
                let _rug_ed_tests_rug_4_rrrruuuugggg_sample = rug_fuzz_3;
                let _rug_ed_tests_rug_4_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0 = TestBackend::new(10, 10);
        p0.show_cursor().unwrap();
        let _rug_ed_tests_rug_4_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::backend::Backend;
    use crate::backend::test::{Buffer, TestBackend};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_5_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let mut p0 = TestBackend::new(rug_fuzz_0, rug_fuzz_1);
        p0.clear().unwrap();
        let _rug_ed_tests_rug_5_rrrruuuugggg_test_rug = 0;
    }
}
