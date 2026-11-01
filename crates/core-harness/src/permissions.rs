//! Person answers to live AYLUL permission requests.
use crate::{Harness, HarnessError, PermissionAnswer, WaitingPermission};
use core_db::{CommandContext, Storage};
use core_threads::ThreadId;
use shadows_agent::acp::PermissionError;

impl Harness {
    pub async fn waiting_permissions(&self) -> Vec<WaitingPermission> {
        if self.sessions.config.aylul.is_none() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (thread_id, slot) in self.sessions.slots().await {
            let live = slot.lock().await;
            let Some(item) = live.as_ref() else { continue };
            let Some(usage) = &item.opened.aylul_usage else {
                continue;
            };
            if item.opened.connection.is_closed() {
                continue;
            }
            for waiting in item.opened.connection.held_permissions() {
                let nanos = i128::try_from(waiting.asked_at)
                    .unwrap_or(i128::MAX)
                    .saturating_mul(1_000_000);
                let date = time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
                    .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
                out.push(WaitingPermission {
                    thread_id: thread_id.clone(),
                    agent: usage.agent_slug.clone(),
                    request_id: waiting.request,
                    title: waiting.title,
                    options: waiting.options,
                    asked_at: date
                        .format(time::macros::format_description!(
                            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
                        ))
                        .expect("UTC timestamp formats"),
                });
            }
        }
        out.sort_by(|a, b| (&a.asked_at, &a.request_id).cmp(&(&b.asked_at, &b.request_id)));
        out
    }

    pub async fn answer_permission(
        &self,
        command_id: String,
        thread: &ThreadId,
        request: &str,
        option: &str,
    ) -> Result<PermissionAnswer, HarnessError> {
        let mut command = CommandContext::user(
            command_id,
            "thread.permission_answer",
            serde_json::json!({"thread_id":thread,"request_id":request,"option_id":option}),
        );
        command.agent = Some("human".into());
        if let Some(answer) = self.store.permission_replay(&command, thread).await? {
            return Ok(answer);
        }
        let slot = self
            .sessions
            .slots()
            .await
            .get(thread)
            .cloned()
            .ok_or(HarnessError::PermissionNotFound)?;
        let live = slot.lock().await;
        if let Some(answer) = self.store.permission_replay(&command, thread).await? {
            return Ok(answer);
        }
        let item = live
            .as_ref()
            .filter(|item| item.opened.aylul_usage.is_some())
            .ok_or(HarnessError::PermissionNotFound)?;
        let prepared = item
            .opened
            .connection
            .prepare_permission_answer(request, option)
            .await
            .map_err(Self::permission_error)?;
        let answer = PermissionAnswer {
            thread_id: thread.clone(),
            request_id: request.into(),
            option_id: option.into(),
            kind: prepared.option().kind.clone(),
            answered_at: Storage::now(),
        };
        let (answer, written) = self.store.record_permission_answer(command, answer).await?;
        if written {
            prepared.commit().map_err(Self::permission_error)?;
        }
        Ok(answer)
    }

    fn permission_error(error: PermissionError) -> HarnessError {
        match error {
            PermissionError::Unknown => HarnessError::PermissionNotFound,
            PermissionError::Settled => HarnessError::PermissionSettled,
            PermissionError::OptionNotOffered => HarnessError::PermissionOptionNotOffered,
        }
    }
}
