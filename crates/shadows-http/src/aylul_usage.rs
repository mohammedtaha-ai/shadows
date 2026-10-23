//! Usage service HTTP translation.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Query, State},
};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct UsagePeriod {
    from: String,
    to: String,
}

#[utoipa::path(get, path="/api/aylul/usage", tag="aylul", params(UsagePeriod),
    responses((status=200,body=shadows_core::UsageReport),(status=400,body=ErrorBody),
        (status=500,body=ErrorBody)))]
pub(super) async fn usage(
    State(s): State<AppState>,
    Query(q): Query<UsagePeriod>,
) -> Result<Json<shadows_core::UsageReport>, Failure> {
    Ok(Json(s.core.usage().usage(&q.from, &q.to).await?))
}
