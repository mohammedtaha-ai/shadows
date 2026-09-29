# Milestone 3 — The Code Index Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An agent connected to Shadows over `/mcp` asks where a name is defined, who uses it, or what a file holds. Shadows answers with project, path, line, kind and signature, never code. The index stays current while files change.

**Architecture:** This is spec §15.
- A new crate, `shadows-index`, turns one file's text into tags with `tree-sitter-tags`. It knows nothing of SQLite or projects.
- A ninth service, `Code`, in `shadows-core`:
  - stores the tags in SQLite;
  - indexes the active projects;
  - watches them with a plain `notify` watcher and its own ~500 ms gathering, with a periodic scan as the safety net;
  - answers the questions.
- `shadows-mcp` gets three tools, and `shadows-http` gets routes, each calling one `Code` method.

**Tech Stack:** Rust 2024, SQLx 0.9 on SQLite, axum 0.8 + utoipa, `rmcp` 3.4.1. New dependencies, with versions confirmed by Task 0:
- `tree-sitter` 0.27, `tree-sitter-tags` 0.27;
- `tree-sitter-rust` 0.24, `tree-sitter-typescript` 0.23, `tree-sitter-javascript` 0.25;
- `notify` 8.2 (no `notify-debouncer-full`: PROBE.md);
- `ignore` 0.4.

**Spec:** `docs/superpowers/specs/2026-09-30-code-index-design.md` (§15). Where this plan and §15 disagree, §15 is right and the plan is the defect: stop and report.

## Global Constraints

- **Branch** `milestone-3/code-index`, which already holds the spec commits.
  - No git worktrees.
  - One commit per task. Task 0 has two: its source, then its evidence and the deletion.
  - Push only when Mohammed asks.
- **Execution:** every subagent runs on opus, stated explicitly. The reviewer is an opus agent that fixes what it finds, runs the gate and commits. The controller verifies the review and does not repeat it (CLAUDE.md).
- **Read first:** `docs/codebase/README.md`, `docs/codebase/inventory.md`, and the contract of any service you change (`crates/shadows-core/src/<service>/contract.yaml`). Then open only the files the task names.
- **Few tests: one per behaviour** (§15.11). Do not add a test the task does not list. Break a test on purpose, to prove it fails, only where the task says so (the scope test, Task 4).
- **Nothing existing changes behaviour.**
  - No existing route, tool, JSON field, status code, error code, message, command kind or fingerprint changes.
  - `api/openapi.json` changes only by the routes Task 4 adds.
  - `web/` is not touched.
- **A service change updates its contract in the same commit** (`docs/codebase/contracts/TEMPLATE.yaml`). `contracts.rs` finds services through `AppCore`'s accessors, so the task that adds `core.code()` (Task 2) also adds `code/contract.yaml`.
- **Code map:** every task that adds a module or moves a signature:
  - regenerates the map with `UPDATE_CODEMAP=1 cargo test -p shadows --test codemap`;
  - adds each new module's one-job line to `docs/codebase/README.md`, stated without "and".
- **Files:** a file over 300 lines states its one job in the commit message. At 500 lines it splits (CLAUDE.md).
- **Builds** use the C: target directory:
  `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/Temp/claude/E--Globalprojects-shadows/e86a610a-4c5b-4114-884d-faf050ce6a96/scratchpad/target CARGO_INCREMENTAL=0`.
  - Stop any running `shadows.exe` before building or testing.
  - After the task, clean stale artifacts: `cargo clean -p <each workspace package>`. Keep the dependency builds.
