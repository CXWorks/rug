use crate::buffer::Buffer;
use crate::layout::Rect;
use crate::widgets::Widget;
/// A widget to to clear/reset a certain area to allow overdrawing (e.g. for popups)
///
/// # Examples
///
/// ```
/// # use tui::widgets::{Clear, Block, Borders};
/// # use tui::layout::Rect;
/// # use tui::Frame;
/// # use tui::backend::Backend;
/// fn draw_on_clear<B: Backend>(f: &mut Frame<B>, area: Rect) {
///     let block = Block::default().title("Block").borders(Borders::ALL);
///     f.render_widget(Clear, area); // <- this will clear/reset the area first
///     f.render_widget(block, area); // now render the block widget
/// }
/// ```
///
/// # Popup Example
///
/// For a more complete example how to utilize `Clear` to realize popups see
/// the example `examples/popup.rs`
#[derive(Debug, Clone)]
pub struct Clear;
impl Widget for Clear {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                buf.get_mut(x, y).reset();
            }
        }
    }
}
#[cfg(test)]
mod tests_llm_16_99 {
    use super::*;
    use crate::*;
    use crate::buffer::Buffer;
    use crate::layout::Rect;
    use crate::style::{Color, Style, Modifier};
    #[test]
    fn test_render() {
        let _rug_st_tests_llm_16_99_rrrruuuugggg_test_render = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 5;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 10;
        let rug_fuzz_11 = 5;
        let rug_fuzz_12 = 0;
        let rug_fuzz_13 = 10;
        let rug_fuzz_14 = 0;
        let rug_fuzz_15 = 5;
        let mut buf = Buffer::empty(Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        });
        let area = Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        };
        <Clear as Widget>::render(Clear, area, &mut buf);
        let mut expected_buf = Buffer::empty(Rect {
            x: rug_fuzz_8,
            y: rug_fuzz_9,
            width: rug_fuzz_10,
            height: rug_fuzz_11,
        });
        for x in rug_fuzz_12..rug_fuzz_13 {
            for y in rug_fuzz_14..rug_fuzz_15 {
                expected_buf.get_mut(x, y).reset();
            }
        }
        debug_assert_eq!(buf, expected_buf);
        let _rug_ed_tests_llm_16_99_rrrruuuugggg_test_render = 0;
    }
}
