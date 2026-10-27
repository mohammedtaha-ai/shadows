//! Compatibility with the first Windows profiles migration, without rewriting history.
use core_db::Storage;
use sqlx::migrate::{Migration, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{AssertSqlSafe, SqlSafeStr, SqlitePool};

async fn database(path: &std::path::Path, changed_version: i64, suffix: &str) -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    let migrations = sqlx::migrate!("./migrations")
        .iter()
        .map(|migration| {
            if migration.version != changed_version {
                return migration.clone();
            }
            let sql = migration
                .sql
                .as_str()
                .replace("\r\n", "\n")
                .replace('\n', "\r\n");
            Migration::new(
                migration.version,
                migration.description.clone(),
                migration.migration_type,
                AssertSqlSafe(format!("{sql}{suffix}")).into_sql_str(),
                migration.no_tx,
            )
        })
        .collect();
    Migrator::with_migrations(migrations)
        .run(&pool)
        .await
        .unwrap();
    pool
}

// A refused open returns no Storage to close. The parent owns the directory
// until the child process exits, which releases SQLx's worker connections.
fn rejected_open_database(test: &str, filename: &str) -> Option<std::path::PathBuf> {
    const DIRECTORY: &str = "SHADOWS_TEST_REJECTED_OPEN_DIRECTORY";
    if let Some(directory) = std::env::var_os(DIRECTORY) {
        return Some(std::path::PathBuf::from(directory).join(filename));
    }
    let temporary = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env(DIRECTORY, temporary.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "rejected-open child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    temporary
        .close()
        .expect("the exited child releases every database file");
    None
}

async fn checksum(pool: &SqlitePool) -> Vec<u8> {
    sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 29")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn windows_profiles_database_reopens_without_reapplying_or_rewriting_migration() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("existing.sqlite3");
    let pool = database(&path, 29, "").await;
    sqlx::query(
        "INSERT INTO aylul_provider
         (slug,name,base_url,models,context_window,created_at,updated_at)
         VALUES ('existing', 'Existing', 'https://example.test',
         '[\"model\"]', 200000, '2026-10-19', '2026-10-19')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = checksum(&pool).await;
    pool.close().await;

    let storage = Storage::open(&path).await.unwrap();
    assert_eq!(checksum(storage.reader()).await, before);
    let name: String =
        sqlx::query_scalar("SELECT name FROM aylul_provider WHERE slug = 'existing'")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(name, "Existing");
    let executions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 29")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(executions, 1);

    storage.close().await.unwrap();
}

#[tokio::test]
async fn fresh_database_keeps_the_canonical_lf_migration_checksum() {
    let temporary = tempfile::tempdir().unwrap();
    let storage = Storage::open(&temporary.path().join("fresh.sqlite3"))
        .await
        .unwrap();
    let canonical = sqlx::migrate!("./migrations");
    let expected = &canonical
        .iter()
        .find(|migration| migration.version == 29)
        .unwrap()
        .checksum;
    assert_eq!(
        checksum(storage.reader()).await.as_slice(),
        expected.as_ref()
    );

    storage.close().await.unwrap();
}

#[tokio::test]
async fn changed_profiles_sql_still_refuses_startup() {
    let Some(path) = rejected_open_database(
        "changed_profiles_sql_still_refuses_startup",
        "changed.sqlite3",
    ) else {
        return;
    };
    let pool = database(&path, 29, "-- changed beyond line endings\r\n").await;
    let before = checksum(&pool).await;
    pool.close().await;
    let error = Storage::open(&path)
        .await
        .err()
        .expect("changed SQL must be rejected");
    assert!(
        error
            .to_string()
            .contains("previously applied but has been modified")
    );
    let pool = SqlitePoolOptions::new()
        .connect_with(SqliteConnectOptions::new().filename(&path))
        .await
        .unwrap();
    assert_eq!(checksum(&pool).await, before);

    pool.close().await;
}

#[tokio::test]
async fn other_migrations_do_not_receive_the_profiles_compatibility_exception() {
    let Some(path) = rejected_open_database(
        "other_migrations_do_not_receive_the_profiles_compatibility_exception",
        "other.sqlite3",
    ) else {
        return;
    };
    database(&path, 28, "").await.close().await;
    let error = Storage::open(&path)
        .await
        .err()
        .expect("other checksum must be rejected");
    assert!(
        error
            .to_string()
            .contains("migration 28 was previously applied")
    );
}

#[tokio::test]
async fn migration_marks_known_urls_with_their_preset() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("presets.db");
    let pool = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    let migrations = sqlx::migrate!("./migrations")
        .iter()
        .filter(|m| m.version < 30)
        .cloned()
        .collect();
    Migrator::with_migrations(migrations)
        .run(&pool)
        .await
        .unwrap();
    for (slug, url) in [
        ("minimax", "https://api.minimax.io/anthropic"),
        ("custom", "https://custom.test"),
    ] {
        sqlx::query(
            "INSERT INTO aylul_provider
            (slug,name,base_url,models,context_window,created_at,updated_at)
            VALUES (?, 'Existing', ?, '[\"m\"]', 123456, 'old', 'old')",
        )
        .bind(slug)
        .bind(url)
        .execute(&pool)
        .await
        .unwrap();
    }
    pool.close().await;
    let storage = Storage::open(&path).await.unwrap();
    let rows: Vec<(String, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT slug,preset,label FROM aylul_provider ORDER BY slug")
            .fetch_all(storage.reader())
            .await
            .unwrap();
    assert_eq!(
        rows,
        vec![
            ("custom".into(), None, None),
            ("minimax".into(), Some("minimax".into()), None)
        ]
    );

    pool.close().await;
    storage.close().await.unwrap();
}

#[tokio::test]
async fn migration_gives_models_the_provider_window() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("windows.db");
    let pool = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    let migrations = sqlx::migrate!("./migrations")
        .iter()
        .filter(|m| m.version < 31)
        .cloned()
        .collect();
    Migrator::with_migrations(migrations)
        .run(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO aylul_provider
        (slug,name,base_url,models,context_window,created_at,updated_at)
        VALUES ('p','P','https://example.test','[\"z\",\"a\",\"b\"]',123456,'old','old')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let storage = Storage::open(&path).await.unwrap();
    let models: String = sqlx::query_scalar("SELECT models FROM aylul_provider WHERE slug='p'")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&models).unwrap(),
        serde_json::json!([
        {"id":"z","context_window":123456}, {"id":"a","context_window":123456},
        {"id":"b","context_window":123456}])
    );

    pool.close().await;
    storage.close().await.unwrap();
}

#[tokio::test]
async fn migration_colors_agents_purple() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("colors.db");
    let pool = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    let migrations = sqlx::migrate!("./migrations")
        .iter()
        .filter(|m| m.version < 32)
        .cloned()
        .collect();
    Migrator::with_migrations(migrations)
        .run(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO aylul_provider
        (slug,name,base_url,models,context_window,created_at,updated_at)
        VALUES ('p','P','https://example.test','[]',200000,'old','old')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO aylul_agent
        (slug,name,provider,default_model,created_at,updated_at)
        VALUES ('a','A','p','m','old','old')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let storage = Storage::open(&path).await.unwrap();
    let color: String =
        sqlx::query_scalar("SELECT character_color FROM aylul_agent WHERE slug='a'")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(color, "purple");

    pool.close().await;
    storage.close().await.unwrap();
}
