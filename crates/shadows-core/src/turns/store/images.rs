//! Neutral image reads composed through Threads in one snapshot.
use crate::{
    CoreError,
    threads::{AttachmentId, ThreadId, Threads},
};
use core_db::StorageError;
use shadows_agent::acp::ImageBlock;

impl super::TurnsStore {
    pub(in crate::turns) async fn images(
        &self,
        thread: &ThreadId,
        ids: &[AttachmentId],
    ) -> Result<Vec<ImageBlock>, CoreError> {
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
