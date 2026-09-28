mod sqlite;

pub use sqlite::{
    InstructionsVersion, ReconcileReport, StopKind, Storage, StorageError, StoredEvent,
};
// Store helpers a plan or turn write shares inside its one transaction (spec §14.6).
pub(crate) use sqlite::{
    append_entry_in, append_event, classify, insert_thread, now, record_command, remember_settings,
};

/// Test-only access to a private capability. Compiled in only when the
/// `test-support` feature is enabled — enabled automatically for `cargo test`
/// via the self dev-dependency in `Cargo.toml`, and off in an ordinary
/// `cargo build`/`cargo run`, so this capability does not ship. Even then it
/// is not a public API: `#[doc(hidden)]` only suppresses documentation, the
/// `cfg` is what keeps it out of ordinary builds.
#[cfg(feature = "test-support")]
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
