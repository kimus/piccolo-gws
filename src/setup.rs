use crate::backend::backend_auth_state;
use crate::config;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Where OAuth client credentials currently come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialSource {
    /// `client_secret.json` inside the integration auth directory.
    File,
    /// `GOOGLE_WORKSPACE_CLI_CLIENT_ID` + `GOOGLE_WORKSPACE_CLI_CLIENT_SECRET`.
    EnvVars,
    /// No usable client credentials; login/refresh cannot run.
    None,
}

impl CredentialSource {
    pub fn configured(&self) -> bool {
        !matches!(self, CredentialSource::None)
    }

    fn label(&self) -> &'static str {
        match self {
            CredentialSource::File => "client_secret.json",
            CredentialSource::EnvVars => "environment variables",
            CredentialSource::None => "missing",
        }
    }
}

/// Point-in-time view of what the runtime can actually do.
#[derive(Debug, Clone)]
pub struct AuthSnapshot {
    pub auth_dir: PathBuf,
    pub gws_available: bool,
    pub credentials: CredentialSource,
    pub account_authorized: bool,
}

impl AuthSnapshot {
    pub fn access_ready(&self) -> bool {
        self.credentials.configured() && self.account_authorized
    }
}

/// Pure credential-source resolution so detection stays unit-testable.
pub fn detect_credential_source(
    file_exists: bool,
    env_id_set: bool,
    env_secret_set: bool,
) -> CredentialSource {
    if file_exists {
        return CredentialSource::File;
    }
    if env_id_set && env_secret_set {
        return CredentialSource::EnvVars;
    }
    CredentialSource::None
}

/// Reason login cannot run yet, if it cannot.
pub fn login_block_reason(snapshot: &AuthSnapshot) -> Option<&'static str> {
    if snapshot.credentials.configured() {
        None
    } else {
        Some("Login requires OAuth credentials first. Import credentials from Google Cloud Console first.")
    }
}

/// Validate `client_secret.json` content without trusting a specific shape.
pub fn validate_client_secret(raw: &str) -> std::result::Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("No content was provided.".to_string());
    }
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("the client secret is not valid JSON ({err})"))?;
    let candidates: Vec<&Value> = vec![
        &value,
        value.get("installed").unwrap_or(&Value::Null),
        value.get("web").unwrap_or(&Value::Null),
    ];
    let has_pair = |obj: &Value| -> bool {
        let non_empty = |key: &str| {
            obj.get(key)
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        };
        non_empty("client_id") && non_empty("client_secret")
    };
    if candidates.iter().any(|obj| has_pair(obj)) {
        return Ok(());
    }
    Err(
        "Could not find an OAuth client id and secret. Expected a client_secret.json containing \
         \"client_id\" and \"client_secret\" (often nested under \"installed\" or \"web\")."
            .to_string(),
    )
}

/// Adopt a client secret file Piccolo staged from the host.
///
/// Piccolo owns the mount that makes a host file visible here and copies it with
/// owner-only permissions; this validates it and moves it into the integration
/// auth directory. The secret value is never printed.
pub fn store_client_secret(staged: &Path, config_dir: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(staged)
        .with_context(|| format!("unreadable client secret file {}", staged.display()))?;
    validate_client_secret(&raw).map_err(anyhow::Error::msg)?;
    let auth_dir = config_dir.join("auth");
    std::fs::create_dir_all(&auth_dir)
        .with_context(|| format!("creating auth directory {}", auth_dir.display()))?;
    let target = auth_dir.join("client_secret.json");
    if staged != target {
        std::fs::write(&target, raw.as_bytes())
            .with_context(|| format!("writing {}", target.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600));
        }
        // The staged copy is redundant once adopted; remove it so secrets do
        // not linger at a second path.
        let _ = std::fs::remove_file(staged);
    }
    Ok(())
}

pub fn capture_snapshot(auth_dir: &Path) -> AuthSnapshot {
    let state = backend_auth_state(&auth_dir.to_path_buf());
    let detected = detect_credential_source(
        auth_dir.join("client_secret.json").is_file(),
        non_empty_env("GOOGLE_WORKSPACE_CLI_CLIENT_ID"),
        non_empty_env("GOOGLE_WORKSPACE_CLI_CLIENT_SECRET"),
    );
    // The backend also sees credentials this process cannot (for example a
    // discovery path of its own), so trust it before reporting "missing".
    let credentials = match detected {
        CredentialSource::None if state.client_configured() => CredentialSource::File,
        other => other,
    };
    let gws_available = which::which("gws").is_ok();
    AuthSnapshot {
        auth_dir: auth_dir.to_path_buf(),
        gws_available,
        credentials,
        account_authorized: gws_available && state.authenticated(),
    }
}

