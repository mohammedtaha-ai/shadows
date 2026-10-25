use std::collections::BTreeMap;

use super::{baseline, compare, inventory, scanner, Findings};

#[test]
fn preferences_owner_keeps_construction_and_private_internals_out_of_commands() {
    assert_eq!(
        scanner::owner("desktop_preferences"),
        Some("DesktopPreferences")
    );
    catches(
        "commands/preferences.rs",
        "use crate::desktop_preferences::DesktopPreferences as P; fn f() { P::new(); }",
        "construction",
    );
    catches(
        "commands/preferences.rs",
        "fn f() { crate::desktop_preferences::login_item::PluginLoginItem(app); }",
        "construction",
    );
    catches(
        "commands/preferences.rs",
        "fn f(_: crate::desktop_preferences::store::Store) {}",
        "private_access",
    );
}

#[test]
fn preference_store_accessor_is_private_even_for_commands() {
    let found = scanner::scan(&BTreeMap::from([
        (
            "app.rs".into(),
            "struct DesktopApp; impl DesktopApp {
                fn preferences(&self) -> &crate::desktop_preferences::store::Store { todo!() }
            }"
            .into(),
        ),
        (
            "commands/preferences.rs".into(),
            "fn f(desktop: &crate::app::DesktopApp) { desktop.preferences().read(); }".into(),
        ),
    ]));
    assert!(
        found.contains(
            "private_access :: commands/preferences.rs :: desktop_preferences::store::Store #1"
        ),
        "commands must not obtain a private store through composition: {found:#?}"
    );
}

#[test]
fn reexported_login_adapter_construction_is_still_ratcheted() {
    let sources = BTreeMap::from([
        (
            "desktop_preferences/mod.rs".into(),
            "mod login_item; pub(crate) use login_item::PluginLoginItem;".into(),
        ),
        (
            "desktop_preferences/login_item.rs".into(),
            "pub(crate) struct PluginLoginItem(pub(crate) tauri::AppHandle);".into(),
        ),
        (
            "commands/preferences.rs".into(),
            "fn f() { crate::desktop_preferences::PluginLoginItem(app); }".into(),
        ),
    ]);
    let found = scanner::scan(&sources);
    assert!(
        found.contains("construction :: commands/preferences.rs :: desktop_preferences::login_item::PluginLoginItem #1"),
        "the owner entry reexport must retain the adapter construction check: {found:#?}"
    );
}

fn scan(file: &str, source: &str) -> Findings {
    scanner::scan(&BTreeMap::from([(file.into(), source.into())]))
}

#[test]
fn desktop_composition_names_its_values_without_exempting_nested_owners() {
    let source = "struct DesktopApp { store: crate::windows::companion::Store }
        fn start(app: tauri::AppHandle) {
            crate::windows::companion::Store::default();
            app.state::<DesktopApp>();
        }";
    assert!(scan("app.rs", source).is_empty());
    catches("windows/mod.rs", source, "construction");
    catches("windows/mod.rs", source, "managed_state");
    catches("windows/mod.rs", source, "private_access");
    catches(
        "app.rs",
        "mod presence { fn f(app: tauri::AppHandle) { app.state::<u8>(); } }",
        "managed_state",
    );
    catches(
        "app.rs",
        "mod presence { fn f() { crate::windows::companion::Store::default(); } }",
        "construction",
    );
    catches(
        "app.rs",
        "fn f() { crate::presence::watcher::PresenceWatcher::new(); }",
        "private_access",
    );
}

#[test]
fn window_root_lookups_still_require_exact_managed_state_exceptions() {
    for file in ["windows/mod.rs", "windows/companion/mod.rs"] {
        let findings = scan(
            file,
            "fn f(app: tauri::AppHandle) {
                app.state::<std::sync::Arc<crate::app::DesktopApp>>();
            }",
        );
        assert!(
            findings.iter().any(|v| v.starts_with("owner_edge :: ")),
            "{file}: {findings:#?}"
        );
        assert!(findings.iter().any(|v| v.starts_with("managed_state :: ")));
    }
}

