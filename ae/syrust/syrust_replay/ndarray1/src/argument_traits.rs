use std::cell::Cell;
use std::mem::MaybeUninit;
/// A producer element that can be assigned to once
pub trait AssignElem<T> {
    /// Assign the value `input` to the element that self represents.
    fn assign_elem(self, input: T);
}
/// Assignable element, simply `*self = input`.
impl<'a, T> AssignElem<T> for &'a mut T {
    fn assign_elem(self, input: T) {
        *self = input;
    }
}
/// Assignable element, simply `self.set(input)`.
impl<'a, T> AssignElem<T> for &'a Cell<T> {
    fn assign_elem(self, input: T) {
        self.set(input);
    }
}
/// Assignable element, the item in the MaybeUninit is overwritten (prior value, if any, is not
/// read or dropped).
impl<'a, T> AssignElem<T> for &'a mut MaybeUninit<T> {
    fn assign_elem(self, input: T) {
        *self = MaybeUninit::new(input);
    }
}
#[cfg(test)]
mod tests_rug_448 {
    use super::*;
    use crate::AssignElem;
    use crate::Array;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Array::from_vec(vec![rug_fuzz_0, 2, 3]);
        let mut p1 = Array::from_vec(vec![rug_fuzz_1, 5, 6]);
        p0.assign_elem(p1);
        debug_assert_eq!(p0, Array::from_vec(vec![4, 5, 6]));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_449 {
    use crate::AssignElem;
    use std::cell::Cell;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Cell::new(rug_fuzz_0);
        let p1 = rug_fuzz_1;
        p0.assign_elem(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_450 {
    use super::*;
    use crate::AssignElem;
    use std::mem::MaybeUninit;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: MaybeUninit<i32> = MaybeUninit::<i32>::uninit();
        let p1: i32 = rug_fuzz_0;
        p0.assign_elem(p1);
             }
}
}
}    }
}
