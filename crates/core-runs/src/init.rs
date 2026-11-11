//! One job: giving a project folder its git repository.
//!
//! Start refuses a folder that is not a git repository. The Planner may make
//! it one: `git init` and one empty first commit in Shadows' name. Nothing is
//! written into the folder; starter files are a task's work.
use std::path::PathBuf;

use crate::git::Git;
use crate::{Runs, RunsError};
use core_grants::{Grant, GrantKind};

impl Runs {
    /// Makes the folder of `grant`'s project a git repository with one empty
    /// commit, and answers that commit's full SHA. Only a Planner's grant may;
    /// a folder already inside a work tree, its own or a parent's, is refused.
    pub async fn init_git(&self, grant: &Grant) -> Result<String, RunsError> {
        if grant.kind != GrantKind::Thread {
            return Err(RunsError::NotFound);
        }
        let project = self
            .inner
            .projects
            .get(&grant.project_id)
            .await
            .map_err(|error| RunsError::Unavailable(error.to_string()))?;
        let folder = project
            .directory
            .map(PathBuf::from)
            .ok_or(RunsError::NoFolder)?;
        let git = Git::at(&folder);
        if git.prefix().await.is_some() {
            return Err(RunsError::AlreadyGit);
        }
        git.init().await.map_err(RunsError::Unavailable)?;
        let commit = git
            .commit_empty("Initial commit")
            .await
            .map_err(RunsError::Unavailable)?;
        tracing::info!(project = %grant.project_id, commit = %commit, "runs.git_initialised");
        Ok(commit)
    }
}
