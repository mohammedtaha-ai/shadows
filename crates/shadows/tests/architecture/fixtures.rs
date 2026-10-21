use std::path::Path;

use super::{Findings, compare, scanner};

fn put(root: &Path, path: &str, source: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, source).unwrap();
}

fn fixture() -> tempfile::TempDir {
    let tree = tempfile::tempdir().unwrap();
    put(
        tree.path(),
        "crates/shadows-core/src/app.rs",
        "/// | Projects | project ownership | crates/shadows-core/src/projects/contract.yaml |\npub struct AppCore;",
    );
    put(
        tree.path(),
        "docs/codebase/README.md",
        "| `crates/shadows-core/src/db/` | database | `crates/shadows-core/src/db/mod.rs` |\n",
    );
    put(
        tree.path(),
        "crates/shadows-core/src/projects/contract.yaml",
        "name: projects\n",
    );
    put(
        tree.path(),
        "crates/shadows-core/src/projects/mod.rs",
        "pub struct Projects; impl Projects { pub fn new() -> Self { Self } }",
    );
    put(
        tree.path(),
        "crates/shadows-core/src/db/mod.rs",
        "pub struct Storage; impl Storage { pub fn reader(&self) {} }",
    );
    tree
}

fn rejected(tree: &Path, before: &Findings, category: &str) -> Findings {
    let after = scanner::scan(tree);
    let errors = compare(&after, before);
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with(&format!("NEW {category}:"))),
        "expected a new {category} violation, got {errors:#?}"
    );
    after
}

#[test]
fn fixtures_reject_each_rule_and_stale_records() {
    let tree = fixture();
    let root = tree.path();
    let clean = scanner::scan(root);
    assert!(
        clean.values().all(|entries| entries.is_empty()),
        "clean fixture: {clean:#?}"
    );
    put(
        root,
        "crates/shadows-core/src/projects/extra.rs",
        "pub(crate) fn leak() {}\n// used to be elsewhere\n",
    );
    let after = rejected(root, &clean, "public_functions");
    rejected(root, &clean, "comments");
    put(root, "crates/shadows-core/src/projects/extra.rs", "");
    assert!(
        compare(&scanner::scan(root), &after)
            .iter()
            .any(|error| error.starts_with("STALE ")),
        "removed exceptions must fail until their baseline is removed"
    );
    put(
        root,
        "crates/shadows-core/src/new_service/mod.rs",
        "pub struct NewService;",
    );
    rejected(root, &clean, "registration");
    put(
        root,
        "crates/shadows-core/src/projects/sql.rs",
        "fn read() { sqlx::query(\"SELECT * FROM planning_thread\"); }",
    );
    rejected(root, &clean, "ownership");
}

#[test]
fn aliases_do_not_hide_storage_construction_or_appcore_dependencies() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/threads/contract.yaml",
        "name: threads",
    );
    put(
        root,
        "crates/shadows-core/src/threads/store.rs",
        "use crate::db::Storage as Database; impl Database { pub fn get_thread(&self) {} }",
    );
    put(
        root,
        "crates/shadows-core/src/projects/alias.rs",
        "use crate::db::Storage as Db; use crate::app::AppCore as Application; fn bypass(db: &Db, app: &Application) { db.get_thread(); }\nuse crate::projects::Projects as Owner; fn construct() { Owner::new(); }",
    );
    let after = rejected(root, &before, "ownership");
    let text = after["ownership"]
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    for marker in ["Storage::get_thread", "AppCore", "construct"] {
        assert!(
            text.contains(marker),
            "missing alias detection for {marker}: {text}"
        );
    }
}

