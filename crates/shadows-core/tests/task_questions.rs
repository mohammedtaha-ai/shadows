//! Task questions: an executor asks about its own running attempt, and the
//! answer keeps the plan's promises (producers and planned consumers of the
//! attempt's own Frozen version) apart from what the code and the attempts
//! show now. A planned consumer is never a source location, and a language
//! server that cannot answer is never an empty answer. The executor runs on
//! `fake_acp`, held until a `release` file appears.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use core_runs::{Attempt, AttemptState, RunsError, TaskQuestion};
use cross_plan::{SOURCE, command};
use serde_json::{Value, json};
use shadows_core::testing::acp;
use shadows_core::testing::{CommandContext, Harness, LiveHandles, ProjectDirectory, Storage};
use shadows_core::{
    AcceptanceItem, AppCore, CodebaseConfig, ContractSelector, ContractSource, ContractTarget,
    ContractUse, ContractValue, CoreParts, FileAction, FileWork, Grant, GrantId, GrantKind, Link,
    LinkKind, LspOperation, LspQuestion, PlanOp, ProducedOutput, SymbolKind, SymbolWork,
    TaskContent, TaskContract, TaskParent, ThreadId, WorkflowId, Writer,
};

struct Fixture {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    sessions: Arc<Harness>,
    workflow: WorkflowId,
    root: PathBuf,
    project: shadows_core::ProjectId,
    planner: ThreadId,
    _tmp: tempfile::TempDir,
}

const FIND_GO: &str = "package lib\n\n// Find answers the customer with that id.\n\
func Find(id string) string {\n\treturn id\n}\n";
const A_GO: &str = "package lib\n\nfunc Helper() int {\n\treturn 1\n}\n";

fn file(path: &str, action: FileAction) -> FileWork {
    FileWork {
        path: path.into(),
        action,
        purpose: "the work".into(),
    }
}

fn find_symbol(behavior: Vec<String>) -> SymbolWork {
    SymbolWork {
        id: "s1".into(),
        path: "lib/find.go".into(),
        kind: SymbolKind::Function,
        name: "Find".into(),
        inputs: vec![ContractValue {
            name: "id".into(),
            type_name: "string".into(),
            meaning: "the customer's id".into(),
        }],
        returns: Some(ContractValue {
            name: String::new(),
            type_name: "string".into(),
            meaning: "the customer".into(),
        }),
        behavior,
        errors: vec![],
    }
}

/// T1's contract: it modifies `lib/find.go`, builds `Find` (output `o1`)
/// and a helper `Other` (output `o2`), and uses the existing `lib/a.go`.
fn producer(behavior: Vec<String>) -> TaskContract {
    TaskContract {
        files: vec![file("lib/find.go", FileAction::Modify)],
        symbols: vec![
            find_symbol(behavior),
            SymbolWork {
                id: "s2".into(),
                name: "Other".into(),
                behavior: vec!["Answers another thing.".into()],
                ..find_symbol(vec![])
            },
        ],
        produces: vec![
            ProducedOutput {
                id: "o1".into(),
                symbol_id: Some("s1".into()),
                artifact: None,
                acceptance: vec![1],
            },
            ProducedOutput {
                id: "o2".into(),
                symbol_id: Some("s2".into()),
                artifact: None,
                acceptance: vec![],
            },
        ],
        uses: vec![existing_use()],
        steps: vec![],
        invariants: vec![],
        forbidden: vec![],
    }
}

fn existing_use() -> ContractUse {
    ContractUse {
        source: ContractSource::Existing {
            path: "lib/a.go".into(),
            symbol: "Helper".into(),
        },
        purpose: "calls the helper".into(),
    }
}

fn uses_o1() -> ContractUse {
    ContractUse {
        source: ContractSource::Task {
            target: ContractTarget::Local { task: 1 },
            output: "o1".into(),
        },
        purpose: "calls Find".into(),
    }
}

/// A consumer creating `<dir>/use.md` with `uses`.
fn consumer(dir: &str, uses: Vec<ContractUse>) -> TaskContract {
    TaskContract {
        files: vec![file(&format!("{dir}/use.md"), FileAction::Create)],
        symbols: vec![],
        produces: vec![],
        uses,
        steps: vec![],
        invariants: vec![],
        forbidden: vec![],
    }
}

