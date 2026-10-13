use crate::config;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub fn run_setup(config_dir: &Path, picoclaw_config: &Path, assume_yes: bool) -> Result<()> {
    println!("Google Workspace integration setup");
    println!("State directory: {}", config_dir.display());
    config::ensure_state_dir(config_dir)?;

    print_auth_guidance(config_dir);

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

fn print_auth_guidance(config_dir: &Path) {
    println!("\nAuthentication backend: gws CLI (delegated)");
    if which::which("gws").is_ok() {
        let status = Command::new("gws").arg("auth").arg("status").output();
        match status {
            Ok(output) if output.status.success() => println!("gws auth status: ok"),
            Ok(output) => println!(
                "gws auth status needs attention: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
            Err(err) => println!("Could not check gws auth status: {err}"),
        }
    } else {
        println!("gws executable not found on PATH; the packaged runtime wrapper should provide it in Piccolo images.");
    }
    println!(
        "Store OAuth/backend state under: {}",
        config_dir.join("auth").display()
    );
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
