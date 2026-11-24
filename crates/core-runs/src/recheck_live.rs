//! One job: a running recheck batch's output, held in memory for its watchers.
//!
//! Nothing here is journaled or written to the database: the stored output is
//! written when a row finishes, by the store.
use crate::checks::OUTPUT_LIMIT;
use crate::{AttemptId, CheckOutcome, RecheckOutput, RecheckRow, Runs, RunsError};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use tokio::sync::broadcast;

/// The live state of every running batch, by batch id.
#[derive(Default)]
pub(crate) struct LiveRechecks {
    batches: Mutex<HashMap<String, LiveBatch>>,
}

struct LiveBatch {
    sender: broadcast::Sender<RecheckOutput>,
    /// The running row and what it has printed so far, capped as the stored output is.
    current: Option<(u32, AttemptId, String, String)>,
}

impl LiveBatch {
    fn new() -> Self {
        Self {
            sender: broadcast::channel(256).0,
            current: None,
        }
    }
}

/// One running row's sink: the check's output goes to the batch's watchers as it is printed.
pub(crate) struct LiveRow {
    runs: Runs,
    batch: String,
    /// The bytes of a character split across two reads.
    pending: Vec<u8>,
}

impl LiveRow {
    pub(crate) fn chunk(&mut self, bytes: &[u8]) {
        self.pending.extend_from_slice(bytes);
        let text = match std::str::from_utf8(&self.pending) {
            Ok(text) => {
                let text = text.to_string();
                self.pending.clear();
                text
            }
            Err(error) if error.error_len().is_none() => {
                let valid = error.valid_up_to();
                let text = String::from_utf8_lossy(&self.pending[..valid]).into_owned();
                self.pending.drain(..valid);
                text
            }
            Err(_) => String::from_utf8_lossy(&std::mem::take(&mut self.pending)).into_owned(),
        };
        if !text.is_empty() {
            self.runs.live_text(&self.batch, text);
        }
    }
}

/// A watcher of one recheck batch: what the running row printed so far, then each new event,
/// until the batch ends.
pub struct RecheckWatch {
    backlog: VecDeque<RecheckOutput>,
    live: Option<broadcast::Receiver<RecheckOutput>>,
}

impl RecheckWatch {
    /// The next event; none once `End` was given.
    pub async fn next(&mut self) -> Option<RecheckOutput> {
        if let Some(event) = self.backlog.pop_front() {
            if event == RecheckOutput::End {
                self.live = None;
            }
            return Some(event);
        }
        let live = self.live.as_mut()?;
        let event = match live.recv().await {
            Ok(event) => event,
            Err(broadcast::error::RecvError::Lagged(_)) => RecheckOutput::Lagged,
            Err(broadcast::error::RecvError::Closed) => RecheckOutput::End,
        };
        if event == RecheckOutput::End {
            self.live = None;
        }
        Some(event)
    }
}

impl Runs {
    /// The rows of recheck batch `recheck`, in plan version, task and check order.
    pub async fn recheck_rows(&self, recheck: &str) -> Result<Vec<RecheckRow>, RunsError> {
        if self.inner.store.recheck_finished(recheck).await?.is_none() {
            return Err(RunsError::NotFound);
        }
        let rows = self.inner.store.recheck_rows(recheck).await?;
        rows.iter()
            .map(|row| {
                let check = Self::recheck_result(row)?;
                Ok(RecheckRow {
                    workflow_id: core_plans::WorkflowId::from_recorded(row.workflow_id.clone()),
                    version: row.version,
                    task: row.task,
                    attempt_id: AttemptId::from_recorded(row.attempt_id.clone()),
                    check_id: row.check_id.clone(),
                    command: check.run.join(" "),
                    outcome: check.outcome.filter(|_| row.finished_at.is_some()),
                    started_at: row.started_at.clone(),
                    finished_at: row.finished_at.clone(),
                    output: row.output.clone().filter(|_| row.finished_at.is_some()),
                    truncated: row.truncated,
                })
            })
            .collect()
    }

