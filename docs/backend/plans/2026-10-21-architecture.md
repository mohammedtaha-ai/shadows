# Architecture restructuring plan

> **For builders:** each task is an `A-<n>` in [`../stages.md`](../stages.md),
> or an `F-<n>` in [`../../frontend/stages.md`](../../frontend/stages.md). One
> task, one commit, its tests green. Read the root `AGENTS.md` "Code structure"
> first: its strict rule binds every task here.

**Goal:** Shadows Core becomes a modular monolith, as the root `AGENTS.md`
"Code structure" describes. `AppCore` is the map and the only way in. Each
service is its own crate, holding its tables and live state behind a private
store, and the compiler refuses any reach across. Each rule has one owner. The
products are separate workspaces.

**Owner of the rules:** the root `AGENTS.md` (identical to `CLAUDE.md`),
section "Code structure". Where a task changes a decision that an owner spec
records, the task amends that spec in the same commit.

**Source:** the audit of 2026-10-21. Six read-only audits covered
`shadows-core` (three areas), the adapters (`shadows-http`, `shadows-mcp`, the
binary), `shadows-agent`/`shadows-process`/`shadows-index`/`fake-acp`, and
`web/src`. The controller verified the heaviest claims in code: `Profiles` is
built inside `Sessions::new`, `Grants::new` runs twice in `AppCore::start`, the
binary serves its own `/assistant` route, and the Planner holds four approve
tools.

## What the audit found

| Rule | State | Worst cases |
|---|---|---|
| R1 one way in | Mostly holds | `Profiles` built inside `Sessions`; `Grants` built twice; a second composition root and HTTP surface in the binary (`daemon/`); `CoreParts` exposes internals |
| R2 own store, own live state | Broken everywhere | Every service holds `Arc<Storage>`, a facade with domain SQL in 43 files. `design` and `plans` write each other's tables. `Turns` drives `Sessions` and writes harness and thread tables. `Design` and `Plans` hold `LiveHandles`. Threads and Turns form a cycle through `ThreadStopper`. |
| R3 one owner per rule | About 30 duplicated rules | Planner prompt built in 3 places; the "not terminal" status written 5 ways; approver, writer and propose pipelines duplicated; HTTP and MCP map errors differently; the web copies Core's model, label, window and mode rules |
| R4 shape | Uneven | `profiles` holds five jobs; `api/client.ts` holds nine domains; `acp.rs`, `events.rs`, `tools.rs`, `design.rs` and `aylul_profiles.rs` hold several jobs each; free `pub fn` across services |
| R5 comments | About 600 lines cite `§`, about 30 carry history | Two comments contradict the code: the MCP "no tool approves" note, and the `runtime_not_configured` 403/404 note |

## Global constraints

- **Behaviour does not change** unless a task says so. Each task keeps the
  existing tests green. A test that reached into internals is rewritten to go
  through `AppCore` in the same task.
- **The API does not change** (`git diff --exit-code api/`), except where a
  task names an accepted shared request.
- **The build:** one cargo command at a time, never cargo and vitest in
  parallel. One `target/` per workspace.
- **Live keys:** only `OPENROUTER` and `API_KEY_MINIMAX`. Read them with a read
  loop, never `. ./.env`, and never print, log or commit a value.
- **The ratchet** (A-0) only shrinks. A task that removes a violation deletes
  its allowlist line. No task adds one.

## Review focus

0. **A service crate's public surface.** After each move, check that `lib.rs`
   exports the service, its model types and its `*_in(&mut Tx, …)` methods, and
   nothing from `store/`.

1. **A transaction that spans two services.** Plan edits append a thread
   entry, removing a project deletes code and grant rows, and an approval checks
   a grant. These must stay in one transaction. The fix is the owner's `*_in(tx)`
   method taking the caller's transaction, never the caller writing the owner's
   tables itself.
2. **Live state with two owners after a move.** A running turn must be
   reachable through `Turns` only. Look for any remaining direct use of
   `LiveHandles`, `Sessions` or `ThreadStopper`.
