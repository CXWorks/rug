pub(crate) use self::lock::Lock;
#[cfg(threadsafe)]
mod lock {
    use std::sync::{Arc, Mutex, MutexGuard};
    /// Thread safe lock: just wraps a `Mutex`.
    pub(crate) struct Lock<A> {
        lock: Arc<Mutex<A>>,
    }
    impl<A> Lock<A> {
        pub(crate) fn new(value: A) -> Self {
            Lock {
                lock: Arc::new(Mutex::new(value)),
            }
        }
        #[inline]
        pub(crate) fn lock(&mut self) -> Option<MutexGuard<'_, A>> {
            self.lock.lock().ok()
        }
    }
    impl<A> Clone for Lock<A> {
        fn clone(&self) -> Self {
            Lock { lock: self.lock.clone() }
        }
    }
}
#[cfg(not(threadsafe))]
mod lock {
    use std::cell::{RefCell, RefMut};
    use std::rc::Rc;
    /// Single threaded lock: a `RefCell` so we should safely panic if somehow
    /// trying to access the stored data twice from the same thread.
    pub(crate) struct Lock<A> {
        lock: Rc<RefCell<A>>,
    }
    impl<A> Lock<A> {
        pub(crate) fn new(value: A) -> Self {
            Lock {
                lock: Rc::new(RefCell::new(value)),
            }
        }
        #[inline]
        pub(crate) fn lock(&mut self) -> Option<RefMut<'_, A>> {
            self.lock.try_borrow_mut().ok()
        }
    }
    impl<A> Clone for Lock<A> {
        fn clone(&self) -> Self {
            Lock { lock: self.lock.clone() }
        }
    }
}
#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use crate::sync::lock::Lock;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: i32 = rug_fuzz_0;
        Lock::<i32>::new(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_319 {
    use super::*;
    use crate::sync::lock::Lock;
    #[test]
    fn test_lock() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Lock<u32> = Lock::new(rug_fuzz_0);
        p0.lock();
             }
});    }
}
