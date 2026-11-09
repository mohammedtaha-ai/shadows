//! One job: reading what an attempt's commit changed, file by file.
use crate::git::Git;
use crate::model::FileDiff;
use crate::{AttemptId, Runs, RunsError};

/// A patch longer than this is cut, with a marker saying how much was left.
const MAX_LINES: usize = 2_000;

impl Runs {
    /// Each file of the attempt's commit with its counts and patch; nothing
    /// for an attempt with no commit. A binary file counts no lines.
    pub async fn diff(&self, attempt: &AttemptId) -> Result<Vec<FileDiff>, RunsError> {
        let attempt = self.inner.store.get(attempt).await?;
        let Some(commit) = attempt.commit else {
            return Ok(Vec::new());
        };
        let git = Git::at(&self.folder(&attempt.project_id).await?);
        let numstat = git
            .run(&[
                "show",
                "--format=",
                "--numstat",
                "-z",
                "--no-renames",
                &commit,
            ])
            .await
            .map_err(RunsError::Unavailable)?;
        let mut files = Vec::new();
        for record in String::from_utf8_lossy(&numstat).split('\0') {
            let mut fields = record.trim_start_matches('\n').splitn(3, '\t');
            let (Some(added), Some(removed), Some(path)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let patch = git
                .run(&[
                    "show",
                    "--format=",
                    "--no-color",
                    "--no-ext-diff",
                    "--no-renames",
                    &commit,
                    "--",
                    path,
                ])
                .await
                .map_err(RunsError::Unavailable)?;
            files.push(FileDiff {
                path: path.to_string(),
                added: added.parse().unwrap_or(0),
                removed: removed.parse().unwrap_or(0),
                patch: Self::bounded(&String::from_utf8_lossy(&patch)),
            });
        }
        Ok(files)
    }

    fn bounded(patch: &str) -> String {
        let lines: Vec<&str> = patch.lines().collect();
        if lines.len() <= MAX_LINES {
            return patch.to_string();
        }
        let mut cut = lines[..MAX_LINES].join("\n");
        cut.push_str(&format!(
            "\n[cut: {} more lines]\n",
            lines.len() - MAX_LINES
        ));
        cut
    }
}
