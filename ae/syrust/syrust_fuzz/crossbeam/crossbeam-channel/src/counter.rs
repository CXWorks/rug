//! Reference counter for channels.
use std::isize;
use std::ops;
use std::process;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
/// Reference counter internals.
struct Counter<C> {
    /// The number of senders associated with the channel.
    senders: AtomicUsize,
    /// The number of receivers associated with the channel.
    receivers: AtomicUsize,
    /// Set to `true` if the last sender or the last receiver reference deallocates the channel.
    destroy: AtomicBool,
    /// The internal channel.
    chan: C,
}
/// Wraps a channel into the reference counter.
pub fn new<C>(chan: C) -> (Sender<C>, Receiver<C>) {
    let counter = Box::into_raw(
        Box::new(Counter {
            senders: AtomicUsize::new(1),
            receivers: AtomicUsize::new(1),
            destroy: AtomicBool::new(false),
            chan,
        }),
    );
    let s = Sender { counter };
    let r = Receiver { counter };
    (s, r)
}
/// The sending side.
pub struct Sender<C> {
    counter: *mut Counter<C>,
}
impl<C> Sender<C> {
    /// Returns the internal `Counter`.
    fn counter(&self) -> &Counter<C> {
        unsafe { &*self.counter }
    }
    /// Acquires another sender reference.
    pub fn acquire(&self) -> Sender<C> {
        let count = self.counter().senders.fetch_add(1, Ordering::Relaxed);
        if count > isize::MAX as usize {
            process::abort();
        }
        Sender { counter: self.counter }
    }
    /// Releases the sender reference.
    ///
    /// Function `disconnect` will be called if this is the last sender reference.
    pub unsafe fn release<F: FnOnce(&C) -> bool>(&self, disconnect: F) {
        if self.counter().senders.fetch_sub(1, Ordering::AcqRel) == 1 {
            disconnect(&self.counter().chan);
            if self.counter().destroy.swap(true, Ordering::AcqRel) {
                drop(Box::from_raw(self.counter));
            }
        }
    }
}
impl<C> ops::Deref for Sender<C> {
    type Target = C;
    fn deref(&self) -> &C {
        &self.counter().chan
    }
}
impl<C> PartialEq for Sender<C> {
    fn eq(&self, other: &Sender<C>) -> bool {
        self.counter == other.counter
    }
}
/// The receiving side.
pub struct Receiver<C> {
    counter: *mut Counter<C>,
}
impl<C> Receiver<C> {
    /// Returns the internal `Counter`.
    fn counter(&self) -> &Counter<C> {
        unsafe { &*self.counter }
    }
    /// Acquires another receiver reference.
    pub fn acquire(&self) -> Receiver<C> {
        let count = self.counter().receivers.fetch_add(1, Ordering::Relaxed);
        if count > isize::MAX as usize {
            process::abort();
        }
        Receiver { counter: self.counter }
    }
    /// Releases the receiver reference.
    ///
    /// Function `disconnect` will be called if this is the last receiver reference.
    pub unsafe fn release<F: FnOnce(&C) -> bool>(&self, disconnect: F) {
        if self.counter().receivers.fetch_sub(1, Ordering::AcqRel) == 1 {
            disconnect(&self.counter().chan);
            if self.counter().destroy.swap(true, Ordering::AcqRel) {
                drop(Box::from_raw(self.counter));
            }
        }
    }
}
impl<C> ops::Deref for Receiver<C> {
    type Target = C;
    fn deref(&self) -> &C {
        &self.counter().chan
    }
}
impl<C> PartialEq for Receiver<C> {
    fn eq(&self, other: &Receiver<C>) -> bool {
        self.counter == other.counter
    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use crate::{Sender, Receiver};
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_51_rrrruuuugggg_test_new = 0;
        struct MockChannel;
        let p0 = MockChannel;
        crate::counter::new(p0);
        let _rug_ed_tests_rug_51_rrrruuuugggg_test_new = 0;
    }
}
