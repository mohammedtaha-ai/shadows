use core_db::{Storage, StorageError};

#[tokio::test]
async fn close_releases_the_database_and_is_idempotent() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::open(&directory.path().join("close.sqlite3"))
        .await
        .unwrap();
    storage.close().await.unwrap();
    storage.close().await.unwrap();
    assert!(matches!(
        storage.write_txn(|_| Box::pin(async { Ok(()) })).await,
        Err(StorageError::Unavailable(_))
    ));
    let read = sqlx::query("SELECT 1")
        .execute(storage.reader())
        .await
        .map_err(StorageError::from);
    assert!(matches!(read, Err(StorageError::Unavailable(_))));
    directory
        .close()
        .expect("closed storage releases all database files");
}
