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
//! Every notice is resolved once, saved before its first enqueue, and every
//! retry sends that same payload: changed Planner settings or a later attempt
//! state cannot change the replay's fingerprint. A conflict's delivery is
//! `pending` until an enqueue (or its replay) succeeds, `no_planner` when the
//! plan has no live Planner conversation.
//!
//! A version an external agent wrote gets no thread notice: its report is kept for that
//! author (`author.rs`), with the body built here.
use crate::amend::ADDITIONS;
use crate::checks::CONTRACT_CHECK;
use crate::model::{Attempt, AttemptNotice, AttemptState, ConflictDelivery, TaskConflict};
use crate::{CheckResult, Runs, RunsError};
use core_plans::{Plan, WrittenBy};
use core_threads::{ThreadEntryKind, ThreadId};
use core_turns::{Notice, SendTurn};

/// The plan's live Planner conversation and the settings of its last turn.
pub(crate) struct Recipient {
    pub(crate) plan: Plan,
    pub(crate) thread: ThreadId,
    pub(crate) invocation: core_turns::InvocationView,
}

/// A notice as first resolved, so a replay keeps its destination, settings and card.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct NoticeIntent {
    pub(crate) planner: ThreadId,
    pub(crate) body: String,
    pub(crate) model: String,
    pub(crate) mode: String,
    pub(crate) effort: Option<String>,
    pub(crate) card: serde_json::Value,
}

impl NoticeIntent {
    /// Rebuild the same turn and card for each delivery of this command.
    fn rebuild(self, attempt: &Attempt, command_id: String) -> (ThreadId, SendTurn, Notice) {
        self.into_notice(command_id, Some(attempt.thread_id.clone()))
    }

    pub(crate) fn into_notice(
        self,
        command_id: String,
        executor: Option<ThreadId>,
    ) -> (ThreadId, SendTurn, Notice) {
        let turn = SendTurn {
            command_id,
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
            executor,
            card: self.card,
        };
        (self.planner, turn, notice)
    }
}

