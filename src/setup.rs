use crate::backend::backend_auth_state;
use crate::config;
use anyhow::{Context, Result};
use serde_json::{json, Value};
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
    /// Why a present credential document is unusable, when it is. "A file exists"
    /// is not the same question as "login can work", and conflating them is what
    /// let a half-written or web-typed client unlock the login path.
    pub problem: Option<String>,
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

/// Validate `client_secret.json` content: a usable **Desktop** client pair.
///
/// A `web` application client is rejected deliberately. It carries a pair and
/// would pass a presence check, but it registers its redirect URI in the console,
/// so authorizing with one fails with `redirect_uri_mismatch` after a browser round
/// trip. "Present" is not the same as "usable by this integration".
pub fn validate_client_secret(raw: &str) -> std::result::Result<(), String> {
    parse_client_document(raw).map(|_| ())
}

/// The stored Desktop OAuth client, read back so a save can merge onto it and so
/// state can report what is actually usable.
#[derive(Debug, Clone, Default)]
pub struct StoredClient {
    pub client_id: String,
    pub client_secret: String,
    pub project_id: String,
    /// A complete pair by shape. Not a claim that the client still works: only a
    /// login or a connectivity check can say that about one revoked upstream.
    pub complete: bool,
    /// Why a present document is unusable, if it is. `None` means nothing is
    /// stored, which is an empty state rather than a problem.
    pub problem: Option<String>,
}

impl StoredClient {
    pub fn has_pair(&self) -> bool {
        !self.client_id.trim().is_empty() && !self.client_secret.trim().is_empty()
    }
}

fn client_secret_at(auth_dir: &Path) -> PathBuf {
    auth_dir.join("client_secret.json")
}

/// Read the stored client from an integration state directory.
pub fn read_stored_client(config_dir: &Path) -> StoredClient {
    read_stored_client_at(&config_dir.join("auth"))
}

/// Read the stored client from its auth directory.
pub fn read_stored_client_at(auth_dir: &Path) -> StoredClient {
    let Ok(raw) = std::fs::read_to_string(client_secret_at(auth_dir)) else {
        return StoredClient::default();
    };
    match parse_client_document(&raw) {
        Ok(stored) => stored,
        Err(problem) => StoredClient {
            problem: Some(problem),
            ..Default::default()
        },
    }
}

fn string_field(obj: &Value, key: &str) -> String {
    obj.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn parse_client_document(raw: &str) -> std::result::Result<StoredClient, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("No content was provided.".to_string());
    }
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("the client secret is not valid JSON ({err})"))?;
    let installed = value.get("installed").and_then(Value::as_object);
    if installed.is_none() && value.get("web").and_then(Value::as_object).is_some() {
        return Err(
            "this integration authorizes as a Desktop app, but these credentials describe a web \
             application client. Create an OAuth client of type \"Desktop app\"."
                .to_string(),
        );
    }
    let Some(installed) = installed else {
        return Err(
            "Could not find an OAuth client id and secret: expected \"client_id\" and \
             \"client_secret\" inside an \"installed\" object."
                .to_string(),
        );
    };
    let obj = Value::Object(installed.clone());
    let stored = StoredClient {
        client_id: string_field(&obj, "client_id"),
        client_secret: string_field(&obj, "client_secret"),
        project_id: string_field(&obj, "project_id"),
        complete: false,
        problem: None,
    };
    if !stored.has_pair() {
        return Err(
            "the OAuth client id and its secret must both be present; only one of them was stored."
                .to_string(),
        );
    }
    Ok(StoredClient {
        complete: true,
        ..stored
    })
}

/// What a save actually did, so the emitted steps describe reality instead of
/// whichever keys happened to appear in the payload.
#[derive(Debug, Default)]
pub struct CredentialSave {
    pub client_id_replaced: bool,
    pub client_secret_replaced: bool,
    pub project_id_cleared: bool,
}

/// Save whichever fields the host sent.
///
/// Absent means keep. That one rule is what lets a secret stay invisible to the
/// host while the other fields are still being saved, and it is why an untouched
/// secret must never be reported as an empty one: empty clears, absent keeps, and
/// only the sender knows which it meant.
///
/// The merged result must be a usable pair, so no save can leave a half-configured
/// client on disk — and the `credentials` fact stays honest because writes and
/// facts run through the same predicate.
pub fn save_client_credentials(
    client_id: Option<&str>,
    client_secret: Option<&str>,
    project_id: Option<&str>,
    config_dir: &Path,
) -> Result<CredentialSave> {
    if client_id.is_none() && client_secret.is_none() && project_id.is_none() {
        anyhow::bail!("no credential fields were supplied");
    }
    let stored = read_stored_client(config_dir);
    let next_id = match client_id {
        Some(v) => v.trim().to_string(),
        None => stored.client_id.clone(),
    };
    let next_secret = match client_secret {
        Some(v) => v.trim().to_string(),
        None => stored.client_secret.clone(),
    };
    let next_project = match project_id {
        Some(v) => v.trim().to_string(),
        None => stored.project_id.clone(),
    };

    // The id and the secret are one credential, not two independent columns.
    // Presence of both is no evidence they belong together, and only this
    // integration can notice that a rotated id arrived without its secret.
    if !stored.client_id.is_empty()
        && !next_id.is_empty()
        && next_id != stored.client_id
        && client_secret.is_none()
    {
        anyhow::bail!(
            "changing the OAuth client also requires entering its secret: the stored secret \
             belongs to the previous client"
        );
    }
    let merged = StoredClient {
        client_id: next_id.clone(),
        client_secret: next_secret.clone(),
        project_id: next_project.clone(),
        complete: false,
        problem: None,
    };
    if !merged.has_pair() {
        anyhow::bail!(
            "an OAuth client id and its secret are both required; nothing was changed, so the \
             stored credentials still work"
        );
    }
    write_pair(&next_id, &next_secret, &next_project, config_dir)?;
    Ok(CredentialSave {
        client_id_replaced: !stored.client_id.is_empty() && next_id != stored.client_id,
        client_secret_replaced: client_secret.is_some() && !stored.client_secret.is_empty(),
        project_id_cleared: project_id.map(str::trim) == Some(""),
    })
}

