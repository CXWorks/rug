use crate::{
    collect::{
        self,
        OwnedCollect,
        RefMutCollect,
    },
    stream::{
        self,
        Arguments,
        Error,
        Stream,
    },
    value::Value,
};

/**
An owned stream wrapper.

`OwnedStream` is an ergonomic wrapper over a raw [`Stream`] that makes it
easier to stream complex types.
*/
pub struct OwnedStream<S>(OwnedCollect<collect::Default<S>>);

impl<S> OwnedStream<S>
where
    S: Stream,
{
    /**
    Stream a value.
    */
    #[inline]
    pub fn stream(stream: S, value: impl Value) -> Result<S, Error> {
        let mut stream = Self::new(stream);
        stream.any(value)?;
        Ok(stream.into_inner())
    }

    /**
    Begin an owned stream.
    */
    #[inline]
    pub fn new(stream: S) -> Self {
        OwnedStream(OwnedCollect::new(collect::Default(stream)))
    }

    /**
    Unwrap the inner stream.
    */
    #[inline]
    pub fn into_inner(self) -> S {
        self.0.into_inner().0
    }

    /**
    Get a reference to the stream that can be used by a value.
    */
    #[inline]
    pub fn borrow_mut(&mut self) -> RefMutStream {
        RefMutStream(self.0.borrow_mut())
    }

    /**
    Stream a value.
    */
    #[inline]
    pub fn any(&mut self, v: impl Value) -> stream::Result {
        self.0.any(v)
    }

    /**
    Stream a format.
    */
    #[inline]
    pub fn fmt(&mut self, f: Arguments) -> stream::Result {
        self.0.fmt(f)
    }

    /**
    Stream a signed integer.
    */
    #[inline]
    pub fn i64(&mut self, v: i64) -> stream::Result {
        self.0.i64(v)
    }

    /**
    Stream an unsigned integer.
    */
    #[inline]
    pub fn u64(&mut self, v: u64) -> stream::Result {
        self.0.u64(v)
    }

    /**
    Stream a 128-bit signed integer.
    */
    #[inline]
    pub fn i128(&mut self, v: i128) -> stream::Result {
        self.0.i128(v)
    }

    /**
    Stream a 128-bit unsigned integer.
    */
    #[inline]
    pub fn u128(&mut self, v: u128) -> stream::Result {
        self.0.u128(v)
    }

    /**
    Stream a floating point value.
    */
    #[inline]
    pub fn f64(&mut self, v: f64) -> stream::Result {
        self.0.f64(v)
    }

    /**
    Stream a boolean.
    */
    #[inline]
    pub fn bool(&mut self, v: bool) -> stream::Result {
        self.0.bool(v)
    }

    /**
    Stream a unicode character.
    */
    #[inline]
    pub fn char(&mut self, v: char) -> stream::Result {
        self.0.char(v)
    }

    /**
    Stream a UTF8 string.
    */
    #[inline]
    pub fn str(&mut self, v: &str) -> stream::Result {
        self.0.str(v)
    }

    /**
    Stream an empty value.
    */
    #[inline]
    pub fn none(&mut self) -> stream::Result {
        self.0.none()
    }

    /**
    Begin a map.
    */
    #[inline]
    pub fn map_begin(&mut self, len: Option<usize>) -> stream::Result {
        self.0.map_begin(len)
    }

    /**
    Stream a map key.
    */
    #[inline]
    pub fn map_key(&mut self, k: impl Value) -> stream::Result {
        self.0.map_key(k)
    }

    /**
    Stream a map value.
    */
    #[inline]
    pub fn map_value(&mut self, v: impl Value) -> stream::Result {
        self.0.map_value(v)
    }

    /**
    End a map.
    */
    #[inline]
    pub fn map_end(&mut self) -> stream::Result {
        self.0.map_end()
    }

    /**
    Begin a sequence.
    */
    #[inline]
    pub fn seq_begin(&mut self, len: Option<usize>) -> stream::Result {
        self.0.seq_begin(len)
    }

    /**
    Stream a sequence element.
    */
    #[inline]
    pub fn seq_elem(&mut self, v: impl Value) -> stream::Result {
        self.0.seq_elem(v)
    }

    /**
    End a sequence.
    */
    #[inline]
    pub fn seq_end(&mut self) -> stream::Result {
        self.0.seq_end()
    }
}

impl<S> OwnedStream<S>
where
    S: Stream,
{
    /**
    Begin a map key.
    */
    #[inline]
    pub fn map_key_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.map_key_begin()?;

        Ok(self)
    }

    /**
    Begin a map value.
    */
    #[inline]
    pub fn map_value_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.map_value_begin()?;

        Ok(self)
    }

    /**
    Begin a sequence element.
    */
    #[inline]
    pub fn seq_elem_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.seq_elem_begin()?;

        Ok(self)
    }
}

