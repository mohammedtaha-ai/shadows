//! Through the real transports: a Planner's new plan started from an
//! outcome-anchored conversation belongs to that outcome.
use super::*;
use shadows_core::{DesignAnchor, OperationId, OutcomeContent, OutcomeId, ProjectId, ThreadId};

async fn running(l: &listening::Listening, thread: &ThreadId) -> OperationId {
    let op = app::start_on(&l.app, thread.as_str(), "hang", app::default_settings()).await;
    for _ in 0..200 {
        if l.app.handles.running_for(thread).await.as_ref() == Some(&op) {
            return op;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("turn never became live")
}

/// A new conversation of the app's project with a running turn, and its Planner.
async fn conversation(l: &listening::Listening, name: &str) -> (ThreadId, listening::Client) {
    let thread = l
        .app
        .core
        .threads()
        .create(name.into(), &l.app.project, name, None, None)
        .await
        .unwrap()
        .id;
    running(l, &thread).await;
    let planner = thread_client(l, &thread).await;
    (thread, planner)
}

/// Ties the draft to the seeded part, gives it a task and asks the Planner
/// to approve it: the tool call's refusal flag and text.
async fn planner_approves(
    l: &listening::Listening,
    planner: &listening::Client,
    draft: &serde_json::Value,
) -> (bool, String) {
    let plans = shadows_core::AppCore::plans_for_tests(l.app.storage.clone());
    let part = shadows_core::AppCore::design_for_tests(l.app.storage.clone())
        .seed_roadmap_for_tests(&l.app.project)
        .await
        .unwrap();
    let plan = shadows_core::PlanId::from_literal(draft["plan_id"].as_str().unwrap());
    plans
        .link_plan_part_for_test(&l.app.project, &plan, &part)
        .await
        .unwrap();
    let edited = ok(
        planner,
        "plan_edit",
        json!({"workflow_id":draft["workflow_id"],"expected_revision":0,"ops":[plan::add(1)]}),
    )
    .await;
    listening::call(
        planner,
        "plan_approve",
        json!({"workflow_id":draft["workflow_id"],"expected_revision":edited["revision"]}),
    )
    .await
}

async fn workspace_revision(l: &listening::Listening, p: &ProjectId) -> i64 {
    l.app.core.design().vision(p).await.unwrap().revision
}

async fn outcome_revision(l: &listening::Listening, p: &ProjectId, id: &OutcomeId) -> i64 {
    l.app
        .core
        .plans()
        .design_outcome(p, id)
        .await
        .unwrap()
        .outcome
        .revision
}

async fn design_events(l: &listening::Listening, p: &ProjectId) -> Vec<serde_json::Value> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT payload_json, actor_id FROM durable_event \
         WHERE kind='ProjectDesignChanged' AND project_id=? ORDER BY seq",
    )
    .bind(p.as_str())
    .fetch_all(l.app.storage.reader())
    .await
    .unwrap();
    rows.into_iter()
        .map(|(payload, actor)| {
            let mut payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
            payload["actor"] = json!(actor);
            payload
        })
        .collect()
}

async fn draft_count(l: &listening::Listening, p: &ProjectId) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow w JOIN plan p ON p.id=w.plan_id WHERE p.project_id=?",
    )
    .bind(p.as_str())
    .fetch_one(l.app.storage.reader())
    .await
    .unwrap()
}

async fn anchor(l: &listening::Listening, thread: &ThreadId, id: &str, to: Option<DesignAnchor>) {
    l.app
        .core
        .design()
        .set_thread_anchor(id.into(), thread, to)
        .await
        .unwrap();
}

