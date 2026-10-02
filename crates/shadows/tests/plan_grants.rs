//! What a writer's grant permits a plan write (spec §13.5–§13.7): checked
//! inside the write's transaction, scoped to a thread or a project, and the
//! draft refs an external agent starts plans with.

use serde_json::json;
use shadows_core::StorageError;
use shadows_core::testing::ProjectDirectory;
use shadows_core::testing::Writer;
use shadows_core::testing::{Anchor, derived_id};
use shadows_core::{DraftStarted, GrantId, ProjectId, WrittenBy};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{App, ctx, test_app};
use plan::{
    add, an_hour_ago, approved_v1, draft, draft_ref_binding, edit_ctx, events, in_an_hour,
    insert_draft_ref, insert_grant, revision, revoke_grant, writer_ctx,
};

/// An external agent's start under `draft_ref`: from scratch in `project`
/// when `plan` is `None`, else the plan's next version with a reason.
async fn external_start(
    app: &App,
    external: &Writer,
    project: &ProjectId,
    plan: Option<&shadows_core::PlanId>,
    draft_ref: Option<&str>,
    command: &str,
) -> Result<DraftStarted, StorageError> {
    app.storage
        .start_draft(
            &writer_ctx(external, command, "DraftStart", json!({ "r": draft_ref })),
            external,
            project,
            plan,
            None,
            Some(("Search", "find things")),
            Some("the search changed"),
            None,
            draft_ref,
        )
        .await
}

fn ref_command(r: &str) -> String {
    derived_id(Anchor::DraftRef(r), "")
}

