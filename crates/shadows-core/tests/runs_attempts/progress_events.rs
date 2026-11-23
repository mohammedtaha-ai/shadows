//! Each change of a task's progress journals one TaskProgressChanged, in the
//! write that makes it, with identifiers only.
use super::*;

/// The journaled TaskProgressChanged payloads, in sequence order.
async fn progress_events(f: &Fixture) -> Vec<Value> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind, payload_json FROM durable_event
         WHERE kind = 'TaskProgressChanged' ORDER BY seq",
    )
    .fetch_all(f.storage.reader())
    .await
    .unwrap();
    rows.into_iter()
        .map(|(_, payload)| serde_json::from_str(&payload).unwrap())
        .collect()
}

fn expected(f: &Fixture, task: u32, attempt: Option<&str>) -> Value {
    serde_json::json!({
        "project_id": f.project.as_str(),
        "workflow_id": f.workflow.as_str(),
        "version": 1,
        "task": task,
        "attempt_id": attempt,
    })
}

#[tokio::test]
async fn starting_an_attempt_journals_progress_once() {
    let f = fixture().await;
    f.hold();
    let attempt = f.start("s1", 3).await.unwrap();
    let events = progress_events(&f).await;
    assert_eq!(events, vec![expected(&f, 3, Some(attempt.id.as_str()))]);
    let again = f.start("s1", 3).await.unwrap();
    assert_eq!(again.id, attempt.id);
    assert_eq!(
        progress_events(&f).await.len(),
        1,
        "a replayed Start journaled"
    );
    f.finish().await;
}

#[tokio::test]
async fn an_attempt_ending_without_done_journals_progress() {
    let f = fixture().await;
    f.hold();
    let attempt = f.start("s1", 3).await.unwrap();
    f.release();
    let ended = f.settled(&attempt).await;
    assert_eq!(ended.state, AttemptState::Failed);
    let one = expected(&f, 3, Some(attempt.id.as_str()));
    assert_eq!(progress_events(&f).await, vec![one.clone(), one]);
    f.finish().await;
}

#[tokio::test]
async fn a_report_journals_progress() {
    let f = fixture().await;
    let (attempt, grant) = f.begin(3).await;
    assert!(progress_events(&f).await.is_empty());
    f.put("backend/app/a.php", "<?php\n");
    let sha = f.commit(&["backend/app/a.php"], "build");
    let done = f.report(&grant, &sha).await.unwrap();
    assert_eq!(done.state, AttemptState::Committed);
    assert_eq!(
        progress_events(&f).await,
        vec![expected(&f, 3, Some(attempt.as_str()))]
    );
    f.finish().await;
}

#[tokio::test]
async fn a_stop_and_a_startup_interruption_journal_progress() {
    let f = fixture().await;
    f.hold();
    let attempt = f.start("s1", 3).await.unwrap();
    let stopped = f.core.runs().stop(&attempt.id).await.unwrap();
    assert_eq!(stopped.state, AttemptState::Stopped);
    let one = expected(&f, 3, Some(attempt.id.as_str()));
    assert_eq!(progress_events(&f).await, vec![one.clone(), one]);
    f.finish().await;

    let f = fixture().await;
    for (id, task) in [("a1", 3), ("a2", 5)] {
        sqlx::query(
            "INSERT INTO run_attempts
               (id, project_id, workflow_id, version, task, number, thread_id, model, state,
                writes_json, started_at, command_id, fingerprint)
             VALUES (?, ?, ?, 1, ?, 1, 'gone', 'fake-small', 'running', '[]',
                     '2026-10-05T00:00:00Z', ?, 'f')",
        )
        .bind(id)
        .bind(f.project.as_str())
        .bind(f.workflow.as_str())
        .bind(task)
        .bind(id)
        .execute(f.storage.reader())
        .await
        .unwrap();
    }
    assert_eq!(f.core.runs().recover().await.unwrap().interrupted, 2);
    assert_eq!(
        progress_events(&f).await,
        vec![expected(&f, 3, Some("a1")), expected(&f, 5, Some("a2"))]
    );
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_done_and_a_not_done_mark_journal_progress_with_no_attempt() {
    let f = fixture().await;
    let runs = f.core.runs();
    runs.mark_done("m1".into(), &f.workflow, 1, 1, true, None)
        .await
        .unwrap();
    runs.mark_done("m2".into(), &f.workflow, 1, 1, false, None)
        .await
        .unwrap();
    let one = expected(&f, 1, None);
    assert_eq!(progress_events(&f).await, vec![one.clone(), one]);
    runs.mark_done("m1".into(), &f.workflow, 1, 1, true, None)
        .await
        .unwrap();
    assert_eq!(
        progress_events(&f).await.len(),
        2,
        "a replayed mark journaled"
    );
    f.storage.close().await.unwrap();
}
