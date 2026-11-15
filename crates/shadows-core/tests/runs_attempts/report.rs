//! An attempt's reported commit in a real git repository. The test plays the
//! executor: it writes the files, commits them with git itself and calls
//! `Runs::report` with the grant an executor session holds. No agent runs.
use super::*;
use core_runs::AttemptId;
use shadows_core::Grant;

/// Saves T3's context for `attempt`, as Start does, with two checks: `c1` and `c2`.
async fn saved_checks(f: &Fixture, attempt: &AttemptId, grant: &Grant) {
    let mut context = f
        .core
        .plans()
        .task_context(grant, Some(&f.workflow), 3, None)
        .await
        .unwrap();
    context.task.contract = Some(
        serde_json::from_value(serde_json::json!({
            "files": [{"path":"backend/app/a.php","action":"modify","purpose":"Count"}],
            "checks": [{"id":"c1","run":["git","--version"]},
                       {"id":"c2","run":["git","status"]}]
        }))
        .unwrap(),
    );
    sqlx::query(
        "INSERT INTO run_attempt_context (attempt_id, context_json, captured_at) VALUES (?,?,?)",
    )
    .bind(attempt.as_str())
    .bind(serde_json::to_string(&context).unwrap())
    .bind("2026-11-15T00:00:00Z")
    .execute(f.storage.reader())
    .await
    .unwrap();
}

#[tokio::test]
async fn report_needs_each_check() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    saved_checks(&f, &attempt, &grant).await;
    f.put("backend/app/a.php", "one\n");
    let sha = f.commit(&["backend/app/a.php"], "build");
    for checks in [
        serde_json::json!([]),
        serde_json::json!([
            {"id":"c1","outcome":"passed"}, {"id":"c9","outcome":"passed"}
        ]),
        serde_json::json!([
            {"id":"c1","outcome":"passed"}, {"id":"c1","outcome":"passed"}
        ]),
    ] {
        let error = f
            .core
            .runs()
            .report(
                &grant,
                &sha,
                "Built.".into(),
                serde_json::from_value(checks).unwrap(),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code(), "reported_checks");
        f.still_running(&attempt).await;
    }
    f.finish().await;
}

#[tokio::test]
async fn report_records_reported_checks() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    saved_checks(&f, &attempt, &grant).await;
    f.put("backend/app/a.php", "one\n");
    let sha = f.commit(&["backend/app/a.php"], "build");
    let reported = f
        .core
        .runs()
        .report(
            &grant,
            &sha,
            "Built.".into(),
            serde_json::from_value(serde_json::json!([
                {"id":"c2","outcome":"not_run","note":"port busy"},
                {"id":"c1","outcome":"passed","note":"git available"}
            ]))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reported.state, AttemptState::Committed);
    let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT check_id, reported, reported_note, outcome FROM run_checks
         WHERE attempt_id=? ORDER BY ordinal",
    )
    .bind(attempt.as_str())
    .fetch_all(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            ("c1".into(), "passed".into(), "git available".into(), None),
            ("c2".into(), "not_run".into(), "port busy".into(), None)
        ]
    );
    f.finish().await;
}

#[tokio::test]
async fn a_file_is_read_whole_at_the_attempts_commit() {
    let f = fixture().await;
    f.put("backend/app/a.php", "one\n");
    f.put("backend/app/c.php", "c\n");
    f.commit(&["backend/app/a.php", "backend/app/c.php"], "seed");
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "one\ntwo\n");
    f.put("backend/app/b.php", "b\n");
    let sha = f.commit(&["backend/app/a.php", "backend/app/b.php"], "change");
    f.report(&grant, &sha).await.unwrap();
    f.put("backend/app/a.php", "the person's edit\n");
    let runs = f.core.runs();
    let a = runs.file(&attempt, "backend/app/a.php").await.unwrap();
    assert_eq!((a.text.as_deref(), a.cut), (Some("one\ntwo\n"), false));
    let b = runs.file(&attempt, " backend\\app\\b.php ").await.unwrap();
    assert_eq!(b.text.as_deref(), Some("b\n"));
    assert_eq!(b.path, "backend/app/b.php");
    let c = runs.file(&attempt, "backend/app/c.php").await.unwrap();
    assert_eq!(c.text.as_deref(), Some("c\n"));
    f.finish().await;
}

