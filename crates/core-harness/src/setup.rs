//! Planner opening context and the changes a live session has not heard.
//!
//! A resumed Claude session retains its opening append. Changed instructions
//! therefore travel after the person text, without rebuilding the session.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use sha2::{Digest, Sha256};

use super::Harness;
use crate::error::HarnessError;
use crate::executor::ExecutorSession;
use core_db::{Storage, StorageError};
use core_grants::GrantId;
use core_instructions::{Instructions, InstructionsVersion};
use core_projects::{ProjectId, Projects};
use core_standards::{EffectiveStandards, Standards};
use core_threads::{ThreadId, ThreadKind};
use shadows_agent::connection::{McpServerSpec, SessionSetup};
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

/// Whether a thread's session holds its parent's prompt rather than the one
/// it opened with. A Claude fork keeps its parent's appended prompt, so its
/// first turn owes every version. Codex's `thread/fork` takes the developer
/// instructions it is opened with, so a Codex fork heard its own opening, as a
/// new thread does: sending them again would bury the person's text under them.
impl Harness {
    pub(crate) fn keeps_parent_prompt(harness: &str, forked: bool) -> bool {
        forked && harness != policy::CODEX
    }
}

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
pub struct InvocationContext {
    pub latest: Result<Option<super::InvocationVersions>, StorageError>,
    pub standards: Result<EffectiveStandards, StorageError>,
}

pub struct ContextFacts {
    /// What changed since the session last heard, withheld changes included.
    pub changes: Option<String>,
    pub stage: String,
}

pub(crate) struct Setups {
    storage: Arc<Storage>,
    instructions: Instructions,
    grants: core_grants::Grants,
    mcp_url: Option<String>,
    /// The library a Planner's switched-on skills and servers come from.
    pub(super) extensions: Arc<core_extensions::Extensions>,
    /// Where the library lives and Planner skills folders are written; `None`
    /// without AYLUL, when a Planner gets no library item.
    pub(super) aylul_root: Option<PathBuf>,
    held: Mutex<HashMap<ThreadId, Held>>,
    /// Threads that are executor sessions: what their next opening is built from.
    executors: Mutex<HashMap<ThreadId, ExecutorSession>>,
}

impl Setups {
    pub(crate) fn new(
        storage: Arc<Storage>,
        instructions: Instructions,
        mcp_url: Option<String>,
        grants: core_grants::Grants,
        extensions: Arc<core_extensions::Extensions>,
        aylul_root: Option<PathBuf>,
    ) -> Self {
        Self {
            storage,
            instructions,
            grants,
            mcp_url,
            extensions,
            aylul_root,
            held: Mutex::new(HashMap::new()),
            executors: Mutex::new(HashMap::new()),
        }
    }

    /// `thread` stops being an executor session: its next opening is an ordinary one.
    pub(crate) fn clear_executor(&self, thread: &ThreadId) {
        self.executors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(thread);
    }

    /// The next opening of `thread` is an executor session.
    pub(crate) fn expect_executor(&self, thread: &ThreadId, session: ExecutorSession) {
        self.executors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(thread.clone(), session);
    }

