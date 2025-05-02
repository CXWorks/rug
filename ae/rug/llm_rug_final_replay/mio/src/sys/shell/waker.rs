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

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Selector = Selector {};
        let mut p1 = Token(rug_fuzz_0);
        crate::sys::shell::waker::Waker::new(&p0, p1);
             }
}
}
}    }
}
