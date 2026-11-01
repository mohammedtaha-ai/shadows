//! Transactional extension library metadata and per-agent enablement.
use crate::mcp::PreparedMcp;
use crate::{
    AgentExtensions, AgentItem, AgentMcpItem, ExtensionKind, ExtensionLibrary, Extensions,
    ExtensionsError, Item, McpItem, McpTransport,
};
use core_agents::Agents;
use core_db::{CommandContext, Storage, StorageError};
use sqlx::{Row, SqliteConnection};
use std::path::PathBuf;
use std::sync::Arc;

/// The scope replays of library commands are classified under.
const COMMAND_SCOPE: (&str, &str) = ("Profiles", "aylul");

fn item(row: &sqlx::sqlite::SqliteRow) -> Item {
    Item {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        bundled: row.get("bundled"),
    }
}

/// The service's SQL, over the shared storage handle.
pub(super) struct ExtensionsStore {
    storage: Arc<Storage>,
}

impl ExtensionsStore {
    pub(super) async fn bundled_present_or_declined(
        &self,
        kind: ExtensionKind,
        name: &str,
    ) -> Result<bool, ExtensionsError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM aylul_extension WHERE kind=? AND name=?)
             OR EXISTS(SELECT 1 FROM aylul_extension_declined WHERE kind=? AND name=?)",
        )
        .bind(kind.as_str())
        .bind(name)
        .bind(kind.as_str())
        .bind(name)
        .fetch_one(self.storage.reader())
        .await?)
    }

    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub(super) async fn library(&self) -> Result<ExtensionLibrary, ExtensionsError> {
        let rows = sqlx::query("SELECT * FROM aylul_extension ORDER BY name,id")
            .fetch_all(self.storage.reader())
            .await?;
        let mut out = ExtensionLibrary::default();
        for row in rows {
            if row.get::<String, _>("kind") == "plugin" {
                out.plugins.push(item(&row));
            } else {
                out.skills.push(item(&row));
            }
        }
        let rows = sqlx::query("SELECT * FROM aylul_mcp_server ORDER BY name,id")
            .fetch_all(self.storage.reader())
            .await?;
        for row in rows {
            out.mcp.push(mcp_item(&row)?);
        }
        Ok(out)
    }
    pub(super) async fn agent_library(
        &self,
        slug: &str,
    ) -> Result<AgentExtensions, ExtensionsError> {
        let mut conn = self.storage.reader().acquire().await?;
        Agents::require_live_in(&mut conn, slug).await?;
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
                transport: None,
            };
            if row.get::<String, _>("kind") == "plugin" {
                out.plugins.push(v);
            } else {
                out.skills.push(v);
            }
        }
        let rows = sqlx::query(
            "SELECT e.*,COALESCE(a.enabled,0) AS enabled FROM aylul_mcp_server e
             LEFT JOIN aylul_agent_mcp a ON a.mcp_id=e.id AND a.agent_slug=? ORDER BY e.name,e.id",
        )
        .bind(slug)
        .fetch_all(&mut *conn)
        .await?;
        for row in rows {
            out.mcp.push(AgentMcpItem {
                item: mcp_item(&row)?,
                enabled: row.get("enabled"),
            });
        }
        Ok(out)
    }
    async fn record(
        conn: &mut SqliteConnection,
        c: &CommandContext,
        out: &str,
    ) -> Result<(), StorageError> {
        let (scope, key) = COMMAND_SCOPE;
        CommandContext::record_command(conn, c, scope, key, "Extension", out, &Storage::now()).await
    }
    pub(super) async fn add(
        &self,
        c: CommandContext,
        kind: ExtensionKind,
        source: PathBuf,
        library: PathBuf,
        bundled: bool,
    ) -> Result<Item, ExtensionsError> {
        let created = std::sync::Arc::new(std::sync::Mutex::new(None));
        let cleanup = created.clone();
        let result = self
            .storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(out) =
                        CommandContext::classify(conn, &c, COMMAND_SCOPE.0, COMMAND_SCOPE.1).await?
                    {
                        return Ok(serde_json::from_str(&out)?);
                    }
                    let id = uuid::Uuid::new_v4().to_string();
                    let destination = library.join(&id);
                    let (mut item, digest) = tokio::task::spawn_blocking(move || {
                        Extensions::publish(&source, &library, kind, &id)
                    })
                    .await
                    .map_err(|_| StorageError::Unavailable("extension copy".into()))??;
                    item.bundled = bundled;
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
                        return Err(ExtensionsError::NameConflict);
                    }
                    sqlx::query(
                        "INSERT INTO aylul_extension
                         (id,kind,name,description,digest,created_at,bundled)
                         VALUES (?,?,?,?,?,?,?)",
                    )
                    .bind(&item.id)
                    .bind(kind.as_str())
                    .bind(&item.name)
                    .bind(&item.description)
                    .bind(digest)
                    .bind(Storage::now())
                    .bind(bundled)
                    .execute(&mut *conn)
                    .await?;
                    Self::record(conn, &c, &serde_json::to_string(&item)?).await?;
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
        &self,
        c: CommandContext,
        kind: ExtensionKind,
        id: &str,
        secrets: Arc<core_secrets::Secrets>,
    ) -> Result<(), ExtensionsError> {
        let id = id.to_owned();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if CommandContext::classify(conn, &c, COMMAND_SCOPE.0, COMMAND_SCOPE.1)
                        .await?
                        .is_some()
                    {
                        return Ok(());
                    }
                    if kind == ExtensionKind::Mcp {
                        let deleted = sqlx::query("DELETE FROM aylul_mcp_server WHERE id=?")
                            .bind(&id)
                            .execute(&mut *conn)
                            .await?
                            .rows_affected();
                        if deleted == 0 {
                            return Err(ExtensionsError::NotFound);
                        }
                        secrets
                            .remove_scope_in(conn, &core_secrets::Secrets::mcp_scope(&id))
                            .await?;
                        return Ok(Self::record(conn, &c, "deleted").await?);
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
                        return Err(ExtensionsError::NotFound);
                    }
                    sqlx::query(
                        "INSERT OR IGNORE INTO aylul_extension_declined(kind,name,declined_at)
                         SELECT kind,name,? FROM aylul_extension WHERE id=? AND bundled=1",
                    )
                    .bind(Storage::now())
                    .bind(&id)
                    .execute(&mut *conn)
                    .await?;
                    sqlx::query("DELETE FROM aylul_agent_extension WHERE extension_id=?")
                        .bind(&id)
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("DELETE FROM aylul_extension WHERE id=?")
                        .bind(&id)
                        .execute(&mut *conn)
                        .await?;
                    Ok(Self::record(conn, &c, "deleted").await?)
                })
            })
            .await
    }
    pub(super) async fn toggle(
        &self,
        c: CommandContext,
        slug: &str,
        kind: ExtensionKind,
        id: &str,
        enabled: bool,
    ) -> Result<AgentItem, ExtensionsError> {
        let (slug, id) = (slug.to_owned(), id.to_owned());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(out) =
                        CommandContext::classify(conn, &c, COMMAND_SCOPE.0, COMMAND_SCOPE.1).await?
                    {
                        return Ok(serde_json::from_str(&out)?);
                    }
                    Agents::require_live_in(conn, &slug).await?;
                    if kind == ExtensionKind::Mcp {
                        let row = sqlx::query("SELECT * FROM aylul_mcp_server WHERE id=?")
                            .bind(&id)
                            .fetch_optional(&mut *conn)
                            .await?
                            .ok_or(ExtensionsError::NotFound)?;
                        sqlx::query(
                            "INSERT INTO aylul_agent_mcp(agent_slug,mcp_id,enabled) VALUES (?,?,?)
                             ON CONFLICT(agent_slug,mcp_id) DO UPDATE SET enabled=excluded.enabled",
                        )
                        .bind(&slug)
                        .bind(&id)
                        .bind(enabled)
                        .execute(&mut *conn)
                        .await?;
                        let mcp = mcp_item(&row)?;
                        let out = AgentItem {
                            item: Item {
                                id: mcp.id,
                                name: mcp.name,
                                description: mcp.description,
                                bundled: false,
                            },
                            enabled,
                            transport: Some(mcp.transport),
                        };
                        Self::record(conn, &c, &serde_json::to_string(&out)?).await?;
                        return Ok(out);
                    }
                    let row = sqlx::query("SELECT * FROM aylul_extension WHERE id=? AND kind=?")
                        .bind(&id)
                        .bind(kind.as_str())
                        .fetch_optional(&mut *conn)
                        .await?
                        .ok_or(ExtensionsError::NotFound)?;
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
                        transport: None,
                    };
                    Self::record(conn, &c, &serde_json::to_string(&out)?).await?;
                    Ok(out)
                })
            })
            .await
    }

    pub(super) async fn remove_agent_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), ExtensionsError> {
        sqlx::query("DELETE FROM aylul_agent_extension WHERE agent_slug=?")
            .bind(slug)
            .execute(&mut *conn)
            .await?;
        sqlx::query("DELETE FROM aylul_agent_mcp WHERE agent_slug=?")
            .bind(slug)
            .execute(conn)
            .await?;
        Ok(())
    }
}

