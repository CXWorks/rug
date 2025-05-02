mod line;
mod map;
mod points;
mod rectangle;
mod world;
pub use self::line::Line;
pub use self::map::{Map, MapResolution};
pub use self::points::Points;
pub use self::rectangle::Rectangle;
use crate::{
    buffer::Buffer, layout::Rect, style::{Color, Style},
    symbols, widgets::{Block, Widget},
};
use std::fmt::Debug;
/// Interface for all shapes that may be drawn on a Canvas widget.
pub trait Shape {
    fn draw(&self, painter: &mut Painter);
}
/// Label to draw some text on the canvas
#[derive(Debug, Clone)]
pub struct Label<'a> {
    pub x: f64,
    pub y: f64,
    pub text: &'a str,
    pub color: Color,
}
#[derive(Debug, Clone)]
struct Layer {
    string: String,
    colors: Vec<Color>,
}
trait Grid: Debug {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn resolution(&self) -> (f64, f64);
    fn paint(&mut self, x: usize, y: usize, color: Color);
    fn save(&self) -> Layer;
    fn reset(&mut self);
}
#[derive(Debug, Clone)]
struct BrailleGrid {
    width: u16,
    height: u16,
    cells: Vec<u16>,
    colors: Vec<Color>,
}
impl BrailleGrid {
    fn new(width: u16, height: u16) -> BrailleGrid {
        let length = usize::from(width * height);
        BrailleGrid {
            width,
            height,
            cells: vec![symbols::braille::BLANK; length],
            colors: vec![Color::Reset; length],
        }
    }
}
impl Grid for BrailleGrid {
    fn width(&self) -> u16 {
        self.width
    }
    fn height(&self) -> u16 {
        self.height
    }
    fn resolution(&self) -> (f64, f64) {
        (f64::from(self.width) * 2.0 - 1.0, f64::from(self.height) * 4.0 - 1.0)
    }
    fn save(&self) -> Layer {
        Layer {
            string: String::from_utf16(&self.cells).unwrap(),
            colors: self.colors.clone(),
        }
    }
    fn reset(&mut self) {
        for c in &mut self.cells {
            *c = symbols::braille::BLANK;
        }
        for c in &mut self.colors {
            *c = Color::Reset;
        }
    }
    fn paint(&mut self, x: usize, y: usize, color: Color) {
        let index = y / 4 * self.width as usize + x / 2;
        if let Some(c) = self.cells.get_mut(index) {
            *c |= symbols::braille::DOTS[y % 4][x % 2];
        }
        if let Some(c) = self.colors.get_mut(index) {
            *c = color;
        }
    }
}
#[derive(Debug, Clone)]
struct CharGrid {
    width: u16,
    height: u16,
    cells: Vec<char>,
    colors: Vec<Color>,
    cell_char: char,
}
impl CharGrid {
    fn new(width: u16, height: u16, cell_char: char) -> CharGrid {
        let length = usize::from(width * height);
        CharGrid {
            width,
            height,
            cells: vec![' '; length],
            colors: vec![Color::Reset; length],
            cell_char,
        }
    }
}
impl Grid for CharGrid {
    fn width(&self) -> u16 {
        self.width
    }
    fn height(&self) -> u16 {
        self.height
    }
    fn resolution(&self) -> (f64, f64) {
        (f64::from(self.width) - 1.0, f64::from(self.height) - 1.0)
    }
    fn save(&self) -> Layer {
        Layer {
            string: self.cells.iter().collect(),
            colors: self.colors.clone(),
        }
    }
    fn reset(&mut self) {
        for c in &mut self.cells {
            *c = ' ';
        }
        for c in &mut self.colors {
            *c = Color::Reset;
        }
    }
    fn paint(&mut self, x: usize, y: usize, color: Color) {
        let index = y * self.width as usize + x;
        if let Some(c) = self.cells.get_mut(index) {
            *c = self.cell_char;
        }
        if let Some(c) = self.colors.get_mut(index) {
            *c = color;
        }
    }
}
#[derive(Debug)]
pub struct Painter<'a, 'b> {
    context: &'a mut Context<'b>,
    resolution: (f64, f64),
}
impl<'a, 'b> Painter<'a, 'b> {
    /// Convert the (x, y) coordinates to location of a point on the grid
    ///
    /// # Examples:
    /// ```
    /// use tui::{symbols, widgets::canvas::{Painter, Context}};
    ///
    /// let mut ctx = Context::new(2, 2, [1.0, 2.0], [0.0, 2.0], symbols::Marker::Braille);
    /// let mut painter = Painter::from(&mut ctx);
    /// let point = painter.get_point(1.0, 0.0);
    /// assert_eq!(point, Some((0, 7)));
    /// let point = painter.get_point(1.5, 1.0);
    /// assert_eq!(point, Some((1, 3)));
    /// let point = painter.get_point(0.0, 0.0);
    /// assert_eq!(point, None);
    /// let point = painter.get_point(2.0, 2.0);
    /// assert_eq!(point, Some((3, 0)));
    /// let point = painter.get_point(1.0, 2.0);
    /// assert_eq!(point, Some((0, 0)));
    /// ```
    pub fn get_point(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let left = self.context.x_bounds[0];
        let right = self.context.x_bounds[1];
        let top = self.context.y_bounds[1];
        let bottom = self.context.y_bounds[0];
        if x < left || x > right || y < bottom || y > top {
            return None;
        }
        let width = (self.context.x_bounds[1] - self.context.x_bounds[0]).abs();
        let height = (self.context.y_bounds[1] - self.context.y_bounds[0]).abs();
        if width == 0.0 || height == 0.0 {
            return None;
        }
        let x = ((x - left) * self.resolution.0 / width) as usize;
        let y = ((top - y) * self.resolution.1 / height) as usize;
        Some((x, y))
    }
    /// Paint a point of the grid
    ///
    /// # Examples:
    /// ```
    /// use tui::{style::Color, symbols, widgets::canvas::{Painter, Context}};
    ///
    /// let mut ctx = Context::new(1, 1, [0.0, 2.0], [0.0, 2.0], symbols::Marker::Braille);
    /// let mut painter = Painter::from(&mut ctx);
    /// let cell = painter.paint(1, 3, Color::Red);
    /// ```
    pub fn paint(&mut self, x: usize, y: usize, color: Color) {
        self.context.grid.paint(x, y, color);
    }
}
impl<'a, 'b> From<&'a mut Context<'b>> for Painter<'a, 'b> {
    fn from(context: &'a mut Context<'b>) -> Painter<'a, 'b> {
        let resolution = context.grid.resolution();
        Painter { context, resolution }
    }
}
/// Holds the state of the Canvas when painting to it.
#[derive(Debug)]
pub struct Context<'a> {
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    grid: Box<dyn Grid>,
    dirty: bool,
    layers: Vec<Layer>,
    labels: Vec<Label<'a>>,
}
impl<'a> Context<'a> {
    pub fn new(
        width: u16,
        height: u16,
        x_bounds: [f64; 2],
        y_bounds: [f64; 2],
        marker: symbols::Marker,
    ) -> Context<'a> {
        let grid: Box<dyn Grid> = match marker {
            symbols::Marker::Dot => Box::new(CharGrid::new(width, height, '•')),
            symbols::Marker::Block => Box::new(CharGrid::new(width, height, '▄')),
            symbols::Marker::Braille => Box::new(BrailleGrid::new(width, height)),
        };
        Context {
            x_bounds,
            y_bounds,
            grid,
            dirty: false,
            layers: Vec::new(),
            labels: Vec::new(),
        }
    }
    /// Draw any object that may implement the Shape trait
    pub fn draw<S>(&mut self, shape: &S)
    where
        S: Shape,
    {
        self.dirty = true;
        let mut painter = Painter::from(self);
        shape.draw(&mut painter);
    }
    /// Go one layer above in the canvas.
    pub fn layer(&mut self) {
        self.layers.push(self.grid.save());
        self.grid.reset();
        self.dirty = false;
    }
    /// Print a string on the canvas at the given position
    pub fn print(&mut self, x: f64, y: f64, text: &'a str, color: Color) {
        self.labels.push(Label { x, y, text, color });
    }
    /// Push the last layer if necessary
    fn finish(&mut self) {
        if self.dirty {
            self.layer()
        }
    }
}
/// The Canvas widget may be used to draw more detailed figures using braille patterns (each
/// cell can have a braille character in 8 different positions).
/// # Examples
///
/// ```
/// # use tui::widgets::{Block, Borders};
/// # use tui::layout::Rect;
/// # use tui::widgets::canvas::{Canvas, Shape, Line, Rectangle, Map, MapResolution};
/// # use tui::style::Color;
/// Canvas::default()
///     .block(Block::default().title("Canvas").borders(Borders::ALL))
///     .x_bounds([-180.0, 180.0])
///     .y_bounds([-90.0, 90.0])
///     .paint(|ctx| {
///         ctx.draw(&Map {
///             resolution: MapResolution::High,
///             color: Color::White
///         });
///         ctx.layer();
///         ctx.draw(&Line {
///             x1: 0.0,
///             y1: 10.0,
///             x2: 10.0,
///             y2: 10.0,
///             color: Color::White,
///         });
///         ctx.draw(&Rectangle {
///             x: 10.0,
///             y: 20.0,
///             width: 10.0,
///             height: 10.0,
///             color: Color::Red
///         });
///     });
/// ```
pub struct Canvas<'a, F>
where
    F: Fn(&mut Context),
{
    block: Option<Block<'a>>,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    painter: Option<F>,
    background_color: Color,
    marker: symbols::Marker,
}
impl<'a, F> Default for Canvas<'a, F>
where
    F: Fn(&mut Context),
{
    fn default() -> Canvas<'a, F> {
        Canvas {
            block: None,
            x_bounds: [0.0, 0.0],
            y_bounds: [0.0, 0.0],
            painter: None,
            background_color: Color::Reset,
            marker: symbols::Marker::Braille,
        }
    }
}
impl<'a, F> Canvas<'a, F>
where
    F: Fn(&mut Context),
{
    pub fn block(mut self, block: Block<'a>) -> Canvas<'a, F> {
        self.block = Some(block);
        self
    }
    pub fn x_bounds(mut self, bounds: [f64; 2]) -> Canvas<'a, F> {
        self.x_bounds = bounds;
        self
    }
    pub fn y_bounds(mut self, bounds: [f64; 2]) -> Canvas<'a, F> {
        self.y_bounds = bounds;
        self
    }
    /// Store the closure that will be used to draw to the Canvas
    pub fn paint(mut self, f: F) -> Canvas<'a, F> {
        self.painter = Some(f);
        self
    }
    pub fn background_color(mut self, color: Color) -> Canvas<'a, F> {
        self.background_color = color;
        self
    }
    /// Change the type of points used to draw the shapes. By default the braille patterns are used
    /// as they provide a more fine grained result but you might want to use the simple dot or
    /// block instead if the targeted terminal does not support those symbols.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tui::widgets::canvas::Canvas;
    /// # use tui::symbols;
    /// Canvas::default().marker(symbols::Marker::Braille).paint(|ctx| {});
    ///
    /// Canvas::default().marker(symbols::Marker::Dot).paint(|ctx| {});
    ///
    /// Canvas::default().marker(symbols::Marker::Block).paint(|ctx| {});
    /// ```
    pub fn marker(mut self, marker: symbols::Marker) -> Canvas<'a, F> {
        self.marker = marker;
        self
    }
}
impl<'a, F> Widget for Canvas<'a, F>
where
    F: Fn(&mut Context),
{
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let canvas_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        let width = canvas_area.width as usize;
        let painter = match self.painter {
            Some(ref p) => p,
            None => return,
        };
        let mut ctx = Context::new(
            canvas_area.width,
            canvas_area.height,
            self.x_bounds,
            self.y_bounds,
            self.marker,
        );
        painter(&mut ctx);
        ctx.finish();
        for layer in ctx.layers {
            for (i, (ch, color)) in layer
                .string
                .chars()
                .zip(layer.colors.into_iter())
                .enumerate()
            {
                if ch != ' ' && ch != '\u{2800}' {
                    let (x, y) = (i % width, i / width);
                    buf.get_mut(
                            x as u16 + canvas_area.left(),
                            y as u16 + canvas_area.top(),
                        )
                        .set_char(ch)
                        .set_fg(color)
                        .set_bg(self.background_color);
                }
            }
        }
        let style = Style::default().bg(self.background_color);
        let left = self.x_bounds[0];
        let right = self.x_bounds[1];
        let top = self.y_bounds[1];
        let bottom = self.y_bounds[0];
        let width = (self.x_bounds[1] - self.x_bounds[0]).abs();
        let height = (self.y_bounds[1] - self.y_bounds[0]).abs();
        let resolution = {
            let width = f64::from(canvas_area.width - 1);
            let height = f64::from(canvas_area.height - 1);
            (width, height)
        };
        for label in ctx
            .labels
            .iter()
            .filter(|l| l.x >= left && l.x <= right && l.y <= top && l.y >= bottom)
        {
            let x = ((label.x - left) * resolution.0 / width) as u16
                + canvas_area.left();
            let y = ((top - label.y) * resolution.1 / height) as u16 + canvas_area.top();
            buf.set_stringn(
                x,
                y,
                label.text,
                (canvas_area.right() - x) as usize,
                style.fg(label.color),
            );
        }
    }
}
#[cfg(test)]
mod tests_llm_16_62 {
    use super::*;
    use crate::*;
    use crate::widgets::canvas::Grid;
    #[test]
    fn test_height() {
        let _rug_st_tests_llm_16_62_rrrruuuugggg_test_height = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let braille_grid = BrailleGrid::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(braille_grid.height(), 20);
        let _rug_ed_tests_llm_16_62_rrrruuuugggg_test_height = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_63 {
    use super::*;
    use crate::*;
    #[test]
    fn test_paint() {
        let _rug_st_tests_llm_16_63_rrrruuuugggg_test_paint = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 4;
        let rug_fuzz_5 = 2;
        let mut grid = BrailleGrid::new(rug_fuzz_0, rug_fuzz_1);
        let x = rug_fuzz_2;
        let y = rug_fuzz_3;
        let color = Color::Red;
        grid.paint(x, y, color);
        let index = y / rug_fuzz_4 * grid.width as usize + x / rug_fuzz_5;
        debug_assert_eq!(grid.cells[index], symbols::braille::DOTS[y % 4] [x % 2]);
        debug_assert_eq!(grid.colors[index], color);
        let _rug_ed_tests_llm_16_63_rrrruuuugggg_test_paint = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_66 {
    use super::*;
    use crate::*;
    use crate::widgets::canvas::Grid;
    #[test]
    fn test_resolution() {
        let _rug_st_tests_llm_16_66_rrrruuuugggg_test_resolution = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let canvas = BrailleGrid::new(rug_fuzz_0, rug_fuzz_1);
        let (x, y) = <widgets::canvas::BrailleGrid as widgets::canvas::Grid>::resolution(
            &canvas,
        );
        debug_assert_eq!(x, 19.0);
        debug_assert_eq!(y, 79.0);
        let _rug_ed_tests_llm_16_66_rrrruuuugggg_test_resolution = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_67 {
    use super::*;
    use crate::*;
    use crate::widgets::canvas::{Grid, BrailleGrid, Layer, Color};
    #[test]
    fn test_save() {
        let _rug_st_tests_llm_16_67_rrrruuuugggg_test_save = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let grid = BrailleGrid::new(rug_fuzz_0, rug_fuzz_1);
        let layer = grid.save();
        debug_assert_eq!(layer.string, String::new());
        debug_assert_eq!(layer.colors, vec![Color::Reset; 100]);
        let _rug_ed_tests_llm_16_67_rrrruuuugggg_test_save = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_68 {
    use super::*;
    use crate::*;
    use crate::widgets::canvas::Grid;
    #[test]
    fn test_width() {
        let _rug_st_tests_llm_16_68_rrrruuuugggg_test_width = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let grid = BrailleGrid::new(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(grid.width(), 10);
        let _rug_ed_tests_llm_16_68_rrrruuuugggg_test_width = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_73 {
    use super::*;
    use crate::*;
    #[test]
    fn test_height() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_height = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = 'x';
        let grid = CharGrid::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let result = grid.height();
        debug_assert_eq!(result, 20);
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_height = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_75 {
    use super::*;
    use crate::*;
    #[test]
    fn test_paint() {
        let _rug_st_tests_llm_16_75_rrrruuuugggg_test_paint = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = ' ';
        let rug_fuzz_3 = 5;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = ' ';
        let mut char_grid = CharGrid::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        char_grid.paint(rug_fuzz_3, rug_fuzz_4, Color::Red);
        let expected_cells = vec![
            rug_fuzz_5, ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
            ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
            ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
            ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
            ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
            ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
            ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '
        ];
        let expected_colors = vec![
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Reset,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset, Color::Red,
            Color::Reset, Color::Reset, Color::Reset, Color::Reset
        ];
        debug_assert_eq!(char_grid.cells, expected_cells);
        debug_assert_eq!(char_grid.colors, expected_colors);
        let _rug_ed_tests_llm_16_75_rrrruuuugggg_test_paint = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_77 {
    use crate::widgets::canvas::CharGrid;
    use crate::widgets::canvas::Grid;
    use crate::widgets::canvas::Color;
    #[test]
    fn test_reset() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_reset = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = ' ';
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 2;
        let mut char_grid = CharGrid::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        char_grid.paint(rug_fuzz_3, rug_fuzz_4, Color::Red);
        char_grid.paint(rug_fuzz_5, rug_fuzz_6, Color::Blue);
        char_grid.paint(rug_fuzz_7, rug_fuzz_8, Color::Green);
        char_grid.reset();
        let expected_cells = vec![' '; 100];
        let expected_colors = vec![Color::Reset; 100];
        debug_assert_eq!(char_grid.cells, expected_cells);
        debug_assert_eq!(char_grid.colors, expected_colors);
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_reset = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_78 {
    use crate::widgets::canvas::{Grid, CharGrid, Color};
    #[test]
    fn test_resolution() {
        let _rug_st_tests_llm_16_78_rrrruuuugggg_test_resolution = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = ' ';
        let grid = CharGrid::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let (width, height) = grid.resolution();
        debug_assert_eq!(width, 9.0);
        debug_assert_eq!(height, 9.0);
        let _rug_ed_tests_llm_16_78_rrrruuuugggg_test_resolution = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_79 {
    use super::*;
    use crate::*;
    use widgets::canvas::{CharGrid, Grid, Layer};
    #[test]
    fn test_save() {
        let _rug_st_tests_llm_16_79_rrrruuuugggg_test_save = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = '#';
        let char_grid = CharGrid::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let layer = char_grid.save();
        debug_assert_eq!(
            layer.string,
            String::from("#         #         #         #         #         #         #         #         #         #         ")
        );
        debug_assert_eq!(layer.colors, vec![Color::Reset; 100]);
        let _rug_ed_tests_llm_16_79_rrrruuuugggg_test_save = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_80 {
    use super::*;
    use crate::*;
    use crate::widgets::canvas::{Grid, CharGrid};
    #[test]
    fn test_width() {
        let _rug_st_tests_llm_16_80_rrrruuuugggg_test_width = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = '*';
        let width = rug_fuzz_0;
        let height = rug_fuzz_1;
        let cell_char = rug_fuzz_2;
        let grid = CharGrid::new(width, height, cell_char);
        debug_assert_eq!(< CharGrid as Grid > ::width(& grid), width);
        let _rug_ed_tests_llm_16_80_rrrruuuugggg_test_width = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_278 {
    use super::*;
    use crate::*;
    use crate::symbols::braille::BLANK;
    use crate::widgets::canvas::Grid;
    use crate::style::Color;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_278_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let width: u16 = rug_fuzz_0;
        let height: u16 = rug_fuzz_1;
        let grid = BrailleGrid::new(width, height);
        debug_assert_eq!(grid.width, width);
        debug_assert_eq!(grid.height, height);
        debug_assert_eq!(grid.cells.len() as u16, width * height);
        debug_assert_eq!(grid.cells, vec![BLANK; width as usize * height as usize]);
        debug_assert_eq!(grid.colors.len() as u16, width * height);
        debug_assert_eq!(
            grid.colors, vec![Color::Reset; width as usize * height as usize]
        );
        let _rug_ed_tests_llm_16_278_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_288 {
    use super::*;
    use crate::*;
    use crate::style::Color;
    use crate::widgets::canvas::{Canvas, Context};
    #[test]
    fn test_y_bounds() {
        let _rug_st_tests_llm_16_288_rrrruuuugggg_test_y_bounds = 0;
        let rug_fuzz_0 = 90.0;
        let rug_fuzz_1 = 90.0;
        let mut canvas: Canvas<'_, fn(&mut Context)> = Canvas::default();
        let new_canvas = canvas.y_bounds([-rug_fuzz_0, rug_fuzz_1]);
        debug_assert_eq!(new_canvas.y_bounds, [- 90.0, 90.0]);
        let _rug_ed_tests_llm_16_288_rrrruuuugggg_test_y_bounds = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_289 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_289_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 'X';
        let width = rug_fuzz_0;
        let height = rug_fuzz_1;
        let cell_char = rug_fuzz_2;
        let char_grid = CharGrid::new(width, height, cell_char);
        debug_assert_eq!(char_grid.width, width);
        debug_assert_eq!(char_grid.height, height);
        debug_assert_eq!(char_grid.cells.len(), usize::from(width * height));
        debug_assert_eq!(char_grid.colors.len(), usize::from(width * height));
        debug_assert_eq!(char_grid.cell_char, cell_char);
        debug_assert_eq!(char_grid.cells, vec![' '; usize::from(width * height)]);
        debug_assert_eq!(
            char_grid.colors, vec![Color::Reset; usize::from(width * height)]
        );
        let _rug_ed_tests_llm_16_289_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_290 {
    use super::*;
    use crate::*;
    use crate::style::Color;
    #[test]
    fn test_draw() {
        let _rug_st_tests_llm_16_290_rrrruuuugggg_test_draw = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 0.0;
        let rug_fuzz_3 = 10.0;
        let rug_fuzz_4 = 0.0;
        let rug_fuzz_5 = 10.0;
        let rug_fuzz_6 = 0.0;
        let rug_fuzz_7 = 0.0;
        let rug_fuzz_8 = 5.0;
        let rug_fuzz_9 = 5.0;
        let mut context = Context::new(
            rug_fuzz_0,
            rug_fuzz_1,
            [rug_fuzz_2, rug_fuzz_3],
            [rug_fuzz_4, rug_fuzz_5],
            symbols::Marker::Dot,
        );
        let line = Line {
            x1: rug_fuzz_6,
            y1: rug_fuzz_7,
            x2: rug_fuzz_8,
            y2: rug_fuzz_9,
            color: Color::Red,
        };
        context.draw(&line);
        debug_assert!(context.dirty);
        debug_assert_eq!(context.layers.len(), 1);
        debug_assert_eq!(context.labels.len(), 0);
        let _rug_ed_tests_llm_16_290_rrrruuuugggg_test_draw = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_297 {
    use super::*;
    use crate::*;
    use crate::style::Color;
    #[test]
    fn test_print() {
        let _rug_st_tests_llm_16_297_rrrruuuugggg_test_print = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0.0;
        let rug_fuzz_3 = 0.0;
        let rug_fuzz_4 = 5.0;
        let rug_fuzz_5 = 5.0;
        let rug_fuzz_6 = "Hello, world!";
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 0;
        let mut context = Context::new(
            rug_fuzz_0,
            rug_fuzz_1,
            [rug_fuzz_2; 2],
            [rug_fuzz_3; 2],
            symbols::Marker::Dot,
        );
        let x = rug_fuzz_4;
        let y = rug_fuzz_5;
        let text = rug_fuzz_6;
        let color = Color::Red;
        context.print(x, y, text, color);
        debug_assert_eq!(context.labels.len(), 1);
        debug_assert_eq!(context.labels[rug_fuzz_7].x, x);
        debug_assert_eq!(context.labels[rug_fuzz_8].y, y);
        debug_assert_eq!(context.labels[rug_fuzz_9].text, text);
        debug_assert_eq!(context.labels[rug_fuzz_10].color, color);
        let _rug_ed_tests_llm_16_297_rrrruuuugggg_test_print = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_298 {
    use crate::style::Color;
    use crate::symbols;
    use crate::widgets::canvas::{Context, Painter, Shape};
    #[test]
    fn test_get_point() {
        let _rug_st_tests_llm_16_298_rrrruuuugggg_test_get_point = 0;
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
        let rug_fuzz_10 = 0.0;
        let rug_fuzz_11 = 0.0;
        let rug_fuzz_12 = 2.0;
        let rug_fuzz_13 = 2.0;
        let rug_fuzz_14 = 1.0;
        let rug_fuzz_15 = 2.0;
        let mut ctx = Context::new(
            rug_fuzz_0,
            rug_fuzz_1,
            [rug_fuzz_2, rug_fuzz_3],
            [rug_fuzz_4, rug_fuzz_5],
            symbols::Marker::Braille,
        );
        let mut painter = Painter::from(&mut ctx);
        let point = painter.get_point(rug_fuzz_6, rug_fuzz_7);
        debug_assert_eq!(point, Some((0, 7)));
        let point = painter.get_point(rug_fuzz_8, rug_fuzz_9);
        debug_assert_eq!(point, Some((1, 3)));
        let point = painter.get_point(rug_fuzz_10, rug_fuzz_11);
        debug_assert_eq!(point, None);
        let point = painter.get_point(rug_fuzz_12, rug_fuzz_13);
        debug_assert_eq!(point, Some((3, 0)));
        let point = painter.get_point(rug_fuzz_14, rug_fuzz_15);
        debug_assert_eq!(point, Some((0, 0)));
        let _rug_ed_tests_llm_16_298_rrrruuuugggg_test_get_point = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_299 {
    use super::*;
    use crate::*;
    use crate::style::Color;
    use crate::symbols;
    use crate::widgets::canvas::{Painter, Context};
    #[test]
    fn test_paint() {
        let _rug_st_tests_llm_16_299_rrrruuuugggg_test_paint = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 0.0;
        let rug_fuzz_3 = 2.0;
        let rug_fuzz_4 = 0.0;
        let rug_fuzz_5 = 2.0;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 3;
        let mut ctx = Context::new(
            rug_fuzz_0,
            rug_fuzz_1,
            [rug_fuzz_2, rug_fuzz_3],
            [rug_fuzz_4, rug_fuzz_5],
            symbols::Marker::Braille,
        );
        let mut painter = Painter::from(&mut ctx);
        painter.paint(rug_fuzz_6, rug_fuzz_7, Color::Red);
        let _rug_ed_tests_llm_16_299_rrrruuuugggg_test_paint = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::widgets::canvas::Grid;
    use crate::widgets::canvas::BrailleGrid;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_21_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let mut p0 = BrailleGrid::new(rug_fuzz_0, rug_fuzz_1);
        <BrailleGrid as Grid>::reset(&mut p0);
        let _rug_ed_tests_rug_21_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_23 {
    use super::*;
    use crate::widgets::canvas::Context;
    use crate::widgets::canvas::symbols::Marker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_23_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = 0.0;
        let rug_fuzz_3 = 1.0;
        let rug_fuzz_4 = 2.0;
        let rug_fuzz_5 = 3.0;
        let mut p0: u16 = rug_fuzz_0;
        let mut p1: u16 = rug_fuzz_1;
        let mut p2: [f64; 2] = [rug_fuzz_2, rug_fuzz_3];
        let mut p3: [f64; 2] = [rug_fuzz_4, rug_fuzz_5];
        let mut p4: Marker = Marker::Dot;
        Context::new(p0, p1, p2, p3, p4);
        let _rug_ed_tests_rug_23_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::widgets::Widget;
    use crate::widgets::canvas::Canvas;
    use crate::layout::Rect;
    use crate::buffer::{Buffer, Cell};
    use crate::style::{Style, Color};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let mut p0: Canvas<'static, fn(&mut Context)> = Canvas::default();
        let mut p1: Rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let mut p2: Buffer = Buffer::empty(p1);
        p0.render(p1, &mut p2);
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_rug = 0;
    }
}
