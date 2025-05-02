use crate::{Comparator, Op, Version, VersionReq};
pub(crate) fn matches_req(req: &VersionReq, ver: &Version) -> bool {
    for cmp in &req.comparators {
        if !matches_impl(cmp, ver) {
            return false;
        }
    }
    if ver.pre.is_empty() {
        return true;
    }
    for cmp in &req.comparators {
        if pre_is_compatible(cmp, ver) {
            return true;
        }
    }
    false
}
pub(crate) fn matches_comparator(cmp: &Comparator, ver: &Version) -> bool {
    matches_impl(cmp, ver) && (ver.pre.is_empty() || pre_is_compatible(cmp, ver))
}
fn matches_impl(cmp: &Comparator, ver: &Version) -> bool {
    match cmp.op {
        Op::Exact | Op::Wildcard => matches_exact(cmp, ver),
        Op::Greater => matches_greater(cmp, ver),
        Op::GreaterEq => matches_exact(cmp, ver) || matches_greater(cmp, ver),
        Op::Less => matches_less(cmp, ver),
        Op::LessEq => matches_exact(cmp, ver) || matches_less(cmp, ver),
        Op::Tilde => matches_tilde(cmp, ver),
        Op::Caret => matches_caret(cmp, ver),
        #[cfg(no_non_exhaustive)]
        Op::__NonExhaustive => unreachable!(),
    }
}
fn matches_exact(cmp: &Comparator, ver: &Version) -> bool {
    if ver.major != cmp.major {
        return false;
    }
    if let Some(minor) = cmp.minor {
        if ver.minor != minor {
            return false;
        }
    }
    if let Some(patch) = cmp.patch {
        if ver.patch != patch {
            return false;
        }
    }
    ver.pre == cmp.pre
}
fn matches_greater(cmp: &Comparator, ver: &Version) -> bool {
    if ver.major != cmp.major {
        return ver.major > cmp.major;
    }
    match cmp.minor {
        None => return false,
        Some(minor) => {
            if ver.minor != minor {
                return ver.minor > minor;
            }
        }
    }
    match cmp.patch {
        None => return false,
        Some(patch) => {
            if ver.patch != patch {
                return ver.patch > patch;
            }
        }
    }
    ver.pre > cmp.pre
}
fn matches_less(cmp: &Comparator, ver: &Version) -> bool {
    if ver.major != cmp.major {
        return ver.major < cmp.major;
    }
    match cmp.minor {
        None => return false,
        Some(minor) => {
            if ver.minor != minor {
                return ver.minor < minor;
            }
        }
    }
    match cmp.patch {
        None => return false,
        Some(patch) => {
            if ver.patch != patch {
                return ver.patch < patch;
            }
        }
    }
    ver.pre < cmp.pre
}
fn matches_tilde(cmp: &Comparator, ver: &Version) -> bool {
    if ver.major != cmp.major {
        return false;
    }
    if let Some(minor) = cmp.minor {
        if ver.minor != minor {
            return false;
        }
    }
    if let Some(patch) = cmp.patch {
        if ver.patch != patch {
            return ver.patch > patch;
        }
    }
    ver.pre >= cmp.pre
}
fn matches_caret(cmp: &Comparator, ver: &Version) -> bool {
    if ver.major != cmp.major {
        return false;
    }
    let minor = match cmp.minor {
        None => return true,
        Some(minor) => minor,
    };
    let patch = match cmp.patch {
        None => {
            if cmp.major > 0 {
                return ver.minor >= minor;
            } else {
                return ver.minor == minor;
            }
        }
        Some(patch) => patch,
    };
    if cmp.major > 0 {
        if ver.minor != minor {
            return ver.minor > minor;
        } else if ver.patch != patch {
            return ver.patch > patch;
        }
    } else if minor > 0 {
        if ver.minor != minor {
            return false;
        } else if ver.patch != patch {
            return ver.patch > patch;
        }
    } else if ver.minor != minor || ver.patch != patch {
        return false;
    }
    ver.pre >= cmp.pre
}
fn pre_is_compatible(cmp: &Comparator, ver: &Version) -> bool {
    cmp.major == ver.major && cmp.minor == Some(ver.minor)
        && cmp.patch == Some(ver.patch) && !cmp.pre.is_empty()
}
#[cfg(test)]
mod tests_rug_3 {
    use super::*;
    use crate::{Version, VersionReq};
    #[test]
    fn test_matches_req() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(u64, u64, u64, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: VersionReq = VersionReq::default();
        let mut p1: Version = Version::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        debug_assert_eq!(matches_req(& p0, & p1), false);
        let mut p2: VersionReq = VersionReq::parse(rug_fuzz_3).unwrap();
        debug_assert_eq!(matches_req(& p2, & p1), true);
        let mut p3: VersionReq = VersionReq::parse(rug_fuzz_4).unwrap();
        debug_assert_eq!(matches_req(& p3, & p1), true);
        let mut p4: VersionReq = VersionReq::parse(rug_fuzz_5).unwrap();
        debug_assert_eq!(matches_req(& p4, & p1), true);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_4 {
    use crate::{Comparator, Version};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Comparator::parse(rug_fuzz_0).unwrap();
        let mut p1 = Version::parse(rug_fuzz_1).unwrap();
        crate::eval::matches_comparator(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_5 {
    use super::*;
    use crate::{Comparator, Version};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Comparator::parse(rug_fuzz_0).unwrap();
        let mut p1 = Version::parse(rug_fuzz_1).unwrap();
        crate::eval::matches_impl(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_6 {
    use super::*;
    use crate::{Comparator, Version};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Comparator::parse(rug_fuzz_0).unwrap();
        let mut p1 = Version::parse(rug_fuzz_1).unwrap();
        crate::eval::matches_exact(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_7 {
    use crate::{Comparator, Version};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Comparator::parse(rug_fuzz_0).unwrap();
        let mut p1 = Version::parse(rug_fuzz_1).unwrap();
        debug_assert!(crate ::eval::matches_greater(& p0, & p1));
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    use crate::{Comparator, Version};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Comparator::parse(rug_fuzz_0).unwrap();
        let mut p1 = Version::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(matches_less(& p0, & p1), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::{Comparator, Version};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Comparator::parse(rug_fuzz_0).unwrap();
        let mut p1 = Version::parse(rug_fuzz_1).unwrap();
        crate::eval::matches_tilde(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    use crate::{Comparator, Version};
    #[test]
    fn test_matches_caret() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = Comparator::parse(rug_fuzz_0).unwrap();
        let p1 = Version::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(matches_caret(& p0, & p1), false);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::{Comparator, Version};
    #[test]
    fn test_pre_is_compatible() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = Comparator::parse(rug_fuzz_0).unwrap();
        let mut p1 = Version::parse(rug_fuzz_1).unwrap();
        debug_assert_eq!(pre_is_compatible(& p0, & p1), false);
             }
}
}
}    }
}
