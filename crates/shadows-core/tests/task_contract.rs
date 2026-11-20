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

/// A Planner turn on `thread` that stays running, so its proposals are heard.
async fn held_turn(storage: &std::sync::Arc<shadows_core::testing::Storage>, thread: &ThreadId) {
    use shadows_core::testing::turn::{new_turn, turn_command};
    let runtime = shadows_core::testing::Runtime::start(storage.clone())
        .await
        .unwrap();
    let settings = shadows_agent::TurnSettings {
        model: "fake-large".into(),
        mode: "default".into(),
        effort: None,
    };
    let turn = new_turn(thread, &runtime, "decide", &settings);
    let started = shadows_core::Turns::start_turn_for_test(
        storage,
        &turn_command("decide", thread, "decide", &settings),
        turn,
    )
    .await
    .unwrap();
    shadows_core::Turns::mark_operation_started_for_test(
        storage,
        &started.operation_id,
        runtime.instance_id(),
    )
    .await
    .unwrap();
}

fn pinned(number: u32, id: &str, version: i64) -> PlanOp {
    let contract = serde_json::from_value(serde_json::json!({
        "files": [{ "path": "app/Rules.php", "action": "create", "purpose": "The rules" }],
        "decisions": [{ "id": id, "version": version }]
    }))
    .unwrap();
    PlanOp::TaskAdd {
        task: task_with(number, contract),
    }
}

#[tokio::test]
async fn a_missing_element_pin_is_refused_by_design_lookup() {
    let (_tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage);
    let contract = serde_json::from_value(serde_json::json!({
        "files": [{"path": "db/users.sql", "action": "create", "purpose": "Create users"}],
        "elements": [{"id": "E1", "revision": 1, "action": "create", "file": "db/users.sql"}]
    }))
    .unwrap();
    let refused = plans
        .edit_plan_for_test(
            &command("pin"),
            &Writer::Person,
            None,
            &WorkflowId::from_literal(SOURCE),
            0,
            &[PlanOp::TaskAdd {
                task: task_with(1, contract),
            }],
        )
        .await;
    let Err(PlansError::PlanInvalid(problems)) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(
        problems[0].message,
        "T1 pins E1 r1, which this project's design does not have"
    );
}

fn element_task(revision: i64, action: &str, file: &str, symbol: Option<&str>) -> PlanOp {
    let contract = serde_json::from_value(serde_json::json!({
        "files": [{"path": "db/users.sql", "action": "create", "purpose": "Create users"},
            {"path": "web/users.sql", "action": "create", "purpose": "Show users"}],
        "elements": [{"id": "E1", "revision": revision, "action": action,
            "file": file, "symbol_id": symbol}]
    }))
    .unwrap();
    let mut task = task_with(1, contract);
    task.writes = vec!["db/users.sql".into(), "web/users.sql".into()];
    PlanOp::TaskAdd { task }
}

#[tokio::test]
async fn a_pin_names_a_file_the_task_works_and_a_symbol_in_it() {
    let (_tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage);
    let refused = plans
        .edit_plan_for_test(
            &command("bad-element"),
            &Writer::Person,
            None,
            &WorkflowId::from_literal(SOURCE),
            0,
            &[element_task(1, "delete", "db/other.sql", Some("s1"))],
        )
        .await;
    let Err(PlansError::PlanInvalid(problems)) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(
        problems
            .iter()
            .map(|p| p.message.as_str())
            .collect::<Vec<_>>(),
        [
            "T1 contract: the element `E1` names `db/other.sql`, which the task does not create, \
             modify or delete",
            "T1 contract: the element `E1` names the symbol `s1`, which the task does not describe \
             in `db/other.sql`",
            "T1 contract: the element `E1` is deleted by this task, so it names no symbol"
        ]
    );
}

