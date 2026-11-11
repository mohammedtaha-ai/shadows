//! What each tool on `/mcp` does with plans: the reach of
//! a thread grant and a project grant, revision conflicts, replays, draft
//! refs, and text that survives the trip unchanged. The transport itself is
//! `mcp_server.rs`.

use serde_json::json;
use shadows_agent::policy;
use shadows_core::ThreadId;
use shadows_core::WorkflowId;
use shadows_core::testing::CommandContext;

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{App, ctx, other_project, start_settled};
use listening::{call, listening_app, ok, project_client, refused, thread_client};
use plan::{add, an_hour_ago, approved_v1, draft, draft_on, events_of, insert_draft_ref, task};

#[tokio::test]
async fn structure_writing_is_not_an_mcp_tool() {
    let l = listening_app().await;
    let planner = thread_client(&l, &l.app.thread).await;
    let (_, external) = project_client(&l).await;
    for tools in [
        planner.list_all_tools().await.unwrap(),
        external.list_all_tools().await.unwrap(),
    ] {
        assert!(tools.iter().all(|tool| !tool.name.contains("structure")));
    }

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// A turn of the app's thread that hangs, once `LiveHandles` sees it running.
async fn turn_running(app: &App) {
    let op = start_settled(app, "hang").await;
    for _ in 0..200 {
        if app.handles.running_for(&app.thread).await.as_ref() == Some(&op) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

async fn second_thread(app: &App) -> ThreadId {
    shadows_core::Threads::create_planning_thread_for_test(
        &app.storage,
        &ctx("second-thread", "thread.create"),
        &app.project,
        "Second",
        policy::CLAUDE_CODE,
        None,
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn a_thread_grant_reaches_every_plan_in_its_project() {
    let l = listening_app().await;
    let own = draft(&l.app).await.workflow_id;
    let other_thread = second_thread(&l.app).await;
    let other = draft_on(&l.app, &other_thread, "start-other")
        .await
        .workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;

    let read = ok(&planner, "workflow_get", json!({ "workflow_id": other })).await;
    assert_eq!(read["id"], other.as_str());
    let edit = json!({ "workflow_id": other, "expected_revision": 0, "ops": [add(1)] });
    assert_eq!(ok(&planner, "plan_edit", edit).await["revision"], 1);
    assert_eq!(plan::revision(&l.app, &other).await, 1);

    // A version is named even when it is the Planner's own plan.
    let outcome = ok(
        &planner,
        "plan_edit",
        json!({
            "workflow_id": own, "expected_revision": 0, "ops": [add(1)]
        }),
    )
    .await;
    assert_eq!(outcome["workflow_id"], json!(own));
    assert_eq!(outcome["revision"], 1);
    let events = events_of(&l.app, &l.app.thread, "WorkflowEdited").await;
    let edited: Vec<_> = events
        .iter()
        .filter_map(|event| event["workflow_id"].as_str())
        .collect();
    assert_eq!(edited.len(), 2);
    assert!(edited.contains(&other.as_str()));
    assert!(edited.contains(&own.as_str()));

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_project_grant_cannot_reach_another_project() {
    let l = listening_app().await;
    let (_, their_thread) = other_project(&l.app).await;
    let theirs = draft_on(&l.app, &their_thread, "start-theirs")
        .await
        .workflow_id;
    let (_, external) = project_client(&l).await;

    for (tool, args) in [
        ("workflow_get", json!({ "workflow_id": theirs })),
        ("task_get", json!({ "workflow_id": theirs, "number": 1 })),
        (
            "plan_edit",
            json!({ "workflow_id": theirs, "expected_revision": 0, "ops": [add(1)] }),
        ),
    ] {
        let text = refused(&external, tool, args).await;
        assert!(text.starts_with("GRANT_SCOPE: "), "{tool}: {text}");
    }
    let listed = ok(&external, "workflow_list", json!({})).await;
    assert_eq!(listed, json!([]));
    assert_eq!(plan::revision(&l.app, &theirs).await, 0);

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_project_grant_must_name_a_plan_to_read_or_edit() {
    let l = listening_app().await;
    draft(&l.app).await;
    let (_, external) = project_client(&l).await;
    for (tool, args) in [
        ("workflow_get", json!({})),
        (
            "workflow_get",
            json!({ "workflow_id": WorkflowId::generate() }),
        ),
        ("task_get", json!({ "number": 1 })),
        (
            "plan_edit",
            json!({ "expected_revision": 0, "ops": [add(1)] }),
        ),
    ] {
        let text = refused(&external, tool, args).await;
        assert!(text.starts_with("GRANT_SCOPE: "), "{tool}: {text}");
    }

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn parallel_edits_on_one_revision_conflict_and_the_loser_can_retry() {
    let l = listening_app().await;
    let workflow = draft(&l.app).await.workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;
    let first = json!({ "workflow_id": workflow, "expected_revision": 0, "ops": [add(1)] });
    let second = json!({ "workflow_id": workflow, "expected_revision": 0, "ops": [add(2)] });

    let (a, b) = tokio::join!(
        call(&planner, "plan_edit", first.clone()),
        call(&planner, "plan_edit", second.clone())
    );
    let (loser, text) = match (a, b) {
        ((false, _), (true, text)) => (second, text),
        ((true, text), (false, _)) => (first, text),
        other => panic!("expected exactly one conflict: {other:?}"),
    };
    assert!(
        text.starts_with("REVISION_CONFLICT: the plan changed; current revision is 1: "),
        "{text}"
    );
    assert!(text.ends_with("Read it with workflow_get, then rebuild your edit."));

    let mut retry = loser;
    retry["expected_revision"] = json!(1);
    let outcome = ok(&planner, "plan_edit", retry).await;
    assert_eq!(outcome["revision"], 2);
    let plan = shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .get_plan_for_test(&workflow)
        .await
        .unwrap();
    let numbers: Vec<u32> = plan.tasks.iter().map(|t| t.content.number).collect();
    assert_eq!(numbers, [1, 2]);

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_repeated_edit_returns_the_first_result() {
    let l = listening_app().await;
    let workflow = draft(&l.app).await.workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;
    let edit = json!({ "workflow_id": workflow, "expected_revision": 0, "ops": [add(1)] });
    let first = ok(&planner, "plan_edit", edit.clone()).await;
    let again = ok(&planner, "plan_edit", edit).await;
    assert_eq!(first, again);
    assert_eq!(first["revision"], 1);
    let events = events_of(&l.app, &l.app.thread, "WorkflowEdited").await;
    assert_eq!(events.len(), 1);

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn editing_a_frozen_plan_is_refused() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let planner = thread_client(&l, &l.app.thread).await;
    let edit = json!({ "workflow_id": v1, "expected_revision": 1, "ops": [add(3)] });
    let text = refused(&planner, "plan_edit", edit).await;
    assert!(text.starts_with("WORKFLOW_FROZEN_IMMUTABLE: "), "{text}");
    assert!(
        text.ends_with("Start a new version with draft_start."),
        "{text}"
    );
    assert_eq!(plan::revision(&l.app, &v1).await, 1);

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_draft_ref_replays_for_a_new_version_of_a_frozen_plan() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let (_, external) = project_client(&l).await;
    let draft_ref = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let plan_id = shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .get_plan_for_test(&v1)
        .await
        .unwrap()
        .plan_id;
    let start = json!({ "draft_ref": draft_ref, "plan_id": plan_id, "reason": "the API changed" });

    let v2 = ok(&external, "draft_start", start.clone()).await;
    assert_eq!(v2["version"], 2);
    let plan = shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .get_plan_for_test(&v1)
        .await
        .unwrap()
        .plan_id;
    assert_eq!(v2["plan_id"], json!(plan));
    let edit = json!({ "workflow_id": v2["workflow_id"], "expected_revision": 0, "ops": [add(3)] });
    assert_eq!(ok(&external, "plan_edit", edit).await["revision"], 1);

    let again = ok(&external, "draft_start", start).await;
    assert_eq!(again, v2);

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn an_external_start_answers_an_existing_plan_draft() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let plan_id = shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .get_plan_for_test(&v1)
        .await
        .unwrap()
        .plan_id;
    let (_, external) = project_client(&l).await;
    let r1 = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let v2 = ok(
        &external,
        "draft_start",
        json!({ "draft_ref": r1, "plan_id": plan_id, "reason": "the API changed" }),
    )
    .await;

    // A frozen source's plan already has a Draft, so a new ref answers it.
    let second_ref = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let again = ok(
        &external,
        "draft_start",
        json!({ "draft_ref": second_ref, "plan_id": plan_id }),
    )
    .await;
    assert_eq!(again, v2);

    let next_ref = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let answered = ok(
        &external,
        "draft_start",
        json!({ "draft_ref": next_ref, "plan_id": plan_id }),
    )
    .await;
    assert_eq!(answered, v2, "an existing Draft is answered unchanged");
    assert_eq!(
        shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
            .thread_plan_for_test(&l.app.thread)
            .await
            .unwrap(),
        Some(serde_json::from_value(v2["workflow_id"].clone()).unwrap())
    );

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

async fn expire(app: &App, draft_ref: &str) {
    sqlx::query("UPDATE draft_intent SET expires_at = ? WHERE draft_ref = ?")
        .bind(an_hour_ago())
        .bind(draft_ref)
        .execute(app.storage.reader())
        .await
        .unwrap();
}

async fn count(app: &App, table: &str) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
        .fetch_one(app.storage.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn a_bound_draft_ref_still_replays_after_its_hour() {
    let l = listening_app().await;
    let (grant, external) = project_client(&l).await;

    // From scratch: a plan and its v1, with no thread.
    let r1 = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let scratch = json!({ "draft_ref": r1, "title": "Search", "goal": "find things" });
    let first = ok(&external, "draft_start", scratch.clone()).await;
    expire(&l.app, r1.as_str().unwrap()).await;
    let (threads, plans) = (
        count(&l.app, "planning_thread").await,
        count(&l.app, "workflow").await,
    );
    assert_eq!(ok(&external, "draft_start", scratch).await, first);

    // From a frozen plan: its next version.
    let v1 = approved_v1(&l.app).await;
    let r2 = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let v1_plan = shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .get_plan_for_test(&v1)
        .await
        .unwrap()
        .plan_id;
    let next = json!({
        "draft_ref": r2,
        "plan_id": v1_plan,
        "reason": "the API changed",
    });
    let v2 = ok(&external, "draft_start", next.clone()).await;
    expire(&l.app, r2.as_str().unwrap()).await;
    let plans_with_v2 = count(&l.app, "workflow").await;
    assert_eq!(plans_with_v2, plans + 2, "v1 and v2");
    assert_eq!(ok(&external, "draft_start", next).await, v2);
    assert_eq!(count(&l.app, "planning_thread").await, threads);
    assert_eq!(count(&l.app, "workflow").await, plans_with_v2);

    // A ref never used, past its hour, starts nothing.
    let stale = insert_draft_ref(&l.app, &grant, &an_hour_ago()).await;
    let late = json!({ "draft_ref": stale, "title": "Late", "goal": "g" });
    let text = refused(&external, "draft_start", late).await;
    assert!(text.starts_with("GRANT_SCOPE: "), "{text}");
    assert_eq!(count(&l.app, "planning_thread").await, threads);

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn draft_ref_replays_and_distinct_refs_make_distinct_plans() {
    let l = listening_app().await;
    let (_, external) = project_client(&l).await;
    let r1 = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let payments = json!({ "draft_ref": r1, "title": "Payments", "goal": "take payments" });
    let a = ok(&external, "draft_start", payments.clone()).await;
    let b = ok(&external, "draft_start", payments).await;
    assert_eq!(a, b);

    let r2 = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    assert_ne!(r1, r2);
    let again = json!({ "draft_ref": r2, "title": "Payments", "goal": "take payments" });
    let c = ok(&external, "draft_start", again).await;
    assert_ne!(c["workflow_id"], a["workflow_id"]);
    assert_ne!(c["plan_id"], a["plan_id"]);

    let changed = json!({ "draft_ref": r1, "title": "Payments", "goal": "refund payments" });
    let text = refused(&external, "draft_start", changed).await;
    assert!(text.starts_with("COMMAND_CONFLICT: "), "{text}");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn arabic_plan_round_trips_through_mcp() {
    let l = listening_app().await;
    let workflow = draft(&l.app).await.workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;
    let title = "خطة الدفع";
    let goal = "يستطيع الناس الدفع بالبطاقة";
    let mut first = task(1, "تسجيل الدخول");
    first.goal = "يدخل المستخدم بكلمة المرور".into();
    first.acceptance[0].text = "تظهر رسالة خطأ واضحة".into();
    let second = task(2, "الدفع");
    let label = "رمز الجلسة";
    let ops = json!([
        { "op": "plan_put", "title": title, "goal": goal },
        { "op": "task_add", "task": first },
        { "op": "task_add", "task": second },
        { "op": "link_put", "link": { "task": 2, "after": 1, "kind": "needs", "label": label } }
    ]);
    ok(
        &planner,
        "plan_edit",
        json!({ "workflow_id": workflow, "expected_revision": 0, "ops": ops }),
    )
    .await;

    let plan = ok(&planner, "workflow_get", json!({ "workflow_id": workflow })).await;
    assert_eq!(plan["id"], json!(workflow));
    assert_eq!(plan["title"], title);
    assert_eq!(plan["goal"], goal);
    assert_eq!(plan["tasks"][0]["title"], "تسجيل الدخول");
    assert_eq!(plan["tasks"][0]["goal"], "يدخل المستخدم بكلمة المرور");
    assert_eq!(
        plan["tasks"][0]["acceptance"][0]["text"],
        "تظهر رسالة خطأ واضحة"
    );
    assert_eq!(plan["links"][0]["label"], label);
    let one = ok(
        &planner,
        "task_get",
        json!({ "workflow_id": workflow, "number": 1 }),
    )
    .await;
    assert_eq!(one["title"], "تسجيل الدخول");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_thread_grant_starts_a_draft_only_inside_a_turn() {
    let l = listening_app().await;
    let planner = thread_client(&l, &l.app.thread).await;
    let start = json!({ "title": "Login", "goal": "people can log in" });
    let text = refused(&planner, "draft_start", start.clone()).await;
    assert!(
        text.ends_with("no turn is running for this conversation"),
        "{text}"
    );

    turn_running(&l.app).await;
    let started = ok(&planner, "draft_start", start.clone()).await;
    assert_eq!(started["version"], 1);
    assert!(started["plan_id"].is_string());
    // The same call in the same turn is a replay.
    assert_eq!(ok(&planner, "draft_start", start).await, started);
    let events = events_of(&l.app, &l.app.thread, "WorkflowDraftStarted").await;
    assert_eq!(events.len(), 1);
    let id: WorkflowId = serde_json::from_value(started["workflow_id"].clone()).unwrap();
    assert_eq!(
        shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
            .thread_plan_for_test(&l.app.thread)
            .await
            .unwrap(),
        Some(id)
    );

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// A Planner's plan_id and reason are part of its DraftStart identity.
#[tokio::test]
async fn a_planner_draft_start_names_a_project_plan() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let plan_id = shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .get_plan_for_test(&v1)
        .await
        .unwrap()
        .plan_id;
    let planner = thread_client(&l, &l.app.thread).await;
    turn_running(&l.app).await;
    let start = json!({ "plan_id": plan_id, "reason": "the API changed" });
    let v2 = ok(&planner, "draft_start", start.clone()).await;
    assert_eq!(v2["version"], 2);
    assert_eq!(ok(&planner, "draft_start", start).await, v2);

    let recorded: Vec<String> = sqlx::query_scalar(
        "SELECT request_fingerprint FROM command_record
          WHERE command_kind = 'DraftStart' AND command_id LIKE 'op:%'",
    )
    .fetch_all(l.app.storage.reader())
    .await
    .unwrap();
    let params = json!({
        "project": l.app.project,
        "plan_id": plan_id,
        "title": null,
        "goal": null,
        "reason": "the API changed",
    });
    assert_eq!(
        recorded,
        [CommandContext::fingerprint("DraftStart", &params)]
    );
    let events = events_of(&l.app, &l.app.thread, "WorkflowDraftStarted").await;
    assert_eq!(events.len(), 2, "v1 and v2 only");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// A Planner edits only the latest version of a named Active plan.
#[tokio::test]
async fn a_planner_edits_only_the_plan_latest_version() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let v2 = draft_on(&l.app, &l.app.thread, "start-2").await.workflow_id;
    shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .approve_plan_for_test(&ctx("approve-2", "PlanApprove"), &v2, 0)
        .await
        .unwrap();
    let planner = thread_client(&l, &l.app.thread).await;
    turn_running(&l.app).await;
    let v1_plan = shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .get_plan_for_test(&v1)
        .await
        .unwrap();
    let v2_plan = shadows_core::AppCore::plans_for_tests(l.app.storage.clone())
        .get_plan_for_test(&v2)
        .await
        .unwrap();
    assert_eq!(v1_plan.plan_id, v2_plan.plan_id);
    let stale = refused(
        &planner,
        "plan_edit",
        json!({ "workflow_id": v1, "expected_revision": 1, "ops": [] }),
    )
    .await;
    assert!(stale.starts_with("GRANT_SCOPE: "), "{stale}");

    let next = json!({ "plan_id": v1_plan.plan_id, "reason": "the API changed" });
    let v3 = ok(&planner, "draft_start", next).await;
    assert_eq!(v3["version"], 3);
    let stale = refused(
        &planner,
        "plan_edit",
        json!({ "workflow_id": v2, "expected_revision": 0, "ops": [] }),
    )
    .await;
    assert!(stale.starts_with("GRANT_SCOPE: "), "{stale}");
    let started = events_of(&l.app, &l.app.thread, "WorkflowDraftStarted").await;
    assert_eq!(started.len(), 3, "v1, v2 and v3 only");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn both_grants_read_approved_task_context_without_execution_authority() {
    let l = listening_app().await;
    let workflow = approved_v1(&l.app).await;
    let planner = thread_client(&l, &l.app.thread).await;
    let (_, external) = project_client(&l).await;
    for client in [&planner, &external] {
        let tools = client.list_all_tools().await.unwrap();
        let tool = tools.iter().find(|t| t.name == "task_context").unwrap();
        assert!(
            tool.description
                .as_deref()
                .unwrap()
                .contains("no permission to execute")
        );
        let context = ok(
            client,
            "task_context",
            json!({
                "workflow_id": workflow, "task_number": 1,
            }),
        )
        .await;
        assert_eq!(context["plan"]["workflow_id"], json!(workflow));
        assert_eq!(context["task"]["number"], 1);
        assert!(context["standards"]["base"].as_i64().unwrap() >= 3);
        assert!(context.get("execute").is_none());
        assert!(
            refused(
                client,
                "task_context",
                json!({
                    "workflow_id": workflow, "task_number": 99,
                })
            )
            .await
            .contains("T99")
        );
    }
    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn the_planner_edits_phases_and_reads_selected_phase_context() {
    let l = listening_app().await;
    let workflow = draft(&l.app).await.workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;
    ok(
        &planner,
        "plan_edit",
        json!({"workflow_id":workflow,"expected_revision":0,
        "ops":[add(1)]}),
    )
    .await;
    ok(
        &planner,
        "plan_edit",
        json!({"workflow_id":workflow,"expected_revision":1,
        "ops":[{"op":"phases_set","task":1,"phases":[{
            "title":"Build","goal":"Deliver","acceptance":[1],"waits_for":[]
        }]}]}),
    )
    .await;
    let read = ok(&planner, "workflow_get", json!({"workflow_id":workflow})).await;
    let id = read["tasks"][0]["phases"][0]["id"].clone();
    plan::tie_to_part(&l.app, &workflow).await;
    l.app
        .core
        .plans()
        .approve("phase-approve".into(), &workflow, 2)
        .await
        .unwrap();
    let context = ok(
        &planner,
        "task_context",
        json!({
            "workflow_id":workflow,"task_number":1,"phase_id":id,
        }),
    )
    .await;
    assert_eq!(context["phase"]["id"], id);
    assert_eq!(
        context["task"]["acceptance"],
        read["tasks"][0]["acceptance"]
    );
    assert!(
        refused(
            &planner,
            "task_context",
            json!({
                "workflow_id":workflow,"task_number":1,"phase_id":"missing",
            })
        )
        .await
        .contains("phase")
    );
    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// An executor's grant, as Harness issues it when its session opens: through
/// the real `/mcp`, it lists only the executor's tools, reads only its task,
/// and has no plan-writing tool to call.
#[tokio::test]
async fn an_executor_grant_lists_its_tools_and_reads_only_its_task() {
    let l = listening_app().await;
    let workflow = approved_v1(&l.app).await;
    let (project, thread) = (l.app.project.clone(), l.app.thread.clone());
    let scope = shadows_core::testing::TaskScope {
        workflow: workflow.as_str().to_string(),
        version: 1,
        task: 1,
    };
    let (_, token) = l
        .app
        .storage
        .write_txn(move |conn| {
            Box::pin(async move {
                shadows_core::Grants::issue_executor_grant_in(conn, &project, &thread, scope).await
            })
        })
        .await
        .unwrap();
    let executor = listening::mcp_client(&l.base, token.as_str()).await;
    let mut names: Vec<String> = executor
        .list_all_tools()
        .await
        .unwrap()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "outline",
            "task_context",
            "task_report",
            "where_is",
            "who_uses"
        ]
    );
    let own = json!({ "workflow_id": workflow, "task_number": 1 });
    assert_eq!(
        ok(&executor, "task_context", own).await["task"]["number"],
        1
    );
    let other = json!({ "workflow_id": workflow, "task_number": 2 });
    refused(&executor, "task_context", other).await;
    // The grant holds no attempt, so there is nothing of its own to report.
    let report = json!({ "commit": "abc1234", "report": "x" });
    refused(&executor, "task_report", report).await;
    let edit = json!({ "workflow_id": workflow, "expected_revision": 1, "ops": [add(3)] });
    let params = rmcp::model::CallToolRequestParams::new("plan_edit")
        .with_arguments(edit.as_object().unwrap().clone());
    let unknown = executor.call_tool(params).await;
    let answer = format!("{unknown:?}");
    assert!(unknown.is_err(), "plan_edit reached an executor: {answer}");
    assert!(answer.contains("tool not found"), "{answer}");
    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_contract_written_over_mcp_is_read_the_same_over_http() {
    let l = listening_app().await;
    let workflow = draft(&l.app).await.workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;
    let tools = planner.list_all_tools().await.unwrap();
    let describes = |name: &str, text: &str| {
        let tool = tools.iter().find(|t| t.name == name).unwrap();
        assert!(
            tool.description.as_deref().unwrap().contains(text),
            "{name}"
        );
    };
    describes("plan_edit", "A task may carry contract");
    describes("task_context", "the contracts of the outputs it uses");
    let finder = json!({
        "number": 1, "title": "Finder", "goal": "Find a customer", "reads": [],
        "writes": ["app/**"], "acceptance": [{ "number": 1, "text": "A known email is found" }],
        "contract": {
            "files": [{ "path": "app/Services/CustomerFinder.php", "action": "create",
                        "purpose": "Looks customers up" }],
            "symbols": [{ "id": "s1", "path": "app/Services/CustomerFinder.php", "kind": "method",
                "name": "findCustomer",
                "inputs": [{ "name": "email", "type_name": "string", "meaning": "The address" }],
                "returns": { "type_name": "?Customer", "meaning": "The match, or null" },
                "behavior": ["Returns the customer whose email matches"] }],
            "produces": [{ "id": "o1", "symbol_id": "s1", "acceptance": [1] }]
        }
    });
    let redis = json!({
        "number": 2, "title": "Redis", "goal": "Configure Redis", "reads": [],
        "writes": ["docker/**"], "acceptance": [{ "number": 1, "text": "Redis starts" }],
        "contract": {
            "files": [{ "path": "docker/redis/redis.conf", "action": "create",
                        "purpose": "Redis settings" }],
            "uses": [{ "source": { "kind": "task", "target": { "kind": "local", "task": 1 },
                                   "output": "o1" }, "purpose": "Caches lookups" }]
        }
    });
    let ops = json!([
        { "op": "task_add", "task": finder }, { "op": "task_add", "task": redis },
        { "op": "task_add", "task": task(3, "legacy") },
        { "op": "link_put", "link": { "task": 2, "after": 1, "kind": "needs", "label": "finder" } }
    ]);
    let edit = json!({ "workflow_id": workflow, "expected_revision": 0, "ops": ops });
    ok(&planner, "plan_edit", edit).await;
    let mcp = ok(&planner, "workflow_get", json!({ "workflow_id": workflow })).await;
    let (status, http) =
        app::call(&l.app, "GET", &format!("/api/workflows/{workflow}"), None).await;
    assert_eq!(status, 200);
    assert_eq!(http["tasks"][0]["contract"], mcp["tasks"][0]["contract"]);
    assert_eq!(
        http["tasks"][0]["contract"]["symbols"][0]["name"],
        "findCustomer"
    );
    // A file task needs no function, and a legacy task has no contract key.
    assert_eq!(http["tasks"][1]["contract"]["symbols"], json!([]));
    assert!(http["tasks"][2].get("contract").is_none());
    assert_eq!(http["blockers"], json!([]));
    // A broken contract is refused with the plan's own code and the rule's words.
    let mut bad = finder.clone();
    bad["number"] = json!(4);
    bad["contract"]["files"][0]["path"] = json!("../x.php");
    let edit = json!({ "workflow_id": workflow, "expected_revision": 1,
                       "ops": [{ "op": "task_add", "task": bad }] });
    let refusal = refused(&planner, "plan_edit", edit).await;
    assert!(
        refusal.starts_with("WORKFLOW_VALIDATION_FAILED"),
        "{refusal}"
    );
    assert!(
        refusal.contains("T4 contract: `../x.php` is not an exact path"),
        "{refusal}"
    );
    // Approved, the consumer reads its producer's contract.
    plan::tie_to_part(&l.app, &workflow).await;
    l.app
        .core
        .plans()
        .approve("contract-approve".into(), &workflow, 1)
        .await
        .unwrap();
    let args = json!({ "workflow_id": workflow, "task_number": 2 });
    let context = ok(&planner, "task_context", args).await;
    assert_eq!(context["contracts"][0]["symbol"]["name"], "findCustomer");
    assert_eq!(context["contracts"][0]["producer_title"], "Finder");

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[test]
fn the_planner_is_told_what_a_contract_holds() {
    let prompt = shadows_core::testing::PROMPT;
    for text in [
        "it says what to build, never how",
        "Do not write function bodies or finished code in it",
        "inputs (name, type_name, meaning)",
        "do not invent a function for it",
        "Never copy a producer's signature",
        "A task_update sent without the task's contract leaves the task with none.",
        "its path (the file it lives in)",
        "plan_edit's description ends with one complete, valid edit; copy its shape.",
    ] {
        assert!(prompt.contains(text), "{text}");
    }
}

#[tokio::test]
async fn the_plan_edit_example_parses_as_the_ops_it_shows() {
    let l = listening_app().await;
    let planner = thread_client(&l, &l.app.thread).await;
    let tools = planner.list_all_tools().await.unwrap();
    let edit = tools.iter().find(|t| t.name == "plan_edit").unwrap();
    let description = edit.description.as_deref().unwrap();
    let (_, example) = description
        .split_once("Example ops, valid as written: ")
        .unwrap();
    // The deserializer `plan_edit` applies to its `ops` argument.
    let ops: Vec<shadows_core::PlanOp> = serde_json::from_str(example).unwrap();
    assert!(matches!(
        ops.as_slice(),
        [
            shadows_core::PlanOp::TaskAdd { .. },
            shadows_core::PlanOp::LinkPut { .. }
        ]
    ));
    let raw: serde_json::Value = serde_json::from_str(example).unwrap();
    let task = &raw[0]["task"];
    assert!(task["acceptance"][0]["text"].is_string());
    for part in ["files", "symbols", "produces", "uses"] {
        assert_eq!(
            task["contract"][part].as_array().unwrap().len(),
            1,
            "{part}"
        );
    }
    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn the_planner_is_told_an_indexing_listing_is_not_complete() {
    let unfinished = "indexing means the listing is not complete yet: ask again, \
                      never conclude the folder is empty";
    assert!(shadows_core::testing::PROMPT.contains(unfinished), "prompt");
    let l = listening_app().await;
    let planner = thread_client(&l, &l.app.thread).await;
    let tools = planner.list_all_tools().await.unwrap();
    let files = tools.iter().find(|t| t.name == "project_files").unwrap();
    let description = files.description.as_deref().unwrap();
    for state in ["ready", "incomplete", "missing", unfinished] {
        assert!(description.contains(state), "{state}");
    }
    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[test]
fn the_planner_is_told_how_to_run_and_judge_tasks() {
    let prompt = shadows_core::testing::PROMPT;
    let texts: &[&str] = &[
        "project_git_init: when Start is refused",
        "start the task task_done names as next at once with task_start, \
         without waiting for the person",
        "count anything the report says was skipped or could not run as not done",
        "otherwise send task_amend with what is missing or must be extended.",
        "A task takes at most three additions after its Start",
    ];
    for text in texts {
        assert!(prompt.contains(text), "{text}");
    }
}
