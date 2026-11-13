//! One job: answering a `code_lsp` question from the project's language server.
//!
//! The file is checked inside the project by its real path before it is read,
//! its current text goes with the request, and the answer's locations come back
//! project-relative and one-based; a location outside the asker's scope is dropped.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use super::Wanted;
use crate::model::{LspAnswer, LspOperation, LspQuestion, ServerState};
use crate::scope::Asker;
use crate::store::ScopeRow;
use crate::{Codebase, CodebaseError};
use core_projects::ProjectId;

/// The longest one request may take.
const ANSWER_WAIT: Duration = Duration::from_secs(20);
/// The longest a question waits for a server that is starting.
const START_WAIT: Duration = Duration::from_secs(20);
/// The most items an answer carries.
const MAX_ITEMS: usize = 100;
/// The most bytes an answer's result carries.
const MAX_BYTES: usize = 32 * 1024;

/// A project in scope, with the folders its locations are matched against.
struct Root {
    slug: String,
    dirs: Vec<String>,
}

impl Codebase {
    /// Asks the project's language server one LSP operation. The file is refused
    /// unless it is inside the project by its real path; then its current text goes
    /// with the request. `workspaceSymbol` asks every running server and merges.
    pub async fn lsp(
        &self,
        asker: Asker<'_>,
        question: LspQuestion,
    ) -> Result<LspAnswer, CodebaseError> {
        let refuse = |m: &str| CodebaseError::InvalidCommand(m.to_string());
        let scope = self.projects(&asker, question.project.as_deref()).await?;
        self.touch(asker.home()).await;
        let (project, dir) = match &question.project {
            Some(_) => scope.first(),
            None => scope.iter().find(|(id, _, _)| id == asker.home()),
        }
        .and_then(|(id, _, dir)| Some((id.clone(), PathBuf::from(dir.as_ref()?))))
        .ok_or_else(|| refuse("the project has no folder"))?;
        let roots = Codebase::roots(&scope).await;
        if question.operation == LspOperation::WorkspaceSymbol {
            let query = question
                .query
                .ok_or_else(|| refuse("workspaceSymbol needs a query"))?;
            return self.workspace_symbol(&project, &query, &roots).await;
        }
        let file = question
            .file
            .as_deref()
            .ok_or_else(|| refuse("this operation needs a file"))?;
        let rel = Codebase::inside(file)?;
        let full = Codebase::real_file(&dir, &rel).await?;
        let position = match question.operation {
            LspOperation::DocumentSymbol => None,
            _ => {
                let (Some(line), Some(character)) = (question.line, question.character) else {
                    return Err(refuse("this operation needs a line and a character"));
                };
                if line == 0 || character == 0 {
                    return Err(refuse("lines and characters are one-based"));
                }
                Some((line - 1, character - 1))
            }
        };
        let Some(server) = self.server_for(&project, &rel).await? else {
            let extension = Codebase::extension(&rel).unwrap_or("").to_string();
            return self.no_server(&project, &rel, extension).await;
        };
        let Some((wanted, running)) = self.wait_running(&project, &server.id).await else {
            let extension = Codebase::extension(&rel).unwrap_or("").to_string();
            return self.no_server(&project, &rel, extension).await;
        };
        let language = wanted.language(&rel).unwrap_or("plaintext").to_string();
        let bytes = tokio::fs::read(&full)
            .await
            .map_err(|_| refuse("the file is not in the project"))?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let uri = Codebase::uri(&full)?;
        let document = json!({ "uri": uri });
        let at = |(line, character): (u32, u32)| {
            json!({
                "textDocument": document,
                "position": { "line": line, "character": character },
            })
        };
        let ask = |method: &'static str, params: Value| {
            let (running, full, language, text) = (
                running.clone(),
                full.clone(),
                language.clone(),
                text.clone(),
            );
            let id = wanted.id.clone();
            async move {
                running
                    .ask(&full, &language, text, method, params, ANSWER_WAIT)
                    .await
                    .map_err(|e| Codebase::unavailable(&id, e))
            }
        };
        let result = match (question.operation, position) {
            (LspOperation::DocumentSymbol, _) => {
                ask(
                    "textDocument/documentSymbol",
                    json!({ "textDocument": document }),
                )
                .await?
            }
            (LspOperation::FindReferences, Some(p)) => {
                let mut params = at(p);
                params["context"] = json!({ "includeDeclaration": true });
                ask("textDocument/references", params).await?
            }
            (LspOperation::GoToDefinition, Some(p)) => {
                ask("textDocument/definition", at(p)).await?
            }
            (LspOperation::GoToImplementation, Some(p)) => {
                ask("textDocument/implementation", at(p)).await?
            }
            (LspOperation::Hover, Some(p)) => ask("textDocument/hover", at(p)).await?,
            (operation @ (LspOperation::IncomingCalls | LspOperation::OutgoingCalls), Some(p)) => {
                let items = ask("textDocument/prepareCallHierarchy", at(p)).await?;
                let method = if operation == LspOperation::IncomingCalls {
                    "callHierarchy/incomingCalls"
                } else {
                    "callHierarchy/outgoingCalls"
                };
                let mut calls = Vec::new();
                for item in items.as_array().into_iter().flatten() {
                    let answer = ask(method, json!({ "item": item })).await?;
                    calls.extend(answer.as_array().cloned().unwrap_or_default());
                }
                Value::Array(calls)
            }
            _ => return Err(refuse("this operation needs a line and a character")),
        };
        let (result, truncated) = Codebase::bounded(Codebase::located(result, &roots, &rel));
        Ok(LspAnswer::Answered {
            server: wanted.id,
            result,
            truncated,
        })
    }

    /// `workspace/symbol` to every running server of the project, merged.
    async fn workspace_symbol(
        &self,
        project: &ProjectId,
        query: &str,
        roots: &[Root],
    ) -> Result<LspAnswer, CodebaseError> {
        let mut servers = Vec::new();
        let mut merged = Vec::new();
        for wanted in self.wanted_servers(project, &Default::default()).await? {
            let Some((wanted, running)) = self.running_server(project, &wanted.id).await else {
                continue;
            };
            let answer = running
                .request("workspace/symbol", json!({ "query": query }), ANSWER_WAIT)
                .await
                .map_err(|e| Codebase::unavailable(&wanted.id, e))?;
            merged.extend(answer.as_array().cloned().unwrap_or_default());
            servers.push(wanted.id);
        }
        if servers.is_empty() {
            return Ok(LspAnswer::NoServer {
                extension: String::new(),
                server: None,
                state: None,
                install: None,
            });
        }
        let (result, truncated) =
            Codebase::bounded(Codebase::located(Value::Array(merged), roots, ""));
        Ok(LspAnswer::Answered {
            server: servers.join(","),
            result,
            truncated,
        })
    }

    /// The server that reads `rel`, whether or not it runs or is switched off.
    async fn server_for(
        &self,
        _project: &ProjectId,
        rel: &str,
    ) -> Result<Option<Wanted>, CodebaseError> {
        let added = self.inner.store.servers_added().await?;
        Ok(Codebase::every_server(&added)
            .into_iter()
            .find(|server| server.reads(rel)))
    }

    /// Why no server answers `rel`: the server's state and install line.
    async fn no_server(
        &self,
        project: &ProjectId,
        rel: &str,
        extension: String,
    ) -> Result<LspAnswer, CodebaseError> {
        let Some(server) = self.server_for(project, rel).await? else {
            return Ok(LspAnswer::NoServer {
                extension,
                server: None,
                state: None,
                install: None,
            });
        };
        let status = self
            .servers(Asker::Grant(project))
            .await?
            .into_iter()
            .find(|s| s.id == server.id);
        let state = status.as_ref().map(|s| s.state.clone());
        let install = match state {
            Some(ServerState::NotInstalled) => server.install,
            _ => None,
        };
        Ok(LspAnswer::NoServer {
            extension,
            server: Some(server.id),
            state,
            install,
        })
    }

    /// A request to server `id` that failed, in words that tell the model how the
    /// server stands: running but silent, failed with its last error line, or not
    /// startable.
    fn unavailable(id: &str, error: shadows_lsp::LspError) -> CodebaseError {
        use shadows_lsp::LspError;
        let state = match error {
            LspError::Timeout => format!(
                "is running but did not answer in time ({} s); ask again, or ask something \
                 narrower",
                ANSWER_WAIT.as_secs()
            ),
            LspError::Exited(Some(line)) => format!("failed: it exited, its last line: {line}"),
            LspError::Exited(None) => "failed: it exited with no error line".into(),
            LspError::Spawn(why) => format!("is not installed or could not start: {why}"),
            LspError::Initialize(why) => format!("failed: it refused to initialize: {why}"),
            LspError::Server { code, message } => {
                format!("is running and refused this request ({code}): {message}")
            }
        };
        CodebaseError::LanguageServer(format!("the language server {id} {state}"))
    }

    /// The running server `id`, waiting up to `START_WAIT` while it starts.
    async fn wait_running(
        &self,
        project: &ProjectId,
        id: &str,
    ) -> Option<(Wanted, Arc<shadows_lsp::Server>)> {
        let started = Instant::now();
        loop {
            if let Some(running) = self.running_server(project, id).await {
                return Some(running);
            }
            let starting = {
                let active = self.inner.active.lock().await;
                match active.worker(project) {
                    Some(worker) => {
                        matches!(worker.server_state(id), Some(ServerState::Starting) | None)
                    }
                    None => false,
                }
            };
            if !starting || started.elapsed() >= START_WAIT {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    /// `path` relative to `project`'s folder, checked as `lsp`'s file is: inside
    /// the project, and, when it is there, by its real path. A path not on
    /// disk (a deleted file) passes on its text alone, since nothing is read.
    pub async fn project_file(
        &self,
        project: &ProjectId,
        path: &str,
    ) -> Result<String, CodebaseError> {
        let rel = Codebase::inside(path)?;
        if rel.is_empty() {
            return Err(CodebaseError::InvalidCommand(
                "the path must name a file in the project".into(),
            ));
        }
        let dir = self
            .folder(project)
            .await
            .ok_or_else(|| CodebaseError::InvalidCommand("the project has no folder".into()))?;
        if tokio::fs::symlink_metadata(dir.join(&rel)).await.is_ok() {
            Codebase::real_file(&dir, &rel).await?;
        }
        Ok(rel)
    }

    /// `rel` under `dir`, refused unless its real path is that path inside the
    /// folder's real path: a link or junction out of the project is never read.
    async fn real_file(dir: &Path, rel: &str) -> Result<PathBuf, CodebaseError> {
        let refuse = || CodebaseError::InvalidCommand("the file is not in the project".into());
        let full = dir.join(rel);
        let canonical_dir = tokio::fs::canonicalize(dir).await.map_err(|_| refuse())?;
        let real = tokio::fs::canonicalize(&full).await.map_err(|_| refuse())?;
        if rel.is_empty() || !Codebase::real_path_is(&real, &canonical_dir, rel) {
            return Err(CodebaseError::InvalidCommand(
                "the path must be inside the project".into(),
            ));
        }
        if !tokio::fs::metadata(&real).await.is_ok_and(|m| m.is_file()) {
            return Err(refuse());
        }
        Ok(full)
    }

    fn uri(path: &Path) -> Result<String, CodebaseError> {
        lsp_types::Url::from_file_path(path)
            .map(|u| u.to_string())
            .map_err(|()| CodebaseError::InvalidCommand("the path is not absolute".into()))
    }

    /// Each project in scope, with its folder as stored and as it really is.
    async fn roots(scope: &[ScopeRow]) -> Vec<Root> {
        let mut roots = Vec::new();
        for (_, slug, dir) in scope {
            let Some(dir) = dir else {
                continue;
            };
            let mut dirs = vec![Codebase::comparable(dir)];
            if let Ok(real) = tokio::fs::canonicalize(dir).await {
                dirs.push(Codebase::comparable(&real.to_string_lossy()));
            }
            roots.push(Root {
                slug: slug.clone(),
                dirs,
            });
        }
        roots
    }

    /// A path as locations are compared: '/', no `\\?\` prefix, lower case on Windows.
    fn comparable(path: &str) -> String {
        let p = path.replace('\\', "/");
        let p = p
            .strip_prefix("//?/")
            .unwrap_or(&p)
            .trim_end_matches('/')
            .to_string();
        if cfg!(windows) { p.to_lowercase() } else { p }
    }

    /// The project and project-relative path of a `file://` URI, or `None`
    /// outside every project in scope.
    fn place(uri: &str, roots: &[Root]) -> Option<(String, String)> {
        let path = lsp_types::Url::parse(&Codebase::drive_decoded(uri))
            .ok()?
            .to_file_path()
            .ok()?;
        let original = path.to_string_lossy().replace('\\', "/");
        let original = original
            .strip_prefix("//?/")
            .unwrap_or(&original)
            .to_string();
        let compared = Codebase::comparable(&original);
        for root in roots {
            for dir in &root.dirs {
                if let Some(rest) = compared.strip_prefix(&format!("{dir}/")) {
                    let rel = &original[original.len() - rest.len()..];
                    return Some((root.slug.clone(), rel.to_string()));
                }
            }
        }
        None
    }

    /// `uri` with a percent-encoded drive colon decoded: `typescript-go` writes
    /// `file:///f%3A/x`, and a file URL is a path only with `f:` as written.
    fn drive_decoded(uri: &str) -> String {
        let Some(rest) = uri.strip_prefix("file:///") else {
            return uri.to_string();
        };
        let bytes = rest.as_bytes();
        let encoded = bytes.len() >= 4
            && bytes[0].is_ascii_alphabetic()
            && bytes[1..4].eq_ignore_ascii_case(b"%3A");
        if encoded {
            format!("file:///{}:{}", &rest[..1], &rest[4..])
        } else {
            uri.to_string()
        }
    }

    /// The answer with every location rewritten to `{ project, path, line,
    /// character }`, one-based; an item located outside the scope is dropped.
    /// A range with no URI (a document symbol) is in `file`.
    fn located(value: Value, roots: &[Root], file: &str) -> Value {
        match value {
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .filter_map(|item| Codebase::relocated(item, roots, file))
                    .collect(),
            ),
            other => Codebase::relocated(other, roots, file).unwrap_or(Value::Null),
        }
    }

    fn relocated(value: Value, roots: &[Root], file: &str) -> Option<Value> {
        let Value::Object(object) = value else {
            return Some(value);
        };
        let mut out = Map::new();
        let uri = object
            .get("uri")
            .or_else(|| object.get("targetUri"))
            .and_then(Value::as_str);
        let place = match uri {
            Some(uri) => Some(Codebase::place(uri, roots)?),
            None => None,
        };
        let range = object
            .get("targetSelectionRange")
            .or_else(|| object.get("selectionRange"))
            .or_else(|| object.get("range"))
            .or_else(|| object.get("location").and_then(|l| l.get("range")));
        let nested_uri = object
            .get("location")
            .and_then(|l| l.get("uri"))
            .and_then(Value::as_str);
        let place = match (place, nested_uri) {
            (Some(place), _) => Some(place),
            (None, Some(uri)) => Some(Codebase::place(uri, roots)?),
            (None, None) => None,
        };
        if let Some(start) = range.and_then(|r| r.get("start")) {
            let (project, path) = place.unwrap_or_else(|| (String::new(), file.to_string()));
            if !project.is_empty() {
                out.insert("project".into(), Value::String(project));
            }
            out.insert("path".into(), Value::String(path));
            let one = |key: &str| start.get(key).and_then(Value::as_u64).map(|n| n + 1);
            out.insert("line".into(), json!(one("line")));
            out.insert("character".into(), json!(one("character")));
        }
        for (key, item) in object {
            if matches!(
                key.as_str(),
                "uri"
                    | "targetUri"
                    | "range"
                    | "selectionRange"
                    | "targetRange"
                    | "targetSelectionRange"
                    | "originSelectionRange"
                    | "location"
                    | "data"
                    | "fromRanges"
            ) {
                continue;
            }
            let item = match item {
                Value::Array(_) | Value::Object(_) => Codebase::located(item, roots, file),
                other => other,
            };
            out.insert(key, item);
        }
        Some(Value::Object(out))
    }

    /// At most `MAX_ITEMS` items and `MAX_BYTES` bytes; `true` when anything was
    /// left out.
    fn bounded(value: Value) -> (Value, bool) {
        let Value::Array(mut items) = value else {
            let text = value.to_string();
            if text.len() <= MAX_BYTES {
                return (value, false);
            }
            let mut end = MAX_BYTES;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            return (Value::String(text[..end].to_string()), true);
        };
        let mut truncated = items.len() > MAX_ITEMS;
        items.truncate(MAX_ITEMS);
        while !items.is_empty() && Value::Array(items.clone()).to_string().len() > MAX_BYTES {
            items.pop();
            truncated = true;
        }
        (Value::Array(items), truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::{Codebase, Root};

    #[test]
    fn a_percent_encoded_drive_is_decoded() {
        let decoded = Codebase::drive_decoded("file:///f%3A/p/web/greet.ts");
        assert_eq!(decoded, "file:///f:/p/web/greet.ts");
        assert_eq!(
            Codebase::drive_decoded("file:///f:/a.ts"),
            "file:///f:/a.ts"
        );
        assert_eq!(
            Codebase::drive_decoded("file:///home/a.ts"),
            "file:///home/a.ts"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_percent_encoded_drive_is_placed_in_its_project() {
        let roots = [Root {
            slug: "testing4".into(),
            dirs: vec![Codebase::comparable(r"F:\p")],
        }];
        let place = Codebase::place("file:///f%3A/p/web/greet.ts", &roots);
        assert_eq!(place, Some(("testing4".into(), "web/greet.ts".into())));
    }
}
