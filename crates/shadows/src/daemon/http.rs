//! Authenticated native-only assistant operation translation.
use super::bridge::Bridge;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

#[derive(Clone)]
pub struct StateData {
    pub bridge: Arc<Mutex<Bridge>>,
    pub token: String,
    pub admission: Arc<Semaphore>,
}

pub fn router(state: StateData) -> Router {
    Router::new()
        .route("/assistant", post(call))
        .layer(DefaultBodyLimit::max(32_768))
        .with_state(state)
}

async fn call(
    State(state): State<StateData>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let expected = format!("Bearer {}", state.token);
    let authorized = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| equal(value.as_bytes(), expected.as_bytes()));
    if !authorized || headers.contains_key("origin") || headers.get("sec-fetch-site").is_some() {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"request_refused"})),
        );
    }
    let Ok(_permit) = state.admission.try_acquire() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":"unavailable"})),
        );
    };
    let mut bridge = state.bridge.lock().await;
    match bridge.call(request).await {
        Ok(answer) if answer["ok"] == true => (StatusCode::OK, Json(answer["result"].clone())),
        Ok(answer) => {
            let code = answer["error"].as_str().unwrap_or("unavailable");
            let (status, code) = match code {
                "request_invalid" => (StatusCode::BAD_REQUEST, "request_refused"),
                "command_conflict" | "generation_busy" => {
                    (StatusCode::CONFLICT, "request_conflict")
                }
                "not_found" => (StatusCode::NOT_FOUND, "not_found"),
                _ => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            };
            (status, Json(json!({"error":code})))
        }
        Err(code) => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error":code}))),
    }
}

fn equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}
