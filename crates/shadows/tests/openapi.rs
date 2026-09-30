//! The OpenAPI document (spec §1: the protocol is described, not copied).
//!
//! **The decision this file owns.** Clients generate their types from
//! `api/openapi.json`, so that file is the protocol as a client sees it. It is
//! generated from the route table, never written, and the first test below
//! regenerates it and fails on any difference — the same contract
//! `tests/codemap` keeps for the code map. A route change therefore reaches the
//! checked-in document in the same commit, or the build is red:
//!
//! ```text
//! UPDATE_OPENAPI=1 cargo test --test openapi
//! ```
//!
//! The other tests check what drift cannot: that the document is served, that
//! it names every route a client needs, that each path it names is a real
//! route, and that ids and errors are described the way clients rely on.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use shadows::cli::router;
use shadows_core::testing::LiveHandles;
use shadows_core::testing::Runtime;
use shadows_core::testing::Storage;
use shadows_core::{AppCore, CoreParts};
use shadows_http::{AppState, openapi_document};
use tower::ServiceExt;

use shadows_core::testing::acp;

fn checked_in() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../api/openapi.json")
}

fn document() -> Value {
    serde_json::from_str(&openapi_document()).unwrap()
}

#[test]
fn the_checked_in_document_matches_the_routes() {
    let generated = openapi_document();
    let target = checked_in();
    if std::env::var_os("UPDATE_OPENAPI").is_some() {
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, &generated).expect("writing api/openapi.json");
        return;
    }
    let on_disk = std::fs::read_to_string(&target).unwrap_or_default();
    if on_disk == generated {
        return;
    }
    let mismatch = on_disk
        .lines()
        .zip(generated.lines())
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| {
            format!(
                "first difference at line {}:\n  checked in: {a}\n  generated:  {b}",
                i + 1
            )
        })
        .unwrap_or_else(|| {
            format!(
                "one is a prefix of the other ({} lines checked in, {} generated)",
                on_disk.lines().count(),
                generated.lines().count()
            )
        });
    panic!(
        "api/openapi.json no longer describes the routes.\n\n{mismatch}\n\n\
         Regenerate it and include it in the same commit as the route change:\n\
         \n    UPDATE_OPENAPI=1 cargo test --test openapi\n"
    );
}

/// Every route a client needs, by method and path.
#[test]
fn the_document_names_every_route() {
    let doc = document();
    let mut named: Vec<String> = doc["paths"]
        .as_object()
        .unwrap()
        .iter()
        .flat_map(|(path, item)| {
            item.as_object()
                .unwrap()
                .keys()
                .map(move |method| format!("{} {path}", method.to_uppercase()))
        })
        .collect();
    named.sort();
    assert_eq!(
        named,
        [
            "DELETE /api/mcp-grants/{id}",
            "DELETE /api/projects/{id}",
            "DELETE /api/projects/{id}/code/links/{linked}",
            "GET /api/code/settings",
            "GET /api/fs/dirs",
            "GET /api/harnesses",
            "GET /api/openapi.json",
            "GET /api/projects",
            "GET /api/projects/{id}/code/definitions",
            "GET /api/projects/{id}/code/links",
            "GET /api/projects/{id}/code/outline",
            "GET /api/projects/{id}/code/references",
            "GET /api/projects/{id}/code/status",
            "GET /api/projects/{id}/mcp-grants",
            "GET /api/projects/{id}/planner-instructions",
            "GET /api/projects/{id}/threads",
            "GET /api/projects/{id}/workflows",
            "GET /api/subscribe",
            "GET /api/threads/{id}/context",
            "GET /api/threads/{id}/entries",
            "GET /api/threads/{id}/operations",
            "GET /api/workflows/{id}",
            "PATCH /api/projects/{id}",
            "PATCH /api/threads/{id}",
            "POST /api/fs/dirs",
            "POST /api/operations/{id}/stop",
            "POST /api/projects",
            "POST /api/projects/{id}/mcp-grants",
            "POST /api/projects/{id}/threads",
            "POST /api/threads/{id}/fork",
            "POST /api/threads/{id}/session",
            "POST /api/threads/{id}/turns",
            "POST /api/workflows/{id}/approve",
            "PUT /api/code/settings",
            "PUT /api/projects/{id}/code/links/{linked}",
            "PUT /api/projects/{id}/planner-instructions",
            "PUT /api/threads/{id}/session/model",
        ]
    );
}

