//! v1→v2 migration: add unique constraint on durable_seq.

use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite can't add a UNIQUE constraint via ALTER TABLE for a backfilled column,
        // so we must rebuild the table. This is backend-specific.
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE durable_event_v2 (\
                    id TEXT PRIMARY KEY NOT NULL,\
                    durable_seq INTEGER NOT NULL UNIQUE,\
                    kind TEXT NOT NULL,\
                    occurred_at TEXT NOT NULL,\
                    operation_id TEXT,\
                    payload TEXT NOT NULL)",
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared("INSERT INTO durable_event_v2 SELECT * FROM durable_event")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE durable_event")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE durable_event_v2 RENAME TO durable_event")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE durable_event_v2 (\
                    id TEXT PRIMARY KEY NOT NULL,\
                    durable_seq INTEGER NOT NULL,\
                    kind TEXT NOT NULL,\
                    occurred_at TEXT NOT NULL,\
                    operation_id TEXT,\
                    payload TEXT NOT NULL)",
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared("INSERT INTO durable_event_v2 SELECT * FROM durable_event")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE durable_event")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE durable_event_v2 RENAME TO durable_event")
            .await?;
        Ok(())
    }
}
