//! One job: the code index types callers meet.

/// How a project's index stands.
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
    /// A project without a stored directory.
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
    /// `Some("name")` on every `references` hit: matched by name only.
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

/// How the workers keep the index current.
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

/// A project reading another's index. One way: `project` reads
/// `linked`, never the reverse.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ProjectLink {
    /// The slug of the project that reads.
    pub project: String,
    /// The slug of the project it reads.
    pub linked: String,
    pub created_at: String,
}

/// The code index's settings.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct CodeSettings {
    /// How many projects are active at once: 1 to 20, default 5.
    pub active_limit: u32,
    /// The least `active_limit` Core accepts.
    pub active_limit_min: u32,
    /// The most `active_limit` Core accepts.
    pub active_limit_max: u32,
}

/// What a listing asks for. Built by an adapter; never deserialized.
#[derive(Debug, Clone)]
pub struct InventoryQuery {
    /// A folder relative to the project's folder; "" for all of it.
    pub prefix: String,
    /// The `next` of the page before this one.
    pub after: Option<String>,
    /// 1 to 200.
    pub limit: u16,
}

impl Default for InventoryQuery {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            after: None,
            limit: 100,
        }
    }
}

/// What one listed path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InventoryKind {
    Directory,
    File,
    Excluded,
}

/// How a project's file list stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InventoryState {
    Missing,
    Indexing,
    Ready,
    Incomplete,
}

/// One path of a project's folder.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct InventoryEntry {
    /// Relative to the project's folder, with '/'. A folder has no trailing '/'.
    pub path: String,
    pub kind: InventoryKind,
    /// The name `shadows-index` gives the file's extension; null for any other file.
    #[schema(required)]
    pub language: Option<String>,
    /// `dependencies`, `ignored`, `secret` or `link`; null unless `kind` is `excluded`.
    #[schema(required)]
    pub excluded_reason: Option<String>,
    /// The code index holds at least one tag of this file.
    pub has_symbols: bool,
}

/// One page of a project's file list, in path order.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct InventoryPage {
    pub state: InventoryState,
    /// Moves by one each time a listing changed the stored entries; 0 before the first.
    pub generation: u64,
    pub entries: Vec<InventoryEntry>,
    /// What to pass as `after` for the following page; null on the last.
    #[schema(required)]
    pub next: Option<String>,
    /// When the last listing ended (RFC 3339); null before the first.
    #[schema(required)]
    pub observed_at: Option<String>,
}