#[tokio::test]
async fn a_file_outside_writes_or_not_at_the_commit_is_refused() {
    let f = fixture().await;
    f.put("notes.txt", "notes\n");
    f.put("backend/app/c.php", "c\n");
    f.commit(&["notes.txt", "backend/app/c.php"], "seed");
    let (attempt, grant) = f.begin(3).await;
    f.git(&["rm", "backend/app/c.php"]);
    f.put("backend/app/a.php", "a\n");
    let sha = f.commit(&["backend/app/a.php"], "change");
    f.report(&grant, &sha).await.unwrap();
    let runs = f.core.runs();
    for path in ["notes.txt", "backend/app/../../notes.txt"] {
        let refused = runs.file(&attempt, path).await;
        assert!(
            matches!(&refused, Err(RunsError::OutsideWrites { paths }) if paths == &[path]),
            "{refused:?}"
        );
    }
    assert!(matches!(
        runs.file(&attempt, "backend/app/none.php").await,
        Err(RunsError::NotFound)
    ));
    let deleted = runs.file(&attempt, "backend/app/c.php").await.unwrap();
    assert_eq!((deleted.text, deleted.cut), (None, false));
    let (running, _) = f.begin(5).await;
    for id in [running, AttemptId::from_recorded("unknown".into())] {
        assert!(matches!(
            runs.file(&id, "web/x.js").await,
            Err(RunsError::NotFound)
        ));
    }
    f.finish().await;
}

#[tokio::test]
async fn a_long_or_binary_file_is_cut_or_has_no_text() {
    let f = fixture().await;
    f.put("backend/app/logo.bin", "");
    std::fs::write(f.root.join("backend/app/logo.bin"), [0u8, 1, 2]).unwrap();
    f.put("backend/app/invalid.txt", "");
    std::fs::write(f.root.join("backend/app/invalid.txt"), [0xffu8]).unwrap();
    f.commit(&["backend/app/logo.bin", "backend/app/invalid.txt"], "seed");
    let (attempt, grant) = f.begin(3).await;
    let long: String = (1..=2500).map(|n| format!("line {n}\n")).collect();
    f.put("backend/app/long.txt", &long);
    f.put(
        "backend/app/bytes.txt",
        &format!("{}\nend\n", "é".repeat(100_001)),
    );
    let sha = f.commit(&["backend/app/long.txt", "backend/app/bytes.txt"], "long");
    f.report(&grant, &sha).await.unwrap();
    let runs = f.core.runs();
    let long = runs.file(&attempt, "backend/app/long.txt").await.unwrap();
    assert!(long.cut);
    let text = long.text.unwrap();
    assert_eq!(text.lines().count(), 2000);
    assert!(text.ends_with("line 2000\n"));
    let bytes = runs.file(&attempt, "backend/app/bytes.txt").await.unwrap();
    assert!(bytes.cut);
    assert_eq!(bytes.text.as_deref(), Some(""));
    for path in ["backend/app/logo.bin", "backend/app/invalid.txt"] {
        let binary = runs.file(&attempt, path).await.unwrap();
        assert_eq!((binary.text, binary.cut), (None, false));
    }
    f.finish().await;
}

#[tokio::test]
async fn the_planner_reads_the_same_change_the_diff_route_serves() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "a\n");
    let sha = f.commit(&["backend/app/a.php"], "change");
    f.report(&grant, &sha).await.unwrap();
    let planner = planner_grant(&f).await;
    let runs = f.core.runs();
    assert_eq!(
        serde_json::to_value(runs.diff_for(&planner, &f.workflow, 3).await.unwrap()).unwrap(),
        serde_json::to_value(runs.diff(&attempt).await.unwrap()).unwrap()
    );
    let path = "backend/app/a.php";
    assert_eq!(
        serde_json::to_value(runs.file_for(&planner, &f.workflow, 3, path).await.unwrap()).unwrap(),
        serde_json::to_value(runs.file(&attempt, path).await.unwrap()).unwrap()
    );
    let executor = super::planner::executor_grant(&f);
    assert!(matches!(
        runs.diff_for(&executor, &f.workflow, 3).await,
        Err(RunsError::NotFound)
    ));
    assert!(matches!(
        runs.file_for(&executor, &f.workflow, 3, path).await,
        Err(RunsError::NotFound)
    ));
    assert!(matches!(
        runs.diff_for(&planner, &f.workflow, 5).await,
        Err(RunsError::NotFound)
    ));
    f.finish().await;
}

#[tokio::test]
async fn the_planners_diff_shows_patches_up_to_its_bound() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    let paths = [
        "backend/app/f1.txt",
        "backend/app/f2.txt",
        "backend/app/f3.txt",
    ];
    let text: String = (1..=900).map(|n| format!("line {n}\n")).collect();
    for path in paths {
        f.put(path, &text);
    }
    let sha = f.commit(&paths, "three files");
    f.report(&grant, &sha).await.unwrap();
    let grant = planner_grant(&f).await;
    let runs = f.core.runs();
    let diff = runs.diff_for(&grant, &f.workflow, 3).await.unwrap();
    assert_eq!(diff.len(), 3);
    for file in &diff[..2] {
        assert!(file.patch.contains("+line 900"), "{file:?}");
    }
    assert_eq!(diff[2].patch, "[not shown: read it with task_file]");
    assert_eq!((diff[2].added, diff[2].removed), (900, 0));
    assert!(
        runs.diff(&attempt).await.unwrap()[2]
            .patch
            .contains("+line 900")
    );
    f.finish().await;
}

