# Milestone 3, Task 0 — the probe

- **Date:** 2026-09-30
- **Source:** `crates/probe-index`, committed at `11c01a6` and deleted in the
  commit after it. Run it again with `git show 11c01a6:crates/probe-index/src/main.rs`.
- **Machine:** Mohammed's, Windows 11 Pro 10.0.26200, 12 logical CPUs, rustc
  1.98.1. Windows Defender on: `RealTimeProtectionEnabled`,
  `OnAccessProtectionEnabled` both `True`; nothing changed for the probe.
- **Answers §15.10's Task 0.** Facts only. Where one contradicts §15, the
  controller amends §15.

## Summary: one line per question

| Question | §15 assumed | Measured | Task 1 / Task 3 must |
|---|---|---|---|
| Versions agree | tree-sitter-tags 0.27 with the grammar crates | `tree-sitter` 0.27.0, `tree-sitter-tags` 0.27.0, `tree-sitter-rust` 0.24.2, `tree-sitter-typescript` 0.23.2, `tree-sitter-javascript` 0.25.0, `notify` 8.2.0, `notify-debouncer-full` 0.7.0, `ignore` 0.4.33. One `tree-sitter` in the graph. | Task 1: pin these. |
| ABI | each grammar loads under the tags' `tree-sitter` | `tree-sitter` 0.27 accepts ABI 13–15. Rust 15, TypeScript 14, TSX 14, JavaScript 15. `TagsConfiguration::new` and `generate_tags` succeed for all four: no `LanguageError`. | Nothing. |
| Build cost | "how much each adds" | Clean debug build of the probe on 12 cores: 17.4–18.8 s whatever grammars are in it — no grammar is visible above the noise. Serial (`-j1`): 59.5 s with all, 54.4 s with none, so all three grammars together cost about **5 s of CPU**; one at a time is inside the ±2 s noise. Each grammar is a C build script: TypeScript (two grammars) 6.3 s, Rust 3.1 s, JavaScript 3.1 s, run in parallel with the Rust crates. | Nothing. Cost is not a reason to drop a language. |
| Rust `const`, `static` | missing from the query | Missing. Caught by the extra patterns below, kind `constant`. Associated `const` in an `impl` is caught too. | Task 1: append `RUST_EXTRA`. |
| Rust calls through a path | missing | Missing: `Storage::open(p)` gives no tag. Also missing: turbofish calls (`parse::<u32>(x)`, `x.collect::<Vec<_>>()`) and path macros (`tracing::info!`). All caught below, kind `call`. | Task 1: append `RUST_EXTRA`. |
| Rust uses of a type | missing | Missing: the grammar tags a type only in `impl X`. Caught below, kind `type`, role Reference: every `type_identifier` (fields, parameters, generics, returns, struct literals) plus a capitalised path head (`Storage` in `Storage::open`). `app.rs`: 91 type references. | Task 1: append `RUST_EXTRA`. |
| Rust trait method without a body | not listed | **A further gap:** `fn open(path: &Path) -> Self;` in a trait gives no tag (only trait methods with a body do). Caught below, kind `method`. | Task 1: append `RUST_EXTRA`. |
| TS arrow-function consts | missing | **Not missing.** JavaScript's query, appended to TypeScript's, already tags `export const f = () => …` and `export const g = async (…) => …` as `function`, and `const h = function () {}` too. | Nothing. |
| TS type aliases | missing | Missing: `export type Id = string` gives no tag; `client.ts` has 26 of them. Caught below, kind `type`. | Task 1: append `TS_EXTRA`. |
| TS and JS further gaps | not listed | Missing: `enum` declarations; top-level non-function consts (`export const LIMIT = 5`, `const DAEMON_URL = …`); type references inside generics (`Promise<Props>`): TypeScript's query tags a type only as the direct child of an annotation (3 references in `client.ts`, 81 with the extra pattern). Caught below. | Task 1: append `TS_EXTRA` and `JS_EXTRA`. |
| One tag per name | earlier pattern wins | Confirmed in `tree-sitter-tags` (`tags.rs`: one tag per name range, the lower pattern index replaces a higher one) and in the output: a `fn` in an `impl` is `method`; `export const f = () => …` stays `function`, not `constant`; `impl Storage` stays `implementation`, not `type`. So the extra patterns go **after** the grammar's query. | Task 1: append, never prepend. |
| Save through temp file + rename | debounced 500 ms | One save through `lib.rs.tmp` → `lib.rs` arrives as `Remove lib.rs`, `Create lib.rs`, `Modify src` — the temporary name never appears. | Task 3: decide by what is on disk after the batch, never by the event kind. |
| `cargo build` into `target/` | may overflow silently | No overflow at this size. A crate with no dependencies: 25 events, 22 under `target/`. With `ignore` (12 crates, 172 files): 360 events, 357 under `target/`. No error, no `need_rescan()`. Every one arrives at the handler and must be filtered out there. | Task 3: filter `target/` and `node_modules/` first, cheaply. |
| Mass change: 20 000 creations | — | All 20 002 events arrive, through the debouncer. | — |
| Mass change: 20 000 deletions | overflow passes silently | **Through `notify-debouncer-full`, 98.7 % of them are lost silently:** 203–361 events of 20 000 (four runs), no error, no `need_rescan()`. With `NoCache` instead of Windows' default `FileIdMap`: 5 393–5 631, still silent. **A plain `notify::RecommendedWatcher` gets all 20 000** (three runs), and a write afterwards arrives. The loss is the debouncer's: each `Remove` runs `FileIdMap::remove_path` (`paths.retain`, the whole cache) and `queues.retain` (every pending path), so 20 000 removals are quadratic, the handler falls behind, and the kernel buffer overflows — which `notify` 8.2.0 does not report. | **Contradicts §15.4's choice of `notify-debouncer-full`.** Task 3: use a plain `RecommendedWatcher` and gather its paths for ~500 ms itself (a set of paths, flushed when quiet); the periodic scan stays. |
| Default cache's startup cost | — | `watch()` with the default `FileIdMap` walks the whole folder and opens every file for its ID: 0.86–0.89 s for 20 000 files, and it walks `target/` and `node_modules/` too (the Shadows repo has 4 295 and 30 536 files there). With `NoCache`: 0.7 ms. | Another reason for the plain watcher. |
| Rename the watched folder | "can it still be renamed?" | **Yes**, the rename succeeds (`notify` opens the folder with `FILE_SHARE_DELETE`). The watcher reports **nothing** for it. It then keeps reporting changes inside the moved folder **under the old path** (`watched\src\after-move.rs`, a path that no longer exists). | Task 3: an event path that does not exist is not proof of a deletion if the root is gone; the periodic scan must check the root exists and mark the project `directory missing`. |
| Delete the watched folder | stops silently | Delete succeeds; 14 `Remove` events for its ~190 entries (172 of them in `krate/target/`), no error, no `need_rescan()`. A folder recreated at the old path afterwards gives **no** events: the watcher is dead and says nothing. | Task 3: as §15.4 says, the periodic scan restarts a watcher; it must recreate it when the root returns. |
| Periodic scan cost | "one directory walk" every 60 s | §15.4's settings over this repository: **340 files seen, 262 in the language table; median 8.5 ms, worst 15.8 ms** (10 runs; a second 10: median 8.6 ms, worst 9.4 ms). | The 60 s stands; the scan is cheap. Task 2: note `WalkBuilder` also skips hidden entries by default (`.github/`, `.superpowers/`, `.claude/`), which §15.4 does not state. |

