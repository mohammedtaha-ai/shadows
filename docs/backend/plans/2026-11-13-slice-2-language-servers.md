# Slice 2: language servers, the question card and Settings › Code — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` (B-n) or
> `frontend-builder` (F-n) from its entry in [stages.md](../stages.md) or
> [the web's stages.md](../../frontend/stages.md); the entry's Touch and Done when are
> binding, and so are this file's Interfaces and steps. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Shadows runs each active project's language servers and gives the Planner and the
executor their LSP operations through one tool, `code_lsp`; the Planner gets a shell to
install what is missing, under its program's own approval, which reaches the person as a
card; Shadows records every project file the Planner changes and asks it why when it did not
say; the person controls the servers in Settings › Code.

**Architecture:** A new library crate, `shadows-lsp`, talks to one server (start, initialize,
open a file, one request, shut down). `Code` owns the catalog, the person's server settings,
which servers a project runs, their lifecycle inside the project's existing worker, and the
`code_lsp` answer. Activity becomes "a message was sent in the project" with an idle stop.
`Turns`, which already depends on `Code` and `Harness` and owns the waiting notice, owns the
person's questions and the end-of-turn check of the Planner's changes. `Harness` holds the
Planner programs' permission requests for the person. The tree-sitter name index is suspended: its tools and
routes go, its code stays.

**Tech Stack:** Rust 1.94+, `async-lsp` (with its `lsp-types`) and `tokio-util` compat,
SQLx/SQLite, rmcp, axum/utoipa; React client.

**Spec:** [language servers design](../specs/2026-11-13-language-servers-design.md)
(approved by Mohammed, 2026-11-13).

## Global Constraints

- CLAUDE.md's rules: AppCore is the only way in; one build cache (no `CARGO_TARGET_DIR`, no
  worktree build, no `cargo clean`, no `--release`); 100 columns in `crates/**/*.rs`; a
  module's change updates its `contract.yaml` and the code map in the same commit; the full
  gate runs in CI only.
- Tests are real: a temporary folder, the installed `gopls` (and `rust-analyzer`), a command
  that does not exist for a missing server; live Planner tests are `#[ignore]` and need their
  key or Codex home. No new `fake-acp` or `fake-codex` behaviour.
- `async-lsp` is an infrastructure dependency; its commit says so. Spawn only through
  `shadows-process` (never `tokio::process` directly).
- The catalog is the twelve upstream definitions, copied with the Apache-2.0 notice; install
  commands are Windows only in this slice, `null` elsewhere.
- Migrations take the next free numbers from `0058`; renumber only an unshipped one.
- Order: B-138a, B-138, B-139, B-140, B-141, B-142, B-143, B-144, then F-115 and F-116, then B-145.

## Review Focus

1. **A server that never answers or dies mid-request** must not hang a Planner turn: every
   request has a time limit and a dead server reads `failed` with its last stderr line.
   Pinned by `a_server_that_exits_reads_failed` (B-139) and `a_slow_answer_times_out` (B-141).
2. **A path outside the project** (`../`, an absolute path, a link out) in `code_lsp`'s `file`
   is refused before a server sees it, and a location a server answers outside the project
   and its links is dropped. Pinned by `code_lsp_refuses_a_path_outside_the_project` (B-141).
3. **A Planner that changes a project file through its shell** (`sed`, a scaffold command)
   and says nothing: the change is recorded and the Planner is asked why. Pinned by
   `an_unnoted_shell_change_is_recorded_and_asked_about` (B-144).
4. **An npm-installed server on Windows** is a `.cmd` shim, not an `.exe`: it must be found and
   started. Pinned by `a_cmd_shim_on_path_is_found` (B-139).
5. **A question answered twice** (two tabs, a double click) records one answer and sends the
   Planner one message. Pinned by `a_question_is_answered_once` (B-143).

---

## Task B-138a: `core-code` becomes `codebase`

**Ruling (Mohammed, 2026-11-13):** the service that owns a project's code on disk (files,
watcher, activity, language servers, links, the suspended index) is named for what it is,
without `core-`. The other `core-*` crates keep their names until they are touched; a new or
renamed service crate takes its name without `core-`. Every later task in this plan reads
`crates/codebase/` and `Codebase` where it says `crates/core-code/` and `Code`.

