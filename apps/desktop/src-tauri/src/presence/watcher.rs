//! The background observer's owner: one runner per binding, replaced on change.
mod feed;
use super::{
    binding::BindingView,
    projection::{PresenceProjection, Scope},
};
use crate::core_connection::ReadyConnection;
use feed::run;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::watch;

#[derive(Clone)]
pub struct Timing {
    pub snapshot: Duration,
    pub liveness: Duration,
    pub tick: Duration,
    pub retry: Duration,
    pub retry_cap: Duration,
    pub entry: Duration,
}
impl Default for Timing {
    fn default() -> Self {
        Self {
            snapshot: Duration::from_secs(10),
            liveness: Duration::from_secs(10),
            tick: Duration::from_millis(250),
            retry: Duration::from_millis(500),
            retry_cap: Duration::from_secs(10),
            entry: Duration::from_millis(250),
        }
    }
}
#[cfg(test)]
impl Timing {
    pub fn test() -> Self {
        Self {
            snapshot: Duration::from_secs(1),
            liveness: Duration::from_millis(80),
            tick: Duration::from_millis(10),
            retry: Duration::from_millis(10),
            retry_cap: Duration::from_millis(40),
            entry: Duration::from_millis(30),
        }
    }
}
type BindingGuard = Option<(u64, Option<String>)>;
pub struct Update {
    guard: BindingGuard,
    pub scope: Option<Scope>,
    pub projection: PresenceProjection,
    epoch: u64,
    current: Arc<AtomicU64>,
}
impl Update {
    pub fn matches_binding(&self, binding: &BindingView) -> bool {
        if let Some(scope) = self.scope.as_ref() {
            return binding.generation == scope.generation
                && binding.configuration_id.as_deref() == Some(scope.configuration_id.as_str());
        }
        self.guard
            .as_ref()
            .is_none_or(|(generation, configuration)| {
                binding.generation == *generation && binding.configuration_id == *configuration
            })
    }
    pub fn is_current(&self) -> bool {
        self.current.load(Ordering::SeqCst) == self.epoch
    }
}
type Readiness = Option<(u64, Scope)>;
type Notify = Arc<dyn Fn(Update) + Send + Sync>;
#[derive(Clone)]
struct Request {
    guard: BindingGuard,
    epoch: u64,
    binding: Option<(BindingView, ReadyConnection)>,
    stopped: PresenceProjection,
}
pub struct PresenceWatcher {
    current: Arc<AtomicU64>,
    requests: watch::Sender<Request>,
    ready: watch::Receiver<Readiness>,
    last: Mutex<Option<(BindingView, ReadyConnection)>>,
}
impl PresenceWatcher {
    pub fn new(notify: Notify) -> Arc<Self> {
        Self::start(notify, Timing::default())
    }
    #[cfg(test)]
    pub fn with_timing(notify: Notify, timing: Timing) -> Arc<Self> {
        Self::start(notify, timing)
    }
    fn start(notify: Notify, timing: Timing) -> Arc<Self> {
        let current = Arc::new(AtomicU64::new(0));
        let (requests, mut input) = watch::channel(Request {
            guard: None,
            epoch: 0,
            binding: None,
            stopped: PresenceProjection::unbound(),
        });
        let (ready_updates, ready) = watch::channel(None);
        let generation = current.clone();
        // One manager owns one runner future. Replacing input drops old reads before opening the next stream.
        tauri::async_runtime::spawn(async move {
            loop {
                ready_updates.send_replace(None);
                let request = input.borrow_and_update().clone();
                if let Some((binding, connection)) = request.binding {
                    tokio::select! {
                        biased;
                     result=input.changed()=>{if result.is_err(){break}},
                     _=run(binding,connection,request.epoch,generation.clone(),notify.clone(),timing.clone(),ready_updates.clone())=>{
                      if input.changed().await.is_err(){break}
                     }
                    }
                } else {
                    notify(Update {
                        guard: request.guard,
                        scope: None,
                        projection: request.stopped,
                        epoch: request.epoch,
                        current: generation.clone(),
                    });
                    if input.changed().await.is_err() {
                        break;
                    }
                }
            }
        });
        Arc::new(Self {
            current,
            requests,
            ready,
            last: Mutex::new(None),
        })
    }
    pub async fn wait_ready(
        &self,
        binding: &BindingView,
        timeout: Duration,
    ) -> Result<(), &'static str> {
        let epoch = {
            let last = self.last.lock().unwrap();
            if last.as_ref().map(|(current, _)| current) != Some(binding) {
                return Err("companion_binding_changed");
            }
            self.current.load(Ordering::SeqCst)
        };
        let mut ready = self.ready.clone();
        tokio::time::timeout(timeout, async {
            loop {
                if self.current.load(Ordering::SeqCst) != epoch {
                    return Err("companion_binding_changed");
                }
                if ready.borrow().as_ref().is_some_and(|(observed, scope)| {
                    *observed == epoch
                        && scope.generation == binding.generation
                        && binding.configuration_id.as_deref()
                            == Some(scope.configuration_id.as_str())
                        && scope.thread_id
                            == binding
                                .target
                                .as_ref()
                                .map(|target| target.thread_id.clone())
                }) {
                    return Ok(());
                }
                ready
                    .changed()
                    .await
                    .map_err(|_| "companion_watch_unavailable")?;
            }
        })
        .await
        .map_err(|_| "companion_watch_unavailable")?
    }
    pub fn replace(&self, binding: BindingView, connection: ReadyConnection) {
        let mut last = self.last.lock().unwrap();
        if last.as_ref() == Some(&(binding.clone(), connection.clone())) {
            return;
        }
        *last = Some((binding.clone(), connection.clone()));
        let epoch = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.requests.send_replace(Request {
            epoch,
            guard: None,
            binding: Some((binding, connection)),
            stopped: PresenceProjection::unbound(),
        });
    }
    pub fn sync(&self, binding: BindingView, ready: Option<ReadyConnection>) {
        let guard = Some((binding.generation, binding.configuration_id.clone()));
        if binding.agent.is_none() {
            self.stopped(PresenceProjection::unbound(), guard);
        } else if let Some(ready) = ready {
            self.replace(binding, ready);
        } else {
            self.stopped(
                PresenceProjection::unavailable("Core unavailable. Retry local Core in Settings."),
                guard,
            );
        }
    }
    pub fn stop(&self) {
        self.stopped(
            PresenceProjection::unavailable("Core unavailable. Retry local Core in Settings."),
            None,
        );
    }
    fn stopped(&self, projection: PresenceProjection, guard: BindingGuard) {
        let mut last = self.last.lock().unwrap();
        *last = None;
        let epoch = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.requests.send_replace(Request {
            epoch,
            binding: None,
            stopped: projection,
            guard,
        });
    }
}
