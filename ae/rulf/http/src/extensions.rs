use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hasher};
type AnyMap = HashMap<TypeId, Box<dyn Any + Send + Sync>, BuildHasherDefault<IdHasher>>;
#[derive(Default)]
struct IdHasher(u64);
impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }
    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}
/// A type map of protocol extensions.
///
/// `Extensions` can be used by `Request` and `Response` to store
/// extra data derived from the underlying protocol.
#[derive(Default)]
pub struct Extensions {
    map: Option<Box<AnyMap>>,
}
impl Extensions {
    /// Create an empty `Extensions`.
    #[inline]
    pub fn new() -> Extensions {
        Extensions { map: None }
    }
    /// Insert a type into this `Extensions`.
    ///
    /// If a extension of this type already existed, it will
    /// be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// assert!(ext.insert(5i32).is_none());
    /// assert!(ext.insert(4u8).is_none());
    /// assert_eq!(ext.insert(9i32), Some(5i32));
    /// ```
    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) -> Option<T> {
        self.map
            .get_or_insert_with(|| Box::new(HashMap::default()))
            .insert(TypeId::of::<T>(), Box::new(val))
            .and_then(|boxed| {
                (boxed as Box<dyn Any + 'static>).downcast().ok().map(|boxed| *boxed)
            })
    }
    /// Get a reference to a type previously inserted on this `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// assert!(ext.get::<i32>().is_none());
    /// ext.insert(5i32);
    ///
    /// assert_eq!(ext.get::<i32>(), Some(&5i32));
    /// ```
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .as_ref()
            .and_then(|map| map.get(&TypeId::of::<T>()))
            .and_then(|boxed| (&**boxed as &(dyn Any + 'static)).downcast_ref())
    }
    /// Get a mutable reference to a type previously inserted on this `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(String::from("Hello"));
    /// ext.get_mut::<String>().unwrap().push_str(" World");
    ///
    /// assert_eq!(ext.get::<String>().unwrap(), "Hello World");
    /// ```
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.map
            .as_mut()
            .and_then(|map| map.get_mut(&TypeId::of::<T>()))
            .and_then(|boxed| (&mut **boxed as &mut (dyn Any + 'static)).downcast_mut())
    }
    /// Remove a type from this `Extensions`.
    ///
    /// If a extension of this type existed, it will be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(5i32);
    /// assert_eq!(ext.remove::<i32>(), Some(5i32));
    /// assert!(ext.get::<i32>().is_none());
    /// ```
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.map
            .as_mut()
            .and_then(|map| map.remove(&TypeId::of::<T>()))
            .and_then(|boxed| {
                (boxed as Box<dyn Any + 'static>).downcast().ok().map(|boxed| *boxed)
            })
    }
    /// Clear the `Extensions` of all inserted extensions.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(5i32);
    /// ext.clear();
    ///
    /// assert!(ext.get::<i32>().is_none());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        if let Some(ref mut map) = self.map {
            map.clear();
        }
    }
}
impl fmt::Debug for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Extensions").finish()
    }
}
#[test]
fn test_extensions() {
    #[derive(Debug, PartialEq)]
    struct MyType(i32);
    let mut extensions = Extensions::new();
    extensions.insert(5i32);
    extensions.insert(MyType(10));
    assert_eq!(extensions.get(), Some(& 5i32));
    assert_eq!(extensions.get_mut(), Some(& mut 5i32));
    assert_eq!(extensions.remove::< i32 > (), Some(5i32));
    assert!(extensions.get::< i32 > ().is_none());
    assert_eq!(extensions.get::< bool > (), None);
    assert_eq!(extensions.get(), Some(& MyType(10)));
}
#[cfg(test)]
mod tests_llm_16_53 {
    use crate::extensions::IdHasher;
    use std::hash::Hasher;
    #[test]
    fn test_finish() {
        let _rug_st_tests_llm_16_53_rrrruuuugggg_test_finish = 0;
        let rug_fuzz_0 = 42;
        let hasher = IdHasher(rug_fuzz_0);
        let result = <IdHasher as Hasher>::finish(&hasher);
        debug_assert_eq!(result, 42);
        let _rug_ed_tests_llm_16_53_rrrruuuugggg_test_finish = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_54 {
    use std::hash::Hasher;
    use crate::extensions::IdHasher;
    #[test]
    fn test_write() {
        let _rug_st_tests_llm_16_54_rrrruuuugggg_test_write = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 2;
        let rug_fuzz_2 = 3;
        let rug_fuzz_3 = 4;
        let rug_fuzz_4 = 5;
        let mut hasher = IdHasher::default();
        let data = &[rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        hasher.write(data);
        let _rug_ed_tests_llm_16_54_rrrruuuugggg_test_write = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_55 {
    use std::hash::Hasher;
    use crate::extensions::IdHasher;
    #[test]
    fn test_write_u64() {
        let _rug_st_tests_llm_16_55_rrrruuuugggg_test_write_u64 = 0;
        let rug_fuzz_0 = 12345;
        let mut hasher = IdHasher::default();
        hasher.write_u64(rug_fuzz_0);
        debug_assert_eq!(hasher.finish(), 12345);
        let _rug_ed_tests_llm_16_55_rrrruuuugggg_test_write_u64 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_322 {
    use super::*;
    use crate::*;
    use std::collections::HashMap;
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_322_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = 5i32;
        let mut ext = Extensions::new();
        ext.insert(rug_fuzz_0);
        ext.clear();
        debug_assert!(ext.get:: < i32 > ().is_none());
        let _rug_ed_tests_llm_16_322_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_323 {
    use super::*;
    use crate::*;
    use std::any::Any;
    use std::collections::HashMap;
    use std::any::TypeId;
    use std::sync::Arc;
    use bytes::Bytes;
    use bytes::BytesMut;
    use crate::header::HeaderMap;
    #[test]
    fn test_get_existing_value() {
        let _rug_st_tests_llm_16_323_rrrruuuugggg_test_get_existing_value = 0;
        let rug_fuzz_0 = 5i32;
        let mut ext = Extensions::new();
        ext.insert(rug_fuzz_0);
        debug_assert_eq!(ext.get:: < i32 > (), Some(& 5i32));
        let _rug_ed_tests_llm_16_323_rrrruuuugggg_test_get_existing_value = 0;
    }
    #[test]
    fn test_get_non_existing_value() {
        let _rug_st_tests_llm_16_323_rrrruuuugggg_test_get_non_existing_value = 0;
        let ext = Extensions::new();
        debug_assert_eq!(ext.get:: < i32 > (), None);
        let _rug_ed_tests_llm_16_323_rrrruuuugggg_test_get_non_existing_value = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_324 {
    use crate::Extensions;
    use std::any::TypeId;
    #[test]
    fn test_get_mut() {
        let _rug_st_tests_llm_16_324_rrrruuuugggg_test_get_mut = 0;
        let rug_fuzz_0 = "Hello";
        let rug_fuzz_1 = " World";
        let mut ext = Extensions::new();
        ext.insert(String::from(rug_fuzz_0));
        ext.get_mut::<String>().unwrap().push_str(rug_fuzz_1);
        debug_assert_eq!(ext.get:: < String > ().unwrap(), "Hello World");
        let _rug_ed_tests_llm_16_324_rrrruuuugggg_test_get_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_326 {
    use super::*;
    use crate::*;
    use std::any::TypeId;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_326_rrrruuuugggg_test_new = 0;
        let ext = Extensions::new();
        debug_assert!(ext.map.is_none());
        let _rug_ed_tests_llm_16_326_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_263 {
    use super::*;
    use crate::Extensions;
    use std::any::TypeId;
    use std::any::Any;
    use std::collections::HashMap;
    #[test]
    fn test_extensions_insert() {
        let _rug_st_tests_rug_263_rrrruuuugggg_test_extensions_insert = 0;
        let rug_fuzz_0 = 5;
        let mut p0 = Extensions::new();
        let p1: i32 = rug_fuzz_0;
        debug_assert!(p0.insert(p1).is_none());
        let _rug_ed_tests_rug_263_rrrruuuugggg_test_extensions_insert = 0;
    }
}
#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use crate::Extensions;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_264_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 5i32;
        let mut p0 = Extensions::new();
        p0.insert(rug_fuzz_0);
        let result: Option<i32> = Extensions::remove(&mut p0);
        debug_assert_eq!(result, Some(5i32));
        debug_assert_eq!(p0.get:: < i32 > (), None);
        let _rug_ed_tests_rug_264_rrrruuuugggg_test_rug = 0;
    }
}
