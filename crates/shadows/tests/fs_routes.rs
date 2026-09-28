//! The disk routes a client uses to choose a project directory (spec §1),
//! driven through the real router against temporary directories: what a
//! listing contains and in what order, where browsing starts, what creating a
//! directory refuses, and that every refusal is a 4xx with a stable code.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use shadows::cli::router;
use shadows_core::runtime::Runtime;
use shadows_core::storage::Storage;
use shadows_core::testing::LiveHandles;
use shadows_core::{AppCore, CoreParts};
use shadows_http::AppState;
use tower::ServiceExt;

#[path = "fixtures/acp.rs"]
mod acp;

async fn app(tmp: &Path) -> (Router, tokio::sync::watch::Sender<bool>) {
    let storage = Arc::new(Storage::open(&tmp.join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let app = router(AppState {
        core: AppCore::assemble(CoreParts {
            storage,
            runtime: Arc::new(runtime),
            sessions: acp::fake_sessions(&tmp.join("s.sqlite3")).await,
            handles: Arc::new(LiveHandles::default()),
            bus,
            ui: tokio::sync::broadcast::channel(16).0,
            mcp_url: acp::MCP_URL.to_string(),
        }),
        allowed_origins: Vec::new(),
        shutdown,
    });
    (app, stopping)
}

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(b) => request
            .header("content-type", "application/json")
            .body(Body::from(b.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// A path as a query-string value: percent-encoded, so `\`, `:` and spaces
/// in a Windows temp path arrive intact.
fn listing_uri(path: &Path) -> String {
    let encoded: String = path
        .display()
        .to_string()
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("/api/fs/dirs?path={encoded}")
}

fn canonical(path: &Path) -> String {
    dunce::canonicalize(path).unwrap().display().to_string()
}

#[tokio::test]
async fn a_listing_holds_subdirectories_sorted_without_case_and_flags_hidden_ones() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(tmp.path()).await;
    let root = tmp.path().join("browse");
    for dir in ["Zeta", "alpha", "beta", ".hidden"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join("a-file.txt"), "files are never listed").unwrap();

    let (status, listing) = call(&app, "GET", &listing_uri(&root), None).await;
    assert_eq!(status, StatusCode::OK, "{listing}");

    let names: Vec<&str> = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, [".hidden", "alpha", "beta", "Zeta"], "{listing}");

    let dir = canonical(&root);
    assert_eq!(listing["path"], dir.as_str());
    assert_eq!(listing["parent"], canonical(tmp.path()).as_str());
    for entry in listing["entries"].as_array().unwrap() {
        let name = entry["name"].as_str().unwrap();
        let expected = PathBuf::from(&dir).join(name).display().to_string();
        assert_eq!(entry["path"], expected.as_str(), "{entry}");
        assert_eq!(entry["hidden"], name == ".hidden", "{entry}");
    }
}

/// With no path, browsing starts at the roots: every answering drive on
/// Windows, `/` elsewhere. There is nothing above a root.
#[tokio::test]
async fn with_no_path_the_listing_is_the_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(tmp.path()).await;

    let (status, listing) = call(&app, "GET", "/api/fs/dirs", None).await;
    assert_eq!(status, StatusCode::OK, "{listing}");
    assert_eq!(listing["path"], Value::Null);
    assert_eq!(listing["parent"], Value::Null);

    let temp = canonical(tmp.path());
    let root_of_temp = Path::new(&temp)
        .ancestors()
        .last()
        .unwrap()
        .display()
        .to_string();
    let roots: Vec<&str> = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["path"].as_str().unwrap())
        .collect();
    assert!(
        roots.contains(&root_of_temp.as_str()),
        "{root_of_temp} not in {roots:?}"
    );

    // A root's parent is null too, one level down.
    let (_, at_root) = call(&app, "GET", &listing_uri(Path::new(&root_of_temp)), None).await;
    assert_eq!(at_root["parent"], Value::Null, "{}", at_root["path"]);
}

