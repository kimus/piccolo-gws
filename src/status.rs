use crate::backend::{backend_command_name, GwsBackend};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
    Unknown,
    Skip,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatusCheck {
    pub id: &'static str,
    pub label: &'static str,
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub schema: &'static str,
    pub name: &'static str,
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub summary: String,
    pub checks: Vec<StatusCheck>,
}

pub fn run_status(
    config_dir: &Path,
    picoclaw_config: Option<&Path>,
    json_output: bool,
) -> Result<()> {
    if !json_output {
        anyhow::bail!("only --json output is supported");
    }
    let report = build_status(config_dir, picoclaw_config);
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.status == CheckStatus::Fail {
        std::process::exit(1);
    }
    Ok(())
}

pub fn build_status(config_dir: &Path, picoclaw_config: Option<&Path>) -> StatusReport {
    let mut checks = Vec::new();
    checks.push(state_dir_check(config_dir));
    let backend_ok = which::which(backend_command_name()).is_ok();
    checks.push(if backend_ok {
        ok("backend", "gws backend", "gws backend command is available")
    } else {
        fail(
            "backend",
            "gws backend",
            "gws backend command is not available on PATH",
            "Rebuild or upgrade the Piccolo runtime image so the packaged gws backend is available.",
        )
    });

    let auth_ok = backend_ok && auth_status(config_dir);
    checks.push(if auth_ok {
        ok("auth", "Google auth", "authenticated")
    } else {
        fail(
            "auth",
            "Google auth",
            "Google Workspace authentication is incomplete or invalid",
            "Run `piccolo integration setup gws` and complete delegated Google login.",
        )
    });

    if auth_ok {
        let backend = GwsBackend::new(config_dir.to_path_buf());
        checks.push(probe_check(
            &backend,
            "gmail",
            "Gmail",
            "gmail_search",
            json!({"query":"newer_than:3650d", "limit": 1}),
        ));
        checks.push(probe_check(
            &backend,
            "calendar",
            "Calendar",
            "calendar_list",
            json!({"time_min":"1970-01-01T00:00:00Z", "time_max":"2100-01-01T00:00:00Z"}),
        ));
        checks.push(probe_check(
            &backend,
            "drive",
            "Drive",
            "drive_search",
            json!({"query":"", "limit": 1}),
        ));
    } else {
        checks.push(skip_auth("gmail", "Gmail"));
        checks.push(skip_auth("calendar", "Calendar"));
        checks.push(skip_auth("drive", "Drive"));
    }

    if let Some(path) = picoclaw_config {
        checks.push(picoclaw_config_check(path, config_dir));
    }

    let status = derive_overall(&checks);
    let summary = match status {
        CheckStatus::Ok => "Google Workspace connectivity checks passed".to_string(),
        CheckStatus::Warn => "Google Workspace connectivity checks have warnings".to_string(),
        CheckStatus::Fail => "Google Workspace connectivity checks failed".to_string(),
        CheckStatus::Unknown => "Google Workspace connectivity is unknown".to_string(),
        CheckStatus::Skip => "Google Workspace connectivity checks skipped".to_string(),
    };
    StatusReport {
        schema: "piccolo.integration.status.v1",
        name: "gws",
        status,
        summary,
        checks,
    }
}

