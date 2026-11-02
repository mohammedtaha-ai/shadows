//! The background observer's owner: one runner per binding, replaced on change.
mod feed;
use super::{
    binding::BindingView,
    projection::{PresenceProjection, Scope},
};
use crate::core_connection::ReadyConnection;
use feed::{run, Outlet};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};

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
type Acknowledgement = (
    oneshot::Sender<Result<(), &'static str>>,
    Result<(), &'static str>,
);
pub struct Update {
    guard: BindingGuard,
    pub scope: Option<Scope>,
    pub projection: PresenceProjection,
    pub appearance: Option<super::CompanionAppearance>,
    pub republish_only: bool,
    pub waiting: crate::windows::companion::SurfaceWaiting,
    acknowledgement: Option<Acknowledgement>,
    epoch: u64,
    current: Arc<AtomicU64>,
}
impl Update {
    pub fn acknowledge(&mut self) {
        if let Some((reply, result)) = self.acknowledgement.take() {
            let _ = reply.send(if self.is_current() {
                result
            } else {
                Err("companion_binding_changed")
            });
        }
    }
    #[cfg(test)]
    pub fn has_acknowledgement(&self) -> bool {
        self.acknowledgement.is_some()
    }

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
struct RefreshRequest {
    guard: (u64, Option<String>),
    unbound: bool,
    epoch: u64,
    reply: oneshot::Sender<Result<(), &'static str>>,
}
pub struct PresenceWatcher {
    refreshes: mpsc::Sender<RefreshRequest>,

    current: Arc<AtomicU64>,
    requests: watch::Sender<Request>,
    ready: watch::Receiver<Readiness>,
    last: Mutex<Option<(BindingView, ReadyConnection)>>,
}
impl super::Presence {
    pub fn start_watcher(&self, notify: Notify) {
        assert!(
            self.watcher
                .set(PresenceWatcher::start(notify, Timing::default()))
                .is_ok(),
            "companion observer is started once"
        );
    }

    pub(super) fn watcher(&self) -> &PresenceWatcher {
        self.watcher
            .get()
            .expect("companion observer is started before use")
    }

    pub fn sync_watcher(&self, binding: BindingView, connection: Option<ReadyConnection>) {
        self.watcher().sync(binding, connection);
    }

    pub fn stop(&self) {
        self.watcher().stop();
    }
}

impl PresenceWatcher {
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
        let (refreshes, mut refresh_input) = mpsc::channel::<RefreshRequest>(16);
        // One manager owns one runner future. Replacing input drops old reads before opening the next stream.
        tauri::async_runtime::spawn(async move {
            loop {
                ready_updates.send_replace(None);
                let request = input.borrow_and_update().clone();
                if let Some((binding, connection)) = request.binding {
                    tokio::select! {
                        biased;
                     result=input.changed()=>{if result.is_err(){break}},
                     _=run(binding,connection,Outlet{epoch:request.epoch,current:generation.clone(),notify:notify.clone(),ready:ready_updates.clone()},timing.clone(), &mut refresh_input)=>{
                      if input.changed().await.is_err(){break}
                     }
                    }
                } else {
                    notify(Update {
                        guard: request.guard.clone(),
                        scope: None,
                        projection: request.stopped.clone(),
                        appearance: None,
                        republish_only: false,
                        waiting: Default::default(),
                        acknowledgement: None,
                        epoch: request.epoch,
                        current: generation.clone(),
                    });
                    tokio::select! {
                        result = input.changed() => { if result.is_err() {break;} }
                        Some(refresh) = refresh_input.recv() => {
                            if refresh.epoch != request.epoch {
                                let _ = refresh.reply.send(Err("companion_binding_changed"));
                            } else {
                                notify(Update {
                                    guard: Some(refresh.guard), scope: None,
                                    projection: request.stopped.clone(), appearance: None,
                                    republish_only: true,
                                    waiting: Default::default(),
                                    acknowledgement: Some((refresh.reply, if refresh.unbound {Ok(())} else {Err("companion_core_unavailable")})),
                                    epoch: request.epoch, current: generation.clone(),
                                });
                            }
                        }
                    }
                }
            }
        });
        Arc::new(Self {
            refreshes,
            current,
            requests,
            ready,
            last: Mutex::new(None),
        })
    }
    pub async fn refresh_appearance(
        &self,
        binding: &BindingView,
    ) -> Result<super::RefreshAppearanceReply, &'static str> {
        let request = self.requests.borrow().clone();
        let matches = if let Some((current, _)) = &request.binding {
            current == binding
        } else {
            request.guard.as_ref() == Some(&(binding.generation, binding.configuration_id.clone()))
                || (request.guard.is_none() && binding.agent.is_none())
        };
        if !matches {
            return Err("companion_binding_changed");
        }
        let (reply, result) = oneshot::channel();
        self.refreshes
            .send(RefreshRequest {
                guard: (binding.generation, binding.configuration_id.clone()),
                unbound: binding.agent.is_none(),
                epoch: request.epoch,
                reply,
            })
            .await
            .map_err(|_| "companion_core_unavailable")?;
        tokio::time::timeout(Duration::from_secs(22), result)
            .await
            .map_err(|_| "companion_core_unavailable")?
            .map_err(|_| "companion_binding_changed")??;
        Ok(super::RefreshAppearanceReply { ok: true })
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
