## Task 7: The managed process primitive and process-tree containment

**Files:**
- Create: `src/process/mod.rs`, `src/process/containment_windows.rs`, `src/process/containment_unix.rs`
- Create: `src/bin/tree_probe.rs` (test-support binary; see note below)
- Modify: `src/lib.rs`, `Cargo.toml`
- Test: `tests/containment.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks. `process/` knows nothing about Role, Claude, threads, or operations — spec §1.5.
- Produces: `process::ProcessSpec { executable: PathBuf, args: Vec<String>, cwd: PathBuf, env: Vec<(String, String)>, capture_stdout: bool }`; `process::ProcessHandle` with `stdout_lines(&mut self) -> Option<Lines<BufReader<ChildStdout>>>`, `wait(&mut self) -> io::Result<ExitStatus>`, `terminate_tree(&mut self) -> io::Result<()>`, `id(&self) -> Option<u32>`; `process::spawn(spec: ProcessSpec) -> io::Result<ProcessHandle>`.

**Why a test-support binary.** The containment probe in spec §3.7 Layer 4 requires a real `daemon -> child -> grandchild` hierarchy; a direct-child-only test is explicitly insufficient. `tree_probe` spawns a grandchild and then sleeps, so the test has a genuine three-level tree to kill. It is declared as a `[[bin]]` so integration tests can find it through `CARGO_BIN_EXE_tree_probe`. It contains no product logic and ships as part of the test apparatus.

- [ ] **Step 1: Write the failing test**

`tests/containment.rs`:

```rust
use std::time::Duration;

use shadows::process::{spawn, ProcessSpec};

#[cfg(windows)]
fn is_alive(pid: u32) -> bool {
    use std::process::Command;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command",
               &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ 'yes' }} else {{ 'no' }}")])
        .output()
        .expect("powershell should run");
    String::from_utf8_lossy(&out.stdout).trim() == "yes"
}

#[cfg(unix)]
fn is_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Spec §1.5 and §3.7 Layer 4. A managed child and every managed descendant
/// must not survive termination. A direct-child-only assertion is insufficient,
/// so the probe builds a real grandchild and checks that one too.
#[tokio::test]
async fn terminating_a_managed_tree_kills_the_grandchild_too() {
    let probe = env!("CARGO_BIN_EXE_tree_probe");
    let mut handle = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--spawn-grandchild".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
    })
    .expect("spawn should succeed");

    // tree_probe prints its grandchild's pid on its first stdout line.
    let mut lines = handle.stdout_lines().expect("stdout was captured");
    let first = tokio::time::timeout(Duration::from_secs(10), async {
        use tokio::io::AsyncBufReadExt;
        lines.next_line().await
    })
    .await
    .expect("probe should report within 10s")
    .unwrap()
    .expect("probe should print a line");

    let grandchild: u32 = first
        .trim()
        .strip_prefix("grandchild=")
        .expect("probe prints grandchild=<pid>")
        .parse()
        .unwrap();
    let child = handle.id().expect("child has a pid");

    assert!(is_alive(child), "child should be alive before termination");
    assert!(is_alive(grandchild), "grandchild should be alive before termination");

    handle.terminate_tree().expect("termination should succeed");

    // Give the OS a bounded moment to reap, then assert both are gone.
    for _ in 0..50 {
        if !is_alive(child) && !is_alive(grandchild) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("child or grandchild survived termination: child={child} grandchild={grandchild}");
}

/// Spec §1.5: the child's stdin is closed. An open stdin that never receives
/// data costs a fixed stall on every turn — measured at three seconds against
/// the real harness.
#[tokio::test]
async fn a_spawned_child_has_no_inherited_stdin() {
    let probe = env!("CARGO_BIN_EXE_tree_probe");
    let mut handle = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--read-stdin".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
    })
    .unwrap();

    let status = tokio::time::timeout(Duration::from_secs(5), handle.wait())
        .await
        .expect("a child with closed stdin must see EOF immediately, not block")
        .unwrap();
    assert!(status.success());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test containment`
Expected: FAIL — `shadows::process` does not exist.

- [ ] **Step 3: Write `src/bin/tree_probe.rs`**

```rust
//! Test-support binary. Not product code. It exists so the containment test
//! has a real daemon -> child -> grandchild hierarchy to terminate.

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--read-stdin") {
        let mut buf = String::new();
        // With stdin closed this returns Ok(0) immediately. With an inherited
        // console stdin it blocks, and the test times out.
        let _ = std::io::stdin().read_to_string(&mut buf);
        return;
    }

    if args.iter().any(|a| a == "--spawn-grandchild") {
        let me = std::env::current_exe().expect("current exe");
        let grandchild = std::process::Command::new(me)
            .arg("--sleep")
            .spawn()
            .expect("grandchild should spawn");
        println!("grandchild={}", grandchild.id());
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }

    // Both the child and the grandchild end up here and sleep until killed.
    std::thread::sleep(std::time::Duration::from_secs(600));
}
```

Declare it in `Cargo.toml`:

```toml
[[bin]]
name = "tree_probe"
path = "src/bin/tree_probe.rs"
```

- [ ] **Step 4: Write `src/process/mod.rs`**

```rust
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
const WINDOWS_ESSENTIAL_ENV: &[&str] =
    &["SystemRoot", "SystemDrive", "ComSpec", "PATHEXT", "TEMP", "TMP", "USERPROFILE", "APPDATA"];

