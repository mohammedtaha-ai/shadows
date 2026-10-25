//! Exact exceptions make ownership migration measurable without permitting new debt.
mod fixtures;
mod inventory;
mod scanner;

use std::collections::BTreeSet;
use std::path::Path;

type Findings = BTreeSet<String>;

fn compare(actual: &Findings, baseline: &Findings) -> Vec<String> {
    actual
        .difference(baseline)
        .map(|v| format!("NEW {v}"))
        .chain(baseline.difference(actual).map(|v| format!("STALE {v}")))
        .collect()
}

fn baseline(text: &str) -> Findings {
    let mut result = Findings::new();
    for line in text
        .lines()
        .filter(|v| !v.is_empty() && !v.starts_with('#'))
    {
        assert_eq!(line.split(" :: ").count(), 3, "invalid baseline: {line}");
        assert!(result.insert(line.to_owned()), "duplicate baseline: {line}");
    }
    result
}

#[test]
fn native_architecture_ratchet() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sources = scanner::read_sources(&root.join("src"));
    let mut actual = scanner::scan(&sources);
    actual.extend(inventory::scan(root));
    actual.extend(scanner::coverage(root, &sources));
    let errors = compare(&actual, &baseline(include_str!("baseline.txt")));
    assert!(
        errors.is_empty(),
        "native architecture ratchet:\n{}",
        errors.join("\n")
    );
}