/// Write the two values Google Cloud Console shows into the shape the delegated
/// backend expects. Both are required: the file is only ever written complete.
pub fn write_client_credentials(
    client_id: &str,
    client_secret: &str,
    project_id: &str,
    config_dir: &Path,
) -> Result<()> {
    if client_id.trim().is_empty() || client_secret.trim().is_empty() {
        anyhow::bail!("both the OAuth client id and secret are required");
    }
    write_pair(client_id, client_secret, project_id, config_dir)
}

fn write_pair(
    client_id: &str,
    client_secret: &str,
    project_id: &str,
    config_dir: &Path,
) -> Result<()> {
    let auth_dir = config_dir.join("auth");
    std::fs::create_dir_all(&auth_dir)
        .with_context(|| format!("creating auth directory {}", auth_dir.display()))?;
    let doc = json!({
        "installed": {
            "client_id": client_id.trim(),
            "client_secret": client_secret.trim(),
            "project_id": project_id.trim(),
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token",
            "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
            "redirect_uris": ["http://localhost"]
        }
    });
    let rendered = serde_json::to_string_pretty(&doc)?;
    // The same predicate the facts use: a write that would produce an unusable
    // document fails here rather than on disk, later, mid-login.
    validate_client_secret(&rendered).map_err(anyhow::Error::msg)?;
    let target = client_secret_at(&auth_dir);
    let tmp = auth_dir.join("client_secret.json.tmp");
    std::fs::write(&tmp, rendered.as_bytes())
        .with_context(|| format!("writing {}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    // Atomic replace: a half-written credential file would break later logins.
    std::fs::rename(&tmp, &target).with_context(|| format!("replacing {}", target.display()))?;
    Ok(())
}

/// Adopt a client secret file staged at an explicit path (service-account style
/// integrations). Never prints the secret.
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
        let _ = std::fs::remove_file(staged);
    }
    Ok(())
}

pub fn capture_snapshot(auth_dir: &Path) -> AuthSnapshot {
    let state = backend_auth_state(&auth_dir.to_path_buf());
    let stored = read_stored_client_at(auth_dir);
    let env_pair = non_empty_env("GOOGLE_WORKSPACE_CLI_CLIENT_ID")
        && non_empty_env("GOOGLE_WORKSPACE_CLI_CLIENT_SECRET");
    // Order matters. A document that exists but cannot be used must NOT be
    // rescued by the backend's "a file is there" view, or a web-typed or
    // half-written client would keep reporting itself configured.
    let credentials = if stored.complete {
        CredentialSource::File
    } else if env_pair {
        CredentialSource::EnvVars
    } else if stored.problem.is_some() {
        CredentialSource::None
    } else if state.client_configured() {
        // The backend also sees credentials this process cannot (its own discovery
        // path, for example), so trust it before reporting "missing".
        CredentialSource::File
    } else {
        CredentialSource::None
    };
    let gws_available = which::which("gws").is_ok();
    AuthSnapshot {
        auth_dir: auth_dir.to_path_buf(),
        gws_available,
        credentials,
        account_authorized: gws_available && state.authenticated(),
        problem: if stored.complete {
            None
        } else {
            stored.problem.clone()
        },
    }
}

