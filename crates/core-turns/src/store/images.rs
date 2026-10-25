//! Neutral image reads composed through Threads in one snapshot.
use crate::TurnsError;
use core_db::StorageError;
use core_threads::{AttachmentId, ThreadId, Threads};
use shadows_agent::acp::ImageBlock;

impl super::TurnsStore {
    pub(crate) async fn images(
        &self,
        thread: &ThreadId,
        ids: &[AttachmentId],
    ) -> Result<Vec<ImageBlock>, TurnsError> {
        let mut conn = self
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        let mut images = Vec::with_capacity(ids.len());
        for id in ids {
            let (record, bytes) = Threads::attachment_in(&mut conn, thread, id).await?;
            images.push(ImageBlock {
                media_type: record.media_type,
                bytes,
            });
        }
        Ok(images)
    }
}
