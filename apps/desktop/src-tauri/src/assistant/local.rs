//! Native-only access to an operator-configured product daemon.
use serde::Deserialize;
use serde_json::Value;
use std::{io::Read, path::Path, time::Duration};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Connection {
    origin: String,
    installation_id: String,
    token: String,
}
impl Connection {
    fn read(path: &Path) -> Result<Self, &'static str> {
        if !path.is_absolute() {
            return Err("local_unavailable");
        }
        let file = std::fs::File::open(path).map_err(|_| "local_unavailable")?;
        let mut data = Vec::new();
        file.take(16_385)
            .read_to_end(&mut data)
            .map_err(|_| "local_unavailable")?;
        if data.len() > 16_384 {
            return Err("local_unavailable");
        }
        let connection: Self = serde_json::from_slice(&data).map_err(|_| "local_unavailable")?;
        let url = url::Url::parse(&connection.origin).map_err(|_| "local_unavailable")?;
        if url.scheme() != "http"
            || !matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
            || url.port().is_none()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || connection.token.len() != 64
            || !connection
                .token
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || connection.installation_id.len() != 36
            || !connection
                .installation_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
        {
            return Err("local_unavailable");
        }
        Ok(connection)
    }
    fn call(&self, request: super::Request) -> Result<Value, &'static str> {
        request.operation()?;
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(45))
            .no_proxy()
            .build()
            .map_err(|_| "local_unavailable")?;
        let response = client
            .post(format!("{}/assistant", self.origin.trim_end_matches('/')))
            .bearer_auth(&self.token)
            .json(&request)
            .send()
            .map_err(|_| "local_unavailable")?;
        let status = response.status();
        let mut data = Vec::new();
        response
            .take(131_073)
            .read_to_end(&mut data)
            .map_err(|_| "local_unavailable")?;
        if data.len() > 131_072 {
            return Err("local_unavailable");
        }
        let result: Value = serde_json::from_slice(&data).map_err(|_| "local_unavailable")?;
        if status.is_success() {
            return Ok(result);
        }
        Err(match result["error"].as_str() {
            Some("request_conflict") => "request_conflict",
            Some("request_refused") => "request_refused",
            _ => "local_unavailable",
        })
    }
}
fn configured() -> Result<Option<Connection>, &'static str> {
    match std::env::var_os("SHADOWS_DAEMON_DESCRIPTOR") {
        Some(path) => Connection::read(Path::new(&path)).map(Some),
        None => Ok(None),
    }
}
pub fn installation() -> Result<Option<String>, &'static str> {
    Ok(configured()?.map(|connection| connection.installation_id))
}
pub fn call(request: super::Request, installation_id: &str) -> Result<Value, &'static str> {
    let connection = configured()?.ok_or("local_unavailable")?;
    if connection.installation_id != installation_id {
        return Err("source_changed");
    }
    connection.call(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_targets_are_native_loopback_only() {
        let root =
            std::env::temp_dir().join(format!("shadows-native-descriptor-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("connection.json");
        for origin in [
            "https://public.test",
            "http://127.0.0.1:4320/other",
            "http://user@127.0.0.1:4320",
            "http://127.0.0.1:4320/?key=x",
        ] {
            std::fs::write(&path,serde_json::to_vec(&serde_json::json!({"origin":origin,"installationId":"00000000-0000-4000-8000-000000000001","token":"a".repeat(64)})).unwrap()).unwrap();
            assert!(Connection::read(&path).is_err());
        }
        std::fs::write(&path,serde_json::to_vec(&serde_json::json!({"origin":"http://127.0.0.1:4320","installationId":"00000000-0000-4000-8000-000000000001","token":"a".repeat(64)})).unwrap()).unwrap();
        assert!(Connection::read(&path).is_ok());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&root).unwrap();
    }
}
