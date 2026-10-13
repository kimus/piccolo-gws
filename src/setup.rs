use crate::config;
use anyhow::{Context, Result};
use inquire::{Confirm, Select};
use serde_json::Value;
use std::io::{self, IsTerminal};
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
        Some("Login requires OAuth credentials first. Choose Import credentials or Manual instructions.")
    }
}

/// Validate pasted `client_secret.json` content without trusting a specific shape.
pub fn validate_client_secret(raw: &str) -> std::result::Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("No content was pasted.".to_string());
    }
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("The pasted text is not valid JSON ({err})."))?;
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

pub fn run_setup(config_dir: &Path, picoclaw_config: &Path, assume_yes: bool) -> Result<()> {
    println!("Google Workspace integration setup");
    println!("State directory: {}", config_dir.display());
    config::ensure_state_dir(config_dir)?;

    let auth_dir = config_dir.join("auth");
    std::fs::create_dir_all(&auth_dir)
        .with_context(|| format!("creating auth directory {}", auth_dir.display()))?;

    if assume_yes {
        let snapshot = capture_snapshot(&auth_dir);
        print_state(&snapshot);
        if !snapshot.access_ready() {
            println!("Authentication is still incomplete; run setup interactively to finish it.");
            println!("  piccolo integration setup kimus/piccolo-gws");
        }
    } else if io::stdin().is_terminal() {
        guided_menu(&auth_dir)?;
    } else {
        println!("No terminal attached; skipping interactive Google auth setup.");
        println!("Re-run setup interactively (or with --yes after completing auth) to configure Google access.");
    }

    let mut current = config::read_config(picoclaw_config)?;
    let patch = config::mcp_server_patch(config_dir);
    let before = current.clone();
    config::merge_patch(&mut current, &patch);
    println!(
        "\nPlanned PicoClaw config change:\n{}",
        config::summarize_patch(&before, &current)
    );

    let apply = if assume_yes {
        true
    } else if io::stdin().is_terminal() {
        Confirm::new("Apply this config patch?")
            .with_default(false)
            .prompt()
            .unwrap_or(false)
    } else {
        println!("Skipping config write without confirmation (pass --yes to apply).");
        false
    };

    if !apply {
        println!("Setup cancelled before writing config.");
        return Ok(());
    }

    let backup = config::write_config_with_backup(picoclaw_config, &current)
        .with_context(|| format!("patching {}", picoclaw_config.display()))?;
    println!("Updated {}", picoclaw_config.display());
    println!("Backup: {}", backup.display());
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Guided,
    Import,
    Login,
    Manual,
    Skip,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Action::Guided => "Set up Google Workspace access",
            Action::Import => "Import credentials",
            Action::Login => "Login / authorize Google account",
            Action::Manual => "Manual instructions",
            Action::Skip => "Skip",
        };
        f.write_str(label)
    }
}

