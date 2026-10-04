//! The plan's storage contract (spec §13.2–§13.5, §13.15): versions, edits
//! under a revision check, approval, and commands that answer on replay what
//! they answered when they committed. Separate from `storage_contract.rs`, an
//! accretion point.

use serde_json::{Value, json};
use shadows_core::OperationId;
use shadows_core::StorageError;
use shadows_core::testing::{Anchor, derived_id};
use shadows_core::testing::{Writer, fingerprint};
use shadows_core::{EntryRef, ThreadEntryKind};
use shadows_core::{LinkKind, Plan, PlanOp, TaskContent, WorkflowState, WrittenBy};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{ctx, entries_on, test_app};
use plan::{
    add, approved_v1, draft, draft_on, edit, edit_ctx, events_of, needs, planner,
    project_events_of, revision, task, writer_ctx,
};

#[tokio::test]
async fn a_new_draft_is_version_one_at_revision_zero() {
    let app = test_app().await;
    let started = draft(&app).await;
    assert_eq!(started.version, 1);

    let plan = app.storage.get_plan(&started.workflow_id).await.unwrap();
    assert!(
        matches!(
            &plan.written_by,
            WrittenBy::Planner { thread_id, .. } if thread_id == &app.thread
        ),
        "{:?}",
        plan.written_by
    );
    assert_eq!(plan.plan_id, started.plan_id);
    assert_eq!(
        (plan.version, plan.revision, plan.state),
        (1, 0, WorkflowState::Draft)
    );
    assert_eq!(
        (plan.title.as_str(), plan.goal.as_str()),
        ("Login", "people can log in")
    );
    assert_eq!(plan.project_id, app.project);
    assert!(plan.previous.is_none() && plan.next.is_none() && plan.last_edit.is_none());
    assert!(plan.tasks.is_empty());
    assert!(
        !plan.blockers.is_empty(),
        "an empty draft cannot be approved"
    );
    assert_eq!(
        app.storage.thread_plan(&app.thread).await.unwrap(),
        Some(started.workflow_id.clone())
    );
    let started_events = events_of(&app, &app.thread, "WorkflowDraftStarted").await;
    assert_eq!(
        started_events,
        vec![json!({ "workflow_id": started.workflow_id, "version": 1 })]
    );
}

#[tokio::test]
async fn start_draft_answers_the_existing_draft_unchanged() {
    let app = test_app().await;
    let first = draft(&app).await;
    edit(&app, &first.workflow_id, 0, &[add(1)]).await;

    let writer = planner(&app, &app.thread).await;
    let again = app
        .storage
        .start_draft(
            &writer_ctx(&writer, "start-2", "DraftStart", json!({ "n": 2 })),
            &writer,
            &app.project,
            Some(&first.plan_id),
            None,
            Some(("Other", "ignored")),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(again, first);
    let plan = app.storage.get_plan(&first.workflow_id).await.unwrap();
    assert_eq!((plan.title.as_str(), plan.revision), ("Login", 1));
    assert_eq!(
        events_of(&app, &app.thread, "WorkflowDraftStarted")
            .await
            .len(),
        1
    );
}

#[tokio::test]
async fn an_edit_moves_the_revision_once_and_records_one_event() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let ops = [add(1), add(2), needs(2, 1)];
    let outcome = app
        .storage
        .edit_plan(
            &edit_ctx(&Writer::Person, &v1, 0, &ops),
            &Writer::Person,
            None,
            &v1,
            0,
            &ops,
        )
        .await
        .unwrap();
    assert_eq!((outcome.version, outcome.revision), (1, 1));
    assert_eq!(outcome.changed_tasks, vec![1, 2]);

    let edited = project_events_of(&app, &app.project, "WorkflowEdited").await;
    assert_eq!(edited.len(), 1);
    assert_eq!(edited[0]["changed_tasks"], json!([1, 2]));
    assert_eq!(edited[0]["summary"], json!(outcome.summary));
    assert_eq!(edited[0]["revision"], json!(1));

    let plan = app.storage.get_plan(&v1).await.unwrap();
    assert_eq!(plan.revision, 1);
    assert_eq!(
        plan.tasks
            .iter()
            .map(|t| t.content.number)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(plan.links.len(), 1);
    assert_eq!((plan.links[0].task, plan.links[0].after), (2, 1));
    let last = plan.last_edit.unwrap();
    assert_eq!(
        (last.revision, last.summary, last.changed_tasks),
        (1, outcome.summary, vec![1, 2])
    );
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
}

#[tokio::test]
async fn an_edit_on_a_stale_revision_is_refused_with_the_current_one_and_a_summary() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let first = app
        .storage
        .edit_plan(
            &edit_ctx(&Writer::Person, &v1, 0, &[add(1)]),
            &Writer::Person,
            None,
            &v1,
            0,
            &[add(1)],
        )
        .await
        .unwrap();

    let stale = [add(2)];
    let refused = app
        .storage
        .edit_plan(
            &edit_ctx(&Writer::Person, &v1, 0, &stale),
            &Writer::Person,
            None,
            &v1,
            0,
            &stale,
        )
        .await
        .unwrap_err();
    match refused {
        StorageError::RevisionConflict { current, summary } => {
            assert_eq!(current, 1);
            assert_eq!(summary, first.summary);
        }
        other => panic!("expected RevisionConflict, got {other:?}"),
    }
    assert_eq!(revision(&app, &v1).await, 1);
}

