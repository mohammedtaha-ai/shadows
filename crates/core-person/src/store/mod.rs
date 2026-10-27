//! SQL for the one person row.
use super::{Person, PersonPatch, Theme};
use core_db::{CommandContext, Storage, StorageError};
use sqlx::Row;
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
        match row {
            None => Ok(Person::default()),
            Some(row) => Ok(Person {
                name: row.get("name"),
                aylul_calls_me: row.get("aylul_calls_me"),
                language: row.get("language"),
                theme: serde_json::from_value(serde_json::Value::String(row.get("theme")))?,
                updated_at: Some(row.get("updated_at")),
            }),
        }
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
                    // Merge against the current row under the writer lock to preserve unsent fields.
                    let row = sqlx::query(
                        "SELECT name,aylul_calls_me,language,theme FROM aylul_person WHERE id=1",
                    )
                    .fetch_optional(&mut *conn)
                    .await?;
                    let stored = match row {
                        Some(row) => Person {
                            name: row.get("name"),
                            aylul_calls_me: row.get("aylul_calls_me"),
                            language: row.get("language"),
                            theme: serde_json::from_value(serde_json::Value::String(
                                row.get("theme"),
                            ))?,
                            updated_at: None,
                        },
                        None => Person::default(),
                    };
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