fn task(number: u32, writes: &str, contract: TaskContract) -> PlanOp {
    PlanOp::TaskAdd {
        task: TaskContent {
            phases: Vec::new(),
            contract: Some(contract),
            number,
            title: format!("Task {number}"),
            goal: "A usable result".into(),
            reads: vec![],
            writes: vec![writes.into()],
            acceptance: vec![AcceptanceItem {
                number: 1,
                text: "The result works".into(),
            }],
        },
    }
}

fn needs(task: u32, after: u32) -> PlanOp {
    PlanOp::LinkPut {
        link: Link {
            task,
            after: TaskParent::Local(after),
            kind: LinkKind::Needs,
            label: "Find".into(),
            waiting_items: vec![],
        },
    }
}

/// T1 produces `o1`; T2..=T(1+consumers) use it, the last also the existing
/// `lib/a.go`; one more task uses only `lib/a.go`.
fn tasks(behavior: Vec<String>, consumers: u32) -> Vec<PlanOp> {
    let mut ops = vec![task(1, "lib/**", producer(behavior))];
    let last = 1 + consumers;
    for n in 2..=last {
        let mut uses = vec![uses_o1()];
        if n == last {
            uses.push(existing_use());
        }
        ops.push(task(
            n,
            &format!("d{n}/**"),
            consumer(&format!("d{n}"), uses),
        ));
        ops.push(needs(n, 1));
    }
    let only = last + 1;
    ops.push(task(
        only,
        &format!("d{only}/**"),
        consumer(&format!("d{only}"), vec![existing_use()]),
    ));
    ops
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {:?}", out);
    String::from_utf8(out.stdout).unwrap()
}

fn repository(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "runs@example.invalid"]);
    git(root, &["config", "user.name", "Runs"]);
    git(root, &["config", "core.autocrlf", "false"]);
    for (path, text) in [
        ("go.mod", "module example.com/demo\n\ngo 1.21\n"),
        ("lib/find.go", FIND_GO),
        ("lib/a.go", A_GO),
        ("notes/far.zzz", "far away\n"),
    ] {
        let at = root.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
        git(root, &["add", path]);
    }
    git(root, &["commit", "-q", "-m", "start"]);
}

async fn fixture() -> Fixture {
    fixture_with(tasks(vec!["Answers the customer.".into()], 2)).await
}

async fn fixture_with(ops: Vec<PlanOp>) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let root = tmp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    repository(&root);
    let ctx = |id: &str, kind: &str| CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: CommandContext::fingerprint(kind, &Value::Null),
    };
    let project = shadows_core::testing::create_project(
        &storage,
        &ctx("c1", "project.create"),
        "demo",
        "Demo",
        &ProjectDirectory::resolve(&root).unwrap(),
        &shadows_agent::policy::default_modes(),
    )
    .await
    .unwrap();
    let writer = shadows_core::Threads::create_planning_thread_for_test(
        &storage,
        &ctx("c2", "thread.create"),
        &project.id,
        "Planner",
        "claude-code",
        None,
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO plan (id, project_id, state, created_at)
         VALUES ('source-plan', ?, 'Active', '2026-10-04T00:00:00Z')",
    )
    .bind(project.id.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow
           (id, plan_id, state, version, revision, title, goal, written_by_thread,
            created_at, updated_at)
         VALUES (?, 'source-plan', 'Draft', 1, 0, 'Web', 'A usable web app', ?,
           '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
    )
    .bind(SOURCE)
    .bind(writer.id.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    cross_plan::approve_roadmap(&storage, project.id.as_str(), &["source-plan"]).await;
    let workflow = WorkflowId::from_literal(SOURCE);
    let plans = AppCore::plans_for_tests(storage.clone());
    plans
        .edit_plan_for_test(&command("tasks"), &Writer::Person, None, &workflow, 0, &ops)
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("approve"), &workflow, 1)
        .await
        .unwrap();
    let (runtime, _report) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
    let sessions = acp::fake_sessions(storage.clone());
    let core = AppCore::assemble(CoreParts {
        operations: AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        sessions: sessions.clone(),
        storage: storage.clone(),
        runtime: Arc::new(runtime),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(64).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
    });
    core.runs().set_first_prompt_for_tests("wait-for-release");
    Fixture {
        core,
        storage,
        sessions,
        workflow,
        root,
        project: project.id,
        planner: writer.id,
        _tmp: tmp,
    }
}

