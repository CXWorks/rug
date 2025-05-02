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
            #[doc = " Cached thread-local context."] static CONTEXT : Cell < Option <
            Context >> = Cell::new(Some(Context::new()));
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
        self.inner.select.store(Selected::Waiting.into(), Ordering::Release);
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
            let sel = Selected::from(self.inner.select.load(Ordering::Acquire));
            if sel != Selected::Waiting {
                return sel;
            }
            if let Some(end) = deadline {
                let now = Instant::now();
                if now < end {
                    thread::park_timeout(end - now);
                } else {
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
        let _rug_st_tests_rug_121_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_121_prepare {
            use std::cell::Cell;
            use crate::context;
            use crate::RecvError;
            #[test]
            fn sample() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

                let _rug_st_tests_rug_121_rrrruuuugggg_sample = rug_fuzz_0;
                let v41: Box<RecvError> = Box::new(RecvError);
                let _rug_ed_tests_rug_121_rrrruuuugggg_sample = rug_fuzz_1;
             }
});            }
        }
        let p0: Box<dyn FnOnce(&context::Context) -> ()> = Box::new(|_cx| {});
        context::Context::with(p0);
        let _rug_ed_tests_rug_121_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::context;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_122_rrrruuuugggg_test_rug = 0;
        context::Context::new();
        let _rug_ed_tests_rug_122_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::context;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_125_rrrruuuugggg_test_rug = 0;
        let mut p0 = context::Context::new();
        context::Context::selected(&p0);
        let _rug_ed_tests_rug_125_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::context;
    #[test]
    fn test_store_packet() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut context = context::Context::new();
        let packet = rug_fuzz_0;
        context.store_packet(packet);
             }
});    }
}
#[cfg(test)]
mod tests_rug_128 {
    use super::*;
    use std::time::Instant;
    use crate::context;
    use crate::context::Context;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_128_rrrruuuugggg_test_rug = 0;
        let mut p0 = Context::new();
        let mut p1: Option<Instant> = Some(Instant::now());
        context::Context::wait_until(&p0, p1);
        let _rug_ed_tests_rug_128_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::context;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_129_rrrruuuugggg_test_rug = 0;
        let p0 = context::Context::new();
        crate::context::Context::unpark(&p0);
        let _rug_ed_tests_rug_129_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::context;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_130_rrrruuuugggg_test_rug = 0;
        let mut p0 = context::Context::new();
        crate::context::Context::thread_id(&p0);
        let _rug_ed_tests_rug_130_rrrruuuugggg_test_rug = 0;
    }
}
