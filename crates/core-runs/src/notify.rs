//! One job: telling a plan's Planner how an attempt ended.
//!
//! The notice is a waiting message on the plan's own conversation, sent
//! through `Turns::queue` with the settings of that conversation's last turn:
//! a busy Planner hears it when its turn ends, an idle one at once, and a
//! restart does not lose it. Its command id names the attempt, so a repeat is
//! a replay. A notice that cannot be sent is logged and never fails the
//! attempt.
use crate::Runs;
use crate::amend::ADDITIONS;
use crate::model::{Attempt, AttemptState};
use core_plans::WrittenBy;
use core_turns::SendTurn;

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
            .unwrap_or_default();
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
                let report = attempt
                    .report
                    .clone()
                    .filter(|r| !r.trim().is_empty())
                    .unwrap_or_else(|| "(none)".to_string());
                let last = self
                    .inner
                    .store
                    .additions_since_start(&attempt.workflow_id, attempt.version, attempt.task)
                    .await
                    .is_ok_and(|spent| spent >= ADDITIONS);
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
            _ => format!(
                "{head} failed: {}\nTell the person; start it again only if they ask.",
                attempt.reason.as_deref().unwrap_or_default()
            ),
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
        if let Err(error) = self.inner.turns.queue(thread_id.clone(), turn).await {
            tracing::warn!(attempt = %attempt.id, %error, "runs.notify_failed");
        }
    }
}
