//! Channel that delivers a message after a certain amount of time.
//!
//! Messages cannot be sent into this kind of channel; they are materialized on demand.
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use crate::context::Context;
use crate::err::{RecvTimeoutError, TryRecvError};
use crate::select::{Operation, SelectHandle, Token};
use crate::utils;
/// Result of a receive operation.
pub type AfterToken = Option<Instant>;
/// Channel that delivers a message after a certain amount of time.
pub struct Channel {
    /// The instant at which the message will be delivered.
    delivery_time: Instant,
    /// `true` if the message has been received.
    received: AtomicBool,
}
impl Channel {
    /// Creates a channel that delivers a message after a certain duration of time.
    #[inline]
    pub fn new(dur: Duration) -> Self {
        Channel {
            delivery_time: Instant::now() + dur,
            received: AtomicBool::new(false),
        }
    }
    /// Attempts to receive a message without blocking.
    #[inline]
    pub fn try_recv(&self) -> Result<Instant, TryRecvError> {
        if self.received.load(Ordering::Relaxed) {
            return Err(TryRecvError::Empty);
        }
        if Instant::now() < self.delivery_time {
            return Err(TryRecvError::Empty);
        }
        if !self.received.swap(true, Ordering::SeqCst) {
            Ok(self.delivery_time)
        } else {
            Err(TryRecvError::Empty)
        }
    }
    /// Receives a message from the channel.
    #[inline]
    pub fn recv(&self, deadline: Option<Instant>) -> Result<Instant, RecvTimeoutError> {
        if self.received.load(Ordering::Relaxed) {
            utils::sleep_until(deadline);
            return Err(RecvTimeoutError::Timeout);
        }
        loop {
            let now = Instant::now();
            if now >= self.delivery_time {
                break;
            }
            if let Some(d) = deadline {
                if now >= d {
                    return Err(RecvTimeoutError::Timeout);
                }
                thread::sleep(self.delivery_time.min(d) - now);
            } else {
                thread::sleep(self.delivery_time - now);
            }
        }
        if !self.received.swap(true, Ordering::SeqCst) {
            Ok(self.delivery_time)
        } else {
            utils::sleep_until(None);
            unreachable!()
        }
    }
    /// Reads a message from the channel.
    #[inline]
    pub unsafe fn read(&self, token: &mut Token) -> Result<Instant, ()> {
        token.after.ok_or(())
    }
    /// Returns `true` if the channel is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        if self.received.load(Ordering::Relaxed) {
            return true;
        }
        if Instant::now() < self.delivery_time {
            return true;
        }
        self.received.load(Ordering::SeqCst)
    }
    /// Returns `true` if the channel is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        !self.is_empty()
    }
    /// Returns the number of messages in the channel.
    #[inline]
    pub fn len(&self) -> usize {
        if self.is_empty() { 0 } else { 1 }
    }
    /// Returns the capacity of the channel.
    #[inline]
    pub fn capacity(&self) -> Option<usize> {
        Some(1)
    }
}
impl SelectHandle for Channel {
    #[inline]
    fn try_select(&self, token: &mut Token) -> bool {
        match self.try_recv() {
            Ok(msg) => {
                token.after = Some(msg);
                true
            }
            Err(TryRecvError::Disconnected) => {
                token.after = None;
                true
            }
            Err(TryRecvError::Empty) => false,
        }
    }
    #[inline]
    fn deadline(&self) -> Option<Instant> {
        if self.received.load(Ordering::Relaxed) {
            None
        } else {
            Some(self.delivery_time)
        }
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
        !self.is_empty()
    }
    #[inline]
    fn watch(&self, _oper: Operation, _cx: &Context) -> bool {
        self.is_ready()
    }
    #[inline]
    fn unwatch(&self, _oper: Operation) {}
}
#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use std::time::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Duration::new(rug_fuzz_0, rug_fuzz_1);
        crate::flavors::after::Channel::new(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    use crate::flavors::after::Channel;
    use std::time::{Duration, Instant};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        debug_assert_eq!(p0.try_recv(), Err(TryRecvError::Empty));
        std::thread::sleep(Duration::from_secs(rug_fuzz_1));
        debug_assert_eq!(p0.try_recv(), Ok(Instant::now()));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use std::time::{Instant, Duration};
    use std::thread;
    use crate::flavors::after::{Channel, utils, RecvTimeoutError};
    use std::sync::atomic::{AtomicBool, Ordering};
    #[test]
    fn test_recv() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        let p1: Option<Instant> = Some(Instant::now() + Duration::from_secs(rug_fuzz_1));
        p0.recv(p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::flavors::after::Channel;
    use std::time::{Duration, Instant};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        debug_assert_eq!(p0.is_empty(), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_151 {
    use super::*;
    use crate::flavors::after::Channel;
    use std::time::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        debug_assert!(! p0.is_full());
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_152 {
    use super::*;
    use crate::flavors::after::Channel;
    use std::time::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        debug_assert_eq!(p0.len(), 1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::flavors::after::Channel;
    use std::time::Duration;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        debug_assert_eq!(p0.capacity(), Some(1));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::after::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        p0.deadline();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::flavors::after::Channel;
    use crate::internal::SelectHandle;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        p0.is_ready();
             }
}
}
}    }
}
