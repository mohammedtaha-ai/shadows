//! One job: telling a plan's Planner how an attempt ended, or that its
//! executor reported a contract conflict.
//!
//! The notice is a waiting message on the plan's own conversation, sent
//! through `Turns::queue_notice` in the executor's name, with its card (an
//! `AttemptNotice`) and the settings of that conversation's last turn:
//! a busy Planner hears it when its turn ends, an idle one at once, and a
//! restart does not lose it. Its command id names the attempt, so a repeat is
//! a replay. A notice that cannot be sent never fails the attempt; the caller
//! logs it.
//!
//! A conflict's notice is resolved once, saved on the attempt before its
//! first enqueue, and every retry sends that same payload: changed Planner
//! settings or a later attempt state cannot change the replay. Its delivery
//! is `pending` until an enqueue (or its replay) succeeds, `no_planner` when
//! the plan has no live Planner conversation.
use crate::amend::ADDITIONS;
use crate::model::{Attempt, AttemptNotice, AttemptState, ConflictDelivery, TaskConflict};
use crate::{Runs, RunsError};
use core_plans::{Plan, WrittenBy};
use core_threads::{ThreadEntryKind, ThreadId};
use core_turns::{Notice, SendTurn};

/// The plan's live Planner conversation and the settings of its last turn.
struct Recipient {
    plan: Plan,
    thread: ThreadId,
    invocation: core_turns::InvocationView,
}

/// A conflict notice as first resolved: whom it goes to, its text, the
/// settings it is sent with and its card. Saved on the attempt as JSON.
#[derive(serde::Serialize, serde::Deserialize)]
struct ConflictNoticeIntent {
    planner: ThreadId,
    body: String,
    model: String,
    mode: String,
    effort: Option<String>,
    card: serde_json::Value,
}

impl ConflictNoticeIntent {
    /// The turn and notice every delivery of `attempt`'s conflict sends.
    fn rebuild(self, attempt: &Attempt) -> (ThreadId, SendTurn, Notice) {
        let turn = SendTurn {
            command_id: format!("notify-{}", attempt.id),
            prompt: self.body,
            attachments: vec![],
            model: self.model,
            mode: self.mode,
            effort: self.effort,
            focus: None,
            plan: None,
            client_tab: None,
        };
        let notice = Notice {
            executor: Some(attempt.thread_id.clone()),
            card: self.card,
        };
        (self.planner, turn, notice)
    }
}

impl Runs {
    /// Queues, on the plan's Planner conversation, the notice that `attempt`
    /// committed or failed, or, for an attempt with a contract conflict, the
    /// conflict's one notice, sent again only while it is pending.
    pub(crate) async fn notify(&self, attempt: &Attempt) -> Result<(), RunsError> {
        match self.inner.store.notice_intent(&attempt.id).await? {
            None => self.notify_ended(attempt).await,
            Some((ConflictDelivery::Pending, saved)) => self.notify_conflict(attempt, saved).await,
            Some(_) => Ok(()),
        }
    }

    /// `notify` for an attempt whose conflict notice is still pending, and
    /// nothing for any other: an explicit Stop and recovery retry only that.
    pub(crate) async fn retry_conflict_notice(&self, attempt: &Attempt) -> Result<(), RunsError> {
        match self.inner.store.notice_intent(&attempt.id).await? {
            Some((ConflictDelivery::Pending, _)) => self.notify(attempt).await,
            _ => Ok(()),
        }
    }

    /// The plan's live Planner and its last turn's settings; `None` when the
    /// plan is gone or has no live Planner conversation.
    async fn recipient(&self, attempt: &Attempt) -> Result<Option<Recipient>, RunsError> {
        let plan = match self.read_plan(&attempt.workflow_id).await {
            Ok(plan) => plan,
            Err(RunsError::NotFound) => return Ok(None),
            Err(error) => return Err(error),
        };
        let WrittenBy::Planner {
            thread_id,
            thread_removed: false,
            ..
        } = &plan.written_by
        else {
            return Ok(None);
        };
        let thread = thread_id.clone();
        let invocation = self.last_invocation(&thread).await.ok_or_else(|| {
            RunsError::Unavailable("the Planner has no turn whose settings a notice takes".into())
        })?;
        Ok(Some(Recipient {
            plan,
            thread,
            invocation,
        }))
    }

