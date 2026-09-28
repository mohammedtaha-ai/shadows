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

use crate::mcp::grant::GrantId;
use crate::project::ProjectId;
use crate::storage::{InstructionsVersion, Storage, StorageError};
use crate::thread::ThreadId;
use shadows_agent::acp::{McpServerSpec, SessionSetup};

/// Shadows' instructions to the Planner, compiled in.
const PROMPT: &str = include_str!("prompt.txt");
/// Pre-approves Shadows' tools and nothing else; the grant stays the authority.
const ALLOWED: &str = "mcp__shadows__*";
const CHANGED: &str = "[Shadows] The project's Planner instructions changed. \
                       They replace the project instructions you were given before:";
const REMOVED: &str = "[Shadows] The project's Planner instructions were removed.";
const OURS: &str = "[Shadows] Shadows' instructions for you:";

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
            },
        );
        let append = match body(current.as_ref()) {
            Some(body) => format!("{}\n\n## Project instructions\n\n{body}", ours()),
            None => ours().to_string(),
        };
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
    ) -> Result<Option<String>, StorageError> {
        let context = self.storage.turn_context(thread).await?;
        let latest = self.storage.latest_invocation_versions(thread).await?;
        let current = self
            .storage
            .current_planner_instructions(&context.project_id)
            .await?;
        let current_id = current.as_ref().map(|v| v.id.clone());
        // `None` for the instructions sent: unknown, so sent again.
        let (ours_sent, instructions_sent) = match latest {
            Some((prompt, instructions)) => (
                prompt.as_deref() == Some(prompt_version()),
                Some(instructions),
            ),
            None if context.fork_session_id.is_some() => (false, None),
            None => {
                let opened = self.lock().get(thread).map(|h| h.instructions.clone());
                match opened {
                    Some(instructions) => (true, Some(instructions)),
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
        Ok((!parts.is_empty()).then(|| parts.join("\n\n")))
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
