#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;
use cross_plan::{SOURCE, command, fixture, task};
use shadows_core::{AppCore, PlanOp, WorkflowId, Writer};

#[tokio::test]
async fn stored_phases_survive_task_updates_and_database_reopen() {
    let (tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let workflow = WorkflowId::from_literal(SOURCE);
    plans
        .edit_plan_for_test(
            &command("add"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[task(1)],
        )
        .await
        .unwrap();
    let phases = serde_json::json!([{
        "id":"persisted-phase", "number":1, "title":"Build", "goal":"Deliver",
        "acceptance":[1], "waits_for":[]
    }]);
    // Seed a recorded contract to exercise persistence before phases_set is available.
    sqlx::query(
        "UPDATE task SET contract_json=json_set(contract_json,'$.phases',json(?))
        WHERE workflow_id=? AND number=1",
    )
    .bind(phases.to_string())
    .bind(workflow.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    let PlanOp::TaskAdd { mut task } = task(1) else {
        unreachable!()
    };
    task.title = "Changed title".into();
    plans
        .edit_plan_for_test(
            &command("update"),
            &Writer::Person,
            None,
            &workflow,
            1,
            &[PlanOp::TaskUpdate { task }],
        )
        .await
        .unwrap();
    let read = plans.get(&workflow).await.unwrap();
    assert_eq!(
        serde_json::to_value(&read.tasks[0].content.phases).unwrap(),
        phases
    );
    let reopened = std::sync::Arc::new(
        shadows_core::testing::Storage::open(&tmp.path().join("links.sqlite3"))
            .await
            .unwrap(),
    );
    let read = AppCore::plans_for_tests(reopened.clone())
        .get(&workflow)
        .await
        .unwrap();
    assert_eq!(read.tasks[0].content.phases[0].title, "Build");
    assert_eq!(read.tasks[0].content.title, "Changed title");
    reopened.close().await.unwrap();
    storage.close().await.unwrap();
}

fn phase(title: &str) -> shadows_core::PhaseInput {
    shadows_core::PhaseInput {
        id: None,
        title: title.into(),
        goal: "Deliver".into(),
        acceptance: vec![1],
        waits_for: vec![],
    }
}

#[tokio::test]
async fn phase_pins_require_approved_versions_and_never_follow_latest() {
    use cross_plan::{TARGET, TARGET_PLAN};
    use shadows_core::{PhaseRef, PlanId};
    let (_tmp, storage) = fixture().await;
    cross_plan::approve_roadmap(&storage, "p", &[TARGET_PLAN]).await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    for (id, workflow, n) in [("source", &source, 1), ("target", &target, 3)] {
        plans
            .edit_plan_for_test(&command(id), &Writer::Person, None, workflow, 0, &[task(n)])
            .await
            .unwrap();
    }
    plans
        .edit_plan_for_test(
            &command("target-phases"),
            &Writer::Person,
            None,
            &target,
            1,
            &[PlanOp::PhasesSet {
                task: 3,
                phases: vec![phase("Prepare")],
            }],
        )
        .await
        .unwrap();
    let target_phase = plans.get(&target).await.unwrap().tasks[0].content.phases[0]
        .id
        .clone();
    let mut waiting = phase("Build");
    waiting.waits_for.push(PhaseRef::Plan {
        plan_id: PlanId::from_literal(TARGET_PLAN),
        version: 1,
        task: 3,
        phase: target_phase,
    });
    let ops = [PlanOp::PhasesSet {
        task: 1,
        phases: vec![waiting],
    }];
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    let result = plans
        .edit_plan_for_test(
            &command("source-phases"),
            &Writer::Person,
            None,
            &source,
            1,
            &ops,
        )
        .await;
    assert!(result.is_err(), "an unapproved pin was accepted");
    assert_eq!(plans.get(&source).await.unwrap().revision, 1);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(before, after);
    plans
        .approve_plan_for_test(&command("approve-target"), &target, 2)
        .await
        .unwrap();
    let result = plans
        .edit_plan_for_test(
            &command("source-phases"),
            &Writer::Person,
            None,
            &source,
            1,
            &ops,
        )
        .await
        .unwrap();
    let id = plans.get(&source).await.unwrap().tasks[0].content.phases[0]
        .id
        .clone();
    let replay = plans
        .edit_plan_for_test(
            &command("source-phases"),
            &Writer::Person,
            None,
            &source,
            1,
            &ops,
        )
        .await
        .unwrap();
    assert_eq!(result, replay);
    assert_eq!(
        plans.get(&source).await.unwrap().tasks[0].content.phases[0].id,
        id
    );
    sqlx::query(
        "INSERT INTO workflow (id,plan_id,state,version,revision,title,goal,
        written_by_thread,previous_version_id,created_at,updated_at)
        SELECT 'target-v2',plan_id,'Draft',2,0,'New target','Changed',
        written_by_thread,id,created_at,updated_at FROM workflow WHERE id=?",
    )
    .bind(target.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    let read = serde_json::to_value(plans.get(&source).await.unwrap()).unwrap();
    assert_eq!(read["linked_phases"][0]["target"]["version"], 1);
    assert_eq!(read["linked_phases"][0]["target"]["workflow_id"], TARGET);
    assert!(read["linked_phases"][0]["broken"].is_null());
    let incoming = serde_json::to_value(plans.get(&target).await.unwrap()).unwrap();
    assert_eq!(
        incoming["linked_phases"][0]["source"]["workflow_id"],
        SOURCE
    );
    sqlx::query(
        "UPDATE task SET contract_json=json_set(contract_json,
        '$.phases[0].waits_for[0].phase','missing-phase') WHERE workflow_id=? AND number=1",
    )
    .bind(source.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    let broken = plans.get(&source).await.unwrap();
    assert!(
        broken
            .blockers
            .iter()
            .any(|p| p.message.contains("missing T3 phase"))
    );
    let broken = serde_json::to_value(broken).unwrap();
    assert!(broken["linked_phases"][0]["target"].is_null());
    assert_eq!(
        broken["linked_phases"][0]["reference"]["phase"],
        "missing-phase"
    );
    assert!(
        broken["linked_phases"][0]["broken"]
            .as_str()
            .unwrap()
            .contains("missing")
    );
    storage.close().await.unwrap();
}

#[tokio::test]
async fn concurrent_approval_and_a_mixed_phase_cycle_cannot_freeze_invalid_content() {
    use cross_plan::{TARGET, TARGET_PLAN};
    use shadows_core::{Link, LinkKind, PhaseRef, PlanId, WorkflowState};
    let (tmp, storage) = fixture().await;
    cross_plan::approve_roadmap(&storage, "p", &[TARGET_PLAN]).await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    for (id, workflow, n) in [("source", &source, 1), ("target", &target, 3)] {
        plans
            .edit_plan_for_test(&command(id), &Writer::Person, None, workflow, 0, &[task(n)])
            .await
            .unwrap();
    }
    plans
        .edit_plan_for_test(
            &command("target-phase"),
            &Writer::Person,
            None,
            &target,
            1,
            &[PlanOp::PhasesSet {
                task: 3,
                phases: vec![phase("Prepare")],
            }],
        )
        .await
        .unwrap();
    plans
        .edit_plan_for_test(
            &command("target-needs-source"),
            &Writer::Person,
            None,
            &target,
            2,
            &[PlanOp::LinkPut {
                link: Link {
                    task: 3,
                    after: shadows_core::TaskParent::Plan {
                        plan_id: PlanId::from_literal("source-plan"),
                        task: 1,
                    },
                    kind: LinkKind::Needs,
                    label: "Build first".into(),
                    waiting_items: vec![],
                },
            }],
        )
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("freeze-target"), &target, 3)
        .await
        .unwrap();
    let id = plans.get(&target).await.unwrap().tasks[0].content.phases[0]
        .id
        .clone();
    let mut input = phase("Build");
    input.waits_for.push(PhaseRef::Plan {
        plan_id: PlanId::from_literal(TARGET_PLAN),
        version: 1,
        task: 3,
        phase: id,
    });
    let ops = [PlanOp::PhasesSet {
        task: 1,
        phases: vec![input],
    }];
    // First prove the mixed cycle, then contend on the same unchanged Draft.
    let error = plans
        .edit_plan_for_test(&command("cycle"), &Writer::Person, None, &source, 1, &ops)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("cycle") && error.contains("P1"), "{error}");
    let other_storage = std::sync::Arc::new(
        shadows_core::testing::Storage::open(&tmp.path().join("links.sqlite3"))
            .await
            .unwrap(),
    );
    let other = AppCore::plans_for_tests(other_storage.clone());
    let approval = command("freeze-source");
    let edit = command("concurrent-cycle");
    let (approved, edited) = tokio::join!(
        plans.approve_plan_for_test(&approval, &source, 1),
        other.edit_plan_for_test(&edit, &Writer::Person, None, &source, 1, &ops),
    );
    assert!(approved.is_ok(), "{approved:?}");
    assert!(edited.is_err(), "{edited:?}");
    let read = plans.get(&source).await.unwrap();
    assert_eq!(read.state, WorkflowState::Frozen);
    assert!(read.tasks[0].content.phases.is_empty());
    other_storage.close().await.unwrap();
    storage.close().await.unwrap();
}