#[tokio::test]
async fn an_outcome_anchored_draft_links_once_and_a_removed_anchor_rolls_back() {
    let l = listening::listening_app().await;
    let (p, thread) = (l.app.project.clone(), l.app.thread.clone());
    let ids: Vec<OutcomeId> = (0..3).map(|_| OutcomeId::generate()).collect();
    let ops = ids
        .iter()
        .enumerate()
        .map(|(i, id)| DesignOp::OutcomeCreate {
            id: id.clone(),
            parent: None,
            before: None,
            content: OutcomeContent {
                title: format!("Outcome {i}"),
                intended_result: "A result".into(),
                acceptance: vec![],
            },
        })
        .collect();
    let start = workspace_revision(&l, &p).await;
    l.app
        .core
        .plans()
        .edit_design("outcomes".into(), &p, start, ops)
        .await
        .unwrap();
    anchor(
        &l,
        &thread,
        "anchor-a",
        Some(DesignAnchor::Outcome(ids[0].clone())),
    )
    .await;
    let op = running(&l, &thread).await;
    let planner = thread_client(&l, &thread).await;

    let before = workspace_revision(&l, &p).await;
    let outcome_before = outcome_revision(&l, &p, &ids[0]).await;
    let events_before = design_events(&l, &p).await.len();
    let args = json!({"title":"Anchored","goal":"Ship"});
    let draft = ok(&planner, "draft_start", args.clone()).await;
    let plan_id = draft["plan_id"].as_str().unwrap().to_string();
    let linked = l
        .app
        .core
        .plans()
        .design_outcome(&p, &ids[0])
        .await
        .unwrap();
    assert_eq!(
        linked
            .plans
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>(),
        std::slice::from_ref(&plan_id)
    );
    assert_eq!(workspace_revision(&l, &p).await, before + 1);
    assert_eq!(outcome_revision(&l, &p, &ids[0]).await, outcome_before + 1);
    let events = design_events(&l, &p).await;
    assert_eq!(events.len(), events_before + 1);
    let last = events.last().unwrap();
    assert_eq!(last["changed_outcomes"], json!([ids[0].to_string()]));
    assert_eq!(last["actor"], json!(thread.to_string()));

    // The replay answers the same start and links nothing to a new anchor.
    anchor(
        &l,
        &thread,
        "anchor-b",
        Some(DesignAnchor::Outcome(ids[1].clone())),
    )
    .await;
    let replay = ok(&planner, "draft_start", args).await;
    assert_eq!(replay, draft);
    let other = l
        .app
        .core
        .plans()
        .design_outcome(&p, &ids[1])
        .await
        .unwrap();
    assert!(other.plans.is_empty());
    assert_eq!(workspace_revision(&l, &p).await, before + 1);
    assert_eq!(design_events(&l, &p).await.len(), events_before + 1);

    // An outcome editor save made with the revision read before the draft.
    let stale = l
        .app
        .core
        .plans()
        .edit_design(
            "stale-save".into(),
            &p,
            before,
            vec![DesignOp::OutcomePut {
                id: ids[0].clone(),
                content: OutcomeContent {
                    title: "Renamed".into(),
                    intended_result: "A result".into(),
                    acceptance: vec![],
                },
            }],
        )
        .await
        .unwrap_err();
    assert!(
        format!("{stale:?}").contains("RevisionConflict"),
        "{stale:?}"
    );
    // The Planner approves at the person's request: the anchored plan serves its outcome.
    let (error, text) = planner_approves(&l, &planner, &draft).await;
    assert!(!error, "{text}");

    planner.cancel().await.unwrap();
    l.app.core.turns().stop(&op).await.unwrap();

    // Control: a conversation with no anchor starts an unlinked draft.
    let (_, planner) = conversation(&l, "plain").await;
    let revision = workspace_revision(&l, &p).await;
    let drafts = draft_count(&l, &p).await;
    let plain = ok(
        &planner,
        "draft_start",
        json!({"title":"Plain","goal":"Ship"}),
    )
    .await;
    assert_eq!(draft_count(&l, &p).await, drafts + 1);
    assert_eq!(workspace_revision(&l, &p).await, revision);
    let first = l
        .app
        .core
        .plans()
        .design_outcome(&p, &ids[0])
        .await
        .unwrap();
    assert!(
        !first
            .plans
            .iter()
            .any(|id| id.to_string() == plain["plan_id"].as_str().unwrap())
    );
    let (error, text) = planner_approves(&l, &planner, &plain).await;
    assert!(error, "{text}");
    assert!(
        text.contains("plan has no roadmap outcome: link it on the Roadmap page"),
        "{text}"
    );
    planner.cancel().await.unwrap();

    // An anchored outcome that is gone refuses the start whole.
    let (gone_thread, planner) = conversation(&l, "gone").await;
    anchor(
        &l,
        &gone_thread,
        "anchor-c",
        Some(DesignAnchor::Outcome(ids[2].clone())),
    )
    .await;
    sqlx::query("DELETE FROM design_outcome WHERE id=?")
        .bind(ids[2].as_str())
        .execute(l.app.storage.reader())
        .await
        .unwrap();
    let (drafts, events) = (draft_count(&l, &p).await, design_events(&l, &p).await.len());
    let (error, text) = listening::call(
        &planner,
        "draft_start",
        json!({"title":"Gone","goal":"Ship"}),
    )
    .await;
    assert!(error, "{text}");
    assert!(!text.contains("no turn"), "{text}");
    assert_eq!(draft_count(&l, &p).await, drafts);
    assert_eq!(design_events(&l, &p).await.len(), events);
    assert_eq!(workspace_revision(&l, &p).await, revision);
    planner.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}
