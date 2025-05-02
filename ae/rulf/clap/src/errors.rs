use std::convert::From;
use std::error::Error as StdError;
use std::fmt as std_fmt;
use std::fmt::Display;
use std::io::{self, Write};
use std::process;
use std::result::Result as StdResult;
use args::AnyArg;
use fmt::{ColorWhen, Colorizer, ColorizerOption};
use suggestions;
/// Short hand for [`Result`] type
///
/// [`Result`]: https://doc.rust-lang.org/std/result/enum.Result.html
pub type Result<T> = StdResult<T, Error>;
/// Command line argument parser kind of error
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ErrorKind {
    /// Occurs when an [`Arg`] has a set of possible values,
    /// and the user provides a value which isn't in that set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .arg(Arg::with_name("speed")
    ///         .possible_value("fast")
    ///         .possible_value("slow"))
    ///     .get_matches_from_safe(vec!["prog", "other"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::InvalidValue);
    /// ```
    /// [`Arg`]: ./struct.Arg.html
    InvalidValue,
    /// Occurs when a user provides a flag, option, argument or subcommand which isn't defined.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .arg(Arg::from_usage("--flag 'some flag'"))
    ///     .get_matches_from_safe(vec!["prog", "--other"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::UnknownArgument);
    /// ```
    UnknownArgument,
    /// Occurs when the user provides an unrecognized [`SubCommand`] which meets the threshold for
    /// being similar enough to an existing subcommand.
    /// If it doesn't meet the threshold, or the 'suggestions' feature is disabled,
    /// the more general [`UnknownArgument`] error is returned.
    ///
    /// # Examples
    ///
    #[cfg_attr(not(feature = "suggestions"), doc = " ```no_run")]
    #[cfg_attr(feature = "suggestions", doc = " ```")]
    /// # use clap::{App, Arg, ErrorKind, SubCommand};
    /// let result = App::new("prog")
    ///     .subcommand(SubCommand::with_name("config")
    ///         .about("Used for configuration")
    ///         .arg(Arg::with_name("config_file")
    ///             .help("The configuration file to use")
    ///             .index(1)))
    ///     .get_matches_from_safe(vec!["prog", "confi"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::InvalidSubcommand);
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`UnknownArgument`]: ./enum.ErrorKind.html#variant.UnknownArgument
    InvalidSubcommand,
    /// Occurs when the user provides an unrecognized [`SubCommand`] which either
    /// doesn't meet the threshold for being similar enough to an existing subcommand,
    /// or the 'suggestions' feature is disabled.
    /// Otherwise the more detailed [`InvalidSubcommand`] error is returned.
    ///
    /// This error typically happens when passing additional subcommand names to the `help`
    /// subcommand. Otherwise, the more general [`UnknownArgument`] error is used.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind, SubCommand};
    /// let result = App::new("prog")
    ///     .subcommand(SubCommand::with_name("config")
    ///         .about("Used for configuration")
    ///         .arg(Arg::with_name("config_file")
    ///             .help("The configuration file to use")
    ///             .index(1)))
    ///     .get_matches_from_safe(vec!["prog", "help", "nothing"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::UnrecognizedSubcommand);
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`InvalidSubcommand`]: ./enum.ErrorKind.html#variant.InvalidSubcommand
    /// [`UnknownArgument`]: ./enum.ErrorKind.html#variant.UnknownArgument
    UnrecognizedSubcommand,
    /// Occurs when the user provides an empty value for an option that does not allow empty
    /// values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::with_name("color")
    ///          .long("color")
    ///          .empty_values(false))
    ///     .get_matches_from_safe(vec!["prog", "--color="]);
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::EmptyValue);
    /// ```
    EmptyValue,
    /// Occurs when the user provides a value for an argument with a custom validation and the
    /// value fails that validation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// fn is_numeric(val: String) -> Result<(), String> {
    ///     match val.parse::<i64>() {
    ///         Ok(..) => Ok(()),
    ///         Err(..) => Err(String::from("Value wasn't a number!")),
    ///     }
    /// }
    ///
    /// let result = App::new("prog")
    ///     .arg(Arg::with_name("num")
    ///          .validator(is_numeric))
    ///     .get_matches_from_safe(vec!["prog", "NotANumber"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::ValueValidation);
    /// ```
    ValueValidation,
    /// Occurs when a user provides more values for an argument than were defined by setting
    /// [`Arg::max_values`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .arg(Arg::with_name("arg")
    ///         .multiple(true)
    ///         .max_values(2))
    ///     .get_matches_from_safe(vec!["prog", "too", "many", "values"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::TooManyValues);
    /// ```
    /// [`Arg::max_values`]: ./struct.Arg.html#method.max_values
    TooManyValues,
    /// Occurs when the user provides fewer values for an argument than were defined by setting
    /// [`Arg::min_values`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .arg(Arg::with_name("some_opt")
    ///         .long("opt")
    ///         .min_values(3))
    ///     .get_matches_from_safe(vec!["prog", "--opt", "too", "few"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::TooFewValues);
    /// ```
    /// [`Arg::min_values`]: ./struct.Arg.html#method.min_values
    TooFewValues,
    /// Occurs when the user provides a different number of values for an argument than what's
    /// been defined by setting [`Arg::number_of_values`] or than was implicitly set by
    /// [`Arg::value_names`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .arg(Arg::with_name("some_opt")
    ///         .long("opt")
    ///         .takes_value(true)
    ///         .number_of_values(2))
    ///     .get_matches_from_safe(vec!["prog", "--opt", "wrong"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::WrongNumberOfValues);
    /// ```
    ///
    /// [`Arg::number_of_values`]: ./struct.Arg.html#method.number_of_values
    /// [`Arg::value_names`]: ./struct.Arg.html#method.value_names
    WrongNumberOfValues,
    /// Occurs when the user provides two values which conflict with each other and can't be used
    /// together.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .arg(Arg::with_name("debug")
    ///         .long("debug")
    ///         .conflicts_with("color"))
    ///     .arg(Arg::with_name("color")
    ///         .long("color"))
    ///     .get_matches_from_safe(vec!["prog", "--debug", "--color"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::ArgumentConflict);
    /// ```
    ArgumentConflict,
    /// Occurs when the user does not provide one or more required arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .arg(Arg::with_name("debug")
    ///         .required(true))
    ///     .get_matches_from_safe(vec!["prog"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    MissingRequiredArgument,
    /// Occurs when a subcommand is required (as defined by [`AppSettings::SubcommandRequired`]),
    /// but the user does not provide one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, AppSettings, SubCommand, ErrorKind};
    /// let err = App::new("prog")
    ///     .setting(AppSettings::SubcommandRequired)
    ///     .subcommand(SubCommand::with_name("test"))
    ///     .get_matches_from_safe(vec![
    ///         "myprog",
    ///     ]);
    /// assert!(err.is_err());
    /// assert_eq!(err.unwrap_err().kind, ErrorKind::MissingSubcommand);
    /// # ;
    /// ```
    /// [`AppSettings::SubcommandRequired`]: ./enum.AppSettings.html#variant.SubcommandRequired
    MissingSubcommand,
    /// Occurs when either an argument or [`SubCommand`] is required, as defined by
    /// [`AppSettings::ArgRequiredElseHelp`], but the user did not provide one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, AppSettings, ErrorKind, SubCommand};
    /// let result = App::new("prog")
    ///     .setting(AppSettings::ArgRequiredElseHelp)
    ///     .subcommand(SubCommand::with_name("config")
    ///         .about("Used for configuration")
    ///         .arg(Arg::with_name("config_file")
    ///             .help("The configuration file to use")))
    ///     .get_matches_from_safe(vec!["prog"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::MissingArgumentOrSubcommand);
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`AppSettings::ArgRequiredElseHelp`]: ./enum.AppSettings.html#variant.ArgRequiredElseHelp
    MissingArgumentOrSubcommand,
    /// Occurs when the user provides multiple values to an argument which doesn't allow that.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .arg(Arg::with_name("debug")
    ///         .long("debug")
    ///         .multiple(false))
    ///     .get_matches_from_safe(vec!["prog", "--debug", "--debug"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::UnexpectedMultipleUsage);
    /// ```
    UnexpectedMultipleUsage,
    /// Occurs when the user provides a value containing invalid UTF-8 for an argument and
    /// [`AppSettings::StrictUtf8`] is set.
    ///
    /// # Platform Specific
    ///
    /// Non-Windows platforms only (such as Linux, Unix, macOS, etc.)
    ///
    /// # Examples
    ///
    #[cfg_attr(not(unix), doc = " ```ignore")]
    #[cfg_attr(unix, doc = " ```")]
    /// # use clap::{App, Arg, ErrorKind, AppSettings};
    /// # use std::os::unix::ffi::OsStringExt;
    /// # use std::ffi::OsString;
    /// let result = App::new("prog")
    ///     .setting(AppSettings::StrictUtf8)
    ///     .arg(Arg::with_name("utf8")
    ///         .short("u")
    ///         .takes_value(true))
    ///     .get_matches_from_safe(vec![OsString::from("myprog"),
    ///                                 OsString::from("-u"),
    ///                                 OsString::from_vec(vec![0xE9])]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::InvalidUtf8);
    /// ```
    /// [`AppSettings::StrictUtf8`]: ./enum.AppSettings.html#variant.StrictUtf8
    InvalidUtf8,
    /// Not a true "error" as it means `--help` or similar was used.
    /// The help message will be sent to `stdout`.
    ///
    /// **Note**: If the help is displayed due to an error (such as missing subcommands) it will
    /// be sent to `stderr` instead of `stdout`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .get_matches_from_safe(vec!["prog", "--help"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::HelpDisplayed);
    /// ```
    HelpDisplayed,
    /// Not a true "error" as it means `--version` or similar was used.
    /// The message will be sent to `stdout`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let result = App::new("prog")
    ///     .get_matches_from_safe(vec!["prog", "--version"]);
    /// assert!(result.is_err());
    /// assert_eq!(result.unwrap_err().kind, ErrorKind::VersionDisplayed);
    /// ```
    VersionDisplayed,
    /// Occurs when using the [`value_t!`] and [`values_t!`] macros to convert an argument value
    /// into type `T`, but the argument you requested wasn't used. I.e. you asked for an argument
    /// with name `config` to be converted, but `config` wasn't used by the user.
    /// [`value_t!`]: ./macro.value_t!.html
    /// [`values_t!`]: ./macro.values_t!.html
    ArgumentNotFound,
    /// Represents an [I/O error].
    /// Can occur when writing to `stderr` or `stdout` or reading a configuration file.
    /// [I/O error]: https://doc.rust-lang.org/std/io/struct.Error.html
    Io,
    /// Represents a [Format error] (which is a part of [`Display`]).
    /// Typically caused by writing to `stderr` or `stdout`.
    ///
    /// [`Display`]: https://doc.rust-lang.org/std/fmt/trait.Display.html
    /// [Format error]: https://doc.rust-lang.org/std/fmt/struct.Error.html
    Format,
}
/// Command Line Argument Parser Error
#[derive(Debug)]
pub struct Error {
    /// Formatted error message
    pub message: String,
    /// The type of error
    pub kind: ErrorKind,
    /// Any additional information passed along, such as the argument name that caused the error
    pub info: Option<Vec<String>>,
}
impl Error {
    /// Should the message be written to `stdout` or not
    pub fn use_stderr(&self) -> bool {
        match self.kind {
            ErrorKind::HelpDisplayed | ErrorKind::VersionDisplayed => false,
            _ => true,
        }
    }
    /// Prints the error to `stderr` and exits with a status of `1`
    pub fn exit(&self) -> ! {
        if self.use_stderr() {
            wlnerr!("{}", self.message);
            process::exit(1);
        }
        let out = io::stdout();
        writeln!(& mut out.lock(), "{}", self.message)
            .expect("Error writing Error to stdout");
        process::exit(0);
    }
    #[doc(hidden)]
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write!(w, "{}", self.message)
    }
    #[doc(hidden)]
    pub fn argument_conflict<O, U>(
        arg: &AnyArg,
        other: Option<O>,
        usage: U,
        color: ColorWhen,
    ) -> Self
    where
        O: Into<String>,
        U: Display,
    {
        let mut v = vec![arg.name().to_owned()];
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The argument '{}' cannot be used with {}\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(&* arg.to_string()), match other {
                Some(name) => { let n = name.into(); v.push(n.clone()); c
                .warning(format!("'{}'", n)) } None => c
                .none("one or more of the other specified arguments".to_owned()), },
                usage, c.good("--help")
            ),
            kind: ErrorKind::ArgumentConflict,
            info: Some(v),
        }
    }
    #[doc(hidden)]
    pub fn empty_value<U>(arg: &AnyArg, usage: U, color: ColorWhen) -> Self
    where
        U: Display,
    {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The argument '{}' requires a value but none was supplied\
                 \n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(arg.to_string()), usage, c.good("--help")
            ),
            kind: ErrorKind::EmptyValue,
            info: Some(vec![arg.name().to_owned()]),
        }
    }
    #[doc(hidden)]
    pub fn invalid_value<B, G, U>(
        bad_val: B,
        good_vals: &[G],
        arg: &AnyArg,
        usage: U,
        color: ColorWhen,
    ) -> Self
    where
        B: AsRef<str>,
        G: AsRef<str> + Display,
        U: Display,
    {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        let suffix = suggestions::did_you_mean_value_suffix(
            bad_val.as_ref(),
            good_vals.iter(),
        );
        let mut sorted = vec![];
        for v in good_vals {
            let val = format!("{}", c.good(v));
            sorted.push(val);
        }
        sorted.sort();
        let valid_values = sorted.join(", ");
        Error {
            message: format!(
                "{} '{}' isn't a valid value for '{}'\n\t\
                 [possible values: {}]\n\
                 {}\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(bad_val.as_ref()), c.warning(arg
                .to_string()), valid_values, suffix.0, usage, c.good("--help")
            ),
            kind: ErrorKind::InvalidValue,
            info: Some(vec![arg.name().to_owned(), bad_val.as_ref().to_owned()]),
        }
    }
    #[doc(hidden)]
    pub fn invalid_subcommand<S, D, N, U>(
        subcmd: S,
        did_you_mean: D,
        name: N,
        usage: U,
        color: ColorWhen,
    ) -> Self
    where
        S: Into<String>,
        D: AsRef<str> + Display,
        N: Display,
        U: Display,
    {
        let s = subcmd.into();
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The subcommand '{}' wasn't recognized\n\t\
                 Did you mean '{}'?\n\n\
                 If you believe you received this message in error, try \
                 re-running with '{} {} {}'\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(&* s), c.good(did_you_mean.as_ref()), name,
                c.good("--"), &* s, usage, c.good("--help")
            ),
            kind: ErrorKind::InvalidSubcommand,
            info: Some(vec![s]),
        }
    }
    #[doc(hidden)]
    pub fn unrecognized_subcommand<S, N>(subcmd: S, name: N, color: ColorWhen) -> Self
    where
        S: Into<String>,
        N: Display,
    {
        let s = subcmd.into();
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The subcommand '{}' wasn't recognized\n\n\
                 {}\n\t\
                 {} help <subcommands>...\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(&* s), c.warning("USAGE:"), name, c
                .good("--help")
            ),
            kind: ErrorKind::UnrecognizedSubcommand,
            info: Some(vec![s]),
        }
    }
    #[doc(hidden)]
    pub fn missing_required_argument<R, U>(
        required: R,
        usage: U,
        color: ColorWhen,
    ) -> Self
    where
        R: Display,
        U: Display,
    {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The following required arguments were not provided:{}\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), required, usage, c.good("--help")
            ),
            kind: ErrorKind::MissingRequiredArgument,
            info: None,
        }
    }
    #[doc(hidden)]
    pub fn missing_subcommand<N, U>(name: N, usage: U, color: ColorWhen) -> Self
    where
        N: AsRef<str> + Display,
        U: Display,
    {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} '{}' requires a subcommand, but one was not provided\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(name), usage, c.good("--help")
            ),
            kind: ErrorKind::MissingSubcommand,
            info: None,
        }
    }
    #[doc(hidden)]
    pub fn invalid_utf8<U>(usage: U, color: ColorWhen) -> Self
    where
        U: Display,
    {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} Invalid UTF-8 was detected in one or more arguments\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), usage, c.good("--help")
            ),
            kind: ErrorKind::InvalidUtf8,
            info: None,
        }
    }
    #[doc(hidden)]
    pub fn too_many_values<V, U>(
        val: V,
        arg: &AnyArg,
        usage: U,
        color: ColorWhen,
    ) -> Self
    where
        V: AsRef<str> + Display + ToOwned,
        U: Display,
    {
        let v = val.as_ref();
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The value '{}' was provided to '{}', but it wasn't expecting \
                 any more values\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(v), c.warning(arg.to_string()), usage, c
                .good("--help")
            ),
            kind: ErrorKind::TooManyValues,
            info: Some(vec![arg.name().to_owned(), v.to_owned()]),
        }
    }
    #[doc(hidden)]
    pub fn too_few_values<U>(
        arg: &AnyArg,
        min_vals: u64,
        curr_vals: usize,
        usage: U,
        color: ColorWhen,
    ) -> Self
    where
        U: Display,
    {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The argument '{}' requires at least {} values, but only {} w{} \
                 provided\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(arg.to_string()), c.warning(min_vals
                .to_string()), c.warning(curr_vals.to_string()), if curr_vals > 1 { "ere"
                } else { "as" }, usage, c.good("--help")
            ),
            kind: ErrorKind::TooFewValues,
            info: Some(vec![arg.name().to_owned()]),
        }
    }
    #[doc(hidden)]
    pub fn value_validation(
        arg: Option<&AnyArg>,
        err: String,
        color: ColorWhen,
    ) -> Self {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} Invalid value{}: {}", c.error("error:"), if let Some(a) = arg {
                format!(" for '{}'", c.warning(a.to_string())) } else { "".to_string() },
                err
            ),
            kind: ErrorKind::ValueValidation,
            info: None,
        }
    }
    #[doc(hidden)]
    pub fn value_validation_auto(err: String) -> Self {
        let n: Option<&AnyArg> = None;
        Error::value_validation(n, err, ColorWhen::Auto)
    }
    #[doc(hidden)]
    pub fn wrong_number_of_values<S, U>(
        arg: &AnyArg,
        num_vals: u64,
        curr_vals: usize,
        suffix: S,
        usage: U,
        color: ColorWhen,
    ) -> Self
    where
        S: Display,
        U: Display,
    {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The argument '{}' requires {} values, but {} w{} \
                 provided\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(arg.to_string()), c.warning(num_vals
                .to_string()), c.warning(curr_vals.to_string()), suffix, usage, c
                .good("--help")
            ),
            kind: ErrorKind::WrongNumberOfValues,
            info: Some(vec![arg.name().to_owned()]),
        }
    }
    #[doc(hidden)]
    pub fn unexpected_multiple_usage<U>(arg: &AnyArg, usage: U, color: ColorWhen) -> Self
    where
        U: Display,
    {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} The argument '{}' was provided more than once, but cannot \
                 be used multiple times\n\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(arg.to_string()), usage, c.good("--help")
            ),
            kind: ErrorKind::UnexpectedMultipleUsage,
            info: Some(vec![arg.name().to_owned()]),
        }
    }
    #[doc(hidden)]
    pub fn unknown_argument<A, U>(
        arg: A,
        did_you_mean: &str,
        usage: U,
        color: ColorWhen,
    ) -> Self
    where
        A: Into<String>,
        U: Display,
    {
        let a = arg.into();
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!(
                "{} Found argument '{}' which wasn't expected, or isn't valid in \
                 this context{}\n\
                 {}\n\n\
                 For more information try {}",
                c.error("error:"), c.warning(&* a), if did_you_mean.is_empty() { "\n"
                .to_owned() } else { format!("{}\n", did_you_mean) }, usage, c
                .good("--help")
            ),
            kind: ErrorKind::UnknownArgument,
            info: Some(vec![a]),
        }
    }
    #[doc(hidden)]
    pub fn io_error(e: &Error, color: ColorWhen) -> Self {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: color,
        });
        Error {
            message: format!("{} {}", c.error("error:"), e.description()),
            kind: ErrorKind::Io,
            info: None,
        }
    }
    #[doc(hidden)]
    pub fn argument_not_found_auto<A>(arg: A) -> Self
    where
        A: Into<String>,
    {
        let a = arg.into();
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: ColorWhen::Auto,
        });
        Error {
            message: format!(
                "{} The argument '{}' wasn't found", c.error("error:"), a.clone()
            ),
            kind: ErrorKind::ArgumentNotFound,
            info: Some(vec![a]),
        }
    }
    /// Create an error with a custom description.
    ///
    /// This can be used in combination with `Error::exit` to exit your program
    /// with a custom error message.
    pub fn with_description(description: &str, kind: ErrorKind) -> Self {
        let c = Colorizer::new(ColorizerOption {
            use_stderr: true,
            when: ColorWhen::Auto,
        });
        Error {
            message: format!("{} {}", c.error("error:"), description),
            kind: kind,
            info: None,
        }
    }
}
impl StdError for Error {
    fn description(&self) -> &str {
        &*self.message
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std_fmt::Formatter) -> std_fmt::Result {
        writeln!(f, "{}", self.message)
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::with_description(e.description(), ErrorKind::Io)
    }
}
impl From<std_fmt::Error> for Error {
    fn from(e: std_fmt::Error) -> Self {
        Error::with_description(e.description(), ErrorKind::Format)
    }
}
#[cfg(test)]
mod tests_llm_16_263 {
    use super::*;
    use crate::*;
    #[test]
    fn use_stderr_help_displayed() {
        let _rug_st_tests_llm_16_263_rrrruuuugggg_use_stderr_help_displayed = 0;
        let rug_fuzz_0 = "Help message";
        let error = Error {
            message: String::from(rug_fuzz_0),
            kind: ErrorKind::HelpDisplayed,
            info: None,
        };
        debug_assert_eq!(error.use_stderr(), false);
        let _rug_ed_tests_llm_16_263_rrrruuuugggg_use_stderr_help_displayed = 0;
    }
    #[test]
    fn use_stderr_version_displayed() {
        let _rug_st_tests_llm_16_263_rrrruuuugggg_use_stderr_version_displayed = 0;
        let rug_fuzz_0 = "Version message";
        let error = Error {
            message: String::from(rug_fuzz_0),
            kind: ErrorKind::VersionDisplayed,
            info: None,
        };
        debug_assert_eq!(error.use_stderr(), false);
        let _rug_ed_tests_llm_16_263_rrrruuuugggg_use_stderr_version_displayed = 0;
    }
    #[test]
    fn use_stderr_other_errors() {
        let _rug_st_tests_llm_16_263_rrrruuuugggg_use_stderr_other_errors = 0;
        let rug_fuzz_0 = "Other error message";
        let error = Error {
            message: String::from(rug_fuzz_0),
            kind: ErrorKind::InvalidValue,
            info: None,
        };
        debug_assert_eq!(error.use_stderr(), true);
        let _rug_ed_tests_llm_16_263_rrrruuuugggg_use_stderr_other_errors = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_266 {
    use super::*;
    use crate::*;
    #[test]
    fn test_with_description() {
        let _rug_st_tests_llm_16_266_rrrruuuugggg_test_with_description = 0;
        let rug_fuzz_0 = "Test description";
        let description = rug_fuzz_0;
        let kind = ErrorKind::InvalidValue;
        let error = Error::with_description(description, kind);
        debug_assert_eq!(error.message, "error: Test description");
        debug_assert_eq!(error.kind, kind);
        debug_assert_eq!(error.info, None);
        let _rug_ed_tests_llm_16_266_rrrruuuugggg_test_with_description = 0;
    }
}
#[cfg(test)]
mod tests_rug_511 {
    use super::*;
    #[test]
    fn test_exit() {
        let _rug_st_tests_rug_511_rrrruuuugggg_test_exit = 0;
        let rug_fuzz_0 = "Test Error Message";
        let rug_fuzz_1 = "arg1";
        let error = Error {
            message: rug_fuzz_0.to_string(),
            kind: ErrorKind::ArgumentNotFound,
            info: Some(vec![rug_fuzz_1.to_string()]),
        };
        error.exit();
        let _rug_ed_tests_rug_511_rrrruuuugggg_test_exit = 0;
    }
}
#[cfg(test)]
mod tests_rug_518 {
    use super::*;
    use crate::Error;
    use crate::fmt::{Colorizer, ColorizerOption, ColorWhen};
    use std::fmt::Display;
    #[test]
    fn test_missing_required_argument() {
        let _rug_st_tests_rug_518_rrrruuuugggg_test_missing_required_argument = 0;
        let rug_fuzz_0 = "input";
        let rug_fuzz_1 = "myapp [FLAGS] [OPTIONS] <input>";
        let required: &str = rug_fuzz_0;
        let usage: &str = rug_fuzz_1;
        let color: ColorWhen = ColorWhen::Never;
        Error::missing_required_argument(required, usage, color);
        let _rug_ed_tests_rug_518_rrrruuuugggg_test_missing_required_argument = 0;
    }
}
#[cfg(test)]
mod tests_rug_520 {
    use super::*;
    use crate::errors::{Error, ErrorKind};
    use crate::fmt::{Colorizer, ColorizerOption, ColorWhen};
    use crate::completions::Shell;
    #[test]
    fn test_invalid_utf8() {
        let _rug_st_tests_rug_520_rrrruuuugggg_test_invalid_utf8 = 0;
        let usage: Shell = Shell::Zsh;
        let color: ColorWhen = ColorWhen::Auto;
        Error::invalid_utf8(usage, color);
        let _rug_ed_tests_rug_520_rrrruuuugggg_test_invalid_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_rug_523 {
    use super::*;
    use crate::args::any_arg::AnyArg;
    use crate::fmt;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_523_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Invalid value";
        let mut p0: Option<&dyn AnyArg> = None;
        let p1: String = rug_fuzz_0.to_string();
        let p2: fmt::ColorWhen = fmt::ColorWhen::Always;
        crate::errors::Error::value_validation(p0, p1, p2);
        let _rug_ed_tests_rug_523_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_524 {
    use super::*;
    use crate::errors::{Error, ColorWhen};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_524_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "This is an error message";
        let mut p0: std::string::String = rug_fuzz_0.to_string();
        let _ = Error::value_validation_auto(p0);
        let _rug_ed_tests_rug_524_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_528 {
    use super::*;
    use crate::errors::{Error, ErrorKind};
    use crate::fmt::{Colorizer, ColorizerOption, ColorWhen};
    #[test]
    fn test_io_error() {
        let _rug_st_tests_rug_528_rrrruuuugggg_test_io_error = 0;
        let rug_fuzz_0 = "error message";
        let rug_fuzz_1 = true;
        let e = Error {
            message: String::from(rug_fuzz_0),
            kind: ErrorKind::Io,
            info: None,
        };
        let color = ColorWhen::Always;
        let c = Colorizer::new(ColorizerOption {
            use_stderr: rug_fuzz_1,
            when: color,
        });
        let result = Error::io_error(&e, color);
        debug_assert_eq!(
            result.message, format!("{} {}", c.error("error:"), e.description())
        );
        debug_assert_eq!(result.kind, ErrorKind::Io);
        debug_assert_eq!(result.info, None);
        let _rug_ed_tests_rug_528_rrrruuuugggg_test_io_error = 0;
    }
}
#[cfg(test)]
mod tests_rug_530 {
    use super::*;
    use crate::Error;
    #[test]
    fn test_description() {
        let _rug_st_tests_rug_530_rrrruuuugggg_test_description = 0;
        let rug_fuzz_0 = "Error message";
        let mut p0: Error = Error::with_description(
            rug_fuzz_0,
            ErrorKind::HelpDisplayed,
        );
        p0.description();
        let _rug_ed_tests_rug_530_rrrruuuugggg_test_description = 0;
    }
}
