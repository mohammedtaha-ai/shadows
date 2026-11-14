//! Slice 4 on the real programs: executors on the signed-in Claude Code ask
//! `task_brief` and `task_question` about a real Go module with gopls, report
//! a contract conflict that reaches their Planner, and commit side by side;
//! Planners on Claude Code and Codex start a compatible ready group. Live
//! only: run with `-- --ignored --test-threads=1` once gopls is on PATH and
//! the programs are signed in (Codex through `SHADOWS_CODEX_PATH` and
//! `SHADOWS_CODEX_HOME`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use shadows_core::testing::acp;
use shadows_core::{
    AcceptanceItem, Attempt, AttemptState, ContractSource, ContractTarget, ContractUse,
    ContractValue, FileAction, FileWork, Operation, PlanOp, ProducedOutput, SendTurn, SymbolKind,
    SymbolWork, TaskContent, TaskContract, ThreadEntry, ThreadEntryKind, ThreadId, WorkflowId,
};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/live.rs"]
mod live;
#[path = "fixtures/plan.rs"]
mod plan;

const GO_MOD: &str = "module example.com/demo\n\ngo 1.21\n";
const GREET_GO: &str = "package demo\n\n// Hello answers a greeting.\nfunc Hello() string {\n\
                        \treturn \"hi\"\n}\n";
const CALLER_GO: &str = "package demo\n\nfunc Caller() string {\n\treturn Hello()\n}\n";
const MODEL: &str = live::SUBSCRIPTION_MODEL;

/// A listening app on the subscription whose project folder is a Go module
/// in a Git repository, and the Planner thread its tests talk to.
struct World {
    l: listening::Listening,
    root: PathBuf,
    planner: ThreadId,
    codex: bool,
    _live: live::LiveAdapter,
    _tmp: tempfile::TempDir,
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn var(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} must be set")))
}

