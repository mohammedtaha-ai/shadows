//! A managed Windows child keeps its per-user cache location under an explicit environment.
#![cfg(windows)]

use shadows_process::{ProcessSpec, spawn};

#[tokio::test]
async fn a_windows_child_keeps_local_app_data() {
    let expected = std::env::var("LOCALAPPDATA").unwrap();
    let mut child = spawn(ProcessSpec {
        executable: std::env::var_os("ComSpec").unwrap().into(),
        args: vec!["/d".into(), "/c".into(), "echo %LOCALAPPDATA%".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: false,
    })
    .unwrap();
    let mut lines = child.take_stdout_lines().unwrap();
    let mut stdout = String::new();
    while let Some(line) = lines.next_line().await.unwrap() {
        stdout.push_str(&line);
        stdout.push('\n');
    }
    assert!(child.wait().await.unwrap().success());
    assert_eq!(stdout.trim(), expected);
}
