//! What tests in every crate share, and nothing more: the internals a test
//! drives below the services, the fake adapter and the paths that belong to
//! this crate. Compiled only with `test-support`, which a crate enables only in
//! its `[dev-dependencies]` (spec §14.6): a normal build that names
//! `shadows_core::testing` does not compile.
//!
//! `acp` builds sessions over the fake adapter, and `turn` starts a Planner
//! turn without HTTP; both were fixture copies in two test crates.

use std::sync::Arc;

use crate::app::{AppCore, CoreParts};

pub mod acp;
pub mod turn;

/// The `fake-acp` binary, built once per test process. `CARGO_BIN_EXE_*`
/// only names binaries of the test's own package, and `fake-acp` is its own
/// package, so it is located through `escargot` (spec §14.3).
pub fn fake_acp_path() -> std::path::PathBuf {
    static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| built("fake-acp", "fake-acp")).clone()
}

/// `shadows-process`'s `tree_probe`, which the daemon tests run as a stand-in
/// executable (moved here from Task 1's `fixtures/probe.rs`).
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
pub const PROMPT: &str = include_str!("../harness/prompt.txt");

/// This crate's migrations directory.
pub fn migrations_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

/// Plans' pure rules, which `tests/workflow_rules.rs` calls directly. They
/// are private to `plans` everywhere else: a caller changes a plan through
/// `Plans` only.
pub use crate::plans::for_tests::{Applied, apply, approval_problems, edit_problems};

/// The turn machinery below `Turns`, which the turn, shutdown and recovery
/// tests drive directly. Private to `turns` everywhere else: a caller starts
/// or stops a turn through `Turns` only.
pub use crate::turns::LiveHandles;
pub use crate::turns::for_tests::{
    FailureStage, NewTurn, PlannerTurn, PlannerTurnRequest, StartedTurn, StopOutcome, shut_down,
};

/// Storage, the runtime and the sessions, which the storage, recovery and
/// session tests open directly and `CoreParts` is built from. Private to the
/// crate everywhere else (§14.6).
pub use crate::app::{Bus, adapter_version};
pub use crate::command::derive::{Anchor, derived_id};
pub use crate::command::{CommandContext, Writer, fingerprint};
pub use crate::db::{Storage, append_event_for_test};
pub use crate::events::{Causation, DurableEvent, EventCursor};
pub use crate::grants::{IssuedGrant, Token, hash_token};
pub use crate::harness::{LeaseError, OpenSession, Sessions, SessionsConfig, prompt_version};
pub use crate::projects::ProjectDirectory;
pub use crate::runtime::{ReconcileReport, Runtime};
pub use crate::threads::{NewThreadEntry, TurnContext};

impl crate::code::Code {
    /// `scan`, now, on the test's task: the index a test asks about is the
    /// one its files were just written into.
    pub async fn scan_for_test(
        &self,
        project: &crate::projects::ProjectId,
    ) -> Result<(), crate::error::CoreError> {
        self.scan(project).await
    }
}

impl AppCore {
    /// The application over parts a test built, so the test keeps its own
    /// handles on the same `Arc`s. The binary builds it with `start`.
    pub fn assemble(parts: CoreParts) -> Arc<AppCore> {
        AppCore::from_parts(parts)
    }
}
