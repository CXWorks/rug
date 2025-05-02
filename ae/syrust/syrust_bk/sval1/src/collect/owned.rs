use crate::{
    collect::{
        self,
        value::Value,
        Collect,
        Error,
    },
    stream::Arguments,
    value,
};

pub(crate) struct OwnedCollect<TStream> {
    stream: TStream,
}

impl<TStream> OwnedCollect<TStream>
where
    TStream: Collect,
{
    #[inline]
    pub(crate) fn new(stream: TStream) -> Self {
        OwnedCollect { stream }
    }

    #[inline]
    pub(crate) fn into_inner(self) -> TStream {
        self.stream
    }

    #[inline]
    pub(crate) fn borrow_mut(&mut self) -> RefMutCollect {
        RefMutCollect(OwnedCollect::new(&mut self.stream))
    }

    #[inline]
    pub fn any(&mut self, v: impl value::Value) -> collect::Result {
        v.stream(&mut value::Stream::new(self.borrow_mut()))
    }

    #[inline]
    pub fn fmt(&mut self, f: Arguments) -> collect::Result {
        self.stream.fmt(f)
    }

    #[inline]
    pub fn i64(&mut self, v: i64) -> collect::Result {
        self.stream.i64(v)
    }

    #[inline]
    pub fn u64(&mut self, v: u64) -> collect::Result {
        self.stream.u64(v)
    }

    #[inline]
    pub fn i128(&mut self, v: i128) -> collect::Result {
        self.stream.i128(v)
    }

    #[inline]
    pub fn u128(&mut self, v: u128) -> collect::Result {
        self.stream.u128(v)
    }

    #[inline]
    pub fn f64(&mut self, v: f64) -> collect::Result {
        self.stream.f64(v)
    }

    #[inline]
    pub fn bool(&mut self, v: bool) -> collect::Result {
        self.stream.bool(v)
    }

    #[inline]
    pub fn char(&mut self, v: char) -> collect::Result {
        self.stream.char(v)
    }

    #[inline]
    pub fn str(&mut self, v: &str) -> collect::Result {
        self.stream.str(v)
    }

    #[inline]
    pub fn none(&mut self) -> collect::Result {
        self.stream.none()
    }

    #[inline]
    pub fn map_begin(&mut self, len: Option<usize>) -> collect::Result {
        self.stream.map_begin(len)
    }

    #[inline]
    pub fn map_key(&mut self, k: impl value::Value) -> collect::Result {
        self.stream.map_key_collect(Value::new(&k))
    }

    #[inline]
    pub fn map_value(&mut self, v: impl value::Value) -> collect::Result {
        self.stream.map_value_collect(Value::new(&v))
    }

    #[inline]
    pub fn map_end(&mut self) -> collect::Result {
        self.stream.map_end()
    }

    #[inline]
    pub fn seq_begin(&mut self, len: Option<usize>) -> collect::Result {
        self.stream.seq_begin(len)
    }

    #[inline]
    pub fn seq_elem(&mut self, v: impl value::Value) -> collect::Result {
        self.stream.seq_elem_collect(Value::new(&v))
    }

    #[inline]
    pub fn seq_end(&mut self) -> collect::Result {
        self.stream.seq_end()
    }

    #[inline]
    pub fn map_key_begin(&mut self) -> Result<&mut Self, Error> {
        self.stream.map_key()?;

        Ok(self)
    }

    #[inline]
    pub fn map_value_begin(&mut self) -> Result<&mut Self, Error> {
        self.stream.map_value()?;

        Ok(self)
    }

    #[inline]
    pub fn seq_elem_begin(&mut self) -> Result<&mut Self, Error> {
        self.stream.seq_elem()?;

        Ok(self)
    }
}

pub(crate) struct RefMutCollect<'a>(OwnedCollect<&'a mut dyn Collect>);

