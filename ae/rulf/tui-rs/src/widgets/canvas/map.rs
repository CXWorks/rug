use crate::{
    style::Color,
    widgets::canvas::{
        world::{WORLD_HIGH_RESOLUTION, WORLD_LOW_RESOLUTION},
        Painter, Shape,
    },
};
#[derive(Debug, Clone, Copy)]
pub enum MapResolution {
    Low,
    High,
}
impl MapResolution {
    fn data(self) -> &'static [(f64, f64)] {
        match self {
            MapResolution::Low => &WORLD_LOW_RESOLUTION,
            MapResolution::High => &WORLD_HIGH_RESOLUTION,
        }
    }
}
/// Shape to draw a world map with the given resolution and color
#[derive(Debug, Clone)]
pub struct Map {
    pub resolution: MapResolution,
    pub color: Color,
}
impl Default for Map {
    fn default() -> Map {
        Map {
            resolution: MapResolution::Low,
            color: Color::Reset,
        }
    }
}
impl Shape for Map {
    fn draw(&self, painter: &mut Painter) {
        for (x, y) in self.resolution.data() {
            if let Some((x, y)) = painter.get_point(*x, *y) {
                painter.paint(x, y, self.color);
            }
        }
    }
}
#[cfg(test)]
mod tests_llm_16_88_llm_16_87 {
    use super::*;
    use crate::*;
    use crate::{
        symbols::Marker, style::Color, widgets::canvas::Context,
        widgets::canvas::Painter, widgets::canvas::map::Map,
        widgets::canvas::map::MapResolution,
    };
    #[test]
    fn test_draw() {
        let _rug_st_tests_llm_16_88_llm_16_87_rrrruuuugggg_test_draw = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0.0;
        let rug_fuzz_3 = 1.0;
        let rug_fuzz_4 = 0.0;
        let rug_fuzz_5 = 1.0;
        let mut ctx = Context::new(
            rug_fuzz_0,
            rug_fuzz_1,
            [rug_fuzz_2, rug_fuzz_3],
            [rug_fuzz_4, rug_fuzz_5],
            Marker::Dot,
        );
        let mut painter = Painter::from(&mut ctx);
        let map = Map {
            resolution: MapResolution::Low,
            color: Color::Red,
        };
        map.draw(&mut painter);
        let _rug_ed_tests_llm_16_88_llm_16_87_rrrruuuugggg_test_draw = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_86_rrrruuuugggg_test_rug = 0;
        use crate::widgets::canvas::map::MapResolution;
        let mut p0 = MapResolution::Low;
        crate::widgets::canvas::map::MapResolution::data(p0);
        let _rug_ed_tests_rug_86_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::widgets::canvas::map::{Map, MapResolution};
    use crate::style::Color;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_87_rrrruuuugggg_test_rug = 0;
        let _map: Map = Default::default();
        let _rug_ed_tests_rug_87_rrrruuuugggg_test_rug = 0;
    }
}
