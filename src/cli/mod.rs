//! One job: the daemon's entry point — assemble the product and serve it.

pub mod args;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::agent::claude::{ClaudeAdapter, ClaudeHarness};
use crate::config::{Config, adapter_version};
use crate::planner::{LiveHandles, Sessions, SessionsConfig, shut_down};
use crate::process::{ProcessSpec, spawn};
use crate::protocol::{AppState, router};
use crate::runtime::Runtime;
use crate::storage::Storage;

/// Binds, prints exactly one address, and serves. Spec §1.0: it never opens a
/// browser. The user chooses which browser to use.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let storage = Arc::new(Storage::open(&config.db_path).await?);
    // `Runtime::start` logs recovery (`recovery.reconcile`).
    let (runtime, _report) = Runtime::start(storage.clone()).await?;
    let runtime = Arc::new(runtime);

    let version = harness_version(&config.harness_path).await;
    let adapter_version = adapter_version(&config.adapter_path);
    tracing::info!(adapter_version, claude_version = %version, "harness.versions");
    let sessions = Sessions::new(
        Arc::new(ClaudeAdapter {
            node: config.node_path.clone(),
            adapter: config.adapter_path.clone(),
            agent: config.harness_path.clone(),
            adapter_version: adapter_version.to_string(),
            agent_version: version.clone(),
        }),
        Storage::open(&config.db_path).await?,
        SessionsConfig::default(),
    );
    let (bus, _) = tokio::sync::broadcast::channel(4096);
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let state = AppState {
        runtime: runtime.clone(),
        storage,
        handles: Arc::new(LiveHandles::default()),
        harness: Arc::new(ClaudeHarness::new(config.harness_path.clone(), version)),
        sessions: Some(sessions.clone()),
        bus,
        allowed_origins: config.allowed_origins.clone(),
        shutdown,
    };

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let addr = listener.local_addr()?;
    println!("shadows serve listening on http://{addr}");
    if let Some(path) = &config.debug_log {
        println!("shadows serve debug log: {}", path.display());
    }

    // Spec §8.5: shutdown reuses the cancellation path. There is no drain mode.
    // The listener keeps accepting until this future returns — axum stops
    // accepting only then — so a turn requested meanwhile is refused by the
    // closed registry, not by the socket.
    let handles = state.handles.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            if let Err(error) = tokio::signal::ctrl_c().await {
                // Without a signal there is no way to be asked to stop, and
                // stopping now would end a daemon nobody asked to end. Serve
                // on; a killed daemon's turns are recovered at next startup.
                tracing::error!(%error, "shutdown.signal_unavailable");
                std::future::pending::<()>().await;
            }
            tracing::info!("stop signal received; cancelling this runtime's operations");
            let second_signal = async {
                // An error here means no second signal can ever arrive, which
                // is the same as one that has not arrived yet.
                if tokio::signal::ctrl_c().await.is_err() {
                    std::future::pending::<()>().await;
                }
            };
            match shut_down(runtime, handles, CONFIRMATION_BOUND, second_signal).await {
                Ok(kind) => tracing::info!(stop_kind = ?kind, "shutdown.recorded"),
                Err(error) => {
                    tracing::error!(%error, "shutdown.unrecorded: the stop could not be written")
                }
            }
            if let Err(error) = sessions.close_all().await {
                tracing::error!(%error, "shutdown.sessions_close_failed");
            }
            // Last: open live streams end here, so a graceful HTTP shutdown
            // is not left waiting on a response that never finishes.
            stopping.send_replace(true);
        })
        .await?;
    Ok(())
}

/// How long shutdown waits for every owned operation to be confirmed terminal
/// before it records `Escalated` instead of `Graceful` (§8.5). A terminated
/// tree is reaped, and its outcome written, in milliseconds; the bound exists
/// for the turn that never confirms, so a daemon told once to stop still ends.
const CONFIRMATION_BOUND: Duration = Duration::from_secs(10);

/// Spec §1.4: read the harness's self-reported version and record it. The
/// measured stream contract belongs to one installation at one version.
///
/// It goes through `process::spawn` rather than `tokio::process` directly:
/// CLAUDE.md makes `process/` the sole owner of that API, and a version probe
/// is no less a child process than a turn is. An unreadable version is not a
/// startup failure — the daemon still serves, and records that it does not
/// know.
async fn harness_version(path: &Path) -> String {
    let Ok(cwd) = std::env::current_dir() else {
        return "unknown".to_string();
    };
    let mut handle = match spawn(ProcessSpec {
        executable: path.to_path_buf(),
        args: vec!["--version".to_string()],
        cwd,
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: false,
    }) {
        Ok(h) => h,
        Err(_) => return "unknown".to_string(),
    };
    let lines = handle.take_stdout_lines();
    // Bounded: this runs before the daemon binds, so an executable that never
    // answers `--version` would otherwise hold startup forever. On expiry the
    // handle is dropped here, and its kill-on-drop ends the probe's tree.
    let probe = async {
        let first = match lines {
            Some(mut lines) => lines.next_line().await.ok().flatten(),
            None => None,
        };
        let _ = handle.wait().await;
        first
    };
    match tokio::time::timeout(VERSION_BOUND, probe).await {
        Ok(Some(line)) if !line.trim().is_empty() => line.trim().to_string(),
        Ok(_) => "unknown".to_string(),
        Err(_) => {
            tracing::warn!(
                executable = %path.display(),
                "harness.version_timeout: `--version` did not answer; recorded as unknown"
            );
            "unknown".to_string()
        }
    }
}

/// How long startup waits for the harness to state its version.
const VERSION_BOUND: Duration = Duration::from_secs(5);
