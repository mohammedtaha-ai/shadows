use super::*;

async fn start_ask(w: &World, thread: &ThreadId) -> shadows_core::OperationId {
    let mut turn = request("fake-large", "high");
    turn.prompt = "ask-permission".into();
    w.core.turns().send(thread.clone(), turn).await.unwrap()
}

async fn waiting(w: &World, count: usize) -> Vec<shadows_core::WaitingPermission> {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let list = w.core.harness().waiting_permissions().await;
            if list.len() == count {
                return list;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

async fn ended(w: &World, op: &shadows_core::OperationId) {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if shadows_core::Turns::get_operation_for_test(&w.storage, op)
                .await
                .unwrap()
                .finished_at
                .is_some()
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

async fn close(w: &World) {
    w.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn ending_a_turn_settles_its_unanswered_requests() {
    let w = world().await;
    let mut turn = request("fake-large", "high");
    turn.prompt = "end-with-permission".into();
    let op = w.core.turns().send(w.thread.clone(), turn).await.unwrap();
    waiting(&w, 1).await;
    std::fs::write(w.home.join("end-permission-turn"), "release").unwrap();
    ended(&w, &op).await;
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.kind == ThreadEntryKind::PermissionAsked)
    );
    assert!(w.core.harness().waiting_permissions().await.is_empty());
    assert!(entries.iter().any(|entry| {
        entry
            .card
            .as_ref()
            .is_some_and(|card| card["reason"] == "ended")
    }));
    close(&w).await;
}

#[tokio::test]
async fn disconnecting_settles_requests_before_the_turn_is_collected() {
    use shadows_agent::acp::{SessionSetup, SessionStart};
    let w = world().await;
    let opened = w.sessions.open(&w.thread).await.unwrap();
    let other = opened
        .connection()
        .start_session(&w.home, SessionStart::New, &SessionSetup::default())
        .await
        .unwrap();
    let op = start_ask(&w, &w.thread).await;
    let ask = waiting(&w, 1).await.remove(0);
    assert!(
        opened
            .connection()
            .prompt(&other.session_id, "exit", &[])
            .await
            .is_err()
    );
    assert!(matches!(
        opened
            .connection()
            .answer_permission(&ask.request_id, "allow_once")
            .await,
        Err(shadows_agent::acp::PermissionError::Settled)
    ));
    ended(&w, &op).await;
    assert!(w.core.harness().waiting_permissions().await.is_empty());
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    assert!(entries.iter().any(|entry| {
        entry
            .card
            .as_ref()
            .is_some_and(|card| card["request_id"] == ask.request_id && card["reason"] == "closed")
    }));
    let project = w.core.agents().assistant_project().await.unwrap();
    let events = w
        .core
        .events()
        .read_project_events_after(core_events::EventCursor(0), &project, 100)
        .await
        .unwrap();
    assert!(events.iter().any(|event| event.kind == "PermissionsChanged"
        && serde_json::from_str::<Value>(&event.payload_json).unwrap()["state"] == "refused"));
    close(&w).await;
}

#[tokio::test]
async fn concurrent_equal_answers_replay_the_winner() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    let request = waiting(&w, 1).await.remove(0);
    let (first, second) = tokio::join!(
        w.core.harness().answer_permission(
            "concurrent".into(),
            &w.thread,
            &request.request_id,
            "allow_once"
        ),
        w.core.harness().answer_permission(
            "concurrent".into(),
            &w.thread,
            &request.request_id,
            "allow_once"
        ),
    );
    assert_eq!(
        serde_json::to_value(first.unwrap()).unwrap(),
        serde_json::to_value(second.unwrap()).unwrap()
    );
    ended(&w, &op).await;
    assert_eq!(
        w.core
            .threads()
            .entries(&w.thread)
            .await
            .unwrap()
            .iter()
            .filter(|entry| entry.kind == ThreadEntryKind::PermissionAnswered)
            .count(),
        1
    );
    close(&w).await;
}

#[tokio::test]
async fn a_replay_does_not_answer_a_later_request() {
    let w = world().await;
    let mut turn = request("fake-large", "high");
    turn.prompt = "two-permissions".into();
    let op = w.core.turns().send(w.thread.clone(), turn).await.unwrap();
    let first = waiting(&w, 1).await.remove(0);
    w.core
        .harness()
        .answer_permission(
            "first-answer".into(),
            &w.thread,
            &first.request_id,
            "allow_once",
        )
        .await
        .unwrap();
    let second = waiting(&w, 1).await.remove(0);
    assert_ne!(first.request_id, second.request_id);
    w.core
        .harness()
        .answer_permission(
            "first-answer".into(),
            &w.thread,
            &first.request_id,
            "allow_once",
        )
        .await
        .unwrap();
    assert_eq!(waiting(&w, 1).await[0].request_id, second.request_id);
    w.core
        .harness()
        .answer_permission(
            "second-answer".into(),
            &w.thread,
            &second.request_id,
            "reject_once",
        )
        .await
        .unwrap();
    ended(&w, &op).await;
    close(&w).await;
}

#[tokio::test]
async fn a_failed_permission_record_leaves_the_request_waiting() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    let request = waiting(&w, 1).await.remove(0);
    sqlx::query(
        "CREATE TRIGGER refuse_permission_record BEFORE INSERT ON command_record
        WHEN NEW.command_kind='thread.permission_answer'
        BEGIN SELECT RAISE(ABORT,'synthetic write failure'); END",
    )
    .execute(w.storage.reader())
    .await
    .unwrap();
    assert!(
        w.core
            .harness()
            .answer_permission(
                "write-failure".into(),
                &w.thread,
                &request.request_id,
                "allow_once"
            )
            .await
            .is_err()
    );
    assert_eq!(waiting(&w, 1).await[0].request_id, request.request_id);
    assert!(
        w.core
            .threads()
            .entries(&w.thread)
            .await
            .unwrap()
            .iter()
            .all(|entry| entry.kind != ThreadEntryKind::PermissionAnswered)
    );
    sqlx::query("DROP TRIGGER refuse_permission_record")
        .execute(w.storage.reader())
        .await
        .unwrap();
    w.core
        .harness()
        .answer_permission(
            "write-failure".into(),
            &w.thread,
            &request.request_id,
            "allow_once",
        )
        .await
        .unwrap();
    ended(&w, &op).await;
    close(&w).await;
}