#[tokio::test]
async fn a_listing_that_cannot_be_made_is_refused_with_a_stable_code() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(tmp.path()).await;
    let file = tmp.path().join("a-file.txt");
    std::fs::write(&file, "x").unwrap();

    for (path, status, code) in [
        (
            Path::new("relative"),
            StatusCode::BAD_REQUEST,
            "PATH_INVALID",
        ),
        (
            &tmp.path().join("missing"),
            StatusCode::NOT_FOUND,
            "PATH_NOT_FOUND",
        ),
        (&file, StatusCode::BAD_REQUEST, "PATH_NOT_A_DIRECTORY"),
    ] {
        let (got, body) = call(&app, "GET", &listing_uri(path), None).await;
        assert_eq!((got, body["code"].as_str()), (status, Some(code)), "{body}");
    }
}

#[tokio::test]
async fn creating_a_directory_returns_it_and_a_second_time_is_a_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(tmp.path()).await;
    let request = json!({ "parent": tmp.path(), "name": "new project" });

    let (status, created) = call(&app, "POST", "/api/fs/dirs", Some(request.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["name"], "new project");
    let expected = PathBuf::from(canonical(tmp.path())).join("new project");
    assert_eq!(created["path"], expected.display().to_string().as_str());
    assert!(expected.is_dir());

    let (status, again) = call(&app, "POST", "/api/fs/dirs", Some(request)).await;
    assert_eq!(
        (status, again["code"].as_str()),
        (StatusCode::CONFLICT, Some("PATH_ALREADY_EXISTS")),
        "{again}"
    );

    let (status, orphan) = call(
        &app,
        "POST",
        "/api/fs/dirs",
        Some(json!({ "parent": tmp.path().join("missing"), "name": "x" })),
    )
    .await;
    assert_eq!(
        (status, orphan["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("PATH_NOT_FOUND")),
        "{orphan}"
    );
}

/// A name must be exactly one new component that Windows would accept. Each
/// of these is refused with `PATH_INVALID`, and none of them creates anything.
#[tokio::test]
async fn a_name_that_is_not_one_valid_component_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(tmp.path()).await;
    let parent = tmp.path().join("parent");
    std::fs::create_dir(&parent).unwrap();

    for name in [
        "",
        ".",
        "..",
        "a/b",
        "a\\b",
        "a:b",
        "a*b",
        "a?b",
        "a\"b",
        "a<b",
        "a|b",
        "tab\tname",
        "trailing.",
        "trailing ",
        "CON",
        "nul.txt",
        "com1",
    ] {
        let (status, body) = call(
            &app,
            "POST",
            "/api/fs/dirs",
            Some(json!({ "parent": &parent, "name": name })),
        )
        .await;
        assert_eq!(
            (status, body["code"].as_str()),
            (StatusCode::BAD_REQUEST, Some("PATH_INVALID")),
            "{name:?}: {body}"
        );
    }
    assert_eq!(
        std::fs::read_dir(&parent).unwrap().count(),
        0,
        "a refused name created something"
    );
}

/// Axum refuses a malformed request before any handler runs, and answers it in
/// plain text of its own. A client matches on codes (spec §3.4), so those
/// refusals carry the same `ErrorBody` as every other error.
#[tokio::test]
async fn a_request_refused_before_its_handler_still_answers_an_error_body() {
    let tmp = tempfile::tempdir().unwrap();
    let (app, _stopping) = app(tmp.path()).await;
    let bad_json = Request::post("/api/fs/dirs")
        .header("content-type", "application/json")
        .body(Body::from("{\"parent\": 1}"))
        .unwrap();
    let not_json = Request::post("/api/fs/dirs")
        .header("content-type", "text/plain")
        .body(Body::from("{}"))
        .unwrap();
    let missing_query = Request::get("/api/subscribe").body(Body::empty()).unwrap();
    for request in [bad_json, not_json, missing_query] {
        let what = format!("{} {}", request.method(), request.uri());
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| panic!("{what}: not JSON: {}", String::from_utf8_lossy(&bytes)));
        assert!(status.is_client_error(), "{what}: {status}");
        assert_eq!(body["code"], "INVALID_COMMAND", "{what}: {body}");
        assert!(
            body["message"].as_str().is_some_and(|m| !m.is_empty()),
            "{what}"
        );
    }
}
