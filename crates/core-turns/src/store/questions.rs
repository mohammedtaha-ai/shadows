//! One job: the `person_question` queries — a question asked, and its one answer.

use core_db::{Actor, CommandContext, Storage, StorageError};
use core_operations::Operations;
use core_threads::{NewThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, ThreadKind, Threads};

use super::queue::{QueuedFields, SCOPE};
use crate::TurnsError;
use crate::model::{QuestionId, QuestionOption, QueuedMessage, QueuedMessageId};

/// The settings the answer's turn runs under: model, mode and effort.
pub(crate) type AnswerSettings = (String, String, Option<String>);

/// A question's card: what the conversation draws, `chosen` null until answered.
/// `thread_id` is the thread asked: a fork copies the card unchanged, so a
/// card whose thread is not its conversation's is history, not a question.
fn card(
    id: &QuestionId,
    thread: &ThreadId,
    question: &str,
    options: &[QuestionOption],
    chosen: Option<usize>,
) -> serde_json::Value {
    serde_json::json!({
        "question_id": id,
        "thread_id": thread,
        "question": question,
        "options": options,
        "chosen": chosen,
    })
}

impl super::TurnsStore {
    /// Records the question and its entry in one transaction; an executor's
    /// thread asks no one.
    pub async fn ask_question(
        &self,
        thread: &ThreadId,
        question: &str,
        options: &[QuestionOption],
    ) -> Result<QuestionId, TurnsError> {
        let (thread, question, options) = (thread.clone(), question.to_owned(), options.to_vec());
        let ts = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let context = Threads::turn_context_in(conn, &thread).await?;
                    if context.kind == ThreadKind::Executor {
                        return Err(TurnsError::InvalidCommand(
                            "only the Planner asks the person".into(),
                        ));
                    }
                    let operation = Operations::pending_or_running_in(conn, &thread).await?;
                    let id = QuestionId::generate();
                    let card = card(&id, &thread, &question, &options, None);
                    let entry = Threads::append_entry_in(
                        conn,
                        &thread,
                        NewThreadEntry {
                            kind: ThreadEntryKind::PersonQuestion,
                            author: Actor {
                                kind: "Agent".into(),
                                id: "Planner".into(),
                            },
                            body: &question,
                            refs: &[],
                            card: Some(&card),
                            operation_id: operation.as_ref().map(|op| op.as_str()),
                        },
                        &ts,
                    )
                    .await?;
                    sqlx::query(
                        "INSERT INTO person_question
                           (id, thread_id, entry_id, question, options, chosen, asked_at,
                            answered_at)
                         VALUES (?,?,?,?,?,NULL,?,NULL)",
                    )
                    .bind(id.as_str())
                    .bind(thread.as_str())
                    .bind(entry.id.as_str())
                    .bind(&question)
                    .bind(serde_json::to_string(&options)?)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    Ok(id)
                })
            })
            .await
    }

    /// In one transaction: the choice, the card that shows it, and the
    /// person's message waiting to be sent. Answers the message and whether
    /// the thread was idle; a replay answers the first message, idle, so the
    /// caller asks for its turn again.
    pub async fn answer_question(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        question: &QuestionId,
        option: usize,
        settings: AnswerSettings,
    ) -> Result<(QueuedMessage, bool), TurnsError> {
        let (ctx, thread, question) = (ctx.clone(), thread.clone(), question.clone());
        let ts = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(outcome) =
                        CommandContext::classify(conn, &ctx, SCOPE, thread.as_str()).await?
                    {
                        return Ok((serde_json::from_str(&outcome)?, true));
                    }
                    Threads::require_live_in(conn, &thread).await?;
                    let row: Option<(String, String, String, String, Option<i64>)> =
                        sqlx::query_as(
                            "SELECT thread_id, entry_id, question, options, chosen
                               FROM person_question WHERE id = ?",
                        )
                        .bind(question.as_str())
                        .fetch_optional(&mut *conn)
                        .await?;
                    let (asked_in, entry, text, options, chosen) =
                        row.ok_or(StorageError::NotFound("person_question"))?;
                    if asked_in != thread.as_str() {
                        return Err(TurnsError::InvalidCommand(
                            "the question was asked in another conversation".into(),
                        ));
                    }
                    if chosen.is_some() {
                        return Err(StorageError::CommandConflict.into());
                    }
                    let options: Vec<QuestionOption> = serde_json::from_str(&options)?;
                    let Some(label) = options.get(option).map(|o| o.label.clone()) else {
                        return Err(TurnsError::InvalidCommand(
                            "the question has no such option".into(),
                        ));
                    };
                    sqlx::query(
                        "UPDATE person_question SET chosen = ?, answered_at = ? WHERE id = ?",
                    )
                    .bind(option as i64)
                    .bind(&ts)
                    .bind(question.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let entry = ThreadEntryId::from_recorded(entry);
                    let card = card(&question, &thread, &text, &options, Some(option));
                    Threads::set_card_in(conn, &thread, &entry, &card).await?;
                    let idle = !Operations::has_open_in(conn, &thread).await?;
                    let (model, mode, effort) = settings;
                    let fields: QueuedFields = (
                        format!("Answer to \"{text}\": {label}"),
                        model,
                        mode,
                        effort,
                        None,
                        None,
                    );
                    let id = QueuedMessageId::generate();
                    let queued =
                        Self::insert_queued_in(conn, &ctx, &thread, &id, &fields, &[], None, &ts)
                            .await?;
                    Ok((queued, idle))
                })
            })
            .await
    }
}
