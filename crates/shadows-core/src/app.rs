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

use crate::agents::Agents;
use crate::code::{Code, CodeConfig};
use crate::db::Storage;
use crate::design::Design;
use crate::error::CoreError;
use crate::events::{Events, UiSignal};
use crate::extensions::Extensions;
use crate::grants::Grants;
use crate::harness::{Harness, Sessions, SessionsConfig};
use crate::instructions::Instructions;
use crate::person::PersonSettings;
use crate::plans::Plans;
use crate::projects::Projects;
use crate::providers::Providers;
use crate::runtime::Runtime;
use crate::runtime::StopKind;
use crate::secrets::{SecretKey, Secrets};
use crate::standards::Standards;
use crate::threads::{ThreadId, Threads};
use crate::turns::{LiveHandles, OperationId, ThreadStopper, Turns};
use crate::usage::Usage;

/// A running turn's live harness events, for every live subscriber.
pub type Bus = tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>;

/// What `AppCore` is assembled from. Tests build it so the fixture keeps its
/// own handles on the same `Arc`s; production builds it in `start`. Its fields
/// are public by design: it is the one way in, and only construction uses it.
/// Outside the crate it is named only under `test-support` (§14.6): its
/// fields are the internals no adapter reaches.
pub struct CoreParts {
    pub secrets: Arc<Secrets>,
    pub storage: Arc<Storage>,
    pub runtime: Arc<Runtime>,
    pub sessions: Arc<Sessions>,
    pub handles: Arc<LiveHandles>,
    pub bus: Bus,
    /// What `plan_show` signals the tab that sent the turn (§13.9): live
    /// only, never stored.
    pub ui: tokio::sync::broadcast::Sender<UiSignal>,
    /// `http://<bound address>/mcp`: what a grant's `claude mcp add` names.
    /// Test-only: `assemble` builds its `Grants` over it, while `start`
    /// builds the one `Grants` itself, before the sessions.
    #[cfg(feature = "test-support")]
    pub mcp_url: String,
}

/// The application: every service, built once.
///
/// Start here for Core ownership; each source is in the existing `shadows-core`
/// crate. The contract describes that owner's current methods, rules and tests.
///
/// | Service | One job | Source | Contract |
/// |---|---|---|---|
/// | `Secrets` | encrypted environment configuration | `crates/shadows-core/src/secrets` | `crates/shadows-core/src/secrets/contract.yaml` |
/// | `Providers` | AYLUL provider configuration | `crates/shadows-core/src/providers` | `crates/shadows-core/src/providers/contract.yaml` |
/// | `Agents` | permanent AYLUL agent identities | `crates/shadows-core/src/agents` | `crates/shadows-core/src/agents/contract.yaml` |
/// | `Extensions` | the shared extension library | `crates/shadows-core/src/extensions` | `crates/shadows-core/src/extensions/contract.yaml` |
/// | `Usage` | measured AYLUL usage | `crates/shadows-core/src/usage` | `crates/shadows-core/src/usage/contract.yaml` |
/// | `PersonSettings` | the local person record | `crates/shadows-core/src/person` | `crates/shadows-core/src/person/contract.yaml` |
/// | `Standards` | project standards versions | `crates/shadows-core/src/standards` | `crates/shadows-core/src/standards/contract.yaml` |
/// | `Design` | project design revisions | `crates/shadows-core/src/design` | `crates/shadows-core/src/design/contract.yaml` |
/// | `Plans` | plan version lifecycle | `crates/shadows-core/src/plans` | `crates/shadows-core/src/plans/contract.yaml` |
/// | `Grants` | grant authorization | `crates/shadows-core/src/grants` | `crates/shadows-core/src/grants/contract.yaml` |
/// | `Turns` | turn execution lifecycle | `crates/shadows-core/src/turns` | `crates/shadows-core/src/turns/contract.yaml` |
/// | `Harness` | thread harness sessions | `crates/shadows-core/src/harness` | `crates/shadows-core/src/harness/contract.yaml` |
/// | `Projects` | project directory registration | `crates/shadows-core/src/projects` | `crates/shadows-core/src/projects/contract.yaml` |
/// | `Threads` | planning thread lifecycle | `crates/shadows-core/src/threads` | `crates/shadows-core/src/threads/contract.yaml` |
/// | `Instructions` | versioned Planner instructions | `crates/shadows-core/src/instructions` | `crates/shadows-core/src/instructions/contract.yaml` |
/// | `Events` | thread or project event subscriptions | `crates/shadows-core/src/events` | `crates/shadows-core/src/events/contract.yaml` |
/// | `Code` | project code indexing | `crates/shadows-core/src/code` | `crates/shadows-core/src/code/contract.yaml` |
/// | `Runtime` | runtime instance lifecycle | `crates/shadows-core/src/runtime` | `crates/shadows-core/src/runtime/contract.yaml` |
///
/// The first seventeen owners have typed accessors on `AppCore`. `Runtime` is
/// internal: `start` builds it before assembly, then `Turns` holds the shared
/// instance for execution and shutdown. There is no `AppCore::runtime` accessor.
/// `start` builds the AYLUL configuration owners (`AylulOwners`) and `Grants`
/// once. `Sessions` receives those owners and holds them one way (deleting an
/// agent is `Harness`'s, since it closes live adapters); `Harness` shares
/// `Sessions`, and the stored `Grants` is the one that revoked thread grants at
/// startup. No service receives `AppCore`.
/// `Threads` owns a private store; transaction-local callers use `Threads::*_in`
/// over the shared connection, without depending on its retained lifecycle effects.
pub struct AppCore {
    secrets: Arc<Secrets>,
    providers: Arc<Providers>,
    agents: Arc<Agents>,
    extensions: Arc<Extensions>,
    usage: Usage,
    person: PersonSettings,
    design: Design,
    plans: Plans,
    grants: Grants,
    turns: Turns,
    harness: Arc<Harness>,
    projects: Projects,
    threads: Threads,
    instructions: Instructions,
    standards: Standards,
    events: Events,
    code: Code,
}

