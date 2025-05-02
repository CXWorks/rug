#[cfg(all(feature = "color", not(target_os = "windows")))]
use ansi_term::ANSIString;
#[cfg(all(feature = "color", not(target_os = "windows")))]
use ansi_term::Colour::{Green, Red, Yellow};
#[cfg(feature = "color")]
use atty;
use std::env;
use std::fmt;
#[doc(hidden)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}
#[cfg(feature = "color")]
pub fn is_a_tty(stderr: bool) -> bool {
    debugln!("is_a_tty: stderr={:?}", stderr);
    let stream = if stderr { atty::Stream::Stderr } else { atty::Stream::Stdout };
    atty::is(stream)
}
#[cfg(not(feature = "color"))]
pub fn is_a_tty(_: bool) -> bool {
    debugln!("is_a_tty;");
    false
}
pub fn is_term_dumb() -> bool {
    env::var("TERM").ok() == Some(String::from("dumb"))
}
#[doc(hidden)]
pub struct ColorizerOption {
    pub use_stderr: bool,
    pub when: ColorWhen,
}
#[doc(hidden)]
pub struct Colorizer {
    when: ColorWhen,
}
macro_rules! color {
    ($_self:ident, $c:ident, $m:expr) => {
        match $_self .when { ColorWhen::Auto => Format::$c ($m), ColorWhen::Always =>
        Format::$c ($m), ColorWhen::Never => Format::None($m), }
    };
}
impl Colorizer {
    pub fn new(option: ColorizerOption) -> Colorizer {
        let is_a_tty = is_a_tty(option.use_stderr);
        let is_term_dumb = is_term_dumb();
        Colorizer {
            when: match option.when {
                ColorWhen::Auto if is_a_tty && !is_term_dumb => ColorWhen::Auto,
                ColorWhen::Auto => ColorWhen::Never,
                when => when,
            },
        }
    }
    pub fn good<T>(&self, msg: T) -> Format<T>
    where
        T: fmt::Display + AsRef<str>,
    {
        debugln!("Colorizer::good;");
        color!(self, Good, msg)
    }
    pub fn warning<T>(&self, msg: T) -> Format<T>
    where
        T: fmt::Display + AsRef<str>,
    {
        debugln!("Colorizer::warning;");
        color!(self, Warning, msg)
    }
    pub fn error<T>(&self, msg: T) -> Format<T>
    where
        T: fmt::Display + AsRef<str>,
    {
        debugln!("Colorizer::error;");
        color!(self, Error, msg)
    }
    pub fn none<T>(&self, msg: T) -> Format<T>
    where
        T: fmt::Display + AsRef<str>,
    {
        debugln!("Colorizer::none;");
        Format::None(msg)
    }
}
impl Default for Colorizer {
    fn default() -> Self {
        Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: ColorWhen::Auto,
        })
    }
}
/// Defines styles for different types of error messages. Defaults to Error=Red, Warning=Yellow,
/// and Good=Green
#[derive(Debug)]
#[doc(hidden)]
pub enum Format<T> {
    /// Defines the style used for errors, defaults to Red
    Error(T),
    /// Defines the style used for warnings, defaults to Yellow
    Warning(T),
    /// Defines the style used for good values, defaults to Green
    Good(T),
    /// Defines no formatting style
    None(T),
}
#[cfg(all(feature = "color", not(target_os = "windows")))]
impl<T: AsRef<str>> Format<T> {
    fn format(&self) -> ANSIString {
        match *self {
            Format::Error(ref e) => Red.bold().paint(e.as_ref()),
            Format::Warning(ref e) => Yellow.paint(e.as_ref()),
            Format::Good(ref e) => Green.paint(e.as_ref()),
            Format::None(ref e) => ANSIString::from(e.as_ref()),
        }
    }
}
#[cfg(any(not(feature = "color"), target_os = "windows"))]
#[cfg_attr(feature = "lints", allow(match_same_arms))]
impl<T: fmt::Display> Format<T> {
    fn format(&self) -> &T {
        match *self {
            Format::Error(ref e) => e,
            Format::Warning(ref e) => e,
            Format::Good(ref e) => e,
            Format::None(ref e) => e,
        }
    }
}
#[cfg(all(feature = "color", not(target_os = "windows")))]
impl<T: AsRef<str>> fmt::Display for Format<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", & self.format())
    }
}
#[cfg(any(not(feature = "color"), target_os = "windows"))]
impl<T: fmt::Display> fmt::Display for Format<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", & self.format())
    }
}
#[cfg(all(test, feature = "color", not(target_os = "windows")))]
mod test {
    use super::Format;
    use ansi_term::ANSIString;
    use ansi_term::Colour::{Green, Red, Yellow};
    #[test]
    fn colored_output() {
        let err = Format::Error("error");
        assert_eq!(&* format!("{}", err), &* format!("{}", Red.bold().paint("error")));
        let good = Format::Good("good");
        assert_eq!(&* format!("{}", good), &* format!("{}", Green.paint("good")));
        let warn = Format::Warning("warn");
        assert_eq!(&* format!("{}", warn), &* format!("{}", Yellow.paint("warn")));
        let none = Format::None("none");
        assert_eq!(&* format!("{}", none), &* format!("{}", ANSIString::from("none")));
    }
}
#[cfg(test)]
mod tests_llm_16_185 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_185_rrrruuuugggg_test_default = 0;
        let default_colorizer = <Colorizer as std::default::Default>::default();
        debug_assert_eq!(default_colorizer.when, ColorWhen::Auto);
        let _rug_ed_tests_llm_16_185_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_268 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_268_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = true;
        let option = ColorizerOption {
            use_stderr: rug_fuzz_0,
            when: ColorWhen::Auto,
        };
        let colorizer = Colorizer::new(option);
        debug_assert_eq!(colorizer.when, ColorWhen::Auto);
        let _rug_ed_tests_llm_16_268_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_269 {
    use super::*;
    use crate::*;
    use ansi_term::Colour;
    #[test]
    fn test_format_error() {
        let _rug_st_tests_llm_16_269_rrrruuuugggg_test_format_error = 0;
        let rug_fuzz_0 = "error message";
        let error = Format::Error(rug_fuzz_0);
        let result = error.format().to_string();
        debug_assert_eq!(result, Colour::Red.bold().paint("error message").to_string());
        let _rug_ed_tests_llm_16_269_rrrruuuugggg_test_format_error = 0;
    }
    #[test]
    fn test_format_warning() {
        let _rug_st_tests_llm_16_269_rrrruuuugggg_test_format_warning = 0;
        let rug_fuzz_0 = "warning message";
        let warning = Format::Warning(rug_fuzz_0);
        let result = warning.format().to_string();
        debug_assert_eq!(result, Colour::Yellow.paint("warning message").to_string());
        let _rug_ed_tests_llm_16_269_rrrruuuugggg_test_format_warning = 0;
    }
    #[test]
    fn test_format_good() {
        let _rug_st_tests_llm_16_269_rrrruuuugggg_test_format_good = 0;
        let rug_fuzz_0 = "good message";
        let good = Format::Good(rug_fuzz_0);
        let result = good.format().to_string();
        debug_assert_eq!(result, Colour::Green.paint("good message").to_string());
        let _rug_ed_tests_llm_16_269_rrrruuuugggg_test_format_good = 0;
    }
    #[test]
    fn test_format_none() {
        let _rug_st_tests_llm_16_269_rrrruuuugggg_test_format_none = 0;
        let rug_fuzz_0 = "none message";
        let none = Format::None(rug_fuzz_0);
        let result = none.format().to_string();
        debug_assert_eq!(result, "none message");
        let _rug_ed_tests_llm_16_269_rrrruuuugggg_test_format_none = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_271 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_term_dumb() {
        let _rug_st_tests_llm_16_271_rrrruuuugggg_test_is_term_dumb = 0;
        let rug_fuzz_0 = "TERM";
        let rug_fuzz_1 = "dumb";
        debug_assert_eq!(is_term_dumb(), false);
        env::set_var(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(is_term_dumb(), true);
        let _rug_ed_tests_llm_16_271_rrrruuuugggg_test_is_term_dumb = 0;
    }
}
#[cfg(test)]
mod tests_rug_180 {
    use super::*;
    #[test]
    fn test_is_a_tty() {
        let _rug_st_tests_rug_180_rrrruuuugggg_test_is_a_tty = 0;
        let rug_fuzz_0 = false;
        let mut p0: bool = rug_fuzz_0;
        crate::fmt::is_a_tty(p0);
        let _rug_ed_tests_rug_180_rrrruuuugggg_test_is_a_tty = 0;
    }
}
#[cfg(test)]
mod tests_rug_182 {
    use super::*;
    use crate::fmt::{Colorizer, Format};
    use std::sync::Arc;
    #[test]
    fn test_warning() {
        let _rug_st_tests_rug_182_rrrruuuugggg_test_warning = 0;
        let rug_fuzz_0 = "Warning message";
        let p0: Colorizer = Colorizer::default();
        let p1: Arc<str> = Arc::from(rug_fuzz_0);
        let result: Format<Arc<str>> = p0.warning(p1);
        let _rug_ed_tests_rug_182_rrrruuuugggg_test_warning = 0;
    }
}
