//! This module defines the [SizeFlag]. To set it up from [ArgMatches], a [Config] and its
//! [Default] value, use its [configure_from](Configurable::configure_from) method.

use super::Configurable;

use crate::config_file::Config;

use clap::ArgMatches;
use serde::Deserialize;

/// The flag showing which file size units to use.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SizeFlag {
    /// The variant to show file size with SI unit prefix and a B for bytes.
    Default,
    /// The variant to show file size with only the SI unit prefix.
    Short,
    /// The variant to show file size in bytes.
    Bytes,
}

impl SizeFlag {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Self::Default),
            "short" => Some(Self::Short),
            "bytes" => Some(Self::Bytes),
            _ => {
                panic!(
                    "Size can only be one of default, short or bytes, but got {}.",
                    value
                );
            }
        }
    }
}

impl Configurable<Self> for SizeFlag {
    /// Get a potential `SizeFlag` variant from [ArgMatches].
    ///
    /// If any of the "default", "short" or "bytes" arguments is passed, the corresponding
    /// `SizeFlag` variant is returned in a [Some]. If neither of them is passed, this returns
    /// [None].
    fn from_arg_matches(matches: &ArgMatches) -> Option<Self> {
        if matches.is_present("classic") {
            return Some(Self::Bytes);
        } else if matches.occurrences_of("size") > 0 {
            if let Some(size) = matches.values_of("size")?.last() {
                return Self::from_str(size);
            }
        }
        None
    }

    /// Get a potential `SizeFlag` variant from a [Config].
    ///
    /// If the `Config::size` has value and is one of "default", "short" or "bytes",
    /// this returns the corresponding `SizeFlag` variant in a [Some].
    /// Otherwise this returns [None].
    fn from_config(config: &Config) -> Option<Self> {
        if let Some(true) = config.classic {
            Some(Self::Bytes)
        } else {
            config.size
        }
    }
}

/// The default value for `SizeFlag` is [SizeFlag::Default].
impl Default for SizeFlag {
    fn default() -> Self {
        Self::Default
    }
}

#[cfg(test)]
mod test {
    use super::SizeFlag;

    use crate::app;
    use crate::config_file::Config;
    use crate::flags::Configurable;

    #[test]
    fn test_default() {
        assert_eq!(SizeFlag::Default, SizeFlag::default());
    }

