//! Validate embedded migrations against the recorded SQLite catalog.
use sqlx::migrate::{Migration, Migrator};
use sqlx::{AssertSqlSafe, SqlSafeStr, SqlitePool};

pub(super) struct EmbeddedMigrations;

impl EmbeddedMigrations {
    pub(super) async fn for_database(pool: &SqlitePool) -> Result<Migrator, sqlx::Error> {
        let canonical = sqlx::migrate!("./migrations");
        let recorded: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema
         WHERE type = 'table' AND name = '_sqlx_migrations')",
        )
        .fetch_one(pool)
        .await?;
        if !recorded {
            return Ok(canonical);
        }
        let applied: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 29")
                .fetch_optional(pool)
                .await?;
        let Some(applied) = applied else {
            return Ok(canonical);
        };
        let profiles = canonical
            .iter()
            .find(|migration| migration.version == 29)
            .expect("embedded profiles migration exists");
        // Only newline conversion of this embedded SQL is accepted. SQLx still checks
        // every checksum and dirty migration; no applied record or user row is changed.
        let windows_sql = profiles
            .sql
            .as_str()
            .replace("\r\n", "\n")
            .replace('\n', "\r\n");
        let windows = Migration::new(
            profiles.version,
            profiles.description.clone(),
            profiles.migration_type,
            AssertSqlSafe(windows_sql).into_sql_str(),
            profiles.no_tx,
        );
        if windows.checksum.as_ref() != applied.as_slice() {
            return Ok(canonical);
        }
        Ok(Migrator::with_migrations(
            canonical
                .iter()
                .map(|migration| {
                    if migration.version == 29 {
                        windows.clone()
                    } else {
                        migration.clone()
                    }
                })
                .collect(),
        ))
    }
}
