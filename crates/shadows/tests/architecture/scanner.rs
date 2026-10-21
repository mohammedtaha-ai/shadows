use super::Findings;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser, Tree};

#[path = "resolution.rs"]
mod resolution;
#[path = "sql.rs"]
mod sql;

const CORE: &str = "crates/shadows-core/src/";
const FOUNDATIONS: [&str; 3] = ["db", "command", "testing"];
const POOL_METHODS: [&str; 7] = [
    "reader",
    "write_txn",
    "watch_committed",
    "now",
    "append_event",
    "classify",
    "record_command",
];

struct Source {
    path: String,
    module: String,
    owner: String,
    service: bool,
    adapter: bool,
    text: String,
    tree: Tree,
}

impl Source {
    fn text(&self, node: Node<'_>) -> &str {
        &self.text[node.byte_range()]
    }
}

fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn nodes<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    out.push(node);
    for child in children(node) {
        nodes(child, out);
    }
}

fn all_nodes(node: Node<'_>) -> Vec<Node<'_>> {
    let mut out = Vec::new();
    nodes(node, &mut out);
    out
}

fn name(source: &Source, node: Node<'_>) -> String {
    node.child_by_field_name("name")
        .map(|node| source.text(node).to_owned())
        .unwrap_or_default()
}

fn symbol(source: &Source, mut node: Node<'_>) -> String {
    let mut parts = Vec::new();
    loop {
        if matches!(
            node.kind(),
            "function_item" | "struct_item" | "enum_item" | "const_item" | "mod_item" | "type_item"
        ) {
            parts.push(name(source, node));
        } else if node.kind() == "impl_item"
            && let Some(ty) = node.child_by_field_name("type")
        {
            parts.push(source.text(ty).to_owned());
        }
        match node.parent() {
            Some(parent) => node = parent,
            None => break,
        }
    }
    parts.reverse();
    if parts.is_empty() {
        "module".to_owned()
    } else {
        parts.join("::")
    }
}

#[derive(Default)]
struct Recorder {
    findings: Findings,
    occurrences: BTreeMap<(String, String), usize>,
}

impl Recorder {
    fn add(&mut self, category: &str, path: &str, description: &str, evidence: &str) {
        // Separate ordinals keep an added identical call visible to the ratchet.
        let normalized = evidence.split_whitespace().collect::<Vec<_>>().join(" ");
        let hash = normalized
            .bytes()
            .fold(0xcbf29ce484222325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
            });
        let identity = format!("{path} :: {description} [{hash:016x}]");
        let occurrence = self
            .occurrences
            .entry((category.to_owned(), identity.clone()))
            .or_default();
        *occurrence += 1;
        self.findings
            .entry(category.to_owned())
            .or_default()
            .insert(format!("{identity} #{}", *occurrence));
    }
    fn at(&mut self, category: &str, source: &Source, node: Node<'_>, reason: &str) {
        self.add(
            category,
            &source.path,
            &format!("{} {reason}", symbol(source, node)),
            source.text(node),
        );
    }
}

fn files(directory: &Path, out: &mut Vec<PathBuf>) {
    if !directory.is_dir() {
        return;
    }
    let mut entries = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

fn sources(root: &Path) -> Vec<Source> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).expect("crate directory") {
        files(&entry.unwrap().path().join("src"), &mut paths);
    }
    paths.sort();
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    paths
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let parts = relative.split('/').collect::<Vec<_>>();
            let krate = parts[1];
            let owner = if relative.starts_with(CORE) {
                parts
                    .get(3)
                    .filter(|part| !part.ends_with(".rs"))
                    .copied()
                    .unwrap_or("composition")
                    .to_owned()
            } else {
                krate.strip_prefix("core-").unwrap_or(krate).to_owned()
            };
            let service = (relative.starts_with(CORE)
                && owner != "composition"
                && !FOUNDATIONS.contains(&owner.as_str()))
                || (krate.starts_with("core-") && krate != "core-db");
            let mut modules = vec![krate.replace('-', "_")];
            for part in &parts[3..] {
                if !["mod.rs", "lib.rs", "main.rs"].contains(part) {
                    modules.push(part.trim_end_matches(".rs").to_string());
                }
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let tree = parser.parse(&text, None).unwrap();
            assert!(
                !tree.root_node().has_error(),
                "Rust parser cannot classify {relative}"
            );
            Source {
                path: relative.clone(),
                module: modules.join("::"),
                owner,
                service,
                adapter: ["shadows", "shadows-http", "shadows-mcp"].contains(&krate),
                text,
                tree,
            }
        })
        .collect()
}

