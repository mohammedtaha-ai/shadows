//! Persistent top-level processes; managed harness children use the normal spawn.
use crate::ProcessSpec;
use std::{
    io,
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub started_at: u64,
    pub executable: PathBuf,
}
impl ProcessIdentity {
    /// Compare an OS process instance, not merely a reusable pid.
    pub fn is_current(&self) -> io::Result<bool> {
        Ok(inspect_process(self.pid)?.as_ref() == Some(self))
    }
}

/// Start an independent process without pipe or kill-on-owner-close ownership.
pub fn spawn_persistent(spec: ProcessSpec) -> io::Result<ProcessIdentity> {
    if spec.capture_stdout
        || spec.pipe_stdin
        || !spec.executable.is_absolute()
        || !spec.cwd.is_absolute()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "persistent launch requires absolute paths and closed stdio",
        ));
    }
    let mut command = Command::new(&spec.executable);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
        for key in crate::WINDOWS_ESSENTIAL_ENV {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.envs(spec.env);
    let mut child = command.spawn()?;
    let identity = match inspect_process(child.id()) {
        Ok(Some(identity)) => identity,
        result => {
            let _ = child.kill();
            let _ = child.wait();
            return match result {
                Err(error) => Err(error),
                _ => Err(io::Error::other(
                    "persistent process exited before identification",
                )),
            };
        }
    };
    // Reap when possible without imposing a lifetime on the independent process.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(identity)
}

#[cfg(windows)]
#[allow(unsafe_code)] // OS identity inspection is isolated here; callers use a safe API.
pub fn inspect_process(pid: u32) -> io::Result<Option<ProcessIdentity>> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, FILETIME, HANDLE},
        System::Threading::{
            GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
    };
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            // SAFETY: this guard owns the handle and closes it exactly once.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    // SAFETY: OpenProcess receives no borrowed pointers; Handle owns the returned handle.
    let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if raw.is_null() {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            return Ok(None);
        }
        return Err(error);
    }
    let handle = Handle(raw);
    let mut exit = 0;
    if unsafe { GetExitCodeProcess(handle.0, &mut exit) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if exit != 259 {
        return Ok(None);
    }
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut creation, mut ended, mut kernel, mut user) = (zero, zero, zero, zero);
    if unsafe { GetProcessTimes(handle.0, &mut creation, &mut ended, &mut kernel, &mut user) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle.0, 0, buffer.as_mut_ptr(), &mut length) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let executable = PathBuf::from(
        String::from_utf16(&buffer[..length as usize])
            .map_err(|_| io::Error::other("process image path is not valid UTF-16"))?,
    );
    Ok(Some(ProcessIdentity {
        pid,
        executable,
        started_at: (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime),
    }))
}

#[cfg(unix)]
pub fn inspect_process(pid: u32) -> io::Result<Option<ProcessIdentity>> {
    let root = PathBuf::from(format!("/proc/{pid}"));
    let stat = match std::fs::read_to_string(root.join("stat")) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let rest = stat
        .rsplit_once(") ")
        .ok_or_else(|| io::Error::other("invalid process stat"))?
        .1;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.first() == Some(&"Z") {
        return Ok(None);
    }
    let started_at = fields
        .get(19)
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| io::Error::other("invalid process creation time"))?;
    let executable = match std::fs::read_link(root.join("exe")) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(Some(ProcessIdentity {
        pid,
        started_at,
        executable,
    }))
}
