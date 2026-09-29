//! One job: which harness a thread, or a project's modes, may name.

use shadows_agent::policy;

use crate::error::CoreError;

/// A harness Shadows knows, or `SettingNotOffered` naming it. One rule for
/// both callers: a thread's harness (`Threads`) and a project's modes
/// (`Projects`).
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
