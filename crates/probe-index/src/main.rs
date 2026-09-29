//! Milestone 3, Task 0: a spike. It answers the questions of spec §15.10 and is
//! deleted once `docs/evidence/milestone3/PROBE.md` records its output.
//!
//! `probe-index tags` — every tag of each sample, with and without our extra patterns.
//! `probe-index notify <dir>` — notify-debouncer-full on Windows.
//! `probe-index scan <root> <runs>` — the periodic scan's walk.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use tree_sitter_tags::{TagsConfiguration, TagsContext};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("tags") => tags(),
        Some("notify") => notify(Path::new(&args[2])),
        Some("rawdelete") => raw_delete(Path::new(&args[2])),
        Some("debdelete") => debounced_delete(
            Path::new(&args[2]),
            args.get(3).is_some_and(|a| a == "nocache"),
        ),
        Some("scan") => scan(Path::new(&args[2]), args[3].parse().expect("runs")),
        _ => eprintln!("usage: probe-index tags | notify <dir> | scan <root> <runs>"),
    }
}

// ---------------------------------------------------------------- tags

#[cfg(feature = "rust")]
const RUST_EXTRA: &str = r#"
(const_item name: (identifier) @name) @definition.constant
(static_item name: (identifier) @name) @definition.constant
(function_signature_item name: (identifier) @name) @definition.method
(call_expression function: (scoped_identifier name: (identifier) @name)) @reference.call
(call_expression function: (generic_function function: (identifier) @name)) @reference.call
(call_expression function: (generic_function function: (scoped_identifier name: (identifier) @name))) @reference.call
(call_expression function: (generic_function function: (field_expression field: (field_identifier) @name))) @reference.call
(macro_invocation macro: (scoped_identifier name: (identifier) @name)) @reference.call
((type_identifier) @name @reference.type (#not-match? @name "^(_|Self)$"))
((scoped_identifier path: (identifier) @name) @reference.type (#match? @name "^[A-Z]"))
"#;

#[cfg(feature = "typescript")]
const TS_EXTRA: &str = r#"
(type_alias_declaration name: (type_identifier) @name) @definition.type
(enum_declaration name: (identifier) @name) @definition.enum
((type_identifier) @name @reference.type)
"#;

/// Appended for JavaScript, and for TypeScript and TSX after `TS_EXTRA`.
#[cfg(any(feature = "typescript", feature = "javascript"))]
const JS_EXTRA: &str = r#"
(program (lexical_declaration (variable_declarator name: (identifier) @name) @definition.constant))
(program (export_statement (lexical_declaration (variable_declarator name: (identifier) @name) @definition.constant)))
"#;

#[cfg(feature = "rust")]
const RUST_SAMPLE: &str = r#"
pub const LIMIT: usize = 5;
static NAME: &str = "x";
pub trait Store { fn open(path: &Path) -> Self; fn close(&self) {} }
struct Storage;
impl Storage { const MAX: u8 = 1; fn open(p: &Path) -> Storage { Storage } }
fn f(s: Storage) -> Vec<Storage> {
    let a = Storage::open(p);
    let b = std::fs::read(p);
    let c = parse::<u32>(x);
    let d = Vec::<Storage>::new();
    let e = x.collect::<Vec<_>>();
    tracing::info!("x");
    helper(a);
    vec![s]
}
"#;

#[cfg(feature = "typescript")]
const TS_SAMPLE: &str = r#"
export interface Props { a: string }
export type Id = string
type Local = Props | Id
export const f = () => 1
export const g = async (x: Id): Promise<Props> => ({ a: x })
const h = function () {}
export const LIMIT = 5
export function k(p: Props): Id { return p.a }
export enum Color { Red }
export class C { m(): void {} }
export function z() { const inner = 1; return inner }
export abstract class A { abstract n(): void }
"#;

#[cfg(feature = "javascript")]
const JS_SAMPLE: &str = r#"
export const f = () => 1
const g = function () {}
export function k(p) { return helper(p) }
class C { m() { this.n() } }
export const LIMIT = 5
module.exports.x = function () {}
new C()
"#;

fn config(language: tree_sitter::Language, query: &str) -> TagsConfiguration {
    match TagsConfiguration::new(language, query, "") {
        Ok(c) => c,
        Err(e) => panic!("TagsConfiguration: {e:?}"),
    }
}

fn print_tags(title: &str, config: &TagsConfiguration, source: &str) {
    println!("### {title}");
    let mut ctx = TagsContext::new();
    let (iter, has_error) = ctx
        .generate_tags(config, source.as_bytes(), None)
        .expect("generate_tags");
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for tag in iter {
        let tag = tag.expect("tag");
        let name = &source[tag.name_range.clone()];
        let kind = config.syntax_type_name(tag.syntax_type_id);
        let role = if tag.is_definition { "def" } else { "ref" };
        println!("{name} {kind} {role} {}", tag.span.start.row + 1);
        *kinds.entry(format!("{kind}/{role}")).or_default() += 1;
    }
    println!("-- parse error: {has_error}; counts: {kinds:?}\n");
}

fn both(title: &str, language: tree_sitter::Language, base: &str, extra: &str, src: &str) {
    print_tags(
        &format!("{title} (grammar query)"),
        &config(language.clone(), base),
        src,
    );
    let full = format!("{base}\n{extra}");
    print_tags(
        &format!("{title} (with extra)"),
        &config(language, &full),
        src,
    );
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn tags() {
    println!(
        "tree-sitter LANGUAGE_VERSION {} MIN_COMPATIBLE {}",
        tree_sitter::LANGUAGE_VERSION,
        tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION
    );
    #[cfg(feature = "rust")]
    {
        let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        println!("rust ABI {}", lang.abi_version());
        let q = tree_sitter_rust::TAGS_QUERY;
        both(
            "rust app.rs",
            lang.clone(),
            q,
            RUST_EXTRA,
            &read("crates/shadows-core/src/app.rs"),
        );
        both("rust sample", lang, q, RUST_EXTRA, RUST_SAMPLE);
    }
    #[cfg(feature = "typescript")]
    {
        #[cfg(feature = "javascript")]
        let q = format!(
            "{}\n{}",
            tree_sitter_typescript::TAGS_QUERY,
            tree_sitter_javascript::TAGS_QUERY
        );
        #[cfg(not(feature = "javascript"))]
        let q = tree_sitter_typescript::TAGS_QUERY.to_string();
        let ts: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        let tsx: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TSX.into();
        println!(
            "typescript ABI {} tsx ABI {}",
            ts.abi_version(),
            tsx.abi_version()
        );
        let extra = format!("{TS_EXTRA}\n{JS_EXTRA}");
        both(
            "ts client.ts",
            ts.clone(),
            &q,
            &extra,
            &read("web/src/api/client.ts"),
        );
        both("ts sample", ts, &q, &extra, TS_SAMPLE);
        both(
            "tsx copy-button.tsx",
            tsx,
            &q,
            &extra,
            &read("web/src/app/copy-button.tsx"),
        );
    }
    #[cfg(feature = "javascript")]
    {
        let js: tree_sitter::Language = tree_sitter_javascript::LANGUAGE.into();
        println!("javascript ABI {}", js.abi_version());
        both(
            "js sample",
            js,
            tree_sitter_javascript::TAGS_QUERY,
            JS_EXTRA,
            JS_SAMPLE,
        );
    }
}

// ---------------------------------------------------------------- notify

fn notify(dir: &Path) {
    use notify::RecursiveMode;
    use notify_debouncer_full::{DebounceEventResult, new_debouncer};

    let _ = std::fs::remove_dir_all(dir);
    let root = dir.join("watched");
    std::fs::create_dir_all(root.join("src")).expect("mkdir");
    std::fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").expect("write");

    let (tx, rx) = mpsc::channel::<DebounceEventResult>();
    let mut debouncer = new_debouncer(Duration::from_millis(500), None, tx).expect("new_debouncer");
    debouncer
        .watch(&root, RecursiveMode::Recursive)
        .expect("watch");

    // 1. Save through a temporary file and a rename, as editors do.
    println!("### 1. save through temp file + rename");
    std::fs::write(root.join("src/lib.rs.tmp"), "pub fn b() {}\n").expect("write tmp");
    std::fs::rename(root.join("src/lib.rs.tmp"), root.join("src/lib.rs")).expect("rename");
    drain(&rx, &root, Duration::from_secs(2), true);

    // 2. cargo build of a small crate inside the folder (its own target/).
    for (label, manifest) in [
        ("no dependencies", ""),
        ("with `ignore` (offline, 12 crates)", "ignore = \"0.4\"\n"),
    ] {
        println!("### 2. cargo build, {label}");
        let krate = root.join("krate");
        let _ = std::fs::remove_dir_all(&krate);
        std::fs::create_dir_all(krate.join("src")).expect("mkdir");
        std::fs::write(
            krate.join("Cargo.toml"),
            format!(
                "[package]\nname = \"krate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n[dependencies]\n{manifest}"
            ),
        )
        .expect("manifest");
        std::fs::write(krate.join("src/main.rs"), "fn main() {}\n").expect("main");
        drain(&rx, &root, Duration::from_secs(2), false);
        let started = Instant::now();
        let status = Command::new("cargo")
            .args(["build", "--offline", "--quiet"])
            .current_dir(&krate)
            .env_remove("CARGO_TARGET_DIR")
            .env("CARGO_INCREMENTAL", "0")
            .status()
            .expect("cargo");
        println!("cargo build: {status} in {:?}", started.elapsed());
        let files = walkdir_count(&krate.join("target"));
        println!("files in target/ afterwards: {files}");
        drain(&rx, &root, Duration::from_secs(3), false);
    }

    // 2b. A burst larger than a build: 20 000 new files in one folder.
    println!("### 2b. burst of 20000 file creations in one folder");
    let burst = root.join("burst");
    std::fs::create_dir_all(&burst).expect("mkdir");
    drain(&rx, &root, Duration::from_secs(2), false);
    for i in 0..20_000 {
        std::fs::write(burst.join(format!("f{i}.txt")), "x").expect("burst");
    }
    std::fs::write(root.join("src/after-burst.rs"), "fn c() {}\n").expect("after");
    drain(&rx, &root, Duration::from_secs(4), false);
    // Does the watcher still work after the burst?
    std::fs::write(root.join("src/later.rs"), "fn d() {}\n").expect("later");
    println!("-- a write after the burst:");
    drain(&rx, &root, Duration::from_secs(2), true);

    // 2c. Delete those 20 000 files, the folder staying watched.
    println!("### 2c. delete the 20000 files one by one (burst/ stays)");
    for i in 0..20_000 {
        std::fs::remove_file(burst.join(format!("f{i}.txt"))).expect("rm");
    }
    drain(&rx, &root, Duration::from_secs(4), false);
    std::fs::write(root.join("src/later2.rs"), "fn d2() {}\n").expect("later2");
    println!("-- a write after the delete:");
    drain(&rx, &root, Duration::from_secs(2), true);

    // 3. Rename the watched folder while it is watched.
    println!("### 3. rename the watched folder");
    let moved = dir.join("watched-renamed");
    match std::fs::rename(&root, &moved) {
        Ok(()) => println!("rename succeeded"),
        Err(e) => println!("rename FAILED: {e}"),
    }
    drain(&rx, &root, Duration::from_secs(2), true);
    let _ = std::fs::write(moved.join("src/after-move.rs"), "fn e() {}\n");
    println!("-- a write inside the renamed folder:");
    drain(&rx, &root, Duration::from_secs(2), true);
    // And deleting it.
    println!("### 3b. delete the (renamed) watched folder");
    match std::fs::remove_dir_all(&moved) {
        Ok(()) => println!("delete succeeded"),
        Err(e) => println!("delete FAILED: {e}"),
    }
    drain(&rx, &root, Duration::from_secs(2), false);
    // The folder comes back at its first path: does the watcher see it?
    std::fs::create_dir_all(root.join("src")).expect("recreate");
    std::fs::write(root.join("src/back.rs"), "fn f() {}\n").expect("back");
    println!("-- the folder recreated at its first path, a file written:");
    drain(&rx, &root, Duration::from_secs(2), true);
    drop(debouncer);
}

fn walkdir_count(dir: &Path) -> usize {
    ignore::WalkBuilder::new(dir)
        .standard_filters(false)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .count()
}

/// Collects debounced results until `quiet` passes with nothing new.
fn drain(
    rx: &mpsc::Receiver<notify_debouncer_full::DebounceEventResult>,
    root: &Path,
    quiet: Duration,
    verbose: bool,
) {
    let mut events = 0usize;
    let mut paths = 0usize;
    let mut under_target = 0usize;
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut rescan = 0usize;
    let mut errors = Vec::new();
    while let Ok(result) = rx.recv_timeout(quiet) {
        match result {
            Ok(batch) => {
                for e in batch {
                    events += 1;
                    if e.need_rescan() {
                        rescan += 1;
                    }
                    *kinds.entry(format!("{:?}", e.kind)).or_default() += 1;
                    for p in &e.paths {
                        paths += 1;
                        let rel: PathBuf = p.strip_prefix(root).unwrap_or(p).to_path_buf();
                        if rel.starts_with("krate/target") {
                            under_target += 1;
                        }
                    }
                    if verbose {
                        println!("  {:?} {:?}", e.kind, e.paths);
                    }
                }
            }
            Err(errs) => errors.extend(errs.into_iter().map(|e| format!("{e:?}"))),
        }
    }
    println!(
        "-- events {events}, paths {paths}, under krate/target {under_target}, need_rescan {rescan}, errors {errors:?}"
    );
    if !verbose {
        for (k, n) in kinds {
            println!("   {n:6} {k}");
        }
    }
}

// ---------------------------------------------------------------- scan

fn scan(root: &Path, runs: usize) {
    let mut times = Vec::new();
    let mut seen = 0usize;
    let mut kept = 0usize;
    for _ in 0..runs {
        let started = Instant::now();
        let mut files = 0usize;
        let mut code = 0usize;
        let mut total_size = 0u64;
        let walk = ignore::WalkBuilder::new(root)
            .parents(false)
            .git_global(false)
            .require_git(false)
            .follow_links(false)
            .filter_entry(|e| {
                !(e.file_type().is_some_and(|t| t.is_dir())
                    && matches!(e.file_name().to_str(), Some("target" | "node_modules")))
            })
            .build();
        for entry in walk {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            files += 1;
            total_size += meta.len();
            let _ = meta.modified();
            let ext = entry.path().extension().and_then(|e| e.to_str());
            if matches!(
                ext,
                Some("rs" | "ts" | "mts" | "cts" | "tsx" | "js" | "mjs" | "cjs" | "jsx")
            ) {
                code += 1;
            }
        }
        let t = started.elapsed();
        println!("run: {files} files ({code} in the language table), {total_size} bytes, {t:?}");
        times.push(t);
        seen = files;
        kept = code;
    }
    times.sort();
    println!(
        "files {seen}, kept {kept}; median {:?}, worst {:?}, best {:?}",
        times[times.len() / 2],
        times[times.len() - 1],
        times[0]
    );
}

/// 2d. The same deletion through a plain `notify` watcher, no debouncer:
/// is the loss in the kernel buffer or in the debouncer?
fn raw_delete(dir: &Path) {
    use notify::{RecursiveMode, Watcher};
    let _ = std::fs::remove_dir_all(dir);
    let burst = dir.join("raw/burst");
    std::fs::create_dir_all(&burst).expect("mkdir");
    for i in 0..20_000 {
        std::fs::write(burst.join(format!("f{i}.txt")), "x").expect("write");
    }
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(tx).expect("watcher");
    watcher
        .watch(&dir.join("raw"), RecursiveMode::Recursive)
        .expect("watch");
    for i in 0..20_000 {
        std::fs::remove_file(burst.join(format!("f{i}.txt"))).expect("rm");
    }
    let (mut ok, mut rescan, mut errors) = (0usize, 0usize, 0usize);
    while let Ok(r) = rx.recv_timeout(Duration::from_secs(3)) {
        match r {
            Ok(e) if e.need_rescan() => rescan += 1,
            Ok(_) => ok += 1,
            Err(_) => errors += 1,
        }
    }
    println!(
        "### 2d. raw notify watcher, 20000 removals: events {ok}, need_rescan {rescan}, errors {errors}"
    );
    std::fs::write(burst.join("after.txt"), "x").expect("after");
    let after = rx.recv_timeout(Duration::from_secs(2)).is_ok();
    println!("-- a write afterwards reported: {after}");
}

/// 2e. The same deletion through notify-debouncer-full with `NoCache` instead
/// of Windows' default `FileIdMap`, whose `remove_path` walks the whole cache.
fn debounced_delete(dir: &Path, no_cache: bool) {
    use notify::RecursiveMode;
    use notify_debouncer_full::{DebounceEventResult, NoCache, RecommendedCache};
    let _ = std::fs::remove_dir_all(dir);
    let root = dir.join("deb");
    let burst = root.join("burst");
    std::fs::create_dir_all(&burst).expect("mkdir");
    for i in 0..20_000 {
        std::fs::write(burst.join(format!("f{i}.txt")), "x").expect("write");
    }
    let (tx, rx) = mpsc::channel::<DebounceEventResult>();
    let config = notify::Config::default();
    let timeout = Duration::from_millis(500);
    let removed = |rx: &mpsc::Receiver<DebounceEventResult>| {
        let (mut n, mut rescan, mut errors) = (0usize, 0usize, 0usize);
        while let Ok(r) = rx.recv_timeout(Duration::from_secs(4)) {
            match r {
                Ok(batch) => {
                    rescan += batch.iter().filter(|e| e.need_rescan()).count();
                    n += batch.len();
                }
                Err(e) => errors += e.len(),
            }
        }
        (n, rescan, errors)
    };
    let started = Instant::now();
    let result = if no_cache {
        let mut d = notify_debouncer_full::new_debouncer_opt::<_, notify::RecommendedWatcher, _>(
            timeout, None, tx, NoCache, config,
        )
        .expect("debouncer");
        d.watch(&root, RecursiveMode::Recursive).expect("watch");
        println!("watch() took {:?}", started.elapsed());
        for i in 0..20_000 {
            std::fs::remove_file(burst.join(format!("f{i}.txt"))).expect("rm");
        }
        removed(&rx)
    } else {
        let mut d = notify_debouncer_full::new_debouncer_opt::<_, notify::RecommendedWatcher, _>(
            timeout,
            None,
            tx,
            RecommendedCache::new(),
            config,
        )
        .expect("debouncer");
        d.watch(&root, RecursiveMode::Recursive).expect("watch");
        println!("watch() took {:?}", started.elapsed());
        for i in 0..20_000 {
            std::fs::remove_file(burst.join(format!("f{i}.txt"))).expect("rm");
        }
        removed(&rx)
    };
    println!(
        "### 2e. debouncer, cache {}, 20000 removals: events {}, need_rescan {}, errors {}",
        if no_cache {
            "NoCache"
        } else {
            "FileIdMap (default)"
        },
        result.0,
        result.1,
        result.2
    );
}
