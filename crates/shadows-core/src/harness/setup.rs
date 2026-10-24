//! Planner opening context and the changes a live session has not heard.
//!
//! A resumed Claude session retains its opening append. Changed instructions
//! therefore travel after the person text, without rebuilding the session.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use sha2::{Digest, Sha256};

use super::Harness;
use crate::error::CoreError;
use crate::grants::GrantId;
use crate::standards::{EffectiveStandards, Standards};
use crate::threads::ThreadId;
use core_db::{Storage, StorageError};
use core_instructions::{Instructions, InstructionsVersion};
use core_projects::ProjectId;
use shadows_agent::acp::{McpServerSpec, SessionSetup};
use shadows_agent::policy;

/// Shadows' instructions to the Planner, compiled in.
const PROMPT: &str = include_str!("prompt.txt");
/// Pre-approves Shadows' tools and nothing else; the grant stays the authority.
const ALLOWED: &str = "mcp__shadows__*";
const CHANGED: &str = "[Shadows] The project's Planner instructions changed. \
                       They replace the project instructions you were given before:";
const REMOVED: &str = "[Shadows] The project's Planner instructions were removed.";
const OURS: &str = "[Shadows] Shadows' instructions for you:";
const STANDARDS: &str = "[Shadows] The project's standards changed. \
    They replace the standards you were given before:";

/// The version `agent_invocation.prompt_version` records: the first 16 hex
/// digits of the SHA-256 of `prompt.txt`, computed once from the compiled text.
impl Harness {
    pub fn prompt_version() -> &'static str {
        static VERSION: OnceLock<String> = OnceLock::new();
        VERSION.get_or_init(|| format!("{:x}", Sha256::digest(PROMPT.as_bytes()))[..16].to_string())
    }
}

/// What the thread's live adapter was opened with.
struct Held {
    grant: Option<GrantId>,
    /// The instructions version its `append` carried.
    instructions: Option<String>,
    standards: Option<String>,
    /// What the session heard before the command turns that skipped their
    /// changes: the next normal turn compares with it, so it carries every
    /// change since, once, in its current form.
    withheld: Option<Heard>,
    /// What this turn compared with, kept until `withhold` learns whether the
    /// turn carried its changes.
    pending: Option<Heard>,
}

/// The versions a session last heard. `None` is unknown, so sent again.
#[derive(Clone)]
struct Heard {
    ours: bool,
    instructions: Option<Option<String>>,
    /// The base standards version and the additions id.
    standards: Option<(Option<i64>, Option<String>)>,
}

/// What the harness knows about a turn's context. `Turns` decides which of it
/// the turn sends.
/// Read failures are applied only when the session needs the corresponding context.
pub(crate) struct InvocationContext {
    pub(crate) latest: Result<Option<super::InvocationVersions>, StorageError>,
    pub(crate) standards: Result<EffectiveStandards, StorageError>,
}

pub(crate) struct ContextFacts {
    /// What changed since the session last heard, withheld changes included.
    pub(crate) changes: Option<String>,
    pub(crate) stage: String,
}

pub(crate) struct Setups {
    storage: Arc<Storage>,
    instructions: Instructions,
    grants: crate::grants::Grants,
    mcp_url: Option<String>,
    held: Mutex<HashMap<ThreadId, Held>>,
}

impl Setups {
    pub(crate) fn new(
        storage: Arc<Storage>,
        instructions: Instructions,
        mcp_url: Option<String>,
        grants: crate::grants::Grants,
    ) -> Self {
        Self {
            storage,
            instructions,
            grants,
            mcp_url,
            held: Mutex::new(HashMap::new()),
        }
    }

