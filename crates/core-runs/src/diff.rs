//! One job: reading what an attempt's commit changed, file by file.
use crate::git::Git;
use crate::model::{AttemptFile, FileDiff};
use crate::scope::Scope;
use crate::{AttemptId, Runs, RunsError};
use shadows_agent::events::{DiffHunk, DiffLine, DiffOp};

/// A patch longer than this is cut, with a marker saying how much was left.
pub(crate) const MAX_LINES: usize = 2_000;
const MAX_BYTES: usize = 200_000;

impl Runs {
    /// Reads a path inside the attempt's writes from its accepted commit.
    pub async fn file(&self, attempt: &AttemptId, path: &str) -> Result<AttemptFile, RunsError> {
        let attempt = self.inner.store.get(attempt).await?;
        let commit = attempt.commit.ok_or(RunsError::NotFound)?;
        let path = path.trim().replace('\\', "/");
        let git = Git::at(&self.folder(&attempt.project_id).await?);
        let prefix = git.prefix().await.ok_or(RunsError::NotGit)?;
        let writes = self.inner.store.writes(&attempt.id).await?;
        let matcher = Scope::new(&writes).map_err(RunsError::BadScope)?;
        if !matcher.admits_under(&prefix, &path) {
            return Err(RunsError::OutsideWrites { paths: vec![path] });
        }
        let mut file = AttemptFile {
            path,
            text: None,
            cut: false,
        };
        let blob = git
            .blob(&commit, &file.path)
            .await
            .map_err(RunsError::Unavailable)?;
        let Some(blob) = blob else {
            if git
                .changes(&commit)
                .await
                .map_err(RunsError::Unavailable)?
                .iter()
                .any(|change| change.path == file.path)
            {
                return Ok(file);
            }
            return Err(RunsError::NotFound);
        };
        if blob[..blob.len().min(8_000)].contains(&0) {
            return Ok(file);
        }
        let Ok(text) = std::str::from_utf8(&blob) else {
            return Ok(file);
        };
        let mut end = 0;
        for (number, line) in text.split_inclusive('\n').enumerate() {
            if number == MAX_LINES || end + line.len() > MAX_BYTES {
                break;
            }
            end += line.len();
        }
        file.cut = end < text.len();
        file.text = Some(text[..end].to_string());
        Ok(file)
    }

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
            let patch = String::from_utf8_lossy(&patch);
            files.push(FileDiff {
                path: path.to_string(),
                added: added.parse().unwrap_or(0),
                removed: removed.parse().unwrap_or(0),
                patch: Self::bounded(&patch),
                hunks: Self::hunks(path, &patch),
            });
        }
        Ok(files)
    }

    /// The patch's hunks, read before it is cut; their lines stop at
    /// `MAX_LINES` in all, as the patch does.
    fn hunks(path: &str, patch: &str) -> Vec<DiffHunk> {
        let mut hunks: Vec<DiffHunk> = Vec::new();
        let mut kept = 0;
        for line in patch.lines() {
            if let Some(header) = line.strip_prefix("@@ -") {
                let mut ranges = header.split(' ');
                let start = |range: Option<&str>| {
                    range
                        .and_then(|r| r.split(',').next())
                        .and_then(|n| n.parse::<u32>().ok())
                };
                let old_start = start(ranges.next());
                let new_start = start(ranges.next().map(|r| r.trim_start_matches('+')));
                hunks.push(DiffHunk {
                    path: path.to_string(),
                    old_start,
                    new_start,
                    lines: Vec::new(),
                });
                continue;
            }
            let Some(hunk) = hunks.last_mut() else {
                continue;
            };
            let op = match line.chars().next() {
                Some(' ') => DiffOp::Context,
                Some('+') => DiffOp::Add,
                Some('-') => DiffOp::Remove,
                _ => continue,
            };
            if kept == MAX_LINES {
                break;
            }
            kept += 1;
            hunk.lines.push(DiffLine {
                op,
                text: line[1..].to_string(),
            });
        }
        hunks
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
