use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_lsp::router::Router;
use async_lsp::{MainLoop, ServerSocket};
use lsp_types::notification::{
    DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument, Exit, Initialized,
};
use lsp_types::request::{Initialize, Shutdown};
use lsp_types::{
    ClientCapabilities, DidChangeWatchedFilesClientCapabilities, DidChangeWatchedFilesParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, FileChangeType, FileEvent,
    InitializeParams, InitializedParams, TextDocumentIdentifier, TextDocumentItem, Url,
    WorkspaceClientCapabilities, WorkspaceFolder,
};
use serde_json::Value;
use shadows_process::{ProcessHandle, ProcessSpec};
use tokio::io::AsyncBufReadExt;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::{FileChange, LspError, ServerSpec};

/// A request whose params and answer stay JSON. `async-lsp` names a request by a
/// type with a constant method, so each method a caller may ask is one type here.
macro_rules! raw_requests {
    ($($name:ident => $method:literal),* $(,)?) => {
        $(
            enum $name {}
            impl lsp_types::request::Request for $name {
                type Params = Value;
                type Result = Value;
                const METHOD: &'static str = $method;
            }
        )*

        async fn send_raw(
            socket: &ServerSocket,
            method: &str,
            params: Value,
        ) -> Result<Value, async_lsp::Error> {
            match method {
                $($method => socket.request::<$name>(params).await,)*
                other => Err(async_lsp::Error::Response(async_lsp::ResponseError::new(
                    async_lsp::ErrorCode::METHOD_NOT_FOUND,
                    format!("{other} is not a request this client sends"),
                ))),
            }
        }
    };
}

raw_requests! {
    Definition => "textDocument/definition",
    Declaration => "textDocument/declaration",
    TypeDefinition => "textDocument/typeDefinition",
    Implementation => "textDocument/implementation",
    References => "textDocument/references",
    Hover => "textDocument/hover",
    DocumentSymbol => "textDocument/documentSymbol",
    PrepareCallHierarchy => "textDocument/prepareCallHierarchy",
    IncomingCalls => "callHierarchy/incomingCalls",
    OutgoingCalls => "callHierarchy/outgoingCalls",
    PrepareTypeHierarchy => "textDocument/prepareTypeHierarchy",
    Supertypes => "typeHierarchy/supertypes",
    Subtypes => "typeHierarchy/subtypes",
    WorkspaceSymbol => "workspace/symbol",
}

/// One running language server. Every question opens its file, asks, and closes it,
/// so no file stays open between questions.
pub struct Server {
    socket: ServerSocket,
    capabilities: Value,
    error_lines: Arc<Mutex<ErrorLines>>,
    handle: Mutex<ProcessHandle>,
    /// One question at a time: a file is opened, asked and closed whole.
    asking: tokio::sync::Mutex<HashMap<PathBuf, i32>>,
}

impl Server {
    pub async fn start(spec: ServerSpec, timeout: Duration) -> Result<Server, LspError> {
        let root_uri = Url::from_directory_path(&spec.root)
            .map_err(|()| LspError::Spawn(format!("{} is not a folder", spec.root.display())))?;
        let mut handle = shadows_process::spawn(ProcessSpec {
            executable: spec.executable.clone(),
            args: spec.args,
            cwd: spec.root.clone(),
            env: spec.env,
            capture_stdout: true,
            pipe_stdin: true,
        })
        .map_err(|e| LspError::Spawn(format!("{}: {e}", spec.executable.display())))?;
        let (stdin, stdout, stderr) = handle
            .take_stdio()
            .ok_or_else(|| LspError::Spawn("the server's stdio was not piped".into()))?;

        let error_lines = Arc::new(Mutex::new(ErrorLines::default()));
        let keep = error_lines.clone();
        tokio::spawn(async move {
            let mut lines = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(mut kept) = keep.lock() {
                    kept.push(line);
                }
            }
        });

