//! Waking mechanism for threads blocked on channel operations.
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, ThreadId};
use crate::context::Context;
use crate::select::{Operation, Selected};
use crate::utils::Spinlock;
/// Represents a thread blocked on a specific channel operation.
pub struct Entry {
    /// The operation.
    pub oper: Operation,
    /// Optional packet.
    pub packet: usize,
    /// Context associated with the thread owning this operation.
    pub cx: Context,
}
/// A queue of threads blocked on channel operations.
///
/// This data structure is used by threads to register blocking operations and get woken up once
/// an operation becomes ready.
pub struct Waker {
    /// A list of select operations.
    selectors: Vec<Entry>,
    /// A list of operations waiting to be ready.
    observers: Vec<Entry>,
}
impl Waker {
    /// Creates a new `Waker`.
    #[inline]
    pub fn new() -> Self {
        Waker {
            selectors: Vec::new(),
            observers: Vec::new(),
        }
    }
    /// Registers a select operation.
    #[inline]
    pub fn register(&mut self, oper: Operation, cx: &Context) {
        self.register_with_packet(oper, 0, cx);
    }
    /// Registers a select operation and a packet.
    #[inline]
    pub fn register_with_packet(
        &mut self,
        oper: Operation,
        packet: usize,
        cx: &Context,
    ) {
        self.selectors
            .push(Entry {
                oper,
                packet,
                cx: cx.clone(),
            });
    }
    /// Unregisters a select operation.
    #[inline]
    pub fn unregister(&mut self, oper: Operation) -> Option<Entry> {
        if let Some((i, _))
            = self.selectors.iter().enumerate().find(|&(_, entry)| entry.oper == oper)
        {
            let entry = self.selectors.remove(i);
            Some(entry)
        } else {
            None
        }
    }
    /// Attempts to find another thread's entry, select the operation, and wake it up.
    #[inline]
    pub fn try_select(&mut self) -> Option<Entry> {
        let mut entry = None;
        if !self.selectors.is_empty() {
            let thread_id = current_thread_id();
            for i in 0..self.selectors.len() {
                if self.selectors[i].cx.thread_id() != thread_id {
                    let sel = Selected::Operation(self.selectors[i].oper);
                    let res = self.selectors[i].cx.try_select(sel);
                    if res.is_ok() {
                        self.selectors[i].cx.store_packet(self.selectors[i].packet);
                        self.selectors[i].cx.unpark();
                        entry = Some(self.selectors.remove(i));
                        break;
                    }
                }
            }
        }
        entry
    }
    /// Returns `true` if there is an entry which can be selected by the current thread.
    #[inline]
    pub fn can_select(&self) -> bool {
        if self.selectors.is_empty() {
            false
        } else {
            let thread_id = current_thread_id();
            self.selectors
                .iter()
                .any(|entry| {
                    entry.cx.thread_id() != thread_id
                        && entry.cx.selected() == Selected::Waiting
                })
        }
    }
    /// Registers an operation waiting to be ready.
    #[inline]
    pub fn watch(&mut self, oper: Operation, cx: &Context) {
        self.observers
            .push(Entry {
                oper,
                packet: 0,
                cx: cx.clone(),
            });
    }
    /// Unregisters an operation waiting to be ready.
    #[inline]
    pub fn unwatch(&mut self, oper: Operation) {
        self.observers.retain(|e| e.oper != oper);
    }
    /// Notifies all operations waiting to be ready.
    #[inline]
    pub fn notify(&mut self) {
        for entry in self.observers.drain(..) {
            if entry.cx.try_select(Selected::Operation(entry.oper)).is_ok() {
                entry.cx.unpark();
            }
        }
    }
    /// Notifies all registered operations that the channel is disconnected.
    #[inline]
    pub fn disconnect(&mut self) {
        for entry in self.selectors.iter() {
            if entry.cx.try_select(Selected::Disconnected).is_ok() {
                entry.cx.unpark();
            }
        }
        self.notify();
    }
}
impl Drop for Waker {
    #[inline]
    fn drop(&mut self) {
        debug_assert_eq!(self.selectors.len(), 0);
        debug_assert_eq!(self.observers.len(), 0);
    }
}
/// A waker that can be shared among threads without locking.
///
/// This is a simple wrapper around `Waker` that internally uses a mutex for synchronization.
pub struct SyncWaker {
    /// The inner `Waker`.
    inner: Spinlock<Waker>,
    /// `true` if the waker is empty.
    is_empty: AtomicBool,
}
impl SyncWaker {
    /// Creates a new `SyncWaker`.
    #[inline]
    pub fn new() -> Self {
        SyncWaker {
            inner: Spinlock::new(Waker::new()),
            is_empty: AtomicBool::new(true),
        }
    }
    /// Registers the current thread with an operation.
    #[inline]
    pub fn register(&self, oper: Operation, cx: &Context) {
        let mut inner = self.inner.lock();
        inner.register(oper, cx);
        self.is_empty
            .store(
                inner.selectors.is_empty() && inner.observers.is_empty(),
                Ordering::SeqCst,
            );
    }
    /// Unregisters an operation previously registered by the current thread.
    #[inline]
    pub fn unregister(&self, oper: Operation) -> Option<Entry> {
        let mut inner = self.inner.lock();
        let entry = inner.unregister(oper);
        self.is_empty
            .store(
                inner.selectors.is_empty() && inner.observers.is_empty(),
                Ordering::SeqCst,
            );
        entry
    }
    /// Attempts to find one thread (not the current one), select its operation, and wake it up.
    #[inline]
    pub fn notify(&self) {
        if !self.is_empty.load(Ordering::SeqCst) {
            let mut inner = self.inner.lock();
            if !self.is_empty.load(Ordering::SeqCst) {
                inner.try_select();
                inner.notify();
                self.is_empty
                    .store(
                        inner.selectors.is_empty() && inner.observers.is_empty(),
                        Ordering::SeqCst,
                    );
            }
        }
    }
    /// Registers an operation waiting to be ready.
    #[inline]
    pub fn watch(&self, oper: Operation, cx: &Context) {
        let mut inner = self.inner.lock();
        inner.watch(oper, cx);
        self.is_empty
            .store(
                inner.selectors.is_empty() && inner.observers.is_empty(),
                Ordering::SeqCst,
            );
    }
    /// Unregisters an operation waiting to be ready.
    #[inline]
    pub fn unwatch(&self, oper: Operation) {
        let mut inner = self.inner.lock();
        inner.unwatch(oper);
        self.is_empty
            .store(
                inner.selectors.is_empty() && inner.observers.is_empty(),
                Ordering::SeqCst,
            );
    }
    /// Notifies all threads that the channel is disconnected.
    #[inline]
    pub fn disconnect(&self) {
        let mut inner = self.inner.lock();
        inner.disconnect();
        self.is_empty
            .store(
                inner.selectors.is_empty() && inner.observers.is_empty(),
                Ordering::SeqCst,
            );
    }
}
impl Drop for SyncWaker {
    #[inline]
    fn drop(&mut self) {
        debug_assert_eq!(self.is_empty.load(Ordering::SeqCst), true);
    }
}
/// Returns the id of the current thread.
#[inline]
fn current_thread_id() -> ThreadId {
    thread_local! {
        #[doc = " Cached thread-local id."] static THREAD_ID : ThreadId =
        thread::current().id();
    }
    THREAD_ID.try_with(|id| *id).unwrap_or_else(|_| thread::current().id())
}
#[cfg(test)]
mod tests_rug_101 {
    use super::current_thread_id;
    use std::thread::ThreadId;
    use std::thread;
    #[test]
    fn test_current_thread_id() {
        let _rug_st_tests_rug_101_rrrruuuugggg_test_current_thread_id = 0;
        let id = current_thread_id();
        let current_id = thread::current().id();
        debug_assert_eq!(id, current_id);
        let _rug_ed_tests_rug_101_rrrruuuugggg_test_current_thread_id = 0;
    }
}
#[cfg(test)]
mod tests_rug_102 {
    use super::*;
    use crate::waker::Waker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_102_rrrruuuugggg_test_rug = 0;
        let waker: Waker = Waker::new();
        let _rug_ed_tests_rug_102_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_106 {
    use super::*;
    use crate::waker::Waker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_106_rrrruuuugggg_test_rug = 0;
        let mut p0 = Waker::new();
        p0.try_select();
        let _rug_ed_tests_rug_106_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_107 {
    use super::*;
    use crate::waker::Waker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_107_rrrruuuugggg_test_rug = 0;
        let mut p0 = Waker::new();
        debug_assert_eq!(p0.can_select(), false);
        let _rug_ed_tests_rug_107_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use crate::waker::Waker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_110_rrrruuuugggg_test_rug = 0;
        let mut p0 = Waker::new();
        p0.notify();
        let _rug_ed_tests_rug_110_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_111 {
    use super::*;
    use crate::waker::Waker;
    #[test]
    fn test_disconnect() {
        let _rug_st_tests_rug_111_rrrruuuugggg_test_disconnect = 0;
        let mut p0 = Waker::new();
        p0.disconnect();
        let _rug_ed_tests_rug_111_rrrruuuugggg_test_disconnect = 0;
    }
}
#[cfg(test)]
mod tests_rug_113 {
    use super::*;
    use crate::waker::SyncWaker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_113_rrrruuuugggg_test_rug = 0;
        SyncWaker::new();
        let _rug_ed_tests_rug_113_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    use crate::waker::SyncWaker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_116_rrrruuuugggg_test_rug = 0;
        let mut p0 = SyncWaker::new();
        SyncWaker::notify(&p0);
        let _rug_ed_tests_rug_116_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_119 {
    use super::*;
    use crate::waker::SyncWaker;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_119_rrrruuuugggg_test_rug = 0;
        let mut p0 = SyncWaker::new();
        SyncWaker::disconnect(&p0);
        let _rug_ed_tests_rug_119_rrrruuuugggg_test_rug = 0;
    }
}