impl World {
    /// The world, its Planner on Codex when `codex`, else on Claude Code.
    async fn start(codex: bool) -> Self {
        let live = live::subscription_adapter();
        let tmp = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let mcp_url = format!("{base}/mcp");
        let program = codex.then(|| {
            Arc::new(shadows_agent::codex::CodexProgram::new(
                var("SHADOWS_CODEX_PATH"),
                var("SHADOWS_CODEX_HOME"),
            ))
        });
        let config = shadows_core::testing::SessionsConfig {
            mcp_url: Some(mcp_url.clone()),
            codex: program,
            // A cold Codex start takes longer than the fake's setup wait.
            setup_wait: Duration::from_secs(120),
            ..acp::test_config()
        };
        let app = app::test_app_on(tmp.path(), config, &mcp_url, live.adapter.clone()).await;
        let router = app.router.clone();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let project = app.core.projects().get(&app.project).await.unwrap();
        let root = PathBuf::from(project.directory.unwrap());
        for (name, text) in [
            ("go.mod", GO_MOD),
            ("greet.go", GREET_GO),
            ("caller.go", CALLER_GO),
            (".gitignore", "*.sqlite3*\n"),
        ] {
            std::fs::write(root.join(name), text).unwrap();
        }
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "live@example.invalid"]);
        git(&root, &["config", "user.name", "Live"]);
        git(&root, &["config", "core.autocrlf", "false"]);
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "start"]);
        let planner = match codex {
            true => {
                shadows_core::Threads::create_planning_thread_for_test(
                    &app.storage,
                    &app::ctx("codex-thread", "thread.create"),
                    &app.project,
                    "Codex",
                    shadows_agent::policy::CODEX,
                    None,
                )
                .await
                .unwrap()
                .id
            }
            false => app.thread.clone(),
        };
        Self {
            l: listening::Listening { app, base },
            root,
            planner,
            codex,
            _live: live,
            _tmp: tmp,
        }
    }

    fn app(&self) -> &app::App {
        &self.l.app
    }

    /// v1 of the Planner's plan holding `tasks` and `links`, approved.
    async fn approved(&self, tasks: Vec<TaskContent>, links: Vec<PlanOp>) -> WorkflowId {
        let app = self.app();
        let v1 = plan::draft_on(app, &self.planner, "start-1")
            .await
            .workflow_id;
        let mut ops: Vec<PlanOp> = tasks
            .into_iter()
            .map(|task| PlanOp::TaskAdd { task })
            .collect();
        ops.extend(links);
        plan::edit(app, &v1, 0, &ops).await;
        shadows_core::AppCore::plans_for_tests(app.storage.clone())
            .approve_plan_for_test(&app::ctx("approve-1", "PlanApprove"), &v1, 1)
            .await
            .unwrap();
        v1
    }

    /// Sends `prompt` on the Planner's thread and waits up to ten minutes for
    /// the turn, allowing any request its program holds on the way.
    async fn planner_turn(&self, command: &str, prompt: &str) -> Operation {
        let app = self.app();
        let (model, mode) = match self.codex {
            true => {
                app.core.harness().open(&self.planner).await.unwrap();
                let offered = app.core.harness().offered(&self.planner).await.unwrap();
                let mode = shadows_agent::policy::CODEX_ASK.to_string();
                (offered.current.model, mode)
            }
            false => (MODEL.to_string(), "acceptEdits".to_string()),
        };
        let turn = SendTurn {
            command_id: command.into(),
            prompt: prompt.into(),
            attachments: Vec::new(),
            model,
            mode,
            effort: None,
            focus: None,
            plan: None,
            client_tab: None,
        };
        let op = app
            .core
            .turns()
            .send_with_default_effort(self.planner.clone(), turn)
            .await
            .unwrap();
        for _ in 0..6000 {
            self.allow_held().await;
            let now = shadows_core::Turns::get_operation_for_test(&app.storage, &op)
                .await
                .unwrap();
            if now.finished_at.is_some() {
                return now;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the Planner's turn did not end in ten minutes");
    }

    /// Allows every request held on the Planner's thread.
    async fn allow_held(&self) {
        let harness = self.app().core.harness();
        for held in harness.waiting_permissions().await {
            if held.thread_id != self.planner {
                continue;
            }
            let allow = held
                .options
                .iter()
                .find(|o| o.kind.starts_with("allow") || o.option_id == "accept");
            if let Some(option) = allow {
                let _ = harness
                    .answer_permission(
                        format!("allow-{}", held.request_id),
                        &self.planner,
                        &held.request_id,
                        &option.option_id,
                    )
                    .await;
            }
        }
    }

    /// One recorded Planner turn: what a notice to the Planner is sent on.
    async fn planner_ready(&self) {
        let done = self
            .planner_turn("ready", "Answer with the word ready.")
            .await;
        assert_eq!(done.status_kind, "Completed", "{done:?}");
    }

    /// The attempt once it left `Running` and its turn ended, polled up to `seconds`.
    async fn settled(&self, attempt: &Attempt, seconds: u64) -> Attempt {
        let app = self.app();
        for _ in 0..seconds * 10 {
            self.allow_held().await;
            let now = app.core.runs().attempt(&attempt.id).await.unwrap();
            let ended = app
                .core
                .turns()
                .operations(&attempt.thread_id)
                .await
                .unwrap()
                .iter()
                .any(|op| Some(&op.id) == now.operation_id.as_ref() && op.finished_at.is_some());
            if now.state != AttemptState::Running && ended {
                return now;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the attempt did not settle: {attempt:?}");
    }

    /// An MCP client holding an executor grant on `attempt`'s thread and task:
    /// what the executor's own client holds.
    async fn executor_client(&self, attempt: &Attempt) -> listening::Client {
        let (project, thread) = (attempt.project_id.clone(), attempt.thread_id.clone());
        let scope = shadows_core::testing::TaskScope {
            workflow: attempt.workflow_id.as_str().to_string(),
            version: attempt.version,
            task: attempt.task,
        };
        let (_, token) = self
            .app()
            .storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    shadows_core::Grants::issue_executor_grant_in(conn, &project, &thread, scope)
                        .await
                })
            })
            .await
            .unwrap();
        listening::mcp_client(&self.l.base, token.as_str()).await
    }

    async fn close(self) {
        self.app()
            .core
            .shut_down(Duration::from_secs(10), std::future::pending())
            .await
            .unwrap();
    }
}

