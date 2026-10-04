//! Dependency views read the target's latest content without rewriting links.

#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use cross_plan::{SOURCE, TARGET, TARGET_PLAN, command, fixture, task};
use shadows_core::testing::Writer;
use shadows_core::{Link, LinkKind, PlanId, PlanOp, TaskParent, WorkflowId};

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
