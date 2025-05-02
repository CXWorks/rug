//! Structured representation of a plugin's features and capabilities.
use std::path::PathBuf;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{self, Value};
use crate::syntax::{LanguageDefinition, LanguageId};
/// Describes attributes and capabilities of a plugin.
///
/// Note: - these will eventually be loaded from manifest files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PluginDescription {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub scope: PluginScope,
    /// path to plugin executable
    #[serde(deserialize_with = "platform_exec_path")]
    pub exec_path: PathBuf,
    /// Events that cause this plugin to run
    #[serde(default)]
    pub activations: Vec<PluginActivation>,
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub languages: Vec<LanguageDefinition>,
}
fn platform_exec_path<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<PathBuf, D::Error> {
    let exec_path = PathBuf::deserialize(deserializer)?;
    if cfg!(windows) { Ok(exec_path.with_extension("exe")) } else { Ok(exec_path) }
}
/// `PluginActivation`s represent events that trigger running a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginActivation {
    /// Always run this plugin, when available.
    Autorun,
    /// Run this plugin if the provided SyntaxDefinition is active.
    #[allow(dead_code)]
    OnSyntax(LanguageId),
    /// Run this plugin in response to a given command.
    #[allow(dead_code)]
    OnCommand,
}
/// Describes the scope of events a plugin receives.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginScope {
    /// The plugin receives events from multiple buffers.
    Global,
    /// The plugin receives events for a single buffer.
    BufferLocal,
    /// The plugin is launched in response to a command, and receives no
    /// further updates.
    SingleInvocation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents a custom command provided by a plugin.