fn own() -> TaskQuestion {
    TaskQuestion {
        selector: ContractSelector::Own,
        code: None,
        after: None,
        limit: 20,
    }
}

fn hover(file: &str, project: Option<&str>) -> LspQuestion {
    LspQuestion {
        operation: LspOperation::Hover,
        file: Some(file.into()),
        line: Some(4),
        character: Some(6),
        query: None,
        project: project.map(str::to_string),
    }
}

impl Fixture {
    async fn start(&self, command: &str, task: u32) -> Attempt {
        self.core
            .runs()
            .start(command.into(), &self.workflow, 1, task, "fake-large".into())
            .await
            .unwrap()
    }

    /// A stored executor grant for `attempt`'s thread and task, as Harness
    /// issues one when the session opens (the fake session issues none).
    async fn executor(&self, attempt: &Attempt) -> Grant {
        let (project, thread) = (self.project.clone(), attempt.thread_id.clone());
        let scope = shadows_core::testing::TaskScope {
            workflow: SOURCE.into(),
            version: 1,
            task: attempt.task,
        };
        let (grant, _token) = self
            .storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    shadows_core::Grants::issue_executor_grant_in(conn, &project, &thread, scope)
                        .await
                })
            })
            .await
            .unwrap();
        grant
    }

    fn planner(&self) -> Grant {
        Grant {
            id: GrantId::generate(),
            kind: GrantKind::Thread,
            project_id: self.project.clone(),
            thread_id: Some(self.planner.clone()),
            created_at: "2026-11-14T00:00:00Z".into(),
            revoked_at: None,
            scope: None,
        }
    }

    async fn ask(&self, grant: &Grant, question: TaskQuestion) -> Result<Value, RunsError> {
        let answer = self.core.runs().question(grant, question).await?;
        Ok(serde_json::to_value(answer).unwrap())
    }

    /// Commits a change to `lib/find.go`, reports it for `attempt`, marks
    /// its task done and answers the commit.
    async fn commit_and_judge(&self, attempt: &Attempt) -> String {
        std::fs::write(self.root.join("lib/find.go"), format!("{FIND_GO}// done\n")).unwrap();
        git(&self.root, &["add", "--", "lib/find.go"]);
        git(&self.root, &["commit", "-q", "-m", "T1"]);
        let sha = git(&self.root, &["rev-parse", "HEAD"]).trim().to_string();
        let grant = self.executor(attempt).await;
        let reported = self
            .core
            .runs()
            .report(&grant, &sha, "Built Find.".into())
            .await
            .unwrap();
        assert_eq!(reported.state, AttemptState::Committed);
        self.core
            .runs()
            .mark_done("done-1".into(), &self.workflow, 1, attempt.task, true)
            .await
            .unwrap();
        sha
    }

    async fn finish(self) {
        std::fs::write(self.root.join("release"), b"").unwrap();
        self.sessions.close_all().await.unwrap();
        self.storage.close().await.unwrap();
    }
}

