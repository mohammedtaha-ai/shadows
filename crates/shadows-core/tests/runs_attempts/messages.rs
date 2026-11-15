//! Questions use real executor turns; they never become attempts or additions.
use super::*;
use core_runs::{AgentMessageId, AgentMessageState};

async fn waiting() -> (Fixture, Attempt, Grant, ThreadId) {
    let f = fixture().await;
    let planner = super::checks::prepare_planner(&f).await;
    f.hold();
    let attempt = f.start("work", 3).await.unwrap();
    let grant = planner_grant(&f).await;
    (f, attempt, grant, planner)
}

async fn committed(f: &Fixture, attempt: &Attempt) {
    let (project, thread) = (f.project.clone(), attempt.thread_id.clone());
    let scope = TaskScope {
        workflow: f.workflow.as_str().into(),
        version: 1,
        task: 3,
    };
    let (grant, _) = f
        .storage
        .write_txn(move |tx| {
            Box::pin(async move {
                shadows_core::Grants::issue_executor_grant_in(tx, &project, &thread, scope).await
            })
        })
        .await
        .unwrap();
    f.put("backend/app/a.php", "one\n");
    let sha = f.commit(&["backend/app/a.php"], "build");
    f.core
        .runs()
        .report(&grant, &sha, "Built.".into(), vec![])
        .await
        .unwrap();
}

