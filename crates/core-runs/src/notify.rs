//! One job: telling a plan's Planner how an attempt ended.
//!
//! The notice is a waiting message on the plan's own conversation, sent
//! through `Turns::queue_notice` in the executor's name, with its card (an
//! `AttemptNotice`) and the settings of that conversation's last turn:
//! a busy Planner hears it when its turn ends, an idle one at once, and a
//! restart does not lose it. Its command id names the attempt, so a repeat is
//! a replay. A notice that cannot be sent is logged and never fails the
//! attempt.
use crate::Runs;
use crate::amend::ADDITIONS;
use crate::model::{Attempt, AttemptNotice, AttemptState};
use core_plans::WrittenBy;
use core_threads::ThreadEntryKind;
use core_turns::{Notice, SendTurn};

impl Runs {
    /// Queues, on the plan's Planner conversation, the notice that `attempt`
    /// committed or failed.
    pub(crate) async fn notify(&self, attempt: &Attempt) {
        let plan = match self.read_plan(&attempt.workflow_id).await {
            Ok(plan) => plan,
            Err(error) => {
                tracing::warn!(attempt = %attempt.id, %error, "runs.notify_no_planner");
                return;
            }
        };
        let WrittenBy::Planner {
            thread_id,
            thread_removed: false,
            ..
        } = &plan.written_by
        else {
            tracing::warn!(attempt = %attempt.id, "runs.notify_no_planner");
            return;
        };
        let Some(invocation) = self.last_invocation(thread_id).await else {
            tracing::warn!(attempt = %attempt.id, "runs.notify_no_turn");
            return;
        };
        let title = plan
            .tasks
            .iter()
            .find(|t| t.content.number == attempt.task)
            .map(|t| t.content.title.clone())
            .unwrap_or_else(|| format!("T{}", attempt.task));
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
        let card = match serde_json::to_value(&card) {
            Ok(card) => card,
            Err(error) => {
                tracing::warn!(attempt = %attempt.id, %error, "runs.notify_failed");
                return;
            }
        };
        let notice = Notice {
            executor: Some(attempt.thread_id.clone()),
            card,
        };
        let queued = self
            .inner
            .turns
            .queue_notice(thread_id.clone(), turn, notice);
        if let Err(error) = queued.await {
            tracing::warn!(attempt = %attempt.id, %error, "runs.notify_failed");
        }
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