    /// Issues the thread's grant (revoking any previous one) and builds the
    /// setup. With no MCP URL (tests that need no MCP) it issues none. On an
    /// error nothing it issued is left live. An executor thread opens only for
    /// its attempt: opened any other way it would get a Planner's setup.
    pub(crate) async fn for_opening(
        &self,
        thread: &ThreadId,
        project: &ProjectId,
        kind: ThreadKind,
        harness: &str,
        cwd: &Path,
    ) -> Result<SessionSetup, String> {
        self.forget(thread).await;
        let executor = self
            .executors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(thread)
            .cloned();
        match (kind, executor) {
            (ThreadKind::Executor, Some(executor)) => {
                return self.for_executor(thread, project, executor, harness).await;
            }
            (ThreadKind::Executor, None) => {
                return Err("an executor thread opens only for its attempt".into());
            }
            (ThreadKind::Planning, _) => {}
        }
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
        let claude = harness == policy::CLAUDE_CODE;
        let reads_claude_md = claude
            && Projects::planner_reads_claude_md_in(&mut snapshot, project)
                .await
                .map_err(|e| e.to_string())?;
        snapshot.commit().await.map_err(|e| e.to_string())?;
        let standards = additions.as_ref().map(|v| v.id.clone());
        let effective = EffectiveStandards {
            base: Standards::base().clone(),
            additions,
        };
        let switched = match &self.aylul_root {
            Some(root) if claude => Some(
                self.switched_on(root, thread, project)
                    .await
                    .map_err(|e| e.to_string())?,
            ),
            _ => None,
        };
        let (grant, mut mcp) = match &self.mcp_url {
            Some(url) => {
                let thread = thread.clone();
                let (grant, token) = self
                    .storage
                    .write_txn(move |conn| {
                        Box::pin(async move {
                            core_threads::Threads::require_live_in(conn, &thread)
                                .await
                                .map_err(|e| match e {
                                    StorageError::NotFound(_) => StorageError::NotFound("thread"),
                                    other => other,
                                })?;
                            let project = core_threads::Threads::project_in(conn, &thread).await?;
                            core_grants::Grants::issue_thread_grant_in(conn, &project, &thread)
                                .await
                        })
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                let spec = McpServerSpec::Http {
                    name: "shadows".into(),
                    url: url.clone(),
                    headers: vec![("Authorization".into(), format!("Bearer {}", token.as_str()))],
                };
                (Some(grant.id), vec![spec])
            }
            None => (None, vec![]),
        };
        let mut tools: Vec<String> = PLANNER_TOOLS.iter().map(|t| t.to_string()).collect();
        let (mut plugins, mut skills) = (vec![], None);
        if let Some(switched) = switched {
            mcp.extend(switched.mcp);
            if let Some(dir) = switched.folder {
                plugins.push(dir);
                tools.push("Skill".to_string());
                skills = Some(switched.skills);
            }
        }
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
        if reads_claude_md && let Some(text) = Self::claude_md(project, cwd) {
            append.push_str(&format!("\n\n## The project's CLAUDE.md\n\n{text}"));
        }
        Ok(SessionSetup {
            mcp,
            append: Some(append),
            allowed_tools: vec![ALLOWED.to_string()],
            tools: Some(tools),
            setting_sources: None,
            permissions: vec![],
            plugins,
            skills,
        })
    }

    /// The executor's setup: its task's grant, its tools and its instructions,
    /// with no permission rules, since it runs in `auto`. On an error
    /// nothing it issued is left live.
    async fn for_executor(
        &self,
        thread: &ThreadId,
        project: &ProjectId,
        executor: ExecutorSession,
        harness: &str,
    ) -> Result<SessionSetup, String> {
        let (grant, mcp) = match &self.mcp_url {
            Some(url) => {
                let (thread, project) = (thread.clone(), project.clone());
                let scope = executor.scope.clone();
                let (grant, token) = self
                    .storage
                    .write_txn(move |conn| {
                        Box::pin(async move {
                            core_threads::Threads::require_live_in(conn, &thread).await?;
                            core_grants::Grants::issue_executor_grant_in(
                                conn, &project, &thread, scope,
                            )
                            .await
                        })
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                let spec = McpServerSpec::Http {
                    name: "shadows".into(),
                    url: url.clone(),
                    headers: vec![("Authorization".into(), format!("Bearer {}", token.as_str()))],
                };
                (Some(grant.id), vec![spec])
            }
            None => (None, vec![]),
        };
        self.lock().insert(
            thread.clone(),
            Held {
                grant,
                instructions: None,
                standards: None,
                withheld: None,
                pending: None,
            },
        );
        Ok(SessionSetup {
            mcp,
            append: Some(ExecutorSession::instructions(harness)),
            allowed_tools: vec![ALLOWED.to_string()],
            tools: ExecutorSession::tools(harness),
            setting_sources: None,
            permissions: vec![],
            plugins: vec![],
            skills: None,
        })
    }

    /// The facts a turn composes its prompt from, read before it is its
    /// thread's latest.
    pub(crate) async fn context_facts(
        &self,
        thread: &ThreadId,
        invocation: Option<InvocationContext>,
        stage: String,
    ) -> Result<ContextFacts, HarnessError> {
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

    /// The session lost what it heard: compare the next turn with what it
    /// opened with. A thread with no held setup has no live session.
    pub(crate) fn reset(&self, thread: &ThreadId, parent_prompt: bool) {
        let opened = self.heard_first(thread, parent_prompt);
        if let Some(held) = self.lock().get_mut(thread) {
            held.withheld = opened;
        }
    }

    /// Before a turn: the context block of what changed, when the versions
    /// the session last heard differ from the current ones. Versions held
    /// back by command turns come first; else the latest started turn's.
    async fn context_before_turn(
        &self,
        thread: &ThreadId,
        invocation: Option<InvocationContext>,
    ) -> Result<Option<String>, HarnessError> {
        let context = core_threads::Threads::turn_context_in(
            &mut *self
                .storage
                .reader()
                .begin()
                .await
                .map_err(core_db::StorageError::from)?,
            thread,
        )
        .await?;
        // An executor's prompts carry no Planner instructions or standards, so
        // nothing changes for it between turns.
        if context.harness == policy::AYLUL || context.kind == ThreadKind::Executor {
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
                None => self.heard_first(
                    thread,
                    Harness::keeps_parent_prompt(
                        &context.harness,
                        context.fork_session_id.is_some(),
                    ),
                ),
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
    /// with, unless that session kept its parent's prompt, which holds
    /// versions no turn of this thread recorded. `None` when there is nothing
    /// to compare with.
    fn heard_first(&self, thread: &ThreadId, parent_prompt: bool) -> Option<Heard> {
        if parent_prompt {
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
        self.remove_skills(thread);
        let held = self.lock().remove(thread);
        if let Some(grant) = held.and_then(|h| h.grant)
            && let Err(error) = self.grants.revoke_thread_grant(&grant).await
        {
            tracing::error!(thread_id = %thread, %error, "planner.grant_revoke_failed");
        }
    }

    /// The project's switched-on MCP servers, and its switched-on skills
    /// written to the thread's skills folder. The library snapshot is held
    /// until the folder is written, so no change lands halfway.
    async fn switched_on(
        &self,
        root: &Path,
        thread: &ThreadId,
        project: &ProjectId,
    ) -> Result<Switched, HarnessError> {
        let snapshot = self.extensions.project_snapshot(project).await?;
        let skills: Vec<_> = snapshot
            .library
            .skills
            .iter()
            .filter(|i| i.enabled)
            .map(|i| i.item.clone())
            .collect();
        let folder = self.write_skills(root, thread, &skills)?;
        let mcp = snapshot
            .mcp
            .into_iter()
            .map(super::profile_home::Homes::mcp_spec)
            .collect();
        Ok(Switched {
            mcp,
            folder,
            skills: skills
                .iter()
                .map(|i| format!("shadows-skills:{}", i.name))
                .collect(),
        })
    }

    /// The project's `CLAUDE.md` in the session's directory: a regular file,
    /// not a symlink, of at most 64 KiB, in UTF-8. Anything else is skipped
    /// and logged with its reason.
    fn claude_md(project: &ProjectId, cwd: &Path) -> Option<String> {
        let path = cwd.join("CLAUDE.md");
        let read = || -> Result<String, &'static str> {
            let meta = std::fs::symlink_metadata(&path).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => "missing",
                _ => "unreadable",
            })?;
            if !meta.file_type().is_file() {
                return Err("not_a_file");
            }
            if meta.len() > CLAUDE_MD_LIMIT {
                return Err("too_large");
            }
            let file = std::fs::File::open(&path).map_err(|_| "unreadable")?;
            let mut bytes = Vec::new();
            file.take(CLAUDE_MD_LIMIT + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "unreadable")?;
            if bytes.len() as u64 > CLAUDE_MD_LIMIT {
                return Err("too_large");
            }
            String::from_utf8(bytes).map_err(|_| "not_utf8")
        };
        match read() {
            Ok(text) => Some(text),
            Err(reason) => {
                tracing::info!(project_id = %project, reason, "planner.claude_md_skipped");
                None
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<ThreadId, Held>> {
        self.held
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// The largest `CLAUDE.md` a Planner opening reads, in bytes.
const CLAUDE_MD_LIMIT: u64 = 65_536;

/// What a project switched on for its Planner, ready for the setup.
struct Switched {
    mcp: Vec<McpServerSpec>,
    /// The skills folder, when the project switched on any skill.
    folder: Option<String>,
    /// The skills by their plugin-qualified names.
    skills: Vec<String>,
}

/// Built-in tools permitted in every Planner session. `Bash` is its shell, for
/// installing tools and preparing the environment; what its program asks about
/// waits for the person.
pub const PLANNER_TOOLS: [&str; 9] = [
    "Read",
    "Grep",
    "Glob",
    "WebSearch",
    "WebFetch",
    "ToolSearch",
    "Agent",
    "TodoWrite",
    "Bash",
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
