//! One job: what a code question may read: the projects
//! in its scope, and a path inside one. `Code`'s questions call these first.

use crate::CodeError;
use core_projects::ProjectId;

use super::Code;
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

impl Code {
    /// The projects a question may read: the asker's project, then its direct
    /// links, by slug. `only` narrows it to one of them. A project with no
    /// folder is in it; its status says so.
    pub(super) async fn projects(
        &self,
        asker: &Asker<'_>,
        only: Option<&str>,
    ) -> Result<Vec<ScopeRow>, CodeError> {
        let refuse: fn(String) -> CodeError = match asker {
            Asker::Person(_) => CodeError::InvalidCommand,
            Asker::Grant(_) => CodeError::GrantScope,
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

impl Code {
    /// A relative path inside the project, with '/', or the refusal
    /// INVALID_COMMAND "the path must be inside the project".
    pub(super) fn inside(path: &str) -> Result<String, CodeError> {
        let p = path.replace('\\', "/");
        let p = p.trim_start_matches("./").trim_end_matches('/');
        let bad = p.starts_with('/') || p.contains(':') || p.split('/').any(|c| c == "..");
        if bad {
            return Err(CodeError::InvalidCommand(
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