fn registration(root: &Path, sources: &[Source], recorder: &mut Recorder) {
    let app = sources
        .iter()
        .find(|source| source.path == format!("{CORE}app.rs"))
        .expect("AppCore source");
    // Rust's doc attributes keep block, line and explicit doc forms equivalent.
    let parsed = syn::parse_file(&app.text).expect("AppCore syntax");
    let declaration = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == "AppCore" => Some(item),
            _ => None,
        })
        .expect("AppCore declaration");
    let map = declaration
        .attrs
        .iter()
        .filter_map(|attribute| {
            if !attribute.path().is_ident("doc") {
                return None;
            }
            let syn::Meta::NameValue(value) = &attribute.meta else {
                return None;
            };
            let syn::Expr::Lit(value) = &value.value else {
                return None;
            };
            let syn::Lit::Str(value) = &value.lit else {
                return None;
            };
            Some(value.value())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let codemap = std::fs::read_to_string(root.join("docs/codebase/README.md")).expect("code map");
    let mut services = BTreeSet::new();
    for entry in std::fs::read_dir(root.join(CORE)).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            let owner = path.file_name().unwrap().to_string_lossy().to_string();
            if FOUNDATIONS.contains(&owner.as_str()) {
                let relative = format!("{CORE}{owner}/");
                if !codemap
                    .lines()
                    .any(|line| line.starts_with('|') && line.contains(&relative))
                {
                    recorder.add(
                        "registration",
                        &relative,
                        "foundation missing code-map row",
                        &relative,
                    );
                }
            } else {
                services.insert((
                    format!("{CORE}{owner}"),
                    format!("{CORE}{owner}/contract.yaml"),
                ));
            }
        }
    }
    for entry in std::fs::read_dir(root.join("crates")).unwrap() {
        let path = entry.unwrap().path();
        let krate = path.file_name().unwrap().to_string_lossy().to_string();
        if krate.starts_with("core-") && path.join("src").is_dir() {
            if krate == "core-db" {
                if !codemap.contains("crates/core-db/src/") {
                    recorder.add(
                        "registration",
                        "crates/core-db/src",
                        "foundation missing code-map row",
                        "core-db",
                    );
                }
            } else {
                services.insert((
                    format!("crates/{krate}/src"),
                    format!("crates/{krate}/contract.yaml"),
                ));
            }
        }
    }
    let mut registered = BTreeSet::new();
    for line in map.lines() {
        let line = line.trim().trim_start_matches('*').trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells = line
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`'))
            .collect::<Vec<_>>();
        if cells
            .first()
            .is_some_and(|cell| *cell == "Service" || cell.starts_with("---"))
        {
            continue;
        }
        let valid = if let [owner, job, directory, contract] = cells.as_slice() {
            let known = services.contains(&(directory.to_string(), contract.to_string()));
            let entry = sources.iter().find(|source| {
                source.path == format!("{directory}/mod.rs")
                    || source.path == format!("{directory}/lib.rs")
            });
            let declared = entry.is_some_and(|source| {
                children(source.tree.root_node())
                    .iter()
                    .any(|node| node.kind() == "struct_item" && name(source, *node) == *owner)
            });
            known
                && declared
                && !job.is_empty()
                && !job
                    .split(|ch: char| !ch.is_alphabetic())
                    .any(|word| word.eq_ignore_ascii_case("and"))
        } else {
            false
        };
        if !valid {
            recorder.add("registration", &app.path, "invalid AppCore map row", line);
            continue;
        }
        let directory = cells[2].to_string();
        if !registered.insert(directory) {
            recorder.add("registration", &app.path, "duplicate AppCore map row", line);
        }
    }
    for (directory, contract) in services {
        if !root.join(&contract).is_file() {
            recorder.add("registration", &directory, "missing contract", &contract);
        }
        if !registered.contains(&directory) {
            recorder.add(
                "registration",
                &directory,
                "missing AppCore map row",
                &contract,
            );
        }
    }
}

fn history(line: &str) -> bool {
    let lower = line.to_lowercase();
    line.contains('§')
        || ["used to", "no longer", "milestone", "probe.md"]
            .iter()
            .any(|marker| lower.contains(marker))
        || lower
            .split_whitespace()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|pair| pair[0] == "task" && pair[1].starts_with(|ch: char| ch.is_ascii_digit()))
        || line
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|word| {
                (7..=40).contains(&word.len()) && word.bytes().all(|ch| ch.is_ascii_hexdigit())
            })
}

pub(super) fn scan(root: &Path) -> Findings {
    let sources = sources(root);
    let mut recorder = Recorder::default();
    registration(root, &sources, &mut recorder);
    let index = resolution::Index::new(&sources);
    let tables = sql::table_owners();
    for source in &sources {
        for node in all_nodes(source.tree.root_node()) {
            match node.kind() {
                "line_comment" | "block_comment" => {
                    for line in source.text(node).lines().filter(|line| history(line)) {
                        recorder.add(
                            "comments",
                            &source.path,
                            &format!("{} comment", symbol(source, node)),
                            line,
                        );
                    }
                }
                "function_item"
                    if source.service
                        && node.parent().is_some_and(|parent| {
                            ["source_file", "declaration_list"].contains(&parent.kind())
                        }) =>
                {
                    let inside_impl = node
                        .parent()
                        .and_then(|parent| parent.parent())
                        .is_some_and(|parent| matches!(parent.kind(), "impl_item" | "trait_item"));
                    if !inside_impl
                        && children(node)
                            .iter()
                            .any(|child| child.kind() == "visibility_modifier")
                    {
                        let signature = source.text(node).split('{').next().unwrap();
                        recorder.add(
                            "public_functions",
                            &source.path,
                            &format!("{} free public function", symbol(source, node)),
                            signature,
                        );
                    }
                }
                "string_literal" | "raw_string_literal" => {
                    if let Ok(literal) = syn::parse_str::<syn::LitStr>(source.text(node)) {
                        sql::check(source, node, &literal.value(), &tables, &mut recorder);
                    }
                }
                _ => {}
            }
            resolution::check(source, node, &index, &mut recorder);
        }
    }
    recorder.findings
}
