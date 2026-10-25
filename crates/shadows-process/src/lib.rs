mod launch;
mod persistent;
pub use persistent::{ProcessIdentity, inspect_process, spawn_persistent};

use std::io;
use std::path::PathBuf;

use process_wrap::tokio::ChildWrapper;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

/// OS-level intent and nothing else. `shadows-process` knows nothing about
/// Role, Claude, Codex, planning, workflows, or verification.
#[derive(Clone)]
pub struct ProcessSpec {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// Built explicitly. The daemon's own environment is never mutated for a
    /// child, and clearing wholesale is not the same as isolating: on Windows a
    /// child that loses SystemRoot, SystemDrive, ComSpec or PATHEXT fails in
    /// ways that never appear on Linux.
    pub env: Vec<(String, String)>,
    pub capture_stdout: bool,
    pub pipe_stdin: bool,
}

/// Names the environment's variables, never their values: a provider key
/// travels here (AYLUL's `ANTHROPIC_AUTH_TOKEN`), and `{:?}` must not print it.
impl std::fmt::Debug for ProcessSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessSpec")
            .field("executable", &self.executable)
            .field("args", &self.args)
            .field("cwd", &self.cwd)
            .field("env", &self.env.iter().map(|(k, _)| k).collect::<Vec<_>>())
            .field("capture_stdout", &self.capture_stdout)
            .field("pipe_stdin", &self.pipe_stdin)
            .finish()
    }
}

pub type ChildIn = ChildStdin;
pub type ChildOut = ChildStdout;
pub type ChildErr = ChildStderr;

/// A captured child's stdout, read line by line. Named here so callers can
/// hold one in a signature without importing `tokio::process` themselves —
/// only `shadows-process` may touch that API (CLAUDE.md), and a type alias is the
/// difference between honouring that boundary and a caller quietly crossing it.
pub type StdoutLines = Lines<BufReader<ChildStdout>>;

pub struct ProcessHandle {
    child: Box<dyn ChildWrapper>,
    stdin: Option<ChildIn>,
    stdout_raw: Option<ChildOut>,
    stderr: Option<ChildErr>,
    stdout: Option<Lines<BufReader<ChildStdout>>>,
    /// Test-only. Termination failure has no reachable test otherwise: on Windows — this
    /// project's acceptance gate — `start_kill` on a live child does not fail
    /// on demand, so the branch that must not claim outcome ownership could
    /// only be reviewed, never exercised. Gated behind `test-support`, which
    /// `cargo test` enables through the self dev-dependency and `cargo build`
    /// never does, so the field does not exist in anything that ships.
    #[cfg(feature = "test-support")]
    termination_fails: bool,
}

impl ProcessHandle {
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Takes ownership of the captured stdout reader, leaving the handle
    /// without one. The stream reader must outlive the call that
    /// registers this handle in `LiveHandles` (a `'static` task reading lines
    /// while the handle itself is moved into a shared map), which an
    /// accessor that only lends `&mut` cannot support.
    pub fn take_stdout_lines(&mut self) -> Option<Lines<BufReader<ChildStdout>>> {
        self.stdout.take()
    }

    /// Takes all stdio handles once for an interactive child protocol.
    pub fn take_stdio(&mut self) -> Option<(ChildIn, ChildOut, ChildErr)> {
        if self.stdin.is_none() || self.stdout_raw.is_none() || self.stderr.is_none() {
            return None;
        }
        Some((
            self.stdin.take()?,
            self.stdout_raw.take()?,
            self.stderr.take()?,
        ))
    }

    /// Completion is the LEADER's exit, and whatever the leader left behind is
    /// killed once it has exited — `waiting_for_the_leader_reaps_any_remaining_grandchild`
    /// is that rule.
    ///
    /// This is why it polls `try_wait` instead of awaiting `child.wait()`:
    /// process-wrap's `wait` returns only once every member of the group or job
    /// has exited, so a harness that leaves a helper running would hang here
    /// forever instead of completing its turn. Do not "simplify" this loop back
    /// into `self.child.wait().await`.
    pub async fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        loop {
            if let Some(status) = self.child.try_wait()? {
                // The leader is reaped; the caller's
                // span supplies the operation and thread.
                tracing::info!(%status, "process.exit");
                match self.child.start_kill() {
                    Ok(()) => return Ok(status),
                    #[cfg(unix)]
                    Err(error) if error.raw_os_error() == Some(libc::ESRCH) => {
                        return Ok(status);
                    }
                    Err(error) => return Err(error),
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    /// Whether the leader has already exited, without waiting for it.
    /// A cancellation that arrives
    /// after the process ended on its own must not claim to have stopped it,
    /// and asking is only useful if it answers immediately.
    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// Terminates the whole managed tree through the containment handle that
    /// owns it. Never by signalling a PID read from the database,
    /// because the operating system reuses PIDs.
    pub fn terminate_tree(&mut self) -> io::Result<()> {
        let pid = self.child.id();
        #[cfg(feature = "test-support")]
        let result = if self.termination_fails {
            Err(io::Error::other(
                "termination failure forced by test support",
            ))
        } else {
            self.child.start_kill()
        };
        #[cfg(not(feature = "test-support"))]
        let result = self.child.start_kill();
        match &result {
            Ok(()) => tracing::info!(pid, "process.terminate"),
            Err(error) => tracing::warn!(pid, %error, "process.terminate_failed"),
        }
        result
    }

    /// Test-only: makes `terminate_tree` report failure so the failure path
    /// can be exercised rather than assumed. See the field's comment.
    #[cfg(feature = "test-support")]
    pub fn force_termination_failure(&mut self) {
        self.termination_fails = true;
    }
}

/// A child's stderr is a pipe, and a pipe nobody reads fills: once its buffer
/// is full the child blocks on its next write and never exits, so a harness
/// that warns at length would hang its turn forever. It is read to its end
/// here. Only each line's length is logged, at debug, keeping a
/// harness's own output out of the log by default, and nothing yet says which
/// of its stderr is safe to keep.
fn drain_stderr(stderr: tokio::process::ChildStderr) {
    use tokio::io::AsyncBufReadExt;
    use tracing::Instrument;
    tokio::spawn(
        async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(bytes = line.len(), "process.stderr");
            }
        }
        .in_current_span(),
    );
}

pub fn spawn(spec: ProcessSpec) -> io::Result<ProcessHandle> {
    let mut wrapped = launch::Launch::managed(&spec);

    // The executable and working directory are
    // logged; the arguments and environment never are — a harness's arguments
    // carry the prompt, and the environment can carry credentials.
    let mut child = wrapped.spawn().inspect_err(|error| {
        tracing::warn!(
            executable = %spec.executable.display(),
            cwd = %spec.cwd.display(),
            %error,
            "process.spawn_failed"
        );
    })?;
    tracing::info!(
        pid = child.id(),
        executable = %spec.executable.display(),
        cwd = %spec.cwd.display(),
        "process.spawn"
    );
    let stdin = child.stdin().take();
    let mut stdout_raw = child.stdout().take();
    let mut stderr = child.stderr().take();
    let stdout = if spec.pipe_stdin && spec.capture_stdout {
        None
    } else {
        stdout_raw.take().map(|out| BufReader::new(out).lines())
    };
    if !(spec.pipe_stdin && spec.capture_stdout)
        && let Some(stderr) = stderr.take()
    {
        drain_stderr(stderr);
    }

    Ok(ProcessHandle {
        child,
        stdin,
        stdout_raw,
        stderr,
        stdout,
        #[cfg(feature = "test-support")]
        termination_fails: false,
    })
}
