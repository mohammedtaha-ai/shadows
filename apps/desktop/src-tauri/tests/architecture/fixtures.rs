use std::collections::BTreeMap;

use super::{baseline, compare, inventory, scanner, Findings};

fn scan(file: &str, source: &str) -> Findings {
    scanner::scan(&BTreeMap::from([(file.into(), source.into())]))
}

fn catches(file: &str, source: &str, rule: &str) {
    let findings = scan(file, source);
    assert!(
        findings
            .iter()
            .any(|v| v.starts_with(&format!("{rule} :: "))),
        "{rule} must reject {source}: {findings:#?}"
    );
}

#[test]
fn forbidden_owner_edges_cover_import_alias_and_qualified_path() {
    catches(
        "navigation.rs",
        "use crate::auth::AuthController as Account; fn f(_: Account) {}",
        "owner_edge",
    );
    catches(
        "navigation.rs",
        "fn f(_: crate::auth::AuthController) {}",
        "owner_edge",
    );
}

#[test]
fn reexport_cannot_launder_an_owner_or_constructor() {
    let sources = BTreeMap::from([
        (
            "navigation.rs".into(),
            "pub use crate::auth::AuthController as Hidden;".into(),
        ),
        (
            "commands/mod.rs".into(),
            "use crate::navigation::Hidden as Alias; fn f() { Alias::new(); }".into(),
        ),
    ]);
    let found = scanner::scan(&sources);
    assert!(
        found
            .iter()
            .any(|s| s.starts_with("construction :: commands/mod.rs :: auth::AuthController::new")),
        "{found:#?}"
    );
    assert!(
        found
            .iter()
            .any(|s| s.starts_with("owner_edge :: commands/mod.rs :: auth::AuthController")),
        "{found:#?}"
    );
}

#[test]
fn private_submodule_and_store_cannot_cross_an_allowed_edge() {
    catches(
        "presence/mod.rs",
        "use crate::core_connection::config::read_json as read;",
        "private_access",
    );
    catches(
        "presence/mod.rs",
        "fn f() { crate::core_connection::config::read_json(); }",
        "private_access",
    );
    catches(
        "presence/mod.rs",
        "use crate::windows::companion::Store;",
        "private_access",
    );
}

#[test]
fn managed_state_covers_state_try_state_and_ufcs() {
    for source in [
        "fn f(_: tauri::State<'_, crate::presence::Presence>) {}",
        "fn f() { app.state::<crate::presence::Presence>(); }",
        "fn f() { app.try_state::<crate::presence::Presence>(); }",
        "use tauri::Manager as M; fn f() { M::state::<crate::presence::Presence>(&app); }",
    ] {
        catches("commands/mod.rs", source, "managed_state");
    }
    assert!(scan(
        "lib.rs",
        "fn f() { app.state::<u8>(); app.try_state::<u8>(); }"
    )
    .is_empty());
}

#[test]
fn construction_covers_alias_qualified_default_tuple_and_struct() {
    for source in [
        "use crate::presence::Presence as P; fn f() { P::new(); }",
        "fn f() { crate::presence::Presence::new(); }",
        "fn f() { crate::navigation::Navigation::default(); }",
        "fn f() { <crate::navigation::Navigation as Default>::default(); }",
        "fn f() { crate::sidecar::SidecarState(None); }",
        "fn f() { crate::presence::Presence {}; }",
        "fn f() { let make = crate::presence::Presence::new; make(); }",
    ] {
        catches("commands/mod.rs", source, "construction");
    }
    assert!(!scan("lib.rs", "fn f() { presence::Presence::new(); }")
        .iter()
        .any(|s| s.starts_with("construction")));
}

#[test]
fn core_http_covers_alias_qualified_and_macro_endpoint() {
    catches(
        "navigation.rs",
        "use reqwest::*; fn f() { get(url); }",
        "core_http",
    );
    catches(
        "navigation.rs",
        "use reqwest::Client as Http; fn f() { Http::new(); }",
        "core_http",
    );
    catches(
        "navigation.rs",
        "fn f() { reqwest::blocking::get(url); }",
        "core_http",
    );
    catches(
        "navigation.rs",
        "fn f() { format!(\"{base}/api/threads\"); }",
        "core_http",
    );
    assert!(scan(
        "presence/client.rs",
        "fn f() { reqwest::Client::new(); format!(\"/api/threads\"); }"
    )
    .is_empty());
    assert!(scan("auth/http.rs", "fn f() { reqwest::Client::new(); }").is_empty());
}

#[test]
fn importing_a_private_module_is_access_even_without_a_member_path() {
    let sources = BTreeMap::from([
        ("core_connection/mod.rs".into(), "mod config;".into()),
        (
            "presence/mod.rs".into(),
            "use crate::core_connection::config as hidden;".into(),
        ),
    ]);
    assert!(scanner::scan(&sources)
        .contains("private_access :: presence/mod.rs :: core_connection::config #1"));
}

#[test]
fn unknown_owner_fails_coverage() {
    catches("new_service/mod.rs", "pub struct Service;", "coverage");
}

#[test]
fn owner_cycle_fails_even_when_each_owner_has_a_public_entry() {
    let sources = BTreeMap::from([
        (
            "presence/mod.rs".into(),
            "use crate::core_connection::Connection;".into(),
        ),
        (
            "core_connection/mod.rs".into(),
            "use crate::presence::Presence;".into(),
        ),
    ]);
    assert!(scanner::scan(&sources)
        .iter()
        .any(|s| s.starts_with("owner_cycle :: ")));
}

