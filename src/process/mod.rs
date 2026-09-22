use std::io;
use std::path::PathBuf;
use std::process::Stdio;

#[cfg(windows)]
use process_wrap::tokio::JobObject;
#[cfg(unix)]
use process_wrap::tokio::ProcessSession;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use tokio::io::{BufReader, Lines};
use tokio::process::{ChildStdout, Command};

/// OS-level intent and nothing else. Spec §1.5: `process/` knows nothing about
/// Role, Claude, Codex, planning, workflows, or verification.
#[derive(Debug, Clone)]
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
}

pub struct ProcessHandle {
    child: Box<dyn ChildWrapper>,
    stdout: Option<Lines<BufReader<ChildStdout>>>,
    /// Test-only. Spec §8.4 case 6 ("termination fails or cannot be
    /// confirmed") has no reachable test otherwise: on Windows — this
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
    /// without one. Spec §8.3's stream reader must outlive the call that
    /// registers this handle in `LiveHandles` (a `'static` task reading lines
    /// while the handle itself is moved into a shared map), which an
    /// accessor that only lends `&mut` cannot support — see Task 7's deferred
    /// minor M2.
    pub fn take_stdout_lines(&mut self) -> Option<Lines<BufReader<ChildStdout>>> {
        self.stdout.take()
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

    /// Whether the leader has already exited, without waiting for it. Spec
    /// §8.4 case 4 needs this fact and not a wait: a cancellation that arrives
    /// after the process ended on its own must not claim to have stopped it,
    /// and asking is only useful if it answers immediately.
    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// Terminates the whole managed tree through the containment handle that
    /// owns it. Spec §8.3: never by signalling a PID read from the database,
    /// because the operating system reuses PIDs.
    pub fn terminate_tree(&mut self) -> io::Result<()> {
        #[cfg(feature = "test-support")]
        if self.termination_fails {
            return Err(io::Error::other(
                "termination failure forced by test support",
            ));
        }
        self.child.start_kill()
    }

    /// Test-only: makes `terminate_tree` report failure so spec §8.4 case 6
    /// can be exercised rather than assumed. See the field's comment.
    #[cfg(feature = "test-support")]
    pub fn force_termination_failure(&mut self) {
        self.termination_fails = true;
    }
}

/// Windows environment variables a child needs even under an otherwise
/// explicit environment. Losing any of these breaks the child in ways that
/// never reproduce on Linux.
#[cfg(windows)]
const WINDOWS_ESSENTIAL_ENV: &[&str] = &[
    "SystemRoot",
    "SystemDrive",
    "ComSpec",
    "PATHEXT",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "APPDATA",
];

pub fn spawn(spec: ProcessSpec) -> io::Result<ProcessHandle> {
    let mut cmd = Command::new(&spec.executable);
    cmd.args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        // Spec §1.5: stdin is closed unless the harness contract requires
        // streaming input.
        .stdin(Stdio::null())
        .stdout(if spec.capture_stdout {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::piped());

    #[cfg(windows)]
    for key in WINDOWS_ESSENTIAL_ENV {
        if let Ok(value) = std::env::var(key) {
            cmd.env(key, value);
        }
    }
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }

    let mut wrapped = CommandWrap::from(cmd);
    // Load-bearing on Windows, not a convenience. `JobObject` asks
    // `make_job_object` for `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` only when a
    // `KillOnDrop` wrapper is registered alongside it, and that flag IS §1.5's
    // kill-on-owner-close guarantee: without it the harness outlives a killed
    // daemon. Measured 2026-09-22: removing this line leaves every containment
    // test green and `clippy -D warnings` clean, so nothing here would tell you.
    wrapped.wrap(KillOnDrop);
    #[cfg(windows)]
    wrapped.wrap(JobObject);
    #[cfg(unix)]
    wrapped.wrap(ProcessSession);

    let mut child = wrapped.spawn()?;
    let stdout = child.stdout().take().map(|out| {
        use tokio::io::AsyncBufReadExt;
        BufReader::new(out).lines()
    });

    Ok(ProcessHandle {
        child,
        stdout,
        #[cfg(feature = "test-support")]
        termination_fails: false,
    })
}
