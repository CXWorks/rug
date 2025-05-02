use std::io;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Duration;
pub type Event = usize;
pub type Events = Vec<Event>;
#[derive(Debug)]
pub struct Selector {}
impl Selector {
    pub fn try_clone(&self) -> io::Result<Selector> {
        os_required!();
    }
    pub fn select(&self, _: &mut Events, _: Option<Duration>) -> io::Result<()> {
        os_required!();
    }
    #[cfg(all(debug_assertions, not(target_os = "wasi")))]
    pub fn register_waker(&self) -> bool {
        os_required!();
    }
}
#[cfg(unix)]
cfg_any_os_ext! {
    use crate :: { Interest, Token }; impl Selector { pub fn register(& self, _ : RawFd,
    _ : Token, _ : Interest) -> io::Result < () > { os_required!(); } pub fn reregister(&
    self, _ : RawFd, _ : Token, _ : Interest) -> io::Result < () > { os_required!(); }
    pub fn deregister(& self, _ : RawFd) -> io::Result < () > { os_required!(); } }
}
#[cfg(target_os = "wasi")]
cfg_any_os_ext! {
    use crate :: { Interest, Token }; impl Selector { pub fn register(& self, _ :
    wasi::Fd, _ : Token, _ : Interest) -> io::Result < () > { os_required!(); } pub fn
    reregister(& self, _ : wasi::Fd, _ : Token, _ : Interest) -> io::Result < () > {
    os_required!(); } pub fn deregister(& self, _ : wasi::Fd) -> io::Result < () > {
    os_required!(); } }
}
cfg_io_source! {
    #[cfg(debug_assertions)] impl Selector { pub fn id(& self) -> usize { os_required!();
    } }
}
#[cfg(unix)]
impl AsRawFd for Selector {
    fn as_raw_fd(&self) -> RawFd {
        os_required!()
    }
}
#[allow(clippy::trivially_copy_pass_by_ref)]
pub mod event {
    use crate::sys::Event;
    use crate::Token;
    use std::fmt;
    pub fn token(_: &Event) -> Token {
        os_required!();
    }
    pub fn is_readable(_: &Event) -> bool {
        os_required!();
    }
    pub fn is_writable(_: &Event) -> bool {
        os_required!();
    }
    pub fn is_error(_: &Event) -> bool {
        os_required!();
    }
    pub fn is_read_closed(_: &Event) -> bool {
        os_required!();
    }
    pub fn is_write_closed(_: &Event) -> bool {
        os_required!();
    }
    pub fn is_priority(_: &Event) -> bool {
        os_required!();
    }
    pub fn is_aio(_: &Event) -> bool {
        os_required!();
    }
    pub fn is_lio(_: &Event) -> bool {
        os_required!();
    }
    pub fn debug_details(_: &mut fmt::Formatter<'_>, _: &Event) -> fmt::Result {
        os_required!();
    }
}
#[cfg(test)]
mod tests_rug_4 {
    use super::*;
    #[test]
    fn test_rug() {
        let p0: &Event = unimplemented!("provide sample Event data here");
        crate::sys::shell::selector::event::is_error(p0);
    }
}
#[cfg(test)]
mod tests_rug_7 {
    use super::*;
    use crate::sys::shell::selector::Event;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_7_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: usize = rug_fuzz_0;
        crate::sys::shell::selector::event::is_priority(&mut p0);
        let _rug_ed_tests_rug_7_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_8_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0: usize = rug_fuzz_0;
        crate::sys::shell::selector::event::is_aio(&p0);
        let _rug_ed_tests_rug_8_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::sys::shell::selector::Selector;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_11_rrrruuuugggg_test_rug = 0;
        let mut p0: Selector = Selector {};
        crate::sys::shell::selector::Selector::try_clone(&p0);
        let _rug_ed_tests_rug_11_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use crate::sys::shell::selector::Selector;
    use std::vec::Vec;
    use std::time::Duration;
    #[test]
    fn test_select() {
        let _rug_st_tests_rug_12_rrrruuuugggg_test_select = 0;
        let rug_fuzz_0 = 5;
        let rug_fuzz_1 = 0;
        let mut p0: Selector = Selector {};
        let mut p1: &mut Vec<usize> = &mut Vec::new();
        let mut p2: Option<Duration> = Some(Duration::new(rug_fuzz_0, rug_fuzz_1));
        p0.select(p1, p2).unwrap();
        let _rug_ed_tests_rug_12_rrrruuuugggg_test_select = 0;
    }
}
#[cfg(test)]
mod tests_rug_13_prepare {
    use crate::sys::shell::selector::Selector;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_13_prepare_rrrruuuugggg_sample = 0;
        let mut p0: Selector = Selector {};
        let _rug_ed_tests_rug_13_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    use crate::sys::shell::selector::Selector;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_13_rrrruuuugggg_test_rug = 0;
        let mut p0: Selector = Selector {};
        //Selector::register_waker(&p0);
        let _rug_ed_tests_rug_13_rrrruuuugggg_test_rug = 0;
    }
}
