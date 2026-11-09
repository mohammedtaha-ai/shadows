//! One job: accepting the commit an executor made and its report of what it
//! did.
//!
//! The executor knows what it did, so it stages its own files and commits
//! them with git itself; Runs checks the commit it reports against the
//! folder's repository and the task's `writes`, and records it. A refusal
//! leaves the attempt `Running`, so the executor can fix its commit and
//! report again.
use crate::git::Git;
use crate::model::{Attempt, AttemptState};
use crate::scope::Scope;
use crate::{Runs, RunsError};
use core_grants::{Grant, GrantKind};

impl Runs {
    /// Accepts the commit `commit` as the work of the attempt `grant` belongs
    /// to, with `report`, the executor's account of it. The commit must be
    /// on the checked-out branch, new since Start, change only paths inside
    /// `writes`, and bring no binary file. One accepted report per attempt.
    pub async fn report(
        &self,
        grant: &Grant,
        commit: &str,
        report: String,
    ) -> Result<Attempt, RunsError> {
        let (Some(thread), Some(scope), GrantKind::Executor) =
            (&grant.thread_id, &grant.scope, grant.kind)
        else {
            return Err(RunsError::NotFound);
        };
        let attempt = self.inner.store.executor_attempt(thread, scope).await?;
        Self::still_running(&attempt)?;
        let git = Git::at(&self.folder(&attempt.project_id).await?);
        let sha = self.verify(&git, &attempt, commit.trim()).await?;
        if !self
            .inner
            .store
            .record_report(&attempt.id, &sha, report.trim())
            .await?
        {
            // A Stop or another report moved it first.
            return Self::still_running(&self.inner.store.get(&attempt.id).await?)
                .and(Err(RunsError::NotFound));
        }
        tracing::info!(attempt = %attempt.id, commit = %sha, "runs.reported");
        self.inner.store.get(&attempt.id).await
    }

    /// Only a `Running` attempt takes a report; a `Committed` one already did.
    fn still_running(attempt: &Attempt) -> Result<(), RunsError> {
        match attempt.state {
            AttemptState::Running => Ok(()),
            AttemptState::Committed => Err(RunsError::AlreadyCommitted),
            _ => Err(RunsError::NotFound),
        }
    }

    /// The full SHA of `commit` once it passes every check, in the order an
    /// executor fixes them: that it exists, is on the branch, is new, and
    /// what it changes.
    async fn verify(
        &self,
        git: &Git,
        attempt: &Attempt,
        commit: &str,
    ) -> Result<String, RunsError> {
        let unknown = || RunsError::UnknownCommit {
            commit: commit.to_string(),
        };
        // Only a SHA is taken: anything else could be read by git as an option or a ref.
        if commit.len() < 4 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(unknown());
        }
        let sha = git.commit(commit).await.ok_or_else(unknown)?;
        if !git.reaches("HEAD", &sha).await {
            return Err(RunsError::NotOnBranch { commit: sha });
        }
        if let Some(start) = self.inner.store.start_head(&attempt.id).await?
            && git.reaches(&start, &sha).await
        {
            return Err(RunsError::NotNew { commit: sha });
        }
        let prefix = git.prefix().await.ok_or(RunsError::NotGit)?;
        let writes = self.inner.store.writes(&attempt.id).await?;
        let matcher = Scope::new(&writes).map_err(RunsError::BadScope)?;
        let changes = git.changes(&sha).await.map_err(RunsError::Unavailable)?;
        // A path outside the project folder is outside every pattern.
        let outside: Vec<String> = changes
            .iter()
            .filter(|c| {
                !c.path
                    .strip_prefix(prefix.as_str())
                    .is_some_and(|inside| matcher.admits(inside))
            })
            .map(|c| c.path.clone())
            .collect();
        if !outside.is_empty() {
            return Err(RunsError::OutsideWrites { paths: outside });
        }
        let binary: Vec<String> = changes
            .into_iter()
            .filter(|c| c.binary)
            .map(|c| c.path)
            .collect();
        if !binary.is_empty() {
            return Err(RunsError::BinaryFiles { paths: binary });
        }
        Ok(sha)
    }
}
