//! SQL for the one person row.
use super::{Person, PersonPatch, Theme};
use core_db::{CommandContext, Storage, StorageError};
use sqlx::{Row, sqlite::SqliteRow};
use std::sync::Arc;

/// The scope replays of configuration commands are classified under.
const COMMAND_SCOPE: (&str, &str) = ("Profiles", "aylul");

/// The service's SQL, over the shared storage handle.
#[derive(Clone)]
pub(super) struct PersonStore {
    storage: Arc<Storage>,
}

impl PersonStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub(super) async fn get(&self) -> Result<Person, StorageError> {
        let row = sqlx::query(
            "SELECT name,aylul_calls_me,language,theme,updated_at
            FROM aylul_person WHERE id=1",
        )
        .fetch_optional(self.storage.reader())
        .await?;
        person_from(row)
    }

    pub(super) async fn save(
        &self,
        command: CommandContext,
        input: PersonPatch,
    ) -> Result<(), StorageError> {
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let (scope, key) = COMMAND_SCOPE;
                    if CommandContext::classify(conn, &command, scope, key)
                        .await?
                        .is_some()
                    {
                        return Ok(());
                    }
                    // Merge under the writer lock, so a concurrent update keeps its unsent fields.
                    let stored = person_from(
                        sqlx::query(
                            "SELECT name,aylul_calls_me,language,theme,updated_at
                            FROM aylul_person WHERE id=1",
                        )
                        .fetch_optional(&mut *conn)
                        .await?,
                    )?;
                    let theme = match input.theme.unwrap_or(stored.theme) {
                        Theme::Dark => "dark",
                        Theme::Light => "light",
                        Theme::System => "system",
                    };
                    sqlx::query(
                        "INSERT INTO aylul_person (id,name,aylul_calls_me,language,theme,updated_at)
                VALUES (1,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,
                aylul_calls_me=excluded.aylul_calls_me, language=excluded.language,
                theme=excluded.theme, updated_at=excluded.updated_at",
                    )
                    .bind(input.name.unwrap_or(stored.name))
                    .bind(input.aylul_calls_me.unwrap_or(stored.aylul_calls_me))
                    .bind(input.language.unwrap_or(stored.language))
                    .bind(theme)
                    .bind(Storage::now())
                    .execute(&mut *conn)
                    .await?;
                    CommandContext::record_command(
                        conn,
                        &command,
                        scope,
                        key,
                        "Profile",
                        "person",
                        &Storage::now(),
                    )
                    .await
                })
            })
            .await
    }
}

/// The stored row as a person, or the defaults before the first write.
fn person_from(row: Option<SqliteRow>) -> Result<Person, StorageError> {
    let Some(row) = row else {
        return Ok(Person::default());
    };
    Ok(Person {
        name: row.get("name"),
        aylul_calls_me: row.get("aylul_calls_me"),
        language: row.get("language"),
        theme: serde_json::from_value(serde_json::Value::String(row.get("theme")))?,
        updated_at: Some(row.get("updated_at")),
    })
}