- **The gate before every commit** (CLAUDE.md):
  1. `cargo fmt --all --check`
  2. `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
  3. `cargo test --workspace`
  4. `cargo clippy --workspace -- -D warnings`
  5. `cargo tree -e features,no-dev --workspace | grep test-support` prints nothing
  6. `git diff --exit-code api/`, except in Task 4, which regenerates it.

  Report the Rust test count. Take it at the start of Task 1; after that it may only grow.
- **Limits from §15:**
  - signature ≤ 200 characters;
  - file size cap 1 MB;
  - 50 hits and 10 suggestions per answer;
  - debounce about 500 ms;
  - periodic scan every 60 s;
  - `active_limit` defaults to 5 and ranges from 1 to 20.
- **Never read outside a project's folder.** No path argument is joined to a folder before `scope::inside` has accepted it (Task 2).

## Review Focus

1. **A save through rename.** VS Code and other editors write a temporary file, then rename it over the original. The person expects the new line numbers on the next question. (Task 3 test `watching_and_the_periodic_scan_keep_the_index_true` saves this way.)
2. **A build writing into `target/`.** The person expects no `target/` file in the index, and no stall. (Task 2 test `indexing_follows_the_files` puts a `.rs` file under `target/` and asserts it is absent. Task 0 measures the watcher's behaviour during a build.)
3. **A project that is not a Git repository, or has a nested `.gitignore`.** The person expects `.gitignore` to be honoured anyway. (Task 2 test: no `.git` folder, and a nested `sub/.gitignore` that hides `sub/gen.rs`.)
4. **Arabic or other non-ASCII text on a long signature line.** The person expects no panic and no cut through a character. (Task 1 test `rust_tags` has a 300-character line with Arabic in a string.)
5. **A question while the first index is still running.** The person expects an answer with the status `indexing`, not an error. (Task 3 test `only_the_most_recent_projects_are_watched` asks the sixth project at once.)

---

## Execution map

```text
Task 0  probe (spike): queries, grammar ABI and build cost, notify on Windows, scan cost → evidence; code deleted
Task 1  crates/shadows-index: the language table and extract
Task 2  Code: migration 0008, store, scan, questions, scope; AppCore accessor; contract
Task 3  Code: workers, watcher, periodic scan, re-check of hits, active set, touch, start/shut_down
Task 4  Code: links and the active limit (commands); MCP tools; HTTP routes; openapi   ← the scope test
Task 5  documents (§15.12)
Task I  (controller) whole-branch review, Mohammed's Windows run, evidence, PR
```

---

### Task 0: The probe

A spike. Its code is **deleted** once its evidence file is written (CLAUDE.md). Nothing it builds stays in the tree.

**Files:**
- Create, temporarily: `crates/probe-index/` (a binary crate added to the workspace). It is deleted in Step 6.
- Create: `docs/evidence/milestone3/PROBE.md`.

**Interfaces:**
- Produces: facts that Tasks 1–3 rely on, recorded in `PROBE.md`. If one contradicts §15, the controller amends §15 before Task 1.

- [ ] **Step 1: Queries.** Build a `TagsConfiguration` for each language from its crate's `LANGUAGE` and `TAGS_QUERY`.
  - TypeScript and TSX use `format!("{}\n{}", tree_sitter_typescript::TAGS_QUERY, tree_sitter_javascript::TAGS_QUERY)`.
  - Run each over a sample, and record every tag as `name kind role line`. For Rust, run over `crates/shadows-core/src/app.rs`; for TypeScript and TSX, over `web/src/api/client.ts` and one `.tsx` file in `web/src`; for JavaScript, over a small hand-written sample.
  - Record what is missing that §15.3 needs:
    - Rust: `const`, `static`, calls through a path (`Storage::open`), uses of a type;
    - TypeScript: arrow-function consts (`export const f = () => …`) and type aliases.

  For each gap, write the extra query pattern that catches it and show that it does. These patterns are what Task 1 appends.
- [ ] **Step 2: ABI and build cost.** Record:
  - that each grammar loads under the `tree-sitter` version `tree-sitter-tags` uses (no `LanguageError`);
  - the exact versions that agree;
  - the clean-build time of the probe crate, and of the same crate with each grammar removed in turn. That gives each grammar's cost.
- [ ] **Step 3: `notify` on Windows.** Run `notify-debouncer-full` on a temporary folder (500 ms), and record the events for:
  - a save through a temporary file and a rename;
  - `cargo build` of a small crate inside the folder (its `target/`): the number of events, and whether any error or `need_rescan()` event arrived;
  - renaming the watched folder while it is watched. Does the rename succeed, and what does the watcher report?
- [ ] **Step 4: The periodic scan's cost.** Time `ignore::WalkBuilder` over this repository with the §15.4 settings:
  - `parents(false)`, `git_global(false)`, `require_git(false)`, `follow_links(false)`;
  - `target/` and `node_modules/` filtered out;
  - reading only `metadata()` (size and modified time).

  Run it 10 times, with Windows Defender on as it normally is. Record the files seen, and the median and worst time.
- [ ] **Step 5: Commit the probe's source**, so Git history keeps it (CLAUDE.md). Run `cargo fmt --all --check` and `cargo clippy -p probe-index -- -D warnings` first.

```bash
git add crates/probe-index Cargo.toml Cargo.lock
git commit -m "spike(M3): the probe's source, deleted in the next commit"
```

- [ ] **Step 6: Write `docs/evidence/milestone3/PROBE.md`.** Include:
  - the date;
  - the commit that holds the probe's source (Step 5);
  - the raw outputs, trimmed to what matters;
  - one line per question: what §15 assumed, what was measured, and what Task 1 or Task 3 must do.
- [ ] **Step 7: Delete the probe.** Remove `crates/probe-index/` and its workspace entry. `Cargo.toml` and `Cargo.lock` must then equal their state before Step 5 (`git diff <before Step 5> -- Cargo.toml Cargo.lock` is empty).
- [ ] **Step 8: Commit.** Run the whole gate: the tree is back to product code only.

```bash
git add -A crates/probe-index docs/evidence/milestone3/PROBE.md Cargo.toml Cargo.lock
git commit -m "evidence(M3): the probe — tags queries, grammar cost, notify on Windows, scan cost"
```

(The controller then amends §15 where `PROBE.md` proved it wrong, as its own docs commit, before Task 1.)

---

### Task 1: `shadows-index`

**Files:**
- Create: `crates/shadows-index/Cargo.toml`
- Create: `crates/shadows-index/src/lib.rs`: the public surface.
- Create: `crates/shadows-index/src/languages.rs`: the language table.
- Create: `crates/shadows-index/src/extract.rs`: one file's text to tags.
- Create: `crates/shadows-index/queries/`: one `.scm` file of extra patterns per language that Task 0 found needs them, e.g. `rust.scm`.
- Create: `crates/shadows-index/tests/extract.rs`
- Modify: root `Cargo.toml`, adding `[workspace.dependencies]` for the tree-sitter crates and `shadows-index = { path = "crates/shadows-index" }`.
- Modify: `docs/codebase/README.md`, adding the rows for `crates/shadows-index/src/` (one job each).

**Interfaces:**
- Produces, used by Task 2:

```rust
// crates/shadows-index/src/lib.rs
pub use extract::{Role, Tag, extract};
pub use languages::{Language, language_for};

// languages.rs
pub struct Language {
    pub name: &'static str,                 // "rust", "typescript", "tsx", "javascript"
    pub extensions: &'static [&'static str], // without the dot
    // private: the built TagsConfiguration
}
/// The language a path's extension names, or None.
pub fn language_for(path: &std::path::Path) -> Option<&'static Language>;

// extract.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role { Definition, Reference }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub kind: String,              // the capture's kind: "function", "method", "class", …
    pub role: Role,
    pub line: u32,                 // 1-based: the line the name is on
    pub signature: Option<String>, // definitions only: that line, trimmed, ≤ 200 chars
}
/// Blocking: parses `text`. A file that does not fully parse still gives
/// the tags found. Never panics on any input.
pub fn extract(language: &Language, text: &str) -> Vec<Tag>;
```

- [ ] **Step 1: The crate.** `Cargo.toml` uses `workspace = true` for version, edition, lints and every dependency. Its dependencies are `tree-sitter`, `tree-sitter-tags` and the three grammar crates, at the versions `PROBE.md` recorded. It has no dependency on any Shadows crate.

- [ ] **Step 2: Write the failing tests** in `crates/shadows-index/tests/extract.rs`:

```rust
use shadows_index::{Role, Tag, extract, language_for};
use std::path::Path;

fn defs(tags: &[Tag]) -> Vec<(String, String, u32)> {
    tags.iter()
        .filter(|t| t.role == Role::Definition)
        .map(|t| (t.name.clone(), t.kind.clone(), t.line))
        .collect()
}

#[test]
fn rust_tags() {
    let long = format!("    let s = \"{}\";", "سلام ".repeat(60)); // > 200 chars, Arabic
    let src = format!(
        "pub struct Storage;\n\
         impl Storage {{\n    pub fn open(path: &str) -> Self {{\n{long}\n        Storage\n    }}\n}}\n\
         pub const LIMIT: u32 = 5;\n\
         fn main() {{ let _ = Storage::open(\"x\"); helper(); }}\n\
         fn helper() {{}}\n"
    );
    let lang = language_for(Path::new("a.rs")).expect("rust is in the table");
    let tags = extract(lang, &src);
    let d = defs(&tags);
    assert!(d.contains(&("Storage".into(), "class".into(), 1)), "{d:?}");
    assert!(d.contains(&("open".into(), "method".into(), 3)), "{d:?}");
    // Lines: 1 struct, 2 impl, 3 fn open, 4 the long line, 5–7 the body's end, 8 const, 9 main, 10 helper.
    assert!(d.iter().any(|(n, _, l)| n == "LIMIT" && *l == 8), "const is a definition: {d:?}");
    let open = tags.iter().find(|t| t.name == "open" && t.role == Role::Definition).unwrap();
    assert_eq!(open.signature.as_deref(), Some("pub fn open(path: &str) -> Self {"));
    // A reference through a path, and a plain call.
    assert!(tags.iter().any(|t| t.name == "open" && t.role == Role::Reference && t.line == 9));
    assert!(tags.iter().any(|t| t.name == "helper" && t.role == Role::Reference));
    // The long line is no definition, but every signature respects the limit.
    assert!(tags.iter().filter_map(|t| t.signature.as_ref()).all(|s| s.chars().count() <= 200));
}

