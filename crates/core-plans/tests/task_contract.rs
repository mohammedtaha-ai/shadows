use core_plans::for_tests::Ops;
use core_plans::{AcceptanceItem, PlanContent, PlanOp, Problem, TaskContent, TaskContract};
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
