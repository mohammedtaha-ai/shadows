use crate::operations::Operations;
use crate::runtime::{RuntimeInstanceId, StopKind};
use core_db::StorageError;
impl Operations {
    pub async fn recover_for_test(
        &self,
        current: &RuntimeInstanceId,
    ) -> Result<crate::operations::ReconcileReport, StorageError> {
        self.recover(current).await
    }

    pub async fn stop_runtime_instance_for_test(
        &self,
        id: &RuntimeInstanceId,
        kind: StopKind,
    ) -> Result<(), StorageError> {
        self.stop_runtime(id, kind).await
    }
}
