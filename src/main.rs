use anyhow::Result;
use clap::{Parser, Subcommand};
use piccolo_gws::println_ok;
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
    /// Report setup state without changing anything (standalone diagnostic).
    /// Piccolo drives interactive setup through `state` and `call`.
    Setup {
        /// Integration state directory.
        #[arg(long, default_value = piccolo_gws::DEFAULT_CONFIG_DIR)]
        config_dir: PathBuf,
    },
    /// Report integration state facts as JSON (consumed by Piccolo's menu).
    State {
        /// Integration state directory.
        #[arg(long, default_value = piccolo_gws::DEFAULT_CONFIG_DIR)]
        config_dir: PathBuf,
    },
    /// Run one declared action: answers on stdin, JSONL events on stdout.
    Call {
        /// Action id from `describe` output.
        action_id: String,
        /// Integration state directory.
        #[arg(long, default_value = piccolo_gws::DEFAULT_CONFIG_DIR)]
        config_dir: PathBuf,
    },
    /// Run live Google Workspace connectivity checks.
    Status {
        /// Integration state directory.
        #[arg(long, default_value = piccolo_gws::DEFAULT_CONFIG_DIR)]
        config_dir: PathBuf,
        /// Optional PicoClaw config path to validate MCP registration.
        #[arg(long)]
        picoclaw_config: Option<PathBuf>,
        /// Emit JSON status output.
        #[arg(long)]
        json: bool,
    },
    /// List the delegated backend API methods this build depends on.
    /// Used by release packaging to detect gws grammar drift.
    BackendMethods,
    /// Run the stdio MCP server.
    Mcp {
        /// Integration state directory.
        #[arg(long, default_value = piccolo_gws::DEFAULT_CONFIG_DIR)]
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
            println_ok!(
                "{}",
                serde_json::to_string_pretty(&piccolo_gws::descriptor::descriptor())?
            );
        }
        Commands::BackendMethods => {
            for method in piccolo_gws::backend::mapped_methods() {
                println_ok!("{method}");
            }
        }
        Commands::Setup { config_dir } => {
            piccolo_gws::setup::run_setup(&config_dir)?;
        }
        Commands::State { config_dir } => {
            piccolo_gws::actions::run_state(&config_dir)?;
        }
        Commands::Call {
            action_id,
            config_dir,
        } => {
            // An Err reaching here would exit with an anyhow message on stderr and
            // no protocol event, which Piccolo can only read as a crash. The
            // contract is "always end the stream", so say so in the stream.
            let code = match piccolo_gws::actions::run_call(&action_id, &config_dir) {
                Ok(code) => code,
                Err(err) => {
                    let payload = serde_json::json!({
                        "schema": piccolo_gws::actions::EVENT_SCHEMA,
                        "type": "error",
                        "code": "internal",
                        "message": err.to_string(),
                        "remediation": "Rerun the action; if it repeats, report this message.",
                    });
                    println!("{}", serde_json::to_string(&payload).unwrap_or_default());
                    1
                }
            };
            if code != 0 {
                std::process::exit(code);
            }
        }
        Commands::Status {
            config_dir,
            picoclaw_config,
            json,
        } => {
            piccolo_gws::status::run_status(&config_dir, picoclaw_config.as_deref(), json)?;
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
