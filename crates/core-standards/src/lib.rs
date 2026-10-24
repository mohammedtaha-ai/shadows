//! Project standards: the bundled base with immutable project additions.

mod base;
mod model;
mod store;
mod validation;

use core_projects::ProjectId;
mod error;
use core_db::{CommandContext, Storage};
pub use error::StandardsError;
pub use model::{
    AdditionalPart, BaseStandards, ContractTemplate, EffectiveStandards, MandatoryPart,
    StandardRule, StandardsAdditions, StandardsAdditionsVersion,
};
use std::sync::Arc;
use store::StandardsStore;

#[derive(Clone)]
pub struct Standards {
    store: StandardsStore,
}

impl Standards {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            store: StandardsStore::new(storage),
        }
    }

    /// The latest project additions, refusing an unknown or removed project.
    pub async fn current(
        &self,
        project: &ProjectId,
    ) -> Result<Option<StandardsAdditionsVersion>, StandardsError> {
        Ok(self.store.current(project).await?)
    }

    pub async fn standards(
        &self,
        project: &ProjectId,
    ) -> Result<EffectiveStandards, StandardsError> {
        Ok(EffectiveStandards {
            base: Self::base().clone(),
            additions: self.store.current(project).await?,
        })
    }

    pub async fn save_standards_additions(
        &self,
        command_id: String,
        project: &ProjectId,
        content: StandardsAdditions,
    ) -> Result<StandardsAdditionsVersion, StandardsError> {
        let params = serde_json::json!({"project": project, "content": content});
        let ctx = CommandContext::user(command_id, "StandardsAdditionsSave", params);
        self.store
            .save_standards_additions(&ctx, project, &content)
            .await
            .map_err(|error| match error {
                core_db::StorageError::Constraint(message) => {
                    StandardsError::InvalidAdditions(message)
                }
                other => other.into(),
            })
    }
}
