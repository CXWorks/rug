//! Channel that never delivers messages.
//!
//! Messages cannot be sent into this kind of channel.
use std::marker::PhantomData;
use std::time::Instant;
use crate::context::Context;
use crate::err::{RecvTimeoutError, TryRecvError};
use crate::select::{Operation, SelectHandle, Token};
use crate::utils;
/// This flavor doesn't need a token.
pub type NeverToken = ();
/// Channel that never delivers messages.
pub struct Channel<T> {
    _marker: PhantomData<T>,
}
impl<T> Channel<T> {
    /// Creates a channel that never delivers messages.
    #[inline]
    pub fn new() -> Self {
        Channel { _marker: PhantomData }
    }
    /// Attempts to receive a message without blocking.
    #[inline]
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        Err(TryRecvError::Empty)
    }
    /// Receives a message from the channel.
    #[inline]
    pub fn recv(&self, deadline: Option<Instant>) -> Result<T, RecvTimeoutError> {
        utils::sleep_until(deadline);
        Err(RecvTimeoutError::Timeout)
    }
    /// Reads a message from the channel.
    #[inline]
    pub unsafe fn read(&self, _token: &mut Token) -> Result<T, ()> {
        Err(())
    }
    /// Returns `true` if the channel is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        true
    }
    /// Returns `true` if the channel is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        true
    }
    /// Returns the number of messages in the channel.
    #[inline]
    pub fn len(&self) -> usize {
        0
    }
    /// Returns the capacity of the channel.
    #[inline]
    pub fn capacity(&self) -> Option<usize> {
        Some(0)
    }
}
impl<T> SelectHandle for Channel<T> {
    #[inline]
    fn try_select(&self, _token: &mut Token) -> bool {
        false
    }
    #[inline]
    fn deadline(&self) -> Option<Instant> {
        None
    }
    #[inline]
    fn register(&self, _oper: Operation, _cx: &Context) -> bool {
        self.is_ready()
    }
    #[inline]
    fn unregister(&self, _oper: Operation) {}
    #[inline]
    fn accept(&self, token: &mut Token, _cx: &Context) -> bool {
        self.try_select(token)
    }
    #[inline]
    fn is_ready(&self) -> bool {
        false
    }
    #[inline]
    fn watch(&self, _oper: Operation, _cx: &Context) -> bool {
        self.is_ready()
    }
    #[inline]
    fn unwatch(&self, _oper: Operation) {}
}
#[cfg(test)]
mod tests_rug_237 {
    use super::*;
    use crate::flavors::never::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_237_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        debug_assert_eq!(p0.try_recv(), Err(TryRecvError::Empty));
        let _rug_ed_tests_rug_237_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    use crate::flavors::never::Channel;
    use crate::RecvTimeoutError;
    use std::time::Instant;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_238_rrrruuuugggg_test_rug = 0;
        let p0: Channel<()> = unimplemented!();
        let p1: Option<Instant> = unimplemented!();
        p0.recv(p1);
        let _rug_ed_tests_rug_238_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    use crate::flavors::never::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_240_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let p0: Channel<i32> = Channel::<i32>::new();
        debug_assert_eq!(rug_fuzz_0, < Channel < i32 > > ::is_empty(& p0));
        let _rug_ed_tests_rug_240_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::flavors::never::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_241_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<i32> = Channel::new();
        debug_assert_eq!(Channel:: < i32 > ::is_full(& p0), true);
        let _rug_ed_tests_rug_241_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_242 {
    use super::*;
    use crate::flavors::never::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_242_rrrruuuugggg_test_rug = 0;
        let p0: Channel<i32> = Channel::new();
        p0.len();
        let _rug_ed_tests_rug_242_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_243 {
    use super::*;
    use crate::flavors::never::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_243_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<u32> = Channel::new();
        Channel::<u32>::capacity(&p0);
        let _rug_ed_tests_rug_243_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::never::Channel;
    use crate::select::Token;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_244_rrrruuuugggg_test_rug = 0;
        let mut p0: Channel<u32> = Channel::<u32>::new();
        let mut p1: Token = Token::default();
        p0.try_select(&mut p1);
        let _rug_ed_tests_rug_244_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_245 {
    use super::*;
    use crate::{flavors::never::Channel, internal::SelectHandle};
    #[test]
    fn test_deadline() {
        let _rug_st_tests_rug_245_rrrruuuugggg_test_deadline = 0;
        let mut p0: Channel<i32> = Channel::<i32>::new();
        p0.deadline();
        let _rug_ed_tests_rug_245_rrrruuuugggg_test_deadline = 0;
    }
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::never::Channel;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_249_rrrruuuugggg_test_rug = 0;
        let mut p0 = Channel::<usize>::new();
        p0.is_ready();
        let _rug_ed_tests_rug_249_rrrruuuugggg_test_rug = 0;
    }
}
