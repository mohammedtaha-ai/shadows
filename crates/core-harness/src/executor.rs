//! One job: opening the session of an executor, built as the Planner's is —
//! a fixed tool list and its compiled-in instructions, in mode `auto`.
use std::sync::LazyLock;

use super::Harness;
use super::sessions::{OpenError, OpenSession};
use core_grants::TaskScope;
use core_threads::ThreadId;

/// The executor's one file, compiled in: front matter naming its built-in
/// tools, then its instructions. A project does not change it.
const FILE: &str = include_str!("executor.md");

/// `FILE` read once: its tools and its instructions.
static EXECUTOR: LazyLock<(Vec<&'static str>, &'static str)> =
    LazyLock::new(|| parse(FILE).expect("executor.md has front matter naming its tools"));

/// Nothing is asked and nothing refused while the executor works: the check
/// on its reported commit is its one write boundary.
const MODE: &str = shadows_agent::policy::EXECUTOR_MODE;

/// What an executor session is opened for: the task its grant reads. It
/// works in its project's folder, as every session of the project does.
#[derive(Clone)]
pub(crate) struct ExecutorSession {
    pub(crate) scope: TaskScope,
}

impl ExecutorSession {
    /// The body of `executor.md`, trimmed.
    pub(crate) fn instructions() -> &'static str {
        EXECUTOR.1
    }

    /// The built-in tools `executor.md`'s front matter names, in its order.
    pub(crate) fn tools() -> Vec<String> {
        EXECUTOR.0.iter().map(|tool| tool.to_string()).collect()
    }
}

/// Splits a file opened by a `---` line into the comma list of its front
/// matter's `tools:` and the body after the closing `---` line.
fn parse(text: &str) -> Option<(Vec<&str>, &str)> {
    let (opening, rest) = text.split_once('\n')?;
    if opening.trim() != "---" {
        return None;
    }
    let (front, after) = rest.split_once("\n---")?;
    let (closing, body) = after.split_once('\n')?;
    if !closing.trim().is_empty() {
        return None;
    }
    let tools = front
        .lines()
        .find_map(|line| line.trim().strip_prefix("tools:"))?;
    let tools: Vec<&str> = tools
        .split(',')
        .map(str::trim)
        .filter(|tool| !tool.is_empty())
        .collect();
    (!tools.is_empty()).then_some((tools, body.trim()))
}

impl Harness {
    /// The attempt cannot write any more: forgets the thread's executor setup,
    /// so it cannot open again, and closes the session, which revokes its grant.
    pub async fn release_executor(&self, thread: &ThreadId) -> std::io::Result<()> {
        self.sessions.setups.clear_executor(thread);
        self.sessions.terminate(thread).await
    }

    /// Opens the executor session of `thread` in its project's folder: only
    /// its tools and its task's grant, on `model`, in mode `auto`.
    pub async fn open_executor(
        &self,
        thread: &ThreadId,
        grant: TaskScope,
        model: String,
    ) -> Result<OpenSession, OpenError> {
        self.sessions
            .setups
            .expect_executor(thread, ExecutorSession { scope: grant });
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

#[cfg(test)]
mod tests {
    use super::{FILE, parse};

    #[test]
    fn the_executor_reports_commands_and_leaves_its_writes_clean() {
        for text in [
            "you ran (tests, build, lint) with its exit code",
            "Do not paste file contents",
            "not even a change of line endings",
        ] {
            assert!(FILE.contains(text), "{text}");
        }
    }

    #[test]
    fn the_executor_file_parses_and_names_its_eight_tools() {
        let (tools, body) = parse(FILE).expect("executor.md parses");
        assert_eq!(
            tools,
            [
                "Read",
                "Grep",
                "Glob",
                "Edit",
                "Write",
                "MultiEdit",
                "Bash",
                "TodoWrite"
            ]
        );
        assert!(body.starts_with("[Shadows] You are an executor."), "{body}");
        assert!(body.contains("## Tools"), "{body}");
        assert!(body.contains("Never use Python"), "{body}");
        assert!(!body.contains("tools:"), "{body}");
    }

    #[test]
    fn a_file_without_front_matter_or_tools_does_not_parse() {
        assert!(parse("[Shadows] no front matter\n").is_none());
        assert!(parse("---\nmodel: x\n---\nbody\n").is_none());
        assert!(parse("---\ntools: Read\nbody without a closing line\n").is_none());
    }
}
