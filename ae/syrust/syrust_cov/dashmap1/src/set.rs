use crate::iter_set::{Iter, OwningIter};
#[cfg(feature = "raw-api")]
use crate::lock::RwLock;
use crate::setref::one::Ref;
use crate::DashMap;
#[cfg(feature = "raw-api")]
use crate::HashMap;
use ahash::RandomState;
use cfg_if::cfg_if;
use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::iter::FromIterator;

/// DashSet is a thin wrapper around [`DashMap`] using `()` as the value type. It uses
/// methods and types which are more convenient to work with on a set.
///
/// [`DashMap`]: struct.DashMap.html

pub struct DashSet<K, S = RandomState> {
    inner: DashMap<K, (), S>,
}

impl<K: Eq + Hash + fmt::Debug, S: BuildHasher + Clone> fmt::Debug for DashSet<K, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<K: Eq + Hash + Clone, S: Clone> Clone for DashSet<K, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.inner.clone_from(&source.inner)
    }
}

impl<K, S> Default for DashSet<K, S>
where
    K: Eq + Hash,
    S: Default + BuildHasher + Clone,
{
    fn default() -> Self {
        Self::with_hasher(Default::default())
    }
}

impl<'a, K: 'a + Eq + Hash> DashSet<K, RandomState> {
    /// Creates a new DashSet with a capacity of 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let games = DashSet::new();
    /// games.insert("Veloren");
    /// ```

    pub fn new() -> Self {
        Self::with_hasher(RandomState::default())
    }

    /// Creates a new DashMap with a specified starting capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let numbers = DashSet::with_capacity(2);
    /// numbers.insert(2);
    /// numbers.insert(8);
    /// ```

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::default())
    }
}

impl<'a, K: 'a + Eq + Hash, S: BuildHasher + Clone> DashSet<K, S> {
    /// Creates a new DashMap with a capacity of 0 and the provided hasher.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let s = RandomState::new();
    /// let games = DashSet::with_hasher(s);
    /// games.insert("Veloren");
    /// ```

    pub fn with_hasher(hasher: S) -> Self {
        Self::with_capacity_and_hasher(0, hasher)
    }

