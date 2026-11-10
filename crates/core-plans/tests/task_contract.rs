use core_plans::for_tests::{Ops, Rules};
use core_plans::{
    AcceptanceItem, Link, LinkKind, PlanContent, PlanOp, Problem, TaskContent, TaskContract,
};
use serde_json::json;

fn task(n: u32, title: &str) -> TaskContent {
    TaskContent {
        phases: Vec::new(),
        contract: None,
        number: n,
        title: title.into(),
        goal: format!("goal {n}"),
        reads: vec![],
        writes: vec![format!("src/t{n}")],
        acceptance: vec![AcceptanceItem {
            number: 1,
            text: format!("T{n} works"),
        }],
    }
}

fn empty() -> PlanContent {
    PlanContent {
        bindings: Vec::new(),
        title: "Login".into(),
        goal: "people can log in".into(),
        tasks: Default::default(),
        links: vec![],
    }
}

fn messages(p: Vec<Problem>) -> Vec<String> {
    p.into_iter().map(|p| p.message).collect()
}

fn find_customer() -> TaskContract {
    serde_json::from_value(json!({
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

fn with(contract: TaskContract) -> TaskContent {
    TaskContent {
        contract: Some(contract),
        ..task(1, "Ship")
    }
}

#[test]
fn a_task_without_a_contract_serializes_as_before() {
    let legacy = task(1, "Ship");
    let json = serde_json::to_value(&legacy).unwrap();
    assert!(json.get("contract").is_none());
    assert_eq!(serde_json::from_value::<TaskContent>(json).unwrap(), legacy);
    let op = serde_json::to_string(&PlanOp::TaskAdd { task: legacy }).unwrap();
    assert!(!op.contains("contract"), "{op}");
}

#[test]
fn a_contract_names_files_and_a_symbol_and_round_trips() {
    let task = TaskContent {
        writes: vec!["app/**".into()],
        ..with(find_customer())
    };
    let op = PlanOp::TaskAdd { task: task.clone() };
    assert!(Ops::apply(&empty(), &[op]).is_ok());
    let json = serde_json::to_value(&task).unwrap();
    assert_eq!(
        json["contract"]["symbols"][0]["returns"]["type_name"],
        "?Customer"
    );
    assert_eq!(serde_json::from_value::<TaskContent>(json).unwrap(), task);
}

#[test]
fn named_work_and_a_glob_scope_coexist() {
    let contract = serde_json::from_value(json!({
        "files": [{ "path": "docker/redis/redis.conf", "action": "create",
                    "purpose": "Redis settings" }]
    }))
    .unwrap();
    let task = TaskContent {
        writes: vec!["docker/**".into()],
        ..with(contract)
    };
    assert!(Ops::apply(&empty(), &[PlanOp::TaskAdd { task }]).is_ok());
}

#[test]
fn a_contract_is_refused_with_every_problem_named() {
    let contract = serde_json::from_value(json!({
        "files": [
            { "path": "../etc/passwd", "action": "read", "purpose": "x" },
            { "path": "app/*.php", "action": "modify", "purpose": "x" },
            { "path": "app/A.php", "action": "create", "purpose": "x" },
            { "path": "app/A.php", "action": "create", "purpose": " " }
        ],
        "symbols": [
            { "id": "s1", "path": "app/B.php", "kind": "function", "name": " ",
              "inputs": [{ "name": "a", "type_name": " ", "meaning": "x" }],
              "behavior": [] },
            { "id": "s1", "path": "app/A.php", "kind": "function", "name": "two",
              "returns": { "type_name": "int", "meaning": "x" },
              "behavior": ["Answers two"] }
        ],
        "steps": [" "]
    }))
    .unwrap();
    let task = with(contract);
    let refused = messages(Ops::apply(&empty(), &[PlanOp::TaskAdd { task }]).unwrap_err());
    for expected in [
        "T1 contract: `../etc/passwd` is not an exact path inside the project",
        "T1 contract: `app/*.php` is not an exact path inside the project",
        "T1 contract: the file `app/A.php` is named more than once",
        "T1 contract: the file `app/A.php` has no purpose",
        "T1 contract: missing or repeated symbol id `s1`",
        "T1 contract: the symbol `s1` has no name",
        "T1 contract: the symbol `s1` is in `app/B.php`, which the task does not create or modify",
        "T1 contract: an input of `s1` needs a name and a type",
        "T1 contract: `s1` needs a return type",
        "T1 contract: `s1` says no behavior",
        "T1 contract: a blank line in steps",
    ] {
        assert!(
            refused.iter().any(|m| m == expected),
            "{expected}: {refused:?}"
        );
    }
}

#[test]
fn an_empty_contract_a_class_and_a_misspelt_key() {
    let none = with(serde_json::from_value(json!({})).unwrap());
    let refused = messages(Ops::apply(&empty(), &[PlanOp::TaskAdd { task: none }]).unwrap_err());
    assert!(
        refused.iter().any(|m| m == "T1 contract: it names no file"),
        "{refused:?}"
    );
    let class = serde_json::from_value(json!({
        "files": [{ "path": "app/User.php", "action": "create", "purpose": "The user" }],
        "symbols": [{ "id": "s1", "path": "app/User.php", "kind": "class", "name": "User",
                      "behavior": ["Holds one person"] }]
    }))
    .unwrap();
    let task = with(class);
    assert!(Ops::apply(&empty(), &[PlanOp::TaskAdd { task }]).is_ok());
    let misspelt = serde_json::from_value::<TaskContract>(json!({ "file": [] }));
    let error = misspelt.unwrap_err().to_string();
    assert!(error.contains("unknown field `file`"), "{error}");
}

fn needs(task: u32, after: u32) -> Link {
    Link {
        task,
        after: after.into(),
        kind: LinkKind::Needs,
        label: "api".into(),
        waiting_items: vec![],
    }
}

/// T1 "Finder", producing `o1` from its symbol `s1`.
fn producer() -> TaskContent {
    let mut contract = serde_json::to_value(find_customer()).unwrap();
    contract["produces"] = json!([{ "id": "o1", "symbol_id": "s1", "acceptance": [1] }]);
    TaskContent {
        contract: Some(serde_json::from_value(contract).unwrap()),
        ..task(1, "Finder")
    }
}

/// T2, using T1's `o1`.
fn consumer() -> TaskContent {
    let contract = serde_json::from_value(json!({
        "files": [{ "path": "app/Http/CustomerController.php", "action": "create",
                    "purpose": "The endpoint" }],
        "uses": [{ "source": { "kind": "task", "target": { "kind": "local", "task": 1 },
                               "output": "o1" },
                   "purpose": "Looks the customer up" }]
    }))
    .unwrap();
    TaskContent {
        contract: Some(contract),
        ..task(2, "Endpoint")
    }
}

#[test]
fn a_use_needs_its_producer_its_output_and_a_link() {
    let approval = |ops: &[PlanOp]| {
        messages(Rules::approval_problems(
            &Ops::apply(&empty(), ops).unwrap().content,
        ))
    };
    let add = |task: TaskContent| PlanOp::TaskAdd { task };
    // The edit is accepted each time; only approval is blocked.
    assert_eq!(
        approval(&[add(consumer())]),
        ["T2 uses the output `o1` of T1, which does not exist"]
    );
    assert_eq!(
        approval(&[add(task(1, "Finder")), add(consumer())]),
        ["T2 uses the output `o1`, which T1 does not produce"]
    );
    assert_eq!(
        approval(&[add(producer()), add(consumer())]),
        ["T2 uses T1's output `o1` without a link to T1; add a needs link"]
    );
    let linked = [
        add(producer()),
        add(consumer()),
        PlanOp::LinkPut { link: needs(2, 1) },
    ];
    assert!(approval(&linked).is_empty());
}

#[test]
fn two_tasks_using_each_other_are_refused_as_a_cycle() {
    let mut first = serde_json::to_value(producer().contract.unwrap()).unwrap();
    first["uses"] = json!([{ "source": { "kind": "task", "target": { "kind": "local", "task": 2 },
                                         "output": "o2" },
                             "purpose": "Calls the endpoint" }]);
    let t1 = TaskContent {
        contract: Some(serde_json::from_value(first).unwrap()),
        ..producer()
    };
    let mut second = serde_json::to_value(consumer().contract.unwrap()).unwrap();
    second["produces"] = json!([{ "id": "o2", "artifact": "app/Http/CustomerController.php" }]);
    let t2 = TaskContent {
        contract: Some(serde_json::from_value(second).unwrap()),
        ..consumer()
    };
    let ops = [
        PlanOp::TaskAdd { task: t1 },
        PlanOp::TaskAdd { task: t2 },
        PlanOp::LinkPut { link: needs(2, 1) },
        PlanOp::LinkPut { link: needs(1, 2) },
    ];
    assert_eq!(
        messages(Ops::apply(&empty(), &ops).unwrap_err()),
        ["these links form a cycle: T1, T2"]
    );
}

#[test]
fn an_output_and_a_use_are_refused_with_every_problem_named() {
    let mut contract = serde_json::to_value(find_customer()).unwrap();
    contract["produces"] = json!([
        { "id": "o1", "symbol_id": "s1", "artifact": "app/Services/CustomerFinder.php" },
        { "id": "o1", "symbol_id": "s9" },
        { "id": "o2", "artifact": "app/Other.php", "acceptance": [7] }
    ]);
    let existing = json!({ "kind": "existing", "path": "app/Models/Customer.php" });
    contract["uses"] = json!([
        { "source": { "kind": "task", "target": { "kind": "local", "task": 1 }, "output": "o1" },
          "purpose": "Itself" },
        { "source": { "kind": "existing", "path": "app/**" }, "purpose": " " },
        { "source": existing, "purpose": "The model" },
        { "source": existing, "purpose": "The model again" }
    ]);
    let task = with(serde_json::from_value(contract).unwrap());
    let refused = messages(Ops::apply(&empty(), &[PlanOp::TaskAdd { task }]).unwrap_err());
    for expected in [
        "T1 contract: missing or repeated output id `o1`",
        "T1 contract: the output `o1` names exactly one of a symbol or an artifact",
        "T1 contract: the output `o1` names the symbol `s9`, which the task does not describe",
        "T1 contract: the output `o2` names `app/Other.php`, \
         which the task does not create or modify",
        "T1 contract: the output `o2` names acceptance item 7, which T1 does not have",
        "T1 contract: a use has no purpose",
        "T1 contract: `app/**` is not an exact path inside the project",
        "T1 contract: a task cannot use its own output",
        "T1 contract: the same source is used more than once",
    ] {
        assert!(
            refused.iter().any(|m| m == expected),
            "{expected}: {refused:?}"
        );
    }
}
