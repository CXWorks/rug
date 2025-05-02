use crate::{style::Color, widgets::canvas::{Painter, Shape}};
/// A shape to draw a group of points with the given color
#[derive(Debug, Clone)]
pub struct Points<'a> {
    pub coords: &'a [(f64, f64)],
    pub color: Color,
}
impl<'a> Shape for Points<'a> {
    fn draw(&self, painter: &mut Painter) {
        for (x, y) in self.coords {
            if let Some((x, y)) = painter.get_point(*x, *y) {
                painter.paint(x, y, self.color);
            }
        }
    }
}
impl<'a> Default for Points<'a> {
    fn default() -> Points<'a> {
        Points {
            coords: &[],
            color: Color::Reset,
        }
    }
}
#[cfg(test)]
mod tests_llm_16_92_llm_16_91 {
    use crate::{
        style::Color, symbols, widgets::canvas::{Context, Painter, Shape, Points},
    };
    #[test]
    fn test_draw() {
        let _rug_st_tests_llm_16_92_llm_16_91_rrrruuuugggg_test_draw = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 1.0;
        let rug_fuzz_3 = 2.0;
        let rug_fuzz_4 = 0.0;
        let rug_fuzz_5 = 2.0;
        let rug_fuzz_6 = 1.0;
        let rug_fuzz_7 = 0.0;
        let rug_fuzz_8 = 1.5;
        let rug_fuzz_9 = 1.0;
        let mut ctx = Context::new(
            rug_fuzz_0,
            rug_fuzz_1,
            [rug_fuzz_2, rug_fuzz_3],
            [rug_fuzz_4, rug_fuzz_5],
            symbols::Marker::Braille,
        );
        let mut painter = Painter::from(&mut ctx);
        let coords = &[(rug_fuzz_6, rug_fuzz_7), (rug_fuzz_8, rug_fuzz_9)];
        let points = Points {
            coords,
            color: Color::Red,
        };
        points.draw(&mut painter);
        let _rug_ed_tests_llm_16_92_llm_16_91_rrrruuuugggg_test_draw = 0;
    }
}
#[cfg(test)]
mod tests_rug_88 {
    use super::*;
    use crate::widgets::canvas::points::Points;
    use crate::style::Color;
    use std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_88_rrrruuuugggg_test_rug = 0;
        Points::<'static>::default();
        let _rug_ed_tests_rug_88_rrrruuuugggg_test_rug = 0;
    }
}
