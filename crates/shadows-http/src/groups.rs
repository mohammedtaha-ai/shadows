//! HTTP translation for agent groups.
use super::{
    AppState, Failure,
    failure::ErrorBody,
    transport::{EnvJson, Transport},
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use shadows_core::{Group, GroupId, Message};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct CreateGroup {
    command_id: String,
    name: String,
    members: Vec<String>,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct SendMessage {
    command_id: String,
    text: String,
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct MessageQuery {
    after: Option<i64>,
}

#[utoipa::path(get, path="/api/groups", tag="aylul", operation_id="list_groups",
    responses((status=200,body=[Group]),(status=500,body=ErrorBody)))]
pub(super) async fn list(State(s): State<AppState>) -> Result<Json<Vec<Group>>, Failure> {
    Ok(Json(s.core.messages().groups().await?))
}

#[utoipa::path(post, path="/api/groups", tag="aylul", operation_id="create_group",
    request_body=CreateGroup, responses((status=200,body=Group),(status=400,body=ErrorBody),
    (status=404,body=ErrorBody),(status=409,body=ErrorBody),(status=422,body=ErrorBody),
    (status=500,body=ErrorBody)))]
pub(super) async fn create(
    State(s): State<AppState>,
    EnvJson(b): EnvJson<CreateGroup>,
) -> Result<Json<Group>, Failure> {
    Transport::detached(async move {
        Ok(Json(
            s.core
                .messages()
                .create_group(b.command_id, &b.name, &b.members)
                .await?,
        ))
    })
    .await
}

#[utoipa::path(get, path="/api/groups/{id}", tag="aylul", operation_id="get_group",
    params(("id"=GroupId,Path)),
    responses((status=200,body=Group),(status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn get(
    State(s): State<AppState>,
    Path(id): Path<GroupId>,
) -> Result<Json<Group>, Failure> {
    Ok(Json(s.core.messages().group(&id).await?))
}

#[utoipa::path(get, path="/api/groups/{id}/messages", tag="aylul",
    operation_id="list_group_messages", params(("id"=GroupId,Path), MessageQuery),
    responses((status=200,body=[Message]),(status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn messages(
    State(s): State<AppState>,
    Path(id): Path<GroupId>,
    Query(q): Query<MessageQuery>,
) -> Result<Json<Vec<Message>>, Failure> {
    Ok(Json(s.core.messages().messages(&id, q.after).await?))
}

#[utoipa::path(post, path="/api/groups/{id}/messages", tag="aylul",
    operation_id="send_group_message", params(("id"=GroupId,Path)), request_body=SendMessage,
    responses((status=200,body=Message),(status=400,body=ErrorBody),(status=404,body=ErrorBody),
    (status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn send(
    State(s): State<AppState>,
    Path(id): Path<GroupId>,
    EnvJson(b): EnvJson<SendMessage>,
) -> Result<Json<Message>, Failure> {
    Transport::detached(async move {
        Ok(Json(
            s.core.messages().send(b.command_id, &id, &b.text).await?,
        ))
    })
    .await
}
