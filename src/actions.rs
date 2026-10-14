//! Action layer exposed to Piccolo.
//!
//! Piccolo renders the menu and collects answers; this module executes one
//! action, emitting newline-delimited JSON events on stdout. Human-readable
//! logs go to stderr so stdout stays a clean protocol channel.
//!
//! Secrets therefore travel on stdin only: argv is visible through `docker top`
//! and environment variables through `docker inspect`.

use crate::backend::{backend_auth_state, GwsBackend};
use crate::config;
use anyhow::Result;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

pub const EVENT_SCHEMA: &str = "piccolo.integration.event.v1";
pub const STATE_SCHEMA: &str = "piccolo.integration.state.v1";

/// The single `tools.mcp.servers.<key>` entry this integration may propose to
/// manage; Piccolo enforces the same path in its own allowlist.
pub const MCP_SERVER_KEY: &str = "google_workspace";

/// Backstop for a human finishing the browser authorization. Piccolo's own
/// per-action budget expires slightly earlier so the orchestrator, not the
/// child process, decides when an attempt is abandoned.
const LOGIN_WAIT: Duration = Duration::from_secs(20 * 60);

/// State keys actions may `requires` against. Every key must be reported by
/// [`state_facts`], or Piccolo refuses the descriptor.
pub const STATE_KEYS: [&str; 3] = ["credentials", "login", "apis_enabled"];

#[derive(Debug, Clone, Serialize)]
pub struct ActionInput {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    /// The host blocks advancing past, or submitting with, an empty required
    /// field. Presence is declared here rather than written into the title, so the
    /// renderer enforces it and the label stays readable.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    pub id: String,
    pub role: String,
    pub title: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<ActionInput>,
}

fn input(key: &str, kind: &str, title: &str) -> ActionInput {
    ActionInput {
        key: key.into(),
        kind: kind.into(),
        title: title.into(),
        required: false,
        destination: None,
        options: None,
        sensitive: false,
    }
}

fn credential_inputs() -> Vec<ActionInput> {
    // Google Cloud Console shows exactly these values for a Desktop OAuth client.
    // A file download is not required: the delegated backend persists what it
    // needs from these values alone.
    //
    // Both credential fields are required and neither is ever hidden. They used to
    // carry `whenUnmet: credentials`, which hid them once a client existed — so
    // the action that exists to set credentials could no longer ask for them, and
    // the form submitted whatever optional field happened to survive.
    let mut id = input(
        "client_id",
        "text",
        "OAuth client ID (client type must be \"Desktop app\")",
    );
    id.required = true;
    let mut secret = input("client_secret", "secret", "OAuth client secret");
    secret.required = true;
    secret.sensitive = true;
    // Optional: enables a deep link to this project's credentials page when Google
    // rejects the request (it otherwise tells the user nothing useful). Deliberately
    // not required, because deleting its value is how it gets cleared.
    let project = input(
        "project_id",
        "text",
        "Google Cloud project ID (optional, used for Console links)",
    );
    vec![id, secret, project]
}