fn capture_snapshot(auth_dir: &Path) -> AuthSnapshot {
    let state = crate::backend::backend_auth_state(&auth_dir.to_path_buf());
    let detected = detect_credential_source(
        auth_dir.join("client_secret.json").is_file(),
        non_empty_env("GOOGLE_WORKSPACE_CLI_CLIENT_ID"),
        non_empty_env("GOOGLE_WORKSPACE_CLI_CLIENT_SECRET"),
    );
    // The backend also knows about credentials we cannot see from this process
    // (e.g. a credentials file it discovered on its own), so trust it as a
    // fallback before concluding credentials are missing.
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

fn print_state(snapshot: &AuthSnapshot) {
    println!("\nCurrent Google Workspace state:");
    println!("  gws CLI:           {}", yes_no(snapshot.gws_available));
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

fn yes_no(v: bool) -> &'static str {
    if v {
        "available"
    } else {
        "not found"
    }
}

fn guided_menu(auth_dir: &Path) -> Result<()> {
    let mut snapshot = capture_snapshot(auth_dir);
    print_state(&snapshot);
    if !snapshot.gws_available {
        println!("\ngws executable not found on PATH; the packaged runtime should provide it.");
        return Ok(());
    }

    loop {
        let items = vec![
            Action::Guided,
            Action::Import,
            Action::Login,
            Action::Manual,
            Action::Skip,
        ];
        let chosen = Select::new("What do you want to do?", items)
            .with_help_message(
                "Credentials identify the app; login authorizes your Google account.",
            )
            .prompt()
            .map_err(|err| anyhow::anyhow!("setup selection failed: {err}"))?;

        match chosen {
            Action::Guided => guided_flow(auth_dir, &mut snapshot)?,
            Action::Import => import_credentials(auth_dir, &mut snapshot)?,
            Action::Login => authorize_account(auth_dir, &mut snapshot)?,
            Action::Manual => print_manual_instructions(auth_dir),
            Action::Skip => {
                println!("Skipping Google auth. Workspace tools will report login-required until it is completed.");
                return Ok(());
            }
        }

        print_state(&snapshot);
        if snapshot.access_ready() {
            println!("\nGoogle Workspace access is configured.");
            return Ok(());
        }
        if !Confirm::new("Continue with more auth options?")
            .with_default(true)
            .prompt()
            .unwrap_or(false)
        {
            return Ok(());
        }
    }
}

fn guided_flow(auth_dir: &Path, snapshot: &mut AuthSnapshot) -> Result<()> {
    if !snapshot.credentials.configured() {
        println!("\nNo OAuth client credentials are configured for this workspace.");
        println!("OAuth client credentials identify the application and are created once in Google Cloud Console.");
        println!("Import a downloaded client_secret.json here, or view the manual instructions.");
        offer_fallback(auth_dir, snapshot)?;
    }

    if !snapshot.credentials.configured() {
        println!("Still no OAuth client credentials, so Google account authorization cannot run.");
        return Ok(());
    }

    if snapshot.account_authorized {
        if confirm_default_no("Re-authorize / switch Google account?") {
            authorize_account(auth_dir, snapshot)?;
        }
        return Ok(());
    }

    println!("\nNext: authorize your Google account in the browser.");
    if confirm_default_yes("Open Google login now?") {
        authorize_account(auth_dir, snapshot)?;
    } else {
        println!("Authorization skipped; tools will report login-required until it is completed.");
    }
    Ok(())
}

fn offer_fallback(auth_dir: &Path, snapshot: &mut AuthSnapshot) -> Result<()> {
    let items = vec![Action::Import, Action::Manual, Action::Skip];
    let chosen = Select::new("How do you want to proceed?", items)
        .prompt()
        .map_err(|err| anyhow::anyhow!("setup selection failed: {err}"))?;
    match chosen {
        Action::Import => import_credentials(auth_dir, snapshot)?,
        Action::Manual => print_manual_instructions(auth_dir),
        _ => println!("Auth left for later; rerun setup when ready."),
    }
    Ok(())
}

fn import_credentials(auth_dir: &Path, snapshot: &mut AuthSnapshot) -> Result<()> {
    let target = auth_dir.join("client_secret.json");
    if target.is_file() && !confirm_default_no("A client_secret.json already exists. Replace it?") {
        return Ok(());
    }
    println!("Paste the full client_secret.json content, then submit an empty line ('.' on its own line also ends input).");
    let pasted = read_pasted_json()?;
    if let Err(reason) = validate_client_secret(&pasted) {
        println!("Not saved: {reason}");
        return Ok(());
    }
    std::fs::write(&target, pasted.as_bytes())
        .with_context(|| format!("writing {}", target.display()))?;
    println!("Saved OAuth client credentials to {}", target.display());
    *snapshot = capture_snapshot(auth_dir);
    if snapshot.credentials.configured() && !snapshot.account_authorized {
        authorize_account(auth_dir, snapshot)?;
    }
    Ok(())
}

fn read_pasted_json() -> Result<String> {
    let mut buf = String::new();
    let mut line = String::new();
    let stdin = io::stdin();
    loop {
        line.clear();
        let read = stdin
            .read_line(&mut line)
            .context("reading pasted credentials")?;
        if read == 0 {
            break;
        }
        if line.trim() == "." {
            break;
        }
        buf.push_str(&line);
    }
    Ok(buf)
}

fn authorize_account(auth_dir: &Path, snapshot: &mut AuthSnapshot) -> Result<()> {
    if let Some(reason) = login_block_reason(snapshot) {
        println!("\n{reason}");
        return Ok(());
    }
    println!("\nRunning: gws auth login (OAuth2 in the browser)");
    println!("If the browser does not open automatically, open the printed URL in the container host's browser.");
    println!("A localhost redirect may need a port forward from the container to the host.");
    match run_gws_auth(auth_dir, "login") {
        Ok(()) => println!("Google account authorization completed."),
        Err(err) => println!("Login did not complete: {err}"),
    }
    *snapshot = capture_snapshot(auth_dir);
    Ok(())
}

fn print_manual_instructions(auth_dir: &Path) {
    println!("\nManual Google Workspace credential setup:");
    println!(
        "  1. In Google Cloud Console, create or select a project and enable the APIs you need"
    );
    println!("     (Gmail, Calendar, Drive).");
    println!(
        "  2. Create an OAuth client id of type 'Desktop app' and download client_secret.json."
    );
    println!(
        "  3. Place that file at the integration auth directory so 'gws auth login' can use it:"
    );
    println!(
        "       container: {}",
        auth_dir.join("client_secret.json").display()
    );
    if let Some(host) = host_side_path(auth_dir) {
        println!("       host:      {host}");
    }
    println!("  4. Alternatively provide credentials via GOOGLE_WORKSPACE_CLI_CLIENT_ID and");
    println!("     GOOGLE_WORKSPACE_CLI_CLIENT_SECRET in the container environment.");
    println!("  5. Rerun setup and choose 'Login / authorize Google account'.");
}

fn host_side_path(auth_dir: &Path) -> Option<String> {
    let container_prefix = "/root/.piccolo/integrations/";
    let text = auth_dir.to_string_lossy();
    text.strip_prefix(container_prefix)
        .map(|rest| format!("integrations/{rest}"))
}

fn confirm_default_yes(prompt: &str) -> bool {
    Confirm::new(prompt)
        .with_default(true)
        .prompt()
        .unwrap_or(false)
}

fn confirm_default_no(prompt: &str) -> bool {
    Confirm::new(prompt)
        .with_default(false)
        .prompt()
        .unwrap_or(false)
}

fn run_gws_auth(auth_dir: &Path, subcommand: &str) -> Result<()> {
    println!("\nRunning: gws auth {subcommand}");
    let status = gws_command(auth_dir)
        .arg("auth")
        .arg(subcommand)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("running gws auth {subcommand}"))?;
    if !status.success() {
        anyhow::bail!("gws auth {subcommand} exited {status}");
    }
    Ok(())
}