impl<'a> RefMutCollect<'a> {
    #[inline]
    pub fn fmt(&mut self, f: Arguments) -> value::Result {
        self.0.fmt(f)
    }

    #[inline]
    pub fn any(&mut self, v: impl value::Value) -> collect::Result {
        self.0.any(v)
    }

    #[inline]
    pub fn i64(&mut self, v: i64) -> value::Result {
        self.0.i64(v)
    }

    #[inline]
    pub fn u64(&mut self, v: u64) -> value::Result {
        self.0.u64(v)
    }

    #[inline]
    pub fn i128(&mut self, v: i128) -> value::Result {
        self.0.i128(v)
    }

    #[inline]
    pub fn u128(&mut self, v: u128) -> value::Result {
        self.0.u128(v)
    }

    #[inline]
    pub fn f64(&mut self, v: f64) -> value::Result {
        self.0.f64(v)
    }

    #[inline]
    pub fn bool(&mut self, v: bool) -> value::Result {
        self.0.bool(v)
    }

    #[inline]
    pub fn char(&mut self, v: char) -> value::Result {
        self.0.char(v)
    }

    #[inline]
    pub fn str(&mut self, v: &str) -> value::Result {
        self.0.str(v)
    }

    #[inline]
    pub fn none(&mut self) -> value::Result {
        self.0.none()
    }

    #[inline]
    pub fn map_begin(&mut self, len: Option<usize>) -> value::Result {
        self.0.map_begin(len)
    }

    #[inline]
    pub fn map_key(&mut self, k: impl value::Value) -> value::Result {
        self.0.map_key(k)
    }

    #[inline]
    pub fn map_value(&mut self, v: impl value::Value) -> value::Result {
        self.0.map_value(v)
    }

    #[inline]
    pub fn map_end(&mut self) -> value::Result {
        self.0.map_end()
    }

    #[inline]
    pub fn seq_begin(&mut self, len: Option<usize>) -> value::Result {
        self.0.seq_begin(len)
    }

    #[inline]
    pub fn seq_elem(&mut self, v: impl value::Value) -> value::Result {
        self.0.seq_elem(v)
    }

    #[inline]
    pub fn seq_end(&mut self) -> value::Result {
        self.0.seq_end()
    }
}

impl<'a> RefMutCollect<'a> {
    #[inline]
    pub fn map_key_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.map_key_begin()?;

        Ok(self)
    }

    #[inline]
    pub fn map_value_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.map_value_begin()?;

        Ok(self)
    }

    #[inline]
    pub fn seq_elem_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.seq_elem_begin()?;

        Ok(self)
    }
}
#[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::{value, collect::owned::RefMutCollect};

    #[test]
    fn test_rug() {
        let mut p0: collect::owned::RefMutCollect = unimplemented!();
        let p1: i64 = 42;

        p0.i64(p1);
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;

    use crate::collect::owned::RefMutCollect;

    #[test]
    fn test_rug() {
        let mut p0: collect::owned::RefMutCollect<'_> = unimplemented!();
        let p1: u64 = 42;

        p0.u64(p1);
    }
}
#[cfg(test)]
mod tests_rug_145 {
    use super::*;
    use crate::collect::{self, owned};

    #[test]
    fn test_rug() {
        let mut p0: owned::RefMutCollect<'_> = todo!();
        let p1: i128 = 42;

        p0.i128(p1);
    }
}#[cfg(test)]
mod tests_rug_146 {
    use super::*;
    use crate::collect::{self, owned};

    #[test]
    fn test_rug() {
        let mut p0: collect::owned::RefMutCollect = unimplemented!();
        let p1: u128 = 42;

        p0.u128(p1);
    }
}#[cfg(test)]
mod tests_rug_147 {
    use super::*;
    use crate::collect::owned::RefMutCollect;

