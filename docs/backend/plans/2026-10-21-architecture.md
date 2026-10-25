# Architecture restructuring plan

> **For builders:** each task is an `A-<n>` in [`../stages.md`](../stages.md),
> or an `F-<n>` in [`../../frontend/stages.md`](../../frontend/stages.md). One
> task, one commit, its tests green. Read the root `AGENTS.md` "Code structure"
> first: its strict rule binds every task here.

**Goal:** Shadows Core becomes a modular monolith, as the root `AGENTS.md`
"Code structure" describes. `AppCore` is the map and the only way in. Each
service owns its tables and live state behind a private store. Services are
extracted into crates only after their dependencies are acyclic. Rust privacy
protects store APIs; the architecture ratchet also checks SQL ownership. Each
rule has one owner. Product workspace moves and replacing ACP are later deliveries.

**Owner of the architecture:** [application core design, section 14](../../superpowers/specs/2026-09-26-application-core-design.md).
The root `AGENTS.md` (identical to `CLAUDE.md`) gives the working rules.
Where a task changes another owner spec's decision, it amends that spec in the
same commit. A source contract describes implemented behavior, not this target.

## Selected style and agent navigation

Use a modular monolith with explicit service objects (`struct` + `impl`),
composition, private fields and constructor injection. `AppCore` is built once
per runtime and owns assembly and shutdown. HTTP and MCP enter through its typed
accessors. Each service receives only the services it calls; no service receives
`AppCore`, looks up arbitrary services, or constructs another service. A cloned
`Arc` shares the same instance; it does not create another owner.

The navigation path is `AppCore` accessor -> owning service contract -> public
method -> private implementation -> callers/tests. `AppCore`'s map names actual
paths and contracts, never planned crates as if they already exist. LSP answers
symbol locations; comments explain reasons, not alternate call paths.

The supplied generic contract example and `docs/codebase/contracts/TEMPLATE.yaml`
are byte-identical at this review. Keep the repository's template and its native
contracts. Retain `obligations`, `agreements`, `not_the_caller's`, `gaps` and
`open_questions` where applicable; read test bodies before claiming coverage.
Do not generate a second manually maintained symbol index or copy type variants
from another owner. Contract checks establish structure, not behavioral truth.

Three approaches were considered: documentation alone leaves existing bypasses;
immediate crate extraction breaks existing type and storage dependencies;
establishing owners in place, then extracting proven boundaries, keeps each
commit buildable. This plan selects the third. Class inheritance, a DI framework,
a generic repository, and a trait for every service add no required capability.

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
  through the owner's API in the same task. Cross-service acceptance tests use
  `AppCore`; service-local tests use that service's public API and scoped fixtures.
- **The API does not change** (`git diff --exit-code api/`), except where a
  task names an accepted shared request.
- **The build:** one cargo command at a time, never cargo and vitest in
  parallel. One `target/` per workspace.
- **Live keys:** only `OPENROUTER` and `API_KEY_MINIMAX`. Read them with a read
  loop, never `. ./.env`, and never print, log or commit a value.
- **The ratchet** (A-0) only shrinks. A task that removes a violation deletes
  its allowlist line. No task adds a violation. A pure file move may rename the
  same entry, with a one-to-one diff; moving a violation never counts as fixing it.
- **Verification:** each commit runs the affected tests plus architecture,
  contract and code-map checks when ownership or paths change. Compile affected
  targets after a move. The full gate runs once before each delivery branch's PR,
  not once per service. Run cargo commands serially in the existing cache.
- **Scope:** the controller records exact caller/test files in `Touch:` before
  dispatch. Contracts, maps and generated artifacts affected by that task are
  included explicitly. No builder discovers an extra owner and silently expands.

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
  and `record_command` exempt); also a direct SQL reference to another owner's
  table, even when the query uses an allowed pool/transaction method. Include
  adapter access to stores/live registries, service construction outside the
  composition root, and a service depending on `AppCore` in this ownership check;
- (b) a free `pub fn` / `pub(crate) fn` at the top level of a service folder;
- (c) a `.rs` comment line containing `§`, or a history marker (`used to`,
  `no longer`, `Milestone`, `Task <n>`, `PROBE.md`, a commit hash);
- (d) a service folder or extracted `crates/core-*/src/` missing from the
  `AppCore` map (A-1) or without a `contract.yaml`. Foundations are registered
  separately in the code map; they need no invented application accessor.

