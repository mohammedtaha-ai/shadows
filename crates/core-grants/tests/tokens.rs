//! Grant bearer persistence and revocation without an application fixture.
use core_db::{CommandContext, Storage, StorageError};
use core_grants::{GrantKind, Grants, Token};
use core_projects::{ProjectId, Projects};
use core_threads::{ThreadId, Threads};
use std::sync::Arc;

struct World {
    storage: Arc<Storage>,
    project: ProjectId,
    thread: ThreadId,
    _tmp: tempfile::TempDir,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let projects = Projects::new(storage.clone());
    let project = projects
        .create(
            "project".into(),
            "project",
            "Project",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    let threads = Threads::new(storage.clone(), projects);
    let thread = threads
        .create("thread".into(), &project, "Chat", None, None)
        .await
        .unwrap()
        .id;
    World {
        _tmp: tmp,
        storage,
        project,
        thread,
    }
}

fn ctx(id: &str, kind: &str) -> CommandContext {
    CommandContext::user(id.into(), kind, serde_json::json!({}))
}

fn issue_ctx(id: &str) -> CommandContext {
    ctx(id, "McpGrantIssue")
}

#[tokio::test]
async fn external_connection_replays_without_a_token_and_can_be_revoked() {
    let app = world().await;
    let grants = Grants::new(app.storage.clone(), "http://127.0.0.1:4318/mcp".into());
    let issued = grants
        .issue_external("global".into(), None, vec![app.project.clone()])
        .await
        .unwrap();
    let token = issued.token.unwrap();
    assert_eq!(
        grants.authorize_external(&token).await.unwrap().unwrap(),
        issued.connection
    );
    assert!(cells_holding(&app, &token).await.is_empty());
    let scope = grants
        .external_project(&issued.connection.id, &app.project)
        .await
        .unwrap();
    let replay = grants
        .issue_external("global".into(), None, vec![app.project.clone()])
        .await
        .unwrap();
    assert_eq!(replay.connection, issued.connection);
    assert!(replay.token.is_none());
    grants
        .revoke_external("revoke".into(), &issued.connection.id)
        .await
        .unwrap();
    assert!(grants.authorize_external(&token).await.unwrap().is_none());
    let mut tx = app.storage.reader().begin().await.unwrap();
    assert!(matches!(
        Grants::check_live_in(&mut tx, &scope).await,
        Err(core_grants::GrantsError::GrantInvalid)
    ));
}

#[tokio::test]
async fn upgrading_revokes_old_external_bearers_and_keeps_their_grant_id() {
    use sqlx::migrate::Migrator;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("upgrade.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    let before = sqlx::migrate!("../core-db/migrations");
    Migrator::with_migrations(
        before
            .iter()
            .filter(|migration| migration.version <= 73)
            .cloned()
            .collect(),
    )
    .run(&pool)
    .await
    .unwrap();
    let token = Token::generate();
    sqlx::query(
        "INSERT INTO project (id, slug, name, directory, created_at)
         VALUES ('p', 'old', 'Old', ?, '2026-01-01')",
    )
    .bind(temp.path().to_str().unwrap())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO mcp_grant (id, kind, project_id, token_hash, created_at)
         VALUES ('old-grant', 'project', 'p', ?, '2026-01-01')",
    )
    .bind(token.hash())
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let storage = Arc::new(Storage::open(&path).await.unwrap());
    let grants = Grants::new(storage.clone(), "http://127.0.0.1:4318/mcp".into());
    assert!(grants.authorize(token.as_str()).await.unwrap().is_none());
    assert!(
        grants
            .authorize_external(token.as_str())
            .await
            .unwrap()
            .is_none()
    );
    let revoked: Option<String> =
        sqlx::query_scalar("SELECT revoked_at FROM mcp_grant WHERE id = 'old-grant'")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert!(revoked.is_some());
    storage.close().await.unwrap();
}

/// Every text cell of every table that holds `needle`, as `table.column`.
async fn cells_holding(app: &World, needle: &str) -> Vec<String> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
          ORDER BY name",
    )
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    let mut found = Vec::new();
    for table in tables {
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                .bind(&table)
                .fetch_all(app.storage.reader())
                .await
                .unwrap();
        for column in columns {
            // Names read from the schema itself, never from input.
            let hits: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT COUNT(*) FROM \"{table}\" WHERE instr(CAST(\"{column}\" AS TEXT), ?) > 0"
            )))
            .bind(needle)
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
            if hits > 0 {
                found.push(format!("{table}.{column}"));
            }
        }
    }
    found
}

