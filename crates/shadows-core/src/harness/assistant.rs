//! One job: what an AYLUL session opens with. Claude Code on its own profile,
//! in the profile's home, with none of the Planner's prompt, tools, MCP grant
//! or stage text, loading only the profile's user settings.

use std::path::PathBuf;

use shadows_agent::acp::SessionSetup;
use shadows_agent::policy;
use shadows_agent::profile::ClaudeProfile;
use shadows_process::ProcessSpec;

use super::sessions::{OpenError, Sessions, workspace};
use crate::threads::{ThreadId, TurnContext};

/// The daemon's `aylul` section, as the core needs it.
#[derive(Debug, Clone)]
pub struct AylulConfig {
    pub profile: ClaudeProfile,
    /// The slug of the one project AYLUL's conversations live in.
    pub project_slug: String,
}

/// What an AYLUL session opens with: nothing of the Planner's.
pub(super) fn aylul_setup() -> SessionSetup {
    SessionSetup {
        mcp: None,
        append: None,
        allowed_tools: vec![],
        tools: None,
        setting_sources: Some(vec!["user".into()]),
    }
}

/// Where an AYLUL adapter runs and how it starts.
pub(super) struct Opening {
    pub(super) cwd: PathBuf,
    pub(super) setup: SessionSetup,
    pub(super) spec: ProcessSpec,
}

impl Sessions {
    /// The one answer to "can a turn run on `kind` here": AYLUL only with a profile.
    pub(crate) fn harness_available(&self, kind: &str) -> bool {
        policy::is_available_with(kind, self.config.aylul.is_some())
    }

    pub(super) fn aylul_slug(&self) -> Option<&str> {
        self.config.aylul.as_ref().map(|a| a.project_slug.as_str())
    }

    /// Where the thread's adapter runs and what it opens with. A Planner
    /// thread: its project folder and the Planner's setup (which issues its
    /// grant). An AYLUL thread: the profile home and spec carrying the
    /// profile, refused before any spawn when the key variable is unset.
    pub(super) async fn opening(
        &self,
        thread: &ThreadId,
        context: &TurnContext,
    ) -> Result<Opening, OpenError> {
        if context.harness != policy::AYLUL {
            let cwd = workspace(context).map_err(OpenError::Workspace)?;
            let setup = (self.setups.for_opening(thread, &context.project_id).await)
                .map_err(OpenError::Start)?;
            let spec = self.adapter.process_spec(&cwd);
            return Ok(Opening { cwd, setup, spec });
        }
        let config = (self.config.aylul.as_ref())
            .ok_or_else(|| OpenError::Start("aylul is not configured".into()))?;
        let cwd = config.profile.home.clone();
        let spec = (self.adapter.process_spec_with(&cwd, Some(&config.profile)))
            .map_err(|e| OpenError::Start(e.to_string()))?;
        Ok(Opening {
            cwd,
            setup: aylul_setup(),
            spec,
        })
    }
}
