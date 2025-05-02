use crate::event::Event;
use crate::sys;
use std::fmt;
/// A collection of readiness events.
///
/// `Events` is passed as an argument to [`Poll::poll`] and will be used to
/// receive any new readiness events received since the last poll. Usually, a
/// single `Events` instance is created at the same time as a [`Poll`] and
/// reused on each call to [`Poll::poll`].
///
/// See [`Poll`] for more documentation on polling.
///
/// [`Poll::poll`]: ../struct.Poll.html#method.poll
/// [`Poll`]: ../struct.Poll.html
///
/// # Examples
///
#[cfg_attr(feature = "os-poll", doc = "```")]
#[cfg_attr(not(feature = "os-poll"), doc = "```ignore")]
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use mio::{Events, Poll};
/// use std::time::Duration;
///
/// let mut events = Events::with_capacity(1024);
/// let mut poll = Poll::new()?;
/// #
/// # assert!(events.is_empty());
///
/// // Register `event::Source`s with `poll`.
///
/// poll.poll(&mut events, Some(Duration::from_millis(100)))?;
///
/// for event in events.iter() {
///     println!("Got an event for {:?}", event.token());
/// }
/// #     Ok(())
/// # }
/// ```
pub struct Events {
    inner: sys::Events,
}
/// [`Events`] iterator.
///
/// This struct is created by the [`iter`] method on [`Events`].
///
/// [`Events`]: struct.Events.html
/// [`iter`]: struct.Events.html#method.iter
///
/// # Examples
///
#[cfg_attr(feature = "os-poll", doc = "```")]
#[cfg_attr(not(feature = "os-poll"), doc = "```ignore")]
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use mio::{Events, Poll};
/// use std::time::Duration;
///
/// let mut events = Events::with_capacity(1024);
/// let mut poll = Poll::new()?;
///
/// // Register handles with `poll`.
///
/// poll.poll(&mut events, Some(Duration::from_millis(100)))?;
///
/// for event in events.iter() {
///     println!("Got an event for {:?}", event.token());
/// }
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Iter<'a> {
    inner: &'a Events,
    pos: usize,
}
impl Events {
    /// Return a new `Events` capable of holding up to `capacity` events.
    ///
    /// # Examples
    ///
    /// ```
    /// use mio::Events;
    ///
    /// let events = Events::with_capacity(1024);
    /// assert_eq!(1024, events.capacity());
    /// ```
    pub fn with_capacity(capacity: usize) -> Events {
        Events {
            inner: sys::Events::with_capacity(capacity),
        }
    }
    /// Returns the number of `Event` values that `self` can hold.
    ///
    /// ```
    /// use mio::Events;
    ///
    /// let events = Events::with_capacity(1024);
    /// assert_eq!(1024, events.capacity());
    /// ```
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }
    /// Returns `true` if `self` contains no `Event` values.
    ///
    /// # Examples
    ///
    /// ```
    /// use mio::Events;
    ///
    /// let events = Events::with_capacity(1024);
    /// assert!(events.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    /// Returns an iterator over the `Event` values.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "os-poll", doc = "```")]
    #[cfg_attr(not(feature = "os-poll"), doc = "```ignore")]
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use mio::{Events, Poll};
    /// use std::time::Duration;
    ///
    /// let mut events = Events::with_capacity(1024);
    /// let mut poll = Poll::new()?;
    ///
    /// // Register handles with `poll`.
    ///
    /// poll.poll(&mut events, Some(Duration::from_millis(100)))?;
    ///
    /// for event in events.iter() {
    ///     println!("Got an event for {:?}", event.token());
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    pub fn iter(&self) -> Iter<'_> {
        Iter { inner: self, pos: 0 }
    }
    /// Clearing all `Event` values from container explicitly.
    ///
    /// # Notes
    ///
    /// Events are cleared before every `poll`, so it is not required to call
    /// this manually.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "os-poll", doc = "```")]
    #[cfg_attr(not(feature = "os-poll"), doc = "```ignore")]
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use mio::{Events, Poll};
    /// use std::time::Duration;
    ///
    /// let mut events = Events::with_capacity(1024);
    /// let mut poll = Poll::new()?;
    ///
    /// // Register handles with `poll`.
    ///
    /// poll.poll(&mut events, Some(Duration::from_millis(100)))?;
    ///
    /// // Clear all events.
    /// events.clear();
    /// assert!(events.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    /// Returns the inner `sys::Events`.
    pub(crate) fn sys(&mut self) -> &mut sys::Events {
        &mut self.inner
    }
}
impl<'a> IntoIterator for &'a Events {
    type Item = &'a Event;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a> Iterator for Iter<'a> {
    type Item = &'a Event;
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.inner.inner.get(self.pos).map(Event::from_sys_event_ref);
        self.pos += 1;
        ret
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.inner.inner.len();
        (size, Some(size))
    }
    fn count(self) -> usize {
        self.inner.inner.len()
    }
}
impl fmt::Debug for Events {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::Events;
    #[test]
    fn test_with_capacity() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_with_capacity = 0;
        let rug_fuzz_0 = 1024;
        let p0: usize = rug_fuzz_0;
        let events = <Events>::with_capacity(p0);
        debug_assert_eq!(p0, events.capacity());
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_with_capacity = 0;
    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::event::events::Events;
    #[test]
    fn test_capacity() {
        let _rug_st_tests_rug_53_rrrruuuugggg_test_capacity = 0;
        let rug_fuzz_0 = 1024;
        let mut p0 = Events::with_capacity(rug_fuzz_0);
        Events::capacity(&mut p0);
        let _rug_ed_tests_rug_53_rrrruuuugggg_test_capacity = 0;
    }
}
#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use crate::Events;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_54_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 1024;
        let mut p0 = Events::with_capacity(rug_fuzz_0);
        <Events>::is_empty(&p0);
        let _rug_ed_tests_rug_54_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_55 {
    use super::*;
    use crate::Events;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_55_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = Events::with_capacity(rug_fuzz_0);
        crate::event::events::Events::iter(&p0);
        let _rug_ed_tests_rug_55_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_56 {
    use super::*;
    use crate::Events;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_56_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = Events::with_capacity(rug_fuzz_0);
        crate::event::events::Events::clear(&mut p0);
        let _rug_ed_tests_rug_56_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_57 {
    use super::*;
    use crate::Events;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_57_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = 10;
        #[cfg(test)]
        mod tests_rug_57_prepare {
            use crate::Events;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_57_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 10;
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_57_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v10 = Events::with_capacity(rug_fuzz_0);
                let _rug_ed_tests_rug_57_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_57_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0 = Events::with_capacity(10);
        crate::event::events::Events::sys(&mut p0);
        let _rug_ed_tests_rug_57_rrrruuuugggg_sample = 0;
    }
}
