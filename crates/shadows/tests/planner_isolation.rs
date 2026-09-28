//! Stop reaches only its own turn. Two turns on two threads in two projects run
//! at once under one runtime, each on its own adapter; stopping one terminates
//! its adapter's tree and records it `Cancelled`, and the other keeps running
//! and then completes normally. The
//! process-level half of the same claim — a tree's termination never reaches
//! another tree or a process outside every tree — is
//! `crates/shadows-process/tests/containment.rs`.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use shadows::command::{CommandContext, fingerprint};
use shadows::events::Actor;
use shadows::operation::{Operation, OperationId};
use shadows::planner::{LiveHandles, PlannerTurn, StopOutcome};
use shadows::runtime::Runtime;
use shadows::storage::Storage;
use shadows::thread::ThreadId;

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/turn.rs"]
mod turn;

/// A project with its own directory, and one thread in it.
async fn project_with_thread(runtime: &Runtime, slug: &str, dir: &Path) -> ThreadId {
    let params = serde_json::json!({ "slug": slug });
    let ctx = CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: format!("{slug}-project"),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("project.create", &params),
    };
    let project = runtime
        .storage
        .create_project(
            &ctx,
            slug,
            slug,
            &shadows::project::ProjectDirectory::resolve(dir).unwrap(),
            &shadows::agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let tctx = CommandContext {
        command_id: format!("{slug}-thread"),
        command_kind: "thread.create".into(),
        request_fingerprint: fingerprint("thread.create", &params),
        ..ctx
    };
    runtime
        .storage
        .create_planning_thread(&tctx, &project.id, "T", "claude-code")
        .await
        .unwrap()
        .id
}

async fn wait_for_status(runtime: &Runtime, op: &OperationId, wanted: &str) -> Operation {
    for _ in 0..200 {
        let loaded = runtime.storage.get_operation(op).await.unwrap();
        if loaded.status_kind == wanted {
            return loaded;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("operation never reached {wanted}");
}

/// Duplicated from `crates/shadows-process/tests/containment.rs`, for the reason
/// `tests/planner_turn.rs` gives.
#[cfg(windows)]
fn is_alive(pid: u32) -> bool {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ 'yes' }} else {{ 'no' }}"),
        ])
        .output()
        .expect("powershell should run");
    String::from_utf8_lossy(&out.stdout).trim() == "yes"
}

#[cfg(unix)]
fn is_alive(pid: u32) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    let Some(rest) = stat.rsplit_once(") ") else {
        return false;
    };
    !matches!(rest.1.chars().next(), Some('Z') | None)
}

#[tokio::test]
async fn stopping_one_turn_leaves_a_concurrent_turn_running_to_completion() {
    let tmp = tempfile::tempdir().unwrap();
    let (stopped_dir, kept_dir) = (tmp.path().join("one"), tmp.path().join("two"));
    std::fs::create_dir(&stopped_dir).unwrap();
    std::fs::create_dir(&kept_dir).unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage).await.unwrap();
    let runtime = Arc::new(runtime);
    let stopped_thread = project_with_thread(&runtime, "one", &stopped_dir).await;
    let kept_thread = project_with_thread(&runtime, "two", &kept_dir).await;

    let handles = Arc::new(LiveHandles::default());
    let (bus, _rx) = tokio::sync::broadcast::channel(64);
    let sessions = acp::fake_sessions(&tmp.path().join("s.sqlite3")).await;
    let start = async |thread: &ThreadId, prompt: &str| {
        turn::start_direct(&runtime, &handles, &sessions, &bus, thread, prompt)
            .await
            .unwrap()
    };
    // `ignore-cancel`: the harness never confirms, so Stop has to terminate
    // the stopped thread's adapter — the case that could reach the other.
    let stopped = start(&stopped_thread, "ignore-cancel").await;
    let kept = start(&kept_thread, "wait-for-release").await;
    wait_for_status(&runtime, &stopped, "Running").await;
    wait_for_status(&runtime, &kept, "Running").await;
    let stopped_pid = sessions.pid(&stopped_thread).await.expect("live");
    let kept_pid = sessions.pid(&kept_thread).await.expect("live");

    let outcome = PlannerTurn::stop(
        runtime.clone(),
        handles.clone(),
        sessions.clone(),
        &stopped,
        Actor::user("local"),
    )
    .await
    .unwrap();

    assert_eq!(outcome, StopOutcome::Cancelled);
    assert!(
        !is_alive(stopped_pid),
        "the stopped turn's process survived"
    );
    let other = runtime.storage.get_operation(&kept).await.unwrap();
    assert_eq!(
        other.status_kind, "Running",
        "stopping one turn ended another"
    );
    assert!(
        other.cancel_requested_at.is_none(),
        "the other turn must not even be asked to stop"
    );
    assert!(
        is_alive(kept_pid),
        "stopping one turn killed another's process"
    );
    assert!(handles.contains(&kept).await);

    // Released only now, so it was provably running through the stop above.
    std::fs::write(kept_dir.join("release"), b"").unwrap();
    let finished = wait_for_status(&runtime, &kept, "Completed").await;
    assert!(finished.outcome_json.is_some());
}