fn consumer_tasks(answer: &Value) -> Vec<u64> {
    answer["planning"]["consumers"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"))
        .iter()
        .map(|c| c["task"].as_u64().unwrap())
        .collect()
}

#[tokio::test]
async fn planned_consumers_are_not_code_callers() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    let answer = f.ask(&grant, own()).await.unwrap();
    assert_eq!(answer["attempt_id"], json!(attempt.id), "{answer}");
    assert_eq!(answer["task"], 1);
    assert_eq!(answer["planned_scope"], "this_frozen_version");
    assert!(
        answer["observed_at"]
            .as_str()
            .is_some_and(|t| !t.is_empty())
    );
    assert_eq!(answer["planning"]["selector"], json!({ "kind": "own" }));
    assert_eq!(answer["planning"]["contract"]["symbols"][0]["name"], "Find");
    assert_eq!(consumer_tasks(&answer), [2, 3]);
    for c in answer["planning"]["consumers"].as_array().unwrap() {
        // A planned reference: its task, title, output and use, never a place in source.
        let mut keys: Vec<&str> = c.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(
            keys,
            ["output", "purpose", "task", "title", "use_index"],
            "{c}"
        );
        assert_eq!(c["output"], "o1");
        assert_eq!(c["use_index"], 0);
    }
    // No code question, no code fact and no code failure.
    assert_eq!(answer["code"], Value::Null);
    assert_eq!(answer["code_error"], Value::Null);
    assert_eq!(answer["planning"]["next"], Value::Null);
    // The brief carries the same planned consumers, from the same version.
    let brief = f.core.runs().brief(&attempt.id).await.unwrap();
    let tasks: Vec<u32> = brief.consumers.iter().map(|c| c.task).collect();
    assert_eq!(tasks, [2, 3]);
    assert!(!brief.consumers_more);
    // A symbol selector answers that symbol and the outputs naming it only.
    let symbol = TaskQuestion {
        selector: ContractSelector::Symbol {
            symbol_id: "s2".into(),
        },
        ..own()
    };
    let answer = f.ask(&grant, symbol).await.unwrap();
    let contract = &answer["planning"]["contract"];
    assert_eq!(contract["symbols"].as_array().unwrap().len(), 1, "{answer}");
    assert_eq!(contract["symbols"][0]["id"], "s2");
    assert_eq!(contract["produces"][0]["id"], "o2");
    assert_eq!(contract["files"][0]["path"], "lib/find.go");
    assert_eq!(contract["uses"], json!([]));
    assert_eq!(consumer_tasks(&answer), Vec::<u64>::new(), "nobody uses o2");
    f.finish().await;
}

#[tokio::test]
async fn question_uses_the_attempt_version() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    // A corrective version 2 adds another consumer of T1's output.
    sqlx::query(
        "INSERT INTO workflow (id,plan_id,state,version,revision,title,goal,
         written_by_thread,previous_version_id,created_at,updated_at)
         SELECT 'next-draft',plan_id,'Draft',2,0,'Changed','Changed',
         written_by_thread,id,created_at,updated_at FROM workflow WHERE id=?",
    )
    .bind(SOURCE)
    .execute(f.storage.reader())
    .await
    .unwrap();
    let answer = f.ask(&grant, own()).await.unwrap();
    assert_eq!(answer["plan"]["version"], 1, "{answer}");
    assert_eq!(answer["plan"]["workflow_id"], SOURCE);
    assert_eq!(consumer_tasks(&answer), [2, 3]);
    assert_eq!(answer["planned_scope"], "this_frozen_version");
    f.finish().await;
}

#[tokio::test]
async fn a_use_resolves_its_producer() {
    let f = fixture().await;
    let one = f.start("s1", 1).await;
    let sha = f.commit_and_judge(&one).await;
    let two = f.start("s2", 2).await;
    let grant = f.executor(&two).await;
    let by_task = TaskQuestion {
        selector: ContractSelector::Use {
            source: uses_o1().source,
        },
        ..own()
    };
    let answer = f.ask(&grant, by_task).await.unwrap();
    let planning = &answer["planning"];
    assert_eq!(planning["contract"], Value::Null, "{answer}");
    assert_eq!(planning["resolved"].as_array().unwrap().len(), 1);
    assert_eq!(planning["resolved"][0]["producer_title"], "Task 1");
    assert_eq!(planning["resolved"][0]["symbol"]["name"], "Find");
    assert_eq!(
        planning["producers"],
        json!([{ "kind": "local", "task": 1 }])
    );
    // Another planned user of the same output; never the asker itself.
    assert_eq!(consumer_tasks(&answer), [3]);
    let status = &answer["producer_status"];
    assert_eq!(status.as_array().unwrap().len(), 1, "{answer}");
    assert_eq!(status[0]["target"], json!({ "kind": "local", "task": 1 }));
    assert_eq!(status[0]["done"], true);
    assert_eq!(status[0]["latest_commit"], json!(sha));
    // An existing source names its other users, with no output.
    let three = f.start("s3", 3).await;
    let grant = f.executor(&three).await;
    let by_path = TaskQuestion {
        selector: ContractSelector::Use {
            source: existing_use().source,
        },
        ..own()
    };
    let answer = f.ask(&grant, by_path).await.unwrap();
    assert_eq!(consumer_tasks(&answer), [1, 4], "{answer}");
    for c in answer["planning"]["consumers"].as_array().unwrap() {
        assert_eq!(c["output"], Value::Null);
    }
    assert_eq!(answer["planning"]["producers"], json!([]));
    // A source this task does not declare is not a selector.
    let undeclared = TaskQuestion {
        selector: ContractSelector::Use {
            source: ContractSource::Existing {
                path: "lib/other.go".into(),
                symbol: String::new(),
            },
        },
        ..own()
    };
    let refused = f.ask(&grant, undeclared).await;
    assert!(
        matches!(refused, Err(RunsError::InvalidQuestion(_))),
        "{refused:?}"
    );
    assert_eq!(refused.unwrap_err().code(), "invalid_question");
    f.finish().await;
}

