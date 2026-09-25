//! What each tool on `/mcp` does with plans (spec §13.5–§13.6): the reach of
//! a thread grant and a project grant, revision conflicts, replays, draft
//! refs, and text that survives the trip unchanged. The transport itself is
//! `mcp_server.rs`.

use serde_json::json;
use shadows::agent::policy;
use shadows::thread::ThreadId;
use shadows::workflow::WorkflowId;

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{App, ctx, other_project, start_settled};
use listening::{call, listening_app, ok, project_client, refused, thread_client};
use plan::{add, an_hour_ago, approved_v1, draft, draft_on, events_of, insert_draft_ref, task};

async fn second_thread(app: &App) -> ThreadId {
    app.storage
        .create_planning_thread(
            &ctx("second-thread", "thread.create"),
            &app.project,
            "Second",
            policy::CLAUDE_CODE,
        )
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn a_thread_grant_edits_only_its_own_threads_plan() {
    let l = listening_app().await;
    let own = draft(&l.app).await.workflow_id;
    let other_thread = second_thread(&l.app).await;
    let other = draft_on(&l.app, &other_thread, "start-other")
        .await
        .workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;

    let edit = json!({ "workflow_id": other, "expected_revision": 0, "ops": [add(1)] });
    let text = refused(&planner, "plan_edit", edit).await;
    assert!(text.starts_with("GRANT_SCOPE: "), "{text}");
    let text = refused(&planner, "workflow_get", json!({ "workflow_id": other })).await;
    assert!(text.starts_with("GRANT_SCOPE: "), "{text}");
    assert_eq!(plan::revision(&l.app, &other).await, 0);

    // Naming nothing, it edits its own thread's plan.
    let outcome = ok(
        &planner,
        "plan_edit",
        json!({ "expected_revision": 0, "ops": [add(1)] }),
    )
    .await;
    assert_eq!(outcome["workflow_id"], json!(own));
    assert_eq!(outcome["revision"], 1);
    let events = events_of(&l.app, &l.app.thread, "WorkflowEdited").await;
    assert_eq!(events.len(), 1);
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
}

#[tokio::test]
async fn parallel_edits_on_one_revision_conflict_and_the_loser_can_retry() {
    let l = listening_app().await;
    let workflow = draft(&l.app).await.workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;
    let first = json!({ "expected_revision": 0, "ops": [add(1)] });
    let second = json!({ "expected_revision": 0, "ops": [add(2)] });

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
    let plan = l.app.storage.get_plan(&workflow).await.unwrap();
    let numbers: Vec<u32> = plan.tasks.iter().map(|t| t.content.number).collect();
    assert_eq!(numbers, [1, 2]);
}

#[tokio::test]
async fn a_repeated_edit_returns_the_first_result() {
    let l = listening_app().await;
    draft(&l.app).await;
    let planner = thread_client(&l, &l.app.thread).await;
    let edit = json!({ "expected_revision": 0, "ops": [add(1)] });
    let first = ok(&planner, "plan_edit", edit.clone()).await;
    let again = ok(&planner, "plan_edit", edit).await;
    assert_eq!(first, again);
    assert_eq!(first["revision"], 1);
    let events = events_of(&l.app, &l.app.thread, "WorkflowEdited").await;
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn editing_a_frozen_plan_is_refused() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let planner = thread_client(&l, &l.app.thread).await;
    let edit = json!({ "expected_revision": 1, "ops": [add(3)] });
    let text = refused(&planner, "plan_edit", edit).await;
    assert!(text.starts_with("WORKFLOW_FROZEN_IMMUTABLE: "), "{text}");
    assert!(
        text.ends_with("Start a new version with draft_start."),
        "{text}"
    );
    assert_eq!(plan::revision(&l.app, &v1).await, 1);
}

#[tokio::test]
async fn a_draft_ref_replays_for_a_new_version_of_a_frozen_plan() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let (_, external) = project_client(&l).await;
    let draft_ref = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let start = json!({ "draft_ref": draft_ref, "from_workflow_id": v1 });

    let v2 = ok(&external, "draft_start", start.clone()).await;
    assert_eq!(v2["version"], 2);
    assert_eq!(v2["thread_id"], json!(l.app.thread));
    let edit = json!({ "workflow_id": v2["workflow_id"], "expected_revision": 0, "ops": [add(3)] });
    assert_eq!(ok(&external, "plan_edit", edit).await["revision"], 1);

    let again = ok(&external, "draft_start", start).await;
    assert_eq!(again, v2);
}

#[tokio::test]
async fn an_external_draft_source_must_be_frozen() {
    let l = listening_app().await;
    let v1 = approved_v1(&l.app).await;
    let (_, external) = project_client(&l).await;
    let r1 = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let v2 = ok(
        &external,
        "draft_start",
        json!({ "draft_ref": r1, "from_workflow_id": v1 }),
    )
    .await;

    // A frozen source's thread already has a Draft, so a new ref answers it.
    let second_ref = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let again = ok(
        &external,
        "draft_start",
        json!({ "draft_ref": second_ref, "from_workflow_id": v1 }),
    )
    .await;
    assert_eq!(again, v2);

    let invalid_ref = ok(&external, "draft_prepare", json!({})).await["draft_ref"].clone();
    let text = refused(
        &external,
        "draft_start",
        json!({ "draft_ref": invalid_ref, "from_workflow_id": v2["workflow_id"] }),
    )
    .await;
    assert!(text.starts_with("GRANT_SCOPE: "), "{text}");
    assert_eq!(
        l.app.storage.thread_plan(&l.app.thread).await.unwrap(),
        Some(serde_json::from_value(v2["workflow_id"].clone()).unwrap())
    );
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

    // From scratch: a thread and its v1.
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
    let next = json!({ "draft_ref": r2, "from_workflow_id": v1 });
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
    assert_ne!(c["thread_id"], a["thread_id"]);

    let changed = json!({ "draft_ref": r1, "title": "Payments", "goal": "refund payments" });
    let text = refused(&external, "draft_start", changed).await;
    assert!(text.starts_with("COMMAND_CONFLICT: "), "{text}");
}

#[tokio::test]
async fn arabic_plan_round_trips_through_mcp() {
    let l = listening_app().await;
    let workflow = draft(&l.app).await.workflow_id;
    let planner = thread_client(&l, &l.app.thread).await;
    let (title, goal) = ("خطة الدفع", "يستطيع الناس الدفع بالبطاقة");
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
        json!({ "expected_revision": 0, "ops": ops }),
    )
    .await;

    let plan = ok(&planner, "workflow_get", json!({})).await;
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
    let one = ok(&planner, "task_get", json!({ "number": 1 })).await;
    assert_eq!(one["title"], "تسجيل الدخول");
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

    let op = start_settled(&l.app, "hang").await;
    for _ in 0..200 {
        if l.app.handles.running_for(&l.app.thread).await.as_ref() == Some(&op) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let started = ok(&planner, "draft_start", start.clone()).await;
    assert_eq!(started["version"], 1);
    assert_eq!(started["thread_id"], json!(l.app.thread));
    // The same call in the same turn is a replay.
    assert_eq!(ok(&planner, "draft_start", start).await, started);
    let events = events_of(&l.app, &l.app.thread, "WorkflowDraftStarted").await;
    assert_eq!(events.len(), 1);
    let id: WorkflowId = serde_json::from_value(started["workflow_id"].clone()).unwrap();
    assert_eq!(
        l.app.storage.thread_plan(&l.app.thread).await.unwrap(),
        Some(id)
    );
}
