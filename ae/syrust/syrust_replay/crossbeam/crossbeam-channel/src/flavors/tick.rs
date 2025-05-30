//! Channel that delivers messages periodically.
//!
//! Messages cannot be sent into this kind of channel; they are materialized on demand.
use std::thread;
use std::time::{Duration, Instant};
use crossbeam_utils::atomic::AtomicCell;
use crate::context::Context;
use crate::err::{RecvTimeoutError, TryRecvError};
use crate::select::{Operation, SelectHandle, Token};
/// Result of a receive operation.
pub type TickToken = Option<Instant>;
/// Channel that delivers messages periodically.
pub struct Channel {
    /// The instant at which the next message will be delivered.
    delivery_time: AtomicCell<Instant>,
    /// The time interval in which messages get delivered.
    duration: Duration,
}
impl Channel {
    /// Creates a channel that delivers messages periodically.
    #[inline]
    pub fn new(dur: Duration) -> Self {
        Channel {
            delivery_time: AtomicCell::new(Instant::now() + dur),
            duration: dur,
        }
    }
    /// Attempts to receive a message without blocking.
    #[inline]
    pub fn try_recv(&self) -> Result<Instant, TryRecvError> {
        loop {
            let now = Instant::now();
            let delivery_time = self.delivery_time.load();
            if now < delivery_time {
                return Err(TryRecvError::Empty);
            }
            if self
                .delivery_time
                .compare_exchange(delivery_time, now + self.duration)
                .is_ok()
            {
                return Ok(delivery_time);
            }
        }
    }
    /// Receives a message from the channel.
    #[inline]
    pub fn recv(&self, deadline: Option<Instant>) -> Result<Instant, RecvTimeoutError> {
        loop {
            let delivery_time = self.delivery_time.load();
            let now = Instant::now();
            if let Some(d) = deadline {
                if d < delivery_time {
                    if now < d {
                        thread::sleep(d - now);
                    }
                    return Err(RecvTimeoutError::Timeout);
                }
            }
            if self
                .delivery_time
                .compare_exchange(delivery_time, delivery_time.max(now) + self.duration)
                .is_ok()
            {
                if now < delivery_time {
                    thread::sleep(delivery_time - now);
                }
                return Ok(delivery_time);
            }
        }
    }
    /// Reads a message from the channel.
    #[inline]
    pub unsafe fn read(&self, token: &mut Token) -> Result<Instant, ()> {
        token.tick.ok_or(())
    }
    /// Returns `true` if the channel is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        Instant::now() < self.delivery_time.load()
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
                token.tick = Some(msg);
                true
            }
            Err(TryRecvError::Disconnected) => {
                token.tick = None;
                true
            }
            Err(TryRecvError::Empty) => false,
        }
    }
    #[inline]
    fn deadline(&self) -> Option<Instant> {
        Some(self.delivery_time.load())
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
mod tests_rug_252 {
    use super::*;
    use std::time::Duration;
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let dur = Duration::new(rug_fuzz_0, rug_fuzz_1);
        let channel = Channel::new(dur);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_253 {
    use super::*;
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(std::time::Duration::from_secs(rug_fuzz_0));
        p0.try_recv();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_254 {
    use super::*;
    use std::time::{Instant, Duration};
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(Duration::from_secs(rug_fuzz_0));
        let mut p1: Option<Instant> = None;
        p0.recv(p1).unwrap();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_256 {
    use super::*;
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(std::time::Duration::from_secs(rug_fuzz_0));
        debug_assert_eq!(p0.is_empty(), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_257 {
    use super::*;
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(std::time::Duration::from_secs(rug_fuzz_0));
        debug_assert!(! p0.is_full());
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_258 {
    use super::*;
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = Channel::new(std::time::Duration::from_secs(rug_fuzz_0));
        debug_assert_eq!(p0.len(), 1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_259 {
    use super::*;
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(std::time::Duration::from_secs(rug_fuzz_0));
        debug_assert_eq!(p0.capacity(), Some(1));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_260 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::tick::Channel;
    use crate::select::Token;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(std::time::Duration::from_secs(rug_fuzz_0));
        let mut p1 = Token::default();
        p0.try_select(&mut p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_261 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(std::time::Duration::from_secs(rug_fuzz_0));
        p0.deadline();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_265 {
    use super::*;
    use crate::internal::SelectHandle;
    use crate::flavors::tick::Channel;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Channel::new(std::time::Duration::from_secs(rug_fuzz_0));
        p0.is_ready();
             }
}
}
}    }
}
