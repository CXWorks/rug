//! Interface to the select mechanism.
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::time::{Duration, Instant};
use crossbeam_utils::Backoff;
use crate::channel::{self, Receiver, Sender};
use crate::context::Context;
use crate::err::{ReadyTimeoutError, TryReadyError};
use crate::err::{RecvError, SendError};
use crate::err::{SelectTimeoutError, TrySelectError};
use crate::flavors;
use crate::utils;
/// Temporary data that gets initialized during select or a blocking operation, and is consumed by
/// `read` or `write`.
///
/// Each field contains data associated with a specific channel flavor.
#[derive(Debug, Default)]
pub struct Token {
    pub after: flavors::after::AfterToken,
    pub array: flavors::array::ArrayToken,
    pub list: flavors::list::ListToken,
    pub never: flavors::never::NeverToken,
    pub tick: flavors::tick::TickToken,
    pub zero: flavors::zero::ZeroToken,
}
/// Identifier associated with an operation by a specific thread on a specific channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation(usize);
impl Operation {
    /// Creates an operation identifier from a mutable reference.
    ///
    /// This function essentially just turns the address of the reference into a number. The
    /// reference should point to a variable that is specific to the thread and the operation,
    /// and is alive for the entire duration of select or blocking operation.
    #[inline]
    pub fn hook<T>(r: &mut T) -> Operation {
        let val = r as *mut T as usize;
        assert!(val > 2);
        Operation(val)
    }
}
/// Current state of a select or a blocking operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selected {
    /// Still waiting for an operation.
    Waiting,
    /// The attempt to block the current thread has been aborted.
    Aborted,
    /// An operation became ready because a channel is disconnected.
    Disconnected,
    /// An operation became ready because a message can be sent or received.
    Operation(Operation),
}
impl From<usize> for Selected {
    #[inline]
    fn from(val: usize) -> Selected {
        match val {
            0 => Selected::Waiting,
            1 => Selected::Aborted,
            2 => Selected::Disconnected,
            oper => Selected::Operation(Operation(oper)),
        }
    }
}
impl Into<usize> for Selected {
    #[inline]
    fn into(self) -> usize {
        match self {
            Selected::Waiting => 0,
            Selected::Aborted => 1,
            Selected::Disconnected => 2,
            Selected::Operation(Operation(val)) => val,
        }
    }
}
/// A receiver or a sender that can participate in select.
///
/// This is a handle that assists select in executing an operation, registration, deciding on the
/// appropriate deadline for blocking, etc.
pub trait SelectHandle {
    /// Attempts to select an operation and returns `true` on success.
    fn try_select(&self, token: &mut Token) -> bool;
    /// Returns a deadline for an operation, if there is one.
    fn deadline(&self) -> Option<Instant>;
    /// Registers an operation for execution and returns `true` if it is now ready.
    fn register(&self, oper: Operation, cx: &Context) -> bool;
    /// Unregisters an operation for execution.
    fn unregister(&self, oper: Operation);
    /// Attempts to select an operation the thread got woken up for and returns `true` on success.
    fn accept(&self, token: &mut Token, cx: &Context) -> bool;
    /// Returns `true` if an operation can be executed without blocking.
    fn is_ready(&self) -> bool;
    /// Registers an operation for readiness notification and returns `true` if it is now ready.
    fn watch(&self, oper: Operation, cx: &Context) -> bool;
    /// Unregisters an operation for readiness notification.
    fn unwatch(&self, oper: Operation);
}
impl<T: SelectHandle> SelectHandle for &T {
    fn try_select(&self, token: &mut Token) -> bool {
        (**self).try_select(token)
    }
    fn deadline(&self) -> Option<Instant> {
        (**self).deadline()
    }
    fn register(&self, oper: Operation, cx: &Context) -> bool {
        (**self).register(oper, cx)
    }
    fn unregister(&self, oper: Operation) {
        (**self).unregister(oper);
    }
    fn accept(&self, token: &mut Token, cx: &Context) -> bool {
        (**self).accept(token, cx)
    }
    fn is_ready(&self) -> bool {
        (**self).is_ready()
    }
    fn watch(&self, oper: Operation, cx: &Context) -> bool {
        (**self).watch(oper, cx)
    }
    fn unwatch(&self, oper: Operation) {
        (**self).unwatch(oper)
    }
}
/// Determines when a select operation should time out.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Timeout {
    /// No blocking.
    Now,
    /// Block forever.
    Never,
    /// Time out after the time instant.
    At(Instant),
}
/// Runs until one of the operations is selected, potentially blocking the current thread.
///
/// Successful receive operations will have to be followed up by `channel::read()` and successful
/// send operations by `channel::write()`.
fn run_select(
    handles: &mut [(&dyn SelectHandle, usize, *const u8)],
    timeout: Timeout,
) -> Option<(Token, usize, *const u8)> {
    if handles.is_empty() {
        match timeout {
            Timeout::Now => return None,
            Timeout::Never => {
                utils::sleep_until(None);
                unreachable!();
            }
            Timeout::At(when) => {
                utils::sleep_until(Some(when));
                return None;
            }
        }
    }
    utils::shuffle(handles);
    let mut token = Token::default();
    for &(handle, i, ptr) in handles.iter() {
        if handle.try_select(&mut token) {
            return Some((token, i, ptr));
        }
    }
    loop {
        let res = Context::with(|cx| {
            let mut sel = Selected::Waiting;
            let mut registered_count = 0;
            let mut index_ready = None;
            if let Timeout::Now = timeout {
                cx.try_select(Selected::Aborted).unwrap();
            }
            for (handle, i, _) in handles.iter_mut() {
                registered_count += 1;
                if handle.register(Operation::hook::<&dyn SelectHandle>(handle), cx) {
                    sel = match cx.try_select(Selected::Aborted) {
                        Ok(()) => {
                            index_ready = Some(*i);
                            Selected::Aborted
                        }
                        Err(s) => s,
                    };
                    break;
                }
                sel = cx.selected();
                if sel != Selected::Waiting {
                    break;
                }
            }
            if sel == Selected::Waiting {
                let mut deadline: Option<Instant> = match timeout {
                    Timeout::Now => return None,
                    Timeout::Never => None,
                    Timeout::At(when) => Some(when),
                };
                for &(handle, _, _) in handles.iter() {
                    if let Some(x) = handle.deadline() {
                        deadline = deadline.map(|y| x.min(y)).or(Some(x));
                    }
                }
                sel = cx.wait_until(deadline);
            }
            for (handle, _, _) in handles.iter_mut().take(registered_count) {
                handle.unregister(Operation::hook::<&dyn SelectHandle>(handle));
            }
            match sel {
                Selected::Waiting => unreachable!(),
                Selected::Aborted => {
                    if let Some(index_ready) = index_ready {
                        for &(handle, i, ptr) in handles.iter() {
                            if i == index_ready && handle.try_select(&mut token) {
                                return Some((i, ptr));
                            }
                        }
                    }
                }
                Selected::Disconnected => {}
                Selected::Operation(_) => {
                    for (handle, i, ptr) in handles.iter_mut() {
                        if sel
                            == Selected::Operation(
                                Operation::hook::<&dyn SelectHandle>(handle),
                            )
                        {
                            if handle.accept(&mut token, cx) {
                                return Some((*i, *ptr));
                            }
                        }
                    }
                }
            }
            None
        });
        if let Some((i, ptr)) = res {
            return Some((token, i, ptr));
        }
        for &(handle, i, ptr) in handles.iter() {
            if handle.try_select(&mut token) {
                return Some((token, i, ptr));
            }
        }
        match timeout {
            Timeout::Now => return None,
            Timeout::Never => {}
            Timeout::At(when) => {
                if Instant::now() >= when {
                    return None;
                }
            }
        }
    }
}
/// Runs until one of the operations becomes ready, potentially blocking the current thread.
fn run_ready(
    handles: &mut [(&dyn SelectHandle, usize, *const u8)],
    timeout: Timeout,
) -> Option<usize> {
    if handles.is_empty() {
        match timeout {
            Timeout::Now => return None,
            Timeout::Never => {
                utils::sleep_until(None);
                unreachable!();
            }
            Timeout::At(when) => {
                utils::sleep_until(Some(when));
                return None;
            }
        }
    }
    utils::shuffle(handles);
    loop {
        let backoff = Backoff::new();
        loop {
            for &(handle, i, _) in handles.iter() {
                if handle.is_ready() {
                    return Some(i);
                }
            }
            if backoff.is_completed() {
                break;
            } else {
                backoff.snooze();
            }
        }
        match timeout {
            Timeout::Now => return None,
            Timeout::Never => {}
            Timeout::At(when) => {
                if Instant::now() >= when {
                    return None;
                }
            }
        }
        let res = Context::with(|cx| {
            let mut sel = Selected::Waiting;
            let mut registered_count = 0;
            for (handle, _, _) in handles.iter_mut() {
                registered_count += 1;
                let oper = Operation::hook::<&dyn SelectHandle>(handle);
                if handle.watch(oper, cx) {
                    sel = match cx.try_select(Selected::Operation(oper)) {
                        Ok(()) => Selected::Operation(oper),
                        Err(s) => s,
                    };
                    break;
                }
                sel = cx.selected();
                if sel != Selected::Waiting {
                    break;
                }
            }
            if sel == Selected::Waiting {
                let mut deadline: Option<Instant> = match timeout {
                    Timeout::Now => unreachable!(),
                    Timeout::Never => None,
                    Timeout::At(when) => Some(when),
                };
                for &(handle, _, _) in handles.iter() {
                    if let Some(x) = handle.deadline() {
                        deadline = deadline.map(|y| x.min(y)).or(Some(x));
                    }
                }
                sel = cx.wait_until(deadline);
            }
            for (handle, _, _) in handles.iter_mut().take(registered_count) {
                handle.unwatch(Operation::hook::<&dyn SelectHandle>(handle));
            }
            match sel {
                Selected::Waiting => unreachable!(),
                Selected::Aborted => {}
                Selected::Disconnected => {}
                Selected::Operation(_) => {
                    for (handle, i, _) in handles.iter_mut() {
                        let oper = Operation::hook::<&dyn SelectHandle>(handle);
                        if sel == Selected::Operation(oper) {
                            return Some(*i);
                        }
                    }
                }
            }
            None
        });
        if res.is_some() {
            return res;
        }
    }
}
/// Attempts to select one of the operations without blocking.
#[inline]
pub fn try_select<'a>(
    handles: &mut [(&'a dyn SelectHandle, usize, *const u8)],
) -> Result<SelectedOperation<'a>, TrySelectError> {
    match run_select(handles, Timeout::Now) {
        None => Err(TrySelectError),
        Some((token, index, ptr)) => {
            Ok(SelectedOperation {
                token,
                index,
                ptr,
                _marker: PhantomData,
            })
        }
    }
}
/// Blocks until one of the operations becomes ready and selects it.
#[inline]
pub fn select<'a>(
    handles: &mut [(&'a dyn SelectHandle, usize, *const u8)],
) -> SelectedOperation<'a> {
    if handles.is_empty() {
        panic!("no operations have been added to `Select`");
    }
    let (token, index, ptr) = run_select(handles, Timeout::Never).unwrap();
    SelectedOperation {
        token,
        index,
        ptr,
        _marker: PhantomData,
    }
}
/// Blocks for a limited time until one of the operations becomes ready and selects it.
#[inline]
pub fn select_timeout<'a>(
    handles: &mut [(&'a dyn SelectHandle, usize, *const u8)],
    timeout: Duration,
) -> Result<SelectedOperation<'a>, SelectTimeoutError> {
    let timeout = Timeout::At(Instant::now() + timeout);
    match run_select(handles, timeout) {
        None => Err(SelectTimeoutError),
        Some((token, index, ptr)) => {
            Ok(SelectedOperation {
                token,
                index,
                ptr,
                _marker: PhantomData,
            })
        }
    }
}
/// Selects from a set of channel operations.
///
/// `Select` allows you to define a set of channel operations, wait until any one of them becomes
/// ready, and finally execute it. If multiple operations are ready at the same time, a random one
/// among them is selected.
///
/// An operation is considered to be ready if it doesn't have to block. Note that it is ready even
/// when it will simply return an error because the channel is disconnected.
///
/// The [`select!`] macro is a convenience wrapper around `Select`. However, it cannot select over a
/// dynamically created list of channel operations.
///
/// Once a list of operations has been built with `Select`, there are two different ways of
/// proceeding:
///
/// * Select an operation with [`try_select`], [`select`], or [`select_timeout`]. If successful,
///   the returned selected operation has already begun and **must** be completed. If we don't
///   complete it, a panic will occur.
///
/// * Wait for an operation to become ready with [`try_ready`], [`ready`], or [`ready_timeout`]. If
///   successful, we may attempt to execute the operation, but are not obliged to. In fact, it's
///   possible for another thread to make the operation not ready just before we try executing it,
///   so it's wise to use a retry loop. However, note that these methods might return with success
///   spuriously, so it's a good idea to always double check if the operation is really ready.
///
/// # Examples
///
/// Use [`select`] to receive a message from a list of receivers:
///
/// ```
/// use crossbeam_channel::{Receiver, RecvError, Select};
///
/// fn recv_multiple<T>(rs: &[Receiver<T>]) -> Result<T, RecvError> {
///     // Build a list of operations.
///     let mut sel = Select::new();
///     for r in rs {
///         sel.recv(r);
///     }
///
///     // Complete the selected operation.
///     let oper = sel.select();
///     let index = oper.index();
///     oper.recv(&rs[index])
/// }
/// ```
///
/// Use [`ready`] to receive a message from a list of receivers:
///
/// ```
/// use crossbeam_channel::{Receiver, RecvError, Select};
///
/// fn recv_multiple<T>(rs: &[Receiver<T>]) -> Result<T, RecvError> {
///     // Build a list of operations.
///     let mut sel = Select::new();
///     for r in rs {
///         sel.recv(r);
///     }
///
///     loop {
///         // Wait until a receive operation becomes ready and try executing it.
///         let index = sel.ready();
///         let res = rs[index].try_recv();
///
///         // If the operation turns out not to be ready, retry.
///         if let Err(e) = res {
///             if e.is_empty() {
///                 continue;
///             }
///         }
///
///         // Success!
///         return res.map_err(|_| RecvError);
///     }
/// }
/// ```
///
/// [`select!`]: macro.select.html
/// [`try_select`]: struct.Select.html#method.try_select
/// [`select`]: struct.Select.html#method.select
/// [`select_timeout`]: struct.Select.html#method.select_timeout
/// [`try_ready`]: struct.Select.html#method.try_ready
/// [`ready`]: struct.Select.html#method.ready
/// [`ready_timeout`]: struct.Select.html#method.ready_timeout
pub struct Select<'a> {
    /// A list of senders and receivers participating in selection.
    handles: Vec<(&'a dyn SelectHandle, usize, *const u8)>,
    /// The next index to assign to an operation.
    next_index: usize,
}
unsafe impl Send for Select<'_> {}
unsafe impl Sync for Select<'_> {}
impl<'a> Select<'a> {
    /// Creates an empty list of channel operations for selection.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::Select;
    ///
    /// let mut sel = Select::new();
    ///
    /// // The list of operations is empty, which means no operation can be selected.
    /// assert!(sel.try_select().is_err());
    /// ```
    pub fn new() -> Select<'a> {
        Select {
            handles: Vec::with_capacity(4),
            next_index: 0,
        }
    }
    /// Adds a send operation.
    ///
    /// Returns the index of the added operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s, r) = unbounded::<i32>();
    ///
    /// let mut sel = Select::new();
    /// let index = sel.send(&s);
    /// ```
    pub fn send<T>(&mut self, s: &'a Sender<T>) -> usize {
        let i = self.next_index;
        let ptr = s as *const Sender<_> as *const u8;
        self.handles.push((s, i, ptr));
        self.next_index += 1;
        i
    }
    /// Adds a receive operation.
    ///
    /// Returns the index of the added operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s, r) = unbounded::<i32>();
    ///
    /// let mut sel = Select::new();
    /// let index = sel.recv(&r);
    /// ```
    pub fn recv<T>(&mut self, r: &'a Receiver<T>) -> usize {
        let i = self.next_index;
        let ptr = r as *const Receiver<_> as *const u8;
        self.handles.push((r, i, ptr));
        self.next_index += 1;
        i
    }
    /// Removes a previously added operation.
    ///
    /// This is useful when an operation is selected because the channel got disconnected and we
    /// want to try again to select a different operation instead.
    ///
    /// If new operations are added after removing some, the indices of removed operations will not
    /// be reused.
    ///
    /// # Panics
    ///
    /// An attempt to remove a non-existing or already removed operation will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s1, r1) = unbounded::<i32>();
    /// let (_, r2) = unbounded::<i32>();
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.recv(&r1);
    /// let oper2 = sel.recv(&r2);
    ///
    /// // Both operations are initially ready, so a random one will be executed.
    /// let oper = sel.select();
    /// assert_eq!(oper.index(), oper2);
    /// assert!(oper.recv(&r2).is_err());
    /// sel.remove(oper2);
    ///
    /// s1.send(10).unwrap();
    ///
    /// let oper = sel.select();
    /// assert_eq!(oper.index(), oper1);
    /// assert_eq!(oper.recv(&r1), Ok(10));
    /// ```
    pub fn remove(&mut self, index: usize) {
        assert!(
            index < self.next_index, "index out of bounds; {} >= {}", index, self
            .next_index,
        );
        let i = self
            .handles
            .iter()
            .enumerate()
            .find(|(_, (_, i, _))| *i == index)
            .expect("no operation with this index")
            .0;
        self.handles.swap_remove(i);
    }
    /// Attempts to select one of the operations without blocking.
    ///
    /// If an operation is ready, it is selected and returned. If multiple operations are ready at
    /// the same time, a random one among them is selected. If none of the operations are ready, an
    /// error is returned.
    ///
    /// An operation is considered to be ready if it doesn't have to block. Note that it is ready
    /// even when it will simply return an error because the channel is disconnected.
    ///
    /// The selected operation must be completed with [`SelectedOperation::send`]
    /// or [`SelectedOperation::recv`].
    ///
    /// [`SelectedOperation::send`]: struct.SelectedOperation.html#method.send
    /// [`SelectedOperation::recv`]: struct.SelectedOperation.html#method.recv
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s1, r1) = unbounded();
    /// let (s2, r2) = unbounded();
    ///
    /// s1.send(10).unwrap();
    /// s2.send(20).unwrap();
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.recv(&r1);
    /// let oper2 = sel.recv(&r2);
    ///
    /// // Both operations are initially ready, so a random one will be executed.
    /// let oper = sel.try_select();
    /// match oper {
    ///     Err(_) => panic!("both operations should be ready"),
    ///     Ok(oper) => match oper.index() {
    ///         i if i == oper1 => assert_eq!(oper.recv(&r1), Ok(10)),
    ///         i if i == oper2 => assert_eq!(oper.recv(&r2), Ok(20)),
    ///         _ => unreachable!(),
    ///     }
    /// }
    /// ```
    pub fn try_select(&mut self) -> Result<SelectedOperation<'a>, TrySelectError> {
        try_select(&mut self.handles)
    }
    /// Blocks until one of the operations becomes ready and selects it.
    ///
    /// Once an operation becomes ready, it is selected and returned. If multiple operations are
    /// ready at the same time, a random one among them is selected.
    ///
    /// An operation is considered to be ready if it doesn't have to block. Note that it is ready
    /// even when it will simply return an error because the channel is disconnected.
    ///
    /// The selected operation must be completed with [`SelectedOperation::send`]
    /// or [`SelectedOperation::recv`].
    ///
    /// [`SelectedOperation::send`]: struct.SelectedOperation.html#method.send
    /// [`SelectedOperation::recv`]: struct.SelectedOperation.html#method.recv
    ///
    /// # Panics
    ///
    /// Panics if no operations have been added to `Select`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::thread;
    /// use std::time::Duration;
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s1, r1) = unbounded();
    /// let (s2, r2) = unbounded();
    ///
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_secs(1));
    ///     s1.send(10).unwrap();
    /// });
    /// thread::spawn(move || s2.send(20).unwrap());
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.recv(&r1);
    /// let oper2 = sel.recv(&r2);
    ///
    /// // The second operation will be selected because it becomes ready first.
    /// let oper = sel.select();
    /// match oper.index() {
    ///     i if i == oper1 => assert_eq!(oper.recv(&r1), Ok(10)),
    ///     i if i == oper2 => assert_eq!(oper.recv(&r2), Ok(20)),
    ///     _ => unreachable!(),
    /// }
    /// ```
    pub fn select(&mut self) -> SelectedOperation<'a> {
        select(&mut self.handles)
    }
    /// Blocks for a limited time until one of the operations becomes ready and selects it.
    ///
    /// If an operation becomes ready, it is selected and returned. If multiple operations are
    /// ready at the same time, a random one among them is selected. If none of the operations
    /// become ready for the specified duration, an error is returned.
    ///
    /// An operation is considered to be ready if it doesn't have to block. Note that it is ready
    /// even when it will simply return an error because the channel is disconnected.
    ///
    /// The selected operation must be completed with [`SelectedOperation::send`]
    /// or [`SelectedOperation::recv`].
    ///
    /// [`SelectedOperation::send`]: struct.SelectedOperation.html#method.send
    /// [`SelectedOperation::recv`]: struct.SelectedOperation.html#method.recv
    ///
    /// # Examples
    ///
    /// ```
    /// use std::thread;
    /// use std::time::Duration;
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s1, r1) = unbounded();
    /// let (s2, r2) = unbounded();
    ///
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_secs(1));
    ///     s1.send(10).unwrap();
    /// });
    /// thread::spawn(move || s2.send(20).unwrap());
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.recv(&r1);
    /// let oper2 = sel.recv(&r2);
    ///
    /// // The second operation will be selected because it becomes ready first.
    /// let oper = sel.select_timeout(Duration::from_millis(500));
    /// match oper {
    ///     Err(_) => panic!("should not have timed out"),
    ///     Ok(oper) => match oper.index() {
    ///         i if i == oper1 => assert_eq!(oper.recv(&r1), Ok(10)),
    ///         i if i == oper2 => assert_eq!(oper.recv(&r2), Ok(20)),
    ///         _ => unreachable!(),
    ///     }
    /// }
    /// ```
    pub fn select_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<SelectedOperation<'a>, SelectTimeoutError> {
        select_timeout(&mut self.handles, timeout)
    }
    /// Attempts to find a ready operation without blocking.
    ///
    /// If an operation is ready, its index is returned. If multiple operations are ready at the
    /// same time, a random one among them is chosen. If none of the operations are ready, an error
    /// is returned.
    ///
    /// An operation is considered to be ready if it doesn't have to block. Note that it is ready
    /// even when it will simply return an error because the channel is disconnected.
    ///
    /// Note that this method might return with success spuriously, so it's a good idea to always
    /// double check if the operation is really ready.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s1, r1) = unbounded();
    /// let (s2, r2) = unbounded();
    ///
    /// s1.send(10).unwrap();
    /// s2.send(20).unwrap();
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.recv(&r1);
    /// let oper2 = sel.recv(&r2);
    ///
    /// // Both operations are initially ready, so a random one will be chosen.
    /// match sel.try_ready() {
    ///     Err(_) => panic!("both operations should be ready"),
    ///     Ok(i) if i == oper1 => assert_eq!(r1.try_recv(), Ok(10)),
    ///     Ok(i) if i == oper2 => assert_eq!(r2.try_recv(), Ok(20)),
    ///     Ok(_) => unreachable!(),
    /// }
    /// ```
    pub fn try_ready(&mut self) -> Result<usize, TryReadyError> {
        match run_ready(&mut self.handles, Timeout::Now) {
            None => Err(TryReadyError),
            Some(index) => Ok(index),
        }
    }
    /// Blocks until one of the operations becomes ready.
    ///
    /// Once an operation becomes ready, its index is returned. If multiple operations are ready at
    /// the same time, a random one among them is chosen.
    ///
    /// An operation is considered to be ready if it doesn't have to block. Note that it is ready
    /// even when it will simply return an error because the channel is disconnected.
    ///
    /// Note that this method might return with success spuriously, so it's a good idea to always
    /// double check if the operation is really ready.
    ///
    /// # Panics
    ///
    /// Panics if no operations have been added to `Select`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::thread;
    /// use std::time::Duration;
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s1, r1) = unbounded();
    /// let (s2, r2) = unbounded();
    ///
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_secs(1));
    ///     s1.send(10).unwrap();
    /// });
    /// thread::spawn(move || s2.send(20).unwrap());
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.recv(&r1);
    /// let oper2 = sel.recv(&r2);
    ///
    /// // The second operation will be selected because it becomes ready first.
    /// match sel.ready() {
    ///     i if i == oper1 => assert_eq!(r1.try_recv(), Ok(10)),
    ///     i if i == oper2 => assert_eq!(r2.try_recv(), Ok(20)),
    ///     _ => unreachable!(),
    /// }
    /// ```
    pub fn ready(&mut self) -> usize {
        if self.handles.is_empty() {
            panic!("no operations have been added to `Select`");
        }
        run_ready(&mut self.handles, Timeout::Never).unwrap()
    }
    /// Blocks for a limited time until one of the operations becomes ready.
    ///
    /// If an operation becomes ready, its index is returned. If multiple operations are ready at
    /// the same time, a random one among them is chosen. If none of the operations become ready
    /// for the specified duration, an error is returned.
    ///
    /// An operation is considered to be ready if it doesn't have to block. Note that it is ready
    /// even when it will simply return an error because the channel is disconnected.
    ///
    /// Note that this method might return with success spuriously, so it's a good idea to double
    /// check if the operation is really ready.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::thread;
    /// use std::time::Duration;
    /// use crossbeam_channel::{unbounded, Select};
    ///
    /// let (s1, r1) = unbounded();
    /// let (s2, r2) = unbounded();
    ///
    /// thread::spawn(move || {
    ///     thread::sleep(Duration::from_secs(1));
    ///     s1.send(10).unwrap();
    /// });
    /// thread::spawn(move || s2.send(20).unwrap());
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.recv(&r1);
    /// let oper2 = sel.recv(&r2);
    ///
    /// // The second operation will be selected because it becomes ready first.
    /// match sel.ready_timeout(Duration::from_millis(500)) {
    ///     Err(_) => panic!("should not have timed out"),
    ///     Ok(i) if i == oper1 => assert_eq!(r1.try_recv(), Ok(10)),
    ///     Ok(i) if i == oper2 => assert_eq!(r2.try_recv(), Ok(20)),
    ///     Ok(_) => unreachable!(),
    /// }
    /// ```
    pub fn ready_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<usize, ReadyTimeoutError> {
        let timeout = Timeout::At(Instant::now() + timeout);
        match run_ready(&mut self.handles, timeout) {
            None => Err(ReadyTimeoutError),
            Some(index) => Ok(index),
        }
    }
}
impl<'a> Clone for Select<'a> {
    fn clone(&self) -> Select<'a> {
        Select {
            handles: self.handles.clone(),
            next_index: self.next_index,
        }
    }
}
impl<'a> Default for Select<'a> {
    fn default() -> Select<'a> {
        Select::new()
    }
}
impl fmt::Debug for Select<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Select { .. }")
    }
}
/// A selected operation that needs to be completed.
///
/// To complete the operation, call [`send`] or [`recv`].
///
/// # Panics
///
/// Forgetting to complete the operation is an error and might lead to deadlocks. If a
/// `SelectedOperation` is dropped without completion, a panic occurs.
///
/// [`send`]: struct.SelectedOperation.html#method.send
/// [`recv`]: struct.SelectedOperation.html#method.recv
#[must_use]
pub struct SelectedOperation<'a> {
    /// Token needed to complete the operation.
    token: Token,
    /// The index of the selected operation.
    index: usize,
    /// The address of the selected `Sender` or `Receiver`.
    ptr: *const u8,
    /// Indicates that `Sender`s and `Receiver`s are borrowed.
    _marker: PhantomData<&'a ()>,
}
impl SelectedOperation<'_> {
    /// Returns the index of the selected operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::{bounded, Select};
    ///
    /// let (s1, r1) = bounded::<()>(0);
    /// let (s2, r2) = bounded::<()>(0);
    /// let (s3, r3) = bounded::<()>(1);
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.send(&s1);
    /// let oper2 = sel.recv(&r2);
    /// let oper3 = sel.send(&s3);
    ///
    /// // Only the last operation is ready.
    /// let oper = sel.select();
    /// assert_eq!(oper.index(), 2);
    /// assert_eq!(oper.index(), oper3);
    ///
    /// // Complete the operation.
    /// oper.send(&s3, ()).unwrap();
    /// ```
    pub fn index(&self) -> usize {
        self.index
    }
    /// Completes the send operation.
    ///
    /// The passed [`Sender`] reference must be the same one that was used in [`Select::send`]
    /// when the operation was added.
    ///
    /// # Panics
    ///
    /// Panics if an incorrect [`Sender`] reference is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::{bounded, Select, SendError};
    ///
    /// let (s, r) = bounded::<i32>(0);
    /// drop(r);
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.send(&s);
    ///
    /// let oper = sel.select();
    /// assert_eq!(oper.index(), oper1);
    /// assert_eq!(oper.send(&s, 10), Err(SendError(10)));
    /// ```
    ///
    /// [`Sender`]: struct.Sender.html
    /// [`Select::send`]: struct.Select.html#method.send
    pub fn send<T>(mut self, s: &Sender<T>, msg: T) -> Result<(), SendError<T>> {
        assert!(
            s as * const Sender < T > as * const u8 == self.ptr,
            "passed a sender that wasn't selected",
        );
        let res = unsafe { channel::write(s, &mut self.token, msg) };
        mem::forget(self);
        res.map_err(SendError)
    }
    /// Completes the receive operation.
    ///
    /// The passed [`Receiver`] reference must be the same one that was used in [`Select::recv`]
    /// when the operation was added.
    ///
    /// # Panics
    ///
    /// Panics if an incorrect [`Receiver`] reference is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_channel::{bounded, Select, RecvError};
    ///
    /// let (s, r) = bounded::<i32>(0);
    /// drop(s);
    ///
    /// let mut sel = Select::new();
    /// let oper1 = sel.recv(&r);
    ///
    /// let oper = sel.select();
    /// assert_eq!(oper.index(), oper1);
    /// assert_eq!(oper.recv(&r), Err(RecvError));
    /// ```
    ///
    /// [`Receiver`]: struct.Receiver.html
    /// [`Select::recv`]: struct.Select.html#method.recv
    pub fn recv<T>(mut self, r: &Receiver<T>) -> Result<T, RecvError> {
        assert!(
            r as * const Receiver < T > as * const u8 == self.ptr,
            "passed a receiver that wasn't selected",
        );
        let res = unsafe { channel::read(r, &mut self.token) };
        mem::forget(self);
        res.map_err(|_| RecvError)
    }
}
impl fmt::Debug for SelectedOperation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("SelectedOperation { .. }")
    }
}
impl Drop for SelectedOperation<'_> {
    fn drop(&mut self) {
        panic!("dropped `SelectedOperation` without completing the operation");
    }
}
#[cfg(test)]
mod tests_rug_62 {
    use super::*;
    use crate::{select::run_select, select::{SelectHandle, Timeout}};
    #[test]
    fn test_run_select() {
        let _rug_st_tests_rug_62_rrrruuuugggg_test_run_select = 0;
        let mut p0: Vec<(&dyn SelectHandle, usize, *const u8)> = vec![];
        let p1 = Timeout::Now;
        run_select(&mut p0, p1);
        let _rug_ed_tests_rug_62_rrrruuuugggg_test_run_select = 0;
    }
}
#[cfg(test)]
mod tests_rug_63 {
    use super::*;
    use crate::select::{run_ready, SelectHandle, Timeout};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_63_rrrruuuugggg_test_rug = 0;
        let mut p0: Vec<(&dyn SelectHandle, usize, *const u8)> = Vec::new();
        let p1: Timeout = Timeout::Now;
        run_ready(&mut p0, p1);
        let _rug_ed_tests_rug_63_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_67 {
    use super::*;
    use crate::select::Operation;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: _ = rug_fuzz_0 as usize;
        Operation::hook(&mut p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_68 {
    use super::*;
    use crate::select::{Selected, Operation};
    #[test]
    fn test_from() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: usize = rug_fuzz_0;
        debug_assert_eq!(Selected::from(p0), Selected::Aborted);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_70 {
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
mod tests_rug_71 {
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
mod tests_rug_75 {
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
        debug_assert_eq!(p0.is_ready(), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_78 {
    use super::*;
    use crate::Select;
    #[test]
    fn test_new() {
        let _rug_st_tests_rug_78_rrrruuuugggg_test_new = 0;
        let _sel: Select<'static> = Select::new();
        let _rug_ed_tests_rug_78_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_79 {
    use super::*;
    use crate::{unbounded, Select};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_79_rrrruuuugggg_test_rug = 0;
        let (s, _r) = unbounded::<i32>();
        let mut sel = Select::new();
        let p0 = &mut sel;
        let p1 = &s;
        <Select<'_>>::send(p0, p1);
        let _rug_ed_tests_rug_79_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_80 {
    use super::*;
    use crate::{unbounded, Receiver, Select};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_80_rrrruuuugggg_test_rug = 0;
        let (s, r) = unbounded::<i32>();
        let mut sel = Select::new();
        let mut sel_ref = &mut sel;
        let recv_index = sel_ref.recv::<i32>(&r);
        let _rug_ed_tests_rug_80_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_81 {
    use super::*;
    use crate::{unbounded, Select};
    #[test]
    fn test_remove() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut sel = Select::new();
        let (s1, r1) = unbounded::<i32>();
        let (_, r2) = unbounded::<i32>();
        let oper1 = sel.recv(&r1);
        let oper2 = sel.recv(&r2);
        let index_to_remove = rug_fuzz_0;
        sel.remove(index_to_remove);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_82 {
    use super::*;
    use crate::{unbounded, Select};
    #[test]
    fn test_try_select() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut sel = Select::new();
        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();
        s1.send(rug_fuzz_0).unwrap();
        s2.send(rug_fuzz_1).unwrap();
        let oper1 = sel.recv(&r1);
        let oper2 = sel.recv(&r2);
        let oper = sel.try_select();
        match oper {
            Err(_) => panic!("both operations should be ready"),
            Ok(oper) => {
                match oper.index() {
                    i if i == oper1 => debug_assert_eq!(oper.recv(& r1), Ok(10)),
                    i if i == oper2 => debug_assert_eq!(oper.recv(& r2), Ok(20)),
                    _ => unreachable!(),
                }
            }
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_83 {
    use super::*;
    use crate::{unbounded, Select};
    #[test]
    fn test_select() {
        let _rug_st_tests_rug_83_rrrruuuugggg_test_select = 0;
        let mut sel = Select::new();
        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();
        let oper1 = sel.recv(&r1);
        let oper2 = sel.recv(&r2);
        let oper = sel.select();
        match oper.index() {
            i if i == oper1 => debug_assert_eq!(oper.recv(& r1), Ok(10)),
            i if i == oper2 => debug_assert_eq!(oper.recv(& r2), Ok(20)),
            _ => unreachable!(),
        }
        let _rug_ed_tests_rug_83_rrrruuuugggg_test_select = 0;
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use std::time::Duration;
    use crate::{unbounded, Select};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Select::new();
        let mut p1 = Duration::from_secs(rug_fuzz_0);
        p0.select_timeout(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_85 {
    use super::*;
    use crate::{unbounded, Select};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_85_rrrruuuugggg_test_rug = 0;
        let mut sel = Select::new();
        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();
        sel.recv(&r1);
        sel.recv(&r2);
        match sel.try_ready() {
            Err(_) => panic!("both operations should be ready"),
            Ok(i) => {
                if i == sel.recv(&r1) {
                    debug_assert_eq!(r1.try_recv(), Ok(10));
                } else if i == sel.recv(&r2) {
                    debug_assert_eq!(r2.try_recv(), Ok(20));
                } else {
                    unreachable!();
                }
            }
        }
        let _rug_ed_tests_rug_85_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_86 {
    use super::*;
    use crate::{unbounded, Select};
    #[test]
    fn test_ready() {
        let _rug_st_tests_rug_86_rrrruuuugggg_test_ready = 0;
        let mut sel = Select::new();
        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();
        let oper1 = sel.recv(&r1);
        let oper2 = sel.recv(&r2);
        match sel.ready() {
            i if i == oper1 => debug_assert_eq!(r1.try_recv(), Ok(10)),
            i if i == oper2 => debug_assert_eq!(r2.try_recv(), Ok(20)),
            _ => unreachable!(),
        }
        let _rug_ed_tests_rug_86_rrrruuuugggg_test_ready = 0;
    }
}
#[cfg(test)]
mod tests_rug_87 {
    use super::*;
    use crate::{Select, ReadyTimeoutError};
    use std::thread;
    use std::time::{Duration, Instant};
    use crate::unbounded;
    #[test]
    fn test_ready_timeout() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u64, i32, i32, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(rug_fuzz_0));
            s1.send(rug_fuzz_1).unwrap();
        });
        thread::spawn(move || s2.send(rug_fuzz_2).unwrap());
        let mut sel = Select::new();
        let oper1 = sel.recv(&r1);
        let oper2 = sel.recv(&r2);
        match sel.ready_timeout(Duration::from_millis(rug_fuzz_3)) {
            Err(_) => panic!("should not have timed out"),
            Ok(i) if i == oper1 => debug_assert_eq!(r1.try_recv(), Ok(10)),
            Ok(i) if i == oper2 => debug_assert_eq!(r2.try_recv(), Ok(20)),
            Ok(_) => unreachable!(),
        }
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_89 {
    use super::*;
    use crate::select::Select;
    use std::default::Default;
    #[test]
    fn test_default_select() {
        let _rug_st_tests_rug_89_rrrruuuugggg_test_default_select = 0;
        let default_select: Select = <Select>::default();
        let _rug_ed_tests_rug_89_rrrruuuugggg_test_default_select = 0;
    }
}
#[cfg(test)]
mod tests_rug_90 {
    use super::*;
    use crate::{bounded, Select};
    #[test]
    fn test_index() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let (s1, r1) = bounded::<()>(rug_fuzz_0);
        let (s2, r2) = bounded::<()>(rug_fuzz_1);
        let (s3, r3) = bounded::<()>(rug_fuzz_2);
        let mut sel = Select::new();
        let oper1 = sel.send(&s1);
        let oper2 = sel.recv(&r2);
        let oper3 = sel.send(&s3);
        let oper = sel.select();
        debug_assert_eq!(oper.index(), 2);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_91 {
    use super::*;
    use crate::{bounded, Select, SendError, channel};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut s = bounded::<i32>(rug_fuzz_0).0;
        let mut sel = Select::new();
        let oper1 = sel.send(&s);
        let oper = sel.select();
        debug_assert_eq!(oper.index(), oper1);
        debug_assert_eq!(oper.send(& s, rug_fuzz_1), Err(SendError(10)));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_92 {
    use super::*;
    use crate::{bounded, Select, RecvError};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let (s, r) = bounded::<i32>(rug_fuzz_0);
        drop(s);
        let mut sel = Select::new();
        let oper1 = sel.recv(&r);
        let oper = sel.select();
        debug_assert_eq!(oper.index(), oper1);
        debug_assert_eq!(oper.recv(& r), Err(RecvError));
             }
}
}
}    }
}