impl<S> Stream for OwnedStream<S>
where
    S: Stream,
{
    #[inline]
    fn fmt(&mut self, args: Arguments) -> stream::Result {
        self.fmt(args)
    }

    #[inline]
    fn i64(&mut self, v: i64) -> stream::Result {
        self.i64(v)
    }

    #[inline]
    fn u64(&mut self, v: u64) -> stream::Result {
        self.u64(v)
    }

    #[inline]
    fn i128(&mut self, v: i128) -> stream::Result {
        self.i128(v)
    }

    #[inline]
    fn u128(&mut self, v: u128) -> stream::Result {
        self.u128(v)
    }

    #[inline]
    fn f64(&mut self, v: f64) -> stream::Result {
        self.f64(v)
    }

    #[inline]
    fn bool(&mut self, v: bool) -> stream::Result {
        self.bool(v)
    }

    #[inline]
    fn char(&mut self, v: char) -> stream::Result {
        self.char(v)
    }

    #[inline]
    fn str(&mut self, v: &str) -> stream::Result {
        self.str(v)
    }

    #[inline]
    fn none(&mut self) -> stream::Result {
        self.none()
    }

    #[inline]
    fn map_begin(&mut self, len: Option<usize>) -> stream::Result {
        self.map_begin(len)
    }

    #[inline]
    fn map_key(&mut self) -> stream::Result {
        self.map_key_begin().map(|_| ())
    }

    #[inline]
    fn map_value(&mut self) -> stream::Result {
        self.map_value_begin().map(|_| ())
    }

    #[inline]
    fn map_end(&mut self) -> stream::Result {
        self.map_end()
    }

    #[inline]
    fn seq_begin(&mut self, len: Option<usize>) -> stream::Result {
        self.seq_begin(len)
    }

    #[inline]
    fn seq_elem(&mut self) -> stream::Result {
        self.seq_elem_begin().map(|_| ())
    }

    #[inline]
    fn seq_end(&mut self) -> stream::Result {
        self.seq_end()
    }
}

/**
A borrowed stream wrapper.

This is the result of calling `OwnedStream.borrow_mut`.
*/
pub struct RefMutStream<'a>(RefMutCollect<'a>);

impl<'a> RefMutStream<'a> {
    #[inline]
    pub(crate) fn new(collect: RefMutCollect<'a>) -> Self {
        RefMutStream(collect)
    }

    /**
    Stream a value.
    */
    #[inline]
    pub fn any(&mut self, v: impl Value) -> stream::Result {
        self.0.any(v)
    }

    /**
    Stream a format.
    */
    #[inline]
    pub fn fmt(&mut self, f: Arguments) -> stream::Result {
        self.0.fmt(f)
    }

    /**
    Stream a signed integer.
    */
    #[inline]
    pub fn i64(&mut self, v: i64) -> stream::Result {
        self.0.i64(v)
    }

    /**
    Stream an unsigned integer.
    */
    #[inline]
    pub fn u64(&mut self, v: u64) -> stream::Result {
        self.0.u64(v)
    }

    /**
    Stream a 128-bit signed integer.
    */
    #[inline]
    pub fn i128(&mut self, v: i128) -> stream::Result {
        self.0.i128(v)
    }

    /**
    Stream a 128-bit unsigned integer.
    */
    #[inline]
    pub fn u128(&mut self, v: u128) -> stream::Result {
        self.0.u128(v)
    }

    /**
    Stream a floating point value.
    */
    #[inline]
    pub fn f64(&mut self, v: f64) -> stream::Result {
        self.0.f64(v)
    }

    /**
    Stream a boolean.
    */
    #[inline]
    pub fn bool(&mut self, v: bool) -> stream::Result {
        self.0.bool(v)
    }

    /**
    Stream a unicode character.
    */
    #[inline]
    pub fn char(&mut self, v: char) -> stream::Result {
        self.0.char(v)
    }

    /**
    Stream a UTF8 string.
    */
    #[inline]
    pub fn str(&mut self, v: &str) -> stream::Result {
        self.0.str(v)
    }

    /**
    Stream an empty value.
    */
    #[inline]
    pub fn none(&mut self) -> stream::Result {
        self.0.none()
    }

    /**
    Begin a map.
    */
    #[inline]
    pub fn map_begin(&mut self, len: Option<usize>) -> stream::Result {
        self.0.map_begin(len)
    }

