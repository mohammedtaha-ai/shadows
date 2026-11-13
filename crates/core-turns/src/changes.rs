//! One job: the Planner's changes to the project's files — `project_change_note`,
//! and the end-of-turn check that asks about the changes it did not explain.

use super::handles::Noted;
use super::store::TurnsStore;
use super::{Turns, TurnsError};
use crate::model::OperationId;
use core_threads::ThreadId;

impl Turns {
    /// The Planner says why it changed `paths` (1 to 100, each inside the
    /// project as `code_lsp`'s file is) with `reason` (1 to 2000 characters):
    /// a `ProjectChange` entry `{ paths, reason, noted: true }` in its thread,
    /// and the running turn counts them as explained.
    pub async fn note_change(
        &self,
        thread: &ThreadId,
        paths: Vec<String>,
        reason: String,
    ) -> Result<(), TurnsError> {
        let reason = reason.trim().to_owned();
        if !(1..=2000).contains(&reason.chars().count()) {
            return Err(TurnsError::InvalidCommand(
                "a reason is 1 to 2000 characters".into(),
            ));
        }
        if !(1..=100).contains(&paths.len()) {
            return Err(TurnsError::InvalidCommand(
                "a note names 1 to 100 paths".into(),
            ));
        }
        let project = self.store.thread_project(thread).await?;
        let mut checked = Vec::with_capacity(paths.len());
        for path in &paths {
            let path = self
                .code
                .project_file(&project, path)
                .await
                .map_err(|error| TurnsError::InvalidCommand(error.to_string()))?;
            if !checked.contains(&path) {
                checked.push(path);
            }
        }
        self.store.note_change(thread, &checked, &reason).await?;
        if let Some(noted) = self.handles.noted_for(thread).await
            && let Ok(mut noted) = noted.lock()
        {
            noted.extend(checked);
        }
        Ok(())
    }

    /// When a Planner turn settles: the paths its project's watcher saw change
    /// while it ran, less those it noted, are recorded with no reason and asked
    /// about in one Shadows notice, which waits in the queue. Nothing when
    /// every change was noted. A failure is logged; the turn's ending stands.
    pub(crate) async fn check_changes(
        store: &TurnsStore,
        thread: &ThreadId,
        operation: &OperationId,
        hold: &codebase::ActiveHold,
        noted: &Noted,
    ) {
        let noted = noted.lock().map(|n| n.clone()).unwrap_or_default();
        let unnoted: Vec<String> = hold
            .changed()
            .into_iter()
            .filter(|path| !noted.contains(path))
            .collect();
        if unnoted.is_empty() {
            return;
        }
        let settings = match store.get_operation(operation).await {
            Ok(op) => match op.invocation {
                Some(i) => (i.requested_model, i.requested_mode, i.requested_effort),
                None => return,
            },
            Err(error) => {
                tracing::error!(%error, "planner.changes_check_failed");
                return;
            }
        };
        match store
            .record_unnoted(thread, operation, &unnoted, settings)
            .await
        {
            Ok(true) => tracing::info!(paths = unnoted.len(), "planner.unnoted_changes"),
            Ok(false) => {}
            Err(error) => tracing::error!(%error, "planner.changes_check_failed"),
        }
    }
}
