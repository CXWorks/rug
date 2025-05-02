mod help;
mod meta;
pub mod parser;
mod settings;
mod usage;
mod validator;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::Path;
use std::process;
use std::rc::Rc;
use std::result::Result as StdResult;
#[cfg(feature = "yaml")]
use yaml_rust::Yaml;
pub use self::settings::AppSettings;
use app::help::Help;
use app::parser::Parser;
use args::{AnyArg, Arg, ArgGroup, ArgMatcher, ArgMatches, ArgSettings};
use completions::Shell;
use errors::Result as ClapResult;
use map::{self, VecMap};
/// Used to create a representation of a command line program and all possible command line
/// arguments. Application settings are set using the "builder pattern" with the
/// [`App::get_matches`] family of methods being the terminal methods that starts the
/// runtime-parsing process. These methods then return information about the user supplied
/// arguments (or lack there of).
///
/// **NOTE:** There aren't any mandatory "options" that one must set. The "options" may
/// also appear in any order (so long as one of the [`App::get_matches`] methods is the last method
/// called).
///
/// # Examples
///
/// ```no_run
/// # use clap::{App, Arg};
/// let m = App::new("My Program")
///     .author("Me, me@mail.com")
///     .version("1.0.2")
///     .about("Explains in brief what the program does")
///     .arg(
///         Arg::with_name("in_file").index(1)
///     )
///     .after_help("Longer explanation to appear after the options when \
///                  displaying the help information from --help or -h")
///     .get_matches();
///
/// // Your program logic starts here...
/// ```
/// [`App::get_matches`]: ./struct.App.html#method.get_matches
#[allow(missing_debug_implementations)]
pub struct App<'a, 'b>
where
    'a: 'b,
{
    #[doc(hidden)]
    pub p: Parser<'a, 'b>,
}
impl<'a, 'b> App<'a, 'b> {
    /// Creates a new instance of an application requiring a name. The name may be, but doesn't
    /// have to be same as the binary. The name will be displayed to the user when they request to
    /// print version or help and usage information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// let prog = App::new("My Program")
    /// # ;
    /// ```
    pub fn new<S: Into<String>>(n: S) -> Self {
        App {
            p: Parser::with_name(n.into()),
        }
    }
    /// Get the name of the app
    pub fn get_name(&self) -> &str {
        &self.p.meta.name
    }
    /// Get the name of the binary
    pub fn get_bin_name(&self) -> Option<&str> {
        self.p.meta.bin_name.as_ref().map(|s| s.as_str())
    }
    /// Creates a new instance of an application requiring a name, but uses the [`crate_authors!`]
    /// and [`crate_version!`] macros to fill in the [`App::author`] and [`App::version`] fields.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// let prog = App::with_defaults("My Program")
    /// # ;
    /// ```
    /// [`crate_authors!`]: ./macro.crate_authors!.html
    /// [`crate_version!`]: ./macro.crate_version!.html
    /// [`App::author`]: ./struct.App.html#method.author
    /// [`App::version`]: ./struct.App.html#method.author
    #[deprecated(
        since = "2.14.1",
        note = "Can never work; use explicit App::author() and App::version() calls instead"
    )]
    pub fn with_defaults<S: Into<String>>(n: S) -> Self {
        let mut a = App {
            p: Parser::with_name(n.into()),
        };
        a.p.meta.author = Some("Kevin K. <kbknapp@gmail.com>");
        a.p.meta.version = Some("2.19.2");
        a
    }
    /// Creates a new instance of [`App`] from a .yml (YAML) file. A full example of supported YAML
    /// objects can be found in [`examples/17_yaml.rs`] and [`examples/17_yaml.yml`]. One great use
    /// for using YAML is when supporting multiple languages and dialects, as each language could
    /// be a distinct YAML file and determined at compiletime via `cargo` "features" in your
    /// `Cargo.toml`
    ///
    /// In order to use this function you must compile `clap` with the `features = ["yaml"]` in
    /// your settings for the `[dependencies.clap]` table of your `Cargo.toml`
    ///
    /// **NOTE:** Due to how the YAML objects are built there is a convenience macro for loading
    /// the YAML file at compile time (relative to the current file, like modules work). That YAML
    /// object can then be passed to this function.
    ///
    /// # Panics
    ///
    /// The YAML file must be properly formatted or this function will [`panic!`]. A good way to
    /// ensure this doesn't happen is to run your program with the `--help` switch. If this passes
    /// without error, you needn't worry because the YAML is properly formatted.
    ///
    /// # Examples
    ///
    /// The following example shows how to load a properly formatted YAML file to build an instance
    /// of an [`App`] struct.
    ///
    /// ```ignore
    /// # #[macro_use]
    /// # extern crate clap;
    /// # use clap::App;
    /// # fn main() {
    /// let yml = load_yaml!("app.yml");
    /// let app = App::from_yaml(yml);
    ///
    /// // continued logic goes here, such as `app.get_matches()` etc.
    /// # }
    /// ```
    /// [`App`]: ./struct.App.html
    /// [`examples/17_yaml.rs`]: https://github.com/clap-rs/clap/blob/v2.33.1/examples/17_yaml.rs
    /// [`examples/17_yaml.yml`]: https://github.com/clap-rs/clap/blob/v2.33.1/examples/17_yaml.yml
    /// [`panic!`]: https://doc.rust-lang.org/std/macro.panic!.html
    #[cfg(feature = "yaml")]
    pub fn from_yaml(yaml: &'a Yaml) -> App<'a, 'a> {
        App::from(yaml)
    }
    /// Sets a string of author(s) that will be displayed to the user when they
    /// request the help information with `--help` or `-h`.
    ///
    /// **Pro-tip:** Use `clap`s convenience macro [`crate_authors!`] to automatically set your
    /// application's author(s) to the same thing as your crate at compile time. See the [`examples/`]
    /// directory for more information
    ///
    /// See the [`examples/`]
    /// directory for more information
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///      .author("Me, me@mymain.com")
    /// # ;
    /// ```
    /// [`crate_authors!`]: ./macro.crate_authors!.html
    /// [`examples/`]: https://github.com/clap-rs/clap/tree/v2.33.1/examples
    pub fn author<S: Into<&'b str>>(mut self, author: S) -> Self {
        self.p.meta.author = Some(author.into());
        self
    }
    /// Overrides the system-determined binary name. This should only be used when absolutely
    /// necessary, such as when the binary name for your application is misleading, or perhaps
    /// *not* how the user should invoke your program.
    ///
    /// **Pro-tip:** When building things such as third party `cargo` subcommands, this setting
    /// **should** be used!
    ///
    /// **NOTE:** This command **should not** be used for [`SubCommand`]s.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("My Program")
    ///      .bin_name("my_binary")
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    pub fn bin_name<S: Into<String>>(mut self, name: S) -> Self {
        self.p.meta.bin_name = Some(name.into());
        self
    }
    /// Sets a string describing what the program does. This will be displayed when displaying help
    /// information with `-h`.
    ///
    /// **NOTE:** If only `about` is provided, and not [`App::long_about`] but the user requests
    /// `--help` clap will still display the contents of `about` appropriately
    ///
    /// **NOTE:** Only [`App::about`] is used in completion script generation in order to be
    /// concise
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .about("Does really amazing things to great people")
    /// # ;
    /// ```
    /// [`App::long_about`]: ./struct.App.html#method.long_about
    pub fn about<S: Into<&'b str>>(mut self, about: S) -> Self {
        self.p.meta.about = Some(about.into());
        self
    }
    /// Sets a string describing what the program does. This will be displayed when displaying help
    /// information.
    ///
    /// **NOTE:** If only `long_about` is provided, and not [`App::about`] but the user requests
    /// `-h` clap will still display the contents of `long_about` appropriately
    ///
    /// **NOTE:** Only [`App::about`] is used in completion script generation in order to be
    /// concise
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .long_about(
    /// "Does really amazing things to great people. Now let's talk a little
    ///  more in depth about how this subcommand really works. It may take about
    ///  a few lines of text, but that's ok!")
    /// # ;
    /// ```
    /// [`App::about`]: ./struct.App.html#method.about
    pub fn long_about<S: Into<&'b str>>(mut self, about: S) -> Self {
        self.p.meta.long_about = Some(about.into());
        self
    }
    /// Sets the program's name. This will be displayed when displaying help information.
    ///
    /// **Pro-top:** This function is particularly useful when configuring a program via
    /// [`App::from_yaml`] in conjunction with the [`crate_name!`] macro to derive the program's
    /// name from its `Cargo.toml`.
    ///
    /// # Examples
    /// ```ignore
    /// # #[macro_use]
    /// # extern crate clap;
    /// # use clap::App;
    /// # fn main() {
    /// let yml = load_yaml!("app.yml");
    /// let app = App::from_yaml(yml)
    ///     .name(crate_name!());
    ///
    /// // continued logic goes here, such as `app.get_matches()` etc.
    /// # }
    /// ```
    ///
    /// [`App::from_yaml`]: ./struct.App.html#method.from_yaml
    /// [`crate_name!`]: ./macro.crate_name.html
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.p.meta.name = name.into();
        self
    }
    /// Adds additional help information to be displayed in addition to auto-generated help. This
    /// information is displayed **after** the auto-generated help information. This is often used
    /// to describe how to use the arguments, or caveats to be noted.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::App;
    /// App::new("myprog")
    ///     .after_help("Does really amazing things to great people...but be careful with -R")
    /// # ;
    /// ```
    pub fn after_help<S: Into<&'b str>>(mut self, help: S) -> Self {
        self.p.meta.more_help = Some(help.into());
        self
    }
    /// Adds additional help information to be displayed in addition to auto-generated help. This
    /// information is displayed **before** the auto-generated help information. This is often used
    /// for header information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::App;
    /// App::new("myprog")
    ///     .before_help("Some info I'd like to appear before the help info")
    /// # ;
    /// ```
    pub fn before_help<S: Into<&'b str>>(mut self, help: S) -> Self {
        self.p.meta.pre_help = Some(help.into());
        self
    }
    /// Sets a string of the version number to be displayed when displaying version or help
    /// information with `-V`.
    ///
    /// **NOTE:** If only `version` is provided, and not [`App::long_version`] but the user
    /// requests `--version` clap will still display the contents of `version` appropriately
    ///
    /// **Pro-tip:** Use `clap`s convenience macro [`crate_version!`] to automatically set your
    /// application's version to the same thing as your crate at compile time. See the [`examples/`]
    /// directory for more information
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .version("v0.1.24")
    /// # ;
    /// ```
    /// [`crate_version!`]: ./macro.crate_version!.html
    /// [`examples/`]: https://github.com/clap-rs/clap/tree/v2.33.1/examples
    /// [`App::long_version`]: ./struct.App.html#method.long_version
    pub fn version<S: Into<&'b str>>(mut self, ver: S) -> Self {
        self.p.meta.version = Some(ver.into());
        self
    }
    /// Sets a string of the version number to be displayed when displaying version or help
    /// information with `--version`.
    ///
    /// **NOTE:** If only `long_version` is provided, and not [`App::version`] but the user
    /// requests `-V` clap will still display the contents of `long_version` appropriately
    ///
    /// **Pro-tip:** Use `clap`s convenience macro [`crate_version!`] to automatically set your
    /// application's version to the same thing as your crate at compile time. See the [`examples/`]
    /// directory for more information
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .long_version(
    /// "v0.1.24
    ///  commit: abcdef89726d
    ///  revision: 123
    ///  release: 2
    ///  binary: myprog")
    /// # ;
    /// ```
    /// [`crate_version!`]: ./macro.crate_version!.html
    /// [`examples/`]: https://github.com/clap-rs/clap/tree/v2.33.1/examples
    /// [`App::version`]: ./struct.App.html#method.version
    pub fn long_version<S: Into<&'b str>>(mut self, ver: S) -> Self {
        self.p.meta.long_version = Some(ver.into());
        self
    }
    /// Sets a custom usage string to override the auto-generated usage string.
    ///
    /// This will be displayed to the user when errors are found in argument parsing, or when you
    /// call [`ArgMatches::usage`]
    ///
    /// **CAUTION:** Using this setting disables `clap`s "context-aware" usage strings. After this
    /// setting is set, this will be the only usage string displayed to the user!
    ///
    /// **NOTE:** You do not need to specify the "USAGE: \n\t" portion, as that will
    /// still be applied by `clap`, you only need to specify the portion starting
    /// with the binary name.
    ///
    /// **NOTE:** This will not replace the entire help message, *only* the portion
    /// showing the usage.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .usage("myapp [-clDas] <some_file>")
    /// # ;
    /// ```
    /// [`ArgMatches::usage`]: ./struct.ArgMatches.html#method.usage
    pub fn usage<S: Into<&'b str>>(mut self, usage: S) -> Self {
        self.p.meta.usage_str = Some(usage.into());
        self
    }
    /// Sets a custom help message and overrides the auto-generated one. This should only be used
    /// when the auto-generated message does not suffice.
    ///
    /// This will be displayed to the user when they use `--help` or `-h`
    ///
    /// **NOTE:** This replaces the **entire** help message, so nothing will be auto-generated.
    ///
    /// **NOTE:** This **only** replaces the help message for the current command, meaning if you
    /// are using subcommands, those help messages will still be auto-generated unless you
    /// specify a [`Arg::help`] for them as well.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myapp")
    ///     .help("myapp v1.0\n\
    ///            Does awesome things\n\
    ///            (C) me@mail.com\n\n\
    ///
    ///            USAGE: myapp <opts> <command>\n\n\
    ///
    ///            Options:\n\
    ///            -h, --help       Display this message\n\
    ///            -V, --version    Display version info\n\
    ///            -s <stuff>       Do something with stuff\n\
    ///            -v               Be verbose\n\n\
    ///
    ///            Commmands:\n\
    ///            help             Prints this message\n\
    ///            work             Do some work")
    /// # ;
    /// ```
    /// [`Arg::help`]: ./struct.Arg.html#method.help
    pub fn help<S: Into<&'b str>>(mut self, help: S) -> Self {
        self.p.meta.help_str = Some(help.into());
        self
    }
    /// Sets the [`short`] for the auto-generated `help` argument.
    ///
    /// By default `clap` automatically assigns `h`, but this can be overridden if you have a
    /// different argument which you'd prefer to use the `-h` short with. This can be done by
    /// defining your own argument with a lowercase `h` as the [`short`].
    ///
    /// `clap` lazily generates these `help` arguments **after** you've defined any arguments of
    /// your own.
    ///
    /// **NOTE:** Any leading `-` characters will be stripped, and only the first
    /// non `-` character will be used as the [`short`] version
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .help_short("H") // Using an uppercase `H` instead of the default lowercase `h`
    /// # ;
    /// ```
    /// [`short`]: ./struct.Arg.html#method.short
    pub fn help_short<S: AsRef<str> + 'b>(mut self, s: S) -> Self {
        self.p.help_short(s.as_ref());
        self
    }
    /// Sets the [`short`] for the auto-generated `version` argument.
    ///
    /// By default `clap` automatically assigns `V`, but this can be overridden if you have a
    /// different argument which you'd prefer to use the `-V` short with. This can be done by
    /// defining your own argument with an uppercase `V` as the [`short`].
    ///
    /// `clap` lazily generates these `version` arguments **after** you've defined any arguments of
    /// your own.
    ///
    /// **NOTE:** Any leading `-` characters will be stripped, and only the first
    /// non `-` character will be used as the `short` version
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .version_short("v") // Using a lowercase `v` instead of the default capital `V`
    /// # ;
    /// ```
    /// [`short`]: ./struct.Arg.html#method.short
    pub fn version_short<S: AsRef<str>>(mut self, s: S) -> Self {
        self.p.version_short(s.as_ref());
        self
    }
    /// Sets the help text for the auto-generated `help` argument.
    ///
    /// By default `clap` sets this to `"Prints help information"`, but if you're using a
    /// different convention for your help messages and would prefer a different phrasing you can
    /// override it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .help_message("Print help information") // Perhaps you want imperative help messages
    ///
    /// # ;
    /// ```
    pub fn help_message<S: Into<&'a str>>(mut self, s: S) -> Self {
        self.p.help_message = Some(s.into());
        self
    }
    /// Sets the help text for the auto-generated `version` argument.
    ///
    /// By default `clap` sets this to `"Prints version information"`, but if you're using a
    /// different convention for your help messages and would prefer a different phrasing then you
    /// can change it.
    ///
    /// # Examples
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .version_message("Print version information") // Perhaps you want imperative help messages
    /// # ;
    /// ```
    pub fn version_message<S: Into<&'a str>>(mut self, s: S) -> Self {
        self.p.version_message = Some(s.into());
        self
    }
    /// Sets the help template to be used, overriding the default format.
    ///
    /// Tags arg given inside curly brackets.
    ///
    /// Valid tags are:
    ///
    ///   * `{bin}`         - Binary name.
    ///   * `{version}`     - Version number.
    ///   * `{author}`      - Author information.
    ///   * `{about}`       - General description (from [`App::about`])
    ///   * `{usage}`       - Automatically generated or given usage string.
    ///   * `{all-args}`    - Help for all arguments (options, flags, positionals arguments,
    ///                       and subcommands) including titles.
    ///   * `{unified}`     - Unified help for options and flags. Note, you must *also* set
    ///                       [`AppSettings::UnifiedHelpMessage`] to fully merge both options and
    ///                       flags, otherwise the ordering is "best effort"
    ///   * `{flags}`       - Help for flags.
    ///   * `{options}`     - Help for options.
    ///   * `{positionals}` - Help for positionals arguments.
    ///   * `{subcommands}` - Help for subcommands.
    ///   * `{after-help}`  - Help from [`App::after_help`]
    ///   * `{before-help}`  - Help from [`App::before_help`]
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .version("1.0")
    ///     .template("{bin} ({version}) - {usage}")
    /// # ;
    /// ```
    /// **NOTE:** The template system is, on purpose, very simple. Therefore the tags have to be
    /// written in lowercase and without spacing.
    ///
    /// [`App::about`]: ./struct.App.html#method.about
    /// [`App::after_help`]: ./struct.App.html#method.after_help
    /// [`App::before_help`]: ./struct.App.html#method.before_help
    /// [`AppSettings::UnifiedHelpMessage`]: ./enum.AppSettings.html#variant.UnifiedHelpMessage
    pub fn template<S: Into<&'b str>>(mut self, s: S) -> Self {
        self.p.meta.template = Some(s.into());
        self
    }
    /// Enables a single command, or [`SubCommand`], level settings.
    ///
    /// See [`AppSettings`] for a full list of possibilities and examples.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg, AppSettings};
    /// App::new("myprog")
    ///     .setting(AppSettings::SubcommandRequired)
    ///     .setting(AppSettings::WaitOnError)
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`AppSettings`]: ./enum.AppSettings.html
    pub fn setting(mut self, setting: AppSettings) -> Self {
        self.p.set(setting);
        self
    }
    /// Enables multiple command, or [`SubCommand`], level settings
    ///
    /// See [`AppSettings`] for a full list of possibilities and examples.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg, AppSettings};
    /// App::new("myprog")
    ///     .settings(&[AppSettings::SubcommandRequired,
    ///                  AppSettings::WaitOnError])
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`AppSettings`]: ./enum.AppSettings.html
    pub fn settings(mut self, settings: &[AppSettings]) -> Self {
        for s in settings {
            self.p.set(*s);
        }
        self
    }
    /// Enables a single setting that is propagated down through all child [`SubCommand`]s.
    ///
    /// See [`AppSettings`] for a full list of possibilities and examples.
    ///
    /// **NOTE**: The setting is *only* propagated *down* and not up through parent commands.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg, AppSettings};
    /// App::new("myprog")
    ///     .global_setting(AppSettings::SubcommandRequired)
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`AppSettings`]: ./enum.AppSettings.html
    pub fn global_setting(mut self, setting: AppSettings) -> Self {
        self.p.set(setting);
        self.p.g_settings.set(setting);
        self
    }
    /// Enables multiple settings which are propagated *down* through all child [`SubCommand`]s.
    ///
    /// See [`AppSettings`] for a full list of possibilities and examples.
    ///
    /// **NOTE**: The setting is *only* propagated *down* and not up through parent commands.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg, AppSettings};
    /// App::new("myprog")
    ///     .global_settings(&[AppSettings::SubcommandRequired,
    ///                  AppSettings::ColoredHelp])
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`AppSettings`]: ./enum.AppSettings.html
    pub fn global_settings(mut self, settings: &[AppSettings]) -> Self {
        for s in settings {
            self.p.set(*s);
            self.p.g_settings.set(*s)
        }
        self
    }
    /// Disables a single command, or [`SubCommand`], level setting.
    ///
    /// See [`AppSettings`] for a full list of possibilities and examples.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, AppSettings};
    /// App::new("myprog")
    ///     .unset_setting(AppSettings::ColorAuto)
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`AppSettings`]: ./enum.AppSettings.html
    pub fn unset_setting(mut self, setting: AppSettings) -> Self {
        self.p.unset(setting);
        self
    }
    /// Disables multiple command, or [`SubCommand`], level settings.
    ///
    /// See [`AppSettings`] for a full list of possibilities and examples.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, AppSettings};
    /// App::new("myprog")
    ///     .unset_settings(&[AppSettings::ColorAuto,
    ///                       AppSettings::AllowInvalidUtf8])
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`AppSettings`]: ./enum.AppSettings.html
    pub fn unset_settings(mut self, settings: &[AppSettings]) -> Self {
        for s in settings {
            self.p.unset(*s);
        }
        self
    }
    /// Sets the terminal width at which to wrap help messages. Defaults to `120`. Using `0` will
    /// ignore terminal widths and use source formatting.
    ///
    /// `clap` automatically tries to determine the terminal width on Unix, Linux, macOS and Windows
    /// if the `wrap_help` cargo "feature" has been used while compiling. If the terminal width
    /// cannot be determined, `clap` defaults to `120`.
    ///
    /// **NOTE:** This setting applies globally and *not* on a per-command basis.
    ///
    /// **NOTE:** This setting must be set **before** any subcommands are added!
    ///
    /// # Platform Specific
    ///
    /// Only Unix, Linux, macOS and Windows support automatic determination of terminal width.
    /// Even on those platforms, this setting is useful if for any reason the terminal width
    /// cannot be determined.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::App;
    /// App::new("myprog")
    ///     .set_term_width(80)
    /// # ;
    /// ```
    pub fn set_term_width(mut self, width: usize) -> Self {
        self.p.meta.term_w = Some(width);
        self
    }
    /// Sets the max terminal width at which to wrap help messages. Using `0` will ignore terminal
    /// widths and use source formatting.
    ///
    /// `clap` automatically tries to determine the terminal width on Unix, Linux, macOS and Windows
    /// if the `wrap_help` cargo "feature" has been used while compiling, but one might want to
    /// limit the size (e.g. when the terminal is running fullscreen).
    ///
    /// **NOTE:** This setting applies globally and *not* on a per-command basis.
    ///
    /// **NOTE:** This setting must be set **before** any subcommands are added!
    ///
    /// # Platform Specific
    ///
    /// Only Unix, Linux, macOS and Windows support automatic determination of terminal width.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::App;
    /// App::new("myprog")
    ///     .max_term_width(100)
    /// # ;
    /// ```
    pub fn max_term_width(mut self, w: usize) -> Self {
        self.p.meta.max_w = Some(w);
        self
    }
    /// Adds an [argument] to the list of valid possibilities.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     // Adding a single "flag" argument with a short and help text, using Arg::with_name()
    ///     .arg(
    ///         Arg::with_name("debug")
    ///            .short("d")
    ///            .help("turns on debugging mode")
    ///     )
    ///     // Adding a single "option" argument with a short, a long, and help text using the less
    ///     // verbose Arg::from_usage()
    ///     .arg(
    ///         Arg::from_usage("-c --config=[CONFIG] 'Optionally sets a config file to use'")
    ///     )
    /// # ;
    /// ```
    /// [argument]: ./struct.Arg.html
    pub fn arg<A: Into<Arg<'a, 'b>>>(mut self, a: A) -> Self {
        self.p.add_arg(a.into());
        self
    }
    /// Adds multiple [arguments] to the list of valid possibilities
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .args(
    ///         &[Arg::from_usage("[debug] -d 'turns on debugging info'"),
    ///          Arg::with_name("input").index(1).help("the input file to use")]
    ///     )
    /// # ;
    /// ```
    /// [arguments]: ./struct.Arg.html
    pub fn args(mut self, args: &[Arg<'a, 'b>]) -> Self {
        for arg in args {
            self.p.add_arg_ref(arg);
        }
        self
    }
    /// A convenience method for adding a single [argument] from a usage type string. The string
    /// used follows the same rules and syntax as [`Arg::from_usage`]
    ///
    /// **NOTE:** The downside to using this method is that you can not set any additional
    /// properties of the [`Arg`] other than what [`Arg::from_usage`] supports.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .arg_from_usage("-c --config=<FILE> 'Sets a configuration file to use'")
    /// # ;
    /// ```
    /// [argument]: ./struct.Arg.html
    /// [`Arg`]: ./struct.Arg.html
    /// [`Arg::from_usage`]: ./struct.Arg.html#method.from_usage
    pub fn arg_from_usage(mut self, usage: &'a str) -> Self {
        self.p.add_arg(Arg::from_usage(usage));
        self
    }
    /// Adds multiple [arguments] at once from a usage string, one per line. See
    /// [`Arg::from_usage`] for details on the syntax and rules supported.
    ///
    /// **NOTE:** Like [`App::arg_from_usage`] the downside is you only set properties for the
    /// [`Arg`]s which [`Arg::from_usage`] supports.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// App::new("myprog")
    ///     .args_from_usage(
    ///         "-c --config=[FILE] 'Sets a configuration file to use'
    ///          [debug]... -d 'Sets the debugging level'
    ///          <FILE> 'The input file to use'"
    ///     )
    /// # ;
    /// ```
    /// [arguments]: ./struct.Arg.html
    /// [`Arg::from_usage`]: ./struct.Arg.html#method.from_usage
    /// [`App::arg_from_usage`]: ./struct.App.html#method.arg_from_usage
    /// [`Arg`]: ./struct.Arg.html
    pub fn args_from_usage(mut self, usage: &'a str) -> Self {
        for line in usage.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            self.p.add_arg(Arg::from_usage(l));
        }
        self
    }
    /// Allows adding a [`SubCommand`] alias, which function as "hidden" subcommands that
    /// automatically dispatch as if this subcommand was used. This is more efficient, and easier
    /// than creating multiple hidden subcommands as one only needs to check for the existence of
    /// this command, and not all variants.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg, SubCommand};
    /// let m = App::new("myprog")
    ///             .subcommand(SubCommand::with_name("test")
    ///                 .alias("do-stuff"))
    ///             .get_matches_from(vec!["myprog", "do-stuff"]);
    /// assert_eq!(m.subcommand_name(), Some("test"));
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    pub fn alias<S: Into<&'b str>>(mut self, name: S) -> Self {
        if let Some(ref mut als) = self.p.meta.aliases {
            als.push((name.into(), false));
        } else {
            self.p.meta.aliases = Some(vec![(name.into(), false)]);
        }
        self
    }
    /// Allows adding [`SubCommand`] aliases, which function as "hidden" subcommands that
    /// automatically dispatch as if this subcommand was used. This is more efficient, and easier
    /// than creating multiple hidden subcommands as one only needs to check for the existence of
    /// this command, and not all variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, SubCommand};
    /// let m = App::new("myprog")
    ///             .subcommand(SubCommand::with_name("test")
    ///                 .aliases(&["do-stuff", "do-tests", "tests"]))
    ///                 .arg(Arg::with_name("input")
    ///                             .help("the file to add")
    ///                             .index(1)
    ///                             .required(false))
    ///             .get_matches_from(vec!["myprog", "do-tests"]);
    /// assert_eq!(m.subcommand_name(), Some("test"));
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    pub fn aliases(mut self, names: &[&'b str]) -> Self {
        if let Some(ref mut als) = self.p.meta.aliases {
            for n in names {
                als.push((n, false));
            }
        } else {
            self
                .p
                .meta
                .aliases = Some(names.iter().map(|n| (*n, false)).collect::<Vec<_>>());
        }
        self
    }
    /// Allows adding a [`SubCommand`] alias that functions exactly like those defined with
    /// [`App::alias`], except that they are visible inside the help message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg, SubCommand};
    /// let m = App::new("myprog")
    ///             .subcommand(SubCommand::with_name("test")
    ///                 .visible_alias("do-stuff"))
    ///             .get_matches_from(vec!["myprog", "do-stuff"]);
    /// assert_eq!(m.subcommand_name(), Some("test"));
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`App::alias`]: ./struct.App.html#method.alias
    pub fn visible_alias<S: Into<&'b str>>(mut self, name: S) -> Self {
        if let Some(ref mut als) = self.p.meta.aliases {
            als.push((name.into(), true));
        } else {
            self.p.meta.aliases = Some(vec![(name.into(), true)]);
        }
        self
    }
    /// Allows adding multiple [`SubCommand`] aliases that functions exactly like those defined
    /// with [`App::aliases`], except that they are visible inside the help message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg, SubCommand};
    /// let m = App::new("myprog")
    ///             .subcommand(SubCommand::with_name("test")
    ///                 .visible_aliases(&["do-stuff", "tests"]))
    ///             .get_matches_from(vec!["myprog", "do-stuff"]);
    /// assert_eq!(m.subcommand_name(), Some("test"));
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`App::aliases`]: ./struct.App.html#method.aliases
    pub fn visible_aliases(mut self, names: &[&'b str]) -> Self {
        if let Some(ref mut als) = self.p.meta.aliases {
            for n in names {
                als.push((n, true));
            }
        } else {
            self
                .p
                .meta
                .aliases = Some(names.iter().map(|n| (*n, true)).collect::<Vec<_>>());
        }
        self
    }
    /// Adds an [`ArgGroup`] to the application. [`ArgGroup`]s are a family of related arguments.
    /// By placing them in a logical group, you can build easier requirement and exclusion rules.
    /// For instance, you can make an entire [`ArgGroup`] required, meaning that one (and *only*
    /// one) argument from that group must be present at runtime.
    ///
    /// You can also do things such as name an [`ArgGroup`] as a conflict to another argument.
    /// Meaning any of the arguments that belong to that group will cause a failure if present with
    /// the conflicting argument.
    ///
    /// Another added benefit of [`ArgGroup`]s is that you can extract a value from a group instead
    /// of determining exactly which argument was used.
    ///
    /// Finally, using [`ArgGroup`]s to ensure exclusion between arguments is another very common
    /// use
    ///
    /// # Examples
    ///
    /// The following example demonstrates using an [`ArgGroup`] to ensure that one, and only one,
    /// of the arguments from the specified group is present at runtime.
    ///
    /// ```no_run
    /// # use clap::{App, ArgGroup};
    /// App::new("app")
    ///     .args_from_usage(
    ///         "--set-ver [ver] 'set the version manually'
    ///          --major         'auto increase major'
    ///          --minor         'auto increase minor'
    ///          --patch         'auto increase patch'")
    ///     .group(ArgGroup::with_name("vers")
    ///          .args(&["set-ver", "major", "minor","patch"])
    ///          .required(true))
    /// # ;
    /// ```
    /// [`ArgGroup`]: ./struct.ArgGroup.html
    pub fn group(mut self, group: ArgGroup<'a>) -> Self {
        self.p.add_group(group);
        self
    }
    /// Adds multiple [`ArgGroup`]s to the [`App`] at once.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, ArgGroup};
    /// App::new("app")
    ///     .args_from_usage(
    ///         "--set-ver [ver] 'set the version manually'
    ///          --major         'auto increase major'
    ///          --minor         'auto increase minor'
    ///          --patch         'auto increase patch'
    ///          -c [FILE]       'a config file'
    ///          -i [IFACE]      'an interface'")
    ///     .groups(&[
    ///         ArgGroup::with_name("vers")
    ///             .args(&["set-ver", "major", "minor","patch"])
    ///             .required(true),
    ///         ArgGroup::with_name("input")
    ///             .args(&["c", "i"])
    ///     ])
    /// # ;
    /// ```
    /// [`ArgGroup`]: ./struct.ArgGroup.html
    /// [`App`]: ./struct.App.html
    pub fn groups(mut self, groups: &[ArgGroup<'a>]) -> Self {
        for g in groups {
            self = self.group(g.into());
        }
        self
    }
    /// Adds a [`SubCommand`] to the list of valid possibilities. Subcommands are effectively
    /// sub-[`App`]s, because they can contain their own arguments, subcommands, version, usage,
    /// etc. They also function just like [`App`]s, in that they get their own auto generated help,
    /// version, and usage.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg, SubCommand};
    /// App::new("myprog")
    ///     .subcommand(SubCommand::with_name("config")
    ///         .about("Controls configuration features")
    ///         .arg_from_usage("<config> 'Required configuration file to use'"))
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`App`]: ./struct.App.html
    pub fn subcommand(mut self, subcmd: App<'a, 'b>) -> Self {
        self.p.add_subcommand(subcmd);
        self
    }
    /// Adds multiple subcommands to the list of valid possibilities by iterating over an
    /// [`IntoIterator`] of [`SubCommand`]s
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, SubCommand};
    /// # App::new("myprog")
    /// .subcommands( vec![
    ///        SubCommand::with_name("config").about("Controls configuration functionality")
    ///                                 .arg(Arg::with_name("config_file").index(1)),
    ///        SubCommand::with_name("debug").about("Controls debug functionality")])
    /// # ;
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    /// [`IntoIterator`]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html
    pub fn subcommands<I>(mut self, subcmds: I) -> Self
    where
        I: IntoIterator<Item = App<'a, 'b>>,
    {
        for subcmd in subcmds {
            self.p.add_subcommand(subcmd);
        }
        self
    }
    /// Allows custom ordering of [`SubCommand`]s within the help message. Subcommands with a lower
    /// value will be displayed first in the help message. This is helpful when one would like to
    /// emphasise frequently used subcommands, or prioritize those towards the top of the list.
    /// Duplicate values **are** allowed. Subcommands with duplicate display orders will be
    /// displayed in alphabetical order.
    ///
    /// **NOTE:** The default is 999 for all subcommands.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, SubCommand};
    /// let m = App::new("cust-ord")
    ///     .subcommand(SubCommand::with_name("alpha") // typically subcommands are grouped
    ///                                                // alphabetically by name. Subcommands
    ///                                                // without a display_order have a value of
    ///                                                // 999 and are displayed alphabetically with
    ///                                                // all other 999 subcommands
    ///         .about("Some help and text"))
    ///     .subcommand(SubCommand::with_name("beta")
    ///         .display_order(1)   // In order to force this subcommand to appear *first*
    ///                             // all we have to do is give it a value lower than 999.
    ///                             // Any other subcommands with a value of 1 will be displayed
    ///                             // alphabetically with this one...then 2 values, then 3, etc.
    ///         .about("I should be first!"))
    ///     .get_matches_from(vec![
    ///         "cust-ord", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays the following help message
    ///
    /// ```text
    /// cust-ord
    ///
    /// USAGE:
    ///     cust-ord [FLAGS] [OPTIONS]
    ///
    /// FLAGS:
    ///     -h, --help       Prints help information
    ///     -V, --version    Prints version information
    ///
    /// SUBCOMMANDS:
    ///     beta    I should be first!
    ///     alpha   Some help and text
    /// ```
    /// [`SubCommand`]: ./struct.SubCommand.html
    pub fn display_order(mut self, ord: usize) -> Self {
        self.p.meta.disp_ord = ord;
        self
    }
    /// Prints the full help message to [`io::stdout()`] using a [`BufWriter`] using the same
    /// method as if someone ran `-h` to request the help message
    ///
    /// **NOTE:** clap has the ability to distinguish between "short" and "long" help messages
    /// depending on if the user ran [`-h` (short)] or [`--help` (long)]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::App;
    /// let mut app = App::new("myprog");
    /// app.print_help();
    /// ```
    /// [`io::stdout()`]: https://doc.rust-lang.org/std/io/fn.stdout.html
    /// [`BufWriter`]: https://doc.rust-lang.org/std/io/struct.BufWriter.html
    /// [`-h` (short)]: ./struct.Arg.html#method.help
    /// [`--help` (long)]: ./struct.Arg.html#method.long_help
    pub fn print_help(&mut self) -> ClapResult<()> {
        self.p.propagate_globals();
        self.p.propagate_settings();
        self.p.derive_display_order();
        self.p.create_help_and_version();
        let out = io::stdout();
        let mut buf_w = BufWriter::new(out.lock());
        self.write_help(&mut buf_w)
    }
    /// Prints the full help message to [`io::stdout()`] using a [`BufWriter`] using the same
    /// method as if someone ran `--help` to request the help message
    ///
    /// **NOTE:** clap has the ability to distinguish between "short" and "long" help messages
    /// depending on if the user ran [`-h` (short)] or [`--help` (long)]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::App;
    /// let mut app = App::new("myprog");
    /// app.print_long_help();
    /// ```
    /// [`io::stdout()`]: https://doc.rust-lang.org/std/io/fn.stdout.html
    /// [`BufWriter`]: https://doc.rust-lang.org/std/io/struct.BufWriter.html
    /// [`-h` (short)]: ./struct.Arg.html#method.help
    /// [`--help` (long)]: ./struct.Arg.html#method.long_help
    pub fn print_long_help(&mut self) -> ClapResult<()> {
        let out = io::stdout();
        let mut buf_w = BufWriter::new(out.lock());
        self.write_long_help(&mut buf_w)
    }
    /// Writes the full help message to the user to a [`io::Write`] object in the same method as if
    /// the user ran `-h`
    ///
    /// **NOTE:** clap has the ability to distinguish between "short" and "long" help messages
    /// depending on if the user ran [`-h` (short)] or [`--help` (long)]
    ///
    /// **NOTE:** There is a known bug where this method does not write propagated global arguments
    /// or autogenerated arguments (i.e. the default help/version args). Prefer
    /// [`App::write_long_help`] instead if possible!
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::App;
    /// use std::io;
    /// let mut app = App::new("myprog");
    /// let mut out = io::stdout();
    /// app.write_help(&mut out).expect("failed to write to stdout");
    /// ```
    /// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`-h` (short)]: ./struct.Arg.html#method.help
    /// [`--help` (long)]: ./struct.Arg.html#method.long_help
    pub fn write_help<W: Write>(&self, w: &mut W) -> ClapResult<()> {
        Help::write_app_help(w, self, false)
    }
    /// Writes the full help message to the user to a [`io::Write`] object in the same method as if
    /// the user ran `--help`
    ///
    /// **NOTE:** clap has the ability to distinguish between "short" and "long" help messages
    /// depending on if the user ran [`-h` (short)] or [`--help` (long)]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::App;
    /// use std::io;
    /// let mut app = App::new("myprog");
    /// let mut out = io::stdout();
    /// app.write_long_help(&mut out).expect("failed to write to stdout");
    /// ```
    /// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`-h` (short)]: ./struct.Arg.html#method.help
    /// [`--help` (long)]: ./struct.Arg.html#method.long_help
    pub fn write_long_help<W: Write>(&mut self, w: &mut W) -> ClapResult<()> {
        self.p.propagate_globals();
        self.p.propagate_settings();
        self.p.derive_display_order();
        self.p.create_help_and_version();
        Help::write_app_help(w, self, true)
    }
    /// Writes the version message to the user to a [`io::Write`] object as if the user ran `-V`.
    ///
    /// **NOTE:** clap has the ability to distinguish between "short" and "long" version messages
    /// depending on if the user ran [`-V` (short)] or [`--version` (long)]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::App;
    /// use std::io;
    /// let mut app = App::new("myprog");
    /// let mut out = io::stdout();
    /// app.write_version(&mut out).expect("failed to write to stdout");
    /// ```
    /// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`-V` (short)]: ./struct.App.html#method.version
    /// [`--version` (long)]: ./struct.App.html#method.long_version
    pub fn write_version<W: Write>(&self, w: &mut W) -> ClapResult<()> {
        self.p.write_version(w, false).map_err(From::from)
    }
    /// Writes the version message to the user to a [`io::Write`] object
    ///
    /// **NOTE:** clap has the ability to distinguish between "short" and "long" version messages
    /// depending on if the user ran [`-V` (short)] or [`--version` (long)]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::App;
    /// use std::io;
    /// let mut app = App::new("myprog");
    /// let mut out = io::stdout();
    /// app.write_long_version(&mut out).expect("failed to write to stdout");
    /// ```
    /// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`-V` (short)]: ./struct.App.html#method.version
    /// [`--version` (long)]: ./struct.App.html#method.long_version
    pub fn write_long_version<W: Write>(&self, w: &mut W) -> ClapResult<()> {
        self.p.write_version(w, true).map_err(From::from)
    }
    /// Generate a completions file for a specified shell at compile time.
    ///
    /// **NOTE:** to generate the file at compile time you must use a `build.rs` "Build Script"
    ///
    /// # Examples
    ///
    /// The following example generates a bash completion script via a `build.rs` script. In this
    /// simple example, we'll demo a very small application with only a single subcommand and two
    /// args. Real applications could be many multiple levels deep in subcommands, and have tens or
    /// potentially hundreds of arguments.
    ///
    /// First, it helps if we separate out our `App` definition into a separate file. Whether you
    /// do this as a function, or bare App definition is a matter of personal preference.
    ///
    /// ```
    /// // src/cli.rs
    ///
    /// use clap::{App, Arg, SubCommand};
    ///
    /// pub fn build_cli() -> App<'static, 'static> {
    ///     App::new("compl")
    ///         .about("Tests completions")
    ///         .arg(Arg::with_name("file")
    ///             .help("some input file"))
    ///         .subcommand(SubCommand::with_name("test")
    ///             .about("tests things")
    ///             .arg(Arg::with_name("case")
    ///                 .long("case")
    ///                 .takes_value(true)
    ///                 .help("the case to test")))
    /// }
    /// ```
    ///
    /// In our regular code, we can simply call this `build_cli()` function, then call
    /// `get_matches()`, or any of the other normal methods directly after. For example:
    ///
    /// ```ignore
    /// // src/main.rs
    ///
    /// mod cli;
    ///
    /// fn main() {
    ///     let m = cli::build_cli().get_matches();
    ///
    ///     // normal logic continues...
    /// }
    /// ```
    ///
    /// Next, we set up our `Cargo.toml` to use a `build.rs` build script.
    ///
    /// ```toml
    /// # Cargo.toml
    /// build = "build.rs"
    ///
    /// [build-dependencies]
    /// clap = "2.23"
    /// ```
    ///
    /// Next, we place a `build.rs` in our project root.
    ///
    /// ```ignore
    /// extern crate clap;
    ///
    /// use clap::Shell;
    ///
    /// include!("src/cli.rs");
    ///
    /// fn main() {
    ///     let outdir = match env::var_os("OUT_DIR") {
    ///         None => return,
    ///         Some(outdir) => outdir,
    ///     };
    ///     let mut app = build_cli();
    ///     app.gen_completions("myapp",      // We need to specify the bin name manually
    ///                         Shell::Bash,  // Then say which shell to build completions for
    ///                         outdir);      // Then say where write the completions to
    /// }
    /// ```
    /// Now, once we compile there will be a `{bin_name}.bash` file in the directory.
    /// Assuming we compiled with debug mode, it would be somewhere similar to
    /// `<project>/target/debug/build/myapp-<hash>/out/myapp.bash`.
    ///
    /// Fish shell completions will use the file format `{bin_name}.fish`
    pub fn gen_completions<T: Into<OsString>, S: Into<String>>(
        &mut self,
        bin_name: S,
        for_shell: Shell,
        out_dir: T,
    ) {
        self.p.meta.bin_name = Some(bin_name.into());
        self.p.gen_completions(for_shell, out_dir.into());
    }
    /// Generate a completions file for a specified shell at runtime.  Until `cargo install` can
    /// install extra files like a completion script, this may be used e.g. in a command that
    /// outputs the contents of the completion script, to be redirected into a file by the user.
    ///
    /// # Examples
    ///
    /// Assuming a separate `cli.rs` like the [example above](./struct.App.html#method.gen_completions),
    /// we can let users generate a completion script using a command:
    ///
    /// ```ignore
    /// // src/main.rs
    ///
    /// mod cli;
    /// use std::io;
    ///
    /// fn main() {
    ///     let matches = cli::build_cli().get_matches();
    ///
    ///     if matches.is_present("generate-bash-completions") {
    ///         cli::build_cli().gen_completions_to("myapp", Shell::Bash, &mut io::stdout());
    ///     }
    ///
    ///     // normal logic continues...
    /// }
    ///
    /// ```
    ///
    /// Usage:
    ///
    /// ```shell
    /// $ myapp generate-bash-completions > /usr/share/bash-completion/completions/myapp.bash
    /// ```
    pub fn gen_completions_to<W: Write, S: Into<String>>(
        &mut self,
        bin_name: S,
        for_shell: Shell,
        buf: &mut W,
    ) {
        self.p.meta.bin_name = Some(bin_name.into());
        self.p.gen_completions_to(for_shell, buf);
    }
    /// Starts the parsing process, upon a failed parse an error will be displayed to the user and
    /// the process will exit with the appropriate error code. By default this method gets all user
    /// provided arguments from [`env::args_os`] in order to allow for invalid UTF-8 code points,
    /// which are legal on many platforms.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// let matches = App::new("myprog")
    ///     // Args and options go here...
    ///     .get_matches();
    /// ```
    /// [`env::args_os`]: https://doc.rust-lang.org/std/env/fn.args_os.html
    pub fn get_matches(self) -> ArgMatches<'a> {
        self.get_matches_from(&mut env::args_os())
    }
    /// Starts the parsing process. This method will return a [`clap::Result`] type instead of exiting
    /// the process on failed parse. By default this method gets matches from [`env::args_os`]
    ///
    /// **NOTE:** This method WILL NOT exit when `--help` or `--version` (or short versions) are
    /// used. It will return a [`clap::Error`], where the [`kind`] is a
    /// [`ErrorKind::HelpDisplayed`] or [`ErrorKind::VersionDisplayed`] respectively. You must call
    /// [`Error::exit`] or perform a [`std::process::exit`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// let matches = App::new("myprog")
    ///     // Args and options go here...
    ///     .get_matches_safe()
    ///     .unwrap_or_else( |e| e.exit() );
    /// ```
    /// [`env::args_os`]: https://doc.rust-lang.org/std/env/fn.args_os.html
    /// [`ErrorKind::HelpDisplayed`]: ./enum.ErrorKind.html#variant.HelpDisplayed
    /// [`ErrorKind::VersionDisplayed`]: ./enum.ErrorKind.html#variant.VersionDisplayed
    /// [`Error::exit`]: ./struct.Error.html#method.exit
    /// [`std::process::exit`]: https://doc.rust-lang.org/std/process/fn.exit.html
    /// [`clap::Result`]: ./type.Result.html
    /// [`clap::Error`]: ./struct.Error.html
    /// [`kind`]: ./struct.Error.html
    pub fn get_matches_safe(self) -> ClapResult<ArgMatches<'a>> {
        self.get_matches_from_safe(&mut env::args_os())
    }
    /// Starts the parsing process. Like [`App::get_matches`] this method does not return a [`clap::Result`]
    /// and will automatically exit with an error message. This method, however, lets you specify
    /// what iterator to use when performing matches, such as a [`Vec`] of your making.
    ///
    /// **NOTE:** The first argument will be parsed as the binary name unless
    /// [`AppSettings::NoBinaryName`] is used
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// let arg_vec = vec!["my_prog", "some", "args", "to", "parse"];
    ///
    /// let matches = App::new("myprog")
    ///     // Args and options go here...
    ///     .get_matches_from(arg_vec);
    /// ```
    /// [`App::get_matches`]: ./struct.App.html#method.get_matches
    /// [`clap::Result`]: ./type.Result.html
    /// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    /// [`AppSettings::NoBinaryName`]: ./enum.AppSettings.html#variant.NoBinaryName
    pub fn get_matches_from<I, T>(mut self, itr: I) -> ArgMatches<'a>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        self.get_matches_from_safe_borrow(itr)
            .unwrap_or_else(|e| {
                if e.use_stderr() {
                    wlnerr!("{}", e.message);
                    if self.p.is_set(AppSettings::WaitOnError) {
                        wlnerr!("\nPress [ENTER] / [RETURN] to continue...");
                        let mut s = String::new();
                        let i = io::stdin();
                        i.lock().read_line(&mut s).unwrap();
                    }
                    drop(self);
                    drop(e);
                    process::exit(1);
                }
                drop(self);
                e.exit()
            })
    }
    /// Starts the parsing process. A combination of [`App::get_matches_from`], and
    /// [`App::get_matches_safe`]
    ///
    /// **NOTE:** This method WILL NOT exit when `--help` or `--version` (or short versions) are
    /// used. It will return a [`clap::Error`], where the [`kind`] is a [`ErrorKind::HelpDisplayed`]
    /// or [`ErrorKind::VersionDisplayed`] respectively. You must call [`Error::exit`] or
    /// perform a [`std::process::exit`] yourself.
    ///
    /// **NOTE:** The first argument will be parsed as the binary name unless
    /// [`AppSettings::NoBinaryName`] is used
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// let arg_vec = vec!["my_prog", "some", "args", "to", "parse"];
    ///
    /// let matches = App::new("myprog")
    ///     // Args and options go here...
    ///     .get_matches_from_safe(arg_vec)
    ///     .unwrap_or_else( |e| { panic!("An error occurs: {}", e) });
    /// ```
    /// [`App::get_matches_from`]: ./struct.App.html#method.get_matches_from
    /// [`App::get_matches_safe`]: ./struct.App.html#method.get_matches_safe
    /// [`ErrorKind::HelpDisplayed`]: ./enum.ErrorKind.html#variant.HelpDisplayed
    /// [`ErrorKind::VersionDisplayed`]: ./enum.ErrorKind.html#variant.VersionDisplayed
    /// [`Error::exit`]: ./struct.Error.html#method.exit
    /// [`std::process::exit`]: https://doc.rust-lang.org/std/process/fn.exit.html
    /// [`clap::Error`]: ./struct.Error.html
    /// [`Error::exit`]: ./struct.Error.html#method.exit
    /// [`kind`]: ./struct.Error.html
    /// [`AppSettings::NoBinaryName`]: ./enum.AppSettings.html#variant.NoBinaryName
    pub fn get_matches_from_safe<I, T>(mut self, itr: I) -> ClapResult<ArgMatches<'a>>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        self.get_matches_from_safe_borrow(itr)
    }
    /// Starts the parsing process without consuming the [`App`] struct `self`. This is normally not
    /// the desired functionality, instead prefer [`App::get_matches_from_safe`] which *does*
    /// consume `self`.
    ///
    /// **NOTE:** The first argument will be parsed as the binary name unless
    /// [`AppSettings::NoBinaryName`] is used
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clap::{App, Arg};
    /// let arg_vec = vec!["my_prog", "some", "args", "to", "parse"];
    ///
    /// let mut app = App::new("myprog");
    ///     // Args and options go here...
    /// let matches = app.get_matches_from_safe_borrow(arg_vec)
    ///     .unwrap_or_else( |e| { panic!("An error occurs: {}", e) });
    /// ```
    /// [`App`]: ./struct.App.html
    /// [`App::get_matches_from_safe`]: ./struct.App.html#method.get_matches_from_safe
    /// [`AppSettings::NoBinaryName`]: ./enum.AppSettings.html#variant.NoBinaryName
    pub fn get_matches_from_safe_borrow<I, T>(
        &mut self,
        itr: I,
    ) -> ClapResult<ArgMatches<'a>>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        if !self.p.is_set(AppSettings::Propagated) {
            self.p.propagate_globals();
            self.p.propagate_settings();
            self.p.derive_display_order();
            self.p.set(AppSettings::Propagated);
        }
        let mut matcher = ArgMatcher::new();
        let mut it = itr.into_iter();
        if !self.p.is_set(AppSettings::NoBinaryName) {
            if let Some(name) = it.next() {
                let bn_os = name.into();
                let p = Path::new(&*bn_os);
                if let Some(f) = p.file_name() {
                    if let Some(s) = f.to_os_string().to_str() {
                        if self.p.meta.bin_name.is_none() {
                            self.p.meta.bin_name = Some(s.to_owned());
                        }
                    }
                }
            }
        }
        if let Err(e) = self.p.get_matches_with(&mut matcher, &mut it.peekable()) {
            return Err(e);
        }
        let global_arg_vec: Vec<&str> = (&self)
            .p
            .global_args
            .iter()
            .map(|ga| ga.b.name)
            .collect();
        matcher.propagate_globals(&global_arg_vec);
        Ok(matcher.into())
    }
}
#[cfg(feature = "yaml")]
impl<'a> From<&'a Yaml> for App<'a, 'a> {
    fn from(mut yaml: &'a Yaml) -> Self {
        use args::SubCommand;
        let mut is_sc = None;
        let mut a = if let Some(name) = yaml["name"].as_str() {
            App::new(name)
        } else {
            let yaml_hash = yaml.as_hash().unwrap();
            let sc_key = yaml_hash.keys().nth(0).unwrap();
            is_sc = Some(yaml_hash.get(sc_key).unwrap());
            App::new(sc_key.as_str().unwrap())
        };
        yaml = if let Some(sc) = is_sc { sc } else { yaml };
        macro_rules! yaml_str {
            ($a:ident, $y:ident, $i:ident) => {
                if let Some(v) = $y [stringify!($i)].as_str() { $a = $a .$i (v); } else
                if $y [stringify!($i)] != Yaml::BadValue {
                panic!("Failed to convert YAML value {:?} to a string", $y
                [stringify!($i)]); }
            };
        }
        yaml_str!(a, yaml, version);
        yaml_str!(a, yaml, long_version);
        yaml_str!(a, yaml, author);
        yaml_str!(a, yaml, bin_name);
        yaml_str!(a, yaml, about);
        yaml_str!(a, yaml, long_about);
        yaml_str!(a, yaml, before_help);
        yaml_str!(a, yaml, after_help);
        yaml_str!(a, yaml, template);
        yaml_str!(a, yaml, usage);
        yaml_str!(a, yaml, help);
        yaml_str!(a, yaml, help_short);
        yaml_str!(a, yaml, version_short);
        yaml_str!(a, yaml, help_message);
        yaml_str!(a, yaml, version_message);
        yaml_str!(a, yaml, alias);
        yaml_str!(a, yaml, visible_alias);
        if let Some(v) = yaml["display_order"].as_i64() {
            a = a.display_order(v as usize);
        } else if yaml["display_order"] != Yaml::BadValue {
            panic!("Failed to convert YAML value {:?} to a u64", yaml["display_order"]);
        }
        if let Some(v) = yaml["setting"].as_str() {
            a = a.setting(v.parse().expect("unknown AppSetting found in YAML file"));
        } else if yaml["setting"] != Yaml::BadValue {
            panic!(
                "Failed to convert YAML value {:?} to an AppSetting", yaml["setting"]
            );
        }
        if let Some(v) = yaml["settings"].as_vec() {
            for ys in v {
                if let Some(s) = ys.as_str() {
                    a = a
                        .setting(
                            s.parse().expect("unknown AppSetting found in YAML file"),
                        );
                }
            }
        } else if let Some(v) = yaml["settings"].as_str() {
            a = a.setting(v.parse().expect("unknown AppSetting found in YAML file"));
        } else if yaml["settings"] != Yaml::BadValue {
            panic!("Failed to convert YAML value {:?} to a string", yaml["settings"]);
        }
        if let Some(v) = yaml["global_setting"].as_str() {
            a = a.setting(v.parse().expect("unknown AppSetting found in YAML file"));
        } else if yaml["global_setting"] != Yaml::BadValue {
            panic!(
                "Failed to convert YAML value {:?} to an AppSetting", yaml["setting"]
            );
        }
        if let Some(v) = yaml["global_settings"].as_vec() {
            for ys in v {
                if let Some(s) = ys.as_str() {
                    a = a
                        .global_setting(
                            s.parse().expect("unknown AppSetting found in YAML file"),
                        );
                }
            }
        } else if let Some(v) = yaml["global_settings"].as_str() {
            a = a
                .global_setting(
                    v.parse().expect("unknown AppSetting found in YAML file"),
                );
        } else if yaml["global_settings"] != Yaml::BadValue {
            panic!(
                "Failed to convert YAML value {:?} to a string", yaml["global_settings"]
            );
        }
        macro_rules! vec_or_str {
            ($a:ident, $y:ident, $as_vec:ident, $as_single:ident) => {
                { let maybe_vec = $y [stringify!($as_vec)].as_vec(); if let Some(vec) =
                maybe_vec { for ys in vec { if let Some(s) = ys.as_str() { $a = $a
                .$as_single (s); } else {
                panic!("Failed to convert YAML value {:?} to a string", ys); } } } else {
                if let Some(s) = $y [stringify!($as_vec)].as_str() { $a = $a .$as_single
                (s); } else if $y [stringify!($as_vec)] != Yaml::BadValue {
                panic!("Failed to convert YAML value {:?} to either a vec or string", $y
                [stringify!($as_vec)]); } } $a }
            };
        }
        a = vec_or_str!(a, yaml, aliases, alias);
        a = vec_or_str!(a, yaml, visible_aliases, visible_alias);
        if let Some(v) = yaml["args"].as_vec() {
            for arg_yaml in v {
                a = a.arg(Arg::from_yaml(arg_yaml.as_hash().unwrap()));
            }
        }
        if let Some(v) = yaml["subcommands"].as_vec() {
            for sc_yaml in v {
                a = a.subcommand(SubCommand::from_yaml(sc_yaml));
            }
        }
        if let Some(v) = yaml["groups"].as_vec() {
            for ag_yaml in v {
                a = a.group(ArgGroup::from(ag_yaml.as_hash().unwrap()));
            }
        }
        a
    }
}
impl<'a, 'b> Clone for App<'a, 'b> {
    fn clone(&self) -> Self {
        App { p: self.p.clone() }
    }
}
impl<'n, 'e> AnyArg<'n, 'e> for App<'n, 'e> {
    fn name(&self) -> &'n str {
        ""
    }
    fn overrides(&self) -> Option<&[&'e str]> {
        None
    }
    fn requires(&self) -> Option<&[(Option<&'e str>, &'n str)]> {
        None
    }
    fn blacklist(&self) -> Option<&[&'e str]> {
        None
    }
    fn required_unless(&self) -> Option<&[&'e str]> {
        None
    }
    fn val_names(&self) -> Option<&VecMap<&'e str>> {
        None
    }
    fn is_set(&self, _: ArgSettings) -> bool {
        false
    }
    fn val_terminator(&self) -> Option<&'e str> {
        None
    }
    fn set(&mut self, _: ArgSettings) {
        unreachable!("App struct does not support AnyArg::set, this is a bug!")
    }
    fn has_switch(&self) -> bool {
        false
    }
    fn max_vals(&self) -> Option<u64> {
        None
    }
    fn num_vals(&self) -> Option<u64> {
        None
    }
    fn possible_vals(&self) -> Option<&[&'e str]> {
        None
    }
    fn validator(&self) -> Option<&Rc<Fn(String) -> StdResult<(), String>>> {
        None
    }
    fn validator_os(&self) -> Option<&Rc<Fn(&OsStr) -> StdResult<(), OsString>>> {
        None
    }
    fn min_vals(&self) -> Option<u64> {
        None
    }
    fn short(&self) -> Option<char> {
        None
    }
    fn long(&self) -> Option<&'e str> {
        None
    }
    fn val_delim(&self) -> Option<char> {
        None
    }
    fn takes_value(&self) -> bool {
        true
    }
    fn help(&self) -> Option<&'e str> {
        self.p.meta.about
    }
    fn long_help(&self) -> Option<&'e str> {
        self.p.meta.long_about
    }
    fn default_val(&self) -> Option<&'e OsStr> {
        None
    }
    fn default_vals_ifs(
        &self,
    ) -> Option<map::Values<(&'n str, Option<&'e OsStr>, &'e OsStr)>> {
        None
    }
    fn env<'s>(&'s self) -> Option<(&'n OsStr, Option<&'s OsString>)> {
        None
    }
    fn longest_filter(&self) -> bool {
        true
    }
    fn aliases(&self) -> Option<Vec<&'e str>> {
        if let Some(ref aliases) = self.p.meta.aliases {
            let vis_aliases: Vec<_> = aliases
                .iter()
                .filter_map(|&(n, v)| if v { Some(n) } else { None })
                .collect();
            if vis_aliases.is_empty() { None } else { Some(vis_aliases) }
        } else {
            None
        }
    }
}
impl<'n, 'e> fmt::Display for App<'n, 'e> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.p.meta.name)
    }
}
#[cfg(test)]
mod tests_rug_239 {
    use super::*;
    use app::App;
    use crate::App as ClapApp;
    #[test]
    fn test_get_name() {
        let _rug_st_tests_rug_239_rrrruuuugggg_test_get_name = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        <App<'static, 'static>>::get_name(&p0);
        let _rug_ed_tests_rug_239_rrrruuuugggg_test_get_name = 0;
    }
}
#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    use crate::App as ClapApp;
    use app::App;
    #[test]
    fn test_get_bin_name() {
        let _rug_st_tests_rug_240_rrrruuuugggg_test_get_bin_name = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let result = p0.get_bin_name();
        debug_assert_eq!(result, None);
        let _rug_ed_tests_rug_240_rrrruuuugggg_test_get_bin_name = 0;
    }
}
#[cfg(test)]
mod tests_rug_242_prepare {
    use app::App;
    use crate::App as ClapApp;
    #[test]
    fn sample() {
        let _rug_st_tests_rug_242_prepare_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "my-app";
        let mut v32: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let _rug_ed_tests_rug_242_prepare_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_243 {
    use super::*;
    use crate::App;
    #[test]
    fn test_bin_name() {
        let _rug_st_tests_rug_243_rrrruuuugggg_test_bin_name = 0;
        let rug_fuzz_0 = "My Program";
        let rug_fuzz_1 = "my_binary";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: String = rug_fuzz_1.into();
        p0.bin_name(p1);
        let _rug_ed_tests_rug_243_rrrruuuugggg_test_bin_name = 0;
    }
}
#[cfg(test)]
mod tests_rug_244 {
    use super::*;
    use crate::App;
    #[test]
    fn test_about() {
        let _rug_st_tests_rug_244_rrrruuuugggg_test_about = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = "Does really amazing things to great people";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: &'static str = rug_fuzz_1;
        p0.about(p1);
        let _rug_ed_tests_rug_244_rrrruuuugggg_test_about = 0;
    }
}
#[cfg(test)]
mod tests_rug_245 {
    use super::*;
    use crate::App;
    #[test]
    fn test_long_about() {
        let _rug_st_tests_rug_245_rrrruuuugggg_test_long_about = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "Does really amazing things to great people. Now let's talk a little more in depth about how this subcommand really works. It may take about a few lines of text, but that's ok!";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1 = rug_fuzz_1;
        p0.long_about(p1);
        let _rug_ed_tests_rug_245_rrrruuuugggg_test_long_about = 0;
    }
}
#[cfg(test)]
mod tests_rug_246 {
    use super::*;
    use crate::app::App;
    use crate::App as ClapApp;
    #[test]
    fn test_name() {
        let _rug_st_tests_rug_246_rrrruuuugggg_test_name = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = "my-app-name";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let p1: String = String::from(rug_fuzz_1);
        p0.name(p1);
        let _rug_ed_tests_rug_246_rrrruuuugggg_test_name = 0;
    }
}
#[cfg(test)]
mod tests_rug_247 {
    use super::*;
    use crate::{App, AppSettings};
    #[test]
    fn test_after_help() {
        let _rug_st_tests_rug_247_rrrruuuugggg_test_after_help = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "Does really amazing things to great people...but be careful with -R";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let p1: &'static str = rug_fuzz_1;
        p0 = p0.after_help(p1);
        let _rug_ed_tests_rug_247_rrrruuuugggg_test_after_help = 0;
    }
}
#[cfg(test)]
mod tests_rug_249 {
    use super::*;
    use crate::App;
    use crate::crate_version;
    use crate::Arg;
    #[test]
    fn test_version() {
        let _rug_st_tests_rug_249_rrrruuuugggg_test_version = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "v0.1.24";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: &'static str = rug_fuzz_1;
        p0.version(p1);
        let _rug_ed_tests_rug_249_rrrruuuugggg_test_version = 0;
    }
}
#[cfg(test)]
mod tests_rug_250 {
    use super::*;
    use crate::{App, Arg};
    #[test]
    fn test_long_version() {
        let _rug_st_tests_rug_250_rrrruuuugggg_test_long_version = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "v0.1.24\ncommit: abcdef89726d\nrevision: 123\nrelease: 2\nbinary: myprog";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: &'static str = rug_fuzz_1;
        p0 = p0.long_version(p1);
        let _rug_ed_tests_rug_250_rrrruuuugggg_test_long_version = 0;
    }
}
#[cfg(test)]
mod tests_rug_251 {
    use super::*;
    use app::App;
    use crate::App as ClapApp;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_251_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = "";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let p1 = rug_fuzz_1;
        p0.usage(p1);
        let _rug_ed_tests_rug_251_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_252 {
    use super::*;
    use crate::{App, Arg};
    #[test]
    fn test_help() {
        let _rug_st_tests_rug_252_rrrruuuugggg_test_help = 0;
        let rug_fuzz_0 = "myapp";
        let rug_fuzz_1 = "myapp v1.0\n\
            Does awesome things\n\
            (C) me@mail.com\n\n\
            USAGE: myapp <opts> <command>\n\n\
            Options:\n\
            -h, --help       Display this message\n\
            -V, --version    Display version info\n\
            -s <stuff>       Do something with stuff\n\
            -v               Be verbose\n\n\
            Commmands:\n\
            help             Prints this message\n\
            work             Do some work";
        let mut p0 = App::new(rug_fuzz_0);
        let p1: &str = rug_fuzz_1;
        p0 = p0.help(p1);
        let _rug_ed_tests_rug_252_rrrruuuugggg_test_help = 0;
    }
}
#[cfg(test)]
mod tests_rug_253 {
    use super::*;
    use crate::App;
    use std::borrow::Cow;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_253_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "H";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let p1: Cow<str> = Cow::Borrowed(rug_fuzz_1);
        p0.help_short(p1);
        let _rug_ed_tests_rug_253_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_254 {
    use super::*;
    use crate::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_254_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "v";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let mut p1: Box<str> = Box::from(rug_fuzz_1);
        p0.version_short(p1);
        let _rug_ed_tests_rug_254_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_255 {
    use super::*;
    use crate::{App, Arg};
    #[test]
    fn test_help_message() {
        let _rug_st_tests_rug_255_rrrruuuugggg_test_help_message = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "Print help information";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: &'static str = rug_fuzz_1;
        p0.help_message(p1);
        let _rug_ed_tests_rug_255_rrrruuuugggg_test_help_message = 0;
    }
}
#[cfg(test)]
mod tests_rug_256 {
    use super::*;
    use crate::{App, Arg, ArgMatches};
    #[test]
    fn test_version_message() {
        let _rug_st_tests_rug_256_rrrruuuugggg_test_version_message = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "Print version information";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let p1: &str = rug_fuzz_1;
        p0.version_message(p1);
        let _rug_ed_tests_rug_256_rrrruuuugggg_test_version_message = 0;
    }
}
#[cfg(test)]
mod tests_rug_257 {
    use super::*;
    use crate::{App, Arg};
    #[test]
    fn test_template() {
        let _rug_st_tests_rug_257_rrrruuuugggg_test_template = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "{bin} ({version}) - {usage}";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let p1: &'static str = rug_fuzz_1;
        p0.template(p1);
        let _rug_ed_tests_rug_257_rrrruuuugggg_test_template = 0;
    }
}
#[cfg(test)]
mod tests_rug_258 {
    use super::*;
    use crate::{App, AppSettings};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_258_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: AppSettings = AppSettings::SubcommandRequired;
        p0.setting(p1);
        let _rug_ed_tests_rug_258_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_259 {
    use super::*;
    use crate::app::App;
    use crate::App as ClapApp;
    use crate::app::settings::AppSettings;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_259_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'_, '_> = ClapApp::new(rug_fuzz_0).into();
        let mut p1: Vec<AppSettings> = Vec::new();
        App::settings(p0, &p1);
        let _rug_ed_tests_rug_259_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_260 {
    use super::*;
    use app::App;
    use crate::App as ClapApp;
    use app::settings::AppSettings;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_260_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let p1: AppSettings = AppSettings::SubcommandRequired;
        App::global_setting(p0, p1);
        let _rug_ed_tests_rug_260_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_261 {
    use super::*;
    use app::App;
    use crate::App as ClapApp;
    use crate::app::settings::AppSettings;
    #[test]
    fn test_global_settings() {
        let _rug_st_tests_rug_261_rrrruuuugggg_test_global_settings = 0;
        let rug_fuzz_0 = "my-app";
        let p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let p1: Vec<AppSettings> = Vec::new();
        p0.global_settings(&p1);
        let _rug_ed_tests_rug_261_rrrruuuugggg_test_global_settings = 0;
    }
}
#[cfg(test)]
mod tests_rug_262 {
    use super::*;
    use crate::App;
    use crate::AppSettings;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_262_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let mut p1: AppSettings = AppSettings::ColorAuto;
        p0.unset_setting(p1);
        let _rug_ed_tests_rug_262_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_263 {
    use super::*;
    use app::App;
    use crate::App as ClapApp;
    use app::settings::AppSettings;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_263_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let mut p1: Vec<AppSettings> = Vec::new();
        p0.unset_settings(&p1);
        let _rug_ed_tests_rug_263_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_264 {
    use super::*;
    use app::App;
    use crate::App as ClapApp;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_264_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = 80;
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let p1: usize = rug_fuzz_1;
        p0.set_term_width(p1);
        let _rug_ed_tests_rug_264_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_265 {
    use super::*;
    use crate::App;
    #[test]
    fn test_max_term_width() {
        let _rug_st_tests_rug_265_rrrruuuugggg_test_max_term_width = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = 100;
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: usize = rug_fuzz_1;
        p0.max_term_width(p1);
        let _rug_ed_tests_rug_265_rrrruuuugggg_test_max_term_width = 0;
    }
}
#[cfg(test)]
mod tests_rug_266 {
    use super::*;
    use crate::{App, Arg};
    #[test]
    fn test_arg() {
        let _rug_st_tests_rug_266_rrrruuuugggg_test_arg = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "debug";
        let rug_fuzz_2 = "d";
        let rug_fuzz_3 = "turns on debugging mode";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let p1: Arg = Arg::with_name(rug_fuzz_1).short(rug_fuzz_2).help(rug_fuzz_3);
        p0.arg(p1);
        let _rug_ed_tests_rug_266_rrrruuuugggg_test_arg = 0;
    }
}
#[cfg(test)]
mod tests_rug_267 {
    use super::*;
    use crate::App;
    use crate::Arg;
    #[test]
    fn test_args() {
        let _rug_st_tests_rug_267_rrrruuuugggg_test_args = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "[debug] -d 'turns on debugging info'";
        let rug_fuzz_2 = "input";
        let rug_fuzz_3 = 1;
        let rug_fuzz_4 = "the input file to use";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let mut p1: Vec<Arg<'static, 'static>> = Vec::new();
        p1.push(Arg::from_usage(rug_fuzz_1));
        p1.push(Arg::with_name(rug_fuzz_2).index(rug_fuzz_3).help(rug_fuzz_4));
        p0.args(&p1);
        let _rug_ed_tests_rug_267_rrrruuuugggg_test_args = 0;
    }
}
#[cfg(test)]
mod tests_rug_268 {
    use super::*;
    use crate::{App, Arg};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_268_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "-c --config=<FILE> 'Sets a configuration file to use'";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: &str = rug_fuzz_1;
        App::<'static, 'static>::arg_from_usage(p0, p1);
        let _rug_ed_tests_rug_268_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_269 {
    use super::*;
    use app::App;
    use crate::App as ClapApp;
    use crate::Arg;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_269_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "-c --config=[FILE] 'Sets a configuration file to use'
                     [debug]... -d 'Sets the debugging level'
                     <FILE> 'The input file to use'";
        let rug_fuzz_1 = "my-app";
        let usage = rug_fuzz_0;
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_1).into();
        let p1: &str = usage;
        p0.args_from_usage(p1);
        let _rug_ed_tests_rug_269_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_270 {
    use super::*;
    use crate::App;
    use crate::Arg;
    use crate::SubCommand;
    #[test]
    fn test_alias() {
        let _rug_st_tests_rug_270_rrrruuuugggg_test_alias = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "do-stuff";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let p1: &str = rug_fuzz_1;
        p0.alias(p1);
        let _rug_ed_tests_rug_270_rrrruuuugggg_test_alias = 0;
    }
}
#[cfg(test)]
mod tests_rug_271 {
    use super::*;
    use crate::{App, Arg, SubCommand};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_271_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = "alias1";
        let rug_fuzz_2 = "alias2";
        let rug_fuzz_3 = "alias3";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: [&'static str; 3] = [rug_fuzz_1, rug_fuzz_2, rug_fuzz_3];
        p0.aliases(&p1);
        let _rug_ed_tests_rug_271_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_273 {
    use super::*;
    use crate::{App, Arg, SubCommand};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_273_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = "alias1";
        let rug_fuzz_2 = "alias2";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let p1: [&'static str; 2] = [rug_fuzz_1, rug_fuzz_2];
        p0.visible_aliases(&p1);
        let _rug_ed_tests_rug_273_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_274 {
    use super::*;
    use app::App;
    use crate::App as ClapApp;
    use args::ArgGroup;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_274_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = "group_name";
        let rug_fuzz_2 = "arg1";
        let rug_fuzz_3 = "arg2";
        let rug_fuzz_4 = "arg3";
        let rug_fuzz_5 = true;
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let mut p1: ArgGroup<'static> = ArgGroup::with_name(rug_fuzz_1)
            .args(&[rug_fuzz_2, rug_fuzz_3, rug_fuzz_4])
            .required(rug_fuzz_5);
        App::group(p0, p1);
        let _rug_ed_tests_rug_274_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_275 {
    use super::*;
    use crate::App;
    use crate::ArgGroup;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_275_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let mut p1: Vec<ArgGroup<'static>> = Vec::new();
        p0.groups(&p1);
        let _rug_ed_tests_rug_275_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_276 {
    use super::*;
    use crate::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_276_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = "1.0";
        let rug_fuzz_2 = "John Doe";
        let rug_fuzz_3 = "sub-command";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0)
            .version(rug_fuzz_1)
            .author(rug_fuzz_2)
            .into();
        let mut p1: App<'static, 'static> = App::new(rug_fuzz_3).into();
        crate::app::App::subcommand(p0, p1);
        let _rug_ed_tests_rug_276_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_277 {
    use super::*;
    use crate::{App, Arg, SubCommand};
    #[test]
    fn test_subcommands() {
        let _rug_st_tests_rug_277_rrrruuuugggg_test_subcommands = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "config";
        let rug_fuzz_2 = "Controls configuration functionality";
        let rug_fuzz_3 = "config_file";
        let rug_fuzz_4 = 1;
        let p0 = App::new(rug_fuzz_0);
        let p1 = vec![
            SubCommand::with_name(rug_fuzz_1).about(rug_fuzz_2)
            .arg(Arg::with_name(rug_fuzz_3).index(rug_fuzz_4),),
            SubCommand::with_name("debug").about("Controls debug functionality")
        ];
        p0.subcommands(p1);
        let _rug_ed_tests_rug_277_rrrruuuugggg_test_subcommands = 0;
    }
}
#[cfg(test)]
mod tests_rug_278 {
    use super::*;
    use crate::{App, SubCommand};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_278_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let rug_fuzz_1 = 1;
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let p1: usize = rug_fuzz_1;
        p0.display_order(p1);
        let _rug_ed_tests_rug_278_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_279 {
    use super::*;
    use std::io;
    use std::io::Write;
    use std::io::BufWriter;
    #[cfg(test)]
    use app::App;
    #[cfg(test)]
    use crate::App as ClapApp;
    #[test]
    fn test_print_help() {
        let _rug_st_tests_rug_279_rrrruuuugggg_test_print_help = 0;
        let rug_fuzz_0 = "myprog";
        let mut p0: App<'_, '_> = ClapApp::new(rug_fuzz_0).into();
        p0.print_help().unwrap();
        let _rug_ed_tests_rug_279_rrrruuuugggg_test_print_help = 0;
    }
}
#[cfg(test)]
mod tests_rug_280 {
    use super::*;
    use crate::App as ClapApp;
    use std::io::{self, Write};
    use std::io::BufWriter;
    #[test]
    fn test_print_long_help() {
        let _rug_st_tests_rug_280_rrrruuuugggg_test_print_long_help = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        p0.print_long_help().unwrap();
        let _rug_ed_tests_rug_280_rrrruuuugggg_test_print_long_help = 0;
    }
}
#[cfg(test)]
mod tests_rug_281 {
    use super::*;
    use std::io::stdout;
    use crate::{App, ErrorKind};
    #[test]
    fn test_write_help() {
        let _rug_st_tests_rug_281_rrrruuuugggg_test_write_help = 0;
        let rug_fuzz_0 = "myprog";
        let mut app: App<'static, 'static> = App::new(rug_fuzz_0);
        let mut out = stdout();
        let result = app.write_help(&mut out);
        debug_assert_eq!(result.unwrap_err().kind, ErrorKind::Io);
        let _rug_ed_tests_rug_281_rrrruuuugggg_test_write_help = 0;
    }
}
#[cfg(test)]
mod tests_rug_282 {
    use super::*;
    use std::io::stdout;
    use app::App;
    use crate::App as ClapApp;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_282_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "failed to write to stdout";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        let mut p1: &mut dyn std::io::Write = &mut stdout();
        App::write_long_help(&mut p0, &mut p1).expect(rug_fuzz_1);
        let _rug_ed_tests_rug_282_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_283 {
    use super::*;
    use std::io::Write;
    use std::io;
    use std::net::TcpStream;
    use crate::App;
    #[test]
    fn test_write_version() {
        let _rug_st_tests_rug_283_rrrruuuugggg_test_write_version = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "127.0.0.1:8080";
        let rug_fuzz_2 = "failed to write to stdout";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let mut p1: &mut TcpStream = &mut TcpStream::connect(rug_fuzz_1).unwrap();
        p0.write_version(p1).expect(rug_fuzz_2);
        let _rug_ed_tests_rug_283_rrrruuuugggg_test_write_version = 0;
    }
}
#[cfg(test)]
mod tests_rug_284 {
    use super::*;
    use crate::App;
    use std::io::{Cursor, Write};
    #[test]
    fn test_write_long_version() {
        let _rug_st_tests_rug_284_rrrruuuugggg_test_write_long_version = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "failed to write to stdout";
        let mut app: App<'static, 'static> = App::new(rug_fuzz_0).into();
        let mut out: Cursor<[u8; 10]> = Cursor::new([rug_fuzz_1; 10]);
        app.write_long_version(&mut out).expect(rug_fuzz_2);
        let _rug_ed_tests_rug_284_rrrruuuugggg_test_write_long_version = 0;
    }
}
#[cfg(test)]
mod tests_rug_287 {
    use super::*;
    use crate::App as ClapApp;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_287_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = ClapApp::new(rug_fuzz_0).into();
        <App<'static, 'static>>::get_matches(p0);
        let _rug_ed_tests_rug_287_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_288 {
    use super::*;
    use crate::{App, ArgMatches};
    #[test]
    fn test_get_matches_safe() {
        let _rug_st_tests_rug_288_rrrruuuugggg_test_get_matches_safe = 0;
        let rug_fuzz_0 = "myprog";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let result: ClapResult<ArgMatches<'static>> = p0.get_matches_safe();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_rug_288_rrrruuuugggg_test_get_matches_safe = 0;
    }
}
#[cfg(test)]
mod tests_rug_289 {
    use super::*;
    use crate::{App, ArgMatches};
    use std::{env, ffi::OsString};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_289_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "my_prog";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let p1: Vec<&str> = vec![rug_fuzz_1, "some", "args", "to", "parse"];
        let itr: Vec<OsString> = p1.into_iter().map(|s| OsString::from(s)).collect();
        p0.get_matches_from(itr);
        let _rug_ed_tests_rug_289_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_290 {
    use super::*;
    use std::path::Path;
    use crate::App;
    use crate::Arg;
    use crate::ArgMatches;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_290_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myprog";
        let rug_fuzz_1 = "path/to/sample/file";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let mut p1: &'static Path = Path::new(rug_fuzz_1);
        let _: ClapResult<ArgMatches> = p0.get_matches_from_safe(p1);
        let _rug_ed_tests_rug_290_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_291 {
    use super::*;
    use app::App;
    use crate::{App as ClapApp, Arg};
    use crate::ArgMatches;
    use crate::Error as ClapError;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_291_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my_prog";
        let rug_fuzz_1 = "myprog";
        let arg_vec = vec![rug_fuzz_0, "some", "args", "to", "parse"];
        let mut app = App::new(rug_fuzz_1);
        let matches: Result<ArgMatches, ClapError> = app
            .get_matches_from_safe_borrow(arg_vec);
        debug_assert!(matches.is_ok());
        let _rug_ed_tests_rug_291_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_292 {
    use super::*;
    use crate::App;
    use std::clone::Clone;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_292_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "my-app";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0).into();
        <App<'static, 'static> as std::clone::Clone>::clone(&p0);
        let _rug_ed_tests_rug_292_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_293 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_293_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg>::name(&p0);
        let _rug_ed_tests_rug_293_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_294 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_294_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg>::overrides(&p0);
        let _rug_ed_tests_rug_294_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_295 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_requires() {
        let _rug_st_tests_rug_295_rrrruuuugggg_test_requires = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0 = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::requires(&p0);
        let _rug_ed_tests_rug_295_rrrruuuugggg_test_requires = 0;
    }
}
#[cfg(test)]
mod tests_rug_296 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_296_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::blacklist(&p0);
        let _rug_ed_tests_rug_296_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_297 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_required_unless() {
        let _rug_st_tests_rug_297_rrrruuuugggg_test_required_unless = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg>::required_unless(&p0);
        let _rug_ed_tests_rug_297_rrrruuuugggg_test_required_unless = 0;
    }
}
#[cfg(test)]
mod tests_rug_298 {
    use super::*;
    use crate::app::App;
    use crate::args::AnyArg;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_298_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::val_names(&p0);
        let _rug_ed_tests_rug_298_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_299 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    use crate::args::settings::ArgSettings;
    use std::str::FromStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_299_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let rug_fuzz_1 = "required";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let mut p1: ArgSettings = ArgSettings::from_str(rug_fuzz_1).unwrap();
        <App<'static, 'static> as AnyArg<'static, 'static>>::is_set(&p0, p1);
        let _rug_ed_tests_rug_299_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_300 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_300_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::val_terminator(&p0);
        let _rug_ed_tests_rug_300_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_301 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    use crate::args::settings::ArgSettings;
    use std::str::FromStr;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_301_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let rug_fuzz_1 = "required";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let mut p1: ArgSettings = ArgSettings::from_str(rug_fuzz_1).unwrap();
        p0.set(p1);
        let _rug_ed_tests_rug_301_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_302 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_302_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::has_switch(&p0);
        let _rug_ed_tests_rug_302_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_303 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_303_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg>::max_vals(&p0);
        let _rug_ed_tests_rug_303_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_304 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_304_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        p0.num_vals();
        let _rug_ed_tests_rug_304_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_305 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_possible_vals() {
        let _rug_st_tests_rug_305_rrrruuuugggg_test_possible_vals = 0;
        let rug_fuzz_0 = "myapp";
        let mut app: App<'_, '_> = App::new(rug_fuzz_0);
        let p0 = &app;
        <App<'_, '_> as AnyArg<'_, '_>>::possible_vals(p0);
        let _rug_ed_tests_rug_305_rrrruuuugggg_test_possible_vals = 0;
    }
}
#[cfg(test)]
mod tests_rug_306 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    use std::rc::Rc;
    use std::result::Result as StdResult;
    #[test]
    fn test_validator() {
        let _rug_st_tests_rug_306_rrrruuuugggg_test_validator = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        p0.validator();
        let _rug_ed_tests_rug_306_rrrruuuugggg_test_validator = 0;
    }
}
#[cfg(test)]
mod tests_rug_307 {
    use super::*;
    use crate::args::AnyArg;
    use std::ffi::{OsStr, OsString};
    use std::result::Result as StdResult;
    use std::rc::Rc;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_307_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::validator_os(&p0);
        let _rug_ed_tests_rug_307_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_308 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_308_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        #[allow(unused_mut)]
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg>::min_vals(&p0);
        let _rug_ed_tests_rug_308_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_309 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_short() {
        let _rug_st_tests_rug_309_rrrruuuugggg_test_short = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        p0.short();
        let _rug_ed_tests_rug_309_rrrruuuugggg_test_short = 0;
    }
}
#[cfg(test)]
mod tests_rug_310 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_310_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg>::long(&p0);
        let _rug_ed_tests_rug_310_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_311 {
    use super::*;
    use crate::args::AnyArg;
    #[test]
    fn test_val_delim() {
        let _rug_st_tests_rug_311_rrrruuuugggg_test_val_delim = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::val_delim(&p0);
        let _rug_ed_tests_rug_311_rrrruuuugggg_test_val_delim = 0;
    }
}
#[cfg(test)]
mod tests_rug_312 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_312_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut v121: App<'static, 'static> = App::new(rug_fuzz_0);
        v121.takes_value();
        let _rug_ed_tests_rug_312_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_314 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_314_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::long_help(&p0);
        let _rug_ed_tests_rug_314_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_315 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_315_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg<'static, 'static>>::default_val(&p0);
        let _rug_ed_tests_rug_315_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_316 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_316_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut v121: App<'static, 'static> = App::new(rug_fuzz_0);
        let p0 = &v121;
        <App<'static, 'static> as AnyArg>::default_vals_ifs(p0);
        let _rug_ed_tests_rug_316_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_317 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    use std::ffi::{OsStr, OsString};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_317_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = "myapp";
        #[cfg(test)]
        mod tests_rug_317_prepare {
            use crate::app::App;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_317_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = "myapp";
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_317_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut v121: App<'static, 'static> = App::new(rug_fuzz_0);
                let _rug_ed_tests_rug_317_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_317_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: App<'static, 'static> = App::new("myapp");
        <App<'static, 'static> as AnyArg<'static, 'static>>::env(&p0);
        let _rug_ed_tests_rug_317_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_318 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_318_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        <App<'static, 'static> as AnyArg>::longest_filter(&p0);
        let _rug_ed_tests_rug_318_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_319 {
    use super::*;
    use crate::args::AnyArg;
    use crate::app::App;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_319_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "myapp";
        let mut p0: App<'static, 'static> = App::new(rug_fuzz_0);
        let result = <App<'static, 'static> as AnyArg<'static, 'static>>::aliases(&p0);
        let _rug_ed_tests_rug_319_rrrruuuugggg_test_rug = 0;
    }
}