#[test]
fn contract_and_map_coverage_use_owner_entries() {
    let sources = BTreeMap::from([("presence/mod.rs".into(), "struct Presence;".into())]);
    let absent = scanner::check_coverage(&sources, &Default::default(), "");
    assert!(absent
        .iter()
        .any(|s| s.contains("missing presence/contract.yaml")));
    assert!(absent.iter().any(|s| s.contains("missing map")));
    let contracts = ["presence/contract.yaml".into()].into();
    assert!(scanner::check_coverage(
        &sources,
        &contracts,
        "| `apps/desktop/src-tauri/src/presence/` |"
    )
    .is_empty());
}

#[test]
fn type_alias_and_foreign_glob_require_the_same_boundary() {
    catches(
        "commands/mod.rs",
        "type P = crate::presence::Presence; fn f() { P::new(); }",
        "construction",
    );
    catches(
        "commands/mod.rs",
        "use crate::presence::*;",
        "unresolved_import",
    );
}

#[test]
fn path_attribute_keeps_tests_with_their_declaring_owner() {
    let sources = BTreeMap::from([
        ("lib.rs".into(), "mod windows;".into()),
        ("windows/mod.rs".into(), "mod preferences;".into()),
        (
            "windows/preferences.rs".into(),
            "pub struct Store; #[path = \"preferences_tests.rs\"] mod tests;".into(),
        ),
        (
            "windows/preferences_tests.rs".into(),
            "use super::*; fn f() { Store::new(); }".into(),
        ),
    ]);
    let found = scanner::scan(&sources);
    assert!(
        found.contains(
            "construction :: windows/preferences_tests.rs :: windows::preferences::Store::new #1"
        ),
        "{found:#?}"
    );
    assert!(
        !found
            .iter()
            .any(|s| s.starts_with("owner_edge") || s.starts_with("owner_cycle")),
        "{found:#?}"
    );
}

#[test]
fn allowed_graph_and_ordinary_dtos_are_not_violations() {
    for (file, source) in [
        (
            "presence/mod.rs",
            "use crate::core_connection::Connection; use crate::navigation::Navigation;",
        ),
        ("navigation.rs", "fn f() { crate::windows::open_main(); }"),
        (
            "windows/mod.rs",
            "use crate::windows::preferences::DesktopPreferences;",
        ),
        (
            "auth/service.rs",
            "use crate::assistant::Request; use crate::native_login::browser::open;",
        ),
        (
            "presence/mod.rs",
            "struct View; impl View { fn new() -> Self { Self } } fn f() { View::new(); }",
        ),
    ] {
        assert!(
            scan(file, source).is_empty(),
            "allowed source: {file}: {:?}",
            scan(file, source)
        );
    }
}

#[test]
fn local_values_named_after_owners_are_not_dependencies() {
    assert!(scan(
        "navigation.rs",
        "fn f(presence: u8, auth: u8) -> u8 { presence + auth }"
    )
    .is_empty());
}

#[test]
fn exact_baseline_rejects_new_stale_and_duplicate_occurrences() {
    let first = scan("navigation.rs", "fn f() { app.state::<u8>(); }");
    assert!(compare(&first, &first).is_empty());
    assert!(compare(&first, &Findings::new())
        .iter()
        .all(|s| s.starts_with("NEW ")));
    assert!(compare(&Findings::new(), &first)
        .iter()
        .all(|s| s.starts_with("STALE ")));
    let twice = scan(
        "navigation.rs",
        "fn f() { app.state::<u8>(); app.state::<u8>(); }",
    );
    assert_eq!(compare(&twice, &first).len(), 1);
    assert_eq!(
        baseline(&first.iter().cloned().collect::<Vec<_>>().join("\n")),
        first
    );
}

#[test]
#[should_panic(expected = "duplicate baseline")]
fn duplicate_baseline_is_invalid() {
    baseline("rule :: file :: target\nrule :: file :: target");
}

#[test]
fn command_inventory_checks_each_surface_in_both_directions() {
    let lib = "fn f() { tauri::generate_handler![commands::one, commands::two]; }";
    let build = "fn main() { manifest.commands(&[\"one\", \"two\"]); }";
    let permissions = BTreeMap::from([(
        "permissions.toml".into(),
        "[[permission]]\nidentifier = 'all'\ncommands.allow = ['one', 'two']".into(),
    )]);
    assert!(inventory::check(lib, build, &permissions).is_empty());
    for (mutated_lib, mutated_build, mutated_permissions, target) in [
        (
            lib.replace(", commands::two", ""),
            build.to_owned(),
            permissions.clone(),
            "lib.rs :: missing two",
        ),
        (
            lib.to_owned(),
            build.replace(", \"two\"", ""),
            permissions.clone(),
            "build.rs :: missing two",
        ),
        (
            lib.to_owned(),
            build.to_owned(),
            BTreeMap::new(),
            "permissions/ :: missing one",
        ),
    ] {
        let found = inventory::check(&mutated_lib, &mutated_build, &mutated_permissions);
        assert!(
            found.contains(&format!("command_inventory :: {target}")),
            "{found:#?}"
        );
    }
    let extra = BTreeMap::from([(
        "extra.toml".into(),
        "[[permission]]\ncommands.deny = ['extra']".into(),
    )]);
    assert!(inventory::check(lib, build, &extra)
        .contains("command_inventory :: lib.rs :: missing extra"));
}
