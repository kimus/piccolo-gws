use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
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
            return Err(login_required());
        }
        let argv: Vec<String> = match name {
            "gmail_search" => vec![
                "gmail".into(),
                "search".into(),
                arg_string(&args, "query"),
                "--limit".into(),
                arg_limit(&args),
            ],
            "gmail_read" => vec!["gmail".into(), "read".into(), arg_string(&args, "id")],
            "calendar_list" => vec![
                "calendar".into(),
                "list".into(),
                "--time-min".into(),
                arg_string(&args, "time_min"),
                "--time-max".into(),
                arg_string(&args, "time_max"),
            ],
            "calendar_read" => vec!["calendar".into(), "read".into(), arg_string(&args, "id")],
            "drive_search" => vec![
                "drive".into(),
                "search".into(),
                arg_string(&args, "query"),
                "--limit".into(),
                arg_limit(&args),
            ],
            "drive_read" => vec!["drive".into(), "read".into(), arg_string(&args, "id")],
            _ => {
                return Err(ToolError {
                    code: "unknown_tool".into(),
                    message: format!("unknown tool: {name}"),
                    login_required: false,
                })
            }
        };
        self.run_gws(&argv).map_err(normalize_backend_error)
    }

    pub fn is_authenticated(&self) -> bool {
        if which::which("gws").is_err() {
            return false;
        }
        Command::new("gws")
            .arg("auth")
            .arg("status")
            .env("GOOGLE_WORKSPACE_CLI_CONFIG_DIR", self.config_dir.join("auth"))
            .env("GWS_CONFIG_DIR", self.config_dir.join("auth"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn run_gws(&self, args: &[String]) -> Result<Value> {
        let mut child = Command::new("gws")
            .args(args)
            .env("GOOGLE_WORKSPACE_CLI_CONFIG_DIR", self.config_dir.join("auth"))
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

fn arg_limit(args: &Value) -> String {
    args.get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(10)
        .min(50)
        .to_string()
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

pub fn login_required() -> ToolError {
    ToolError {
        code: "login_required".into(),
        message: "Google Workspace authentication is incomplete. Run `piccolo-gws setup --config-dir /root/.picoclaw/integrations/google` and complete delegated Google login.".into(),
        login_required: true,
    }
}

fn normalize_backend_error(err: anyhow::Error) -> ToolError {
    let message = err.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("auth") || lower.contains("login") || lower.contains("credential") {
        login_required()
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
        let err = login_required();
        assert_eq!(err.code, "login_required");
        assert!(err.login_required);
    }
}