fn gws_command(auth_dir: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new("gws");
    cmd.env("GOOGLE_WORKSPACE_CLI_CONFIG_DIR", auth_dir)
        // Older/internal wrappers may also honor this; setting both is harmless.
        .env("GWS_CONFIG_DIR", auth_dir);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_client_secret_file_first() {
        let dir = std::env::temp_dir().join(format!("piccolo-gws-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("client_secret.json"), "{}").unwrap();
        assert_eq!(
            detect_credential_source(dir.join("client_secret.json").is_file(), false, false),
            CredentialSource::File
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn env_pair_counts_as_credentials() {
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
        let snapshot = snapshot_with(CredentialSource::None);
        assert!(login_block_reason(&snapshot).is_some());
        let snapshot = snapshot_with(CredentialSource::File);
        assert!(login_block_reason(&snapshot).is_none());
    }

    #[test]
    fn accepts_installed_and_web_shapes() {
        let installed = r#"{"installed":{"client_id":"abc.apps.googleusercontent.com","client_secret":"s3cret"}}"#;
        assert!(validate_client_secret(installed).is_ok());
        let web = r#"{"web":{"client_id":"id","client_secret":"secret"}}"#;
        assert!(validate_client_secret(web).is_ok());
    }

    #[test]
    fn rejects_invalid_or_incomplete_input() {
        assert!(validate_client_secret("").is_err());
        assert!(validate_client_secret("not json").is_err());
        let missing = r#"{"installed":{"client_id":"id"}}"#;
        assert!(validate_client_secret(missing).is_err());
    }

    #[test]
    fn host_path_translation_for_known_container_prefix() {
        let auth = Path::new("/root/.piccolo/integrations/kimus/piccolo-gws/auth");
        assert_eq!(
            host_side_path(auth).as_deref(),
            Some("integrations/kimus/piccolo-gws/auth")
        );
        assert_eq!(host_side_path(Path::new("/somewhere/else")), None);
    }

    fn snapshot_with(credentials: CredentialSource) -> AuthSnapshot {
        AuthSnapshot {
            auth_dir: PathBuf::from("/tmp/piccolo-gws-test"),
            gws_available: true,
            credentials,
            account_authorized: false,
        }
    }
}
