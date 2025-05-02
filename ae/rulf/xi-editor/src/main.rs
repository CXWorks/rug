#[macro_use]
extern crate log;
extern crate chrono;
extern crate fern;
extern crate dirs;
extern crate xi_core_lib;
extern crate xi_rpc;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use xi_core_lib::XiCore;
use xi_rpc::RpcLoop;
const XI_LOG_DIR: &str = "xi-core";
const XI_LOG_FILE: &str = "xi-core.log";
fn get_logging_directory_path<P: AsRef<Path>>(
    directory: P,
) -> Result<PathBuf, io::Error> {
    match dirs::data_local_dir() {
        Some(mut log_dir) => {
            log_dir.push(directory);
            Ok(log_dir)
        }
        None => {
            Err(
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "No standard logging directory known for this platform",
                ),
            )
        }
    }
}
/// This function tries to create the parent directories for a file
///
/// It wraps around the `parent()` function of `Path` which returns an `Option<&Path>` and
/// `fs::create_dir_all` which returns an `io::Result<()>`.
///
/// This allows you to use `?`/`try!()` to create the dir and you recive the additional custom error for when `parent()`
/// returns nothing.
///
/// # Errors
/// This can return an `io::Error` if `fs::create_dir_all` fails or if `parent()` returns `None`.
/// See `Path`'s `parent()` function for more details.
/// # Examples
/// ```
/// use std::path::Path;
/// use std::ffi::OsStr;
///
/// let path_with_file = Path::new("/some/directory/then/file");
/// assert_eq!(Some(OsStr::new("file")), path_with_file.file_name());
/// assert_eq!(create_log_directory(path_with_file).is_ok(), true);
///
/// let path_with_other_file = Path::new("/other_file");
/// assert_eq!(Some(OsStr::new("other_file")), path_with_other_file.file_name());
/// assert_eq!(create_log_directory(path_with_file).is_ok(), true);
///
/// // Path that is just the root or prefix:
/// let path_without_file = Path::new("/");
/// assert_eq!(None, path_without_file.file_name());
/// assert_eq!(create_log_directory(path_without_file).is_ok(), false);
/// ```
fn create_log_directory(path_with_file: &Path) -> io::Result<()> {
    let log_dir = path_with_file
        .parent()
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Unable to get the parent of the following Path: {}, Your path should contain a file name",
                path_with_file.display(),
            ),
        ))?;
    fs::create_dir_all(log_dir)?;
    Ok(())
}
fn setup_logging(logging_path: Option<&Path>) -> Result<(), fern::InitError> {
    let level_filter = match std::env::var("XI_LOG") {
        Ok(level) => {
            match level.to_lowercase().as_ref() {
                "trace" => log::LevelFilter::Trace,
                "debug" => log::LevelFilter::Debug,
                _ => log::LevelFilter::Info,
            }
        }
        Err(_) => log::LevelFilter::Info,
    };
    let mut fern_dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(
                format_args!(
                    "{}[{}][{}] {}", chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(), record.level(), message,
                ),
            )
        })
        .level(level_filter)
        .chain(io::stderr());
    if let Some(logging_file_path) = logging_path {
        create_log_directory(logging_file_path)?;
        fern_dispatch = fern_dispatch.chain(fern::log_file(logging_file_path)?);
    }
    fern_dispatch.apply()?;
    info!("Logging with fern is set up");
    match logging_path {
        Some(logging_file_path) => {
            info!("Writing logs to: {}", logging_file_path.display())
        }
        None => {
            warn!(
                "No path was supplied for the log file. Not saving logs to disk, falling back to just stderr"
            )
        }
    }
    Ok(())
}
fn generate_logging_path(logfile_config: LogfileConfig) -> Result<PathBuf, io::Error> {
    let logfile_file_name = match logfile_config.file {
        Some(file_name) => file_name,
        None => PathBuf::from(XI_LOG_FILE),
    };
    if logfile_file_name.eq(Path::new("")) {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "A blank file name was supplied"),
        );
    }
    let logfile_directory_name = match logfile_config.directory {
        Some(dir) => dir,
        None => PathBuf::from(XI_LOG_DIR),
    };
    let mut logging_directory_path = get_logging_directory_path(logfile_directory_name)?;
    logging_directory_path.push(logfile_file_name);
    Ok(logging_directory_path)
}
fn get_flags() -> HashMap<String, Option<String>> {
    let mut flags: HashMap<String, Option<String>> = HashMap::new();
    let flag_prefix = "-";
    let mut args_iterator = std::env::args().peekable();
    while let Some(arg) = args_iterator.next() {
        if arg.starts_with(flag_prefix) {
            let key = arg.trim_start_matches(flag_prefix).to_string();
            let next_arg_not_a_flag: bool = args_iterator
                .peek()
                .map_or(false, |val| !val.starts_with(flag_prefix));
            if next_arg_not_a_flag {
                flags.insert(key, args_iterator.next());
            }
        }
    }
    flags
}
struct EnvFlagConfig {
    env_name: &'static str,
    flag_name: &'static str,
}
/// Extracts a value from the flags and the env.
///
/// In this order: `String` from the flags, then `String` from the env, then `None`
fn extract_env_or_flag(
    flags: &HashMap<String, Option<String>>,
    conf: &EnvFlagConfig,
) -> Option<String> {
    flags
        .get(conf.flag_name)
        .cloned()
        .unwrap_or_else(|| std::env::var(conf.env_name).ok())
}
struct LogfileConfig {
    directory: Option<PathBuf>,
    file: Option<PathBuf>,
}
fn generate_logfile_config(flags: &HashMap<String, Option<String>>) -> LogfileConfig {
    let log_dir_env_flag = EnvFlagConfig {
        env_name: "XI_LOG_DIR",
        flag_name: "log-dir",
    };
    let log_file_env_flag = EnvFlagConfig {
        env_name: "XI_LOG_FILE",
        flag_name: "log-file",
    };
    let log_dir_flag_option = extract_env_or_flag(&flags, &log_dir_env_flag)
        .map(PathBuf::from);
    let log_file_flag_option = extract_env_or_flag(&flags, &log_file_env_flag)
        .map(PathBuf::from);
    LogfileConfig {
        directory: log_dir_flag_option,
        file: log_file_flag_option,
    }
}
fn main() {
    let mut state = XiCore::new();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut rpc_looper = RpcLoop::new(stdout);
    let flags = get_flags();
    let logfile_config = generate_logfile_config(&flags);
    let logging_path_result = generate_logging_path(logfile_config);
    let logging_path = logging_path_result
        .as_ref()
        .map(|p: &PathBuf| -> &Path { p.as_path() })
        .ok();
    if let Err(e) = setup_logging(logging_path) {
        eprintln!("[ERROR] setup_logging returned error, logging not enabled: {:?}", e);
    }
    if let Err(e) = logging_path_result.as_ref() {
        warn!("Unable to generate the logging path to pass to set up: {}", e)
    }
    match rpc_looper.mainloop(|| stdin.lock(), &mut state) {
        Ok(_) => {}
        Err(err) => {
            error!("xi-core exited with error:\n{:?}", err);
            process::exit(1);
        }
    }
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;
    use crate::*;
    use std::path::Path;
    use std::ffi::OsStr;
    #[test]
    fn test_create_log_directory() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_create_log_directory = 0;
        let rug_fuzz_0 = "/some/directory/then/file";
        let rug_fuzz_1 = "file";
        let rug_fuzz_2 = "/other_file";
        let rug_fuzz_3 = "other_file";
        let rug_fuzz_4 = "/";
        let path_with_file = Path::new(rug_fuzz_0);
        debug_assert_eq!(Some(OsStr::new(rug_fuzz_1)), path_with_file.file_name());
        debug_assert_eq!(create_log_directory(path_with_file).is_ok(), true);
        let path_with_other_file = Path::new(rug_fuzz_2);
        debug_assert_eq!(Some(OsStr::new(rug_fuzz_3)), path_with_other_file.file_name());
        debug_assert_eq!(create_log_directory(path_with_file).is_ok(), true);
        let path_without_file = Path::new(rug_fuzz_4);
        debug_assert_eq!(None, path_without_file.file_name());
        debug_assert_eq!(create_log_directory(path_without_file).is_ok(), false);
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_create_log_directory = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_2 {
    use std::collections::HashMap;
    use super::*;
    use crate::*;
    #[test]
    fn test_extract_env_or_flag_with_flag_value() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_extract_env_or_flag_with_flag_value = 0;
        let rug_fuzz_0 = "flag";
        let rug_fuzz_1 = "value";
        let rug_fuzz_2 = "ENV_NAME";
        let rug_fuzz_3 = "flag";
        let mut flags = HashMap::new();
        flags.insert(rug_fuzz_0.to_string(), Some(rug_fuzz_1.to_string()));
        let conf = EnvFlagConfig {
            env_name: rug_fuzz_2,
            flag_name: rug_fuzz_3,
        };
        let result = extract_env_or_flag(&flags, &conf);
        debug_assert_eq!(result, Some("value".to_string()));
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_extract_env_or_flag_with_flag_value = 0;
    }
    #[test]
    fn test_extract_env_or_flag_with_env_value() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_extract_env_or_flag_with_env_value = 0;
        let rug_fuzz_0 = "ENV_NAME";
        let rug_fuzz_1 = "flag";
        let rug_fuzz_2 = "ENV_NAME";
        let rug_fuzz_3 = "value";
        let mut flags = HashMap::new();
        let conf = EnvFlagConfig {
            env_name: rug_fuzz_0,
            flag_name: rug_fuzz_1,
        };
        std::env::set_var(rug_fuzz_2, rug_fuzz_3);
        let result = extract_env_or_flag(&flags, &conf);
        debug_assert_eq!(result, Some("value".to_string()));
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_extract_env_or_flag_with_env_value = 0;
    }
    #[test]
    fn test_extract_env_or_flag_with_empty_values() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_extract_env_or_flag_with_empty_values = 0;
        let rug_fuzz_0 = "ENV_NAME";
        let rug_fuzz_1 = "flag";
        let rug_fuzz_2 = "ENV_NAME";
        let flags = HashMap::new();
        let conf = EnvFlagConfig {
            env_name: rug_fuzz_0,
            flag_name: rug_fuzz_1,
        };
        std::env::remove_var(rug_fuzz_2);
        let result = extract_env_or_flag(&flags, &conf);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_extract_env_or_flag_with_empty_values = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_4_llm_16_3 {
    use super::*;
    use crate::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    #[derive(Debug, PartialEq)]
    struct LogfileConfig {
        directory: Option<PathBuf>,
        file: Option<PathBuf>,
    }
    struct EnvFlagConfig {
        env_name: &'static str,
        flag_name: &'static str,
    }
    fn extract_env_or_flag<'a>(
        flags: &'a HashMap<String, Option<String>>,
        env_flag: &'a EnvFlagConfig,
    ) -> Option<String> {
        todo!()
    }
    fn generate_logfile_config(
        flags: &HashMap<String, Option<String>>,
    ) -> LogfileConfig {
        todo!()
    }
    #[test]
    fn test_generate_logfile_config() {
        let mut flags = HashMap::new();
        flags.insert(String::from("log-dir"), Some(String::from("/path/to/logs")));
        flags.insert(String::from("log-file"), Some(String::from("app.log")));
        let expected = LogfileConfig {
            directory: Some(PathBuf::from("/path/to/logs")),
            file: Some(PathBuf::from("app.log")),
        };
        let result = generate_logfile_config(&flags);
        assert_eq!(result, expected);
    }
    #[test]
    fn test_generate_logfile_config_without_flags() {
        let flags = HashMap::new();
        let expected = LogfileConfig {
            directory: None,
            file: None,
        };
        let result = generate_logfile_config(&flags);
        assert_eq!(result, expected);
    }
}
#[cfg(test)]
mod tests_llm_16_7 {
    use super::*;
    use crate::*;
    use std::collections::HashMap;
    #[test]
    fn test_get_flags() {
        let _rug_st_tests_llm_16_7_rrrruuuugggg_test_get_flags = 0;
        let rug_fuzz_0 = "a";
        let rug_fuzz_1 = "value_a";
        let rug_fuzz_2 = "b";
        let rug_fuzz_3 = "value_b";
        let rug_fuzz_4 = "c";
        let rug_fuzz_5 = "d";
        let rug_fuzz_6 = "value_d";
        let mut expected_flags: HashMap<String, Option<String>> = HashMap::new();
        expected_flags.insert(rug_fuzz_0.to_string(), Some(rug_fuzz_1.to_string()));
        expected_flags.insert(rug_fuzz_2.to_string(), Some(rug_fuzz_3.to_string()));
        expected_flags.insert(rug_fuzz_4.to_string(), None);
        expected_flags.insert(rug_fuzz_5.to_string(), Some(rug_fuzz_6.to_string()));
        debug_assert_eq!(get_flags(), expected_flags);
        let _rug_ed_tests_llm_16_7_rrrruuuugggg_test_get_flags = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_9_llm_16_8 {
    use std::io;
    use std::path::{Path, PathBuf};
    use dirs;
    use crate::get_logging_directory_path;
    #[test]
    fn test_get_logging_directory_path_success() {
        let _rug_st_tests_llm_16_9_llm_16_8_rrrruuuugggg_test_get_logging_directory_path_success = 0;
        let rug_fuzz_0 = "logs";
        let rug_fuzz_1 = "logs";
        let result = get_logging_directory_path(rug_fuzz_0);
        debug_assert!(result.is_ok());
        let logging_directory_path = result.unwrap();
        let expected_path: PathBuf = [dirs::data_local_dir().unwrap(), rug_fuzz_1.into()]
            .iter()
            .collect();
        debug_assert_eq!(logging_directory_path, expected_path);
        let _rug_ed_tests_llm_16_9_llm_16_8_rrrruuuugggg_test_get_logging_directory_path_success = 0;
    }
    #[test]
    fn test_get_logging_directory_path_failure() {
        let _rug_st_tests_llm_16_9_llm_16_8_rrrruuuugggg_test_get_logging_directory_path_failure = 0;
        let rug_fuzz_0 = "logs";
        let rug_fuzz_1 = "No standard logging directory known for this platform";
        let result = get_logging_directory_path(rug_fuzz_0);
        debug_assert!(result.is_err());
        let expected_error = io::Error::new(io::ErrorKind::NotFound, rug_fuzz_1);
        debug_assert_eq!(result.unwrap_err().kind(), expected_error.kind());
        let _rug_ed_tests_llm_16_9_llm_16_8_rrrruuuugggg_test_get_logging_directory_path_failure = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_12 {
    use super::*;
    use crate::*;
    use std::io;
    use std::path::Path;
    use fern;
    use log::*;
    #[test]
    fn test_setup_logging() {
        let _rug_st_tests_llm_16_12_rrrruuuugggg_test_setup_logging = 0;
        let rug_fuzz_0 = "path/to/logfile.log";
        let rug_fuzz_1 = "XI_LOG";
        let rug_fuzz_2 = "invalid_level";
        let rug_fuzz_3 = "XI_LOG";
        let rug_fuzz_4 = "trace";
        let rug_fuzz_5 = "XI_LOG";
        let rug_fuzz_6 = "debug";
        debug_assert!(setup_logging(None).is_ok());
        debug_assert!(setup_logging(Some(Path::new(rug_fuzz_0))).is_ok());
        std::env::set_var(rug_fuzz_1, rug_fuzz_2);
        debug_assert!(setup_logging(None).is_ok());
        std::env::set_var(rug_fuzz_3, rug_fuzz_4);
        debug_assert!(setup_logging(None).is_ok());
        std::env::set_var(rug_fuzz_5, rug_fuzz_6);
        debug_assert!(setup_logging(None).is_ok());
        let _rug_ed_tests_llm_16_12_rrrruuuugggg_test_setup_logging = 0;
    }
}