pub struct Command {
    /// Human readable title, for display in (for example) a menu.
    pub title: String,
    /// A short description of the command.
    pub description: String,
    /// Template of the command RPC as it should be sent to the plugin.
    pub rpc_cmd: PlaceholderRpc,
    /// A list of `CommandArgument`s, which the client should use to build the RPC.
    pub args: Vec<CommandArgument>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// A user provided argument to a plugin command.
pub struct CommandArgument {
    /// A human readable name for this argument, for use as placeholder
    /// text or equivelant.
    pub title: String,
    /// A short (single sentence) description of this argument's use.
    pub description: String,
    pub key: String,
    pub arg_type: ArgumentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// If `arg_type` is `Choice`, `options` must contain a list of options.
    pub options: Option<Vec<ArgumentOption>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArgumentType {
    Number,
    Int,
    PosInt,
    Bool,
    String,
    Choice,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Represents an option for a user-selectable argument.
pub struct ArgumentOption {
    pub title: String,
    pub value: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
/// A placeholder type which can represent a generic RPC.
///
/// This is the type used for custom plugin commands, which may have arbitrary
/// method names and parameters.
pub struct PlaceholderRpc {
    pub method: String,
    pub params: Value,
    pub rpc_type: RpcType,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RpcType {
    Notification,
    Request,
}
impl Command {
    pub fn new<S, V>(title: S, description: S, rpc_cmd: PlaceholderRpc, args: V) -> Self
    where
        S: AsRef<str>,
        V: Into<Option<Vec<CommandArgument>>>,
    {
        let title = title.as_ref().to_owned();
        let description = description.as_ref().to_owned();
        let args = args.into().unwrap_or_else(Vec::new);
        Command {
            title,
            description,
            rpc_cmd,
            args,
        }
    }
}
impl CommandArgument {
    pub fn new<S: AsRef<str>>(
        title: S,
        description: S,
        key: S,
        arg_type: ArgumentType,
        options: Option<Vec<ArgumentOption>>,
    ) -> Self {
        let key = key.as_ref().to_owned();
        let title = title.as_ref().to_owned();
        let description = description.as_ref().to_owned();
        if arg_type == ArgumentType::Choice {
            assert!(options.is_some())
        }
        CommandArgument {
            title,
            description,
            key,
            arg_type,
            options,
        }
    }
}
impl ArgumentOption {
    pub fn new<S: AsRef<str>, V: Serialize>(title: S, value: V) -> Self {
        let title = title.as_ref().to_owned();
        let value = serde_json::to_value(value).unwrap();
        ArgumentOption { title, value }
    }
}
impl PlaceholderRpc {
    pub fn new<S, V>(method: S, params: V, request: bool) -> Self
    where
        S: AsRef<str>,
        V: Into<Option<Value>>,
    {
        let method = method.as_ref().to_owned();
        let params = params.into().unwrap_or(json!({}));
        let rpc_type = if request { RpcType::Request } else { RpcType::Notification };
        PlaceholderRpc {
            method,
            params,
            rpc_type,
        }
    }
    pub fn is_request(&self) -> bool {
        self.rpc_type == RpcType::Request
    }
    /// Returns a reference to the placeholder's params.
    pub fn params_ref(&self) -> &Value {
        &self.params
    }
    /// Returns a mutable reference to the placeholder's params.
    pub fn params_ref_mut(&mut self) -> &mut Value {
        &mut self.params
    }
    /// Returns a reference to the placeholder's method.
    pub fn method_ref(&self) -> &str {
        &self.method
    }
}
impl PluginDescription {
    /// Returns `true` if this plugin is globally scoped, else `false`.
    pub fn is_global(&self) -> bool {
        matches!(self.scope, PluginScope::Global)
    }
}
impl Default for PluginScope {
    fn default() -> Self {
        PluginScope::BufferLocal
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    #[test]
    fn platform_exec_path() {
        let json = r#"
        {
            "name": "test_plugin",
            "version": "0.0.0",
            "scope": "global",
            "exec_path": "path/to/binary",
            "activations": [],
            "commands": [],
            "languages": []
        }
        "#;
        let plugin_desc: PluginDescription = serde_json::from_str(&json).unwrap();
        if cfg!(windows) {
            assert!(plugin_desc.exec_path.ends_with("binary.exe"));
        } else {
            assert!(plugin_desc.exec_path.ends_with("binary"));
        }
    }
    #[test]
    fn test_serde_command() {
        let json = r#"
    {
        "title": "Test Command",
        "description": "Passes the current test",
        "rpc_cmd": {
            "rpc_type": "notification",
            "method": "test.cmd",
            "params": {
                "view": "",
                "non_arg": "plugin supplied value",
                "arg_one": "",
                "arg_two": ""
            }
        },
        "args": [
            {
                "title": "First argument",
                "description": "Indicates something",
                "key": "arg_one",
                "arg_type": "Bool"
            },
            {
                "title": "Favourite Number",
                "description": "A number used in a test.",
                "key": "arg_two",
                "arg_type": "Choice",
                "options": [
                    {"title": "Five", "value": 5},
                    {"title": "Ten", "value": 10}
                ]
            }
        ]
    }
        "#;
        let command: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(command.title, "Test Command");
        assert_eq!(command.args[0].arg_type, ArgumentType::Bool);
        assert_eq!(command.rpc_cmd.params_ref() ["non_arg"], "plugin supplied value");
        assert_eq!(command.args[1].options.clone().unwrap() [1].value, json!(10));
    }
}
#[cfg(test)]
mod tests_llm_16_630 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_630_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "Test Title";
        let rug_fuzz_1 = 10;
        let title: &str = rug_fuzz_0;
        let value: u8 = rug_fuzz_1;
        let argument_option = ArgumentOption::new(title, value);
        debug_assert_eq!(argument_option.title, "Test Title");
        debug_assert_eq!(argument_option.value, serde_json::to_value(10).unwrap());
        let _rug_ed_tests_llm_16_630_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_635 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_request_true() {
        let _rug_st_tests_llm_16_635_rrrruuuugggg_test_is_request_true = 0;
        let rug_fuzz_0 = "test_method";
        let rpc = PlaceholderRpc {
            method: rug_fuzz_0.to_owned(),
            params: serde_json::json!({}),
            rpc_type: RpcType::Request,
        };
        debug_assert_eq!(rpc.is_request(), true);
        let _rug_ed_tests_llm_16_635_rrrruuuugggg_test_is_request_true = 0;
    }
    #[test]
    fn test_is_request_false() {
        let _rug_st_tests_llm_16_635_rrrruuuugggg_test_is_request_false = 0;
        let rug_fuzz_0 = "test_method";
        let rpc = PlaceholderRpc {
            method: rug_fuzz_0.to_owned(),
            params: serde_json::json!({}),
            rpc_type: RpcType::Notification,
        };
        debug_assert_eq!(rpc.is_request(), false);
        let _rug_ed_tests_llm_16_635_rrrruuuugggg_test_is_request_false = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_636 {
    use super::*;
    use crate::*;
    use serde_json::json;
    #[test]
    fn test_method_ref() {
        let _rug_st_tests_llm_16_636_rrrruuuugggg_test_method_ref = 0;
        let rug_fuzz_0 = "test_method";
        let rug_fuzz_1 = true;
        let rpc = PlaceholderRpc::new(
            rug_fuzz_0,
            json!({ "param" : "value" }),
            rug_fuzz_1,
        );
        debug_assert_eq!(rpc.method_ref(), "test_method");
        let _rug_ed_tests_llm_16_636_rrrruuuugggg_test_method_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_638 {
    use super::*;
    use crate::*;
    use serde_json::json;
    #[test]
    fn test_new_placeholder_rpc() {
        let _rug_st_tests_llm_16_638_rrrruuuugggg_test_new_placeholder_rpc = 0;
        let rug_fuzz_0 = "test_method";
        let rug_fuzz_1 = true;
        let method = rug_fuzz_0;
        let params = json!({ "param1" : "value1" });
        let request = rug_fuzz_1;
        let rpc = PlaceholderRpc::new(method, params.clone(), request);
        debug_assert_eq!(rpc.method, method.to_owned());
        debug_assert_eq!(rpc.params, params);
        debug_assert_eq!(rpc.rpc_type, RpcType::Request);
        let _rug_ed_tests_llm_16_638_rrrruuuugggg_test_new_placeholder_rpc = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_639 {
    use super::*;
    use crate::*;
    use serde_json::json;
    #[test]
    fn test_params_ref() {
        let _rug_st_tests_llm_16_639_rrrruuuugggg_test_params_ref = 0;
        let rug_fuzz_0 = "method";
        let rug_fuzz_1 = true;
        let params = json!({ "key" : "value" });
        let rpc = PlaceholderRpc::new(rug_fuzz_0, params, rug_fuzz_1);
        debug_assert_eq!(rpc.params_ref(), & json!({ "key" : "value" }));
        let _rug_ed_tests_llm_16_639_rrrruuuugggg_test_params_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_640 {
    use super::*;
    use crate::*;
    use serde_json::json;
    #[test]
    fn test_params_ref_mut() {
        let _rug_st_tests_llm_16_640_rrrruuuugggg_test_params_ref_mut = 0;
        let rug_fuzz_0 = "method";
        let rug_fuzz_1 = true;
        let mut placeholder = PlaceholderRpc::new(
            rug_fuzz_0,
            json!({ "param1" : "value1" }),
            rug_fuzz_1,
        );
        let params_ref_mut = placeholder.params_ref_mut();
        debug_assert_eq!(params_ref_mut, & mut json!({ "param1" : "value1" }));
        let _rug_ed_tests_llm_16_640_rrrruuuugggg_test_params_ref_mut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_641 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_global() {
        let _rug_st_tests_llm_16_641_rrrruuuugggg_test_is_global = 0;
        let rug_fuzz_0 = "test-plugin";
        let rug_fuzz_1 = "1.0.0";
        let rug_fuzz_2 = "/path/to/executable";
        let plugin = PluginDescription {
            name: String::from(rug_fuzz_0),
            version: String::from(rug_fuzz_1),
            scope: PluginScope::Global,
            exec_path: PathBuf::from(rug_fuzz_2),
            activations: Vec::new(),
            commands: Vec::new(),
            languages: Vec::new(),
        };
        debug_assert!(plugin.is_global());
        let _rug_ed_tests_llm_16_641_rrrruuuugggg_test_is_global = 0;
    }
}
