//! The routes over a task's attempts, through the real router. The project
//! folder is a real git repository. The adapter is missing, so an attempt that
//! passes every check is answered `failed` without any agent running; `live`
//! runs the real adapter.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use shadows_core::WorkflowId;
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "runs_routes/live.rs"]
mod live;
#[path = "fixtures/plan.rs"]
mod plan;
#[path = "fixtures/stream.rs"]
mod stream;

use app::{App, call, post};

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {:?}", out);
    String::from_utf8(out.stdout).unwrap()
}

/// The project folder as a person's repository: one commit, the daemon's
/// database ignored.
fn repository(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "routes@example.invalid"]);
    git(root, &["config", "user.name", "Routes"]);
    git(root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join(".gitignore"), "*.sqlite3*\n").unwrap();
    git(root, &["add", ".gitignore"]);
    git(root, &["commit", "-q", "-m", "start"]);
}

async fn folder(app: &App) -> PathBuf {
    let project = app.core.projects().get(&app.project).await.unwrap();
    PathBuf::from(project.directory.unwrap())
}

/// An app whose v1 holds T1 (`src/t1.rs`) and T2 (`src/t2.rs`, needs T1),
/// approved, in a git repository, with no adapter to open.
async fn runs_app() -> (App, WorkflowId) {
    let tmp = tempfile::tempdir().unwrap();
    let missing = acp::adapter_at(PathBuf::from("no-such-adapter"));
    let app = app::test_app_on(tmp.path(), acp::test_config(), acp::MCP_URL, missing)
        .await
        .owning(tmp);
    repository(&folder(&app).await);
    let v1 = plan::approved_v1(&app).await;
    (app, v1)
}

fn tasks(v1: &WorkflowId) -> String {
    format!("/api/workflows/{v1}/versions/1/tasks")
}

async fn start(app: &App, v1: &WorkflowId, task: u32, command: &str) -> (u16, Value) {
    seed_choice(app, v1, task).await;
    let path = format!("{}/{task}/attempts", tasks(v1));
    post(app, &path, json!({ "command_id": command })).await
}

async fn seed_choice(app: &App, workflow: &WorkflowId, task: u32) {
    app.core
        .runs()
        .choose_executor_for_test(workflow, task, "claude-code", "fake-small", Some("high"))
        .await
        .unwrap();
}

#[tokio::test]
async fn the_task_answers_its_needs_start_and_executor_choice() {
    let (app, v1) = runs_app().await;
    let (status, t2) = call(&app, "GET", &format!("{}/2", tasks(&v1)), None).await;
    assert_eq!(status, 200, "{t2}");
    assert_eq!(t2["task"], 2);
    assert_eq!(t2["done"], false);
    assert_eq!(t2["latest"], Value::Null);
    assert_eq!(
        t2["needs"],
        json!([{ "task": 1, "title": "task 1", "done": false }])
    );
    assert_eq!(t2["start"]["allowed"], false);
    assert_eq!(t2["start"]["code"], "waiting");
    assert!(
        t2["start"]["reason"].as_str().unwrap().contains("T1"),
        "{t2}"
    );
    assert_eq!(t2["executor"], Value::Null);
    seed_choice(&app, &v1, 1).await;
    let (_, t1) = call(&app, "GET", &format!("{}/1", tasks(&v1)), None).await;
    assert_eq!(
        t1["start"],
        json!({ "allowed": true, "code": null, "reason": null })
    );
    let (missing, _) = call(&app, "GET", &format!("{}/9", tasks(&v1)), None).await;
    assert_eq!(missing, 404);
}

#[tokio::test]
async fn start_while_waiting_is_409_with_the_waiting_tasks() {
    let (app, v1) = runs_app().await;
    let (status, body) = start(&app, &v1, 2, "s1").await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["code"], "waiting");
    assert!(body["reason"].as_str().unwrap().contains("T1"), "{body}");
}

