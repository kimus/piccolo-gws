use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

#[test]
fn describe_json_cli_outputs_descriptor() {
    let output = piccolo_gws_cmd()
        .args(["describe", "--json"])
        .output()
        .unwrap();
    assert_success(&output);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "piccolo.integration.v1");
    assert_eq!(value["name"], "gws");
    assert_eq!(
        value["setup"]["defaultConfigDir"],
        piccolo_gws::DEFAULT_CONFIG_DIR
    );
    assert_eq!(
        value["commands"]["setup"]["args"][2],
        piccolo_gws::DEFAULT_CONFIG_DIR
    );
    assert_eq!(
        value["commands"]["mcp"]["args"][2],
        piccolo_gws::DEFAULT_CONFIG_DIR
    );
    assert_eq!(
        value["commands"]["status"]["args"][2],
        piccolo_gws::DEFAULT_CONFIG_DIR
    );
}

#[test]
fn setup_reports_state_and_never_writes_runtime_config() {
    let tmp = test_temp_dir("setup-readonly");
    let state = tmp.join("state");
    let config = tmp.join("config.json");
    std::fs::write(&config, r#"{"unknown":true}"#).unwrap();
    let before = std::fs::read(&config).unwrap();

    let output = piccolo_gws_cmd()
        .args(["setup", "--config-dir", state.to_str().unwrap()])
        .output()
        .unwrap();
    assert_success(&output);
    let text = String::from_utf8_lossy(&output.stdout);
    // The integration proposes the MCP registration instead of writing it.
    assert!(text.contains("Configuration Piccolo will apply"), "{text}");
    assert!(text.contains("google_workspace"), "{text}");
    assert_eq!(
        std::fs::read(&config).unwrap(),
        before,
        "config.json must be untouched by the integration"
    );
    // And it must not leave a backup file behind either.
    assert!(
        !tmp.join("config.json.bak").exists(),
        "no second backup convention may be created here"
    );
    for name in std::fs::read_dir(&tmp).unwrap() {
        let path = name.unwrap().path();
        assert!(
            !path.to_string_lossy().contains(".bak"),
            "unexpected backup: {}",
            path.display()
        );
    }
}

#[test]
fn state_reports_facts_as_json_without_a_flag() {
    let tmp = test_temp_dir("state");
    let state = tmp.join("state");
    let output = piccolo_gws_cmd()
        .args(["state", "--config-dir", state.to_str().unwrap()])
        .output()
        .unwrap();
    assert_success(&output);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "piccolo.integration.state.v1");
    assert_eq!(value["name"], "gws");
    // Nothing configured yet: every gating fact must read false.
    assert_eq!(value["facts"]["credentials"], false);
    assert_eq!(value["facts"]["login"], false);
}

