//! One job: which harness a thread, or a project's modes, may name.

use shadows_agent::policy;

use crate::error::CoreError;

/// AYLUL's harness lives only in the AYLUL project, and that project allows
/// nothing else. The AYLUL project is the one `Projects::ensure_assistant`
/// marked `assistant`, never one that merely holds the configured slug.
pub(crate) fn harness_fits_project(harness: &str, assistant: bool) -> Result<(), CoreError> {
    if (harness == policy::AYLUL) == assistant {
        Ok(())
    } else {
        Err(CoreError::SettingNotOffered {
            what: "harness".into(),
            id: harness.into(),
            detail: Some("aylul runs only in the AYLUL project, which runs nothing else".into()),
        })
    }
}

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