#[tokio::test]
async fn a_token_is_stored_only_as_its_hash() {
    let app = world().await;
    let issued = Grants::issue_project_grant_for_test(&app.storage, &issue_ctx("g1"), &app.project)
        .await
        .unwrap();
    let project_token = issued.token.expect("the first issue shows the token");
    let (thread_grant, thread_token) =
        Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
            .await
            .unwrap();
    assert_eq!(issued.grant.kind, GrantKind::Project);
    assert_eq!(issued.grant.thread_id, None);
    assert_eq!(thread_grant.kind, GrantKind::Thread);
    assert_eq!(thread_grant.thread_id.as_ref(), Some(&app.thread));

    for (grant, token) in [
        (&issued.grant, &project_token),
        (&thread_grant, &thread_token),
    ] {
        // "shd_" and two UUIDs' 32 hex digits each.
        assert!(token.as_str().starts_with("shd_"), "{}", token.as_str());
        assert_eq!(token.as_str().len(), 4 + 64);
        assert!(
            !format!("{token:?}").contains(&token.as_str()[4..]),
            "Debug prints the token: {token:?}"
        );
        assert_eq!(
            cells_holding(&app, token.as_str()).await,
            Vec::<String>::new()
        );
        let stored: String = sqlx::query_scalar("SELECT token_hash FROM mcp_grant WHERE id = ?")
            .bind(grant.id.as_str())
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
        assert_eq!(stored, Token::hash_token(token.as_str()));
        assert_eq!(stored, token.hash());
        assert_eq!(
            Grants::grant_for_token_for_test(&app.storage, token.as_str())
                .await
                .unwrap(),
            Some(grant.clone())
        );
    }
    assert_ne!(project_token.as_str(), thread_token.as_str());
    assert_eq!(
        Grants::grant_for_token_for_test(&app.storage, "shd_unknown")
            .await
            .unwrap(),
        None
    );

    app.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_revoked_grant_no_longer_resolves() {
    let app = world().await;
    let issued = Grants::issue_project_grant_for_test(&app.storage, &issue_ctx("g1"), &app.project)
        .await
        .unwrap();
    let token = issued.token.unwrap();
    let revoked =
        Grants::revoke_grant_for_test(&app.storage, &ctx("r1", "McpGrantRevoke"), &issued.grant.id)
            .await
            .unwrap();
    assert!(revoked.revoked_at.is_some());
    assert_eq!(
        Grants::grant_for_token_for_test(&app.storage, token.as_str())
            .await
            .unwrap(),
        None
    );
    // A replay answers the same revoked grant.
    let again =
        Grants::revoke_grant_for_test(&app.storage, &ctx("r1", "McpGrantRevoke"), &issued.grant.id)
            .await
            .unwrap();
    assert_eq!(again, revoked);

    let (thread_grant, thread_token) =
        Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
            .await
            .unwrap();
    Grants::revoke_thread_grant_for_test(&app.storage, &thread_grant.id)
        .await
        .unwrap();
    assert_eq!(
        Grants::grant_for_token_for_test(&app.storage, thread_token.as_str())
            .await
            .unwrap(),
        None
    );
    // The person's revoke is for project grants: the Planner's is not theirs.
    let refused =
        Grants::revoke_grant_for_test(&app.storage, &ctx("r2", "McpGrantRevoke"), &thread_grant.id)
            .await;
    assert!(
        matches!(refused, Err(StorageError::NotFound(_))),
        "{refused:?}"
    );

    app.storage.close().await.unwrap();
}