    /// Creates a new DashMap with a specified starting capacity and hasher.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let s = RandomState::new();
    /// let numbers = DashSet::with_capacity_and_hasher(2, s);
    /// numbers.insert(2);
    /// numbers.insert(8);
    /// ```

    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self {
            inner: DashMap::with_capacity_and_hasher(capacity, hasher),
        }
    }

    /// Hash a given item to produce a usize.
    /// Uses the provided or default HashBuilder.

    pub fn hash_usize<T: Hash>(&self, item: &T) -> usize {
        self.inner.hash_usize(item)
    }

    cfg_if! {
        if #[cfg(feature = "raw-api")] {
            /// Allows you to peek at the inner shards that store your data.
            /// You should probably not use this unless you know what you are doing.
            ///
            /// Requires the `raw-api` feature to be enabled.
            ///
            /// # Examples
            ///
            /// ```
            /// use dashmap::DashSet;
            ///
            /// let set = DashSet::<()>::new();
            /// println!("Amount of shards: {}", set.shards().len());
            /// ```

            pub fn shards(&self) -> &[RwLock<HashMap<K, (), S>>] {
                self.inner.shards()
            }
        }
    }

    cfg_if! {
        if #[cfg(feature = "raw-api")] {
            /// Finds which shard a certain key is stored in.
            /// You should probably not use this unless you know what you are doing.
            /// Note that shard selection is dependent on the default or provided HashBuilder.
            ///
            /// Requires the `raw-api` feature to be enabled.
            ///
            /// # Examples
            ///
            /// ```
            /// use dashmap::DashSet;
            ///
            /// let set = DashSet::new();
            /// set.insert("coca-cola");
            /// println!("coca-cola is stored in shard: {}", set.determine_map("coca-cola"));
            /// ```

            pub fn determine_map<Q>(&self, key: &Q) -> usize
            where
                K: Borrow<Q>,
                Q: Hash + Eq + ?Sized,
            {
                self.inner.determine_map(key)
            }
        }
    }

    cfg_if! {
        if #[cfg(feature = "raw-api")] {
            /// Finds which shard a certain hash is stored in.
            ///
            /// Requires the `raw-api` feature to be enabled.
            ///
            /// # Examples
            ///
            /// ```
            /// use dashmap::DashSet;
            ///
            /// let set: DashSet<i32> = DashSet::new();
            /// let key = "key";
            /// let hash = set.hash_usize(&key);
            /// println!("hash is stored in shard: {}", set.determine_shard(hash));
            /// ```

            pub fn determine_shard(&self, hash: usize) -> usize {
                self.inner.determine_shard(hash)
            }
        }
    }

    /// Inserts a key into the set. Returns true if the key was not already in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let set = DashSet::new();
    /// set.insert("I am the key!");
    /// ```

    pub fn insert(&self, key: K) -> bool {
        self.inner.insert(key, ()).is_none()
    }

    /// Removes an entry from the map, returning the key if it existed in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let soccer_team = DashSet::new();
    /// soccer_team.insert("Jack");
    /// assert_eq!(soccer_team.remove("Jack").unwrap(), "Jack");
    /// ```

    pub fn remove<Q>(&self, key: &Q) -> Option<K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(key).map(|(k, _)| k)
    }

    /// Removes an entry from the set, returning the key
    /// if the entry existed and the provided conditional function returned true.
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let soccer_team = DashSet::new();
    /// soccer_team.insert("Sam");
    /// soccer_team.remove_if("Sam", |player| player.starts_with("Ja"));
    /// assert!(soccer_team.contains("Sam"));
    /// ```
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let soccer_team = DashSet::new();
    /// soccer_team.insert("Sam");
    /// soccer_team.remove_if("Jacob", |player| player.starts_with("Ja"));
    /// assert!(!soccer_team.contains("Jacob"));
    /// ```

    pub fn remove_if<Q>(&self, key: &Q, f: impl FnOnce(&K) -> bool) -> Option<K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        // TODO: Don't create another closure around f
        self.inner.remove_if(key, |k, _| f(k)).map(|(k, _)| k)
    }

    /// Creates an iterator over a DashMap yielding immutable references.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let words = DashSet::new();
    /// words.insert("hello");
    /// assert_eq!(words.iter().count(), 1);
    /// ```

    pub fn iter(&'a self) -> Iter<'a, K, S, DashMap<K, (), S>> {
        let iter = self.inner.iter();

        Iter::new(iter)
    }

    /// Get a reference to an entry in the set
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let youtubers = DashSet::new();
    /// youtubers.insert("Bosnian Bill");
    /// assert_eq!(*youtubers.get("Bosnian Bill").unwrap(), "Bosnian Bill");
    /// ```

    pub fn get<Q>(&'a self, key: &Q) -> Option<Ref<'a, K, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get(key).map(Ref::new)
    }

    /// Remove excess capacity to reduce memory usage.

    pub fn shrink_to_fit(&self) {
        self.inner.shrink_to_fit()
    }

    /// Retain elements that whose predicates return true
    /// and discard elements whose predicates return false.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let people = DashSet::new();
    /// people.insert("Albin");
    /// people.insert("Jones");
    /// people.insert("Charlie");
    /// people.retain(|name| name.contains('i'));
    /// assert_eq!(people.len(), 2);
    /// ```

    pub fn retain(&self, mut f: impl FnMut(&K) -> bool) {
        // TODO: Don't create another closure
        self.inner.retain(|k, _| f(k))
    }

    /// Fetches the total number of keys stored in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let people = DashSet::new();
    /// people.insert("Albin");
    /// people.insert("Jones");
    /// people.insert("Charlie");
    /// assert_eq!(people.len(), 3);
    /// ```

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Checks if the set is empty or not.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let map = DashSet::<()>::new();
    /// assert!(map.is_empty());
    /// ```

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Removes all keys in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let people = DashSet::new();
    /// people.insert("Albin");
    /// assert!(!people.is_empty());
    /// people.clear();
    /// assert!(people.is_empty());
    /// ```

    pub fn clear(&self) {
        self.inner.clear()
    }

    /// Returns how many keys the set can store without reallocating.

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Checks if the set contains a specific key.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashmap::DashSet;
    ///
    /// let people = DashSet::new();
    /// people.insert("Dakota Cherries");
    /// assert!(people.contains("Dakota Cherries"));
    /// ```

    pub fn contains<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(key)
    }
}

