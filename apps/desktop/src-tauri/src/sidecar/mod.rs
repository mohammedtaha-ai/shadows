//! Starts AYLUL's Node sidecar, relays the window's calls over stdio, stops it on exit.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

/// The only methods the window may call (sidecar `makeHandlers`).
pub const METHODS: &[&str] = &[
    "state",
    "steps",
    "request.answer",
    "refusal.preview",
    "refusal.approve",
    "refusal.dismiss",
    "hooks.preview",
    "hooks.install",
];

type Waiting = Arc<Mutex<HashMap<u64, Sender<Value>>>>;

pub struct Sidecar {
    child: Mutex<Child>,
    /// `None` once `stop` has closed it.
    stdin: Mutex<Option<ChildStdin>>,
    waiting: Waiting,
    next: AtomicU64,
}

impl Sidecar {
    pub fn start(node: &Path, script: &Path, hook_exe: &Path) -> std::io::Result<Self> {
        let mut command = Command::new(node);
        command
            .arg(script)
            .env("AYLUL_HOOK_EXE", hook_exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or(std::io::ErrorKind::BrokenPipe)?;
        let stdout = child.stdout.take().ok_or(std::io::ErrorKind::BrokenPipe)?;
        let waiting: Waiting = Arc::default();
        let readers = waiting.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let Ok(reply) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let Some(id) = reply.get("id").and_then(Value::as_u64) else {
                    continue;
                };
                if let Some(sender) = readers.lock().unwrap().remove(&id) {
                    let _ = sender.send(reply);
                }
            }
        });
        Ok(Self {
            child: Mutex::new(child),
            stdin: Mutex::new(Some(stdin)),
            waiting,
            next: AtomicU64::new(1),
        })
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        if !METHODS.contains(&method) {
            return Err("unknown_method".into());
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = channel();
        self.waiting.lock().unwrap().insert(id, sender);
        let mut line = json!({ "id": id, "method": method, "params": params }).to_string();
        line.push('\n');
        let sent = match self.stdin.lock().unwrap().as_mut() {
            Some(stdin) => stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()),
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        };
        if sent.is_err() {
            self.waiting.lock().unwrap().remove(&id);
            return Err("sidecar_unavailable".into());
        }
        let reply = receiver
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| {
                self.waiting.lock().unwrap().remove(&id);
                "sidecar_timeout".to_string()
            })?;
        match reply.get("error").and_then(Value::as_str) {
            Some(code) => Err(code.to_string()),
            None => Ok(reply.get("result").cloned().unwrap_or(Value::Null)),
        }
    }

    /// Closes stdin, so the sidecar removes hook.json and exits by itself;
    /// kills it only if it is still running 3 s later.
    pub fn stop(&self) {
        drop(self.stdin.lock().unwrap().take());
        let mut child = self.child.lock().unwrap();
        for _ in 0..60 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Managed state: `None` when the sidecar could not start; the app still runs.
pub struct SidecarState(pub Option<Sidecar>);

impl SidecarState {
    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        match &self.0 {
            Some(sidecar) => sidecar.call(method, params),
            None => Err("sidecar_unavailable".into()),
        }
    }

    pub fn stop(&self) {
        if let Some(sidecar) = &self.0 {
            sidecar.stop();
        }
    }
}

/// Development locations; the installed app's locations are not decided yet.
pub fn start_default() -> SidecarState {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let node = std::env::var_os("AYLUL_NODE")
        .map(PathBuf::from)
        .unwrap_or_else(|| "node".into());
    let script = std::env::var_os("AYLUL_SIDECAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("../sidecar/src/main.ts"));
    let hook_exe = std::env::var_os("AYLUL_HOOK_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let target = std::env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| manifest.join("../hook/target"));
            target.join("release").join("aylul-hook.exe")
        });
    match Sidecar::start(&node, &script, &hook_exe) {
        Ok(sidecar) => SidecarState(Some(sidecar)),
        Err(error) => {
            eprintln!("aylul: sidecar did not start: {error}");
            SidecarState(None)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    fn echo() -> Sidecar {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sidecar/echo-fixture.mjs");
        Sidecar::start(Path::new("node"), &script, Path::new("unused.exe")).expect("node on PATH")
    }

    #[test]
    fn a_call_reaches_the_sidecar_and_returns_its_result() {
        let sidecar = echo();
        assert_eq!(
            sidecar.call("state", json!({ "x": 1 })),
            Ok(json!({ "x": 1 }))
        );
        sidecar.stop();
    }

    #[test]
    fn a_sidecar_error_code_comes_back_as_the_error() {
        let sidecar = echo();
        assert_eq!(
            sidecar.call("request.answer", json!({})),
            Err("not_pending".to_string())
        );
        sidecar.stop();
    }

    #[test]
    fn an_unlisted_method_is_refused_before_it_reaches_the_sidecar() {
        let sidecar = echo();
        assert_eq!(
            sidecar.call("shell.exec", json!({})),
            Err("unknown_method".to_string())
        );
        sidecar.stop();
    }

    #[test]
    fn stop_lets_the_sidecar_exit_by_itself() {
        let sidecar = echo();
        sidecar.stop();
        // A killed process exits with code 1 on Windows; closing stdin lets it end with 0.
        let status = sidecar
            .child
            .lock()
            .unwrap()
            .try_wait()
            .unwrap()
            .expect("exited");
        assert!(status.success());
        assert_eq!(
            sidecar.call("state", json!({})),
            Err("sidecar_unavailable".to_string())
        );
    }

    #[test]
    fn no_sidecar_answers_unavailable() {
        let state = SidecarState(None);
        assert_eq!(
            state.call("state", json!({})),
            Err("sidecar_unavailable".to_string())
        );
    }
}