#[tokio::test]
async fn stopping_all_sessions_refuses_waiting_requests() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    waiting(&w, 1).await;
    w.sessions.close_all().await.unwrap();
    ended(&w, &op).await;
    assert!(w.core.harness().waiting_permissions().await.is_empty());
    assert!(
        w.core
            .threads()
            .entries(&w.thread)
            .await
            .unwrap()
            .iter()
            .any(|entry| entry
                .card
                .as_ref()
                .is_some_and(|card| card["reason"] == "stopped"))
    );
    close(&w).await;
}

#[tokio::test]
async fn an_aylul_request_waits_and_is_listed() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    let list = waiting(&w, 1).await;
    assert_eq!(list[0].thread_id, w.thread);
    assert_eq!(list[0].agent, "aylul");
    assert_eq!(list[0].title, "Run echo probe");
    assert_eq!(list[0].options[0].option_id, "allow_once");
    assert!(list[0].asked_at.ends_with('Z'));
    assert!(
        shadows_core::Turns::get_operation_for_test(&w.storage, &op)
            .await
            .unwrap()
            .finished_at
            .is_none()
    );
    w.core.turns().stop(&op).await.unwrap();
    close(&w).await;
}

#[tokio::test]
async fn answering_runs_the_tool_and_records_by_you() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    let list = waiting(&w, 1).await;
    let answer = w
        .core
        .harness()
        .answer_permission("allow".into(), &w.thread, &list[0].request_id, "allow_once")
        .await
        .unwrap();
    assert_eq!(answer.kind, "allow_once");
    ended(&w, &op).await;
    assert!(w.core.harness().waiting_permissions().await.is_empty());
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    let asked = entries
        .iter()
        .find(|entry| entry.kind == ThreadEntryKind::PermissionAsked)
        .unwrap();
    assert_eq!(asked.author.id, "aylul");
    let entry = entries
        .iter()
        .find(|entry| entry.kind == ThreadEntryKind::PermissionAnswered)
        .unwrap();
    assert_eq!(entry.author.kind, "User");
    assert_eq!(entry.body, "Allow");
    let audit: (String, String) = sqlx::query_as(
        "SELECT audit_principal,audit_agent FROM command_record WHERE command_id='allow'",
    )
    .fetch_one(w.storage.reader())
    .await
    .unwrap();
    assert_eq!(audit, ("user:local".into(), "human".into()));
    assert_eq!(
        entry.card.as_ref().unwrap()["by"],
        json!({"agent":"human","principal":"user:local"})
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.body.contains("permission: allow_once"))
    );
    close(&w).await;
}

#[tokio::test]
async fn a_rejected_request_ends_rejected_and_the_turn_goes_on() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    let list = waiting(&w, 1).await;
    w.core
        .harness()
        .answer_permission(
            "reject".into(),
            &w.thread,
            &list[0].request_id,
            "reject_once",
        )
        .await
        .unwrap();
    ended(&w, &op).await;
    assert!(
        w.core
            .threads()
            .entries(&w.thread)
            .await
            .unwrap()
            .iter()
            .any(|entry| entry.body.contains("permission: reject_once"))
    );
    close(&w).await;
}

