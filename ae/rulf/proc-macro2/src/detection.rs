use std::panic::{self, PanicInfo};
use std::sync::atomic::*;
use std::sync::Once;
static WORKS: AtomicUsize = AtomicUsize::new(0);
static INIT: Once = Once::new();
pub(crate) fn inside_proc_macro() -> bool {
    match WORKS.load(Ordering::SeqCst) {
        1 => return false,
        2 => return true,
        _ => {}
    }
    INIT.call_once(initialize);
    inside_proc_macro()
}
pub(crate) fn force_fallback() {
    WORKS.store(1, Ordering::SeqCst);
}
pub(crate) fn unforce_fallback() {
    initialize();
}
fn initialize() {
    type PanicHook = dyn Fn(&PanicInfo) + Sync + Send + 'static;
    let null_hook: Box<PanicHook> = Box::new(|_panic_info| {});
    let sanity_check = &*null_hook as *const PanicHook;
    let original_hook = panic::take_hook();
    panic::set_hook(null_hook);
    let works = panic::catch_unwind(proc_macro::Span::call_site).is_ok();
    WORKS.store(works as usize + 1, Ordering::SeqCst);
    let hopefully_null_hook = panic::take_hook();
    panic::set_hook(original_hook);
    if sanity_check != &*hopefully_null_hook {
        panic!("observed race condition in proc_macro2::inside_proc_macro");
    }
}
#[cfg(test)]
mod tests_llm_16_221_llm_16_220 {
    #[test]
    fn test_initialize() {
        let _rug_st_tests_llm_16_221_llm_16_220_rrrruuuugggg_test_initialize = 0;
        super::initialize();
        let _rug_ed_tests_llm_16_221_llm_16_220_rrrruuuugggg_test_initialize = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_224 {
    use super::*;
    use crate::*;
    #[test]
    fn test_unforce_fallback() {
        let _rug_st_tests_llm_16_224_rrrruuuugggg_test_unforce_fallback = 0;
        unforce_fallback();
        let _rug_ed_tests_llm_16_224_rrrruuuugggg_test_unforce_fallback = 0;
    }
}
#[cfg(test)]
mod tests_rug_34 {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Once;
    static WORKS: AtomicUsize = AtomicUsize::new(0);
    static INIT: Once = Once::new();
    fn initialize() {
        let _rug_st_tests_rug_34_rrrruuuugggg_initialize = 0;
        let rug_fuzz_0 = 1;
        WORKS.store(rug_fuzz_0, Ordering::SeqCst);
        let _rug_ed_tests_rug_34_rrrruuuugggg_initialize = 0;
    }
    #[test]
    fn test_inside_proc_macro() {
        let _rug_st_tests_rug_34_rrrruuuugggg_test_inside_proc_macro = 0;
        let rug_fuzz_0 = 2;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 2;
        let rug_fuzz_5 = 1;
        debug_assert_eq!(inside_proc_macro(), false);
        WORKS.store(rug_fuzz_0, Ordering::SeqCst);
        debug_assert_eq!(inside_proc_macro(), true);
        WORKS.store(rug_fuzz_1, Ordering::SeqCst);
        debug_assert_eq!(inside_proc_macro(), false);
        INIT.call_once(initialize);
        debug_assert_eq!(inside_proc_macro(), false);
        WORKS.store(rug_fuzz_2, Ordering::SeqCst);
        debug_assert_eq!(inside_proc_macro(), true);
        WORKS.store(rug_fuzz_3, Ordering::SeqCst);
        INIT.call_once(initialize);
        debug_assert_eq!(inside_proc_macro(), false);
        WORKS.store(rug_fuzz_4, Ordering::SeqCst);
        INIT.call_once(initialize);
        debug_assert_eq!(inside_proc_macro(), false);
        WORKS.store(rug_fuzz_5, Ordering::SeqCst);
        INIT.call_once(|| panic!("Should not initialize again"));
        debug_assert_eq!(inside_proc_macro(), true);
        let _rug_ed_tests_rug_34_rrrruuuugggg_test_inside_proc_macro = 0;
    }
}
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[test]
    fn test_force_fallback() {
        const WORKS: AtomicUsize = AtomicUsize::new(0);
        force_fallback();
        assert_eq!(WORKS.load(Ordering::SeqCst), 1);
    }
}
