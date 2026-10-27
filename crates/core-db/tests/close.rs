use core_db::{Storage, StorageError};
use std::future::Future;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

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

#[tokio::test]
async fn close_refuses_writes_while_waiting_for_a_borrowed_reader() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&directory.path().join("close.sqlite3"))
            .await
            .unwrap(),
    );
    let reader = storage.reader().acquire().await.unwrap();
    let closing = tokio::spawn({
        let storage = storage.clone();
        async move { storage.close().await }
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !storage.reader().is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("close must start draining readers");
    assert!(
        !closing.is_finished(),
        "close must await the borrowed reader"
    );

    let write = tokio::time::timeout(
        Duration::from_secs(2),
        storage.write_txn(|_| Box::pin(async { Ok(()) })),
    )
    .await;
    drop(reader);
    closing.await.unwrap().unwrap();
    directory.close().unwrap();
    assert!(
        matches!(write, Ok(Err(StorageError::Unavailable(_)))),
        "a reader holder must be able to finish a write attempt during close: {write:?}"
    );
}

#[tokio::test]
async fn close_waits_for_an_in_flight_transaction_before_closing_readers() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&directory.path().join("close.sqlite3"))
            .await
            .unwrap(),
    );
    let (started, entered) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let writing = tokio::spawn({
        let storage = storage.clone();
        async move {
            storage
                .write_txn(|_| {
                    Box::pin(async move {
                        started.send(()).unwrap();
                        released.await.unwrap();
                        Ok(())
                    })
                })
                .await
        }
    });
    entered.await.unwrap();
    let closing = storage.close();
    tokio::pin!(closing);
    assert!(
        std::future::poll_fn(|cx| Poll::Ready(closing.as_mut().poll(cx).is_pending())).await,
        "close must await the transaction that holds the writer"
    );
    assert!(!storage.reader().is_closed());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        writing.await.unwrap().unwrap();
        closing.await.unwrap();
    })
    .await
    .expect("the completed write must release close");
    directory.close().unwrap();
}