3. **A slash command in a Planner turn.** `/context`, `/compact` or any other
   `/` command typed in the Planner runs as a command, not as text (A-2).
4. **Errors seen by the person.** After A-4, the same failure gives the same
   code over HTTP and MCP. Check the codes that differ today: `Constraint`,
   `TransitionConflict` and `ThreadBusy`.
5. **The provider form.** Enter in "Add a model by id" adds the model, and
   nothing else submits (F-16).

---

## Phase 0 · Guards first

### A-0 · The architecture ratchet test
New `crates/shadows/tests/architecture/main.rs`, which reads the source and fails on:
- (a) a call from one service folder into a `Storage` method defined in another
  service's folder (the audit's cross-call list, with `db/` pool methods
  `reader`, `write_txn`, `watch_committed`, `now`, `append_event`, `classify`
  and `record_command` exempt);
- (b) a free `pub fn` / `pub(crate) fn` at the top level of a service folder;
- (c) a `.rs` comment line containing `§`, or a history marker (`used to`,
  `no longer`, `Milestone`, `Task <n>`, `PROBE.md`, a commit hash);
- (d) a service folder under `crates/shadows-core/src/` missing from the
  `AppCore` map (A-1) or without a `contract.yaml`.

Each check holds an allowlist file under `crates/shadows/tests/architecture/`,
seeded with today's violations, one per line (`file :: symbol`). The test also
fails when an allowlisted line no longer matches anything, so the list can only
shrink.
- Done when: `cargo test -p shadows --test architecture` passes on today's
  tree, and fails when one new violation of each kind is added (checked by hand
  and reverted).

### A-1 · `AppCore` is the map
The doc comment on `AppCore` in `app.rs` becomes the service table: one line per
service, with its name, its one job (no "and"), and its `contract.yaml` path.
`docs/codebase/README.md` keeps the crates outside Core and links to `app.rs`
for Core. Check (d) of A-0 reads this table.
- Done when: `cargo test -p shadows --test architecture` and
  `cargo test -p shadows --test codemap` pass.

## Phase 1 · The defects the audit found, fixed in their owners

### A-2 · `Turns` owns what a turn sends
`Turns::compose_prompt(&TurnRequest) -> Prompt` (private, in `turns/prompt.rs`)
is the one place a Planner or AYLUL turn's blocks are built: the person's text,
the context since the session last heard, focus, continue-plan, and the stage
line. `spawn.rs`, `steer.rs` and `harness/setup.rs` stop assembling blocks.
`Harness` returns only facts (`ContextFacts`). **A text that starts with `/` is
sent alone**, as the adapter requires. The context it skipped is not marked as
heard, so the next normal turn carries it.
- Tests: `a_slash_command_is_sent_as_one_block`,
  `context_skipped_by_a_command_reaches_the_next_turn`, and the existing
  planner context tests.

### A-3 · The AYLUL window follows the current model
The window comes from the session's current model, at open and on every model
switch, through one `Harness` method. The open path and the switch path are
listed as an `agreements` pair in `harness/contract.yaml`.
- Tests: `switching_model_uses_the_new_models_window`, plus the agreement's test.

### A-4 · One error mapping for HTTP and MCP
`CoreError::code()` (in Core) gives the one `ErrorCode` for every error.
`shadows-http/src/failure/` and `shadows-mcp/src/refusal.rs` translate that code
and hold no branching of their own. The `_ =>` fallback to
`AYLUL_NOT_CONFIGURED` is removed.
- Tests: one table test over every `CoreError` variant, asserting the same code
  through both adapters.

### A-5 · ACP connection safety
`Connection::prompt` resets the session's `active` flag on drop (a guard), so
a cancelled caller cannot wedge the session. Harness stderr is logged by length
only, as `shadows-process` already does.
- Tests: `a_dropped_prompt_frees_the_session`; the logging test in
  `shadows-process`.

