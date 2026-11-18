//! One job: the Codex protocol verdict for one daemon run.
use std::sync::atomic::{AtomicUsize, Ordering};

use shadows_agent::codex::CheckedCodex;
use shadows_agent::connection::ConnectionError;
use shadows_agent::policy;
use tokio::sync::Mutex;

use crate::HarnessError;

use super::sessions::Sessions;

/// The verdict is kept in memory only: a CLI is checked once per daemon run, and again only when
/// an opening finds it changed. The lock is held through the check, so concurrent openings wait
/// on one check.
#[derive(Default)]
pub(super) struct CodexCheck {
    verdict: Mutex<Option<Result<CheckedCodex, String>>>,
    checks: AtomicUsize,
}

impl CodexCheck {
    #[cfg(feature = "test-support")]
    pub(super) fn checks(&self) -> usize {
        self.checks.load(Ordering::Relaxed)
    }
}

impl Sessions {
    /// The installed Codex CLI, checked against the tested protocol. A refusal is
    /// `HarnessStartFailed` with what changed; Codex not configured is `HarnessUnavailable`.
    pub(crate) async fn codex_checked(&self) -> Result<CheckedCodex, HarnessError> {
        let program = self
            .config
            .codex
            .as_ref()
            .ok_or_else(|| HarnessError::HarnessUnavailable(policy::CODEX.into()))?;
        let mut verdict = self.codex_check.verdict.lock().await;
        if verdict.is_none() {
            self.codex_check.checks.fetch_add(1, Ordering::Relaxed);
            let scratch = self
                .config
                .catalog_root
                .join("codex-protocol")
                .join(uuid::Uuid::new_v4().to_string());
            let checked = program.check(&scratch).await.map_err(|error| match error {
                ConnectionError::Rpc(message) => message,
                other => other.to_string(),
            });
            if let Err(reason) = &checked {
                tracing::warn!(%reason, "harness.codex_check_refused");
            }
            *verdict = Some(checked);
        }
        verdict
            .clone()
            .unwrap_or_else(|| Err("Codex was not checked".into()))
            .map_err(HarnessError::HarnessStartFailed)
    }

    /// Clears the verdict when an opening found another version than the checked one, so the
    /// next opening checks the CLI now installed.
    pub(super) async fn forget_codex_check_if_changed(&self, error: &ConnectionError) {
        let changed = "Codex CLI changed to";
        if matches!(error, ConnectionError::Rpc(message) if message.starts_with(changed)) {
            *self.codex_check.verdict.lock().await = None;
        }
    }
}
