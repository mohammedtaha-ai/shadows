//! Transactional SQL for agent identities.
use super::{Agent, AgentInput, Agents, AgentsError};
use crate::CoreError;
use crate::providers::{Providers, SlugBase};
use core_db::{CommandContext, Storage, StorageError};
use futures_core::future::BoxFuture;
use sqlx::{Row, SqliteConnection};
use std::path::PathBuf;
use std::sync::Arc;

/// The scope replays of configuration commands are classified under.
pub(super) const COMMAND_SCOPE: (&str, &str) = ("Profiles", "aylul");

fn agent_row(row: sqlx::sqlite::SqliteRow) -> Agent {
    Agent {
        slug: row.get("slug"),
        name: row.get("name"),
        provider: row.get::<Option<String>, _>("provider").unwrap_or_default(),
        default_model: row.get("default_model"),
        character_color: row.get("character_color"),
        deleted: row.get::<Option<String>, _>("deleted_at").is_some(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

/// The service's SQL, over the shared storage handle.
pub(super) struct AgentsStore {
    storage: Arc<Storage>,
}

impl AgentsStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// One write on the shared connection; the closure refuses with any owner's error.
    pub(super) async fn write<F, T>(&self, f: F) -> Result<T, CoreError>
    where
        F: for<'a> FnOnce(&'a mut SqliteConnection) -> BoxFuture<'a, Result<T, CoreError>>,
    {
        self.storage.write_txn_as(f).await
    }

    pub(super) async fn list(&self) -> Result<Vec<Agent>, AgentsError> {
        Ok(sqlx::query(
            "SELECT * FROM aylul_agent WHERE deleted_at IS NULL ORDER BY created_at, slug",
        )
        .fetch_all(self.storage.reader())
        .await?
        .into_iter()
        .map(agent_row)
        .collect())
    }

    pub(super) async fn get(&self, slug: &str) -> Result<Agent, AgentsError> {
        Ok(agent_row(
            sqlx::query("SELECT * FROM aylul_agent WHERE slug = ?")
                .bind(slug)
                .fetch_optional(self.storage.reader())
                .await?
                .ok_or_else(|| AgentsError::NotFound(slug.into()))?,
        ))
    }

    /// Whether the agent is live, read outside any write.
    pub(super) async fn require_live(&self, slug: &str) -> Result<(), AgentsError> {
        let mut conn = self.storage.reader().acquire().await?;
        Self::require_live_in(&mut conn, slug).await
    }

    pub(super) async fn require_live_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), AgentsError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM aylul_agent WHERE slug=? AND deleted_at IS NULL)",
        )
        .bind(slug)
        .fetch_one(conn)
        .await?;
        if exists {
            Ok(())
        } else {
            Err(AgentsError::NotFound(slug.into()))
        }
    }

    /// Refuses a provider change that leaves a live agent without its default model;
    /// `models` `None` refuses any live user.
    pub(super) async fn users_in(
        conn: &mut SqliteConnection,
        provider: &str,
        models: Option<&[String]>,
    ) -> Result<(), AgentsError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT name, default_model FROM aylul_agent WHERE provider=? AND deleted_at IS NULL
             ORDER BY slug",
        )
        .bind(provider)
        .fetch_all(&mut *conn)
        .await?;
        let names: Vec<String> = rows
            .into_iter()
            .filter(|(_, model)| models.is_none_or(|models| !models.contains(model)))
            .map(|(name, _)| name)
            .collect();
        if names.is_empty() {
            Ok(())
        } else {
            Err(AgentsError::ProviderInUse(names.join(", ")))
        }
    }

    pub(super) async fn save(
        &self,
        command_id: String,
        existing: Option<String>,
        mut input: AgentInput,
        root: PathBuf,
    ) -> Result<String, CoreError> {
        self.write(move |conn| {
            Box::pin(async move {
                if input.character_color.is_none() {
                    let stored = if let Some(id) = &existing {
                        sqlx::query_scalar::<_, String>(
                            "SELECT character_color FROM aylul_agent WHERE slug=?",
                        )
                        .bind(id)
                        .fetch_optional(&mut *conn)
                        .await
                        .map_err(StorageError::from)?
                    } else {
                        None
                    };
                    input.character_color = Some(stored.unwrap_or_else(|| "purple".into()));
                }
                let command = match &existing {
                    Some(id) => CommandContext::user(
                        command_id,
                        "agent.update",
                        serde_json::json!({"slug":id,"input":input}),
                    ),
                    None => {
                        CommandContext::user(command_id, "agent.create", serde_json::json!(input))
                    }
                };
                if let Some(id) =
                    CommandContext::classify(conn, &command, COMMAND_SCOPE.0, COMMAND_SCOPE.1)
                        .await?
                {
                    return Ok(id);
                }
                let models = Providers::models_in(conn, &input.provider).await?;
                if !models.contains(&input.default_model) {
                    return Err(AgentsError::Invalid("default model is not offered".into()).into());
                }
                let id = match existing {
                    Some(id) => {
                        Self::require_live_in(conn, &id).await?;
                        id
                    }
                    None => Self::free_slug_in(conn, &SlugBase::new(&input.name, "agent")).await?,
                };
                Agents::seed_home(&root.join("profiles").join(&id))?;
                Self::upsert_in(conn, &id, &input).await?;
                Self::record_in(conn, &command, &id).await?;
                Ok(id)
            })
        })
        .await
    }

    async fn free_slug_in(
        conn: &mut SqliteConnection,
        base: &SlugBase,
    ) -> Result<String, AgentsError> {
        let mut n = 1;
        loop {
            let candidate = base.candidate(n);
            let taken: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM aylul_agent WHERE slug=?)")
                    .bind(&candidate)
                    .fetch_one(&mut *conn)
                    .await?;
            if !taken {
                return Ok(candidate);
            }
            n += 1;
        }
    }

    async fn upsert_in(
        conn: &mut SqliteConnection,
        id: &str,
        input: &AgentInput,
    ) -> Result<(), AgentsError> {
        let ts = Storage::now();
        sqlx::query(
            "INSERT INTO aylul_agent
            (slug,name,provider,default_model,created_at,updated_at,character_color)
            VALUES (?,?,?,?,?,?,?)
            ON CONFLICT(slug) DO UPDATE SET name=excluded.name, provider=excluded.provider,
            default_model=excluded.default_model, updated_at=excluded.updated_at,
            character_color=excluded.character_color",
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.provider)
        .bind(&input.default_model)
        .bind(&ts)
        .bind(&ts)
        .bind(&input.character_color)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub(super) async fn tombstone_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), AgentsError> {
        sqlx::query("UPDATE aylul_agent SET deleted_at=?, updated_at=? WHERE slug=?")
            .bind(Storage::now())
            .bind(Storage::now())
            .bind(slug)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub(super) async fn record_in(
        conn: &mut SqliteConnection,
        command: &CommandContext,
        id: &str,
    ) -> Result<(), StorageError> {
        let (scope, key) = COMMAND_SCOPE;
        CommandContext::record_command(conn, command, scope, key, "Profile", id, &Storage::now())
            .await
    }
}