### A-6 · The Planner approve tools match the spec
Settle §13.2 against the code: either the four `*_approve` tools leave
`THREAD_TOOLS`, or the spec and the `workflow.rs` header say the Planner
approves with the person's consent. Mohammed rules first; the task then changes
the one side that is wrong. The tool list itself moves into Core
(`Grants::tools_for(kind)`), so the adapter holds no authorization policy.
- Needs: Mohammed's ruling.

### A-7 · The context-meter records corrected
- Rewrite `docs/evidence/2026-10-21-context-meter-probe.md`. Its run set
  `CLAUDE_CODE_MAX_CONTEXT_TOKENS=200000` and used an expired `OPENROUTER` key
  for part of the run. Add the controller's run of 2026-10-21:
  - `/context` alone works on the subscription and on OpenRouter.
  - With a second block, the command is lost.
  - The model sees the `/context` table afterwards.
  - `used` grows every turn: 32.4k, 37.0k and 39.8k on the subscription;
    12.7k, 13.7k and 14.2k on OpenRouter.
- Amend §12.8:
  - The ring keeps `used`.
  - Close the OPEN block: a reading reaches the model. So the breakdown route is
    removed, as §12.8 already decided for that outcome, and the person types
    `/context` in the conversation instead (it works after A-2).
  - The live per-turn figure moves to the runtime relay (Phase 6).
- Remove B-2 and B-3 from `stages.md`.

## Phase 2 · One composition root

### A-8 · Every service is built in `AppCore::start`
- `Profiles` is built in `start` and handed to `Sessions`, so the `Weak`
  back-reference goes.
- `Grants` is built once.
- `Sessions`, `LiveHandles`, `Runtime` and `Bus` are built in `start`, and each
  is handed only to the service that owns it.
- `CoreParts` gives tests `AppCore` and a temporary database, and nothing else.
  The test helpers that bypass `AppCore` move to methods behind `test-support`
  on the owning service.
- Tests that built services or `Storage` themselves (about 86 files) go through
  `AppCore`. They are split by service in A-9 to A-19, one service at a time,
  and each of those tasks moves its own tests.

### A-8b · The binary has one start and no second surface
`daemon/mod.rs` and `cli/mod.rs` call one start function. The `/assistant`
route moves into `shadows-http` behind its guard. If it is dead, it is deleted
instead. The master-key, desktop-receipt and assistant-bridge rules move into
Core behind `AppCore` methods. The binary keeps only configuration, tracing,
binding and signals.

### A-8c · `core-db`, the foundation crate
New `crates/core-db`: `Storage::open`, the pool, `Tx` (the write transaction
every store shares), the journal (`append_event`, `MAX_SEQ`, `watch_committed`),
`classify` and `record_command`, `now`, migrations, and `StorageError`.
`shadows-core` depends on it. No domain SQL moves into it: `db/approval.rs` and
`write_observer.rs` stay behind for A-15, A-16 and A-18 to place.
- Done when: `cargo test --workspace` passes, and `core-db` names no table that
  a service owns, except in the migrations.

## Phase 3 · One crate per service

Each task moves one service out of `shadows-core` into its own crate,
`crates/core-<service>`, in the shape the root `AGENTS.md` rule 4 gives, and
takes it through R2 to R5 at once:
- Its store becomes private to the crate, which replaces `Arc<Storage>` in that
  service. From here on the compiler refuses any other crate's SQL on its tables.
- Its `Cargo.toml` names only `core-db` and the service crates it calls. A cycle
  fails the build: the task breaks it (by moving the rule to its owner) before
  the move, never by merging crates.
- `AppCore::start` builds it and hands it its dependencies. `shadows-core`
  re-exports the public types the adapters need.
- Every read of another service's tables becomes a call to that service's
  method.
- Every cross-service write inside one transaction becomes the owner's
  `*_in(tx)` method.
- Its duplicated rules get one owner.
- Its files are split by job, its free `pub fn`s become methods or private
  functions, and its comments lose `§` and history.
- Its `contract.yaml` gains one entry per module.
- Its tests move into its crate's `tests/`, out of `storage_contract/` and
  the shared test trees, and go through its public API.