/// Ids are UUID strings under their own names; every error a route documents
/// carries `ErrorBody`; the stream is `text/event-stream` and says what its
/// frames are.
#[test]
fn ids_errors_and_the_stream_are_described_as_clients_rely_on() {
    let doc = document();
    let schemas = &doc["components"]["schemas"];
    for id in [
        "ProjectId",
        "ThreadId",
        "ThreadEntryId",
        "OperationId",
        "RuntimeInstanceId",
        "GrantId",
    ] {
        assert_eq!(schemas[id]["type"], "string", "{id}: {}", schemas[id]);
        assert_eq!(schemas[id]["format"], "uuid", "{id}: {}", schemas[id]);
    }
    assert_eq!(
        schemas["Project"]["properties"]["id"]["$ref"],
        "#/components/schemas/ProjectId"
    );

    for (path, item) in doc["paths"].as_object().unwrap() {
        for (method, op) in item.as_object().unwrap() {
            for (status, response) in op["responses"].as_object().unwrap() {
                if status.starts_with('4') || status.starts_with('5') {
                    assert_eq!(
                        response["content"]["application/json"]["schema"]["$ref"],
                        "#/components/schemas/ErrorBody",
                        "{method} {path} {status}"
                    );
                }
            }
        }
    }

    let stream = &doc["paths"]["/api/subscribe"]["get"]["responses"]["200"];
    assert!(
        stream["content"]["text/event-stream"].is_object(),
        "{stream}"
    );
    let description = stream["description"].as_str().unwrap();
    for kind in [
        "`durable`",
        "`caught-up`",
        "`delta`",
        "`turn-end`",
        "`lagged`",
        "`fatal`",
    ] {
        assert!(description.contains(kind), "{kind} is not described");
    }
}

/// The router serves the document, and every path it names is a real route:
/// none answers the router's own empty 404 or a 405.
#[tokio::test]
async fn the_document_is_served_and_every_path_it_names_is_routed() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = Runtime::start(storage.clone()).await.unwrap();
    let (bus, _) = tokio::sync::broadcast::channel(64);
    let (_stopping, shutdown) = tokio::sync::watch::channel(false);
    let app = router(AppState {
        core: AppCore::assemble(CoreParts {
            sessions: acp::fake_sessions(storage.clone()),
            storage,
            runtime: Arc::new(runtime),
            handles: Arc::new(LiveHandles::default()),
            bus,
            ui: tokio::sync::broadcast::channel(16).0,
            mcp_url: acp::MCP_URL.to_string(),
        }),
        allowed_origins: Vec::new(),
        shutdown,
    });

    let response = app
        .clone()
        .oneshot(
            Request::get("/api/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(std::str::from_utf8(&body).unwrap(), openapi_document());

    let doc = document();
    for (path, item) in doc["paths"].as_object().unwrap() {
        let uri = path.replace("{id}", "00000000-0000-4000-8000-000000000000");
        let uri = if uri == "/api/subscribe" {
            format!("{uri}?thread_id=00000000-0000-4000-8000-000000000000")
        } else {
            uri
        };
        for method in item.as_object().unwrap().keys() {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method.to_uppercase().as_str())
                        .uri(&uri)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = response.status();
            assert_ne!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
            // Only a 404's body is read: the stream's never ends.
            if status == StatusCode::NOT_FOUND {
                let body = axum::body::to_bytes(response.into_body(), 1 << 16)
                    .await
                    .unwrap();
                assert!(
                    !body.is_empty(),
                    "{method} {path} is documented and not routed"
                );
            }
        }
    }
}