#[tokio::test]
async fn a_replayed_start_returns_the_same_attempt() {
    let (app, v1) = runs_app().await;
    let (status, first) = start(&app, &v1, 1, "s1").await;
    assert_eq!(status, 201, "{first}");
    assert_eq!(first["number"], 1);
    // Every check passed; with no adapter to open, the attempt is answered failed.
    assert_eq!(first["state"], "failed");
    assert!(
        first["reason"]
            .as_str()
            .unwrap()
            .starts_with("session did not open"),
        "{first}"
    );
    let (status, again) = start(&app, &v1, 1, "s1").await;
    assert_eq!(status, 201);
    assert_eq!(again["id"], first["id"]);
    let path = format!("{}/2/attempts", tasks(&v1));
    let (status, other) = post(&app, &path, json!({ "command_id": "s1" })).await;
    assert_eq!(status, 409);
    assert_eq!(other["code"], "command_conflict");
}

#[tokio::test]
async fn start_without_a_choice_is_409_no_executor_choice() {
    let (app, v1) = runs_app().await;
    let (status, body) = post(
        &app,
        &format!("{}/1/attempts", tasks(&v1)),
        json!({"command_id": "no-choice"}),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["code"], "no_executor_choice");
}

#[tokio::test]
async fn the_person_saves_a_tasks_executor_over_http() {
    let app = app::test_app().await;
    let v1 = plan::approved_v1(&app).await;
    let task = format!("{}/1", tasks(&v1));
    let (status, saved) = call(
        &app,
        "PUT",
        &format!("{task}/executor"),
        Some(json!({
            "command_id": "save", "harness": "claude-code", "model": "fake-large", "effort": "low",
        })),
    )
    .await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["chosen_by"], "person");
    let (status, result) = call(&app, "GET", &task, None).await;
    assert_eq!(status, 200);
    assert_eq!(result["executor"], saved);
}

#[tokio::test]
async fn the_attempts_are_listed_newest_first() {
    let (app, v1) = runs_app().await;
    start(&app, &v1, 1, "s1").await;
    start(&app, &v1, 1, "s2").await;
    let (status, list) = call(&app, "GET", &format!("{}/1/attempts", tasks(&v1)), None).await;
    assert_eq!(status, 200);
    let numbers: Vec<i64> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["number"].as_i64().unwrap())
        .collect();
    assert_eq!(numbers, [2, 1]);
    let (_, t1) = call(&app, "GET", &format!("{}/1", tasks(&v1)), None).await;
    assert_eq!(t1["latest"]["number"], 2);
}

#[tokio::test]
async fn stop_answers_the_attempt_as_it_stands() {
    let (app, v1) = runs_app().await;
    let (_, attempt) = start(&app, &v1, 1, "s1").await;
    let id = attempt["id"].as_str().unwrap();
    let (status, stopped) = post(&app, &format!("/api/attempts/{id}/stop"), json!({})).await;
    assert_eq!(status, 200, "{stopped}");
    // It had already ended: it keeps its state.
    assert_eq!(stopped["state"], "failed");
    let (missing, _) = post(&app, "/api/attempts/nope/stop", json!({})).await;
    assert_eq!(missing, 404);
}

#[tokio::test]
async fn the_diff_of_an_attempt_with_no_commit_is_empty() {
    let (app, v1) = runs_app().await;
    let (_, attempt) = start(&app, &v1, 1, "s1").await;
    let id = attempt["id"].as_str().unwrap();
    let (status, diff) = call(&app, "GET", &format!("/api/attempts/{id}/diff"), None).await;
    assert_eq!(status, 200);
    assert_eq!(diff, json!([]));
}

#[tokio::test]
async fn a_done_mark_frees_the_tasks_that_need_it() {
    let (app, v1) = runs_app().await;
    let path = format!("{}/1/done", tasks(&v1));
    let mark = json!({ "command_id": "m1", "done": true });
    let (status, t1) = call(&app, "PUT", &path, Some(mark.clone())).await;
    assert_eq!(status, 200, "{t1}");
    assert_eq!(t1["done"], true);
    assert!(t1["done_at"].is_string());
    seed_choice(&app, &v1, 2).await;
    let (_, t2) = call(&app, "GET", &format!("{}/2", tasks(&v1)), None).await;
    assert_eq!(t2["needs"][0]["done"], true);
    assert_eq!(t2["start"]["allowed"], true);
    let other = json!({ "command_id": "m1", "done": false });
    let (status, conflict) = call(&app, "PUT", &path, Some(other)).await;
    assert_eq!(status, 409);
    assert_eq!(conflict["code"], "command_conflict");
}

