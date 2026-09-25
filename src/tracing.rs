//! One job: tracing subscriber setup.
//!
//! Spec §8.7. Durable transition logs are emitted only after their transaction
//! commits; that discipline lives at the call sites, not here. What lives here
//! is where lines go and at which level: stderr always, and in debug mode a
//! plain-text file as well, so a person can read back what a run did after it
//! ended.

use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::fmt::format::{DefaultFields, Writer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, filter::filter_fn, fmt};

/// The log file debug mode writes. Hold it for the daemon's whole lifetime:
/// the file is written by a background worker, and dropping this is what
/// flushes the lines still queued — so the last lines of a run, the ones that
/// explain how it ended, are only on disk once this is dropped.
pub struct DebugLog {
    path: PathBuf,
    _guard: WorkerGuard,
}

impl DebugLog {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Installs the global subscriber. Call once per process.
///
/// `debug_data_dir` is debug mode (`shadows serve --debug`): the default
/// filter becomes `shadows=debug`, and lines also go to a new file under
/// `<data dir>/logs/`, one per daemon start, named for the UTC start time and
/// the process id. Without it: `shadows=info`, stderr only. `verbose` raises
/// the stderr level without writing a file. `RUST_LOG`, when set, overrides
/// the level in every mode, except ACP request logs, which are capped at INFO
/// because their DEBUG fields can contain a live MCP bearer.
///
/// Diagnostics go to stderr, never stdout. Spec §1.0 gives stdout one job —
/// printing the local address `shadows serve` binds (and, in debug mode, the
/// log file's path after it) — and startup recovery logs before that print.
pub fn init(verbose: bool, debug_data_dir: Option<&Path>) -> anyhow::Result<Option<DebugLog>> {
    let default = if verbose || debug_data_dir.is_some() {
        "shadows=debug,info"
    } else {
        "shadows=info,warn"
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));

    let (file, log) = match debug_data_dir {
        Some(dir) => {
            let (writer, log) = open_log_file(dir)?;
            let layer = fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .fmt_fields(PlainFields(DefaultFields::new()))
                .with_writer(writer);
            (Some(layer), Some(log))
        }
        None => (None, None),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(filter_fn(acp_log_allowed))
        .with(fmt::layer().with_target(true).with_writer(std::io::stderr))
        .with(file)
        .try_init()?;
    Ok(log)
}

/// ACP 2.2.0 logs whole outgoing requests at DEBUG, including mcpServers'
/// Authorization header. Apply this to both outputs after RUST_LOG is parsed.
fn acp_log_allowed(meta: &tracing::Metadata<'_>) -> bool {
    !meta.target().starts_with("agent_client_protocol") || *meta.level() <= tracing::Level::INFO
}

/// The file layer's field formatter: the default one, under its own type.
///
/// A span's fields are formatted once and cached on the span, keyed by the
/// formatter's *type*. With two layers sharing `DefaultFields`, whichever
/// formats a span first fixes its text for both — so the stderr layer's colour
/// codes appeared inside every span in the file. A distinct type gets its own
/// cache entry, formatted without colour.
struct PlainFields(DefaultFields);

impl<'writer> FormatFields<'writer> for PlainFields {
    fn format_fields<R: RecordFields>(
        &self,
        writer: Writer<'writer>,
        fields: R,
    ) -> std::fmt::Result {
        self.0.format_fields(writer, fields)
    }
}

/// A new file under `<data_dir>/logs/`. `tracing-appender` owns the writing;
/// `Rotation::NEVER` because a run gets its own file by name, not by date.
fn open_log_file(data_dir: &Path) -> anyhow::Result<(NonBlocking, DebugLog)> {
    let dir = data_dir.join("logs");
    std::fs::create_dir_all(&dir)?;
    let started = time::OffsetDateTime::now_utc().format(time::macros::format_description!(
        "[year][month][day]T[hour][minute][second]Z"
    ))?;
    let stem = format!("shadows-{started}-{}", std::process::id());
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::NEVER)
        .filename_prefix(&stem)
        .filename_suffix("log")
        .build(&dir)?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    Ok((
        writer,
        DebugLog {
            path: dir.join(format!("{stem}.log")),
            _guard: guard,
        },
    ))
}