Check Rust imports/calls and literal SQL with the existing parsing dependencies;
do not equate a text match with a resolved call. Keep the table-owner list with
the guard. Dynamic SQL and unclassified queries require explicit review and
cannot silently pass as owned. After extraction, scan both locations and keep
the SQL check: Cargo does not enforce table ownership. Add fixtures proving that
renamed imports, a cross-owner SQL query, and a moved service do not evade checks.

Each check holds an allowlist file under `crates/shadows/tests/architecture/`,
seeded with today's violations, one per line (`file :: symbol`). The test also
fails when an allowlisted line no longer matches anything, so the list can only
shrink.
- Done when: `cargo test -p shadows --test architecture` passes on today's
  tree, and fails when one new violation of each kind is added (checked by hand
  and reverted).

Execution clarification: exercise mutations on temporary fixture trees with the
same scanner and baseline comparison used for the real repository; never edit
live product files merely to demonstrate rejection. Include stale-entry rejection,
aliased imports, SQL across owners and discovery of an extracted service. Missing
A-1 map rows are explicit existing violations until A-1 adds the real map. A-0
may add the existing workspace Rust parser as a dev-dependency to `shadows`;
no new parser library or product dependency is needed. Unknown ownership or
unsupported SQL must be reported explicitly, never treated as a proven clean path.

### A-1 · `AppCore` is the map
The doc comment on `AppCore` in `app.rs` becomes the service table: one line per
service, with its name, its one job (no "and"), and its `contract.yaml` path.
`docs/codebase/README.md` keeps the crates outside Core and links to `app.rs`
for Core. Check (d) of A-0 reads this table.
- Keep module rows needed by the codemap test; change its reader in the same
  task if the service table becomes its source. A method or private module still
  has an owner after the map moves. The map says what exists at this commit.
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
and retain only transport-specific status/envelope handling. Authorization,
redaction and the meaning of an operation remain Core's. The `_ =>` fallback to
`AYLUL_NOT_CONFIGURED` is removed.
- Tests: one table test over every `CoreError` variant, asserting the same code
  for equivalent operations through both adapters. Preserve intentional transport
  differences; a changed public error code needs an accepted shared request.

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

### A-7 · Correct the context-meter evidence
Preserve the original run and append a dated correction, with the source log or
commit for each new claim. The recorded probe forced
`CLAUDE_CODE_MAX_CONTEXT_TOKENS=200000` and did not complete its direct check of
what the model remembers. An expired-key diagnosis and the controller's later
figures need their own inspected evidence; this plan is not that evidence.
Keep §12.8's OPEN until its control-run trigger is met, or amend the trigger
explicitly with the verified evidence and rationale. Keep the ring's `used`.
This documentation task changes no route and does not promise live relay usage.

### A-7b · Remove the breakdown route after agreement
Blocked on A-2, A-7's evidence decision and an accepted shared request naming
`GET /api/threads/{id}/context`, the summary-only ring and explicit `/context`
in the conversation. The controller records that request in both stages files.
Backend removes the route and its unsupported service surface, updates contracts
and §12.8, then regenerates OpenAPI with
`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` (PowerShell: set the
environment variable for that command and restore its previous value afterwards).
F-22 regenerates the web schema and removes the UI caller on the same delivery
branch before release. Test that the ring sends no hidden `/context` request,
the summary still renders, and an explicit slash command uses A-2's normal path.
Do not delete all coverage together with the route: retain the command and
summary regressions. `api/` changes only by generation for this accepted request.

## Phase 2 · One composition root

### A-8 · Every service is built in `AppCore::start`
- `Profiles` is built in `start` and handed to `Sessions`, so the `Weak`
  back-reference goes.
- `Grants` is built once.
- `Sessions`, `LiveHandles`, `Runtime` and `Bus` are built in `start`, and each
  is handed only to the service that owns it.
- The replacement application fixture gives tests `AppCore` and a temporary
  database, and nothing else. Test helpers that bypass the owner's API move to
  methods behind `test-support` on the owning service. Introduce the restricted
  fixture first; retire each
  old field only when its last test caller has migrated. A-8 does not remove
  fixtures still needed by the per-service tasks.
- Tests that built services or `Storage` themselves (about 86 files) go through
  `AppCore`. They are split by service in A-9 to A-19, one service at a time,
  and each of those tasks moves its own tests.

### A-8b · The binary has one start and no second surface
Before dispatch, the controller names the existing owner for each product-host
operation and amends the product-daemon integration spec if ownership changes.
Preserve assistant-only startup, desktop receipts, shutdown order and the native
caller contract. A route is not dead merely because the web client does not use it.
`daemon/mod.rs` and `cli/mod.rs` call one start function. The `/assistant`
route moves into `shadows-http` behind its guard. If it is dead, it is deleted
instead. The master-key, desktop-receipt and assistant-bridge rules move into
Core behind `AppCore` methods. The binary keeps only configuration, tracing,
binding and signals.