#[tokio::test]
async fn the_folder_decides_start_not_git_and_dirty_scope() {
    let (app, v1) = runs_app().await;
    let root = folder(&app).await;
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src").join("t1.rs"), "fn main() {}\n").unwrap();
    // A change outside T1's writes does not count.
    std::fs::write(root.join("notes.md"), "mine\n").unwrap();
    let (_, t1) = call(&app, "GET", &format!("{}/1", tasks(&v1)), None).await;
    assert_eq!(t1["start"]["code"], "dirty_scope");
    assert!(
        t1["start"]["reason"]
            .as_str()
            .unwrap()
            .contains("src/t1.rs")
    );
    let (status, body) = start(&app, &v1, 1, "s1").await;
    assert_eq!((status, body["code"].as_str()), (409, Some("dirty_scope")));
    std::fs::remove_dir_all(root.join(".git")).unwrap();
    let (_, t1) = call(&app, "GET", &format!("{}/1", tasks(&v1)), None).await;
    assert_eq!(t1["start"]["code"], "not_git");
}

#[tokio::test]
async fn a_file_at_an_attempts_commit_is_served() {
    let (app, v1) = runs_app().await;
    let root = folder(&app).await;
    let first = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src").join("t1.rs"), "fn main() {}\n").unwrap();
    git(&root, &["add", "src/t1.rs"]);
    git(&root, &["commit", "-q", "-m", "T1"]);
    let sha = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
    let id = committed(&app, &v1, "s1", sha).await;
    // An attempt whose commit holds no src/t1.rs: inside writes, not there.
    let before = committed(&app, &v1, "s2", first).await;
    let (_, diff) = call(&app, "GET", &format!("/api/attempts/{id}/diff"), None).await;
    let path = diff[0]["path"].as_str().unwrap().to_string();
    assert_eq!(diff[0]["hunks"][0]["lines"][0]["op"], "add", "{diff}");
    let files = format!("/api/attempts/{id}/files?path=");
    let (status, file) = call(&app, "GET", &format!("{files}{path}"), None).await;
    assert_eq!(status, 200, "{file}");
    assert_eq!(
        file,
        json!({ "path": path, "text": "fn main() {}\n", "cut": false })
    );
    let before = format!("/api/attempts/{before}/files?path=src/t1.rs");
    let (missing, _) = call(&app, "GET", &before, None).await;
    assert_eq!(missing, 404);
    let (outside, body) = call(&app, "GET", &format!("{files}nope.txt"), None).await;
    assert_eq!(
        (outside, body["code"].as_str()),
        (409, Some("outside_writes"))
    );
    let unknown = "/api/attempts/nope/files?path=src/t1.rs";
    let (unknown, _) = call(&app, "GET", unknown, None).await;
    assert_eq!(unknown, 404);
}

/// Starts T1 (answered failed, no adapter) and records `sha` as its commit,
/// as an accepted report would; no executor runs.
async fn committed(app: &App, v1: &WorkflowId, command: &str, sha: String) -> String {
    let (_, attempt) = start(app, v1, 1, command).await;
    let id = attempt["id"].as_str().unwrap().to_string();
    let row = id.clone();
    app.storage
        .write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "UPDATE run_attempts SET state = 'committed', commit_sha = ? WHERE id = ?",
                )
                .bind(&sha)
                .bind(&row)
                .execute(&mut *conn)
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    id
}

async fn recorded_check(app: &App, id: &str, outcome: &str) {
    sqlx::query(
        "INSERT INTO run_checks
        (attempt_id,for_task,check_id,ordinal,run_json,reported,outcome,
         exit_code,output,finished_at)
        VALUES (?,1,'c1',0,'[\"git\",\"--version\"]','passed',?,1,'check output',
                '2026-11-15T00:00:00Z')",
    )
    .bind(id)
    .bind(outcome)
    .execute(app.storage.reader())
    .await
    .unwrap();
}