    /// The conflict notice of `attempt`, resolved and saved before its first
    /// enqueue, then sent as saved.
    async fn notify_conflict(
        &self,
        attempt: &Attempt,
        saved: Option<String>,
    ) -> Result<(), RunsError> {
        let saved = match saved {
            Some(saved) => saved,
            None => {
                let Some(recipient) = self.recipient(attempt).await? else {
                    tracing::warn!(attempt = %attempt.id, "runs.notify_no_planner");
                    return (self.inner.store)
                        .settle_delivery(&attempt.id, ConflictDelivery::NoPlanner)
                        .await;
                };
                let intent = self.conflict_intent(attempt, recipient).await?;
                (self.inner.store)
                    .keep_notice_intent(&attempt.id, serde_json::to_string(&intent)?)
                    .await?
            }
        };
        let intent: ConflictNoticeIntent = serde_json::from_str(&saved)?;
        let (planner, turn, notice) = intent.rebuild(attempt);
        match self.inner.turns.queue_notice(planner, turn, notice).await {
            Ok(_) => {
                (self.inner.store)
                    .settle_delivery(&attempt.id, ConflictDelivery::Delivered)
                    .await
            }
            Err(error) => {
                if matches!(self.recipient(attempt).await, Ok(None)) {
                    tracing::warn!(attempt = %attempt.id, "runs.notify_no_planner");
                    return (self.inner.store)
                        .settle_delivery(&attempt.id, ConflictDelivery::NoPlanner)
                        .await;
                }
                Err(RunsError::Unavailable(format!(
                    "the Planner's notice was not queued: {error}"
                )))
            }
        }
    }

    /// Resolves the conflict notice from the conflict as it stands now.
    async fn conflict_intent(
        &self,
        attempt: &Attempt,
        recipient: Recipient,
    ) -> Result<ConflictNoticeIntent, RunsError> {
        let conflict = (self.conflict(&attempt.id).await?).ok_or(RunsError::NotFound)?;
        let title = Self::title(&recipient.plan, attempt);
        let card = AttemptNotice {
            workflow_id: attempt.workflow_id.clone(),
            version: attempt.version,
            task: attempt.task,
            title: title.clone(),
            attempt: attempt.number,
            attempt_id: attempt.id.clone(),
            model: attempt.model.clone(),
            outcome: conflict.attempt_state,
            commit: None,
            report: None,
            reason: attempt.reason.clone(),
            last_addition: false,
            conflict: Some(conflict.clone()),
        };
        Ok(ConflictNoticeIntent {
            planner: recipient.thread,
            body: Self::conflict_body(attempt, &title, &conflict),
            model: recipient.invocation.requested_model,
            mode: recipient.invocation.requested_mode,
            effort: recipient.invocation.requested_effort,
            card: serde_json::to_value(&card)?,
        })
    }

