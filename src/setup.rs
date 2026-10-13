use crate::config;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_setup(config_dir: &Path, picoclaw_config: &Path, assume_yes: bool) -> Result<()> {
    println!("Google Workspace integration setup");
    println!("State directory: {}", config_dir.display());
    config::ensure_state_dir(config_dir)?;

    ensure_google_auth(config_dir, assume_yes)?;

    let mut current = config::read_config(picoclaw_config)?;
    let patch = config::mcp_server_patch(config_dir);
    let before = current.clone();
    config::merge_patch(&mut current, &patch);
    println!(
        "\nPlanned PicoClaw config change:\n{}",
        config::summarize_patch(&before, &current)
    );

    if !assume_yes && !confirm("Apply this config patch? [y/N] ")? {
        println!("Setup cancelled before writing config.");
        return Ok(());
    }

    let backup = config::write_config_with_backup(picoclaw_config, &current)
        .with_context(|| format!("patching {}", picoclaw_config.display()))?;
    println!("Updated {}", picoclaw_config.display());
    println!("Backup: {}", backup.display());
    Ok(())
}

fn ensure_google_auth(config_dir: &Path, assume_yes: bool) -> Result<()> {
    let auth_dir = config_dir.join("auth");
    std::fs::create_dir_all(&auth_dir)
        .with_context(|| format!("creating auth directory {}", auth_dir.display()))?;
    println!("\nAuthentication backend: gws CLI (delegated)");
    println!("Store OAuth/backend state under: {}", auth_dir.display());
    if which::which("gws").is_err() {
        println!("gws executable not found on PATH; the packaged runtime wrapper should provide it in Piccolo images.");
        return Ok(());
    }

    match gws_auth_status(&auth_dir) {
        Ok(true) => {
            println!("gws auth status: ok");
            if assume_yes {
                return Ok(());
            }
            println!("\nGoogle Workspace auth is already configured for this Piccolo workspace.");
            println!("Use delegated setup to change Google Cloud/OAuth configuration, or login to switch/refresh credentials.");
            if !prompt_gws_auth(
                &auth_dir,
                "Reconfigure Google auth? [s]etup / [l]ogin / [n]one: ",
            )? {
                return Ok(());
            }
        }
        Ok(false) => {
            println!("gws auth status: login required");
            println!("\nGoogle Workspace is not authenticated for this Piccolo workspace.");
            println!("Setup can delegate to the packaged gws CLI. Use 'setup' for first-time Google Cloud project/API/OAuth configuration, or 'login' if credentials are already configured.");
            if assume_yes {
                println!("Skipping delegated gws auth in --yes mode. Run `piccolo integration setup gws` interactively to complete Google login.");
                return Ok(());
            }
            if !prompt_gws_auth(&auth_dir, "Run gws auth now? [s]etup / [l]ogin / [n]one: ")? {
                println!("Skipping Google auth. MCP tools will report login_required until auth is completed.");
                return Ok(());
            }
        }
        Err(err) => {
            println!("Could not check gws auth status: {err}");
            println!("\nSetup can delegate to the packaged gws CLI. Use 'setup' for first-time Google Cloud project/API/OAuth configuration, or 'login' if credentials are already configured.");
            if assume_yes {
                println!("Skipping delegated gws auth in --yes mode. Run `piccolo integration setup gws` interactively to complete Google login.");
                return Ok(());
            }
            if !prompt_gws_auth(&auth_dir, "Run gws auth now? [s]etup / [l]ogin / [n]one: ")? {
                println!("Skipping Google auth.");
                return Ok(());
            }
        }
    }

    match gws_auth_status(&auth_dir) {
        Ok(true) => println!("gws auth status: ok"),
        Ok(false) => println!("gws auth status still requires attention."),
        Err(err) => println!("Could not re-check gws auth status: {err}"),
    }
    Ok(())
}

fn prompt_gws_auth(auth_dir: &Path, prompt: &str) -> Result<bool> {
    loop {
        print!("{prompt}");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim().to_ascii_lowercase().as_str() {
            "s" | "setup" => {
                if which::which("gcloud").is_err() {
                    println!("\n`gws auth setup` requires the Google Cloud SDK (`gcloud`).");
                    println!("Piccolo's default runtime image does not package it because it adds hundreds of MB to every workspace.");
                    println!("Use Google Cloud Console or a host-side gcloud install to create/configure the OAuth client, then run login here.");
                    if confirm("Run `gws auth login` instead? [y/N] ")? {
                        run_gws_auth(auth_dir, "login")?;
                        return Ok(true);
                    }
                    return Ok(false);
                }
                run_gws_auth(auth_dir, "setup")?;
                return Ok(true);
            }
            "l" | "login" => {
                run_gws_auth(auth_dir, "login")?;
                return Ok(true);
            }
            "n" | "no" | "" => return Ok(false),
            _ => println!("Please enter s, l, or n."),
        }
    }
}

fn gws_auth_status(auth_dir: &Path) -> Result<bool> {
    let output = gws_command(auth_dir)
        .arg("auth")
        .arg("status")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("running gws auth status")?;
    Ok(output.status.success())
}

fn run_gws_auth(auth_dir: &Path, subcommand: &str) -> Result<()> {
    println!("\nRunning: gws auth {subcommand}");
    let status = gws_command(auth_dir)
        .arg("auth")
        .arg(subcommand)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("running gws auth {subcommand}"))?;
    if !status.success() {
        anyhow::bail!("gws auth {subcommand} exited {status}");
    }
    Ok(())
}

fn gws_command(auth_dir: &Path) -> Command {
    let mut cmd = Command::new("gws");
    cmd.env("GOOGLE_WORKSPACE_CLI_CONFIG_DIR", auth_dir)
        // Older/internal wrappers may also honor this; setting both is harmless.
        .env("GWS_CONFIG_DIR", auth_dir);
    cmd
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
