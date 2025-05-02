//! This module defines the [HyperlinkOption]. To set it up from [ArgMatches], a [Config] and its
//! [Default] value, use its [configure_from](Configurable::configure_from) method.

use super::Configurable;

use crate::config_file::Config;

use clap::ArgMatches;
use serde::Deserialize;

/// The flag showing when to use hyperlink in the output.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HyperlinkOption {
    Always,
    Auto,
    Never,
}

impl Configurable<Self> for HyperlinkOption {
    /// Get a potential `HyperlinkOption` variant from [ArgMatches].
    ///
    /// If the "classic" argument is passed, then this returns the [HyperlinkOption::Never] variant in
    /// a [Some]. Otherwise if the argument is passed, this returns the variant corresponding to
    /// its parameter in a [Some]. Otherwise this returns [None].
    fn from_arg_matches(matches: &ArgMatches) -> Option<Self> {
        if matches.is_present("classic") {
            Some(Self::Never)
        } else if matches.occurrences_of("hyperlink") > 0 {
            match matches.values_of("hyperlink")?.last() {
                Some("always") => Some(Self::Always),
                Some("auto") => Some(Self::Auto),
                Some("never") => Some(Self::Never),
                _ => panic!("This should not be reachable!"),
            }
        } else {
            None
        }
    }

    /// Get a potential `HyperlinkOption` variant from a [Config].
    ///
    /// If the `Configs::classic` has value and is "true" then this returns Some(HyperlinkOption::Never).
    /// Otherwise if the `Config::hyperlink::when` has value and is one of "always", "auto" or "never",
    /// this returns its corresponding variant in a [Some].
    /// Otherwise this returns [None].
    fn from_config(config: &Config) -> Option<Self> {
        if let Some(true) = &config.classic {
            return Some(Self::Never);
        }

        config.hyperlink
    }
}

/// The default value for the `HyperlinkOption` is [HyperlinkOption::Auto].
impl Default for HyperlinkOption {
    fn default() -> Self {
        Self::Never
    }
}

#[cfg(test)]
mod test_hyperlink_option {
    use super::HyperlinkOption;

    use crate::app;
    use crate::config_file::Config;
    use crate::flags::Configurable;

    #[test]
    fn test_from_arg_matches_none() {
        let argv = vec!["lsd"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(None, HyperlinkOption::from_arg_matches(&matches));
    }

    #[test]
    fn test_from_arg_matches_always() {
        let argv = vec!["lsd", "--hyperlink", "always"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(
            Some(HyperlinkOption::Always),
            HyperlinkOption::from_arg_matches(&matches)
        );
    }

    #[test]
    fn test_from_arg_matches_autp() {
        let argv = vec!["lsd", "--hyperlink", "auto"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(
            Some(HyperlinkOption::Auto),
            HyperlinkOption::from_arg_matches(&matches)
        );
    }

    #[test]
    fn test_from_arg_matches_never() {
        let argv = vec!["lsd", "--hyperlink", "never"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(
            Some(HyperlinkOption::Never),
            HyperlinkOption::from_arg_matches(&matches)
        );
    }

    #[test]
    fn test_from_arg_matches_classic_mode() {
        let argv = vec!["lsd", "--hyperlink", "always", "--classic"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(
            Some(HyperlinkOption::Never),
            HyperlinkOption::from_arg_matches(&matches)
        );
    }

    #[test]
    fn test_from_arg_matches_hyperlink_when_multi() {
        let argv = vec!["lsd", "--hyperlink", "always", "--hyperlink", "never"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(
            Some(HyperlinkOption::Never),
            HyperlinkOption::from_arg_matches(&matches)
        );
    }

    #[test]
    fn test_from_config_none() {
        assert_eq!(None, HyperlinkOption::from_config(&Config::with_none()));
    }

    #[test]
    fn test_from_config_always() {
        let mut c = Config::with_none();
        c.hyperlink = Some(HyperlinkOption::Always);
        assert_eq!(
            Some(HyperlinkOption::Always),
            HyperlinkOption::from_config(&c)
        );
    }

    #[test]
    fn test_from_config_auto() {
        let mut c = Config::with_none();
        c.hyperlink = Some(HyperlinkOption::Auto);
        assert_eq!(
            Some(HyperlinkOption::Auto),
            HyperlinkOption::from_config(&c)
        );
    }

    #[test]
    fn test_from_config_never() {
        let mut c = Config::with_none();
        c.hyperlink = Some(HyperlinkOption::Never);
        assert_eq!(
            Some(HyperlinkOption::Never),
            HyperlinkOption::from_config(&c)
        );
    }

    #[test]
    fn test_from_config_classic_mode() {
        let mut c = Config::with_none();
        c.classic = Some(true);
        c.hyperlink = Some(HyperlinkOption::Always);
        assert_eq!(
            Some(HyperlinkOption::Never),
            HyperlinkOption::from_config(&c)
        );
    }
}
#[cfg(test)]
mod tests_llm_16_51 {
    use super::*;

use crate::*;
    use crate::config_file::Config;

    #[test]
    fn test_from_config_classic_true() {
        let config = Config {
            classic: Some(true),
            ..Default::default()
        };
        assert_eq!(
            <flags::hyperlink::HyperlinkOption as flags::Configurable<flags::hyperlink::HyperlinkOption>>::from_config(&config),
            Some(flags::hyperlink::HyperlinkOption::Never)
        );
    }

    #[test]
    fn test_from_config_classic_false_hyperlink_always() {
        let config = Config {
            classic: Some(false),
            hyperlink: Some(flags::hyperlink::HyperlinkOption::Always),
            ..Default::default()
        };
        assert_eq!(
            <flags::hyperlink::HyperlinkOption as flags::Configurable<flags::hyperlink::HyperlinkOption>>::from_config(&config),
            Some(flags::hyperlink::HyperlinkOption::Always)
        );
    }

    #[test]
    fn test_from_config_classic_false_hyperlink_auto() {
        let config = Config {
            classic: Some(false),
            hyperlink: Some(flags::hyperlink::HyperlinkOption::Auto),
            ..Default::default()
        };
        assert_eq!(
            <flags::hyperlink::HyperlinkOption as flags::Configurable<flags::hyperlink::HyperlinkOption>>::from_config(&config),
            Some(flags::hyperlink::HyperlinkOption::Auto)
        );
    }

    #[test]
    fn test_from_config_classic_false_hyperlink_never() {
        let config = Config {
            classic: Some(false),
            hyperlink: Some(flags::hyperlink::HyperlinkOption::Never),
            ..Default::default()
        };
        assert_eq!(
            <flags::hyperlink::HyperlinkOption as flags::Configurable<flags::hyperlink::HyperlinkOption>>::from_config(&config),
            Some(flags::hyperlink::HyperlinkOption::Never)
        );
    }

    #[test]
    fn test_from_config_classic_false_hyperlink_none() {
        let config = Config {
            classic: Some(false),
            hyperlink: None,
            ..Default::default()
        };
        assert_eq!(
            <flags::hyperlink::HyperlinkOption as flags::Configurable<flags::hyperlink::HyperlinkOption>>::from_config(&config),
            None
        );
    }
}#[cfg(test)]
mod tests_llm_16_52 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_default() {
        let result = <flags::hyperlink::HyperlinkOption as std::default::Default>::default();
        assert_eq!(result, flags::hyperlink::HyperlinkOption::Never);
    }
}