#[tokio::test]
async fn a_replayed_edit_returns_its_own_outcome_after_a_later_edit() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let a_ctx = edit_ctx(&Writer::Person, &v1, 0, &[add(1)]);
    let a = app
        .storage
        .edit_plan(&a_ctx, &Writer::Person, None, &v1, 0, &[add(1)])
        .await
        .unwrap();
    assert_eq!(edit(&app, &v1, 1, &[add(2)]).await, 2);

    let replayed = app
        .storage
        .edit_plan(&a_ctx, &Writer::Person, None, &v1, 0, &[add(1)])
        .await
        .unwrap();
    assert_eq!(replayed, a);
    assert_eq!(replayed.revision, 1);
    assert_eq!(revision(&app, &v1).await, 2);
    assert_eq!(
        project_events_of(&app, &app.project, "WorkflowEdited")
            .await
            .len(),
        2
    );
}

#[tokio::test]
async fn a_replayed_draft_start_answers_the_same_version_after_edits() {
    let app = test_app().await;
    let first = draft(&app).await;
    edit(&app, &first.workflow_id, 0, &[add(1)]).await;
    edit(&app, &first.workflow_id, 1, &[add(2)]).await;
    assert_eq!(draft(&app).await, first);
}

#[tokio::test]
async fn the_same_explicit_command_id_with_different_ops_is_a_command_conflict() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let explicit = |ops: &[PlanOp]| {
        writer_ctx(
            &Writer::Person,
            "cmd-edit",
            "PlanEdit",
            json!({ "workflow": v1, "expected_revision": 0, "ops": ops }),
        )
    };
    app.storage
        .edit_plan(
            &explicit(&[add(1)]),
            &Writer::Person,
            None,
            &v1,
            0,
            &[add(1)],
        )
        .await
        .unwrap();
    let conflict = app
        .storage
        .edit_plan(
            &explicit(&[add(2)]),
            &Writer::Person,
            None,
            &v1,
            0,
            &[add(2)],
        )
        .await;
    assert!(
        matches!(conflict, Err(StorageError::CommandConflict)),
        "{conflict:?}"
    );
}

#[tokio::test]
async fn an_invalid_edit_writes_nothing() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let ops = [add(1), needs(1, 9)];
    let refused = app
        .storage
        .edit_plan(
            &edit_ctx(&Writer::Person, &v1, 0, &ops),
            &Writer::Person,
            None,
            &v1,
            0,
            &ops,
        )
        .await;
    match refused {
        Err(StorageError::PlanInvalid(problems)) => assert!(!problems.is_empty()),
        other => panic!("expected PlanInvalid, got {other:?}"),
    }
    assert_eq!(revision(&app, &v1).await, 0);
    assert!(app.storage.get_plan(&v1).await.unwrap().tasks.is_empty());
    assert!(
        project_events_of(&app, &app.project, "WorkflowEdited")
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn a_frozen_version_refuses_edits() {
    let app = test_app().await;
    let v1 = approved_v1(&app).await;
    let refused = app
        .storage
        .edit_plan(
            &edit_ctx(&Writer::Person, &v1, 1, &[add(3)]),
            &Writer::Person,
            None,
            &v1,
            1,
            &[add(3)],
        )
        .await;
    assert!(
        matches!(refused, Err(StorageError::WorkflowFrozen)),
        "{refused:?}"
    );
}

#[tokio::test]
async fn approval_freezes_and_writes_a_plan_approved_entry() {
    let app = test_app().await;
    let v1 = approved_v1(&app).await;

    let plan = app.storage.get_plan(&v1).await.unwrap();
    assert_eq!(plan.state, WorkflowState::Frozen);
    assert!(plan.frozen_at.is_some());
    assert!(plan.blockers.is_empty());

    let entry = entries_on(&app, &app.thread).await.pop().unwrap();
    assert_eq!(entry.kind, ThreadEntryKind::PlanApproved);
    assert_eq!(entry.body, "Plan v1 approved");
    assert_eq!(entry.refs, vec![EntryRef::Workflow(v1.clone())]);

    let frozen = events_of(&app, &app.thread, "WorkflowFrozen").await;
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0]["workflow_id"], json!(v1));
    assert_eq!(frozen[0]["frozen_at"], json!(plan.frozen_at));
}

