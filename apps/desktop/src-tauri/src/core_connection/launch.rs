use super::{
    config::{read_json, write_json, Failure, LaunchConfig},
    ReadyConnection,
};
use serde::{Deserialize, Serialize};
use shadows_process::{spawn_persistent, ProcessIdentity, ProcessSpec};
use std::{
    fs,
    io::Read,
    net::TcpStream,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Receipt {
    pub runtime_id: String,
    pub configuration_id: String,
    pub origin: String,
    pub database: PathBuf,
    pub process: ProcessIdentity,
}
#[derive(Serialize, Deserialize)]
struct Pending {
    configuration_id: String,
    process: ProcessIdentity,
}
pub fn record_path(root: &Path, id: &str) -> PathBuf {
    root.join("runtimes").join(format!("{id}.json"))
}
pub fn pending_path(root: &Path, id: &str) -> PathBuf {
    record_path(root, id).with_extension("pending.json")
}
#[cfg(test)]
pub fn read_receipt(path: &Path) -> Result<Receipt, Failure> {
    read_json(path)?.ok_or("core_unavailable")
}
fn remaining(deadline: Instant) -> Result<Duration, Failure> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        Err("core_start_timeout")
    } else {
        Ok(left)
    }
}
fn verify(
    config: &LaunchConfig,
    receipt: &Receipt,
    deadline: Instant,
) -> Result<ReadyConnection, Failure> {
    if receipt.configuration_id != config.configuration_id
        || receipt.origin != config.origin
        || fs::canonicalize(&receipt.database).ok().as_ref() != Some(&config.db)
        || fs::canonicalize(&receipt.process.executable).ok().as_ref() != Some(&config.executable)
        || !receipt.process.is_current().unwrap_or(false)
    {
        return Err("core_identity_mismatch");
    }
    let timeout = remaining(deadline)?.min(Duration::from_secs(1));
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .map_err(|_| "core_unavailable")?;
    let response = client
        .get(format!("{}/api/runtime", config.origin))
        .send()
        .map_err(|_| "core_unavailable")?;
    if response.status() != 200 {
        return Err("core_identity_mismatch");
    }
    let mut bytes = Vec::new();
    response
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| "core_unavailable")?;
    if bytes.len() > 4096 {
        return Err("core_identity_mismatch");
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Identity {
        runtime_id: String,
        configuration_id: String,
    }
    let identity: Identity =
        serde_json::from_slice(&bytes).map_err(|_| "core_identity_mismatch")?;
    if identity.runtime_id != receipt.runtime_id
        || identity.configuration_id != config.configuration_id
        || uuid::Uuid::parse_str(&identity.runtime_id).is_err()
    {
        return Err("core_identity_mismatch");
    }
    Ok(ReadyConnection {
        origin: config.origin.clone(),
        configuration_id: config.configuration_id.clone(),
        runtime_id: identity.runtime_id,
    })
}
pub fn ensure_running(
    config: &LaunchConfig,
    root: &Path,
    timeout: Duration,
) -> Result<ReadyConnection, Failure> {
    let deadline = Instant::now() + timeout;
    let record = record_path(root, &config.configuration_id);
    fs::create_dir_all(record.parent().unwrap()).map_err(|_| "core_config_unavailable")?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(record.with_extension("startup"))
        .map_err(|_| "core_config_unavailable")?;
    while lock.try_lock().is_err() {
        remaining(deadline)?;
        std::thread::sleep(Duration::from_millis(20));
    }
    if let Some(receipt) = read_json::<Receipt>(&record)? {
        if receipt.process.is_current().unwrap_or(false) {
            return verify(config, &receipt, deadline);
        }
    }
    let pending_file = pending_path(root, &config.configuration_id);
    let mut pending = read_json::<Pending>(&pending_file)?.filter(|pending| {
        pending.configuration_id == config.configuration_id
            && pending.process.is_current().unwrap_or(false)
    });
    if pending.is_none() {
        let socket = config.socket()?;
        if TcpStream::connect_timeout(
            &socket,
            remaining(deadline)?.min(Duration::from_millis(250)),
        )
        .is_ok()
        {
            return Err("core_address_in_use");
        }
        // SQLite interprets the verbatim Windows prefix as URL query syntax.
        // Simplify only safe canonical paths at the CLI boundary.
        let text = |path: &Path| dunce::simplified(path).to_string_lossy().into_owned();
        let args = vec![
            "serve".into(),
            "--db".into(),
            text(&config.db),
            "--bind".into(),
            socket.to_string(),
            "--node".into(),
            text(&config.node),
            "--adapter".into(),
            text(&config.adapter),
            "--harness".into(),
            text(&config.harness),
            "--aylul-config".into(),
            text(&config.aylul_config),
            "--desktop-record".into(),
            text(&record),
            "--desktop-configuration".into(),
            config.configuration_id.clone(),
        ];
        let allowed = [
            "SystemRoot",
            "SystemDrive",
            "ComSpec",
            "PATHEXT",
            "PATH",
            "USERPROFILE",
            "HOME",
            "APPDATA",
            "LOCALAPPDATA",
            "TEMP",
            "TMP",
        ];
        let env = allowed
            .into_iter()
            .filter_map(|key| {
                std::env::var(key)
                    .ok()
                    .map(|value| (key.to_string(), value))
            })
            .collect();
        let process = spawn_persistent(ProcessSpec {
            executable: config.executable.clone(),
            args,
            cwd: root.to_path_buf(),
            env,
            capture_stdout: false,
            pipe_stdin: false,
        })
        .map_err(|_| "core_start_failed")?;
        let launched = Pending {
            configuration_id: config.configuration_id.clone(),
            process,
        };
        write_json(&pending_file, &launched)?;
        pending = Some(launched);
    }
    let pending = pending.unwrap();
    loop {
        remaining(deadline)?;
        if let Some(receipt) = read_json::<Receipt>(&record)? {
            if receipt.process == pending.process {
                match verify(config, &receipt, deadline) {
                    Ok(ready) => {
                        let _ = fs::remove_file(&pending_file);
                        return Ok(ready);
                    }
                    Err("core_unavailable") => {}
                    Err(error) => return Err(error),
                }
            } else if receipt.process.is_current().unwrap_or(false) {
                return Err("core_identity_mismatch");
            }
        }
        if !pending.process.is_current().unwrap_or(false) {
            return Err("core_start_failed");
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}
