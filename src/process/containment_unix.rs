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
