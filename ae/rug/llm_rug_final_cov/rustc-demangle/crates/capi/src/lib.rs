extern crate rustc_demangle;
use std::io::Write;
use std::os::raw::{c_char, c_int};
/// C-style interface for demangling.
/// Demangles symbol given in `mangled` argument into `out` buffer
///
/// Unsafe as it handles buffers by raw pointers.
///
/// Returns 0 if `mangled` is not Rust symbol or if `out` buffer is too small
/// Returns 1 otherwise
#[no_mangle]
pub unsafe extern "C" fn rustc_demangle(
    mangled: *const c_char,
    out: *mut c_char,
    out_size: usize,
) -> c_int {
    let mangled_str = match std::ffi::CStr::from_ptr(mangled).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    match rustc_demangle::try_demangle(mangled_str) {
        Ok(demangle) => {
            let mut out_slice = std::slice::from_raw_parts_mut(out as *mut u8, out_size);
            match write!(out_slice, "{:#}\0", demangle) {
                Ok(_) => return 1,
                Err(_) => return 0,
            }
        }
        Err(_) => return 0,
    }
}
#[cfg(test)]
mod tests {
    use std;
    use std::os::raw::c_char;
    #[test]
    fn demangle_c_str_large() {
        let mangled = "_ZN4testE\0";
        let mut out_buf: Vec<u8> = vec![42; 8];
        let res = unsafe {
            super::rustc_demangle(
                mangled.as_ptr() as *const c_char,
                out_buf.as_mut_ptr() as *mut c_char,
                8,
            )
        };
        assert_eq!(res, 1);
        let out_str = std::str::from_utf8(&out_buf[..5]).unwrap();
        assert_eq!(out_str, "test\0");
    }
    #[test]
    fn demangle_c_str_exact() {
        let mangled = "_ZN4testE\0";
        let mut out_buf: Vec<u8> = vec![42; 8];
        let res = unsafe {
            super::rustc_demangle(
                mangled.as_ptr() as *const c_char,
                out_buf.as_mut_ptr() as *mut c_char,
                5,
            )
        };
        assert_eq!(res, 1);
        let out_str = std::str::from_utf8(&out_buf).unwrap();
        assert_eq!(out_str, "test\0***");
    }
    #[test]
    fn demangle_c_str_small() {
        let mangled = "_ZN4testE\0";
        let mut out_buf: Vec<u8> = vec![42; 8];
        let res = unsafe {
            super::rustc_demangle(
                mangled.as_ptr() as *const c_char,
                out_buf.as_mut_ptr() as *mut c_char,
                4,
            )
        };
        assert_eq!(res, 0);
        let out_str = std::str::from_utf8(&out_buf[4..]).unwrap();
        assert_eq!(out_str, "****");
    }
    #[test]
    fn demangle_c_str_smaller() {
        let mangled = "_ZN4testE\0";
        let mut out_buf: Vec<u8> = vec![42; 8];
        let res = unsafe {
            super::rustc_demangle(
                mangled.as_ptr() as *const c_char,
                out_buf.as_mut_ptr() as *mut c_char,
                3,
            )
        };
        assert_eq!(res, 0);
        let out_str = std::str::from_utf8(&out_buf[3..]).unwrap();
        assert_eq!(out_str, "*****");
    }
    #[test]
    fn demangle_c_str_zero() {
        let mangled = "_ZN4testE\0";
        let mut out_buf: Vec<u8> = vec![42; 8];
        let res = unsafe {
            super::rustc_demangle(
                mangled.as_ptr() as *const c_char,
                out_buf.as_mut_ptr() as *mut c_char,
                0,
            )
        };
        assert_eq!(res, 0);
        let out_str = std::str::from_utf8(&out_buf).unwrap();
        assert_eq!(out_str, "********");
    }
    #[test]
    fn demangle_c_str_not_rust_symbol() {
        let mangled = "la la la\0";
        let mut out_buf: Vec<u8> = vec![42; 8];
        let res = unsafe {
            super::rustc_demangle(
                mangled.as_ptr() as *const c_char,
                out_buf.as_mut_ptr() as *mut c_char,
                8,
            )
        };
        assert_eq!(res, 0);
    }
    #[test]
    fn demangle_c_str_null() {
        let mangled = "\0";
        let mut out_buf: Vec<u8> = vec![42; 8];
        let res = unsafe {
            super::rustc_demangle(
                mangled.as_ptr() as *const c_char,
                out_buf.as_mut_ptr() as *mut c_char,
                8,
            )
        };
        assert_eq!(res, 0);
    }
    #[test]
    fn demangle_c_str_invalid_utf8() {
        let mangled = [116, 101, 115, 116, 165, 0];
        let mut out_buf: Vec<u8> = vec![42; 8];
        let res = unsafe {
            super::rustc_demangle(
                mangled.as_ptr() as *const c_char,
                out_buf.as_mut_ptr() as *mut c_char,
                8,
            )
        };
        assert_eq!(res, 0);
    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use libc::{c_char, c_int};
    #[test]
    fn test_rustc_demangle() {
        let _rug_st_tests_rug_49_rrrruuuugggg_test_rustc_demangle = 0;
        let rug_fuzz_0 = "6rustc_Test17h8b38ac0bcdc1444eE";
        let rug_fuzz_1 = 0u8;
        let mangled_str = rug_fuzz_0;
        let c_mangled = std::ffi::CString::new(mangled_str).unwrap();
        let mangled_ptr = c_mangled.as_ptr();
        let out_str = [rug_fuzz_1; 128];
        let out_ptr = out_str.as_ptr() as *mut c_char;
        let out_size = out_str.len();
        unsafe {
            let result: c_int = crate::rustc_demangle(mangled_ptr, out_ptr, out_size);
            debug_assert_eq!(result, 1);
            let demangled_str = std::ffi::CStr::from_ptr(out_ptr).to_string_lossy();
            debug_assert_eq!(demangled_str, "rustc_Test::h8b38ac0bcdc1444e");
        }
        let _rug_ed_tests_rug_49_rrrruuuugggg_test_rustc_demangle = 0;
    }
}
