//! One job: the code index types callers meet (spec §15.5).

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
