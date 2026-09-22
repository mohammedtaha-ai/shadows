use std::path::PathBuf;

use crate::operation::OperationId;
use crate::process::ProcessSpec;

pub mod claude;

/// Frozen at claim time, not read at spawn time. Spec §8.2: reading any of
/// these later would let a configuration change between claim and spawn alter
/// what the durable record says was run.
#[derive(Debug, Clone)]
pub struct AgentInvocation {
    /// Spec §4.1: the operation this invocation belongs to, typed. A public
    /// `String` here is the same hole the newtype sweep closed everywhere
    /// else — it lets outside code hand this field a thread id, a session id,
    /// or any other string and compile.
    pub operation_id: OperationId,
    pub role: String,
    pub model: String,
    pub prompt: String,
    pub cwd: PathBuf,
    /// Present on a resumed turn. Continuity belongs to the harness, not to us.
    pub resume_session_id: Option<String>,
    /// Used on a first turn so the session id is ours to record and resume.
    pub session_id: String,
}

/// The four stream classes. Only `Entry` and `TurnEnd` ever reach storage.
#[derive(Debug, Clone)]
pub enum StreamItem {
    /// Transient. Render only, hundreds per turn, never persisted.
    Delta {
        text: String,
    },
    /// Durable. Complete, final, carries a harness-assigned uuid that becomes
    /// the entry's harness-side identity.
    Entry {
        uuid: String,
        role: String,
        text: String,
    },
    /// Exactly one per turn, always last.
    TurnEnd {
        subtype: String,
        stop_reason: Option<String>,
    },
    /// Diagnostics and UI signal: system/*, rate_limit_event. Never conversation.
    Operational {
        label: String,
        session: Option<String>,
    },
    Unparsed(String),
}

pub trait AgentHarness {
    fn to_process_spec(&self, invocation: &AgentInvocation) -> ProcessSpec;
    fn classify(&self, line: &str) -> StreamItem;
}
