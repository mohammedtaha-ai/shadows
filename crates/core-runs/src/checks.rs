//! Confirming an attempt's command checks on the shared project folder.
use crate::git::Git;
use crate::scope::Scope;
use crate::store::{Continued, checks::Finished};
use crate::{Attempt, AttemptId, AttemptOrigin, CheckOutcome, CheckResult, Runs, RunsError};
use core_db::Storage;
use core_plans::TaskCheck;
use core_projects::ProjectId;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

const OUTPUT_LIMIT: usize = 65_536;

impl Runs {
    pub async fn check_results(&self, id: &AttemptId) -> Result<Vec<CheckResult>, RunsError> {
        self.inner.store.get(id).await?;
        self.inner.store.check_results(id).await
    }

    pub(crate) async fn confirm(&self, attempt: &Attempt) {
        if let Err(error) = self.confirm_attempt(attempt).await {
            tracing::warn!(attempt=%attempt.id, %error, "runs.confirm_failed");
        }
    }

    async fn wait_for_quiet(&self, attempt: &Attempt) -> Result<(), RunsError> {
        loop {
            let mut busy = false;
            for thread in self.inner.store.quiet_threads(&attempt.project_id).await? {
                if self
                    .inner
                    .turns
                    .operations(&thread)
                    .await
                    .map_err(|e| RunsError::Unavailable(e.to_string()))?
                    .iter()
                    .any(|op| op.finished_at.is_none())
                {
                    busy = true;
                    break;
                }
            }
            if !busy {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn overlapped(
        &self,
        project: &ProjectId,
        start: &str,
        end: &str,
    ) -> Result<bool, RunsError> {
        for thread in self.inner.store.executor_threads(project).await? {
            let operations = self
                .inner
                .turns
                .operations(&thread)
                .await
                .map_err(|e| RunsError::Unavailable(e.to_string()))?;
            if operations.iter().any(|op| {
                op.started_at.as_deref().is_none_or(|started| started < end)
                    && op
                        .finished_at
                        .as_deref()
                        .is_none_or(|finished| finished > start)
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn confirm_attempt(&self, attempt: &Attempt) -> Result<(), RunsError> {
        if self.inner.store.continued(&attempt.id).await? {
            return Ok(());
        }
        let checks = self.inner.store.check_specs(&attempt.id).await?;
        if checks.is_empty() {
            return self.after_checks(attempt).await;
        }
        let recorded = self.inner.store.check_results(&attempt.id).await?;
        if checks.iter().all(|(task, _, check)| {
            recorded
                .iter()
                .any(|row| row.for_task == *task && row.id == check.id && row.finished_at.is_some())
        }) {
            return self.after_checks(attempt).await;
        }
        self.wait_for_quiet(attempt).await?;
        let lock = self
            .inner
            .checking
            .lock()
            .await
            .entry(attempt.project_id.clone())
            .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        let held = lock.lock().await;
        self.wait_for_quiet(attempt).await?;
        let folder = self.folder(&attempt.project_id).await?;
        let git = Git::at(&folder);
        let prefix = git.prefix().await.ok_or(RunsError::NotGit)?;
        let scopes = self
            .inner
            .store
            .attempt_writes(&attempt.project_id)
            .await?
            .iter()
            .map(|s| Scope::new(s).map_err(RunsError::BadScope))
            .collect::<Result<Vec<_>, _>>()?;
        for (task, ordinal, check) in checks {
            for repetition in 0..3 {
                self.wait_for_quiet(attempt).await?;
                let dirty = git.changed(&prefix).await.map_err(RunsError::Unavailable)?;
                let leftovers: Vec<_> = dirty
                    .iter()
                    .filter(|p| scopes.iter().any(|s| s.admits(p)))
                    .cloned()
                    .collect();
                let before = git.head().await;
                self.inner
                    .store
                    .start_check(&attempt.id, task, &check, ordinal, before.clone())
                    .await?;
                if !leftovers.is_empty() {
                    self.inner
                        .store
                        .finish_check(
                            &attempt.id,
                            task,
                            &check.id,
                            Finished {
                                outcome: CheckOutcome::Inconclusive,
                                exit_code: None,
                                head: before,
                                output: format!(
                                    "uncommitted changes inside task writes: {}",
                                    leftovers.join(", ")
                                ),
                                truncated: false,
                            },
                        )
                        .await?;
                    break;
                }
                let hashes = git.hash(&dirty).await.map_err(RunsError::Unavailable)?;
                let start = Storage::now();
                let mut result = Self::execute_check(&folder, &check).await;
                let end = Storage::now();
                result.head = git.head().await;
                let after = git.changed(&prefix).await.map_err(RunsError::Unavailable)?;
                let after_hashes = git.hash(&after).await.map_err(RunsError::Unavailable)?;
                if self.overlapped(&attempt.project_id, &start, &end).await? {
                    if repetition < 2 {
                        continue;
                    }
                    result.outcome = CheckOutcome::Inconclusive;
                    result.output = "another executor's turn ran during the check".into();
                } else if !matches!(
                    result.outcome,
                    CheckOutcome::NotRun | CheckOutcome::TimedOut
                ) && (before != result.head || dirty != after || hashes != after_hashes)
                {
                    result.outcome = CheckOutcome::Inconclusive;
                }
                self.inner
                    .store
                    .finish_check(&attempt.id, task, &check.id, result)
                    .await?;
                break;
            }
        }
        drop(held);
        self.after_checks(attempt).await
    }

    async fn execute_check(root: &Path, check: &TaskCheck) -> Finished {
        let mut result = Finished {
            outcome: CheckOutcome::NotRun,
            exit_code: None,
            head: None,
            output: String::new(),
            truncated: false,
        };
        let folder = root.join(check.folder.as_deref().unwrap_or("."));
        let canonical = match (root.canonicalize(), folder.canonicalize()) {
            (Ok(root), Ok(folder)) if folder.starts_with(&root) => folder,
            _ => {
                result.output = "check folder is unavailable or outside the project".into();
                return result;
            }
        };
        let Some(program) = check.run.first() else {
            result.output = "check has no program".into();
            return result;
        };
        let env = std::env::vars()
            .filter(|(name, _)| {
                let key = name.to_ascii_uppercase();
                !["KEY", "TOKEN", "SECRET", "PASSWORD"]
                    .iter()
                    .any(|s| key.contains(s))
                    && (["PATH", "HOME"].contains(&key.as_str())
                        || [
                            "GO",
                            "CARGO_",
                            "RUSTUP_",
                            "JAVA_",
                            "NODE_",
                            "NPM_",
                            "PYTHON",
                            "VIRTUAL_ENV",
                            "COMPOSER_",
                            "PHP",
                        ]
                        .iter()
                        .any(|s| key.starts_with(s)))
            })
            .collect();
        let mut handle = match shadows_process::spawn(shadows_process::ProcessSpec {
            executable: PathBuf::from(program),
            args: check.run[1..].to_vec(),
            cwd: canonical,
            env,
            capture_stdout: true,
            pipe_stdin: true,
        }) {
            Ok(h) => h,
            Err(e) => {
                result.output = e.to_string();
                return result;
            }
        };
        let Some((stdin, stdout, stderr)) = handle.take_stdio() else {
            result.output = "check output was unavailable".into();
            return result;
        };
        drop(stdin);
        let (send, mut receive) = tokio::sync::mpsc::channel(16);
        let out = tokio::spawn(read_check_output(stdout, send.clone()));
        let err = tokio::spawn(read_check_output(stderr, send.clone()));
        drop(send);
        let collect = async {
            let mut bytes = VecDeque::new();
            let mut truncated = false;
            while let Some(chunk) = receive.recv().await {
                bytes.extend(chunk);
                if bytes.len() > OUTPUT_LIMIT {
                    bytes.drain(..bytes.len() - OUTPUT_LIMIT);
                    truncated = true;
                }
            }
            (bytes.into_iter().collect::<Vec<_>>(), truncated)
        };
        let wait = async {
            match tokio::time::timeout(check.timeout(), handle.wait()).await {
                Ok(status) => (false, status.map_err(|e| e.to_string())),
                Err(_) => {
                    let killed = handle.terminate_tree().map_err(|e| e.to_string());
                    let status = if killed.is_ok() {
                        handle.wait().await.map_err(|e| e.to_string())
                    } else {
                        killed.map(|_| unreachable!())
                    };
                    (true, status)
                }
            }
        };
        let ((timed_out, status), (bytes, truncated)) = tokio::join!(wait, collect);
        let reads = tokio::join!(out, err);
        result.output = String::from_utf8_lossy(&bytes).into_owned();
        result.truncated = truncated;
        result.outcome = if timed_out {
            CheckOutcome::TimedOut
        } else {
            match status {
                Ok(status) => {
                    result.exit_code = status.code();
                    if status.success() {
                        CheckOutcome::Passed
                    } else {
                        CheckOutcome::Failed
                    }
                }
                Err(error) => {
                    result.output.push_str(&error);
                    CheckOutcome::NotRun
                }
            }
        };
        if !matches!(reads, (Ok(Ok(())), Ok(Ok(())))) {
            result.outcome = CheckOutcome::Inconclusive;
            result
                .output
                .push_str("\ncheck output could not be fully read");
        }
        result
    }

    async fn after_checks(&self, attempt: &Attempt) -> Result<(), RunsError> {
        if self.inner.store.continued(&attempt.id).await? {
            return Ok(());
        }
        let results = self.check_results(&attempt.id).await?;
        if results
            .iter()
            .any(|c| c.outcome != Some(CheckOutcome::Passed))
            && self.inner.store.returns_since(&attempt.id).await? < 2
        {
            let sha = attempt.commit.as_deref().unwrap_or_default();
            let details = results
                .iter()
                .map(|c| {
                    format!(
                        "T{} {} {:?}\n{}",
                        c.for_task,
                        c.id,
                        c.outcome,
                        c.output
                            .as_deref()
                            .unwrap_or_default()
                            .chars()
                            .take(4000)
                            .collect::<String>()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let prompt = format!(
                "Shadows ran T{}'s checks on commit {sha} once the project was free, \
                and they did not all pass:\n{details}\nThe project is free now: run the checks and \
                finish inside the task's writes, then commit and report as before. \
                If nothing needs to change, report the same commit {sha}. {}",
                attempt.task,
                crate::COMMIT
            );
            let next = self
                .inner
                .store
                .continue_attempt(
                    &attempt.id,
                    Continued {
                        addition: prompt.clone(),
                        start_head: Git::at(&self.folder(&attempt.project_id).await?)
                            .head()
                            .await,
                        origin: AttemptOrigin::Checks,
                        command_id: format!("checks-{}", attempt.id),
                    },
                )
                .await;
            match next {
                Ok(next) => {
                    self.run_turn(next, prompt).await?;
                    return Ok(());
                }
                Err(error) => tracing::warn!(attempt=%attempt.id,%error,"runs.return_refused"),
            }
        }
        self.notify(attempt).await?;
        self.inner.store.mark_notice_sent(&attempt.id).await
    }
}

async fn read_check_output<T: AsyncRead + Unpin>(
    mut stream: T,
    send: tokio::sync::mpsc::Sender<Vec<u8>>,
) -> std::io::Result<()> {
    let mut buf = [0; 8192];
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        if send.send(buf[..n].to_vec()).await.is_err() {
            return Ok(());
        }
    }
}