    /**
    Stream a map key.
    */
    #[inline]
    pub fn map_key(&mut self, k: impl Value) -> stream::Result {
        self.0.map_key(k)
    }

    /**
    Stream a map value.
    */
    #[inline]
    pub fn map_value(&mut self, v: impl Value) -> stream::Result {
        self.0.map_value(v)
    }

    /**
    End a map.
    */
    #[inline]
    pub fn map_end(&mut self) -> stream::Result {
        self.0.map_end()
    }

    /**
    Begin a sequence.
    */
    #[inline]
    pub fn seq_begin(&mut self, len: Option<usize>) -> stream::Result {
        self.0.seq_begin(len)
    }

    /**
    Stream a sequence element.
    */
    #[inline]
    pub fn seq_elem(&mut self, v: impl Value) -> stream::Result {
        self.0.seq_elem(v)
    }

    /**
    End a sequence.
    */
    #[inline]
    pub fn seq_end(&mut self) -> stream::Result {
        self.0.seq_end()
    }
}

impl<'a> RefMutStream<'a> {
    /**
    Begin a map key.
    */
    #[inline]
    pub fn map_key_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.map_key_begin()?;

        Ok(self)
    }

    /**
    Begin a map value.
    */
    #[inline]
    pub fn map_value_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.map_value_begin()?;

        Ok(self)
    }

    /**
    Begin a sequence element.
    */
    #[inline]
    pub fn seq_elem_begin(&mut self) -> Result<&mut Self, Error> {
        self.0.seq_elem_begin()?;

        Ok(self)
    }
}

impl<'a> Stream for RefMutStream<'a> {
    #[inline]
    fn fmt(&mut self, args: Arguments) -> stream::Result {
        self.fmt(args)
    }

    #[inline]
    fn i64(&mut self, v: i64) -> stream::Result {
        self.i64(v)
    }

    #[inline]
    fn u64(&mut self, v: u64) -> stream::Result {
        self.u64(v)
    }

    #[inline]
    fn i128(&mut self, v: i128) -> stream::Result {
        self.i128(v)
    }

    #[inline]
    fn u128(&mut self, v: u128) -> stream::Result {
        self.u128(v)
    }

    #[inline]
    fn f64(&mut self, v: f64) -> stream::Result {
        self.f64(v)
    }

    #[inline]
    fn bool(&mut self, v: bool) -> stream::Result {
        self.bool(v)
    }

    #[inline]
    fn char(&mut self, v: char) -> stream::Result {
        self.char(v)
    }

    #[inline]
    fn str(&mut self, v: &str) -> stream::Result {
        self.str(v)
    }

    #[inline]
    fn none(&mut self) -> stream::Result {
        self.none()
    }

    #[inline]
    fn map_begin(&mut self, len: Option<usize>) -> stream::Result {
        self.map_begin(len)
    }

    #[inline]
    fn map_key(&mut self) -> stream::Result {
        self.map_key_begin().map(|_| ())
    }

    #[inline]
    fn map_value(&mut self) -> stream::Result {
        self.map_value_begin().map(|_| ())
    }

    #[inline]
    fn map_end(&mut self) -> stream::Result {
        self.map_end()
    }

    #[inline]
    fn seq_begin(&mut self, len: Option<usize>) -> stream::Result {
        self.seq_begin(len)
    }

    #[inline]
    fn seq_elem(&mut self) -> stream::Result {
        self.seq_elem_begin().map(|_| ())
    }

    #[inline]
    fn seq_end(&mut self) -> stream::Result {
        self.seq_end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::stream::Stack;

    #[test]
    fn owned_stream_method_resolution() {
        fn takes_owned_stream(mut stream: OwnedStream<impl Stream>) -> stream::Result {
            stream.map_begin(None)?;
            stream.map_key("key")?;
            stream.map_value(42)?;
            stream.map_end()
        }

        fn takes_stream(mut stream: impl Stream) -> stream::Result {
            stream.map_begin(None)?;
            stream.map_key()?;
            stream.str("key")?;
            stream.map_value()?;
            stream.i64(42)?;
            stream.map_end()
        }

        takes_owned_stream(OwnedStream::new(Stack::default())).expect("failed to use owned stream");
        takes_stream(OwnedStream::new(Stack::default())).expect("failed to use stream");
    }

    #[test]
    fn ref_mut_stream_method_resolution() {
        fn takes_ref_mut_stream(mut stream: RefMutStream) -> stream::Result {
            stream.map_begin(None)?;
            stream.map_key("key")?;
            stream.map_value(42)?;
            stream.map_end()
        }

        fn takes_stream(mut stream: impl Stream) -> stream::Result {
            stream.map_begin(None)?;
            stream.map_key()?;
            stream.str("key")?;
            stream.map_value()?;
            stream.i64(42)?;
            stream.map_end()
        }

        takes_ref_mut_stream(OwnedStream::new(Stack::default()).borrow_mut())
            .expect("failed to use ref mut stream");
        takes_stream(OwnedStream::new(Stack::default()).borrow_mut())
            .expect("failed to use stream");
    }
}
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use crate::stream::owned::RefMutStream;
    
