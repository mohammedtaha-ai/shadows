//! Planner approval boundaries through the real MCP transport.
use super::*;

async fn running(l: &listening::Listening) -> shadows_core::OperationId {
    let op = app::start_settled(&l.app, "hang").await;
    for _ in 0..200 {
        if l.app.handles.running_for(&l.app.thread).await.as_ref() == Some(&op) {
            return op;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("turn never became live")
}

#[tokio::test]
async fn the_planner_approves_a_vision_card_and_replay_appends_nothing() {
    let l = listening_app().await;
    let op = running(&l).await;
    let c = thread_client(&l, &l.app.thread).await;
    let p = ok(
        &c,
        "vision_propose",
        json!({"action":"add","field":"goals","text":"Ship"}),
    )
    .await;
    // Same durable entry shape committed by Send now, within the current operation.
    let steered = shadows_core::Threads::append_thread_entry_for_test(
        &l.app.storage,
        &l.app.thread,
        shadows_core::testing::NewThreadEntry {
            kind: shadows_core::ThreadEntryKind::UserMessage,
            author: shadows_core::Actor::user("local"),
            body: "Approve this now",
            refs: &[],
            card: None,
            operation_id: Some(op.as_str()),
        },
    )
    .await
    .unwrap();
    let args = json!({"proposal_id":p["id"]});
    let first = ok(&c, "vision_approve", args.clone()).await;
    let cursor = shadows_core::Events::current_cursor_for_test(&l.app.storage)
        .await
        .unwrap();
    assert_eq!(ok(&c, "vision_approve", args).await, first);
    assert_eq!(
        shadows_core::Events::current_cursor_for_test(&l.app.storage)
            .await
            .unwrap(),
        cursor
    );
    let proposals = l
        .app
        .core
        .design()
        .vision_proposals(&l.app.project)
        .await
        .unwrap();
    let by = proposals[0].approved_by.as_ref().unwrap();
    assert_eq!(by.agent, "planner:claude-code");
    assert_eq!(by.principal, "user:local");
    assert_eq!(by.request.as_ref().unwrap().thread_id, l.app.thread);
    assert_eq!(by.request.as_ref().unwrap().entry_id, steered.id);
    let kind: String = sqlx::query_scalar("SELECT kind FROM thread_entry WHERE id=?")
        .bind(by.request.as_ref().unwrap().entry_id.as_str())
        .fetch_one(l.app.storage.reader())
        .await
        .unwrap();
    assert_eq!(kind, "UserMessage");
    let audit: (String, String) = sqlx::query_as(
        "SELECT audit_principal,audit_agent FROM command_record
         WHERE command_kind='VisionProposalApprove'",
    )
    .fetch_one(l.app.storage.reader())
    .await
    .unwrap();
    assert_eq!(audit, ("user:local".into(), "planner:claude-code".into()));
    c.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

async fn vision(l: &listening::Listening) {
    let ops = [
        VisionField::Purpose,
        VisionField::Users,
        VisionField::Goals,
        VisionField::Boundaries,
        VisionField::TechnicalDirection,
    ]
    .into_iter()
    .map(|field| DesignOp::VisionItemAdd {
        field,
        text: "Complete".into(),
    })
    .collect();
    l.app
        .core
        .plans()
        .edit_design("vision".into(), &l.app.project, 0, ops)
        .await
        .unwrap();
}

#[tokio::test]
async fn outdated_superseded_and_already_approved_cards_are_refused() {
    let l = listening_app().await;
    vision(&l).await;
    let op = running(&l).await;
    let c = thread_client(&l, &l.app.thread).await;
    let stale = ok(
        &c,
        "vision_propose",
        json!({"action":"edit","item_id":"G1","text":"First"}),
    )
    .await;
    l.app
        .core
        .plans()
        .edit_design(
            "change-goal".into(),
            &l.app.project,
            1,
            vec![DesignOp::VisionItemPut {
                id: "G1".into(),
                text: "Changed".into(),
            }],
        )
        .await
        .unwrap();
    let (error, _) =
        listening::call(&c, "vision_approve", json!({"proposal_id":stale["id"]})).await;
    assert!(error);
    let replaced = ok(
        &c,
        "vision_propose",
        json!({"action":"edit","item_id":"G1","text":"Next"}),
    )
    .await;
    let latest = ok(
        &c,
        "vision_propose",
        json!({"action":"edit","item_id":"G1","text":"Final",
        "replaces":replaced["id"]}),
    )
    .await;
    let (error, _) =
        listening::call(&c, "vision_approve", json!({"proposal_id":replaced["id"]})).await;
    assert!(error);
    ok(&c, "vision_approve", json!({"proposal_id":latest["id"]})).await;
    l.app.core.turns().stop(&op).await.unwrap();
    // `stop` only asks: the turn ends, and its handle goes, a moment later.
    app::wait_terminal(&l.app, &op).await;
    for _ in 0..200 {
        if l.app.handles.running_for(&l.app.thread).await.is_none() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    running(&l).await;
    let (error, _) =
        listening::call(&c, "vision_approve", json!({"proposal_id":latest["id"]})).await;
    assert!(
        error,
        "new operation cannot replay another operation's approval"
    );
    c.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn the_planner_refuses_waiver_maps_but_the_person_approves() {
    let l = listening::listening_unseeded().await;
    vision(&l).await;
    running(&l).await;
    let c = thread_client(&l, &l.app.thread).await;
    let parts: Vec<_> = ["backend", "database", "api"]
        .into_iter()
        .map(|kind| {
            json!({"key":kind,"content":{"title":kind,"responsibility":"owns this part",
            "design":"","kind":kind,"serves":["G1"],"folders":[kind]}})
        })
        .collect();
    let p = ok(
        &c,
        "map_propose",
        json!({"action":"first","parts":parts,
        "waivers":[{"part":"frontend","reason":"Terminal tool"}]}),
    )
    .await;
    let (error, text) = listening::call(&c, "map_approve", json!({"proposal_id":p["id"]})).await;
    assert!(error && text.contains("person's button"), "{text}");
    assert_eq!(
        l.app
            .core
            .design()
            .map_proposals(&l.app.project)
            .await
            .unwrap()[0]
            .status,
        shadows_core::ProposalStatus::Pending
    );
    l.app
        .core
        .design()
        .approve_map_proposal(
            "human-map".into(),
            &l.app.project,
            p["id"].as_str().unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        l.app
            .core
            .design()
            .map_proposals(&l.app.project)
            .await
            .unwrap()[0]
            .approved_by
            .as_ref()
            .unwrap()
            .agent,
        "human"
    );
    c.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn the_planner_approves_a_map_without_waivers_and_a_plan() {
    let l = listening::listening_unseeded().await;
    vision(&l).await;
    running(&l).await;
    let c = thread_client(&l, &l.app.thread).await;
    let parts: Vec<_> = ["backend", "database", "api", "frontend"]
        .into_iter()
        .map(|kind| {
            json!({"key":kind,"content":{"title":kind,"responsibility":"owns this part",
            "design":"","kind":kind,"serves":["G1"],"folders":[kind]}})
        })
        .collect();
    let p = ok(&c, "map_propose", json!({"action":"first","parts":parts})).await;
    ok(&c, "map_approve", json!({"proposal_id":p["id"]})).await;
    assert_eq!(
        l.app
            .core
            .design()
            .map_proposals(&l.app.project)
            .await
            .unwrap()[0]
            .approved_by
            .as_ref()
            .unwrap()
            .agent,
        "planner:claude-code"
    );
    // Plans come after the roadmap (§23.6); the plan is tied to a part of the map.
    let part = shadows_core::AppCore::design_for_tests(l.app.storage.clone())
        .seed_roadmap_for_tests(&l.app.project)
        .await
        .unwrap();
    let draft = ok(
        &c,
        "draft_start",
        json!({"title":"Plan","goal":"Ship","parts":[part]}),
    )
    .await;
    let edited = ok(
        &c,
        "plan_edit",
        json!({"workflow_id":draft["workflow_id"],
        "expected_revision":0,"ops":[plan::add(1)]}),
    )
    .await;
    ok(
        &c,
        "plan_approve",
        json!({"workflow_id":draft["workflow_id"],
        "expected_revision":edited["revision"]}),
    )
    .await;
    let id = shadows_core::WorkflowId::from_literal(draft["workflow_id"].as_str().unwrap());
    let plan = l.app.core.plans().get(&id).await.unwrap();
    assert_eq!(plan.approved_by.unwrap().agent, "planner:claude-code");
    let body: String =
        sqlx::query_scalar("SELECT body FROM thread_entry WHERE kind='PlanApproved'")
            .fetch_one(l.app.storage.reader())
            .await
            .unwrap();
    assert_eq!(body, "Plan v1 approved by the Planner at your request");
    c.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}