## The extra query patterns (Task 1 appends these, verified below)

Rust, after `tree_sitter_rust::TAGS_QUERY`:

```scheme
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
```

TypeScript and TSX, after `format!("{}\n{}", typescript::TAGS_QUERY, javascript::TAGS_QUERY)`,
`TS_EXTRA` then `JS_EXTRA`:

```scheme
; TS_EXTRA
(type_alias_declaration name: (type_identifier) @name) @definition.type
(enum_declaration name: (identifier) @name) @definition.enum
((type_identifier) @name @reference.type)
```

JavaScript, after `tree_sitter_javascript::TAGS_QUERY`, and TypeScript/TSX after `TS_EXTRA`:

```scheme
; JS_EXTRA: top-level consts only, never a local inside a function
(program (lexical_declaration (variable_declarator name: (identifier) @name) @definition.constant))
(program (export_statement (lexical_declaration (variable_declarator name: (identifier) @name) @definition.constant)))
```

`#match?`/`#not-match?` are applied: the JavaScript grammar's own query already
relies on `#not-match?`, and `_` and `Self` are absent from the output.

## Kind names the queries produce

| Construct | kind | role |
|---|---|---|
| Rust `struct` (and `enum`, `union`, `type` alias) | `class` | Definition |
| Rust `trait` | `interface` | Definition |
| Rust `fn` in an `impl` (or a trait) | `method` | Definition |
| Rust free `fn` | `function` | Definition |
| Rust `const` | `constant` (extra) | Definition |
| Rust `static` | `constant` (extra) | Definition |
| Rust `mod` / `macro_rules!` | `module` / `macro` | Definition |
| Rust `impl X` / `impl T for X` | `implementation` (names `X` / `T`) | Reference |
| Rust call, method call, macro call | `call` | Reference |
| Rust type use | `type` (extra) | Reference |
| TS `interface` | `interface` | Definition |
| TS `type` alias | `type` (extra) | Definition |
| TS `enum` | `enum` (extra) | Definition |
| TS/JS arrow-function const | `function` | Definition |
| TS/JS top-level other const | `constant` (extra) | Definition |
| TS/JS `class`, `abstract class` | `class` | Definition |
| TS/JS method | `method` | Definition |
| TS type use | `type` | Reference |
| TS/JS `new X()` | `class` | Reference |

