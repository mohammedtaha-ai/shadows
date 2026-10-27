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
    let code = core_code::Code::new(storage.clone(), projects.clone());
    let threads = Threads::new(storage.clone(), code, projects);
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
