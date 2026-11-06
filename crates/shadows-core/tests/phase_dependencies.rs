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
