//! Attachment HTTP translation; Threads validates and stores images.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Multipart, Path, State},
    response::{IntoResponse, Response},
};
use shadows_core::{Attachment, AttachmentId, CoreError, ErrorCode, ThreadId};

#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub(super) struct Upload {
    command_id: String,
    #[schema(value_type = String, format = Binary)]
    file: Vec<u8>,
}

#[utoipa::path(post, path = "/api/threads/{id}/attachments", tag = "threads",
    params(("id" = ThreadId, Path)),
    request_body(content = Upload, content_type = "multipart/form-data"),
    responses((status = 200, body = Attachment),
        (status = 400, body = ErrorBody), (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody), (status = 422, body = ErrorBody)))]
pub(super) async fn upload(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    mut multipart: Multipart,
) -> Result<Json<Attachment>, Failure> {
    let invalid = || CoreError::Refused {
        code: ErrorCode::InvalidCommand,
        message: "Expected command_id and one image file of at most 5 MiB".into(),
    };
    let (mut command, mut file) = (None, None);
    while let Some(field) = multipart.next_field().await.map_err(|_| invalid())? {
        match field.name() {
            Some("command_id") if command.is_none() => {
                let text = field.text().await.map_err(|_| invalid())?;
                command = Some(text);
            }
            Some("file") if file.is_none() => {
                let name = field.file_name().map(str::to_owned);
                let media = field.content_type().unwrap_or("").to_owned();
                let bytes = field.bytes().await.map_err(|_| invalid())?;
                file = Some((media, name, bytes.to_vec()));
            }
            _ => return Err(invalid().into()),
        }
    }
    let (media, name, bytes) = file.ok_or_else(invalid)?;
    let command = command
        .filter(|c| !c.trim().is_empty())
        .ok_or_else(invalid)?;
    Ok(Json(
        s.core
            .threads()
            .upload_attachment(command, &thread, &media, name, bytes)
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/threads/{id}/attachments/{attachment_id}", tag = "threads",
    params(("id" = ThreadId, Path), ("attachment_id" = AttachmentId, Path)),
    responses((status = 200, content_type = "application/octet-stream", body = Vec<u8>),
        (status = 404, body = ErrorBody)))]
pub(super) async fn read(
    State(s): State<AppState>,
    Path((thread, id)): Path<(ThreadId, AttachmentId)>,
) -> Result<Response, Failure> {
    let (record, bytes) = s.core.threads().attachment(&thread, &id).await?;
    Ok((
        [
            ("content-type", record.media_type.as_str()),
            ("x-content-type-options", "nosniff"),
            ("cache-control", "no-store"),
        ],
        bytes,
    )
        .into_response())
}