fn auth_status(config_dir: &Path) -> bool {
    Command::new(backend_command_name())
        .arg("auth")
        .arg("status")
        .env("GOOGLE_WORKSPACE_CLI_CONFIG_DIR", config_dir.join("auth"))
        .env("GWS_CONFIG_DIR", config_dir.join("auth"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn probe_check(
    backend: &GwsBackend,
    id: &'static str,
    label: &'static str,
    tool: &str,
    args: Value,
) -> StatusCheck {
    let value = backend.call_tool(tool, args);
    if value.get("ok").and_then(Value::as_bool) == Some(true) {
        ok(id, label, "connected")
    } else {
        let message = value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("connectivity check failed");
        let remediation = remediation_for(message);
        StatusCheck {
            id,
            label,
            status: CheckStatus::Fail,
            message: message.to_string(),
            remediation: Some(remediation),
        }
    }
}

fn state_dir_check(config_dir: &Path) -> StatusCheck {
    if config_dir.exists() {
        ok(
            "state-dir",
            "State directory",
            &format!("{} exists", config_dir.display()),
        )
    } else {
        StatusCheck {
            id: "state-dir",
            label: "State directory",
            status: CheckStatus::Warn,
            message: format!("{} does not exist yet", config_dir.display()),
            remediation: Some("Run `piccolo integration setup gws`.".to_string()),
        }
    }
}

fn picoclaw_config_check(path: &Path, config_dir: &Path) -> StatusCheck {
    match read_json(path) {
        Ok(value) => {
            let server = value.pointer("/tools/mcp/servers/google_workspace");
            let command_ok = server
                .and_then(|s| s.get("command"))
                .and_then(Value::as_str)
                == Some("piccolo-gws");
            let args = server.and_then(|s| s.get("args")).and_then(Value::as_array);
            let config_arg = config_dir.to_string_lossy().to_string();
            let args_ok = args
                .map(|items| {
                    let strings: Vec<String> = items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect();
                    strings.contains(&"mcp".to_string())
                        && strings.contains(&"--read-only".to_string())
                        && strings.contains(&config_arg)
                })
                .unwrap_or(false);
            if command_ok && args_ok {
                ok(
                    "picoclaw-config",
                    "PicoClaw config",
                    "google_workspace MCP server is registered",
                )
            } else {
                fail(
                    "picoclaw-config",
                    "PicoClaw config",
                    "google_workspace MCP server is missing or does not match expected args",
                    "Run `piccolo integration setup gws` to patch PicoClaw config.",
                )
            }
        }
        Err(err) => fail(
            "picoclaw-config",
            "PicoClaw config",
            &err.to_string(),
            "Run `piccolo integration setup gws` after fixing config.json.",
        ),
    }
}

fn read_json(path: &Path) -> Result<Value> {
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&data).with_context(|| format!("parsing {}", path.display()))
}

fn derive_overall(checks: &[StatusCheck]) -> CheckStatus {
    if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        return CheckStatus::Fail;
    }
    if checks
        .iter()
        .any(|c| matches!(c.status, CheckStatus::Warn | CheckStatus::Unknown))
    {
        return CheckStatus::Warn;
    }
    CheckStatus::Ok
}

fn ok(id: &'static str, label: &'static str, message: &str) -> StatusCheck {
    StatusCheck {
        id,
        label,
        status: CheckStatus::Ok,
        message: message.to_string(),
        remediation: None,
    }
}

fn fail(id: &'static str, label: &'static str, message: &str, remediation: &str) -> StatusCheck {
    StatusCheck {
        id,
        label,
        status: CheckStatus::Fail,
        message: message.to_string(),
        remediation: Some(remediation.to_string()),
    }
}

fn skip_auth(id: &'static str, label: &'static str) -> StatusCheck {
    StatusCheck {
        id,
        label,
        status: CheckStatus::Skip,
        message: "skipped because Google auth is not valid".to_string(),
        remediation: Some(
            "Run `piccolo integration setup gws` and complete delegated Google login.".to_string(),
        ),
    }
}

fn remediation_for(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    if lower.contains("scope") || lower.contains("permission") || lower.contains("forbidden") {
        "Run `piccolo integration setup gws` and grant the required read-only Google Workspace scopes.".to_string()
    } else if lower.contains("timeout") {
        "Check network connectivity and try again.".to_string()
    } else if lower.contains("api") || lower.contains("disabled") {
        "Enable the relevant Google Workspace API in the Google Cloud project, then rerun setup if needed.".to_string()
    } else {
        "Run `piccolo integration setup gws` to verify Google auth and configuration.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn derives_overall_status() {
        assert_eq!(derive_overall(&[ok("x", "X", "ok")]), CheckStatus::Ok);
        assert_eq!(
            derive_overall(&[StatusCheck {
                id: "x",
                label: "X",
                status: CheckStatus::Warn,
                message: "w".into(),
                remediation: None
            }]),
            CheckStatus::Warn
        );
        assert_eq!(
            derive_overall(&[fail("x", "X", "bad", "fix")]),
            CheckStatus::Fail
        );
    }

    #[test]
    fn picoclaw_config_check_accepts_expected_server() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join("config.json");
        let state = tmp.path().join("state");
        std::fs::write(&cfg, serde_json::to_vec(&json!({"tools":{"mcp":{"servers":{"google_workspace":{"command":"piccolo-gws","args":["mcp","--config-dir",state.to_string_lossy(),"--read-only"]}}}}})).unwrap()).unwrap();
        let check = picoclaw_config_check(&cfg, &state);
        assert_eq!(check.status, CheckStatus::Ok);
    }

    #[test]
    fn picoclaw_config_check_reports_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join("config.json");
        let state = tmp.path().join("state");
        std::fs::write(&cfg, b"{\"tools\":{}}").unwrap();
        let check = picoclaw_config_check(&cfg, &state);
        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.remediation.unwrap().contains("setup"));
    }

    #[test]
    fn remediation_for_common_backend_errors() {
        assert!(remediation_for("insufficient scope").contains("scopes"));
        assert!(remediation_for("backend_timeout").contains("network"));
        assert!(remediation_for("API disabled").contains("Enable"));
    }
}
