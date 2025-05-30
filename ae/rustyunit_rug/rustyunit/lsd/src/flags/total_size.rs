//! This module defines the [TotalSize] flag. To set it up from [ArgMatches], a [Config] and its
//! [Default] value, use the [configure_from](Configurable::configure_from) method.

use super::Configurable;

use crate::config_file::Config;

use clap::ArgMatches;

/// The flag showing whether to show the total size for directories.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub struct TotalSize(pub bool);

impl Configurable<Self> for TotalSize {
    /// Get a potential `TotalSize` value from [ArgMatches].
    ///
    /// If the "total-size" argument is passed, this returns a `TotalSize` with value `true` in a
    /// [Some]. Otherwise this returns [None].
    fn from_arg_matches(matches: &ArgMatches) -> Option<Self> {
        if matches.is_present("total-size") {
            Some(Self(true))
        } else {
            None
        }
    }

    /// Get a potential `TotalSize` value from a [Config].
    ///
    /// If the `Config::total-size` has value,
    /// this returns it as the value of the `TotalSize`, in a [Some].
    /// Otherwise this returns [None].
    fn from_config(config: &Config) -> Option<Self> {
        config.total_size.map(Self)
    }
}

#[cfg(test)]
mod test {
    use super::TotalSize;

    use crate::app;
    use crate::config_file::Config;
    use crate::flags::Configurable;

    #[test]
    fn test_from_arg_matches_none() {
        let argv = vec!["lsd"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(None, TotalSize::from_arg_matches(&matches));
    }

    #[test]
    fn test_from_arg_matches_true() {
        let argv = vec!["lsd", "--total-size"];
        let matches = app::build().get_matches_from_safe(argv).unwrap();
        assert_eq!(Some(TotalSize(true)), TotalSize::from_arg_matches(&matches));
    }

    #[test]
    fn test_from_config_none() {
        assert_eq!(None, TotalSize::from_config(&Config::with_none()));
    }

    #[test]
    fn test_from_config_true() {
        let mut c = Config::with_none();
        c.total_size = Some(true);
        assert_eq!(Some(TotalSize(true)), TotalSize::from_config(&c));
    }

    #[test]
    fn test_from_config_false() {
        let mut c = Config::with_none();
        c.total_size = Some(false);
        assert_eq!(Some(TotalSize(false)), TotalSize::from_config(&c));
    }
}
#[cfg(test)]
mod tests_llm_16_113 {
    use super::*;

use crate::*;
    use config_file::Config;

    #[test]
    fn test_from_config_with_total_size() {
        let mut config = Config::with_none();
        config.total_size = Some(true);

        let result = <flags::total_size::TotalSize as flags::Configurable<flags::total_size::TotalSize>>::from_config(&config);

        assert_eq!(result, Some(flags::total_size::TotalSize(true)));
    }

    #[test]
    fn test_from_config_without_total_size() {
        let config = Config::with_none();

        let result = <flags::total_size::TotalSize as flags::Configurable<flags::total_size::TotalSize>>::from_config(&config);

        assert_eq!(result, None);
    }
}#[cfg(test)]
mod tests_rug_84_prepare {
    use clap::ArgMatches; // import the necessary module

    #[test]
    fn sample() {
        let mut p0: ArgMatches<'static> = ArgMatches::default(); // create the local variable p0 with type clap::ArgMatches<'_>
    }
}
#[cfg(test)]
mod tests_rug_84 {
    use super::*;
    use clap::ArgMatches;
    use crate::flags::{Configurable, total_size};

    #[test]
    fn test_from_arg_matches() {
        let mut p0: ArgMatches<'static> = ArgMatches::default();
        
        total_size::TotalSize::from_arg_matches(&p0);
    }
}