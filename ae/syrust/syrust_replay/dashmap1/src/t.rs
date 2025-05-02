//! Central map trait to ease modifications and extensions down the road.
use crate::iter::{Iter, IterMut};
use crate::lock::{RwLockReadGuard, RwLockWriteGuard};
use crate::mapref::entry::Entry;
use crate::mapref::one::{Ref, RefMut};
use crate::HashMap;
use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
/// Implementation detail that is exposed due to generic constraints in public types.
pub trait Map<'a, K: 'a + Eq + Hash, V: 'a, S: 'a + Clone + BuildHasher> {
    fn _shard_count(&self) -> usize;
    /// # Safety
    ///
    /// The index must not be out of bounds.
    unsafe fn _get_read_shard(&'a self, i: usize) -> &'a HashMap<K, V, S>;
    /// # Safety
    ///
    /// The index must not be out of bounds.
    unsafe fn _yield_read_shard(
        &'a self,
        i: usize,
    ) -> RwLockReadGuard<'a, HashMap<K, V, S>>;
    /// # Safety
    ///
    /// The index must not be out of bounds.
    unsafe fn _yield_write_shard(
        &'a self,
        i: usize,
    ) -> RwLockWriteGuard<'a, HashMap<K, V, S>>;
    fn _insert(&self, key: K, value: V) -> Option<V>;
    fn _remove<Q>(&self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    fn _remove_if<Q>(&self, key: &Q, f: impl FnOnce(&K, &V) -> bool) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    fn _iter(&'a self) -> Iter<'a, K, V, S, Self>
    where
        Self: Sized;
    fn _iter_mut(&'a self) -> IterMut<'a, K, V, S, Self>
    where
        Self: Sized;
    fn _get<Q>(&'a self, key: &Q) -> Option<Ref<'a, K, V, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    fn _get_mut<Q>(&'a self, key: &Q) -> Option<RefMut<'a, K, V, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    fn _shrink_to_fit(&self);
    fn _retain(&self, f: impl FnMut(&K, &mut V) -> bool);
    fn _len(&self) -> usize;
    fn _capacity(&self) -> usize;
    fn _alter<Q>(&self, key: &Q, f: impl FnOnce(&K, V) -> V)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    fn _alter_all(&self, f: impl FnMut(&K, V) -> V);
    fn _entry(&'a self, key: K) -> Entry<'a, K, V, S>;
    fn _hasher(&self) -> S;
    fn _clear(&self) {
        self._retain(|_, _| false)
    }
    fn _contains_key<Q>(&'a self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self._get(key).is_some()
    }
    fn _is_empty(&self) -> bool {
        self._len() == 0
    }
}
#[cfg(test)]
mod tests_rug_97 {
    use super::*;
    use crate::{DashMap, RandomState, ReadOnlyView};
    use std::collections::HashMap;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: DashMap<i32, ReadOnlyView<i32, String>> = DashMap::new();
        p0.insert(rug_fuzz_0, ReadOnlyView::new(DashMap::new()));
        crate::t::Map::_clear(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use crate::{DashMap, ReadOnlyView, RandomState};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(i32, i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: DashMap<i32, i32, RandomState> = DashMap::new();
        p0.insert(rug_fuzz_0, rug_fuzz_1);
        let p1: i32 = rug_fuzz_2;
        debug_assert!(p0._contains_key(& p1));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::{DashMap, ReadOnlyView, RandomState};
    #[test]
    fn test_is_empty_true() {
        let _rug_st_tests_rug_99_rrrruuuugggg_test_is_empty_true = 0;
        let mut map: DashMap<u32, &str, RandomState> = DashMap::new();
        debug_assert!(map.is_empty());
        let _rug_ed_tests_rug_99_rrrruuuugggg_test_is_empty_true = 0;
    }
    #[test]
    fn test_is_empty_false() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u32, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut map: DashMap<u32, &str, RandomState> = DashMap::new();
        map.insert(rug_fuzz_0, rug_fuzz_1);
        debug_assert!(! map.is_empty());
             }
}
}
}    }
}
