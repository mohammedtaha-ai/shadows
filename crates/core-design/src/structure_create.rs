//! Coordinating a person's structure write.
use super::store::structure::{FileHash, Read};
use super::{Design, structure, structure_write};
use crate::{DesignError, ProjectId, StorageError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct StructureWritten {
    pub map_version: i64,
    pub paths: Vec<String>,
    pub completed_at: String,
    pub resumed: bool,
}
impl Design {
    pub async fn create_structure(
        &self,
        command: String,
        project: &ProjectId,
        map_revision: i64,
    ) -> Result<StructureWritten, DesignError> {
        self.create_structure_inner(command, project, map_revision, None)
            .await
    }
    #[cfg(feature = "test-support")]
    pub async fn create_structure_stopping_after(
        &self,
        command: String,
        p: &ProjectId,
        map_revision: i64,
        n: usize,
    ) -> Result<StructureWritten, DesignError> {
        self.create_structure_inner(command, p, map_revision, Some(n))
            .await
    }
    async fn create_structure_inner(
        &self,
        command: String,
        p: &ProjectId,
        map_revision: i64,
        stop: Option<usize>,
    ) -> Result<StructureWritten, DesignError> {
        let _guard = self.structure_lock.lock().await;
        let ctx = core_db::CommandContext::user(
            command,
            "CreateStructure",
            serde_json::json!({"project":p,"map_revision":map_revision}),
        );
        let mut snapshot = match self.store.structure_snapshot(&ctx, p, map_revision).await? {
            Read::Replay(result) => return Ok(result),
            Read::Snapshot(snapshot) => snapshot,
        };
        let plan =
            structure::Structure::plan(&snapshot).map_err(crate::DesignError::StageBlocked)?;
        let hashes: Vec<_> = plan
            .files
            .iter()
            .map(|f| FileHash {
                path: f.path.clone(),
                sha256: structure::Structure::sha256_hex(&f.bytes),
            })
            .collect();
        if snapshot.resumed {
            let recorded: Vec<FileHash> =
                serde_json::from_str(&snapshot.intent.files_json).map_err(StorageError::from)?;
            let folders: Vec<String> =
                serde_json::from_str(&snapshot.intent.folders_json).map_err(StorageError::from)?;
            if recorded != hashes || folders != plan.folders {
                return Err(crate::DesignError::StageBlocked(
                    "recorded structure differs; cannot resume".into(),
                ));
            }
        }
        let root = structure_write::StructureWrite::root(&snapshot.directory).await?;
        let cleanup =
            structure_write::StructureWrite::check(&root, &plan, snapshot.resumed).await?;
        let map_version = snapshot.intent.map_number;
        if !snapshot.resumed {
            snapshot.intent.files_json =
                serde_json::to_string(&hashes).map_err(StorageError::from)?;
            snapshot.intent.folders_json =
                serde_json::to_string(&plan.folders).map_err(StorageError::from)?;
            self.store
                .start_structure(p, snapshot.revision, snapshot.intent)
                .await?;
        }
        structure_write::StructureWrite::write(&root, &plan, cleanup, stop).await?;
        self.store
            .finish_structure(
                &ctx,
                p,
                StructureWritten {
                    map_version,
                    paths: plan.files.into_iter().map(|f| f.path).collect(),
                    completed_at: core_db::Storage::now(),
                    resumed: snapshot.resumed,
                },
            )
            .await
    }
}
