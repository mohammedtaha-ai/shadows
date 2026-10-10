//! One job: what a Planner session opens with (spec §13.7–§13.8) — the
//! grant its adapter holds, Shadows' instructions with the project's, and
//! the context block that carries what changed since.
//!
//! A Claude session keeps the `append` it was created with for good: a resume
//! restores it and ignores a new one (`MCP_PROBE.md` §3). So `for_opening`
//! sends the current instructions, which reach Claude only when its session is
//! created, and `context_before_turn` sends whatever changed after that as a
//! block after the person's text. A session is never rebuilt for it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use sha2::{Digest, Sha256};

use crate::db::{Storage, StorageError};
use crate::design::{EffectiveStandards, base_standards};
use crate::grants::GrantId;
use crate::instructions::InstructionsVersion;
use crate::projects::ProjectId;
use crate::threads::ThreadId;
use crate::turns::OperationId;
use shadows_agent::acp::{McpServerSpec, SessionSetup};

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
pub fn prompt_version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| format!("{:x}", Sha256::digest(PROMPT.as_bytes()))[..16].to_string())
}

/// What the thread's live adapter was opened with.
struct Held {
    grant: Option<GrantId>,
    /// The instructions version its `append` carried.
    instructions: Option<String>,
    standards: Option<String>,
}

pub(crate) struct Setups {
    storage: Arc<Storage>,
    mcp_url: Option<String>,
    held: Mutex<HashMap<ThreadId, Held>>,
}

impl Setups {
    pub(crate) fn new(storage: Arc<Storage>, mcp_url: Option<String>) -> Self {
        Self {
            storage,
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
            .storage
            .current_planner_instructions(project)
            .await
            .map_err(|e| e.to_string())?;
        let additions = self
            .storage
            .current_standards_additions(project)
            .await
            .map_err(|e| e.to_string())?;
        let standards = additions.as_ref().map(|v| v.id.clone());
        let effective = EffectiveStandards {
            base: base_standards().clone(),
            additions,
        };
        let (grant, mcp) = match &self.mcp_url {
            Some(url) => {
                let (grant, token) = self
                    .storage
                    .issue_thread_grant(thread)
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
            },
        );
        let mut append = format!("{}\n\n{}", ours(), super::standards::render(&effective));
        if let Some(body) = body(current.as_ref()) {
            append.push_str(&format!("\n\n## Project instructions\n\n{body}"));
        }
        Ok(SessionSetup {
            mcp,
            append: Some(append),
            allowed_tools: vec![ALLOWED.to_string()],
        })
    }

    /// Before a turn: §13.8's context block when the versions the thread's
    /// latest invocation recorded differ from the current ones. A thread with
    /// none is compared with what its session opened with, unless that
    /// session is a fork's, whose parent holds versions no invocation of this
    /// thread recorded.
    pub(crate) async fn context_before_turn(
        &self,
        thread: &ThreadId,
        operation: &OperationId,
    ) -> Result<Option<String>, StorageError> {
        let context = self.storage.turn_context(thread).await?;
        let latest = self.storage.latest_invocation_versions(thread).await?;
        let current = self
            .storage
            .current_planner_instructions(&context.project_id)
            .await?;
        let current_id = current.as_ref().map(|v| v.id.clone());
        let additions = self
            .storage
            .standards_additions_for_invocation(operation)
            .await?;
        let additions_id = additions.as_ref().map(|v| v.id.clone());
        let effective = EffectiveStandards {
            base: base_standards().clone(),
            additions,
        };
        // `None` for the instructions sent: unknown, so sent again.
        let (ours_sent, instructions_sent, standards_sent) = match latest {
            Some(latest) => (
                latest.prompt.as_deref() == Some(prompt_version()),
                Some(latest.instructions),
                latest.standards == Some(effective.base.version)
                    && latest.additions == additions_id,
            ),
            None if context.fork_session_id.is_some() => (false, None, false),
            None => {
                let opened = self
                    .lock()
                    .get(thread)
                    .map(|h| (h.instructions.clone(), h.standards.clone()));
                match opened {
                    Some((instructions, standards)) => {
                        (true, Some(instructions), standards == additions_id)
                    }
                    None => return Ok(None),
                }
            }
        };
        let mut parts = Vec::new();
        if !ours_sent {
            parts.push(format!("{OURS}\n\n{}", ours()));
        }
        if instructions_sent.as_ref() != Some(&current_id) {
            parts.push(match body(current.as_ref()) {
                Some(body) => format!("{CHANGED}\n\n{body}"),
                None => REMOVED.to_string(),
            });
        }
        if !standards_sent {
            parts.push(format!(
                "{STANDARDS}\n\n{}",
                super::standards::render(&effective)
            ));
        }
        Ok((!parts.is_empty()).then(|| parts.join("\n\n")))
    }

    /// The stage line every person's message carries (§23.4).
    pub(crate) async fn stage_line(&self, thread: &ThreadId) -> Result<String, StorageError> {
        let context = self.storage.turn_context(thread).await?;
        let view = self.storage.project_stage(&context.project_id).await?;
        Ok(crate::design::stage::line(&view))
    }

    /// The adapter closed: revoke its grant. A failed revocation is logged;
    /// the next daemon start revokes every thread grant left live (§13.7).
    pub(crate) async fn forget(&self, thread: &ThreadId) {
        let held = self.lock().remove(thread);
        if let Some(grant) = held.and_then(|h| h.grant)
            && let Err(error) = self.storage.revoke_thread_grant(&grant).await
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