#[tokio::test]
async fn checks_route_answers_results() {
    let (app, v1) = runs_app().await;
    let sha = git(&folder(&app).await, &["rev-parse", "HEAD"])
        .trim()
        .to_string();
    let id = committed(&app, &v1, "checks", sha).await;
    recorded_check(&app, &id, "failed").await;
    let (status, body) = call(&app, "GET", &format!("/api/attempts/{id}/checks"), None).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body[0]["id"], "c1");
    assert_eq!(body[0]["outcome"], "failed");
    assert_eq!(body[0]["matches"], false);
    assert_eq!(body[0]["output"], "check output");
}

#[tokio::test]
async fn done_route_refuses_without_reason() {
    let (app, v1) = runs_app().await;
    let sha = git(&folder(&app).await, &["rev-parse", "HEAD"])
        .trim()
        .to_string();
    let id = committed(&app, &v1, "checks", sha).await;
    recorded_check(&app, &id, "failed").await;
    let path = format!("{}/1/done", tasks(&v1));
    let (status, body) = call(
        &app,
        "PUT",
        &path,
        Some(json!({"command_id":"judge","done":true})),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["code"], "checks_not_passed");
    let (status, body) = call(
        &app,
        "PUT",
        &path,
        Some(json!({"command_id":"judge","done":true,
        "reason":"Known environment failure; verified manually."})),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["done_reason"],
        "Known environment failure; verified manually."
    );
}

/// The state's counts of the plan's tasks, and T1's line in its part's detail.
async fn state_of(app: &App, part: &shadows_core::PartId) -> (Value, Value) {
    let path = format!("/api/projects/{}/state", app.project);
    let (status, whole) = call(app, "GET", &path, None).await;
    assert_eq!(status, 200, "{whole}");
    let counts = whole["unassigned"]
        .as_array()
        .unwrap()
        .iter()
        .chain(
            whole["parts"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|p| p["plans"].as_array().unwrap().iter()),
        )
        .next()
        .unwrap()["tasks"]
        .clone();
    let (status, detail) = call(app, "GET", &format!("{path}?part_id={part}"), None).await;
    assert_eq!(status, 200, "{detail}");
    let line = detail["part"]["plans"][0]["tasks"][0].clone();
    (counts, line)
}

#[tokio::test]
async fn an_attempt_starting_and_ending_reaches_the_project_stream_and_the_state() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    let (app, v1) = runs_app().await;
    let part = shadows_core::AppCore::design_for_tests(app.storage.clone())
        .seed_roadmap_for_tests(&app.project)
        .await
        .unwrap();
    let (counts, line) = state_of(&app, &part).await;
    assert_eq!(
        counts,
        json!({"total": 2, "done": 0, "started": 0, "not_started": 2})
    );
    assert_eq!(
        (&line["number"], &line["attempt"]),
        (&json!(1), &Value::Null)
    );

    let cursor: i64 = sqlx::query_scalar("SELECT MAX(seq) FROM durable_event")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    let path = format!("/api/projects/{}/events?after={cursor}", app.project);
    let response = app
        .router
        .clone()
        .oneshot(Request::get(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let mut body = response.into_body().into_data_stream();
    let mut pending = String::new();
    assert_eq!(
        stream::next_frame(&mut body, &mut pending).await.0,
        "caught-up"
    );
    let (status, attempt) = start(&app, &v1, 1, "s1").await;
    assert_eq!(status, 201, "{attempt}");
    assert_eq!(attempt["state"], "failed");
    let named = json!({
        "project_id": app.project, "workflow_id": v1, "version": 1, "task": 1,
        "attempt_id": attempt["id"],
    });
    for _ in 0..2 {
        let (_, event) = stream::next_frame(&mut body, &mut pending).await;
        assert_eq!(event["kind"], "TaskProgressChanged", "{event}");
        assert_eq!(event["payload"], named);
    }

    let (counts, line) = state_of(&app, &part).await;
    // T1 started; T2, which needs it, has not.
    assert_eq!(
        counts,
        json!({"total": 2, "done": 0, "started": 1, "not_started": 1})
    );
    assert_eq!(line["done"], false);
    assert_eq!(line["attempt"], "failed");
}