async fn delivered(f: &Fixture, id: &AgentMessageId) -> String {
    for _ in 0..600 {
        let row: (String, Option<String>) =
            sqlx::query_as("SELECT state,reply FROM run_agent_messages WHERE id=?")
                .bind(id.as_str())
                .fetch_one(f.storage.reader())
                .await
                .unwrap();
        if row.0 == "delivered" {
            return row.1.unwrap_or_default();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("reply was not delivered");
}

async fn ended(f: &Fixture, attempt: &Attempt) {
    for _ in 0..600 {
        if f.core
            .turns()
            .operations(&attempt.thread_id)
            .await
            .unwrap()
            .iter()
            .any(|op| Some(&op.id) == attempt.operation_id.as_ref() && op.finished_at.is_some())
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("executor turn did not end");
}

#[tokio::test]
async fn message_changes_no_attempt() {
    let (f, attempt, grant, _) = waiting().await;
    let before =
        serde_json::to_value(f.core.runs().attempts(&f.workflow, 1, 3).await.unwrap()).unwrap();
    let message = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Why this choice?".into())
        .await
        .unwrap();
    assert_eq!(message.state, AgentMessageState::Waiting);
    assert_eq!(message.attempt_id, attempt.id);
    let after =
        serde_json::to_value(f.core.runs().attempts(&f.workflow, 1, 3).await.unwrap()).unwrap();
    assert_eq!(before, after);
    f.finish().await;
}

#[tokio::test]
async fn message_waits_while_committed_turn_runs() {
    let (f, attempt, grant, _) = waiting().await;
    committed(&f, &attempt).await;
    let message = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Which tests ran?".into())
        .await
        .unwrap();
    assert_eq!(message.state, AgentMessageState::Waiting);
    assert_eq!(
        f.core
            .turns()
            .operations(&attempt.thread_id)
            .await
            .unwrap()
            .len(),
        1
    );
    f.release();
    let reply = delivered(&f, &message.id).await;
    assert!(!reply.is_empty());
    assert_ne!(reply, "waiting");
    f.finish().await;
}

#[tokio::test]
async fn messages_go_one_at_a_time() {
    let (f, attempt, grant, _) = waiting().await;
    let runs = f.core.runs();
    let (a, b) = tokio::join!(
        runs.message_agent(&grant, &f.workflow, 3, "First question".into()),
        runs.message_agent(&grant, &f.workflow, 3, "Second question".into())
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    f.release();
    delivered(&f, &a.id).await;
    delivered(&f, &b.id).await;
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT operation_id FROM run_agent_messages ORDER BY ordinal")
            .fetch_all(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(ids.len(), 2);
    let ops = f.core.turns().operations(&attempt.thread_id).await.unwrap();
    let first = ops.iter().find(|o| o.id.as_str() == ids[0]).unwrap();
    let second = ops.iter().find(|o| o.id.as_str() == ids[1]).unwrap();
    assert!(first.finished_at.as_ref().unwrap() <= &second.created_at);
    assert_eq!(
        f.core
            .runs()
            .attempts(&f.workflow, 1, 3)
            .await
            .unwrap()
            .len(),
        1
    );
    f.finish().await;
}

#[tokio::test]
async fn same_text_is_one_message() {
    let (f, _, grant, _) = waiting().await;
    let runs = f.core.runs();
    let (a, b) = tokio::join!(
        runs.message_agent(&grant, &f.workflow, 3, " Why? ".into()),
        runs.message_agent(&grant, &f.workflow, 3, "Why?".into())
    );
    assert_eq!(a.unwrap().id, b.unwrap().id);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run_agent_messages")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
    f.finish().await;
}

#[tokio::test]
async fn blank_message_is_refused() {
    let (f, _, grant, _) = waiting().await;
    let error = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, " \n ".into())
        .await
        .unwrap_err();
    assert_eq!(error.code(), "empty_message");
    f.finish().await;
}

#[tokio::test]
async fn message_after_done_reopens_session() {
    let (f, attempt, grant, _) = waiting().await;
    committed(&f, &attempt).await;
    f.release();
    ended(&f, &attempt).await;
    f.core
        .runs()
        .mark_done("done".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    let message = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Explain your result.".into())
        .await
        .unwrap();
    assert!(!delivered(&f, &message.id).await.is_empty());
    assert!(f.core.runs().task(&f.workflow, 1, 3).await.unwrap().done);
    assert_eq!(
        f.core
            .runs()
            .attempts(&f.workflow, 1, 3)
            .await
            .unwrap()
            .len(),
        1
    );
    f.finish().await;
}

#[tokio::test]
async fn amend_waits_for_reply() {
    let (f, attempt, grant, _) = waiting().await;
    committed(&f, &attempt).await;
    f.core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Why?".into())
        .await
        .unwrap();
    let error = f
        .core
        .runs()
        .amend(&grant, &f.workflow, 3, "Change the result.".into())
        .await
        .unwrap_err();
    assert_eq!(error.code(), "agent_answering");
    f.finish().await;
}

#[tokio::test]
async fn done_waits_for_reply() {
    let (f, attempt, grant, _) = waiting().await;
    committed(&f, &attempt).await;
    let message = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Why?".into())
        .await
        .unwrap();
    let marked = f
        .core
        .runs()
        .mark_done("done".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    assert!(marked.done);
    assert_eq!(marked.start.code.as_deref(), Some("task_busy"));
    f.release();
    delivered(&f, &message.id).await;
    for _ in 0..600 {
        if f.core
            .runs()
            .task(&f.workflow, 1, 3)
            .await
            .unwrap()
            .start
            .allowed
        {
            f.finish().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the done attempt scope stayed held after its reply");
}

#[tokio::test]
async fn reply_reaches_planner_once() {
    let (f, _, grant, planner) = waiting().await;
    let message = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Why?".into())
        .await
        .unwrap();
    f.release();
    delivered(&f, &message.id).await;
    sqlx::query("UPDATE run_agent_messages SET state='replied',delivered_at=NULL WHERE id=?")
        .bind(message.id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    delivered(&f, &message.id).await;
    let notices = f.core.turns().queued(&planner).await.unwrap();
    assert_eq!(
        notices
            .iter()
            .filter(|n| n
                .notice
                .as_ref()
                .is_some_and(|notice| notice.card.get("reply").is_some()))
            .count(),
        1
    );
    f.finish().await;
}

#[tokio::test]
async fn reply_replay_keeps_planner_settings() {
    let (f, _, grant, planner) = waiting().await;
    let message = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Why?".into())
        .await
        .unwrap();
    f.release();
    delivered(&f, &message.id).await;
    for op in f.core.turns().operations(&planner).await.unwrap() {
        if op.finished_at.is_none() {
            shadows_core::Turns::mark_operation_failed_for_test(
                &f.storage,
                &op.id,
                shadows_core::testing::FailureStage::Prepare,
                "recorded only",
            )
            .await
            .unwrap();
        }
    }
    planner_turn(&f, "settings-changed", "claude-code", "fake-small").await;
    let runtime = shadows_core::testing::Runtime::start(f.storage.clone())
        .await
        .unwrap();
    shadows_core::Turns::create_pending_operation_for_test(
        &f.storage,
        &planner,
        runtime.instance_id(),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE run_agent_messages SET state='replied',delivered_at=NULL WHERE id=?")
        .bind(message.id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    delivered(&f, &message.id).await;
    let notices = f.core.turns().queued(&planner).await.unwrap();
    assert_eq!(
        notices
            .iter()
            .filter(|n| n
                .notice
                .as_ref()
                .is_some_and(|v| v.card.get("reply").is_some()))
            .count(),
        1
    );
    f.finish().await;
}

#[tokio::test]
async fn restart_delivers_an_undelivered_reply() {
    let (f, _, grant, planner) = waiting().await;
    let message = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Why?".into())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE run_agent_messages SET state='replied',reply='Saved answer',
        answered_at='2026-11-15T00:00:00Z' WHERE id=?",
    )
    .bind(message.id.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.core.runs().recover().await.unwrap();
    assert_eq!(delivered(&f, &message.id).await, "Saved answer");
    assert!(
        f.core
            .turns()
            .queued(&planner)
            .await
            .unwrap()
            .iter()
            .any(|n| n
                .notice
                .as_ref()
                .is_some_and(|notice| notice.card["reply"] == "Saved answer"))
    );
    f.finish().await;
}

#[tokio::test]
async fn restart_reads_a_finished_reply_turn() {
    let (f, attempt, grant, _) = waiting().await;
    let message = f
        .core
        .runs()
        .message_agent(&grant, &f.workflow, 3, "Why?".into())
        .await
        .unwrap();
    f.release();
    let reply = delivered(&f, &message.id).await;
    sqlx::query(
        "UPDATE run_agent_messages SET state='sent',reply=NULL,answered_at=NULL,
        delivered_at=NULL WHERE id=?",
    )
    .bind(message.id.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.core.runs().recover().await.unwrap();
    assert_eq!(delivered(&f, &message.id).await, reply);
    assert_eq!(
        f.core
            .runs()
            .attempts(&f.workflow, 1, attempt.task)
            .await
            .unwrap()
            .len(),
        1
    );
    f.finish().await;
}