fn non_empty_env(key: &str) -> bool {
    std::env::var(key)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

pub fn print_state(snapshot: &AuthSnapshot) {
    println_ok!("Current Google Workspace state:");
    println_ok!(
        "  gws CLI:           {}",
        if snapshot.gws_available {
            "available"
        } else {
            "not found"
        }
    );
    println_ok!("  OAuth client:      {}", snapshot.credentials.label());
    if let Some(problem) = &snapshot.problem {
        // Say what is wrong, not merely that something is missing: the file is
        // there, and "not configured" would send the user looking in the wrong place.
        println_ok!("    note: {problem}");
    }
    println_ok!(
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
    println_ok!("Google Workspace integration setup");
    println_ok!("State directory: {}", config_dir.display());
    config::ensure_state_dir(config_dir)?;

    let auth_dir = config_dir.join("auth");
    std::fs::create_dir_all(&auth_dir)
        .with_context(|| format!("creating auth directory {}", auth_dir.display()))?;

    let snapshot = capture_snapshot(&auth_dir);
    print_state(&snapshot);

    // Show what Piccolo would be asked to register, without touching it.
    let patch = config::mcp_server_patch(config_dir);
    println_ok!("\nConfiguration Piccolo will apply on your approval:");
    println_ok!("{}", serde_json::to_string_pretty(&patch)?);

    if snapshot.access_ready() {
        println_ok!("\nGoogle Workspace access is configured.");
    } else {
        println_ok!("\nRun `piccolo integration setup kimus/piccolo-gws` from an interactive terminal to complete setup.");
        println_ok!(
            "This command never writes config.json; Piccolo applies the patch shown above."
        );
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
            problem: None,
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
    /// A web application client carries a complete pair and would satisfy any
    /// presence check, but it registers its redirect URI in the console, so
    /// authorizing with one ends in `redirect_uri_mismatch` after a browser round
    /// trip. "Present" is not "usable by this integration".
    #[test]
    fn accepts_installed_and_rejects_web_client_shapes() {
        let installed = r#"{"installed":{"client_id":"abc.apps.googleusercontent.com","client_secret":"s3cret"}}"#;
        assert!(validate_client_secret(installed).is_ok());
        let web = r#"{"web":{"client_id":"id","client_secret":"secret"}}"#;
        let err = validate_client_secret(web)
            .expect_err("a web client cannot authorize as a desktop app");
        assert!(
            err.to_lowercase().contains("desktop"),
            "the rejection must name the client type needed, got {err}"
        );
    }

    #[test]
    fn a_stored_half_pair_is_reported_not_silently_accepted() {
        let root = temp_dir("stored-half");
        let auth = root.join("auth");
        std::fs::create_dir_all(&auth).unwrap();
        std::fs::write(
            auth.join("client_secret.json"),
            r#"{"installed":{"client_id":"only-id"}}"#,
        )
        .unwrap();

        let stored = read_stored_client_at(&auth);
        assert!(!stored.complete, "a single field is not a pair");
        assert!(stored.problem.is_some(), "and it must say what is wrong");

        // The snapshot must not call this configured, which is what unlocked
        // login on a stale file during the original report.
        let snapshot = capture_snapshot(&auth);
        assert!(
            !snapshot.credentials.configured(),
            "an unusable document must not report credentials as configured"
        );
        assert!(snapshot.problem.is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn save_keeps_absent_fields_and_clears_empty_ones() {
        let root = temp_dir("save-merge");
        let config_dir = root.join("state");
        write_client_credentials("client-a", "secret-a", "project-a", &config_dir).unwrap();

        // An absent secret keeps the stored one: this is the whole point of
        // "absent means keep", and it is how a host can save the id without ever
        // having seen the secret.
        save_client_credentials(Some("client-a"), None, None, &config_dir).unwrap();
        let stored = read_stored_client(&config_dir);
        assert_eq!(stored.client_secret, "secret-a", "absent must keep");
        assert_eq!(stored.project_id, "project-a");

        // Present-but-empty clears a non-secret field.
        save_client_credentials(None, None, Some(""), &config_dir).unwrap();
        let stored = read_stored_client(&config_dir);
        assert!(stored.project_id.is_empty(), "empty must clear");
        assert_eq!(stored.client_secret, "secret-a");
        assert_eq!(stored.client_id, "client-a");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The id and the secret are one credential. Rotating one without the other
    /// leaves a document that passes every presence check and cannot authorize.
    #[test]
    fn rotating_the_client_id_requires_its_secret() {
        let root = temp_dir("rotate");
        let config_dir = root.join("state");
        write_client_credentials("client-a", "secret-a", "", &config_dir).unwrap();

        let err = save_client_credentials(Some("client-b"), None, None, &config_dir)
            .expect_err("a new id with the old secret is not a usable pair");
        let msg = err.to_string();
        assert!(msg.contains("secret"), "refusal must explain itself: {msg}");

        // The previous document survives: a refused save must not half-apply.
        let stored = read_stored_client(&config_dir);
        assert_eq!(stored.client_id, "client-a");
        assert_eq!(stored.client_secret, "secret-a");

        // Rotating both together works.
        save_client_credentials(Some("client-b"), Some("secret-b"), None, &config_dir).unwrap();
        let stored = read_stored_client(&config_dir);
        assert_eq!(stored.client_id, "client-b");
        assert_eq!(stored.client_secret, "secret-b");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn save_with_no_fields_at_all_is_refused() {
        let root = temp_dir("save-empty");
        let config_dir = root.join("state");
        let err = save_client_credentials(None, None, None, &config_dir)
            .expect_err("nothing supplied must not rewrite the credential file");
        assert!(err.to_string().contains("no credential fields"));
        assert!(!config_dir.join("auth").join("client_secret.json").exists());
        let _ = std::fs::remove_dir_all(&root);
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
            problem: None,
        };
        assert!(!snapshot.access_ready());
        let _ = std::fs::remove_dir_all(&base);
    }
}
