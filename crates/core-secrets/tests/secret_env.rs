//! Encrypted environment persistence through the public service.
use std::sync::Arc;

use core_db::{Storage, StorageError};
use core_secrets::{SecretKey, Secrets, SecretsError};

const VALUE: &str = "synthetic-fixture-value-9154";

struct World {
    storage: Arc<Storage>,
    secrets: Arc<Secrets>,
    tmp: tempfile::TempDir,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let service = Secrets::new(storage.clone(), Some(SecretKey::from_bytes([7; 32])));
    World {
        tmp,
        storage,
        secrets: service,
    }
}

async fn set(w: &World, id: &str, name: &str, value: &str) {
    w.secrets
        .set_env(id.into(), "aylul", name, value)
        .await
        .unwrap();
}

#[tokio::test]
async fn stored_ciphertext_is_not_the_value() {
    let w = world().await;
    set(&w, "a", "API_KEY", VALUE).await;
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT ciphertext FROM secret_env")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert!(bytes != VALUE.as_bytes());

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn list_names_in_order_without_values() {
    let w = world().await;
    set(&w, "a", "Z", VALUE).await;
    set(&w, "b", "API_KEY", VALUE).await;
    let entries = w.secrets.list_env("aylul").await.unwrap();
    assert_eq!(
        entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["API_KEY", "Z"]
    );
    assert!(!serde_json::to_string(&entries).unwrap().contains(VALUE));

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn replace_keeps_one_row() {
    let w = world().await;
    set(&w, "a", "API_KEY", VALUE).await;
    set(&w, "b", "API_KEY", "synthetic-replacement").await;
    assert_eq!(w.secrets.list_env("aylul").await.unwrap().len(), 1);
    let pairs = w.secrets.resolve_env("aylul").await.unwrap();
    assert!(
        pairs[0].1 == "synthetic-replacement",
        "replacement resolves"
    );

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn remove_drops_the_row() {
    let w = world().await;
    set(&w, "a", "API_KEY", VALUE).await;
    w.secrets
        .remove_env("b".into(), "aylul", "API_KEY")
        .await
        .unwrap();
    assert!(w.secrets.list_env("aylul").await.unwrap().is_empty());
    w.secrets
        .remove_env("b".into(), "aylul", "API_KEY")
        .await
        .unwrap();

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn replay_same_value_replays() {
    let w = world().await;
    let first = w
        .secrets
        .set_env("a".into(), "aylul", "API_KEY", VALUE)
        .await
        .unwrap();
    set(&w, "b", "API_KEY", "synthetic-replacement").await;
    let replay = w
        .secrets
        .set_env("a".into(), "aylul", "API_KEY", VALUE)
        .await
        .unwrap();
    assert_eq!(first.updated_at, replay.updated_at);
    let pairs = w.secrets.resolve_env("aylul").await.unwrap();
    assert!(
        pairs[0].1 == "synthetic-replacement",
        "replacement resolves"
    );

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn replay_other_value_conflicts() {
    let w = world().await;
    set(&w, "a", "API_KEY", VALUE).await;
    let result = w
        .secrets
        .set_env("a".into(), "aylul", "API_KEY", "different")
        .await;
    assert!(matches!(
        result,
        Err(SecretsError::Storage(StorageError::CommandConflict))
    ));

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn moved_row_is_unreadable() {
    let w = world().await;
    set(&w, "a", "API_KEY", VALUE).await;
    sqlx::query("UPDATE secret_env SET name = 'MOVED'")
        .execute(w.storage.reader())
        .await
        .unwrap();
    assert!(matches!(w.secrets.resolve_env("aylul").await,
        Err(SecretsError::Unreadable(name)) if name == "MOVED"));

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn other_master_key_lists_but_cannot_resolve() {
    let w = world().await;
    set(&w, "a", "API_KEY", VALUE).await;
    let other = Secrets::new(w.storage.clone(), Some(SecretKey::from_bytes([8; 32])));
    assert_eq!(other.list_env("aylul").await.unwrap().len(), 1);
    assert!(matches!(other.resolve_env("aylul").await,
        Err(SecretsError::Unreadable(name)) if name == "API_KEY"));

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn lower_case_bad_and_reserved_names_are_refused() {
    let w = world().await;
    for name in [
        "api_key",
        "1A",
        "A-B",
        &"A".repeat(129),
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "CLAUDE_CONFIG_DIR",
    ] {
        assert!(matches!(
            w.secrets.set_env("a".into(), "aylul", name, VALUE).await,
            Err(SecretsError::NameInvalid(_) | SecretsError::NameReserved(_))
        ));
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn empty_nul_and_oversized_values_are_refused() {
    let w = world().await;
    for value in ["", "a\0b", &"x".repeat(32769)] {
        assert!(matches!(
            w.secrets
                .set_env("a".into(), "aylul", "API_KEY", value)
                .await,
            Err(SecretsError::ValueInvalid(_))
        ));
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn no_key_refuses_every_method() {
    let w = world().await;
    let service = Secrets::new(w.storage.clone(), None);
    assert!(matches!(
        service.list_env("p").await,
        Err(SecretsError::Unavailable)
    ));
    assert!(matches!(
        service.set_env("a".into(), "p", "A", VALUE).await,
        Err(SecretsError::Unavailable)
    ));
    assert!(matches!(
        service.remove_env("a".into(), "p", "A").await,
        Err(SecretsError::Unavailable)
    ));
    assert!(matches!(
        service.resolve_env("p").await,
        Err(SecretsError::Unavailable)
    ));

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn value_absent_from_db_wal_shm_and_command_record() {
    let w = world().await;
    set(&w, "a", "API_KEY", VALUE).await;
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT request_fingerprint, outcome_ref FROM command_record")
            .fetch_all(w.storage.reader())
            .await
            .unwrap();
    assert!(
        rows.iter()
            .all(|(a, b)| !a.contains(VALUE) && !b.contains(VALUE))
    );
    for file in ["test.db", "test.db-wal", "test.db-shm"] {
        if let Ok(bytes) = std::fs::read(w.tmp.path().join(file)) {
            assert!(!bytes.windows(VALUE.len()).any(|v| v == VALUE.as_bytes()));
        }
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn reserved_name_can_be_removed() {
    let w = world().await;
    // A stored row with a reserved name fails opening; removal must stay possible.
    assert!(
        w.secrets
            .remove_env("r".into(), "aylul", "ANTHROPIC_BASE_URL")
            .await
            .is_ok()
    );
    assert!(matches!(
        w.secrets.remove_env("r2".into(), "aylul", "bad").await,
        Err(SecretsError::NameInvalid(_))
    ));

    w.storage.close().await.unwrap();
}

async fn put_mcp(w: &World, command: &str, id: &str, name: &str) -> Result<(), SecretsError> {
    let write = w.secrets.mcp_value_write(command.into(), id, name, VALUE)?;
    w.storage
        .write_txn_as::<_, _, SecretsError>(move |conn| {
            Box::pin(async move {
                Secrets::put_in(conn, write).await?;
                Ok(())
            })
        })
        .await
}

#[tokio::test]
async fn mcp_names_follow_their_own_rule() {
    let w = world().await;
    let long = "b".repeat(128);
    for (i, name) in ["Authorization", "x-api_key-9", "a", long.as_str()]
        .into_iter()
        .enumerate()
    {
        put_mcp(&w, &format!("ok{i}"), "m1", name).await.unwrap();
    }
    let too_long = "b".repeat(129);
    for (i, name) in ["", "a b", "a.b", "é", too_long.as_str()]
        .into_iter()
        .enumerate()
    {
        assert!(matches!(
            put_mcp(&w, &format!("bad{i}"), "m1", name).await,
            Err(SecretsError::NameInvalid(_))
        ));
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn mcp_scope_refuses_reserved_names() {
    let w = world().await;
    assert_eq!(Secrets::mcp_scope("m1"), "mcp/m1");
    for name in [
        "ANTHROPIC_API_KEY",
        "CLAUDE_CONFIG_DIR",
        "ANTHROPIC_BASE_URL",
    ] {
        assert!(matches!(
            put_mcp(&w, "r", "m1", name).await,
            Err(SecretsError::NameReserved(_))
        ));
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn mcp_values_round_trip_under_their_scope() {
    let w = world().await;
    put_mcp(&w, "a", "m1", "Authorization").await.unwrap();
    put_mcp(&w, "b", "m2", "token").await.unwrap();
    let scope = Secrets::mcp_scope("m1");
    let values = w.secrets.resolve_env(&scope).await.unwrap();
    assert_eq!(values, [("Authorization".to_string(), VALUE.to_string())]);
    assert!(w.secrets.list_env("m1").await.unwrap().is_empty());
    let removal = w
        .secrets
        .mcp_value_removal("rm".into(), "m1", "Authorization")
        .unwrap();
    w.storage
        .write_txn_as::<_, _, SecretsError>(move |conn| {
            Box::pin(async move {
                Secrets::remove_in(conn, removal).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    assert!(w.secrets.list_env(&scope).await.unwrap().is_empty());
    let other = Secrets::mcp_scope("m2");
    assert_eq!(w.secrets.list_env(&other).await.unwrap().len(), 1);

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn mcp_value_absent_from_db_and_command_record() {
    let w = world().await;
    put_mcp(&w, "a", "m1", "Authorization").await.unwrap();
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT request_fingerprint, outcome_ref FROM command_record")
            .fetch_all(w.storage.reader())
            .await
            .unwrap();
    assert!(!rows.is_empty());
    assert!(
        rows.iter()
            .all(|(a, b)| !a.contains(VALUE) && !b.contains(VALUE))
    );
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT ciphertext FROM secret_env")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert!(!bytes.windows(VALUE.len()).any(|v| v == VALUE.as_bytes()));

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn mcp_write_fingerprint_is_keyed_and_value_sensitive() {
    let w = world().await;
    let a = w
        .secrets
        .mcp_value_write("one".into(), "server", "Token", "synthetic-a")
        .unwrap();
    let b = w
        .secrets
        .mcp_value_write("two".into(), "server", "Token", "synthetic-a")
        .unwrap();
    let c = w
        .secrets
        .mcp_value_write("three".into(), "server", "Token", "synthetic-b")
        .unwrap();
    let other = Secrets::new(w.storage.clone(), Some(SecretKey::from_bytes([9; 32])));
    let other_write = other
        .mcp_value_write("four".into(), "server", "Token", "synthetic-a")
        .unwrap();
    assert_eq!(a.value_mac(), b.value_mac());
    assert_ne!(a.value_mac(), c.value_mac());
    assert_ne!(a.value_mac(), other_write.value_mac());
    assert!(!a.value_mac().contains("synthetic-a"));
}
