//! Cross-plan link identities under §16.7.

use serde_json::json;
use shadows_core::Link;

use shadows_core::testing::{Storage, Writer};
use shadows_core::{PlanId, PlanOp, ProjectId, TaskParent, WorkflowId};

#[path = "fixtures/cross_plan.rs"]
mod cross_plan;
use cross_plan::{SOURCE, TARGET, TARGET_PLAN, command, fixture, task};

#[test]
fn local_link_parent_keeps_its_existing_wire_shape() {
    let wire = json!({
        "task": 4,
        "after": 3,
        "kind": "needs",
        "label": "API result",
        "waiting_items": [],
    });
    let link: Link = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(link).unwrap(), wire);
}

#[test]
fn cross_plan_link_parent_roundtrips_as_plan_identity() {
    let wire = json!({
        "task": 4,
        "after": {
            "plan_id": "11111111-1111-4111-8111-111111111111",
            "task": 3,
        },
        "kind": "needs",
        "label": "API result",
        "waiting_items": [],
    });
    let link: Link = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(link).unwrap(), wire);
}

#[tokio::test]
async fn cross_plan_links_persist_replay_and_reopen_without_duplicate_events() {
    let (temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    storage
        .edit_plan(
            &command("target-task"),
            &Writer::Person,
            None,
            &target,
            0,
            &[task(3)],
        )
        .await
        .unwrap();
    let link = Link {
        task: 4,
        after: TaskParent::Plan {
            plan_id: PlanId::from_literal(TARGET_PLAN),
            task: 3,
        },
        kind: shadows_core::LinkKind::Needs,
        label: "Login result".into(),
        waiting_items: vec![],
    };
    let ops = [task(4), PlanOp::LinkPut { link: link.clone() }];
    let context = command("source-link");
    let first = storage
        .edit_plan(&context, &Writer::Person, None, &source, 0, &ops)
        .await
        .unwrap();
    let replay = storage
        .edit_plan(&context, &Writer::Person, None, &source, 0, &ops)
        .await
        .unwrap();
    assert_eq!(first, replay);
    assert_eq!(
        storage.get_plan(&source).await.unwrap().links,
        vec![link.clone()]
    );
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(events, 2);
    drop(storage);
    let reopened = Storage::open(&temp.path().join("links.sqlite3"))
        .await
        .unwrap();
    assert_eq!(reopened.get_plan(&source).await.unwrap().links, vec![link]);
}

#[tokio::test]
async fn a_frozen_versions_cross_links_refuse_every_database_write() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    storage
        .edit_plan(
            &command("guard-target"),
            &Writer::Person,
            None,
            &WorkflowId::from_literal(TARGET),
            0,
            &[task(3)],
        )
        .await
        .unwrap();
    let link = Link {
        task: 4,
        after: TaskParent::Plan {
            plan_id: PlanId::from_literal(TARGET_PLAN),
            task: 3,
        },
        kind: shadows_core::LinkKind::Needs,
        label: "Login result".into(),
        waiting_items: vec![],
    };
    storage
        .edit_plan(
            &command("frozen-link"),
            &Writer::Person,
            None,
            &source,
            0,
            &[task(4), PlanOp::LinkPut { link: link.clone() }],
        )
        .await
        .unwrap();
    sqlx::query("UPDATE workflow SET state='Frozen', frozen_at=? WHERE id=?")
        .bind("2026-10-04T00:00:00Z")
        .bind(SOURCE)
        .execute(storage.reader())
        .await
        .unwrap();
    for statement in [
        "DELETE FROM task_plan_parent WHERE workflow_id=?",
        "UPDATE task_plan_parent SET label='changed' WHERE workflow_id=?",
        "INSERT INTO task_plan_parent
           SELECT workflow_id, task_id, parent_plan_id, parent_task+1,
                  kind, label, waiting_items FROM task_plan_parent WHERE workflow_id=?",
    ] {
        let error = sqlx::query(statement)
            .bind(SOURCE)
            .execute(storage.reader())
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("a frozen plan version never changes"),
            "{error}"
        );
        assert_eq!(
            storage.get_plan(&source).await.unwrap().links,
            vec![link.clone()]
        );
    }
    let issued = storage
        .issue_project_grant(&command("copy-grant"), &ProjectId::from_literal("p"))
        .await
        .unwrap();
    let grant = issued.grant.id;
    let draft_ref = storage.prepare_draft(&grant).await.unwrap();
    let mut context = command("copy-links");
    context.principal_kind = "Grant".into();
    context.principal_id = grant.to_string();
    context.command_kind = "DraftStart".into();
    context.command_id = draft_ref.clone();
    let next = storage
        .start_draft(
            &context,
            &Writer::External { grant },
            &ProjectId::from_literal("p"),
            Some(&PlanId::from_literal("source-plan")),
            Some(&source),
            None,
            Some("Continue the same dependency"),
            None,
            Some(&draft_ref),
        )
        .await
        .unwrap();
    assert_eq!(next.version, 2);
    assert_eq!(
        storage.get_plan(&next.workflow_id).await.unwrap().links,
        vec![link]
    );
}

#[tokio::test]
async fn a_failed_cross_link_batch_leaves_no_tasks_revision_or_event() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let invalid = Link {
        task: 4,
        after: TaskParent::Plan {
            plan_id: PlanId::from_literal("55555555-5555-4555-8555-555555555555"),
            task: 3,
        },
        kind: shadows_core::LinkKind::Needs,
        label: "Missing plan".into(),
        waiting_items: vec![],
    };
    assert!(
        storage
            .edit_plan(
                &command("invalid-link"),
                &Writer::Person,
                None,
                &source,
                0,
                &[task(4), PlanOp::LinkPut { link: invalid }],
            )
            .await
            .is_err()
    );
    let plan = storage.get_plan(&source).await.unwrap();
    assert_eq!(plan.revision, 0);
    assert!(plan.tasks.is_empty());
    assert!(plan.links.is_empty());
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(events, 0);
}