§15.3's kind list has no `implementation`, `call` or `enum`; they are what the
queries produce. Note `class` means any Rust ADT, and `type` is both a TS alias
definition and a type reference (the role tells them apart).

## Raw output, trimmed

### Tags

```text
tree-sitter LANGUAGE_VERSION 15 MIN_COMPATIBLE 13
rust ABI 15 / typescript ABI 14 tsx ABI 14 / javascript ABI 15

### rust app.rs (grammar query)
-- parse error: false; counts: {"call/ref": 68, "class/def": 4, "function/def": 3, "implementation/ref": 1, "method/def": 11}
### rust app.rs (with extra)
-- parse error: false; counts: {"call/ref": 100, "class/def": 4, "constant/def": 1, "function/def": 3, "implementation/ref": 1, "method/def": 11, "type/ref": 91}
  (definitions: Bus class 33, CoreParts class 40, AppCore class 54, StartConfig class 68,
   user_command function 83, start method 103, from_parts method 150, plans…events method 181–209,
   shut_down method 216, harness_version function 233, VERSION_BOUND constant 274,
   adapter_version function 279; line 104: "Storage type ref", "open call ref" — absent without the extra)

### rust sample (grammar query)          ### rust sample (with extra)
Store interface def 4                    LIMIT constant def 2
close method def 4                       NAME constant def 3
Storage class def 5                      Store interface def 4
Storage implementation ref 6             open method def 4        <- trait fn without body
open method def 6                        Path type ref 4
f function def 7                         close method def 4
helper call ref 14                       Storage class def 5
vec call ref 15                          Storage implementation ref 6
                                         MAX constant def 6        <- associated const
                                         open method def 6
                                         Storage type ref 8        <- Storage::open(p)
                                         open call ref 8
                                         read call ref 9           <- std::fs::read(p)
                                         parse call ref 10         <- parse::<u32>(x)
                                         new call ref 11           <- Vec::<Storage>::new()
                                         collect call ref 12       <- x.collect::<Vec<_>>()
                                         info call ref 13          <- tracing::info!
                                         helper call ref 14
                                         vec call ref 15

### ts client.ts (grammar query)
-- counts: {"call/ref": 57, "class/ref": 1, "function/def": 27, "type/ref": 3}
### ts client.ts (with extra)
-- counts: {"call/ref": 57, "class/ref": 1, "constant/def": 2, "function/def": 27, "type/def": 26, "type/ref": 81}
  (Schemas type def 12, Project type def 13 … IssuedGrant type def 37, DAEMON_URL constant def 40, client constant def 48)

### ts sample (with extra)
Props interface def 2 / Id type def 3 / Local type def 4 / f function def 5 / g function def 6 /
Promise type ref 6 / h function def 7 / LIMIT constant def 8 / k function def 9 / Color enum def 10 /
C class def 11 / m method def 11 / z function def 12 (its local `inner` untagged) / A class def 13 / n method def 13
(without the extra: no Id, Local, LIMIT, Color, Promise)

### tsx copy-button.tsx: grammar query {"call/ref": 9, "function/def": 2};
    with extra adds COPIED_MS constant def 9, ComponentProps type ref 16

### js sample: with extra adds only LIMIT constant def 6; f, g (arrow/function consts),
    k, C, m, module.exports.x are tagged by the grammar's query already
```

