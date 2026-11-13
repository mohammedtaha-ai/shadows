//! The installed Codex app-server and its process-local Planner policy.
mod connection;
mod mapping;
mod prompt;
mod session;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use shadows_process::{ProcessHandle, ProcessSpec};

use crate::connection::{ConnectionError, McpServerSpec, SessionSetup};
use crate::events::HarnessEvent;
use connection::Rpc;
use prompt::Prompts;
use session::Selection;

const GRANT_ENV: &str = "SHADOWS_CODEX_MCP_TOKEN";
const MODE: &str = crate::policy::CODEX_READ_ONLY;
const VERSION: &str = "0.167.0";

#[derive(Debug)]
pub struct CodexProgram {
    executable: PathBuf,
    home: PathBuf,
}

impl CodexProgram {
    pub fn new(executable: PathBuf, home: PathBuf) -> Self {
        Self { executable, home }
    }

    pub fn process_spec(
        &self,
        cwd: &Path,
        setup: &SessionSetup,
    ) -> Result<ProcessSpec, ConnectionError> {
        let (_, token) = planner_server(setup)?;
        let project = cwd
            .to_str()
            .ok_or(ConnectionError::Protocol("project path is not UTF-8"))?;
        let home = self
            .home
            .to_str()
            .ok_or(ConnectionError::Protocol("Codex home is not UTF-8"))?;
        let mut args = vec!["app-server".into(), "--listen".into(), "stdio://".into()];
        let trust = format!(
            "projects={{{}={{trust_level=\"untrusted\"}}}}",
            json!(project)
        );
        let shell = format!("shell_environment_policy.exclude=[\"{GRANT_ENV}\"]");
        for setting in [
            "features.apps=false",
            "features.plugins=false",
            "features.hooks=false",
            "features.multi_agent=false",
            "web_search=\"disabled\"",
            &trust,
            &shell,
        ] {
            args.extend(["-c".into(), setting.into()]);
        }
        #[cfg(windows)]
        args.extend(["-c".into(), "windows.sandbox=\"mxc\"".into()]);
        let mut env: Vec<_> = [
            "PATH",
            "HOME",
            "TMPDIR",
            "LANG",
            "LOCALAPPDATA",
            "PROGRAMDATA",
        ]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok().map(|value| (key.into(), value)))
        .collect();
        env.extend([
            ("CODEX_HOME".into(), home.into()),
            (GRANT_ENV.into(), token.into()),
        ]);
        Ok(ProcessSpec {
            executable: self.executable.clone(),
            args,
            cwd: cwd.to_path_buf(),
            env,
            capture_stdout: true,
            pipe_stdin: true,
        })
    }
}

fn planner_server(setup: &SessionSetup) -> Result<(&str, &str), ConnectionError> {
    let [McpServerSpec::Http { name, url, headers }] = setup.mcp.as_slice() else {
        return Err(ConnectionError::Protocol(
            "Codex needs one Shadows HTTP MCP server",
        ));
    };
    let token = headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("Authorization"))
        .and_then(|(_, value)| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty());
    match token {
        Some(token) if name == "shadows" => Ok((url, token)),
        _ => Err(ConnectionError::Protocol(
            "Codex needs a Shadows thread grant",
        )),
    }
}

#[derive(Clone)]
pub struct Connection {
    rpc: Rpc,
    /// `requestApproval`s waiting for the person, held as ACP's are.
    permissions: crate::acp::Permissions,
    prompts: Arc<Prompts>,
    selection: Arc<Mutex<Option<Selection>>>,
}

impl Connection {
    pub async fn open(
        handle: &mut ProcessHandle,
        events: impl Fn(HarnessEvent) + Clone + Send + Sync + 'static,
    ) -> Result<Self, ConnectionError> {
        let permissions = crate::acp::Permissions::new(events.clone());
        let prompts = Arc::new(Prompts::new(events));
        let notify = prompts.clone();
        let closed = prompts.clone();
        let refused = permissions.clone();
        let rpc = Rpc::open(
            handle,
            move |method, params| notify.notification(method, params),
            move |error| {
                refused.refuse_matching("the program closed", |_| true);
                closed.close(error)
            },
            permissions.clone(),
        )?;
        let init: Value = rpc
            .request(
                "initialize",
                json!({
                    "clientInfo": {"name":"shadows", "version":env!("CARGO_PKG_VERSION")},
                    "capabilities":{"experimentalApi":false}
                }),
            )
            .await?;
        if !init["userAgent"]
            .as_str()
            .unwrap_or("")
            .split_ascii_whitespace()
            .any(|part| part.ends_with(&format!("/{VERSION}")))
        {
            return Err(ConnectionError::Rpc(format!(
                "Codex CLI {VERSION} is required"
            )));
        }
        rpc.notify("initialized")?;
        Ok(Self {
            rpc,
            permissions,
            prompts,
            selection: Arc::default(),
        })
    }

    pub fn is_closed(&self) -> bool {
        self.rpc.is_closed()
    }

    pub fn held_permissions(&self) -> Vec<crate::acp::WaitingPermission> {
        self.permissions.held()
    }

    pub async fn prepare_permission_answer(
        &self,
        request: &str,
        option: &str,
    ) -> Result<crate::acp::PreparedPermissionAnswer, crate::acp::PermissionError> {
        self.permissions
            .prepare(request, option, self.is_closed())
            .await
    }

    pub async fn refuse_held(&self, reason: &str) {
        self.permissions.refuse_all(reason).await;
    }
}