fn value(name: &str, type_name: &str, meaning: &str) -> ContractValue {
    ContractValue {
        name: name.into(),
        type_name: type_name.into(),
        meaning: meaning.into(),
    }
}

/// Task `n` creating `file` with one function `name(name string) string`,
/// produced as `o1`, built on `uses`.
fn go_task(n: u32, file: &str, name: &str, behavior: &str, uses: ContractUse) -> TaskContent {
    TaskContent {
        phases: Vec::new(),
        number: n,
        title: format!("Add {name}"),
        goal: format!("Add {name} in {file} exactly as the contract says."),
        reads: vec!["greet.go".into()],
        writes: vec![file.into()],
        acceptance: vec![AcceptanceItem {
            number: 1,
            text: format!("{name} answers as its contract says"),
        }],
        contract: Some(TaskContract {
            files: vec![FileWork {
                path: file.into(),
                action: FileAction::Create,
                purpose: format!("holds {name}"),
            }],
            symbols: vec![SymbolWork {
                id: "s1".into(),
                path: file.into(),
                kind: SymbolKind::Function,
                name: name.into(),
                inputs: vec![value("name", "string", "who is greeted")],
                returns: Some(value("", "string", "the greeting")),
                behavior: vec![behavior.into()],
                errors: Vec::new(),
            }],
            produces: vec![ProducedOutput {
                id: "o1".into(),
                symbol_id: Some("s1".into()),
                artifact: None,
                acceptance: vec![1],
            }],
            uses: vec![uses],
            steps: Vec::new(),
            invariants: Vec::new(),
            forbidden: Vec::new(),
        }),
    }
}

fn existing(symbol: &str) -> ContractUse {
    ContractUse {
        source: ContractSource::Existing {
            path: "greet.go".into(),
            symbol: symbol.into(),
        },
        purpose: format!("{symbol} from greet.go gives the greeting"),
    }
}

/// T`n` writing `shout.go`, built on `symbol` of greet.go.
fn shout(n: u32, symbol: &str) -> TaskContent {
    let behavior = format!("Answers {symbol}'s greeting, a space, then the name in upper case.");
    go_task(n, "shout.go", "Shout", &behavior, existing(symbol))
}

/// T`n` writing `wave.go` on Hello, independent of shout.go.
fn wave(n: u32) -> TaskContent {
    let behavior = "Answers Hello's greeting, a comma, then the name.";
    go_task(n, "wave.go", "Wave", behavior, existing("Hello"))
}

/// T2 writing `banner.go`, the planned consumer of T1's `o1`.
fn banner() -> TaskContent {
    let uses = ContractUse {
        source: ContractSource::Task {
            target: ContractTarget::Local { task: 1 },
            output: "o1".into(),
        },
        purpose: "Banner shouts the name with T1's Shout".into(),
    };
    let behavior = "Answers Shout(name) between two stars.";
    go_task(2, "banner.go", "Banner", behavior, uses)
}

fn called(entries: &[ThreadEntry], tool: &str) -> bool {
    entries
        .iter()
        .any(|e| e.kind == ThreadEntryKind::ToolCall && format!("{e:?}").contains(tool))
}

