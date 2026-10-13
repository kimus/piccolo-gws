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
fn setup_patches_config_with_yes() {
    let tmp = test_temp_dir("setup");
    let state = tmp.join("state");
    let config = tmp.join("config.json");
    std::fs::write(&config, r#"{"unknown":true}"#).unwrap();
    let output = piccolo_gws_cmd()
        .args([
            "setup",
            "--config-dir",
            state.to_str().unwrap(),
            "--picoclaw-config",
            config.to_str().unwrap(),
            "--yes",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Updated"));
    let patched: Value = serde_json::from_slice(&std::fs::read(config).unwrap()).unwrap();
    assert_eq!(patched["unknown"], true);
    assert_eq!(
        patched["tools"]["mcp"]["servers"]["google_workspace"]["command"],
        "piccolo-gws"
    );
    assert_eq!(
        patched["tools"]["mcp"]["servers"]["google_workspace"]["args"][2],
        state.to_str().unwrap()
    );
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

#[test]
fn mcp_initialize_over_stdio_starts() {
    let output = piccolo_gws_cmd()
        .args(["mcp", "--config-dir", "/tmp/piccolo-gws-test"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(
                b"Content-Length: 46\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}",
            )?;
            child.wait_with_output()
        })
        .unwrap();
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Content-Length:"));
    assert!(stdout.contains("piccolo-gws"));
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