### A-8c · Settle the extraction dependencies before moving crates
Controller task: amend this plan and §14 with a complete dependency ledger from
the then-current code. Each row names the owner, runtime dependencies, public
types/errors, table ownership, cross-service transactions, test dependencies,
and the exact task that removes each cycle. Record the resolved graph and
topological extraction order; builders do not choose new owners during a move.

These inspected edges must be accounted for, not assumed to be leaves:

| Source | Existing dependency or boundary problem |
|---|---|
| `instructions/{mod,store}.rs` | `ProjectId`, `CommandContext`, `Actor`, `DurableEvent`, `CoreError`, `app::user_command`; SQL reads `project` |
| `projects/{mod,store}.rs` | `Code`; removal checks threads and calls plans, code and grants in one write |
| `db/command.rs` | `CommandContext` lives in `command` |
| `db/journal.rs` | `DurableEvent` lives in `events` and contains service-owned IDs |
| `db/error.rs` | `PlanId`, `Problem`, `WorkflowId` belong to `plans` |
| service `store` modules | Inherent `impl Storage` blocks cannot remain after `Storage` moves to another crate |
| `threads` / `harness` / `turns` | Session lifecycle, title writes, stopping and ID types must be checked separately |
| `profiles/person.rs` | `person` / `set_person` need an owner when profiles splits; four destination names do not cover this job |

Decisions the ledger must close before extraction: separate domain errors from
database failures; give journal/command persistence inputs a downward-only type
boundary; remove `app::user_command` as a dependency from lower services; retain
service-owned ID types without a grab-bag types crate. A service's test crate
must not require the production `shadows-core` composition root to construct it.

Keep `Storage` in `shadows-core` throughout A-9 to A-19. Move domain methods onto
private service store types there first. Do not use extension traits to preserve
the global domain-storage facade. `db/write_observer.rs` is test-only SQLite
`PRAGMA data_version` observation, so it follows the foundation under
`test-support`; it is not domain SQL to distribute among services.

Done when every inspected edge has a named owner and migration step in the
ledger, exact `Touch:` lists and affected test commands are recorded, and the
controller verifies the proposed graph is acyclic. Until then crate extraction
is blocked, while A-0 to A-7's independent fixes can proceed.

#### A-8c ledger (controller, 2026-10-22, inspected at `cdc218b`)

Sources:
- `use crate::…` and inline `crate::…` paths in `crates/shadows-core/src`, with test modules excluded.
- `crates/shadows/tests/architecture/ownership.allowlist`, which lists the cross-owner SQL, store calls and `Storage` calls.
- `table-owners.txt`.

These rows are decisions. A builder applies them and does not reopen them. If a row proves wrong in code, the task stops and returns to the controller.

**Foundation (`core-db`, extracted in A-19b; prepared in A-8d).**

