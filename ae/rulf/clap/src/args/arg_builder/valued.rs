use std::ffi::{OsStr, OsString};
use std::rc::Rc;
use map::VecMap;
use Arg;
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct Valued<'a, 'b>
where
    'a: 'b,
{
    pub possible_vals: Option<Vec<&'b str>>,
    pub val_names: Option<VecMap<&'b str>>,
    pub num_vals: Option<u64>,
    pub max_vals: Option<u64>,
    pub min_vals: Option<u64>,
    pub validator: Option<Rc<Fn(String) -> Result<(), String>>>,
    pub validator_os: Option<Rc<Fn(&OsStr) -> Result<(), OsString>>>,
    pub val_delim: Option<char>,
    pub default_val: Option<&'b OsStr>,
    pub default_vals_ifs: Option<VecMap<(&'a str, Option<&'b OsStr>, &'b OsStr)>>,
    pub env: Option<(&'a OsStr, Option<OsString>)>,
    pub terminator: Option<&'b str>,
}
impl<'n, 'e> Default for Valued<'n, 'e> {
    fn default() -> Self {
        Valued {
            possible_vals: None,
            num_vals: None,
            min_vals: None,
            max_vals: None,
            val_names: None,
            validator: None,
            validator_os: None,
            val_delim: None,
            default_val: None,
            default_vals_ifs: None,
            env: None,
            terminator: None,
        }
    }
}
impl<'n, 'e> Valued<'n, 'e> {
    pub fn fill_in(&mut self) {
        if let Some(ref vec) = self.val_names {
            if vec.len() > 1 {
                self.num_vals = Some(vec.len() as u64);
            }
        }
    }
}
impl<'n, 'e, 'z> From<&'z Arg<'n, 'e>> for Valued<'n, 'e> {
    fn from(a: &'z Arg<'n, 'e>) -> Self {
        let mut v = a.v.clone();
        if let Some(ref vec) = a.v.val_names {
            if vec.len() > 1 {
                v.num_vals = Some(vec.len() as u64);
            }
        }
        v
    }
}
#[cfg(test)]
mod tests_rug_468 {
    use super::*;
    use crate::args::arg_builder::valued::Valued;
    use crate::std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_468_rrrruuuugggg_test_default = 0;
        let default_valued: Valued<'static, 'static> = Default::default();
        let _rug_ed_tests_rug_468_rrrruuuugggg_test_default = 0;
    }
}
mod tests_rug_469 {
    use super::*;
    use crate::args::arg_builder::valued::Valued;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_469_rrrruuuugggg_test_rug = 0;
        let mut p0: Valued<'_, '_> = Valued::<'_, '_> {
            val_names: None,
            num_vals: None,
            ..Default::default()
        };
        Valued::fill_in(&mut p0);
        let _rug_ed_tests_rug_469_rrrruuuugggg_test_rug = 0;
    }
}
