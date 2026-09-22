use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use shadows::config::Config;

#[derive(Parser)]
#[command(name = "shadows")]
struct Cli {
    #[arg(long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the local daemon.
    Serve {
        #[arg(long, default_value = "shadows.sqlite3")]
        db: PathBuf,
        #[arg(long, default_value = "127.0.0.1:4318")]
        bind: SocketAddr,
        /// Absolute path to the Claude Code executable. Required, with no
        /// default: spec §1.4 resolves the harness from explicit configuration
        /// and never from `PATH`, and a default of `claude` would have been a
        /// PATH lookup in every ordinary run. `SHADOWS_HARNESS` is accepted
        /// because an environment variable is still explicit configuration.
        #[arg(long, env = "SHADOWS_HARNESS", value_parser = parse_harness)]
        harness: PathBuf,
    },
}

/// Rejects a bare program name or a relative path at parse time, so the failure
/// is a startup error naming the rule rather than a turn that silently ran an
/// unknown binary.
fn parse_harness(raw: &str) -> Result<PathBuf, String> {
    shadows::config::harness_path(std::path::Path::new(raw)).map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    shadows::tracing::init(cli.verbose);
    match cli.command {
        Commands::Serve { db, bind, harness } => {
            shadows::cli::serve(Config {
                db_path: db,
                bind,
                harness_path: harness,
            })
            .await
        }
    }
}