impl<'a, K: Eq + Hash, S: BuildHasher + Clone> IntoIterator for DashSet<K, S> {
    type Item = K;

    type IntoIter = OwningIter<K, S>;

    fn into_iter(self) -> Self::IntoIter {
        OwningIter::new(self.inner.into_iter())
    }
}

impl<K: Eq + Hash, S: BuildHasher + Clone> Extend<K> for DashSet<K, S> {
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        let iter = iter.into_iter().map(|k| (k, ()));

        self.inner.extend(iter)
    }
}

impl<K: Eq + Hash> FromIterator<K> for DashSet<K, RandomState> {
    fn from_iter<I: IntoIterator<Item = K>>(iter: I) -> Self {
        let mut set = DashSet::new();

        set.extend(iter);

        set
    }
}

#[cfg(test)]

mod tests {

    use crate::DashSet;

    #[test]

    fn test_basic() {
        let set = DashSet::new();

        set.insert(0);

        assert_eq!(set.get(&0).as_deref(), Some(&0));
    }

    #[test]

    fn test_default() {
        let set: DashSet<u32> = DashSet::default();

        set.insert(0);

        assert_eq!(set.get(&0).as_deref(), Some(&0));
    }

    #[test]

    fn test_multiple_hashes() {
        let set = DashSet::<u32>::default();

        for i in 0..100 {
            assert!(set.insert(i));
        }

        for i in 0..100 {
            assert!(!set.insert(i));
        }

        for i in 0..100 {
            assert_eq!(Some(i), set.remove(&i));
        }

        for i in 0..100 {
            assert_eq!(None, set.remove(&i));
        }
    }
}
#[cfg(test)]
mod tests_rug_173 {
    use super::*;
    use crate::set::DashSet;
    use std::default::Default;
    
    #[test]
    fn test_default() {
        let set: DashSet<i32> = <DashSet<i32>>::default();
        // Add assertions or further test logic here
    }
}
#[cfg(test)]
mod tests_rug_174 {
    use super::*;
    use crate::DashSet;
    use std::collections::hash_map::RandomState;

    #[test]
    fn test_rug() {
        let _games: DashSet<&str> = DashSet::new();
    }
}
#[cfg(test)]
mod tests_rug_175 {
    use super::*;
    use crate::DashSet;

    #[test]
    fn test_with_capacity() {
        let capacity: usize = 5;
        
        let numbers = DashSet::<i32>::with_capacity(capacity);

        // Add assertions or further tests here
        assert_eq!(numbers.len(), 0);
    }
}#[cfg(test)]
mod tests_rug_176 {
    use super::*;
    use crate::DashSet;
    use std::collections::hash_map::RandomState;

    #[test]
    fn test_rug() {
        let mut p0: RandomState = RandomState::new();
        DashSet::<&str, _>::with_hasher(p0);
    }
}#[cfg(test)]
mod tests_rug_178 {
    use super::*;
    use crate::DashSet;

    #[test]
    fn test_rug() {
        let mut p0: DashSet<i32> = DashSet::new();
        let p1: i32 = 42;
        
        p0.hash_usize(&p1);
    }
}#[cfg(test)]
mod tests_rug_179 {
    use super::*;
    use crate::DashSet;

    #[test]
    fn test_rug() {
        let mut p0: DashSet<i32> = DashSet::new();
        let mut p1: i32 = 42;

        p0.insert(p1);
    }
}
#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    use crate::DashSet;

    #[test]
    fn test_rug() {
        let mut p0: DashSet<i32> = DashSet::new();
        let mut p1: i32 = 42;

        p0.remove(&p1);
    }
}
#[cfg(test)]
mod tests_rug_181 {
    use crate::{DashSet};
    use std::borrow::Borrow;
    use std::hash::Hash;

    #[test]
    fn test_rug() {
        let mut soccer_team: DashSet<&str> = DashSet::new();
        soccer_team.insert("Sam");
        let key = "Sam";
        let f = |player: &&str| player.starts_with("Ja");

        soccer_team.remove_if(&key, f);
        assert!(soccer_team.contains("Sam"));
    }
}#[cfg(test)]
mod tests_rug_182 {
    use super::*;
    use crate::{DashSet, DashMap};

    #[test]
    fn test_iter() {
        let words = DashSet::<&str, RandomState>::new();
        words.insert("hello");

        let p0 = &words;

        DashSet::<&str, RandomState>::iter(p0);
    }
}#[cfg(test)]
mod tests_rug_183 {
    use super::*;
    use crate::DashSet;
    use std::borrow::Borrow;
    use std::hash::Hash;

