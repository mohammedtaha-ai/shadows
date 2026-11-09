//! One job: committing an attempt's work — only the changed paths inside its
//! task's `writes`, on the branch the folder has checked out.
//!
//! Git is written first, then SQLite: the commit carries the attempt in its
//! `Shadows-Attempt` trailer, so a crash between the two is repaired by
//! `recover`. One commit at a time per project folder.
use std::path::PathBuf;
use std::sync::Arc;

use crate::git::Git;
use crate::model::{Attempt, AttemptState, CommitResult};
use crate::scope::Scope;
use crate::{Runs, RunsError};
use core_grants::{Grant, GrantKind};
use core_projects::ProjectId;

/// A temporary file removed when it goes out of use.
struct TempFile(PathBuf);

impl TempFile {
    fn new(kind: &str) -> Self {
        Self(std::env::temp_dir().join(format!("shadows-{kind}-{}", uuid::Uuid::new_v4())))
    }

    fn path(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

impl Runs {
    /// Commits the work of the attempt `grant` belongs to. A changed path
    /// outside `writes` stays in the folder, uncommitted, and is answered in
    /// `left`. A path in `writes` reached through a link fails the attempt:
    /// git would read another folder's files through it.
    pub async fn commit(&self, grant: &Grant, message: String) -> Result<CommitResult, RunsError> {
        let (Some(thread), Some(scope), GrantKind::Executor) =
            (&grant.thread_id, &grant.scope, grant.kind)
        else {
            return Err(RunsError::NotFound);
        };
        let lock = self.commit_lock(&grant.project_id);
        let _one_at_a_time = lock.lock().await;
        let attempt = self.inner.store.executor_attempt(thread, scope).await?;
        match attempt.state {
            AttemptState::Running => {}
            AttemptState::Committed => return Err(RunsError::AlreadyCommitted),
            _ => return Err(RunsError::NotFound),
        }
        let git = Git::at(&self.folder(&attempt.project_id).await?);
        let prefix = git.prefix().await.ok_or(RunsError::NotGit)?;
        let writes = self.inner.store.writes(&attempt.id).await?;
        let matcher = Scope::new(&writes).map_err(RunsError::BadScope)?;
        let (files, left): (Vec<String>, Vec<String>) = git
            .changed(&prefix)
            .await
            .map_err(RunsError::Unavailable)?
            .into_iter()
            .partition(|path| matcher.admits(path));
        let linked: Vec<String> = files
            .iter()
            .filter(|path| git.through_link(path))
            .cloned()
            .collect();
        if !linked.is_empty() {
            let reason = format!("a link inside writes: {}", linked.join(", "));
            self.abandon(attempt, reason).await?;
            return Err(RunsError::LinkInScope { paths: linked });
        }
        if files.is_empty() {
            return Err(RunsError::NothingToCommit { left });
        }
        let title = self.task_title(&attempt).await?;
        let text = format!(
            "T{}: {title}\n\n{}\n\nShadows-Attempt: {}\n",
            attempt.task,
            message.trim(),
            attempt.id
        );
        let sha = self.write_commit(&git, &attempt, &files, &text).await?;
        #[cfg(feature = "test-support")]
        if self.hooks().fail_record {
            return Err(RunsError::Unavailable("the record failed (test)".into()));
        }
        self.inner.store.record_commit(&attempt.id, &sha).await?;
        tracing::info!(attempt = %attempt.id, commit = %sha, files = files.len(), "runs.committed");
        Ok(CommitResult {
            attempt: self.inner.store.get(&attempt.id).await?,
            files,
            left,
        })
    }

    /// Builds the commit in a temporary index from HEAD and the files, moves
    /// HEAD only if it has not moved, then brings those paths of the real
    /// index to the commit, leaving the person's other staged entries alone.
    async fn write_commit(
        &self,
        git: &Git,
        attempt: &Attempt,
        files: &[String],
        text: &str,
    ) -> Result<String, RunsError> {
        let failed = RunsError::Unavailable;
        let old = git.head().await;
        let index = TempFile::new("index");
        let message = TempFile::new("message");
        std::fs::write(&message.0, text).map_err(|e| failed(e.to_string()))?;
        let index_path = index.path();
        let env = [("GIT_INDEX_FILE", index_path.as_str())];
        match &old {
            Some(head) => git.run_with(&["read-tree", head], &env).await,
            None => git.run_with(&["read-tree", "--empty"], &env).await,
        }
        .map_err(failed)?;
        git.each_paths(&["update-index", "--add", "--remove"], files, &env)
            .await
            .map_err(failed)?;
        let tree = git.run_with(&["write-tree"], &env).await.map_err(failed)?;
        let tree = String::from_utf8_lossy(&tree).trim().to_string();
        let message_path = message.path();
        let mut args = vec!["commit-tree", tree.as_str()];
        if let Some(head) = &old {
            args.extend(["-p", head.as_str()]);
        }
        args.extend(["-F", message_path.as_str()]);
        let new = git.run(&args).await.map_err(failed)?;
        let new = String::from_utf8_lossy(&new).trim().to_string();
        #[cfg(feature = "test-support")]
        if let Some(hook) = self.hooks().before_update_ref.clone() {
            hook(git.root());
        }
        let reflog = format!("shadows: T{} attempt {}", attempt.task, attempt.number);
        let expected = old.clone().unwrap_or_default();
        if let Err(error) = git
            .run(&["update-ref", "-m", &reflog, "HEAD", &new, &expected])
            .await
        {
            tracing::warn!(attempt = %attempt.id, %error, "runs.head_moved");
            return Err(RunsError::HeadMoved);
        }
        if let Err(error) = git.each_paths(&["reset", "-q", &new], files, &[]).await {
            tracing::warn!(attempt = %attempt.id, %error, "runs.index_not_refreshed");
        }
        Ok(new)
    }

    /// The folder `project` works in.
    pub(crate) async fn folder(&self, project: &ProjectId) -> Result<PathBuf, RunsError> {
        let project = self
            .inner
            .projects
            .get(project)
            .await
            .map_err(|error| RunsError::Unavailable(error.to_string()))?;
        project
            .directory
            .map(PathBuf::from)
            .ok_or(RunsError::NotGit)
    }

    async fn task_title(&self, attempt: &Attempt) -> Result<String, RunsError> {
        let plan = self.plan(&attempt.workflow_id, attempt.version).await?;
        Ok(plan
            .tasks
            .iter()
            .find(|t| t.content.number == attempt.task)
            .map(|t| t.content.title.clone())
            .unwrap_or_default())
    }

    fn commit_lock(&self, project: &ProjectId) -> Arc<tokio::sync::Mutex<()>> {
        self.inner
            .commits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(project.as_str().to_string())
            .or_default()
            .clone()
    }
}
