//! One job: which harness a thread, or a project's modes, may name.

use shadows_agent::policy;

use super::{ProjectId, Projects};
use crate::error::CoreError;

impl Projects {
    /// A harness Shadows knows, or `SettingNotOffered` naming it. One rule for
    /// both callers: a thread's harness (`Threads`) and a project's modes
    /// (`set_modes`).
    pub(crate) fn known_harness(harness: &str) -> Result<(), CoreError> {
        if policy::is_known(harness) {
            Ok(())
        } else {
            Err(CoreError::SettingNotOffered {
                what: "harness".into(),
                id: harness.into(),
                detail: None,
            })
        }
    }

    /// AYLUL's harness lives only in the AYLUL project, and that project
    /// allows nothing else. The AYLUL project is the one `ensure_assistant`
    /// marked `assistant`, never one that merely holds the configured slug. A
    /// project that is unknown or removed is not judged here: the write that
    /// follows answers it.
    pub(crate) async fn harness_fits(
        &self,
        project: &ProjectId,
        harness: &str,
    ) -> Result<(), CoreError> {
        let Ok(found) = self.get(project).await else {
            return Ok(());
        };
        if (harness == policy::AYLUL) == found.assistant {
            Ok(())
        } else {
            Err(CoreError::SettingNotOffered {
                what: "harness".into(),
                id: harness.into(),
                detail: Some(
                    "aylul runs only in the AYLUL project, which runs nothing else".into(),
                ),
            })
        }
    }
}
