use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::HashMap;
use cassowary::strength::{REQUIRED, WEAK};
use cassowary::WeightedRelation::*;
use cassowary::{Constraint as CassowaryConstraint, Expression, Solver, Variable};
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Constraint {
    Percentage(u16),
    Ratio(u32, u32),
    Length(u16),
    Max(u16),
    Min(u16),
}
impl Constraint {
    pub fn apply(&self, length: u16) -> u16 {
        match *self {
            Constraint::Percentage(p) => length * p / 100,
            Constraint::Ratio(num, den) => {
                let r = num * u32::from(length) / den;
                r as u16
            }
            Constraint::Length(l) => length.min(l),
            Constraint::Max(m) => length.min(m),
            Constraint::Min(m) => length.max(m),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Margin {
    pub vertical: u16,
    pub horizontal: u16,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Layout {
    direction: Direction,
    margin: Margin,
    constraints: Vec<Constraint>,
}
thread_local! {
    static LAYOUT_CACHE : RefCell < HashMap < (Rect, Layout), Vec < Rect >>> =
    RefCell::new(HashMap::new());
}
impl Default for Layout {
    fn default() -> Layout {
        Layout {
            direction: Direction::Vertical,
            margin: Margin {
                horizontal: 0,
                vertical: 0,
            },
            constraints: Vec::new(),
        }
    }
}
impl Layout {
    pub fn constraints<C>(mut self, constraints: C) -> Layout
    where
        C: Into<Vec<Constraint>>,
    {
        self.constraints = constraints.into();
        self
    }
    pub fn margin(mut self, margin: u16) -> Layout {
        self
            .margin = Margin {
            horizontal: margin,
            vertical: margin,
        };
        self
    }
    pub fn horizontal_margin(mut self, horizontal: u16) -> Layout {
        self.margin.horizontal = horizontal;
        self
    }
    pub fn vertical_margin(mut self, vertical: u16) -> Layout {
        self.margin.vertical = vertical;
        self
    }
    pub fn direction(mut self, direction: Direction) -> Layout {
        self.direction = direction;
        self
    }
    /// Wrapper function around the cassowary-rs solver to be able to split a given
    /// area into smaller ones based on the preferred widths or heights and the direction.
    ///
    /// # Examples
    /// ```
    /// # use tui::layout::{Rect, Constraint, Direction, Layout};
    /// let chunks = Layout::default()
    ///     .direction(Direction::Vertical)
    ///     .constraints([Constraint::Length(5), Constraint::Min(0)].as_ref())
    ///     .split(Rect {
    ///         x: 2,
    ///         y: 2,
    ///         width: 10,
    ///         height: 10,
    ///     });
    /// assert_eq!(
    ///     chunks,
    ///     vec![
    ///         Rect {
    ///             x: 2,
    ///             y: 2,
    ///             width: 10,
    ///             height: 5
    ///         },
    ///         Rect {
    ///             x: 2,
    ///             y: 7,
    ///             width: 10,
    ///             height: 5
    ///         }
    ///     ]
    /// );
    ///
    /// let chunks = Layout::default()
    ///     .direction(Direction::Horizontal)
    ///     .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)].as_ref())
    ///     .split(Rect {
    ///         x: 0,
    ///         y: 0,
    ///         width: 9,
    ///         height: 2,
    ///     });
    /// assert_eq!(
    ///     chunks,
    ///     vec![
    ///         Rect {
    ///             x: 0,
    ///             y: 0,
    ///             width: 3,
    ///             height: 2
    ///         },
    ///         Rect {
    ///             x: 3,
    ///             y: 0,
    ///             width: 6,
    ///             height: 2
    ///         }
    ///     ]
    /// );
    /// ```
    pub fn split(&self, area: Rect) -> Vec<Rect> {
        LAYOUT_CACHE
            .with(|c| {
                c.borrow_mut()
                    .entry((area, self.clone()))
                    .or_insert_with(|| split(area, self))
                    .clone()
            })
    }
}
fn split(area: Rect, layout: &Layout) -> Vec<Rect> {
    let mut solver = Solver::new();
    let mut vars: HashMap<Variable, (usize, usize)> = HashMap::new();
    let elements = layout
        .constraints
        .iter()
        .map(|_| Element::new())
        .collect::<Vec<Element>>();
    let mut results = layout
        .constraints
        .iter()
        .map(|_| Rect::default())
        .collect::<Vec<Rect>>();
    let dest_area = area.inner(&layout.margin);
    for (i, e) in elements.iter().enumerate() {
        vars.insert(e.x, (i, 0));
        vars.insert(e.y, (i, 1));
        vars.insert(e.width, (i, 2));
        vars.insert(e.height, (i, 3));
    }
    let mut ccs: Vec<CassowaryConstraint> = Vec::with_capacity(
        elements.len() * 4 + layout.constraints.len() * 6,
    );
    for elt in &elements {
        ccs.push(elt.width | GE(REQUIRED) | 0f64);
        ccs.push(elt.height | GE(REQUIRED) | 0f64);
        ccs.push(elt.left() | GE(REQUIRED) | f64::from(dest_area.left()));
        ccs.push(elt.top() | GE(REQUIRED) | f64::from(dest_area.top()));
        ccs.push(elt.right() | LE(REQUIRED) | f64::from(dest_area.right()));
        ccs.push(elt.bottom() | LE(REQUIRED) | f64::from(dest_area.bottom()));
    }
    if let Some(first) = elements.first() {
        ccs.push(
            match layout.direction {
                Direction::Horizontal => {
                    first.left() | EQ(REQUIRED) | f64::from(dest_area.left())
                }
                Direction::Vertical => {
                    first.top() | EQ(REQUIRED) | f64::from(dest_area.top())
                }
            },
        );
    }
    if let Some(last) = elements.last() {
        ccs.push(
            match layout.direction {
                Direction::Horizontal => {
                    last.right() | EQ(REQUIRED) | f64::from(dest_area.right())
                }
                Direction::Vertical => {
                    last.bottom() | EQ(REQUIRED) | f64::from(dest_area.bottom())
                }
            },
        );
    }
    match layout.direction {
        Direction::Horizontal => {
            for pair in elements.windows(2) {
                ccs.push((pair[0].x + pair[0].width) | EQ(REQUIRED) | pair[1].x);
            }
            for (i, size) in layout.constraints.iter().enumerate() {
                ccs.push(elements[i].y | EQ(REQUIRED) | f64::from(dest_area.y));
                ccs.push(
                    elements[i].height | EQ(REQUIRED) | f64::from(dest_area.height),
                );
                ccs.push(
                    match *size {
                        Constraint::Length(v) => {
                            elements[i].width | EQ(WEAK) | f64::from(v)
                        }
                        Constraint::Percentage(v) => {
                            elements[i].width | EQ(WEAK)
                                | (f64::from(v * dest_area.width) / 100.0)
                        }
                        Constraint::Ratio(n, d) => {
                            elements[i].width | EQ(WEAK)
                                | (f64::from(dest_area.width) * f64::from(n) / f64::from(d))
                        }
                        Constraint::Min(v) => elements[i].width | GE(WEAK) | f64::from(v),
                        Constraint::Max(v) => elements[i].width | LE(WEAK) | f64::from(v),
                    },
                );
            }
        }
        Direction::Vertical => {
            for pair in elements.windows(2) {
                ccs.push((pair[0].y + pair[0].height) | EQ(REQUIRED) | pair[1].y);
            }
            for (i, size) in layout.constraints.iter().enumerate() {
                ccs.push(elements[i].x | EQ(REQUIRED) | f64::from(dest_area.x));
                ccs.push(elements[i].width | EQ(REQUIRED) | f64::from(dest_area.width));
                ccs.push(
                    match *size {
                        Constraint::Length(v) => {
                            elements[i].height | EQ(WEAK) | f64::from(v)
                        }
                        Constraint::Percentage(v) => {
                            elements[i].height | EQ(WEAK)
                                | (f64::from(v * dest_area.height) / 100.0)
                        }
                        Constraint::Ratio(n, d) => {
                            elements[i].height | EQ(WEAK)
                                | (f64::from(dest_area.height) * f64::from(n)
                                    / f64::from(d))
                        }
                        Constraint::Min(v) => {
                            elements[i].height | GE(WEAK) | f64::from(v)
                        }
                        Constraint::Max(v) => {
                            elements[i].height | LE(WEAK) | f64::from(v)
                        }
                    },
                );
            }
        }
    }
    solver.add_constraints(&ccs).unwrap();
    for &(var, value) in solver.fetch_changes() {
        let (index, attr) = vars[&var];
        let value = if value.is_sign_negative() { 0 } else { value as u16 };
        match attr {
            0 => {
                results[index].x = value;
            }
            1 => {
                results[index].y = value;
            }
            2 => {
                results[index].width = value;
            }
            3 => {
                results[index].height = value;
            }
            _ => {}
        }
    }
    if let Some(last) = results.last_mut() {
        match layout.direction {
            Direction::Vertical => {
                last.height = dest_area.bottom() - last.y;
            }
            Direction::Horizontal => {
                last.width = dest_area.right() - last.x;
            }
        }
    }
    results
}
/// A container used by the solver inside split
struct Element {
    x: Variable,
    y: Variable,
    width: Variable,
    height: Variable,
}
impl Element {
    fn new() -> Element {
        Element {
            x: Variable::new(),
            y: Variable::new(),
            width: Variable::new(),
            height: Variable::new(),
        }
    }
    fn left(&self) -> Variable {
        self.x
    }
    fn top(&self) -> Variable {
        self.y
    }
    fn right(&self) -> Expression {
        self.x + self.width
    }
    fn bottom(&self) -> Expression {
        self.y + self.height
    }
}
/// A simple rectangle used in the computation of the layout and to give widgets an hint about the
/// area they are supposed to render to.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
impl Default for Rect {
    fn default() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}
impl Rect {
    /// Creates a new rect, with width and height limited to keep the area under max u16.
    /// If clipped, aspect ratio will be preserved.
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Rect {
        let max_area = u16::max_value();
        let (clipped_width, clipped_height) = if u32::from(width) * u32::from(height)
            > u32::from(max_area)
        {
            let aspect_ratio = f64::from(width) / f64::from(height);
            let max_area_f = f64::from(max_area);
            let height_f = (max_area_f / aspect_ratio).sqrt();
            let width_f = height_f * aspect_ratio;
            (width_f as u16, height_f as u16)
        } else {
            (width, height)
        };
        Rect {
            x,
            y,
            width: clipped_width,
            height: clipped_height,
        }
    }
    pub fn area(self) -> u16 {
        self.width * self.height
    }
    pub fn left(self) -> u16 {
        self.x
    }
    pub fn right(self) -> u16 {
        self.x + self.width
    }
    pub fn top(self) -> u16 {
        self.y
    }
    pub fn bottom(self) -> u16 {
        self.y + self.height
    }
    pub fn inner(self, margin: &Margin) -> Rect {
        if self.width < 2 * margin.horizontal || self.height < 2 * margin.vertical {
            Rect::default()
        } else {
            Rect {
                x: self.x + margin.horizontal,
                y: self.y + margin.vertical,
                width: self.width - 2 * margin.horizontal,
                height: self.height - 2 * margin.vertical,
            }
        }
    }
    pub fn union(self, other: Rect) -> Rect {
        let x1 = min(self.x, other.x);
        let y1 = min(self.y, other.y);
        let x2 = max(self.x + self.width, other.x + other.width);
        let y2 = max(self.y + self.height, other.y + other.height);
        Rect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
    pub fn intersection(self, other: Rect) -> Rect {
        let x1 = max(self.x, other.x);
        let y1 = max(self.y, other.y);
        let x2 = min(self.x + self.width, other.x + other.width);
        let y2 = min(self.y + self.height, other.y + other.height);
        Rect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.x + other.width && self.x + self.width > other.x
            && self.y < other.y + other.height && self.y + self.height > other.y
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_vertical_split_by_height() {
        let target = Rect {
            x: 2,
            y: 2,
            width: 10,
            height: 10,
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [Constraint::Percentage(10), Constraint::Max(5), Constraint::Min(1)]
                    .as_ref(),
            )
            .split(target);
        assert_eq!(target.height, chunks.iter().map(| r | r.height).sum::< u16 > ());
        chunks.windows(2).for_each(|w| assert!(w[0].y <= w[1].y));
    }
    #[test]
    fn test_rect_size_truncation() {
        for width in 256u16..300u16 {
            for height in 256u16..300u16 {
                let rect = Rect::new(0, 0, width, height);
                rect.area();
                assert!(rect.width < width || rect.height < height);
                assert!(
                    (f64::from(rect.width) / f64::from(rect.height) - f64::from(width) /
                    f64::from(height)).abs() < 1.0
                )
            }
        }
        let width = 900;
        let height = 100;
        let rect = Rect::new(0, 0, width, height);
        assert_ne!(rect.width, 900);
        assert_ne!(rect.height, 100);
        assert!(rect.width < width || rect.height < height);
    }
    #[test]
    fn test_rect_size_preservation() {
        for width in 0..256u16 {
            for height in 0..256u16 {
                let rect = Rect::new(0, 0, width, height);
                rect.area();
                assert_eq!(rect.width, width);
                assert_eq!(rect.height, height);
            }
        }
        let rect = Rect::new(0, 0, 300, 100);
        assert_eq!(rect.width, 300);
        assert_eq!(rect.height, 100);
    }
}
#[cfg(test)]
mod tests_llm_16_29 {
    use super::*;
    use crate::*;
    use crate::layout::{Direction, Layout, Margin, Constraint, Rect};
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_29_rrrruuuugggg_test_default = 0;
        let layout = Layout::default();
        debug_assert_eq!(layout.direction, Direction::Vertical);
        debug_assert_eq!(layout.margin.horizontal, 0);
        debug_assert_eq!(layout.margin.vertical, 0);
        debug_assert_eq!(layout.constraints, Vec:: < Constraint > ::new());
        let _rug_ed_tests_llm_16_29_rrrruuuugggg_test_default = 0;
    }
    #[test]
    fn test_split_vertical() {
        let _rug_st_tests_llm_16_29_rrrruuuugggg_test_split_vertical = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 10;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 10;
        let rug_fuzz_8 = 5;
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(rug_fuzz_0), Constraint::Min(0)]);
        let area = Rect {
            x: rug_fuzz_1,
            y: rug_fuzz_2,
            width: rug_fuzz_3,
            height: rug_fuzz_4,
        };
        let chunks = layout.split(area);
        let expected_chunks = vec![
            Rect { x : rug_fuzz_5, y : rug_fuzz_6, width : rug_fuzz_7, height :
            rug_fuzz_8 }, Rect { x : 2, y : 7, width : 10, height : 5 }
        ];
        debug_assert_eq!(chunks, expected_chunks);
        let _rug_ed_tests_llm_16_29_rrrruuuugggg_test_split_vertical = 0;
    }
    #[test]
    fn test_split_horizontal() {
        let _rug_st_tests_llm_16_29_rrrruuuugggg_test_split_horizontal = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 9;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let rug_fuzz_8 = 3;
        let rug_fuzz_9 = 2;
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                vec![Constraint::Ratio(rug_fuzz_0, rug_fuzz_1), Constraint::Ratio(2, 3)],
            );
        let area = Rect {
            x: rug_fuzz_2,
            y: rug_fuzz_3,
            width: rug_fuzz_4,
            height: rug_fuzz_5,
        };
        let chunks = layout.split(area);
        let expected_chunks = vec![
            Rect { x : rug_fuzz_6, y : rug_fuzz_7, width : rug_fuzz_8, height :
            rug_fuzz_9 }, Rect { x : 3, y : 0, width : 6, height : 2 }
        ];
        debug_assert_eq!(chunks, expected_chunks);
        let _rug_ed_tests_llm_16_29_rrrruuuugggg_test_split_horizontal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_30 {
    use crate::layout::Rect;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_30_rrrruuuugggg_test_default = 0;
        let rect = Rect::default();
        debug_assert_eq!(rect.x, 0);
        debug_assert_eq!(rect.y, 0);
        debug_assert_eq!(rect.width, 0);
        debug_assert_eq!(rect.height, 0);
        let _rug_ed_tests_llm_16_30_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_170 {
    use crate::layout::Constraint;
    #[test]
    fn test_apply_percentage() {
        let _rug_st_tests_llm_16_170_rrrruuuugggg_test_apply_percentage = 0;
        let rug_fuzz_0 = 50;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 50;
        let constraint = Constraint::Percentage(rug_fuzz_0);
        let length = rug_fuzz_1;
        let expected_output = rug_fuzz_2;
        debug_assert_eq!(constraint.apply(length), expected_output);
        let _rug_ed_tests_llm_16_170_rrrruuuugggg_test_apply_percentage = 0;
    }
    #[test]
    fn test_apply_ratio() {
        let _rug_st_tests_llm_16_170_rrrruuuugggg_test_apply_ratio = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 100;
        let rug_fuzz_3 = 66;
        let constraint = Constraint::Ratio(rug_fuzz_0, rug_fuzz_1);
        let length = rug_fuzz_2;
        let expected_output = rug_fuzz_3;
        debug_assert_eq!(constraint.apply(length), expected_output);
        let _rug_ed_tests_llm_16_170_rrrruuuugggg_test_apply_ratio = 0;
    }
    #[test]
    fn test_apply_length() {
        let _rug_st_tests_llm_16_170_rrrruuuugggg_test_apply_length = 0;
        let rug_fuzz_0 = 50;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 50;
        let constraint = Constraint::Length(rug_fuzz_0);
        let length = rug_fuzz_1;
        let expected_output = rug_fuzz_2;
        debug_assert_eq!(constraint.apply(length), expected_output);
        let _rug_ed_tests_llm_16_170_rrrruuuugggg_test_apply_length = 0;
    }
    #[test]
    fn test_apply_max() {
        let _rug_st_tests_llm_16_170_rrrruuuugggg_test_apply_max = 0;
        let rug_fuzz_0 = 75;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 75;
        let constraint = Constraint::Max(rug_fuzz_0);
        let length = rug_fuzz_1;
        let expected_output = rug_fuzz_2;
        debug_assert_eq!(constraint.apply(length), expected_output);
        let _rug_ed_tests_llm_16_170_rrrruuuugggg_test_apply_max = 0;
    }
    #[test]
    fn test_apply_min() {
        let _rug_st_tests_llm_16_170_rrrruuuugggg_test_apply_min = 0;
        let rug_fuzz_0 = 30;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 100;
        let constraint = Constraint::Min(rug_fuzz_0);
        let length = rug_fuzz_1;
        let expected_output = rug_fuzz_2;
        debug_assert_eq!(constraint.apply(length), expected_output);
        let _rug_ed_tests_llm_16_170_rrrruuuugggg_test_apply_min = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_173 {
    use super::*;
    use crate::*;
    #[test]
    fn test_left() {
        let _rug_st_tests_llm_16_173_rrrruuuugggg_test_left = 0;
        let element = Element::new();
        let result = element.left();
        debug_assert_eq!(result, element.x);
        let _rug_ed_tests_llm_16_173_rrrruuuugggg_test_left = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_178 {
    use super::*;
    use crate::*;
    use crate::layout::Element;
    use crate::layout::Variable;
    #[test]
    fn test_top() {
        let _rug_st_tests_llm_16_178_rrrruuuugggg_test_top = 0;
        let element = Element {
            x: Variable::new(),
            y: Variable::new(),
            width: Variable::new(),
            height: Variable::new(),
        };
        let result = element.top();
        debug_assert_eq!(result, element.y);
        let _rug_ed_tests_llm_16_178_rrrruuuugggg_test_top = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_179 {
    use super::*;
    use crate::*;
    use crate::layout::Rect;
    use crate::layout::Constraint;
    use crate::layout::Direction;
    use crate::layout::Margin;
    #[test]
    fn test_constraints() {
        let _rug_st_tests_llm_16_179_rrrruuuugggg_test_constraints = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 5;
        let layout = Layout::default();
        let constraints = vec![Constraint::Length(rug_fuzz_0), Constraint::Min(0)];
        let new_layout = layout.constraints(constraints);
        let expected_constraints = vec![
            Constraint::Length(rug_fuzz_1), Constraint::Min(0)
        ];
        debug_assert_eq!(new_layout.constraints, expected_constraints);
        let _rug_ed_tests_llm_16_179_rrrruuuugggg_test_constraints = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_180 {
    use super::*;
    use crate::*;
    use crate::layout::Rect;
    use crate::layout::Constraint;
    use crate::layout::Direction;
    use crate::layout::Layout;
    #[test]
    fn test_direction() {
        let _rug_st_tests_llm_16_180_rrrruuuugggg_test_direction = 0;
        let layout = Layout::default().direction(Direction::Vertical);
        debug_assert_eq!(layout.direction, Direction::Vertical);
        let layout = Layout::default().direction(Direction::Horizontal);
        debug_assert_eq!(layout.direction, Direction::Horizontal);
        let _rug_ed_tests_llm_16_180_rrrruuuugggg_test_direction = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_181 {
    use super::*;
    use crate::*;
    #[test]
    fn test_horizontal_margin() {
        let _rug_st_tests_llm_16_181_rrrruuuugggg_test_horizontal_margin = 0;
        let rug_fuzz_0 = 10;
        let layout = Layout::default().horizontal_margin(rug_fuzz_0);
        debug_assert_eq!(layout.margin.horizontal, 10);
        let _rug_ed_tests_llm_16_181_rrrruuuugggg_test_horizontal_margin = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_182 {
    use super::*;
    use crate::*;
    use crate::layout::{Direction, Layout};
    #[test]
    fn test_margin() {
        let _rug_st_tests_llm_16_182_rrrruuuugggg_test_margin = 0;
        let rug_fuzz_0 = 10;
        let layout = Layout::default().margin(rug_fuzz_0);
        debug_assert_eq!(layout.margin, Margin { horizontal : 10, vertical : 10 });
        let _rug_ed_tests_llm_16_182_rrrruuuugggg_test_margin = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_183 {
    use super::*;
    use crate::*;
    use crate::layout::{Rect, Constraint, Direction, Layout};
    #[test]
    fn test_split_vertical() {
        let _rug_st_tests_llm_16_183_rrrruuuugggg_test_split_vertical = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 10;
        let rug_fuzz_5 = 10;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 10;
        let rug_fuzz_9 = 5;
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [Constraint::Length(rug_fuzz_0), Constraint::Min(rug_fuzz_1)].as_ref(),
            );
        let area = Rect {
            x: rug_fuzz_2,
            y: rug_fuzz_3,
            width: rug_fuzz_4,
            height: rug_fuzz_5,
        };
        let chunks = layout.split(area);
        let expected = vec![
            Rect { x : rug_fuzz_6, y : rug_fuzz_7, width : rug_fuzz_8, height :
            rug_fuzz_9, }, Rect { x : 2, y : 7, width : 10, height : 5, }
        ];
        debug_assert_eq!(chunks, expected);
        let _rug_ed_tests_llm_16_183_rrrruuuugggg_test_split_vertical = 0;
    }
    #[test]
    fn test_split_horizontal() {
        let _rug_st_tests_llm_16_183_rrrruuuugggg_test_split_horizontal = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 3;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 9;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 3;
        let rug_fuzz_11 = 2;
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Ratio(rug_fuzz_0, rug_fuzz_1),
                    Constraint::Ratio(rug_fuzz_2, rug_fuzz_3),
                ]
                    .as_ref(),
            );
        let area = Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        };
        let chunks = layout.split(area);
        let expected = vec![
            Rect { x : rug_fuzz_8, y : rug_fuzz_9, width : rug_fuzz_10, height :
            rug_fuzz_11, }, Rect { x : 3, y : 0, width : 6, height : 2, }
        ];
        debug_assert_eq!(chunks, expected);
        let _rug_ed_tests_llm_16_183_rrrruuuugggg_test_split_horizontal = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_184 {
    use super::*;
    use crate::*;
    use crate::layout::*;
    #[test]
    fn test_vertical_margin() {
        let _rug_st_tests_llm_16_184_rrrruuuugggg_test_vertical_margin = 0;
        let rug_fuzz_0 = 2;
        let layout = Layout::default().vertical_margin(rug_fuzz_0);
        debug_assert_eq!(layout.margin.vertical, 2);
        let _rug_ed_tests_llm_16_184_rrrruuuugggg_test_vertical_margin = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_185 {
    use super::*;
    use crate::*;
    #[test]
    fn test_area() {
        let _rug_st_tests_llm_16_185_rrrruuuugggg_test_area = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        debug_assert_eq!(rect.area(), 50);
        let _rug_ed_tests_llm_16_185_rrrruuuugggg_test_area = 0;
    }
    #[test]
    fn test_area_with_zero_dimensions() {
        let _rug_st_tests_llm_16_185_rrrruuuugggg_test_area_with_zero_dimensions = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        debug_assert_eq!(rect.area(), 0);
        let _rug_ed_tests_llm_16_185_rrrruuuugggg_test_area_with_zero_dimensions = 0;
    }
    #[test]
    fn test_area_with_large_dimensions() {
        let _rug_st_tests_llm_16_185_rrrruuuugggg_test_area_with_large_dimensions = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: u16::max_value(),
            height: u16::max_value(),
        };
        debug_assert_eq!(rect.area(), 1);
        let _rug_ed_tests_llm_16_185_rrrruuuugggg_test_area_with_large_dimensions = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_186 {
    use super::*;
    use crate::*;
    #[test]
    fn test_bottom() {
        let _rug_st_tests_llm_16_186_rrrruuuugggg_test_bottom = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 15;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 10;
        let rug_fuzz_6 = 8;
        let rug_fuzz_7 = 4;
        let rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        debug_assert_eq!(rect.bottom(), 15);
        let rect = Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        };
        debug_assert_eq!(rect.bottom(), 14);
        let _rug_ed_tests_llm_16_186_rrrruuuugggg_test_bottom = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_188 {
    use crate::layout::{Rect, Margin};
    #[test]
    fn test_inner() {
        let _rug_st_tests_llm_16_188_rrrruuuugggg_test_inner = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 1;
        let rug_fuzz_6 = 1;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 8;
        let rug_fuzz_9 = 8;
        let rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let margin = Margin {
            vertical: rug_fuzz_4,
            horizontal: rug_fuzz_5,
        };
        let expected = Rect {
            x: rug_fuzz_6,
            y: rug_fuzz_7,
            width: rug_fuzz_8,
            height: rug_fuzz_9,
        };
        debug_assert_eq!(rect.inner(& margin), expected);
        let _rug_ed_tests_llm_16_188_rrrruuuugggg_test_inner = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_189 {
    use super::*;
    use crate::*;
    #[test]
    fn test_intersection() {
        let _rug_st_tests_llm_16_189_rrrruuuugggg_test_intersection = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 10;
        let rug_fuzz_8 = 5;
        let rug_fuzz_9 = 5;
        let rug_fuzz_10 = 5;
        let rug_fuzz_11 = 5;
        let rect1 = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let rect2 = Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        };
        let expected = Rect {
            x: rug_fuzz_8,
            y: rug_fuzz_9,
            width: rug_fuzz_10,
            height: rug_fuzz_11,
        };
        debug_assert_eq!(rect1.intersection(rect2), expected);
        let _rug_ed_tests_llm_16_189_rrrruuuugggg_test_intersection = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_190 {
    use super::*;
    use crate::*;
    #[test]
    fn test_intersects() {
        let _rug_st_tests_llm_16_190_rrrruuuugggg_test_intersects = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 10;
        let rug_fuzz_8 = 20;
        let rug_fuzz_9 = 20;
        let rug_fuzz_10 = 10;
        let rug_fuzz_11 = 10;
        let rect1 = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let rect2 = Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        };
        let rect3 = Rect {
            x: rug_fuzz_8,
            y: rug_fuzz_9,
            width: rug_fuzz_10,
            height: rug_fuzz_11,
        };
        debug_assert!(rect1.intersects(rect2));
        debug_assert!(rect2.intersects(rect1));
        debug_assert!(! rect1.intersects(rect3));
        debug_assert!(! rect3.intersects(rect1));
        let _rug_ed_tests_llm_16_190_rrrruuuugggg_test_intersects = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_191 {
    use super::*;
    use crate::*;
    #[test]
    fn test_left() {
        let _rug_st_tests_llm_16_191_rrrruuuugggg_test_left = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = 30;
        let rug_fuzz_3 = 40;
        let rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        debug_assert_eq!(rect.left(), 10);
        let _rug_ed_tests_llm_16_191_rrrruuuugggg_test_left = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_192 {
    use super::*;
    use crate::*;
    use crate::layout::Margin;
    #[test]
    fn test_new_rect() {
        let _rug_st_tests_llm_16_192_rrrruuuugggg_test_new_rect = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 20;
        let rug_fuzz_7 = 15;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 500;
        let rug_fuzz_11 = 500;
        let rect1 = Rect::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3);
        debug_assert_eq!(rect1.x, 0);
        debug_assert_eq!(rect1.y, 0);
        debug_assert_eq!(rect1.width, 10);
        debug_assert_eq!(rect1.height, 10);
        let rect2 = Rect::new(rug_fuzz_4, rug_fuzz_5, rug_fuzz_6, rug_fuzz_7);
        debug_assert_eq!(rect2.x, 0);
        debug_assert_eq!(rect2.y, 0);
        debug_assert_eq!(rect2.width, 20);
        debug_assert_eq!(rect2.height, 15);
        let rect3 = Rect::new(rug_fuzz_8, rug_fuzz_9, rug_fuzz_10, rug_fuzz_11);
        debug_assert_eq!(rect3.x, 0);
        debug_assert_eq!(rect3.y, 0);
        debug_assert_eq!(rect3.width, 655);
        debug_assert_eq!(rect3.height, 655);
        let _rug_ed_tests_llm_16_192_rrrruuuugggg_test_new_rect = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_193 {
    use super::*;
    use crate::*;
    #[test]
    fn test_right() {
        let _rug_st_tests_llm_16_193_rrrruuuugggg_test_right = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 5;
        let rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        debug_assert_eq!(rect.right(), 10);
        let _rug_ed_tests_llm_16_193_rrrruuuugggg_test_right = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_194 {
    use super::*;
    use crate::*;
    #[test]
    fn test_top() {
        let _rug_st_tests_llm_16_194_rrrruuuugggg_test_top = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 20;
        let rug_fuzz_2 = 30;
        let rug_fuzz_3 = 40;
        let rect = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        debug_assert_eq!(rect.top(), 20);
        let _rug_ed_tests_llm_16_194_rrrruuuugggg_test_top = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_195 {
    use super::*;
    use crate::*;
    #[test]
    fn test_union() {
        let _rug_st_tests_llm_16_195_rrrruuuugggg_test_union = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 5;
        let rug_fuzz_5 = 5;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 10;
        let rug_fuzz_8 = 0;
        let rug_fuzz_9 = 0;
        let rug_fuzz_10 = 15;
        let rug_fuzz_11 = 15;
        let rect1 = Rect {
            x: rug_fuzz_0,
            y: rug_fuzz_1,
            width: rug_fuzz_2,
            height: rug_fuzz_3,
        };
        let rect2 = Rect {
            x: rug_fuzz_4,
            y: rug_fuzz_5,
            width: rug_fuzz_6,
            height: rug_fuzz_7,
        };
        let expected = Rect {
            x: rug_fuzz_8,
            y: rug_fuzz_9,
            width: rug_fuzz_10,
            height: rug_fuzz_11,
        };
        let result = rect1.union(rect2);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_195_rrrruuuugggg_test_union = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_196 {
    use super::*;
    use crate::*;
    use std::collections::HashMap;
    #[test]
    fn test_split() {
        let _rug_st_tests_llm_16_196_rrrruuuugggg_test_split = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 5;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 20;
        let rug_fuzz_6 = 10;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 1;
        let rug_fuzz_9 = 5;
        let rug_fuzz_10 = 8;
        let layout = Layout {
            direction: Direction::Horizontal,
            margin: Margin {
                horizontal: rug_fuzz_0,
                vertical: rug_fuzz_1,
            },
            constraints: vec![
                Constraint::Length(rug_fuzz_2), Constraint::Percentage(50),
                Constraint::Ratio(1, 2)
            ],
        };
        let area = Rect {
            x: rug_fuzz_3,
            y: rug_fuzz_4,
            width: rug_fuzz_5,
            height: rug_fuzz_6,
        };
        let result = split(area, &layout);
        let expected = vec![
            Rect { x : rug_fuzz_7, y : rug_fuzz_8, width : rug_fuzz_9, height :
            rug_fuzz_10, }, Rect { x : 8, y : 1, width : 6, height : 8, }, Rect { x : 15,
            y : 1, width : 4, height : 8, }
        ];
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_196_rrrruuuugggg_test_split = 0;
    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::layout::Element;
    use crate::layout::Variable;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_6_rrrruuuugggg_test_rug = 0;
        Element::new();
        let _rug_ed_tests_rug_6_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    use crate::layout::Element;
    #[test]
    fn test_right() {
        let _rug_st_tests_rug_7_rrrruuuugggg_test_right = 0;
        let mut p0 = Element::new();
        p0.right();
        let _rug_ed_tests_rug_7_rrrruuuugggg_test_right = 0;
    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use crate::layout::Element;
    #[test]
    fn test_bottom() {
        let _rug_st_tests_rug_8_rrrruuuugggg_test_bottom = 0;
        let mut p0 = Element::new();
        let result = p0.bottom();
        let _rug_ed_tests_rug_8_rrrruuuugggg_test_bottom = 0;
    }
}
