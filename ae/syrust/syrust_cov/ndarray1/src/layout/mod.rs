mod layoutfmt;

// public struct but users don't interact with it
#[doc(hidden)]
/// Memory layout description
#[derive(Copy, Clone)]
pub struct Layout(u32);

impl Layout {
    #[inline(always)]
    pub(crate) fn new(x: u32) -> Self {
        Layout(x)
    }

    #[inline(always)]
    pub(crate) fn is(self, flag: u32) -> bool {
        self.0 & flag != 0
    }
    #[inline(always)]
    pub(crate) fn and(self, flag: Layout) -> Layout {
        Layout(self.0 & flag.0)
    }

    #[inline(always)]
    pub(crate) fn flag(self) -> u32 {
        self.0
    }
}

impl Layout {
    #[doc(hidden)]
    #[inline(always)]
    pub fn one_dimensional() -> Layout {
        Layout(CORDER | FORDER)
    }
    #[doc(hidden)]
    #[inline(always)]
    pub fn c() -> Layout {
        Layout(CORDER)
    }
    #[doc(hidden)]
    #[inline(always)]
    pub fn f() -> Layout {
        Layout(FORDER)
    }
    #[inline(always)]
    #[doc(hidden)]
    pub fn none() -> Layout {
        Layout(0)
    }
}

pub const CORDER: u32 = 0b01;
pub const FORDER: u32 = 0b10;
#[cfg(test)]
mod tests_rug_1070 {
    use super::*;
    use crate::layout;

    #[test]
    fn test_layout_new() {
        let mut p0: u32 = 5;

        layout::Layout::new(p0);
    }
}#[cfg(test)]
mod tests_rug_1071_prepare {
    use crate::layout::Layout;

    #[test]
    fn sample() {
        let mut v148 = Layout::one_dimensional();
    }
}

#[cfg(test)]
mod tests_rug_1071 {
    use super::*;
    use crate::layout::Layout;

    #[test]
    fn test_rug() {
        let mut p0 = Layout::one_dimensional();
        let mut p1: u32 = 5;

        assert_eq!(p0.is(p1), false);
    }
}#[cfg(test)]
mod tests_rug_1072 {
    use super::*;
    use crate::layout::Layout;

    #[test]
    fn test_rug() {
        let mut p0 = Layout::one_dimensional();
        let mut p1 = Layout::one_dimensional();
                
        p0 = p0.and(p1);
    }
}#[cfg(test)]
mod tests_rug_1073 {
    use super::*;
    use crate::layout::Layout;

    #[test]
    fn test_rug() {
        let mut p0 = Layout::one_dimensional();

        assert_eq!(p0.flag(), p0.0);
    }
}#[cfg(test)]
mod tests_rug_1074 {
    use super::*;
    use crate::layout::Layout;
    
    #[test]
    fn test_rug() {
        let _result = Layout::one_dimensional();
    }
}#[cfg(test)]
mod tests_rug_1075 {
    use super::*;
    use crate::layout::Layout;
    use crate::layout::CORDER;

    #[test]
    fn test_rug() {
        Layout::c();
    }
}#[cfg(test)]
mod tests_rug_1076 {
    use super::*;
    use crate::layout::Layout;
    use crate::layout::FORDER;
    
    #[test]
    fn test_rug() {
        Layout::f();
    }
}#[cfg(test)]
mod tests_rug_1077 {
    use super::*;
    use crate::layout;

    #[test]
    fn test_rug() {
        let result = layout::Layout::none();
        assert_eq!(result.0, 0);
    }
}