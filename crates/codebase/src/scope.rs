//! One job: what a code question may read: the projects
//! in its scope, and a path inside one. `Codebase`'s questions call these first.

use crate::CodebaseError;
use core_projects::ProjectId;

use super::Codebase;
use super::store::ScopeRow;

/// Who asks a question, which decides the project it starts from and the
/// code a refusal carries.
#[derive(Debug, Clone, Copy)]
pub enum Asker<'a> {
    /// A person, over HTTP: refusals are `INVALID_COMMAND`.
    Person(&'a ProjectId),
    /// An agent holding an MCP grant: refusals are `GRANT_SCOPE`.
    Grant(&'a ProjectId),
}

impl Asker<'_> {
    /// The project a question starts from, which it also touches.
    pub(super) fn home(&self) -> &ProjectId {
        match self {
            Asker::Person(p) | Asker::Grant(p) => p,
        }
    }
}

impl Codebase {
    /// The projects a question may read: the asker's project, then its direct
    /// links, by slug. `only` narrows it to one of them. A project with no
    /// folder is in it; its status says so.
    pub(super) async fn projects(
        &self,
        asker: &Asker<'_>,
        only: Option<&str>,
    ) -> Result<Vec<ScopeRow>, CodebaseError> {
        let refuse: fn(String) -> CodebaseError = match asker {
            Asker::Person(_) => CodebaseError::InvalidCommand,
            Asker::Grant(_) => CodebaseError::GrantScope,
        };
        let scope = self.inner.store.code_scope(asker.home()).await?;
        let Some(slug) = only else {
            return Ok(scope);
        };
        let named: Vec<ScopeRow> = scope.into_iter().filter(|(_, s, _)| s == slug).collect();
        if named.is_empty() {
            return Err(refuse(format!(
                "the project {slug} is not linked to this project"
            )));
        }
        Ok(named)
    }
}

impl Codebase {
    /// A relative path inside the project, with '/', or the refusal
    /// INVALID_COMMAND "the path must be inside the project".
    pub(super) fn inside(path: &str) -> Result<String, CodebaseError> {
        let p = path.replace('\\', "/");
        let p = p.trim_start_matches("./").trim_end_matches('/');
        let bad = p.starts_with('/') || p.contains(':') || p.split('/').any(|c| c == "..");
        if bad {
            return Err(CodebaseError::InvalidCommand(
                "the path must be inside the project".into(),
            ));
        }
        Ok(if p == "." {
            String::new()
        } else {
            p.to_string()
        })
    }
}
