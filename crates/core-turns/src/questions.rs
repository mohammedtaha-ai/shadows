//! One job: the Planner's questions to the person — `person_ask`'s card, and
//! the click that answers it as the person's message.

use super::{Turns, TurnsError};
use crate::model::{QuestionId, QuestionOption, Queued};
use core_db::CommandContext;
use core_threads::ThreadId;

impl Turns {
    /// Asks the person in `thread`: 2 to 5 options, each label 1 to 80
    /// characters, the question 1 to 500. The question is an entry whose card
    /// the person answers with one click.
    pub async fn ask(
        &self,
        thread: &ThreadId,
        question: String,
        options: Vec<QuestionOption>,
    ) -> Result<QuestionId, TurnsError> {
        let question = question.trim().to_owned();
        if !(1..=500).contains(&question.chars().count()) {
            return Err(TurnsError::InvalidCommand(
                "a question is 1 to 500 characters".into(),
            ));
        }
        if !(2..=5).contains(&options.len()) {
            return Err(TurnsError::InvalidCommand(
                "a question offers 2 to 5 options".into(),
            ));
        }
        let options: Vec<QuestionOption> = options
            .into_iter()
            .map(|o| QuestionOption {
                label: o.label.trim().to_owned(),
            })
            .collect();
        if options
            .iter()
            .any(|o| !(1..=80).contains(&o.label.chars().count()))
        {
            return Err(TurnsError::InvalidCommand(
                "an option's label is 1 to 80 characters".into(),
            ));
        }
        self.store.ask_question(thread, &question, &options).await
    }

    /// The person's click on `option`, which is their message: the choice,
    /// the card that shows it and the message `Answer to "<question>": <label>`
    /// are recorded together, then the message is sent as a typed one is —
    /// at once on an idle thread, waiting while a turn runs. A second answer
    /// is `CommandConflict`; a replay answers as the first did.
    pub async fn answer(
        &self,
        command: String,
        thread: &ThreadId,
        question: &QuestionId,
        option: usize,
    ) -> Result<Queued, TurnsError> {
        let params = serde_json::json!({
            "thread_id": thread,
            "question_id": question,
            "option": option,
        });
        let command = CommandContext::user(command, "question.answer", params);
        let (message, idle) = match self.store.replayed_queue(&command, thread).await? {
            Some(message) => (message, true),
            None => {
                let settings = self.answer_settings(thread).await?;
                self.store
                    .answer_question(&command, thread, question, option, settings)
                    .await?
            }
        };
        if !idle {
            return Ok(Queued::Waiting { message });
        }
        match self.send_queued(thread, &message, None).await {
            Ok(operation_id) => Ok(Queued::Started { operation_id }),
            Err(TurnsError::ThreadBusy | TurnsError::QueuedMessageGone) => {
                Ok(Queued::Waiting { message })
            }
            Err(error) => {
                let reason = error.to_string();
                self.store.fail_queued(thread, &message.id, &reason).await?;
                Err(error)
            }
        }
    }

    /// The settings of the thread's latest turn, which the answer continues.
    async fn answer_settings(
        &self,
        thread: &ThreadId,
    ) -> Result<crate::store::AnswerSettings, TurnsError> {
        let operations = self.store.list_operations_for_thread(thread).await?;
        let latest = operations
            .into_iter()
            .filter_map(|op| op.invocation.map(|i| (op.created_at, op.seq, i)))
            .max_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
        let Some((_, _, invocation)) = latest else {
            return Err(TurnsError::InvalidCommand(
                "the conversation has no turn to answer".into(),
            ));
        };
        Ok((
            invocation.requested_model,
            invocation.requested_mode,
            invocation.requested_effort,
        ))
    }
}