    #[test]
    fn test_from_arg_matches_none() {
        let argv = vec!["lsd"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(None, SizeFlag::from_arg_matches(&matches));
    }

    #[test]
    fn test_from_arg_matches_default() {
        let argv = vec!["lsd", "--size", "default"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(
            Some(SizeFlag::Default),
            SizeFlag::from_arg_matches(&matches)
        );
    }

    #[test]
    fn test_from_arg_matches_short() {
        let args = vec!["lsd", "--size", "short"];
        let matches = app::build().get_matches_from_safe(args).unwrap();
        assert_eq!(Some(SizeFlag::Short), SizeFlag::from_arg_matches(&matches));
    }

    #[test]
    fn test_from_arg_matches_bytes() {
        let args = vec!["lsd", "--size", "bytes"];
        let matches = app::build().get_matches_from_safe(args).unwrap();
        assert_eq!(Some(SizeFlag::Bytes), SizeFlag::from_arg_matches(&matches));
    }

    #[test]
    #[should_panic]
    fn test_from_arg_matches_unknonwn() {
        let args = vec!["lsd", "--size", "unknown"];
        let _ = app::build().get_matches_from_safe(args).unwrap();
    }
    #[test]
    fn test_from_arg_matches_size_multi() {
        let args = vec!["lsd", "--size", "bytes", "--size", "short"];
        let matches = app::build().get_matches_from_safe(args).unwrap();
        assert_eq!(Some(SizeFlag::Short), SizeFlag::from_arg_matches(&matches));
    }

    #[test]
    fn test_from_arg_matches_size_classic() {
        let args = vec!["lsd", "--size", "short", "--classic"];
        let matches = app::build().get_matches_from_safe(args).unwrap();
        assert_eq!(Some(SizeFlag::Bytes), SizeFlag::from_arg_matches(&matches));
    }

    #[test]
    fn test_from_config_none() {
        assert_eq!(None, SizeFlag::from_config(&Config::with_none()));
    }

    #[test]
    fn test_from_config_default() {
        let mut c = Config::with_none();
        c.size = Some(SizeFlag::Default);
        assert_eq!(Some(SizeFlag::Default), SizeFlag::from_config(&c));
    }

    #[test]
    fn test_from_config_short() {
        let mut c = Config::with_none();
        c.size = Some(SizeFlag::Short);
        assert_eq!(Some(SizeFlag::Short), SizeFlag::from_config(&c));
    }

    #[test]
    fn test_from_config_bytes() {
        let mut c = Config::with_none();
        c.size = Some(SizeFlag::Bytes);
        assert_eq!(Some(SizeFlag::Bytes), SizeFlag::from_config(&c));
    }

    #[test]
    fn test_from_config_classic_mode() {
        let mut c = Config::with_none();
        c.classic = Some(true);
        assert_eq!(Some(SizeFlag::Bytes), SizeFlag::from_config(&c));
    }
}
#[cfg(test)]
mod tests_llm_16_86 {
    use super::*;

use crate::*;
    use serde_yaml;

    #[test]
    fn test_from_config_bytes() {
        let config = Config {
            classic: Some(false),
            size: Some(SizeFlag::Bytes),
            ..Config::with_none()
        };
        assert_eq!(
            <SizeFlag as Configurable<SizeFlag>>::from_config(&config),
            Some(SizeFlag::Bytes)
        );
    }

    #[test]
    fn test_from_config_short() {
        let config = Config {
            classic: Some(false),
            size: Some(SizeFlag::Short),
            ..Config::with_none()
        };
        assert_eq!(
            <SizeFlag as Configurable<SizeFlag>>::from_config(&config),
            Some(SizeFlag::Short)
        );
    }

    #[test]
    fn test_from_config_default() {
        let config = Config {
            classic: Some(false),
            size: Some(SizeFlag::Default),
            ..Config::with_none()
        };
        assert_eq!(
            <SizeFlag as Configurable<SizeFlag>>::from_config(&config),
            Some(SizeFlag::Default)
        );
    }

    #[test]
    fn test_from_config_none() {
        let config = Config {
            classic: Some(false),
            size: None,
            ..Config::with_none()
        };
        assert_eq!(
            <SizeFlag as Configurable<SizeFlag>>::from_config(&config),
            None
        );
    }

    #[test]
    fn test_from_config_classic() {
        let config = Config {
            classic: Some(true),
            size: Some(SizeFlag::Default),
            ..Config::with_none()
        };
        assert_eq!(
            <SizeFlag as Configurable<SizeFlag>>::from_config(&config),
            Some(SizeFlag::Bytes)
        );
    }

    #[test]
    fn test_from_config_no_classic() {
        let config = Config {
            classic: None,
            size: Some(SizeFlag::Default),
            ..Config::with_none()
        };
        assert_eq!(
            <SizeFlag as Configurable<SizeFlag>>::from_config(&config),
            Some(SizeFlag::Default)
        );
    }
}#[cfg(test)]
mod tests_llm_16_88 {
    use super::*;

use crate::*;
    use crate::flags::size::Configurable;
    use clap::ArgMatches;
    use serde::Deserialize;

    #[test]
    fn test_default() {
        assert_eq!(<flags::size::SizeFlag as std::default::Default>::default(), SizeFlag::Default);
    }
}#[cfg(test)]
mod tests_llm_16_220 {
    use super::*;

use crate::*;
    use crate::flags::size::SizeFlag;

    #[test]
    fn test_from_str() {
        assert_eq!(SizeFlag::from_str("default"), Some(SizeFlag::Default));
        assert_eq!(SizeFlag::from_str("short"), Some(SizeFlag::Short));
        assert_eq!(SizeFlag::from_str("bytes"), Some(SizeFlag::Bytes));
    }

    #[test]
    #[should_panic(expected = "Size can only be one of default, short or bytes")]
    fn test_from_str_panic() {
        SizeFlag::from_str("invalid_value");
    }
}