        // async-lsp's loop panics when every socket to it is dropped while it runs
        // ("Sender is alive"), so its router keeps one: the loop then ends only when
        // the server's stdout closes, which a refused start's kill makes happen.
        let (main_loop, socket) = MainLoop::new_client(|own: ServerSocket| {
            let mut router = Router::new(own);
            router
                .unhandled_request(|_, request| {
                    // `workspace/configuration` wants one answer per item; every
                    // other server request (registrations, progress) wants null.
                    let answer = match request.params.get("items").and_then(Value::as_array) {
                        Some(items) if request.method == "workspace/configuration" => {
                            Value::Array(vec![Value::Null; items.len()])
                        }
                        _ => Value::Null,
                    };
                    async move { Ok(answer) }
                })
                .unhandled_notification(|_, _| ControlFlow::Continue(()));
            router
        });
        let running = tokio::spawn(async move {
            let _ = main_loop
                .run_buffered(stdout.compat(), stdin.compat_write())
                .await;
        });

        let server = Server {
            socket,
            capabilities: Value::Null,
            error_lines,
            handle: Mutex::new(handle),
            asking: tokio::sync::Mutex::new(HashMap::new()),
        };
        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(root_uri.clone()),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: root_uri,
                name: spec
                    .root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "root".into()),
            }]),
            capabilities: ClientCapabilities {
                workspace: Some(WorkspaceClientCapabilities {
                    did_change_watched_files: Some(DidChangeWatchedFilesClientCapabilities {
                        dynamic_registration: Some(true),
                        relative_pattern_support: None,
                    }),
                    workspace_folders: Some(true),
                    configuration: Some(true),
                    ..WorkspaceClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        };
        let answer = match tokio::time::timeout(
            timeout,
            server.socket.request::<Initialize>(params),
        )
        .await
        {
            Err(_) => Err(LspError::Timeout),
            Ok(Err(async_lsp::Error::Response(e))) => Err(LspError::Initialize(e.message)),
            Ok(Err(_)) => Err(LspError::Exited(server.error_line())),
            Ok(Ok(answer)) => Ok(answer),
        };
        let answer = match answer {
            Ok(answer) => answer,
            Err(error) => {
                server.kill();
                // The loop ends with the killed process's stdout; a start that failed
                // leaves no task behind.
                let _ = tokio::time::timeout(Duration::from_secs(2), running).await;
                return Err(error);
            }
        };
        let mut server = server;
        server.capabilities = serde_json::to_value(answer.capabilities).unwrap_or(Value::Null);
        server
            .socket
            .notify::<Initialized>(InitializedParams {})
            .map_err(|_| server.exited())?;
        Ok(server)
    }

    /// Opens `path` with `text`, sends one request, and closes the file, whatever the answer.
    pub async fn ask(
        &self,
        path: &Path,
        language_id: &str,
        text: String,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, LspError> {
        let uri = Self::uri(path)?;
        let mut versions = self.asking.lock().await;
        let version = versions.entry(path.to_path_buf()).or_insert(0);
        *version += 1;
        self.socket
            .notify::<DidOpenTextDocument>(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: language_id.to_owned(),
                    version: *version,
                    text,
                },
            })
            .map_err(|_| self.exited())?;
        let answer = self.request(method, params, timeout).await;
        let closed = self
            .socket
            .notify::<DidCloseTextDocument>(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri },
            });
        drop(versions);
        let answer = answer?;
        closed.map_err(|_| self.exited())?;
        Ok(answer)
    }

    /// One request with no file opened, such as `workspace/symbol`.
    pub async fn request(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, LspError> {
        match tokio::time::timeout(timeout, send_raw(&self.socket, method, params)).await {
            Err(_) => Err(LspError::Timeout),
            Ok(Ok(answer)) => Ok(answer),
            Ok(Err(async_lsp::Error::Response(e))) => Err(LspError::Server {
                code: i64::from(e.code.0),
                message: e.message,
            }),
            Ok(Err(_)) => Err(self.exited()),
        }
    }

    /// A server keeps its own copy of a closed file and reads it again only when told,
    /// so every change on disk is told here.
    pub async fn files_changed(&self, changes: &[(PathBuf, FileChange)]) -> Result<(), LspError> {
        let changes = changes
            .iter()
            .map(|(path, change)| {
                Ok(FileEvent {
                    uri: Self::uri(path)?,
                    typ: match change {
                        FileChange::Created => FileChangeType::CREATED,
                        FileChange::Changed => FileChangeType::CHANGED,
                        FileChange::Deleted => FileChangeType::DELETED,
                    },
                })
            })
            .collect::<Result<Vec<_>, LspError>>()?;
        self.socket
            .notify::<DidChangeWatchedFiles>(DidChangeWatchedFilesParams { changes })
            .map_err(|_| self.exited())
    }

    pub fn capabilities(&self) -> &Value {
        &self.capabilities
    }

    /// The stderr line that best says why the server failed (see `ErrorLines`).
    pub fn error_line(&self) -> Option<String> {
        self.error_lines
            .lock()
            .ok()
            .and_then(|kept| kept.reported())
    }

    pub fn pid(&self) -> Option<u32> {
        self.handle.lock().ok().and_then(|handle| handle.id())
    }

    /// Asks the server to stop, then ends its process tree whether or not it did.
    pub async fn shutdown(self) {
        let wait = Duration::from_secs(2);
        if let Ok(Ok(())) = tokio::time::timeout(wait, self.socket.request::<Shutdown>(())).await {
            let _ = self.socket.notify::<Exit>(());
        }
        let Ok(mut handle) = self.handle.into_inner() else {
            return;
        };
        if tokio::time::timeout(wait, handle.wait()).await.is_err() {
            let _ = handle.terminate_tree();
            let _ = handle.wait().await;
        }
    }

    fn kill(&self) {
        if let Ok(mut handle) = self.handle.lock() {
            let _ = handle.terminate_tree();
        }
    }

    fn exited(&self) -> LspError {
        LspError::Exited(self.error_line())
    }

    fn uri(path: &Path) -> Result<Url, LspError> {
        Url::from_file_path(path).map_err(|()| LspError::Server {
            code: -32602,
            message: format!("{} is not an absolute path", path.display()),
        })
    }
}

