//! Latest-version resolution and cross-plan approval.

#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use cross_plan::{SOURCE, TARGET, TARGET_PLAN, command, fixture, task};
use shadows_core::testing::Writer;
use shadows_core::{Link, LinkKind, PlanId, PlanOp, TaskParent, WorkflowId};

fn dependency(task: u32, plan: &str, after: u32) -> PlanOp {
    PlanOp::LinkPut {
        link: Link {
            task,
            after: TaskParent::Plan {
                plan_id: PlanId::from_literal(plan),
                task: after,
            },
            kind: LinkKind::Needs,
            label: "API result".into(),
            waiting_items: vec![],
        },
    }
}

#[tokio::test]
async fn a_removed_target_task_keeps_the_link_and_blocks_source_approval() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("target"),
            &Writer::Person,
            None,
            &target,
            0,
            &[task(3)],
        )
        .await
        .unwrap();
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("source"),
            &Writer::Person,
            None,
            &source,
            0,
            &[task(4), dependency(4, TARGET_PLAN, 3)],
        )
        .await
        .unwrap();
    assert!(
        shadows_core::AppCore::plans_for_tests(storage.clone())
            .get_plan_for_test(&source)
            .await
            .unwrap()
            .blockers
            .is_empty()
    );
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("remove-target"),
            &Writer::Person,
            None,
            &target,
            1,
            &[PlanOp::TaskRemove { number: 3 }],
        )
        .await
        .unwrap();
    let broken = shadows_core::AppCore::plans_for_tests(storage.clone())
        .get_plan_for_test(&source)
        .await
        .unwrap();
    assert_eq!(broken.links.len(), 1);
    assert!(
        broken.blockers.iter().any(|p| p.message.contains("T3")),
        "{:?}",
        broken.blockers
    );
    assert!(
        shadows_core::AppCore::plans_for_tests(storage.clone())
            .approve_plan_for_test(&command("approve"), &source, 1)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_cross_plan_cycle_names_its_tasks_and_blocks_approval_until_removed() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("target"),
            &Writer::Person,
            None,
            &target,
            0,
            &[task(3)],
        )
        .await
        .unwrap();
    let local = PlanOp::LinkPut {
        link: Link {
            task: 2,
            after: 4.into(),
            kind: LinkKind::Needs,
            label: "Web foundation".into(),
            waiting_items: vec![],
        },
    };
    let completion = PlanOp::LinkPut {
        link: Link {
            task: 4,
            after: TaskParent::Plan {
                plan_id: PlanId::from_literal(TARGET_PLAN),
                task: 3,
            },
            kind: LinkKind::CompletesAfter,
            label: "API acceptance".into(),
            waiting_items: vec![1],
        },
    };
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("source"),
            &Writer::Person,
            None,
            &source,
            0,
            &[task(2), task(4), local, completion],
        )
        .await
        .unwrap();
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("cycle"),
            &Writer::Person,
            None,
            &target,
            1,
            &[dependency(3, "source-plan", 2)],
        )
        .await
        .unwrap();
    let blocked = shadows_core::AppCore::plans_for_tests(storage.clone())
        .get_plan_for_test(&source)
        .await
        .unwrap();
    let cycle = blocked
        .blockers
        .iter()
        .find(|p| p.message.contains("cycle"))
        .expect("the cross-plan cycle must be visible");
    for number in ["T2", "T3", "T4"] {
        assert!(cycle.message.contains(number), "{}", cycle.message);
    }
    assert!(
        shadows_core::AppCore::plans_for_tests(storage.clone())
            .approve_plan_for_test(&command("approve"), &source, 1)
            .await
            .is_err()
    );
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("uncycle"),
            &Writer::Person,
            None,
            &target,
            2,
            &[PlanOp::LinkRemove {
                task: 3,
                after: TaskParent::Plan {
                    plan_id: PlanId::from_literal("source-plan"),
                    task: 2,
                },
                kind: LinkKind::Needs,
            }],
        )
        .await
        .unwrap();
    assert!(
        shadows_core::AppCore::plans_for_tests(storage.clone())
            .get_plan_for_test(&source)
            .await
            .unwrap()
            .blockers
            .is_empty()
    );
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .approve_plan_for_test(&command("approve"), &source, 1)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_foreign_target_requires_an_outgoing_project_link_and_breaks_after_unlink() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("target"),
            &Writer::Person,
            None,
            &target,
            0,
            &[task(3)],
        )
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO project (id, slug, name, directory, created_at)
           VALUES ('q', 'q', 'Target project', './q', '2026-10-04T00:00:00Z');
         INSERT INTO planning_thread (id, project_id, title, status, created_at)
           VALUES ('q-writer', 'q', 'Target writer', 'Open', '2026-10-04T00:00:00Z');
         UPDATE workflow SET written_by_thread='q-writer'
           WHERE id='33333333-3333-4333-8333-333333333333';
         UPDATE plan SET project_id='q'
           WHERE id='44444444-4444-4444-8444-444444444444';",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    cross_plan::approve_roadmap(&storage, "q", &[TARGET_PLAN]).await;
    let ops = [task(4), dependency(4, TARGET_PLAN, 3)];
    let refused = shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("unlinked"),
            &Writer::Person,
            None,
            &source,
            0,
            &ops,
        )
        .await;
    assert!(
        refused.is_err(),
        "a foreign link must be refused before the project link exists"
    );
    assert_eq!(
        shadows_core::AppCore::plans_for_tests(storage.clone())
            .get_plan_for_test(&source)
            .await
            .unwrap()
            .revision,
        0
    );
    sqlx::query("INSERT INTO project_link VALUES ('p', 'q', '2026-10-04T00:00:00Z')")
        .execute(storage.reader())
        .await
        .unwrap();
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(&command("linked"), &Writer::Person, None, &source, 0, &ops)
        .await
        .unwrap();
    assert!(
        shadows_core::AppCore::plans_for_tests(storage.clone())
            .get_plan_for_test(&source)
            .await
            .unwrap()
            .blockers
            .is_empty()
    );
    sqlx::query("DELETE FROM project_link WHERE project_id='p' AND linked_project_id='q'")
        .execute(storage.reader())
        .await
        .unwrap();
    let broken = shadows_core::AppCore::plans_for_tests(storage.clone())
        .get_plan_for_test(&source)
        .await
        .unwrap();
    assert_eq!(broken.links.len(), 1);
    assert!(
        broken
            .blockers
            .iter()
            .any(|p| p.message.contains("project link"))
    );
    assert!(
        shadows_core::AppCore::plans_for_tests(storage.clone())
            .approve_plan_for_test(&command("approve"), &source, 1)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn an_unrelated_cycle_in_a_target_plan_does_not_block_this_dependency() {
    let (_temp, storage) = fixture().await;
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("target"),
            &Writer::Person,
            None,
            &target,
            0,
            &[task(3), task(5)],
        )
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO plan (id, project_id, state, created_at)
           VALUES ('third-plan', 'p', 'Active', '2026-10-04T00:00:00Z');
         INSERT INTO workflow
           (id, plan_id, state, version, revision, title, goal, written_by_thread,
            created_at, updated_at)
           VALUES ('third', 'third-plan', 'Draft', 1, 0, 'Other work', 'Independent',
             'writer', '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z');",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    let third = WorkflowId::from_literal("third");
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("third"),
            &Writer::Person,
            None,
            &third,
            0,
            &[task(6)],
        )
        .await
        .unwrap();
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("target-cycle"),
            &Writer::Person,
            None,
            &target,
            1,
            &[dependency(5, "third-plan", 6)],
        )
        .await
        .unwrap();
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("third-cycle"),
            &Writer::Person,
            None,
            &third,
            1,
            &[dependency(6, TARGET_PLAN, 5)],
        )
        .await
        .unwrap();
    assert!(
        !shadows_core::AppCore::plans_for_tests(storage.clone())
            .get_plan_for_test(&target)
            .await
            .unwrap()
            .blockers
            .is_empty()
    );
    shadows_core::AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("source"),
            &Writer::Person,
            None,
            &source,
            0,
            &[task(4), dependency(4, TARGET_PLAN, 3)],
        )
        .await
        .unwrap();
    let plan = shadows_core::AppCore::plans_for_tests(storage.clone())
        .get_plan_for_test(&source)
        .await
        .unwrap();
    assert!(
        plan.blockers.is_empty(),
        "unrelated work leaked into approval: {:?}",
        plan.blockers
    );
}
