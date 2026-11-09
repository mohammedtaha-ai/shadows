//! One job: settling the attempts a stopped daemon left running.
use crate::git::Git;
use crate::{RecoverReport, Runs, RunsError};

impl Runs {
    /// Every `Running` attempt found at startup is settled, none restarted:
    /// one whose commit git holds (its `Shadows-Attempt` trailer) becomes
    /// `Committed`, since git is written before SQLite; every other becomes
    /// `Interrupted`. Grants are revoked and every scope still held is freed.
    pub async fn recover(&self) -> Result<RecoverReport, RunsError> {
        let mut found = Vec::new();
        for project in self.inner.store.running_projects().await? {
            let Ok(folder) = self.folder(&project).await else {
                continue;
            };
            match Git::at(&folder).attempt_commits().await {
                Ok(commits) => found.extend(commits),
                Err(error) => tracing::warn!(%project, %error, "runs.recover_git_unread"),
            }
        }
        let (committed, interrupted) = self.inner.store.settle_running(found).await?;
        Ok(RecoverReport {
            committed,
            interrupted,
        })
    }
}
