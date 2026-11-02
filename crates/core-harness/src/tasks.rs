//! Session-scoped background task state.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use core_threads::ThreadId;
use shadows_agent::events::{HarnessEvent, TaskUpdate};
use tokio::sync::broadcast;

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
    pub(super) fn forget(&self, thread: &ThreadId) {
        let mut all = self.latest.lock().expect("tasks lock");
        if let Some((revision, list)) = all.get_mut(thread) {
            if list.is_empty() {
                return;
            }
            list.clear();
            *revision = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = self.changed.send((thread.clone(), *revision, Vec::new()));
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
    pub fn tasks_of(&self, thread: &ThreadId) -> Vec<BackgroundTask> {
        self.sessions.tasks.get(thread).1
    }
    pub fn task_snapshot(&self, thread: &ThreadId) -> (u64, Vec<BackgroundTask>) {
        self.sessions.tasks.get(thread)
    }
    pub fn watch_tasks(&self) -> broadcast::Receiver<(ThreadId, u64, Vec<BackgroundTask>)> {
        self.sessions.tasks.subscribe()
    }
    #[cfg(feature = "test-support")]
    pub fn report_task_for_test(&self, thread: &ThreadId, update: TaskUpdate) {
        self.sessions.tasks.record(thread, update);
    }
    pub async fn stop_task(
        &self,
        thread: &ThreadId,
        id: &str,
    ) -> Result<Option<bool>, crate::HarnessError> {
        let slot = self.sessions.slot(thread).await;
        let mut live = slot.lock().await;
        let Some(live) = live.as_mut() else {
            return Ok(None);
        };
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
        if live.opened.connection.is_closed() || live.handle.has_exited() {
            return Ok(None);
        }
        let tasks = self.sessions.tasks.get(thread).1;
        let Some(task) = tasks.iter().find(|task| task.id == id) else {
            return Ok(None);
        };
        if task.state != "running" || !task.can_stop {
            return Ok(Some(false));
        }
        live.opened
            .connection
            .stop_task(&live.opened.session_id, id)
            .await
            .map(Some)
            .map_err(|error| crate::HarnessError::HarnessStartFailed(error.to_string()))
    }
}