impl Runs {
    /// Queues, on the plan's Planner conversation, the notice that `attempt`
    /// committed or failed, or, for an attempt with a contract conflict, the
    /// conflict's one notice, sent again only while it is pending.
    /// An attempt of a version an external agent wrote keeps its report for that author
    /// instead.
    pub(crate) async fn notify(&self, attempt: &Attempt) -> Result<(), RunsError> {
        let intent = self.inner.store.notice_intent(&attempt.id).await?;
        if let Some((plan, grant)) = self.external_author(attempt).await? {
            return (self.keep_for_author(attempt, &plan, &grant, intent.is_some())).await;
        }
        match intent {
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
        self.recipient_for(&attempt.workflow_id).await
    }

    pub(crate) async fn recipient_for(
        &self,
        workflow: &core_plans::WorkflowId,
    ) -> Result<Option<Recipient>, RunsError> {
        let plan = match self.read_plan(workflow).await {
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
        let intent: NoticeIntent = serde_json::from_str(&saved)?;
        let (planner, turn, notice) = intent.rebuild(attempt, format!("notify-{}", attempt.id));
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
    ) -> Result<NoticeIntent, RunsError> {
        let conflict = (self.conflict(&attempt.id).await?).ok_or(RunsError::NotFound)?;
        let title = Self::title(&recipient.plan, attempt);
        let card = AttemptNotice {
            left_files: None,
            checks: vec![],
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
        Ok(NoticeIntent {
            planner: recipient.thread,
            body: Self::conflict_body(attempt, &title, &conflict),
            model: recipient.invocation.requested_model,
            mode: recipient.invocation.requested_mode,
            effort: recipient.invocation.requested_effort,
            card: serde_json::to_value(&card)?,
        })
    }

    pub(crate) fn conflict_body(attempt: &Attempt, title: &str, conflict: &TaskConflict) -> String {
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
    pub(crate) fn title(plan: &Plan, attempt: &Attempt) -> String {
        plan.tasks
            .iter()
            .find(|t| t.content.number == attempt.task)
            .map(|t| t.content.title.clone())
            .unwrap_or_else(|| format!("T{}", attempt.task))
    }

    /// `check` as a conversation card carries it, the attempt's and the recheck's alike: a
    /// command's output is left to `task_checks`, while Shadows' own check keeps its short
    /// verdict, so the person reads what is missing as the Planner does.
    pub(crate) fn card_check(mut check: CheckResult) -> CheckResult {
        if !check.by_shadows {
            check.output = None;
        }
        check
    }

    /// The notice that `attempt`, without a conflict, committed or failed:
    /// resolved once and saved, so a retry after a restart replays it.
    async fn notify_ended(&self, attempt: &Attempt) -> Result<(), RunsError> {
        let saved = match self.inner.store.ended_notice(&attempt.id).await? {
            Some(saved) => saved,
            None => {
                let Some(intent) = self.ended_intent(attempt).await? else {
                    return Ok(());
                };
                self.inner
                    .store
                    .keep_ended_notice(&attempt.id, serde_json::to_string(&intent)?)
                    .await?
            }
        };
        let intent: NoticeIntent = serde_json::from_str(&saved)?;
        let (thread, turn, notice) = intent.rebuild(attempt, format!("notify-{}", attempt.id));
        self.inner
            .turns
            .queue_notice(thread, turn, notice)
            .await
            .map_err(|error| {
                RunsError::Unavailable(format!("the notice was not queued: {error}"))
            })?;
        self.inner.store.mark_notice_sent(&attempt.id).await
    }

    /// Resolves the ended notice from the attempt as it stands now; `None`
    /// when the plan has no live Planner conversation.
    async fn ended_intent(&self, attempt: &Attempt) -> Result<Option<NoticeIntent>, RunsError> {
        let Some(Recipient {
            plan,
            thread: thread_id,
            invocation,
        }) = self.recipient(attempt).await?
        else {
            tracing::warn!(attempt = %attempt.id, "runs.notify_no_planner");
            return Ok(None);
        };
        let (body, card) = self.ended_notice(attempt, &plan).await?;
        Ok(Some(NoticeIntent {
            planner: thread_id,
            body,
            model: invocation.requested_model,
            mode: invocation.requested_mode,
            effort: invocation.requested_effort,
            card: serde_json::to_value(&card)?,
        }))
    }

    /// The ended notice's body and card from the attempt as it stands now, for the Planner
    /// and an external author alike.
    pub(crate) async fn ended_notice(
        &self,
        attempt: &Attempt,
        plan: &Plan,
    ) -> Result<(String, AttemptNotice), RunsError> {
        let title = Self::title(plan, attempt);
        let mut card = AttemptNotice {
            left_files: None,
            checks: vec![],
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
                let results = self.check_results(&attempt.id).await?;
                card.checks = results.iter().cloned().map(Self::card_check).collect();
                let mut checks = results
                    .iter()
                    .map(|c| {
                        let line = Self::check_line(c, attempt.task, "the executor");
                        // The contract check's output is Shadows' own verdict, so the
                        // notice says it: what is missing, or why nothing could be checked.
                        match c.output.as_deref().filter(|output| {
                            c.id == CONTRACT_CHECK
                                && c.outcome != Some(crate::CheckOutcome::Passed)
                                && !output.is_empty()
                        }) {
                            Some(output) => {
                                format!("{line}\n{}\n", Self::output_excerpt(&c.id, output))
                            }
                            None => format!("{line}\n"),
                        }
                    })
                    .collect::<String>();
                if results
                    .iter()
                    .any(|c| c.outcome != Some(crate::CheckOutcome::Passed))
                {
                    let returns = self.inner.store.returns_since(&attempt.id).await?;
                    checks.push_str(if returns >= 2 {
                        "Shadows returned it to the executor twice; read task_checks for the \
                         output.\n"
                    } else {
                        "Read task_checks for the output.\n"
                    });
                }
                let checks = if checks.is_empty() {
                    checks
                } else {
                    format!("Checks:\n{checks}")
                };
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
                     {last}{checks}Read its change with task_diff and task_file, then judge \
                     it against T{}'s acceptance items before you mark it done.",
                    attempt.commit.as_deref().unwrap_or_default(),
                    attempt.task
                )
            }
            AttemptState::Interrupted | AttemptState::Stopped => {
                let files = async {
                    let folder = self.folder(&attempt.project_id).await?;
                    let git = crate::git::Git::at(&folder);
                    let prefix = git.prefix().await.ok_or(RunsError::NotGit)?;
                    let task = plan
                        .tasks
                        .iter()
                        .find(|t| t.content.number == attempt.task)
                        .ok_or(RunsError::NotFound)?;
                    let scope = crate::scope::Scope::new(&task.content.writes)
                        .map_err(RunsError::BadScope)?;
                    self.dirty(&git, &prefix, &scope).await
                }
                .await;
                card.left_files = match files {
                    Ok(files) => Some(files),
                    Err(error) => {
                        tracing::warn!(attempt=%attempt.id, %error, "runs.notify_dirty_unread");
                        None
                    }
                };
                card.reason = None;
                let dirty = match &card.left_files {
                    Some(files) if files.is_empty() => "none".into(),
                    Some(files) => files.join(", "),
                    None => "unknown".into(),
                };
                let last = match &attempt.operation_id {
                    Some(operation) => self.last_message(&attempt.thread_id, operation).await,
                    None => None,
                }
                .map(|text| format!("The executor's last message:\n{text}\n"))
                .unwrap_or_default();
                if attempt.state == AttemptState::Interrupted {
                    format!(
                        "{head} was interrupted: Shadows stopped while it ran, and nothing \
                        was committed.\nUncommitted files in its writes: {dirty}\n{last}Tell the \
                        person; start it again only if they ask. A new Start continues from \
                        these files."
                    )
                } else {
                    format!(
                        "{head} was stopped by the person.\nUncommitted files in its writes: \
                        {dirty}\n{last}Do not start it again unless they ask. A new Start \
                        continues \
                        from these files."
                    )
                }
            }
            _ => {
                card.report = match &attempt.operation_id {
                    Some(operation) => self.last_message(&attempt.thread_id, operation).await,
                    None => None,
                };
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
        Ok((prompt, card))
    }

    /// Keeps the executor's answer, cut at 4,000 chars, when no commit
    /// carries a report to the Planner.
    pub(super) async fn last_message(
        &self,
        thread: &ThreadId,
        operation: &core_operations::OperationId,
    ) -> Option<String> {
        let entries = self.inner.threads.entries(thread).await.ok()?;
        let entry = entries.into_iter().rev().find(|entry| {
            entry.kind == ThreadEntryKind::AgentMessage
                && entry.operation_id.as_deref() == Some(operation.as_str())
        })?;
        let mut chars = entry.body.chars();
        let mut text: String = chars.by_ref().take(4000).collect();
        if chars.next().is_some() {
            text.push_str("[cut]");
        }
        Some(text)
    }

    /// The answered question's notice text, for the Planner and an external author alike.
    pub(crate) fn reply_body(
        attempt: &Attempt,
        title: &str,
        message: &crate::store::messages::MessageRecord,
    ) -> String {
        match &message.reason {
            Some(reason) => format!(
                "[Shadows] No reply from task agent · T{}: {reason}",
                attempt.task
            ),
            None => format!(
                "[Shadows] Reply from task agent · T{} \"{title}\":\n{}",
                attempt.task,
                message.reply.as_deref().unwrap_or_default()
            ),
        }
    }

    /// Delivers an answered question: to the Planner as a waiting notice or, for a version an
    /// external agent wrote, as a `reply` report kept for that author.
    pub(super) async fn notify_reply(
        &self,
        message: &crate::store::messages::MessageRecord,
    ) -> Result<(), RunsError> {
        let attempt = self
            .inner
            .store
            .get(&crate::AttemptId::from_recorded(message.attempt_id.clone()))
            .await?;
        let saved = match &message.notice_json {
            Some(saved) => saved.clone(),
            None => {
                if let Some((plan, grant)) = self.external_author(&attempt).await? {
                    return (self.keep_reply_for_author(&attempt, &plan, &grant, message)).await;
                }
                let Some(Recipient {
                    plan,
                    thread,
                    invocation,
                }) = self.recipient(&attempt).await?
                else {
                    return Err(RunsError::Unavailable(
                        "the plan has no Planner conversation".into(),
                    ));
                };
                let title = Self::title(&plan, &attempt);
                let card = crate::AgentReplyNotice {
                    workflow_id: attempt.workflow_id.clone(),
                    version: attempt.version,
                    task: attempt.task,
                    title: title.clone(),
                    attempt_id: attempt.id.clone(),
                    message: message.message.clone(),
                    reply: message.reply.clone(),
                    reason: message.reason.clone(),
                };
                let prompt = Self::reply_body(&attempt, &title, message);

                let intent = NoticeIntent {
                    planner: thread,
                    body: prompt,
                    model: invocation.requested_model,
                    mode: invocation.requested_mode,
                    effort: invocation.requested_effort,
                    card: serde_json::to_value(card)?,
                };
                self.inner
                    .store
                    .keep_message_notice(&message.id, serde_json::to_string(&intent)?)
                    .await?
            }
        };
        let intent: NoticeIntent = serde_json::from_str(&saved)?;
        let (thread, turn, notice) = intent.rebuild(&attempt, format!("reply-{}", message.id));
        self.inner
            .turns
            .queue_notice(thread, turn, notice)
            .await
            .map_err(|e| RunsError::Unavailable(e.to_string()))?;
        self.inner.store.message_delivered(&message.id).await
    }
}
