//! One job: who may do what on `/mcp` (spec §13.7).

use sha2::{Digest, Sha256};

use crate::id::newtype_id;
use crate::project::ProjectId;
use crate::thread::ThreadId;

newtype_id! {
    /// Spec §13.7. A thread grant (the internal Planner) or a project grant
    /// (an external agent); its token is stored only as a hash.
    GrantId
}

/// Whose grant it is: the Planner of one thread, or an external agent bound
/// to one project. On the wire and in storage, `thread` and `project`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum GrantKind {
    Thread,
    Project,
}

impl GrantKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GrantKind::Thread => "thread",
            GrantKind::Project => "project",
        }
    }
}

/// A grant as stored: never its token.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Grant {
    pub id: GrantId,
    pub kind: GrantKind,
    pub project_id: ProjectId,
    /// The Planner's thread; `null` for a project grant.
    pub thread_id: Option<ThreadId>,
    pub created_at: String,
    /// When it stopped being honoured; `null` while it is live.
    pub revoked_at: Option<String>,
}

/// A bearer token in clear. It exists only between issue and the one answer
/// that shows it: storage keeps `hash()`, and `Debug` prints its prefix only,
/// so a logged value never carries it.
pub struct Token(String);

impl Token {
    /// `shd_` and two v4 UUIDs without hyphens: 244 random bits.
    pub fn generate() -> Self {
        Self(format!(
            "shd_{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// What storage keeps in its place.
    pub fn hash(&self) -> String {
        hash_token(&self.0)
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(shd_…)")
    }
}

/// The SHA-256 of a raw token, as lowercase hex: the only form of it stored.
/// A token carries 244 random bits, so an unsalted fast hash is enough — no
/// dictionary reaches it.
pub fn hash_token(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

/// What issuing answers. `token` is `Some` on the first issue only: a replay
/// of the same command answers the grant without it, because it is shown once.
#[derive(Debug)]
pub struct IssuedGrant {
    pub grant: Grant,
    pub token: Option<Token>,
}
