use crate::{style::Color, widgets::canvas::{Line, Painter, Shape}};
/// Shape to draw a rectangle from a `Rect` with the given color
#[derive(Debug, Clone)]
pub struct Rectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: Color,
}
impl Shape for Rectangle {
    fn draw(&self, painter: &mut Painter) {
        let lines: [Line; 4] = [
            Line {
                x1: self.x,
                y1: self.y,
                x2: self.x,
                y2: self.y + self.height,
                color: self.color,
            },
            Line {
                x1: self.x,
                y1: self.y + self.height,
                x2: self.x + self.width,
                y2: self.y + self.height,
                color: self.color,
            },
            Line {
                x1: self.x + self.width,
                y1: self.y,
                x2: self.x + self.width,
                y2: self.y + self.height,
                color: self.color,
            },
            Line {
                x1: self.x,
                y1: self.y,
                x2: self.x + self.width,
                y2: self.y,
                color: self.color,
            },
        ];
        for line in &lines {
            line.draw(painter);
        }
    }
}
#[cfg(test)]
mod tests_llm_16_93 {
    use super::*;
    use crate::*;
    use crate::{
        style::Color,
        widgets::canvas::{Context, Painter, rectangle::Rectangle, Line, Shape},
    };
    #[test]
    fn test_draw() {
        let _rug_st_tests_llm_16_93_rrrruuuugggg_test_draw = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0.0;
        let rug_fuzz_3 = 10.0;
        let rug_fuzz_4 = 0.0;
        let rug_fuzz_5 = 10.0;
        let rug_fuzz_6 = 0.0;
        let rug_fuzz_7 = 0.0;
        let rug_fuzz_8 = 5.0;
        let rug_fuzz_9 = 5.0;
        let mut ctx = Context::new(
            rug_fuzz_0,
            rug_fuzz_1,
            [rug_fuzz_2, rug_fuzz_3],
            [rug_fuzz_4, rug_fuzz_5],
            symbols::Marker::Dot,
        );
        let mut painter = Painter::from(&mut ctx);
        let rect = Rectangle {
            x: rug_fuzz_6,
            y: rug_fuzz_7,
            width: rug_fuzz_8,
            height: rug_fuzz_9,
            color: Color::Red,
        };
        rect.draw(&mut painter);
        let _rug_ed_tests_llm_16_93_rrrruuuugggg_test_draw = 0;
    }
}
