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
        /// Path to the Claude Code executable. Spec §1.4 forbids PATH lookup.
        #[arg(long, default_value = "claude")]
        harness: PathBuf,
    },
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
