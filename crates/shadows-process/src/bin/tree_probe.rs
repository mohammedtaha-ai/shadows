//! Test-support binary. Not product code. It exists so the containment test
//! has a real daemon -> child -> grandchild hierarchy to terminate.

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("--persistent-launch") {
        use shadows_process::{ProcessSpec, spawn_persistent};
        let record = std::path::Path::new(&args[2]);
        let mut payload = vec![
            "--persistent-report".into(),
            args[4].clone(),
            args[5].clone(),
        ];
        payload.extend_from_slice(&args[6..]);
        let identity = spawn_persistent(ProcessSpec {
            executable: args[3].clone().into(),
            args: payload,
            cwd: record.parent().unwrap().to_path_buf(),
            env: vec![("PERSISTENT_TEST_VALUE".into(), "configured".into())],
            capture_stdout: false,
            pipe_stdin: false,
        })
        .expect("persistent fixture launch");
        std::fs::write(record, serde_json::to_vec(&identity).unwrap()).unwrap();
        return;
    }
    if args.get(1).map(String::as_str) == Some("--persistent-report") {
        let report = serde_json::json!({
            "args": &args[4..],
            "configured": std::env::var("PERSISTENT_TEST_VALUE").unwrap_or_default(),
            "providerTokenPresent": std::env::var_os("ANTHROPIC_AUTH_TOKEN").is_some(),
        });
        // Atomic publication keeps the reader from observing a partially written report.
        let temporary = format!("{}.tmp", args[2]);
        std::fs::write(&temporary, serde_json::to_vec(&report).unwrap()).unwrap();
        std::fs::rename(temporary, &args[2]).unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !std::path::Path::new(&args[3]).exists() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        return;
    }

    if args.iter().any(|a| a == "--read-stdin") {
        let mut buf = String::new();
        // With stdin closed this returns Ok(0) immediately. With an inherited
        // console stdin it blocks, and the test times out.
        let _ = std::io::stdin().read_to_string(&mut buf);
        return;
    }

    if args.iter().any(|a| a == "echo") {
        use std::io::{BufRead, Write};
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line.expect("stdin line");
            writeln!(stdout, "{line}").expect("stdout write");
            stdout.flush().expect("stdout flush");
        }
        return;
    }

    // Far more than any pipe buffer holds, then a clean exit: a child whose
    // stderr nobody reads blocks on it here and never exits.
    if args.iter().any(|a| a == "--flood-stderr") {
        let line = "x".repeat(1023);
        let mut stderr = std::io::stderr().lock();
        for _ in 0..1024 {
            use std::io::Write;
            writeln!(stderr, "{line}").expect("stderr write");
        }
        return;
    }

    let spawn_grandchild = args.iter().any(|a| a == "--spawn-grandchild");
    let exit_after_spawn = args.iter().any(|a| a == "--spawn-grandchild-and-exit");
    if spawn_grandchild || exit_after_spawn {
        let me = std::env::current_exe().expect("current exe");
        // The grandchild sleeps 600s; the containment test kills the whole
        // tree through the Job Object. It is never waited on — that would
        // deadlock the probe and defeat the test — so only its pid is kept.
        let grandchild = std::process::Command::new(me)
            .arg("--sleep")
            .spawn()
            .expect("grandchild should spawn")
            .id();
        println!("grandchild={grandchild}");
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        if exit_after_spawn {
            return;
        }
    }

    // Both the child and the grandchild end up here and sleep until killed.
    std::thread::sleep(std::time::Duration::from_secs(600));
}