#[tokio::test]
async fn unavailable_code_is_not_absence() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    let asked = TaskQuestion {
        code: Some(hover("lib/find.go", None)),
        ..own()
    };
    let answer = f.ask(&grant, asked).await.unwrap();
    // No server runs: that is a failure to look, never an empty finding.
    assert_eq!(answer["code"], Value::Null, "{answer}");
    let error = answer["code_error"]
        .as_str()
        .unwrap_or_else(|| panic!("{answer}"));
    assert!(!error.is_empty() && error.len() <= 2048, "{error}");
    // The planning facts still answer.
    assert_eq!(consumer_tasks(&answer), [2, 3]);
    f.finish().await;
}

#[tokio::test]
async fn project_code_outside_reads_can_be_asked() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    // `notes/` is neither in T1's reads nor in its writes.
    let asked = TaskQuestion {
        code: Some(hover("notes/far.zzz", None)),
        ..own()
    };
    let answer = f.ask(&grant, asked).await.unwrap();
    assert_eq!(answer["code"], Value::Null, "{answer}");
    assert!(answer["code_error"].as_str().is_some(), "{answer}");
    f.finish().await;
}

#[tokio::test]
async fn unrelated_project_and_link_out_are_refused() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    let elsewhere = TaskQuestion {
        code: Some(hover("lib/find.go", Some("not-linked"))),
        ..own()
    };
    let refused = f.ask(&grant, elsewhere).await;
    assert!(matches!(refused, Err(RunsError::Code(_))), "{refused:?}");
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.go"), "package x\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), f.root.join("out")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(outside.path(), f.root.join("out")).unwrap();
    let linked = TaskQuestion {
        code: Some(hover("out/secret.go", None)),
        ..own()
    };
    let refused = f.ask(&grant, linked).await;
    assert!(matches!(refused, Err(RunsError::Code(_))), "{refused:?}");
    let escaping = TaskQuestion {
        code: Some(hover("../s.sqlite3", None)),
        ..own()
    };
    let refused = f.ask(&grant, escaping).await;
    assert!(matches!(refused, Err(RunsError::Code(_))), "{refused:?}");
    f.finish().await;
}

#[tokio::test]
async fn consumer_pages_are_stable_and_local() {
    let f = fixture_with(tasks(vec!["Answers the customer.".into()], 5)).await;
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    let page = |after: Option<u32>, limit: u16| TaskQuestion {
        after,
        limit,
        ..own()
    };
    let first = f.ask(&grant, page(None, 2)).await.unwrap();
    assert_eq!(consumer_tasks(&first), [2, 3]);
    assert_eq!(first["planning"]["next"], 2);
    let second = f.ask(&grant, page(Some(2), 2)).await.unwrap();
    assert_eq!(consumer_tasks(&second), [4, 5]);
    assert_eq!(second["planning"]["next"], 4);
    let last = f.ask(&grant, page(Some(4), 2)).await.unwrap();
    assert_eq!(consumer_tasks(&last), [6]);
    assert_eq!(last["planning"]["next"], Value::Null);
    // The same page twice is the same page.
    assert_eq!(
        f.ask(&grant, page(Some(2), 2)).await.unwrap()["planning"],
        second["planning"]
    );
    for (after, limit) in [(None, 0), (None, 51), (Some(6), 2)] {
        let refused = f.ask(&grant, page(after, limit)).await;
        assert!(
            matches!(refused, Err(RunsError::InvalidQuestion(_))),
            "{after:?} {limit}: {refused:?}"
        );
    }
    // The brief lists them all from the attempt's version.
    let brief = f.core.runs().brief_for(&grant).await.unwrap();
    assert_eq!(brief.consumers.len(), 5);
    assert!(!brief.consumers_more);
    f.finish().await;
}

