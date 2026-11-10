//! A task contract through real storage.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;
use cross_plan::{SOURCE, command, fixture, task};
use shadows_core::{AppCore, PlanOp, PlansError, TaskContent, TaskContract, WorkflowId, Writer};

fn find_customer() -> TaskContract {
    serde_json::from_value(serde_json::json!({
        "files": [
            { "path": "app/Services/CustomerFinder.php", "action": "create",
              "purpose": "Looks customers up" },
            { "path": "app/Models/Customer.php", "action": "read", "purpose": "The model" }
        ],
        "symbols": [{
            "id": "s1", "path": "app/Services/CustomerFinder.php", "kind": "method",
            "name": "findCustomer",
            "inputs": [{ "name": "email", "type_name": "string", "meaning": "The address" }],
            "returns": { "type_name": "?Customer", "meaning": "The match, or null" },
            "behavior": ["Returns the customer whose email matches, ignoring case"],
            "errors": ["InvalidArgumentException when email is empty"]
        }]
    }))
    .unwrap()
}

/// Task `number` of the fixture, carrying `contract`.
fn task_with(number: u32, contract: TaskContract) -> TaskContent {
    let PlanOp::TaskAdd { task } = task(number) else {
        unreachable!("the fixture adds a task")
    };
    TaskContent {
        contract: Some(contract),
        ..task
    }
}

#[tokio::test]
async fn a_contract_is_stored_and_read_back_and_a_legacy_row_is_untouched() {
    let (_tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let workflow = WorkflowId::from_literal(SOURCE);
    let t2 = task_with(2, find_customer());
    plans
        .edit_plan_for_test(
            &command("both"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[task(1), PlanOp::TaskAdd { task: t2 }],
        )
        .await
        .unwrap();
    let stored: Vec<(i64, String)> = sqlx::query_as(
        "SELECT number, contract_json FROM task WHERE workflow_id = ? ORDER BY number",
    )
    .bind(SOURCE)
    .fetch_all(storage.reader())
    .await
    .unwrap();
    assert!(!stored[0].1.contains("contract"), "{}", stored[0].1);
    assert!(stored[1].1.contains("findCustomer"));
    let plan = plans.get_plan_for_test(&workflow).await.unwrap();
    assert_eq!(plan.tasks[0].content.contract, None);
    assert_eq!(plan.tasks[1].content.contract, Some(find_customer()));
    // A refused contract writes nothing.
    let bad = serde_json::from_value(serde_json::json!({
        "files": [{ "path": "../x", "action": "create", "purpose": "Out" }]
    }))
    .unwrap();
    let bad = task_with(3, bad);
    let refused = plans
        .edit_plan_for_test(
            &command("bad"),
            &Writer::Person,
            None,
            &workflow,
            1,
            &[PlanOp::TaskAdd { task: bad }],
        )
        .await;
    assert!(
        matches!(refused, Err(PlansError::PlanInvalid(_))),
        "{refused:?}"
    );
    assert_eq!(
        plans.get_plan_for_test(&workflow).await.unwrap().revision,
        1
    );
}
