use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "piccolo-gws",
    version,
    about = "Google Workspace integration for Piccolo"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print Piccolo integration metadata.
    Describe {
        /// Emit JSON descriptor.
        #[arg(long)]
        json: bool,
    },
    /// Run interactive setup for the integration.
    Setup {
        /// Integration state directory.
        #[arg(long, default_value = "/root/.picoclaw/integrations/google")]
        config_dir: PathBuf,
        /// PicoClaw config path to patch.
        #[arg(long, default_value = "/root/.picoclaw/config.json")]
        picoclaw_config: PathBuf,
        /// Apply without prompting. Intended for tests/automation only.
        #[arg(long)]
        yes: bool,
    },
    /// Run the stdio MCP server.
    Mcp {
        /// Integration state directory.
        #[arg(long, default_value = "/root/.picoclaw/integrations/google")]
        config_dir: PathBuf,
        /// Restrict tools to read-only operations.
        #[arg(long, default_value_t = true)]
        read_only: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Describe { json } => {
            if !json {
                anyhow::bail!("only --json output is supported");
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&piccolo_gws::descriptor::descriptor())?
            );
        }
        Commands::Setup {
            config_dir,
            picoclaw_config,
            yes,
        } => {
            piccolo_gws::setup::run_setup(&config_dir, &picoclaw_config, yes)?;
        }
        Commands::Mcp {
            config_dir,
            read_only,
        } => {
            piccolo_gws::mcp::serve_stdio(config_dir, read_only)?;
        }
    }
    Ok(())
}
