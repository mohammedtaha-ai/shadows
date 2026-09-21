mod sqlite;

pub use sqlite::{Storage, StorageError};

/// Test-only access to a private capability. Not compiled into the library for
/// consumers, and not a public API.
#[doc(hidden)]
pub mod test_support {
    use sqlx::SqliteConnection;

    use super::StorageError;
    use crate::events::DurableEvent;

    pub async fn append_event_for_test(
        conn: &mut SqliteConnection,
        event: &DurableEvent,
        now: &str,
    ) -> Result<i64, StorageError> {
        super::sqlite::events::append_event(conn, event, now).await
    }
}