| Today | Decision |
|---|---|
| `app::user_command` (called by 11 services) | Becomes `CommandContext::user(command_id, kind, params)`. Services stop importing `app`. |
| `command::{CommandContext, fingerprint, derived_id}` | Move to the foundation, because command persistence needs them. `Anchor::Operation` takes the operation id as `&str`. |
| `command::{Writer, Approver, ApprovalRequest, ApprovedBy}` | Owned by `grants`. They name `ThreadId` and `ThreadEntryId`, and they are authorization. |
| `events::{Actor, Causation}` | Move to the foundation. They carry no service types. |
| `events::DurableEvent` (holds `ProjectId`, `ThreadId`, `OperationId`) | The journal takes a `JournalEntry` whose subject ids are strings. Each service converts its own ids. `DurableEvent`, `UiSignal` and the subscription stay with `events`, which becomes the top read service. The plan payload projection moves to `plans` (A-19). |
| `app::Bus` (a running turn's live harness events: `(ThreadId, OperationId, HarnessEvent)`) | **Corrected at A-8d:** it is not the journal notifier (`Storage::committed` already is, in `db/`). It is owned by `turns`, which produces it. `Events`, which sits above turns, subscribes through a `Turns` method. It stays in `app.rs` until A-18 moves it. |
| `id::newtype_id!` | The foundation owns the macro. Each id type stays with its service. There is no shared types crate. |
| `db::StorageError` | Keeps only database failures: `Unavailable`, `Migration`, `Constraint`, `NotFound`, `TransitionConflict`, `RevisionConflict`, `CommandConflict`, `Json`, `Database`. Every other variant moves to its owner's error in that owner's task: plans gets `PlanInvalid(Problem)`, `PlanLinkInvalid`, `WorkflowFrozen`, `NotLatestVersion`, `PlanArchived`, `TaskNotInPlan`, `ReasonMissing`; grants gets `GrantInvalid`, `GrantScope`; threads gets `HarnessLocked`, `ForkPointNotSupported`; turns gets `ThreadBusy`, `QueuedMessageGone`; profiles gets `Extension*`, `Aylul*`, `ProviderInUse`, `ProfileInvalid`; design gets `StageBlocked`; project removal gets `ProjectHasThreads`. |
| `error::CoreError` (holds `OpenError`, `DirectoryError`, `StartError`) | `CoreError`, `ErrorCode`, `Refused` and `InvalidCommand` hold no service type. Each `From<ServiceError> for CoreError` lives with that service. At extraction the vocabulary becomes `core-error`, and A-4's single HTTP/MCP mapping stays in `shadows-core`. |
| `db/approval.rs` | `authorise_in` and `read_in` move to `grants`. Grants reads the request entry through `threads`' `entry_in`. |
| `db/write_observer.rs` | Follows the foundation under `test-support`. |

**Tables whose owner changes** (A-0's `table-owners.txt` changes in the task named):

| Table | From → to | Task | Why |
|---|---|---|---|
| `standards_additions_version` | design → `standards` | A-12b | Harness and turns read standards and sit below design |
| `aylul_provider` / `aylul_agent` / `aylul_extension` + `aylul_agent_extension` / `aylul_usage` / `aylul_person` | profiles → `providers` / `agents` / `extensions` / `usage` / `person` | A-14 | The five owners A-14 requires |
| `draft_intent` | plans → grants | A-13 | It is the state behind issuing a draft ref, and plans calls `bind_draft_ref`. Grants stores the workflow as an opaque ref and never imports `WorkflowId`. |
| `operation` | turns → `operations` | A-17b | Design and plans check a running turn, and turns reads plans for focus. Splitting operation lifecycle from turn sending is the only cut without a callback. |
| `agent_invocation` | turns → harness | A-17 | It records what context a session heard. Turns writes it through `Harness::record_invocation_in`. |
| `design_part_plan`, `design_outcome_plan` | design → plans | A-16 | Links from plans to design entities. Design then needs no `PlanId`. |
| `command_record`, `durable_event` | db → foundation | A-8d / A-19b | — |

**New owners.** These are controller decisions. None of them is a helper or a facade.
- **`standards`:** the project standards and their additions.
- **`operations`:** the durable operation lifecycle: operation rows, transitions, the running-turn check, and recovery. Recovery leaves `runtime`. (Amended 2026-10-22 on A-17b's blocker: `LiveHandles` and stop stay with `turns`, because a live turn holds harness's `OpenSession` and turns' `SteerRequest`, and stopping it closes the harness session; in `operations` they would make a cycle or need a callback. Turns sits above harness and operations, so it owns the live turn and calls `Operations` to record the transitions.)
- **`project-removal`:** removing a project. Inside one transaction it checks threads, invalidates plan dependencies, cleans code and revokes grants. After the commit it stops the code worker. Every service it touches sits below it, so `projects` stays a lower service. A "renamed registry" would not be an owner. This is a use case with its own rule (`ProjectHasThreads`) and its own test (`project_remove.rs`).

**The service cycles found and how each is cut:**

| Cycle | Cut | Task |
|---|---|---|
| projects ↔ code ↔ plans | Removal moves to `project-removal` (A-16b). `code_order` stops joining `operation`/`planning_thread`, and calls `Projects::by_recent_activity()`, which projects computes from the journal (A-11, A-12). The link commands become `Plans::link_projects` / `unlink_projects`, which call code's `put_link_in` / `remove_link_in` and then notify (A-16). | A-11, A-12, A-16, A-16b |
| projects ↔ threads | `known_harness` and the harness policy move to `projects` | A-11 |
| secrets ↔ profiles | Secrets stores by scope and checks nothing upward. Setting an agent's or a provider's env becomes `Agents::set_env` / `Providers::set_key`, which check existence and call `Secrets::put_in`. | A-10 creates `put_in`; A-14 moves the callers |
| profiles ↔ harness | `harness/profile_home.rs` (`seed`) moves to `agents`. Harness asks agents for the home. | A-14 |
| threads ↔ harness ↔ turns | `Threads::set_harness`'s session close becomes `Harness::set_thread_harness`. `Threads::remove`'s stop, close and revoke become `Turns::remove_thread`. Threads then calls neither. | A-17, A-18 |
| grants ↔ threads, grants ↔ plans | Grants is above threads. Harness issues the thread grant after checking the thread. `draft_intent` moves to grants with an opaque workflow ref. | A-13 |
| harness ↔ turns | `agent_invocation` moves to harness. `thread_is_busy` calls `operations`. | A-17, A-17b |
| design ↔ plans | Plans owns the plan links (above). The design operations that read plan bindings (`agreement_participants_in`, `part_bindings_in`, `check_design_plan`) become `Plans` methods. | A-16 |
| design/plans ↔ turns | The running-turn checks call `operations`, not turns | A-17b, A-16 |
| runtime ↔ turns | Recovery becomes `Operations::recover`. `Runtime` drops `pub storage`. | A-17b |

**The resolved graph** (each service may depend only on those listed; `core-db` and `core-error` are implicit). A script checked this graph for cycles at `cdc218b`. Extraction follows this order:

| # | Service | Depends on |
|---|---|---|
| 1 | `secrets` | — |
| 2 | `projects` | — |
| 3 | `instructions` | projects |
| 4 | `standards` | projects |
| 5 | `code` | projects |
| 6 | `providers` | secrets |
| 7 | `agents` | providers, secrets, projects |
| 8 | `extensions` | agents |
| 9 | `usage` | agents |
| 10 | `person` | — |
| 11 | `threads` | projects, code, agents |
| 12 | `grants` | projects, threads |
| 13 | `runtime` | — |
| 14 | `operations` | threads, runtime |
| 15 | `harness` | threads, grants, operations, agents, providers, extensions, secrets, instructions, standards, projects |
| 16 | `design` | projects, threads, grants, operations, standards, harness (read: a proposal's pinned standards) |
| 17 | `plans` | design, code, grants, threads, operations, projects, harness (read: a version's writer model) |
| 18 | `turns` | harness, operations, threads, grants, plans, design, standards, instructions, usage, runtime, code, projects |
| 19 | `project-lifecycle` (was `project-removal`; A-16b ruling) | projects, threads, plans, code, grants |
| 20 | `events` | harness, plans, projects, threads, turns, operations |

`shadows-core` (`AppCore`) depends on all of them. Adapters depend on `shadows-core` only.

**Preparation order.** The task numbers are not this order. Each task's `Needs:` names the one before it:

A-8d → A-9 → A-10 → A-11 → A-12 → A-12b → A-14 → A-15 → A-13 → A-17b → A-17 → A-16 → A-18 → A-16b → A-19.

**The remaining inspected edges:**
- **`instructions`:** its SQL reads of `project` become `Projects::require_live_in(tx, id)` (A-9). Its other imports resolve to the foundation.
- **`projects` → `Code`:** after the removal moves out, projects only stops the code worker for a removed project, and that call now sits in `project-removal`.
- **`events` → `Harness`, plans and `workflow` SQL:** `events` is the top read service. `read_project_events_after` reads `workflow` through `Plans` (A-19).
- **Inherent `impl Storage` blocks:** each service task moves its blocks to a private `<Service>Store` type that holds the shared handle. This is Phase 3's rule; no extension traits.

**Tests.**
- Each service's tests use its public API, through a `test-support` constructor that takes its declared dependencies, plus the foundation's temporary storage. They never use `AppCore`.
- Tests that cross services stay with `shadows-core` through `testing::app`.
- Adapter tests stay in `crates/shadows/tests`.
- Each task's `Done when:` names its test binaries. `cargo test -p shadows --test architecture` and `--test codemap` run in every task.
- The 27 `CoreParts` files and the 10 `Sessions::new` callers listed in A-8's report move in the task that owns them, and the last one deletes `from_parts`.

## Phase 3 · Establish each service boundary in place

Each task prepares one service inside `shadows-core`; these are ownership tasks,
not crate moves. A-8c supplies the dependency decisions and exact dispatch scope.
- Its private `<Service>Store` owns its SQL and holds the existing shared
  storage handle. Domain methods leave `impl Storage`; no second pool is opened.
- `AppCore::start` builds the service and hands it its explicit dependencies.
  Cross-service call and type cycles are resolved according to the ledger.
- Every read of another service's tables becomes a call to that service's
  method.
- Every cross-service write inside one transaction becomes the owner's
  `*_in(tx)` method.
- Its duplicated rules get one owner.
- Its files are split by job, its free `pub fn`s become methods or private
  functions, and its comments lose `§` and history.
- Its `contract.yaml` gains one entry per module.
- Its tests are grouped by owner and use its public API. Cross-service tests
  remain at the composition layer through `AppCore`; storage rollback and
  cancellation tests stay with the foundation. No service test depends on an
  adapter or requires another public constructor for the whole application.
- Its allowlist lines in A-0 are deleted.

The task numbers identify ownership, not a topological order. The controller
orders these preparations from A-8c's ledger and updates `Needs:` accordingly.
Create a needed callee method before migrating its caller. Do not dispatch a
task whose required method, owner or scoped test fixture is missing:

| Task | Service | Its rules to merge, besides R2 |
|---|---|---|
| A-9 | `instructions` | — |
| A-10 | `secrets` | Provider-key naming (`provider/{slug}` + `PROVIDER_API_KEY`) owned here; the copies in `profiles` and `harness` call it |
| A-11 | `projects` | `create` and `ensure_assistant` share one creation path. Removal preserves the thread check, plan dependency invalidation, code cleanup and grant revocation in one transaction, then stops the code worker after commit. |
| A-12 | `code` | Store helpers that other services call become methods (`project_reachable_in`, `selected_project_in`) |
| A-13 | `grants` | `tools_for(kind)` (A-6); `check_writer` and `bind_draft_ref` become methods |
| A-14 | `profiles` → `providers`, `agents`, `extensions`, `usage`, `person` | The controller names all five owners in A-8c. Preserve local person reads/updates. One provider test (absorbs `Harness::test_saved_provider`), one preset lookup, one owner of label/name limits; pricing leaves the store. |
| A-15 | `threads` | Stop and close on removal move to `Turns`, which breaks the Threads ↔ Turns cycle. `db/approval.rs` domain SQL moves to `threads`, `grants` and `turns`. |
| A-16 | `design` + `plans` (one task: their mutual reach is undone together) | One planner approver, writer resolution, propose pipeline, stage decision, slug/cycle rule and `Constraint → Refused` mapping (A-4). Running-turn checks go through `operations` (A-8c ledger) with the existing transaction/race guarantees; a renamed registry beside it is not another owner. Shared types stay with their actual owner. |
| A-17 | `harness` | `Sessions` becomes private to `Harness`. The §14.9 order check stated once. The turn-model rule shared with `prepare_turn`. `titles` calls `Threads`. |
| A-18 | `turns` | It reaches Sessions only through `Harness`. The "not terminal" status is one store constant. One stop path. One writer of `Interrupted` (recovery moves from `runtime` to `Operations::recover`, A-17b). |
| A-19 | `events` + `runtime` + `command` | The plan payload projection moves to `plans`. `runtime` keeps register and stop, and drops `pub storage`. `command` gets its contract, and its free functions become methods. |

When these preparations are done, the services still live in `shadows-core`,
but no domain method extends `Storage` and no caller uses another owner's store
or tables. A-0's (a)/(b) allowlists are empty. Keep all four checks permanently.

### A-19b · Extract the prepared database foundation
After A-8c and A-9 to A-19, move only the prepared database foundation to
`crates/core-db`. It owns connection lifetime, serialized writes, rollback on
cancellation, committed journal notification, migrations and command persistence.
It imports no service or `shadows-core`; the ledger's persistence inputs contain
no dependency back to domain services. Domain errors stay with their owners.
`Tx` preserves the existing SQLx/`BEGIN IMMEDIATE` behavior; do not introduce a
custom ORM or another transaction engine. `*_in` methods never commit, start
nested transactions or publish before the caller commits.

Move the database tests and the feature-gated write observer with it. Verify
rollback of partial cross-owner writes, replay/conflict behavior, cancellation
recovery and notification only after commit. Compile the workspace's targets;
run the affected foundation and cross-service tests, not the full release gate.

Completed on `context-meter` (2026-10-24). `crates/core-db` now owns the eight
foundation source modules, migration build script and all 37 migration files.
Their Git blobs are unchanged. `Tx` names the borrowed SQLx connection; the same
serialized `BEGIN IMMEDIATE`, rollback recovery and after-commit watch remain in
`Storage`. Domain stores call `CommandContext` and `JournalEntry` methods, and
convert private durable events explicitly; adapters retain only `shadows-core`.
The exported id macro leaves each id with its service. Only `test-support`
forwards the write observer and test hooks; the production feature tree is clean.

The pinned fingerprint/derived-id test bodies remain unchanged. Foundation tests
cover cancellation of partial command/event writes, recovery, replay/conflict,
observer rollback and committed notification. The application test forces command
recording to fail after project state and its event have been written, proving
that partial writes roll back together. The migration compatibility tests moved
with the foundation; domain migration tests use the new migration directory.
Contract validation follows the moved contract. Architecture fixtures classify
catalog reads and `PRAGMA data_version` only in the foundation, preserve adapter
sensitivity to the moved crate and reject service dependencies in its package.
All three A-19b SQL-review allowlist records are deleted; none were added.

Verification: the exact CI awk line-length command, fmt, both assigned workspace
clippy modes, production feature-leak check and unchanged API diff passed.
`core-db` passed 20 tests; the named Core integrations passed 31; Core units passed
40; the named adapter suites passed 113. The two affected domain migration suites
add six passes and one pre-existing ignored manual probe. These are local automated
checks; browser, native and production acceptance were not run in this extraction.

Review (2026-10-24): a regression reproduced permanent writer failure after an
external writer made BEGIN time out. Storage now uses SQLx's BEGIN IMMEDIATE
transaction guard, including SQLx drop rollback, rather than a manual open flag.
The timeout regression and cancellation/recovery tests pass. All 37 Git blobs
match `0327292`; a pinned SHA-384 catalog also checks the embedded versions,
descriptions, order and checksums. It caught CRLF bytes in the local migration 37;
restoring its canonical LF bytes changed no Git blob. The build script remains
unchanged. All eight moved storage tests retain their assertions, and both pinned
identity test bodies match `6c4505a`. Moved test comments now state the invariant.
The review commands passed 22 foundation tests, 77 Core tests (one existing
manual migration probe ignored), and 129 architecture/adapter tests. No browser,
native or production acceptance was run.

### A-19c · Extract services in the verified order

Controller ruling (2026-10-24): a comment ratchet record keyed to a moved file is relocated one-for-one in the extraction commit, with identical text and count and only its path changed. This is a rename. API-feeding doc comments move unchanged; the API artifact diff must remain empty. Other allowlists may only shrink.

Controller continuation (2026-10-24, `9b3735e`): service-owned error enums replace
CoreError in each owner, lowest first, in its extraction commit; CoreError stays
in shadows-core and converts those errors without changing adapter responses.
Harness owns AylulOwners; AppCore constructs the bundle and its services. Plans
imports operation and invocation types from operations and harness directly.

The conservative event boundary uses the existing core-db JournalEntry as the
only durable publication vocabulary: its subject ids are strings, and each
publisher converts its owned ids without changing payload serialization, UUIDs,
column values or append order. The typed DurableEvent bridge is removed instead
of introducing another vocabulary crate. The live plan-show value UiSignal moves
to Plans, below its publisher Turns and consumer Events; its typed fields and
Serialize derive stay identical, and shadows-core preserves its public re-export.
This leaves event subscription with Events and plan-view vocabulary with its
existing owner, without a shared types crate or a dependency back to publishers.


Dispatch verification (2026-10-24, `e89e9c4`): the expanded tasks and current
runtime/type/test graph are recorded under A-19c in `docs/backend/stages.md`.
Rust LSP verifies runtime as the only independent extractable service. The
remaining services are blocked by the unextracted CoreError vocabulary, with
additional DurableEvent/UiSignal cycles and Harness's app::AylulOwners edge.
Usage no longer depends on agents. Import paths through turns must be resolved
to the actual operations/harness owners before those moves. A-19c.1 moves
runtime; A-19c.2 through A-19c.20 explicitly retain their blocking edges.
This verification supersedes A-8c's acyclicity claim for today's source; it
does not authorize changing error semantics or introducing another owner.

The controller expands this milestone into one task per service using A-8c's
resolved graph, including type-only and test dependencies. It is blocked until
those exact tasks exist. Each move keeps the service API and table schema stable,
updates manifests/lockfile, re-exports, `AppCore` wiring, contracts, map and tests
in one commit. Run its public-API tests plus affected callers and architecture
checks. Adapters continue to depend on `shadows-core` only. Foundation and
service-local tests do not obtain their fixture from `AppCore`; application
integration tests remain with `shadows-core` or the adapter acceptance suite.

At completion `shadows-core` contains composition and application-facing error
translation/re-exports. It owns no second domain rule, store or live registry.
Cargo rejects dependency cycles; privacy hides stores; the ratchet still checks
SQL ownership, adapter access and map coverage across every extracted crate.

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
Separate delivery after A-19c. Its controller preparation must enumerate every
path dependency, dev-dependency, test binary lookup, lockfile, feature and native
consumer before authorizing a move. Keeping all Core crates in the root workspace
is the default during the service migration; a separate workspace is not needed
to obtain private service stores or one `AppCore`.

| Workspace | Root | Crates |
|---|---|---|
| Core | root `Cargo.toml`, members in `crates/` | `shadows`, `shadows-http`, `shadows-mcp`, `shadows-core` (`AppCore`), `core-db`, every `core-<service>` |
| Runtime | `runtime/` | `shadows-process` → `runtime-process`, `shadows-agent` → `runtime-acp` (until Phase 6 retires it), `fake-acp`, then `runtime-events`, `runtime-relay`, `runtime-claude` |
| Code intelligence | `code-intelligence/` | `shadows-index` |
| AYLUL desktop, hook | `apps/desktop/…` | unchanged |

- Core depends on the other two as path libraries.
- Each workspace gets its own `Cargo.lock`, its own `target/` and its own CI job.
  Define a gate for the packages that actually exist there: fmt, clippy for
  tests and production, tests, and production feature-leak checks. Core alone
  checks its OpenAPI artifact. Do not pass `fake-acp/test-support` to a workspace
  that no longer contains that package; adapt its feature/test dependency checks.
  Native consumers compile against the moved libraries in their own workspace.
- `CLAUDE.md`, `docs/codebase/README.md`, the codemap test and `ci.yml` follow
  the new paths in the same task.
- Done when: every workspace's gate passes from its own root, and
  `cargo test --workspace` in Core still passes.

## Phase 6 · Our own runtime replaces ACP

A separate dated design spec and acceptance plan come first under
`docs/backend/specs/`. This phase is not part of the architecture refactor's
implementation authorization or estimate. It covers the relay, the
Claude driver, the event types, and the replacement checklist. The checklist is
every capability `shadows-agent` gives Core today:
- launch, profile, initialize;
- open, resume and fork a session;
- prompt, cancel and steer;
- set an option, offered choices, mode policy, and permission refusal;
- events, tool and subagent metadata, slash commands;
- usage, cost, context usage, breakdown, limits, errors.

Its tasks are written when the spec is accepted:
- **R-1:** investigate a relay between Claude Code and supported providers.
  Establish subscription/BYOK compatibility, the source and meaning of usage,
  streaming availability, authentication ownership and cancellation before
  promising exact readings or keeping credentials out of the child process.
- **R-2:** `runtime-claude` drives Claude Code in `stream-json` mode.
- **R-3:** Core switches from `runtime-acp` to `runtime-claude`, and
  `runtime-acp` is deleted.

**OPEN for the runtime design:** determine which supported providers expose
output usage during streaming. Where only a final count is available, choose
the product behavior from measured evidence:
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
| F-22 | After A-7b's accepted shared request: regenerate the schema, keep the summary ring, remove the hidden breakdown request and cover explicit `/context`. |

## Delivery order and acceptance

1. **Guards and behavior fixes:** A-0/A-1, A-2 to A-5, A-7 and F-16/F-17.
   A-6 waits for the authorization ruling; it does not block independent fixes.
   A-7b/F-22 wait for their evidence decision and shared request.
2. **Ownership preparation:** A-8, the scoped A-8b and controller A-8c, then
   A-9 to A-19 in the ledger's dependency order. A-13's tool policy follows A-6.
3. **Crate extraction:** A-19b, then A-19c's concrete per-service tasks.
   Recheck the graph after each extraction; a remaining cycle returns to the
   controller's ledger, not a builder-created facade or new registry.
4. **Adapters and cleanup:** A-20/A-21 and A-23 can be separate scoped commits
   once their owners stabilize. Coordinate F-18/F-19 through shared requests;
   do not combine competing edits to `web/src/api/` just because task IDs differ.
5. **Product workspaces:** A-22 is a later delivery with its own gate changes.
   **Runtime replacement:** Phase 6 has its own accepted design and evidence.

Use short delivery branches and one task per commit. Before each PR, run that
branch's full gate, one whole-branch review, and a real `shadows serve` browser
journey with logs for the behavior touched. During ownership/extraction, include
send/stop, model change, project removal, replay/conflict and restart recovery.
Run native acceptance when daemon/desktop behavior changes. Record automated,
browser, native and provider evidence separately. Do not infer one from another.

The previous day estimates assumed an unverified leaf order. Re-estimate after
A-8c from the actual caller/test moves. The controller records each next batch's
base commit, tasks, exact `Touch:` and test commands before dispatch; completing
one batch does not imply the later workspace/runtime work is complete.
