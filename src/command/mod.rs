use sha2::{Digest, Sha256};

use crate::events::Actor;
use crate::mcp::grant::GrantId;
use crate::thread::ThreadId;

pub mod derive;

/// Spec section 5.2, external write origin: everything needed to decide whether
/// a submission is new, a replay, or a conflict.
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub principal_kind: String,
    pub principal_id: String,
    pub command_id: String,
    pub command_kind: String,
    pub command_schema_ver: i64,
    pub request_fingerprint: String,
}

/// Who writes a plan (spec §13.5), and so the principal its commands are
/// recorded under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Writer {
    /// A person, through the HTTP API.
    Person,
    /// The internal Planner of `thread`, holding `grant`. Its principal is the
    /// thread, not the grant: the grant is replaced whenever an adapter opens,
    /// and a retry after a resume must still find the first result.
    Planner { thread: ThreadId, grant: GrantId },
    /// An external agent holding a project grant.
    External { grant: GrantId },
}

impl Writer {
    /// `(principal_kind, principal_id)` of the command record.
    pub fn principal(&self) -> (&'static str, String) {
        match self {
            Writer::Person => ("User", "local".to_string()),
            Writer::Planner { thread, .. } => ("Thread", thread.to_string()),
            Writer::External { grant } => ("Grant", grant.to_string()),
        }
    }

    /// The actor of the events the writer's commands append: its principal.
    pub fn actor(&self) -> Actor {
        let (kind, id) = self.principal();
        Actor {
            kind: kind.to_string(),
            id,
        }
    }

    /// The grant a write must find valid and in scope (§13.7); none for a person.
    pub fn grant(&self) -> Option<&GrantId> {
        match self {
            Writer::Person => None,
            Writer::Planner { grant, .. } | Writer::External { grant } => Some(grant),
        }
    }
}

/// Canonicalise before hashing, so that key order — which carries no meaning —
/// cannot turn a replay into a conflict. The command kind is mixed in so the
/// same params under a different command are not interchangeable.
pub fn fingerprint(command_kind: &str, params: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(command_kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(canonical(params).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn canonical(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .iter()
                .map(|k| {
                    let key_json = serde_json::to_string(*k).expect("string keys always serialise");
                    format!("{}:{}", key_json, canonical(&map[*k]))
                })
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        serde_json::Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", inner.join(","))
        }
        other => other.to_string(),
    }
}