    fn conflict_body(attempt: &Attempt, title: &str, conflict: &TaskConflict) -> String {
        let evidence = conflict
            .evidence
            .iter()
            .map(|e| match e.line {
                Some(line) => format!("- {}:{line}: {}", e.path, e.summary),
                None => format!("- {}: {}", e.path, e.summary),
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "[Shadows] T{task} \"{title}\" of plan version {workflow}: attempt {number} \
             ({id}) reported a contract conflict. Its executor was told to stop work and end \
             its turn; that it ended is not confirmed yet.\nExpected: {expected}\n\
             Observed: {observed}\nEvidence:\n{evidence}\n\
             Assess which other tasks this affects. If this attempt or another affected \
             executor has not ended, stop it with task_stop; leave independent work running. \
             Tell the person, and correct the plan in a new version for them to approve; never \
             start this task again on your own.",
            task = attempt.task,
            workflow = attempt.workflow_id,
            number = attempt.number,
            id = attempt.id,
            expected = conflict.expected,
            observed = conflict.observed,
        )
    }

    /// The task's title in `plan`, or `T{task}` when it has none.
    fn title(plan: &Plan, attempt: &Attempt) -> String {
        plan.tasks
            .iter()
            .find(|t| t.content.number == attempt.task)
            .map(|t| t.content.title.clone())
            .unwrap_or_else(|| format!("T{}", attempt.task))
    }

    /// The notice that `attempt`, without a conflict, committed or failed.
    async fn notify_ended(&self, attempt: &Attempt) -> Result<(), RunsError> {
        let Some(Recipient {
            plan,
            thread: thread_id,
            invocation,
        }) = self.recipient(attempt).await?
        else {
            tracing::warn!(attempt = %attempt.id, "runs.notify_no_planner");
            return Ok(());
        };
        let title = Self::title(&plan, attempt);
        let mut card = AttemptNotice {
            workflow_id: attempt.workflow_id.clone(),
            version: attempt.version,
            task: attempt.task,
            title: title.clone(),
            attempt: attempt.number,
            attempt_id: attempt.id.clone(),
            model: attempt.model.clone(),
            outcome: attempt.state,
            commit: attempt.commit.clone(),
            report: None,
            reason: attempt.reason.clone(),
            last_addition: false,
            conflict: None,
        };
        let head = format!(
            "[Shadows] T{} \"{title}\" of plan version {}: attempt {}",
            attempt.task, attempt.workflow_id, attempt.number
        );
        let prompt = match attempt.state {
            AttemptState::Committed => {
                let files = match self.diff(&attempt.id).await {
                    Ok(files) if !files.is_empty() => files
                        .iter()
                        .map(|f| format!("{} (+{} -{})", f.path, f.added, f.removed))
                        .collect::<Vec<_>>()
                        .join(", "),
                    Ok(_) => "none".to_string(),
                    Err(error) => {
                        tracing::warn!(attempt = %attempt.id, %error, "runs.notify_diff_unread");
                        "none".to_string()
                    }
                };
                card.report = attempt.report.clone().filter(|r| !r.trim().is_empty());
                let report = card.report.clone().unwrap_or_else(|| "(none)".to_string());
                let last = self
                    .inner
                    .store
                    .additions_since_start(&attempt.workflow_id, attempt.version, attempt.task)
                    .await
                    .is_ok_and(|spent| spent >= ADDITIONS);
                card.last_addition = last;
                let last = if last {
                    format!(
                        "This attempt carried T{}'s third and last addition: mark it done, or \
                         the task stops and goes back to the person.\n",
                        attempt.task
                    )
                } else {
                    String::new()
                };
                format!(
                    "{head} committed {}.\nFiles: {files}\nThe executor's report:\n{report}\n\
                     {last}Read its change with task_diff and task_file, then judge it against \
                     T{}'s acceptance items before you mark it done.",
                    attempt.commit.as_deref().unwrap_or_default(),
                    attempt.task
                )
            }
            _ => {
                card.report = self.failed_message(attempt).await;
                let last = card
                    .report
                    .as_ref()
                    .map(|text| format!("The executor's last message:\n{text}\n"))
                    .unwrap_or_default();
                format!(
                    "{head} failed: {}\n{last}Tell the person; start it again only if they ask.",
                    attempt.reason.as_deref().unwrap_or_default()
                )
            }
        };
        let turn = SendTurn {
            command_id: format!("notify-{}", attempt.id),
            prompt,
            attachments: vec![],
            model: invocation.requested_model,
            mode: invocation.requested_mode,
            effort: invocation.requested_effort,
            focus: None,
            plan: None,
            client_tab: None,
        };
        let notice = Notice {
            executor: Some(attempt.thread_id.clone()),
            card: serde_json::to_value(&card)?,
        };
        let queued = self.inner.turns.queue_notice(thread_id, turn, notice);
        queued
            .await
            .map(|_| ())
            .map_err(|error| RunsError::Unavailable(format!("the notice was not queued: {error}")))
    }

    /// Keeps the executor's answer, cut at 4,000 chars, when no commit
    /// carries a report to the Planner.
    async fn failed_message(&self, attempt: &Attempt) -> Option<String> {
        let Some(operation) = &attempt.operation_id else {
            tracing::warn!(attempt = %attempt.id, "runs.notify_no_message");
            return None;
        };
        let entries = match self.inner.threads.entries(&attempt.thread_id).await {
            Ok(entries) => entries,
            Err(error) => {
                tracing::warn!(attempt = %attempt.id, %error, "runs.notify_no_message");
                return None;
            }
        };
        let Some(entry) = entries.into_iter().rev().find(|entry| {
            entry.kind == ThreadEntryKind::AgentMessage
                && entry.operation_id.as_deref() == Some(operation.as_str())
        }) else {
            tracing::warn!(attempt = %attempt.id, "runs.notify_no_message");
            return None;
        };
        let mut chars = entry.body.chars();
        let mut text: String = chars.by_ref().take(4_000).collect();
        if chars.next().is_some() {
            text.push_str("[cut]");
        }
        Some(text)
    }
}
