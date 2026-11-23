//! The installed Codex app-server and its process-local Planner policy.
mod connection;
mod mapping;
mod prompt;
mod protocol;
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
const SCHEMA_BUNDLE: &str = "codex_app_server_protocol.schemas.json";
const CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Debug)]
pub struct CodexProgram {
    executable: PathBuf,
    home: PathBuf,
}

/// A Codex CLI whose protocol matched the tested one, at the version that was checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedCodex {
    pub version: String,
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
        let mut spec = self.catalog_spec(cwd)?;
        spec.env.push((GRANT_ENV.into(), token.into()));
        Ok(spec)
    }

    /// Launches the app-server without the credentials of a thread grant.
    pub fn catalog_spec(&self, cwd: &Path) -> Result<ProcessSpec, ConnectionError> {
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
        let profile = format!("default_permissions=\"{}\"", session::ASK_PROFILE);
        let filesystem = Self::ask_filesystem(project);
        for setting in [
            "features.apps=false",
            "features.plugins=false",
            "features.hooks=false",
            "features.multi_agent=false",
            "web_search=\"disabled\"",
            &trust,
            &shell,
            &profile,
            &filesystem,
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
        env.push(("CODEX_HOME".into(), home.into()));
        Ok(ProcessSpec {
            executable: self.executable.clone(),
            args,
            cwd: cwd.to_path_buf(),
            env,
            capture_stdout: true,
            pipe_stdin: true,
        })
    }

    /// The `ask` mode's filesystem: the project and its `.git` writable, so a commit needs no
    /// approval; `.git/hooks` and `.git/config` read-only and `.git/config.lock` denied, since a
    /// hook or a `core.hooksPath` would run outside the sandbox on the person's next git
    /// command; the temporary folder writable; everything else readable.
    fn ask_filesystem(project: &str) -> String {
        let root = project.replace('\\', "/");
        let root = root.trim_end_matches('/');
        let entries = [
            (":root".to_string(), "read"),
            (":tmpdir".to_string(), "write"),
            (root.to_string(), "write"),
            (format!("{root}/.git"), "write"),
            (format!("{root}/.git/hooks"), "read"),
            (format!("{root}/.git/config"), "read"),
            (format!("{root}/.git/config.lock"), "deny"),
        ]
        .map(|(path, access)| format!("{}=\"{access}\"", json!(path)))
        .join(",");
        format!(
            "permissions.{}.filesystem={{{entries}}}",
            session::ASK_PROFILE
        )
    }

    /// Checks the installed CLI against the tested protocol. `scratch` receives the generated
    /// schema and is removed afterwards.
    pub async fn check(&self, scratch: &Path) -> Result<CheckedCodex, ConnectionError> {
        let checked = self.checked(scratch).await;
        let _ = std::fs::remove_dir_all(scratch);
        checked
    }

    async fn checked(&self, scratch: &Path) -> Result<CheckedCodex, ConnectionError> {
        let version = self.installed_version(scratch).await?;
        let floor = protocol::tested_version();
        if !protocol::version_at_least(&version, &floor) {
            return Err(ConnectionError::Rpc(format!(
                "Codex CLI {floor} or later is required"
            )));
        }
        let found = self.installed_protocol(scratch, &version).await?;
        let tested: Value = serde_json::from_str(protocol::TESTED)
            .map_err(|_| ConnectionError::Protocol("the tested Codex protocol is not JSON"))?;
        let changes = protocol::compare(&tested, &found);
        if changes.is_empty() {
            Ok(CheckedCodex { version })
        } else {
            Err(ConnectionError::Rpc(protocol::refusal(&version, &changes)))
        }
    }

    /// The installed CLI's protocol in the tested snapshot's form, as the text to commit when
    /// a new version is adopted. `scratch` is removed afterwards.
    pub async fn adopt(&self, scratch: &Path) -> Result<String, ConnectionError> {
        let adopted = async {
            let version = self.installed_version(scratch).await?;
            let found = self.installed_protocol(scratch, &version).await?;
            serde_json::to_string_pretty(&protocol::sorted(found))
                .map(|text| text + "\n")
                .map_err(|error| ConnectionError::Rpc(error.to_string()))
        }
        .await;
        let _ = std::fs::remove_dir_all(scratch);
        adopted
    }

    async fn installed_version(&self, scratch: &Path) -> Result<String, ConnectionError> {
        let output = self.run(scratch, &["--version"]).await?;
        output
            .split_whitespace()
            .last()
            .map(str::to_owned)
            .ok_or_else(|| ConnectionError::Rpc("Codex CLI printed no version".into()))
    }

    async fn installed_protocol(
        &self,
        scratch: &Path,
        version: &str,
    ) -> Result<Value, ConnectionError> {
        let out = scratch
            .to_str()
            .ok_or(ConnectionError::Protocol("scratch path is not UTF-8"))?;
        self.run(
            scratch,
            // Experimental, as the connection opens: `ask` selects its permission profile
            // through the experimental `permissions` field.
            &[
                "app-server",
                "generate-json-schema",
                "--experimental",
                "--out",
                out,
            ],
        )
        .await?;
        let bundle = std::fs::read_to_string(scratch.join(SCHEMA_BUNDLE)).map_err(|error| {
            ConnectionError::Rpc(format!("Codex CLI wrote no {SCHEMA_BUNDLE}: {error}"))
        })?;
        let bundle: Value = serde_json::from_str(&bundle)
            .map_err(|error| ConnectionError::Rpc(format!("{SCHEMA_BUNDLE}: {error}")))?;
        protocol::extract(&bundle, version).map_err(ConnectionError::Rpc)
    }

    /// Runs the CLI with `catalog_spec`'s environment and the given arguments, and answers its
    /// stdout. A failure answers its first stderr line.
    async fn run(&self, scratch: &Path, args: &[&str]) -> Result<String, ConnectionError> {
        use tokio::io::AsyncReadExt;
        std::fs::create_dir_all(scratch)
            .map_err(|error| ConnectionError::Rpc(format!("Codex check folder: {error}")))?;
        let mut spec = self.catalog_spec(scratch)?;
        spec.args = args.iter().map(|arg| (*arg).to_owned()).collect();
        let mut handle = shadows_process::spawn(spec)
            .map_err(|error| ConnectionError::Rpc(format!("Codex CLI could not start: {error}")))?;
        let (stdin, mut stdout, mut stderr) = handle
            .take_stdio()
            .ok_or(ConnectionError::Protocol("Codex CLI has no stdio"))?;
        drop(stdin);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let finished = tokio::time::timeout(CHECK_TIMEOUT, async {
            let _ = tokio::join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut err));
            handle.wait().await
        })
        .await;
        let status = match finished {
            Ok(Ok(status)) => status,
            Ok(Err(error)) => return Err(ConnectionError::Rpc(error.to_string())),
            Err(_) => {
                let _ = handle.terminate_tree();
                return Err(ConnectionError::Rpc(
                    "Codex CLI did not finish its check".into(),
                ));
            }
        };
        if status.success() {
            return Ok(String::from_utf8_lossy(&out).into_owned());
        }
        let err = String::from_utf8_lossy(&err);
        Err(ConnectionError::Rpc(
            err.lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map_or_else(|| format!("Codex CLI exited with {status}"), str::to_owned),
        ))
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
        version: &str,
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
                    "capabilities":{"experimentalApi":true}
                }),
            )
            .await?;
        let agent = init["userAgent"].as_str().unwrap_or("");
        if !agent
            .split_ascii_whitespace()
            .any(|part| part.ends_with(&format!("/{version}")))
        {
            let found = agent
                .split_ascii_whitespace()
                .find_map(|part| part.split_once('/').map(|(_, found)| found))
                .unwrap_or("an unknown version");
            return Err(ConnectionError::Rpc(format!(
                "Codex CLI changed to {found}; it is checked again on the next opening"
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

    /// The last error Codex logged on its stderr, as a person may read it.
    pub fn last_error(&self) -> Option<String> {
        self.rpc.last_error()
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
