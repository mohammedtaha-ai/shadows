//! Transactional SQL for provider configuration.
use super::{Agent, AgentInput, Provider, ProviderInput};
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, classify, now, record_command};
use sqlx::{Row, SqliteConnection};

fn provider_row(row: sqlx::sqlite::SqliteRow) -> Result<Provider, StorageError> {
    Ok(Provider {
        slug: row.get("slug"),
        name: row.get("name"),
        base_url: row.get("base_url"),
        preset: row.get("preset"),
        label: row.get("label"),
        models: serde_json::from_str(row.get("models"))?,
        key_set: row.get("key_set"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn agent_row(row: sqlx::sqlite::SqliteRow) -> Agent {
    Agent {
        slug: row.get("slug"),
        name: row.get("name"),
        provider: row.get::<Option<String>, _>("provider").unwrap_or_default(),
        default_model: row.get("default_model"),
        deleted: row.get::<Option<String>, _>("deleted_at").is_some(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

const PROVIDER_SELECT: &str = "SELECT p.*, EXISTS(SELECT 1 FROM secret_env s
    WHERE s.profile = 'provider/' || p.slug AND s.name = 'PROVIDER_API_KEY') AS key_set
    FROM aylul_provider p";

pub(super) async fn providers(storage: &Storage) -> Result<Vec<Provider>, StorageError> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "{PROVIDER_SELECT} ORDER BY p.created_at, p.slug"
    )))
    .fetch_all(storage.reader())
    .await?
    .into_iter()
    .map(provider_row)
    .collect()
}

pub(super) async fn provider(storage: &Storage, slug: &str) -> Result<Provider, StorageError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{PROVIDER_SELECT} WHERE p.slug = ?"
    )))
    .bind(slug)
    .fetch_optional(storage.reader())
    .await?
    .ok_or_else(|| StorageError::AylulProviderNotFound(slug.into()))?;
    provider_row(row)
}

pub(super) async fn agents(storage: &Storage) -> Result<Vec<Agent>, StorageError> {
    Ok(
        sqlx::query("SELECT * FROM aylul_agent WHERE deleted_at IS NULL ORDER BY created_at, slug")
            .fetch_all(storage.reader())
            .await?
            .into_iter()
            .map(agent_row)
            .collect(),
    )
}

pub(super) async fn agent(storage: &Storage, slug: &str) -> Result<Agent, StorageError> {
    Ok(agent_row(
        sqlx::query("SELECT * FROM aylul_agent WHERE slug = ?")
            .bind(slug)
            .fetch_optional(storage.reader())
            .await?
            .ok_or_else(|| StorageError::AylulProfileNotFound(slug.into()))?,
    ))
}

async fn slug(
    conn: &mut SqliteConnection,
    name: &str,
    table: &str,
    fallback: &str,
) -> Result<String, StorageError> {
    let mut base = String::new();
    for c in name.to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            base.push(c);
        } else if !base.is_empty() && !base.ends_with('-') {
            base.push('-');
        }
    }
    base = base.trim_matches('-').into();
    if base.is_empty() {
        base = fallback.into();
    }
    if !base.starts_with(|c: char| c.is_ascii_lowercase()) {
        base = format!("{fallback}-{base}");
    }
    base.truncate(29);
    let mut candidate = base.clone();
    let mut n = 2;
    loop {
        let taken: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT EXISTS(SELECT 1 FROM {table} WHERE slug=?)"
        )))
        .bind(&candidate)
        .fetch_one(&mut *conn)
        .await?;
        if !taken {
            return Ok(candidate);
        }
        let suffix = format!("-{n}");
        candidate = format!("{}{}", &base[..base.len().min(32 - suffix.len())], suffix);
        n += 1;
    }
}

async fn record(
    conn: &mut SqliteConnection,
    command: &CommandContext,
    id: &str,
) -> Result<(), StorageError> {
    record_command(conn, command, "Profiles", "aylul", "Profile", id, &now()).await
}

async fn users(
    conn: &mut SqliteConnection,
    provider: &str,
    models: Option<&[String]>,
) -> Result<(), StorageError> {
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
        Err(StorageError::ProviderInUse(names.join(", ")))
    }
}

