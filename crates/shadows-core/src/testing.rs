//! What tests in every crate share: where the fake adapter is, and the paths
//! a test needs that belong to this crate. Compiled only with `test-support`.

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
pub const PROMPT: &str = include_str!("harness/prompt.txt");

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