#[test]
fn typescript_and_tsx_tags() {
    let ts = "export interface Plan { id: string }\n\
              export function load(id: string): Plan { return get(id); }\n\
              export class Client { fetch() { return load('x'); } }\n";
    let d = defs(&extract(language_for(Path::new("a.ts")).unwrap(), ts));
    assert!(d.contains(&("Plan".into(), "interface".into(), 1)), "{d:?}");
    assert!(d.contains(&("load".into(), "function".into(), 2)), "{d:?}");
    assert!(d.contains(&("Client".into(), "class".into(), 3)), "{d:?}");
    let tsx = "export function Card() { return <div>{title()}</div>; }\n";
    let d = defs(&extract(language_for(Path::new("a.tsx")).unwrap(), tsx));
    assert!(d.contains(&("Card".into(), "function".into(), 1)), "{d:?}");
}

#[test]
fn javascript_tags_and_broken_code() {
    let js = "function start() { stop(); }\nclass Job {}\nfunction half( {\n";
    let tags = extract(language_for(Path::new("a.mjs")).unwrap(), js);
    let d = defs(&tags);
    assert!(d.contains(&("start".into(), "function".into(), 1)), "{d:?}");
    assert!(d.contains(&("Job".into(), "class".into(), 2)), "{d:?}");
    assert!(tags.iter().any(|t| t.name == "stop" && t.role == Role::Reference));
    assert!(language_for(Path::new("a.py")).is_none());
}
```

Kinds are what the queries capture. If `PROBE.md` recorded a different kind (e.g. `struct`), use the recorded one in the assertion. Adjust the extra patterns so that `const` and path calls are caught, and say so in the report.

- [ ] **Step 3: Run the tests to watch them fail.** `cargo test -p shadows-index`. Expected: they fail to compile, because the crate is empty.
- [ ] **Step 4: `languages.rs`.**

```rust
//! One job: the table of languages Shadows indexes (spec §15.3). Adding a
//! language is one entry here, its grammar crate, and its extra query file.

use std::path::Path;
use std::sync::LazyLock;

use tree_sitter_tags::TagsConfiguration;

pub struct Language {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub(crate) config: TagsConfiguration,
}

fn config(language: tree_sitter::Language, queries: &[&str]) -> TagsConfiguration {
    TagsConfiguration::new(language, &queries.join("\n"), "")
        .expect("a compiled-in tags query is valid")
}

static LANGUAGES: LazyLock<Vec<Language>> = LazyLock::new(|| {
    let (ts, js) = (tree_sitter_typescript::TAGS_QUERY, tree_sitter_javascript::TAGS_QUERY);
    vec![
        Language {
            name: "rust",
            extensions: &["rs"],
            config: config(
                tree_sitter_rust::LANGUAGE.into(),
                &[tree_sitter_rust::TAGS_QUERY, include_str!("../queries/rust.scm")],
            ),
        },
        Language {
            name: "typescript",
            extensions: &["ts", "mts", "cts"],
            config: config(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), &[ts, js]),
        },
        Language {
            name: "tsx",
            extensions: &["tsx"],
            config: config(tree_sitter_typescript::LANGUAGE_TSX.into(), &[ts, js]),
        },
        Language {
            name: "javascript",
            extensions: &["js", "mjs", "cjs", "jsx"],
            config: config(tree_sitter_javascript::LANGUAGE.into(), &[js]),
        },
    ]
});

pub fn language_for(path: &Path) -> Option<&'static Language> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    LANGUAGES.iter().find(|l| l.extensions.contains(&ext.as_str()))
}
```

If `TagsConfiguration` is not `Sync` in the pinned version, keep a `thread_local!` table instead of the `LazyLock`, and say so in the report.

The extra patterns are in `docs/evidence/milestone3/PROBE.md`, "The extra query patterns", copied verbatim into three files, each **after** the grammar's query (the earlier pattern wins):
- `queries/rust.scm` → Rust: `[rust TAGS_QUERY, rust.scm]`;
- `queries/typescript.scm` (TS_EXTRA) → TypeScript and TSX: `[ts, js, typescript.scm, javascript.scm]`;
- `queries/javascript.scm` (JS_EXTRA) → JavaScript: `[js, javascript.scm]`.

- [ ] **Step 5: `extract.rs`.**

```rust
//! One job: one file's text in, its tags out (spec §15.3).

use std::cell::RefCell;

use tree_sitter_tags::TagsContext;

use crate::languages::Language;

thread_local! {
    static CONTEXT: RefCell<TagsContext> = RefCell::new(TagsContext::new());
}

const SIGNATURE_CHARS: usize = 200;

pub fn extract(language: &Language, text: &str) -> Vec<Tag> {
    let source = text.as_bytes();
    let lines: Vec<&str> = text.lines().collect();
    CONTEXT.with_borrow_mut(|ctx| {
        let Ok((tags, _failed)) = ctx.generate_tags(&language.config, source, None) else {
            return Vec::new();
        };
        tags.filter_map(Result::ok)
            .filter_map(|tag| {
                let name = text.get(tag.name_range.clone())?.to_string();
                let kind = language.config.syntax_type_name(tag.syntax_type_id).to_string();
                let row = tag.span.start.row; // the row the name's node starts on
                let line = u32::try_from(row + 1).ok()?;
                let signature = tag.is_definition.then(|| {
                    lines.get(row).map(|l| l.trim().chars().take(SIGNATURE_CHARS).collect())
                }).flatten();
                let role = if tag.is_definition { Role::Definition } else { Role::Reference };
                Some(Tag { name, kind, role, line, signature })
            })
            .collect()
    })
}
```

`line` is the row of the **name**. Check in the pinned version whether `tag.span` or `tag.name_range` gives that row, and use whichever does. The test fixes it: `open` is on line 3.

- [ ] **Step 6: Run the tests until they pass.** `cargo test -p shadows-index`. Expected: 3 passed.
- [ ] **Step 7: Code map, then the gate.** Add the rows for `crates/shadows-index/src/lib.rs`, `languages.rs` and `extract.rs` to the README. Run `UPDATE_CODEMAP=1 cargo test -p shadows --test codemap`, then the whole gate.
- [ ] **Step 8: Commit.**

```bash
git add crates/shadows-index Cargo.toml Cargo.lock docs/codebase
git commit -m "feat(M3): shadows-index — the language table and extract (§15.3)"
```

---

### Task 2: `Code`: storage, scan, questions

**Files:**
- Create: `crates/shadows-core/migrations/0008_code_index.sql`
- Create in `crates/shadows-core/src/code/`, one job each:
  - `mod.rs`: the `Code` service's methods;
  - `model.rs`: its types, with no sqlx;
  - `store.rs`: its queries;
  - `scan.rs`: walking a project and indexing what changed;
  - `scope.rs`: which projects and paths a question may read;
  - `contract.yaml`: its contract.
- Modify: `crates/shadows-core/src/lib.rs` (`mod code;` and re-export the public types), `crates/shadows-core/src/app.rs` (the `Code` field, `from_parts`, `pub fn code(&self) -> &Code`), and `crates/shadows-core/Cargo.toml` (`shadows-index` and `ignore`).
- Test: `crates/shadows-core/tests/code_index.rs`.

**Interfaces:**
- Consumes: `shadows_index::{language_for, extract, Tag, Role}` (Task 1).
- Produces, used by Tasks 3 and 4:

```rust
// code/model.rs — all derive Debug, Clone, serde::Serialize, utoipa::ToSchema
pub enum IndexState {                       // serde tag = "state", snake_case
    Ready,
    Indexing { done: u32, found: u32 },
    Inactive,
    NoDirectory,
    DirectoryMissing,
}
pub struct ProjectStatus {
    pub project: String,                    // the slug
    pub state: IndexState,
    pub files: u32,
    pub skipped: Vec<Skipped>,              // one per reason, with its count
    pub updated_at: Option<String>,
}
pub struct Skipped { pub reason: String, pub count: u32 }   // "too_large" | "binary" | "not_utf8"
pub struct Hit {
    pub project: String,                    // the slug
    pub path: String,                       // relative, with '/'
    pub line: u32,
    pub kind: String,
    pub name: String,
    pub signature: Option<String>,
    pub matched_by: Option<String>,         // Some("name") on every `references` hit
}
pub struct Answer {
    pub hits: Vec<Hit>,                     // ≤ 50, by project, path, line
    pub more: bool,
    pub suggestions: Vec<String>,           // ≤ 10, only when hits is empty
    pub status: Vec<ProjectStatus>,         // one per project in the scope
}

