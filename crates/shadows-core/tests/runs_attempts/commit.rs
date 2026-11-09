//! An attempt's commit in a real git repository. The test plays the
//! executor: it writes the files and calls `Runs::commit` with the grant an
//! executor session holds. No agent runs.
use super::*;
use core_runs::AttemptId;
use shadows_core::Grant;
use shadows_core::testing::{RunHooks, TaskScope};

/// T3 writes `backend/app/**`, T5 `web/**`.
const WRITES: [(u32, &str); 2] = [(3, "backend/app/**"), (5, "web/**")];

impl Fixture {
    /// An attempt at `task` as Start leaves it once its session is open:
    /// `Running`, on an executor thread, holding its task's grant.
    async fn begin(&self, task: u32) -> (AttemptId, Grant) {
        let writes = WRITES.iter().find(|(n, _)| *n == task).unwrap().1;
        let writes = serde_json::to_string(&[writes]).unwrap();
        let (project, workflow) = (self.project.clone(), self.workflow.clone());
        let id = format!("attempt-{task}");
        let row = id.clone();
        let grant = self
            .storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let thread = shadows_core::Threads::create_executor_in(
                        conn,
                        &project,
                        format!("T{task}"),
                        "claude-code",
                    )
                    .await?;
                    sqlx::query(
                        "INSERT INTO run_attempts
                           (id, project_id, workflow_id, version, task, number, thread_id,
                            model, state, writes_json, started_at, command_id, fingerprint)
                         VALUES (?, ?, ?, 1, ?, 1, ?, 'fake-small', 'running', ?,
                                 '2026-10-05T00:00:00Z', ?, 'f')",
                    )
                    .bind(&row)
                    .bind(project.as_str())
                    .bind(workflow.as_str())
                    .bind(i64::from(task))
                    .bind(thread.as_str())
                    .bind(&writes)
                    .bind(&row)
                    .execute(&mut *conn)
                    .await?;
                    let scope = TaskScope {
                        workflow: workflow.as_str().to_string(),
                        version: 1,
                        task,
                    };
                    let (grant, _) = shadows_core::Grants::issue_executor_grant_in(
                        conn, &project, &thread, scope,
                    )
                    .await?;
                    Ok(grant)
                })
            })
            .await
            .unwrap();
        (AttemptId::from_recorded(id), grant)
    }

    fn put(&self, path: &str, text: &str) {
        let at = self.root.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
    }

    fn git(&self, args: &[&str]) -> String {
        git(&self.root, args)
    }

    /// Commits `paths` as the person would, before any attempt.
    fn seed(&self, files: &[(&str, &str)]) {
        for (path, text) in files {
            self.put(path, text);
            self.git(&["add", path]);
        }
        self.git(&["commit", "-q", "-m", "seed"]);
    }

    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }

    /// The paths the commit `rev` changed, sorted.
    fn committed(&self, rev: &str) -> Vec<String> {
        let mut paths: Vec<String> = self
            .git(&["show", "--name-only", "--format=", rev])
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        paths.sort();
        paths
    }
}

#[tokio::test]
async fn commit_takes_only_the_paths_in_writes() {
    let f = fixture().await;
    f.seed(&[("backend/app/a.php", "<?php\n"), ("web/b.ts", "a\n")]);
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "<?php echo 1;\n");
    f.put("web/b.ts", "b\n");
    let done = f
        .core
        .runs()
        .commit(&grant, "Echo one".into())
        .await
        .unwrap();
    assert_eq!(done.files, ["backend/app/a.php"]);
    assert_eq!(done.left, ["web/b.ts"]);
    assert_eq!(done.attempt.id, attempt);
    assert_eq!(done.attempt.state, AttemptState::Committed);
    assert_eq!(done.attempt.commit.as_deref(), Some(f.head().as_str()));
    assert_eq!(f.committed("HEAD"), ["backend/app/a.php"]);
    let message = f.git(&["log", "-1", "--format=%B"]);
    assert!(message.starts_with("T3: Task 3\n\nEcho one"), "{message}");
    assert!(
        message.contains(&format!("Shadows-Attempt: {attempt}")),
        "{message}"
    );
    // The file outside writes is still changed, and nothing committed shows.
    let status = f.git(&["status", "--porcelain"]);
    assert_eq!(status.trim(), "M web/b.ts", "{status}");
    let again = f.core.runs().commit(&grant, "again".into()).await;
    assert!(
        matches!(again, Err(RunsError::AlreadyCommitted)),
        "{again:?}"
    );
    f.finish().await;
}

