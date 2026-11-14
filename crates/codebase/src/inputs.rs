//! One job: the real-path check of a project path, whether it exists or not.
//!
//! The containment check is the one `lsp`'s file, a report's paths and a
//! task's required inputs all pass. An absent path is checked through its
//! nearest existing folder, so a missing file under a link out of the project
//! is refused, never answered as absent.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::scope::Asker;
use crate::{Codebase, CodebaseError};

impl Codebase {
    /// The paths of `paths`, project-relative in the asker's own project, that
    /// name no file there: sorted, each once. Every path is checked first: it
    /// must be inside the project, name no secret and, where it exists, be a
    /// regular file by its real path. A path that cannot be read is an error,
    /// never absent.
    pub async fn missing_inputs(
        &self,
        asker: Asker<'_>,
        paths: &[String],
    ) -> Result<Vec<String>, CodebaseError> {
        let dir = self
            .folder(asker.home())
            .await
            .ok_or_else(|| CodebaseError::InvalidCommand("the project has no folder".into()))?;
        let mut checked = Vec::with_capacity(paths.len());
        for path in paths {
            let rel = Codebase::inside(path)?;
            if rel.split('/').any(Codebase::secret_name) {
                return Err(CodebaseError::InvalidCommand(format!(
                    "{rel} holds a secret; a task cannot require it"
                )));
            }
            checked.push(rel);
        }
        checked.sort_unstable();
        checked.dedup();
        let mut missing = Vec::new();
        for rel in checked {
            let full = Codebase::checked_path(&dir, &rel, true).await?;
            match tokio::fs::symlink_metadata(&full).await {
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => missing.push(rel),
                Err(error) => return Err(Codebase::unreadable(&rel, &error)),
            }
        }
        Ok(missing)
    }

    /// `relative` under `dir`, refused unless it is inside the folder's real
    /// path: a link or junction out of the project is never followed. A path
    /// on disk must be a regular file. With `allow_missing`, an absent path
    /// passes when its nearest existing ancestor is inside the folder.
    pub(crate) async fn checked_path(
        dir: &Path,
        relative: &str,
        allow_missing: bool,
    ) -> Result<PathBuf, CodebaseError> {
        let refuse = || CodebaseError::InvalidCommand("the file is not in the project".into());
        let outside =
            || CodebaseError::InvalidCommand("the path must be inside the project".into());
        let rel = Codebase::inside(relative)?;
        if rel.is_empty() {
            return Err(outside());
        }
        let canonical_dir = tokio::fs::canonicalize(dir).await.map_err(|_| refuse())?;
        let full = dir.join(&rel);
        let parts: Vec<&str> = rel.split('/').collect();
        for depth in (0..=parts.len()).rev() {
            let prefix = parts[..depth].join("/");
            let here = dir.join(&prefix);
            match tokio::fs::symlink_metadata(&here).await {
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    if !allow_missing {
                        return Err(refuse());
                    }
                    continue;
                }
                Err(error) => return Err(Codebase::unreadable(&prefix, &error)),
            }
            let real = tokio::fs::canonicalize(&here)
                .await
                .map_err(|error| Codebase::unreadable(&prefix, &error))?;
            let inside = if prefix.is_empty() {
                real == canonical_dir
            } else {
                Codebase::real_path_is(&real, &canonical_dir, &prefix)
            };
            if !inside {
                return Err(outside());
            }
            if depth == parts.len() {
                let meta = tokio::fs::metadata(&real)
                    .await
                    .map_err(|error| Codebase::unreadable(&rel, &error))?;
                if !meta.is_file() {
                    return Err(refuse());
                }
            } else if !tokio::fs::metadata(&real).await.is_ok_and(|m| m.is_dir()) {
                return Err(refuse());
            }
            return Ok(full);
        }
        Err(refuse())
    }

    fn unreadable(path: &str, error: &std::io::Error) -> CodebaseError {
        CodebaseError::InvalidCommand(format!("{path} could not be read: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::Codebase;

    #[tokio::test]
    async fn an_absent_path_under_a_real_folder_passes() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        let full = Codebase::checked_path(tmp.path(), "src/new/a.rs", true)
            .await
            .unwrap();
        assert_eq!(full, tmp.path().join("src/new/a.rs"));
        assert!(
            Codebase::checked_path(tmp.path(), "src/new/a.rs", false)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_folder_is_not_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        assert!(
            Codebase::checked_path(tmp.path(), "src", true)
                .await
                .is_err()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_absent_path_under_a_link_out_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let (project, away) = (tmp.path().join("p"), tmp.path().join("away"));
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&away).unwrap();
        std::os::unix::fs::symlink(&away, project.join("out")).unwrap();
        let refused = Codebase::checked_path(&project, "out/a.rs", true).await;
        assert!(refused.is_err(), "{refused:?}");
    }
}
