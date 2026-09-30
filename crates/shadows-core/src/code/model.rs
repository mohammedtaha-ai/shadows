//! One job: the code index types callers meet (spec §15.5–§15.6).

/// How a project's index stands (§15.5).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum IndexState {
    Ready,
    /// Files done out of files found, so an agent knows an answer may be incomplete.
    Indexing {
        done: u32,
        found: u32,
    },
    /// Not in the active set: the index is as it was when the project left it.
    Inactive,
    /// A project created before projects owned a folder (migration 0003).
    NoDirectory,
    DirectoryMissing,
}

/// A project's index: its state, its counts and its last update.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ProjectStatus {
    /// The project's slug.
    pub project: String,
    pub state: IndexState,
    /// Files indexed: the skipped ones are counted in `skipped` only.
    pub files: u32,
    /// One per reason, with its count, by reason.
    pub skipped: Vec<Skipped>,
    pub updated_at: Option<String>,
}

/// Files not parsed, for one reason: `too_large`, `binary` or `not_utf8`.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Skipped {
    pub reason: String,
    pub count: u32,
}

/// One tag an answer points at.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Hit {
    /// The project's slug.
    pub project: String,
    /// Relative to the project's folder, with '/'.
    pub path: String,
    pub line: u32,
    pub kind: String,
    pub name: String,
    pub signature: Option<String>,
    /// `Some("name")` on every `references` hit: matched by name only (§15.5).
    pub matched_by: Option<String>,
}

/// What a question answers.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Answer {
    /// At most 50, by project (the asker's first, then its links by slug),
    /// path and line.
    pub hits: Vec<Hit>,
    /// More hits than the 50 answered.
    pub more: bool,
    /// Up to 10 names containing the text, only when `hits` is empty.
    pub suggestions: Vec<String>,
    /// One per project in the scope.
    pub status: Vec<ProjectStatus>,
}

/// How the workers keep the index current (§15.4).
#[derive(Debug, Clone, Copy)]
pub struct CodeConfig {
    /// How often each active project is scanned: the watcher can lose changes.
    pub rescan_every: std::time::Duration,
    /// How long no change has to arrive before the gathered paths are indexed.
    pub debounce: std::time::Duration,
}

impl Default for CodeConfig {
    fn default() -> Self {
        Self {
            rescan_every: std::time::Duration::from_secs(60),
            debounce: std::time::Duration::from_millis(500),
        }
    }
}

/// A project reading another's index (§15.6). One way: `project` reads
/// `linked`, never the reverse.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ProjectLink {
    /// The slug of the project that reads.
    pub project: String,
    /// The slug of the project it reads.
    pub linked: String,
    pub created_at: String,
}

/// The code index's settings (§15.6).
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct CodeSettings {
    /// How many projects are active at once: 1 to 20, default 5.
    pub active_limit: u32,
}
