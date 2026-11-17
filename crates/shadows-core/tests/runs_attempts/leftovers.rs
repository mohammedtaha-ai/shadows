//! A new attempt carries on with the files an ended attempt left.
use super::*;

async fn leftover(f: &Fixture, state: &str) {
    let first = f.start("first", 3).await.unwrap();
    f.core.runs().stop(&first.id).await.unwrap();
    if state == "interrupted" {
        sqlx::query("UPDATE run_attempts SET state='interrupted' WHERE id=?")
            .bind(first.id.as_str())
            .execute(f.storage.reader())
            .await
            .unwrap();
    }
    std::fs::create_dir_all(f.root.join("backend/app")).unwrap();
    std::fs::write(f.root.join("backend/app/a.php"), "unfinished\n").unwrap();
}

async fn continued(state: &str) {
    let f = fixture().await;
    leftover(&f, state).await;
    let next = f.start("next", 3).await.unwrap();
    let entries = f.core.threads().entries(&next.thread_id).await.unwrap();
    assert!(
        entries.iter().any(|entry| {
            entry.kind == ThreadEntryKind::UserMessage
                && entry.body.contains("Attempt 1")
                && entry.body.contains("backend/app/a.php")
        }),
        "{entries:?}"
    );
    assert_eq!(
        std::fs::read_to_string(f.root.join("backend/app/a.php")).unwrap(),
        "unfinished\n"
    );
    f.finish().await;
}

#[tokio::test]
async fn a_start_after_a_stopped_attempt_continues_from_its_files() {
    continued("stopped").await;
}

#[tokio::test]
async fn a_start_after_an_interrupted_attempt_continues_from_its_files() {
    continued("interrupted").await;
}

#[tokio::test]
async fn the_task_read_allows_a_start_over_leftovers() {
    let f = fixture().await;
    leftover(&f, "stopped").await;
    assert!(
        f.core
            .runs()
            .task(&f.workflow, 1, 3)
            .await
            .unwrap()
            .start
            .allowed
    );
    f.finish().await;
}

#[tokio::test]
async fn dirty_files_without_an_ended_attempt_still_refuse_start() {
    let f = fixture().await;
    std::fs::create_dir_all(f.root.join("backend/app")).unwrap();
    std::fs::write(f.root.join("backend/app/a.php"), "personal\n").unwrap();
    assert!(matches!(
        f.start("dirty", 3).await,
        Err(RunsError::DirtyScope { .. })
    ));
    let thread = f.executor_thread(3).await;
    f.chain_attempt(&thread, 3, 1, "committed", None, true)
        .await;
    f.core
        .runs()
        .mark_done("done".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    assert!(matches!(
        f.start("dirty-after-commit", 3).await,
        Err(RunsError::DirtyScope { .. })
    ));
    f.finish().await;
}
