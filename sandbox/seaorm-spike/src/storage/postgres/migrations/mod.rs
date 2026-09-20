//! Postgres migrations for SeaORM spike.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Init;

#[async_trait::async_trait]
impl MigrationTrait for Init {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("v1_init.sql"))
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS research_artifact, durable_event, command_record, operation, project CASCADE")
            .await?;
        Ok(())
    }
}

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(Init)]
    }
}
