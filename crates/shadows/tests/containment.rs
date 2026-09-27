use std::time::Duration;

use shadows::process::{ProcessSpec, spawn};

#[cfg(windows)]
fn is_alive(pid: u32) -> bool {
    use std::process::Command;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command",
               &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ 'yes' }} else {{ 'no' }}")])
        .output()
        .expect("powershell should run");
    String::from_utf8_lossy(&out.stdout).trim() == "yes"
}

/// A process that has been killed but not yet reaped keeps its `/proc` entry,
/// so existence is not life — the zombie state is what tells a corpse from a
/// running process. Windows has no such state, which is why an existence check
/// passed there and failed only on Linux.
#[cfg(unix)]
fn is_alive(pid: u32) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    // Field 2 is `comm`, parenthesised and free to contain spaces and
    // parentheses; the state character is the first field after the LAST `)`.
    let Some(rest) = stat.rsplit_once(") ") else {
        return false;
    };
    !matches!(rest.1.chars().next(), Some('Z') | None)
}

/// Spawns `tree_probe --spawn-grandchild` as a managed tree and answers its
/// handle with the leader's and the grandchild's pids, both confirmed alive.
async fn spawn_probe_tree() -> (shadows::process::ProcessHandle, u32, u32) {
    let mut handle = spawn(ProcessSpec {
        executable: env!("CARGO_BIN_EXE_tree_probe").into(),
        args: vec!["--spawn-grandchild".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: false,
    })
    .expect("spawn should succeed");
    let mut lines = handle.take_stdout_lines().expect("stdout was captured");
    let first = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
        .await
        .expect("probe should report within 10s")
        .unwrap()
        .expect("probe should print a line");
    let grandchild: u32 = first
        .trim()
        .strip_prefix("grandchild=")
        .expect("probe prints grandchild=<pid>")
        .parse()
        .unwrap();
    let child = handle.id().expect("child has a pid");
    assert!(
        is_alive(child) && is_alive(grandchild),
        "the tree must start alive"
    );
    (handle, child, grandchild)
}

/// Waits, bounded, for every pid to be gone; panics naming the survivors.
async fn wait_until_gone(pids: &[u32], what: &str) {
    for _ in 0..50 {
        if pids.iter().all(|pid| !is_alive(*pid)) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let alive: Vec<u32> = pids.iter().copied().filter(|pid| is_alive(*pid)).collect();
    panic!("{what} survived termination: {alive:?}");
}

/// A process this test started outside any managed tree, killed when the test
/// ends however it ends — a failing assertion must not leave it sleeping.
struct Bystander(std::process::Child);

impl Drop for Bystander {
    fn drop(&mut self) {
        // Best effort by nature: this runs during a panic too, where there is
        // no one left to report a failed cleanup to.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Stop must kill only what its turn started. The user runs Claude desktop and
/// other Claude Code sessions on the same machine as Shadows, and those are
/// processes with the same executable name as a turn's harness. Termination
/// goes through the containment handle (§1.5, §8.3), so a process outside the
/// managed tree is out of its reach — even one running the very same binary,
/// which is what this bystander is, so that a kill by name would find it.
///
/// On Unix the tree is a process session, and the bystander stays in this
/// test's own session, so the same claim is measured there too.
#[tokio::test]
async fn terminating_a_managed_tree_leaves_a_process_outside_it_alive() {
    let mut bystander = Bystander(
        std::process::Command::new(env!("CARGO_BIN_EXE_tree_probe"))
            .arg("--sleep")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the bystander should start"),
    );
    let (mut managed, child, grandchild) = spawn_probe_tree().await;

    managed
        .terminate_tree()
        .expect("termination should succeed");
    wait_until_gone(&[child, grandchild], "the managed tree").await;

    assert!(
        bystander.0.try_wait().unwrap().is_none() && is_alive(bystander.0.id()),
        "terminating a managed tree killed a process outside it"
    );
}

/// Two turns in two projects are two managed trees. Stopping one must leave
/// the other — leader and grandchild — running.
#[tokio::test]
async fn terminating_one_managed_tree_leaves_another_alive() {
    let (mut stopped, stopped_child, stopped_grandchild) = spawn_probe_tree().await;
    let (mut kept, kept_child, kept_grandchild) = spawn_probe_tree().await;

    stopped
        .terminate_tree()
        .expect("termination should succeed");
    wait_until_gone(&[stopped_child, stopped_grandchild], "the stopped tree").await;

    assert!(!kept.has_exited(), "the other tree's leader was killed");
    assert!(
        is_alive(kept_child) && is_alive(kept_grandchild),
        "the other tree was reached: leader alive={}, grandchild alive={}",
        is_alive(kept_child),
        is_alive(kept_grandchild)
    );

    kept.terminate_tree().expect("cleanup should succeed");
    wait_until_gone(&[kept_child, kept_grandchild], "the kept tree").await;
}

/// Spec §1.5 and §3.7 Layer 4. A managed child and every managed descendant
/// must not survive termination. A direct-child-only assertion is insufficient,
/// so the probe builds a real grandchild and checks that one too.
#[tokio::test]
async fn terminating_a_managed_tree_kills_the_grandchild_too() {
    let probe = env!("CARGO_BIN_EXE_tree_probe");
    let mut handle = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--spawn-grandchild".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: false,
    })
    .expect("spawn should succeed");

    // tree_probe prints its grandchild's pid on its first stdout line.
    let mut lines = handle.take_stdout_lines().expect("stdout was captured");
    let first = tokio::time::timeout(Duration::from_secs(10), async { lines.next_line().await })
        .await
        .expect("probe should report within 10s")
        .unwrap()
        .expect("probe should print a line");

    let grandchild: u32 = first
        .trim()
        .strip_prefix("grandchild=")
        .expect("probe prints grandchild=<pid>")
        .parse()
        .unwrap();
    let child = handle.id().expect("child has a pid");

    assert!(is_alive(child), "child should be alive before termination");
    assert!(
        is_alive(grandchild),
        "grandchild should be alive before termination"
    );

    handle.terminate_tree().expect("termination should succeed");

    // Give the OS a bounded moment to reap, then assert both are gone.
    for _ in 0..50 {
        if !is_alive(child) && !is_alive(grandchild) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "survived termination: child={child} alive={}, grandchild={grandchild} alive={}",
        is_alive(child),
        is_alive(grandchild)
    );
}

/// Completion belongs to the managed tree, not only its leader. A harness that
/// exits after leaving a helper behind must not let that helper outlive wait().
#[tokio::test]
async fn waiting_for_the_leader_reaps_any_remaining_grandchild() {
    let probe = env!("CARGO_BIN_EXE_tree_probe");
    let mut handle = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--spawn-grandchild-and-exit".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: false,
    })
    .expect("spawn should succeed");

    let mut lines = handle.take_stdout_lines().expect("stdout was captured");
    let first = tokio::time::timeout(Duration::from_secs(10), async { lines.next_line().await })
        .await
        .expect("probe should report within 10s")
        .unwrap()
        .expect("probe should print a line");
    let grandchild: u32 = first
        .trim()
        .strip_prefix("grandchild=")
        .expect("probe prints grandchild=<pid>")
        .parse()
        .unwrap();
    assert!(is_alive(grandchild), "grandchild must start alive");

    handle.wait().await.expect("leader wait should succeed");
    for _ in 0..50 {
        if !is_alive(grandchild) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("grandchild survived a completed managed wait: grandchild={grandchild}");
}

/// Spec §1.5: the child's stdin is closed. An open stdin that never receives
/// data costs a fixed stall on every turn — measured at three seconds against
/// the real harness.
#[tokio::test]
async fn a_spawned_child_has_no_inherited_stdin() {
    let probe = env!("CARGO_BIN_EXE_tree_probe");
    let mut handle = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--read-stdin".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: false,
    })
    .unwrap();

    let status = tokio::time::timeout(Duration::from_secs(5), handle.wait())
        .await
        .expect("a child with closed stdin must see EOF immediately, not block")
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn a_piped_child_echoes_stdin_and_its_tree_is_contained() {
    let mut handle = shadows::process::spawn(ProcessSpec {
        executable: env!("CARGO_BIN_EXE_tree_probe").into(),
        args: vec!["echo".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: true,
    })
    .unwrap();
    let (mut stdin, stdout, _stderr) = handle.take_stdio().expect("stdio taken once");
    assert!(handle.take_stdio().is_none());
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    stdin.write_all(b"ping\n").await.unwrap();
    let mut lines = BufReader::new(stdout).lines();
    assert_eq!(lines.next_line().await.unwrap().as_deref(), Some("ping"));
    handle.terminate_tree().unwrap();
    handle.wait().await.unwrap();
}

/// Spec §8.4 case 4 turns on one fact and one only: had the process already
/// exited when cancellation arrived. `PlannerTurn::stop` asks `has_exited` and
/// must get an answer immediately — a wait there would hang cancellation on
/// the very process it is trying to outlive.
///
/// The full-stack window this fact serves (the child has exited, but the turn
/// watcher has not yet claimed its registration) cannot be entered
/// deterministically from a test: closing that gap would need a pause knob
/// inside the interlock, and machinery in the arbitration costs more than it
/// proves. So the fact is tested where it is defined, and the planner's own
/// suite covers the two branches that read it.
#[tokio::test]
async fn has_exited_answers_immediately_and_tells_the_two_states_apart() {
    let probe = env!("CARGO_BIN_EXE_tree_probe");
    let mut running = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--sleep".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: false,
        pipe_stdin: false,
    })
    .expect("spawn should succeed");

    let asked_at = std::time::Instant::now();
    let answer = running.has_exited();
    let took = asked_at.elapsed();
    assert!(!answer, "a child sleeping for ten minutes has not exited");
    assert!(
        took < Duration::from_secs(1),
        "has_exited must answer without waiting; took {took:?}"
    );

    // The same question about a child that really has ended. `--read-stdin`
    // returns as soon as it sees EOF on the stdin this crate closes.
    let mut finished = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--read-stdin".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: false,
        pipe_stdin: false,
    })
    .expect("spawn should succeed");
    finished.wait().await.expect("the child should exit");
    assert!(
        finished.has_exited(),
        "a reaped child must report that it exited"
    );

    running.terminate_tree().expect("cleanup should succeed");
    running.wait().await.expect("the killed child should reap");
}

/// A child's stderr is a pipe the daemon owns. Left unread it fills, the child
/// blocks on its next write, and it never exits: a harness that warns at
/// length would hang its turn forever. A megabyte of stderr must not stop a
/// child from finishing.
#[tokio::test]
async fn a_child_that_floods_stderr_still_exits() {
    let mut handle = spawn(ProcessSpec {
        executable: env!("CARGO_BIN_EXE_tree_probe").into(),
        args: vec!["--flood-stderr".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: false,
    })
    .expect("spawn should succeed");
    let status = tokio::time::timeout(Duration::from_secs(20), handle.wait())
        .await
        .expect("the child blocked on its stderr and never exited")
        .unwrap();
    assert!(status.success(), "{status}");
}
