use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(Debug, Clone)]
pub struct GwsBackend {
    pub config_dir: PathBuf,
    pub timeout: Duration,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub login_required: bool,
}

impl GwsBackend {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            timeout: Duration::from_secs(20),
        }
    }

    pub fn call_tool(&self, name: &str, args: Value) -> Value {
        match self.call_tool_result(name, args) {
            Ok(value) => json!({"ok": true, "data": value}),
            Err(err) => json!({"ok": false, "error": err}),
        }
    }

    fn call_tool_result(&self, name: &str, args: Value) -> std::result::Result<Value, ToolError> {
        if !self.is_authenticated() {
            return Err(login_required_for(&self.config_dir));
        }
        let argv = tool_invocation(name, &args)?;
        let config_dir = self.config_dir.clone();
        self.run_gws(&argv)
            .map_err(|err| normalize_backend_error(err, &config_dir))
    }

    pub fn is_authenticated(&self) -> bool {
        if which::which(backend_command_name()).is_err() {
            return false;
        }
        backend_auth_state(&self.config_dir.join("auth")).authenticated()
    }

    fn run_gws(&self, args: &[String]) -> Result<Value> {
        let mut child = Command::new(backend_command_name())
            .args(args)
            .env(
                "GOOGLE_WORKSPACE_CLI_CONFIG_DIR",
                self.config_dir.join("auth"),
            )
            .env("GWS_CONFIG_DIR", self.config_dir.join("auth"))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("running gws backend")?;
        let status = match child.wait_timeout(self.timeout)? {
            Some(status) => status,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("backend_timeout");
            }
        };
        let output = child.wait_with_output()?;
        if !status.success() {
            anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
        }
        parse_output(&output.stdout)
    }
}

fn arg_string(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn arg_limit(args: &Value) -> u64 {
    args.get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(10)
        .min(50)
}

/// Map an MCP tool name + args onto a verified `gws` argv invocation of the form
/// `gws <service> <resource> [sub-resource] <method> --params <JSON>`.
///
/// Grammar verified against the pinned gws version (see the unit tests).
/// Optional parameters that arrive empty are omitted rather than sent as "",
/// which the Google APIs reject.
pub fn tool_invocation(name: &str, args: &Value) -> std::result::Result<Vec<String>, ToolError> {
    let mut params = serde_json::Map::new();
    let path: Vec<&str> = match name {
        "gmail_search" => {
            params.insert("userId".into(), json!(user_id(args)));
            params.insert("q".into(), json!(arg_string(args, "query")));
            params.insert("maxResults".into(), json!(arg_limit(args)));
            vec!["gmail", "users", "messages", "list"]
        }
        "gmail_read" => {
            params.insert("userId".into(), json!(user_id(args)));
            params.insert("id".into(), json!(arg_string(args, "id")));
            params.insert("format".into(), json!("full"));
            vec!["gmail", "users", "messages", "get"]
        }
        "calendar_list" => {
            params.insert("calendarId".into(), json!(calendar_id(args)));
            insert_if_non_empty(&mut params, "timeMin", arg_string(args, "time_min"));
            insert_if_non_empty(&mut params, "timeMax", arg_string(args, "time_max"));
            params.insert("maxResults".into(), json!(arg_limit(args)));
            vec!["calendar", "events", "list"]
        }
        "calendar_read" => {
            params.insert("calendarId".into(), json!(calendar_id(args)));
            // The Calendar API names this parameter eventId, not id.
            params.insert("eventId".into(), json!(arg_string(args, "id")));
            vec!["calendar", "events", "get"]
        }
        "drive_search" => {
            insert_if_non_empty(&mut params, "q", arg_string(args, "query"));
            params.insert("pageSize".into(), json!(arg_limit(args)));
            vec!["drive", "files", "list"]
        }
        "drive_read" => {
            params.insert("fileId".into(), json!(arg_string(args, "id")));
            vec!["drive", "files", "get"]
        }
        _ => {
            return Err(ToolError {
                code: "unknown_tool".into(),
                message: format!("unknown tool: {name}"),
                login_required: false,
            })
        }
    };
    let mut argv: Vec<String> = path.iter().map(|s| (*s).to_string()).collect();
    argv.push("--params".into());
    argv.push(Value::Object(params).to_string());
    Ok(argv)
}

/// Backend methods referenced by the tool mapping, used by release packaging to
/// fail early when a pinned gws version no longer provides them.
pub fn mapped_methods() -> Vec<&'static str> {
    vec![
        "gmail.users.messages.list",
        "gmail.users.messages.get",
        "calendar.events.list",
        "calendar.events.get",
        "drive.files.list",
        "drive.files.get",
    ]
}

