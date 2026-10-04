//! The ownership map, and the test that keeps it honest.
//!
//! **The decision this file owns.** `docs/codebase/README.md` is written by
//! hand and holds the one thing no tool can derive: what each module *owns*.
//! The test below checks its factual claims: that the modules and reference
//! files it names exist, that every module in the tree has a row, and that no
//! job is stated with "and".
//!
//! The generated inventory of every declaration was retired on 2026-10-01: the
//! Rust LSP, and Shadows' own `where_is`, `who_uses` and `outline`, answer
//! where a name is, its signature and its callers exactly, with no upkeep.

use std::path::{Path, PathBuf};

/// The workspace root, two levels above this crate's manifest. The map spans
/// every crate in the workspace, and `docs/codebase/` lives at the root.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// This test does not judge whether an ownership phrase is *true* — no test
/// can — but it does refuse the three ways the file can be mechanically wrong:
/// naming something that does not exist, omitting a module that does, and
/// describing a module with a conjunction, which is how a file acquires a
/// second responsibility without anyone deciding to give it one.
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

/// `| `crates/shadows/src/storage/` | persistence |
///  `crates/shadows/src/storage/sqlite/project.rs` |`
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
    for (krate, src) in crate_sources(workspace) {
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

/// Every workspace crate's `src/` directory as `(crate folder, path)`, sorted
/// by folder name. A folder under `crates/` without a `src/` is not a crate
/// the map describes.
fn crate_sources(workspace: &Path) -> Vec<(String, PathBuf)> {
    let crates = workspace.join("crates");
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(&crates)
        .unwrap_or_else(|e| panic!("reading {}: {e}", crates.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.join("src").is_dir())
        .map(|path| {
            let name = path
                .file_name()
                .expect("a named entry")
                .to_string_lossy()
                .to_string();
            (name, path.join("src"))
        })
        .collect();
    out.sort();
    out
}
