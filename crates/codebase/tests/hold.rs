//! What a hold reports: the project's own files that changed while it lived,
//! on a real folder under real git, with the project's watcher running.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use codebase::{ActiveHold, Asker, Codebase, CodebaseConfig};
use core_db::Storage;
use core_projects::{ProjectId, Projects};

/// A git repository with an ignore file and two committed files, as a
/// project whose worker has listed it once; the temporary folders come last.
async fn listed_project() -> (Codebase, ProjectId, tempfile::TempDir, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::write(
        d.join(".gitignore"),
        "__pycache__/\n.pytest_cache/\n/out/*.exe\n",
    )
    .unwrap();
    std::fs::write(d.join("tracked.txt"), "one\n").unwrap();
    std::fs::write(d.join("gone.txt"), "soon gone\n").unwrap();
    std::fs::create_dir(d.join("report")).unwrap();
    std::fs::write(d.join("report/kept.txt"), "kept\n").unwrap();
    git(d, &["init", "-q"]);
    git(d, &["add", "."]);
    git(
        d,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "init",
        ],
    );
    let db = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&db.path().join("code.sqlite3"))
            .await
            .unwrap(),
    );
    let projects = Projects::new(storage.clone());
    let project = projects
        .create("hold".into(), "hold", "Hold", d.to_str().unwrap())
        .await
        .unwrap()
        .id;
    let code = Codebase::new(storage, projects);
    let config = CodebaseConfig::default();
    code.start(config).await.unwrap();
    code.refresh_inventory(Asker::Person(&project))
        .await
        .unwrap();
    (code, project, dir, db)
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn write(dir: &Path, path: &str, text: &str) {
    let full = dir.join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, text).unwrap();
}

/// Waits until `hold` reports every path of `want`, then a little longer so
/// a path that should be left out has had its chance to arrive.
async fn settled(hold: &ActiveHold, want: &[&str]) -> Vec<String> {
    let start = Instant::now();
    loop {
        let changed = hold.changed().await;
        if want.iter().all(|w| changed.iter().any(|c| c == w)) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(15), "{changed:?}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    hold.changed().await
}

/// A tracked file modified, one deleted; next to them an ignored file, an
/// ignored folder, a created folder, and a file made and removed at once.
fn changes(d: &Path) {
    write(d, "tracked.txt", "two\n");
    std::fs::remove_file(d.join("gone.txt")).unwrap();
    write(d, "__pycache__/m.cpython-313.pyc", "\0");
    write(d, ".pytest_cache/v/cache/nodeids", "[]");
    write(d, "out/water.exe", "\0");
    std::fs::create_dir(d.join("made")).unwrap();
    write(d, "temp.txt", "brief");
    std::fs::remove_file(d.join("temp.txt")).unwrap();
}

#[tokio::test]
async fn a_hold_reports_only_project_files() {
    let (code, project, dir, _db) = listed_project().await;
    let hold = code.hold(&project).await;
    changes(dir.path());
    let changed = settled(&hold, &["gone.txt", "tracked.txt"]).await;
    assert_eq!(changed, ["gone.txt", "tracked.txt"]);
    drop(hold);
    code.shut_down_for_test().await;
}

#[tokio::test]
async fn every_reported_path_is_one_a_note_accepts() {
    let (code, project, dir, _db) = listed_project().await;
    let hold = code.hold(&project).await;
    changes(dir.path());
    write(dir.path(), "new/fresh.txt", "new\n");
    let changed = settled(&hold, &["gone.txt", "new/fresh.txt", "tracked.txt"]).await;
    assert!(!changed.is_empty());
    for path in &changed {
        let accepted = code.project_file(&project, path).await;
        assert!(accepted.is_ok(), "{path}: {accepted:?}");
    }
    drop(hold);
    code.shut_down_for_test().await;
}

#[tokio::test]
async fn a_claimed_path_goes_to_its_claiming_hold_only() {
    let (code, project, dir, _db) = listed_project().await;
    let planner = code.hold(&project).await;
    let executor = code
        .hold_claiming(&project, |path| path.starts_with("report/"))
        .await;
    write(dir.path(), "report/kept.txt", "changed\n");
    write(dir.path(), "outside.txt", "out of scope\n");
    let theirs = settled(&executor, &["outside.txt", "report/kept.txt"]).await;
    assert_eq!(theirs, ["outside.txt", "report/kept.txt"]);
    let ours = settled(&planner, &["outside.txt"]).await;
    assert_eq!(ours, ["outside.txt"]);
    drop((planner, executor));
    code.shut_down_for_test().await;
}