    #[test]
    fn test_rug() {
        // Constructing the DashSet<K, S>
        let mut p0: DashSet<i32> = DashSet::new();
        
        // Constructing the key parameter
        let p1: &i32 = &42;

        <DashSet<i32>>::get(&p0, p1);
    }
}#[cfg(test)]
mod tests_rug_184 {
    use super::*;
    use crate::set::DashSet;

    #[test]
    fn test_rug() {
        let mut p0: DashSet<i32, RandomState> = DashSet::new();
        
        DashSet::<i32, RandomState>::shrink_to_fit(&p0);
    }
}#[cfg(test)]
mod tests_rug_185 {
    use super::*;
    use crate::DashSet;

    #[test]
    fn test_retain() {
        let mut p0 = DashSet::new();
        p0.insert("Albin");
        p0.insert("Jones");
        p0.insert("Charlie");
        
        let mut p1 = |name: &&str| -> bool { name.contains('i') };

        DashSet::<&str>::retain(&p0, &mut p1);

        assert_eq!(p0.len(), 2);
    }
}#[cfg(test)]
mod tests_rug_186 {
    use super::*;
    use crate::DashSet;
    
    #[test]
    fn test_len() {
        let people = DashSet::new();
        people.insert("Albin");
        people.insert("Jones");
        people.insert("Charlie");

        assert_eq!(<DashSet<&str>>::len(&people), 3);
    }
}
#[cfg(test)]
mod tests_rug_187 {
    use super::*;
    use crate::DashSet;

    #[test]
    fn test_is_empty() {
        let set: DashSet<i32> = DashSet::new();
        assert!(<DashSet<i32>>::is_empty(&set));
    }
}
#[cfg(test)]
mod tests_rug_188 {
    use super::*;
    use crate::DashSet;

    #[test]
    fn test_rug() {
        let mut p0: DashSet<&str>;

        p0 = DashSet::new();
        p0.insert("Albin");
        
        <DashSet<&str>>::clear(&p0);

        assert_eq!(p0.len(), 0);
    }
}#[cfg(test)]
mod tests_rug_189 {
    use super::*;
    use crate::set::DashSet;

    #[test]
    fn test_capacity() {
        let mut p0: DashSet<i32, RandomState> = DashSet::new();

        p0.capacity();
    }
}#[cfg(test)]
mod tests_rug_190 {
    use super::*;
    use crate::DashSet;

    #[test]
    fn test_contains() {
        let mut set: DashSet<i32> = DashSet::new();
        set.insert(42);
        let key: i32 = 42;

        assert!(set.contains(&key));
    }
}#[cfg(test)]
mod tests_rug_192 {
    use super::*;
    use crate::DashSet;
    use ahash::RandomState;

    #[test]
    fn test_extend_dashset() {
        #[cfg(test)]
        mod tests_rug_192_prepare {
            use crate::DashSet;
            use ahash::RandomState;

            #[test]
            fn sample() {
                let mut v19: i32 = 42;
                let v38 = RandomState::new();
            }
        }

        let mut dash_set: DashSet<i32, RandomState> = DashSet::with_hasher(RandomState::new());
        dash_set.insert(1);

        let mut iter_set: DashSet<i32, RandomState> = DashSet::with_hasher(RandomState::new());
        iter_set.insert(2);
        
        dash_set.extend(iter_set.clone());

        assert!(dash_set.contains(&1));
        assert!(dash_set.contains(&2));
    }
}
#[cfg(test)]
mod tests_rug_193 {
    use super::*;
    use crate::DashSet;
    use ahash::RandomState;

    #[test]
    fn test_dashset_from_iter() {
        #[cfg(test)]
        mod tests_rug_193_prepare {
            use crate::DashSet;
            use ahash::RandomState;

            #[test]
            fn sample_dashset() {
                let mut v19: i32 = 42;
                let v38 = RandomState::new();
                let mut dashset: DashSet<i32, RandomState> = DashSet::with_hasher(v38);
                dashset.insert(v19);
            }
        }

        let mut p0 = DashSet::new();
        p0.insert(42);

        <DashSet<i32, RandomState>>::from_iter(p0);

    }
}