#[tokio::test]
async fn a_persons_staged_file_stays_staged() {
    let f = fixture().await;
    let (_, grant) = f.begin(3).await;
    f.put("notes.md", "mine\n");
    f.git(&["add", "notes.md"]);
    f.put("backend/app/x.php", "<?php\n");
    f.core.runs().commit(&grant, "Add x".into()).await.unwrap();
    assert_eq!(f.committed("HEAD"), ["backend/app/x.php"]);
    let staged = f.git(&["diff", "--cached", "--name-only"]);
    assert_eq!(staged.trim(), "notes.md");
    f.finish().await;
}

#[tokio::test]
async fn files_a_generator_wrote_are_committed() {
    let f = fixture().await;
    let (_, grant) = f.begin(3).await;
    f.put("backend/app/composer.json", "{}\n");
    f.put("backend/app/Models/User.php", "<?php\n");
    let done = f
        .core
        .runs()
        .commit(&grant, "Scaffold".into())
        .await
        .unwrap();
    assert_eq!(
        f.committed("HEAD"),
        ["backend/app/Models/User.php", "backend/app/composer.json"]
    );
    assert!(done.left.is_empty(), "{:?}", done.left);
    f.finish().await;
}

#[tokio::test]
async fn ignored_files_never_enter_the_commit() {
    let f = fixture().await;
    f.seed(&[("backend/app/.gitignore", "vendor/\n.env\n")]);
    let (_, grant) = f.begin(3).await;
    f.put("backend/app/vendor/x.php", "<?php\n");
    f.put("backend/app/.env", "SECRET=1\n");
    let nothing = f.core.runs().commit(&grant, "Nothing".into()).await;
    assert!(
        matches!(nothing, Err(RunsError::NothingToCommit { .. })),
        "{nothing:?}"
    );
    f.put("backend/app/routes.php", "<?php\n");
    f.core.runs().commit(&grant, "Routes".into()).await.unwrap();
    assert_eq!(f.committed("HEAD"), ["backend/app/routes.php"]);
    f.finish().await;
}

#[tokio::test]
async fn a_rename_from_outside_writes_leaves_its_origin() {
    let f = fixture().await;
    f.seed(&[("web/a.ts", "a\n")]);
    let (_, grant) = f.begin(3).await;
    std::fs::create_dir_all(f.root.join("backend").join("app")).unwrap();
    f.git(&["mv", "web/a.ts", "backend/app/a.ts"]);
    let done = f.core.runs().commit(&grant, "Move".into()).await.unwrap();
    assert_eq!(done.files, ["backend/app/a.ts"]);
    assert_eq!(done.left, ["web/a.ts"]);
    assert_eq!(f.committed("HEAD"), ["backend/app/a.ts"]);
    // The origin's deletion is still staged, as the person's move left it.
    let status = f.git(&["status", "--porcelain"]);
    assert_eq!(status.trim(), "D  web/a.ts", "{status}");
    f.finish().await;
}

/// A folder link: a junction on Windows, a symbolic link elsewhere.
fn link(at: &Path, to: &Path) {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(at)
            .arg(to)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(to, at).unwrap();
}

#[tokio::test]
async fn a_junction_inside_writes_refuses_the_commit() {
    let f = fixture().await;
    f.put("web/secret.ts", "s\n");
    let app = f.root.join("backend").join("app");
    std::fs::create_dir_all(&app).unwrap();
    link(&app.join("link"), &f.root.join("web"));
    let before = f.head();
    let (attempt, grant) = f.begin(3).await;
    let refused = f.core.runs().commit(&grant, "Linked".into()).await;
    let Err(RunsError::LinkInScope { paths }) = &refused else {
        panic!("{refused:?}");
    };
    assert!(
        !paths.is_empty() && paths.iter().all(|p| p.starts_with("backend/app/link")),
        "{paths:?}"
    );
    assert_eq!(f.head(), before);
    let failed = f.core.runs().attempt(&attempt).await.unwrap();
    assert_eq!(failed.state, AttemptState::Failed);
    assert!(
        failed
            .reason
            .as_deref()
            .unwrap()
            .contains("backend/app/link"),
        "{:?}",
        failed.reason
    );
    f.finish().await;
}

