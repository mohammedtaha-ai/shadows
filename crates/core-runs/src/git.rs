//! One job: running git in a project's folder, through `shadows-process`.
//!
//! Each call is one `git` child with the folder as its working directory and
//! no shell between. Paths are relative to the project folder, with `/`.
use std::path::{Path, PathBuf};

use shadows_process::{ProcessSpec, spawn};
use tokio::io::AsyncReadExt;

/// How many paths go on one command line: Windows limits its length.
const PATHS_PER_CALL: usize = 100;

pub(crate) struct Git {
    root: PathBuf,
}

impl Git {
    pub(crate) fn at(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// The folder, for a test's hook.
    #[cfg(feature = "test-support")]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Runs `git args`, answering its stdout, or its stderr as the error.
    pub(crate) async fn run(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        self.run_with(args, &[]).await
    }

    /// As `run`, with more environment: the temporary index.
    pub(crate) async fn run_with(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<Vec<u8>, String> {
        // The child's environment is built, never inherited: git needs to be
        // found, to read the person's identity, and never to prompt.
        let mut vars: Vec<(String, String)> = ["PATH", "HOME"]
            .iter()
            .filter_map(|name| std::env::var(name).ok().map(|v| (name.to_string(), v)))
            .collect();
        vars.push(("GIT_TERMINAL_PROMPT".into(), "0".into()));
        vars.extend(env.iter().map(|(k, v)| (k.to_string(), v.to_string())));
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

    /// The commit HEAD names, or `None` on a branch with no commit yet.
    pub(crate) async fn head(&self) -> Option<String> {
        let out = self
            .run(&["rev-parse", "--verify", "-q", "HEAD^{commit}"])
            .await
            .ok()?;
        Some(String::from_utf8_lossy(&out).trim().to_string())
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

    /// Runs `git args -- paths`, a bounded number of paths at a time.
    pub(crate) async fn each_paths(
        &self,
        args: &[&str],
        paths: &[String],
        env: &[(&str, &str)],
    ) -> Result<(), String> {
        for chunk in paths.chunks(PATHS_PER_CALL) {
            let mut argv: Vec<&str> = args.to_vec();
            argv.push("--");
            argv.extend(chunk.iter().map(String::as_str));
            self.run_with(&argv, env).await?;
        }
        Ok(())
    }

    /// Whether `path`, or a folder on the way to it, is a link or junction.
    pub(crate) fn through_link(&self, path: &str) -> bool {
        let mut at = self.root.clone();
        path.split('/').filter(|s| !s.is_empty()).any(|segment| {
            at.push(segment);
            std::fs::symlink_metadata(&at).is_ok_and(|m| m.file_type().is_symlink())
        })
    }

    /// Each commit reachable from any ref whose `Shadows-Attempt` trailer
    /// names an attempt: `(attempt, commit)`.
    pub(crate) async fn attempt_commits(&self) -> Result<Vec<(String, String)>, String> {
        let out = self
            .run(&[
                "log",
                "--all",
                "-z",
                "--grep=Shadows-Attempt:",
                "--format=%H %(trailers:key=Shadows-Attempt,valueonly)",
            ])
            .await?;
        let mut found = Vec::new();
        for record in String::from_utf8_lossy(&out).split('\0') {
            let Some((sha, values)) = record.trim().split_once(' ') else {
                continue;
            };
            for value in values.lines().map(str::trim).filter(|v| !v.is_empty()) {
                found.push((value.to_string(), sha.to_string()));
            }
        }
        Ok(found)
    }
}
