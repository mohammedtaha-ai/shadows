//! One job: the code index (spec §15.4–§15.5) — the `Code` service. It walks
//! a project's folder into SQLite and answers three questions from it:
//! where a name is defined, who uses it, and what a path holds.
//!
//! `model` holds the types, `store` the queries, `scan` the walk and the
//! indexing of one file, `scope` who may read what.

mod model;
mod scan;
mod scope;
mod store;

use std::sync::Arc;

pub use model::{Answer, Hit, IndexState, ProjectStatus, Skipped};
pub use scope::Asker;

use crate::db::Storage;
use crate::error::CoreError;
use crate::projects::ProjectId;
use store::ScopeRow;

/// The hits an answer carries at most (§15.5).
const HITS: usize = 50;

struct Inner {
    storage: Arc<Storage>,
}

/// The code index. Cheap to clone: `Projects`, `Threads` and `Turns` hold a
/// clone to call `touch` (§15.6).
#[derive(Clone)]
pub struct Code {
    inner: Arc<Inner>,
}

impl Code {
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self {
            inner: Arc::new(Inner { storage }),
        }
    }

    /// Where `name` is defined, exactly and case-sensitively.
    pub async fn definitions(
        &self,
        asker: Asker<'_>,
        only: Option<&str>,
        name: &str,
    ) -> Result<Answer, CoreError> {
        self.by_name(asker, only, name, "definition").await
    }

    /// Who uses `name`: matched by name only, so every hit says `matched_by: name`.
    pub async fn references(
        &self,
        asker: Asker<'_>,
        only: Option<&str>,
        name: &str,
    ) -> Result<Answer, CoreError> {
        let mut answer = self.by_name(asker, only, name, "reference").await?;
        for hit in &mut answer.hits {
            hit.matched_by = Some("name".into());
        }
        Ok(answer)
    }

    /// The definitions at or under `path`, relative to each project's folder;
    /// `""` or `"."` is the whole project. It reads the index and opens no path.
    pub async fn outline(
        &self,
        asker: Asker<'_>,
        only: Option<&str>,
        path: &str,
    ) -> Result<Answer, CoreError> {
        let path = scope::inside(path)?;
        let scope = scope::projects(&self.inner.storage, &asker, only).await?;
        let rows = self
            .inner
            .storage
            .code_outline(&scope, &scan::path_key(&path))
            .await?;
        self.answer(&scope, rows, Vec::new()).await
    }

    /// How the project's index stands.
    pub async fn status(&self, project: &ProjectId) -> Result<ProjectStatus, CoreError> {
        let p = self.inner.storage.get_project(project).await?;
        self.status_of(&(p.id, p.slug, p.directory)).await
    }

    /// Scans `project` once, now, on the caller's task. Tests and Task 3's worker call it.
    #[cfg_attr(
        not(feature = "test-support"),
        expect(dead_code, reason = "only tests scan until the workers of §15.4 exist")
    )]
    pub(crate) async fn scan(&self, project: &ProjectId) -> Result<(), CoreError> {
        self.scan_project(project).await
    }

    async fn by_name(
        &self,
        asker: Asker<'_>,
        only: Option<&str>,
        name: &str,
        role: &str,
    ) -> Result<Answer, CoreError> {
        let storage = &self.inner.storage;
        let scope = scope::projects(storage, &asker, only).await?;
        let rows = storage.code_by_name(&scope, name, role).await?;
        let suggestions = if rows.is_empty() {
            storage.code_suggestions(&scope, name, role).await?
        } else {
            Vec::new()
        };
        self.answer(&scope, rows, suggestions).await
    }

    /// At most 50 hits, `more` from the 51st, and each project's status.
    async fn answer(
        &self,
        scope: &[ScopeRow],
        mut hits: Vec<Hit>,
        suggestions: Vec<String>,
    ) -> Result<Answer, CoreError> {
        let more = hits.len() > HITS;
        hits.truncate(HITS);
        let mut status = Vec::with_capacity(scope.len());
        for project in scope {
            status.push(self.status_of(project).await?);
        }
        Ok(Answer {
            hits,
            more,
            suggestions,
            status,
        })
    }

    /// Until the active set exists, `Ready` for a project with rows and
    /// `Inactive` for one without.
    async fn status_of(
        &self,
        (id, slug, directory): &ScopeRow,
    ) -> Result<ProjectStatus, CoreError> {
        let (files, skipped) = self.inner.storage.code_counts(id).await?;
        let rows = files + skipped.iter().map(|s| s.count).sum::<u32>();
        let state = match directory {
            None => IndexState::NoDirectory,
            Some(_) if rows > 0 => IndexState::Ready,
            Some(_) => IndexState::Inactive,
        };
        Ok(ProjectStatus {
            project: slug.clone(),
            state,
            files,
            skipped,
            updated_at: None,
        })
    }
}