- Its allowlist lines in A-0 are deleted.

The order runs from the leaves up, so each service's dependencies are already
clean when it is done:

| Task | Service | Its rules to merge, besides R2 |
|---|---|---|
| A-9 | `instructions` | — |
| A-10 | `secrets` | Provider-key naming (`provider/{slug}` + `PROVIDER_API_KEY`) owned here; the copies in `profiles` and `harness` call it |
| A-11 | `projects` | `create` and `ensure_assistant` share one creation path. Removing a project calls `code` and `grants` `*_in(tx)` methods. |
| A-12 | `code` | Store helpers that other services call become methods (`project_reachable_in`, `selected_project_in`) |
| A-13 | `grants` | `tools_for(kind)` (A-6); `check_writer` and `bind_draft_ref` become methods |
| A-14 | `profiles` → split by job into `providers`, `agents`, `extensions`, `usage` | One provider test (it absorbs `Harness::test_saved_provider`). One preset lookup. Label and name limits stated once. The pricing rule leaves the store file. |
| A-15 | `threads` | Stop and close on removal move to `Turns`, which breaks the Threads ↔ Turns cycle. `db/approval.rs` domain SQL moves to `threads`, `grants` and `turns`. |
| A-16 | `design` + `plans` (one task: their mutual reach is undone together) | One planner approver, one writer resolution, one propose pipeline for map, vision and roadmap, one stage decision, one slug and cycle rule, one `Constraint → Refused` mapping (with A-4), one "no turn is running" check. `RunningTurns` replaces `LiveHandles`. Shared types move to the owner or to `command`. |
| A-17 | `harness` | `Sessions` becomes private to `Harness`. The §14.9 order check stated once. The turn-model rule shared with `prepare_turn`. `titles` calls `Threads`. |
| A-18 | `turns` | It reaches Sessions only through `Harness`. The "not terminal" status is one store constant. One stop path. One writer of `Interrupted` (recovery moves from `runtime` to `Turns::recover`). |
| A-19 | `events` + `runtime` + `command` | The plan payload projection moves to `plans`. `runtime` keeps register and stop, and drops `pub storage`. `command` gets its contract, and its free functions become methods. |

When A-19 is done, `shadows-core` holds `AppCore` and its wiring only, every
service is a crate on `core-db`, and the A-0 allowlists for (a) and (b) are
empty. The compiler now enforces (a). The test keeps (b), (c) and (d).

## Phase 4 · Adapters and the other crates

### A-20 · `shadows-http` and `shadows-mcp` split by job
- `design.rs`, `aylul_profiles.rs` and `tools.rs` split by the service they
  translate.
- The shared extractors (`EnvJson`, `detached`) move to one transport module.
- The `pub` handlers in `sse.rs` become `pub(super)`.
- Each HTTP route and MCP tool pair (`get`/`get_for`, `approve`/`approve_as_planner`
  and the five agreement pairs) is either one method, or is listed as an
  `agreements` pair with its test.

### A-21 · `shadows-agent` and `shadows-process` split by job
- `acp.rs` splits into connection, session, prompt and event mapping.
- `events.rs` splits into events, subagent, limits and usage.
- `usage.rs`'s turn state gets one owner.
- `choices` and `breakdown` free functions become methods.
- `ClaudeAdapter` and `ClaudeProfile` get private fields.
- The environment and containment setup that `spawn` and `spawn_persistent`
  each write is stated once.
- `fake-acp/main.rs` splits its handlers out of the session lifecycle.

## Phase 5 · Workspaces

### A-22 · Split the repository by product
| Workspace | Root | Crates |
|---|---|---|
| Core | `crates/` (unchanged path) | `shadows`, `shadows-http`, `shadows-mcp`, `shadows-core` (`AppCore`), `core-db`, every `core-<service>` |
| Runtime | `runtime/` | `shadows-process` → `runtime-process`, `shadows-agent` → `runtime-acp` (until Phase 6 retires it), `fake-acp`, then `runtime-events`, `runtime-relay`, `runtime-claude` |
| Code intelligence | `code-intelligence/` | `shadows-index` |
| AYLUL desktop, hook | `apps/desktop/…` | unchanged |

