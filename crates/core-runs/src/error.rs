//! Why a run command was refused.
use core_db::StorageError;
use core_plans::TaskParent;

#[derive(Debug, thiserror::Error)]
pub enum RunsError {
    #[error("only an approved plan version can be run")]
    NotFrozen,
    #[error("a newer version of this plan exists")]
    Superseded,
    #[error("the plan is archived")]
    Archived,
    #[error("the task waits for {}", RunsError::named(tasks))]
    Waiting { tasks: Vec<TaskParent> },
    #[error("the task already has an attempt that can still write")]
    TaskBusy,
    #[error("task {task} is running and may write the same files")]
    Overlap { task: u32 },
    #[error("the project folder is not a git repository")]
    NotGit,
    #[error("uncommitted changes inside the task's writes: {}", paths.join(", "))]
    DirtyScope { paths: Vec<String> },
    #[error("the task's required files do not exist: {}", paths.join(", "))]
    MissingInput { paths: Vec<String> },
    #[error("the task's required files cannot be checked: {0}")]
    InputUnavailable(String),
    #[error("this attempt already reported its commit")]
    AlreadyCommitted,
    #[error(
        "`{commit}` is not a commit in this repository; commit your files, then report \
         the commit's full sha"
    )]
    UnknownCommit { commit: String },
    #[error(
        "commit {commit} is not on the branch checked out (not reachable from HEAD); commit \
         on the current branch, without switching it, and report again"
    )]
    NotOnBranch { commit: String },
    #[error(
        "commit {commit} was already on the branch when this attempt started; commit your \
         own work and report that commit"
    )]
    NotNew { commit: String },
    #[error(
        "the commit {commit} changes no file: commit only work; if nothing needs to change, \
         do not commit, and say why in your last message"
    )]
    EmptyCommit { commit: String },
    #[error(
        "the commit changes paths outside the task's writes: {}; take them out of the \
         commit and report again",
        paths.join(", ")
    )]
    OutsideWrites { paths: Vec<String> },
    #[error(
        "the commit adds or changes binary files: {}; take them out of the commit and \
         report again",
        paths.join(", ")
    )]
    BinaryFiles { paths: Vec<String> },
    #[error("the project folder is already inside a git repository")]
    AlreadyGit,
    #[error("the project has no folder")]
    NoFolder,
    #[error("the folder's file listing is not complete yet; call project_git_init again")]
    NotListed,
    #[error(
        "no model to run the executor on: name one the executor's harness (claude-code) offers"
    )]
    NoModel,
    #[error("T{task} has no committed attempt waiting for your judgment")]
    NotAwaiting { task: u32 },
    #[error("an addition says what is missing or must be extended; it cannot be empty")]
    EmptyAddition,
    #[error(
        "T{task} had its 3 additions: it is stopped and goes back to the person; tell them \
         what is still missing; then call task_lessons with what stopped the work and what \
         the plan should have said"
    )]
    AdditionsSpent { task: u32 },
    #[error("T{task} has no record of a stop on the cap")]
    NoLesson { task: u32 },
    #[error("stopped_by and plan_should_have_said must both say something")]
    EmptyLesson,
    #[error("the command id was used for another request")]
    CommandConflict,
    #[error("not found")]
    NotFound,
    #[error("the task's writes are not valid patterns: {0}")]
    BadScope(String),
    #[error("{0}")]
    Unavailable(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl RunsError {
    /// The opaque id clients match on, never the text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFrozen => "not_frozen",
            Self::Superseded => "superseded",
            Self::Archived => "archived",
            Self::Waiting { .. } => "waiting",
            Self::TaskBusy => "task_busy",
            Self::Overlap { .. } => "overlap",
            Self::NotGit => "not_git",
            Self::DirtyScope { .. } => "dirty_scope",
            Self::MissingInput { .. } => "missing_input",
            Self::InputUnavailable(_) => "input_unavailable",
            Self::AlreadyCommitted => "already_committed",
            Self::UnknownCommit { .. } => "unknown_commit",
            Self::NotOnBranch { .. } => "not_on_branch",
            Self::NotNew { .. } => "not_new",
            Self::EmptyCommit { .. } => "empty_commit",
            Self::OutsideWrites { .. } => "outside_writes",
            Self::BinaryFiles { .. } => "binary_files",
            Self::AlreadyGit => "already_git",
            Self::NoFolder => "no_folder",
            Self::NotListed => "not_listed",
            Self::NoModel => "no_model",
            Self::NotAwaiting { .. } => "not_awaiting",
            Self::EmptyAddition => "empty_addition",
            Self::AdditionsSpent { .. } => "additions_spent",
            Self::NoLesson { .. } => "no_lesson",
            Self::EmptyLesson => "empty_lesson",
            Self::CommandConflict => "command_conflict",
            Self::NotFound => "not_found",
            Self::BadScope(_) => "bad_scope",
            Self::Unavailable(_) => "unavailable",
            Self::Storage(_) => "storage",
        }
    }

    /// Whether this is a reason Start does not run now, as opposed to a
    /// request that names nothing or a failure to read.
    pub fn refuses_start(&self) -> bool {
        matches!(
            self,
            Self::NotFrozen
                | Self::Superseded
                | Self::Archived
                | Self::Waiting { .. }
                | Self::TaskBusy
                | Self::Overlap { .. }
                | Self::NotGit
                | Self::DirtyScope { .. }
                | Self::MissingInput { .. }
                | Self::InputUnavailable(_)
                | Self::BadScope(_)
        )
    }

    /// A plan read that failed: an unknown version is not found, anything else
    /// is unavailable.
    pub(crate) fn from_plans(error: core_plans::PlansError) -> Self {
        match error {
            core_plans::PlansError::Storage(StorageError::NotFound(_)) => Self::NotFound,
            other => Self::Unavailable(other.to_string()),
        }
    }

    /// `T1, plan … T2`: the tasks a refusal names, as a person reads them.
    fn named(tasks: &[TaskParent]) -> String {
        tasks
            .iter()
            .map(TaskParent::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl From<core_plans::ExecutionRefusal> for RunsError {
    fn from(refusal: core_plans::ExecutionRefusal) -> Self {
        match refusal {
            core_plans::ExecutionRefusal::Archived => Self::Archived,
            core_plans::ExecutionRefusal::NotFrozen => Self::NotFrozen,
            core_plans::ExecutionRefusal::Superseded => Self::Superseded,
        }
    }
}

impl From<sqlx::Error> for RunsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<serde_json::Error> for RunsError {
    fn from(error: serde_json::Error) -> Self {
        Self::Storage(error.into())
    }
}
