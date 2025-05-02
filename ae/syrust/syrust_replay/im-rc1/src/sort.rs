use crate::vector::FocusMut;
use rand_core::{RngCore, SeedableRng};
use std::cmp::Ordering;
use std::mem;
fn gen_range<R: RngCore>(rng: &mut R, min: usize, max: usize) -> usize {
    let range = max - min;
    min + (rng.next_u64() as usize % range)
}
fn do_quicksort<A, F, R>(vector: FocusMut<'_, A>, cmp: &F, rng: &mut R)
where
    A: Clone,
    F: Fn(&A, &A) -> Ordering,
    R: RngCore,
{
    if vector.len() <= 1 {
        return;
    }
    let pivot_index = gen_range(rng, 0, vector.len());
    let (mut first, mut rest) = vector.split_at(1);
    if pivot_index > 0 {
        mem::swap(rest.index_mut(pivot_index - 1), first.index_mut(0));
    }
    let pivot_item = first.index(0);
    let mut less_count = 0;
    let mut equal_count = 0;
    for index in 0..rest.len() {
        let item = rest.index(index);
        let comp = cmp(item, pivot_item);
        match comp {
            Ordering::Less => less_count += 1,
            Ordering::Equal => equal_count += 1,
            Ordering::Greater => {}
        }
    }
    if less_count == 0 {
        do_quicksort(rest, cmp, rng);
        return;
    }
    less_count -= 1;
    equal_count += 1;
    let first_item = first.index_mut(0);
    mem::swap(first_item, rest.index_mut(less_count));
    for index in 0..rest.len() {
        if index == less_count {
            continue;
        }
        let rest_item = rest.index_mut(index);
        if cmp(rest_item, first_item) == Ordering::Less {
            mem::swap(first_item, rest_item);
        }
    }
    let (remaining, mut greater_focus) = rest.split_at(less_count + equal_count);
    let (mut less_focus, mut equal_focus) = remaining.split_at(less_count);
    let mut less_position = 0;
    let mut equal_position = 0;
    let mut greater_position = 0;
    while less_position != less_focus.len() || greater_position != greater_focus.len() {
        let mut equal_swap_side = None;
        let equal_item = equal_focus.index(equal_position);
        while less_position != less_focus.len() {
            let less_item = less_focus.index(less_position);
            match cmp(less_item, equal_item) {
                Ordering::Equal => {
                    equal_swap_side = Some(Ordering::Less);
                    break;
                }
                Ordering::Greater => {
                    break;
                }
                _ => {}
            }
            less_position += 1;
        }
        while greater_position != greater_focus.len() {
            let greater_item = greater_focus.index(greater_position);
            match cmp(greater_item, equal_item) {
                Ordering::Less => break,
                Ordering::Equal => {
                    equal_swap_side = Some(Ordering::Greater);
                    break;
                }
                _ => {}
            }
            greater_position += 1;
        }
        if let Some(swap_side) = equal_swap_side {
            let item = if swap_side == Ordering::Less {
                less_focus.index_mut(less_position)
            } else {
                greater_focus.index_mut(greater_position)
            };
            while cmp(item, equal_focus.index(equal_position)) == Ordering::Equal {
                equal_position += 1;
            }
            mem::swap(item, equal_focus.index_mut(equal_position));
        } else if less_position != less_focus.len()
            && greater_position != greater_focus.len()
        {
            debug_assert_ne!(
                cmp(less_focus.index(less_position), equal_focus.index(equal_position)),
                Ordering::Equal
            );
            debug_assert_ne!(
                cmp(greater_focus.index(greater_position), equal_focus
                .index(equal_position)), Ordering::Equal
            );
            mem::swap(
                less_focus.index_mut(less_position),
                greater_focus.index_mut(greater_position),
            );
            less_position += 1;
            greater_position += 1;
        }
    }
    do_quicksort(less_focus, cmp, rng);
    if !greater_focus.is_empty() {
        do_quicksort(greater_focus, cmp, rng);
    }
}
pub(crate) fn quicksort<A, F>(vector: FocusMut<'_, A>, cmp: &F)
where
    A: Clone,
    F: Fn(&A, &A) -> Ordering,
{
    let mut rng = rand_xoshiro::Xoshiro256Plus::seed_from_u64(0);
    do_quicksort(vector, cmp, &mut rng);
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::test::is_sorted;
    use crate::vector::proptest::vector;
    use ::proptest::num::i32;
    use ::proptest::proptest;
    proptest! {
        #[test] fn test_quicksort(ref input in vector(i32::ANY, 0..10000)) { let mut vec
        = input.clone(); let len = vec.len(); if len > 1 { quicksort(vec.focus_mut(), &
        Ord::cmp); } assert!(is_sorted(vec)); }
    }
}
#[cfg(test)]
mod tests_rug_36 {
    use super::*;
    use crate::ordmap::OrdMap;
    use rand::prelude::*;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(u64, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: std::boxed::Box<StdRng> = Box::new(
            StdRng::seed_from_u64(rug_fuzz_0),
        );
        let p1: usize = rug_fuzz_1;
        let p2: usize = rug_fuzz_2;
        crate::sort::gen_range(&mut *p0, p1, p2);
             }
}
}
}    }
}
