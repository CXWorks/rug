//! Thread-local context used in select.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, Thread, ThreadId};
use std::time::Instant;

use crossbeam_utils::Backoff;

use crate::select::Selected;

/// Thread-local context used in select.
#[derive(Debug, Clone)]
pub struct Context {
    inner: Arc<Inner>,
}

/// Inner representation of `Context`.
#[derive(Debug)]
struct Inner {
    /// Selected operation.
    select: AtomicUsize,

    /// A slot into which another thread may store a pointer to its `Packet`.
    packet: AtomicUsize,

    /// Thread handle.
    thread: Thread,

    /// Thread id.
    thread_id: ThreadId,
}

impl Context {
    /// Creates a new context for the duration of the closure.
    #[inline]
    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Context) -> R,
    {
        thread_local! {
            /// Cached thread-local context.
            static CONTEXT: Cell<Option<Context>> = Cell::new(Some(Context::new()));
        }

        let mut f = Some(f);
        let mut f = move |cx: &Context| -> R {
            let f = f.take().unwrap();
            f(cx)
        };

        CONTEXT
            .try_with(|cell| match cell.take() {
                None => f(&Context::new()),
                Some(cx) => {
                    cx.reset();
                    let res = f(&cx);
                    cell.set(Some(cx));
                    res
                }
            })
            .unwrap_or_else(|_| f(&Context::new()))
    }

    /// Creates a new `Context`.
    #[cold]
    fn new() -> Context {
        Context {
            inner: Arc::new(Inner {
                select: AtomicUsize::new(Selected::Waiting.into()),
                packet: AtomicUsize::new(0),
                thread: thread::current(),
                thread_id: thread::current().id(),
            }),
        }
    }

    /// Resets `select` and `packet`.
    #[inline]
    fn reset(&self) {
        self.inner
            .select
            .store(Selected::Waiting.into(), Ordering::Release);
        self.inner.packet.store(0, Ordering::Release);
    }

    /// Attempts to select an operation.
    ///
    /// On failure, the previously selected operation is returned.
    #[inline]
    pub fn try_select(&self, select: Selected) -> Result<(), Selected> {
        self.inner
            .select
            .compare_exchange(
                Selected::Waiting.into(),
                select.into(),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|e| e.into())
    }

    /// Returns the selected operation.
    #[inline]
    pub fn selected(&self) -> Selected {
        Selected::from(self.inner.select.load(Ordering::Acquire))
    }

    /// Stores a packet.
    ///
    /// This method must be called after `try_select` succeeds and there is a packet to provide.
    #[inline]
    pub fn store_packet(&self, packet: usize) {
        if packet != 0 {
            self.inner.packet.store(packet, Ordering::Release);
        }
    }

    /// Waits until a packet is provided and returns it.
    #[inline]
    pub fn wait_packet(&self) -> usize {
        let backoff = Backoff::new();
        loop {
            let packet = self.inner.packet.load(Ordering::Acquire);
            if packet != 0 {
                return packet;
            }
            backoff.snooze();
        }
    }

    /// Waits until an operation is selected and returns it.
    ///
    /// If the deadline is reached, `Selected::Aborted` will be selected.
    #[inline]
    pub fn wait_until(&self, deadline: Option<Instant>) -> Selected {
        // Spin for a short time, waiting until an operation is selected.
        let backoff = Backoff::new();
        loop {
            let sel = Selected::from(self.inner.select.load(Ordering::Acquire));
            if sel != Selected::Waiting {
                return sel;
            }

            if backoff.is_completed() {
                break;
            } else {
                backoff.snooze();
            }
        }

        loop {
            // Check whether an operation has been selected.
            let sel = Selected::from(self.inner.select.load(Ordering::Acquire));
            if sel != Selected::Waiting {
                return sel;
            }

            // If there's a deadline, park the current thread until the deadline is reached.
            if let Some(end) = deadline {
                let now = Instant::now();

                if now < end {
                    thread::park_timeout(end - now);
                } else {
                    // The deadline has been reached. Try aborting select.
                    return match self.try_select(Selected::Aborted) {
                        Ok(()) => Selected::Aborted,
                        Err(s) => s,
                    };
                }
            } else {
                thread::park();
            }
        }
    }

    /// Unparks the thread this context belongs to.
    #[inline]
    pub fn unpark(&self) {
        self.inner.thread.unpark();
    }

    /// Returns the id of the thread this context belongs to.
    #[inline]
    pub fn thread_id(&self) -> ThreadId {
        self.inner.thread_id
    }
}
use crate::context;

#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_121_prepare {
            use std::cell::Cell;
            use crate::context;
            use crate::RecvError;

            #[test]
            fn sample() {
                let v41: Box<RecvError> = Box::new(RecvError);
            }
        }

        let p0: Box<dyn FnOnce(&context::Context) -> ()> = Box::new(|_cx| {
            // Test logic can be added here
        });

        context::Context::with(p0);
    }
}#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::context;
    
    #[test]
    fn test_rug() {
        context::Context::new();
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::context;

    #[test]
    fn test_rug() {
        let mut p0 = context::Context::new();

        context::Context::selected(&p0);
    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::context;

    #[test]
    fn test_store_packet() {
        let mut context = context::Context::new();

        let packet = 42;

        context.store_packet(packet);
    }
}#[cfg(test)]
mod tests_rug_128 {
    use super::*;
    use std::time::Instant;
    use crate::context;
    use crate::context::Context;
    
    #[test]
    fn test_rug() {
        let mut p0 = Context::new(); // Constructing the first argument using Context::new()
        let mut p1: Option<Instant> = Some(Instant::now()); // Constructing the second argument using Some(Instant::now())
        
        context::Context::wait_until(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::context;

    #[test]
    fn test_rug() {
        let p0 = context::Context::new();

        crate::context::Context::unpark(&p0);
    }
}#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::context;

    #[test]
    fn test_rug() {
        let mut p0 = context::Context::new();

        crate::context::Context::thread_id(&p0);
    }
}