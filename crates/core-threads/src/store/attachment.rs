//! Immutable image persistence and thread ownership references.
use super::ThreadsStore;
use crate::{Attachment, AttachmentId, AttachmentRef, ThreadId, Threads};
use core_db::{CommandContext, Storage, StorageError};
use sqlx::SqliteConnection;

type AttachmentRow = (String, String, i64, Option<String>, Vec<u8>);

impl ThreadsStore {
    pub(in super::super) async fn read_attachment(
        &self,
        thread: &ThreadId,
        id: &AttachmentId,
    ) -> Result<(Attachment, Vec<u8>), crate::ThreadsError> {
        let mut conn = self
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        Ok(Threads::attachment_in(&mut conn, thread, id).await?)
    }

    pub(in super::super) async fn upload_attachment(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        media_type: &str,
        name: Option<String>,
        digest: Vec<u8>,
        bytes: Vec<u8>,
    ) -> Result<Attachment, crate::ThreadsError> {
        let (ctx, thread, media_type) = (ctx.clone(), thread.clone(), media_type.to_owned());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(answer) =
                        CommandContext::classify(conn, &ctx, "Thread", thread.as_str()).await?
                    {
                        return Ok(serde_json::from_str(&answer)?);
                    }
                    Threads::require_live_in(conn, &thread).await?;
                    let attachment = Attachment {
                        id: AttachmentId::generate(),
                        media_type,
                        byte_len: bytes.len() as u64,
                        name,
                    };
                    let ts = Storage::now();
                    sqlx::query(
                        "INSERT INTO attachment
                (id,media_type,byte_len,sha256,name,bytes,created_at) VALUES (?,?,?,?,?,?,?)",
                    )
                    .bind(attachment.id.as_str())
                    .bind(&attachment.media_type)
                    .bind(bytes.len() as i64)
                    .bind(digest)
                    .bind(&attachment.name)
                    .bind(bytes)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    sqlx::query(
                        "INSERT INTO thread_attachment (thread_id,attachment_id) VALUES (?,?)",
                    )
                    .bind(thread.as_str())
                    .bind(attachment.id.as_str())
                    .execute(&mut *conn)
                    .await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Thread",
                        thread.as_str(),
                        "Attachment",
                        &serde_json::to_string(&attachment)?,
                        &ts,
                    )
                    .await?;
                    Ok(attachment)
                })
            })
            .await
    }
}

impl Threads {
    pub async fn attachment_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        id: &AttachmentId,
    ) -> Result<(Attachment, Vec<u8>), StorageError> {
        Self::require_live_in(conn, thread).await?;
        let row: Option<AttachmentRow> = sqlx::query_as(
            "SELECT a.id,a.media_type,a.byte_len,a.name,a.bytes FROM attachment a
             JOIN thread_attachment t ON t.attachment_id=a.id
             WHERE t.thread_id=? AND a.id=?",
        )
        .bind(thread.as_str())
        .bind(id.as_str())
        .fetch_optional(conn)
        .await?;
        let (stored_id, media_type, byte_len, name, bytes) =
            row.ok_or(StorageError::NotFound("attachment"))?;
        Ok((
            Attachment {
                id: AttachmentId::from_stored(stored_id),
                media_type,
                byte_len: byte_len as u64,
                name,
            },
            bytes,
        ))
    }

    pub async fn attachment_refs_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        ids: &[AttachmentId],
    ) -> Result<Vec<AttachmentRef>, StorageError> {
        let mut refs = Vec::with_capacity(ids.len());
        for id in ids {
            let (a, _) = Self::attachment_in(conn, thread, id).await?;
            refs.push(AttachmentRef {
                id: a.id,
                media_type: a.media_type,
                name: a.name,
            });
        }
        Ok(refs)
    }
}