/// Authentication state as reported by the delegated backend.
///
/// `gws auth status` exits 0 even when nothing is authenticated, so state MUST
/// be read from its JSON payload rather than the process exit code.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BackendAuthState {
    pub auth_method: String,
    pub credential_source: String,
    pub client_config_exists: bool,
    pub token_cache_exists: bool,
}

impl BackendAuthState {
    /// True only when the backend holds usable credentials for API calls.
    pub fn authenticated(&self) -> bool {
        !self.auth_method.is_empty() && self.auth_method != "none"
    }

    /// True when OAuth client credentials are discoverable by the backend.
    pub fn client_configured(&self) -> bool {
        self.client_config_exists
            || !self.credential_source.is_empty() && self.credential_source != "none"
    }
}

pub fn backend_auth_state(auth_dir: &PathBuf) -> BackendAuthState {
    let output = Command::new(backend_command_name())
        .arg("auth")
        .arg("status")
        .env("GOOGLE_WORKSPACE_CLI_CONFIG_DIR", auth_dir)
        .env("GWS_CONFIG_DIR", auth_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output();
    let Ok(output) = output else {
        return BackendAuthState::default();
    };
    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        return BackendAuthState::default();
    };
    BackendAuthState {
        auth_method: value
            .get("auth_method")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        credential_source: value
            .get("credential_source")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        client_config_exists: value
            .get("client_config_exists")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        token_cache_exists: value
            .get("token_cache_exists")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn user_id(args: &Value) -> String {
    let id = arg_string(args, "user");
    if id.is_empty() {
        "me".to_string()
    } else {
        id
    }
}

fn calendar_id(args: &Value) -> String {
    let id = arg_string(args, "calendar_id");
    if id.is_empty() {
        "primary".to_string()
    } else {
        id
    }
}

fn insert_if_non_empty(map: &mut serde_json::Map<String, Value>, key: &str, value: String) {
    if !value.trim().is_empty() {
        map.insert(key.to_string(), json!(value));
    }
}

pub fn backend_command_name() -> std::ffi::OsString {
    std::env::var_os("PICCOLO_GWS_BACKEND_BIN").unwrap_or_else(|| std::ffi::OsString::from("gws"))
}

pub fn parse_output(stdout: &[u8]) -> Result<Value> {
    let text = String::from_utf8_lossy(stdout).trim().to_string();
    if text.is_empty() {
        return Ok(json!({"items": []}));
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => Ok(normalize_value(value)),
        Err(_) => Ok(json!({"text": text})),
    }
}

pub fn normalize_value(value: Value) -> Value {
    match value {
        Value::Array(items) => json!({"items": items}),
        Value::Object(mut map) => {
            map.entry("items").or_insert_with(|| json!([]));
            Value::Object(map)
        }
        other => json!({"value": other}),
    }
}

pub fn login_required_for(config_dir: &Path) -> ToolError {
    ToolError {
        code: "login_required".into(),
        message: format!(
            "Google Workspace authentication is incomplete. Run `piccolo-gws setup --config-dir {}` and complete delegated Google login.",
            config_dir.display()
        ),
        login_required: true,
    }
}

fn normalize_backend_error(err: anyhow::Error, config_dir: &Path) -> ToolError {
    let message = err.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("auth") || lower.contains("login") || lower.contains("credential") {
        login_required_for(config_dir)
    } else if lower.contains("timeout") {
        ToolError {
            code: "backend_timeout".into(),
            message,
            login_required: false,
        }
    } else {
        ToolError {
            code: "backend_error".into(),
            message,
            login_required: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_for(name: &str, args: Value) -> (Vec<String>, Value) {
        let argv = tool_invocation(name, &args).unwrap();
        let json_index = argv.iter().position(|a| a == "--params").unwrap();
        let params: Value = serde_json::from_str(&argv[json_index + 1]).unwrap();
        (argv[..json_index].to_vec(), params)
    }

    #[test]
    fn gmail_search_uses_real_resource_method() {
        let (path, params) = params_for("gmail_search", json!({"query":"invoice","limit":5}));
        assert_eq!(path, vec!["gmail", "users", "messages", "list"]);
        assert_eq!(params["userId"], "me");
        assert_eq!(params["q"], "invoice");
        assert_eq!(params["maxResults"], 5);
    }

    #[test]
    fn gmail_read_defaults_to_authenticated_user() {
        let (path, params) = params_for("gmail_read", json!({"id":"msg1"}));
        assert_eq!(path, vec!["gmail", "users", "messages", "get"]);
        assert_eq!(params["userId"], "me");
        assert_eq!(params["id"], "msg1");
        assert_eq!(params["format"], "full");
    }

    #[test]
    fn gmail_user_override_is_honored() {
        let (_, params) = params_for("gmail_search", json!({"query":"","user":"a@b.com"}));
        assert_eq!(params["userId"], "a@b.com");
    }

    #[test]
    fn calendar_list_defaults_to_primary_and_drops_empty_range() {
        let (path, params) = params_for("calendar_list", json!({"time_min":"", "time_max":""}));
        assert_eq!(path, vec!["calendar", "events", "list"]);
        assert_eq!(params["calendarId"], "primary");
        assert!(params.get("timeMin").is_none());
        assert!(params.get("timeMax").is_none());
    }

    #[test]
    fn calendar_list_passes_through_time_range() {
        let (_, params) = params_for(
            "calendar_list",
            json!({"time_min":"2026-01-01T00:00:00Z","time_max":"2026-02-01T00:00:00Z"}),
        );
        assert_eq!(params["timeMin"], "2026-01-01T00:00:00Z");
        assert_eq!(params["timeMax"], "2026-02-01T00:00:00Z");
    }

    #[test]
    fn calendar_read_uses_event_id_parameter_name() {
        let (path, params) = params_for("calendar_read", json!({"id":"evt1"}));
        assert_eq!(path, vec!["calendar", "events", "get"]);
        assert_eq!(params["calendarId"], "primary");
        assert_eq!(params["eventId"], "evt1");
        assert!(params.get("id").is_none());
    }

    #[test]
    fn drive_search_omits_empty_query() {
        let (path, params) = params_for("drive_search", json!({"query":"","limit":3}));
        assert_eq!(path, vec!["drive", "files", "list"]);
        assert!(params.get("q").is_none());
        assert_eq!(params["pageSize"], 3);
    }

    #[test]
    fn drive_read_uses_file_id_parameter_name() {
        let (path, params) = params_for("drive_read", json!({"id":"file1"}));
        assert_eq!(path, vec!["drive", "files", "get"]);
        assert_eq!(params["fileId"], "file1");
    }

    #[test]
    fn unknown_tool_is_rejected() {
        assert!(tool_invocation("gmail_send", &json!({})).is_err());
    }

    #[test]
    fn mapped_methods_cover_every_tool() {
        assert_eq!(mapped_methods().len(), 6);
    }

    #[test]
    fn limit_is_capped_for_backend() {
        let (_, params) = params_for("gmail_search", json!({"query":"x","limit":500}));
        assert_eq!(params["maxResults"], 50);
    }

    #[test]
    fn parses_json_array_as_items() {
        assert_eq!(
            parse_output(br#"[{"id":"1"}]"#).unwrap(),
            json!({"items":[{"id":"1"}]})
        );
    }

    #[test]
    fn parses_text_as_text() {
        assert_eq!(parse_output(b"hello").unwrap(), json!({"text":"hello"}));
    }

    #[test]
    fn auth_error_shape_is_stable() {
        let dir = std::path::Path::new("/root/.piccolo/integrations/kimus/piccolo-gws");
        let err = login_required_for(dir);
        assert_eq!(err.code, "login_required");
        assert!(err.login_required);
        // The message must name the workspace's real state dir, not a stale default.
        assert!(err.message.contains(dir.to_str().unwrap()));
        assert!(!err.message.contains(crate::DEFAULT_CONFIG_DIR));
    }
}
