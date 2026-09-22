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
    })
    .unwrap();

    let status = tokio::time::timeout(Duration::from_secs(5), handle.wait())
        .await
        .expect("a child with closed stdin must see EOF immediately, not block")
        .unwrap();
    assert!(status.success());
}
