use std::collections::hash_map::{Entry, Iter};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::mem;
use std::ops::Deref;
use args::settings::ArgSettings;
use args::AnyArg;
use args::{ArgMatches, MatchedArg, SubCommand};
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ArgMatcher<'a>(pub ArgMatches<'a>);
impl<'a> Default for ArgMatcher<'a> {
    fn default() -> Self {
        ArgMatcher(ArgMatches::default())
    }
}
impl<'a> ArgMatcher<'a> {
    pub fn new() -> Self {
        ArgMatcher::default()
    }
    pub fn process_arg_overrides<'b>(
        &mut self,
        a: Option<&AnyArg<'a, 'b>>,
        overrides: &mut Vec<(&'b str, &'a str)>,
        required: &mut Vec<&'a str>,
        check_all: bool,
    ) {
        debugln!(
            "ArgMatcher::process_arg_overrides:{:?};", a.map_or(None, | a | Some(a
            .name()))
        );
        if let Some(aa) = a {
            let mut self_done = false;
            if let Some(a_overrides) = aa.overrides() {
                for overr in a_overrides {
                    debugln!("ArgMatcher::process_arg_overrides:iter:{};", overr);
                    if overr == &aa.name() {
                        self_done = true;
                        self.handle_self_overrides(a);
                    } else if self.is_present(overr) {
                        debugln!(
                            "ArgMatcher::process_arg_overrides:iter:{}: removing from matches;",
                            overr
                        );
                        self.remove(overr);
                        for i in (0..required.len()).rev() {
                            if &required[i] == overr {
                                debugln!(
                                    "ArgMatcher::process_arg_overrides:iter:{}: removing required;",
                                    overr
                                );
                                required.swap_remove(i);
                                break;
                            }
                        }
                        overrides.push((overr, aa.name()));
                    } else {
                        overrides.push((overr, aa.name()));
                    }
                }
            }
            if check_all && !self_done {
                self.handle_self_overrides(a);
            }
        }
    }
    pub fn handle_self_overrides<'b>(&mut self, a: Option<&AnyArg<'a, 'b>>) {
        debugln!(
            "ArgMatcher::handle_self_overrides:{:?};", a.map_or(None, | a | Some(a
            .name()))
        );
        if let Some(aa) = a {
            if !aa.has_switch() || aa.is_set(ArgSettings::Multiple) {
                return;
            }
            if let Some(ma) = self.get_mut(aa.name()) {
                if ma.vals.len() > 1 {
                    ma.vals.remove(0);
                    ma.occurs = 1;
                } else if !aa.takes_value() && ma.occurs > 1 {
                    ma.occurs = 1;
                }
            }
        }
    }
    pub fn is_present(&self, name: &str) -> bool {
        self.0.is_present(name)
    }
    pub fn propagate_globals(&mut self, global_arg_vec: &[&'a str]) {
        debugln!("ArgMatcher::get_global_values: global_arg_vec={:?}", global_arg_vec);
        let mut vals_map = HashMap::new();
        self.fill_in_global_values(global_arg_vec, &mut vals_map);
    }
    fn fill_in_global_values(
        &mut self,
        global_arg_vec: &[&'a str],
        vals_map: &mut HashMap<&'a str, MatchedArg>,
    ) {
        for global_arg in global_arg_vec {
            if let Some(ma) = self.get(global_arg) {
                let to_update = if let Some(parent_ma) = vals_map.get(global_arg) {
                    if parent_ma.occurs > 0 && ma.occurs == 0 {
                        parent_ma.clone()
                    } else {
                        ma.clone()
                    }
                } else {
                    ma.clone()
                };
                vals_map.insert(global_arg, to_update);
            }
        }
        if let Some(ref mut sc) = self.0.subcommand {
            let mut am = ArgMatcher(mem::replace(&mut sc.matches, ArgMatches::new()));
            am.fill_in_global_values(global_arg_vec, vals_map);
            mem::swap(&mut am.0, &mut sc.matches);
        }
        for (name, matched_arg) in vals_map.into_iter() {
            self.0.args.insert(name, matched_arg.clone());
        }
    }
    pub fn get_mut(&mut self, arg: &str) -> Option<&mut MatchedArg> {
        self.0.args.get_mut(arg)
    }
    pub fn get(&self, arg: &str) -> Option<&MatchedArg> {
        self.0.args.get(arg)
    }
    pub fn remove(&mut self, arg: &str) {
        self.0.args.remove(arg);
    }
    pub fn remove_all(&mut self, args: &[&str]) {
        for &arg in args {
            self.0.args.remove(arg);
        }
    }
    pub fn insert(&mut self, name: &'a str) {
        self.0.args.insert(name, MatchedArg::new());
    }
    pub fn contains(&self, arg: &str) -> bool {
        self.0.args.contains_key(arg)
    }
    pub fn is_empty(&self) -> bool {
        self.0.args.is_empty()
    }
    pub fn usage(&mut self, usage: String) {
        self.0.usage = Some(usage);
    }
    pub fn arg_names(&'a self) -> Vec<&'a str> {
        self.0.args.keys().map(Deref::deref).collect()
    }
    pub fn entry(&mut self, arg: &'a str) -> Entry<&'a str, MatchedArg> {
        self.0.args.entry(arg)
    }
    pub fn subcommand(&mut self, sc: SubCommand<'a>) {
        self.0.subcommand = Some(Box::new(sc));
    }
    pub fn subcommand_name(&self) -> Option<&str> {
        self.0.subcommand_name()
    }
    pub fn iter(&self) -> Iter<&str, MatchedArg> {
        self.0.args.iter()
    }
    pub fn inc_occurrence_of(&mut self, arg: &'a str) {
        debugln!("ArgMatcher::inc_occurrence_of: arg={}", arg);
        if let Some(a) = self.get_mut(arg) {
            a.occurs += 1;
            return;
        }
        debugln!("ArgMatcher::inc_occurrence_of: first instance");
        self.insert(arg);
    }
    pub fn inc_occurrences_of(&mut self, args: &[&'a str]) {
        debugln!("ArgMatcher::inc_occurrences_of: args={:?}", args);
        for arg in args {
            self.inc_occurrence_of(arg);
        }
    }
    pub fn add_val_to(&mut self, arg: &'a str, val: &OsStr) {
        let ma = self
            .entry(arg)
            .or_insert(MatchedArg {
                occurs: 0,
                indices: Vec::with_capacity(1),
                vals: Vec::with_capacity(1),
            });
        ma.vals.push(val.to_owned());
    }
    pub fn add_index_to(&mut self, arg: &'a str, idx: usize) {
        let ma = self
            .entry(arg)
            .or_insert(MatchedArg {
                occurs: 0,
                indices: Vec::with_capacity(1),
                vals: Vec::new(),
            });
        ma.indices.push(idx);
    }
    pub fn needs_more_vals<'b, A>(&self, o: &A) -> bool
    where
        A: AnyArg<'a, 'b>,
    {
        debugln!("ArgMatcher::needs_more_vals: o={}", o.name());
        if let Some(ma) = self.get(o.name()) {
            if let Some(num) = o.num_vals() {
                debugln!("ArgMatcher::needs_more_vals: num_vals...{}", num);
                return if o.is_set(ArgSettings::Multiple) {
                    ((ma.vals.len() as u64) % num) != 0
                } else {
                    num != (ma.vals.len() as u64)
                };
            } else if let Some(num) = o.max_vals() {
                debugln!("ArgMatcher::needs_more_vals: max_vals...{}", num);
                return !((ma.vals.len() as u64) > num);
            } else if o.min_vals().is_some() {
                debugln!("ArgMatcher::needs_more_vals: min_vals...true");
                return true;
            }
            return o.is_set(ArgSettings::Multiple);
        }
        true
    }
}
impl<'a> Into<ArgMatches<'a>> for ArgMatcher<'a> {
    fn into(self) -> ArgMatches<'a> {
        self.0
    }
}
#[cfg(test)]
mod tests_rug_471 {
    use super::*;
    use crate::args::arg_matcher::{ArgMatcher, ArgMatches};
    use crate::std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_471_rrrruuuugggg_test_rug = 0;
        let _ = <ArgMatcher<'static> as Default>::default();
        let _rug_ed_tests_rug_471_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_472 {
    use super::*;
    use args::arg_matcher::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_472_rrrruuuugggg_test_rug = 0;
        let arg_matcher: ArgMatcher = ArgMatcher::new();
        let _rug_ed_tests_rug_472_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_474 {
    use super::*;
    use crate::args::any_arg::AnyArg;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_474_rrrruuuugggg_test_rug = 0;
        let mut p0: crate::args::arg_matcher::ArgMatcher<'static> = Default::default();
        let p1: Option<&dyn AnyArg<'static, 'static>> = None;
        crate::args::arg_matcher::ArgMatcher::handle_self_overrides(&mut p0, p1);
        let _rug_ed_tests_rug_474_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_475 {
    use super::*;
    use crate::args::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_475_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "some_arg_name";
        let mut p0: ArgMatcher<'static> = ArgMatcher::default();
        let p1: &str = rug_fuzz_0;
        let _ = p0.is_present(p1);
        let _rug_ed_tests_rug_475_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_476 {
    use super::*;
    use crate::args::ArgMatcher;
    use std::collections::HashMap;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_476_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "global1";
        let rug_fuzz_1 = "global2";
        let mut p0 = ArgMatcher::default();
        let p1: [&str; 2] = [rug_fuzz_0, rug_fuzz_1];
        p0.propagate_globals(&p1);
        let _rug_ed_tests_rug_476_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_477 {
    use super::*;
    use std::collections::HashMap;
    use crate::args::arg_matcher::{ArgMatcher, MatchedArg};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_477_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "global_arg1";
        let rug_fuzz_1 = "global_arg2";
        let rug_fuzz_2 = "global_arg3";
        let mut p0: ArgMatcher = unimplemented!();
        let p1: [&str; 3] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut p2: HashMap<&str, MatchedArg> = unimplemented!();
        p0.fill_in_global_values(&p1, &mut p2);
        let _rug_ed_tests_rug_477_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_478 {
    use super::*;
    use crate::args::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_478_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example_arg";
        let mut p0: ArgMatcher<'static> = ArgMatcher::<'static> {
            0: Default::default(),
        };
        let p1: &'static str = rug_fuzz_0;
        p0.get_mut(p1);
        let _rug_ed_tests_rug_478_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_479 {
    use super::*;
    use crate::args::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_479_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_arg";
        let arg_matcher = ArgMatcher::new();
        let arg = rug_fuzz_0;
        arg_matcher.get(arg);
        let _rug_ed_tests_rug_479_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_480 {
    use super::*;
    use crate::args::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_480_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example";
        let mut p0: ArgMatcher = ArgMatcher::new();
        let p1: &str = rug_fuzz_0;
        p0.remove(p1);
        let _rug_ed_tests_rug_480_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_481 {
    use super::*;
    use crate::args::arg_matcher::ArgMatcher;
    #[test]
    fn test_remove_all() {
        let _rug_st_tests_rug_481_rrrruuuugggg_test_remove_all = 0;
        let rug_fuzz_0 = "arg1";
        let rug_fuzz_1 = "arg2";
        let mut matcher = ArgMatcher::<'static>::default();
        let args = &[rug_fuzz_0, rug_fuzz_1];
        matcher.remove_all(args);
        let _rug_ed_tests_rug_481_rrrruuuugggg_test_remove_all = 0;
    }
}
#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use crate::args::{ArgMatcher, MatchedArg};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_482_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example_arg";
        let mut p0: ArgMatcher<'static> = ArgMatcher::new();
        let p1: &'static str = rug_fuzz_0;
        p0.insert(p1);
        let _rug_ed_tests_rug_482_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_483 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_483_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "arg_name";
        let mut p0 = crate::args::arg_matcher::ArgMatcher::<'_>::default();
        let p1 = rug_fuzz_0;
        p0.contains(&p1);
        let _rug_ed_tests_rug_483_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_484 {
    use super::*;
    use crate::args::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_484_rrrruuuugggg_test_rug = 0;
        let mut p0: ArgMatcher<'static> = ArgMatcher::new();
        ArgMatcher::is_empty(&p0);
        let _rug_ed_tests_rug_484_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_485 {
    use super::*;
    use crate::args::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_485_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "This is a usage message.";
        let mut p0: ArgMatcher<'static> = ArgMatcher::new();
        let p1: String = String::from(rug_fuzz_0);
        p0.usage(p1);
        let _rug_ed_tests_rug_485_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_486 {
    use super::*;
    use crate::args::arg_matcher::ArgMatcher;
    #[test]
    fn test_arg_names() {
        let _rug_st_tests_rug_486_rrrruuuugggg_test_arg_names = 0;
        let p0: ArgMatcher<'_> = unimplemented!();
        p0.arg_names();
        let _rug_ed_tests_rug_486_rrrruuuugggg_test_arg_names = 0;
    }
}
#[cfg(test)]
mod tests_rug_487 {
    use super::*;
    use crate::args::arg_matcher::{ArgMatcher, Entry};
    #[test]
    fn test_entry() {
        let _rug_st_tests_rug_487_rrrruuuugggg_test_entry = 0;
        let rug_fuzz_0 = "arg1";
        let mut p0: ArgMatcher = ArgMatcher::new();
        let p1: &str = rug_fuzz_0;
        p0.entry(p1);
        let _rug_ed_tests_rug_487_rrrruuuugggg_test_entry = 0;
    }
}
#[cfg(test)]
mod tests_rug_489 {
    use super::*;
    use crate::args::arg_matcher::ArgMatcher;
    #[test]
    fn test_subcommand_name() {
        let _rug_st_tests_rug_489_rrrruuuugggg_test_subcommand_name = 0;
        let mut p0: ArgMatcher<'static> = unimplemented!();
        p0.subcommand_name();
        let _rug_ed_tests_rug_489_rrrruuuugggg_test_subcommand_name = 0;
    }
}
#[cfg(test)]
mod tests_rug_490 {
    use super::*;
    use crate::args::{ArgMatcher, MatchedArg};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_490_rrrruuuugggg_test_rug = 0;
        let mut p0: ArgMatcher = unimplemented!();
        <ArgMatcher<'static>>::iter(&p0);
        let _rug_ed_tests_rug_490_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_491 {
    use super::*;
    use args::{arg_matcher, ArgMatcher};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_491_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "test_argument";
        let mut p0: ArgMatcher = arg_matcher::ArgMatcher::<'static>::new();
        let p1: &'static str = rug_fuzz_0;
        p0.inc_occurrence_of(p1);
        let _rug_ed_tests_rug_491_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_492 {
    use super::*;
    use args::arg_matcher::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_492_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "arg1";
        let rug_fuzz_1 = "arg2";
        let rug_fuzz_2 = "arg3";
        let mut p0: ArgMatcher<'static> = ArgMatcher::new();
        let p1: [&'static str; 3] = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        p0.inc_occurrences_of(&p1);
        let _rug_ed_tests_rug_492_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_493 {
    use super::*;
    use args::arg_matcher::ArgMatcher;
    use std::ffi::OsStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_493_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "arg1";
        let rug_fuzz_1 = "sample";
        let mut p0: ArgMatcher<'static> = ArgMatcher::new();
        let p1: &str = rug_fuzz_0;
        let mut p2 = OsStr::new(rug_fuzz_1);
        p0.add_val_to(p1, &p2);
        let _rug_ed_tests_rug_493_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_494 {
    use super::*;
    use crate::args::ArgMatcher;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_494_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "sample_arg";
        let rug_fuzz_1 = 0;
        let mut p0: ArgMatcher<'static> = ArgMatcher::new();
        let p1: &str = rug_fuzz_0;
        let p2: usize = rug_fuzz_1;
        p0.add_index_to(p1, p2);
        let _rug_ed_tests_rug_494_rrrruuuugggg_test_rug = 0;
    }
}
