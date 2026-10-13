use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn ensure_state_dir(config_dir: &Path) -> Result<()> {
    fs::create_dir_all(config_dir.join("auth"))
        .with_context(|| format!("creating state directory {}", config_dir.display()))?;
    let marker = config_dir.join("backend.json");
    if !marker.exists() {
        fs::write(
            &marker,
            serde_json::to_vec_pretty(&json!({
                "backend": "gws",
                "auth": { "status": "unknown", "stateDir": config_dir.join("auth") }
            }))?,
        )?;
    }
    Ok(())
}

pub fn mcp_server_patch(config_dir: &Path) -> Value {
    json!({
        "tools": {
            "mcp": {
                "servers": {
                    "google_workspace": {
                        "enabled": true,
                        "type": "stdio",
                        "command": "piccolo-gws",
                        "args": ["mcp", "--config-dir", config_dir.to_string_lossy(), "--read-only"]
                    }
                }
            }
        }
    })
}

pub fn merge_patch(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target_map), Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                if patch_value.is_null() {
                    target_map.remove(key);
                } else {
                    merge_patch(
                        target_map.entry(key.clone()).or_insert(Value::Null),
                        patch_value,
                    );
                }
            }
        }
        (target_slot, patch_value) => *target_slot = patch_value.clone(),
    }
}

pub fn read_config(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(json!({}));
    }
    Ok(serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))?)
}

pub fn backup_path(path: &Path) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let file = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("config.json");
    path.with_file_name(format!("{file}.bak.{ts}"))
}

pub fn write_config_with_backup(path: &Path, next: &Value) -> Result<PathBuf> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let backup = backup_path(path);
    if path.exists() {
        fs::copy(path, &backup).with_context(|| format!("backing up {}", path.display()))?;
    } else {
        fs::write(&backup, b"{}\n")?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(next)?)?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(backup)
}

pub fn summarize_patch(current: &Value, next: &Value) -> String {
    let mut lines = Vec::new();
    let before = current.pointer("/tools/mcp/servers/google_workspace");
    let after = next.pointer("/tools/mcp/servers/google_workspace");
    match (before, after) {
        (None, Some(_)) => lines.push("+ register tools.mcp.servers.google_workspace".to_string()),
        (Some(b), Some(a)) if b != a => {
            lines.push("~ update tools.mcp.servers.google_workspace".to_string())
        }
        (Some(_), Some(_)) => {
            lines.push("= tools.mcp.servers.google_workspace already configured".to_string())
        }
        _ => lines.push("no google_workspace server change".to_string()),
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_patch_preserves_unknown_keys() {
        let mut current = json!({"user": {"theme": "dark"}, "tools": {"other": true}});
        merge_patch(&mut current, &mcp_server_patch(Path::new("/state")));
        assert_eq!(current["user"]["theme"], "dark");
        assert_eq!(current["tools"]["other"], true);
        assert_eq!(
            current["tools"]["mcp"]["servers"]["google_workspace"]["command"],
            "piccolo-gws"
        );
        assert_eq!(
            current["tools"]["mcp"]["servers"]["google_workspace"]["args"][2],
            "/state"
        );
    }

    #[test]
    fn backup_created_before_write() {
        let tmp = test_temp_dir("backup-created");
        let config = tmp.join("config.json");
        fs::write(&config, "{\"keep\":true}\n").unwrap();
        let backup = write_config_with_backup(&config, &json!({"next": true})).unwrap();
        assert!(backup.exists());
        assert_eq!(read_config(&config).unwrap()["next"], true);
    }

    fn test_temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "piccolo-gws-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
