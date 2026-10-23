//! Coordinating a person's structure write.
use super::store::structure::{FileHash, Read};
use super::{Design, structure, structure_write};
use crate::{CoreError, ProjectId, StorageError};

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
    ) -> Result<StructureWritten, CoreError> {
        self.create_structure_inner(command, project, None).await
    }
    #[cfg(feature = "test-support")]
    pub async fn create_structure_stopping_after(
        &self,
        command: String,
        p: &ProjectId,
        n: usize,
    ) -> Result<StructureWritten, CoreError> {
        self.create_structure_inner(command, p, Some(n)).await
    }
    async fn create_structure_inner(
        &self,
        command: String,
        p: &ProjectId,
        stop: Option<usize>,
    ) -> Result<StructureWritten, CoreError> {
        let _guard = self.structure_lock.lock().await;
        let ctx = crate::db::CommandContext::user(
            command,
            "CreateStructure",
            serde_json::json!({"project":p}),
        );
        let mut snapshot = match self.store.structure_snapshot(&ctx, p).await? {
            Read::Replay(result) => return Ok(result),
            Read::Snapshot(snapshot) => snapshot,
        };
        let plan = structure::Structure::render(&structure::RenderInput {
            project_name: &snapshot.name,
            map_number: snapshot.intent.map_number,
            parts: &snapshot.parts,
            standards_base: snapshot.intent.standards_version,
            standards_additions: snapshot.intent.standards_additions_version_id.as_deref(),
        })
        .map_err(crate::design::DesignError::StageBlocked)?;
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
                return Err(crate::design::DesignError::StageBlocked(
                    "recorded structure differs; cannot resume".into(),
                )
                .into());
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
                    completed_at: crate::db::now(),
                    resumed: snapshot.resumed,
                },
            )
            .await
    }
}
