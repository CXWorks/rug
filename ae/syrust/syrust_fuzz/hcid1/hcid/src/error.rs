/// a simple struct(String) for reporting hcid errors
#[derive(Debug, PartialEq, Clone)]
pub struct HcidError(pub String);
/// hcid Result type
pub type HcidResult<T> = Result<T, HcidError>;
impl std::fmt::Display for HcidError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for HcidError {
    fn description(&self) -> &str {
        &self.0
    }
    fn cause(&self) -> Option<&std::error::Error> {
        None
    }
}
impl From<String> for HcidError {
    fn from(error: String) -> Self {
        Self(error)
    }
}
impl<'a> From<&'a str> for HcidError {
    fn from(error: &'a str) -> Self {
        Self(error.to_string())
    }
}
impl From<reed_solomon::DecoderError> for HcidError {
    fn from(error: reed_solomon::DecoderError) -> Self {
        Self(format!("{:?}", error))
    }
}
impl From<std::num::ParseIntError> for HcidError {
    fn from(error: std::num::ParseIntError) -> Self {
        Self(format!("{:?}", error))
    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    use crate::error::HcidError;
    use std::convert::From;
    use std::string::String;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: String = String::from(rug_fuzz_0);
        HcidError::from(p0);
             }
});    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::error::HcidError;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        HcidError::from(p0);
             }
});    }
}