#[tokio::test]
async fn an_option_not_offered_is_refused() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    let list = waiting(&w, 1).await;
    let err = w
        .core
        .harness()
        .answer_permission("invalid".into(), &w.thread, &list[0].request_id, "invented")
        .await
        .unwrap_err();
    assert_eq!(
        CoreError::from(err).code(),
        shadows_core::ErrorCode::PermissionOptionNotOffered
    );
    assert_eq!(waiting(&w, 1).await[0].request_id, list[0].request_id);
    w.core.turns().stop(&op).await.unwrap();
    close(&w).await;
}

#[tokio::test]
async fn a_second_answer_is_settled() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    let list = waiting(&w, 1).await;
    w.core
        .harness()
        .answer_permission("first".into(), &w.thread, &list[0].request_id, "allow_once")
        .await
        .unwrap();
    let err = w
        .core
        .harness()
        .answer_permission(
            "second".into(),
            &w.thread,
            &list[0].request_id,
            "allow_once",
        )
        .await
        .unwrap_err();
    assert_eq!(
        CoreError::from(err).code(),
        shadows_core::ErrorCode::PermissionSettled
    );
    ended(&w, &op).await;
    close(&w).await;
}

#[tokio::test]
async fn an_equal_replay_answers_the_first_result() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    let list = waiting(&w, 1).await;
    let first = w
        .core
        .harness()
        .answer_permission(
            "replay".into(),
            &w.thread,
            &list[0].request_id,
            "allow_once",
        )
        .await
        .unwrap();
    ended(&w, &op).await;
    w.sessions.terminate(&w.thread).await.unwrap();
    let replay = w
        .core
        .harness()
        .answer_permission(
            "replay".into(),
            &w.thread,
            &list[0].request_id,
            "allow_once",
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    let err = w
        .core
        .harness()
        .answer_permission(
            "replay".into(),
            &w.thread,
            &list[0].request_id,
            "reject_once",
        )
        .await
        .unwrap_err();
    assert_eq!(
        CoreError::from(err).code(),
        shadows_core::ErrorCode::CommandConflict
    );
    close(&w).await;
}

#[tokio::test]
async fn cancelling_while_asked_ends_refused_and_publishes() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    waiting(&w, 1).await;
    w.core.turns().stop(&op).await.unwrap();
    ended(&w, &op).await;
    assert!(w.core.harness().waiting_permissions().await.is_empty());
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    assert!(entries.iter().any(|entry| {
        entry.kind == ThreadEntryKind::PermissionRefused
            && entry
                .card
                .as_ref()
                .is_some_and(|card| card["reason"] == "cancelled")
    }));
    close(&w).await;
}

#[tokio::test]
async fn closing_the_session_while_asked_ends_refused() {
    let w = world().await;
    let op = start_ask(&w, &w.thread).await;
    waiting(&w, 1).await;
    w.sessions.terminate(&w.thread).await.unwrap();
    ended(&w, &op).await;
    assert!(w.core.harness().waiting_permissions().await.is_empty());
    assert!(
        w.core
            .threads()
            .entries(&w.thread)
            .await
            .unwrap()
            .iter()
            .any(|entry| entry
                .card
                .as_ref()
                .is_some_and(|card| card["reason"] == "closed"))
    );
    close(&w).await;
}

#[tokio::test]
async fn waiting_is_listed_oldest_first_across_agents() {
    let w = world().await;
    let other = another(&w, "Work").await;
    let first = start_ask(&w, &w.thread).await;
    let earlier = waiting(&w, 1).await.remove(0);
    let second = start_ask(&w, &other).await;
    let list = waiting(&w, 2).await;
    assert_eq!(list[0].request_id, earlier.request_id);
    assert_eq!(list[1].agent, "work");
    assert!(
        (list[0].asked_at.clone(), list[0].request_id.clone())
            <= (list[1].asked_at.clone(), list[1].request_id.clone())
    );
    w.core.turns().stop(&first).await.unwrap();
    w.core.turns().stop(&second).await.unwrap();
    close(&w).await;
}

#[tokio::test]
async fn permissions_changed_reaches_the_aylul_project_stream() {
    let w = world().await;
    let project = w.core.agents().assistant_project().await.unwrap();
    let mut stream = w.core.events().subscribe_project(project, 0).await.unwrap();
    let op = start_ask(&w, &w.thread).await;
    let list = waiting(&w, 1).await;
    let event = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let shadows_core::Delivery::Durable(event) = stream.next().await.unwrap()
                && event.kind == "PermissionsChanged"
            {
                return event;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(event.thread_id.as_ref(), Some(&w.thread));
    assert_eq!(
        serde_json::from_str::<Value>(&event.payload_json).unwrap()["request_id"],
        list[0].request_id
    );
    assert_eq!(w.core.harness().waiting_permissions().await.len(), 1);
    w.core.turns().stop(&op).await.unwrap();
    close(&w).await;
}