#[tokio::test]
async fn a_replayed_approval_answers_its_own_outcome_after_v2_exists() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    edit(&app, &v1, 0, &[add(1)]).await;
    let approve = ctx("approve-1", "PlanApprove");
    let first = app.storage.approve_plan(&approve, &v1, 1).await.unwrap();
    assert_eq!((first.version, first.revision), (1, 1));

    let v2 = draft_on(&app, &app.thread, "start-v2").await.workflow_id;
    let replayed = app.storage.approve_plan(&approve, &v1, 1).await.unwrap();
    assert_eq!(replayed, first);
    assert_eq!(app.storage.get_plan(&v1).await.unwrap().next, Some(v2));
    assert_eq!(
        events_of(&app, &app.thread, "WorkflowFrozen").await.len(),
        1
    );
}

#[tokio::test]
async fn approval_with_blockers_is_refused_with_the_list() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let refused = app
        .storage
        .approve_plan(&ctx("approve-1", "PlanApprove"), &v1, 0)
        .await;
    match refused {
        Err(StorageError::PlanInvalid(problems)) => {
            let plan = app.storage.get_plan(&v1).await.unwrap();
            assert_eq!(problems, plan.blockers);
            assert_eq!(plan.state, WorkflowState::Draft);
        }
        other => panic!("expected PlanInvalid, got {other:?}"),
    }
}

#[tokio::test]
async fn after_approval_start_draft_copies_tasks_and_links_with_their_numbers() {
    let app = test_app().await;
    let v1 = approved_v1(&app).await;
    let before = app.storage.get_plan(&v1).await.unwrap();

    let v2 = draft_on(&app, &app.thread, "start-v2").await;
    assert_eq!(v2.version, 2);
    let copy = app.storage.get_plan(&v2.workflow_id).await.unwrap();
    assert_eq!(
        (copy.state, copy.revision, copy.previous.as_ref()),
        (WorkflowState::Draft, 0, Some(&v1))
    );
    assert_eq!(
        (copy.title.as_str(), copy.goal.as_str()),
        ("Login", "people can log in")
    );
    let contents =
        |p: &Plan| -> Vec<TaskContent> { p.tasks.iter().map(|t| t.content.clone()).collect() };
    assert_eq!(contents(&copy), contents(&before));
    assert_eq!(copy.links, before.links);
    for (new, old) in copy.tasks.iter().zip(&before.tasks) {
        assert_ne!(new.id, old.id);
    }
    assert_eq!(app.storage.get_plan(&v1).await.unwrap(), {
        let mut b = before.clone();
        b.next = Some(v2.workflow_id.clone());
        b
    });
    assert_eq!(
        app.storage.thread_plan(&app.thread).await.unwrap(),
        Some(v2.workflow_id.clone())
    );
    let listed = app.storage.list_plans(&app.project, false).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        (listed[0].id.clone(), listed[0].version),
        (v2.workflow_id, 2)
    );
}

/// §13.3: task scope stays in scope_json, separate from its contract.
#[tokio::test]
async fn reads_and_writes_are_stored_in_scope_json() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    edit(
        &app,
        &v1,
        0,
        &[PlanOp::TaskAdd {
            task: task(1, "مهمة"),
        }],
    )
    .await;
    let (contract, scope): (String, String) = sqlx::query_as(
        "SELECT contract_json, scope_json FROM task WHERE workflow_id = ? AND number = 1",
    )
    .bind(v1.as_str())
    .fetch_one(app.storage.reader())
    .await
    .unwrap();
    let contract: Value = serde_json::from_str(&contract).unwrap();
    let scope: Value = serde_json::from_str(&scope).unwrap();
    let mut keys: Vec<&String> = contract.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["acceptance", "goal", "title"]);
    assert_eq!(contract["title"], json!("مهمة"));
    assert_eq!(
        scope,
        json!({ "reads": ["docs/t1.md"], "writes": ["src/t1.rs"] })
    );
    let plan = app.storage.get_plan(&v1).await.unwrap();
    assert_eq!(plan.tasks[0].content, task(1, "مهمة"));
}

#[tokio::test]
async fn completes_after_links_keep_their_waiting_items() {
    let app = test_app().await;
    let v1 = draft(&app).await.workflow_id;
    let link = shadows_core::Link {
        task: 2,
        after: 1,
        kind: LinkKind::CompletesAfter,
        label: "schema".into(),
        waiting_items: vec![1],
    };
    edit(
        &app,
        &v1,
        0,
        &[add(1), add(2), PlanOp::LinkPut { link: link.clone() }],
    )
    .await;
    assert_eq!(app.storage.get_plan(&v1).await.unwrap().links, vec![link]);
}

#[test]
fn derived_ids_carry_their_anchor() {
    let fp = fingerprint("PlanEdit", &json!({ "a": 1 }));
    assert_eq!(
        derived_id(Anchor::Revision(7), &fp),
        format!("rev:7:{}", &fp[..16])
    );
    let op = OperationId::from_literal("op-1");
    assert_eq!(
        derived_id(Anchor::Operation(&op), &fp),
        format!("op:op-1:{}", &fp[..16])
    );
    assert_eq!(derived_id(Anchor::DraftRef("dr-1"), &fp), "ref:dr-1");
}
