//! Contracts and tests name each other.
//!
//! **The decision this file owns.** A `contract.yaml` says which tests protect
//! each rule. This test refuses a cited test name that no test function carries
//! (a stale contract), and prints the two lists to act on: rules no test is
//! named for, and tests no contract names.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use yaml_rust2::{Yaml, YamlLoader};

/// The workspace root, two levels above this crate's manifest.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file with the extension under `dir`, sorted, skipping build output.
fn files_under(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == "node_modules")
            {
                continue;
            }
            files_under(&path, extension, out);
        } else if path.extension().is_some_and(|e| e == extension) {
            out.push(path);
        }
    }
    out.sort();
}

/// The crate directory name a path under `crates/` belongs to.
fn crate_of(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root.join("crates")).unwrap_or(path);
    relative.components().next().map_or_else(String::new, |c| {
        c.as_os_str().to_string_lossy().into_owned()
    })
}

/// The test function names in one source text: a `fn` that follows a `#[test]`
/// or `#[tokio::test...]` line, with other attributes or doc lines in between.
fn test_functions(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut armed = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("#[test]") || line.starts_with("#[tokio::test") {
            armed = true;
        } else if armed && (line.starts_with("#[") || line.starts_with("//")) {
            continue;
        } else if armed {
            if let Some(at) = line.find("fn ") {
                let name: String = line[at + 3..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                found.push(name);
            }
            armed = false;
        }
    }
    found
}

/// The comparable form of a cited name: its last word, then its last `::` part.
fn normalize(cited: &str) -> String {
    let word = cited.split_whitespace().last().unwrap_or("");
    word.rsplit("::").next().unwrap_or("").to_string()
}

/// The names a `tested_by` value cites; `none`, `unknown` and empty cite none.
fn cited_names(value: &Yaml) -> Vec<String> {
    let raw: Vec<String> = match value {
        Yaml::String(s) => vec![s.clone()],
        Yaml::Array(items) => items
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    };
    raw.iter()
        .map(|s| normalize(s))
        .filter(|n| !n.is_empty() && n != "none" && n != "unknown")
        .collect()
}

#[test]
fn contracts_cite_only_tests_that_exist() {
    let root = workspace_root();
    let mut sources = Vec::new();
    files_under(&root.join("crates"), "rs", &mut sources);

    let mut tests_by_crate: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut all_tests: BTreeSet<String> = BTreeSet::new();
    for path in &sources {
        let text = std::fs::read_to_string(path).expect("reading a source file");
        for name in test_functions(&text) {
            all_tests.insert(name.clone());
            tests_by_crate
                .entry(crate_of(&root, path))
                .or_default()
                .insert(name);
        }
    }

    let mut contracts = Vec::new();
    files_under(&root.join("crates"), "yaml", &mut contracts);
    contracts.retain(|p| p.file_name().is_some_and(|n| n == "contract.yaml"));
    assert!(
        !contracts.is_empty(),
        "no contract.yaml found under crates/"
    );

    let mut cited: BTreeSet<String> = BTreeSet::new();
    let mut stale: Vec<String> = Vec::new();
    let mut unprotected: Vec<String> = Vec::new();
    for path in &contracts {
        let shown = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        let shown = shown.replace('\\', "/");
        let text = std::fs::read_to_string(path).expect("reading a contract");
        let docs = YamlLoader::load_from_str(&text)
            .unwrap_or_else(|e| panic!("{shown} is not valid YAML: {e}"));
        let Some(doc) = docs.first() else {
            continue;
        };
        let mut names: Vec<String> = Vec::new();
        for obligation in doc["obligations"].as_vec().into_iter().flatten() {
            let these = cited_names(&obligation["tested_by"]);
            if these.is_empty() {
                let rule = obligation["rule"].as_str().unwrap_or("");
                let rule: String = rule.chars().take(80).collect();
                unprotected.push(format!("{shown}: {rule}"));
            }
            names.extend(these);
        }
        for test in doc["tests"].as_vec().into_iter().flatten() {
            if let Some(name) = test["name"].as_str() {
                names.push(normalize(name));
            }
        }
        for name in names {
            if !all_tests.contains(&name) {
                stale.push(format!("{shown}: {name}"));
            }
            cited.insert(name);
        }
    }

    unprotected.sort();
    println!("\nRules with no test ({}):", unprotected.len());
    for line in &unprotected {
        println!("  {line}");
    }
    println!("\nTests no contract cites, per crate:");
    for (krate, tests) in &tests_by_crate {
        let uncited: Vec<&String> = tests.iter().filter(|t| !cited.contains(*t)).collect();
        println!("  {krate} ({})", uncited.len());
        for name in uncited {
            println!("    {name}");
        }
    }

    stale.sort();
    assert!(
        stale.is_empty(),
        "{} cited test(s) match no test function:\n  {}",
        stale.len(),
        stale.join("\n  ")
    );
}