#[test]
fn moved_service_is_discovered_and_still_checked_for_sql() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    std::fs::create_dir_all(root.join("crates/core-projects")).unwrap();
    std::fs::rename(
        root.join("crates/shadows-core/src/projects"),
        root.join("crates/core-projects/src"),
    )
    .unwrap();
    std::fs::rename(
        root.join("crates/core-projects/src/mod.rs"),
        root.join("crates/core-projects/src/lib.rs"),
    )
    .unwrap();
    std::fs::rename(
        root.join("crates/core-projects/src/contract.yaml"),
        root.join("crates/core-projects/contract.yaml"),
    )
    .unwrap();
    rejected(root, &before, "registration");
    put(
        root,
        "crates/shadows-core/src/app.rs",
        "/// | Projects | project ownership | crates/core-projects/contract.yaml |\npub struct AppCore;",
    );
    assert!(
        compare(&scanner::scan(root), &before).is_empty(),
        "a fully registered move retains the same clean boundary"
    );
    put(
        root,
        "crates/core-projects/src/query.rs",
        "pub fn leak() {} fn query() { sqlx::query(\"SELECT * FROM planning_thread\"); }",
    );
    rejected(root, &before, "public_functions");
    rejected(root, &before, "ownership");
}

#[test]
fn comment_tokens_ignore_strings_and_detect_history_and_block_comments() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/projects/comment.rs",
        "const TEXT: &str = r#\"// § used to Task 42 abcdef1\"#; /* ordinary rationale */",
    );
    assert!(
        compare(&scanner::scan(root), &before).is_empty(),
        "strings are not comments"
    );
    for marker in [
        "§",
        "used to",
        "no longer",
        "Milestone",
        "Task 42",
        "PROBE.md",
        "abcdef1",
    ] {
        put(
            root,
            "crates/shadows-core/src/projects/comment.rs",
            &format!("/* rationale\n{marker}\n*/"),
        );
        rejected(root, &before, "comments");
    }
}

#[test]
fn repeated_occurrences_do_not_collapse_into_one_exception() {
    let tree = fixture();
    let root = tree.path();
    let file = "crates/shadows-core/src/projects/sql.rs";
    put(
        root,
        file,
        "fn read() { sqlx::query(\"SELECT * FROM planning_thread\"); }",
    );
    let before = scanner::scan(root);
    put(
        root,
        file,
        "fn read() { sqlx::query(\"SELECT * FROM planning_thread\"); sqlx::query(\"SELECT * FROM planning_thread\"); }",
    );
    rejected(root, &before, "ownership");
}

#[test]
fn dynamic_and_unknown_sql_require_review_and_adapters_cannot_reach_live_state() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/projects/sql.rs",
        "fn read(sql: &str) { sqlx::query(sql); sqlx::query(\"SELECT * FROM mysterious\"); }",
    );
    let after = rejected(root, &before, "ownership");
    let entries = after["ownership"]
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        entries.contains("review"),
        "unclassified SQL needs explicit review"
    );
    put(
        root,
        "crates/shadows-http/src/lib.rs",
        "use shadows_core::harness::Sessions as Live; fn access(s: &Live) { s.open(); }",
    );
    rejected(root, &after, "ownership");
}

#[test]
fn sql_aliases_macros_and_builders_require_review_for_dynamic_queries() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    for expression in [
        "use sqlx::query as execute_sql; fn read(sql: &str) { execute_sql(sql); }",
        "fn read(sql: &str) { sqlx::query!(sql); }",
        "fn read(sql: &str) { sqlx::QueryBuilder::new(sql); }",
        "fn read(sql: &str) { sqlx::QueryBuilder::<sqlx::Sqlite>::new(sql); }",
        "fn read() { sqlx::query(\"-- comment\\nSELECT * FROM planning_thread\"); }",
        "fn read() { sqlx::query(\"unclassified statement\"); }",
    ] {
        put(
            root,
            "crates/shadows-core/src/projects/query.rs",
            expression,
        );
        rejected(root, &before, "ownership");
    }
}

