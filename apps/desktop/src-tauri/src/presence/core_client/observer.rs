//! Async Core requests for observation: metadata checks, snapshots and SSE.
use super::RuntimeIdentity;
use crate::{
    core_connection::ReadyConnection,
    presence::{
        binding::{eligible_agent, BindingView, Thread},
        projection::{Entry, Operation},
        CompanionAppearance,
    },
};
use eventsource_stream::{Event, EventStreamError, Eventsource};
use futures_util::{Stream, StreamExt};
use serde::Deserialize;
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
const JSON_LIMIT: usize = 16 * 1024 * 1024;
const FRAME_LIMIT: usize = 1024 * 1024;
#[derive(Deserialize)]
struct Agent {
    slug: String,
    provider: String,
    deleted: bool,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    character_color: Option<String>,
    #[serde(default)]
    outfit: Option<String>,
}
#[derive(Deserialize)]
struct Workspace {
    project_id: String,
    agents: Vec<Agent>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    Transport,
    Missing,
    Invalid,
    Changed,
    Agent,
    Provider,
    Key,
    Thread,
}
impl Problem {
    pub fn status(self) -> &'static str {
        match self {
            Self::Transport => "Reconnecting",
            Self::Missing | Self::Invalid => "Unavailable. Retry local Core in Settings.",
            Self::Changed => "Core changed. Retry local Core in Settings.",
            Self::Agent => "Agent unavailable. Choose an agent.",
            Self::Provider => "Provider unavailable. Check Settings.",
            Self::Key => "Add a provider key in Settings.",
            Self::Thread => "Conversation unavailable. Choose another conversation.",
        }
    }
    pub fn terminal(self) -> bool {
        matches!(self, Self::Missing | Self::Invalid | Self::Changed)
    }
}
pub struct Client {
    ready: ReadyConnection,
    http: reqwest::Client,
    timeout: Duration,
}
pub struct Snapshot {
    pub operations: Vec<Operation>,
    pub entries: Vec<Entry>,
}
type Frames = Pin<Box<dyn Stream<Item = Result<Event, Problem>> + Send>>;
pub struct Feed {
    frames: Frames,
    budget: Arc<AtomicUsize>,
}
impl Feed {
    pub async fn next(&mut self) -> Option<Result<Event, Problem>> {
        let result = self.frames.next().await;
        self.budget.store(0, Ordering::SeqCst);
        result
    }
}
impl Client {
    pub fn new(ready: ReadyConnection, timeout: Duration) -> Result<Self, Problem> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(timeout)
            .build()
            .map_err(|_| Problem::Transport)?;
        Ok(Self {
            ready,
            http,
            timeout,
        })
    }
    async fn read<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Problem> {
        let response = self
            .http
            .get(format!("{}{path}", self.ready.origin))
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|_| Problem::Transport)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Problem::Missing);
        }
        if !response.status().is_success() {
            return Err(Problem::Transport);
        }
        if response
            .content_length()
            .is_some_and(|size| size > JSON_LIMIT as u64)
        {
            return Err(Problem::Invalid);
        }
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| Problem::Transport)?;
            if bytes.len().saturating_add(chunk.len()) > JSON_LIMIT {
                return Err(Problem::Invalid);
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| Problem::Invalid)
    }
    #[allow(
        dead_code,
        reason = "Preserve the unit-result validation interface for native callers."
    )]
    pub async fn validate(&self, binding: &BindingView) -> Result<(), Problem> {
        self.validate_appearance(binding).await.map(|_| ())
    }
    pub async fn validate_appearance(
        &self,
        binding: &BindingView,
    ) -> Result<Option<CompanionAppearance>, Problem> {
        tokio::time::timeout(self.timeout, self.validate_inner(binding))
            .await
            .map_err(|_| Problem::Transport)?
    }
    async fn workspace(&self, binding: &BindingView) -> Result<Workspace, Problem> {
        let identity: RuntimeIdentity = self.read("/api/runtime").await.map_err(|problem| {
            if problem == Problem::Missing {
                Problem::Changed
            } else {
                problem
            }
        })?;
        if identity.runtime_id != self.ready.runtime_id
            || identity.configuration_id != self.ready.configuration_id
            || binding.configuration_id.as_deref() != Some(&identity.configuration_id)
        {
            return Err(Problem::Changed);
        }
        self.read("/api/aylul/agents").await.map_err(|problem| {
            if problem == Problem::Missing {
                Problem::Agent
            } else {
                problem
            }
        })
    }
    pub async fn panel_agents(
        &self,
        binding: &BindingView,
    ) -> Result<Vec<crate::presence::PanelAgent>, Problem> {
        let workspace = tokio::time::timeout(self.timeout, self.workspace(binding))
            .await
            .map_err(|_| Problem::Transport)??;
        Ok(workspace
            .agents
            .into_iter()
            .filter(|agent| !agent.deleted)
            .take(64)
            .map(|agent| crate::presence::PanelAgent {
                bound: binding.agent.as_deref() == Some(agent.slug.as_str()),
                name: if agent.name.is_empty() {
                    agent.slug.clone()
                } else {
                    agent.name.chars().take(128).collect()
                },
                description: agent.description.chars().take(200).collect(),
                color: agent.character_color.unwrap_or_else(|| "purple".into()),
                outfit: agent.outfit.unwrap_or_else(|| "default".into()),
                slug: agent.slug,
            })
            .collect())
    }
    async fn validate_inner(
        &self,
        binding: &BindingView,
    ) -> Result<Option<CompanionAppearance>, Problem> {
        let workspace = self.workspace(binding).await?;
        let agent = workspace
            .agents
            .iter()
            .find(|agent| {
                binding
                    .agent
                    .as_deref()
                    .is_some_and(|bound| eligible_agent(&agent.slug, agent.deleted, bound))
            })
            .ok_or(Problem::Agent)?;
        #[derive(Deserialize)]
        struct Provider {
            slug: String,
            key_set: bool,
        }
        let providers: Vec<Provider> =
            self.read("/api/aylul/providers").await.map_err(|problem| {
                if problem == Problem::Missing {
                    Problem::Provider
                } else {
                    problem
                }
            })?;
        let provider = providers
            .iter()
            .find(|provider| provider.slug == agent.provider)
            .ok_or(Problem::Provider)?;
        if !provider.key_set {
            return Err(Problem::Key);
        }
        if let Some(target) = &binding.target {
            uuid::Uuid::parse_str(&target.thread_id).map_err(|_| Problem::Thread)?;
            let thread: Thread = self
                .read(&format!("/api/threads/{}", target.thread_id))
                .await
                .map_err(|problem| {
                    if problem == Problem::Missing {
                        Problem::Thread
                    } else {
                        problem
                    }
                })?;
            if !thread.eligible(&target.thread_id, &workspace.project_id, &agent.slug)
                || thread.project_id != target.project_id
            {
                return Err(Problem::Thread);
            }
        }
        Ok(agent
            .character_color
            .clone()
            .map(|color| CompanionAppearance {
                color,
                outfit: agent.outfit.clone(),
            }))
    }
    pub async fn snapshot(&self, thread: &str) -> Result<Snapshot, Problem> {
        tokio::time::timeout(self.timeout, self.snapshot_inner(thread))
            .await
            .map_err(|_| Problem::Transport)?
    }
    async fn snapshot_inner(&self, thread: &str) -> Result<Snapshot, Problem> {
        uuid::Uuid::parse_str(thread).map_err(|_| Problem::Invalid)?;
        let operations = self
            .read(&format!("/api/threads/{thread}/operations"))
            .await?;
        let entries = self.read(&format!("/api/threads/{thread}/entries")).await?;
        Ok(Snapshot {
            operations,
            entries,
        })
    }
    pub async fn subscribe(&self, thread: &str, after: i64) -> Result<Feed, Problem> {
        uuid::Uuid::parse_str(thread).map_err(|_| Problem::Invalid)?;
        let request = self
            .http
            .get(format!("{}/api/subscribe", self.ready.origin))
            .query(&[
                ("thread_id", thread.to_owned()),
                ("after", after.to_string()),
            ]);
        // Only the header handshake has a deadline; a healthy SSE body has none.
        let response = tokio::time::timeout(self.timeout, request.send())
            .await
            .map_err(|_| Problem::Transport)?
            .map_err(|_| Problem::Transport)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Problem::Missing);
        }
        if !response.status().is_success() {
            return Err(Problem::Transport);
        }
        if !response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"))
        {
            return Err(Problem::Invalid);
        }
        let budget = Arc::new(AtomicUsize::new(0));
        let count = budget.clone();
        let oversized = Arc::new(AtomicBool::new(false));
        let exceeded = oversized.clone();
        let bytes = response.bytes_stream().map(move |chunk| {
            let chunk = chunk.map_err(|_| std::io::Error::other("stream unavailable"))?;
            if count
                .fetch_add(chunk.len(), Ordering::SeqCst)
                .saturating_add(chunk.len())
                > FRAME_LIMIT
            {
                exceeded.store(true, Ordering::SeqCst);
                return Err(std::io::Error::other("frame limit"));
            }
            Ok(chunk)
        });
        let frames = bytes.eventsource().map(move |event| match event {
            Ok(event)
                if event.data.len() <= FRAME_LIMIT
                    && event.event.len() <= 1024
                    && event.id.len() <= 1024 =>
            {
                Ok(event)
            }
            Ok(_) => Err(Problem::Invalid),
            Err(EventStreamError::Transport(_)) if !oversized.load(Ordering::SeqCst) => {
                Err(Problem::Transport)
            }
            Err(_) => Err(Problem::Invalid),
        });
        Ok(Feed {
            frames: Box::pin(frames),
            budget,
        })
    }
}
