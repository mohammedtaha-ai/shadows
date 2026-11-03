use crate::{Group, GroupId, Member, Message, MessageId, MessageKind, MessagesError};
use core_db::Tx;
use core_threads::ThreadId;

pub(super) struct Store;

impl Store {
    pub(super) async fn insert_group(
        conn: &mut Tx,
        id: &GroupId,
        project: &str,
        name: &str,
        now: &str,
    ) -> Result<(), MessagesError> {
        sqlx::query(
            "INSERT INTO message_group (id, project_id, name, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(project)
        .bind(name)
        .bind(now)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub(super) async fn insert_member(
        conn: &mut Tx,
        id: &GroupId,
        slug: &str,
        name: &str,
        thread: &ThreadId,
    ) -> Result<(), MessagesError> {
        sqlx::query(
            "INSERT INTO message_group_member (group_id, slug, name, thread_id) VALUES (?, ?, ?, ?)",
        ).bind(id.as_str()).bind(slug).bind(name).bind(thread.as_str()).execute(conn).await?;
        Ok(())
    }

    pub(super) async fn group(conn: &mut Tx, id: &GroupId) -> Result<Group, MessagesError> {
        let (name, created_at) =
            sqlx::query_as("SELECT name, created_at FROM message_group WHERE id = ?")
                .bind(id.as_str())
                .fetch_optional(&mut *conn)
                .await?
                .ok_or(MessagesError::NotFound)?;
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT slug, name, thread_id FROM message_group_member WHERE group_id = ? ORDER BY slug",
        ).bind(id.as_str()).fetch_all(&mut *conn).await?;
        let members = rows
            .into_iter()
            .map(|(slug, name, thread)| Member {
                slug,
                name,
                thread_id: ThreadId::from_recorded(thread),
            })
            .collect();
        let needs_you: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM message m, json_each(m.to_json) a
                WHERE m.group_id = ? AND json_extract(a.value, '$.kind') = 'person'
                  AND m.seq > COALESCE((
                    SELECT MAX(seq) FROM message
                    WHERE group_id = m.group_id AND json_extract(speaker_json, '$.kind') = 'person'
                  ), 0)
            )",
        )
        .bind(id.as_str())
        .fetch_one(conn)
        .await?;
        Ok(Group {
            id: id.clone(),
            name,
            members,
            needs_you,
            created_at,
        })
    }

    pub(super) async fn groups(conn: &mut Tx) -> Result<Vec<Group>, MessagesError> {
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM message_group ORDER BY name, id")
            .fetch_all(&mut *conn)
            .await?;
        let mut groups = Vec::with_capacity(ids.len());
        for id in ids {
            groups.push(Self::group(conn, &GroupId::from_stored(id)).await?);
        }
        Ok(groups)
    }

    pub(super) async fn messages(
        conn: &mut Tx,
        group: &GroupId,
        after: i64,
    ) -> Result<Vec<Message>, MessagesError> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM message_group WHERE id = ?)")
                .bind(group.as_str())
                .fetch_one(&mut *conn)
                .await?;
        if !exists {
            return Err(MessagesError::NotFound);
        }
        type Row = (String, i64, String, String, String, String, String);
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, seq, speaker_json, to_json, kind, body, created_at FROM message
              WHERE group_id = ? AND seq > ? ORDER BY seq",
        )
        .bind(group.as_str())
        .bind(after)
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|(id, seq, speaker, to, kind, body, created_at)| {
                Ok(Message {
                    id: MessageId::from_stored(id),
                    group_id: group.clone(),
                    seq,
                    speaker: serde_json::from_str(&speaker)?,
                    to: serde_json::from_str(&to)?,
                    kind: if kind == "chat" {
                        MessageKind::Chat
                    } else {
                        MessageKind::Notice
                    },
                    body,
                    created_at,
                })
            })
            .collect()
    }
}