#[tokio::test]
async fn oversized_contract_is_not_silently_cut() {
    let behavior: Vec<String> = (0..120)
        .map(|n| format!("Behavior line {n}: {}", "x".repeat(600)))
        .collect();
    let f = fixture_with(tasks(behavior.clone(), 1)).await;
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    let refused = f.ask(&grant, own()).await;
    assert!(
        matches!(refused, Err(RunsError::QuestionTooLarge)),
        "{refused:?}"
    );
    assert_eq!(refused.unwrap_err().code(), "question_too_large");
    let symbol = TaskQuestion {
        selector: ContractSelector::Symbol {
            symbol_id: "s1".into(),
        },
        ..own()
    };
    assert!(matches!(
        f.ask(&grant, symbol).await,
        Err(RunsError::QuestionTooLarge)
    ));
    // A narrower selector answers, whole.
    let narrow = TaskQuestion {
        selector: ContractSelector::Output {
            output_id: "o2".into(),
        },
        ..own()
    };
    let answer = f.ask(&grant, narrow).await.unwrap();
    assert_eq!(answer["planning"]["contract"]["symbols"][0]["id"], "s2");
    assert!(serde_json::to_vec(&answer).unwrap().len() <= 64 * 1024);
    f.finish().await;
}

#[tokio::test]
async fn a_non_executor_cannot_ask() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await;
    let refused = f.ask(&f.planner(), own()).await;
    assert!(matches!(refused, Err(RunsError::NotFound)), "{refused:?}");
    // A grant naming another task than its attempt's reaches nothing.
    let mut crossed = f.executor(&attempt).await;
    crossed.scope.as_mut().unwrap().task = 2;
    let refused = f.ask(&crossed, own()).await;
    assert!(matches!(refused, Err(RunsError::NotFound)), "{refused:?}");
    // A revoked executor grant is refused, whatever it asks.
    let grant = f.executor(&attempt).await;
    sqlx::query("UPDATE mcp_grant SET revoked_at = '2026-11-14T00:00:00Z' WHERE id = ?")
        .bind(grant.id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    let refused = f.ask(&grant, own()).await;
    assert!(matches!(refused, Err(RunsError::Grants(_))), "{refused:?}");
    f.finish().await;
}

#[tokio::test]
async fn an_ended_attempt_cannot_ask() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    f.commit_and_judge(&attempt).await;
    let refused = f.ask(&grant, own()).await;
    assert!(refused.is_err(), "{refused:?}");
    f.finish().await;
}

/// A real Go module: `Find` exists in source and T2's planned use of it is
/// not implemented. gopls finds the declaration only; the planned consumer
/// stays a plan reference with no location.
#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn gopls_answers_beside_a_planned_consumer() {
    let f = fixture().await;
    f.core
        .codebase()
        .start(CodebaseConfig::default())
        .await
        .unwrap();
    let attempt = f.start("s1", 1).await;
    let grant = f.executor(&attempt).await;
    let asked = TaskQuestion {
        code: Some(LspQuestion {
            operation: LspOperation::FindReferences,
            ..hover("lib/find.go", None)
        }),
        ..own()
    };
    let answer = f.ask(&grant, asked).await.unwrap();
    assert_eq!(answer["code"]["answer"], "answered", "{answer}");
    let paths: Vec<&str> = answer["code"]["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["lib/find.go"], "{answer}");
    assert_eq!(consumer_tasks(&answer), [2, 3]);
    assert!(!paths.iter().any(|p| p.starts_with("d2/")));
    f.finish().await;
}