fn non_empty_env(key: &str) -> bool {
    std::env::var(key)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

pub fn print_state(snapshot: &AuthSnapshot) {
    println!("Current Google Workspace state:");
    println!(
        "  gws CLI:           {}",
        if snapshot.gws_available {
            "available"
        } else {
            "not found"
        }
    );
    println!("  OAuth client:      {}", snapshot.credentials.label());
    println!(
        "  Google account:    {}",
        if !snapshot.credentials.configured() && snapshot.account_authorized {
            "token present but unusable without credentials"
        } else if snapshot.account_authorized {
            "authorized"
        } else {
            "not authorized"
        }
    );
}

/// Standalone, non-interactive report.
///
/// Piccolo drives real setup through `piccolo-gws call <action-id>`; this
/// command exists so the binary remains usable from a shell or container
/// without an orchestrator. It never writes PicoClaw's `config.json`: the
/// integration proposes that change and Piccolo applies it.
pub fn run_setup(config_dir: &Path) -> Result<()> {
    println!("Google Workspace integration setup");
    println!("State directory: {}", config_dir.display());
    config::ensure_state_dir(config_dir)?;

    let auth_dir = config_dir.join("auth");
    std::fs::create_dir_all(&auth_dir)
        .with_context(|| format!("creating auth directory {}", auth_dir.display()))?;

    let snapshot = capture_snapshot(&auth_dir);
    print_state(&snapshot);

    // Show what Piccolo would be asked to register, without touching it.
    let patch = config::mcp_server_patch(config_dir);
    println!("\nConfiguration Piccolo will apply on your approval:");
    println!("{}", serde_json::to_string_pretty(&patch)?);

    if snapshot.access_ready() {
        println!("\nGoogle Workspace access is configured.");
    } else {
        println!("\nRun `piccolo integration setup kimus/piccolo-gws` from an interactive terminal to complete setup.");
        println!("This command never writes config.json; Piccolo applies the patch shown above.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("piccolo-gws-setup-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn credential_detection_prefers_file_then_env_pair() {
        assert_eq!(
            detect_credential_source(true, false, false),
            CredentialSource::File
        );
        assert_eq!(
            detect_credential_source(false, true, true),
            CredentialSource::EnvVars
        );
        assert_eq!(
            detect_credential_source(false, true, false),
            CredentialSource::None
        );
    }

    #[test]
    fn login_blocked_without_credentials() {
        let base = temp_dir("login-block");
        let blocked = AuthSnapshot {
            auth_dir: base.clone(),
            gws_available: true,
            credentials: CredentialSource::None,
            account_authorized: false,
        };
        assert!(login_block_reason(&blocked).is_some());
        let ready = AuthSnapshot {
            credentials: CredentialSource::File,
            ..blocked.clone()
        };
        assert!(login_block_reason(&ready).is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn accepts_installed_and_web_client_shapes() {
        let installed = r#"{"installed":{"client_id":"abc.apps.googleusercontent.com","client_secret":"s3cret"}}"#;
        assert!(validate_client_secret(installed).is_ok());
        let web = r#"{"web":{"client_id":"id","client_secret":"secret"}}"#;
        assert!(validate_client_secret(web).is_ok());
    }

    #[test]
    fn rejects_invalid_or_incomplete_client_secret() {
        assert!(validate_client_secret("").is_err());
        assert!(validate_client_secret("not json").is_err());
        assert!(validate_client_secret(r#"{"installed":{"client_id":"id"}}"#).is_err());
    }

    #[test]
    fn store_client_secret_adopts_staged_file_and_removes_it() {
        let root = temp_dir("store");
        let staged = root.join("staged.json");
        std::fs::write(
            &staged,
            r#"{"installed":{"client_id":"id","client_secret":"secret"}}"#,
        )
        .unwrap();
        let config_dir = root.join("state");

        store_client_secret(&staged, &config_dir).unwrap();

        let target = config_dir.join("auth").join("client_secret.json");
        assert!(target.is_file());
        assert!(
            !staged.exists(),
            "the staged duplicate must not linger with a secret in it"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn store_client_secret_rejects_garbage_without_adopting() {
        let root = temp_dir("store-bad");
        let staged = root.join("staged.json");
        std::fs::write(&staged, "definitely not json").unwrap();
        let config_dir = root.join("state");

        assert!(store_client_secret(&staged, &config_dir).is_err());
        assert!(!config_dir.join("auth").join("client_secret.json").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn token_without_credentials_is_reported_as_unusable() {
        let base = temp_dir("snapshot");
        let snapshot = AuthSnapshot {
            auth_dir: base.clone(),
            gws_available: true,
            credentials: CredentialSource::None,
            account_authorized: true,
        };
        assert!(!snapshot.access_ready());
        let _ = std::fs::remove_dir_all(&base);
    }
}
