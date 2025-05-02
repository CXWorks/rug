use crate::{style::Color, widgets::canvas::{Painter, Shape}};
/// Shape to draw a line from (x1, y1) to (x2, y2) with the given color
#[derive(Debug, Clone)]
pub struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub color: Color,
}
impl Shape for Line {
    fn draw(&self, painter: &mut Painter) {
        let (x1, y1) = match painter.get_point(self.x1, self.y1) {
            Some(c) => c,
            None => return,
        };
        let (x2, y2) = match painter.get_point(self.x2, self.y2) {
            Some(c) => c,
            None => return,
        };
        let (dx, x_range) = if x2 >= x1 {
            (x2 - x1, x1..=x2)
        } else {
            (x1 - x2, x2..=x1)
        };
        let (dy, y_range) = if y2 >= y1 {
            (y2 - y1, y1..=y2)
        } else {
            (y1 - y2, y2..=y1)
        };
        if dx == 0 {
            for y in y_range {
                painter.paint(x1, y, self.color);
            }
        } else if dy == 0 {
            for x in x_range {
                painter.paint(x, y1, self.color);
            }
        } else if dy < dx {
            if x1 > x2 {
                draw_line_low(painter, x2, y2, x1, y1, self.color);
            } else {
                draw_line_low(painter, x1, y1, x2, y2, self.color);
            }
        } else if y1 > y2 {
            draw_line_high(painter, x2, y2, x1, y1, self.color);
        } else {
            draw_line_high(painter, x1, y1, x2, y2, self.color);
        }
    }
}
fn draw_line_low(
    painter: &mut Painter,
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    color: Color,
) {
    let dx = (x2 - x1) as isize;
    let dy = (y2 as isize - y1 as isize).abs();
    let mut d = 2 * dy - dx;
    let mut y = y1;
    for x in x1..=x2 {
        painter.paint(x, y, color);
        if d > 0 {
            y = if y1 > y2 { y.saturating_sub(1) } else { y.saturating_add(1) };
            d -= 2 * dx;
        }
        d += 2 * dy;
    }
}
fn draw_line_high(
    painter: &mut Painter,
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    color: Color,
) {
    let dx = (x2 as isize - x1 as isize).abs();
    let dy = (y2 - y1) as isize;
    let mut d = 2 * dx - dy;
    let mut x = x1;
    for y in y1..=y2 {
        painter.paint(x, y, color);
        if d > 0 {
            x = if x1 > x2 { x.saturating_sub(1) } else { x.saturating_add(1) };
            d -= 2 * dy;
        }
        d += 2 * dx;
    }
}
#[cfg(test)]
mod tests_llm_16_302 {
    use super::*;
    use crate::*;
    use crate::style::Color;
    use crate::widgets::canvas::{Painter, Context};
    #[test]
    fn test_draw_line_low() {
        let _rug_st_tests_llm_16_302_rrrruuuugggg_test_draw_line_low = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0.0;
        let rug_fuzz_3 = 10.0;
        let rug_fuzz_4 = 0.0;
        let rug_fuzz_5 = 10.0;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 9;
        let rug_fuzz_9 = 9;
        let mut context = Context::new(
            rug_fuzz_0,
            rug_fuzz_1,
            [rug_fuzz_2, rug_fuzz_3],
            [rug_fuzz_4, rug_fuzz_5],
            symbols::Marker::Braille,
        );
        let mut painter = Painter::from(&mut context);
        let color = Color::Red;
        let x1 = rug_fuzz_6;
        let y1 = rug_fuzz_7;
        let x2 = rug_fuzz_8;
        let y2 = rug_fuzz_9;
        draw_line_low(&mut painter, x1, y1, x2, y2, color);
        let _rug_ed_tests_llm_16_302_rrrruuuugggg_test_draw_line_low = 0;
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::widgets::canvas::Painter;
    use crate::style::Color;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_9_rrrruuuugggg_test_rug = 0;
        let mut p0: Painter<'_, '_> = unimplemented!();
        let p1: usize = unimplemented!();
        let p2: usize = unimplemented!();
        let p3: usize = unimplemented!();
        let p4: usize = unimplemented!();
        let mut p5: Color = Color::Reset;
        crate::widgets::canvas::line::draw_line_high(&mut p0, p1, p2, p3, p4, p5);
        let _rug_ed_tests_rug_9_rrrruuuugggg_test_rug = 0;
    }
}
