//! Session-scoped background task state.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use core_threads::ThreadId;
use shadows_agent::events::{HarnessEvent, TaskUpdate};
use tokio::sync::broadcast;

/// One background task of an open session, as the `tasks` frame carries it.
/// `state` is `running`, `completed`, `failed` or `stopped`; times are Unix
/// milliseconds as Core saw the adapter's reports.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct BackgroundTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_type: String,
    pub state: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub summary: Option<String>,
    pub can_stop: bool,
}

/// Every open session's task list in memory, never in the database. One
/// revision counter orders all changes, so a snapshot can be compared with a
/// queued change of the same thread.
pub(super) struct Tasks {
    latest: Mutex<HashMap<ThreadId, (u64, Vec<BackgroundTask>)>>,
    revision: AtomicU64,
    changed: broadcast::Sender<(ThreadId, u64, Vec<BackgroundTask>)>,
}

impl Tasks {
    pub(super) fn new() -> Self {
        Self {
            latest: Mutex::new(HashMap::new()),
            revision: AtomicU64::new(0),
            changed: broadcast::channel(256).0,
        }
    }
    pub(super) fn get(&self, thread: &ThreadId) -> (u64, Vec<BackgroundTask>) {
        self.latest
            .lock()
            .expect("tasks lock")
            .get(thread)
            .cloned()
            .unwrap_or_default()
    }
    pub(super) fn subscribe(&self) -> broadcast::Receiver<(ThreadId, u64, Vec<BackgroundTask>)> {
        self.changed.subscribe()
    }
    /// Drops the closed session's list; its subscribers get the empty list
    /// once, when there was one.
    pub(super) fn forget(&self, thread: &ThreadId) {
        let Some((_, list)) = self.latest.lock().expect("tasks lock").remove(thread) else {
            return;
        };
        if !list.is_empty() {
            let revision = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = self.changed.send((thread.clone(), revision, Vec::new()));
        }
    }
    fn record(&self, thread: &ThreadId, update: TaskUpdate) {
        let mut all = self.latest.lock().expect("tasks lock");
        let (revision, list) = all.entry(thread.clone()).or_default();
        match update {
            TaskUpdate::Spawned {
                async_task_id,
                name,
                task_type,
                description,
                can_stop,
                ..
            } => {
                if list.iter().any(|task| task.id == async_task_id) {
                    return;
                }
                list.push(BackgroundTask {
                    id: async_task_id,
                    name,
                    description,
                    task_type,
                    state: "running".into(),
                    started_at: now(),
                    ended_at: None,
                    summary: None,
                    can_stop,
                });
            }
            TaskUpdate::State {
                async_task_id,
                state,
                summary,
                ..
            } => {
                if !matches!(state.as_str(), "completed" | "failed" | "stopped") {
                    return;
                }
                let Some(task) = list.iter_mut().find(|task| task.id == async_task_id) else {
                    return;
                };
                if task.state == state && task.summary == summary {
                    return;
                }
                task.state = state;
                task.summary = summary;
                task.ended_at.get_or_insert_with(now);
            }
            TaskUpdate::Progress { .. } => return,
        }
        *revision = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.changed.send((thread.clone(), *revision, list.clone()));
    }
    pub(super) fn intercept(
        tasks: Arc<Self>,
        thread: ThreadId,
        next: impl Fn(HarnessEvent) + Clone + Send + Sync + 'static,
    ) -> impl Fn(HarnessEvent) + Clone + Send + Sync + 'static {
        move |event| match event {
            HarnessEvent::Task(update) => tasks.record(&thread, update),
            other => next(other),
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl crate::Harness {
    /// The thread's open session's tasks in start order; empty when it has none.
    pub fn tasks_of(&self, thread: &ThreadId) -> Vec<BackgroundTask> {
        self.sessions.tasks.get(thread).1
    }

    /// The list with the revision it was published at, so a subscriber that
    /// sends it after `caught-up` can skip the older changes still queued.
    pub fn task_snapshot(&self, thread: &ThreadId) -> (u64, Vec<BackgroundTask>) {
        self.sessions.tasks.get(thread)
    }

    /// Every change to any thread's task list, as it happens. `Events` takes
    /// it for each subscriber, before the journal is read.
    pub fn watch_tasks(&self) -> broadcast::Receiver<(ThreadId, u64, Vec<BackgroundTask>)> {
        self.sessions.tasks.subscribe()
    }

    #[cfg(feature = "test-support")]
    pub fn report_task_for_test(&self, thread: &ThreadId, update: TaskUpdate) {
        self.sessions.tasks.record(thread, update);
    }

    /// Asks the adapter to stop one running task. `None` for an unknown
    /// thread, a closed session or an unknown task; `Some(false)` for a task
    /// that already ended or cannot be stopped. The list changes only when the
    /// adapter reports the task stopped.
    pub async fn stop_task(
        &self,
        thread: &ThreadId,
        id: &str,
    ) -> Result<Option<bool>, crate::HarnessError> {
        // Checked before the slot, which would otherwise be added for any id.
        let mut conn = self
            .store
            .storage
            .reader()
            .begin()
            .await
            .map_err(core_db::StorageError::from)?;
        match core_threads::Threads::turn_context_in(&mut conn, thread).await {
            Ok(_) => {}
            Err(core_db::StorageError::NotFound(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        drop(conn);
        // The slot is released before the request: a turn or a close must not
        // wait on the adapter's answer.
        let opened = {
            let slot = self.sessions.slot(thread).await;
            let mut live = slot.lock().await;
            let Some(live) = live.as_mut() else {
                return Ok(None);
            };
            if live.opened.connection.is_closed() || live.handle.has_exited() {
                return Ok(None);
            }
            live.opened.clone()
        };
        let tasks = self.sessions.tasks.get(thread).1;
        let Some(task) = tasks.iter().find(|task| task.id == id) else {
            return Ok(None);
        };
        if task.state != "running" || !task.can_stop {
            return Ok(Some(false));
        }
        opened
            .connection
            .stop_task(&opened.session_id, id)
            .await
            .map(Some)
            .map_err(|error| crate::HarnessError::HarnessStartFailed(error.to_string()))
    }
}
