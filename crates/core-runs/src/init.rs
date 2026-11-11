//! One job: giving a project folder its git repository.
//!
//! Start refuses a folder that is not a git repository. The Planner may make
//! it one: `git init` and a first commit, in Shadows' name, of the files the
//! folder already holds. What Code's file list classes as a secret or a
//! dependency is left out of it, through git's local exclude list, and stays
//! in the folder untracked. No project file is written; starter files are a
//! task's work.
use std::path::{Path, PathBuf};

use crate::git::Git;
use crate::{Runs, RunsError};
use core_code::{Asker, CodeError, InventoryKind, InventoryQuery, InventoryState};
use core_grants::{Grant, GrantKind};
use core_projects::ProjectId;

/// The most entries one inventory page is asked for.
const PAGE: u16 = 200;

impl Runs {
    /// Makes the folder of `grant`'s project a git repository with a first
    /// commit of its files, secrets and dependencies left out, and answers
    /// that commit's full SHA. Only a Planner's grant may; a folder already
    /// inside a work tree, its own or a parent's, is refused.
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
        let left_out = self.left_out(&grant.project_id, &folder).await?;
        git.init().await.map_err(RunsError::Unavailable)?;
        git.exclude(&left_out)
            .await
            .map_err(RunsError::Unavailable)?;
        let commit = git
            .commit_all("Initial commit")
            .await
            .map_err(RunsError::Unavailable)?;
        tracing::info!(
            project = %grant.project_id,
            commit = %commit,
            left_out = left_out.len(),
            "runs.git_initialised"
        );
        Ok(commit)
    }

    /// Every path Code's fresh file list classes as a secret or a dependency,
    /// each with `true` for a folder. A list not complete yet is `NotListed`;
    /// a list that changed while it was read is read once more from its start.
    async fn left_out(
        &self,
        project: &ProjectId,
        folder: &Path,
    ) -> Result<Vec<(String, bool)>, RunsError> {
        let code = &self.inner.code;
        let unavailable = |error: CodeError| RunsError::Unavailable(error.to_string());
        code.refresh_inventory(Asker::Grant(project))
            .await
            .map_err(unavailable)?;
        let mut fresh_start = true;
        let mut paths = Vec::new();
        let mut after = None;
        loop {
            let query = InventoryQuery {
                prefix: String::new(),
                after: after.take(),
                limit: PAGE,
            };
            let page = match code.inventory(Asker::Grant(project), query).await {
                Ok(page) => page,
                Err(CodeError::InventoryStale) if fresh_start => {
                    fresh_start = false;
                    paths.clear();
                    continue;
                }
                Err(CodeError::InventoryStale) => return Err(RunsError::NotListed),
                Err(error) => return Err(unavailable(error)),
            };
            if matches!(
                page.state,
                InventoryState::Indexing | InventoryState::Missing
            ) {
                return Err(RunsError::NotListed);
            }
            for entry in page.entries {
                let reason = entry.excluded_reason.as_deref();
                if matches!(reason, Some("secret" | "dependencies")) {
                    let is_dir = entry.kind == InventoryKind::Directory
                        || reason == Some("dependencies")
                        || tokio::fs::metadata(folder.join(&entry.path))
                            .await
                            .is_ok_and(|m| m.is_dir());
                    paths.push((entry.path, is_dir));
                }
            }
            match page.next {
                Some(next) => after = Some(next),
                None => return Ok(paths),
            }
        }
    }
}
