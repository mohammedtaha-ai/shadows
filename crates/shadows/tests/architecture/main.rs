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

#[test]
fn database_foundation_depends_on_no_service() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
    let package = lock
        .split("[[package]]")
        .find(|entry| entry.lines().any(|line| line == r#"name = "core-db""#))
        .expect("core-db is a workspace package");
    for line in package
        .lines()
        .filter(|line| line.trim_start().starts_with('"'))
    {
        let name = line.trim().trim_matches(',').trim_matches('"');
        assert!(
            !name.starts_with("shadows") && (!name.starts_with("core-") || name == "core-db"),
            "foundation must not depend on a service: {name}"
        );
    }
}

#[test]
fn extracted_runtime_depends_only_on_the_foundation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
    let package = lock
        .split("[[package]]")
        .find(|entry| entry.lines().any(|line| line == r#"name = "core-runtime""#))
        .expect("core-runtime is a workspace package");
    for line in package
        .lines()
        .filter(|line| line.trim_start().starts_with('"'))
    {
        let name = line.trim().trim_matches(',').trim_matches('"');
        assert!(
            !name.starts_with("shadows") && (!name.starts_with("core-") || name == "core-db"),
            "runtime must not depend on a service or composition: {name}"
        );
    }
}
