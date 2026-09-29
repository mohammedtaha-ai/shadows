//! The code map, and the contract that keeps it honest.
//!
//! **The decision this file owns.** A code map that is written by hand rots,
//! and a rotted map is worse than none: it is read with the same trust as a
//! true one. So the map is split by what can be checked.
//!
//! - `docs/codebase/inventory.md` is **generated** from every `crates/*/src`
//!   and holds the mechanical facts — every externally reachable declaration, with its full
//!   signature. The first test below regenerates it and fails on any
//!   difference, so `cargo test` is what stops it from drifting. Since Windows
//!   `cargo test` is the CI acceptance gate, a stale map cannot reach `main`.
//! - `docs/codebase/README.md` is **written by hand** and holds the one thing
//!   no generator can derive: what each module *owns*. Its factual claims —
//!   that the modules and reference files it names exist, and that every module
//!   in the tree is accounted for — are checked by the second test.
//!
//! **Excluded on purpose:** line numbers. A line number is wrong as soon as a
//! line is inserted above it, and nothing fails when it lies. One field that
//! rots silently costs the reader their trust in every field beside it.
//!
//! **Why not built/not-built columns:** the tree already answers that, and
//! `docs/status.md` already narrates progress. Writing it a third time would
//! record one fact in three places, which is the failure this project's
//! documentation rules exist to prevent.
//!
//! Regenerate with `UPDATE_CODEMAP=1 cargo test -p shadows --test codemap`.

mod render;
mod scan;

use std::path::{Path, PathBuf};

/// The workspace root, two levels above this crate's manifest. The map spans
/// every crate in the workspace, and `docs/codebase/` lives at the root.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_inventory_matches_the_source_tree() {
    let root = workspace_root();
    let generated = render::document(&scan::scan(&root));
    let target = root.join("docs/codebase/inventory.md");

    if std::env::var_os("UPDATE_CODEMAP").is_some() {
        std::fs::write(&target, &generated).expect("writing the inventory");
        return;
    }

    let checked_in = std::fs::read_to_string(&target).unwrap_or_default();
    if checked_in == generated {
        return;
    }

    // Name the first differing line rather than dumping two documents: the
    // reader needs to know what moved, and the fix is one command either way.
    let mismatch = checked_in
        .lines()
        .zip(generated.lines())
        .position(|(a, b)| a != b)
        .map(|i| {
            format!(
                "first difference at line {}:\n  checked in: {}\n  generated:  {}",
                i + 1,
                checked_in.lines().nth(i).unwrap_or(""),
                generated.lines().nth(i).unwrap_or("")
            )
        })
        .unwrap_or_else(|| {
            format!(
                "the shorter document ends early: {} checked-in lines against {} generated",
                checked_in.lines().count(),
                generated.lines().count()
            )
        });

    panic!(
        "docs/codebase/inventory.md no longer describes crates/*/src.\n\n{mismatch}\n\n\
         Regenerate it and include it in the same commit as the code change:\n\
         \n    UPDATE_CODEMAP=1 cargo test -p shadows --test codemap\n"
    );
}

/// The hand-written half. This test does not judge whether an ownership phrase
/// is *true* — no test can — but it does refuse the three ways the file can be
/// mechanically wrong: naming something that does not exist, omitting a module
/// that does, and describing a module with a conjunction, which is how a file
/// acquires a second responsibility without anyone deciding to give it one.
#[test]
fn the_ownership_map_accounts_for_every_module() {
    let root = workspace_root();
    let readme = root.join("docs/codebase/README.md");
    let text = std::fs::read_to_string(&readme).expect("reading docs/codebase/README.md");

    let rows: Vec<Row> = text.lines().filter_map(parse_row).collect();
    assert!(
        rows.len() > 1,
        "no ownership rows parsed from docs/codebase/README.md — has the table format changed?"
    );

    for row in &rows {
        assert!(
            root.join(&row.module).exists(),
            "the map names `{}`, which is not in the tree",
            row.module
        );
        assert!(
            root.join(&row.reference).exists(),
            "`{}` is given as the reference file for `{}`, and does not exist",
            row.reference,
            row.module
        );
        assert!(
            !row.owns.to_lowercase().contains(" and "),
            "`{}` is described as \"{}\". A job stated with \"and\" is two jobs; \
             split the module or name the single job (CLAUDE.md, \"A file earns its size\").",
            row.module,
            row.owns
        );
    }

    for module in top_level_modules(&root) {
        assert!(
            rows.iter().any(|r| r.module == module),
            "`{module}` exists in the tree and is not in docs/codebase/README.md. \
             Add a row naming the one thing it owns — a module nobody has assigned a \
             job to is where unowned code collects."
        );
    }
}

struct Row {
    module: String,
    owns: String,
    reference: String,
}

/// `| `crates/shadows/src/storage/` | persistence | `crates/shadows/src/storage/sqlite/project.rs` |`
fn parse_row(line: &str) -> Option<Row> {
    let line = line.trim();
    if !line.starts_with('|') {
        return None;
    }
    let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
    if cells.len() != 3 {
        return None;
    }
    let module = cells[0].trim_matches('`');
    if !names_a_crate_source(module) {
        return None; // the header row and its `|---|` separator
    }
    Some(Row {
        module: module.trim_end_matches('/').to_string(),
        owns: cells[1].to_string(),
        reference: cells[2].trim_matches('`').to_string(),
    })
}

/// `crates/<crate>/src/…`: a path inside one workspace crate's source tree.
fn names_a_crate_source(path: &str) -> bool {
    let mut parts = path.split('/');
    parts.next() == Some("crates")
        && parts.next().is_some_and(|name| !name.is_empty())
        && parts.next() == Some("src")
        && parts.next().is_some_and(|rest| !rest.is_empty())
}

/// Each direct child of every `crates/*/src` is a unit someone must own.
/// `lib.rs` and `main.rs` are a crate's two entry points rather than modules,
/// so they are the only exemptions.
fn top_level_modules(workspace: &Path) -> Vec<String> {
    let mut modules = Vec::new();
    for (krate, src) in scan::crate_sources(workspace) {
        for entry in
            std::fs::read_dir(&src).unwrap_or_else(|e| panic!("reading {}: {e}", src.display()))
        {
            let path = entry.expect("directory entry").path();
            let name = path
                .file_name()
                .expect("a named entry")
                .to_string_lossy()
                .to_string();
            if name == "lib.rs" || name == "main.rs" {
                continue;
            }
            // A directory keeps its bare name (`crates/shadows/src/storage`)
            // and a single-file module keeps its extension
            // (`crates/shadows/src/config.rs`), because both forms are then
            // checked for existence exactly as written.
            modules.push(format!("crates/{krate}/src/{name}"));
        }
    }
    modules.sort();
    modules
}