/// Answers `question` through the executor's own tool, retried for up to two
/// minutes while gopls starts and the code half is not answered yet.
async fn asked(client: &listening::Client, question: Value) -> Value {
    let mut last = Value::Null;
    for _ in 0..60 {
        last = listening::ok(client, "task_question", question.clone()).await;
        if last["code"]["answer"] == "answered" {
            return last;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    panic!("the code half was never answered: {last}");
}

/// An executor asks about the output it builds: the plan names T2 as its
/// planned consumer while gopls answers only the code that exists, and the
/// two never mix.
#[tokio::test]
#[ignore = "live: needs gopls on PATH and the signed-in Claude Code subscription"]
async fn live_executor_asks_about_a_planned_consumer() {
    let w = World::start(false).await;
    let app = w.app();
    let mut t1 = shout(1, "Hello");
    t1.goal = "Add Shout in shout.go exactly as the contract says. Before you write it, ask \
               task_question which planned task uses your output, and ask it with code \
               findReferences on Hello in greet.go, line 4, character 6."
        .into();
    let v1 = w
        .approved(vec![t1, banner()], vec![plan::needs(2, 1)])
        .await;
    w.planner_ready().await;
    let attempt = app
        .core
        .runs()
        .start("q-t1".into(), &v1, 1, 1, MODEL.into())
        .await
        .unwrap();
    let client = w.executor_client(&attempt).await;
    let answer = asked(
        &client,
        json!({
            "selector": { "kind": "own" },
            "code": {
                "operation": "findReferences",
                "file": "greet.go",
                "line": 4,
                "character": 6,
            },
        }),
    )
    .await;
    println!("live task_question answer: {answer}");
    assert_eq!(answer["attempt_id"], attempt.id.as_str(), "{answer}");
    assert_eq!(answer["planned_scope"], "this_frozen_version", "{answer}");
    assert!(answer["code_error"].is_null(), "{answer}");
    let consumers = answer["planning"]["consumers"].as_array().unwrap();
    assert!(
        consumers
            .iter()
            .any(|c| c["task"] == 2 && c["output"] == "o1"),
        "T2 is the planned consumer: {answer}"
    );
    let code = answer["code"]["result"].to_string();
    assert!(code.contains("caller.go"), "{code}");
    assert!(
        !code.contains("banner.go"),
        "a planned consumer is no source location: {code}"
    );
    let now = w.settled(&attempt, 900).await;
    let entries = app
        .core
        .threads()
        .entries(&attempt.thread_id)
        .await
        .unwrap();
    for tool in ["task_brief", "task_question"] {
        assert!(called(&entries, tool), "the executor did not call {tool}");
    }
    println!(
        "live executor: state={:?} commit={:?}",
        now.state, now.commit
    );
    w.close().await;
}

/// The contract names a function greet.go does not hold: the executor
/// reports a conflict, the Planner hears it with its card, and the attempt
/// ends stopped with its scope freed and no commit.
async fn conflict_reaches_the_planner(codex: bool) {
    let w = World::start(codex).await;
    let app = w.app();
    let v1 = w.approved(vec![shout(1, "Farewell")], Vec::new()).await;
    w.planner_ready().await;
    let attempt = app
        .core
        .runs()
        .start("c-t1".into(), &v1, 1, 1, MODEL.into())
        .await
        .unwrap();
    let mut conflict = None;
    for _ in 0..6000 {
        conflict = app.core.runs().conflict(&attempt.id).await.unwrap();
        if conflict.is_some() {
            break;
        }
        let now = app.core.runs().attempt(&attempt.id).await.unwrap();
        assert_eq!(now.state, AttemptState::Running, "no conflict: {now:?}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let conflict = conflict.expect("the executor reported no conflict");
    println!(
        "live conflict: expected={:?} observed={:?}",
        conflict.expected, conflict.observed
    );
    let mut notice = None;
    for _ in 0..1800 {
        let entries = app::entries_on(app, &w.planner).await;
        notice = entries.into_iter().find(|e| {
            e.card
                .as_ref()
                .is_some_and(|c| !c["conflict"].is_null() && c.get("conflict").is_some())
        });
        if notice.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let notice = notice.expect("the Planner was not told of the conflict");
    let card = notice.card.clone().unwrap();
    assert_eq!(notice.author.id, attempt.thread_id.as_str());
    assert_eq!(
        card["conflict"]["attempt_id"],
        attempt.id.as_str(),
        "{card}"
    );
    println!("live conflict card: {card}");
    let now = w.settled(&attempt, 900).await;
    assert_eq!(now.state, AttemptState::Stopped, "{now:?}");
    assert!(now.commit.is_none(), "{now:?}");
    println!("live conflict stop: reason={:?}", now.reason);
    let settled = app
        .core
        .runs()
        .conflict(&attempt.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!settled.scope_held, "{settled:?}");
    assert_eq!(
        format!("{:?}", settled.delivery),
        "Delivered",
        "{settled:?}"
    );
    let entries = app
        .core
        .threads()
        .entries(&attempt.thread_id)
        .await
        .unwrap();
    assert!(called(&entries, "task_conflict"));
    assert!(!called(&entries, "task_report"));
    assert!(!app.core.runs().task(&v1, 1, 1).await.unwrap().done);
    w.close().await;
}

#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_contract_conflict_reaches_the_planner() {
    conflict_reaches_the_planner(false).await;
}

#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription and codex \
            (SHADOWS_CODEX_PATH) signed in to SHADOWS_CODEX_HOME"]
async fn live_contract_conflict_reaches_a_codex_planner() {
    conflict_reaches_the_planner(true).await;
}

/// Both attempts of `v1` T1 and T2 found `Running` at once, before either
/// reported; polled for up to fifteen minutes.
async fn both_running(app: &app::App, v1: &WorkflowId) -> (Attempt, Attempt) {
    for _ in 0..9000 {
        let one = app.core.runs().attempts(v1, 1, 1).await.unwrap();
        let two = app.core.runs().attempts(v1, 1, 2).await.unwrap();
        if let (Some(a), Some(b)) = (one.first(), two.first()) {
            let running = |x: &Attempt| x.state == AttemptState::Running && x.report.is_none();
            assert!(
                running(a) && running(b),
                "the attempts did not overlap: {a:?} {b:?}"
            );
            return (a.clone(), b.clone());
        }
        for a in one.iter().chain(two.iter()) {
            assert_eq!(a.state, AttemptState::Running, "one ended alone: {a:?}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the two tasks were not both started");
}

/// Asked to run the plan, the Planner asks `task_ready` and starts both
/// independent tasks, which run at the same time.
async fn planner_starts_independent_tasks(codex: bool) {
    let w = World::start(codex).await;
    let app = w.app();
    let v1 = w
        .approved(vec![shout(1, "Hello"), wave(2)], Vec::new())
        .await;
    let prompt = format!(
        "Plan version {v1} is approved. Call task_ready for it once, then call task_start with \
         model {MODEL} for every task it lists, one after the other, without waiting for any to \
         finish. Do nothing else."
    );
    let (turn, (a, b)) = tokio::join!(w.planner_turn("group", &prompt), both_running(app, &v1));
    assert_eq!(turn.status_kind, "Completed", "{turn:?}");
    let entries = app::entries_on(app, &w.planner).await;
    for tool in ["task_ready", "task_start"] {
        assert!(called(&entries, tool), "the Planner did not call {tool}");
    }
    println!(
        "live group: T1 started {} T2 started {}",
        a.started_at, b.started_at
    );
    for attempt in [&a, &b] {
        let stopped = app.core.runs().stop(&attempt.id).await.unwrap();
        println!("live group stop: {:?}", stopped.state);
    }
    w.close().await;
}

#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_planner_starts_independent_tasks_together() {
    planner_starts_independent_tasks(false).await;
}

#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription and codex \
            (SHADOWS_CODEX_PATH) signed in to SHADOWS_CODEX_HOME"]
async fn live_codex_planner_starts_independent_tasks_together() {
    planner_starts_independent_tasks(true).await;
}

const FOREIGN: &str = "staged by someone else\n";

/// Two executors work at once beside a change someone else staged: each
/// reports an accepted commit of its own file only, and the staged change
/// stays staged as it was.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_parallel_commits_keep_their_own_files() {
    let w = World::start(false).await;
    let app = w.app();
    let v1 = w
        .approved(vec![shout(1, "Hello"), wave(2)], Vec::new())
        .await;
    w.planner_ready().await;
    std::fs::write(w.root.join("notes.txt"), FOREIGN).unwrap();
    git(&w.root, &["add", "notes.txt"]);
    let runs = app.core.runs();
    let one = runs.start("p-t1".into(), &v1, 1, 1, MODEL.into()).await;
    let two = runs.start("p-t2".into(), &v1, 1, 2, MODEL.into()).await;
    let (one, two) = (one.unwrap(), two.unwrap());
    let (one, two) = tokio::join!(w.settled(&one, 900), w.settled(&two, 900));
    for (attempt, file) in [(&one, "shout.go"), (&two, "wave.go")] {
        assert_eq!(attempt.state, AttemptState::Committed, "{attempt:?}");
        let sha = attempt.commit.clone().unwrap();
        let files = git(&w.root, &["show", "--name-only", "--format=", &sha]);
        assert_eq!(files.trim(), file, "{sha} holds {files}");
        println!("live commit T{}: {sha} {}", attempt.task, files.trim());
    }
    let (first_end, second_start) = match one.started_at <= two.started_at {
        true => (
            one.ended_at.clone().unwrap_or_else(|| "9999".into()),
            &two.started_at,
        ),
        false => (
            two.ended_at.clone().unwrap_or_else(|| "9999".into()),
            &one.started_at,
        ),
    };
    assert!(
        second_start < &first_end,
        "the attempts did not overlap: {one:?} {two:?}"
    );
    let staged = git(&w.root, &["diff", "--cached", "--name-only"]);
    assert_eq!(staged.trim(), "notes.txt", "{staged}");
    assert_eq!(git(&w.root, &["show", ":notes.txt"]), FOREIGN);
    assert_eq!(
        std::fs::read_to_string(w.root.join("notes.txt")).unwrap(),
        FOREIGN
    );
    for n in [1, 2] {
        assert!(
            !runs.task(&v1, 1, n).await.unwrap().done,
            "T{n} marked done"
        );
    }
    w.close().await;
}

const WAVE_GO: &str = "package demo\n\n// Wave is another task's staged work.\n\
                       func Wave(name string) string {\n\treturn Hello() + \", \" + name\n}\n";
const SHOUT_GO: &str = "package demo\n\nimport \"strings\"\n\n\
                        func Shout(name string) string {\n\
                        \treturn Hello() + \" \" + strings.ToUpper(name)\n}\n";

/// A commit that swept in another task's staged file is no accepted report:
/// `task_report` refuses it as outside the writes, nothing is marked done,
/// and the other task's change stays as it was.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_cross_scope_staging_is_not_an_accepted_report() {
    let w = World::start(false).await;
    let app = w.app();
    let v1 = w
        .approved(vec![shout(1, "Hello"), wave(2)], Vec::new())
        .await;
    w.planner_ready().await;
    let attempt = app
        .core
        .runs()
        .start("s-t1".into(), &v1, 1, 1, MODEL.into())
        .await
        .unwrap();
    let client = w.executor_client(&attempt).await;
    std::fs::write(w.root.join("wave.go"), WAVE_GO).unwrap();
    git(&w.root, &["add", "wave.go"]);
    std::fs::write(w.root.join("shout.go"), SHOUT_GO).unwrap();
    git(&w.root, &["add", "shout.go"]);
    git(
        &w.root,
        &["commit", "-q", "-m", "shout, with what was staged"],
    );
    let sha = git(&w.root, &["rev-parse", "HEAD"]).trim().to_string();
    let swept = git(&w.root, &["show", "--name-only", "--format=", &sha]);
    assert!(swept.contains("wave.go"), "{swept}");
    let args = json!({ "commit": sha, "report": "Added Shout." });
    let refusal = listening::refused(&client, "task_report", args).await;
    println!("live refusal: {refusal}");
    assert!(
        refusal.to_lowercase().contains("outside") && refusal.contains("wave.go"),
        "{refusal}"
    );
    let now = app.core.runs().attempt(&attempt.id).await.unwrap();
    assert!(now.commit.is_none(), "{now:?}");
    assert_ne!(now.state, AttemptState::Committed, "{now:?}");
    let stopped = app.core.runs().stop(&attempt.id).await.unwrap();
    assert_ne!(stopped.state, AttemptState::Committed, "{stopped:?}");
    assert!(stopped.commit.is_none(), "{stopped:?}");
    for n in [1, 2] {
        let task = app.core.runs().task(&v1, 1, n).await.unwrap();
        assert!(!task.done, "T{n} marked done");
    }
    assert_eq!(
        std::fs::read_to_string(w.root.join("wave.go")).unwrap(),
        WAVE_GO
    );
    assert_eq!(git(&w.root, &["show", "HEAD:wave.go"]), WAVE_GO);
    w.close().await;
}
