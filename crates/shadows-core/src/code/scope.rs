//! One job: what a code question may read (spec §15.5, §15.6): the projects
//! in its scope, and a path inside one. `Code`'s questions call these first.

use crate::db::Storage;
use crate::error::{CoreError, ErrorCode};
use crate::grants::Grant;
use crate::projects::ProjectId;

use super::store::ScopeRow;

/// Who asks a question, which decides the project it starts from and the
/// code a refusal carries.
#[derive(Debug, Clone, Copy)]
pub enum Asker<'a> {
    /// A person, over HTTP: refusals are `INVALID_COMMAND`.
    Person(&'a ProjectId),
    /// An agent holding an MCP grant: refusals are `GRANT_SCOPE`.
    Grant(&'a Grant),
}

/// The projects a question may read: the asker's project, then its direct
/// links, by slug. `only` narrows it to one of them. A project with no
/// folder is in it; its status says so.
pub(super) async fn projects(
    storage: &Storage,
    asker: &Asker<'_>,
    only: Option<&str>,
) -> Result<Vec<ScopeRow>, CoreError> {
    let (home, code) = match asker {
        Asker::Person(p) => (*p, ErrorCode::InvalidCommand),
        Asker::Grant(g) => (&g.project_id, ErrorCode::GrantScope),
    };
    let scope = storage.code_scope(home).await?;
    let Some(slug) = only else {
        return Ok(scope);
    };
    let named: Vec<ScopeRow> = scope.into_iter().filter(|(_, s, _)| s == slug).collect();
    if named.is_empty() {
        return Err(CoreError::Refused {
            code,
            message: format!("the project {slug} is not linked to this project"),
        });
    }
    Ok(named)
}

/// A relative path inside the project, with '/', or the refusal
/// INVALID_COMMAND "the path must be inside the project".
pub(super) fn inside(path: &str) -> Result<String, CoreError> {
    let p = path.replace('\\', "/");
    let p = p.trim_start_matches("./").trim_end_matches('/');
    let bad = p.starts_with('/') || p.contains(':') || p.split('/').any(|c| c == "..");
    if bad {
        return Err(CoreError::Refused {
            code: ErrorCode::InvalidCommand,
            message: "the path must be inside the project".into(),
        });
    }
    Ok(if p == "." {
        String::new()
    } else {
        p.to_string()
    })
}
