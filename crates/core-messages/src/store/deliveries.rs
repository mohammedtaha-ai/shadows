use super::Store;
use crate::{GroupId, MessagesError};
use core_db::Tx;
use core_threads::ThreadId;

#[derive(sqlx::FromRow)]
pub(in super::super) struct Delivery {
    pub id: String,
    pub group_id: String,
    pub slug: String,
    pub name: String,
    pub thread_id: String,
    pub delivered_seq: i64,
    pub cause_seq: i64,
    pub trigger_seq: i64,
    pub round: i64,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub mode: Option<String>,
}

impl Delivery {
    pub fn group(&self) -> GroupId {
        GroupId::from_stored(self.group_id.clone())
    }

    pub fn thread(&self) -> ThreadId {
        ThreadId::from_recorded(self.thread_id.clone())
    }
}

const COLUMNS: &str = "SELECT d.id, d.group_id, d.slug, m.name, m.thread_id, m.delivered_seq,
            d.cause_seq, d.trigger_seq, d.round, d.prompt, d.model, d.mode
       FROM message_delivery d JOIN message_group_member m USING (group_id, slug)";

impl Store {
    pub(in super::super) async fn candidates(
        conn: &mut Tx,
    ) -> Result<Vec<Delivery>, MessagesError> {
        let sql = format!(
            "{COLUMNS}
              WHERE d.state = 'starting' OR (d.state = 'waiting' AND NOT EXISTS (
                 SELECT 1 FROM message_delivery earlier
                 WHERE earlier.group_id=d.group_id AND earlier.slug=d.slug
                   AND (earlier.state IN ('starting','running')
                        OR (earlier.state='waiting' AND earlier.trigger_seq < d.trigger_seq))
              ))
              ORDER BY d.state, d.group_id, d.trigger_seq, d.slug"
        );
        Ok(sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .fetch_all(conn)
            .await?)
    }

    pub(in super::super) async fn prepare(
        conn: &mut Tx,
        delivery: &Delivery,
        prompt: &str,
        model: &str,
        mode: &str,
    ) -> Result<(), MessagesError> {
        sqlx::query(
            "UPDATE message_delivery SET state='starting', prompt=?, model=?, mode=? WHERE id=?",
        )
        .bind(prompt)
        .bind(model)
        .bind(mode)
        .bind(&delivery.id)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub(in super::super) async fn link(
        conn: &mut Tx,
        delivery: &Delivery,
        operation: &str,
    ) -> Result<(), MessagesError> {
        sqlx::query("UPDATE message_delivery SET state='running', operation_id=? WHERE id=?")
            .bind(operation)
            .bind(&delivery.id)
            .execute(&mut *conn)
            .await?;
        sqlx::query("UPDATE message_group_member SET delivered_seq=? WHERE group_id=? AND slug=?")
            .bind(delivery.trigger_seq)
            .bind(&delivery.group_id)
            .bind(&delivery.slug)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub(in super::super) async fn by_operation(
        conn: &mut Tx,
        operation: &str,
    ) -> Result<Option<Delivery>, MessagesError> {
        let sql = format!("{COLUMNS} WHERE d.operation_id=? AND d.state='running'");
        Ok(sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(operation)
            .fetch_optional(conn)
            .await?)
    }

    pub(in super::super) async fn finish(
        conn: &mut Tx,
        delivery: &Delivery,
        answered: bool,
    ) -> Result<(), MessagesError> {
        sqlx::query("UPDATE message_delivery SET state=? WHERE id=?")
            .bind(if answered { "answered" } else { "failed" })
            .bind(&delivery.id)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub(in super::super) async fn cursor(conn: &mut Tx) -> Result<i64, MessagesError> {
        Ok(
            sqlx::query_scalar("SELECT seq FROM message_cursor WHERE name='turn_ends'")
                .fetch_optional(conn)
                .await?
                .unwrap_or(0),
        )
    }

    pub(in super::super) async fn advance(conn: &mut Tx, seq: i64) -> Result<(), MessagesError> {
        sqlx::query(
            "INSERT INTO message_cursor (name, seq) VALUES ('turn_ends', ?)
             ON CONFLICT(name) DO UPDATE SET seq=excluded.seq WHERE seq < excluded.seq",
        )
        .bind(seq)
        .execute(conn)
        .await?;
        Ok(())
    }
}
