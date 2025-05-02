use crate::{
    collect::{
        self,
        Collect,
        OwnedCollect,
    },
    value,
};

pub(crate) struct Value<'a> {
    value: &'a dyn value::Value,
}

impl<'a> Value<'a> {
    #[inline]
    pub(crate) fn new(value: &'a impl value::Value) -> Self {
        Value { value }
    }

    #[inline]
    pub(crate) fn stream(self, stream: impl Collect) -> collect::Result {
        let mut stream = OwnedCollect::new(stream);

        self.value
            .stream(&mut value::Stream::new(stream.borrow_mut()))?;

        Ok(())
    }
}
#[cfg(test)]
mod tests_rug_162 {
    use super::*;
    use crate::value::{Value, Stream, Result};
    
    struct DummyValue;
    
    impl Value for DummyValue {
        fn stream(&self, _stream: &mut Stream) -> Result {
            unimplemented!()
        }
    }
    
    #[test]
    fn test_new_value() {
        let dummy_value = DummyValue;
        <collect::value::Value<'_>>::new(&dummy_value);
    }
}