pub fn spawn(spec: ProcessSpec) -> io::Result<ProcessHandle> {
    let mut cmd = Command::new(&spec.executable);
    cmd.args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        // Spec §1.5: stdin is closed unless the harness contract requires
        // streaming input.
        .stdin(Stdio::null())
        .stdout(if spec.capture_stdout { Stdio::piped() } else { Stdio::null() })
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
    let pid = child.id().ok_or_else(|| {
        io::Error::other("child exited before a pid could be observed")
    })?;
    let containment = containment::attach(pid)?;

    let stdout = child
        .stdout
        .take()
        .map(|out| {
            use tokio::io::AsyncBufReadExt;
            BufReader::new(out).lines()
        });

    Ok(ProcessHandle { child, containment, stdout })
}
```

- [ ] **Step 5: Write `src/process/containment_windows.rs`**

```rust
use std::io;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

pub struct Containment {
    job: HANDLE,
}

// The handle is owned exclusively by this struct for its lifetime.
unsafe impl Send for Containment {}
unsafe impl Sync for Containment {}

pub fn configure(_cmd: &mut tokio::process::Command) {
    // Nothing to set before spawn on Windows; the job is assigned after.
}

/// Spec §1.5: Job Object kill-on-owner-close semantics, with breakaway
/// prevented. `JOB_OBJECT_LIMIT_BREAKAWAY_OK` and
/// `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` are deliberately not set, so a
/// descendant cannot leave the job.
pub fn attach(pid: u32) -> io::Result<Containment> {
    unsafe {
        let job = CreateJobObjectW(None, None).map_err(io::Error::other)?;

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(io::Error::other)?;

        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
            .map_err(io::Error::other)?;
        let assign = AssignProcessToJobObject(job, process);
        let _ = CloseHandle(process);
        assign.map_err(io::Error::other)?;

        Ok(Containment { job })
    }
}

impl Containment {
    pub fn terminate(&mut self) -> io::Result<()> {
        unsafe { TerminateJobObject(self.job, 1).map_err(io::Error::other) }
    }
}

impl Drop for Containment {
    fn drop(&mut self) {
        // Closing the last handle kills the job, which is the guarantee the
        // spec asks for: the tree does not outlive its owning runtime.
        unsafe {
            let _ = CloseHandle(self.job);
        }
    }
}
```

Add to `Cargo.toml`:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_System_JobObjects",
    "Win32_System_Threading",
] }
```

- [ ] **Step 6: Write `src/process/containment_unix.rs`**

```rust
use std::io;

pub struct Containment {
    pgid: i32,
}

/// Spec §1.5: a process group alone is NOT accepted as proof that descendants
/// die when the daemon crashes. It is used here for the deliberate-termination
/// path only. The parent-death half of the contract is not implemented in
/// Milestone 0, and `tests/containment.rs` is expected to fail on Linux until
/// it is — which is the honest state, not a passing test that proves nothing.
pub fn configure(cmd: &mut tokio::process::Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        cmd.pre_exec(|| {
            // New process group, so the whole group can be signalled at once.
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

pub fn attach(pid: u32) -> io::Result<Containment> {
    Ok(Containment { pgid: pid as i32 })
}

impl Containment {
    pub fn terminate(&mut self) -> io::Result<()> {
        unsafe {
            if libc::killpg(self.pgid, libc::SIGKILL) == -1 {
                let err = io::Error::last_os_error();
                if err.raw_os_error() != Some(libc::ESRCH) {
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}
```

Add to `Cargo.toml`:

```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

- [ ] **Step 7: Run tests to verify they pass on Windows**

Run: `cargo test --test containment`
Expected: PASS on Windows.

On Linux, expect `terminating_a_managed_tree_kills_the_grandchild_too` to pass for deliberate termination and the parent-death half to be unimplemented. Do not gate the test off to make the suite green. Record the gap; spec §11.3 requires containment on both platforms before the runtime is called cross-platform, and Milestone 0 only claims Windows.

- [ ] **Step 8: Commit**

```bash
git add src/process src/bin Cargo.toml src/lib.rs tests/containment.rs
git commit -m "feat(process): managed spawn with Job Object tree containment on Windows"
```

---