#[test]
fn root_accessors_preserve_owner_edges_private_access_and_cycles() {
    let root = "use crate::{presence, desktop_preferences};
        struct DesktopApp;
        impl DesktopApp {
            fn renamed_presence(&self) -> &std::sync::Arc<presence::Presence> { todo!() }
            fn preferences(&self) -> &desktop_preferences::store::Store { todo!() }
        }";
    let found = scanner::scan(&BTreeMap::from([
        ("app.rs".into(), root.into()),
        ("windows/companion/input.rs".into(), "fn f(desktop: &crate::app::DesktopApp) { let owner = desktop.renamed_presence(); owner.context(); }".into()),
        ("windows/mod.rs".into(), "fn f(desktop: &crate::app::DesktopApp) { desktop.preferences().close_to_tray(); }".into()),
        ("presence/mod.rs".into(), "use crate::windows::companion::Store;".into()),
    ]));
    for entry in [
        "owner_edge :: windows/companion/input.rs :: presence::Presence #1",
        "private_access :: windows/mod.rs :: desktop_preferences::store::Store #1",
        "owner_cycle :: windows/companion/input.rs :: CompanionWindow -> Companion (cycle edge)",
    ] {
        assert!(found.contains(entry), "missing {entry}: {found:#?}");
    }
    let commands = scanner::scan(&BTreeMap::from([
        ("app.rs".into(), root.replace("desktop_preferences::store::Store", "desktop_preferences::DesktopPreferences")),
        ("commands/mod.rs".into(), "fn f(desktop: &crate::app::DesktopApp) { desktop.renamed_presence(); desktop.preferences(); }".into()),
    ]));
    assert!(
        commands.is_empty(),
        "commands may use the root: {commands:#?}"
    );
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
        ("lib.rs".into(), "mod desktop_preferences;".into()),
        (
            "desktop_preferences/mod.rs".into(),
            "pub struct DesktopPreferences; #[path = \"preferences_tests.rs\"] mod tests;".into(),
        ),
        (
            "desktop_preferences/preferences_tests.rs".into(),
            "use super::*; fn f() { DesktopPreferences::new(); }".into(),
        ),
    ]);
    let found = scanner::scan(&sources);
    assert!(
        found.contains(
            "construction :: desktop_preferences/preferences_tests.rs :: desktop_preferences::DesktopPreferences::new #1"
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
            "use crate::desktop_preferences::PreferencesView;",
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
    let lib =
        "fn f() { app.invoke_handler(tauri::generate_handler![commands::one, commands::two]); }";
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

#[test]
fn inline_owner_in_lib_is_not_composition() {
    catches(
        "lib.rs",
        "mod navigation { fn f() { app.state::<u8>(); } }",
        "managed_state",
    );
    catches(
        "lib.rs",
        "mod navigation { fn f() { crate::presence::Presence::new(); } }",
        "construction",
    );
}

#[test]
fn self_constructor_is_not_an_ordinary_dto() {
    catches(
        "presence/mod.rs",
        "struct Presence; impl Presence { fn duplicate() { Self::new(); } }",
        "construction",
    );
}

#[test]
fn reexport_through_a_private_alias_keeps_the_original_owner() {
    let found = scanner::scan(&BTreeMap::from([
        (
            "navigation.rs".into(),
            "use crate::presence::Presence as P; pub use P as Hidden;".into(),
        ),
        (
            "commands/mod.rs".into(),
            "use crate::navigation::Hidden; fn f() { Hidden::new(); }".into(),
        ),
    ]));
    assert!(
        found.contains("construction :: commands/mod.rs :: presence::Presence::new #1"),
        "{found:#?}"
    );
}

#[test]
fn macro_statement_tokens_do_not_hide_state_or_construction() {
    catches(
        "navigation.rs",
        "fn f() { custom!(let x = app.state::<u8>();); }",
        "managed_state",
    );
    catches(
        "navigation.rs",
        "fn f() { custom!(value => crate::presence::Presence::new()); }",
        "construction",
    );
    catches(
        "navigation.rs",
        "fn f() { tauri::generate_handler![crate::auth::action]; }",
        "owner_edge",
    );
}

#[test]
fn parent_path_attribute_uses_declared_owner() {
    let found = scanner::scan(&BTreeMap::from([
        (
            "lib.rs".into(),
            "mod navigation { #[path = \"../shared.rs\"] mod tests; }".into(),
        ),
        (
            "shared.rs".into(),
            "fn f() { super::super::auth::AuthController::new(); }".into(),
        ),
    ]));
    assert!(
        found.contains("construction :: shared.rs :: auth::AuthController::new #1"),
        "{found:#?}"
    );
    assert!(
        !found.iter().any(|v| v.starts_with("coverage")),
        "{found:#?}"
    );
}

#[test]
fn cfg_test_has_no_architecture_exemption() {
    catches(
        "navigation.rs",
        "#[cfg(test)] mod tests { fn f() { app.try_state::<u8>(); } }",
        "managed_state",
    );
    catches(
        "navigation/child.rs",
        "fn f() { super::super::auth::AuthController::new(); }",
        "construction",
    );
}

#[test]
fn pure_move_requires_one_for_one_baseline_relocation() {
    let before = scan("navigation.rs", "fn f() { app.state::<u8>(); }");
    let after = scan("navigation/moved.rs", "fn f() { app.state::<u8>(); }");
    let diff = compare(&after, &before);
    assert_eq!(diff.len(), 2);
    assert!(diff[0].starts_with("NEW "));
    assert!(diff[1].starts_with("STALE "));
}

#[test]
fn handler_aliases_use_the_registered_command_name() {
    let lib = "use tauri::generate_handler as handlers; use crate::commands::one as renamed; fn f() { app.invoke_handler(handlers![renamed]); }";
    let build = "fn main() { manifest.commands(&[\"one\"]); }";
    let permissions = BTreeMap::from([(
        "one.toml".into(),
        "[[permission]]\ncommands.allow = ['one']".into(),
    )]);
    assert!(inventory::check(lib, build, &permissions).is_empty());
    assert!(inventory::check(lib, "fn main() {}", &permissions)
        .contains("command_inventory :: build.rs :: missing one"));
}

#[test]
fn tauri_glob_and_state_function_values_do_not_hide_lookups() {
    catches(
        "navigation.rs",
        "use tauri::*; fn f(_: State<'_, u8>) {}",
        "managed_state",
    );
    catches(
        "navigation.rs",
        "use tauri::Manager; fn f() { let lookup = Manager::state::<u8>; lookup(&app); }",
        "managed_state",
    );
}

#[test]
fn absolute_crate_paths_do_not_follow_a_shadowing_import() {
    catches(
        "navigation.rs",
        "use std as presence; fn f() { crate::presence::Presence::new(); }",
        "construction",
    );
}

#[test]
fn inline_module_uses_its_own_type_declarations() {
    catches(
        "lib.rs",
        "mod presence { struct Presence; fn f() { Presence::new(); } }",
        "construction",
    );
}

#[test]
fn inventory_does_not_borrow_metadata_from_the_wrong_file() {
    let lib = "fn f() { app.invoke_handler(tauri::generate_handler![commands::one]); fake.commands(&[\"one\"]); }";
    let permissions = BTreeMap::from([(
        "one.toml".into(),
        "[[permission]]\ncommands.allow = ['one']".into(),
    )]);
    assert!(inventory::check(lib, "fn main() {}", &permissions)
        .contains("command_inventory :: build.rs :: missing one"));
    assert!(inventory::check("fn f() {}", lib, &permissions)
        .contains("command_inventory :: lib.rs :: missing one"));
}

#[test]
fn unknown_inline_owner_fails_coverage() {
    catches("lib.rs", "mod hidden_service { fn f() {} }", "coverage");
}

#[test]
fn os_adapter_construction_belongs_to_composition() {
    catches(
        "commands/preferences.rs",
        "fn f() { crate::desktop_preferences::login_item::PluginLoginItem(app); }",
        "construction",
    );
}

#[test]
#[should_panic(expected = "multiple module mounts")]
fn one_source_cannot_silently_replace_another_module_mount() {
    scanner::scan(&BTreeMap::from([
        (
            "lib.rs".into(),
            "#[path = \"shared.rs\"] mod navigation; #[path = \"shared.rs\"] mod presence;".into(),
        ),
        ("shared.rs".into(), "fn f() {}".into()),
    ]));
}

#[test]
#[should_panic(expected = "unscanned path module")]
fn explicit_path_cannot_leave_the_scanned_source_tree() {
    scan("lib.rs", "#[path = \"../outside.rs\"] mod navigation;");
}

#[test]
fn findings_do_not_depend_on_source_insertion_order() {
    let entries = [
        (
            "navigation.rs".into(),
            "fn f() { app.state::<u8>(); }".into(),
        ),
        (
            "presence/mod.rs".into(),
            "fn f() { crate::auth::AuthController::new(); }".into(),
        ),
    ];
    let forward = scanner::scan(&entries.clone().into_iter().collect());
    let backward = scanner::scan(&entries.into_iter().rev().collect());
    assert_eq!(forward, backward);
    let errors = compare(&forward, &Findings::new());
    assert!(errors.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn factory_return_types_identify_nonstandard_constructor_names() {
    catches(
        "presence/mod.rs",
        "struct Presence; fn f() { Presence::with_config(); } impl Presence { fn with_config() -> Self { Self } }",
        "construction",
    );
}

#[test]
fn block_aliases_inside_macros_keep_the_constructor_target() {
    catches(
        "presence/mod.rs",
        "struct Presence; fn f() { custom!(use crate::presence::Presence as P; P::new();); }",
        "construction",
    );
    catches(
        "presence/mod.rs",
        "struct Presence; fn f() { type P = Presence; P::new(); }",
        "construction",
    );
}

#[test]
fn private_imports_in_the_parent_remain_resolvable_from_children() {
    catches("presence/mod.rs", "struct Presence; use crate::presence::Presence as P; mod child { fn f() { super::P::new(); } }", "construction");
}

#[test]
fn public_type_aliases_cannot_hide_construction() {
    let found = scanner::scan(&BTreeMap::from([
        (
            "core_connection/mod.rs".into(),
            "pub struct Connection; pub type Alias = Connection;".into(),
        ),
        (
            "presence/mod.rs".into(),
            "fn f() { crate::core_connection::Alias::new(); }".into(),
        ),
    ]));
    assert!(
        found.contains("construction :: presence/mod.rs :: core_connection::Connection::new #1"),
        "{found:#?}"
    );
}

#[test]
fn reexport_does_not_make_a_private_module_a_public_entry() {
    let found = scanner::scan(&BTreeMap::from([
        (
            "core_connection/mod.rs".into(),
            "pub struct Connection; mod hidden { pub use super::Connection; }".into(),
        ),
        (
            "presence/mod.rs".into(),
            "fn f(_: crate::core_connection::hidden::Connection) {}".into(),
        ),
    ]));
    assert!(
        found.iter().any(|v| v.starts_with(
            "private_access :: presence/mod.rs :: core_connection::hidden::Connection"
        )),
        "{found:#?}"
    );
}

#[test]
fn public_reexport_keeps_the_owner_entry_public() {
    let found = scanner::scan(&BTreeMap::from([
        (
            "core_connection/mod.rs".into(),
            "mod hidden { pub struct Connection; } pub use hidden::Connection;".into(),
        ),
        (
            "presence/mod.rs".into(),
            "use crate::core_connection::Connection; fn f(_: Connection) { type C = Connection; }"
                .into(),
        ),
    ]));
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn test_only_handler_registration_cannot_fill_a_production_omission() {
    let lib = "fn run() { app.invoke_handler(tauri::generate_handler![commands::one]); } #[cfg(test)] mod tests { fn f() { app.invoke_handler(tauri::generate_handler![commands::two]); } }";
    let build = "fn main() { manifest.commands(&[\"one\", \"two\"]); }";
    let permissions = BTreeMap::from([(
        "both.toml".into(),
        "[[permission]]\ncommands.allow = ['one', 'two']".into(),
    )]);
    assert!(inventory::check(lib, build, &permissions)
        .contains("command_inventory :: lib.rs :: missing two"));
}

#[test]
fn unused_handler_lists_cannot_fill_a_registration_omission() {
    let lib = "fn run() { app.invoke_handler(tauri::generate_handler![commands::one]); let unused = tauri::generate_handler![commands::two]; }";
    let build = "fn main() { manifest.commands(&[\"one\", \"two\"]); }";
    let permissions = BTreeMap::from([(
        "both.toml".into(),
        "[[permission]]\ncommands.allow = ['one', 'two']".into(),
    )]);
    assert!(inventory::check(lib, build, &permissions)
        .contains("command_inventory :: lib.rs :: missing two"));
}
