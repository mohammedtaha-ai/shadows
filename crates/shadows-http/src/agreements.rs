//! HTTP translation for shared API agreements.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use shadows_core::{AgreementContent, AgreementId, AgreementReview, AgreementVersion, ProjectId};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct VersionQuery {
    version: Option<i64>,
}

#[utoipa::path(get, path="/api/projects/{id}/agreements", tag="agreements",
    operation_id="agreement_list",
    params(("id"=ProjectId,Path)), responses((status=200,body=Vec<AgreementVersion>),
    (status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn list(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<Vec<AgreementVersion>>, Failure> {
    Ok(Json(s.core.design().agreements(&project).await?))
}
#[utoipa::path(get, path="/api/projects/{id}/agreements/{agreement}", tag="agreements",
    operation_id="agreement_get",
    params(("id"=ProjectId,Path),("agreement"=AgreementId,Path),VersionQuery),
    responses((status=200,body=AgreementVersion),(status=404,body=ErrorBody),
    (status=500,body=ErrorBody)))]
pub(super) async fn get(
    State(s): State<AppState>,
    Path((project, id)): Path<(ProjectId, AgreementId)>,
    Query(query): Query<VersionQuery>,
) -> Result<Json<AgreementVersion>, Failure> {
    Ok(Json(
        s.core
            .design()
            .agreement(&project, &id, query.version)
            .await?,
    ))
}
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct Start {
    command_id: String,
    agreement_id: Option<AgreementId>,
    content: Option<AgreementContent>,
    reason: Option<String>,
}
#[utoipa::path(post, path="/api/projects/{id}/agreements", tag="agreements",
    operation_id="agreement_start",
    params(("id"=ProjectId,Path)), request_body=Start,
    responses((status=200,body=AgreementVersion),(status=404,body=ErrorBody),
    (status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn start(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Json(body): Json<Start>,
) -> Result<Json<AgreementVersion>, Failure> {
    Ok(Json(
        s.core
            .design()
            .start_agreement(
                body.command_id,
                &project,
                body.agreement_id.as_ref(),
                body.content,
                body.reason,
            )
            .await?,
    ))
}
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct Edit {
    command_id: String,
    expected_revision: i64,
    content: AgreementContent,
}
#[utoipa::path(put, path="/api/projects/{id}/agreements/{agreement}", tag="agreements",
    operation_id="agreement_edit",
    params(("id"=ProjectId,Path),("agreement"=AgreementId,Path)), request_body=Edit,
    responses((status=200,body=AgreementVersion),(status=404,body=ErrorBody),
    (status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn edit(
    State(s): State<AppState>,
    Path((project, id)): Path<(ProjectId, AgreementId)>,
    Json(body): Json<Edit>,
) -> Result<Json<AgreementVersion>, Failure> {
    Ok(Json(
        s.core
            .design()
            .edit_agreement(
                body.command_id,
                &project,
                &id,
                body.expected_revision,
                body.content,
            )
            .await?,
    ))
}

#[utoipa::path(get,path="/api/projects/{id}/agreements/{agreement}/review",tag="agreements",
    operation_id="agreement_review",
    params(("id"=ProjectId,Path),("agreement"=AgreementId,Path)),
    responses((status=200,body=AgreementReview),(status=404,body=ErrorBody),
    (status=500,body=ErrorBody)))]
pub(super) async fn review(
    State(s): State<AppState>,
    Path((project, id)): Path<(ProjectId, AgreementId)>,
) -> Result<Json<AgreementReview>, Failure> {
    Ok(Json(s.core.design().review_agreement(&project, &id).await?))
}
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct Agree {
    command_id: String,
    expected_revision: i64,
    review_id: String,
}
#[utoipa::path(post,path="/api/projects/{id}/agreements/{agreement}/agree",tag="agreements",
    operation_id="agreement_agree",
    params(("id"=ProjectId,Path),("agreement"=AgreementId,Path)),request_body=Agree,
    responses((status=200,body=AgreementVersion),(status=404,body=ErrorBody),
    (status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn agree(
    State(s): State<AppState>,
    Path((project, id)): Path<(ProjectId, AgreementId)>,
    Json(body): Json<Agree>,
) -> Result<Json<AgreementVersion>, Failure> {
    Ok(Json(
        s.core
            .design()
            .agree_agreement(
                body.command_id,
                &project,
                &id,
                body.expected_revision,
                body.review_id,
            )
            .await?,
    ))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct BindingEdit {
    command_id: String,
    expected_revision: i64,
    ops: Vec<shadows_core::PlanOp>,
}
#[utoipa::path(post,path="/api/workflows/{id}/bindings",tag="agreements",
    operation_id="plan_binding_edit",
    params(("id"=shadows_core::WorkflowId,Path)),request_body=BindingEdit,
    responses((status=200,body=shadows_core::EditOutcome),(status=404,body=ErrorBody),
    (status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn bindings(
    State(s): State<AppState>,
    Path(id): Path<shadows_core::WorkflowId>,
    Json(body): Json<BindingEdit>,
) -> Result<Json<shadows_core::EditOutcome>, Failure> {
    Ok(Json(
        s.core
            .plans()
            .edit_bindings(body.command_id, &id, body.expected_revision, body.ops)
            .await?,
    ))
}
