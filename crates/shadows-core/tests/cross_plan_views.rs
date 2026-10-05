//! Dependency views read the target's latest content without rewriting links.

#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use cross_plan::{SOURCE, TARGET, TARGET_PLAN, command, fixture, task};
use shadows_core::testing::Writer;
use shadows_core::{Link, LinkKind, PlanId, PlanOp, ProjectId, TaskParent, WorkflowId};

#[tokio::test]
async fn outgoing_and_incoming_views_name_the_related_plan_and_task() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    storage
        .edit_plan(
            &command("target"),
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
        kind: LinkKind::Needs,
        label: "Login API".into(),
        waiting_items: vec![],
    };
    storage
        .edit_plan(
            &command("source"),
            &Writer::Person,
            None,
            &source,
            0,
            &[task(4), PlanOp::LinkPut { link: link.clone() }],
        )
        .await
        .unwrap();
    let outgoing = storage.get_plan(&source).await.unwrap();
    assert_eq!(outgoing.linked_tasks.len(), 1);
    let related = &outgoing.linked_tasks[0];
    assert!(!related.incoming);
    assert_eq!(related.link, link);
    assert_eq!(related.plan_title.as_deref(), Some("Backend"));
    assert_eq!(related.workflow_id.as_ref(), Some(&target));
    assert_eq!(related.task.as_ref().unwrap().number, 3);
    assert!(related.broken.is_none());
    let incoming = storage.get_plan(&target).await.unwrap();
    assert_eq!(incoming.linked_tasks.len(), 1);
    assert!(incoming.linked_tasks[0].incoming);
    assert_eq!(incoming.linked_tasks[0].plan_title.as_deref(), Some("Web"));
    assert_eq!(incoming.linked_tasks[0].task.as_ref().unwrap().number, 4);
    storage
        .edit_plan(
            &command("remove-target"),
            &Writer::Person,
            None,
            &target,
            1,
            &[PlanOp::TaskRemove { number: 3 }],
        )
        .await
        .unwrap();
    let broken = storage.get_plan(&source).await.unwrap();
    assert_eq!(broken.links, vec![link]);
    assert!(broken.linked_tasks[0].task.is_none());
    assert!(
        broken.linked_tasks[0]
            .broken
            .as_ref()
            .unwrap()
            .contains("T3")
    );
}

#[tokio::test]
async fn incoming_previews_obey_the_grants_one_way_project_reach() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    sqlx::raw_sql(
        "INSERT INTO project (id,slug,name,directory,created_at)
           VALUES ('q','q','Private backend','./q','2026-10-04T00:00:00Z');
         INSERT INTO planning_thread (id,project_id,title,status,created_at)
           VALUES ('q-writer','q','Writer','Open','2026-10-04T00:00:00Z');
         UPDATE workflow SET written_by_thread='q-writer'
           WHERE id='33333333-3333-4333-8333-333333333333';
         UPDATE plan SET project_id='q' WHERE id='44444444-4444-4444-8444-444444444444';
         INSERT INTO project_link VALUES ('q','p','2026-10-04T00:00:00Z');",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    storage
        .edit_plan(
            &command("source"),
            &Writer::Person,
            None,
            &source,
            0,
            &[task(4)],
        )
        .await
        .unwrap();
    storage
        .edit_plan(
            &command("target"),
            &Writer::Person,
            None,
            &target,
            0,
            &[
                task(3),
                PlanOp::LinkPut {
                    link: Link {
                        task: 3,
                        after: TaskParent::Plan {
                            plan_id: PlanId::from_literal("source-plan"),
                            task: 4,
                        },
                        kind: LinkKind::Needs,
                        label: "Screen".into(),
                        waiting_items: vec![],
                    },
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        storage.get_plan(&source).await.unwrap().linked_tasks.len(),
        1
    );
    let origin = ProjectId::from_literal("p");
    assert!(
        storage
            .get_plan_scoped(&source, Some(&origin), None)
            .await
            .unwrap()
            .linked_tasks
            .is_empty()
    );
    sqlx::query("INSERT INTO project_link VALUES ('p','q','2026-10-04T00:00:00Z')")
        .execute(storage.reader())
        .await
        .unwrap();
    let allowed = storage
        .get_plan_scoped(&source, Some(&origin), None)
        .await
        .unwrap();
    assert_eq!(
        allowed.linked_tasks[0].project_name.as_deref(),
        Some("Private backend")
    );
    sqlx::query("DELETE FROM project_link WHERE project_id='p' AND linked_project_id='q'")
        .execute(storage.reader())
        .await
        .unwrap();
    assert!(
        storage
            .get_plan_scoped(&source, Some(&origin), None)
            .await
            .unwrap()
            .linked_tasks
            .is_empty()
    );
}

#[tokio::test]
async fn a_frozen_sources_preview_follows_the_targets_new_version() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    storage
        .edit_plan(
            &command("target"),
            &Writer::Person,
            None,
            &target,
            0,
            &[task(3)],
        )
        .await
        .unwrap();
    storage
        .edit_plan(
            &command("source"),
            &Writer::Person,
            None,
            &source,
            0,
            &[
                task(4),
                PlanOp::LinkPut {
                    link: Link {
                        task: 4,
                        after: TaskParent::Plan {
                            plan_id: PlanId::from_literal(TARGET_PLAN),
                            task: 3,
                        },
                        kind: LinkKind::Needs,
                        label: "API".into(),
                        waiting_items: vec![],
                    },
                },
            ],
        )
        .await
        .unwrap();
    storage
        .approve_plan(&command("freeze-source"), &source, 1)
        .await
        .unwrap();
    storage
        .approve_plan(&command("freeze-target"), &target, 1)
        .await
        .unwrap();
    let before = storage.get_plan(&source).await.unwrap().content();
    let project = ProjectId::from_literal("p");
    let grant = storage
        .issue_project_grant(&command("grant"), &project)
        .await
        .unwrap()
        .grant
        .id;
    let draft_ref = storage.prepare_draft(&grant).await.unwrap();
    let mut ctx = command(&draft_ref);
    ctx.principal_kind = "Grant".into();
    ctx.principal_id = grant.to_string();
    ctx.command_kind = "DraftStart".into();
    let next = storage
        .start_draft(
            &ctx,
            &Writer::External { grant },
            &project,
            Some(&PlanId::from_literal(TARGET_PLAN)),
            Some(&target),
            None,
            Some("Replace the API task"),
            None,
            Some(&draft_ref),
        )
        .await
        .unwrap();
    storage
        .edit_plan(
            &command("remove-latest-task"),
            &Writer::Person,
            None,
            &next.workflow_id,
            0,
            &[PlanOp::TaskRemove { number: 3 }],
        )
        .await
        .unwrap();
    let after = storage.get_plan(&source).await.unwrap();
    assert_eq!(after.content(), before);
    assert!(
        after.blockers.is_empty(),
        "Frozen approval is never reopened"
    );
    assert_eq!(
        after.linked_tasks[0].workflow_id.as_ref(),
        Some(&next.workflow_id)
    );
    assert!(after.linked_tasks[0].broken.is_some());
    assert_eq!(storage.get_plan(&target).await.unwrap().tasks.len(), 1);
}