/// The AYLUL configuration owners an opening reads, built in one place so `start`
/// and the test-only sessions wire them the same way.
#[derive(Clone)]
pub(crate) struct AylulOwners {
    pub(crate) providers: Arc<Providers>,
    pub(crate) agents: Arc<Agents>,
    pub(crate) extensions: Arc<Extensions>,
    pub(crate) person: PersonSettings,
}

impl AylulOwners {
    pub(crate) fn build(
        storage: Arc<Storage>,
        secrets: Arc<Secrets>,
        home_root: Option<PathBuf>,
        projects: Projects,
    ) -> Self {
        let providers = Providers::new(storage.clone(), secrets.clone());
        let agents = Agents::new(
            storage.clone(),
            providers.clone(),
            secrets,
            projects,
            home_root.clone(),
        );
        Self {
            providers,
            agents,
            extensions: Extensions::new(storage.clone(), home_root),
            person: PersonSettings::new(storage),
        }
    }
}

/// What `start` needs from the binary's `Config`: the database path, node,
/// adapter and harness paths. `Config` stays in the binary, which owns argument
/// parsing, and builds this.
pub struct StartConfig {
    pub secret_key: Option<SecretKey>,
    pub db_path: PathBuf,
    pub node_path: PathBuf,
    pub adapter_path: PathBuf,
    pub harness_path: PathBuf,
    /// AYLUL's profile; `None` leaves AYLUL absent.
    pub aylul: Option<crate::harness::AylulConfig>,
}

impl AppCore {
    /// Opens storage, starts the runtime (recovery), revokes every thread
    /// grant, reads the harness versions, and builds the sessions on
    /// `mcp_url`. The caller binds first: `mcp_url` names the bound address,
    /// and a daemon that cannot bind must not run recovery (§14.4).
    pub async fn start(config: &StartConfig, mcp_url: String) -> anyhow::Result<Arc<AppCore>> {
        let storage = Arc::new(Storage::open(&config.db_path).await?);
        let secrets = Secrets::new(storage.clone(), config.secret_key.clone());
        // `Runtime::start` logs recovery (`recovery.reconcile`).
        let (runtime, _report) = Runtime::start(storage.clone()).await?;
        let runtime = Arc::new(runtime);
        // Spec §13.7: a Planner's grant lives as long as its adapter, and every
        // adapter of an earlier daemon is gone. Before anything is served.
        // The one `Grants`: it revokes here, before the sessions exist, and
        // then belongs to the core.
        let grants = Grants::new(storage.clone(), mcp_url.clone());
        let revoked = grants.revoke_thread_grants().await?;
        tracing::info!(revoked, "recovery.thread_grants_revoked");

        let version = harness_version(&config.harness_path).await;
        let adapter_version = adapter_version(&config.adapter_path);
        tracing::info!(adapter_version, claude_version = %version, "harness.versions");
        // Every Planner session opens with the `/mcp` of the address actually
        // bound (§13.8), which differs from the configured one on port 0.
        let projects = Projects::new(storage.clone());
        let owners = AylulOwners::build(
            storage.clone(),
            secrets.clone(),
            config.aylul.as_ref().map(|c| c.home_root.clone()),
            projects.clone(),
        );
        let instructions = Instructions::new(storage.clone());
        let standards = Standards::new(storage.clone());
        let sessions = Sessions::with_owners(
            Arc::new(ClaudeAdapter {
                node: config.node_path.clone(),
                adapter: config.adapter_path.clone(),
                agent: config.harness_path.clone(),
                adapter_version: adapter_version.to_string(),
                agent_version: version.clone(),
            }),
            storage.clone(),
            SessionsConfig {
                secrets: Some(secrets.clone()),
                mcp_url: Some(mcp_url.clone()),
                aylul: config.aylul.clone(),
                ..SessionsConfig::default()
            },
            owners.clone(),
            instructions.clone(),
            grants.clone(),
        );
        let (bus, _) = tokio::sync::broadcast::channel(4096);
        let core = Self::assemble_services(
            CoreParts {
                secrets,
                storage,
                runtime,
                sessions,
                handles: Arc::new(LiveHandles::default()),
                bus,
                ui: tokio::sync::broadcast::channel(256).0,
                #[cfg(feature = "test-support")]
                mcp_url,
            },
            owners,
            grants,
            instructions,
            standards,
            projects,
        );
        // §15.8: the active set is chosen and the workers spawned; `serve`
        // waits for no scan. The index never stops the daemon (§15.4).
        if let Err(error) = core.code.start(CodeConfig::default()).await {
            tracing::warn!(error = %error, "code.start_failed");
        }
        Ok(core)
    }