#[tokio::test]
async fn a_task_pins_the_latest_element_in_its_own_part_and_keeps_the_frozen_revision() {
    let (_tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let design = AppCore::design_for_tests(storage.clone());
    let project = shadows_core::ProjectId::from_literal("p");
    let workflow = WorkflowId::from_literal(SOURCE);
    let seed = design.seed_elements_for_tests(&project).await.unwrap();
    held_turn(&storage, &ThreadId::from_literal("writer")).await;
    let planner = grant(&storage).await;
    let input = |id, expected_revision, primary_key| shadows_core::ElementInput {
        id,
        expected_revision,
        part_id: seed.database.clone(),
        outcomes: vec![seed.outcome.clone()],
        content: shadows_core::ElementContent::Table(shadows_core::TableContent {
            name: "users".into(),
            purpose: "Store users".into(),
            columns: vec![shadows_core::TableColumn {
                name: "id".into(),
                type_name: "TEXT".into(),
                meaning: String::new(),
                primary_key,
                references: None,
            }],
        }),
    };
    let first = design
        .put_element_for(&planner, "table-1".into(), input(None, None, false))
        .await
        .unwrap();
    let second = design
        .put_element_for(
            &planner,
            "table-2".into(),
            input(Some(first.id.clone()), Some(first.revision), true),
        )
        .await
        .unwrap();
    for (key, revision, action, file, expected) in [
        (
            "stale",
            1,
            "create",
            "db/users.sql",
            "T1 pins E1 r1, but its latest revision is r2; pin r2".to_owned(),
        ),
        (
            "wrong-part",
            2,
            "create",
            "web/users.sql",
            format!(
                "T1 pins E1 in `web/users.sql`, which its part {} does not own",
                seed.database
            ),
        ),
        (
            "not-deleted",
            2,
            "delete",
            "db/users.sql",
            "T1 deletes E1, but r2 does not delete it; delete it with design_element_delete first"
                .to_owned(),
        ),
    ] {
        let result = plans
            .edit_plan_for_test(
                &command(key),
                &Writer::Person,
                None,
                &workflow,
                0,
                &[element_task(revision, action, file, None)],
            )
            .await;
        let Err(PlansError::PlanInvalid(problems)) = result else {
            panic!("{result:?}")
        };
        assert_eq!(problems[0].message, expected);
    }
    plans
        .edit_plan_for_test(
            &command("latest"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[element_task(2, "create", "db/users.sql", None)],
        )
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("freeze"), &workflow, 1)
        .await
        .unwrap();
    design
        .delete_element_for(
            &planner,
            "remove-table".into(),
            shadows_core::ElementDelete {
                id: second.id.clone(),
                expected_revision: 2,
            },
        )
        .await
        .unwrap();
    let context = plans
        .task_context(&planner, Some(&workflow), 1, None)
        .await
        .unwrap();
    assert_eq!(context.task.contract.unwrap().elements[0].revision, 2);
    assert_eq!(context.elements, vec![second]);

    let next = WorkflowId::from_literal(cross_plan::TARGET);
    let build_deleted = plans
        .edit_plan_for_test(
            &command("build-deleted"),
            &Writer::Person,
            None,
            &next,
            0,
            &[element_task(3, "create", "db/users.sql", None)],
        )
        .await;
    let Err(PlansError::PlanInvalid(problems)) = build_deleted else {
        panic!("{build_deleted:?}");
    };
    assert_eq!(
        problems[0].message,
        "T1 builds E1 r3, which deletes it; use action delete"
    );
    plans
        .edit_plan_for_test(
            &command("pin-deletion"),
            &Writer::Person,
            None,
            &next,
            0,
            &[element_task(3, "delete", "db/users.sql", None)],
        )
        .await
        .unwrap();
}

#[test]
fn a_contract_without_elements_keeps_its_fingerprint() {
    let legacy = serde_json::json!({
        "files": [], "symbols": [], "produces": [], "uses": [], "steps": [],
        "invariants": [], "forbidden": [], "checks": []
    });
    let contract: TaskContract = serde_json::from_value(legacy.clone()).unwrap();
    assert!(contract.elements.is_empty());
    let serialized = serde_json::to_value(contract).unwrap();
    assert_eq!(serialized, legacy);
    assert_eq!(
        shadows_core::testing::CommandContext::fingerprint("TaskContract", &serialized),
        "f49cacdf042a5b7c1a149dccdeef843541994f9de117a9c8f20f1380f4413e15"
    );
}

#[tokio::test]
async fn a_task_pins_only_accepted_decisions() {
    let (_tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let design = AppCore::design_for_tests(storage.clone());
    let workflow = WorkflowId::from_literal(SOURCE);
    held_turn(&storage, &ThreadId::from_literal("writer")).await;
    let planner = grant(&storage).await;
    let input = shadows_core::DecisionInput {
        id: None,
        expected_version: None,
        title: "Queries".into(),
        decision: "Use SQLx.".into(),
        rationale: "Checked queries.".into(),
        source: String::new(),
    };
    design
        .propose_decision(&planner, "d1".into(), input)
        .await
        .unwrap();
    let refused = plans
        .edit_plan_for_test(
            &command("early"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[pinned(1, "D1", 1)],
        )
        .await;
    let Err(PlansError::PlanInvalid(problems)) = refused else {
        panic!("{refused:?}")
    };
    assert!(
        problems
            .iter()
            .any(|p| p.message == "T1 pins D1 v1, which is not an accepted decision"),
        "{problems:?}"
    );
    let shared = shadows_core::ProjectId::from_literal("p");
    design
        .accept_decision("a1".into(), &shared, "D1", 1)
        .await
        .unwrap();
    plans
        .edit_plan_for_test(
            &command("pinned"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[pinned(1, "D1", 1)],
        )
        .await
        .unwrap();
    let plan = plans.get_plan_for_test(&workflow).await.unwrap();
    let saved = plan.tasks[0].content.contract.as_ref().unwrap();
    assert_eq!(
        saved.decisions,
        vec![shadows_core::DecisionPin {
            id: "D1".into(),
            version: 1
        }]
    );
}

#[tokio::test]
async fn a_contract_without_decisions_keeps_its_fingerprint() {
    let json = serde_json::to_string(&find_customer()).unwrap();
    assert!(!json.contains("decisions"), "{json}");
    let back: TaskContract = serde_json::from_str(&json).unwrap();
    assert!(back.decisions.is_empty());
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

/// A task whose contract does `action` to each of `paths`.
fn working(number: u32, work: &[(&str, &str)]) -> PlanOp {
    let files: Vec<_> = work
        .iter()
        .map(|(action, path)| {
            serde_json::json!({ "path": path, "action": action, "purpose": "Work" })
        })
        .collect();
    let contract = serde_json::from_value(serde_json::json!({ "files": files })).unwrap();
    PlanOp::TaskAdd {
        task: task_with(number, contract),
    }
}

fn needs(task: u32, after: u32) -> PlanOp {
    PlanOp::LinkPut {
        link: Link {
            task,
            after: shadows_core::TaskParent::Local(after),
            kind: LinkKind::Needs,
            label: "first".into(),
            waiting_items: vec![],
        },
    }
}

/// Points the fixture's project at `folder`; with `files`, its first listing
/// has finished and holds those files and one ignored path.
async fn listed(
    storage: &shadows_core::testing::Storage,
    folder: &std::path::Path,
    files: Option<&[&str]>,
) {
    sqlx::query("UPDATE project SET directory = ? WHERE id = 'p'")
        .bind(folder.to_str().unwrap())
        .execute(storage.reader())
        .await
        .unwrap();
    let Some(files) = files else { return };
    sqlx::query(
        "INSERT INTO code_inventory_scan (project_id, generation, incomplete, observed_at)
         VALUES ('p', 1, 0, '2026-11-18T00:00:00Z')",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO code_inventory (project_id, path_key, path, kind, excluded_reason)
         VALUES ('p', 'dist/app.js', 'dist/app.js', 'excluded', 'ignored')",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    for file in files {
        sqlx::query(
            "INSERT INTO code_inventory (project_id, path_key, path, kind, excluded_reason)
             VALUES ('p', ?, ?, 'file', NULL)",
        )
        .bind(file)
        .bind(file)
        .execute(storage.reader())
        .await
        .unwrap();
    }
}

async fn edited(
    storage: &std::sync::Arc<shadows_core::testing::Storage>,
    ops: &[PlanOp],
) -> Result<(), Vec<String>> {
    let plans = AppCore::plans_for_tests(storage.clone());
    match plans
        .edit_plan_for_test(
            &command("files"),
            &Writer::Person,
            None,
            &WorkflowId::from_literal(SOURCE),
            0,
            ops,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(PlansError::PlanInvalid(problems)) => {
            Err(problems.into_iter().map(|p| p.message).collect())
        }
        Err(other) => panic!("{other}"),
    }
}

#[tokio::test]
async fn a_task_may_not_modify_a_file_the_project_does_not_have() {
    let (tmp, storage) = fixture().await;
    listed(&storage, tmp.path(), Some(&["src/lib.rs"])).await;
    let refused = edited(
        &storage,
        &[working(
            1,
            &[("modify", "src/missing.rs"), ("delete", "dist/app.js")],
        )],
    )
    .await
    .unwrap_err();
    assert_eq!(
        refused,
        vec![
            concat!(
                "T1 modifies `src/missing.rs`, which the project does not have ",
                "and no task it depends on creates"
            ),
            concat!(
                "T1 deletes `dist/app.js`, which the project does not have ",
                "and no task it depends on creates"
            ),
        ]
    );
    edited(&storage, &[working(1, &[("modify", "src/lib.rs")])])
        .await
        .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_dependency_that_creates_a_file_lets_a_task_modify_it_and_one_that_deletes_it_does_not() {
    let (tmp, storage) = fixture().await;
    listed(&storage, tmp.path(), Some(&["src/lib.rs"])).await;
    let refused = edited(
        &storage,
        &[
            working(1, &[("delete", "src/lib.rs")]),
            working(2, &[("read", "src/lib.rs")]),
            working(3, &[("modify", "src/lib.rs")]),
            needs(2, 1),
            needs(3, 2),
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(
        refused,
        vec!["T3 modifies `src/lib.rs`, which a task it depends on deletes"]
    );
    edited(
        &storage,
        &[
            working(1, &[("create", "src/new.rs")]),
            working(2, &[("read", "src/lib.rs")]),
            working(3, &[("modify", "src/new.rs")]),
            needs(2, 1),
            needs(3, 2),
        ],
    )
    .await
    .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn the_check_is_skipped_when_the_file_list_cannot_answer() {
    let (tmp, storage) = fixture().await;
    // The listing has not finished.
    listed(&storage, tmp.path(), None).await;
    edited(&storage, &[working(1, &[("modify", "src/missing.rs")])])
        .await
        .unwrap();
    storage.close().await.unwrap();
    // The project has no folder, even with a stored listing.
    let (tmp, storage) = fixture().await;
    listed(&storage, &tmp.path().join("gone"), Some(&["src/lib.rs"])).await;
    edited(&storage, &[working(1, &[("modify", "src/missing.rs")])])
        .await
        .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_task_may_not_create_a_file_the_project_already_has() {
    let (tmp, storage) = fixture().await;
    listed(&storage, tmp.path(), Some(&["src/lib.rs"])).await;
    let refused = edited(&storage, &[working(1, &[("create", "src/lib.rs")])])
        .await
        .unwrap_err();
    assert_eq!(
        refused,
        vec![concat!(
            "T1 creates `src/lib.rs`, which the project already has: ",
            "modify it instead, and ask Shadows (code_lsp) what it holds"
        )]
    );
    // Two tasks along one dependency creating the same file, and a create
    // after a dependency deletes the file, are told apart.
    let refused = edited(
        &storage,
        &[
            working(1, &[("create", "src/new.rs")]),
            working(2, &[("create", "src/new.rs")]),
            needs(2, 1),
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(
        refused,
        vec![
            "T2 creates `src/new.rs`, which a task it depends on already creates: modify it instead"
        ]
    );
    edited(
        &storage,
        &[
            working(1, &[("delete", "src/lib.rs")]),
            working(2, &[("create", "src/lib.rs")]),
            needs(2, 1),
        ],
    )
    .await
    .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_create_the_task_already_had_is_not_checked_again() {
    let (tmp, storage) = fixture().await;
    listed(&storage, tmp.path(), Some(&["src/lib.rs"])).await;
    edited(&storage, &[working(1, &[("create", "src/made.rs")])])
        .await
        .unwrap();
    // The task ran and made the file; the next listing holds it.
    sqlx::query(
        "INSERT INTO code_inventory (project_id, path_key, path, kind, excluded_reason)
         VALUES ('p', 'src/made.rs', 'src/made.rs', 'file', NULL)",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    let PlanOp::TaskAdd { task } = working(1, &[("create", "src/made.rs"), ("read", "src/lib.rs")])
    else {
        unreachable!()
    };
    AppCore::plans_for_tests(storage.clone())
        .edit_plan_for_test(
            &command("files-again"),
            &Writer::Person,
            None,
            &WorkflowId::from_literal(SOURCE),
            1,
            &[PlanOp::TaskUpdate { task }],
        )
        .await
        .unwrap();
    storage.close().await.unwrap();
}
