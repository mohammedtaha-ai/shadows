//! A shrinking inventory of source ownership exceptions.

mod fixtures;
mod scanner;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type Findings = BTreeMap<String, BTreeSet<String>>;

const CATEGORIES: [&str; 4] = ["ownership", "public_functions", "comments", "registration"];

fn compare(actual: &Findings, expected: &Findings) -> Vec<String> {
    let mut errors = Vec::new();
    for category in CATEGORIES {
        let empty = BTreeSet::new();
        let found = actual.get(category).unwrap_or(&empty);
        let baseline = expected.get(category).unwrap_or(&empty);
        for entry in found.difference(baseline) {
            errors.push(format!("NEW {category}: {entry}"));
        }
        for entry in baseline.difference(found) {
            errors.push(format!("STALE {category}: {entry}"));
        }
    }
    errors
}

fn baselines(directory: &Path) -> Findings {
    CATEGORIES
        .into_iter()
        .map(|category| {
            let file = directory.join(format!("{category}.allowlist"));
            let text = std::fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
            let mut entries = BTreeSet::new();
            for line in text
                .lines()
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
            {
                assert!(line.contains(" :: "), "invalid baseline record: {line}");
                assert!(
                    entries.insert(line.to_owned()),
                    "duplicate baseline record: {line}"
                );
            }
            (category.to_owned(), entries)
        })
        .collect()
}

#[test]
fn repository_obeys_the_architecture_ratchet() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let actual = scanner::scan(&root);
    let expected = baselines(&root.join("crates/shadows/tests/architecture"));
    let errors = compare(&actual, &expected);
    assert!(
        errors.is_empty(),
        "architecture ratchet:\n{}",
        errors.join("\n")
    );
}

/// Workspace crates every service may use: the foundation and the test doubles.
const BELOW_EVERY_SERVICE: [&str; 3] = ["core-db", "fake-acp", "fake-codex"];

/// Each service crate and the workspace crates its production build may depend on, besides
/// `BELOW_EVERY_SERVICE`. An owner here is one the service calls through AppCore's wiring.
const DEPENDENCY_POLICY: &[(&str, &[&str])] = &[
    ("core-db", &[]),
    ("core-runtime", &[]),
    ("core-secrets", &[]),
    ("core-usage", &[]),
    ("core-person", &[]),
    ("core-providers", &["core-secrets"]),
    ("core-projects", &["shadows-agent"]),
    ("core-instructions", &["core-projects"]),
    ("core-standards", &["core-projects"]),
    (
        "codebase",
        &["core-projects", "shadows-index", "shadows-lsp"],
    ),
    (
        "code-record",
        &[
            "codebase",
            "core-design",
            "core-operations",
            "core-plans",
            "core-projects",
        ],
    ),
    (
        "project-state",
        &[
            "code-record",
            "core-design",
            "core-plans",
            "core-projects",
            "core-runs",
        ],
    ),
    (
        "core-agents",
        &["core-projects", "core-providers", "core-secrets"],
    ),
    (
        "core-extensions",
        &[
            "core-agents",
            "core-projects",
            "core-providers",
            "core-secrets",
            "shadows-process",
        ],
    ),
    (
        "core-threads",
        &["codebase", "core-agents", "core-projects", "shadows-agent"],
    ),
    (
        "core-grants",
        &["codebase", "core-projects", "core-threads", "shadows-agent"],
    ),
    (
        "core-operations",
        &[
            "core-projects",
            "core-runtime",
            "core-threads",
            "shadows-agent",
        ],
    ),
    (
        "core-harness",
        &[
            "core-agents",
            "core-extensions",
            "core-grants",
            "core-instructions",
            "core-operations",
            "core-person",
            "core-projects",
            "core-providers",
            "core-secrets",
            "core-standards",
            "core-threads",
            "shadows-agent",
            "shadows-process",
        ],
    ),
    (
        "core-design",
        &[
            "core-grants",
            "core-harness",
            "core-operations",
            "core-projects",
            "core-standards",
            "core-threads",
            "shadows-agent",
            "shadows-index",
            "shadows-process",
        ],
    ),
    (
        "core-plans",
        &[
            "codebase",
            "core-design",
            "core-grants",
            "core-harness",
            "core-operations",
            "core-projects",
            "core-threads",
            "shadows-agent",
            "shadows-index",
            "shadows-process",
        ],
    ),
    (
        "core-turns",
        &[
            "code-record",
            "codebase",
            "core-design",
            "core-grants",
            "core-harness",
            "core-instructions",
            "core-operations",
            "core-plans",
            "core-projects",
            "core-runtime",
            "core-standards",
            "core-threads",
            "core-usage",
            "shadows-agent",
            "shadows-index",
            "shadows-process",
        ],
    ),
    (
        "core-runs",
        &[
            "code-record",
            "codebase",
            "core-grants",
            "core-harness",
            "core-operations",
            "core-plans",
            "core-projects",
            "core-threads",
            "core-turns",
            "shadows-agent",
            "shadows-process",
        ],
    ),
    (
        "core-messages",
        &[
            "codebase",
            "core-agents",
            "core-events",
            "core-projects",
            "core-providers",
            "core-secrets",
            "core-threads",
            "core-turns",
            "shadows-agent",
        ],
    ),
    (
        "core-project-lifecycle",
        &[
            "code-record",
            "codebase",
            "core-grants",
            "core-plans",
            "core-projects",
            "core-threads",
            "shadows-agent",
            "shadows-index",
            "shadows-process",
        ],
    ),
    (
        "core-events",
        &[
            "core-harness",
            "core-operations",
            "core-plans",
            "core-projects",
            "core-runtime",
            "core-threads",
            "core-turns",
            "shadows-agent",
            "shadows-index",
            "shadows-process",
        ],
    ),
];