    /// Issues the thread's grant (revoking any previous one) and builds the
    /// setup. With no MCP URL (tests that need no MCP) it issues none. On an
    /// error nothing it issued is left live.
    pub(crate) async fn for_opening(
        &self,
        thread: &ThreadId,
        project: &ProjectId,
    ) -> Result<SessionSetup, String> {
        self.forget(thread).await;
        let current = self
            .instructions
            .current(project)
            .await
            .map_err(|e| e.to_string())?;
        let mut snapshot = self
            .storage
            .reader()
            .begin()
            .await
            .map_err(|e| e.to_string())?;
        let additions = Standards::current_in(&mut snapshot, project)
            .await
            .map_err(|e| e.to_string())?;
        snapshot.commit().await.map_err(|e| e.to_string())?;
        let standards = additions.as_ref().map(|v| v.id.clone());
        let effective = EffectiveStandards {
            base: Standards::base().clone(),
            additions,
        };
        let (grant, mcp) = match &self.mcp_url {
            Some(url) => {
                let thread = thread.clone();
                let (grant, token) = self
                    .storage
                    .write_txn(move |conn| {
                        Box::pin(async move {
                            crate::threads::Threads::require_live_in(conn, &thread)
                                .await
                                .map_err(|e| match e {
                                    StorageError::NotFound(_) => StorageError::NotFound("thread"),
                                    other => other,
                                })?;
                            let project =
                                crate::threads::Threads::project_in(conn, &thread).await?;
                            crate::grants::Grants::issue_thread_grant_in(conn, &project, &thread)
                                .await
                        })
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                let spec = McpServerSpec {
                    name: "shadows".into(),
                    url: url.clone(),
                    bearer: token.as_str().to_string(),
                };
                (Some(grant.id), Some(spec))
            }
            None => (None, None),
        };
        let instructions = current.as_ref().map(|v| v.id.clone());
        self.lock().insert(
            thread.clone(),
            Held {
                grant,
                instructions,
                standards,
                withheld: None,
                pending: None,
            },
        );
        let mut append = format!("{}\n\n{}", ours(), Harness::render(&effective));
        if let Some(body) = body(current.as_ref()) {
            append.push_str(&format!("\n\n## Project instructions\n\n{body}"));
        }
        Ok(SessionSetup {
            mcp,
            append: Some(append),
            allowed_tools: vec![ALLOWED.to_string()],
            tools: Some(PLANNER_TOOLS.iter().map(|t| t.to_string()).collect()),
            setting_sources: None,
        })
    }

    /// The facts a turn composes its prompt from, read before it is its
    /// thread's latest.
    pub(crate) async fn context_facts(
        &self,
        thread: &ThreadId,
        invocation: Option<InvocationContext>,
        stage: String,
    ) -> Result<ContextFacts, CoreError> {
        Ok(ContextFacts {
            changes: self.context_before_turn(thread, invocation).await?,
            stage,
        })
    }

    /// Called once the turn has started, which is when the session hears it.
    /// `skipped`: a command went without the turn's changes, so they stay
    /// unheard; otherwise the turn carried them.
    pub(crate) fn withhold(&self, thread: &ThreadId, skipped: bool) {
        if let Some(held) = self.lock().get_mut(thread) {
            let pending = held.pending.take();
            held.withheld = pending.filter(|_| skipped);
        }
    }

    /// Before a turn: the context block of what changed, when the versions
    /// the session last heard differ from the current ones. Versions held
    /// back by command turns come first; else the latest started turn's.
    async fn context_before_turn(
        &self,
        thread: &ThreadId,
        invocation: Option<InvocationContext>,
    ) -> Result<Option<String>, CoreError> {
        let context = crate::threads::Threads::turn_context_in(
            &mut *self
                .storage
                .reader()
                .begin()
                .await
                .map_err(core_db::StorageError::from)?,
            thread,
        )
        .await?;
        if context.harness == policy::AYLUL {
            return Ok(None);
        }
        let invocation = invocation.ok_or(StorageError::NotFound("invocation"))?;
        let withheld = self.lock().get(thread).and_then(|h| h.withheld.clone());
        let heard = match withheld {
            Some(heard) => Some(heard),
            None => match invocation.latest? {
                Some(latest) => Some(Heard {
                    ours: latest.prompt.as_deref() == Some(Harness::prompt_version()),
                    instructions: Some(latest.instructions),
                    standards: Some((latest.standards, latest.additions)),
                }),
                None => self.heard_first(thread, context.fork_session_id.is_some()),
            },
        };
        let Some(heard) = heard else {
            return Ok(None);
        };
        if let Some(held) = self.lock().get_mut(thread) {
            held.pending = Some(heard.clone());
        }
        let current = self.instructions.current(&context.project_id).await?;
        let current_id = current.as_ref().map(|v| v.id.clone());
        let effective = invocation.standards?;
        let additions_id = effective.additions.as_ref().map(|v| v.id.clone());
        let standards_sent = heard.standards == Some((Some(effective.base.version), additions_id));
        let mut parts = Vec::new();
        if !heard.ours {
            parts.push(format!("{OURS}\n\n{}", ours()));
        }
        if heard.instructions.as_ref() != Some(&current_id) {
            parts.push(match body(current.as_ref()) {
                Some(body) => format!("{CHANGED}\n\n{body}"),
                None => REMOVED.to_string(),
            });
        }
        if !standards_sent {
            parts.push(format!("{STANDARDS}\n\n{}", Harness::render(&effective)));
        }
        Ok((!parts.is_empty()).then(|| parts.join("\n\n")))
    }

    /// What a thread with no started turn heard: what its session opened
    /// with, unless that session is a fork's, whose parent holds versions no
    /// turn of this thread recorded. `None` when there is nothing to compare
    /// with.
    fn heard_first(&self, thread: &ThreadId, fork: bool) -> Option<Heard> {
        if fork {
            return Some(Heard {
                ours: false,
                instructions: None,
                standards: None,
            });
        }
        let base = Standards::base().version;
        self.lock().get(thread).map(|h| Heard {
            ours: true,
            instructions: Some(h.instructions.clone()),
            standards: Some((Some(base), h.standards.clone())),
        })
    }

    /// The adapter closed: revoke its grant. A failed revocation is logged;
    /// the next daemon start revokes every thread grant left live.
    pub(crate) async fn forget(&self, thread: &ThreadId) {
        let held = self.lock().remove(thread);
        if let Some(grant) = held.and_then(|h| h.grant)
            && let Err(error) = self.grants.revoke_thread_grant(&grant).await
        {
            tracing::error!(thread_id = %thread, %error, "planner.grant_revoke_failed");
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<ThreadId, Held>> {
        self.held
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Built-in tools permitted in every Planner session.
pub const PLANNER_TOOLS: [&str; 8] = [
    "Read",
    "Grep",
    "Glob",
    "WebSearch",
    "WebFetch",
    "ToolSearch",
    "Agent",
    "TodoWrite",
];

/// `prompt.txt` without its closing newline.
fn ours() -> &'static str {
    PROMPT.trim_end()
}

/// The project's instructions, when it has any: a version saved blank is none.
fn body(current: Option<&InstructionsVersion>) -> Option<&str> {
    current
        .map(|v| v.body.as_str())
        .filter(|b| !b.trim().is_empty())
}
