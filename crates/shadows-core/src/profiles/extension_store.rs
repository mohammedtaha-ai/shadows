//! Transactional extension library metadata and per-agent enablement.
use super::{AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Item, extension_files};
use crate::{
    db::CommandContext,
    db::{Storage, StorageError, classify, now, record_command},
};
use sqlx::{Row, SqliteConnection};
use std::path::PathBuf;

fn item(row: &sqlx::sqlite::SqliteRow) -> Item {
    Item {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
    }
}
pub(super) async fn library(storage: &Storage) -> Result<ExtensionLibrary, StorageError> {
    let rows = sqlx::query("SELECT * FROM aylul_extension ORDER BY name,id")
        .fetch_all(storage.reader())
        .await?;
    let mut out = ExtensionLibrary::default();
    for row in rows {
        if row.get::<String, _>("kind") == "plugin" {
            out.plugins.push(item(&row));
        } else {
            out.skills.push(item(&row));
        }
    }
    Ok(out)
}
pub(super) async fn agent_library(
    storage: &Storage,
    slug: &str,
) -> Result<AgentExtensions, StorageError> {
    let mut conn = storage.reader().acquire().await?;
    super::require_live_agent_in(&mut conn, slug).await?;
    let rows = sqlx::query(
        "SELECT e.*,COALESCE(a.enabled,0) AS enabled FROM aylul_extension e
        LEFT JOIN aylul_agent_extension a ON a.extension_id=e.id AND a.agent_slug=?
        ORDER BY e.name,e.id",
    )
    .bind(slug)
    .fetch_all(&mut *conn)
    .await?;
    let mut out = AgentExtensions::default();
    for row in rows {
        let v = AgentItem {
            item: item(&row),
            enabled: row.get("enabled"),
        };
        if row.get::<String, _>("kind") == "plugin" {
            out.plugins.push(v);
        } else {
            out.skills.push(v);
        }
    }
    Ok(out)
}
async fn record(
    conn: &mut SqliteConnection,
    c: &CommandContext,
    out: &str,
) -> Result<(), StorageError> {
    record_command(conn, c, "Profiles", "aylul", "Extension", out, &now()).await
}
pub(super) async fn add(
    storage: &Storage,
    c: CommandContext,
    kind: ExtensionKind,
    source: PathBuf,
    library: PathBuf,
) -> Result<Item, StorageError> {
    let created = std::sync::Arc::new(std::sync::Mutex::new(None));
    let cleanup = created.clone();
    let result = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                if let Some(out) = classify(conn, &c, "Profiles", "aylul").await? {
                    return Ok(serde_json::from_str(&out)?);
                }
                let id = uuid::Uuid::new_v4().to_string();
                let destination = library.join(&id);
                let (item, digest) = tokio::task::spawn_blocking(move || {
                    extension_files::publish(&source, &library, kind, &id)
                })
                .await
                .map_err(|_| StorageError::Unavailable("extension copy".into()))??;
                *created.lock().expect("import ownership") = Some(destination);
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM aylul_extension
            WHERE kind=? AND name=?)",
                )
                .bind(kind.as_str())
                .bind(&item.name)
                .fetch_one(&mut *conn)
                .await?;
                if exists {
                    return Err(StorageError::ExtensionNameConflict);
                }
                sqlx::query(
                    "INSERT INTO aylul_extension(id,kind,name,description,digest,created_at)
            VALUES (?,?,?,?,?,?)",
                )
                .bind(&item.id)
                .bind(kind.as_str())
                .bind(&item.name)
                .bind(&item.description)
                .bind(digest)
                .bind(now())
                .execute(&mut *conn)
                .await?;
                record(conn, &c, &serde_json::to_string(&item)?).await?;
                Ok(item)
            })
        })
        .await;
    if result.is_err()
        && let Some(path) = cleanup.lock().expect("import ownership").take()
    {
        let _ = std::fs::remove_dir_all(path);
    }
    result
}
pub(super) async fn remove(
    storage: &Storage,
    c: CommandContext,
    kind: ExtensionKind,
    id: &str,
) -> Result<(), StorageError> {
    let id = id.to_owned();
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &c, "Profiles", "aylul").await?.is_some() {
                    return Ok(());
                }
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM aylul_extension
            WHERE id=? AND kind=?)",
                )
                .bind(&id)
                .bind(kind.as_str())
                .fetch_one(&mut *conn)
                .await?;
                if !exists {
                    return Err(StorageError::ExtensionNotFound);
                }
                sqlx::query("DELETE FROM aylul_agent_extension WHERE extension_id=?")
                    .bind(&id)
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("DELETE FROM aylul_extension WHERE id=?")
                    .bind(&id)
                    .execute(&mut *conn)
                    .await?;
                record(conn, &c, "deleted").await
            })
        })
        .await
}
pub(super) async fn toggle(
    storage: &Storage,
    c: CommandContext,
    slug: &str,
    kind: ExtensionKind,
    id: &str,
    enabled: bool,
) -> Result<AgentItem, StorageError> {
    let (slug, id) = (slug.to_owned(), id.to_owned());
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                if let Some(out) = classify(conn, &c, "Profiles", "aylul").await? {
                    return Ok(serde_json::from_str(&out)?);
                }
                super::require_live_agent_in(conn, &slug).await?;
                let row = sqlx::query("SELECT * FROM aylul_extension WHERE id=? AND kind=?")
                    .bind(&id)
                    .bind(kind.as_str())
                    .fetch_optional(&mut *conn)
                    .await?
                    .ok_or(StorageError::ExtensionNotFound)?;
                sqlx::query(
                    "INSERT INTO aylul_agent_extension(agent_slug,extension_id,enabled)
            VALUES (?,?,?) ON CONFLICT(agent_slug,extension_id)
            DO UPDATE SET enabled=excluded.enabled",
                )
                .bind(&slug)
                .bind(&id)
                .bind(enabled)
                .execute(&mut *conn)
                .await?;
                let out = AgentItem {
                    item: item(&row),
                    enabled,
                };
                record(conn, &c, &serde_json::to_string(&out)?).await?;
                Ok(out)
            })
        })
        .await
}
