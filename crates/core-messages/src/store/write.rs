use super::Store;
use crate::{Address, GroupId, Message, MessageId, MessageKind, MessagesError, Speaker};
use core_db::{Actor, JournalEntry, Storage, Tx};

pub(in super::super) struct Draft {
    pub speaker: Speaker,
    pub to: Vec<Address>,
    pub kind: MessageKind,
    pub body: String,
}

impl Store {
    pub(in super::super) fn notice(body: impl Into<String>) -> Draft {
        Draft {
            speaker: Speaker::Shadows,
            to: Vec::new(),
            kind: MessageKind::Notice,
            body: body.into(),
        }
    }

    pub(in super::super) async fn append(
        conn: &mut Tx,
        group: &GroupId,
        draft: Draft,
        cause: Option<i64>,
        operation: Option<&str>,
    ) -> Result<Message, MessagesError> {
        let id = MessageId::generate();
        let created_at = Storage::now();
        let project: String =
            sqlx::query_scalar("SELECT project_id FROM message_group WHERE id = ?")
                .bind(group.as_str())
                .fetch_optional(&mut *conn)
                .await?
                .ok_or(MessagesError::NotFound)?;
        let seq: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) + 1 FROM message WHERE group_id = ?")
                .bind(group.as_str())
                .fetch_one(&mut *conn)
                .await?;
        let cause = if draft.speaker == Speaker::Person {
            Some(seq)
        } else {
            cause
        };
        let (kind, actor) = match &draft.speaker {
            Speaker::Person => ("chat", Actor::user("local")),
            Speaker::Agent { slug, .. } => (
                "chat",
                Actor {
                    kind: "Agent".into(),
                    id: slug.clone(),
                },
            ),
            Speaker::Shadows => ("notice", Actor::system()),
        };
        sqlx::query(
            "INSERT INTO message
             (id, group_id, seq, speaker_json, to_json, kind, body,
              cause_seq, operation_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(group.as_str())
        .bind(seq)
        .bind(serde_json::to_string(&draft.speaker)?)
        .bind(serde_json::to_string(&draft.to)?)
        .bind(kind)
        .bind(&draft.body)
        .bind(cause)
        .bind(operation)
        .bind(&created_at)
        .execute(&mut *conn)
        .await?;
        let event = JournalEntry::new("MessageAdded", actor)
            .with_project(&project)
            .with_payload(serde_json::json!({ "group_id": group, "seq": seq }));
        JournalEntry::append_event(conn, event, &created_at).await?;
        Ok(Message {
            id,
            group_id: group.clone(),
            seq,
            speaker: draft.speaker,
            to: draft.to,
            kind: draft.kind,
            body: draft.body,
            created_at,
        })
    }

    pub(in super::super) async fn delivery_count(
        conn: &mut Tx,
        group: &GroupId,
        cause: i64,
    ) -> Result<i64, MessagesError> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM message_delivery WHERE group_id=? AND cause_seq=?",
        )
        .bind(group.as_str())
        .bind(cause)
        .fetch_one(conn)
        .await?)
    }

    pub(in super::super) async fn has_notice(
        conn: &mut Tx,
        group: &GroupId,
        cause: i64,
        body: &str,
    ) -> Result<bool, MessagesError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM message WHERE group_id=? AND cause_seq=?
             AND kind='notice' AND body=?)",
        )
        .bind(group.as_str())
        .bind(cause)
        .bind(body)
        .fetch_one(conn)
        .await?)
    }

    pub(in super::super) async fn schedule(
        conn: &mut Tx,
        message: &Message,
        slug: &str,
        cause: i64,
        round: i64,
    ) -> Result<(), MessagesError> {
        sqlx::query(
            "INSERT INTO message_delivery
             (id, group_id, slug, cause_seq, round, trigger_seq, state)
             VALUES (?, ?, ?, ?, ?, ?, 'waiting')",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(message.group_id.as_str())
        .bind(slug)
        .bind(cause)
        .bind(round)
        .bind(message.seq)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub(in super::super) async fn message(
        conn: &mut Tx,
        group: &GroupId,
        id: &str,
    ) -> Result<Message, MessagesError> {
        let seq: i64 = sqlx::query_scalar("SELECT seq FROM message WHERE group_id=? AND id=?")
            .bind(group.as_str())
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(MessagesError::NotFound)?;
        let mut rows = Self::messages_between(conn, group, seq - 1, seq, 1).await?;
        rows.pop().ok_or(MessagesError::NotFound)
    }
}
