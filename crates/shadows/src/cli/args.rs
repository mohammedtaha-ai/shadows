//! Parses daemon arguments into explicit, validated startup configuration.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::{self, Config};

#[derive(Parser)]
#[command(name = "shadows")]
pub struct Cli {
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
        /// Absolute path to Claude Code. SHADOWS_HARNESS is also explicit
        /// configuration; a bare name would be resolved through PATH.
        #[arg(long, env = "SHADOWS_HARNESS", value_parser = parse_harness)]
        harness: PathBuf,
        /// Absolute path to the Node.js runtime used for the ACP adapter.
        #[arg(long, value_parser = parse_harness)]
        node: PathBuf,
        /// Absolute entry point of the installed, pinned ACP adapter.
        #[arg(long, value_parser = parse_harness)]
        adapter: PathBuf,
        /// Write debug logging to stderr and a file under the DB directory.
        #[arg(long)]
        debug: bool,
        /// Allowed browser origin. Repeat to replace both default Vite origins.
        #[arg(
            long = "allow-origin",
            value_name = "ORIGIN",
            value_parser = parse_origin,
            default_values = config::DEFAULT_ALLOWED_ORIGINS,
        )]
        allow_origin: Vec<String>,
    },
}

fn parse_origin(raw: &str) -> Result<String, String> {
    config::allowed_origin(raw).map_err(|e| e.to_string())
}

/// Refuse a bare or relative path before attempting any child process.
fn parse_harness(raw: &str) -> Result<PathBuf, String> {
    config::harness_path(std::path::Path::new(raw)).map_err(|e| e.to_string())
}

impl Cli {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Serve {
                db,
                bind,
                harness,
                node,
                adapter,
                debug,
                allow_origin,
            } => {
                let data_dir = config::data_dir(&db);
                // Held until serve returns, so the file log is flushed.
                let log = crate::tracing::init(self.verbose, debug.then_some(data_dir.as_path()))?;
                super::serve(Config {
                    db_path: db,
                    bind,
                    node_path: node,
                    adapter_path: adapter,
                    harness_path: harness,
                    debug_log: log.as_ref().map(|l| l.path().to_path_buf()),
                    allowed_origins: allow_origin,
                })
                .await
            }
        }
    }
}
