//! One job: the application's composition root (spec §14.4) — `AppCore`,
//! built once and shared as `Arc<AppCore>`.
//!
//! Not a global: no `static`, no `OnceLock`, no service locator. The binary
//! builds it with `start`; tests build it with `assemble` from the `Arc`s they
//! keep. A caller reaches a service through its accessor, `core.plans()`.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::claude::ClaudeAdapter;
use shadows_agent::events::HarnessEvent;
use shadows_process::{ProcessSpec, spawn};

use crate::command::{CommandContext, fingerprint};
use crate::error::CoreError;
use crate::events::UiSignal;
use crate::operation::OperationId;
use crate::planner::{LiveHandles, Sessions, SessionsConfig};
use crate::plans::Plans;
use crate::runtime::Runtime;
use crate::storage::{StopKind, Storage};
use crate::thread::ThreadId;

/// A running turn's live harness events, for every live subscriber.
pub type Bus = tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>;

/// What `AppCore` is assembled from. Tests build it so the fixture keeps its
/// own handles on the same `Arc`s; production builds it in `start`. Its fields
/// are public by design: it is the one way in, and only construction uses it.
pub struct CoreParts {
    pub storage: Arc<Storage>,
    pub runtime: Arc<Runtime>,
    pub sessions: Arc<Sessions>,
    pub handles: Arc<LiveHandles>,
    pub bus: Bus,
    /// What `plan_show` signals the tab that sent the turn (§13.9): live
    /// only, never stored.
    pub ui: tokio::sync::broadcast::Sender<UiSignal>,
    /// `http://<bound address>/mcp`: what a grant's `claude mcp add` names.
    pub mcp_url: String,
}

/// The application: every service, built once.
pub struct AppCore {
    plans: Plans,
    // Held while adapters still reach them (Tasks 5–9); each goes when its
    // last reader moves into a service, and Task 10 removes the last.
    storage: Arc<Storage>,
    runtime: Arc<Runtime>,
    sessions: Arc<Sessions>,
    handles: Arc<LiveHandles>,
    bus: Bus,
    ui: tokio::sync::broadcast::Sender<UiSignal>,
    mcp_url: String,
}

/// What `start` needs from the binary's `Config`: the database path, node,
/// adapter and harness paths. `Config` stays in the binary, which owns argument
/// parsing, and builds this.
pub struct StartConfig {
    pub db_path: PathBuf,
    pub node_path: PathBuf,
    pub adapter_path: PathBuf,
    pub harness_path: PathBuf,
}

/// The `CommandContext` a person's command carries: principal `User`, id
/// `local`, schema version 1, the fingerprint of `params`. The same body as
/// `shadows_http::project::ctx`, so no fingerprint moves.
///
/// Every route that mutates carries the caller's command id (spec §3.2), and
/// the fingerprint is derived from the same parameters the capability is about
/// to act on — never supplied by the client, which would let a replay with a
/// different body claim to be the same command.
pub(crate) fn user_command(
    command_id: String,
    kind: &str,
    params: serde_json::Value,
) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id,
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &params),
    }
}

