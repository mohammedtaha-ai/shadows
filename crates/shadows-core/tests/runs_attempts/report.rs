//! An attempt's reported commit in a real git repository. The test plays the
//! executor: it writes the files, commits them with git itself and calls
//! `Runs::report` with the grant an executor session holds. No agent runs.
use super::*;
use core_runs::AttemptId;
use shadows_core::Grant;
use shadows_core::testing::TaskScope;

/// T3 writes `backend/app/**`, T5 `web/**`.
const WRITES: [(u32, &str); 2] = [(3, "backend/app/**"), (5, "web/**")];

impl Fixture {
    /// An attempt at `task` as Start leaves it once its session is open:
    /// `Running`, on an executor thread, holding its task's grant, with the
    /// HEAD of now as its start.
    async fn begin(&self, task: u32) -> (AttemptId, Grant) {
        let writes = WRITES.iter().find(|(n, _)| *n == task).unwrap().1;
        let writes = serde_json::to_string(&[writes]).unwrap();
        let (project, workflow) = (self.project.clone(), self.workflow.clone());
        let id = format!("attempt-{task}");
        let row = id.clone();
        let head = self.head();
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
                            model, state, writes_json, started_at, command_id, fingerprint,
                            start_head)
                         VALUES (?, ?, ?, 1, ?, 1, ?, 'fake-small', 'running', ?,
                                 '2026-10-05T00:00:00Z', ?, 'f', ?)",
                    )
                    .bind(&row)
                    .bind(project.as_str())
                    .bind(workflow.as_str())
                    .bind(i64::from(task))
                    .bind(thread.as_str())
                    .bind(&writes)
                    .bind(&row)
                    .bind(&head)
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

    /// Commits `paths` as the executor does: by name, with its own message.
    fn commit(&self, paths: &[&str], message: &str) -> String {
        for path in paths {
            self.git(&["add", "--", path]);
        }
        self.git(&["commit", "-q", "-m", message]);
        self.head()
    }

    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }

    async fn report(&self, grant: &Grant, commit: &str) -> Result<Attempt, RunsError> {
        self.core
            .runs()
            .report(grant, commit, "Did it; ran the tests, all passed.".into())
            .await
    }

    async fn still_running(&self, attempt: &AttemptId) {
        let now = self.core.runs().attempt(attempt).await.unwrap();
        assert_eq!(now.state, AttemptState::Running, "{now:?}");
        assert!(now.commit.is_none() && now.report.is_none(), "{now:?}");
    }
}

#[tokio::test]
async fn a_reported_commit_inside_writes_is_accepted_with_its_report() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "<?php echo 1;\n");
    f.put("notes.txt", "the person's\n");
    let sha = f.commit(&["backend/app/a.php"], "Echo one");
    let done = f.report(&grant, &sha[..12]).await.unwrap();
    assert_eq!(done.id, attempt);
    assert_eq!(done.state, AttemptState::Committed);
    assert_eq!(done.commit.as_deref(), Some(sha.as_str()));
    assert_eq!(
        done.report.as_deref(),
        Some("Did it; ran the tests, all passed.")
    );
    assert!(done.ended_at.is_some());
    // The person's file outside writes is untouched and still untracked.
    let status = f.git(&["status", "--porcelain", "-uall"]);
    assert_eq!(status.trim(), "?? notes.txt", "{status}");
    let again = f.report(&grant, &sha).await;
    assert!(
        matches!(again, Err(RunsError::AlreadyCommitted)),
        "{again:?}"
    );
    f.finish().await;
}

#[tokio::test]
async fn a_commit_with_paths_outside_writes_is_refused_until_fixed() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "<?php\n");
    f.put("web/b.ts", "b\n");
    let sha = f.commit(&["backend/app/a.php", "web/b.ts"], "Both");
    let refused = f.report(&grant, &sha).await;
    assert!(
        matches!(&refused, Err(RunsError::OutsideWrites { paths }) if paths == &["web/b.ts"]),
        "{refused:?}"
    );
    let text = refused.unwrap_err().to_string();
    assert!(
        text.contains("web/b.ts") && text.contains("report again"),
        "{text}"
    );
    f.still_running(&attempt).await;
    // The executor takes the stray path out of its commit and reports again.
    f.git(&["rm", "-q", "--cached", "web/b.ts"]);
    f.git(&["commit", "-q", "--amend", "-m", "Only mine"]);
    let fixed = f.head();
    let done = f.report(&grant, &fixed).await.unwrap();
    assert_eq!(done.commit.as_deref(), Some(fixed.as_str()));
    f.finish().await;
}

#[tokio::test]
async fn a_commit_with_a_binary_file_is_refused() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "<?php\n");
    std::fs::write(f.root.join("backend/app/python.exe"), b"MZ\0\0\x90\0binary").unwrap();
    let sha = f.commit(
        &["backend/app/a.php", "backend/app/python.exe"],
        "With a runtime",
    );
    let refused = f.report(&grant, &sha).await;
    assert!(
        matches!(&refused, Err(RunsError::BinaryFiles { paths })
            if paths == &["backend/app/python.exe"]),
        "{refused:?}"
    );
    f.still_running(&attempt).await;
    f.finish().await;
}

#[tokio::test]
async fn an_unknown_or_unreachable_commit_is_refused() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    for name in ["deadbeefdeadbeef", "HEAD", "--all", ""] {
        let refused = f.report(&grant, name).await;
        assert!(
            matches!(&refused, Err(RunsError::UnknownCommit { .. })),
            "{name}: {refused:?}"
        );
    }
    // A commit on a branch the folder does not have checked out.
    let main = f
        .git(&["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_string();
    f.git(&["checkout", "-q", "-b", "side"]);
    f.put("backend/app/a.php", "<?php\n");
    let side = f.commit(&["backend/app/a.php"], "Elsewhere");
    f.git(&["checkout", "-q", &main]);
    let refused = f.report(&grant, &side).await;
    assert!(
        matches!(&refused, Err(RunsError::NotOnBranch { commit }) if commit == &side),
        "{refused:?}"
    );
    f.still_running(&attempt).await;
    f.finish().await;
}

#[tokio::test]
async fn a_commit_from_before_the_attempt_is_refused() {
    let f = fixture().await;
    let before = f.head();
    let (attempt, grant) = f.begin(3).await;
    let refused = f.report(&grant, &before).await;
    assert!(
        matches!(&refused, Err(RunsError::NotNew { commit }) if commit == &before),
        "{refused:?}"
    );
    f.still_running(&attempt).await;
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
    f.put("backend/app/a.php", "one\ntwo\n");
    f.commit(&["backend/app/a.php"], "seed");
    let (attempt, grant) = f.begin(3).await;
    assert!(f.core.runs().diff(&attempt).await.unwrap().is_empty());
    f.put("backend/app/a.php", "one\nTWO\nthree\n");
    f.put("backend/app/b.php", "b\n");
    let sha = f.commit(&["backend/app/a.php", "backend/app/b.php"], "Change");
    f.report(&grant, &sha).await.unwrap();
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