// code/scope.rs — not ToSchema
#[derive(Debug, Clone, Copy)]
pub enum Asker<'a> {
    Person(&'a ProjectId),   // HTTP: refusals are INVALID_COMMAND
    Grant(&'a Grant),        // MCP: refusals are GRANT_SCOPE
}

// code/mod.rs
/// Cheap to clone: `Projects`, `Threads` and `Turns` hold a clone to call
/// `touch` (Task 3, §15.8).
#[derive(Clone)]
pub struct Code { inner: Arc<Inner> } // Inner: storage, and from Task 3 the active set and workers
impl Code {
    pub(crate) fn new(storage: Arc<Storage>) -> Self;
    pub async fn definitions(&self, asker: Asker<'_>, only: Option<&str>, name: &str) -> Result<Answer, CoreError>;
    pub async fn references(&self, asker: Asker<'_>, only: Option<&str>, name: &str) -> Result<Answer, CoreError>;
    pub async fn outline(&self, asker: Asker<'_>, only: Option<&str>, path: &str) -> Result<Answer, CoreError>;
    pub async fn status(&self, project: &ProjectId) -> Result<ProjectStatus, CoreError>;
    /// Scans `project` once, now, on the caller's task. Tests and Task 3's worker call it.
    pub(crate) async fn scan(&self, project: &ProjectId) -> Result<(), CoreError>;
}
```

`only` is a project slug. `path` in `outline` is relative; `""` or `"."` means the whole project.

- [ ] **Step 1: Migration `0008_code_index.sql`.** It follows 0007's style.

```sql
-- Milestone 3 (§15.4). The index is derived from the files: it can be deleted
-- and rebuilt, and carries no command or journal row.

CREATE TABLE code_file (
    project_id     TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    path_key       TEXT NOT NULL,          -- `path`, lowercased on Windows
    path           TEXT NOT NULL,          -- relative to the project folder, '/'
    size           INTEGER NOT NULL,
    modified_ms    INTEGER NOT NULL,
    language       TEXT NOT NULL,
    skipped_reason TEXT NULL CHECK (skipped_reason IN ('too_large','binary','not_utf8')),
    PRIMARY KEY (project_id, path_key)
);

CREATE TABLE code_tag (
    project_id TEXT NOT NULL,
    path_key   TEXT NOT NULL,
    path       TEXT NOT NULL,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL,
    role       TEXT NOT NULL CHECK (role IN ('definition','reference')),
    line       INTEGER NOT NULL CHECK (line >= 1),
    signature  TEXT NULL,
    FOREIGN KEY (project_id, path_key) REFERENCES code_file(project_id, path_key) ON DELETE CASCADE
);
CREATE INDEX code_tag_name ON code_tag(project_id, name);
CREATE INDEX code_tag_path ON code_tag(project_id, path_key, line);

CREATE TABLE code_setting (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    active_limit INTEGER NOT NULL CHECK (active_limit BETWEEN 1 AND 20)
);
INSERT INTO code_setting (id, active_limit) VALUES (1, 5);

CREATE TABLE project_link (
    project_id        TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    linked_project_id TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    created_at        TEXT NOT NULL,
    PRIMARY KEY (project_id, linked_project_id),
    CHECK (project_id <> linked_project_id)
);
```

Check that `Storage::open` enables `foreign_keys`. If it does not, the store deletes a file's tags explicitly, and the cascade is not relied on.

- [ ] **Step 2: Write the failing test** in `crates/shadows-core/tests/code_index.rs`.
  - Open a `Storage` on a temporary database, as `project_contract.rs` does.
  - Create a project whose folder is a `tempfile::TempDir`, through `Projects` or the store.
  - Build the core with `AppCore::assemble`. Check how the core tests build `CoreParts`, and reuse their fixture.

```rust
#[tokio::test]
async fn indexing_follows_the_files() {
    let (core, project, dir) = core_with_project().await; // fixture: temp db + project on `dir`
    let w = |p: &str, s: &str| {
        let p = dir.path().join(p);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    };
    w("src/a.rs", "pub fn alpha() {}\n");
    w("src/b.rs", "fn beta() { alpha(); }\n");
    w("target/debug/gen.rs", "fn hidden_in_target() {}\n");       // never walked
    w("sub/.gitignore", "gen.rs\n");                                // no .git anywhere
    w("sub/gen.rs", "fn hidden_by_nested_ignore() {}\n");
    w("big.rs", &"// x\n".repeat(300_000));                         // > 1 MB
    let me = shadows_core::Asker::Person(&project);

    core.code().scan_for_test(&project).await.unwrap();
    let a = core.code().definitions(me, None, "alpha").await.unwrap();
    assert_eq!((a.hits[0].path.as_str(), a.hits[0].line), ("src/a.rs", 1));
    assert_eq!(a.hits[0].signature.as_deref(), Some("pub fn alpha() {}"));
    let r = core.code().references(me, None, "alpha").await.unwrap();
    assert_eq!(r.hits[0].path, "src/b.rs");
    assert_eq!(r.hits[0].matched_by.as_deref(), Some("name"));
    for hidden in ["hidden_in_target", "hidden_by_nested_ignore"] {
        assert!(core.code().definitions(me, None, hidden).await.unwrap().hits.is_empty(), "{hidden}");
    }
    let s = core.code().status(&project).await.unwrap();
    assert!(s.skipped.iter().any(|k| k.reason == "too_large" && k.count == 1));

    // Change one, delete one, add one; the next scan follows each.
    w("src/a.rs", "\n\npub fn alpha() {}\n");
    std::fs::remove_file(dir.path().join("src/b.rs")).unwrap();
    w("src/c.ts", "export function gamma() {}\n");
    core.code().scan_for_test(&project).await.unwrap();
    assert_eq!(core.code().definitions(me, None, "alpha").await.unwrap().hits[0].line, 3);
    assert!(core.code().references(me, None, "alpha").await.unwrap().hits.is_empty());
    assert_eq!(core.code().definitions(me, None, "gamma").await.unwrap().hits.len(), 1);
    let o = core.code().outline(me, None, "src").await.unwrap();
    assert_eq!(o.hits.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(), ["alpha", "gamma"]);
    let near = core.code().definitions(me, None, "amm").await.unwrap();
    assert!(near.hits.is_empty() && near.suggestions == ["gamma"]);
}
```

`scan_for_test` is `scan` exposed through `shadows_core::testing`, as `testing` does for other internals. `Asker` is re-exported from `shadows_core`.

A modified time can match within the filesystem's resolution after a quick rewrite. So the second write also changes the file's size, which the scan compares too.

- [ ] **Step 3: Run the test to watch it fail.** `cargo test -p shadows-core --test code_index`. Expected: it fails to compile, because there is no `code()`.
- [ ] **Step 4: `scope.rs`.** Who may read what, and paths:

```rust
/// The projects a question may read: the asker's project, then its direct
/// links, by slug. `only` narrows it to one of them.
pub(super) async fn projects(storage: &Storage, asker: &Asker<'_>, only: Option<&str>)
    -> Result<Vec<(ProjectId, String /* slug */, Option<String> /* directory */)>, CoreError>;

/// A relative path inside the project, with '/', or the refusal
/// INVALID_COMMAND "the path must be inside the project".
pub(super) fn inside(path: &str) -> Result<String, CoreError> {
    let p = path.replace('\\', "/");
    let p = p.trim_start_matches("./").trim_end_matches('/');
    let bad = p.starts_with('/')
        || p.contains(':')
        || p.split('/').any(|c| c == "..");
    if bad {
        return Err(CoreError::Refused {
            code: ErrorCode::InvalidCommand,
            message: "the path must be inside the project".into(),
        });
    }
    Ok(if p == "." { String::new() } else { p.to_string() })
}
```

An `only` outside the scope is refused:
- for `Asker::Grant`, with code `GrantScope`;
- for `Asker::Person`, with `InvalidCommand`;
- in both cases with the message `the project <slug> is not linked to this project`.

A project with no directory is in the scope, with the state `NoDirectory`.

- [ ] **Step 5: `scan.rs`.** Following §15.4 steps 1–6:
  - **The walk:** `ignore::WalkBuilder::new(dir)` with `.parents(false).git_global(false).require_git(false).follow_links(false)`, and a `filter_entry` that refuses directories named `target` or `node_modules`. Keep only paths for which `language_for` answers.
  - **The key:** `path` is relative, with `/`. `path_key` is `path.to_lowercase()` when `cfg!(windows)`, and `path` elsewhere.
  - **Skipping unchanged files:** a file is skipped when its size and `modified_ms` equal its row.
  - **Reading a changed file:**
    - larger than `1_048_576` bytes → `too_large`;
    - a NUL byte in its first 8 KiB → `binary`;
    - `String::from_utf8` fails → `not_utf8`.

    A skipped file still gets a `code_file` row with its reason, and no tags.
  - **Parsing:** `extract` runs in `tokio::task::spawn_blocking`.
  - **Writing:** one `write_txn` per file. It deletes the file's tags, upserts its row, and inserts its tags.
  - **After the walk:** delete the rows, and their tags, of every `path_key` the walk did not see.
  - A failed write of one file is `tracing::warn!(project, path, error, "code.index_failed")`, and the walk goes on.
  - **One rule, one function.** Two `pub(super)` functions in `scan.rs` hold the rules, and every path that indexes goes through them:
    - `keeps(path) -> bool`: the walk's filter (`target`, `node_modules`, `language_for`). Task 3's watcher filter calls it too; the ignore files are the walk's own.
    - `index_file(project, dir, path)`: steps 2–5 for one file, or deleting its rows if it is gone. The scan calls it per file; Task 3's `Files` and `Recheck` jobs call it too.

    A second copy of either would drift, and the index would then depend on which path saw the change.
- [ ] **Step 6: `store.rs`.** The queries, all through `self.reader()` except the writes:
  - **exact name:** `WHERE project_id IN (…) AND name = ? AND role = ?`, ordered by the scope's order, then `path`, `line`, `kind`. With `LIMIT 51`: 51 rows means `more = true`, and the answer keeps 50. (`code_tag` has no key, so ordering never falls back to insertion order; two identical rows are indistinguishable anyway. CLAUDE.md: ordering is explicit.)
  - **suggestions:** `SELECT DISTINCT name … WHERE project_id IN (…) AND role = ? AND name LIKE '%' || ? || '%' ESCAPE '\'`, with `%`, `_` and `\` escaped in the text, `ORDER BY name LIMIT 10`. The role is the question's own, so `definitions` never suggests a name that is only used.
  - **outline:** definitions where `path_key = ?` or `path_key LIKE ? || '/%' ESCAPE '\'` (escaped as above, since `_` is common in paths; the key of the asked path, lowercased as §15.4 says), or every definition when the path is empty; ordered by `path`, `line`, `name`, with `LIMIT 51`.
  - **status counts:** files, and the skipped files grouped by reason.
- [ ] **Step 7: `mod.rs` and `AppCore`.**
  - The questions resolve the scope, run the query, and fill `status` per project.
  - Until Task 3, the state is `Ready` for a project with rows, and `Inactive` for one without.
  - `app.rs` builds `Code::new(storage.clone())` in `from_parts` and adds the accessor.
  - `lib.rs` re-exports `Code`, `Asker`, `Answer`, `Hit`, `IndexState`, `ProjectStatus` and `Skipped`.
- [ ] **Step 8: `code/contract.yaml`.** Read `docs/codebase/contracts/TEMPLATE.yaml` whole, and follow its ten rules. Write the contract from the code you wrote, not from this plan:
  - the methods, grouped as questions and indexing;
  - the obligations, each with its `tested_by` (a test name, or `none` with a `note`): never read outside a folder; one transaction per file; answers ≤ 50;
  - `not_the_caller's`: an adapter never checks the scope or joins a path itself; it passes the text to `Code`;
  - `tests`: `indexing_follows_the_files`, with what its **body** proves;
  - a `gap`, if any rule is not yet tested.
- [ ] **Step 9: Run the test until it passes, then the gate.** `cargo test -p shadows-core --test code_index`, then `cargo test -p shadows-core --test contracts`. Add the README rows for `crates/shadows-core/src/code/`, regenerate the code map, and run the whole gate.
- [ ] **Step 10: Commit.**

```bash
git add crates/shadows-core docs/codebase
git commit -m "feat(M3): Code — the index's tables, the scan and the three questions (§15.4–§15.5)"
```

---

### Task 3: `Code`: keeping it current

**Files:**
- Create: `crates/shadows-core/src/code/watch.rs`: one active project's worker, with its watcher and its periodic scan.
- Create: `crates/shadows-core/src/code/active.rs`: which projects are active, in order of use.
- Modify: `code/mod.rs` (`start`, `touch`, `shut_down`, and the re-check in the questions), `code/model.rs` (`CodeConfig`), `code/contract.yaml`, `app.rs` (`start` starts `Code`; `shut_down` stops it first), `turns/mod.rs` (`send` touches), `threads/mod.rs` (`list` touches), `projects/mod.rs` (`create` touches), and the contracts of those three services (their calls to `Code::touch`).
- Modify: `crates/shadows-core/Cargo.toml` (`notify` only).
- Test: `crates/shadows-core/tests/code_index.rs` (two tests added).

**Interfaces:**
- Consumes: Task 2's `Code`, `scan`, questions and store.
- Produces:

```rust
// code/model.rs
pub struct CodeConfig {
    pub rescan_every: Duration,   // default 60 s
    pub debounce: Duration,       // default 500 ms
}
impl Default for CodeConfig { … }

// code/mod.rs
impl Code {
    /// Chooses the active set (§15.6) and starts a worker per active project.
    /// `AppCore::start` calls it with the default; a test calls it when it wants workers.
    pub async fn start(&self, config: CodeConfig) -> Result<(), CoreError>;
    /// The project was used: it becomes the most recent, and active. Returns at once.
    /// Before `start`, it only records the use.
    pub async fn touch(&self, project: &ProjectId);
    /// Stops every watcher and worker; a worker finishes the file it is on.
    pub(crate) async fn shut_down(&self);
}
```

- **Worker** (`watch.rs`): one `tokio` task per active project, owning:
  - an `mpsc::Receiver<Job>`;
  - its `notify::RecommendedWatcher` (a plain watcher, **not** `notify-debouncer-full`: §15.4 and PROBE.md);
  - a set of pending paths, and a `tokio::time::interval(rescan_every)`.

  `Job` has three kinds:
  - `Scan`, which runs Task 2's `scan`;
  - `Files(Vec<String>)`, which indexes those relative paths or deletes their rows;
  - `Recheck(Vec<String>, oneshot::Sender<()>)`, which indexes those paths if their size or modified time changed, deletes the rows of those that are gone, then replies.

  The watcher's handler runs on `notify`'s thread and does little:
  - it drops a path under `target/` or `node_modules/` first, cheaply (a build sends hundreds of such events);
  - it forwards the rest, relative, to the worker; a changed `.gitignore`, an error, or `need_rescan()` sends `Scan` instead.

  The worker gathers forwarded paths into its set, and when no path has arrived for `debounce`, filters them with `keeps` and the ignore files and sends itself `Files`. `Files` decides by what is on disk (§15.4), never by the event's kind: a save through rename arrives as `Remove` then `Create`. A missing file's rows are deleted only if the project's folder exists.

  The interval sends `Scan`. The scan first checks the folder exists: if not, the state becomes `DirectoryMissing` and the watcher is dropped. When a later periodic scan finds the folder again, it makes a **new** watcher (the old one stays dead after its folder is deleted, PROBE.md) and scans.
- **The state** of each active project lives in memory: `Indexing { done, found }` while a scan runs, and `Ready` after it. Answers read it. A project not in the set answers `Inactive`.
- **The re-check (§15.5):** before answering, `definitions`, `references` and `outline` compare the size and modified time of the hits' files with their rows. A file that is gone counts as changed. When any differ and the project has a worker, they send `Recheck`, await the reply for at most 2 s, and run the query again once. A project with no worker yet (just touched) answers from its rows, with its status `indexing`: the status says the answer may be stale.
- **The active set** (`active.rs`):
  - At `start`, the order is the newest `operation.created_at` among each project's threads, with `project.created_at` for a project with no operation. Only projects with a directory count. Find the columns in `0001`/`0002` and the thread table.
  - The first `active_limit` projects are active. That number is read from `code_setting`.
  - `touch(p)`: `p`'s direct links become most recent first, then `p` itself.
    - A project that becomes active gets a worker and a `Scan`.
    - Whoever falls past the limit loses its worker.
  - Every question touches the asker's project (§15.6: "a code question names it"), which brings its links in too. A project that was inactive answers `Indexing`.
  - **Before `start`,** `touch` only appends to the order. `start` builds the order from the database as above, then applies the touches recorded before it, oldest first.
- **Callers of `touch`:** `Turns::send` after its checks pass, `Threads::list`, and `Projects::create`. `touch` never fails the caller: it logs and returns.
- **How they hold `Code`:** `from_parts` builds `Code` before `Projects`, `Threads` and `Turns`, and passes each a clone (§15.8). `Code`'s contract lists the three under `called_by_other_services` for `touch`.

- [ ] **Step 1: Write the failing tests** in `code_index.rs`.

```rust
#[tokio::test]
async fn watching_and_the_periodic_scan_keep_the_index_true() {
    let (core, project, dir) = core_with_project().await;
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/a.rs"), "fn one() {}\n").unwrap();
    let fast = CodeConfig { rescan_every: Duration::from_millis(400), debounce: Duration::from_millis(100) };
    core.code().start(fast).await.unwrap();
    core.code().touch(&project).await;
    let me = Asker::Person(&project);
    eventually(|| async { !core.code().definitions(me, None, "one").await.unwrap().hits.is_empty() }).await;

    // An editor's save: write a temporary file, then rename it over the original.
    let tmp = dir.path().join("src/.a.rs.tmp");
    std::fs::write(&tmp, "\nfn one() {}\n").unwrap();
    std::fs::rename(&tmp, dir.path().join("src/a.rs")).unwrap();
    // The re-check answers the new line at once, without waiting for the watcher.
    assert_eq!(core.code().definitions(me, None, "one").await.unwrap().hits[0].line, 2);

    // A file the watcher never reports: the periodic scan finds it.
    core.code().pause_watcher_for_test(&project).await;
    std::fs::write(dir.path().join("src/b.rs"), "fn two() {}\n").unwrap();
    eventually(|| async { !core.code().definitions(me, None, "two").await.unwrap().hits.is_empty() }).await;
    core.code().shut_down_for_test().await;
}

#[tokio::test]
async fn only_the_most_recent_projects_are_watched() {
    let (core, projects) = core_with_projects(6).await; // each with a folder holding src/p<i>.rs: fn p<i>() {}
    core.code().start(CodeConfig::default()).await.unwrap();
    let active = core.code().active_for_test().await;
    assert_eq!(active.len(), 5);
    let sixth = projects.iter().find(|p| !active.contains(p)).unwrap();
    // Asking the sixth brings it in: its answer says it is indexing, not an error.
    let a = core.code().status(sixth).await.unwrap();
    assert!(matches!(a.state, IndexState::Inactive));
    let q = core.code().definitions(Asker::Person(sixth), None, "p0").await.unwrap();
    assert!(matches!(q.status[0].state, IndexState::Indexing { .. } | IndexState::Ready));
    let now = core.code().active_for_test().await;
    assert_eq!(now.len(), 5);
    assert!(now.contains(sixth));
    assert!(!now.contains(&active[active.len() - 1]), "the least recently used left");
    core.code().shut_down_for_test().await;
}
```

  - `eventually` polls every 50 ms for up to 10 s.
  - `pause_watcher_for_test`, `active_for_test` (projects in order of use, most recent first) and `shut_down_for_test` are exposed through `testing` only.
  - `pause_watcher_for_test` makes that project's worker drop every `Files` job until shutdown; the watcher and the interval keep running. It does **not** stop the watcher, because the periodic scan restarts a stopped one, and a restarted watcher could report `b.rs` itself: then the test would pass without proving the periodic scan.
  - `core_with_project` and `core_with_projects(n)` are fixtures in `code_index.rs`: a temporary database, and each project on its own `TempDir`. The six projects get distinct creation times and no turns, so the order is known.
- [ ] **Step 2: Run the tests to watch them fail.** Expected: they fail to compile.
- [ ] **Step 3: Implement `watch.rs`, `active.rs`, and the changes to `mod.rs`** as described above.
  - The watcher's callback runs on `notify`'s own thread. It sends with `blocking_send`, or with `try_send` into a bounded channel whose overflow sends `Scan`, never with an `.await`.
  - Log `code.scan` with the project, the files seen, the files indexed and the elapsed milliseconds.
- [ ] **Step 4: `AppCore`.**
  - `start` calls `core.code().start(CodeConfig::default())` after `from_parts`. It returns once the active set is chosen and the workers are spawned; no scan is awaited (§15.8: `serve` does not wait for indexing).
  - `lib.rs` re-exports `CodeConfig`.
  - `shut_down` calls `self.code.shut_down().await` before `turns.shut_down`, and leaves §8.5 as it is.
  - Add the calls to `touch` in `Turns::send`, `Threads::list` and `Projects::create`, and name them in those services' contracts and in `Code`'s.
- [ ] **Step 5: Run the tests until they pass, then the gate.** Update `code/contract.yaml` with the rules and the two tests, and with an `agreements` entry (TEMPLATE rule 8): `between: [scan, Files job, Recheck job]`, `must: index a file the same way`, `why: all three call index_file`; and one for the walk's filter and the watcher's (`keeps`). Add the README rows for `watch.rs` and `active.rs`, and regenerate the code map.
- [ ] **Step 6: Commit.**

```bash
git add crates/shadows-core docs/codebase
git commit -m "feat(M3): Code — workers, the watcher, the periodic scan and the active set (§15.4, §15.6)"
```

---

### Task 4: Links, the limit, the MCP tools and the routes

**Files:**
- Create: `crates/shadows-core/src/code/links.rs`: the links and the active limit, as commands.
- Create: `crates/shadows-http/src/code.rs`: the code routes.
- Modify: `code/mod.rs`, `code/store.rs`, `code/model.rs` (`ProjectLink`, `CodeSettings`), `code/contract.yaml`, `crates/shadows-mcp/src/tools.rs` (three tools), `crates/shadows-mcp/src/server.rs` (`PROJECT_TOOLS`), `crates/shadows-http/src/lib.rs` (the route table and `paths(…)`), `api/openapi.json` (regenerated).
- Test: `crates/shadows/tests/code_scope.rs`.

**Interfaces:**
- Consumes: Tasks 2–3.
- Produces:

```rust
// code/model.rs
pub struct ProjectLink { pub project: String /* slug */, pub linked: String /* slug */, pub created_at: String }
pub struct CodeSettings { pub active_limit: u32 }

// code/mod.rs
impl Code {
    pub async fn links(&self, project: &ProjectId) -> Result<Vec<ProjectLink>, CoreError>;
    /// "ProjectLinkPut", params { "project", "linked" }; scope "Project", key = project.
    pub async fn link(&self, command_id: String, project: &ProjectId, linked: &ProjectId) -> Result<ProjectLink, CoreError>;
    /// "ProjectLinkRemove", params { "project", "linked" }.
    pub async fn unlink(&self, command_id: String, project: &ProjectId, linked: &ProjectId) -> Result<(), CoreError>;
    pub async fn settings(&self) -> Result<CodeSettings, CoreError>;
    /// "CodeActiveLimitSet", params { "active_limit" }; scope "Code", key "settings".
    /// Outside 1..=20 is INVALID_COMMAND. Lowering it stops the least recent workers at once.
    pub async fn set_active_limit(&self, command_id: String, active_limit: u32) -> Result<CodeSettings, CoreError>;
}
```

- `lib.rs` re-exports `ProjectLink` and `CodeSettings`.
- **The commands** follow `instructions/store.rs::save_planner_instructions` exactly:
  - a `user_command(command_id, kind, params)`;
  - in one `write_txn`: `classify` for a replay, the change, `append_event`, and `record_command`.
  - The events are `ProjectLinked`, `ProjectUnlinked` and `CodeActiveLimitSet`. The linked project's id is in the payload.
  - A link to itself, or to an unknown project, is `INVALID_COMMAND`.
  - Linking twice answers the existing link.
  - Unlinking a link that does not exist is `INVALID_COMMAND` ("no such link").
- **MCP tools** (in `tools.rs`, then added to `PROJECT_TOOLS` only; `THREAD_TOOLS` is unchanged):

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NameArgs {
    /// The exact name: a function, method, type, trait, class, interface or constant.
    name: String,
    /// A linked project's slug, to ask only it. Leave it out to ask this project and every linked one.
    #[serde(default)]
    project: Option<String>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct OutlineArgs {
    /// A file or folder, relative to the project's folder. "" for the whole project.
    path: String,
    /// As for where_is.
    #[serde(default)]
    project: Option<String>,
}
```

  | Tool | Description | Calls |
  |---|---|---|
  | `where_is` | "Where a name is defined: project, file, line, kind and signature. Never the code; open the file if you need it." | `code().definitions(Asker::Grant(&grant), …)` |
  | `who_uses` | "Where a name is used, matched by name only: two things with the same name are not told apart." | `code().references(…)` |
  | `outline` | "The definitions in a file or folder: file, line, kind, name and signature." | `code().outline(…)` |

  - **The answer is text.** For each hit, one line: `{project}: {path}:{line} {kind} {name} — {signature}`. For `who_uses`, the line ends with `(matched by name)` instead of the signature.
  - Then `more results: narrow the name or the path` if `more` is set.
  - Then `did you mean: a, b, c` if there are suggestions.
  - Then one line per project status, e.g. `backend: indexing 340/1200`.
  - A refusal goes through the existing `Refusal::from(CoreError)`.
  - Put the text formatting in `tools.rs`, in a function `lines(&Answer, references: bool) -> String`. Formatting is the adapter's job (§14.5).
- **HTTP routes** in `code.rs`, following `instructions.rs`'s pattern, each with `#[utoipa::path]` and tag `code`:
  - `GET /api/projects/{id}/code/definitions?name=&project=`;
  - `GET /api/projects/{id}/code/references?name=&project=`;
  - `GET /api/projects/{id}/code/outline?path=&project=`;
  - `GET /api/projects/{id}/code/status`;
  - `GET /api/projects/{id}/code/links`;
  - `PUT /api/projects/{id}/code/links/{linked}`, with body `{ command_id }`;
  - `DELETE /api/projects/{id}/code/links/{linked}?command_id=`;
  - `GET /api/code/settings`;
  - `PUT /api/code/settings`, with body `{ command_id, active_limit }`.

  The question routes answer JSON `Answer`, and use `Asker::Person(&id)`.

- [ ] **Step 1: Write the failing test** `crates/shadows/tests/code_scope.rs`, with `fixtures/listening.rs` for a live daemon, and `project_client` for an external grant on its project.

```rust
#[tokio::test]
async fn the_scope_is_the_project_and_its_links_only() {
    let l = listening_app().await;                                  // project A: slug "demo", its folder the test's temp dir
    let a = l.app.project.clone();
    let (b, _dir_b) = other_project_with_folder(&l, "backend", "src/api.rs", "pub fn orders() {}\n").await;
    let (c, _dir_c) = other_project_with_folder(&l, "secret", "src/k.rs", "pub fn key() {}\n").await;
    let (_, client) = project_client(&l).await;
    l.app.core.code().start(CodeConfig::default()).await.unwrap();  // assemble does not start workers

    // Link A → B through the route; C stays unlinked.
    let (status, _) = put_json(&l, &format!("/api/projects/{a}/code/links/{b}"), json!({"command_id": fresh_command()})).await;
    assert_eq!(status, 200);
    settle_index(&l, &[&a, &b, &c]).await;                          // touches each, polls status until Ready

    let found = text(&client, "where_is", json!({"name": "orders"})).await;
    assert!(found.contains("backend: src/api.rs:1 function orders"), "{found}");
    let only_b = text(&client, "where_is", json!({"name": "orders", "project": "backend"})).await;
    assert!(only_b.contains("src/api.rs:1"), "{only_b}");

    // Unlinked: GRANT_SCOPE over MCP, INVALID_COMMAND over HTTP, and never its content.
    let refused_c = refused(&client, "where_is", json!({"name": "key", "project": "secret"})).await;
    assert!(refused_c.contains("GRANT_SCOPE"), "{refused_c}");
    assert!(!text(&client, "where_is", json!({"name": "key"})).await.contains("secret"));
    let (status, body) = get(&l, &format!("/api/projects/{a}/code/definitions?name=key&project=secret")).await;
    assert_eq!((status, body["code"].as_str()), (REFUSED, Some("INVALID_COMMAND")));

    // A path out of the folder is refused.
    let out = refused(&client, "outline", json!({"path": "../"})).await;
    assert!(out.contains("the path must be inside the project"), "{out}");

    // One way: B does not see A.
    let (status, body) = get(&l, &format!("/api/projects/{b}/code/definitions?name=orders&project=demo")).await;
    assert_eq!((status, body["code"].as_str()), (REFUSED, Some("INVALID_COMMAND")));
}
```

  - `Listening` has `app` and `base` only: the project is `l.app.project`, its slug `"demo"`, its core `l.app.core`.
  - Take `refused` from `listening.rs`, and `fresh_command` from `fixtures/app.rs`.
  - Write the small helpers `other_project_with_folder` (each project on its own `TempDir`, returned so it lives), `put_json`, `get`, `text` and `settle_index` in the test file, or in `fixtures/` if two files use them.
  - **Never** use `fixtures::other_project` in a code test: its folder is the system temp directory, and indexing it walks the whole of it.
  - `REFUSED` is a `const` in the test: the status `shadows-http/src/failure.rs` gives `CoreError::Refused` with `InvalidCommand` today. Read it; do not change it.
- [ ] **Step 2: Run the test to watch it fail.** Expected: `where_is` is an unknown tool, or the file does not compile.
- [ ] **Step 3: Implement** `links.rs`, the store queries, the tools, the routes and the route table.
  - Regenerate `api/openapi.json` the way `crates/shadows/tests/openapi.rs` says. Run that test to see how.
  - Then run `web`'s `gen:api` only if `web/` would otherwise fail its own type check. `web/` is not otherwise touched.
- [ ] **Step 4: Run the test until it passes.**
- [ ] **Step 5: Break it on purpose (the scope rule).**
  - In `scope::projects`, temporarily let `only` name any project.
  - Run `code_scope`. It must fail on the `secret` assertions.
  - Restore the code, run it again, and see it pass.
  - Say in the report which assertion failed.
- [ ] **Step 6: Contract, code map, gate.**
  - `code/contract.yaml` gains the link and limit methods, the scope obligations with `the_scope_is_the_project_and_its_links_only`, and the command kinds.
  - Add the README rows for `links.rs` and `shadows-http/src/code.rs`.
  - Regenerate the code map, and run the whole gate. Step 6 of the gate is replaced by: the `api/openapi.json` diff shows only the new paths and schemas.
- [ ] **Step 7: Commit.**

```bash
git add crates api docs/codebase
git commit -m "feat(M3): links, the active limit, the code MCP tools and routes (§15.6–§15.7)"
```

---

### Task 5: Documents

**Files:** the list in §15.12, and nothing else:
- `docs/superpowers/specs/2026-09-26-application-core-design.md`: §14.3's crate list and diagram, and §14.4's service table, with `code/`'s files and `Code` stopping first in the shutdown bullet;
- `docs/superpowers/specs/2026-09-25-planner-workflow-design.md`: §13.6's table, with three rows;
- `docs/superpowers/specs/2026-09-21-sqlite-schema-design.md`: §6, with the four tables of `0008`;
- `CLAUDE.md`: the crate, the diagram, "nine services", and the single-ownership table;
- `docs/codebase/README.md`: the invariant "`shadows-index` knows nothing of SQLite or projects";
- `docs/vision.md`: §2.4's **Today** line, §9 without the "Where is X?" question, and a new §9 question for framework links;
- `docs/status.md`: where M3 is, with the test count.

- [ ] **Step 1:** Make each change. Every amendment is in place, and refers to §15 rather than copying it (CLAUDE.md documentation rules).
- [ ] **Step 2:** Check that §15.2's file list of `code/` and §15.4's columns match what Tasks 2–4 built. Amend §15 in place where they do not.
- [ ] **Step 3:** Run the gate (the code map test reads the README).
- [ ] **Step 4: Commit.**

```bash
git add docs CLAUDE.md
git commit -m "docs(M3): the code index in §6, §13.6, §14, CLAUDE.md, the code map, vision and status"
```

---

### Task I: Finish (controller)

1. **One whole-branch review:** an opus reviewer, which fixes what it finds, against §15 and this plan's Review Focus.
2. **Mohammed's Windows run,** following §15.11's acceptance, steps 1–5. Build `shadows serve` from this branch first, and stop any old daemon.
3. **Evidence:** write `docs/evidence/milestone3/WINDOWS_RUN.md` from his run and the log. It records the first-index time and the `code.scan` lines.
4. **The PR** to `main`, pushed with `git -c http.version=HTTP/1.1 push`. Mohammed merges. Then delete the branch, and this plan's `.superpowers/sdd/` workspace.
