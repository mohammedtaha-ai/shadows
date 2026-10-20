pub mod binding;
mod client;
mod preferences;
mod preview;
pub mod projection;
pub mod send;
#[cfg(test)]
mod send_tests;
#[cfg(test)]
mod tests;
pub mod watcher;
#[cfg(test)]
mod watcher_fixture;
#[cfg(test)]
mod watcher_tests;
use crate::{core_connection::Connection, navigation::MainDestination};
pub use binding::{BindingView, RequestContext};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
type Notify = Arc<dyn Fn(BindingView) + Send + Sync>;
struct Model {
    view: BindingView,
    error: Option<&'static str>,
    ticket: u64,
    send: send::State,
}
pub struct Presence {
    root: PathBuf,
    pub(crate) connection: Arc<Connection>,
    inner: Mutex<Model>,
    notifier: Mutex<Option<Notify>>,
}
impl Presence {
    pub fn new(root: PathBuf, connection: Arc<Connection>) -> Arc<Self> {
        let configuration_id = connection.snapshot().configuration_id;
        let (view, error) = match preferences::load(&root, configuration_id.clone()) {
            Ok(view) => (view, None),
            Err(error) => (
                BindingView {
                    revision: 0,
                    generation: 1,
                    configuration_id,
                    agent: None,
                    target: None,
                    previews: true,
                },
                Some(error),
            ),
        };
        Arc::new(Self {
            root,
            connection,
            inner: Mutex::new(Model {
                send: send::State::new(&view),
                view,
                error,
                ticket: 0,
            }),
            notifier: Mutex::new(None),
        })
    }
    pub fn set_notifier(&self, notify: Notify) {
        *self.notifier.lock().unwrap() = Some(notify);
    }
    fn notify(&self) {
        let notify = self.notifier.lock().unwrap().clone();
        if let Some(notify) = notify {
            notify(self.view());
        }
    }
    pub fn view(&self) -> BindingView {
        self.inner.lock().unwrap().view.clone()
    }
    pub fn snapshot(&self) -> Result<BindingView, &'static str> {
        let inner = self.inner.lock().unwrap();
        if let Some(error) = inner.error {
            return Err(error);
        }
        Ok(inner.view.clone())
    }
    pub fn configuration_changed(&self, configuration_id: Option<String>) {
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.view.configuration_id == configuration_id {
                return;
            }
            let before = inner.view.clone();
            inner.ticket += 1;
            inner.view.revision += 1;
            inner.view.generation += 1;
            inner.view.configuration_id = configuration_id;
            inner.view.agent = None;
            inner.view.target = None;
            inner.error = preferences::save(&self.root, &inner.view).err();
            let after = inner.view.clone();
            inner.send.changed(&before, &after, false);
        }
        self.notify();
    }
    pub fn request(&self, expected: Option<u64>) -> Result<RequestContext, &'static str> {
        let ready = self
            .connection
            .ready()
            .ok_or("companion_core_unavailable")?;
        let mut inner = self.inner.lock().unwrap();
        if inner.view.configuration_id.as_deref() != Some(&ready.configuration_id)
            || expected.is_some_and(|value| value != inner.view.generation)
        {
            return Err("companion_binding_changed");
        }
        inner.ticket += 1;
        Ok(RequestContext {
            ticket: inner.ticket,
            generation: inner.view.generation,
            ready,
        })
    }
    fn commit(
        &self,
        request: RequestContext,
        mutate: impl FnOnce(&mut BindingView),
    ) -> Result<BindingView, &'static str> {
        self.commit_with_origin(request, mutate, false)
    }
    fn commit_with_origin(
        &self,
        request: RequestContext,
        mutate: impl FnOnce(&mut BindingView),
        own_bind: bool,
    ) -> Result<BindingView, &'static str> {
        let view = {
            let mut inner = self.inner.lock().unwrap();
            if inner.ticket != request.ticket
                || inner.view.generation != request.generation
                || self.connection.ready().as_ref() != Some(&request.ready)
                || inner.view.configuration_id.as_deref() != Some(&request.ready.configuration_id)
            {
                return Err("companion_binding_changed");
            }
            let mut next = inner.view.clone();
            mutate(&mut next);
            if next == inner.view && inner.error.is_none() {
                return Ok(next);
            }
            next.generation += 1;
            next.revision += 1;
            preferences::save(&self.root, &next)?;
            inner.error = None;
            let before = inner.view.clone();
            inner.send.changed(&before, &next, own_bind);
            inner.view = next.clone();
            next
        };
        self.notify();
        Ok(view)
    }
    pub fn select(
        &self,
        request: RequestContext,
        agent: Option<String>,
    ) -> Result<BindingView, &'static str> {
        if let Some(agent) = &agent {
            binding::live_agent(&binding::workspace(&request.ready)?, agent)?;
        }
        self.commit(request, move |view| {
            if view.agent != agent {
                view.agent = agent;
                view.target = None;
            }
        })
    }
    pub fn retarget(
        &self,
        request: RequestContext,
        thread_id: String,
    ) -> Result<BindingView, &'static str> {
        let agent = self.view().agent.ok_or("companion_agent_unavailable")?;
        let workspace = binding::workspace(&request.ready)?;
        binding::live_agent(&workspace, &agent)?;
        let target = binding::target(&request.ready, &workspace, &agent, &thread_id)?;
        self.commit(request, move |view| view.target = Some(target))
    }
    pub fn previews(&self, enabled: bool) -> Result<BindingView, &'static str> {
        let view = {
            let mut inner = self.inner.lock().unwrap();
            if inner.view.previews == enabled {
                return Ok(inner.view.clone());
            }
            let mut next = inner.view.clone();
            next.previews = enabled;
            next.revision += 1;
            next.generation += 1;
            preferences::save(&self.root, &next)?;
            inner.ticket += 1;
            inner.error = None;
            let before = inner.view.clone();
            inner.send.changed(&before, &next, false);
            inner.view = next.clone();
            next
        };
        self.notify();
        Ok(view)
    }
    pub fn destination(&self) -> MainDestination {
        let destination = || -> Result<MainDestination, &'static str> {
            let view = self.snapshot()?;
            let Some(agent) = view.agent else {
                return Ok(MainDestination::CompanionSettings);
            };
            let ready = self
                .connection
                .ready()
                .ok_or("companion_core_unavailable")?;
            if view.configuration_id.as_deref() != Some(&ready.configuration_id) {
                return Err("companion_binding_changed");
            }
            let workspace = binding::workspace(&ready)?;
            binding::live_agent(&workspace, &agent)?;
            let result = if let Some(target) = view.target {
                let target = binding::target(&ready, &workspace, &agent, &target.thread_id)?;
                MainDestination::Conversation {
                    project_id: target.project_id,
                    thread_id: target.thread_id,
                }
            } else {
                MainDestination::AgentDraft { agent }
            };
            if self.view().generation != view.generation {
                return Err("companion_binding_changed");
            }
            Ok(result)
        };
        destination().unwrap_or(MainDestination::CompanionSettings)
    }
}
#[cfg(test)]
mod binding_tests;
