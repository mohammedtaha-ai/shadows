//! One job: the application's composition root (spec §14.4) — `AppCore`,
//! built once and shared as `Arc<AppCore>`.
//!
//! Not a global: no `static`, no `OnceLock`, no service locator. The binary
//! builds it with `start`; tests build it with `assemble` (`testing`, under
//! `test-support`) from the `Arc`s they keep. A caller reaches a service
//! through its accessor, `core.plans()`.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::claude::ClaudeAdapter;
use shadows_agent::events::HarnessEvent;
use shadows_process::{ProcessSpec, spawn};

use crate::code::{Code, CodeConfig};
use crate::command::{CommandContext, fingerprint};
use crate::db::Storage;
use crate::error::CoreError;
use crate::events::{Events, UiSignal};
use crate::grants::Grants;
use crate::harness::{Harness, Sessions, SessionsConfig};
use crate::instructions::Instructions;
use crate::plans::Plans;
use crate::projects::Projects;
use crate::runtime::Runtime;
use crate::runtime::StopKind;
use crate::threads::{ThreadId, Threads};
use crate::turns::{LiveHandles, OperationId, ThreadStopper, Turns};

/// A running turn's live harness events, for every live subscriber.
pub type Bus = tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>;

/// What `AppCore` is assembled from. Tests build it so the fixture keeps its
/// own handles on the same `Arc`s; production builds it in `start`. Its fields
/// are public by design: it is the one way in, and only construction uses it.
/// Outside the crate it is named only under `test-support` (§14.6): its
/// fields are the internals no adapter reaches.
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
    grants: Grants,
    turns: Turns,
    harness: Arc<Harness>,
    projects: Projects,
    threads: Threads,
    instructions: Instructions,
    events: Events,
    code: Code,
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
/// `local`, schema version 1, the fingerprint of `params`. The same body the
/// routes built before the services did, so no fingerprint moves.
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
        // Through a `Grants` of its own: `assemble` builds the core's after
        // the sessions, and the revocation keeps its place before them.
        let revoked = Grants::new(storage.clone(), mcp_url.clone())
            .revoke_thread_grants()
            .await?;
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
            storage.clone(),
            SessionsConfig {
                mcp_url: Some(mcp_url.clone()),
                ..SessionsConfig::default()
            },
        );
        let (bus, _) = tokio::sync::broadcast::channel(4096);
        let core = Self::from_parts(CoreParts {
            storage,
            runtime,
            sessions,
            handles: Arc::new(LiveHandles::default()),
            bus,
            ui: tokio::sync::broadcast::channel(256).0,
            mcp_url,
        });
        // §15.8: the active set is chosen and the workers spawned; `serve`
        // waits for no scan. The index never stops the daemon (§15.4).
        if let Err(error) = core.code.start(CodeConfig::default()).await {
            tracing::warn!(error = %error, "code.start_failed");
        }
        Ok(core)
    }

    /// The application over parts already built. Tests reach it as
    /// `AppCore::assemble` (`testing`).
    pub(crate) fn from_parts(parts: CoreParts) -> Arc<AppCore> {
        let CoreParts {
            storage,
            runtime,
            sessions,
            handles,
            bus,
            ui,
            mcp_url,
        } = parts;
        // Before the three services that hold it to call `touch` (§15.8).
        let code = Code::new(storage.clone());
        let harness = Arc::new(Harness::new(storage.clone(), sessions.clone()));
        let stopper = ThreadStopper::new(runtime.clone(), handles.clone(), sessions.clone());
        let threads = Threads::new(storage.clone(), harness.clone(), code.clone(), stopper);
        let events = Events::new(storage.clone(), harness.clone(), bus.clone(), ui.clone());
        Arc::new(AppCore {
            plans: Plans::new(storage.clone(), handles.clone(), ui.clone()),
            grants: Grants::new(storage.clone(), mcp_url),
            turns: Turns::new(
                storage.clone(),
                runtime,
                sessions.clone(),
                handles,
                bus.clone(),
                code.clone(),
            ),
            harness,
            projects: Projects::new(storage.clone(), code.clone()),
            threads,
            instructions: Instructions::new(storage.clone()),
            events,
            code,
        })
    }

    pub fn plans(&self) -> &Plans {
        &self.plans
    }

    pub fn grants(&self) -> &Grants {
        &self.grants
    }

    pub fn turns(&self) -> &Turns {
        &self.turns
    }

    pub fn harness(&self) -> &Harness {
        &self.harness
    }

    pub fn projects(&self) -> &Projects {
        &self.projects
    }

    pub fn threads(&self) -> &Threads {
        &self.threads
    }

    pub fn instructions(&self) -> &Instructions {
        &self.instructions
    }

    pub fn events(&self) -> &Events {
        &self.events
    }

    pub fn code(&self) -> &Code {
        &self.code
    }

    /// First `Code`'s watchers and workers stop (§15.8), each after the file
    /// it is on; then §8.5 through `turns::shut_down(runtime, handles,
    /// sessions, bound, second_signal)`, unchanged: every running turn is
    /// stopped and every adapter closed. The binary keeps the signals and the
    /// transport.
    pub async fn shut_down(
        &self,
        bound: Duration,
        second_signal: impl Future<Output = ()>,
    ) -> Result<StopKind, CoreError> {
        self.code.shut_down().await;
        Ok(self.turns.shut_down(bound, second_signal).await?)
    }
}

/// Spec §1.4: read the harness's self-reported version and record it. The
/// measured stream contract belongs to one installation at one version.
///
/// It goes through `process::spawn` rather than `tokio::process` directly:
/// CLAUDE.md makes `shadows-process` the sole owner of that API, and a version probe
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
