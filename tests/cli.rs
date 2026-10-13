use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

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