#[test]
fn known_unrelated_receiver_is_not_resolved_by_a_method_name_match() {
    let tree = fixture();
    let root = tree.path();
    put(
        root,
        "crates/shadows-core/src/projects/store.rs",
        "use crate::db::Storage; impl Storage { pub fn get_thread(&self) {} }",
    );
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/projects/value.rs",
        "struct Other; impl Other { fn get_thread(&self) {} } fn read(other: &Other) { other.get_thread(); }",
    );
    assert!(
        compare(&scanner::scan(root), &before).is_empty(),
        "a known unrelated type must not become a Storage call"
    );
    put(
        root,
        "crates/shadows-core/src/projects/generic.rs",
        "fn read<T>(value: &T) { value.get_thread(); }",
    );
    let after = rejected(root, &before, "ownership");
    assert!(
        after["ownership"]
            .iter()
            .any(|entry| entry.contains("review")),
        "an unresolved generic receiver needs review"
    );
}

#[test]
fn imported_store_functions_and_aliased_service_reexports_are_checked() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/lib.rs",
        "mod projects; pub use projects::Projects;",
    );
    put(
        root,
        "crates/shadows-core/src/projects/alias.rs",
        "use crate::Projects as Owner; use crate::threads::store::get as load; fn bypass() { Owner::new(); load(); }",
    );
    let after = rejected(root, &before, "ownership");
    let text = after["ownership"]
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.contains("construct shadows_core::projects::Projects"),
        "reexport alias must resolve: {text}"
    );
    assert!(
        text.contains("cross-owner store call"),
        "store function import must resolve: {text}"
    );
}

#[test]
fn type_aliases_and_nested_sql_lists_cannot_hide_cross_owner_access() {
    let tree = fixture();
    let root = tree.path();
    put(
        root,
        "crates/shadows-core/src/threads/store.rs",
        "use crate::db::Storage; impl Storage { pub fn get_thread(&self) {} }",
    );
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/projects/alias.rs",
        "use crate::db::Storage; type Database = Storage; fn read(db: &Database) { db.get_thread(); }",
    );
    let after = rejected(root, &before, "ownership");
    assert!(
        after["ownership"]
            .iter()
            .any(|line| line.contains("Storage::get_thread")),
        "a Storage type alias must resolve"
    );
    for sql in [
        "SELECT * FROM project, planning_thread",
        "SELECT * FROM project, (SELECT 1 FROM project) nested, planning_thread",
        "SELECT * FROM project JOIN project other ON other.id = project.id, planning_thread",
    ] {
        put(
            root,
            "crates/shadows-core/src/projects/query.rs",
            &format!("fn read() {{ sqlx::query({sql:?}); }}"),
        );
        let found = rejected(root, &after, "ownership");
        assert!(
            found["ownership"]
                .iter()
                .any(|line| line.contains("SQL table planning_thread")),
            "nested SQL must retain table ownership: {found:#?}"
        );
    }
}

#[test]
fn service_struct_literals_and_adapter_database_helpers_are_rejected() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/projects/construction.rs",
        "use crate::projects::Projects as Owner; fn build() { let _ = Owner {}; }",
    );
    rejected(root, &before, "ownership");
    put(
        root,
        "crates/shadows-http/src/lib.rs",
        "use shadows_core::db::classify as inspect; fn access() { inspect(); }",
    );
    let after = rejected(root, &before, "ownership");
    assert!(
        after["ownership"]
            .iter()
            .any(|entry| entry.contains("adapter") && entry.contains("classify")),
        "an adapter cannot reach a database helper"
    );
}

#[test]
fn transport_verbs_and_public_dtos_are_not_store_or_sql_access() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/lib.rs",
        "mod projects; pub use projects::store::ProjectView;",
    );
    put(
        root,
        "crates/shadows-core/src/projects/store.rs",
        "pub struct ProjectView;",
    );
    put(
        root,
        "crates/shadows-http/src/lib.rs",
        "use shadows_core::ProjectView; fn response(value: ProjectView) { let _ = (\"DELETE\", \"INSERT\"); }",
    );
    assert!(
        compare(&scanner::scan(root), &before).is_empty(),
        "transport verbs and exported model types are safe"
    );
}
