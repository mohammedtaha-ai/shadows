//! One job: which language servers a project runs, and how each stands.
//!
//! The files choose: a server is wanted when the project holds a file with one of
//! its extensions and the person has not switched it off. The project's worker
//! (`watch.rs`) starts what is wanted and stops what is not.

mod ask;
mod catalog;

use std::collections::{BTreeMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::model::{AddedServer, ServerState, ServerStatus};
use crate::scope::Asker;
use crate::store::AddedRow as Added;
use crate::store::ServerChange;
use crate::{Codebase, CodebaseError};
use core_db::CommandContext;

/// One server a project may run: a catalog entry or one the person added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Wanted {
    pub(crate) id: String,
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    /// Extension, as written, to the language id the server is told.
    pub(crate) extensions: BTreeMap<String, String>,
    pub(crate) install: Option<String>,
    pub(crate) added: bool,
}

impl Wanted {
    /// Whether this server reads `path`, by its extension.
    pub(crate) fn reads(&self, path: &str) -> bool {
        self.language(path).is_some()
    }

    /// The language id this server is told for `path`, by its extension: `.C` and `.H`
    /// as written, any other extension lower-cased.
    pub(crate) fn language(&self, path: &str) -> Option<&str> {
        self.language_of(Codebase::extension(path)?)
    }

    /// The language id for an extension: `.C` and `.H` match as written, any other
    /// extension lower-cased too.
    fn language_of(&self, ext: &str) -> Option<&str> {
        self.extensions
            .get(ext)
            .or_else(|| self.extensions.get(&ext.to_lowercase()))
            .map(String::as_str)
    }
}

impl Codebase {
    /// The extension of a path's file name, with its dot, as written.
    pub(crate) fn extension(path: &str) -> Option<&str> {
        let name = path.rsplit(['/', '\\']).next()?;
        let dot = name.rfind('.').filter(|&i| i > 0)?;
        Some(&name[dot..])
    }

    /// Every server Shadows knows: the catalog's, then the person's added ones.
    fn every_server(added: &[Added]) -> Vec<Wanted> {
        let catalog = Codebase::catalog().iter().map(|e| Wanted {
            id: e.id.clone(),
            command: e.command.clone(),
            args: e.args.clone(),
            extensions: e.extensions.clone(),
            install: e.install.here(),
            added: false,
        });
        let added = added.iter().map(|a| Wanted {
            id: a.id.clone(),
            command: a.command.clone(),
            args: a.args.clone(),
            extensions: a
                .extensions
                .iter()
                .map(|e| (e.clone(), e.trim_start_matches('.').to_lowercase()))
                .collect(),
            install: None,
            added: true,
        });
        catalog.chain(added).collect()
    }

    /// The rule "the files choose": a server is wanted when one of its extensions is
    /// in `extensions` (the project's files) and it is not in `off`.
    fn wanted(
        &self,
        extensions: &HashSet<String>,
        off: &HashSet<String>,
        added: &[Added],
    ) -> Vec<Wanted> {
        Codebase::every_server(added)
            .into_iter()
            .filter(|server| !off.contains(&server.id))
            .filter(|server| extensions.iter().any(|e| server.language_of(e).is_some()))
            .collect()
    }

    /// The servers `project`'s files want now, with `extra` extensions the watcher saw
    /// since the last listing.
    pub(crate) async fn wanted_servers(
        &self,
        project: &core_projects::ProjectId,
        extra: &HashSet<String>,
    ) -> Result<Vec<Wanted>, CodebaseError> {
        let store = &self.inner.store;
        let mut extensions = store.file_extensions(project).await?;
        extensions.extend(extra.iter().cloned());
        let off = store.servers_off().await?;
        let added = store.servers_added().await?;
        Ok(self.wanted(&extensions, &off, &added))
    }

    /// Every catalog and added server, as the asker's project sees it. The state, in
    /// this order: `Off` when switched off; `NotInstalled` when its command is not
    /// found; for a wanted server in an active project, its worker's state; else `Idle`.
    pub async fn servers(&self, asker: Asker<'_>) -> Result<Vec<ServerStatus>, CodebaseError> {
        let project = self.inner.projects.get(asker.home()).await?.id;
        let store = &self.inner.store;
        let extensions = store.file_extensions(&project).await?;
        let off = store.servers_off().await?;
        let added = store.servers_added().await?;
        let wanted: HashSet<String> = self
            .wanted(&extensions, &HashSet::new(), &added)
            .into_iter()
            .map(|w| w.id)
            .collect();
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut out = Vec::new();
        for server in Codebase::every_server(&added) {
            let is_wanted = wanted.contains(&server.id);
            let state = if off.contains(&server.id) {
                ServerState::Off
            } else if Codebase::find_command(&server.command, &path).is_none() {
                ServerState::NotInstalled
            } else if is_wanted {
                let active = self.inner.active.lock().await;
                active
                    .worker(&project)
                    .and_then(|w| w.server_state(&server.id))
                    .unwrap_or(ServerState::Idle)
            } else {
                ServerState::Idle
            };
            out.push(ServerStatus {
                id: server.id,
                added: server.added,
                wanted: is_wanted,
                extensions: server.extensions.into_keys().collect(),
                state,
                install: server.install,
            });
        }
        Ok(out)
    }

