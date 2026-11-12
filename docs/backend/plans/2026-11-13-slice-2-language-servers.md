# Slice 2: language servers, the question card and Settings › Code — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` (B-n) or
> `frontend-builder` (F-n) from its entry in [stages.md](../stages.md) or
> [the web's stages.md](../../frontend/stages.md); the entry's Touch and Done when are
> binding, and so are this file's Interfaces and steps. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Shadows runs each active project's language servers and gives the Planner and the
executor their LSP operations through one tool, `code_lsp`; a missing server is installed
only after the person chooses it on a question card the Planner writes; the person controls
the servers in Settings › Code.

**Architecture:** A new library crate, `shadows-lsp`, talks to one server (start, initialize,
open a file, one request, shut down). `Code` owns the catalog, the person's server settings,
which servers a project runs, their lifecycle inside the project's existing worker, and the
`code_lsp` answer. Activity becomes "a message was sent in the project" with an idle stop.
`Turns`, which already depends on `Code` and `Harness` and owns the waiting notice, owns the
person's questions and the install. The tree-sitter name index is suspended: its tools and
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
- Order: B-138, B-139, B-140, B-141, B-142, B-143, B-144, then F-115 and F-116, then B-145.

## Review Focus

1. **A server that never answers or dies mid-request** must not hang a Planner turn: every
   request has a time limit and a dead server reads `failed` with its last stderr line.
   Pinned by `a_server_that_exits_reads_failed` (B-139) and `a_slow_answer_times_out` (B-141).
2. **A path outside the project** (`../`, an absolute path, a link out) in `code_lsp`'s `file`
   is refused before a server sees it, and a location a server answers outside the project
   and its links is dropped. Pinned by `code_lsp_refuses_a_path_outside_the_project` (B-141).
3. **A model calling `code_server_install` without the person's choice**, or twice on one
   choice, or for another server than the one chosen, is refused. Pinned by
   `install_needs_the_persons_choice_for_that_server` (B-144).
4. **An npm-installed server on Windows** is a `.cmd` shim, not an `.exe`: it must be found and
   started. Pinned by `a_cmd_shim_on_path_is_found` (B-139).
5. **A question answered twice** (two tabs, a double click) records one answer and sends the
   Planner one message. Pinned by `a_question_is_answered_once` (B-143).

---

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
    **No file stays open** (Codex's review, 2026-11-13): the server reads every file it was not
    handed from disk, so an executor's change is never answered from an old copy, and there
    is no open state to keep in step with the watcher. `request(&self, method, params,
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
    rewritten on disk with `Hello` moved two lines down; the next definition `ask` from `b.go`
    answers the new line.
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
- Create: `crates/core-code/src/lsp/mod.rs` (the servers a project runs, their state),
  `crates/core-code/src/lsp/catalog.rs`, `crates/core-code/src/lsp/catalog.json`,
  `crates/core-code/src/lsp/LICENSE-claude-plugins-official` (Apache-2.0 text and source
  commit), `crates/core-db/migrations/0058_code_servers.sql`,
  `crates/core-code/src/store/servers.rs`
- Modify: `crates/core-code/Cargo.toml` (`shadows-lsp`), `crates/core-code/src/lib.rs`,
  `crates/core-code/src/model.rs`, `crates/core-code/src/watch.rs`,
  `crates/core-code/src/store/mod.rs`, `crates/core-code/contract.yaml`,
  `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/code_servers.rs`

**Interfaces:**
- `catalog.json`: an array of
  `{ "id", "command", "args", "extensions": { ".go": "go" }, "install": { "windows": "…" | null, "macos": null, "linux": null }, "source": "<plugin>@<commit>" }`,
  the twelve entries and Windows install lines of the spec's table (builder checks the two
  winget ids with `winget show <id>` and writes `null` for one that does not resolve).
- Migration `0058_code_servers.sql`: `code_server_off(server_id TEXT PRIMARY KEY, at TEXT NOT NULL)`
  and `code_server_added(id TEXT PRIMARY KEY, command TEXT NOT NULL, args TEXT NOT NULL,
  extensions TEXT NOT NULL, at TEXT NOT NULL)` (JSON text for the lists). Rows are the
  person's, for every project.
- Model (`model.rs`):
  `pub enum ServerState { Idle, Starting, Running, NotInstalled, Off, Failed { error: Option<String> } }`;
  `pub struct ServerStatus { pub id: String, pub added: bool, pub wanted: bool, pub extensions: Vec<String>, pub state: ServerState, pub install: Option<String> }`
  (`install` is this system's line).
- `Code`:
  - `pub async fn servers(&self, asker: Asker<'_>) -> Result<Vec<ServerStatus>, CodeError>`:
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
  `Worker::stop` shuts every server down. A start failure is `Failed` with
  `last_error_line`, retried at the next scan.

- [ ] **Step 1: Write the failing tests** in `code_servers.rs`:
  - `the_catalog_has_the_twelve_servers`: 12 ids; clangd's extensions hold `.C` and `.c`.
  - `a_go_file_wants_gopls` (ignored, needs gopls): a project folder with `main.go`, touched;
    within 60 s `servers` shows gopls `Running`, and no other server `Running`.
  - `a_new_extension_starts_its_server` (ignored, needs gopls): an empty folder touched, then
    `main.go` written; gopls becomes `Running` with no refresh call.
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
- Modify: `crates/core-code/src/active.rs`, `crates/core-code/src/lib.rs`,
  `crates/core-code/src/links.rs`, `crates/core-code/src/model.rs`,
  `crates/core-code/contract.yaml`, `crates/core-threads/src/lib.rs` (`list` no longer
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
- `Code::touch(&self, project)` keeps its signature; it now records `last_used = now` and
  starts the worker if none runs. A private sweep (every 30 s) stops a worker whose
  `last_used` is older than `IDLE = Duration::from_secs(10 * 60)` (the spec's "short";
  Mohammed may change it). A worker stopping keeps its rows (index state, inventory).
- Callers that touch: `Turns` when a message is sent (person's or a notice), and every
  `Code` question (`code_lsp`, inventory). Callers that stop touching: `Threads::list`,
  project create. A touch no longer activates linked projects.
- **Work in progress holds the project** (Codex's review, 2026-11-13): a long turn or an
  attempt may call no Code tool for more than the idle time. New
  `Code::hold(&self, project: &ProjectId) -> ActiveHold`; an `ActiveHold` keeps the project
  active while it lives, and its drop counts as a touch, so the idle time starts when the
  work ends. `Turns` holds one for each running turn (from its start until it settles);
  `Runs` holds one for each running attempt (Runs already has `Code`, B-128). Files add:
  `crates/core-runs/src/` (the file that starts and ends an attempt),
  `crates/core-runs/contract.yaml`.
- `CodeSettings`, `set_active_limit` and `LIMITS` go.

- [ ] **Step 1: Write the failing tests**:
  - `a_sent_message_activates_its_project`: send a message in a project's thread; the
    project's status is not `Inactive`.
  - `listing_threads_does_not_activate`: `threads().list` on an idle project; it stays
    `Inactive`.
  - `an_idle_project_stops` with a test-only idle of 1 s (`Code::idle_for_test`): touched,
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
- Create: `crates/core-code/src/lsp/ask.rs`, `crates/shadows/tests/code_lsp.rs`
- Modify: `crates/core-code/src/lib.rs`, `crates/core-code/src/model.rs`,
  `crates/core-code/src/error.rs`, `crates/core-code/src/scan.rs` (tags not extracted),
  `crates/core-code/contract.yaml`, `crates/shadows-mcp/src/code_tools.rs`,
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
- `Code::lsp(&self, asker: Asker<'_>, question: LspQuestion) -> Result<LspAnswer, CodeError>`:
  checks `file` with `Code::inside`, then **resolves its real path and refuses one outside
  the project's real folder** (a link or junction out), the same check the index makes
  before it reads (`scan.rs`, `names_itself` and the canonical folder; Codex's review,
  2026-11-13), and only then reads it; sends the file's current text with the request
  (`Server::ask`, B-138); `IncomingCalls`/`OutgoingCalls` do `textDocument/prepareCallHierarchy` first;
  `WorkspaceSymbol` asks every running server of the project and merges. Bounds: 100 items,
  32 KiB of answer; time limit 20 s per request. `CodeError::InvalidCommand` for a missing
  `file`/`line`/`character`/`query` the operation needs.
- MCP: `code_lsp` replaces `where_is`, `who_uses` and `outline`. It is in `THREAD_TOOLS`,
  `EXECUTOR_TOOLS` and `PROJECT_TOOLS` (my call: external agents keep a code tool).
  Description: "Ask the project's language server: goToDefinition, findReferences, hover,
  documentSymbol, workspaceSymbol, goToImplementation, incomingCalls, outgoingCalls. Lines
  and characters are one-based, as an editor shows them."
- Suspended index: `index_known` writes the file row with no tags (the inventory and the
  file rows stay); `Code::definitions`, `references`, `outline` stay, called by nothing.
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
- Modify: `crates/core-code/src/lsp/mod.rs`, `crates/core-code/src/store/servers.rs`,
  `crates/core-code/contract.yaml`, `crates/shadows-http/src/code.rs`, `api/openapi.json`
- Test: `crates/shadows/tests/code_servers_routes.rs`

**Interfaces:**
- `Code::set_server_off(&self, command: CommandId, id: &str, off: bool) -> Result<(), CodeError>`;
  `Code::add_server(&self, command: CommandId, server: AddedServer) -> Result<ServerStatus, CodeError>`
  (`AddedServer { id, command, args, extensions }`; an id that is a catalog id or already
  added is `InvalidCommand`; an extension must start with `.`);
  `Code::remove_server(&self, command: CommandId, id: &str) -> Result<(), CodeError>` (added
  ones only). Each is idempotent by command id as every mutating command is.
- Routes: `GET /api/projects/{id}/code/servers` → `Vec<ServerStatus>` (`Asker::Person`);
  `PUT /api/code/servers/{server}/off` `{ command_id, off }`;
  `POST /api/code/servers` `{ command_id, id, command, args, extensions }`;
  `DELETE /api/code/servers/{server}` `{ command_id }`.
  Switching off stops the server in every active worker at its next recompute; on starts it.

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
  options TEXT NOT NULL, chosen INTEGER, consent_used INTEGER NOT NULL DEFAULT 0, asked_at TEXT NOT NULL,
  answered_at TEXT)`; `options` is JSON `[{ "label", "consent": null | "server_install:<id>" }]`.
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
- MCP `person_ask { question, options: [{ label, consent? }] }` → `{ question_id }`.
- HTTP: `POST /api/threads/{id}/questions/{question}/answer { command_id, option }`.
- Prompt: "When the person must choose, ask with `person_ask` and wait for the answer; do
  not ask the same thing in text."

- [ ] **Step 1: Failing tests** in `person_questions.rs`: `a_question_is_a_stored_entry`
  (survives reopening the core), `an_answer_reaches_the_planner_as_the_persons_message`
  (thread idle, and thread with a turn running), `a_question_is_answered_once`,
  `an_answered_card_reads_chosen_after_a_reload` (the entries read again from a new core show
  `chosen`), `an_executor_cannot_ask`.
- [ ] **Step 2–4: Run** `cargo test -p shadows-core --test person_questions`, then
  `cargo test -p shadows --test mcp_tools --test planner_mcp`: fail, implement, pass.
- [ ] **Step 5: Commit** `feat(backend): B-143 person_ask, the question card`.

## Task B-144: `code_server_install`

**Files:**
- Create: `crates/core-turns/src/install.rs`, `crates/core-turns/src/store/install.rs`,
  `crates/core-db/migrations/0061_server_install.sql`, `crates/core-code/src/lsp/install.rs`,
  `crates/shadows-core/tests/server_install.rs`
- Modify: `crates/core-turns/src/model.rs` (`Notice.executor: Option<ThreadId>`; `None` is
  Shadows, actor kind `Shadows`), `crates/core-turns/src/lib.rs`,
  `crates/core-turns/contract.yaml`, `crates/core-code/src/lib.rs`,
  `crates/core-code/contract.yaml`, `crates/shadows-mcp/src/code_tools.rs`,
  `crates/shadows-mcp/contract.yaml`, `crates/core-grants/src/tools.rs` (`THREAD_TOOLS`),
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`, `api/openapi.json`

**Interfaces:**
- `Code::install_server(&self, id: &str) -> Result<InstallOutcome, CodeError>`: runs the
  catalog's install line for this system through `shadows-process` (`cmd /c <line>` on
  Windows) in the person's home folder, time limit 10 min;
  `InstallOutcome { ok: bool, tail: String }` (the last 2 KiB of output).
  No install line: `InvalidCommand("no install command for <id> on this system")`.
- `Turns::install_server(&self, thread: &ThreadId, id: &str) -> Result<(), TurnsError>`:
  refused (`Refused("the person has not chosen to install <id>")`) unless the thread's
  latest answered `person_question` chose an option whose consent is
  `server_install:<id>` and whose `consent_used` is 0; in one transaction marks it used and
  writes a `server_install` row (`id, question_id, thread_id, server, state: running,
  started_at, finished_at, tail`; migration `0061_server_install.sql`), answers at once,
  and runs `Code::install_server` in the background; then records the outcome (`ok` or
  `failed`) on the row and queues a notice (`executor: None`, card
  `{ kind: "server_install", server, ok, tail }`) to the thread. On success the next
  recompute starts the server.
- **An install cut short** (Codex's review, 2026-11-13; kept simple): when Shadows starts,
  every `running` row becomes `interrupted` and its thread gets the same notice with
  `ok: false, interrupted: true`. Nothing is installed again on its own: the Planner asks
  the person on a new card, and a new choice is a new consent.
- MCP `code_server_install { server }` → "Installing <server>; you will get a notice."
- Prompt: when `code_lsp` answers `NoServer` with `NotInstalled` and an `install` line,
  ask with `person_ask` (one option with `consent: "server_install:<id>"`, one for "I'll
  install it", and one to leave it), and call `code_server_install` only after the first.

- [ ] **Step 1: Failing tests** in `server_install.rs`:
  `install_needs_the_persons_choice_for_that_server` (no question; a question for another
  server; a used consent: all refused), `an_install_outcome_reaches_the_planner_as_a_notice`
  (an added catalog entry is not installable; the test uses a test-only install line
  `cmd /c echo installed`, `Code::install_line_for_test`), `a_server_with_no_install_line_is_refused`,
  `an_install_running_at_start_reads_interrupted` (a `running` row written, a new core
  started: the row is `interrupted`, one notice reaches the thread, and nothing runs).
- [ ] **Step 2–4: Run** `cargo test -p shadows-core --test server_install`, then
  `--test executor_notice` (or the file holding B-125's notice tests): fail, implement, pass.
- [ ] **Step 5: Commit** `feat(backend): B-144 code_server_install after the person's choice`.

## Task F-115: the question card, and a notice from Shadows

**Files:** `web/src/app/conversation/person-question-card.tsx` (+ test),
`web/src/app/conversation/entry.tsx`, `web/src/app/conversation/notice-item.tsx` (+ test),
`web/src/api/questions.ts` (+ test), `web/src/api/schema.d.ts` (generated),
`docs/frontend/stages.md`, `docs/frontend/codebase/README.md` (if its map asks).

- A `PersonQuestion` entry draws the question and one button per option, styled like the
  permission card. An option with a consent shows under its label what it allows ("Lets
  Shadows install typescript"). A click posts the answer with a new command id; the card then
  shows the chosen option and no buttons. An answered card (from the entry's card or the
  route's conflict) never shows buttons again.
- A notice whose `executor` is null is drawn in Shadows' name ("Shadows"), with the install
  card: server, ok or failed, and the tail in a collapsed block.
- Tests: `renders_the_planners_options`, `a_click_answers_once`,
  `an_answered_question_has_no_buttons`, `a_shadows_notice_names_shadows`.
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
- Browser acceptance on `testing3` (Mohammed): the spec's Tests section, recorded in this
  entry.
- Commit `test(backend): B-145 the Planner asks a real language server`.
