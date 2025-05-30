use alloc::boxed::Box;
use core::fmt;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ptr;
/// Number of words a piece of `Data` can hold.
///
/// Three words should be enough for the majority of cases. For example, you can fit inside it the
/// function pointer together with a fat pointer representing an object that needs to be destroyed.
const DATA_WORDS: usize = 3;
/// Some space to keep a `FnOnce()` object on the stack.
type Data = [usize; DATA_WORDS];
/// A `FnOnce()` that is stored inline if small, or otherwise boxed on the heap.
///
/// This is a handy way of keeping an unsized `FnOnce()` within a sized structure.
pub struct Deferred {
    call: unsafe fn(*mut u8),
    data: Data,
    _marker: PhantomData<*mut ()>,
}
impl fmt::Debug for Deferred {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.pad("Deferred { .. }")
    }
}
impl Deferred {
    /// Constructs a new `Deferred` from a `FnOnce()`.
    pub fn new<F: FnOnce()>(f: F) -> Self {
        let size = mem::size_of::<F>();
        let align = mem::align_of::<F>();
        unsafe {
            if size <= mem::size_of::<Data>() && align <= mem::align_of::<Data>() {
                let mut data = MaybeUninit::<Data>::uninit();
                ptr::write(data.as_mut_ptr() as *mut F, f);
                unsafe fn call<F: FnOnce()>(raw: *mut u8) {
                    let f: F = ptr::read(raw as *mut F);
                    f();
                }
                Deferred {
                    call: call::<F>,
                    data: data.assume_init(),
                    _marker: PhantomData,
                }
            } else {
                let b: Box<F> = Box::new(f);
                let mut data = MaybeUninit::<Data>::uninit();
                ptr::write(data.as_mut_ptr() as *mut Box<F>, b);
                unsafe fn call<F: FnOnce()>(raw: *mut u8) {
                    #[allow(clippy::cast_ptr_alignment)]
                    let b: Box<F> = ptr::read(raw as *mut Box<F>);
                    (*b)();
                }
                Deferred {
                    call: call::<F>,
                    data: data.assume_init(),
                    _marker: PhantomData,
                }
            }
        }
    }
    /// Calls the function.
    #[inline]
    pub fn call(mut self) {
        let call = self.call;
        unsafe { call(&mut self.data as *mut Data as *mut u8) };
    }
}
#[cfg(test)]
mod tests {
    use super::Deferred;
    use std::cell::Cell;
    #[test]
    fn on_stack() {
        let fired = &Cell::new(false);
        let a = [0usize; 1];
        let d = Deferred::new(move || {
            drop(a);
            fired.set(true);
        });
        assert!(! fired.get());
        d.call();
        assert!(fired.get());
    }
    #[test]
    fn on_heap() {
        let fired = &Cell::new(false);
        let a = [0usize; 10];
        let d = Deferred::new(move || {
            drop(a);
            fired.set(true);
        });
        assert!(! fired.get());
        d.call();
        assert!(fired.get());
    }
    #[test]
    fn string() {
        let a = "hello".to_string();
        let d = Deferred::new(move || assert_eq!(a, "hello"));
        d.call();
    }
    #[test]
    fn boxed_slice_i32() {
        let a: Box<[i32]> = vec![2, 3, 5, 7].into_boxed_slice();
        let d = Deferred::new(move || assert_eq!(* a, [2, 3, 5, 7]));
        d.call();
    }
    #[test]
    fn long_slice_usize() {
        let a: [usize; 5] = [2, 3, 5, 7, 11];
        let d = Deferred::new(move || assert_eq!(a, [2, 3, 5, 7, 11]));
        d.call();
    }
}
#[cfg(test)]
mod tests_rug_420 {
    use super::*;
    use std::boxed::Box;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_420_rrrruuuugggg_test_rug = 0;
        #[derive(Debug)]
        struct MyStruct;
        let p0: Box<dyn FnOnce()> = Box::new(|| {
            println!("Hello from p0!");
        });
        crate::deferred::Deferred::new(p0);
        let _rug_ed_tests_rug_420_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_421 {
    use super::*;
    use std::mem;
    use std::ptr;
    use std::mem::MaybeUninit;
    use std::marker::PhantomData;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_421_rrrruuuugggg_test_rug = 0;
        let mut p0 = crate::deferred::Deferred::new(|| {
            println!("Testing call function");
        });
        crate::deferred::Deferred::call(p0);
        let _rug_ed_tests_rug_421_rrrruuuugggg_test_rug = 0;
    }
}