/// What a server wrote to stderr, kept to say why it failed. A crash ends with
/// a backtrace whose last line (`  14: <unknown>`) says nothing, while the
/// reason comes first, so the first line that is not a frame is reported, and
/// the last line only when every line is a frame.
#[derive(Debug, Default)]
struct ErrorLines {
    first: Option<String>,
    last: Option<String>,
}

impl ErrorLines {
    fn push(&mut self, line: String) {
        if line.trim().is_empty() {
            return;
        }
        if self.first.is_none() && !ErrorLines::is_frame(&line) {
            self.first = Some(line.clone());
        }
        self.last = Some(line);
    }

    fn reported(&self) -> Option<String> {
        self.first.clone().or_else(|| self.last.clone())
    }

    /// A backtrace line: `  14: <unknown>`, `      at src/main.rs:3`, or its heading.
    fn is_frame(line: &str) -> bool {
        let trimmed = line.trim_start();
        let numbered = trimmed
            .split_once(':')
            .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        numbered || trimmed.starts_with("at ") || line.starts_with("Stack backtrace")
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorLines;

    #[test]
    fn a_crash_reports_its_first_line_not_its_last_frame() {
        let stderr = concat!(
            "error: Unknown binary 'rust-analyzer' in official toolchain ",
            "'stable-x86_64-unknown-linux-gnu'.\n",
            "Stack backtrace:\n",
            "   0: rustup::main\n",
            "             at src/main.rs:12\n",
            "\n",
            "  13: __libc_start_main\n",
            "  14: <unknown>\n",
        );
        let mut kept = ErrorLines::default();
        for line in stderr.lines() {
            kept.push(line.to_string());
        }
        assert_eq!(
            kept.reported().as_deref(),
            Some(
                "error: Unknown binary 'rust-analyzer' in official toolchain \
                 'stable-x86_64-unknown-linux-gnu'."
            )
        );
        let mut frames = ErrorLines::default();
        frames.push("  13: __libc_start_main".into());
        frames.push("  14: <unknown>".into());
        assert_eq!(frames.reported().as_deref(), Some("  14: <unknown>"));
        assert_eq!(ErrorLines::default().reported(), None);
    }
}