- Core depends on the other two as path libraries.
- Each workspace gets its own `Cargo.lock`, its own `target/` and its own CI job,
  with the same six-step gate.
- `CLAUDE.md`, `docs/codebase/README.md`, the codemap test and `ci.yml` follow
  the new paths in the same task.
- Done when: every workspace's gate passes from its own root, and
  `cargo test --workspace` in Core still passes.

## Phase 6 · Our own runtime replaces ACP

A separate design spec comes first:
`docs/backend/specs/2026-10-2x-runtime-design.md`. It covers the relay, the
Claude driver, the event types, and the replacement checklist. The checklist is
every capability `shadows-agent` gives Core today:
- launch, profile, initialize;
- open, resume and fork a session;
- prompt, cancel and steer;
- set an option, offered choices, mode policy, and permission refusal;
- events, tool and subagent metadata, slash commands;
- usage, cost, context usage, breakdown, limits, errors.

Its tasks are written when the spec is accepted:
- **R-1:** `runtime-relay` sits between Claude Code and the provider. It gives
  the exact usage per call, the live figure while a reply streams, and keeps the
  provider key out of the child process.
- **R-2:** `runtime-claude` drives Claude Code in `stream-json` mode.
- **R-3:** Core switches from `runtime-acp` to `runtime-claude`, and
  `runtime-acp` is deleted.

**Open for Mohammed:** while a reply streams, the provider reports no output
count. A live figure is therefore an estimate until the end of the stream.
Choose one:
- show "~" during the stream, then the exact figure;
- show the exact figure only, after the reply ends.

## Phase 7 · Comments, last sweep

### A-23 · No `§` and no history in code
Whatever A-9 to A-21 did not already remove: about 600 `§` lines and about 30
history comments across the Rust crates. Each `§` that carries a real "why"
becomes a sentence; one that only points to a spec is deleted. The two comments
that contradict the code (MCP "no tool approves" and `runtime_not_configured`)
are corrected. A-0's list (c) is empty.

## Web tasks (Masada's side, `docs/frontend/stages.md`)

| Task | What |
|---|---|
| F-16 | Fix the provider form's Enter trap. "Add a model by id" sits outside the provider `<form>` (or Enter calls Add), and saving with a typed but unadded id says so. |
| F-17 | Every mutation answer is written through `writeSaved` (`agreements-page.tsx`, `agent-extensions.tsx`). One factory owns the query keys; the literal keys in about ten files go. |
| F-18 | Core owns the rules. Delete `model-picks.ts` and show Core's 422 problems instead. The client also stops copying the label limit, window default, model count, the 1..20 active-projects range, `fileCount`, `mode-policy.ts` and the agent defaults: Core returns each value the form needs. Needs a shared request for the fields Core must add. |
| F-19 | One owner per behaviour: plan refresh in `stream/` only (`plan-frames.ts` goes); one `ProposalCard` for map, vision and roadmap; one confirm pattern; `turn-state.ts` reads Core's status instead of mirroring its machine (shared request if a field is missing). |
| F-20 | Split by job: `api/client.ts` by domain, `stream/frames.ts` by frame family, `Composer()` into send, stop and settings; `test-app.tsx` moves to `test/`. |
| F-21 | Comments: about 156 hand-written `§` lines and the history notes. |

## Order and size

1. **A-0 and A-1 first, A-2 to A-7 next:** about one day. These are the fixes a
   person notices.
2. **A-8 and A-8b:** about a day.
3. **A-8c, then A-9 to A-19:** one service crate per task, about 13 to 15 days
   of builder time. They run one after another, because each depends on the
   crates below it.
4. **A-20 to A-23:** about 3 days.
5. **F-16 and F-17:** can run now, in parallel with the backend: different
   files, no shared request needed. F-18 and F-19 wait for their shared
   requests. F-20 and F-21 run any time.
6. **Phase 6** starts after A-22, from its own spec.
