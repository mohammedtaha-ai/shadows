//! HTTP request execution and body extraction.
use super::Failure;
use axum::{
    Json,
    extract::{FromRequest, Request},
};
use tracing::Instrument;

// Never expose serde's rejection: it may include the submitted value.
pub(super) struct EnvJson<T>(pub(super) T);

impl<S: Send + Sync, T: serde::de::DeserializeOwned + Send> FromRequest<S> for EnvJson<T> {
    type Rejection = Failure;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(body)| Self(body))
            .map_err(|_| Failure::invalid_env_json())
    }
}

pub(super) struct Transport;

impl Transport {
    // A disconnected request must not strand a committed command or a session lease.
    pub(super) async fn detached<T: Send + 'static>(
        work: impl Future<Output = Result<T, Failure>> + Send + 'static,
    ) -> Result<T, Failure> {
        match tokio::spawn(work.in_current_span()).await {
            Ok(answer) => answer,
            Err(error) => std::panic::resume_unwind(error.into_panic()),
        }
    }
}
