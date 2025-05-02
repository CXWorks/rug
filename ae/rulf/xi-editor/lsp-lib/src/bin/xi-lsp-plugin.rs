extern crate xi_lsp_lib;
#[macro_use]
extern crate serde_json;
extern crate chrono;
extern crate fern;
extern crate log;
use xi_lsp_lib::{start_mainloop, Config, LspPlugin};
fn init_logger() -> Result<(), fern::InitError> {
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
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(
                format_args!(
                    "{}[{}][{}] {}", chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(), record.level(), message
                ),
            )
        })
        .level(level_filter)
        .chain(std::io::stderr())
        .chain(fern::log_file("xi-lsp-plugin.log")?)
        .apply()
        .map_err(|e| e.into())
}
fn main() {
    let config = json!(
        { "language_config" : { "rust" : { "language_name" : "Rust", "start_command" :
        "rls", "start_arguments" : [], "extensions" : ["rs"], "supports_single_file" :
        false, "workspace_identifier" : "Cargo.toml" }, "json" : { "language_name" :
        "Json", "start_command" : "vscode-json-languageserver", "start_arguments" :
        ["--stdio"], "extensions" : ["json", "jsonc"], "supports_single_file" : true, },
        "typescript" : { "language_name" : "Typescript", "start_command" :
        "javascript-typescript-stdio", "start_arguments" : [], "extensions" : ["ts",
        "js", "jsx", "tsx"], "supports_single_file" : true, "workspace_identifier" :
        "package.json" } } }
    );
    init_logger().expect("Failed to start logger for LSP Plugin");
    let config: Config = serde_json::from_value(config).unwrap();
    let mut plugin = LspPlugin::new(config);
    start_mainloop(&mut plugin);
}
#[cfg(test)]
mod tests_llm_16_1 {
    use super::*;
    use crate::*;
    #[test]
    fn test_init_logger() {
        let _rug_st_tests_llm_16_1_rrrruuuugggg_test_init_logger = 0;
        let result = init_logger();
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_1_rrrruuuugggg_test_init_logger = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_2 {
    use serde_json::json;
    use super::*;
    use crate::*;
    #[test]
    fn test_main() {
        let _rug_st_tests_llm_16_2_rrrruuuugggg_test_main = 0;
        main();
        let _rug_ed_tests_llm_16_2_rrrruuuugggg_test_main = 0;
    }
}
