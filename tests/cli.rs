use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn describe_json_cli_outputs_descriptor() {
    let output = Command::cargo_bin("piccolo-gws")
        .unwrap()
        .args(["describe", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
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
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().join("state");
    let config = tmp.path().join("config.json");
    std::fs::write(&config, r#"{"unknown":true}"#).unwrap();
    Command::cargo_bin("piccolo-gws")
        .unwrap()
        .args([
            "setup",
            "--config-dir",
            state.to_str().unwrap(),
            "--picoclaw-config",
            config.to_str().unwrap(),
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));
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
    let tmp = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("piccolo-gws")
        .unwrap()
        .env("PICCOLO_GWS_BACKEND_BIN", tmp.path().join("missing-gws"))
        .args([
            "status",
            "--config-dir",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
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
    let tmp = tempfile::tempdir().unwrap();
    let fake = tmp.path().join("gws-fake");
    std::fs::write(
        &fake,
        r#"#!/bin/sh
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then exit 0; fi
printf '[]\n'
exit 0
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();
    let output = Command::cargo_bin("piccolo-gws")
        .unwrap()
        .env("PICCOLO_GWS_BACKEND_BIN", &fake)
        .args([
            "status",
            "--config-dir",
            tmp.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
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
fn status_requires_json() {
    Command::cargo_bin("piccolo-gws")
        .unwrap()
        .args(["status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("only --json output is supported"));
}

#[test]
fn mcp_initialize_over_stdio_starts() {
    Command::cargo_bin("piccolo-gws")
        .unwrap()
        .args(["mcp", "--config-dir", "/tmp/piccolo-gws-test"])
        .write_stdin(
            "Content-Length: 46\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("Content-Length:"))
        .stdout(predicate::str::contains("piccolo-gws"));
}
