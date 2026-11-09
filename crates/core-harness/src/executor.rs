//! One job: opening the session of an executor, built as the Planner's is —
//! a fixed tool list and the permission rules of its one task.
use super::Harness;
use super::sessions::{OpenError, OpenSession};
use core_grants::TaskScope;
use core_threads::ThreadId;

/// Instructions to every executor, compiled in.
const INSTRUCTIONS: &str = include_str!("executor.txt");

/// Built-in tools permitted in every executor session.
pub const EXECUTOR_TOOLS: [&str; 8] = [
    "Read",
    "Grep",
    "Glob",
    "Edit",
    "Write",
    "MultiEdit",
    "Bash",
    "TodoWrite",
];

/// The mode in which a request outside the rules reaches Shadows: `auto` and
/// `acceptEdits` ask nothing, so nothing could be refused.
const MODE: &str = shadows_agent::policy::EXECUTOR_MODE;

/// What an executor session is opened for: the paths it may write and the
/// task its grant reads. It works in its project's folder, as every session
/// of the project does.
#[derive(Clone)]
pub(crate) struct ExecutorSession {
    pub(crate) writes: Vec<String>,
    pub(crate) scope: TaskScope,
}

impl ExecutorSession {
    /// `executor.txt` without its closing newline.
    pub(crate) fn instructions() -> &'static str {
        INSTRUCTIONS.trim_end()
    }

    /// The Claude Code rules allowed with no question: reading, the shell and
    /// the todo list anywhere, and file edits on each pattern the task writes.
    pub(crate) fn permissions(&self) -> Vec<String> {
        let mut rules: Vec<String> = ["Read", "Grep", "Glob", "TodoWrite", "Bash"]
            .iter()
            .map(|rule| rule.to_string())
            .collect();
        for pattern in &self.writes {
            for tool in ["Edit", "Write", "MultiEdit"] {
                rules.push(format!("{tool}({pattern})"));
            }
        }
        rules
    }
}

impl Harness {
    /// The attempt cannot write any more: forgets the thread's executor setup,
    /// so it cannot open again, and closes the session, which revokes its grant.
    pub async fn release_executor(&self, thread: &ThreadId) -> std::io::Result<()> {
        self.sessions.setups.clear_executor(thread);
        self.sessions.terminate(thread).await
    }

    /// Opens the executor session of `thread` in its project's folder: only
    /// its task's tools and rules, on `model`, in mode `default`.
    pub async fn open_executor(
        &self,
        thread: &ThreadId,
        writes: &[String],
        grant: TaskScope,
        model: String,
    ) -> Result<OpenSession, OpenError> {
        self.sessions.setups.expect_executor(
            thread,
            ExecutorSession {
                writes: writes.to_vec(),
                scope: grant,
            },
        );
        let opened = self.sessions.open(thread).await?;
        let refused = |what: &str, error: String| OpenError::Start(format!("{what}: {error}"));
        let offered = self
            .sessions
            .offered(thread)
            .await
            .ok_or_else(|| OpenError::Start("the harness session closed".into()))?;
        if !offered.offers_model(&model) {
            return Err(refused("model", format!("{model} is not offered")));
        }
        let offered = if offered.current.model == model {
            offered
        } else {
            let id = offered.ids.model.clone();
            self.sessions
                .set_option(thread, &opened, &id, &model)
                .await
                .map_err(|error| refused("model", error.to_string()))?
        };
        self.sessions
            .set_mode(thread, &opened, offered, MODE)
            .await
            .map_err(|error| refused("mode", error.to_string()))?;
        Ok(opened)
    }
}
