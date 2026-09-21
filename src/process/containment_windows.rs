use std::io;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

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