    /// Where `command` runs from: itself when it names a path, else the first match
    /// on `path` (a `PATH` value), and on Windows with each `PATHEXT` ending, so an
    /// npm `.cmd` shim is found.
    pub fn find_command(command: &str, path: &OsStr) -> Option<PathBuf> {
        let named = Path::new(command);
        // On Windows a bare name runs only with one of `PATHEXT`'s endings: a file with
        // none (a Unix script beside its `.cmd`) is not a program there.
        let endings: Vec<String> = if cfg!(windows) && named.extension().is_none() {
            let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
            pathext
                .split(';')
                .filter(|e| !e.is_empty())
                .map(str::to_lowercase)
                .collect()
        } else {
            vec![String::new()]
        };
        let try_in = |base: &Path| {
            endings.iter().find_map(|ending| {
                let mut candidate = base.as_os_str().to_owned();
                candidate.push(ending);
                let candidate = PathBuf::from(candidate);
                candidate.is_file().then_some(candidate)
            })
        };
        if named.components().count() > 1 || named.is_absolute() {
            return try_in(named);
        }
        std::env::split_paths(path).find_map(|dir| try_in(&dir.join(command)))
    }

    /// The project's running server `id`, through its worker, after the file changes
    /// the worker gathered before this call reached it; `None` when it is not running.
    pub(crate) async fn running_server(
        &self,
        project: &core_projects::ProjectId,
        id: &str,
    ) -> Option<(Wanted, std::sync::Arc<shadows_lsp::Server>)> {
        let jobs = self.inner.active.lock().await.worker(project)?.jobs();
        let (reply, answer) = tokio::sync::oneshot::channel();
        jobs.send(crate::watch::Job::Lsp(id.to_string(), reply))
            .await
            .ok()?;
        answer.await.ok().flatten()
    }

    /// "CodeServerSwitch", params { server, off }: switches a catalog or added server
    /// off or back on, for every project. A running one stops at its worker's next
    /// recompute; one switched on starts then.
    pub async fn set_server_off(
        &self,
        command_id: String,
        id: &str,
        off: bool,
    ) -> Result<(), CodebaseError> {
        let added = self.inner.store.servers_added().await?;
        if !Codebase::every_server(&added).iter().any(|s| s.id == id) {
            return Err(CodebaseError::InvalidCommand(format!(
                "no server {id} is known"
            )));
        }
        let params = serde_json::json!({ "server": id, "off": off });
        let ctx = CommandContext::user(command_id, "CodeServerSwitch", params);
        let change = ServerChange::Off(id.to_string(), off);
        self.inner
            .store
            .change_servers(&ctx, change)
            .await?
            .map_err(CodebaseError::InvalidCommand)
    }

    /// "CodeServerAdd": a server beside the catalog. Its id must be new and not a
    /// catalog id, its command named, and each extension start with `.`.
    pub async fn add_server(
        &self,
        command_id: String,
        server: AddedServer,
    ) -> Result<ServerStatus, CodebaseError> {
        let refuse = |m: String| Err(CodebaseError::InvalidCommand(m));
        let id = server.id.trim();
        if id.is_empty() || server.command.trim().is_empty() {
            return refuse("a server needs an id and a command".into());
        }
        if Codebase::catalog().iter().any(|e| e.id == id) {
            return refuse(format!(
                "{id} is a catalog server; switch it on or off instead"
            ));
        }
        if server.extensions.is_empty()
            || server
                .extensions
                .iter()
                .any(|e| !e.starts_with('.') || e.len() < 2)
        {
            return refuse("each extension must start with a dot, as .go".into());
        }
        let params = serde_json::json!({
            "id": id, "command": server.command, "args": server.args,
            "extensions": server.extensions,
        });
        let ctx = CommandContext::user(command_id, "CodeServerAdd", params);
        let row = Added {
            id: id.to_string(),
            command: server.command.clone(),
            args: server.args.clone(),
            extensions: server.extensions.clone(),
        };
        self.inner
            .store
            .change_servers(&ctx, ServerChange::Add(row))
            .await?
            .map_err(CodebaseError::InvalidCommand)?;
        let path = std::env::var_os("PATH").unwrap_or_default();
        let state = match Codebase::find_command(&server.command, &path) {
            Some(_) => ServerState::Idle,
            None => ServerState::NotInstalled,
        };
        Ok(ServerStatus {
            id: id.to_string(),
            added: true,
            wanted: false,
            extensions: server.extensions,
            state,
            install: None,
        })
    }

    /// "CodeServerRemove": an added server goes; a catalog one cannot.
    pub async fn remove_server(&self, command_id: String, id: &str) -> Result<(), CodebaseError> {
        let params = serde_json::json!({ "server": id });
        let ctx = CommandContext::user(command_id, "CodeServerRemove", params);
        self.inner
            .store
            .change_servers(&ctx, ServerChange::Remove(id.to_string()))
            .await?
            .map_err(CodebaseError::InvalidCommand)
    }

    /// Test only: the catalog's ids and extensions.
    #[cfg(feature = "test-support")]
    pub fn catalog_for_test() -> Vec<(String, Vec<String>)> {
        Codebase::catalog()
            .iter()
            .map(|e| (e.id.clone(), e.extensions.keys().cloned().collect()))
            .collect()
    }
}