    /// Watches recheck batch `recheck`: the running row's output so far, then its new output,
    /// each row's start and finish, and the end; a finished batch answers the end at once.
    pub async fn watch_recheck(&self, recheck: &str) -> Result<RecheckWatch, RunsError> {
        let Some(finished) = self.inner.store.recheck_finished(recheck).await? else {
            return Err(RunsError::NotFound);
        };
        let ended = RecheckWatch {
            backlog: VecDeque::from([RecheckOutput::End]),
            live: None,
        };
        if finished.is_some() {
            return Ok(ended);
        }
        let watch = {
            let mut batches = self.inner.live_rechecks.batches.lock().unwrap();
            let batch = batches
                .entry(recheck.to_string())
                .or_insert_with(LiveBatch::new);
            let mut backlog = VecDeque::new();
            if let Some((task, attempt_id, check_id, text)) = &batch.current {
                backlog.push_back(RecheckOutput::Started {
                    task: *task,
                    attempt_id: attempt_id.clone(),
                    check_id: check_id.clone(),
                });
                if !text.is_empty() {
                    backlog.push_back(RecheckOutput::Chunk {
                        task: *task,
                        attempt_id: attempt_id.clone(),
                        check_id: check_id.clone(),
                        text: text.clone(),
                    });
                }
            }
            RecheckWatch {
                backlog,
                live: Some(batch.sender.subscribe()),
            }
        };
        // The batch may have finished, and dropped its live state, between the read above and
        // the subscription: read again so this watcher does not wait for an end already sent.
        if self
            .inner
            .store
            .recheck_finished(recheck)
            .await?
            .flatten()
            .is_some()
        {
            self.inner
                .live_rechecks
                .batches
                .lock()
                .unwrap()
                .remove(recheck);
            return Ok(ended);
        }
        Ok(watch)
    }

    /// Row (`task`, `attempt`, `check`) of batch `recheck` began: its live output starts empty.
    pub(crate) fn live_started(
        &self,
        recheck: &str,
        task: u32,
        attempt: &AttemptId,
        check: &str,
    ) -> LiveRow {
        let mut batches = self.inner.live_rechecks.batches.lock().unwrap();
        let batch = batches
            .entry(recheck.to_string())
            .or_insert_with(LiveBatch::new);
        batch.current = Some((task, attempt.clone(), check.to_string(), String::new()));
        let _ = batch.sender.send(RecheckOutput::Started {
            task,
            attempt_id: attempt.clone(),
            check_id: check.to_string(),
        });
        LiveRow {
            runs: self.clone(),
            batch: recheck.to_string(),
            pending: Vec::new(),
        }
    }

    fn live_text(&self, recheck: &str, text: String) {
        let mut batches = self.inner.live_rechecks.batches.lock().unwrap();
        let Some(batch) = batches.get_mut(recheck) else {
            return;
        };
        let Some((task, attempt_id, check_id, output)) = &mut batch.current else {
            return;
        };
        output.push_str(&text);
        if output.len() > OUTPUT_LIMIT {
            let mut cut = output.len() - OUTPUT_LIMIT;
            while !output.is_char_boundary(cut) {
                cut += 1;
            }
            output.drain(..cut);
        }
        let _ = batch.sender.send(RecheckOutput::Chunk {
            task: *task,
            attempt_id: attempt_id.clone(),
            check_id: check_id.clone(),
            text,
        });
    }

    /// The running row of batch `recheck` finished with `outcome`; its stored output now stands.
    pub(crate) fn live_finished(&self, recheck: &str, outcome: CheckOutcome) {
        let mut batches = self.inner.live_rechecks.batches.lock().unwrap();
        let Some(batch) = batches.get_mut(recheck) else {
            return;
        };
        if let Some((task, attempt_id, check_id, _)) = batch.current.take() {
            let _ = batch.sender.send(RecheckOutput::Finished {
                task,
                attempt_id,
                check_id,
                outcome,
            });
        }
    }

    /// Batch `recheck` finished: its watchers hear the end and its live state is dropped.
    pub(crate) fn live_ended(&self, recheck: &str) {
        if let Some(batch) = self
            .inner
            .live_rechecks
            .batches
            .lock()
            .unwrap()
            .remove(recheck)
        {
            let _ = batch.sender.send(RecheckOutput::End);
        }
    }
}
