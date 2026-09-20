//! Initial schema migration for SeaORM SQLite.
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Project::Table)
                    .col(ColumnDef::new(Project::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Project::Name).string().not_null())
                    .col(ColumnDef::new(Project::CreatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Operation::Table)
                    .col(
                        ColumnDef::new(Operation::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Operation::Kind).string().not_null())
                    .col(ColumnDef::new(Operation::Status).string().not_null())
                    .col(ColumnDef::new(Operation::ThreadId).uuid())
                    .col(ColumnDef::new(Operation::WorkflowId).uuid())
                    .col(ColumnDef::new(Operation::TaskId).uuid())
                    .col(
                        ColumnDef::new(Operation::RuntimeInstanceId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Operation::CreatedAt).string().not_null())
                    .col(
                        ColumnDef::new(Operation::DurableSeq)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Operation::Outcome).string())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DurableEvent::Table)
                    .col(
                        ColumnDef::new(DurableEvent::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DurableEvent::DurableSeq)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(DurableEvent::Kind).string().not_null())
                    .col(ColumnDef::new(DurableEvent::OccurredAt).string().not_null())
                    .col(ColumnDef::new(DurableEvent::OperationId).uuid())
                    .col(ColumnDef::new(DurableEvent::Payload).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CommandRecord::Table)
                    .col(
                        ColumnDef::new(CommandRecord::CommandId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CommandRecord::Principal).uuid().not_null())
                    .col(ColumnDef::new(CommandRecord::Scope).string().not_null())
                    .col(ColumnDef::new(CommandRecord::OperationId).uuid().not_null())
                    .col(
                        ColumnDef::new(CommandRecord::RecordedAt)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ResearchArtifact::Table)
                    .col(
                        ColumnDef::new(ResearchArtifact::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ResearchArtifact::ProjectId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ResearchArtifact::Title).string().not_null())
                    .col(ColumnDef::new(ResearchArtifact::Source).string())
                    .col(ColumnDef::new(ResearchArtifact::Summary).text().not_null())
                    .col(
                        ColumnDef::new(ResearchArtifact::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // FTS5 virtual table — backend-specific.
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE VIRTUAL TABLE IF NOT EXISTS research_fts USING fts5(\
                title, summary, content='research_artifact', content_rowid='rowid', tokenize='unicode61')",
        )
        .await?;
        // Triggers to keep FTS in sync.
        db.execute_unprepared(
            "CREATE TRIGGER IF NOT EXISTS research_ai AFTER INSERT ON research_artifact BEGIN \
                INSERT INTO research_fts(rowid, title, summary) VALUES (new.rowid, new.title, new.summary); \
             END",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TRIGGER IF NOT EXISTS research_ad AFTER DELETE ON research_artifact BEGIN \
                INSERT INTO research_fts(research_fts, rowid, title, summary) VALUES('delete', old.rowid, old.title, old.summary); \
             END",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TRIGGER IF NOT EXISTS research_au AFTER UPDATE ON research_artifact BEGIN \
                INSERT INTO research_fts(research_fts, rowid, title, summary) VALUES('delete', old.rowid, old.title, old.summary); \
                INSERT INTO research_fts(rowid, title, summary) VALUES (new.rowid, new.title, new.summary); \
             END",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DurableEvent::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CommandRecord::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ResearchArtifact::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Operation::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Project::Table).to_owned())
            .await?;
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS research_fts")
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum Project {
    Table,
    Id,
    Name,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum Operation {
    Table,
    Id,
    Kind,
    Status,
    ThreadId,
    WorkflowId,
    TaskId,
    RuntimeInstanceId,
    CreatedAt,
    DurableSeq,
    Outcome,
}

#[derive(DeriveIden)]
pub enum DurableEvent {
    Table,
    Id,
    DurableSeq,
    Kind,
    OccurredAt,
    OperationId,
    Payload,
}

#[derive(DeriveIden)]
pub enum CommandRecord {
    Table,
    CommandId,
    Principal,
    Scope,
    OperationId,
    RecordedAt,
}

#[derive(DeriveIden)]
pub enum ResearchArtifact {
    Table,
    Id,
    ProjectId,
    Title,
    Source,
    Summary,
    CreatedAt,
}
