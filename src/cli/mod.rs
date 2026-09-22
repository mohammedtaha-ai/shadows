//! One job: the daemon's entry point — assemble the product and serve it.

use std::path::Path;
use std::sync::Arc;

use crate::agent::claude::ClaudeHarness;
use crate::config::Config;
use crate::operation::OperationId;
use crate::planner::{LiveHandles, PlannerTurn};
use crate::process::{ProcessSpec, spawn};
use crate::protocol::{AppState, router};
use crate::runtime::Runtime;
use crate::storage::{StopKind, Storage};

/// Binds, prints exactly one address, and serves. Spec §1.0: it never opens a
/// browser. The user chooses which browser to use.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let storage = Arc::new(Storage::open(&config.db_path).await?);
    let (runtime, report) = Runtime::start(storage.clone()).await?;
    let runtime = Arc::new(runtime);
    tracing::info!(
        interrupted = report.interrupted.len(),
        anomalies = report.anomalies.len(),
        "startup recovery complete"
    );

    let version = harness_version(&config.harness_path).await;
    let (bus, _) = tokio::sync::broadcast::channel(4096);
    let state = AppState {
        runtime: runtime.clone(),
        storage,
        handles: Arc::new(LiveHandles::default()),
        harness: Arc::new(ClaudeHarness::new(config.harness_path.clone(), version)),
        bus,
        project_root: std::env::current_dir()?,
    };

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let addr = listener.local_addr()?;
    println!("shadows serve listening on http://{addr}");

    // Spec §8.5: shutdown reuses the cancellation path. There is no drain mode.
    let shutdown_runtime = runtime.clone();
    let shutdown_state = state.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("stop signal received; cancelling this runtime's operations");
            let ids: Vec<OperationId> = {
                let map = shutdown_state.handles.0.lock().await;
                map.keys().cloned().collect()
            };
            let mut all_confirmed = true;
            for op in ids {
                if PlannerTurn::stop(
                    shutdown_runtime.clone(),
                    shutdown_state.handles.clone(),
                    &op,
                )
                .await
                .is_err()
                {
                    all_confirmed = false;
                }
            }
            let kind = if all_confirmed {
                StopKind::Graceful
            } else {
                StopKind::Escalated
            };
            let _ = shutdown_runtime.stop(kind).await;
        })
        .await?;
    Ok(())
}

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
    }) {
        Ok(h) => h,
        Err(_) => return "unknown".to_string(),
    };
    let first = match handle.take_stdout_lines() {
        Some(mut lines) => lines.next_line().await.ok().flatten(),
        None => None,
    };
    let _ = handle.wait().await;
    match first {
        Some(line) if !line.trim().is_empty() => line.trim().to_string(),
        _ => "unknown".to_string(),
    }
}
