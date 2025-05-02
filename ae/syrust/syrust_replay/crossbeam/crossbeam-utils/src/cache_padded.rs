use core::fmt;
use core::ops::{Deref, DerefMut};
/// Pads and aligns a value to the length of a cache line.
///
/// In concurrent programming, sometimes it is desirable to make sure commonly accessed pieces of
/// data are not placed into the same cache line. Updating an atomic value invalides the whole
/// cache line it belongs to, which makes the next access to the same cache line slower for other
/// CPU cores. Use `CachePadded` to ensure updating one piece of data doesn't invalidate other
/// cached data.
///
/// # Size and alignment
///
/// Cache lines are assumed to be N bytes long, depending on the architecture:
///
/// * On x86-64 and aarch64, N = 128.
/// * On all others, N = 64.
///
/// Note that N is just a reasonable guess and is not guaranteed to match the actual cache line
/// length of the machine the program is running on. On modern Intel architectures, spatial
/// prefetcher is pulling pairs of 64-byte cache lines at a time, so we pessimistically assume that
/// cache lines are 128 bytes long.
///
/// The size of `CachePadded<T>` is the smallest multiple of N bytes large enough to accommodate
/// a value of type `T`.
///
/// The alignment of `CachePadded<T>` is the maximum of N bytes and the alignment of `T`.
///
/// # Examples
///
/// Alignment and padding:
///
/// ```
/// use crossbeam_utils::CachePadded;
///
/// let array = [CachePadded::new(1i8), CachePadded::new(2i8)];
/// let addr1 = &*array[0] as *const i8 as usize;
/// let addr2 = &*array[1] as *const i8 as usize;
///
/// assert!(addr2 - addr1 >= 64);
/// assert_eq!(addr1 % 64, 0);
/// assert_eq!(addr2 % 64, 0);
/// ```
///
/// When building a concurrent queue with a head and a tail index, it is wise to place them in
/// different cache lines so that concurrent threads pushing and popping elements don't invalidate
/// each other's cache lines:
///
/// ```
/// use crossbeam_utils::CachePadded;
/// use std::sync::atomic::AtomicUsize;
///
/// struct Queue<T> {
///     head: CachePadded<AtomicUsize>,
///     tail: CachePadded<AtomicUsize>,
///     buffer: *mut T,
/// }
/// ```
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
#[cfg_attr(any(target_arch = "x86_64", target_arch = "aarch64"), repr(align(128)))]
#[cfg_attr(not(any(target_arch = "x86_64", target_arch = "aarch64")), repr(align(64)))]
pub struct CachePadded<T> {
    value: T,
}
unsafe impl<T: Send> Send for CachePadded<T> {}
unsafe impl<T: Sync> Sync for CachePadded<T> {}
impl<T> CachePadded<T> {
    /// Pads and aligns a value to the length of a cache line.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::CachePadded;
    ///
    /// let padded_value = CachePadded::new(1);
    /// ```
    pub const fn new(t: T) -> CachePadded<T> {
        CachePadded::<T> { value: t }
    }
    /// Returns the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossbeam_utils::CachePadded;
    ///
    /// let padded_value = CachePadded::new(7);
    /// let value = padded_value.into_inner();
    /// assert_eq!(value, 7);
    /// ```
    pub fn into_inner(self) -> T {
        self.value
    }
}
impl<T> Deref for CachePadded<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T> DerefMut for CachePadded<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
impl<T: fmt::Debug> fmt::Debug for CachePadded<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachePadded").field("value", &self.value).finish()
    }
}
impl<T> From<T> for CachePadded<T> {
    fn from(t: T) -> Self {
        CachePadded::new(t)
    }
}
#[cfg(test)]
mod tests_rug_776 {
    use super::*;
    use crate::CachePadded;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(u32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: u32 = rug_fuzz_0;
        <CachePadded<u32>>::new(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_777 {
    use super::*;
    use crate::CachePadded;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = CachePadded::<i32>::new(rug_fuzz_0);
        debug_assert_eq!(CachePadded:: < i32 > ::into_inner(p0), 7);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_778 {
    use super::*;
    use crate::cache_padded::CachePadded;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: CachePadded<i32> = CachePadded::new(rug_fuzz_0);
        p0.deref();
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_779 {
    use super::*;
    use crate::CachePadded;
    use std::ops::DerefMut;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: CachePadded<i32> = CachePadded::new(rug_fuzz_0);
        p0.deref_mut();
             }
}
}
}    }
}
