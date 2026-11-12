//! A task contract through real storage.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;
use cross_plan::{SOURCE, command, fixture, task};
use shadows_core::{
    AppCore, Grant, Grants, Link, LinkKind, PlanOp, PlansError, TaskContent, TaskContract,
    ThreadId, WorkflowId, Writer,
};

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

async fn grant(storage: &std::sync::Arc<shadows_core::testing::Storage>) -> Grant {
    Grants::issue_thread_grant_for_test(storage, &ThreadId::from_literal("writer"))
        .await
        .unwrap()
        .0
}

/// T1 "Finder", producing `o1` from its symbol `s1`.
fn producer() -> TaskContent {
    let mut contract = serde_json::to_value(find_customer()).unwrap();
    contract["produces"] =
        serde_json::json!([{ "id": "o1", "symbol_id": "s1", "acceptance": [1] }]);
    TaskContent {
        title: "Finder".into(),
        writes: vec!["app/**".into()],
        ..task_with(1, serde_json::from_value(contract).unwrap())
    }
}

/// T2, using T1's `o1`.
fn consumer() -> TaskContent {
    let contract = serde_json::from_value(serde_json::json!({
        "files": [{ "path": "app/Http/CustomerController.php", "action": "create",
                    "purpose": "The endpoint" }],
        "uses": [{ "source": { "kind": "task", "target": { "kind": "local", "task": 1 },
                               "output": "o1" },
                   "purpose": "Looks the customer up" }]
    }))
    .unwrap();
    TaskContent {
        writes: vec!["app/**".into()],
        ..task_with(2, contract)
    }
}

#[tokio::test]
async fn task_context_gives_the_consumer_its_producers_contract() {
    let (_tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let grant = grant(&storage).await;
    let workflow = WorkflowId::from_literal(SOURCE);
    let add = |task| PlanOp::TaskAdd { task };
    plans
        .edit_plan_for_test(
            &command("tasks"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[add(producer()), add(consumer())],
        )
        .await
        .unwrap();
    // Without the link, the draft says what blocks it, and approval refuses.
    let blocked = plans.get_plan_for_test(&workflow).await.unwrap().blockers;
    assert!(
        blocked
            .iter()
            .any(|p| p.message.contains("without a link to T1")),
        "{blocked:?}"
    );
    assert!(
        plans
            .approve_plan_for_test(&command("early"), &workflow, 1)
            .await
            .is_err()
    );
    let link = Link {
        task: 2,
        after: 1.into(),
        kind: LinkKind::Needs,
        label: "finder".into(),
        waiting_items: vec![],
    };
    plans
        .edit_plan_for_test(
            &command("link"),
            &Writer::Person,
            None,
            &workflow,
            1,
            &[PlanOp::LinkPut { link }],
        )
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("approve"), &workflow, 2)
        .await
        .unwrap();
    let context = plans
        .task_context(&grant, Some(&workflow), 2, None)
        .await
        .unwrap();
    assert_eq!(context.contracts.len(), 1);
    let resolved = &context.contracts[0];
    assert_eq!(resolved.producer_title.as_deref(), Some("Finder"));
    let symbol = resolved.symbol.as_ref().unwrap();
    assert_eq!(symbol.name, "findCustomer");
    assert_eq!(symbol.returns.as_ref().unwrap().type_name, "?Customer");
    // The consumer stores the reference, never the signature.
    let own = serde_json::to_string(&context.task).unwrap();
    assert!(!own.contains("findCustomer"), "{own}");
    // A task that uses nothing has no `contracts` key.
    let producer = plans
        .task_context(&grant, Some(&workflow), 1, None)
        .await
        .unwrap();
    assert!(
        serde_json::to_value(&producer)
            .unwrap()
            .get("contracts")
            .is_none()
    );
}
