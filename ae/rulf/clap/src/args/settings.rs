#[allow(deprecated, unused_imports)]
use std::ascii::AsciiExt;
use std::str::FromStr;
bitflags! {
    struct Flags : u32 { const REQUIRED = 1; const MULTIPLE = 1 << 1; const EMPTY_VALS =
    1 << 2; const GLOBAL = 1 << 3; const HIDDEN = 1 << 4; const TAKES_VAL = 1 << 5; const
    USE_DELIM = 1 << 6; const NEXT_LINE_HELP = 1 << 7; const R_UNLESS_ALL = 1 << 8; const
    REQ_DELIM = 1 << 9; const DELIM_NOT_SET = 1 << 10; const HIDE_POS_VALS = 1 << 11;
    const ALLOW_TAC_VALS = 1 << 12; const REQUIRE_EQUALS = 1 << 13; const LAST = 1 << 14;
    const HIDE_DEFAULT_VAL = 1 << 15; const CASE_INSENSITIVE = 1 << 16; const
    HIDE_ENV_VALS = 1 << 17; const HIDDEN_SHORT_H = 1 << 18; const HIDDEN_LONG_H = 1 <<
    19; }
}
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct ArgFlags(Flags);
impl ArgFlags {
    pub fn new() -> Self {
        ArgFlags::default()
    }
    impl_settings! {
        ArgSettings, Required => Flags::REQUIRED, Multiple => Flags::MULTIPLE,
        EmptyValues => Flags::EMPTY_VALS, Global => Flags::GLOBAL, Hidden =>
        Flags::HIDDEN, TakesValue => Flags::TAKES_VAL, UseValueDelimiter =>
        Flags::USE_DELIM, NextLineHelp => Flags::NEXT_LINE_HELP, RequiredUnlessAll =>
        Flags::R_UNLESS_ALL, RequireDelimiter => Flags::REQ_DELIM, ValueDelimiterNotSet
        => Flags::DELIM_NOT_SET, HidePossibleValues => Flags::HIDE_POS_VALS,
        AllowLeadingHyphen => Flags::ALLOW_TAC_VALS, RequireEquals =>
        Flags::REQUIRE_EQUALS, Last => Flags::LAST, CaseInsensitive =>
        Flags::CASE_INSENSITIVE, HideEnvValues => Flags::HIDE_ENV_VALS, HideDefaultValue
        => Flags::HIDE_DEFAULT_VAL, HiddenShortHelp => Flags::HIDDEN_SHORT_H,
        HiddenLongHelp => Flags::HIDDEN_LONG_H
    }
}
impl Default for ArgFlags {
    fn default() -> Self {
        ArgFlags(Flags::EMPTY_VALS | Flags::DELIM_NOT_SET)
    }
}
/// Various settings that apply to arguments and may be set, unset, and checked via getter/setter
/// methods [`Arg::set`], [`Arg::unset`], and [`Arg::is_set`]
///
/// [`Arg::set`]: ./struct.Arg.html#method.set
/// [`Arg::unset`]: ./struct.Arg.html#method.unset
/// [`Arg::is_set`]: ./struct.Arg.html#method.is_set
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ArgSettings {
    /// The argument must be used
    Required,
    /// The argument may be used multiple times such as `--flag --flag`
    Multiple,
    /// The argument allows empty values such as `--option ""`
    EmptyValues,
    /// The argument should be propagated down through all child [`SubCommand`]s
    ///
    /// [`SubCommand`]: ./struct.SubCommand.html
    Global,
    /// The argument should **not** be shown in help text
    Hidden,
    /// The argument accepts a value, such as `--option <value>`
    TakesValue,
    /// Determines if the argument allows values to be grouped via a delimiter
    UseValueDelimiter,
    /// Prints the help text on the line after the argument
    NextLineHelp,
    /// Requires the use of a value delimiter for all multiple values
    RequireDelimiter,
    /// Hides the possible values from the help string
    HidePossibleValues,
    /// Allows vals that start with a '-'
    AllowLeadingHyphen,
    /// Require options use `--option=val` syntax
    RequireEquals,
    /// Specifies that the arg is the last positional argument and may be accessed early via `--`
    /// syntax
    Last,
    /// Hides the default value from the help string
    HideDefaultValue,
    /// Makes `Arg::possible_values` case insensitive
    CaseInsensitive,
    /// Hides ENV values in the help message
    HideEnvValues,
    /// The argument should **not** be shown in short help text
    HiddenShortHelp,
    /// The argument should **not** be shown in long help text
    HiddenLongHelp,
    #[doc(hidden)]
    RequiredUnlessAll,
    #[doc(hidden)]
    ValueDelimiterNotSet,
}
impl FromStr for ArgSettings {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match &*s.to_ascii_lowercase() {
            "required" => Ok(ArgSettings::Required),
            "multiple" => Ok(ArgSettings::Multiple),
            "global" => Ok(ArgSettings::Global),
            "emptyvalues" => Ok(ArgSettings::EmptyValues),
            "hidden" => Ok(ArgSettings::Hidden),
            "takesvalue" => Ok(ArgSettings::TakesValue),
            "usevaluedelimiter" => Ok(ArgSettings::UseValueDelimiter),
            "nextlinehelp" => Ok(ArgSettings::NextLineHelp),
            "requiredunlessall" => Ok(ArgSettings::RequiredUnlessAll),
            "requiredelimiter" => Ok(ArgSettings::RequireDelimiter),
            "valuedelimiternotset" => Ok(ArgSettings::ValueDelimiterNotSet),
            "hidepossiblevalues" => Ok(ArgSettings::HidePossibleValues),
            "allowleadinghyphen" => Ok(ArgSettings::AllowLeadingHyphen),
            "requireequals" => Ok(ArgSettings::RequireEquals),
            "last" => Ok(ArgSettings::Last),
            "hidedefaultvalue" => Ok(ArgSettings::HideDefaultValue),
            "caseinsensitive" => Ok(ArgSettings::CaseInsensitive),
            "hideenvvalues" => Ok(ArgSettings::HideEnvValues),
            "hiddenshorthelp" => Ok(ArgSettings::HiddenShortHelp),
            "hiddenlonghelp" => Ok(ArgSettings::HiddenLongHelp),
            _ => Err("unknown ArgSetting, cannot convert from str".to_owned()),
        }
    }
}
#[cfg(test)]
mod test {
    use super::ArgSettings;
    #[test]
    fn arg_settings_fromstr() {
        assert_eq!(
            "allowleadinghyphen".parse::< ArgSettings > ().unwrap(),
            ArgSettings::AllowLeadingHyphen
        );
        assert_eq!(
            "emptyvalues".parse::< ArgSettings > ().unwrap(), ArgSettings::EmptyValues
        );
        assert_eq!("global".parse::< ArgSettings > ().unwrap(), ArgSettings::Global);
        assert_eq!(
            "hidepossiblevalues".parse::< ArgSettings > ().unwrap(),
            ArgSettings::HidePossibleValues
        );
        assert_eq!("hidden".parse::< ArgSettings > ().unwrap(), ArgSettings::Hidden);
        assert_eq!("multiple".parse::< ArgSettings > ().unwrap(), ArgSettings::Multiple);
        assert_eq!(
            "nextlinehelp".parse::< ArgSettings > ().unwrap(), ArgSettings::NextLineHelp
        );
        assert_eq!(
            "requiredunlessall".parse::< ArgSettings > ().unwrap(),
            ArgSettings::RequiredUnlessAll
        );
        assert_eq!(
            "requiredelimiter".parse::< ArgSettings > ().unwrap(),
            ArgSettings::RequireDelimiter
        );
        assert_eq!("required".parse::< ArgSettings > ().unwrap(), ArgSettings::Required);
        assert_eq!(
            "takesvalue".parse::< ArgSettings > ().unwrap(), ArgSettings::TakesValue
        );
        assert_eq!(
            "usevaluedelimiter".parse::< ArgSettings > ().unwrap(),
            ArgSettings::UseValueDelimiter
        );
        assert_eq!(
            "valuedelimiternotset".parse::< ArgSettings > ().unwrap(),
            ArgSettings::ValueDelimiterNotSet
        );
        assert_eq!(
            "requireequals".parse::< ArgSettings > ().unwrap(),
            ArgSettings::RequireEquals
        );
        assert_eq!("last".parse::< ArgSettings > ().unwrap(), ArgSettings::Last);
        assert_eq!(
            "hidedefaultvalue".parse::< ArgSettings > ().unwrap(),
            ArgSettings::HideDefaultValue
        );
        assert_eq!(
            "caseinsensitive".parse::< ArgSettings > ().unwrap(),
            ArgSettings::CaseInsensitive
        );
        assert_eq!(
            "hideenvvalues".parse::< ArgSettings > ().unwrap(),
            ArgSettings::HideEnvValues
        );
        assert_eq!(
            "hiddenshorthelp".parse::< ArgSettings > ().unwrap(),
            ArgSettings::HiddenShortHelp
        );
        assert_eq!(
            "hiddenlonghelp".parse::< ArgSettings > ().unwrap(),
            ArgSettings::HiddenLongHelp
        );
        assert!("hahahaha".parse::< ArgSettings > ().is_err());
    }
}
#[cfg(test)]
mod tests_llm_16_178 {
    use super::*;
    use crate::*;
    #[test]
    fn test_from_str() {
        let _rug_st_tests_llm_16_178_rrrruuuugggg_test_from_str = 0;
        let rug_fuzz_0 = "required";
        let rug_fuzz_1 = "multiple";
        let rug_fuzz_2 = "global";
        let rug_fuzz_3 = "emptyvalues";
        let rug_fuzz_4 = "hidden";
        let rug_fuzz_5 = "takesvalue";
        let rug_fuzz_6 = "usevaluedelimiter";
        let rug_fuzz_7 = "nextlinehelp";
        let rug_fuzz_8 = "requiredunlessall";
        let rug_fuzz_9 = "requiredelimiter";
        let rug_fuzz_10 = "valuedelimiternotset";
        let rug_fuzz_11 = "hidepossiblevalues";
        let rug_fuzz_12 = "allowleadinghyphen";
        let rug_fuzz_13 = "requireequals";
        let rug_fuzz_14 = "last";
        let rug_fuzz_15 = "hidedefaultvalue";
        let rug_fuzz_16 = "caseinsensitive";
        let rug_fuzz_17 = "hideenvvalues";
        let rug_fuzz_18 = "hiddenshorthelp";
        let rug_fuzz_19 = "hiddenlonghelp";
        let rug_fuzz_20 = "invalid";
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_0).unwrap(), ArgSettings::Required
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_1).unwrap(), ArgSettings::Multiple
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_2).unwrap(), ArgSettings::Global
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_3).unwrap(), ArgSettings::EmptyValues
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_4).unwrap(), ArgSettings::Hidden
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_5).unwrap(), ArgSettings::TakesValue
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_6).unwrap(), ArgSettings::UseValueDelimiter
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_7).unwrap(), ArgSettings::NextLineHelp
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_8).unwrap(), ArgSettings::RequiredUnlessAll
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_9).unwrap(), ArgSettings::RequireDelimiter
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_10).unwrap(),
            ArgSettings::ValueDelimiterNotSet
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_11).unwrap(), ArgSettings::HidePossibleValues
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_12).unwrap(), ArgSettings::AllowLeadingHyphen
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_13).unwrap(), ArgSettings::RequireEquals
        );
        debug_assert_eq!(ArgSettings::from_str(rug_fuzz_14).unwrap(), ArgSettings::Last);
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_15).unwrap(), ArgSettings::HideDefaultValue
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_16).unwrap(), ArgSettings::CaseInsensitive
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_17).unwrap(), ArgSettings::HideEnvValues
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_18).unwrap(), ArgSettings::HiddenShortHelp
        );
        debug_assert_eq!(
            ArgSettings::from_str(rug_fuzz_19).unwrap(), ArgSettings::HiddenLongHelp
        );
        debug_assert!(ArgSettings::from_str(rug_fuzz_20).is_err());
        let _rug_ed_tests_llm_16_178_rrrruuuugggg_test_from_str = 0;
    }
}
#[cfg(test)]
mod tests_rug_500 {
    use super::*;
    use crate::args::settings::ArgFlags;
    #[test]
    fn test_arg_flags_new() {
        let _rug_st_tests_rug_500_rrrruuuugggg_test_arg_flags_new = 0;
        ArgFlags::new();
        let _rug_ed_tests_rug_500_rrrruuuugggg_test_arg_flags_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_501 {
    use super::*;
    use crate::args::settings::ArgFlags;
    use crate::std::default::Default;
    #[test]
    fn test_default() {
        let _rug_st_tests_rug_501_rrrruuuugggg_test_default = 0;
        let result: ArgFlags = Default::default();
        let _rug_ed_tests_rug_501_rrrruuuugggg_test_default = 0;
    }
}