/// Read the stored project id so Console links can be deep-linked.
fn stored_project_id(config_dir: &Path) -> String {
    let path = config_dir.join("auth").join("client_secret.json");
    let Ok(raw) = std::fs::read(&path) else {
        return String::new();
    };
    let Ok(value) = serde_json::from_slice::<Value>(&raw) else {
        return String::new();
    };
    value
        .get("installed")
        .and_then(|i| i.get("project_id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn console_credentials_url(config_dir: &Path) -> String {
    let project = stored_project_id(config_dir);
    if project.is_empty() {
        return "https://console.cloud.google.com/apis/credentials".to_string();
    }
    format!("https://console.cloud.google.com/apis/credentials?project={project}")
}

fn scope_input() -> ActionInput {
    // Asked every time, never suppressed: choosing to log in again is how a user
    // picks a different scope, and a form that quietly asks nothing would send the
    // login off with a default they never chose.
    let mut base = input("scope", "select", "Requested scope");
    base.options = Some(vec!["readonly".into(), "full".into()]);
    base
}

pub fn actions() -> Vec<Action> {
    vec![
        Action {
            id: "set_credentials".into(),
            role: "setup".into(),
            title: "Set OAuth client credentials".into(),
            requires: vec![],
            inputs: credential_inputs(),
        },
        Action {
            id: "login".into(),
            role: "setup".into(),
            title: "Login / authorize Google account".into(),
            requires: vec!["credentials".into()],
            inputs: vec![scope_input()],
        },
        Action {
            id: "check_connectivity".into(),
            role: "diagnostic".into(),
            title: "Check Gmail, Calendar, and Drive connectivity".into(),
            requires: vec!["login".into()],
            inputs: vec![],
        },
    ]
}

fn state_facts(config_dir: &Path) -> Value {
    let auth_dir = config_dir.join("auth");
    // Facts come from the backend when it is reachable (it knows about
    // env-provided and keyring credentials we cannot see), but the integration's
    // own state directory is authoritative when the backend is absent or silent.
    // Without that fallback `state` would report credentials as missing right
    // after we stored them.
    let backend = backend_auth_state(&auth_dir.clone());
    let snapshot = crate::setup::capture_snapshot(&auth_dir);
    // A document that exists but cannot authorize must read as unconfigured even
    // though the backend sees a file. Existence used to be the test here, which is
    // how a stale half-written client made `login` look available.
    let credentials = if snapshot.problem.is_some() {
        false
    } else {
        backend.client_configured() || snapshot.credentials.configured()
    };
    let login = backend.authenticated();
    json!({
        "credentials": credentials,
        "login": login,
        // Live API reachability is only observable with a token; probing here would
        // cost a network round trip per menu render, so check_connectivity reports it.
        "apis_enabled": login,
    })
}

/// Current field values, for the host to pre-fill. Absent-and-blank would
/// otherwise be indistinguishable, and a field the user cannot see cannot be
/// deliberately cleared.
///
/// A secret reports presence and never its value, not even masked: what leaves
/// this process ends up in a terminal, and a mask sitting in a field can be
/// half-edited into a wrong secret that still looks present.
pub(crate) fn field_values(config_dir: &Path) -> Value {
    let stored = crate::setup::read_stored_client(config_dir);
    let present = |v: &str| json!({ "value": v, "set": !v.is_empty() });
    json!({
        "client_id": present(&stored.client_id),
        "client_secret": { "set": !stored.client_secret.is_empty() },
        "project_id": present(&stored.project_id),
    })
}

/// Problems worth stating next to the facts, so "credentials: false" is not the
// only thing a user or a doctor run learns about a file that is there.
pub(crate) fn state_notes(config_dir: &Path) -> Value {
    let snapshot = crate::setup::capture_snapshot(&config_dir.join("auth"));
    match snapshot.problem {
        Some(problem) => json!({ "problem": problem }),
        None => json!({}),
    }
}

pub fn run_state(config_dir: &Path) -> Result<()> {
    let out = json!({
        "schema": STATE_SCHEMA,
        "name": "gws",
        "facts": state_facts(config_dir),
        "values": field_values(config_dir),
        "notes": state_notes(config_dir),
    });
    println_ok!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Protocol emitter: stdout only, and never any secret value.
struct Emitter;

impl Emitter {
    fn emit(value: &Value) {
        if let Ok(line) = serde_json::to_string(value) {
            let mut out = io::stdout();
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        }
    }

    fn step(id: &str, status: &str, message: &str) {
        Self::emit(
            &json!({"schema": EVENT_SCHEMA, "type": "step", "id": id, "status": status, "message": message}),
        );
    }

    fn open_url(url: &str, label: &str) {
        Self::emit(
            &json!({"schema": EVENT_SCHEMA, "type": "open_url", "url": url, "label": label}),
        );
    }

    fn wait_redirect(url: &str) {
        Self::emit(&json!({"schema": EVENT_SCHEMA, "type": "wait_redirect", "url": url}));
    }

    fn ok(config_dir: &Path) {
        let patch = config::mcp_server_patch(config_dir);
        Self::emit(&json!({
            "schema": EVENT_SCHEMA,
            "type": "result",
            "status": "ok",
            // Proposed, never applied here: Piccolo validates and writes
            // config.json.
            "result": { "status": "ok", "configPatch": patch },
        }));
    }

    fn error(code: &str, message: &str, remediation: Option<&str>) {
        let mut value =
            json!({"schema": EVENT_SCHEMA, "type": "error", "code": code, "message": message});
        if let Some(r) = remediation {
            value["remediation"] = json!(r);
        }
        Self::emit(&value);
    }

    fn error_with_link(code: &str, message: &str, remediation: &str, url: &str) {
        Self::emit(&json!({
            "schema": EVENT_SCHEMA, "type": "error", "code": code, "message": message,
            "remediation": remediation,
            "remediationLabel": "Open Google Cloud Console",
            "remediationUrl": url,
        }));
    }
}

fn read_answers() -> Result<Value> {
    let mut buf = String::new();
    io::stdin().lock().read_to_string(&mut buf)?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return Ok(json!({}));
    }
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|err| anyhow::anyhow!("integration answers on stdin are not valid JSON: {err}"))?;
    if !value.is_object() {
        anyhow::bail!("integration answers on stdin must be a JSON object");
    }
    Ok(value)
}

pub fn run_call(action_id: &str, config_dir: &Path) -> Result<i32> {
    let known = actions();
    let Some(action) = known.into_iter().find(|a| a.id == action_id) else {
        Emitter::error(
            "unknown_action",
            &format!("unknown action id: {action_id}"),
            Some("Run `piccolo-gws describe` to list action ids."),
        );
        return Ok(2);
    };

    let facts = state_facts(config_dir);
    let missing: Vec<String> = action
        .requires
        .iter()
        .filter(|key| facts.get(*key).and_then(Value::as_bool) != Some(true))
        .cloned()
        .collect();
    if !missing.is_empty() {
        // No side effects when a prerequisite is unmet.
        Emitter::error(
            "prerequisite_unmet",
            &format!("action {} requires: {}", action.id, missing.join(", ")),
            Some(
                "Piccolo gates the menu on these facts; run the action that satisfies them first.",
            ),
        );
        return Ok(3);
    }

    let answers = match read_answers() {
        Ok(value) => value,
        Err(err) => {
            Emitter::error("bad_answers", &err.to_string(), None);
            return Ok(2);
        }
    };

    let mut code = match action.id.as_str() {
        "set_credentials" => set_credentials(&answers, config_dir),
        "login" => authorize(config_dir, &answers),
        "check_connectivity" => connectivity(config_dir),
        other => {
            Emitter::error(
                "unimplemented",
                &format!("action {other} has no handler"),
                None,
            );
            4
        }
    };
    if code == 0 {
        Emitter::ok(config_dir);
    } else {
        code = code.max(1);
    }
    Ok(code)
}

/// Save the fields that were sent.
///
/// Each field is read as present-or-absent rather than defaulted to a string, and
/// that distinction is the whole contract: an absent secret means "keep the one I
/// cannot see", an empty one means "clear this". Reading both as "" would let a
/// save that only carried the client ID wipe a working secret.
fn set_credentials(answers: &Value, config_dir: &Path) -> i32 {
    let supplied = |key: &str| answers.get(key).and_then(Value::as_str).map(str::trim);
    let client_id = supplied("client_id");
    let client_secret = supplied("client_secret");
    let project_id = supplied("project_id");
    if client_id.is_none() && client_secret.is_none() && project_id.is_none() {
        Emitter::step(
            "credentials",
            "fail",
            "No credential fields were supplied; nothing was changed.",
        );
        Emitter::error_with_link(
            "missing_input",
            "client credentials were not supplied",
            "Create a Desktop OAuth client in Google Cloud Console and copy its client ID and secret.",
            "https://console.cloud.google.com/apis/credentials",
        );
        return 5;
    }
    match crate::setup::save_client_credentials(client_id, client_secret, project_id, config_dir) {
        Ok(save) => {
            // Field names and outcomes only. No value is ever echoed, and the
            // client ID is not echoed either: it is identifying rather than
            // secret, and naming it here buys nothing a user needs to read twice.
            if save.client_id_replaced {
                Emitter::step("credentials", "ok", "OAuth client ID replaced.");
            }
            if save.client_secret_replaced {
                Emitter::step("credentials", "ok", "OAuth client secret replaced.");
            } else if client_secret.is_none() {
                // Worth saying out loud: the field was blank in the form, and the
                // stored secret survived anyway.
                Emitter::step("credentials", "ok", "Kept the stored client secret.");
            }
            if save.project_id_cleared {
                Emitter::step("credentials", "ok", "Project ID cleared.");
            }
            Emitter::step(
                "credentials",
                "ok",
                "OAuth client credentials are configured.",
            );
            0
        }
        Err(err) => {
            let message = err.to_string();
            Emitter::step("credentials", "fail", &message);
            Emitter::error_with_link(
                "credential_save_refused",
                &message,
                "Enter the client ID and its secret together to set a client, or change only one of them.",
                "https://console.cloud.google.com/apis/credentials",
            );
            6
        }
    }
}

fn authorize(config_dir: &Path, answers: &Value) -> i32 {
    let auth_dir = config_dir.join("auth");
    let scope_flag = match answers.get("scope").and_then(Value::as_str) {
        Some("full") => "--full",
        _ => "--readonly",
    };
    Emitter::step("login", "running", "Starting browser authorization…");
    let mut child = match Command::new(crate::backend::backend_command_name())
        .arg("auth")
        .arg("login")
        .arg(scope_flag)
        .env("GOOGLE_WORKSPACE_CLI_CONFIG_DIR", &auth_dir)
        .env("GWS_CONFIG_DIR", &auth_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            Emitter::error(
                "backend_unavailable",
                &format!("cannot start the backend login: {err}"),
                None,
            );
            return 7;
        }
    };

    // The backend prints its authorize URL (with an ephemeral loopback redirect)
    // on stderr and then blocks until the browser redirect arrives. Reading it on
    // a worker thread lets the deadline below apply to the HUMAN step: the child
    // only exits once the redirect lands, so a timeout evaluated after the read
    // loop would be unreachable.
    let stderr = match child.stderr.take() {
        Some(stream) => stream,
        None => {
            let _ = child.kill();
            Emitter::error(
                "backend_unavailable",
                "the backend produced no stderr stream",
                None,
            );
            return 7;
        }
    };
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut reader = io::BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let text = line.trim_end().to_string();
            if text.is_empty() {
                continue;
            }
            if tx.send(text).is_err() {
                break;
            }
        }
    });

    let deadline = Instant::now() + LOGIN_WAIT;
    let mut url_seen = false;
    let mut saw_redirect_mismatch = false;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            Emitter::error(
				"login_timeout",
				&format!(
					"authorization did not complete within {}s",
					LOGIN_WAIT.as_secs()
				),
				Some("Rerun login and finish the browser prompt promptly; the callback port changes every attempt."),
			);
            return 10;
        }
        match rx.recv_timeout(remaining) {
            Ok(text) => {
                if let Some(url) = extract_authorize_url(&text) {
                    if !url_seen {
                        url_seen = true;
                        Emitter::open_url(
                            url,
                            "Authorize this Google account in your browser to finish setup",
                        );
                        Emitter::wait_redirect(url);
                    }
                } else {
                    if text.contains("redirect_uri_mismatch") {
                        saw_redirect_mismatch = true;
                    }
                    // Backend chatter stays human-readable and off the protocol.
                    let mut err = io::stderr();
                    let _ = writeln!(err, "{text}");
                }
            }
            // The channel simply had no line yet; the loop re-checks the deadline.
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    if !url_seen {
        let _ = child.kill();
        let _ = child.wait();
        Emitter::error(
            "authorize_url_missing",
            "the backend did not report an authorization URL",
            Some("Check that the OAuth client id and secret are correct."),
        );
        return 8;
    }

    match child.wait() {
        Ok(status) if status.success() => {
            Emitter::step(
                "login",
                "ok",
                "Google account authorized; tokens are stored in this workspace.",
            );
            0
        }
        Ok(_) if saw_redirect_mismatch => {
            // Google's own page never mentions the deciding factor: client type.
            Emitter::open_url(
				&console_credentials_url(config_dir),
				"Open this OAuth client in Google Cloud Console and check its type is \"Desktop app\"",
			);
            Emitter::error(
				"redirect_uri_mismatch",
				"Google rejected the redirect URI. This integration signs in with a loopback 				 redirect on a port chosen fresh for every attempt, so the OAuth client type 				 must be 'Desktop app'; a 'Web application' client requires an exact 				 registered URI and can never match.",
				Some("In Google Cloud Console: Credentials → Create client → Application type: 					  Desktop app, then rerun 'Set OAuth client credentials' with the new id and secret."),
			);
            11
        }
        Ok(status) => {
            Emitter::error(
                "login_failed",
                &format!("browser login exited {status}"),
                Some("Rerun login, or check the Google Cloud OAuth consent configuration."),
            );
            9
        }
        Err(err) => {
            Emitter::error(
                "login_failed",
                &format!("could not wait for the backend: {err}"),
                None,
            );
            9
        }
    }
}

