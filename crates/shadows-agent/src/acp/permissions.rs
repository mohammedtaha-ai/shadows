//! In-flight adapter permission requests and their single-settlement lifecycle.
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::Connection;
use crate::events::{HarnessEvent, PermissionOption};
use agent_client_protocol::Responder;
use agent_client_protocol::schema::v1::{
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPolicy {
    Refuse,
    Ask,
}

#[derive(Debug, Clone)]
pub struct WaitingPermission {
    pub request: String,
    pub session: String,
    pub title: String,
    pub options: Vec<PermissionOption>,
    pub asked_at: u128,
}

#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("permission request is unknown")]
    Unknown,
    #[error("permission request is settled")]
    Settled,
    #[error("permission option was not offered")]
    OptionNotOffered,
}

struct Held {
    waiting: WaitingPermission,
    responder: Responder<RequestPermissionResponse>,
}
#[derive(Default)]
struct State {
    waiting: HashMap<String, Held>,
    settled: HashSet<String>,
}
#[derive(Clone)]
pub(super) struct Permissions {
    state: Arc<Mutex<State>>,
    pub(super) settlement: Arc<tokio::sync::Mutex<()>>,
    events: Arc<dyn Fn(HarnessEvent) + Send + Sync>,
}
impl Permissions {
    pub(super) fn new(events: impl Fn(HarnessEvent) + Send + Sync + 'static) -> Self {
        Self {
            state: Arc::default(),
            settlement: Arc::default(),
            events: Arc::new(events),
        }
    }
    pub(super) fn hold(
        &self,
        request: RequestPermissionRequest,
        responder: Responder<RequestPermissionResponse>,
    ) {
        let waiting = WaitingPermission {
            request: uuid::Uuid::new_v4().to_string(),
            session: request.session_id.to_string(),
            title: request.tool_call.fields.title.unwrap_or_default(),
            options: request
                .options
                .into_iter()
                .map(PermissionOption::from)
                .collect(),
            asked_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        };
        let event = HarnessEvent::PermissionAsked {
            request: waiting.request.clone(),
            title: waiting.title.clone(),
            options: waiting.options.clone(),
            asked_at: waiting.asked_at,
        };
        let mut state = self.state.lock().unwrap();
        state
            .waiting
            .insert(waiting.request.clone(), Held { waiting, responder });
        (self.events)(event);
    }
    fn answer(&self, request: &str, option: &str) -> Result<(), PermissionError> {
        let mut state = self.state.lock().unwrap();
        let held = state.waiting.get(request).ok_or_else(|| {
            if state.settled.contains(request) {
                PermissionError::Settled
            } else {
                PermissionError::Unknown
            }
        })?;
        let choice = held
            .waiting
            .options
            .iter()
            .find(|o| o.option_id == option)
            .ok_or(PermissionError::OptionNotOffered)?;
        let id = choice.option_id.clone();
        let name = choice.name.clone();
        let kind = choice.kind.clone();
        let held = state.waiting.remove(request).unwrap();
        state.settled.insert(request.to_owned());
        (self.events)(HarnessEvent::PermissionAnswered {
            request: request.into(),
            option: option.into(),
            kind,
            name,
        });
        let _ = held.responder.respond(RequestPermissionResponse::new(
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id)),
        ));
        Ok(())
    }
    pub(super) fn refuse_session(&self, session: &str, reason: &str) {
        self.refuse_matching(reason, |w| w.session == session);
    }
    fn refuse_matching(&self, reason: &str, matches: impl Fn(&WaitingPermission) -> bool) {
        let mut state = self.state.lock().unwrap();
        let mut ids: Vec<_> = state
            .waiting
            .iter()
            .filter(|(_, h)| matches(&h.waiting))
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        for id in ids {
            let held = state.waiting.remove(&id).unwrap();
            state.settled.insert(id.clone());
            (self.events)(HarnessEvent::PermissionRefused {
                title: held.waiting.title,
                request: Some(id),
                reason: Some(reason.into()),
            });
            let _ = held.responder.respond(RequestPermissionResponse::new(
                RequestPermissionOutcome::Cancelled,
            ));
        }
    }
}
impl Connection {
    pub fn held_permissions(&self) -> Vec<WaitingPermission> {
        self.permissions
            .state
            .lock()
            .unwrap()
            .waiting
            .values()
            .map(|h| h.waiting.clone())
            .collect()
    }
    pub async fn prepare_permission_answer(
        &self,
        request: &str,
        option: &str,
    ) -> Result<PreparedPermissionAnswer, PermissionError> {
        let guard = self.permissions.settlement.clone().lock_owned().await;
        let state = self.permissions.state.lock().unwrap();
        let held = state.waiting.get(request).ok_or_else(|| {
            if state.settled.contains(request) {
                PermissionError::Settled
            } else {
                PermissionError::Unknown
            }
        })?;
        let choice = held
            .waiting
            .options
            .iter()
            .find(|choice| choice.option_id == option)
            .cloned()
            .ok_or(PermissionError::OptionNotOffered)?;
        Ok(PreparedPermissionAnswer {
            permissions: self.permissions.clone(),
            request: request.into(),
            choice,
            _guard: guard,
        })
    }
    pub async fn answer_permission(
        &self,
        request: &str,
        option: &str,
    ) -> Result<(), PermissionError> {
        self.prepare_permission_answer(request, option)
            .await?
            .commit()
    }
    pub async fn refuse_held(&self, reason: &str) {
        let _guard = self.permissions.settlement.lock().await;
        self.permissions.refuse_matching(reason, |_| true);
    }
}

/// Pins an offered option while its owner durably records the person's command.
/// Dropping it without commit leaves the request waiting.
pub struct PreparedPermissionAnswer {
    permissions: Permissions,
    request: String,
    choice: PermissionOption,
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl PreparedPermissionAnswer {
    pub fn option(&self) -> &PermissionOption {
        &self.choice
    }

    pub fn commit(self) -> Result<(), PermissionError> {
        self.permissions
            .answer(&self.request, &self.choice.option_id)
    }
}
