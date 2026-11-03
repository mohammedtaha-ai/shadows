use super::EventsStore;
use core_db::{Actor, JournalEntry, Storage};
use core_projects::Projects;
use std::sync::Arc;

async fn fixture(kinds: &[&str]) -> (tempfile::TempDir, EventsStore, Vec<i64>) {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let project = Projects::new(storage.clone())
        .create(
            "project".into(),
            "project",
            "Project",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let project_id = project.id.to_string();
    let kinds: Vec<_> = kinds.iter().map(|kind| (*kind).to_owned()).collect();
    let seq = storage.write_txn(move |conn| Box::pin(async move {
        sqlx::query("INSERT INTO runtime_instance (id, version, started_at) VALUES ('r','test','')")
            .execute(&mut *conn).await?;
        for (thread, op) in [("t1", "op1"), ("t2", "op2")] {
            sqlx::query("INSERT INTO planning_thread (id, project_id, title, status, created_at)
                         VALUES (?, ?, 'Thread', 'Open', '')")
                .bind(thread).bind(&project_id).execute(&mut *conn).await?;
            sqlx::query("INSERT INTO operation (id, kind, thread_id, runtime_instance_id, created_at)
                         VALUES (?, 'PlannerTurn', ?, 'r', '')")
                .bind(op).bind(thread).execute(&mut *conn).await?;
        }
        let mut seq = Vec::new();
        for (index, kind) in kinds.into_iter().enumerate() {
            let (thread, op) = if index % 2 == 0 { ("t1", "op1") } else { ("t2", "op2") };
            let event = JournalEntry::new(kind, Actor::system())
                .with_thread(thread)
                .with_operation(op);
            seq.push(JournalEntry::append_event(conn, event, &Storage::now()).await?);
        }
        Ok(seq)
    })).await.unwrap();
    (tmp, EventsStore::with_storage(storage), seq)
}

#[tokio::test]
async fn ended_turns_come_back_in_seq_order_after_the_cursor() {
    let (_tmp, store, seq) = fixture(&[
        "OperationFailed",
        "OperationStarted",
        "OperationCompleted",
        "OperationCancelled",
    ])
    .await;
    let first = store.ended_turns_after(seq[0], 1).await.unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].seq, seq[2]);
    assert_eq!(first[0].thread_id.as_str(), "t1");
    assert_eq!(first[0].operation_id.as_str(), "op1");
    let rest = store.ended_turns_after(first[0].seq, 10).await.unwrap();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].seq, seq[3]);
    assert_eq!(rest[0].thread_id.as_str(), "t2");
    assert_eq!(store.last_seq().await.unwrap(), seq[3]);
}

#[tokio::test]
async fn an_interrupted_turn_is_an_ended_turn() {
    let (_tmp, store, seq) = fixture(&["OperationInterrupted"]).await;
    let rows = store.ended_turns_after(0, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].seq, seq[0]);
    assert_eq!(rows[0].outcome, crate::EndedOutcome::Interrupted);
}
