//! One job: which harness a thread, or a project's modes, may name.

use shadows_agent::policy;

use crate::error::CoreError;

/// AYLUL's harness lives only in the AYLUL project, and that project allows
/// nothing else. `aylul_slug` is None when the daemon has no profile.
pub(crate) fn harness_fits_project(
    harness: &str,
    project_slug: &str,
    aylul_slug: Option<&str>,
) -> Result<(), CoreError> {
    let in_aylul_project = aylul_slug == Some(project_slug);
    if (harness == policy::AYLUL) == in_aylul_project {
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
