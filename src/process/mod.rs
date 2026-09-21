use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::{BufReader, Lines};
use tokio::process::{Child, ChildStdout, Command};

#[cfg(windows)]
mod containment_windows;
#[cfg(windows)]
use containment_windows as containment;

#[cfg(unix)]
mod containment_unix;
#[cfg(unix)]
use containment_unix as containment;

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
    child: Child,
    containment: containment::Containment,
    stdout: Option<Lines<BufReader<ChildStdout>>>,
}

impl ProcessHandle {
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub fn stdout_lines(&mut self) -> Option<&mut Lines<BufReader<ChildStdout>>> {
        self.stdout.as_mut()
    }

    pub async fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }

    /// Terminates the whole managed tree through the containment handle that
    /// owns it. Spec §8.3: never by signalling a PID read from the database,
    /// because the operating system reuses PIDs.
    pub fn terminate_tree(&mut self) -> io::Result<()> {
        self.containment.terminate()
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

    containment::configure(&mut cmd);

    let mut child = cmd.spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| io::Error::other("child exited before a pid could be observed"))?;
    let containment = containment::attach(pid)?;

    let stdout = child.stdout.take().map(|out| {
        use tokio::io::AsyncBufReadExt;
        BufReader::new(out).lines()
    });

    Ok(ProcessHandle {
        child,
        containment,
        stdout,
    })
}
