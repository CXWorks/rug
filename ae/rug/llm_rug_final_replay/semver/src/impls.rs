use crate::backport::*;
use crate::identifier::Identifier;
use crate::{BuildMetadata, Comparator, Prerelease, VersionReq};
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::iter::FromIterator;
use core::ops::Deref;
impl Default for Identifier {
    fn default() -> Self {
        Identifier::empty()
    }
}
impl Eq for Identifier {}
impl Hash for Identifier {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher);
    }
}
impl Deref for Prerelease {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.identifier.as_str()
    }
}
impl Deref for BuildMetadata {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.identifier.as_str()
    }
}
impl PartialOrd for Prerelease {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, rhs))
    }
}
impl PartialOrd for BuildMetadata {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, rhs))
    }
}
impl Ord for Prerelease {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match self.is_empty() {
            true if rhs.is_empty() => return Ordering::Equal,
            true => return Ordering::Greater,
            false if rhs.is_empty() => return Ordering::Less,
            false => {}
        }
        let lhs = self.as_str().split('.');
        let mut rhs = rhs.as_str().split('.');
        for lhs in lhs {
            let rhs = match rhs.next() {
                None => return Ordering::Greater,
                Some(rhs) => rhs,
            };
            let string_cmp = || Ord::cmp(lhs, rhs);
            let is_ascii_digit = |b: u8| b.is_ascii_digit();
            let ordering = match (
                lhs.bytes().all(is_ascii_digit),
                rhs.bytes().all(is_ascii_digit),
            ) {
                (true, true) => Ord::cmp(&lhs.len(), &rhs.len()).then_with(string_cmp),
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                (false, false) => string_cmp(),
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        if rhs.next().is_none() { Ordering::Equal } else { Ordering::Less }
    }
}
impl Ord for BuildMetadata {
    fn cmp(&self, rhs: &Self) -> Ordering {
        let lhs = self.as_str().split('.');
        let mut rhs = rhs.as_str().split('.');
        for lhs in lhs {
            let rhs = match rhs.next() {
                None => return Ordering::Greater,
                Some(rhs) => rhs,
            };
            let is_ascii_digit = |b: u8| b.is_ascii_digit();
            let ordering = match (
                lhs.bytes().all(is_ascii_digit),
                rhs.bytes().all(is_ascii_digit),
            ) {
                (true, true) => {
                    let lhval = lhs.trim_start_matches('0');
                    let rhval = rhs.trim_start_matches('0');
                    Ord::cmp(&lhval.len(), &rhval.len())
                        .then_with(|| Ord::cmp(lhval, rhval))
                        .then_with(|| Ord::cmp(&lhs.len(), &rhs.len()))
                }
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                (false, false) => Ord::cmp(lhs, rhs),
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        if rhs.next().is_none() { Ordering::Equal } else { Ordering::Less }
    }
}
impl FromIterator<Comparator> for VersionReq {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Comparator>,
    {
        let comparators = Vec::from_iter(iter);
        VersionReq { comparators }
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::Identifier;
    use crate::identifier;
    use std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_default = 0;
        let result: Identifier = <Identifier as Default>::default();
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_rug_47 {
    use super::*;
    use crate::Prerelease;
    #[test]
    fn test_deref() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(&str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Prerelease = Prerelease::new(rug_fuzz_0).unwrap();
        <Prerelease as Deref>::deref(&p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::Prerelease;
    use std::cmp::PartialOrd;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Prerelease = Prerelease::new(rug_fuzz_0).unwrap();
        let mut p1: Prerelease = Prerelease::new(rug_fuzz_1).unwrap();
        p0.partial_cmp(&p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_51 {
    use super::*;
    use crate::impls::Prerelease;
    use std::cmp::Ord;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Prerelease = Prerelease::new(rug_fuzz_0).unwrap();
        let mut p1: Prerelease = Prerelease::new(rug_fuzz_1).unwrap();
        <Prerelease>::cmp(&mut p0, &mut p1);
             }
}
}
}    }
}
