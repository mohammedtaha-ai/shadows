//! The project capability's durable contract: identity creation and the
//! external-command idempotency machinery it introduces, reused by every
//! later mutating command. Separate from `tests/storage_contract.rs`, whose
//! one job is the connection and transaction contracts.

use shadows_core::command::{CommandContext, fingerprint};
use shadows_core::project::ProjectDirectory;
use shadows_core::storage::Storage;

/// Any directory that exists: these tests are about identity and
/// idempotency, not about where a turn runs.
fn dir() -> ProjectDirectory {
    ProjectDirectory::resolve(&std::env::temp_dir()).unwrap()
}

fn ctx(command_id: &str, params: &serde_json::Value) -> CommandContext {
    ctx_kind(command_id, "project.create", params)
}

fn ctx_kind(command_id: &str, kind: &str, params: &serde_json::Value) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: command_id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, params),
    }
}

/// Spec section 5.2 and cross-cutting rule 6: an external mutation writes its
/// CommandRecord in the same transaction. Replaying the same command id with
/// the same request returns the stored outcome and creates nothing new.
#[tokio::test]
async fn replaying_an_identical_command_returns_the_stored_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });

    let first = storage
        .create_project(
            &ctx("cmd-1", &params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let second = storage
        .create_project(
            &ctx("cmd-1", &params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();

    assert_eq!(first.id, second.id, "replay must return the same entity");

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(projects, 1, "replay must not create a second project");

    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(events, 1, "replay must not append a second event");
}

/// Replay requires fingerprint equality. The same command id with a different
/// request is CommandConflict and mutates nothing. This is the arm a future
/// contributor most wants to weaken; it stays refused.
#[tokio::test]
async fn the_same_command_id_with_a_different_request_is_a_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let first_params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    storage
        .create_project(
            &ctx("cmd-1", &first_params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();

    let other_params = serde_json::json!({ "slug": "other", "name": "Other" });
    let err = storage
        .create_project(
            &ctx("cmd-1", &other_params),
            "other",
            "Other",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .expect_err("a reused command id with a different request must be refused");
    assert!(matches!(
        err,
        shadows_core::storage::StorageError::CommandConflict
    ));

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(projects, 1, "a conflict must mutate nothing");
}

/// Key order carries no meaning, so reordering JSON keys must not turn a
/// replay into a conflict. A different command kind must not collide.
#[test]
fn the_fingerprint_ignores_key_order_but_not_command_kind() {
    let a = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let b = serde_json::json!({ "name": "Demo", "slug": "demo" });
    assert_eq!(
        fingerprint("project.create", &a),
        fingerprint("project.create", &b)
    );
    assert_ne!(
        fingerprint("project.create", &a),
        fingerprint("thread.create", &a)
    );
}

/// Regression for the canonicalisation collision: an unescaped key can
/// impersonate the separator structure, so `{"a":1,"bc":2}` and
/// `{"a:1,bc":2}` must not hash to the same fingerprint.
#[test]
fn the_fingerprint_does_not_let_a_key_impersonate_the_separator_structure() {
    let a = serde_json::json!({ "a": 1, "bc": 2 });
    let b = serde_json::json!({ "a:1,bc": 2 });
    assert_ne!(fingerprint("k", &a), fingerprint("k", &b));
}

/// A single value change, all keys and command kind held constant, must
/// change the fingerprint. Nothing else in the suite pins this down.
#[test]
fn the_fingerprint_changes_when_a_value_changes() {
    assert_ne!(
        fingerprint("k", &serde_json::json!({ "a": 1 })),
        fingerprint("k", &serde_json::json!({ "a": 2 }))
    );
}

/// Spec section 6.19 names three things storage compares before it calls a
/// submission a replay: command kind, **schema version**, and fingerprint. A
/// bumped schema version means the request was normalised by different rules,
/// so byte-equal params no longer prove the requests are the same one — the
/// only safe answer is CommandConflict. Without this, a version bump silently
/// replays an outcome computed under the old normalisation.
#[tokio::test]
async fn the_same_command_id_under_a_different_schema_version_is_a_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });

    storage
        .create_project(
            &ctx("cmd-1", &params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();

    let mut bumped = ctx("cmd-1", &params);
    bumped.command_schema_ver = 2;
    assert_eq!(
        bumped.request_fingerprint,
        ctx("cmd-1", &params).request_fingerprint,
        "the version is the only difference; the fingerprint must still match"
    );

    let err = storage
        .create_project(
            &bumped,
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .expect_err("a reused command id under a new schema version must be refused");
    assert!(matches!(
        err,
        shadows_core::storage::StorageError::CommandConflict
    ));

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(projects, 1, "a conflict must not create a second project");

    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(events, 1, "a conflict must not append a second event");
}

/// CLAUDE.md: ordering is explicit. Projects are listed in the order they were
/// created, which `created_at` text cannot give — RFC 3339 with trimmed zeros
/// does not sort in time order within a second. Here the stored timestamps are
/// rewritten to sort backwards, and the listing must not follow them.
#[tokio::test]
async fn projects_are_listed_in_creation_order_whatever_their_timestamp_text() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let mut created = Vec::new();
    for (i, stamp) in ["2026-01-01T00:00:00.5Z", "2026-01-01T00:00:00.45Z"]
        .into_iter()
        .enumerate()
    {
        let slug = format!("p{i}");
        let params = serde_json::json!({ "slug": slug });
        let project = storage
            .create_project(
                &ctx(&slug, &params),
                &slug,
                &slug,
                &dir(),
                &shadows_agent::policy::default_modes(),
            )
            .await
            .unwrap();
        sqlx::query("UPDATE project SET created_at = ? WHERE id = ?")
            .bind(stamp)
            .bind(project.id.as_str())
            .execute(storage.reader())
            .await
            .unwrap();
        created.push(project.id);
    }
    let listed: Vec<_> = storage
        .list_projects()
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(listed, created);
}
