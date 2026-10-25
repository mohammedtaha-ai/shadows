//! What tests in every crate share, and nothing more: the internals a test
//! drives below the services, the fake adapter and the paths that belong to
//! this crate. Compiled only with `test-support`, which a crate enables only in
//! its `[dev-dependencies]`: a normal build that names
//! `crate::testing` does not compile.
//!
//! `acp` builds sessions over the fake adapter, and `turn` starts a Planner
//! turn without HTTP; both were fixture copies in two test crates.

use std::sync::Arc;

use crate::app::{AppCore, CoreParts};

/// Test-only injection, without touching the operating system credential store.
pub fn secrets(storage: Arc<Storage>, key: Option<crate::SecretKey>) -> Arc<crate::Secrets> {
    AppCore::secrets_for_tests(storage, key)
}

/// Test-only inspection of the harness's private resolution boundary.
pub async fn resolve_secret_env(
    service: &crate::Secrets,
    profile: &str,
) -> Result<Vec<(String, String)>, crate::CoreError> {
    service.resolve_env(profile).await.map_err(Into::into)
}

/// The projects service over `storage`, for a test that has no `AppCore`.
pub fn projects(storage: Arc<Storage>) -> crate::Projects {
    AppCore::projects_for_tests(storage)
}

/// A project written with the command context the test built.
pub async fn create_project(
    storage: &Arc<Storage>,
    ctx: &CommandContext,
    slug: &str,
    name: &str,
    directory: &ProjectDirectory,
    default_modes: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<crate::Project, crate::StorageError> {
    projects(storage.clone())
        .create_for_test(ctx, slug, name, directory, default_modes)
        .await
}

/// A project's allowed modes replaced with the command context the test built.
pub async fn set_project_modes(
    storage: &Arc<Storage>,
    ctx: &CommandContext,
    project: &crate::ProjectId,
    modes: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<crate::Project, crate::StorageError> {
    projects(storage.clone())
        .set_modes_for_test(ctx, project, modes)
        .await
}

/// The restricted fixture: `AppCore` over a database at `db_path`, built by
/// `AppCore::start` with no secret key, no AYLUL and executables that do not
/// exist (their versions are recorded as unknown). A test reaches services
/// through the accessors only.
pub async fn app(db_path: std::path::PathBuf) -> anyhow::Result<Arc<AppCore>> {
    let missing = db_path.with_file_name("absent");
    AppCore::start(
        &crate::app::StartConfig {
            secret_key: None,
            db_path,
            node_path: missing.clone(),
            adapter_path: missing.clone(),
            harness_path: missing,
            aylul: None,
        },
        "http://127.0.0.1:0/mcp".into(),
    )
    .await
}

pub mod acp;
pub mod queue;
pub mod turn;
pub use core_db::DatabaseWrites;

/// The `fake-acp` binary, built once per test process. `CARGO_BIN_EXE_*`
/// only names binaries of the test's own package, and `fake-acp` is its own
/// package, so it is located through `escargot`.
pub fn fake_acp_path() -> std::path::PathBuf {
    static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| built("fake-acp", "fake-acp")).clone()
}

/// `shadows-process`'s `tree_probe`, which daemon tests run as a stand-in executable.
pub fn tree_probe_path() -> std::path::PathBuf {
    static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| built("shadows-process", "tree_probe"))
        .clone()
}

/// No `.current_target()`: it passes `--target`, which builds into
/// `target/<triple>/` and compiles every dependency a second time.
fn built(package: &str, bin: &str) -> std::path::PathBuf {
    escargot::CargoBuild::new()
        .package(package)
        .bin(bin)
        .features("test-support")
        .current_release()
        .run()
        .unwrap_or_else(|e| panic!("{bin} builds: {e}"))
        .path()
        .to_path_buf()
}

/// The Planner's instructions, as `harness/setup` compiles them in.
pub use core_harness::TEST_PROMPT as PROMPT;

/// The foundation's embedded migrations directory.
pub fn migrations_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../core-db/migrations")
}

/// Plans' pure rules, which `tests/workflow_rules.rs` calls directly. They
/// are private to `plans` everywhere else: a caller changes a plan through
/// `Plans` only.
pub use core_plans::for_tests::{Applied, Ops, Rules};

/// The turn machinery below `Turns`, which the turn, shutdown and recovery
/// tests drive directly. Private to `turns` everywhere else: a caller starts
/// or stops a turn through `Turns` only.
pub use core_turns::LiveHandles;
pub use core_turns::for_tests::{
    Dequeue, FailureStage, InvocationVersions, NewTurn, PlannerTurn, PlannerTurnRequest,
    StartedTurn, StopOutcome,
};

/// Storage, the runtime and the sessions, which the storage, recovery and
/// session tests open directly and `CoreParts` is built from. Private to the
/// crate everywhere else.
pub use crate::app::adapter_version;
pub use core_db::JournalEntry as DurableEvent;
pub use core_db::{Anchor, Causation, CommandContext, JournalEntry, Storage};
pub use core_events::EventCursor;
pub use core_grants::Writer;
pub use core_grants::{IssuedGrant, Token};
pub use core_harness::{Harness, LeaseError, OpenSession, SessionsConfig};
pub use core_operations::ReconcileReport;
pub use core_projects::ProjectDirectory;
pub use core_runtime::Runtime;
pub use core_threads::{NewThreadEntry, TurnContext};
pub use core_turns::Bus;

impl AppCore {
    /// The application over parts a test built, so the test keeps its own
    /// handles on the same `Arc`s. The binary builds it with `start`.
    pub fn assemble(parts: CoreParts) -> Arc<AppCore> {
        AppCore::from_parts(parts)
    }
}