fn connectivity(config_dir: &Path) -> i32 {
    let backend = GwsBackend::new(PathBuf::from(config_dir));
    let probes = [
        ("gmail", "gmail_search", json!({"query": "", "limit": 1})),
        (
            "calendar",
            "calendar_list",
            json!({"time_min": "", "time_max": ""}),
        ),
        ("drive", "drive_search", json!({"query": "", "limit": 1})),
    ];
    let mut failed = false;
    for (id, tool, args) in probes {
        let result = backend.call_tool(tool, args);
        if result.get("ok").and_then(Value::as_bool) == Some(true) {
            Emitter::step(id, "ok", "reachable");
        } else {
            failed = true;
            let message = result
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("call failed")
                .to_string();
            Emitter::step(id, "fail", &message);
        }
    }
    if failed {
        Emitter::error(
            "connectivity_failed",
            "one or more Google Workspace APIs are unreachable",
            Some(
                "Rerun login with a wider scope, or confirm the APIs are enabled for the project.",
            ),
        );
        return 11;
    }
    0
}

fn extract_authorize_url(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.starts_with("https://accounts.google.com/o/oauth2/auth?") {
        return Some(trimmed);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_requires_key_is_a_reported_fact() {
        let facts = state_facts(Path::new("/nonexistent-piccolo-gws-test"));
        for action in actions() {
            for key in &action.requires {
                assert!(
                    facts.get(key).is_some(),
                    "action {} requires undeclared fact {}",
                    action.id,
                    key
                );
            }
        }
    }

    #[test]
    fn state_keys_constant_matches_fact_object() {
        let facts = state_facts(Path::new("/nonexistent-piccolo-gws-test"));
        let obj = facts.as_object().unwrap();
        assert_eq!(obj.len(), STATE_KEYS.len());
        for key in STATE_KEYS {
            assert!(obj.contains_key(key), "missing state key {key}");
        }
    }

    #[test]
    fn action_ids_are_unique_and_roles_known() {
        let mut seen = std::collections::HashSet::new();
        for action in actions() {
            assert!(seen.insert(action.id.clone()), "duplicate id {}", action.id);
            assert!(
                action.role == "setup" || action.role == "diagnostic",
                "unexpected role for {}",
                action.id
            );
            for input in &action.inputs {
                assert!(
                    matches!(
                        input.kind.as_str(),
                        "text" | "secret" | "path" | "select" | "multiselect" | "confirm"
                    ),
                    "unsupported input type {}",
                    input.kind
                );
                if input.kind == "path" {
                    assert!(input.destination.is_some(), "path input needs destination");
                }
            }
        }
    }

    #[test]
    fn authorize_url_detection_ignores_other_output() {
        let url = "https://accounts.google.com/o/oauth2/auth?scope=x&redirect_uri=http%3A%2F%2Flocalhost%3A35935";
        assert_eq!(extract_authorize_url(url), Some(url));
        assert_eq!(
            extract_authorize_url("  Open this URL in your browser:"),
            None
        );
        assert_eq!(
            extract_authorize_url("https://console.cloud.google.com/"),
            None
        );
    }
}
