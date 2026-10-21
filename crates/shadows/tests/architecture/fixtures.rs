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
        "/// | Projects | project ownership | crates/shadows-core/src/projects | crates/shadows-core/src/projects/contract.yaml |\npub struct AppCore;",
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
fn a_service_constructor_or_field_cannot_take_appcore() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/projects/mod.rs",
        "pub struct Projects { app: std::sync::Arc<crate::app::AppCore> }\n\
         impl Projects { pub fn new(app: std::sync::Arc<crate::app::AppCore>) -> Self { Self { app } } }",
    );
    let after = rejected(root, &before, "ownership");
    for symbol in ["Projects", "Projects::new"] {
        let reason = format!(":: {symbol} service depends on AppCore [");
        assert!(
            after["ownership"]
                .iter()
                .any(|record| record.contains(&reason)),
            "missing AppCore dependency in {symbol}: {:#?}",
            after["ownership"]
        );
    }
    put(
        root,
        "crates/shadows-core/src/projects/mod.rs",
        "pub struct Projects; impl Projects { pub fn new() -> Self { Self } }",
    );
    assert!(compare(&scanner::scan(root), &before).is_empty());
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
        "/// | Projects | project ownership | crates/core-projects/src | crates/core-projects/contract.yaml |\npub struct AppCore;",
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

#[test]
fn review_extracted_service_can_call_its_own_store() {
    let tree = fixture();
    let root = tree.path();
    put(root, "crates/core-widgets/contract.yaml", "name: widgets");
    put(
        root,
        "crates/core-widgets/src/lib.rs",
        "mod store; fn read() { crate::store::get(); }",
    );
    put(root, "crates/core-widgets/src/store.rs", "fn get() {}");
    let before = scanner::scan(root);
    assert!(
        !before.get("ownership").is_some_and(|entries| entries
            .iter()
            .any(|entry| entry.contains("cross-owner store call"))),
        "own extracted store is safe: {before:#?}"
    );
    put(
        root,
        "crates/core-widgets/src/lib.rs",
        "fn read() { core_projects::store::get(); }",
    );
    rejected(root, &before, "ownership");
}

#[test]
fn review_each_sql_statement_requires_classification() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/projects/query.rs",
        "fn read() { sqlx::raw_sql(\"BEGIN IMMEDIATE; SELECT 1; COMMIT;\"); let _ = \"rollback failed; the connection is poisoned\"; }",
    );
    assert!(
        compare(&scanner::scan(root), &before).is_empty(),
        "transaction statements are safe and prose is not SQL"
    );
    for sql in [
        "SELECT 1; PRAGMA table_info(project)",
        "SELECT * FROM project; ATTACH DATABASE 'other.db' AS other",
    ] {
        put(
            root,
            "crates/shadows-core/src/projects/query.rs",
            &format!("fn read() {{ sqlx::raw_sql({sql:?}); }}"),
        );
        rejected(root, &before, "ownership");
    }
}

#[test]
fn review_core_root_files_cannot_construct_services() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/rogue.rs",
        "fn build() { crate::projects::Projects::new(); }",
    );
    rejected(root, &before, "ownership");
}

#[test]
fn review_quoted_sql_identifiers_are_not_keywords() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    for sql in [
        "SELECT \"from\", \"join\", 'FROM planning_thread' FROM \"project\"",
        "SELECT `from`, [join] FROM project",
        "SELECT \"a\"\"from\" FROM project",
    ] {
        put(
            root,
            "crates/shadows-core/src/projects/query.rs",
            &format!("fn read() {{ sqlx::query({sql:?}); }}"),
        );
        assert!(
            compare(&scanner::scan(root), &before).is_empty(),
            "quoted identifiers are not SQL clauses: {sql}"
        );
    }
    put(
        root,
        "crates/shadows-core/src/projects/query.rs",
        &format!(
            "fn read() {{ sqlx::query({:?}); }}",
            "SELECT * FROM \"planning_thread\""
        ),
    );
    let after = rejected(root, &before, "ownership");
    assert!(
        after["ownership"]
            .iter()
            .any(|entry| entry.contains("SQL table planning_thread belongs to threads")),
        "quoted table retains its owner: {after:#?}"
    );
}