type Edge = (String, String, String);

/// Whether a workspace crate must have a row in the dependency policy.
fn held_to_policy(name: &str) -> bool {
    name == "core-db" || scanner::service_crate(name)
}

/// The workspace's packages and its internal production edges (normal and build) as
/// `(from, to, kind)`, read once from `cargo metadata`. A dependency is internal when its path
/// is a workspace package's manifest directory, so a renamed dependency resolves to the package
/// it names.
fn production_edges(root: &Path) -> (BTreeSet<String>, BTreeSet<Edge>) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .current_dir(root)
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let packages = metadata["packages"].as_array().expect("packages");
    let by_directory: BTreeMap<std::path::PathBuf, String> = packages
        .iter()
        .map(|package| {
            let manifest = Path::new(package["manifest_path"].as_str().unwrap());
            let name = package["name"].as_str().unwrap().to_owned();
            (manifest.parent().unwrap().to_path_buf(), name)
        })
        .collect();
    let mut edges = BTreeSet::new();
    for package in packages {
        let from = package["name"].as_str().unwrap();
        for dependency in package["dependencies"].as_array().unwrap() {
            let kind = dependency["kind"].as_str().unwrap_or("normal");
            if kind == "dev" {
                continue;
            }
            let Some(path) = dependency["path"].as_str() else {
                continue;
            };
            if let Some(to) = by_directory.get(Path::new(path)) {
                edges.insert((from.to_owned(), to.clone(), kind.to_owned()));
            }
        }
    }
    (by_directory.into_values().collect(), edges)
}

/// Every way the workspace at `root` breaks `policy`: a held crate with no row, an edge from
/// such a crate, an edge its row does not allow, and a row naming no workspace crate.
fn dependency_violations(root: &Path, policy: &[(&str, &[&str])]) -> Vec<String> {
    let (packages, edges) = production_edges(root);
    let rows: BTreeMap<&str, &[&str]> = policy.iter().copied().collect();
    let mut errors = Vec::new();
    for name in packages.iter().filter(|name| held_to_policy(name)) {
        if !rows.contains_key(name.as_str()) {
            errors.push(format!("{name} has no dependency policy row"));
        }
    }
    for row in rows.keys().filter(|row| !packages.contains(**row)) {
        errors.push(format!("policy row {row} names no workspace crate"));
    }
    for (from, to, kind) in &edges {
        match rows.get(from.as_str()) {
            None if held_to_policy(from) => errors.push(format!(
                "{from} -> {to} ({kind}): {from} has no dependency policy row"
            )),
            Some(allowed)
                if !allowed.contains(&to.as_str())
                    && !BELOW_EVERY_SERVICE.contains(&to.as_str()) =>
            {
                errors.push(format!(
                    "{from} -> {to} ({kind}) is not in the dependency policy"
                ))
            }
            _ => {}
        }
    }
    errors
}

#[test]
fn service_crates_depend_only_on_the_owners_their_policy_row_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let errors = dependency_violations(&root, DEPENDENCY_POLICY);
    assert!(
        errors.is_empty(),
        "dependency policy:\n{}",
        errors.join("\n")
    );
}