#[tokio::test]
async fn a_report_is_refused_while_writes_hold_uncommitted_changes() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "<?php\n");
    f.put("backend/app/b.php", "<?php\n");
    f.put("notes.txt", "the person's\n");
    let sha = f.commit(&["backend/app/a.php"], "Add a");
    let refused = f.report(&grant, &sha).await;
    match &refused {
        Err(RunsError::DirtyScope { paths }) => assert_eq!(paths, &["backend/app/b.php"]),
        other => panic!("{other:?}"),
    }
    f.still_running(&attempt).await;
    std::fs::remove_file(f.root.join("backend/app/b.php")).unwrap();
    let done = f.report(&grant, &sha).await.unwrap();
    assert_eq!(done.state, AttemptState::Committed);
    let status = f.git(&["status", "--porcelain", "-uall"]);
    assert_eq!(status.trim(), "?? notes.txt", "{status}");
    f.finish().await;
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
async fn a_setup_task_may_commit_a_binary_file() {
    let f = fixture_titled("Setup").await;
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "<?php\n");
    std::fs::write(
        f.root.join("backend/app/favicon.ico"),
        b"\0\0\x01\0\0\xff\0",
    )
    .unwrap();
    let sha = f.commit(
        &["backend/app/a.php", "backend/app/favicon.ico"],
        "Scaffold with its icon",
    );
    let done = f.report(&grant, &sha).await.unwrap();
    assert_eq!(done.id, attempt);
    assert_eq!(done.state, AttemptState::Committed);
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

#[tokio::test]
async fn an_empty_commit_is_refused_until_it_changes_something() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    f.git(&["commit", "-q", "--allow-empty", "-m", "Nothing"]);
    let empty = f.head();
    let refused = f.report(&grant, &empty[..12]).await.unwrap_err();
    assert!(
        matches!(&refused, RunsError::EmptyCommit { commit } if commit == &empty),
        "{refused:?}"
    );
    assert_eq!(refused.code(), "empty_commit");
    assert_eq!(
        refused.to_string(),
        format!(
            "the commit {empty} changes no file: commit only work; if nothing needs to change, \
             do not commit, and say why in your last message"
        )
    );
    f.still_running(&attempt).await;

    f.put("backend/app/a.php", "<?php echo 1;\n");
    let sha = f.commit(&["backend/app/a.php"], "Echo one");
    let done = f.report(&grant, &sha).await.unwrap();
    assert_eq!(done.id, attempt);
    assert_eq!(done.state, AttemptState::Committed);
    assert_eq!(done.commit.as_deref(), Some(sha.as_str()));
    f.finish().await;
}

#[tokio::test]
async fn diff_answers_each_files_hunks() {
    use shadows_core::DiffOp::{Add, Context, Remove};
    let f = fixture().await;
    f.put("backend/app/a.php", "one\ntwo\n");
    f.commit(&["backend/app/a.php"], "seed");
    let (attempt, grant) = f.begin(3).await;
    f.put("backend/app/a.php", "one\nTWO\nthree\n");
    f.put("backend/app/b.php", "b\n");
    let sha = f.commit(&["backend/app/a.php", "backend/app/b.php"], "Change");
    f.report(&grant, &sha).await.unwrap();
    let diff = f.core.runs().diff(&attempt).await.unwrap();
    let lines = |path: &str| {
        let file = diff.iter().find(|d| d.path == path).unwrap();
        assert_eq!(file.hunks.len(), 1, "{:?}", file.hunks);
        let hunk = &file.hunks[0];
        assert_eq!(hunk.path, path);
        let ops: Vec<_> = hunk
            .lines
            .iter()
            .map(|l| (l.op.clone(), l.text.as_str()))
            .collect();
        (hunk.old_start, hunk.new_start, ops)
    };
    assert_eq!(
        lines("backend/app/a.php"),
        (
            Some(1),
            Some(1),
            vec![
                (Context, "one"),
                (Remove, "two"),
                (Add, "TWO"),
                (Add, "three")
            ]
        )
    );
    assert_eq!(
        lines("backend/app/b.php"),
        (Some(0), Some(1), vec![(Add, "b")])
    );
    f.finish().await;
}