async fn other_project(app: &App) -> ProjectId {
    app.storage
        .create_project(
            &ctx("p2", "project.create"),
            "other",
            "Other",
            &ProjectDirectory::resolve(&std::env::temp_dir()).unwrap(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn a_draft_ref_binds_in_both_paths() {
    let app = test_app().await;
    let grant = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External {
        grant: grant.clone(),
    };

    // From scratch: a new plan and its v1.
    let r1 = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let scratch = external_start(
        &app,
        &external,
        &app.project,
        None,
        Some(&r1),
        &ref_command(&r1),
    )
    .await
    .unwrap();
    assert_eq!(
        draft_ref_binding(&app, &r1).await.as_deref(),
        Some(scratch.workflow_id.as_str())
    );

    // From a frozen plan: its next version.
    let v1 = approved_v1(&app).await;
    let plan = app.storage.get_plan(&v1).await.unwrap().plan_id;
    let r2 = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let v2 = external_start(
        &app,
        &external,
        &app.project,
        Some(&plan),
        Some(&r2),
        &ref_command(&r2),
    )
    .await
    .unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(
        draft_ref_binding(&app, &r2).await.as_deref(),
        Some(v2.workflow_id.as_str())
    );

    // Another project is outside the grant's.
    let foreign = other_project(&app).await;
    let r3 = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let outside = external_start(
        &app,
        &external,
        &foreign,
        None,
        Some(&r3),
        &ref_command(&r3),
    )
    .await;
    assert!(
        matches!(outside, Err(StorageError::GrantScope)),
        "{outside:?}"
    );

    // An expired ref, and another grant's ref.
    let r4 = insert_draft_ref(&app, &grant, &an_hour_ago()).await;
    let other_grant = insert_grant(&app, "project", &app.project, None).await;
    let r5 = insert_draft_ref(&app, &other_grant, &in_an_hour()).await;
    for r in [&r4, &r5] {
        let refused = external_start(
            &app,
            &external,
            &app.project,
            None,
            Some(r),
            &ref_command(r),
        )
        .await;
        assert!(
            matches!(refused, Err(StorageError::GrantScope)),
            "{refused:?}"
        );
    }
    for r in [&r3, &r4, &r5] {
        assert_eq!(
            draft_ref_binding(&app, r).await,
            None,
            "a refused ref stays unbound"
        );
    }
    let plans = app.storage.list_plans(&app.project, true).await.unwrap();
    assert_eq!(plans.len(), 2, "the refused starts created no plan");
    let threads = app
        .storage
        .list_threads_for_project(&app.project)
        .await
        .unwrap();
    assert_eq!(
        threads.len(),
        1,
        "an external start creates no thread (§16.3)"
    );

    // Replaying the first start after its ref expired still answers it.
    sqlx::query("UPDATE draft_intent SET expires_at = ? WHERE draft_ref = ?")
        .bind(an_hour_ago())
        .bind(&r1)
        .execute(app.storage.reader())
        .await
        .unwrap();
    let replayed = external_start(
        &app,
        &external,
        &app.project,
        None,
        Some(&r1),
        &ref_command(&r1),
    )
    .await
    .unwrap();
    assert_eq!(replayed, scratch);
}

#[tokio::test]
async fn an_edit_by_a_revoked_grant_writes_nothing() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let grant = insert_grant(&app, "project", &app.project, None).await;
    revoke_grant(&app, &grant).await;
    let external = Writer::External { grant };
    let refused = app
        .storage
        .edit_plan(
            &edit_ctx(&external, &v1, 0, &[add(1)]),
            &external,
            None,
            &v1,
            0,
            &[add(1)],
        )
        .await;
    assert!(
        matches!(refused, Err(StorageError::GrantInvalid)),
        "{refused:?}"
    );
    assert_eq!(revision(&app, &v1).await, 0);
}

/// §16.4: storage holds a Planner to its own project; which of the project's
/// plans it reaches is `Plans`' scope check, not this one.
#[tokio::test]
async fn a_planner_grant_writes_only_in_its_own_project() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let foreign = other_project(&app).await;
    let other = app
        .storage
        .create_planning_thread(&ctx("t2", "thread.create"), &foreign, "O", "claude-code")
        .await
        .unwrap()
        .id;
    let grant = insert_grant(&app, "thread", &foreign, Some(&other)).await;
    let planner = Writer::Planner {
        thread: other,
        grant,
    };
    let refused = app
        .storage
        .edit_plan(
            &edit_ctx(&planner, &v1, 0, &[add(1)]),
            &planner,
            None,
            &v1,
            0,
            &[add(1)],
        )
        .await;
    assert!(
        matches!(refused, Err(StorageError::GrantScope)),
        "{refused:?}"
    );

    let own = insert_grant(&app, "thread", &app.project, Some(&app.thread)).await;
    let planner = Writer::Planner {
        thread: app.thread.clone(),
        grant: own,
    };
    let done = app
        .storage
        .edit_plan(
            &edit_ctx(&planner, &v1, 0, &[add(1)]),
            &planner,
            None,
            &v1,
            0,
            &[add(1)],
        )
        .await
        .unwrap();
    assert_eq!(done.revision, 1);
    let edited = events(&app, &app.thread).await;
    let last = edited.iter().rfind(|e| e.0 == "WorkflowEdited").unwrap();
    assert_eq!(
        (last.1.as_str(), last.2.as_str()),
        ("Thread", app.thread.as_str())
    );
}

/// `(kind, actor_kind, actor_id, thread_id)` of every event of `project`.
async fn project_events(
    app: &App,
    project: &ProjectId,
) -> Vec<(String, String, String, Option<String>)> {
    sqlx::query_as(
        "SELECT kind, actor_kind, actor_id, thread_id FROM durable_event
          WHERE project_id = ? ORDER BY seq",
    )
    .bind(project.as_str())
    .fetch_all(app.storage.reader())
    .await
    .unwrap()
}

/// §16.3: an external agent's plan from scratch has no conversation; its
/// version is written by the grant, which is also the event's actor.
#[tokio::test]
async fn an_external_start_from_scratch_creates_a_plan_and_no_thread() {
    let app = test_app().await;
    let grant = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External {
        grant: grant.clone(),
    };
    let r = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let started = external_start(
        &app,
        &external,
        &app.project,
        None,
        Some(&r),
        &ref_command(&r),
    )
    .await
    .unwrap();
    assert_eq!(started.version, 1);

    let threads = app
        .storage
        .list_threads_for_project(&app.project)
        .await
        .unwrap();
    assert_eq!(threads.len(), 1, "only the app's own thread");
    let plan = app.storage.get_plan(&started.workflow_id).await.unwrap();
    assert_eq!(
        (plan.title.as_str(), plan.goal.as_str()),
        ("Search", "find things")
    );
    assert_eq!(plan.plan_id, started.plan_id);
    assert_eq!(
        plan.written_by,
        WrittenBy::External {
            grant_id: grant.clone()
        }
    );
    assert_eq!(plan.change_reason, None, "v1 has no reason");

    let all = project_events(&app, &app.project).await;
    let e = all.iter().find(|e| e.0 == "WorkflowDraftStarted").unwrap();
    assert_eq!(
        (e.1.as_str(), e.2.as_str(), e.3.as_deref()),
        ("Grant", grant.as_str(), None)
    );
}

#[tokio::test]
async fn external_draft_starts_require_a_draft_ref() {
    let app = test_app().await;
    let grant = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External { grant };

    let scratch = external_start(
        &app,
        &external,
        &app.project,
        None,
        None,
        "scratch-without-ref",
    )
    .await;
    assert!(
        matches!(scratch, Err(StorageError::GrantScope)),
        "{scratch:?}"
    );

    let frozen = approved_v1(&app).await;
    let plan = app.storage.get_plan(&frozen).await.unwrap().plan_id;
    let in_plan = external_start(
        &app,
        &external,
        &app.project,
        Some(&plan),
        None,
        "plan-without-ref",
    )
    .await;
    assert!(
        matches!(in_plan, Err(StorageError::GrantScope)),
        "{in_plan:?}"
    );
    assert_eq!(
        app.storage.thread_plan(&app.thread).await.unwrap(),
        Some(frozen)
    );
    assert_eq!(
        app.storage
            .list_plans(&app.project, true)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        app.storage
            .list_threads_for_project(&app.project)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn an_external_start_returns_an_existing_draft_without_a_ref() {
    let app = test_app().await;
    let first = draft(&app).await;
    let grant: GrantId = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External { grant };
    let repeated = external_start(
        &app,
        &external,
        &app.project,
        Some(&first.plan_id),
        None,
        "existing-draft",
    )
    .await;
    assert_eq!(repeated.unwrap(), first);
}