    #[test]
    fn test_rug() {
        let mut p0: RefMutCollect = unimplemented!(); // Fill with appropriate RefMutCollect variable
        let p1: f64 = 3.14; // Sample data for f64
        
        p0.f64(p1);
    }
}#[cfg(test)]
mod tests_rug_148 {
    use super::*;
    use crate::collect::owned::RefMutCollect;

    #[test]
    fn test_rug() {
        let mut p0: RefMutCollect = unimplemented!(); // you may need to initialize p0 according to your implementation
        let p1: bool = true; // Sample data for bool argument

        p0.bool(p1);
    }
}#[cfg(test)]
mod tests_rug_149 {
    use super::*;
    use crate::collect::owned::RefMutCollect;
    
    #[test]
    fn test_rug() {
        let mut p0: collect::owned::RefMutCollect<'_> = unimplemented!();
        let p1: char = 'a';

        <collect::owned::RefMutCollect<'_>>::char(&mut p0, p1);
    }
}#[cfg(test)]
mod tests_rug_150 {
    use super::*;
    use crate::{value, collect};

    #[test]
    fn test_rug() {
        let mut p0: collect::owned::RefMutCollect<'_> = unimplemented!();
        let p1: &str = "test_data";

        p0.str(&p1);
    }
}#[cfg(test)]
mod tests_rug_152 {
    use super::*;
    use crate::collect::owned::RefMutCollect;
    use core::option::Option;

    #[test]
    fn test_rug() {
        let mut p0: RefMutCollect<'_> = unimplemented!(); // Fill based on the description
        let p1: Option<usize> = Some(42); // Sample code provided

        p0.map_begin(p1);
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::value::{self, Value};
    use crate::collect::owned::RefMutCollect;

    struct MockValue;

    impl Value for MockValue {
        fn stream(&self, stream: &mut value::Stream) -> value::Result {
            Ok(()) // Implement a valid stream method based on the requirements
        }
    }

    #[test]
    fn test_map_value() {
        // Create a RefMutCollect
        let mut p0: RefMutCollect<'static> = todo!(); // Needs to be implemented based on the description

        // Create a MockValue to pass as the second argument
        let p1 = MockValue;

        p0.map_value(p1);
    }
}
#[cfg(test)]
mod tests_rug_155 {
    use super::*;
    use crate::{collect, value};

    #[test]
    fn test_rug() {
        let mut p0: collect::owned::RefMutCollect<'_> = unimplemented!();

        collect::owned::RefMutCollect::<'_>::map_end(&mut p0);

    }
}
#[cfg(test)]
mod tests_rug_156 {
    use super::*;
    use collect::owned::RefMutCollect;
    use core::option::Option;

    #[test]
    fn test_rug() {
        let mut p0: RefMutCollect<'_> = unimplemented!();
        let mut p1: Option<usize> = Some(42);

        p0.seq_begin(p1);

    }
}#[cfg(test)]
mod tests_rug_158 {
    use super::*;
    use crate::value;

    #[test]
    fn test_rug() {
        let mut p0: collect::owned::RefMutCollect<'_> = unimplemented!();

        p0.seq_end();

    }
}#[cfg(test)]
mod tests_rug_159 {
    use super::*;
    use crate::collect::owned::RefMutCollect;

    #[test]
    fn test_map_key_begin() {
        let mut p0: collect::owned::RefMutCollect<'_> = unimplemented!();

        p0.map_key_begin().unwrap();
    }
}#[cfg(test)]
mod tests_rug_160 {
    use super::*;
    use crate::collect::owned::RefMutCollect;

    #[test]
    fn test_rug() {
        let mut p0: RefMutCollect<'_> = unimplemented!();

        crate::collect::owned::RefMutCollect::<'_>::map_value_begin(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_161 {
    use super::*;
    use crate::collect::owned::RefMutCollect;

    #[test]
    fn test_rug() {
        let mut p0: collect::owned::RefMutCollect<'_> = unimplemented!();
        
        p0.seq_elem_begin().unwrap();
    }
}