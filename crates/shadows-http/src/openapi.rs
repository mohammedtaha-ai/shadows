//! One job: the OpenAPI document that describes this protocol.
//!
//! Spec §1: the protocol is described, not copied. The paths and schemas come
//! from the route table (`utoipa-axum`) and the annotated handlers and types;
//! this file adds only what belongs to the document as a whole, and renders it
//! in the one form that is served, checked in at `api/openapi.json`, and kept
//! current by `tests/openapi.rs`.

use axum::http::header;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "shadows",
        description = "The local daemon's HTTP API. Every client is on another \
                       origin; only origins passed with `--allow-origin` may call \
                       it. There is no authentication: the daemon binds to \
                       loopback (spec §1, OPEN block on remote access). Errors \
                       answered by the API carry `ErrorBody`, whose `code` is \
                       stable; a request axum rejects before a handler runs \
                       (malformed JSON, a missing parameter) is answered in \
                       plain text."
    ),
    tags(
        (name = "design", description = "Project design workspace"),
        (name = "projects", description = "Projects and the directory each owns"),
        (name = "threads", description = "Planning threads and their entries"),
        (name = "turns", description = "Starting and stopping a Planner turn"),
        (name = "workflows", description = "Plan versions and a person's approval of one"),
        (name = "grants", description = "External agents' access to the MCP server"),
        (name = "code", description = "The code index: where a name is defined or used, what a path holds, and the projects a project reads"),
        (name = "harnesses", description = "The CLIs a conversation runs on, and the choices a session offers"),
        (name = "stream", description = "The replay-then-live event stream"),
        (name = "filesystem", description = "Choosing a project directory on this machine"),
        (name = "meta", description = "This document"),
    )
)]
struct ApiDoc;

/// The document-level part, onto which the route table adds its paths.
pub(super) fn base() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

/// The document, rendered: pretty-printed with every object's keys in sorted
/// order, and a final newline. Sorted because the order `utoipa` builds maps
/// in is not a contract, and a checked-in file that reorders itself would
/// fail its drift test on a dependency bump that changed nothing.
pub fn document() -> String {
    let value = serde_json::to_value(super::routes().into_openapi())
        .expect("an OpenAPI document is plain data and always serializes");
    let mut text =
        serde_json::to_string_pretty(&sorted(value)).expect("a JSON value always serializes");
    text.push('\n');
    text
}

/// `serde_json`'s map is ordered by key unless its `preserve_order` feature is
/// on; nothing here turns it on today, and this keeps the output sorted even
/// if a dependency one day does.
fn sorted(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            serde_json::Value::Object(entries.into_iter().map(|(k, v)| (k, sorted(v))).collect())
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sorted).collect())
        }
        other => other,
    }
}

/// This document.
#[utoipa::path(
    get,
    path = "/api/openapi.json",
    tag = "meta",
    responses((status = 200, description = "This OpenAPI document", content_type = "application/json"))
)]
pub(super) async fn serve() -> ([(header::HeaderName, &'static str); 1], String) {
    ([(header::CONTENT_TYPE, "application/json")], document())
}
