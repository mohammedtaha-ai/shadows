//! Standards delivered to Planner sessions.
use super::*;

const STANDARDS: &str = "[Shadows] The project's standards changed";

async fn save_additions(app: &App, command: &str, part: &str) {
    let path = format!("/api/projects/{}/standards/additions", app.project);
    let body = json!({ "command_id": command, "content": {
        "rules": [], "parts": [{ "name": part, "owns": "Payments." }]
    }});
    let (status, saved) = call(app, "PUT", &path, Some(body)).await;
    assert_eq!(status, 200, "{saved}");
}

#[tokio::test]
async fn every_turn_ends_with_the_stage_line() {
    let l = listening_app().await;
    let first = report(&l.app).await;
    let sent = blocks(&first);
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert!(sent[1].starts_with(STAGE_IDEA), "{}", sent[1]);
}

#[tokio::test]
async fn a_session_opens_with_the_effective_standards() {
    let l = listening_app().await;
    save_additions(&l.app, "s1", "billing").await;
    let first = report(&l.app).await;
    let append = first["append"].as_str().expect("append");
    assert!(append.contains("## Shadows standards (base v1; project additions v1)"));
    assert!(append.contains("- backend (never waived):"));
    assert!(append.contains("- billing (project):"));
    assert!(append.contains("S1 ["));
}

#[tokio::test]
async fn changed_standards_reach_the_next_turn_once() {
    let l = listening_app().await;
    reply(&l.app, "hi").await;
    save_additions(&l.app, "s1", "billing").await;
    let next = report(&l.app).await;
    let sent = blocks(&next);
    assert_eq!(sent.len(), 3, "{sent:?}");
    assert!(sent[1].starts_with(STANDARDS) && sent[1].contains("billing"));
    assert!(sent[2].starts_with(STAGE_IDEA));
    assert_eq!(blocks(&report(&l.app).await).len(), 2, "sent once");
}

#[tokio::test]
async fn additions_saved_after_session_open_reach_its_first_turn() {
    let l = listening_app().await;
    let path = format!("/api/threads/{}/session", l.app.thread);
    assert_eq!(post(&l.app, &path, json!({})).await.0, 200);
    save_additions(&l.app, "s1", "billing").await;
    let first = report(&l.app).await;
    let sent = blocks(&first);
    assert_eq!(sent.len(), 3, "{sent:?}");
    assert!(sent[1].starts_with(STANDARDS) && sent[1].contains("billing"));
    assert_eq!(blocks(&report(&l.app).await).len(), 2, "sent once");
}

#[tokio::test]
async fn a_thread_from_before_the_standards_gets_them_once() {
    let l = listening_app().await;
    reply(&l.app, "hi").await;
    sqlx::query("UPDATE agent_invocation SET standards_version = NULL")
        .execute(l.app.storage.reader())
        .await
        .unwrap();
    assert_eq!(blocks(&report(&l.app).await).len(), 3, "standards once");
    assert_eq!(blocks(&report(&l.app).await).len(), 2);
}

#[tokio::test]
async fn a_pending_turn_delivers_the_standards_version_it_recorded() {
    use shadows_core::testing::turn::{new_turn, turn_command};
    use shadows_core::testing::{PlannerTurn, PlannerTurnRequest};
    let l = listening_app().await;
    let app = &l.app;
    let opened = app.sessions.open(&app.thread).await.unwrap();
    let leased = app
        .sessions
        .lease_events(&app.thread, &opened)
        .await
        .unwrap();
    save_additions(app, "s1", "billing").await;
    let first = app
        .storage
        .current_standards_additions(&app.project)
        .await
        .unwrap()
        .unwrap();
    let settings = app::settings();
    let command = turn_command("pinned", &app.thread, "report", &settings);
    let mut turn = new_turn(&app.thread, &app.runtime, "report", &settings);
    turn.standards_additions_version = Some(&first.id);
    let started = app.storage.start_turn(&command, turn).await.unwrap();
    save_additions(app, "s2", "shipping").await;
    PlannerTurn::start(
        app.runtime.clone(),
        app.handles.clone(),
        app.sessions.clone(),
        opened,
        PlannerTurnRequest {
            thread_id: app.thread.clone(),
            harness: "claude-code".into(),
            operation_id: started.operation_id.clone(),
            prompt: "report".into(),
            settings,
            focus: None,
            continue_plan: None,
            client_tab: None,
            events: leased,
            on_completed: None,
        },
        app.bus.clone(),
    )
    .await
    .unwrap();
    assert_eq!(
        wait_terminal(app, &started.operation_id).await.status_kind,
        "Completed"
    );
    let delivered: Value = serde_json::from_str(&last_agent_entry(app).await.body).unwrap();
    let sent = blocks(&delivered);
    assert!(sent[1].contains("billing"), "{sent:?}");
    assert!(!sent[1].contains("shipping"), "{sent:?}");
    let next = report(app).await;
    assert!(blocks(&next)[1].starts_with(STANDARDS));
    assert!(blocks(&next)[1].contains("shipping"));
    assert_eq!(blocks(&report(app).await).len(), 2);
}
