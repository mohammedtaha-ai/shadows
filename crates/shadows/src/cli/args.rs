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
        /// JSON file with AYLUL's home_root section.
        #[arg(long, value_parser = parse_harness)]
        aylul_config: Option<PathBuf>,
        /// Private launch receipt for the desktop runtime.
        #[arg(long, requires = "desktop_configuration", value_parser = parse_record)]
        desktop_record: Option<PathBuf>,
        #[arg(long, requires = "desktop_record", value_parser = parse_configuration)]
        desktop_configuration: Option<String>,
    },
}

fn parse_record(raw: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err("desktop record path must be absolute".into())
    }
}
fn parse_configuration(raw: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(raw)
        .map(|id| id.to_string())
        .map_err(|_| "configuration must be a UUID".into())
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
                aylul_config,
                desktop_record,
                desktop_configuration,
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
                    aylul_config,
                    desktop_launch: desktop_record.zip(desktop_configuration).map(
                        |(record_path, configuration_id)| crate::desktop_runtime::DesktopLaunch {
                            record_path,
                            configuration_id,
                        },
                    ),
                })
                .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DIR: &str = env!("CARGO_MANIFEST_DIR");
    const SERVE: [&str; 8] = [
        "shadows",
        "serve",
        "--harness",
        DIR,
        "--node",
        DIR,
        "--adapter",
        DIR,
    ];

    #[test]
    fn serve_accepts_an_aylul_config_path() {
        let cli = Cli::try_parse_from(SERVE.iter().chain(&["--aylul-config", DIR])).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Serve {
                aylul_config: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn serve_without_the_flag_still_parses() {
        let cli = Cli::try_parse_from(SERVE).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Serve {
                aylul_config: None,
                ..
            }
        ));
    }

    #[test]
    fn daemon_is_an_unknown_command() {
        let error = Cli::try_parse_from(["shadows", "daemon"]).err().unwrap();
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }
}