    /// The application over parts already built. Tests reach it as
    /// `AppCore::assemble` (`testing`); `start` assembles its own.
    #[cfg(feature = "test-support")]
    pub(crate) fn from_parts(parts: CoreParts) -> Arc<AppCore> {
        let owners = parts.sessions.owners.clone();
        let grants = Grants::new(parts.storage.clone(), parts.mcp_url.clone());
        let instructions = Instructions::new(parts.storage.clone());
        let projects = Projects::new(parts.storage.clone());
        let standards = Standards::new(parts.storage.clone());
        Self::assemble_services(parts, owners, grants, instructions, standards, projects)
    }

    /// The services over parts and the owners built before assembly.
    fn assemble_services(
        parts: CoreParts,
        owners: AylulOwners,
        grants: Grants,
        instructions: Instructions,
        standards: Standards,
        projects: Projects,
    ) -> Arc<AppCore> {
        let CoreParts {
            secrets,
            storage,
            runtime,
            sessions,
            handles,
            bus,
            ui,
            // `mcp_url`, under `test-support`: `grants` was built over it.
            ..
        } = parts;
        // Before the three services that hold it to call `touch`. The projects
        // it reads are the copy that holds no code index.
        let code = Code::new(storage.clone(), projects.clone());
        let harness = Arc::new(Harness::new(
            storage.clone(),
            sessions.clone(),
            projects.clone(),
        ));
        let stopper = ThreadStopper::new(runtime.clone(), handles.clone(), sessions.clone());
        let threads = Threads::new(
            storage.clone(),
            harness.clone(),
            code.clone(),
            projects.clone(),
            stopper,
        );
        let events = Events::new(
            storage.clone(),
            harness.clone(),
            projects.clone(),
            bus.clone(),
            ui.clone(),
        );
        let AylulOwners {
            providers,
            agents,
            extensions,
            person,
        } = owners;
        Arc::new(AppCore {
            providers,
            agents,
            extensions,
            usage: Usage::new(storage.clone()),
            person,
            secrets,
            design: Design::new(storage.clone(), handles.clone()),
            plans: Plans::new(storage.clone(), grants.clone(), handles.clone(), ui.clone()),
            grants,
            turns: Turns::new(
                storage.clone(),
                runtime,
                sessions.clone(),
                handles,
                bus.clone(),
                code.clone(),
                instructions.clone(),
                standards.clone(),
                projects.clone(),
            ),
            harness,
            projects: projects.with_code(code.clone()),
            threads,
            instructions,
            standards,
            events,
            code,
        })
    }

    pub fn plans(&self) -> &Plans {
        &self.plans
    }

    pub fn providers(&self) -> &Providers {
        &self.providers
    }

    pub fn agents(&self) -> &Agents {
        &self.agents
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn usage(&self) -> &Usage {
        &self.usage
    }

    pub fn person(&self) -> &PersonSettings {
        &self.person
    }

    pub fn secrets(&self) -> &Secrets {
        &self.secrets
    }

    #[cfg(feature = "test-support")]
    pub fn standards_for_tests(storage: Arc<Storage>) -> Standards {
        Standards::new(storage)
    }

    pub fn standards(&self) -> &Standards {
        &self.standards
    }

    pub fn design(&self) -> &Design {
        &self.design
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

#[cfg(feature = "test-support")]
impl AppCore {
    pub fn grants_for_tests(storage: Arc<Storage>, mcp_url: String) -> Grants {
        Grants::new(storage, mcp_url)
    }
}
