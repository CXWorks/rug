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

    unsafe fn _yield_read_shard(&'a self, i: usize) -> RwLockReadGuard<'a, HashMap<K, V, S>>;

    /// # Safety
    ///
    /// The index must not be out of bounds.

    unsafe fn _yield_write_shard(&'a self, i: usize) -> RwLockWriteGuard<'a, HashMap<K, V, S>>;

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

    // provided
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
        let mut p0: DashMap<i32, ReadOnlyView<i32, String>> = DashMap::new();
        
        p0.insert(1, ReadOnlyView::new(DashMap::new()));

        crate::t::Map::_clear(&p0);
    }
}#[cfg(test)]
mod tests_rug_98 {
    use super::*;
    use crate::{DashMap, ReadOnlyView, RandomState};

    #[test]
    fn test_rug() {
        let mut p0: DashMap<i32, i32, RandomState> = DashMap::new();
        p0.insert(1, 10);

        let p1: i32 = 1;

        assert!(p0._contains_key(&p1));
    }
}#[cfg(test)]
mod tests_rug_99 {
    use super::*;
    use crate::{DashMap, ReadOnlyView, RandomState};

    #[test]
    fn test_is_empty_true() {
        let mut map: DashMap<u32, &str, RandomState> = DashMap::new();
        assert!(map.is_empty());
    }

    #[test]
    fn test_is_empty_false() {
        let mut map: DashMap<u32, &str, RandomState> = DashMap::new();
        map.insert(1, "one");
        assert!(!map.is_empty());
    }
}