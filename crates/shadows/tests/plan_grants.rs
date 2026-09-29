//! What a writer's grant permits a plan write (spec §13.5–§13.7): checked
//! inside the write's transaction, scoped to a thread or a project, and the
//! draft refs an external agent starts plans with.

use serde_json::json;
use shadows_core::StorageError;
use shadows_core::testing::ProjectDirectory;
use shadows_core::testing::Writer;
use shadows_core::testing::{Anchor, derived_id};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{ctx, test_app};
use plan::{
    add, an_hour_ago, approved_v1, draft, draft_ref_binding, edit_ctx, events, in_an_hour,
    insert_draft_ref, insert_grant, revision, revoke_grant, writer_ctx,
};

#[tokio::test]
async fn a_draft_ref_binds_in_both_paths() {
    let app = test_app().await;
    let grant = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External {
        grant: grant.clone(),
    };
    let from_ref = |r: &str| {
        writer_ctx(
            &external,
            &derived_id(Anchor::DraftRef(r), ""),
            "DraftStart",
            json!({ "r": r }),
        )
    };

    // From scratch: a new thread and its v1.
    let r1 = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let scratch = app
        .storage
        .start_thread_with_draft(
            &from_ref(&r1),
            &external,
            &app.project,
            "Search",
            "find things",
            Some(&r1),
        )
        .await
        .unwrap();
    assert_eq!(
        draft_ref_binding(&app, &r1).await.as_deref(),
        Some(scratch.workflow_id.as_str())
    );

    // From a frozen plan: its next version.
    approved_v1(&app).await;
    let r2 = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let v2 = app
        .storage
        .start_draft(
            &from_ref(&r2),
            &external,
            &app.thread,
            None,
            None,
            Some(&r2),
        )
        .await
        .unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(
        draft_ref_binding(&app, &r2).await.as_deref(),
        Some(v2.workflow_id.as_str())
    );

    // A thread in another project is outside the grant's.
    let other_project = app
        .storage
        .create_project(
            &ctx("p2", "project.create"),
            "other",
            "Other",
            &ProjectDirectory::resolve(&std::env::temp_dir()).unwrap(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap()
        .id;
    let foreign_thread = app
        .storage
        .create_planning_thread(
            &ctx("t2", "thread.create"),
            &other_project,
            "F",
            "claude-code",
        )
        .await
        .unwrap()
        .id;
    let r3 = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let outside = app
        .storage
        .start_draft(
            &from_ref(&r3),
            &external,
            &foreign_thread,
            None,
            Some(("X", "x")),
            Some(&r3),
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
        let refused = app
            .storage
            .start_thread_with_draft(&from_ref(r), &external, &app.project, "Nope", "no", Some(r))
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
    let threads = app
        .storage
        .list_threads_for_project(&app.project)
        .await
        .unwrap();
    assert_eq!(threads.len(), 2, "the refused starts created no thread");

    // Replaying the first start after its ref expired still answers it.
    sqlx::query("UPDATE draft_intent SET expires_at = ? WHERE draft_ref = ?")
        .bind(an_hour_ago())
        .bind(&r1)
        .execute(app.storage.reader())
        .await
        .unwrap();
    let replayed = app
        .storage
        .start_thread_with_draft(
            &from_ref(&r1),
            &external,
            &app.project,
            "Search",
            "find things",
            Some(&r1),
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

#[tokio::test]
async fn a_planner_grant_edits_only_its_own_thread() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let other = app
        .storage
        .create_planning_thread(
            &ctx("t2", "thread.create"),
            &app.project,
            "O",
            "claude-code",
        )
        .await
        .unwrap()
        .id;
    let grant = insert_grant(&app, "thread", &app.project, Some(&other)).await;
    let planner = Writer::Planner {
        thread: other,
        grant,
    };
    let refused = app
        .storage
        .edit_plan(
            &edit_ctx(&planner, &v1, 0, &[add(1)]),
            &planner,
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

#[tokio::test]
async fn start_thread_with_draft_creates_both_and_records_the_grant_as_actor() {
    let app = test_app().await;
    let grant = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External {
        grant: grant.clone(),
    };
    let r = insert_draft_ref(&app, &grant, &in_an_hour()).await;
    let started = app
        .storage
        .start_thread_with_draft(
            &writer_ctx(
                &external,
                &derived_id(Anchor::DraftRef(&r), ""),
                "DraftStart",
                json!({}),
            ),
            &external,
            &app.project,
            "Search",
            "find things",
            Some(&r),
        )
        .await
        .unwrap();
    assert_eq!(started.version, 1);
    assert_ne!(started.thread_id, app.thread);

    let threads = app
        .storage
        .list_threads_for_project(&app.project)
        .await
        .unwrap();
    let thread = threads.iter().find(|t| t.id == started.thread_id).unwrap();
    assert_eq!(
        (thread.title.as_str(), thread.harness.as_str()),
        ("Search", "claude-code")
    );
    let plan = app.storage.get_plan(&started.workflow_id).await.unwrap();
    assert_eq!(
        (plan.title.as_str(), plan.goal.as_str()),
        ("Search", "find things")
    );

    let all = events(&app, &started.thread_id).await;
    for kind in ["PlanningThreadCreated", "WorkflowDraftStarted"] {
        let e = all.iter().find(|e| e.0 == kind).unwrap();
        assert_eq!(
            (e.1.as_str(), e.2.as_str()),
            ("Grant", grant.as_str()),
            "{kind}"
        );
    }
}

#[tokio::test]
async fn external_draft_starts_require_a_draft_ref() {
    let app = test_app().await;
    let grant = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External { grant };

    let scratch = app
        .storage
        .start_thread_with_draft(
            &writer_ctx(&external, "scratch-without-ref", "DraftStart", json!({})),
            &external,
            &app.project,
            "Search",
            "find things",
            None,
        )
        .await;
    assert!(
        matches!(scratch, Err(StorageError::GrantScope)),
        "{scratch:?}"
    );

    let frozen = approved_v1(&app).await;
    let in_thread = app
        .storage
        .start_draft(
            &writer_ctx(&external, "thread-without-ref", "DraftStart", json!({})),
            &external,
            &app.thread,
            None,
            None,
            None,
        )
        .await;
    assert!(
        matches!(in_thread, Err(StorageError::GrantScope)),
        "{in_thread:?}"
    );
    assert_eq!(
        app.storage.thread_plan(&app.thread).await.unwrap(),
        Some(frozen)
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
    let grant = insert_grant(&app, "project", &app.project, None).await;
    let external = Writer::External { grant };
    let repeated = app
        .storage
        .start_draft(
            &writer_ctx(&external, "existing-draft", "DraftStart", json!({})),
            &external,
            &app.thread,
            None,
            None,
            None,
        )
        .await;
    assert_eq!(repeated.unwrap(), first);
}