#[test]
fn review_extracted_storage_methods_keep_their_type_owner() {
    let tree = fixture();
    let root = tree.path();
    for owner in ["widgets", "other"] {
        put(
            root,
            &format!("crates/core-{owner}/contract.yaml"),
            &format!("name: {owner}"),
        );
        put(
            root,
            &format!("crates/core-{owner}/src/lib.rs"),
            "pub struct Storage; impl Storage { pub fn read(&self) {} }",
        );
    }
    let before = scanner::scan(root);
    for call in ["storage.read()", "Foreign::read(storage)"] {
        put(
            root,
            "crates/core-widgets/src/query.rs",
            &format!(
                "use core_other::Storage as Foreign; fn query(storage: &Foreign) {{ {call}; }}"
            ),
        );
        rejected(root, &before, "ownership");
    }
}

#[test]
fn review_dynamic_query_builder_fragments_require_review() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    for source in [
        "fn read(table: &str) { let mut query = sqlx::QueryBuilder::new(\"SELECT 1\"); query.push(table); }",
        "use sqlx::QueryBuilder as Builder; fn read(table: &str) { let mut query: Builder<sqlx::Sqlite> = Builder::new(\"SELECT 1\"); query.push(table); }",
        "fn read(table: &str) { sqlx::QueryBuilder::<sqlx::Sqlite>::new(\"SELECT 1\").push(table); }",
        "fn read(table: &str) { sqlx::QueryBuilder::<sqlx::Sqlite>::new(\"SELECT 1\").push_bind(1).push(table); }",
    ] {
        put(root, "crates/shadows-core/src/projects/query.rs", source);
        rejected(root, &before, "ownership");
    }
    put(
        root,
        "crates/shadows-core/src/projects/query.rs",
        "fn read(value: &str) { let mut query = sqlx::QueryBuilder::new(\"SELECT 1\"); query.push_bind(value); let mut values = Vec::new(); values.push(value); }",
    );
    assert!(
        compare(&scanner::scan(root), &before).is_empty(),
        "bound values and unrelated push calls are safe"
    );
}

#[test]
fn appcore_map_rejects_missing_or_misregistered_owners() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    for row in [
        "",
        "| Projects | project ownership | crates/shadows-core/src/projects/contract.yaml |",
        "| Projects | project ownership | crates/shadows-core/src/db | crates/shadows-core/src/projects/contract.yaml |",
        "| Projects | project ownership | crates/shadows-core/src/projects | crates/shadows-core/src/db/contract.yaml |",
        "| Missing | project ownership | crates/shadows-core/src/projects | crates/shadows-core/src/projects/contract.yaml |",
        "| Projects | reads and writes projects | crates/shadows-core/src/projects | crates/shadows-core/src/projects/contract.yaml |",
        "| Projects | | crates/shadows-core/src/projects | crates/shadows-core/src/projects/contract.yaml |",
    ] {
        put(
            root,
            "crates/shadows-core/src/app.rs",
            &format!("/// {row}\npub struct AppCore;"),
        );
        rejected(root, &before, "registration");
    }
}

#[test]
fn appcore_map_rejects_duplicates_and_nonexistent_services() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    let row = "/// | Projects | project ownership | crates/shadows-core/src/projects | crates/shadows-core/src/projects/contract.yaml |\n";
    put(
        root,
        "crates/shadows-core/src/app.rs",
        &format!("{row}{row}pub struct AppCore;"),
    );
    rejected(root, &before, "registration");
    put(
        root,
        "crates/shadows-core/src/app.rs",
        &format!(
            "{row}/// | Ghost | absent owner | crates/core-ghost/src | crates/core-ghost/contract.yaml |\npub struct AppCore;"
        ),
    );
    rejected(root, &before, "registration");
}

#[test]
fn appcore_map_accepts_internal_owner_in_block_documentation() {
    let tree = fixture();
    let root = tree.path();
    let before = scanner::scan(root);
    put(
        root,
        "crates/shadows-core/src/app.rs",
        "/** Internal owner; no accessor is promised.\n| Projects | project ownership | crates/shadows-core/src/projects | crates/shadows-core/src/projects/contract.yaml |\n*/\npub struct AppCore;",
    );
    assert!(compare(&scanner::scan(root), &before).is_empty());
}
