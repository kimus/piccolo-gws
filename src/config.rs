use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_patch_targets_only_the_own_server_entry() {
        let patch = mcp_server_patch(Path::new("/state"));
        let servers = patch["tools"]["mcp"]["servers"].as_object().unwrap();
        // Piccolo's allowlist pins the same single key; a second server here
        // would make the whole patch be refused.
        assert_eq!(servers.keys().collect::<Vec<_>>(), vec!["google_workspace"]);
        assert_eq!(
            patch["tools"]["mcp"]["servers"]["google_workspace"]["command"],
            "piccolo-gws"
        );
        assert_eq!(
            patch["tools"]["mcp"]["servers"]["google_workspace"]["args"][2],
            "/state"
        );
    }
}