pub(super) async fn save_provider(
    storage: &Storage,
    command: CommandContext,
    existing: Option<String>,
    input: ProviderInput,
) -> Result<String, StorageError> {
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &command, "Profiles", "aylul").await? {
                    return Ok(id);
                }
                let id = match existing {
                    Some(id) => {
                        require_provider_in(conn, &id).await?;
                        let ids: Vec<_> = input.models.iter().map(|m| m.id.clone()).collect();
                        users(conn, &id, Some(&ids)).await?;
                        id
                    }
                    None => {
                        slug(
                            conn,
                            input.name.as_deref().unwrap_or(""),
                            "aylul_provider",
                            "provider",
                        )
                        .await?
                    }
                };
                let ts = now();
                sqlx::query(
                    "INSERT INTO aylul_provider
            (slug,name,base_url,models,context_window,created_at,updated_at,preset,label)
            VALUES (?,?,?,?,?,?,?,?,?)
            ON CONFLICT(slug) DO UPDATE SET name=excluded.name, base_url=excluded.base_url,
            models=excluded.models, context_window=excluded.context_window,
            preset=excluded.preset, label=excluded.label,
            updated_at=excluded.updated_at",
                )
                .bind(&id)
                .bind(&input.name)
                .bind(&input.base_url)
                .bind(serde_json::to_string(&input.models)?)
                .bind(200_000)
                .bind(&ts)
                .bind(&ts)
                .bind(&input.preset)
                .bind(&input.label)
                .execute(&mut *conn)
                .await?;
                record(conn, &command, &id).await?;
                Ok(id)
            })
        })
        .await
}

pub(crate) async fn require_provider_in(
    conn: &mut SqliteConnection,
    slug: &str,
) -> Result<Vec<String>, StorageError> {
    let models: Option<String> =
        sqlx::query_scalar("SELECT models FROM aylul_provider WHERE slug=?")
            .bind(slug)
            .fetch_optional(conn)
            .await?;
    let models: Vec<super::ProviderModel> = serde_json::from_str(
        &models.ok_or_else(|| StorageError::AylulProviderNotFound(slug.into()))?,
    )?;
    Ok(models.into_iter().map(|m| m.id).collect())
}

pub(crate) async fn require_live_agent_in(
    conn: &mut SqliteConnection,
    slug: &str,
) -> Result<(), StorageError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM aylul_agent WHERE slug=? AND deleted_at IS NULL)",
    )
    .bind(slug)
    .fetch_one(conn)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(StorageError::AylulProfileNotFound(slug.into()))
    }
}

pub(super) async fn save_agent(
    storage: &Storage,
    command: CommandContext,
    existing: Option<String>,
    input: AgentInput,
    root: std::path::PathBuf,
) -> Result<String, StorageError> {
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &command, "Profiles", "aylul").await? {
                    return Ok(id);
                }
                let models = require_provider_in(conn, &input.provider).await?;
                if !models.contains(&input.default_model) {
                    return Err(StorageError::ProfileInvalid(
                        "default model is not offered".into(),
                    ));
                }
                let id = match existing {
                    Some(id) => {
                        require_live_agent_in(conn, &id).await?;
                        id
                    }
                    None => slug(conn, &input.name, "aylul_agent", "agent").await?,
                };
                crate::harness::profile_home::seed(&root.join("profiles").join(&id))?;
                let ts = now();
                sqlx::query(
                    "INSERT INTO aylul_agent
            (slug,name,provider,default_model,created_at,updated_at) VALUES (?,?,?,?,?,?)
            ON CONFLICT(slug) DO UPDATE SET name=excluded.name, provider=excluded.provider,
            default_model=excluded.default_model, updated_at=excluded.updated_at",
                )
                .bind(&id)
                .bind(&input.name)
                .bind(&input.provider)
                .bind(&input.default_model)
                .bind(&ts)
                .bind(&ts)
                .execute(&mut *conn)
                .await?;
                record(conn, &command, &id).await?;
                Ok(id)
            })
        })
        .await
}

pub(super) async fn delete_agent(
    storage: &Storage,
    command: CommandContext,
    slug: &str,
) -> Result<(), StorageError> {
    let slug = slug.to_owned();
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &command, "Profiles", "aylul")
                    .await?
                    .is_some()
                {
                    return Ok(());
                }
                require_live_agent_in(conn, &slug).await?;
                sqlx::query("UPDATE aylul_agent SET deleted_at=?, updated_at=? WHERE slug=?")
                    .bind(now())
                    .bind(now())
                    .bind(&slug)
                    .execute(&mut *conn)
                    .await?;
                crate::secrets::remove_scope_in(conn, &slug).await?;
                record(conn, &command, &slug).await
            })
        })
        .await
}

pub(super) async fn delete_provider(
    storage: &Storage,
    command: CommandContext,
    slug: &str,
) -> Result<(), StorageError> {
    let slug = slug.to_owned();
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &command, "Profiles", "aylul")
                    .await?
                    .is_some()
                {
                    return Ok(());
                }
                require_provider_in(conn, &slug).await?;
                users(conn, &slug, None).await?;
                crate::secrets::remove_scope_in(conn, &format!("provider/{slug}")).await?;
                sqlx::query("DELETE FROM aylul_provider WHERE slug=?")
                    .bind(&slug)
                    .execute(&mut *conn)
                    .await?;
                record(conn, &command, &slug).await
            })
        })
        .await
}