    #[test]
    fn test_rug() {
        let mut p0: RefMutStream<'_> = unimplemented!();
        let p1: u64 = 42;
        
        p0.u64(p1);
    }
}#[cfg(test)]
mod tests_rug_208 {
    use super::*;
    use crate::stream::owned::RefMutStream;

    #[test]
    fn test_rug() {
        let mut p0: RefMutStream = unimplemented!(); // Placeholder for RefMutStream<'a>
        let p1: i128 = 42; // Sample i128 value

        p0.i128(p1);
    }
}#[cfg(test)]
mod tests_rug_209 {
    use super::*;
    use stream::owned::RefMutStream;

    #[test]
    fn test_u128() {
        let mut p0: stream::owned::RefMutStream<'static> = todo!();
        let p1: u128 = 1234567890;

        p0.u128(p1);
    }
}#[cfg(test)]
mod tests_rug_210 {
    use super::*;
    use crate::stream::owned::RefMutStream;
    use crate::stream::Result;

    #[test]
    fn test_rug() {
        let mut p0: RefMutStream<'_> = todo!("construct p0 using RefMutStream in RefMutStream::<'a>::any");
        let p1: f64 = 3.14;

        p0.f64(p1);
    }
}#[cfg(test)]
mod tests_rug_211 {
    use super::*;
    use crate::stream::owned::RefMutStream;

    #[test]
    fn test_bool() {
        let mut p0: RefMutStream = unimplemented!(); // Fill in with appropriate RefMutStream value
        let p1: bool = true; // Sample boolean value

        p0.bool(p1);
    }
}#[cfg(test)]
mod tests_rug_212 {
    use super::*;
    use crate::stream::owned::RefMutStream;
    
    #[test]
    fn test_char() {
        let mut p0: RefMutStream = unimplemented!();
        let p1: char = 'a';

        p0.char(p1);
    }
}#[cfg(test)]
mod tests_rug_213 {
    use super::*;
    use crate::stream;

    #[test]
    fn test_str() {
        let mut p0: stream::owned::RefMutStream = unimplemented!();
        let p1: &str = "Sample string";

        p0.str(&p1);

    }
}#[cfg(test)]
mod tests_rug_214 {
    use super::*;
    use crate::stream;

    #[test]
    fn test_rug() {
        let mut p0: stream::owned::RefMutStream<'_> = unimplemented!();

        p0.none();
    }
}        
#[cfg(test)]
mod tests_rug_215 {
    use super::*;
    use crate::stream::owned::RefMutStream;
    use crate::stream;
    
    #[test]
    fn test_rug() {
        let mut p0: RefMutStream = unimplemented!();
        let p1: Option<usize> = Some(42);

        p0.map_begin(p1);
    }
}#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::stream::stack;

    #[test]
    fn test_rug() {
        let mut p0: stream::owned::RefMutStream<'_> = unimplemented!();
        let mut p1: core::option::Option<usize> = Some(42);

        p0.seq_begin(p1);

    }
}#[cfg(test)]
mod tests_rug_223 {
    use super::*;
    use crate::stream::owned::RefMutStream;
    
    #[test]
    fn test_rug() {
        let mut p0: RefMutStream<'_> = unimplemented!();
        
        stream::owned::RefMutStream::<'_>::map_value_begin(&mut p0).unwrap();

        // Add assertions or additional test code if needed
    }
}#[cfg(test)]
mod tests_rug_231 {
    use super::*;
    use crate::Stream;
    use stream::owned::RefMutStream;

    #[test]
    fn test_char() {
        let mut p0: RefMutStream<'static> = unimplemented!();
        let p1: char = 'a';

        <RefMutStream<'static> as Stream>::char(&mut p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    use crate::Stream;
    use stream::owned::RefMutStream;
    use core::option::Option;
    
    #[test]
    fn test_rug() {
        let mut p0: RefMutStream<'_> = unimplemented!();
        let p1: Option<usize> = Some(42);

        p0.seq_begin(p1);

    }
}
#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    use crate::Stream;

    #[test]
    fn test_seq_end() {
        let mut p0: stream::owned::RefMutStream<'_> = unimplemented!();

        <stream::owned::RefMutStream<'_>>::seq_end(&mut p0).unwrap();
    }
}