### Build (clean, `CARGO_INCREMENTAL=0`, debug, fresh target dir each)

```text
-j12: all 17.4 s | no rust 18.4 | no typescript 18.8 | no javascript 18.1 | no grammar 17.5 | all again 18.7
-j1:  all 59.4 s | no rust 61.5 | no typescript 54.9 | no javascript 56.7 | no grammar 54.4 | all again 59.6
--timings (-j12), build-script run: tree-sitter-typescript 6.28 s, tree-sitter-rust 3.12 s,
  tree-sitter-javascript 3.12 s, tree-sitter 1.94 s; each grammar's rlib ≤ 1.0 s
```

### notify (500 ms debouncer unless stated)

```text
### 1. save through temp file + rename
  Remove(Any) ["\watched\src\lib.rs"]
  Create(Any) ["\watched\src\lib.rs"]
  Modify(Any) ["\watched\src"]
### 2. cargo build, no dependencies — 0.43 s; 15 files in target/
-- events 25, under krate/target 22, need_rescan 0, errors []
### 2. cargo build, with `ignore` (offline, 12 crates) — 7.9 s; 172 files in target/
-- events 360, under krate/target 357, need_rescan 0, errors []  (261 Create, 50 Modify, 49 Remove)
### 2b. 20000 file creations in one folder
-- events 20002, need_rescan 0, errors []; a write afterwards arrives
### 2c. 20000 files deleted one by one (the folder stays)
-- events 247 / 269 (two runs), all Remove, need_rescan 0, errors []; a write afterwards arrives
### 2d. plain notify::RecommendedWatcher, 20000 removals
-- events 20000, need_rescan 0, errors 0; a write afterwards arrives   (three runs, identical)
### 2e. debouncer, 20000 removals
FileIdMap (default): watch() 0.86 s / 0.89 s; events 203 / 361
NoCache:             watch() 0.7 ms;          events 5393 / 5631   (need_rescan 0, errors 0 in all)
### 3. rename the watched folder
rename succeeded; events 0
-- a write inside the renamed folder:
  Modify(Any) ["\watched\src"]
  Create(Any) ["\watched\src\after-move.rs"]     <- the old path
### 3b. delete the (renamed) watched folder
delete succeeded; events 14 (Remove), need_rescan 0, errors []
-- the folder recreated at its first path, a file written: events 0
```

### Periodic scan

```text
WalkBuilder: parents(false) git_global(false) require_git(false) follow_links(false),
filter_entry drops target/ and node_modules/ dirs, metadata() (len, modified) per file
run: 340 files (262 in the language table), 3306401 bytes, 15.77 ms   <- first run
     … 8.09, 8.25, 8.47, 8.94, 8.52, 8.43, 8.28, 8.49, 8.75 ms
files 340, kept 262; median 8.49 ms, worst 15.77 ms, best 8.09 ms
second set of 10: median 8.58 ms, worst 9.44 ms, best 8.03 ms
```