#[test]
fn call_unknown_action_emits_error_event() {
    let tmp = test_temp_dir("call-unknown");
    let output = piccolo_gws_cmd()
        .args([
            "call",
            "does_not_exist",
            "--config-dir",
            tmp.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null())
        .spawn()
        .and_then(|child| child.wait_with_output())
        .unwrap();
    assert!(
        !output.status.success(),
        "unknown action must exit non-zero"
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let event: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(event["schema"], "piccolo.integration.event.v1");
    assert_eq!(event["type"], "error");
    assert_eq!(event["code"], "unknown_action");
}

#[test]
fn call_gates_on_unmet_prerequisites_without_side_effects() {
    let tmp = test_temp_dir("call-gate");
    let state = tmp.join("state");
    let config = tmp.join("config.json");
    std::fs::write(&config, r#"{"unknown":true}"#).unwrap();
    let before = std::fs::read(&config).unwrap();

    let output = piccolo_gws_cmd()
        .args([
            "call",
            "check_connectivity",
            "--config-dir",
            state.to_str().unwrap(),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|child| child.wait_with_output())
        .unwrap();
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    let event: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(event["code"], "prerequisite_unmet");
    assert!(event["message"].as_str().unwrap().contains("login"));
    assert_eq!(std::fs::read(&config).unwrap(), before);
}

#[test]
fn call_set_credentials_writes_client_secret_from_two_values() {
    let tmp = test_temp_dir("call-set-credentials");
    let state = tmp.join("state");
    // Exactly the two values Google Cloud Console shows: no file, no paste.
    let answers = r#"{"client_id":"abc.apps.googleusercontent.com","client_secret":"shhh","project_id":"my-proj-123"}"#;

    let mut child = piccolo_gws_cmd()
        .args([
            "call",
            "set_credentials",
            "--config-dir",
            state.to_str().unwrap(),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(answers.as_bytes())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert_success(&output);

    let text = String::from_utf8_lossy(&output.stdout);
    let events: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "step" && e["status"] == "ok"),
        "{events:?}"
    );
    let result = events
        .iter()
        .find(|e| e["type"] == "result")
        .expect("result event");
    assert_eq!(
        result["result"]["configPatch"]["tools"]["mcp"]["servers"]["google_workspace"]["command"],
        "piccolo-gws"
    );

    let adopted = state.join("auth").join("client_secret.json");
    assert!(adopted.is_file());
    assert!(
        !text.contains("shhh"),
        "secret material must never appear in events: {text}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&adopted).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "credential file must be owner-only");
    }
    let doc: Value = serde_json::from_slice(&std::fs::read(&adopted).unwrap()).unwrap();
    assert_eq!(
        doc["installed"]["client_id"],
        "abc.apps.googleusercontent.com"
    );

    // State must now report the fact so the menu unlocks login.
    let state_out = piccolo_gws_cmd()
        .args(["state", "--config-dir", state.to_str().unwrap()])
        .output()
        .unwrap();
    let facts: Value = serde_json::from_slice(&state_out.stdout).unwrap();
    assert_eq!(facts["facts"]["credentials"], true);
}

#[test]
fn call_set_credentials_refuses_saves_that_would_break_the_pair() {
    let tmp = test_temp_dir("call-set-credentials-bad");
    let state = tmp.join("state");
    // `missing_input` is nothing at all; the others arrive as real fields but
    // cannot merge into a usable pair, which is a refusal rather than a gap in
    // the form. Both leave the state directory untouched.
    for (answers, code) in [
        ("{}", "missing_input"),
        (r#"{"client_id":"only-id"}"#, "credential_save_refused"),
        (
            r#"{"client_secret":"only-secret"}"#,
            "credential_save_refused",
        ),
        (
            r#"{"client_id":"  ","client_secret":"  "}"#,
            "credential_save_refused",
        ),
    ] {
        let output = piccolo_gws_cmd()
            .args([
                "call",
                "set_credentials",
                "--config-dir",
                state.to_str().unwrap(),
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(answers.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .unwrap();
        assert!(!output.status.success(), "accepted: {answers}");
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains(code), "{answers}: expected {code} in {text}");
        assert!(!state.join("auth").join("client_secret.json").exists());
    }
}

/// Absent means keep. This is what lets a host save the client ID without ever
/// having seen the secret — and why "absent" and "empty" must stay two answers.
#[test]
fn call_set_credentials_keeps_an_absent_secret_and_clears_an_empty_optional() {
    let tmp = test_temp_dir("call-set-credentials-merge");
    let state = tmp.join("state");
    let send = |payload: &str| {
        let output = piccolo_gws_cmd()
            .args([
                "call",
                "set_credentials",
                "--config-dir",
                state.to_str().unwrap(),
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(payload.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .unwrap();
        String::from_utf8_lossy(&output.stdout).into_owned()
    };

    send(r#"{"client_id":"client-a","client_secret":"secret-a","project_id":"proj-a"}"#);
    let stored = || {
        std::fs::read_to_string(state.join("auth").join("client_secret.json")).unwrap_or_default()
    };
    assert!(stored().contains("secret-a"));

    // Same id, no secret key: kept, and the run says so.
    let out = send(r#"{"client_id":"client-a"}"#);
    assert!(out.contains("Kept the stored client secret"), "{out}");
    assert!(
        stored().contains("secret-a"),
        "absent secret must not be cleared"
    );
    assert!(
        stored().contains("proj-a"),
        "absent project must not be cleared"
    );

    // Empty project id is a deliberate clear.
    send(r#"{"project_id":""}"#);
    assert!(
        stored().contains("\"project_id\": \"\""),
        "project id should have been cleared, file holds: {}",
        stored()
    );
    assert!(stored().contains("secret-a"));

    // A new id with no secret is refused: the two are one credential.
    let out = send(r#"{"client_id":"client-b"}"#);
    assert!(out.contains("credential_save_refused"), "{out}");
    assert!(
        stored().contains("client-a"),
        "a refused save must not half-apply"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn describe_publishes_actions_and_no_host_ports() {
    let output = piccolo_gws_cmd()
        .args(["describe", "--json"])
        .output()
        .unwrap();
    assert_success(&output);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["mcpServerKey"], "google_workspace");
    assert!(value["setup"].get("ports").is_none());
    let actions = value["actions"].as_array().unwrap();
    assert!(!actions.is_empty());
    let keys = value["stateKeys"].as_array().unwrap();
    assert!(keys.contains(&Value::String("credentials".into())));
    for action in actions {
        let requires = action["requires"].as_array().cloned().unwrap_or_default();
        for key in requires {
            assert!(keys.contains(&key), "requires undeclared state key {key}");
        }
    }
}

#[test]
fn status_json_reports_missing_backend() {
    let tmp = test_temp_dir("missing-backend");
    let output = piccolo_gws_cmd()
        .env("PICCOLO_GWS_BACKEND_BIN", tmp.join("missing-gws"))
        .args(["status", "--config-dir", tmp.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "piccolo.integration.status.v1");
    assert_eq!(value["name"], "gws");
    assert_eq!(value["status"], "fail");
    let checks = value["checks"].as_array().unwrap();
    assert!(checks
        .iter()
        .any(|c| c["id"] == "backend" && c["status"] == "fail"));
    assert!(checks
        .iter()
        .any(|c| c["id"] == "gmail" && c["status"] == "skip"));
}

#[test]
#[cfg(unix)]
fn status_json_reports_live_service_success() {
    let tmp = test_temp_dir("live-success");
    let fake = tmp.join("gws-fake");
    std::fs::write(
        &fake,
        r#"#!/bin/sh
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"auth_method":"tokens","credential_source":"encrypted_file","client_config_exists":true,"token_cache_exists":true}\n'
  exit 0
fi
printf '[]\n'
exit 0
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();
    let output = piccolo_gws_cmd()
        .env("PICCOLO_GWS_BACKEND_BIN", &fake)
        .args(["status", "--config-dir", tmp.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert_success(&output);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "ok");
    let checks = value["checks"].as_array().unwrap();
    assert!(checks
        .iter()
        .any(|c| c["id"] == "gmail" && c["status"] == "ok"));
    assert!(checks
        .iter()
        .any(|c| c["id"] == "calendar" && c["status"] == "ok"));
    assert!(checks
        .iter()
        .any(|c| c["id"] == "drive" && c["status"] == "ok"));
}

#[test]
fn status_json_reports_unauthenticated_despite_exit_zero() {
    // The real `gws auth status` exits 0 even with no credentials; auth state
    // must come from its JSON payload, never the exit code.
    let tmp = test_temp_dir("unauth-exit-zero");
    let fake = tmp.join("gws-fake");
    std::fs::write(
        &fake,
        r#"#!/bin/sh
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"auth_method":"none","credential_source":"none","client_config_exists":false,"token_cache_exists":false}\n'
  exit 0
fi
printf '[]\n'
exit 0
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();
    let output = piccolo_gws_cmd()
        .env("PICCOLO_GWS_BACKEND_BIN", &fake)
        .args(["status", "--config-dir", tmp.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_ne!(value["status"], "ok");
    let auth = value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "auth")
        .unwrap();
    assert_eq!(auth["status"], "fail");
    let gmail = value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "gmail")
        .unwrap();
    assert_eq!(gmail["status"], "skip");
}

#[test]
fn status_requires_json() {
    let output = piccolo_gws_cmd().args(["status"]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("only --json output is supported"));
}

// MCP over stdio is newline-delimited JSON: one message per line, no framing headers.
// PicoClaw's client is the official MCP Go SDK, which reads stdout line by line and has
// no Content-Length support, so an LSP-framed reply fails the handshake with
// `invalid character 'C' looking for beginning of value` and Piccolo then loses every
// Google Workspace tool. These tests pin the reply shape for both input framings
// `read_message` accepts.
fn mcp_stdio_reply(request: &[u8]) -> String {
    let output = piccolo_gws_cmd()
        .args(["mcp", "--config-dir", "/tmp/piccolo-gws-test"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(request)?;
            child.wait_with_output()
        })
        .unwrap();
    assert_success(&output);
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn assert_newline_delimited_reply(stdout: &str) {
    assert!(
        !stdout.contains("Content-Length"),
        "MCP stdio replies must never be LSP-framed: {stdout}"
    );
    assert!(
        stdout.contains("piccolo-gws"),
        "reply should identify the server: {stdout}"
    );
    let line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON reply line in: {stdout}"));
    let value: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|err| {
        panic!("a reply line must be exactly one JSON value ({err}): {line}")
    });
    assert_eq!(value["result"]["serverInfo"]["name"], "piccolo-gws");
}

#[test]
fn mcp_initialize_over_stdio_replies_with_one_json_line() {
    let stdout = mcp_stdio_reply(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n");
    assert_newline_delimited_reply(&stdout);
}

#[test]
fn mcp_initialize_still_accepts_content_length_framed_request() {
    let stdout = mcp_stdio_reply(
        b"Content-Length: 46\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}",
    );
    assert_newline_delimited_reply(&stdout);
}

fn piccolo_gws_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_piccolo-gws"))
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn test_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "piccolo-gws-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn write_failure_does_not_panic() {
    // Rust ignores SIGPIPE, so a plain `println!` panics (exit 101) the moment a
    // consumer closes the pipe — `describe --json | head -1`, or a pager that
    // quits. Dropping the read end reproduces that exactly, and portably (no
    // /dev/full, which macOS refuses to open). The command still succeeded; only
    // delivery failed, so that must not be reported as a crash.
    for args in [
        vec!["describe", "--json"],
        vec!["backend-methods"],
        vec!["state", "--config-dir", "/tmp/piccolo-gws-pipe-closed"],
    ] {
        let (reader, writer) = std::io::pipe().expect("platform pipe unavailable");
        drop(reader);
        let output = piccolo_gws_cmd()
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(writer))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked"),
            "`{}` panicked on a closed stdout:\n{stderr}",
            args.join(" ")
        );
        assert!(
            !stderr.contains("Broken pipe") && !stderr.contains("EPIPE"),
            "`{}` surfaced a broken-pipe error:\n{stderr}",
            args.join(" ")
        );
    }
}
