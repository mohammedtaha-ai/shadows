//! Top-level migration entry that includes extra counter tables.
use sea_orm_migration::prelude::*;

use crate::storage::sqlite::migrations::m20240101_000001_init as init;
use crate::storage::sqlite::migrations::m20240101_000002_event_seq_unique as seq_uniq;

#[derive(DeriveMigrationName)]
pub struct ExtraTables;

#[async_trait::async_trait]
impl MigrationTrait for ExtraTables {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(crate::storage::EXTRA_SCHEMA_SQLITE)
            .await?;
        Ok(())
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(init::Migration),
            Box::new(seq_uniq::Migration),
            Box::new(ExtraTables),
        ]
    }
}