**Files:**
- Move: `crates/core-code/` → `crates/codebase/` (`git mv`, so history follows).
- Modify: `crates/codebase/Cargo.toml` (`name = "codebase"`), every `Cargo.toml` that
  depends on `core-code`, `Cargo.lock`, every `use core_code::` and `core_code::` path,
  the type `Code` → `Codebase` and AppCore's accessor `code()` → `codebase()`
  (`crates/shadows-core/src/app.rs`), `CodeError` → `CodebaseError`, `CodeConfig` →
  `CodebaseConfig`; every `contract.yaml` that names them; `docs/codebase/README.md`;
  `CLAUDE.md` (the service-crate rule: "`core-<service>` crates, and a new or renamed
  service crate named without `core-`, such as `codebase`"; the layer diagram's line);
  this plan's later tasks and `docs/backend/stages.md` where they name the old paths.
- Modify: `crates/shadows/tests/architecture/scanner.rs` (where `service` is decided and the
  services are collected, lines 174-181 and 272) and `crates/shadows/tests/architecture/main.rs`
  (Codex's review, 2026-11-13): both tell a service crate by the
  `core-` prefix; a service crate becomes "starts with `core-` (except `core-db`), or is in
  `UNPREFIXED_SERVICES = ["codebase"]`", one helper used by every check that reads the prefix.
- Not renamed: database tables (`code_*`), migration files, MCP tool and HTTP route names,
  tracing event names (`code.*`), `IndexState` and the model types other than those above.

- [ ] **Step 1: Run** `cargo test -p shadows --test architecture --test codemap --test contract_tests`
  before the move; record the counts.
- [ ] **Step 2: Move and rename** as listed; `cargo check --workspace --all-targets --features fake-acp/test-support`.
- [ ] **Step 3: Run** the Step 1 command, `cargo test -p codebase`, `cargo test -p shadows-core
  --test code_index` (or the files holding Code's tests); expected: the same counts as Step 1,
  all passing. `rg -n "core-code|core_code"` outside `target/` and `docs/evidence/` finds only
  dated history (plans and stages entries of earlier slices are left as written).
- [ ] **Step 4: Commit** `refactor(backend): B-138a core-code becomes codebase`.

## Task B-138: the `shadows-lsp` client

**Files:**
- Create: `crates/shadows-lsp/Cargo.toml`, `crates/shadows-lsp/src/lib.rs`,
  `crates/shadows-lsp/src/client.rs`, `crates/shadows-lsp/tests/gopls.rs`
- Modify: `Cargo.toml` (workspace member, `async-lsp`, `lsp-types`, `tokio-util`),
  `Cargo.lock`, `docs/codebase/README.md` (the crate's row), `CLAUDE.md` (the crate table
  row: "`shadows-lsp` · One language server's process and its requests")

**Interfaces:**
- Produces:
  - `pub struct ServerSpec { pub executable: PathBuf, pub args: Vec<String>, pub root: PathBuf, pub env: Vec<(String, String)> }`
  - `pub struct Server` with
    `pub async fn start(spec: ServerSpec, timeout: Duration) -> Result<Server, LspError>`
    (spawn through `shadows_process::spawn` with piped stdio, `initialize` with the root's
    `file://` URI as the one workspace folder, then `initialized`);
    `pub async fn ask(&self, path: &Path, language_id: &str, text: String, method: &str, params: serde_json::Value, timeout: Duration) -> Result<serde_json::Value, LspError>`:
    `didOpen` the file with `text`, sends the request, then `didClose`, whatever the answer.
    No file stays open.
    `pub async fn files_changed(&self, changes: &[(PathBuf, FileChange)]) -> Result<(), LspError>`
    (`FileChange { Created, Changed, Deleted }`) sends `workspace/didChangeWatchedFiles`.
    **A server keeps its own copy of closed files** and reads them again only when told
    (Codex's trial on gopls v0.23.0, 2026-11-13: after `didOpen`/`didClose` the moved
    function still answered its old line; after `didChangeWatchedFiles`, the new one), so
    the caller tells it every change. `request(&self, method, params,
    timeout)` is the same with no file (for `workspace/symbol`);
    `pub fn capabilities(&self) -> &serde_json::Value`;
    `pub fn last_error_line(&self) -> Option<String>` (the last stderr line);
    `pub async fn shutdown(self)` (`shutdown`, `exit`, then the handle drops and kills).
  - `pub enum LspError { Spawn(String), Initialize(String), Timeout, Exited(Option<String>), Server { code: i64, message: String } }`.
- The crate depends on `shadows-process` only among Shadows crates; it knows no project.

- [ ] **Step 1: Write the failing tests** in `crates/shadows-lsp/tests/gopls.rs`, each
  `#[ignore = "needs gopls on PATH"]` and run in Done when:
  - `gopls_answers_a_definition`: a temp module (`go.mod`, `a.go` with `func Hello() string`,
    `b.go` calling `Hello()`), `start`, `ask(b.go, "go", <b.go's text>,
    "textDocument/definition", …)` at the call: the answer's URI ends with `a.go` and its line
    is `Hello`'s (zero-based).
  - `an_edit_on_disk_is_seen_by_the_next_question`: after one `ask` on `b.go`, `a.go` is
    rewritten on disk with `Hello` moved two lines down and `files_changed([(a.go, Changed)])`
    is sent; the next definition `ask` from `b.go` answers the new line.
  - `gopls_answers_references`: `textDocument/references` with `includeDeclaration: true` at
    `Hello` finds two locations.
  - `a_missing_command_is_a_spawn_error`: `ServerSpec` with `no-such-server-xyz` →
    `Err(LspError::Spawn(_))` (not ignored).
  - `shutdown_reaps_the_child`: after `shutdown`, the child's pid is not alive.
- [ ] **Step 2: Run** `cargo test -p shadows-lsp -- --include-ignored`; expected: the new
  tests fail (no crate code).
- [ ] **Step 3: Implement** `client.rs` with `async_lsp::MainLoop::new_client` and
  `tokio_util::compat` over `ProcessHandle::take_stdio`; stderr is read line by line, keeping
  the last.
- [ ] **Step 4: Run** the same command; expected: 5 passed.
- [ ] **Step 5: Commit** `feat(backend): B-138 the shadows-lsp client (async-lsp, infrastructure)`.

## Task B-139: the catalog and a project's servers

**Files:**
- Create: `crates/codebase/src/lsp/mod.rs` (the servers a project runs, their state),
  `crates/codebase/src/lsp/catalog.rs`, `crates/codebase/src/lsp/catalog.json`,
  `crates/codebase/src/lsp/LICENSE-claude-plugins-official` (Apache-2.0 text and source
  commit), `crates/core-db/migrations/0058_code_servers.sql`,
  `crates/codebase/src/store/servers.rs`
- Modify: `crates/codebase/Cargo.toml` (`shadows-lsp`), `crates/codebase/src/lib.rs`,
  `crates/codebase/src/model.rs`, `crates/codebase/src/watch.rs`,
  `crates/codebase/src/store/mod.rs`, `crates/codebase/contract.yaml`,
  `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/code_servers.rs`

**Interfaces:**
- `catalog.json`: an array of
  `{ "id", "command", "args", "extensions": { ".go": "go" }, "install": { "windows": "…" | null, "macos": null, "linux": null }, "source": "<plugin>@<commit>" }`,
  the twelve entries and Windows install lines of the spec's table (builder checks the two
  winget ids with `winget show <id>` and writes `null` for one that does not resolve).
  Ids are upstream's: Lua's is `lua`, its command `lua-language-server` (Mohammed,
  2026-11-13, review of B-139: the catalog matches upstream; the spec's table says so).
- Migration `0058_code_servers.sql`: `code_server_off(server_id TEXT PRIMARY KEY, at TEXT NOT NULL)`
  and `code_server_added(id TEXT PRIMARY KEY, command TEXT NOT NULL, args TEXT NOT NULL,
  extensions TEXT NOT NULL, at TEXT NOT NULL)` (JSON text for the lists). Rows are the
  person's, for every project.
- Model (`model.rs`):
  `pub enum ServerState { Idle, Starting, Running, NotInstalled, Off, Failed { error: Option<String> } }`;
  `pub struct ServerStatus { pub id: String, pub added: bool, pub wanted: bool, pub extensions: Vec<String>, pub state: ServerState, pub install: Option<String> }`
  (`install` is this system's line).
- `Codebase`:
  - `pub async fn servers(&self, asker: Asker<'_>) -> Result<Vec<ServerStatus>, CodebaseError>`:
    every catalog and added server, each with `wanted: bool` (the project's files need it).
    The state, in this order: `Off` when switched off; `NotInstalled` when its command is not
    found; for a wanted server in an active project, its worker's state (`Starting`,
    `Running`, `Failed`); otherwise `Idle`.
  - private `fn wanted(&self, extensions: &HashSet<String>, off: &HashSet<String>, added: &[Added]) -> Vec<Wanted>`:
    the rule "the files choose": a server is wanted when one of its extensions is in the
    project's inventory and it is not off. Extensions compare as written for `.C`/`.H` and
    lower-cased otherwise (the catalog's own keys).
  - Finding the command: PATH, and on Windows `PATHEXT` (`.exe`, `.cmd`, `.bat`); not found →
    `NotInstalled`.
- The worker (`watch.rs`) holds `HashMap<String, shadows_lsp::Server>`; after each scan and
  each watcher batch it recomputes `wanted` and starts what is newly wanted, stops what is not;
  **each watcher batch and each scan's changed paths go to every running server as
  `files_changed`** (the paths of that server's extensions); a `code_lsp` question is a
  worker job (`Job::Lsp`), so the changes queued before it reach the servers first
  (Codex's review, 2026-11-13);
  `Worker::stop` shuts every server down. A start failure is `Failed` with
  `last_error_line`, retried at the next scan.
- **Review fix (Mohammed, 2026-11-13): a scan's changed paths.** The index reads only the
  languages `shadows-index` knows (Go is not one), so a scan's changes come from its
  listing: `record_inventory` answers each listed file's stamp (`FileStamps`),
  `scan_project` answers them in `Scanned`, and the worker compares them with its last
  scan (`FileStamps::follow`); the paths added, changed or removed go to the running
  servers as a watcher batch's do, inside the worker, so a later `Job::Lsp` sees them.
  `pause_watcher_for_test` now drops every path the watcher reports (not only the index
  writes), so a test proves the scan. Test: `a_scanned_change_reaches_the_server`.

- [ ] **Step 1: Write the failing tests** in `code_servers.rs`:
  - `the_catalog_has_the_twelve_servers`: 12 ids; clangd's extensions hold `.C` and `.c`.
  - `a_go_file_wants_gopls` (ignored, needs gopls): a project folder with `main.go`, touched;
    within 60 s `servers` shows gopls `Running`, and no other server `Running`.
  - `a_new_extension_starts_its_server` (ignored, needs gopls): an empty folder touched, then
    `main.go` written; gopls becomes `Running` with no refresh call.
  - `a_watched_change_reaches_the_server` (ignored, needs gopls): `Hello` moved in `a.go`
    on disk with no question between; the next definition question answers the new line.
  - `a_missing_command_reads_not_installed`: an added server `no-such-server-xyz` for `.zz`,
    a `x.zz` file: its state is `NotInstalled`.
  - `a_server_that_exits_reads_failed`: an added server whose command exits at once
    (`cmd /c exit 3` on Windows, `sh -c 'exit 3'` elsewhere) reads `Failed`.
  - `a_cmd_shim_on_path_is_found` (Windows only): a temp dir on the spec's PATH holding
    `fake-ls.cmd`; the finder answers that path.
- [ ] **Step 2: Run** `cargo test -p shadows-core --test code_servers`; expected: fail.
- [ ] **Step 3: Implement** the files above.
- [ ] **Step 4: Run** the same with `-- --include-ignored`; expected: all pass.
- [ ] **Step 5: Commit** `feat(backend): B-139 the language-server catalog and a project's servers`.

## Task B-140: activity is a sent message, with an idle stop

**Files:**
- Modify: `crates/codebase/src/active.rs`, `crates/codebase/src/lib.rs`,
  `crates/codebase/src/links.rs`, `crates/codebase/src/model.rs`,
  `crates/codebase/contract.yaml`, `crates/core-threads/src/lib.rs` (`list` no longer
  touches), `crates/core-project-lifecycle/src/lib.rs` (create no longer touches),
  `crates/core-turns/src/` (the send path touches; the file the builder finds through
  `Turns::send`), `crates/core-turns/contract.yaml`, `crates/core-threads/contract.yaml`
- Modify: `crates/shadows-http/src/code.rs` (`/api/code/settings` loses `active_limit`),
  `api/openapi.json`
- Create: `crates/core-db/migrations/0059_code_active_limit_gone.sql` (drops the column or
  table the limit lives in)
- Test: `crates/shadows-core/tests/code_index.rs` (or the test file that holds today's
  active-set tests; their limit cases are replaced)

**Interfaces:**
- `Codebase::touch(&self, project)` keeps its signature; it now records `last_used = now` and
  starts the worker if none runs. A private sweep (every 30 s) stops a worker whose
  `last_used` is older than `IDLE = Duration::from_secs(10 * 60)` (the spec's "short";
  Mohammed may change it). A worker stopping keeps its rows (index state, inventory).
- Callers that touch: `Turns` when a message is sent (person's or a notice), and every
  `Codebase` question (`code_lsp`, inventory). Callers that stop touching: `Threads::list`,
  project create. A touch no longer activates linked projects.
- **Work in progress holds the project** (Codex's review, 2026-11-13): a long turn or an
  attempt may call no Code tool for more than the idle time. New
  `Codebase::hold(&self, project: &ProjectId) -> ActiveHold`; an `ActiveHold` keeps the project
  active while it lives, and its drop counts as a touch, so the idle time starts when the
  work ends. `Turns` holds one for each running turn (from its start until it settles);
  `Runs` holds one for each running attempt (Runs already has `Codebase`, B-128). Files add:
  `crates/core-runs/src/` (the file that starts and ends an attempt),
  `crates/core-runs/contract.yaml`.
- `CodeSettings`, `set_active_limit` and `LIMITS` go.

- [ ] **Step 1: Write the failing tests**:
  - `a_sent_message_activates_its_project`: send a message in a project's thread; the
    project's status is not `Inactive`.
  - `listing_threads_does_not_activate`: `threads().list` on an idle project; it stays
    `Inactive`.
  - `an_idle_project_stops` with a test-only idle of 1 s (`Codebase::idle_for_test`): touched,
    then after 3 s `Inactive`, and its gopls (if any) is gone.
  - `a_touch_does_not_activate_links`.
  - `a_held_project_outlives_the_idle_time`: idle 1 s, a `hold` kept for 3 s: still active;
    1 s after the hold drops plus the sweep, `Inactive`.
- [ ] **Step 2: Run** the test file; expected: fail.
- [ ] **Step 3: Implement**; remove every `active_limit` test.
- [ ] **Step 4: Run** it, `cargo test -p shadows --test openapi` with `UPDATE_OPENAPI=1`
  once, then without; expected: pass.
- [ ] **Step 5: Commit** `feat(backend): B-140 a project is active while a conversation runs`.

## Task B-141: `code_lsp`, and the name index suspended

**Files:**
- Create: `crates/codebase/src/lsp/ask.rs`, `crates/shadows/tests/code_lsp.rs`
- Modify: `crates/codebase/src/lib.rs`, `crates/codebase/src/model.rs`,
  `crates/codebase/src/error.rs`, `crates/codebase/src/scan.rs` (tags not extracted),
  `crates/codebase/contract.yaml`, `crates/shadows-mcp/src/code_tools.rs`,
  `crates/shadows-mcp/contract.yaml`, `crates/core-grants/src/tools.rs`,
  `crates/core-grants/contract.yaml`, `crates/shadows-http/src/code.rs` (the definitions,
  references and outline routes go), `api/openapi.json`,
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/src/executor.md`,
  `crates/core-harness/contract.yaml`, `docs/codebase/README.md`
- Modify (tests of the removed tools): `crates/shadows/tests/code_scope.rs`,
  `crates/shadows/tests/mcp_server.rs`, `crates/shadows/tests/mcp_tools.rs`,
  `crates/shadows-core/tests/executor_session.rs`

**Interfaces:**
- `pub enum LspOperation { GoToDefinition, FindReferences, Hover, DocumentSymbol, WorkspaceSymbol, GoToImplementation, IncomingCalls, OutgoingCalls }`
  (serde camelCase, as the spec names them).
- `pub struct LspQuestion { pub operation: LspOperation, pub file: Option<String>, pub line: Option<u32>, pub character: Option<u32>, pub query: Option<String>, pub project: Option<String> }`.
- `pub enum LspAnswer { Answered { server: String, result: serde_json::Value, truncated: bool }, NoServer { extension: String, server: Option<String>, state: Option<ServerState>, install: Option<String> } }`.
  Locations inside `result` are rewritten to `{ "project", "path", "line", "character" }`,
  one-based, project-relative; a location outside the project and its links is dropped.
- `Codebase::lsp(&self, asker: Asker<'_>, question: LspQuestion) -> Result<LspAnswer, CodebaseError>`:
  checks `file` with `Codebase::inside`, then **resolves its real path and refuses one outside
  the project's real folder** (a link or junction out), the same check the index makes
  before it reads (`scan.rs`, `names_itself` and the canonical folder; Codex's review,
  2026-11-13), and only then reads it; sends the file's current text with the request
  (`Server::ask`, B-138); `IncomingCalls`/`OutgoingCalls` do `textDocument/prepareCallHierarchy` first;
  `WorkspaceSymbol` asks every running server of the project and merges. Bounds: 100 items,
  32 KiB of answer; time limit 20 s per request. `CodebaseError::InvalidCommand` for a missing
  `file`/`line`/`character`/`query` the operation needs.
- MCP: `code_lsp` replaces `where_is`, `who_uses` and `outline`. It is in `THREAD_TOOLS`,
  `EXECUTOR_TOOLS` and `PROJECT_TOOLS` (my call: external agents keep a code tool).
  Description: "Ask the project's language server: goToDefinition, findReferences, hover,
  documentSymbol, workspaceSymbol, goToImplementation, incomingCalls, outgoingCalls. Lines
  and characters are one-based, as an editor shows them."
- Suspended index, **by one switch in AppCore** (Mohammed, 2026-11-13: no code commented
  out or removed): `CodebaseConfig` gains `pub name_index: bool`; `AppCore::start` passes
  `false`. `index_known` extracts tags only when `name_index` is true, and otherwise writes
  the file row with no tags (the inventory and the file rows stay). `Codebase::definitions`,
  `references`, `outline` and `shadows-index` stay as they are. Turning it back on is
  `name_index: true` and the three tools back in the grant lists. Test:
  `the_name_index_is_off_by_default` (a project with a `.rs` file scanned: its file row
  exists, it has no tags). Files add: `crates/shadows-core/src/app.rs` (the config line).
- **Review notes (Mohammed, 2026-11-13).** A server that fails a request is
  `CodebaseError::LanguageServer`, refused with its own code `LANGUAGE_SERVER_UNAVAILABLE`
  (not `HARNESS_UNAVAILABLE`); its text names the server and how it stands (running but did
  not answer in time, failed with its last error line, not startable). Left as built: a
  question waits up to 20 s for a server that is starting (`START_WAIT` in `lsp/ask.rs`),
  then answers `NoServer` with state `starting`. Tests ask through `Codebase::lsp`; the
  test-only `ask_server_for_test` is gone.
- `prompt.txt`: a short section naming `code_lsp`, and that a language with no running
  server is answered "no server" (B-144 adds the install lines). `executor.md:21` becomes
  "Ask `code_lsp` (definitions, references, symbols) before you search the tree yourself."

- [ ] **Step 1: Write the failing tests** in `crates/shadows/tests/code_lsp.rs`:
  - `code_lsp_finds_references_with_gopls` (ignored, needs gopls): the B-138 module; an
    executor-scoped call, `findReferences` at `a.go` `Hello` (one-based) answers both files.
  - `code_lsp_answers_no_server_for_an_unknown_extension`: `x.zz` → `NoServer`.
  - `code_lsp_refuses_a_path_outside_the_project`: `../x.go` and an absolute path →
    `invalid_command`.
  - `code_lsp_refuses_a_link_out_of_the_project`: `src/out` is a symlink (a junction on
    Windows) to a folder outside the project holding `x.go`; `code_lsp` on `src/out/x.go` is
    refused and the server is never asked.
  - `a_slow_answer_times_out`: an added server that never answers (a script that reads stdin
    and writes nothing) → an error whose text says the server did not answer in time, within
    25 s.
  - `the_removed_code_tools_are_listed_nowhere`: no grant kind lists `where_is`, `who_uses`,
    `outline`; all three list `code_lsp`.
- [ ] **Step 2: Run** `cargo test -p shadows --test code_lsp`; expected: fail.
- [ ] **Step 3: Implement**; delete or rewrite the tests of the removed tools in the files
  listed above.
- [ ] **Step 4: Run** `cargo test -p shadows --test code_lsp -- --include-ignored`,
  `--test mcp_tools --test mcp_server --test code_scope`, `cargo test -p shadows-core --test
  executor_session`, openapi as in B-140; expected: pass.
- [ ] **Step 5: Commit** `feat(backend): B-141 code_lsp, and the name index suspended`.

## Task B-142: the person's server settings over HTTP

**Files:**
- Modify: `crates/codebase/src/lsp/mod.rs`, `crates/codebase/src/store/servers.rs`,
  `crates/codebase/contract.yaml`, `crates/shadows-http/src/code.rs`, `api/openapi.json`
- Test: `crates/shadows/tests/code_servers_routes.rs`

**Interfaces:**
- `Codebase::set_server_off(&self, command: CommandId, id: &str, off: bool) -> Result<(), CodebaseError>`;
  `Codebase::add_server(&self, command: CommandId, server: AddedServer) -> Result<ServerStatus, CodebaseError>`
  (`AddedServer { id, command, args, extensions }`; an id that is a catalog id or already
  added is `InvalidCommand`; an extension must start with `.`);
  `Codebase::remove_server(&self, command: CommandId, id: &str) -> Result<(), CodebaseError>` (added
  ones only). Each is idempotent by command id as every mutating command is.
- Routes: `GET /api/projects/{id}/code/servers` → `Vec<ServerStatus>` (`Asker::Person`);
  `PUT /api/code/servers/{server}/off` `{ command_id, off }`;
  `POST /api/code/servers` `{ command_id, id, command, args, extensions }`;
  `DELETE /api/code/servers/{server}` `{ command_id }`.
  Switching off stops the server in every active worker at its next recompute; on starts it.
- **Review notes (Mohammed, 2026-11-13).** Left as built: `DELETE /api/code/servers/{server}`
  takes `command_id` in the query, as the link DELETE does, not in a body. The test-only
  `add_server_for_test` is gone: tests add a server with `add_server` and a command id.

- [ ] **Step 1: Failing tests**: `switching_a_server_off_lists_it_off`,
  `an_added_server_is_listed_and_removable`, `a_catalog_id_cannot_be_added`,
  `a_replayed_command_changes_nothing`.
- [ ] **Step 2–4: Run** `cargo test -p shadows --test code_servers_routes`: fail, implement, pass; openapi as above.
- [ ] **Step 5: Commit** `feat(backend): B-142 the person switches servers off and adds their own`.

## Task B-143: `person_ask`, the question card

**Files:**
- Create: `crates/core-turns/src/questions.rs`, `crates/core-turns/src/store/questions.rs`,
  `crates/core-db/migrations/0060_person_question.sql`,
  `crates/shadows-core/tests/person_questions.rs`
- Modify: `crates/core-turns/src/lib.rs`, `crates/core-turns/src/model.rs`,
  `crates/core-turns/contract.yaml`, `crates/core-threads/src/model.rs`
  (`ThreadEntryKind::PersonQuestion`, wire `PersonQuestion`), `crates/core-threads/contract.yaml`,
  `crates/shadows-mcp/src/turn_tools.rs`, `crates/shadows-mcp/contract.yaml`,
  `crates/core-grants/src/tools.rs` (`person_ask` in `THREAD_TOOLS` only),
  `crates/shadows-http/src/` (the threads routes file), `api/openapi.json`,
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`

**Interfaces:**
- Migration: `person_question(id TEXT PRIMARY KEY, thread_id TEXT NOT NULL, question TEXT NOT NULL,
  options TEXT NOT NULL, chosen INTEGER, asked_at TEXT NOT NULL, answered_at TEXT)`;
  `options` is JSON `[{ "label" }]`.
- `Turns::ask(&self, thread: &ThreadId, question: String, options: Vec<QuestionOption>) -> Result<QuestionId, TurnsError>`:
  2 to 5 options, labels 1 to 80 characters, question 1 to 500; appends a `PersonQuestion`
  entry whose card is `{ question_id, question, options, chosen: null }`.
- `Turns::answer(&self, command: CommandId, thread: &ThreadId, question: &QuestionId, option: usize) -> Result<Queued, TurnsError>`:
  **the click is the person's message** (Codex's review, 2026-11-13: a choice recorded but
  never delivered). In **one transaction** it records `chosen`, rewrites the question
  entry's card to `chosen: <option>` (through a Threads `*_in` method; add
  `Threads::set_card_in(&mut Tx, entry, card)` if none exists), and records the person's
  message `Answer to "<question>": <label>` exactly as a typed message is recorded on the
  same send path: started at once when the thread is idle, waiting when a turn runs. Only
  after the commit is a turn started; a start that fails leaves the recorded message, as a
  typed message's failed start does. A second answer with another command is `Conflict`; a
  replay answers the first result. It is the person's message, so it may approve (B-135).
  Files add: `crates/core-threads/src/lib.rs` and its store, if `set_card_in` is new.
- MCP `person_ask { question, options: [{ label }] }` → `{ question_id }`.
- **A fork copies the card as history** (Codex's review, 2026-11-13): the answer route refuses a
  question whose `thread_id` is not the thread asked (`InvalidCommand`), and a card a fork
  copied (its question's thread is not the entry's thread) is drawn with no buttons.
- HTTP: `POST /api/threads/{id}/questions/{question}/answer { command_id, option }`.
- Prompt: "When the person must choose, ask with `person_ask` and wait for the answer; do
  not ask the same thing in text."

- [ ] **Step 1: Failing tests** in `person_questions.rs`: `a_question_is_a_stored_entry`
  (survives reopening the core), `an_answer_reaches_the_planner_as_the_persons_message`
  (thread idle, and thread with a turn running), `a_question_is_answered_once`,
  `an_answered_card_reads_chosen_after_a_reload` (the entries read again from a new core show
  `chosen`), `a_forked_question_cannot_be_answered_in_the_fork`, `an_executor_cannot_ask`.
- [ ] **Step 2–4: Run** `cargo test -p shadows-core --test person_questions`, then
  `cargo test -p shadows --test mcp_tools --test planner_mcp`: fail, implement, pass.
- [ ] **Step 5: Commit** `feat(backend): B-143 person_ask, the question card`.

## Task B-144: the Planner's shell, its permission cards, and the end-of-turn check

**Ruling (Mohammed, 2026-11-13):** the Planner installs what is missing through its own shell,
under its program's approval (Claude Code `auto`, Codex "approve for me"); the program's
questions reach the person; Shadows records the Planner's file changes and asks about the
unexplained ones. This replaces `code_server_install` and its consent.

**Files:**
- Modify: `crates/core-harness/src/setup.rs` (`Bash` in `PLANNER_TOOLS`; the Planner's Claude
  Code mode `auto`), `crates/shadows-agent/src/codex/session.rs` (the Planner's
  `sandbox: workspaceWrite`, `approvalPolicy: onRequest`, and the check of what Codex
  accepted), `crates/shadows-agent/src/codex/connection.rs` (`requestApproval` held with its
  options and answered by the person, not declined), `crates/shadows-agent/src/acp/permissions.rs`
  (only if Planner sessions do not already hold), `crates/core-harness/src/permissions.rs`
  (`waiting_permissions` lists Planner sessions too), `crates/core-harness/contract.yaml`,
  `crates/shadows-agent/contract.yaml`
- Create: `crates/core-turns/src/changes.rs`, `crates/shadows-core/tests/planner_changes.rs`,
  `crates/shadows-core/tests/planner_permissions.rs`
- Modify: `crates/codebase/src/active.rs` (`ActiveHold::changed(&self) -> Vec<String>`: the
  paths the watcher saw change while it lived, `.git/` left out), `crates/codebase/contract.yaml`,
  `crates/core-turns/src/` (the file where a turn settles), `crates/core-turns/src/model.rs`
  (`Notice.executor: Option<ThreadId>`; `None` is Shadows, actor kind `Shadows`),
  `crates/core-turns/contract.yaml`, `crates/core-threads/src/model.rs`
  (`ThreadEntryKind::ProjectChange`), `crates/core-threads/contract.yaml`,
  `crates/shadows-mcp/src/turn_tools.rs` (`project_change_note`), `crates/shadows-mcp/contract.yaml`,
  `crates/core-grants/src/tools.rs` (`project_change_note` in `THREAD_TOOLS`),
  `crates/core-harness/src/prompt.txt`, `api/openapi.json`

**Interfaces:**
- `project_change_note { paths: [String], reason: String }` records a `ProjectChange` entry
  `{ paths, reason, noted: true }` in the Planner's thread; paths are checked as `code_lsp`'s
  `file` is (inside the project, by real path).
- When a Planner turn settles, Turns takes its hold's `changed()` paths, removes those the
  turn noted, and for the rest (if any), in one transaction, records a `ProjectChange` entry
  `{ paths, reason: null, noted: false }` and queues a notice (`executor: None`, card
  `{ kind: "project_change", paths }`, text "You changed these project files in your last
  turn without project_change_note: <paths>. Say why with project_change_note."). A turn
  that notice starts is checked the same way; at most one notice per turn.
- A Planner session's permission requests wait for the person exactly as AYLUL's do: listed
  by the existing waiting list and answered through
  `POST /api/threads/{id}/session/permissions/{request_id}`; no second list route. A Codex
  `requestApproval` maps to the same held request, and its answer back to
  `{ decision: "accept" | "decline" }`.
- Prompt: "Your shell is for installing tools and preparing the environment (a language
  server, a container, a version check). Executors change the project's files through
  commits; if you must change one yourself, call project_change_note with the paths and why.
  When code_lsp says a language's server is not installed, ask the person with person_ask
  whether you install it, they do, or it is left; on the first, run its install line." The
  Setup section's "you write no file" line is amended to match.

- [ ] **Step 1: Failing tests**:
  - `planner_changes.rs`: `a_noted_change_is_recorded_with_its_reason`,
    `an_unnoted_shell_change_is_recorded_and_asked_about` (a file written in the folder during
    a Planner turn, no note: one `ProjectChange` entry with `noted: false`, one Shadows notice
    naming it), `a_turn_with_no_change_sends_nothing`.
  - `planner_permissions.rs` (ignored, live programs): `a_planner_permission_request_waits_for_the_person`
    (Claude Code: a Planner asked to run a command its program asks about; the request is
    listed and the person's allow lets it run), `codex_approval_is_held_not_declined`.
  - `mcp_tools`: `the_planner_has_bash_and_project_change_note`.
- [ ] **Step 2–4: Run** `cargo test -p shadows-core --test planner_changes --test planner_permissions`
  (`-- --include-ignored` where the live programs are set), `cargo test -p shadows --test
  mcp_tools --test planner_mcp`, `cargo test -p shadows-core --test codex_session`: fail,
  implement, pass.
- [ ] **Step 5: Commit** `feat(backend): B-144 the Planner's shell, its permission cards and its changes`.

## Task F-115: the question card, the Planner's permission cards, and Shadows' notices

**Files:** `web/src/app/conversation/person-question-card.tsx` (+ test),
`web/src/app/conversation/entry.tsx`, `web/src/app/conversation/notice-item.tsx` (+ test),
`web/src/api/questions.ts` (+ test), `web/src/api/schema.d.ts` (generated),
`docs/frontend/stages.md`, `docs/frontend/codebase/README.md` (if its map asks).

- A `PersonQuestion` entry draws the question and one button per option, styled like the
  permission card. A click posts the answer with a new command id; the card then
  shows the chosen option and no buttons. An answered card (from the entry's card or the
  route's conflict) never shows buttons again.
- A card copied by a fork (its question's thread is not the conversation's) shows its
  question and options with no buttons.
- A Planner conversation draws its program's permission requests with the permission card
  that exists (`permission-card.tsx`), as an AYLUL conversation does.
- A `ProjectChange` entry draws its paths and its reason, or "no reason given".
- A notice whose `executor` is null is drawn in Shadows' name ("Shadows").
- Tests: `renders_the_planners_options`, `a_click_answers_once`,
  `an_answered_question_has_no_buttons`, `a_forked_question_has_no_buttons`,
  `a_planner_permission_request_shows_its_card`, `a_project_change_shows_its_reason`,
  `a_shadows_notice_names_shadows`.
- Done when: `npm test -- person-question notice-item questions`, `npm run build`, lint.
  One commit, `feat(web): F-115 the question card and Shadows' notices`.

## Task F-116: Settings › Code

**Files:** `web/src/app/settings/sections.ts`, `web/src/app/settings/settings-dialog.tsx`
(the section's entry in `CONTENT`, which draws it; Codex's review, 2026-11-13),
`web/src/app/settings/settings-dialog.test.tsx`, `web/src/app/settings/sections/code.tsx`
(+ test), `web/src/api/code.ts` (+ test), `web/src/app/settings/general-preferences.tsx`
and `web/src/app/settings/active-projects.tsx` (the limit goes), `docs/frontend/stages.md`.

- A **Code** section in the AI group, after Plugins & Skills. It lists the open project's
  servers (`GET /api/projects/{id}/code/servers`): id, extensions, state, and for
  `NotInstalled` its install line with a copy button. Each has a switch; switching one off
  first shows "Shadows' answers about this language get worse while it is off." with Cancel
  and Switch off. Added servers have Remove.
- "Add a server": id, command, arguments, extensions (comma-separated, each starting with
  `.`), with the line "A server you add runs on your machine on your responsibility."
- General loses "Active projects".
- Tests: `lists_the_servers_with_their_state`, `switching_off_warns_first`,
  `adds_and_removes_a_server`, `general_has_no_active_projects`, and in
  `settings-dialog.test.tsx` `opening_code_shows_its_servers` (the dialog opened on
  `settings=code` draws the section's content, not an empty panel).
- Done when: `npm test -- settings code`, `npm run build`, lint. One commit,
  `feat(web): F-116 Settings › Code`.

## Task B-145: live acceptance

**Files:** Create `crates/shadows/tests/language_servers_live.rs`; modify `docs/backend/stages.md`.

- `live_a_planner_asks_gopls` (`#[ignore]`, needs gopls and the Claude Code subscription,
  as `live_the_planner_sets_up_an_empty_folder`): a Go module with two files; the Planner is
  asked "Which files call Hello? Use code_lsp." The turn's tool entries hold a `code_lsp`
  call, and its reply names both files.
- The same against Codex (`SHADOWS_CODEX_PATH`, `SHADOWS_CODEX_HOME`), as
  `live_a_codex_turn_sees_an_image`.
- Browser: on `testing3`, a `.ts` file added; the Planner asks with `person_ask`; on "you
  install it" it runs the install line in its shell (with a permission card if its program
  asks); the server starts and `code_lsp` answers from it.
- Browser acceptance on `testing3` (Mohammed): the spec's Tests section, recorded in this
  entry.
- Commit `test(backend): B-145 the Planner asks a real language server`.