fn mcp_item(row: &sqlx::sqlite::SqliteRow) -> Result<McpItem, ExtensionsError> {
    Ok(McpItem {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        transport: serde_json::from_str(&row.get::<String, _>("config_json"))?,
    })
}

fn mcp_names(transport: &McpTransport) -> Vec<String> {
    match transport {
        McpTransport::Http { headers, .. } => headers,
        McpTransport::Stdio { env, .. } => env,
    }
    .iter()
    .map(|pair| pair.name.clone())
    .collect()
}

impl ExtensionsStore {
    pub(super) async fn save_mcp(
        &self,
        c: CommandContext,
        prepared: PreparedMcp,
        update: bool,
        secrets: Arc<core_secrets::Secrets>,
    ) -> Result<McpItem, ExtensionsError> {
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(out) =
                        CommandContext::classify(conn, &c, COMMAND_SCOPE.0, COMMAND_SCOPE.1).await?
                    {
                        return Ok(serde_json::from_str(&out)?);
                    }
                    let item = prepared.item;
                    let previous = sqlx::query("SELECT * FROM aylul_mcp_server WHERE id=?")
                        .bind(&item.id)
                        .fetch_optional(&mut *conn)
                        .await?;
                    if update && previous.is_none() {
                        return Err(ExtensionsError::NotFound);
                    }
                    let old_names = match previous {
                        Some(row) => mcp_names(&mcp_item(&row)?.transport),
                        None => Vec::new(),
                    };
                    if prepared
                        .names_without_values
                        .iter()
                        .any(|name| !old_names.contains(name))
                    {
                        return Err(ExtensionsError::Invalid(
                            "a new MCP name needs a value".into(),
                        ));
                    }
                    let duplicate: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM aylul_mcp_server WHERE name=? AND id<>?)",
                    )
                    .bind(&item.name)
                    .bind(&item.id)
                    .fetch_one(&mut *conn)
                    .await?;
                    if duplicate {
                        return Err(ExtensionsError::NameConflict);
                    }
                    let config = serde_json::to_string(&item.transport)?;
                    if update {
                        sqlx::query(
                    "UPDATE aylul_mcp_server SET name=?,description=?,config_json=? WHERE id=?",
                )
                .bind(&item.name).bind(&item.description).bind(config).bind(&item.id)
                .execute(&mut *conn).await?;
                    } else {
                        sqlx::query(
                    "INSERT INTO aylul_mcp_server(id,name,description,config_json,created_at)
                     VALUES (?,?,?,?,?)",
                )
                .bind(&item.id).bind(&item.name).bind(&item.description).bind(config)
                .bind(Storage::now()).execute(&mut *conn).await?;
                    }
                    for write in prepared.writes {
                        if core_secrets::Secrets::replayed_write_in(conn, &write)
                            .await?
                            .is_none()
                        {
                            core_secrets::Secrets::put_in(conn, write).await?;
                        }
                    }
                    let new_names = mcp_names(&item.transport);
                    for old in old_names.into_iter().filter(|old| !new_names.contains(old)) {
                        let removal = secrets.mcp_value_removal(
                            format!("{}:remove:{old}", c.command_id),
                            &item.id,
                            &old,
                        )?;
                        if !core_secrets::Secrets::replayed_removal_in(conn, &removal).await? {
                            core_secrets::Secrets::remove_in(conn, removal).await?;
                        }
                    }
                    Self::record(conn, &c, &serde_json::to_string(&item)?).await?;
                    Ok(item)
                })
            })
            .await
    }
}
