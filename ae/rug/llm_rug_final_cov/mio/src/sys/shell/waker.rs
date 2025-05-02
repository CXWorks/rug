use crate::sys::Selector;
use crate::Token;
use std::io;
#[derive(Debug)]
pub struct Waker {}
impl Waker {
    pub fn new(_: &Selector, _: Token) -> io::Result<Waker> {
        os_required!();
    }
    pub fn wake(&self) -> io::Result<()> {
        os_required!();
    }
}
#[cfg(test)]
mod tests_rug_37 {
    use super::*;
    use crate::sys::shell::selector::Selector;
    use crate::Token;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_37_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1234;
        let mut p0: Selector = Selector {};
        let mut p1 = Token(rug_fuzz_0);
        crate::sys::shell::waker::Waker::new(&p0, p1);
        let _rug_ed_tests_rug_37_rrrruuuugggg_test_rug = 0;
    }
}
