use crate::lock::{RwLockReadGuard, RwLockWriteGuard};
use crate::HashMap;
use ahash::RandomState;
use core::hash::{BuildHasher, Hash};
use core::ops::{Deref, DerefMut};
pub struct Ref<'a, K, V, S = RandomState> {
    _guard: RwLockReadGuard<'a, HashMap<K, V, S>>,
    k: &'a K,
    v: &'a V,
}
unsafe impl<'a, K: Eq + Hash + Send, V: Send, S: BuildHasher> Send for Ref<'a, K, V, S> {}
unsafe impl<'a, K: Eq + Hash + Send + Sync, V: Send + Sync, S: BuildHasher> Sync
for Ref<'a, K, V, S> {}
impl<'a, K: Eq + Hash, V, S: BuildHasher> Ref<'a, K, V, S> {
    pub(crate) fn new(
        guard: RwLockReadGuard<'a, HashMap<K, V, S>>,
        k: &'a K,
        v: &'a V,
    ) -> Self {
        Self { _guard: guard, k, v }
    }
    pub fn key(&self) -> &K {
        self.k
    }
    pub fn value(&self) -> &V {
        self.v
    }
    pub fn pair(&self) -> (&K, &V) {
        (self.k, self.v)
    }
}
impl<'a, K: Eq + Hash, V, S: BuildHasher> Deref for Ref<'a, K, V, S> {
    type Target = V;
    fn deref(&self) -> &V {
        self.value()
    }
}
pub struct RefMut<'a, K, V, S = RandomState> {
    guard: RwLockWriteGuard<'a, HashMap<K, V, S>>,
    k: &'a K,
    v: &'a mut V,
}
unsafe impl<'a, K: Eq + Hash + Send, V: Send, S: BuildHasher> Send
for RefMut<'a, K, V, S> {}
unsafe impl<'a, K: Eq + Hash + Send + Sync, V: Send + Sync, S: BuildHasher> Sync
for RefMut<'a, K, V, S> {}
impl<'a, K: Eq + Hash, V, S: BuildHasher> RefMut<'a, K, V, S> {
    pub(crate) fn new(
        guard: RwLockWriteGuard<'a, HashMap<K, V, S>>,
        k: &'a K,
        v: &'a mut V,
    ) -> Self {
        Self { guard, k, v }
    }
    pub fn key(&self) -> &K {
        self.k
    }
    pub fn value(&self) -> &V {
        self.v
    }
    pub fn value_mut(&mut self) -> &mut V {
        self.v
    }
    pub fn pair(&self) -> (&K, &V) {
        (self.k, self.v)
    }
    pub fn pair_mut(&mut self) -> (&K, &mut V) {
        (self.k, self.v)
    }
    pub fn downgrade(self) -> Ref<'a, K, V, S> {
        Ref::new(self.guard.downgrade(), self.k, self.v)
    }
}
impl<'a, K: Eq + Hash, V, S: BuildHasher> Deref for RefMut<'a, K, V, S> {
    type Target = V;
    fn deref(&self) -> &V {
        self.value()
    }
}
impl<'a, K: Eq + Hash, V, S: BuildHasher> DerefMut for RefMut<'a, K, V, S> {
    fn deref_mut(&mut self) -> &mut V {
        self.value_mut()
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::{DashMap, mapref::one::{RefMut, Ref}};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let map: DashMap<i32, String> = DashMap::new();
        let key = rug_fuzz_0;
        map.insert(key, String::from(rug_fuzz_1));
        let guard = map.get_mut(&key).unwrap();
        let p0: RefMut<i32, String, _> = guard;
        p0.downgrade();
             }
});    }
}
