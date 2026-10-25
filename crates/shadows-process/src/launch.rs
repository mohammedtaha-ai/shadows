//! Explicit child environments and lifetime containment.
use crate::ProcessSpec;
use process_wrap::tokio::{CommandWrap, KillOnDrop};
use std::process::{Command, Stdio};

pub(super) struct Launch;

impl Launch {
    fn command(spec: &ProcessSpec, persistent: bool) -> Command {
        let mut command = Command::new(&spec.executable);
        command.args(&spec.args).current_dir(&spec.cwd).env_clear();
        #[cfg(windows)]
        for key in WINDOWS_ESSENTIAL_ENV {
            // Managed children accept UTF-8 values; independent launches retain OS values.
            if persistent {
                if let Some(value) = std::env::var_os(key) {
                    command.env(key, value);
                }
            } else if let Ok(value) = std::env::var(key) {
                command.env(key, value);
            }
        }
        #[cfg(not(windows))]
        let _ = persistent;
        command.envs(spec.env.iter().map(|(key, value)| (key, value)));
        command
    }

    pub(super) fn managed(spec: &ProcessSpec) -> CommandWrap {
        let mut command = Self::command(spec, false);
        command
            .stdin(if spec.pipe_stdin {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(if spec.capture_stdout {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(Stdio::piped());
        let mut wrapped = CommandWrap::from(tokio::process::Command::from(command));
        // JobObject enables kill-on-owner-close only when KillOnDrop is also registered.
        wrapped.wrap(KillOnDrop);
        #[cfg(windows)]
        wrapped.wrap(process_wrap::tokio::JobObject);
        #[cfg(unix)]
        wrapped.wrap(process_wrap::tokio::ProcessSession);
        wrapped
    }

    pub(super) fn persistent(spec: &ProcessSpec) -> Command {
        let mut command = Self::command(spec, true);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use windows_sys::Win32::System::Threading::{
                CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW,
            };
            command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        command
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