impl AppCore {
    /// Opens storage, starts the runtime (recovery), revokes every thread
    /// grant, reads the harness versions, and builds the sessions on
    /// `mcp_url`. The caller binds first: `mcp_url` names the bound address,
    /// and a daemon that cannot bind must not run recovery (§14.4).
    pub async fn start(config: &StartConfig, mcp_url: String) -> anyhow::Result<Arc<AppCore>> {
        let storage = Arc::new(Storage::open(&config.db_path).await?);
        // `Runtime::start` logs recovery (`recovery.reconcile`).
        let (runtime, _report) = Runtime::start(storage.clone()).await?;
        let runtime = Arc::new(runtime);
        // Spec §13.7: a Planner's grant lives as long as its adapter, and every
        // adapter of an earlier daemon is gone. Before anything is served.
        let revoked = storage.revoke_all_thread_grants().await?;
        tracing::info!(revoked, "recovery.thread_grants_revoked");

        let version = harness_version(&config.harness_path).await;
        let adapter_version = adapter_version(&config.adapter_path);
        tracing::info!(adapter_version, claude_version = %version, "harness.versions");
        // Every Planner session opens with the `/mcp` of the address actually
        // bound (§13.8), which differs from the configured one on port 0.
        let sessions = Sessions::new(
            Arc::new(ClaudeAdapter {
                node: config.node_path.clone(),
                adapter: config.adapter_path.clone(),
                agent: config.harness_path.clone(),
                adapter_version: adapter_version.to_string(),
                agent_version: version.clone(),
            }),
            Storage::open(&config.db_path).await?,
            SessionsConfig {
                mcp_url: Some(mcp_url.clone()),
                ..SessionsConfig::default()
            },
        );
        let (bus, _) = tokio::sync::broadcast::channel(4096);
        Ok(Self::assemble(CoreParts {
            storage,
            runtime,
            sessions,
            handles: Arc::new(LiveHandles::default()),
            bus,
            ui: tokio::sync::broadcast::channel(256).0,
            mcp_url,
        }))
    }

    /// The application over parts already built.
    pub fn assemble(parts: CoreParts) -> Arc<AppCore> {
        let CoreParts {
            storage,
            runtime,
            sessions,
            handles,
            bus,
            ui,
            mcp_url,
        } = parts;
        Arc::new(AppCore {
            plans: Plans::new(storage.clone(), handles.clone(), ui.clone()),
            storage,
            runtime,
            sessions,
            handles,
            bus,
            ui,
            mcp_url,
        })
    }

    pub fn plans(&self) -> &Plans {
        &self.plans
    }

    /// §8.5 through `planner::shut_down(runtime, handles, sessions, bound,
    /// second_signal)`, unchanged: every running turn is stopped and every
    /// adapter closed. The binary keeps the signals and the transport.
    pub async fn shut_down(
        &self,
        bound: Duration,
        second_signal: impl Future<Output = ()>,
    ) -> Result<StopKind, CoreError> {
        Ok(crate::planner::shut_down(
            self.runtime.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            bound,
            second_signal,
        )
        .await?)
    }

    #[doc(hidden)]
    // transitional: removed by Task 10
    pub fn storage(&self) -> &Arc<Storage> {
        &self.storage
    }

    #[doc(hidden)]
    // transitional: removed by Task 7
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    #[doc(hidden)]
    // transitional: removed by Task 10
    pub fn sessions(&self) -> &Arc<Sessions> {
        &self.sessions
    }

    #[doc(hidden)]
    // transitional: removed by Task 7
    pub fn handles(&self) -> &Arc<LiveHandles> {
        &self.handles
    }

    #[doc(hidden)]
    // transitional: removed by Task 10
    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    #[doc(hidden)]
    // transitional: removed by Task 10
    pub fn ui_bus(&self) -> &tokio::sync::broadcast::Sender<UiSignal> {
        &self.ui
    }

    #[doc(hidden)]
    // transitional: removed by Task 6
    pub fn mcp_url(&self) -> &str {
        &self.mcp_url
    }
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
        pipe_stdin: false,
    }) {
        Ok(h) => h,
        Err(_) => return "unknown".to_string(),
    };
    let lines = handle.take_stdout_lines();
    // Bounded: this runs before the daemon serves, so an executable that never
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

/// Reads the installed ACP adapter's package version from its entry point's
/// package root. A missing or malformed package is recorded as unknown so the
/// daemon can still start and report the configuration problem at runtime.
pub fn adapter_version(adapter_entry: &Path) -> String {
    let Some(package_json) = adapter_entry
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("package.json"))
    else {
        return "unknown".to_string();
    };
    std::fs::read(package_json)
        .ok()
        .and_then(|contents| serde_json::from_slice::<serde_json::Value>(&contents).ok())
        .and_then(|package| package.get("version")?.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}