#[tokio::test]
async fn head_moving_during_the_commit_is_refused() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "<?php\n");
    f.core.runs().set_hooks_for_tests(RunHooks {
        before_update_ref: Some(Arc::new(|root: &Path| {
            git(root, &["commit", "-q", "--allow-empty", "-m", "elsewhere"]);
        })),
        fail_record: false,
    });
    let moved = f.core.runs().commit(&grant, "Late".into()).await;
    assert!(matches!(moved, Err(RunsError::HeadMoved)), "{moved:?}");
    assert_eq!(f.git(&["log", "-1", "--format=%s"]).trim(), "elsewhere");
    // Nothing is lost: the work is still in the folder, and the attempt may commit again.
    let status = f.git(&["status", "--porcelain", "-uall"]);
    assert!(status.contains("backend/app/a.php"), "{status}");
    let still = f.core.runs().attempt(&attempt).await.unwrap();
    assert_eq!(still.state, AttemptState::Running);
    f.core.runs().set_hooks_for_tests(RunHooks::default());
    f.core.runs().commit(&grant, "Again".into()).await.unwrap();
    assert_eq!(f.committed("HEAD"), ["backend/app/a.php"]);
    f.finish().await;
}

#[tokio::test]
async fn a_crash_between_git_and_sqlite_recovers_as_committed() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "<?php\n");
    f.core.runs().set_hooks_for_tests(RunHooks {
        before_update_ref: None,
        fail_record: true,
    });
    assert!(f.core.runs().commit(&grant, "Crash".into()).await.is_err());
    let sha = f.head();
    assert_eq!(
        f.core.runs().attempt(&attempt).await.unwrap().state,
        AttemptState::Running
    );
    let report = f.core.runs().recover().await.unwrap();
    assert_eq!((report.committed, report.interrupted), (1, 0));
    let settled = f.core.runs().attempt(&attempt).await.unwrap();
    assert_eq!(settled.state, AttemptState::Committed);
    assert_eq!(settled.commit.as_deref(), Some(sha.as_str()));
    f.finish().await;
}

#[tokio::test]
async fn two_commits_at_once_are_serialized() {
    let f = fixture().await;
    let ((_, backend), (_, web)) = (f.begin(3).await, f.begin(5).await);
    f.put("backend/app/a.php", "<?php\n");
    f.put("web/b.ts", "b\n");
    let runs = f.core.runs();
    let (a, b) = tokio::join!(
        runs.commit(&backend, "Backend".into()),
        runs.commit(&web, "Web".into())
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.files, ["backend/app/a.php"]);
    assert_eq!(b.files, ["web/b.ts"]);
    let (a, b) = (a.attempt.commit.unwrap(), b.attempt.commit.unwrap());
    assert_eq!(f.committed(&a), ["backend/app/a.php"]);
    assert_eq!(f.committed(&b), ["web/b.ts"]);
    let log = f.git(&["log", "--format=%H", "-3"]);
    assert!(log.contains(&a) && log.contains(&b), "{log}");
    f.finish().await;
}

#[tokio::test]
async fn start_refuses_a_folder_that_is_not_git() {
    let f = fixture().await;
    std::fs::remove_dir_all(f.root.join(".git")).unwrap();
    let started = f.start("s1", 3).await;
    assert!(matches!(started, Err(RunsError::NotGit)), "{started:?}");
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn start_refuses_uncommitted_changes_in_writes() {
    let f = fixture().await;
    f.put("backend/app/x.php", "<?php\n");
    let started = f.start("s1", 3).await;
    assert!(
        matches!(&started, Err(RunsError::DirtyScope { paths }) if paths == &["backend/app/x.php"]),
        "{started:?}"
    );
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn diff_returns_each_files_counts_and_patch() {
    let f = fixture().await;
    f.seed(&[("backend/app/a.php", "one\ntwo\n")]);
    let (attempt, grant) = f.begin(3).await;
    assert!(f.core.runs().diff(&attempt).await.unwrap().is_empty());
    f.put("backend/app/a.php", "one\nTWO\nthree\n");
    f.put("backend/app/b.php", "b\n");
    f.core.runs().commit(&grant, "Change".into()).await.unwrap();
    let diff = f.core.runs().diff(&attempt).await.unwrap();
    let a = diff.iter().find(|d| d.path == "backend/app/a.php").unwrap();
    assert_eq!((a.added, a.removed), (2, 1));
    assert!(
        a.patch.contains("+TWO") && a.patch.contains("-two"),
        "{}",
        a.patch
    );
    let b = diff.iter().find(|d| d.path == "backend/app/b.php").unwrap();
    assert_eq!((b.added, b.removed), (1, 0));
    assert_eq!(diff.len(), 2);
    f.finish().await;
}
