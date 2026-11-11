//! One job: running git in a project's folder, through `shadows-process`.
//!
//! Each call is one `git` child with the folder as its working directory and
//! no shell between. Paths are relative to the project folder, with `/`.
use std::path::{Path, PathBuf};

use shadows_process::{ProcessSpec, spawn};
use tokio::io::AsyncReadExt;

pub(crate) struct Git {
    root: PathBuf,
}

/// One path a commit changes, relative to the repository's top.
pub(crate) struct Change {
    pub(crate) path: String,
    /// Git found the file binary and the commit adds or modifies it.
    pub(crate) binary: bool,
}

impl Git {
    pub(crate) fn at(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// Runs `git args`, answering its stdout, or its stderr as the error.
    pub(crate) async fn run(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        // The child's environment is built, never inherited: git needs to be
        // found, to read the person's identity, and never to prompt.
        let mut vars: Vec<(String, String)> = ["PATH", "HOME"]
            .iter()
            .filter_map(|name| std::env::var(name).ok().map(|v| (name.to_string(), v)))
            .collect();
        vars.push(("GIT_TERMINAL_PROMPT".into(), "0".into()));
        let mut argv = vec!["--literal-pathspecs".to_string()];
        argv.extend(args.iter().map(|a| a.to_string()));
        let mut handle = spawn(ProcessSpec {
            executable: PathBuf::from("git"),
            args: argv,
            cwd: self.root.clone(),
            env: vars,
            capture_stdout: true,
            pipe_stdin: true,
        })
        .map_err(|error| format!("git could not start: {error}"))?;
        let (stdin, mut stdout, mut stderr) = handle
            .take_stdio()
            .ok_or_else(|| "git's output could not be read".to_string())?;
        drop(stdin);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let (read_out, read_err) =
            tokio::join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut err));
        read_out.and(read_err).map_err(|error| error.to_string())?;
        let status = handle.wait().await.map_err(|error| error.to_string())?;
        if status.success() {
            Ok(out)
        } else {
            let first = args.first().copied().unwrap_or_default();
            Err(format!(
                "git {first}: {}",
                String::from_utf8_lossy(&err).trim()
            ))
        }
    }

    /// The folder's place in its repository (`""` at the top, `sub/` below
    /// it), or `None` when the folder is not inside a git work tree.
    pub(crate) async fn prefix(&self) -> Option<String> {
        let inside = self
            .run(&["rev-parse", "--is-inside-work-tree"])
            .await
            .ok()?;
        if String::from_utf8_lossy(&inside).trim() != "true" {
            return None;
        }
        let prefix = self.run(&["rev-parse", "--show-prefix"]).await.ok()?;
        Some(String::from_utf8_lossy(&prefix).trim().to_string())
    }

    /// Makes the folder a git repository.
    pub(crate) async fn init(&self) -> Result<(), String> {
        self.run(&["init", "-q"]).await.map(|_| ())
    }

    /// Records an empty commit and answers its full SHA. The identity is
    /// fixed, so the commit does not depend on the person's git configuration
    /// and says who made it.
    pub(crate) async fn commit_empty(&self, message: &str) -> Result<String, String> {
        self.run(&[
            "-c",
            "user.name=Shadows",
            "-c",
            "user.email=shadows@localhost",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            message,
        ])
        .await?;
        self.head()
            .await
            .ok_or_else(|| "the first commit was not recorded".to_string())
    }

    /// The commit HEAD names, or `None` on a branch with no commit yet.
    pub(crate) async fn head(&self) -> Option<String> {
        self.commit("HEAD").await
    }

    /// Reads only a blob at a validated commit and repository path, never the working tree.
    pub(crate) async fn blob(&self, commit: &str, path: &str) -> Result<Option<Vec<u8>>, String> {
        let object = format!("{commit}:{path}");
        let kind = self.run(&["cat-file", "-t", &object]).await;
        if !kind.is_ok_and(|kind| kind == b"blob\n") {
            return Ok(None);
        }
        self.run(&["cat-file", "blob", &object]).await.map(Some)
    }

    /// The full SHA of the commit `name` names, or `None` when it names none.
    pub(crate) async fn commit(&self, name: &str) -> Option<String> {
        let out = self
            .run(&["rev-parse", "--verify", "-q", &format!("{name}^{{commit}}")])
            .await
            .ok()?;
        Some(String::from_utf8_lossy(&out).trim().to_string())
    }

    /// Whether `commit` is `of` or one of its ancestors.
    pub(crate) async fn reaches(&self, of: &str, commit: &str) -> bool {
        self.run(&["merge-base", "--is-ancestor", commit, of])
            .await
            .is_ok()
    }

    /// Every path `commit` changes against its first parent (against nothing
    /// for a root commit), a rename as both ends.
    pub(crate) async fn changes(&self, commit: &str) -> Result<Vec<Change>, String> {
        let parent = self.commit(&format!("{commit}^1")).await;
        let mut args = vec!["diff-tree", "-r", "-z", "--no-renames", "--no-commit-id"];
        let trees: Vec<&str> = match &parent {
            Some(parent) => vec![parent.as_str(), commit],
            None => vec!["--root", commit],
        };
        let deleted = {
            let mut argv = args.clone();
            argv.extend(["--name-only", "--diff-filter=D"]);
            argv.extend(&trees);
            self.run(&argv).await?
        };
        let deleted: Vec<&[u8]> = deleted.split(|b| *b == 0).collect();
        args.push("--numstat");
        args.extend(&trees);
        let out = self.run(&args).await?;
        let mut changes = Vec::new();
        for record in String::from_utf8_lossy(&out).split('\0') {
            let mut fields = record.trim_start_matches('\n').splitn(3, '\t');
            let (Some(added), Some(_), Some(path)) = (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            // numstat counts no lines of a binary file; a deleted one is no
            // binary the commit brings.
            let binary = added == "-" && !deleted.contains(&path.as_bytes());
            changes.push(Change {
                path: path.to_string(),
                binary,
            });
        }
        Ok(changes)
    }

    /// Every changed path inside the folder that git does not ignore:
    /// modified, added, deleted, untracked. A rename is its origin and its
    /// destination, each listed.
    pub(crate) async fn changed(&self, prefix: &str) -> Result<Vec<String>, String> {
        let out = self
            .run(&[
                "status",
                "--porcelain=v1",
                "-z",
                "-uall",
                "--no-renames",
                "--",
                ".",
            ])
            .await?;
        let mut paths = Vec::new();
        for record in out.split(|b| *b == 0).filter(|r| r.len() > 3) {
            let path = String::from_utf8_lossy(&record[3..]).to_string();
            if let Some(inside) = path.strip_prefix(prefix)
                && !paths.iter().any(|p: &String| p == inside)
            {
                paths.push(inside.to_string());
            }
        }
        Ok(paths)
    }
}
