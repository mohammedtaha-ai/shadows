# Milestone 1 — Harness Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the person pick the CLI per conversation and the model, mode and effort per message; show context and account limits; add copy and fork-from-last — all recorded durably and enforced by the daemon.

**Architecture:** The daemon owns a per-harness catalogue (`agent/catalogue.rs`) and serves it; a turn start becomes one idempotent storage command that writes the user entry, the `Pending` operation, its `agent_invocation`, and the command record together; the harness stream is classified into the facts §12.5–§12.6 record. The web client builds every menu from `GET /api/harnesses`. Backend and web are built by separate agents in separate worktrees against a hand-written `api/openapi.json` draft (§12.10).

**Tech Stack:** Rust (SQLx 0.9 on SQLite, axum 0.8, utoipa), React 19 + TypeScript (TanStack Query/Router, shadcn/ui on Base UI, Tailwind v4), Vitest.

**Spec:** `docs/superpowers/specs/2026-09-24-harness-controls-design.md` (§12). Every task's requirements include the spec section it names.

## Global Constraints

- Harness kinds are the strings `claude-code` and `codex`. Codex is `available: false`.
- Claude modes: `acceptEdits` (default of every new conversation, never remembered) and `auto`. No other mode is listed.
- Claude models: `sonnet` (default), `opus`, `haiku`. Efforts per model come from Task 0's evidence file.
- `--permission-prompts none` stays on every Claude invocation.
- New error codes, exactly: `HarnessUnavailable` 422, `SettingNotOffered` 422, `ModeNotAllowed` 403, `HarnessLocked` 409, `ThreadBusy` 409, `ForkPointNotSupported` 422.
- Nothing the stream did not report is estimated; it stays NULL and shows as unavailable.
- Backend agent never opens `web/`; web agent never opens `src/`, `tests/`, `migrations/`, never runs the daemon (§12.10).
- Files: 300 lines needs a stated reason, 500 splits (CLAUDE.md). `storage/mod.rs`, `protocol/`, `tests/storage_contract.rs` are accretion points: new storage capabilities go in new `storage/sqlite/<entity>.rs` files, new tests in new `tests/<topic>.rs` files, never appended to `storage_contract.rs`.
- Every code change that moves a signature regenerates the code map in the same commit: `UPDATE_CODEMAP=1 cargo test --test codemap`.
- Every subagent runs on opus, stated explicitly in the dispatch.

## Review Focus

1. **A retried Send after a network drop** — the person expects one message and one turn, never two. The client must resend the same `command_id`, and the daemon must answer a replay before any validation. (Task W2 test `a retried Send reuses the same command id`; Task B3 tests `a_replayed_turn_start_starts_nothing_and_returns_the_first_operation` and `a_replay_is_answered_even_after_the_mode_was_disallowed`.)
2. **Switching model after choosing an effort the new model does not accept** — the client must move effort to the new model's default, and the daemon must refuse a stale pair rather than run it. (Task B3 test `an_effort_the_model_does_not_accept_is_refused`; Task W2 test `changing_model_resets_an_effort_it_does_not_accept`.)
3. **A project that allowed `auto` and then removed it, while a conversation still shows `auto` selected** — the next Send must be refused with a message, not silently run with `auto`. (Task B3 test `a_mode_the_project_no_longer_allows_is_refused`; Task W2 test `a_mode_the_project_disallowed_is_shown_disabled_and_not_sent`.)
4. **Fork pressed on a conversation whose last turn was stopped** — the person expects a clear refusal, not a fork that has no session to continue. (Task B6 test `a_stopped_last_turn_is_not_a_fork_point`.)
5. **The daemon restarted between two turns** — remembered model/effort, allowed modes, the thread's harness lock, and the last context figure all survive. (Task B5 test `settings_and_limits_survive_a_restart`.)

---

## Execution map

```text
Controller:  Task 0 (measure) → Task 1 (contract draft, commit)
                         ├── backend worktree:  B1 → B2 → B3 → B4 → B5 → B6
                         └── web worktree:      W1 → W2 → W3 → W4
Controller:  Task I (merge, regenerate, reconcile, whole-branch review, run with Mohammed)
```

Both tracks start from the commit Task 1 makes on `milestone-1/harness-controls`.

---

### Task 0: Measure the four open facts (controller)

**Files:**
- Create: `docs/evidence/harness/CONTROLS_PROBE.md`

Runs the real `claude` (2.1.278 at `%USERPROFILE%\.local\bin\claude.exe`) from the scratchpad directory; each run costs one small turn.

- [ ] **Step 1: Fork session.** Run a turn with `--session-id <A>`, then `--resume <A> --fork-session "say ok"` with `--output-format stream-json --verbose`; record the `session_id` the fork's `system/init` line reports (expected: not A), then `--resume <A> "say ok"` again and record that it still succeeds.
- [ ] **Step 2: Effort per model.** For each of `sonnet`, `opus`, `haiku` and each of `low medium high xhigh max`, run `--model <m> --effort <e> "say ok"`; record which exit 0 with `subtype: success` and the exact error text of any that do not.
- [ ] **Step 3: Rate limit line.** Over the runs above, count how many turns emitted a `rate_limit_event` line.
- [ ] **Step 4: Answering model.** For the `--model haiku` run, record the last `assistant` line's `message.model` and the `modelUsage` keys.
- [ ] **Step 5: Write the evidence file** with a table per step, the commands, and the claude version; commit:

```bash
git add docs/evidence/harness/CONTROLS_PROBE.md
git commit -m "docs(evidence): measure fork-session, effort per model, rate-limit lines"
```

If Step 1 shows the source session is broken by a fork, stop and bring it to Mohammed: §12.7 changes.

---

### Task 1: The contract draft (controller)

**Files:**
- Modify: `api/openapi.json` (by hand)

- [ ] **Step 1: Add schemas** under `components.schemas`, keys sorted, matching the existing style (`required` lists, `description` from the spec):
  - `Choice { id, label, description, enabled: boolean, reason: string|null }`
  - `ModelChoice { id, label, description, enabled, reason, efforts: Choice[], default_effort: string }`
  - `LimitWindow { utilization: number, resets_at: integer }`
  - `AccountLimits { five_hour: LimitWindow|null, seven_day: LimitWindow|null, observed_at: string }`
  - `RememberedSettings { model: string, effort: string }`
  - `HarnessInfo { kind, label, available: boolean, models: ModelChoice[], modes: Choice[], default_model: string|null, default_mode: string|null, remembered: RememberedSettings|null, limits: AccountLimits|null }`
  - `InvocationView { harness_kind, harness_version, requested_model, requested_mode, requested_effort, observed_model: string|null, observed_models: string[], context_used: integer|null, context_window: integer|null, output_tokens: integer|null }`
  - `StartTurn { command_id, prompt, model, mode, effort }` (replaces `{ prompt }`)
  - `UpdateThread { command_id, harness }`, `UpdateProject { command_id, allowed_modes: { [harness]: string[] } }`, `ForkThread { command_id, at_entry_id }`
  - `Project` gains `allowed_modes: { [harness]: string[] }`; `PlanningThread` gains `harness: string`, `forked_from_thread: string|null`; `ThreadEntry` gains `operation_id: string|null`; `Operation` gains `invocation: InvocationView|null`; `CreateThread` gains optional `harness`.
  - `ErrorCode` enum gains the six codes in Global Constraints.
- [ ] **Step 2: Add paths:** `GET /api/harnesses` → `HarnessInfo[]`; `PATCH /api/threads/{id}` → `PlanningThread` (409 `HARNESS_LOCKED`); `PATCH /api/projects/{id}` → `Project`; `POST /api/threads/{id}/fork` → 201 `PlanningThread` (409 `THREAD_BUSY`, 422 `FORK_POINT_NOT_SUPPORTED`); on `POST /api/threads/{id}/turns` add 403 `MODE_NOT_ALLOWED`, 409 `THREAD_BUSY`, 409 `COMMAND_CONFLICT`, 422 `SETTING_NOT_OFFERED`/`HARNESS_UNAVAILABLE`.
- [ ] **Step 3: Extend the `/api/subscribe` description** with: `limits` frame, data `{ "harness": string, "five_hour": LimitWindow|null, "seven_day": LimitWindow|null, "observed_at": string }`; and that a `durable` frame of kind `OperationCompleted` carries `payload.invocation: InvocationView`.
- [ ] **Step 4: Validate and commit.** `npx --prefix web openapi-typescript api/openapi.json -o NUL` must succeed (the document parses). Then:

```bash
git add api/openapi.json
git commit -m "docs(api): draft the Milestone 1 contract by hand (spec §12.8)"
```

`cargo test --test openapi` now fails on purpose until B6 regenerates it; neither track treats that as a regression.

---

## Backend track (worktree `m1-backend`)

### Task B1: Migration and the storage shapes

**Files:**
- Create: `migrations/0005_harness_controls.sql`
- Modify: `src/thread/mod.rs`, `src/project/mod.rs`, `src/storage/sqlite/thread.rs`, `src/storage/sqlite/project.rs`, `src/storage/sqlite/mod.rs` (new `StorageError` variants)
- Test: `tests/harness_schema.rs`

**Interfaces:**
- Produces:
  - `PlanningThread { .., harness: String, forked_from_thread: Option<ThreadId> }`
  - `ThreadEntry { .., operation_id: Option<OperationId> }`, `NewThreadEntry { .., operation_id: Option<&'a OperationId> }`
  - `Project { .., allowed_modes: BTreeMap<String, Vec<String>> }`
  - `TurnContext { project_directory, harness_session_id, harness: String, project_id: ProjectId, fork_session_id: Option<String> }`
  - `Storage::create_planning_thread(&self, ctx: &CommandContext, project_id: &ProjectId, title: &str, harness: &str) -> Result<PlanningThread, StorageError>`
  - `Storage::create_project(&self, ctx, slug, name, directory, default_modes: &BTreeMap<String, Vec<String>>) -> Result<Project, StorageError>`
  - `Storage::set_thread_harness(&self, ctx: &CommandContext, thread: &ThreadId, harness: &str) -> Result<PlanningThread, StorageError>` — an idempotent command (`classify`/`record_command`); a replay answers the thread as it now stands
  - `Storage::set_project_modes(&self, ctx: &CommandContext, project: &ProjectId, modes: &BTreeMap<String, Vec<String>>) -> Result<Project, StorageError>`
  - `StorageError::{HarnessLocked, ThreadBusy, ForkPointNotSupported}`

- [ ] **Step 1: Write the failing tests** in `tests/harness_schema.rs` (fixture copied from `tests/thread_session.rs`'s `fixture()`; `create_project` gains `&modes()` where `fn modes() -> BTreeMap<String, Vec<String>> { [("claude-code".into(), vec!["acceptEdits".into(), "auto".into()])].into() }`):

```rust
#[tokio::test]
async fn a_thread_defaults_to_claude_code_and_its_harness_locks_at_its_first_operation() {
    let (fx, thread) = fixture().await;
    let t = fx.storage.set_thread_harness(&ctx("h1", "thread.harness"), &thread, "codex").await.unwrap();
    assert_eq!(t.harness, "codex");
    fx.storage.set_thread_harness(&ctx("h2", "thread.harness"), &thread, "claude-code").await.unwrap();
    fx.storage
        .create_pending_operation(&thread, &fx.runtime.instance_id)
        .await
        .unwrap();
    let locked = fx.storage.set_thread_harness(&ctx("h3", "thread.harness"), &thread, "codex").await;
    assert!(matches!(locked, Err(StorageError::HarnessLocked)));
    // The command that succeeded before the lock replays; it is not refused.
    let replay = fx.storage.set_thread_harness(&ctx("h2", "thread.harness"), &thread, "claude-code").await.unwrap();
    assert_eq!(replay.harness, "claude-code");
}

#[tokio::test]
async fn the_lock_holds_below_the_application() {
    let (fx, thread) = fixture().await;
    fx.storage.create_pending_operation(&thread, &fx.runtime.instance_id).await.unwrap();
    let raw = sqlx::query("UPDATE planning_thread SET harness_kind = 'codex' WHERE id = ?")
        .bind(thread.as_str())
        .execute(fx.storage.reader())
        .await;
    assert!(raw.is_err(), "the trigger must refuse a raw update too");
}

#[tokio::test]
async fn a_project_is_created_with_the_modes_it_was_given_and_they_can_be_replaced() {
    let (fx, _) = fixture().await;
    let p = &fx.project;
    assert_eq!(p.allowed_modes["claude-code"], vec!["acceptEdits", "auto"]);
    let only: BTreeMap<_, _> = [("claude-code".to_string(), vec!["acceptEdits".to_string()])].into();
    let p2 = fx.storage.set_project_modes(&ctx("m1", "project.modes"), &p.id, &only).await.unwrap();
    assert_eq!(p2.allowed_modes["claude-code"], vec!["acceptEdits"]);
}

#[tokio::test]
async fn an_entry_can_name_its_operation_and_old_entries_name_none() {
    let (fx, thread) = fixture().await;
    let op = fx.storage.create_pending_operation(&thread, &fx.runtime.instance_id).await.unwrap();
    let e = fx.storage.append_thread_entry(&thread, NewThreadEntry {
        kind: "AgentMessage", author: Actor::system(), body: "x", refs: &[], operation_id: Some(&op),
    }).await.unwrap();
    assert_eq!(e.operation_id.as_ref(), Some(&op));
    let e2 = fx.storage.append_thread_entry(&thread, NewThreadEntry {
        kind: "UserMessage", author: Actor::user("local"), body: "y", refs: &[], operation_id: None,
    }).await.unwrap();
    assert!(e2.operation_id.is_none());
}
```

(The tests read `create_pending_operation` as it exists today; B3 removes its external use but it stays as the storage primitive `start_turn` calls.)

- [ ] **Step 2: Run** `cargo test --test harness_schema` — expected: compile errors on the new fields and methods.

- [ ] **Step 3: Write `migrations/0005_harness_controls.sql`:**

```sql
ALTER TABLE planning_thread ADD COLUMN harness_kind TEXT NOT NULL DEFAULT 'claude-code';
ALTER TABLE planning_thread ADD COLUMN forked_from_thread TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT;
ALTER TABLE planning_thread ADD COLUMN forked_from_entry TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT;
ALTER TABLE planning_thread ADD COLUMN fork_session_id TEXT NULL;

CREATE TRIGGER planning_thread_fork_all_or_none
BEFORE INSERT ON planning_thread
WHEN NOT ((NEW.forked_from_thread IS NULL AND NEW.forked_from_entry IS NULL AND NEW.fork_session_id IS NULL)
       OR (NEW.forked_from_thread IS NOT NULL AND NEW.forked_from_entry IS NOT NULL AND NEW.fork_session_id IS NOT NULL))
BEGIN SELECT RAISE(ABORT, 'fork_columns_all_or_none'); END;

CREATE TRIGGER planning_thread_fork_immutable
BEFORE UPDATE OF forked_from_thread, forked_from_entry, fork_session_id ON planning_thread
BEGIN SELECT RAISE(ABORT, 'fork_columns_immutable'); END;

CREATE TRIGGER planning_thread_harness_locked
BEFORE UPDATE OF harness_kind ON planning_thread
WHEN NEW.harness_kind <> OLD.harness_kind
 AND EXISTS (SELECT 1 FROM operation WHERE thread_id = OLD.id)
BEGIN SELECT RAISE(ABORT, 'harness_locked'); END;

ALTER TABLE thread_entry ADD COLUMN operation_id TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT;

CREATE TABLE project_mode (
    project_id   TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    harness_kind TEXT NOT NULL,
    mode_id      TEXT NOT NULL,
    PRIMARY KEY (project_id, harness_kind, mode_id)
);
INSERT INTO project_mode (project_id, harness_kind, mode_id)
    SELECT id, 'claude-code', 'acceptEdits' FROM project
    UNION ALL SELECT id, 'claude-code', 'auto' FROM project;

CREATE TABLE agent_invocation (
    id                TEXT PRIMARY KEY,
    operation_id      TEXT NOT NULL UNIQUE REFERENCES operation(id) ON DELETE RESTRICT,
    role              TEXT NOT NULL,
    harness_kind      TEXT NOT NULL,
    harness_path      TEXT NOT NULL,
    harness_version   TEXT NOT NULL,
    requested_model   TEXT NOT NULL,
    requested_mode    TEXT NOT NULL,
    requested_effort  TEXT NOT NULL,
    profile_json      TEXT NOT NULL DEFAULT '{}',
    native_session_id TEXT NULL,
    observed_model    TEXT NULL,
    observed_models   TEXT NULL,
    context_used      INTEGER NULL,
    context_window    INTEGER NULL,
    output_tokens     INTEGER NULL,
    created_at        TEXT NOT NULL
);
CREATE INDEX agent_invocation_by_operation ON agent_invocation(operation_id, created_at, id);

CREATE TRIGGER agent_invocation_requested_immutable
BEFORE UPDATE OF requested_model, requested_mode, requested_effort, harness_path, harness_version ON agent_invocation
BEGIN SELECT RAISE(ABORT, 'invocation_requested_immutable'); END;

CREATE TABLE harness_preference (
    harness_kind TEXT PRIMARY KEY,
    model        TEXT NOT NULL,
    effort       TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE harness_limit (
    harness_kind          TEXT PRIMARY KEY,
    five_hour_utilization REAL NULL,
    five_hour_resets_at   INTEGER NULL,
    seven_day_utilization REAL NULL,
    seven_day_resets_at   INTEGER NULL,
    observed_at           TEXT NOT NULL
);
```

- [ ] **Step 4: Implement** the fields and methods in `thread.rs` and `project.rs` (read `harness_kind`, `forked_from_thread`, `operation_id`). `allowed_modes` is a **set**: read it `ORDER BY harness_kind, mode_id` (an explicit key; never `rowid` — CLAUDE.md ordering rule), and say in the field's doc comment that list order carries no meaning. Clients show modes in catalogue order, not this order. Map a SQLite error whose message contains `harness_locked` to `StorageError::HarnessLocked` in `set_thread_harness`, which also checks `EXISTS operation` first and returns `HarnessLocked` without relying on the trigger. `set_project_modes` is an idempotent command (`classify`/`record_command` exactly as `create_project` uses them) that deletes and reinserts the project's rows for the harnesses named.

- [ ] **Step 5: Fix every existing caller** of `create_planning_thread`, `create_project`, `NewThreadEntry` (pass `"claude-code"`, the default modes map, `operation_id: None`). Run `cargo test` — expected: all green, including `harness_schema`.

- [ ] **Step 6: Regenerate the code map and commit.**

```bash
UPDATE_CODEMAP=1 cargo test --test codemap
git add migrations/0005_harness_controls.sql src tests/harness_schema.rs docs/codebase/inventory.md
git commit -m "feat(storage): migration 0005 for harness controls (spec §12.3-§12.5)"
```

### Task B2: The catalogue and `GET /api/harnesses`

**Files:**
- Create: `src/agent/catalogue.rs`, `src/protocol/harness.rs`, `src/storage/sqlite/harness.rs`
- Modify: `src/agent/mod.rs` (`pub mod catalogue; pub struct TurnSettings`), `src/protocol/mod.rs` (route), `src/protocol/openapi.rs` (register), `docs/codebase/README.md` (owners of the new files)
- Test: `tests/harness_catalogue.rs`

**Interfaces:**
- Produces:
  - `agent::TurnSettings { pub model: String, pub mode: String, pub effort: String }` (Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)
  - `agent::catalogue::{CLAUDE_CODE, CODEX}: &str`
  - `agent::catalogue::Choice`, `ModelChoice`, `HarnessCatalogue` (fields as Task 1's `Choice`, `ModelChoice`, and `HarnessInfo` minus `remembered`/`limits`)
  - `agent::catalogue::catalogue() -> Vec<HarnessCatalogue>`
  - `agent::catalogue::find(kind: &str) -> Option<HarnessCatalogue>`
  - `agent::catalogue::default_modes() -> BTreeMap<String, Vec<String>>` (every enabled mode of every available harness)
  - `enum SettingRefusal { HarnessUnavailable, NotOffered { what: &'static str, id: String } }`
  - `agent::catalogue::validate(kind: &str, s: &TurnSettings) -> Result<(), SettingRefusal>`
  - `Storage::remembered_settings(&self, kind: &str) -> Result<Option<RememberedSettings>, StorageError>`
  - `Storage::latest_limits(&self, kind: &str) -> Result<Option<StoredLimits>, StorageError>`

- [ ] **Step 1: Write failing unit tests** at the bottom of `src/agent/catalogue.rs` (`#[cfg(test)] mod tests`):

```rust
#[test]
fn claude_lists_exactly_accept_edits_and_auto_with_accept_edits_default() {
    let c = find(CLAUDE_CODE).unwrap();
    let ids: Vec<_> = c.modes.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["acceptEdits", "auto"]);
    assert_eq!(c.default_mode.as_deref(), Some("acceptEdits"));
    assert_eq!(c.default_model.as_deref(), Some("sonnet"));
}

#[test]
fn codex_is_listed_and_not_runnable() {
    let c = find(CODEX).unwrap();
    assert!(!c.available);
    assert!(c.models.is_empty() && c.modes.is_empty());
    let s = TurnSettings { model: "x".into(), mode: "y".into(), effort: "z".into() };
    assert!(matches!(validate(CODEX, &s), Err(SettingRefusal::HarnessUnavailable)));
}

#[test]
fn validate_names_what_was_not_offered() {
    let ok = TurnSettings { model: "sonnet".into(), mode: "acceptEdits".into(), effort: "high".into() };
    assert!(validate(CLAUDE_CODE, &ok).is_ok());
    for (s, what) in [
        (TurnSettings { model: "gpt".into(), ..ok.clone() }, "model"),
        (TurnSettings { mode: "bypassPermissions".into(), ..ok.clone() }, "mode"),
        (TurnSettings { effort: "extreme".into(), ..ok.clone() }, "effort"),
    ] {
        assert!(matches!(validate(CLAUDE_CODE, &s), Err(SettingRefusal::NotOffered { what: w, .. }) if w == what));
    }
}

#[test]
fn every_model_default_effort_is_one_it_accepts() {
    for m in find(CLAUDE_CODE).unwrap().models {
        assert!(m.efforts.iter().any(|e| e.id == m.default_effort), "{}", m.id);
    }
}
```

- [ ] **Step 2: Run** `cargo test --lib catalogue` — expected: compile failure (module missing).

- [ ] **Step 3: Implement `catalogue.rs`.** Claude's models are `sonnet`, `opus`, `haiku` in that order; each model's `efforts` is the list Task 0 recorded as accepted for it, labels `Low`/`Medium`/`High`/`Extra high`/`Max`, `default_effort` = `high` where accepted, else the highest accepted below it. Modes:

```rust
Choice { id: "acceptEdits".into(), label: "Accept edits".into(),
         description: "Edits files without asking; anything else that needs approval is refused".into(),
         enabled: true, reason: None },
Choice { id: "auto".into(), label: "Auto".into(),
         description: "Claude decides what is safe to do without asking".into(),
         enabled: true, reason: None },
```

The module doc states: the catalogue is what Shadows supports, not what the account may use (§12.2).

- [ ] **Step 4: `storage/sqlite/harness.rs`** with `remembered_settings`, `latest_limits`, and `pub(in crate::storage) async fn remember_settings(conn, kind, &TurnSettings, ts)` / `pub async fn record_limits(&self, kind: &str, limits: &AccountLimits) -> Result<(), StorageError>` (upsert, latest wins). `AccountLimits` and `LimitWindow` live in `agent::observation` (B4 creates the file; create it now with just these two types).

- [ ] **Step 5: `protocol/harness.rs`:** `GET /api/harnesses` → `Vec<HarnessInfo>` = each catalogue entry + `remembered` + `limits` (`observed_at` from the row). Write the integration test in `tests/harness_catalogue.rs` using the router the way `tests/protocol.rs` does:

```rust
#[tokio::test]
async fn harnesses_route_lists_claude_runnable_and_codex_not() {
    let app = test_app().await;
    let body: Vec<serde_json::Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(body[0]["kind"], "claude-code");
    assert_eq!(body[0]["available"], true);
    assert_eq!(body[0]["default_mode"], "acceptEdits");
    assert!(body[0]["remembered"].is_null());
    assert_eq!(body[1]["kind"], "codex");
    assert_eq!(body[1]["available"], false);
}
```

- [ ] **Step 6:** `cargo test`, clippy (`cargo clippy --all-targets -- -D warnings`), code map, owners in `docs/codebase/README.md` (`agent/catalogue.rs`: "the choices each harness offers"; `protocol/harness.rs`: "the harness catalogue route"; `storage/sqlite/harness.rs`: "per-harness remembered settings and limits"). Commit `feat(agent): harness catalogue and GET /api/harnesses (spec §12.2)`.

### Task B3: Starting a turn as one command

**Files:**
- Create: `src/storage/sqlite/turn.rs`, `tests/turn_command.rs`
- Modify: `src/protocol/conversation.rs` (`StartTurn`, `start`), `src/planner/spawn.rs` (takes a committed operation), `src/agent/mod.rs` (`AgentInvocation` gains `mode`, `effort`, `fork_session_id`), `src/agent/claude.rs` (flags), `src/error.rs` + `src/protocol/failure.rs` (six codes), `src/storage/sqlite/operation.rs` (`create_pending_operation` becomes `pub(in crate::storage)` or stays for tests only), `src/planner/mod.rs` (agent entries carry `operation_id`), `src/bin/fake_claude.rs` (unchanged prompts; `report-invocation` already echoes args)
- Test: `tests/turn_command.rs`; update `tests/thread_session.rs`, `tests/planner_turn.rs` and every caller of `PlannerTurn::start`

**Interfaces:**
- Consumes: B1 (`TurnContext`, `NewThreadEntry.operation_id`), B2 (`TurnSettings`, `validate`, `remember_settings`).
- Produces:
  - `pub struct NewTurn<'a> { pub thread_id: &'a ThreadId, pub runtime: &'a RuntimeInstanceId, pub prompt: &'a str, pub role: &'a str, pub harness_kind: &'a str, pub harness_path: &'a str, pub harness_version: &'a str, pub settings: &'a TurnSettings }`
  - `pub struct StartedTurn { pub operation_id: OperationId, pub entry_id: ThreadEntryId, pub replayed: bool }`
  - `Storage::start_turn(&self, ctx: &CommandContext, turn: NewTurn<'_>) -> Result<StartedTurn, StorageError>` — returns `ThreadBusy` when a non-terminal operation exists on the thread; one transaction as §12.5; on replay returns the recorded ids with `replayed: true`.
  - `PlannerTurnRequest { thread_id: ThreadId, operation_id: OperationId, prompt: String, settings: TurnSettings }` — `PlannerTurn::start` no longer creates the operation; it begins at Prepare.
  - `ClaudeHarness::to_process_spec` adds `--permission-mode <mode> --effort <effort>`, and when `resume_session_id` is `None` and `fork_session_id` is `Some(f)`: `--resume f --fork-session` instead of `--session-id`.
  - `ErrorCode::{HarnessUnavailable, SettingNotOffered, ModeNotAllowed, HarnessLocked, ThreadBusy, ForkPointNotSupported}` with the HTTP statuses in Global Constraints.

- [ ] **Step 1: Write the failing tests** in `tests/turn_command.rs` (fixture as `thread_session.rs`, with a helper `start(fx, cmd_id, prompt, settings) -> Result<StartedTurn, StorageError>` that calls `fx.storage.start_turn`, and `http_start(app, thread, body) -> (StatusCode, Value)` over the router):

```rust
fn sonnet_edits() -> TurnSettings {
    TurnSettings { model: "sonnet".into(), mode: "acceptEdits".into(), effort: "high".into() }
}

#[tokio::test]
async fn one_command_writes_entry_operation_invocation_and_record_together() {
    let (fx, thread) = fixture().await;
    let started = start(&fx, "t1", "hello", &sonnet_edits()).await.unwrap();
    let entries = fx.storage.list_thread_entries(&thread).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].operation_id.as_ref(), Some(&started.operation_id));
    let ops = fx.storage.list_operations_for_thread(&thread).await.unwrap();
    let inv = ops[0].invocation.as_ref().expect("invocation written with Pending");
    assert_eq!((inv.requested_model.as_str(), inv.requested_mode.as_str(), inv.requested_effort.as_str()),
               ("sonnet", "acceptEdits", "high"));
    assert_eq!(inv.harness_version, "fake-1");
    assert!(inv.observed_model.is_none());
}

#[tokio::test]
async fn a_replayed_turn_start_starts_nothing_and_returns_the_first_operation() {
    let (fx, thread) = fixture().await;
    let first = start(&fx, "t1", "hello", &sonnet_edits()).await.unwrap();
    fx.storage.mark_operation_failed(&first.operation_id, FailureStage::Prepare, "test").await.unwrap();
    let again = start(&fx, "t1", "hello", &sonnet_edits()).await.unwrap();
    assert!(again.replayed);
    assert_eq!(again.operation_id, first.operation_id);
    assert_eq!(fx.storage.list_thread_entries(&thread).await.unwrap().len(), 1);
    assert_eq!(fx.storage.list_operations_for_thread(&thread).await.unwrap().len(), 1);
}

#[tokio::test]
async fn the_same_command_id_with_another_body_is_a_conflict() {
    let (fx, _) = fixture().await;
    start(&fx, "t1", "hello", &sonnet_edits()).await.unwrap();
    let other = start(&fx, "t1", "hello", &TurnSettings { model: "opus".into(), ..sonnet_edits() }).await;
    assert!(matches!(other, Err(StorageError::CommandConflict)));
}

#[tokio::test]
async fn a_second_turn_while_one_is_running_is_thread_busy() {
    let (fx, _) = fixture().await;
    start(&fx, "t1", "hello", &sonnet_edits()).await.unwrap();
    assert!(matches!(start(&fx, "t2", "again", &sonnet_edits()).await, Err(StorageError::ThreadBusy)));
}

#[tokio::test]
async fn a_mode_the_project_no_longer_allows_is_refused() {
    let app = test_app().await;
    patch_modes(&app, &["acceptEdits"]).await;
    let (status, body) = http_start(&app, &app.thread, json!({
        "command_id": "t1", "prompt": "hi", "model": "sonnet", "mode": "auto", "effort": "high"
    })).await;
    assert_eq!(status, 403);
    assert_eq!(body["code"], "MODE_NOT_ALLOWED");
    assert!(app.storage.list_thread_entries(&app.thread).await.unwrap().is_empty(), "nothing durable");
}

#[tokio::test]
async fn an_effort_the_model_does_not_accept_is_refused() {
    // Uses the first (model, effort) pair Task 0 recorded as refused. If Task 0
    // found none, this test asserts an effort id outside the catalogue instead.
    let app = test_app().await;
    let (status, body) = http_start(&app, &app.thread, json!({
        "command_id": "t1", "prompt": "hi", "model": "haiku", "mode": "acceptEdits", "effort": "extreme"
    })).await;
    assert_eq!(status, 422);
    assert_eq!(body["code"], "SETTING_NOT_OFFERED");
}

#[tokio::test]
async fn a_replay_is_answered_even_after_the_mode_was_disallowed() {
    let app = test_app().await;
    let body = json!({ "command_id": "t1", "prompt": "hi", "model": "sonnet", "mode": "auto", "effort": "high" });
    let (s1, b1) = http_start(&app, &app.thread, body.clone()).await;
    assert_eq!(s1, 202);
    wait_terminal(&app, &b1["operation_id"]).await;
    patch_modes(&app, &["acceptEdits"]).await;
    let (s2, b2) = http_start(&app, &app.thread, body).await;
    assert_eq!(s2, 202);
    assert_eq!(b2["operation_id"], b1["operation_id"]);
}

#[tokio::test]
async fn the_harness_receives_the_chosen_mode_and_effort() {
    let app = test_app().await;
    let (_, b) = http_start(&app, &app.thread, json!({
        "command_id": "t1", "prompt": "report-invocation", "model": "opus", "mode": "auto", "effort": "max"
    })).await;
    let args = reported_args(&app, &b["operation_id"]).await; // parses the fake's JSON entry
    assert!(args.windows(2).any(|w| w == ["--permission-mode", "auto"]));
    assert!(args.windows(2).any(|w| w == ["--effort", "max"]));
    assert!(args.windows(2).any(|w| w == ["--model", "opus"]));
}
```

- [ ] **Step 2: Run** `cargo test --test turn_command` — expected: compile failure.

- [ ] **Step 3: Implement `Storage::start_turn`** in `storage/sqlite/turn.rs` inside one `write_txn`: `classify(conn, ctx, "thread", thread_id)` → on `Some(outcome_ref)` parse `{operation_id, entry_id}` and return `replayed: true`; else check no row `operation WHERE thread_id = ? AND status_kind NOT IN ('Completed','Failed','Cancelled','Interrupted')` (`ThreadBusy`); refuse if the runtime is stopped (the same check `create_pending_operation` makes); insert the operation (Pending) and its `OperationCreated` event exactly as `create_pending_operation` does (extract its body into a `pub(in crate::storage) async fn insert_pending(conn, ..)` both call); append the user entry with `operation_id` (extract `append_entry_in(conn, ..)` from `append_thread_entry`); insert `agent_invocation`; call `harness::remember_settings`; `record_command(conn, ctx, "thread", thread_id, "operation", &json!({operation_id, entry_id}).to_string(), ts)`. The fingerprint is computed by the caller from `{thread_id, prompt, model, mode, effort}`.

- [ ] **Step 4: Rewrite `protocol/conversation.rs::start`, replay first.** Order:
  1. build the `CommandContext` and fingerprint;
  2. **`storage.replayed_turn(&ctx)`**: a recorded command with the same fingerprint returns its `StartedTurn` at once, with no validation and no spawn — **even while the daemon is stopping**, because a replay reads a recorded result and starts no work; a different fingerprint is `CommandConflict`;
  3. only for a new command: check `is_closed` (`RuntimeStopping`);
  4. load `turn_context`, `catalogue::validate` (`SettingNotOffered`/`HarnessUnavailable`), the project's `allowed_modes` (`ModeNotAllowed`);
  5. `storage.start_turn` (which classifies again inside its transaction, so two concurrent first requests still produce one turn: the loser gets `replayed: true`; and refuses a new `Pending` for a stopped runtime, as today);
  6. spawn via `PlannerTurn::start` only when `replayed` is false.

  Anything checked before step 2 — today's rules, or the daemon stopping — would answer a replay with something other than the first result, which §12.5 forbids. Test in `tests/turn_command.rs`:

  ```rust
  #[tokio::test]
  async fn a_replay_is_answered_while_the_daemon_is_stopping() {
      let app = test_app().await;
      let body = json!({ "command_id": "t1", "prompt": "hi", "model": "sonnet", "mode": "acceptEdits", "effort": "high" });
      let (_, first) = http_start(&app, &app.thread, body.clone()).await;
      wait_terminal(&app, &first["operation_id"]).await;
      app.handles.close().await; // what shutdown does first
      let (s, again) = http_start(&app, &app.thread, body).await;
      assert_eq!((s, &again["operation_id"]), (202, &first["operation_id"]));
      let (s2, b2) = http_start(&app, &app.thread, json!({
          "command_id": "t2", "prompt": "new", "model": "sonnet", "mode": "acceptEdits", "effort": "high" })).await;
      assert_eq!((s2, b2["code"].as_str()), (503, Some("RUNTIME_STOPPING")));
  }
  ```

  (`LiveHandles::close` is `pub(crate)`; the test reaches it through the existing `test-support` feature — add a feature-gated `pub async fn close_for_test` beside `contains` if needed.) `StartTurn` becomes `{ command_id: String, prompt: String, model: String, mode: String, effort: String }`. Add to Produces: `Storage::replayed_turn(&self, ctx: &CommandContext) -> Result<Option<StartedTurn>, StorageError>` (read-only; `CommandConflict` on a fingerprint mismatch).

- [ ] **Step 5: Rewrite `PlannerTurn::start`** to begin after TX #1: no `create_pending_operation`; build `AgentInvocation` from `request.settings` and `context.fork_session_id`; everything from `workspace(&context)` on is unchanged. The agent-entry write in `planner/mod.rs` passes `operation_id: Some(&reader_op)`. Update `claude.rs::to_process_spec` for `--permission-mode`, `--effort`, fork.

- [ ] **Step 6: Errors.** Add the six `ErrorCode` variants (serialized SCREAMING_SNAKE like the rest) and map in `failure.rs`: `StorageError::ThreadBusy` → 409, `HarnessLocked` → 409, `ForkPointNotSupported` → 422; add constructors `Failure::setting_not_offered(what, id)`, `Failure::harness_unavailable()`, `Failure::mode_not_allowed(mode)`.

- [ ] **Step 7:** Update every other caller of `PlannerTurn::start` (`tests/planner_turn.rs`, `tests/thread_session.rs`, `tests/recovery.rs`, `tests/shutdown.rs`, `tests/planner_isolation.rs`, …) to create the operation via `storage.start_turn` first; add a helper to `tests/fixtures/` if three or more files need it. `cargo test` all green; clippy; code map. Commit `feat(turns): start a turn as one idempotent command with its invocation (spec §12.5)`.

### Task B4: What the stream reports

**Files:**
- Modify: `src/agent/observation.rs` (created in B2), `src/agent/mod.rs` (`StreamItem`), `src/agent/claude.rs` (`classify`), `src/planner/mod.rs` (fold observations, write with completion, record limits, publish), `src/storage/sqlite/operation.rs` (`mark_operation_completed` gains the observation), `src/storage/sqlite/operation_read.rs` (`Operation.invocation`), `src/operation/mod.rs` (`Operation.invocation: Option<InvocationView>`), `src/protocol/sse.rs` (`limits` frame), `src/bin/fake_claude.rs` (new prompt `observe`)
- Create: `tests/fixtures/claude_observe.jsonl` (lines copied from Task 0's real stream, trimmed)
- Test: `tests/harness_observation.rs`, unit tests in `claude.rs`

**Interfaces:**
- Produces:
  - `StreamItem::Entry { uuid, role, text, model: Option<String> }`
  - `StreamItem::TurnEnd { subtype, stop_reason, usage: Option<TurnUsage> }`
  - `StreamItem::Limits(AccountLimits)`
  - `agent::observation::TurnUsage { pub context_used: Option<u64>, pub output_tokens: Option<u64>, pub windows: BTreeMap<String, u64>, pub models: Vec<String> }`
  - `agent::observation::TurnObservation { pub observed_model: Option<String>, pub observed_models: Vec<String>, pub context_used: Option<u64>, pub context_window: Option<u64>, pub output_tokens: Option<u64> }` with `pub fn from_turn(answer_model: Option<String>, usage: Option<&TurnUsage>) -> Self`
  - `Storage::mark_operation_completed(&self, op_id: &OperationId, outcome: serde_json::Value, observation: &TurnObservation) -> Result<(), StorageError>` — writes the invocation's observed columns in the same transaction, and puts `invocation` (the `InvocationView`) in the `OperationCompleted` event payload.
  - `operation::InvocationView` (serde + ToSchema, fields as Task 1).

- [ ] **Step 1: Unit tests in `claude.rs`** against fixture lines:

```rust
#[test]
fn result_line_gives_last_iteration_context_and_windows() {
    let line = r#"{"type":"result","subtype":"success","usage":{"input_tokens":9,"output_tokens":40,
      "iterations":[{"input_tokens":2,"cache_read_input_tokens":0,"cache_creation_input_tokens":100,"output_tokens":4},
                    {"input_tokens":3,"cache_read_input_tokens":100,"cache_creation_input_tokens":20,"output_tokens":36}]},
      "modelUsage":{"claude-sonnet-5":{"contextWindow":1000000},"claude-haiku-4-5":{"contextWindow":200000}}}"#;
    let StreamItem::TurnEnd { usage: Some(u), .. } = harness().classify(line) else { panic!() };
    assert_eq!(u.context_used, Some(123));      // 3 + 100 + 20, last iteration only, no output
    assert_eq!(u.output_tokens, Some(40));
    assert_eq!(u.windows["claude-sonnet-5"], 1_000_000);
    assert_eq!(u.models, ["claude-haiku-4-5", "claude-sonnet-5"]); // sorted
}

#[test]
fn a_result_without_iterations_reports_no_context_rather_than_a_guess() {
    let line = r#"{"type":"result","subtype":"success","usage":{"input_tokens":9,"output_tokens":4}}"#;
    let StreamItem::TurnEnd { usage: Some(u), .. } = harness().classify(line) else { panic!() };
    assert_eq!(u.context_used, None);
}

#[test]
fn assistant_line_carries_the_answering_model() {
    let line = r#"{"type":"assistant","uuid":"u","message":{"model":"claude-sonnet-5","content":[{"type":"text","text":"ok"}]}}"#;
    let StreamItem::Entry { model, .. } = harness().classify(line) else { panic!() };
    assert_eq!(model.as_deref(), Some("claude-sonnet-5"));
}

#[test]
fn rate_limit_event_becomes_limits() {
    let line = r#"{"type":"rate_limit_event","rate_limit_info":{"unifiedWindows":{
      "five_hour":{"utilization":0.75,"resetsAt":1790212200},"seven_day":{"utilization":0.91,"resetsAt":1790542800}}}}"#;
    let StreamItem::Limits(l) = harness().classify(line) else { panic!() };
    assert_eq!(l.five_hour.unwrap().resets_at, 1790212200);
    assert!((l.seven_day.unwrap().utilization - 0.91).abs() < 1e-9);
}

#[test]
fn observation_takes_the_window_of_the_model_that_answered() {
    let u = TurnUsage { context_used: Some(5), output_tokens: Some(1),
        windows: [("a".into(), 10), ("b".into(), 20)].into(), models: vec!["a".into(), "b".into()] };
    let o = TurnObservation::from_turn(Some("b".into()), Some(&u));
    assert_eq!(o.context_window, Some(20));
    let none = TurnObservation::from_turn(Some("c".into()), Some(&u));
    assert_eq!(none.context_window, None, "no window for an unreported model");
}
```

- [ ] **Step 2: Run** `cargo test --lib claude` — expected: compile failure.

- [ ] **Step 3: Implement** the three `classify` branches and `TurnObservation::from_turn`. `rate_limit_event` must no longer fall into `Operational`.

- [ ] **Step 4: Watcher.** In `planner/mod.rs` the reader keeps `answer_model: Option<String>` (overwritten by every `Entry` whose `model` is `Some`) and the `TurnEnd`'s `usage`; on `Limits` it calls `storage.record_limits(harness_kind, &limits)` and forwards the item on the bus. The success branch calls `mark_operation_completed(&op, outcome, &TurnObservation::from_turn(answer_model, usage.as_ref()))`. Failed and cancelled paths record no observation (NULLs = unavailable), stated in a comment citing §12.5.

- [ ] **Step 5: SSE.** `transient_event` maps `StreamItem::Limits(l)` to `event("limits")` with data `{ harness, five_hour, seven_day, observed_at }` (the time the watcher recorded it; carry it in the bus item as `Limits` → add `observed_at` to `AccountLimits`' SSE form, not to the storage type).

- [ ] **Step 6: `fake_claude` `observe` prompt** prints the lines of `tests/fixtures/claude_observe.jsonl` (an assistant line with `message.model`, a `rate_limit_event`, a result with `iterations` and `modelUsage`). Integration test in `tests/harness_observation.rs`:

```rust
#[tokio::test]
async fn a_completed_turn_records_what_the_stream_reported() {
    let app = test_app().await;
    let op = start_prompt(&app, "observe").await;
    let done = wait_terminal(&app, &op).await;
    let inv = done.invocation.unwrap();
    assert_eq!(inv.observed_model.as_deref(), Some("claude-sonnet-5"));
    assert!(inv.context_used.is_some() && inv.context_window.is_some());
    let limits = app.storage.latest_limits("claude-code").await.unwrap().unwrap();
    assert!(limits.seven_day.is_some());
}

#[tokio::test]
async fn a_turn_that_ends_without_usage_leaves_the_observation_unavailable() {
    let app = test_app().await;
    let op = start_prompt(&app, "plain").await; // fake's default: result without usage
    let inv = wait_terminal(&app, &op).await.invocation.unwrap();
    assert!(inv.context_used.is_none() && inv.context_window.is_none());
}
```

Also extend `tests/stream_frames.rs` with one test that a `limits` frame reaches a subscriber of that thread.

- [ ] **Step 7:** `cargo test`, clippy, code map. Commit `feat(agent): record the answering model, context and account limits from the stream (spec §12.6)`.

### Task B5: Thread and project routes

**Files:**
- Create: `src/protocol/thread.rs` (PATCH thread; B6 adds fork here)
- Modify: `src/protocol/project.rs` (`CreateThread.harness`, `PATCH /api/projects/{id}`), `src/protocol/mod.rs`, `src/protocol/openapi.rs`, `docs/codebase/README.md`
- Test: `tests/thread_routes.rs`

**Interfaces:**
- Consumes: B1 `set_thread_harness`, `set_project_modes`; B2 `catalogue::find`, `default_modes`.
- Produces: routes `PATCH /api/threads/{id}` (`UpdateThread { command_id, harness }`), `PATCH /api/projects/{id}` (`UpdateProject { command_id, allowed_modes }`); `create_project` passes `catalogue::default_modes()`.
- Uses B1's idempotent `set_thread_harness`. A replay with the same fingerprint answers the thread **as it now stands** with 200 and changes nothing, even if a turn has since locked it; `HarnessLocked` is only ever the answer to a new command.

- [ ] **Step 1: Failing tests** in `tests/thread_routes.rs`:

```rust
#[tokio::test]
async fn a_thread_can_be_created_on_a_harness_and_changed_until_its_first_turn() {
    let app = test_app().await;
    let t = create_thread(&app, json!({ "command_id": "c9", "title": "x", "harness": "codex" })).await;
    assert_eq!(t["harness"], "codex");
    let path = format!("/api/threads/{}", t["id"].as_str().unwrap());
    let (s, t2) = patch(&app, &path, json!({ "command_id": "h1", "harness": "claude-code" })).await;
    assert_eq!((s, t2["harness"].as_str()), (200, Some("claude-code")));
    start_ok(&app, t["id"].as_str().unwrap()).await;
    let (s3, b3) = patch(&app, &path, json!({ "command_id": "h2", "harness": "codex" })).await;
    assert_eq!((s3, b3["code"].as_str()), (409, Some("HARNESS_LOCKED")));
    // A retry of the command that succeeded before the turn is a replay, not a refusal.
    let (s4, b4) = patch(&app, &path, json!({ "command_id": "h1", "harness": "claude-code" })).await;
    assert_eq!((s4, b4["harness"].as_str()), (200, Some("claude-code")));
}

#[tokio::test]
async fn an_unknown_harness_or_mode_is_refused_when_set() {
    let app = test_app().await;
    let (s, b) = patch(&app, &format!("/api/threads/{}", app.thread), json!({ "command_id": "h1", "harness": "gemini" })).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
    let (s2, b2) = patch(&app, &format!("/api/projects/{}", app.project), json!({
        "command_id": "m1", "allowed_modes": { "claude-code": ["bypassPermissions"] } })).await;
    assert_eq!((s2, b2["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
}

#[tokio::test]
async fn a_turn_on_a_codex_thread_is_harness_unavailable() {
    let app = test_app().await;
    let t = create_thread(&app, json!({ "command_id": "c9", "title": "x", "harness": "codex" })).await;
    let (s, b) = http_start(&app, t["id"].as_str().unwrap(), json!({
        "command_id": "t1", "prompt": "hi", "model": "sonnet", "mode": "acceptEdits", "effort": "high" })).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("HARNESS_UNAVAILABLE")));
}

#[tokio::test]
async fn settings_and_limits_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app_at(dir.path()).await;
    start_and_finish(&app, "observe", json!({ "model": "opus", "mode": "auto", "effort": "max" })).await;
    patch(&app, &format!("/api/projects/{}", app.project), json!({
        "command_id": "m1", "allowed_modes": { "claude-code": ["acceptEdits"] } })).await;
    drop(app);
    let app = test_app_at(dir.path()).await; // same database file
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(h[0]["remembered"], json!({ "model": "opus", "effort": "max" }));
    assert!(h[0]["limits"]["seven_day"].is_object());
    let p: Vec<Value> = get_json(&app, "/api/projects").await;
    assert_eq!(p[0]["allowed_modes"]["claude-code"], json!(["acceptEdits"]));
    let ops: Vec<Value> = get_json(&app, &format!("/api/threads/{}/operations", app.thread)).await;
    assert!(ops[0]["invocation"]["context_used"].is_number());
}
```

- [ ] **Step 2:** Run — expected: 404/405 on the new routes.
- [ ] **Step 3:** Implement the routes in `protocol/thread.rs` and `protocol/project.rs`; validate harness against `catalogue::find` and modes against that harness's enabled mode ids (`SettingNotOffered`).
- [ ] **Step 4:** `cargo test`, clippy, code map, owners. Commit `feat(protocol): choose a thread's harness and a project's allowed modes (spec §12.3-§12.4)`.

### Task B6: Fork, then regenerate the contract

**Files:**
- Create: `src/storage/sqlite/fork.rs`, `tests/fork.rs`
- Modify: `src/protocol/thread.rs` (fork route), `src/planner/mod.rs` / `src/planner/spawn.rs` (record the stream's session on a fork's first turn), `api/openapi.json` (regenerated), `docs/codebase/*`

**Interfaces:**
- Consumes: B1 fork columns, B3 fork flags in `to_process_spec`.
- Produces: `Storage::fork_thread(&self, ctx: &CommandContext, source: &ThreadId, at_entry: &ThreadEntryId) -> Result<PlanningThread, StorageError>`; `POST /api/threads/{id}/fork` → 201 `PlanningThread`.

- [ ] **Step 1: Failing tests** in `tests/fork.rs`:

```rust
#[tokio::test]
async fn fork_copies_entries_keeps_their_operation_and_leaves_the_source_alone() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let src = entries(&app, &app.thread).await;
    let last = src.last().unwrap()["id"].as_str().unwrap().to_string();
    let (s, fork) = post(&app, &format!("/api/threads/{}/fork", app.thread), json!({ "command_id": "f1", "at_entry_id": last })).await;
    assert_eq!(s, 201);
    assert_eq!(fork["forked_from_thread"], app.thread.as_str());
    let copied = entries(&app, fork["id"].as_str().unwrap()).await;
    assert_eq!(copied.len(), src.len());
    assert_ne!(copied[0]["id"], src[0]["id"]);
    assert_eq!(copied.last().unwrap()["operation_id"], src.last().unwrap()["operation_id"]);
    assert_eq!(entries(&app, &app.thread).await, src, "source unchanged");
    assert!(get_json::<Vec<Value>>(&app, &format!("/api/threads/{}/operations", fork["id"].as_str().unwrap())).await.is_empty());
}

#[tokio::test]
async fn the_forks_first_turn_resumes_the_source_session_as_a_new_one() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let fork = fork_last(&app).await;
    let args = run_and_report_args(&app, fork["id"].as_str().unwrap()).await; // prompt report-invocation
    let src_session = app.storage.turn_context(&app.thread).await.unwrap().harness_session_id.unwrap();
    assert!(args.windows(2).any(|w| w == ["--resume", src_session.as_str()]));
    assert!(args.iter().any(|a| a == "--fork-session"));
    let fork_ctx = app.storage.turn_context(&ThreadId::from_literal(fork["id"].as_str().unwrap())).await.unwrap();
    assert!(fork_ctx.harness_session_id.is_some());
    assert_ne!(fork_ctx.harness_session_id.unwrap(), src_session);
}

#[tokio::test]
async fn only_the_last_completed_entry_is_a_fork_point() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let first = entries(&app, &app.thread).await[0]["id"].as_str().unwrap().to_string();
    let (s, b) = post(&app, &format!("/api/threads/{}/fork", app.thread), json!({ "command_id": "f1", "at_entry_id": first })).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("FORK_POINT_NOT_SUPPORTED")));
}

#[tokio::test]
async fn a_stopped_last_turn_is_not_a_fork_point() {
    let app = test_app().await;
    let op = start_prompt(&app, "hang").await;
    wait_for_entry(&app).await;
    stop(&app, &op).await;
    let (s, b) = fork_last_raw(&app).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("FORK_POINT_NOT_SUPPORTED")));
}

#[tokio::test]
async fn fork_while_a_turn_runs_is_thread_busy_and_a_replay_returns_the_same_fork() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let a = fork_last(&app).await;
    let b = fork_last(&app).await; // same command_id "f1"
    assert_eq!(a["id"], b["id"]);
    let _running = start_prompt(&app, "hang").await;
    let (s, body) = fork_last_raw_with(&app, "f2").await;
    assert_eq!((s, body["code"].as_str()), (409, Some("THREAD_BUSY")));
}
```

(`fake_claude` must report a `session_id` in a `system`/`init` line that differs from the `--resume` argument when `--fork-session` is present; add that to it in this task.)

- [ ] **Step 2:** Run — expected: route missing.
- [ ] **Step 3: `fork_thread`** in one transaction: `classify` for replay (return the recorded thread); `ThreadBusy` if a non-terminal operation exists on the source; `ForkPointNotSupported` unless `at_entry` is the highest-ordinal entry, has `operation_id`, that operation is `Completed`, and the source has `harness_session_id`; insert the thread (same project, same `harness_kind`, title `"<source title> (fork)"`, the three fork columns); copy entries with new ids, ordinals 1..n, same `kind`, `author`, `body`, `refs_json`, `operation_id`; set `next_entry_ordinal`; `record_command`.
- [ ] **Step 4: Session on a fork's first turn.** `TurnWatch.new_session` becomes `enum NewSession { Keep, Chosen(String), FromStream }`: `FromStream` when the thread has no session and has `fork_session_id`; the reader records the `session` of the first `Operational { session: Some(_) }` once turn-end is reached (same rule as `Chosen`).
- [ ] **Step 5: Regenerate the contract.** `UPDATE_OPENAPI=1 cargo test --test openapi`, then `git diff api/openapi.json` against Task 1's commit. Every difference goes into the report with its reason. Semantic differences (a field name, type, nullability, status code, or path differing from the draft) are fixed in Rust to match the draft unless the draft is wrong — then stop and report (§12.10 rule 4). Cosmetic differences (descriptions, ordering utoipa imposes) are accepted and listed.
- [ ] **Step 6:** `cargo test` (all, now including `openapi`), clippy, code map, owners. Commit `feat(threads): fork from the last completed message; regenerate the contract (spec §12.7)`.

---

## Web track (worktree `m1-web`)

The web agent works from the draft `api/openapi.json` (Task 1). First step of W1 is `npm run gen:api` in `web/`. It never edits `api/openapi.json`.

### Task W1: API functions, queries, UI primitives

**Files:**
- Modify: `web/src/api/schema.d.ts` (generated), `web/src/api/client.ts`, `web/src/api/queries.ts`, `web/src/stream/frames.ts`, `web/src/stream/thread-stream.ts`
- Create: `web/src/components/ui/dropdown-menu.tsx`, `web/src/components/ui/tooltip.tsx` (via `npx shadcn@latest add dropdown-menu tooltip`, Base UI registry as the existing components), `web/src/test/contract-fixtures.ts`
- Test: `web/src/stream/frames.test.ts`, `web/src/api/client.test.ts`

**Interfaces:**
- Produces (in `client.ts`):
  - `type HarnessInfo`, `Choice`, `ModelChoice`, `AccountLimits`, `InvocationView`, `TurnSettings = { model: string; mode: string; effort: string }`
  - `listHarnesses(): Promise<HarnessInfo[]>`
  - `startTurn(threadId: string, commandId: string, prompt: string, settings: TurnSettings): Promise<string>` — the id is the caller's; this function never makes one
  - `setThreadHarness(threadId: string, commandId: string, harness: string): Promise<PlanningThread>`
  - `setProjectModes(projectId: string, allowedModes: Record<string, string[]>): Promise<Project>`
  - `forkThread(threadId: string, atEntryId: string): Promise<PlanningThread>`
  - `harnessesQuery` in `queries.ts` (key `['harnesses']`)
  - `parseLimits(data: string): Limits` in `frames.ts`; `ThreadStream` emits `{ type: 'limits', limits }`
  - `contract-fixtures.ts`: `claudeHarness: HarnessInfo`, `codexHarness: HarnessInfo`, `projectWithModes(modes)`, `completedOperation(invocation)` — typed against `schema.d.ts` so a contract change breaks them at compile time.

- [ ] **Step 1: Failing tests:**

```ts
// frames.test.ts
it('parses a limits frame', () => {
  const l = parseLimits('{"harness":"claude-code","five_hour":{"utilization":0.75,"resets_at":1790212200},"seven_day":null,"observed_at":"2026-09-24T02:49:00Z"}')
  expect(l.fiveHour?.utilization).toBe(0.75)
  expect(l.sevenDay).toBeNull()
})
it('refuses a limits frame without observed_at', () => {
  expect(() => parseLimits('{"harness":"claude-code","five_hour":null,"seven_day":null}')).toThrow(FrameError)
})

// client.test.ts
it('startTurn sends the settings and the caller\'s command id', async () => {
  const bodies: unknown[] = []
  vi.stubGlobal('fetch', async (r: Request) => { bodies.push(await r.json()); return Response.json({ operation_id: 'op1' }, { status: 202 }) })
  await startTurn('t1', 'cmd-1', 'hi', { model: 'sonnet', mode: 'acceptEdits', effort: 'high' })
  expect(bodies[0]).toEqual({ command_id: 'cmd-1', prompt: 'hi', model: 'sonnet', mode: 'acceptEdits', effort: 'high' })
})
```

- [ ] **Step 2:** `npm test` — expected: failures on missing exports.
- [ ] **Step 3:** Implement; add the shadcn components; `npm run typecheck && npm run lint && npm test` green. Commit `feat(web): API for harnesses, settings, fork; limits frame (spec §12.8)`.

### Task W2: CLI picker and composer bar

**Files:**
- Create: `web/src/app/conversation/turn-settings.ts` (pure state), `web/src/app/conversation/cli-picker.tsx`, `web/src/app/conversation/composer-bar.tsx`
- Modify: `web/src/app/conversation/composer.tsx` (sends settings; "Runs in" line moves into the bar), `web/src/app/conversation/conversation.tsx` (header picker)
- Test: `web/src/app/conversation/turn-settings.test.ts`, `web/src/app/conversation/composer-bar.test.tsx`

**Interfaces:**
- Consumes: W1 `harnessesQuery`, `startTurn`, `setThreadHarness`, fixtures.
- Produces:
  - `initialSettings(h: HarnessInfo, allowed: string[]): TurnSettings` — model = `remembered.model` if offered else `default_model`; effort = `remembered.effort` if the model accepts it else the model's `default_effort`; mode = `default_mode` if allowed, else the first allowed enabled mode.
  - `withModel(h: HarnessInfo, s: TurnSettings, model: string): TurnSettings` — keeps effort if the new model accepts it, else its `default_effort`.
  - `modeState(h: HarnessInfo, allowed: string[]): { id: string; label: string; enabled: boolean; reason: string | null }[]` — disabled with reason "Not allowed in this project" when not in `allowed`.

- [ ] **Step 1: Failing unit tests** (`turn-settings.test.ts`):

```ts
it('starts from remembered model and effort but always the default mode', () => {
  const h = { ...claudeHarness, remembered: { model: 'opus', effort: 'max' } }
  expect(initialSettings(h, ['acceptEdits', 'auto'])).toEqual({ model: 'opus', mode: 'acceptEdits', effort: 'max' })
})
it('changing_model_resets_an_effort_it_does_not_accept', () => {
  const h = claudeHarness // haiku's efforts in the fixture exclude 'max' (Task 0 data)
  expect(withModel(h, { model: 'opus', mode: 'acceptEdits', effort: 'max' }, 'haiku').effort)
    .toBe(h.models.find((m) => m.id === 'haiku')!.default_effort)
})
it('a_mode_the_project_disallowed_is_shown_disabled_and_not_sent', () => {
  const modes = modeState(claudeHarness, ['acceptEdits'])
  expect(modes.find((m) => m.id === 'auto')).toMatchObject({ enabled: false, reason: 'Not allowed in this project' })
  expect(initialSettings(claudeHarness, ['auto']).mode).toBe('auto')
})
```

- [ ] **Step 2: Failing app tests** (`composer-bar.test.tsx`, using `startApp` with answers for `GET /api/harnesses`, `GET /api/projects`, `GET /api/projects/p1/threads`, `GET /api/threads/t1/entries`, `GET /api/threads/t1/operations`, `POST /api/threads/t1/turns`, `PATCH /api/threads/t1`):

```tsx
it('sends the chosen model, mode and effort', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers())
  await until(() => app.button('Sonnet') !== undefined)
  // choose Opus from the model menu, Auto from the mode menu, type, Send
  await choose(app, 'Sonnet', 'Opus')
  await choose(app, 'Accept edits', 'Auto')
  typeInto(app.container.querySelector('textarea')!, 'hi')
  act(() => app.button('Send')!.click())
  await until(() => app.calls.includes('POST /api/threads/t1/turns'))
  expect(app.bodies.at(-1)).toMatchObject({ model: 'opus', mode: 'auto', prompt: 'hi' })
  app.unmount()
})
it('the CLI picker changes the harness before the first turn and shows a lock after', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({ operations: [] }))
  await choose(app, 'Claude Code', 'Claude Code') // Codex item is disabled: "Coming later"
  expect(menuItem('Codex')?.getAttribute('aria-disabled')).toBe('true')
  app.unmount()
  const locked = await startApp('/projects/p1/threads/t1', answers({ operations: [completedOperation(null)] }))
  await until(() => locked.container.querySelector('[aria-label="CLI locked for this conversation"]') !== null)
  locked.unmount()
})
it('a retried Send reuses the same command id', async () => {
  let n = 0
  const app = await startApp('/projects/p1/threads/t1', answers({
    start: () => (++n === 1 ? Promise.reject(new TypeError('network down')) : Response.json({ operation_id: 'op1' }, { status: 202 })) }))
  await until(() => app.button('Send') !== undefined)
  typeInto(app.container.querySelector('textarea')!, 'hi')
  act(() => app.button('Send')!.click())
  await until(() => app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length === 1)
  act(() => app.button('Send')!.click()) // the prompt is still there after a failed send
  await until(() => app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length === 2)
  const [a, b] = app.bodies.slice(-2) as { command_id: string }[]
  expect(b.command_id).toBe(a.command_id)
  app.unmount()
})
it('editing the prompt or a setting after a failed send makes a new command id', async () => {
  // same setup; after the failure, change the model, Send again → the two command_ids differ
})
it('shows the refusal when the daemon refuses the mode', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({
    start: () => Response.json({ code: 'MODE_NOT_ALLOWED', message: 'auto is not allowed' }, { status: 403 }) }))
  // send; expect the ErrorLine to show the message and the prompt to be kept
})
```

(`choose(app, trigger, item)` and `menuItem(label)` are helpers added to `test-app.tsx`.)

- [ ] **Step 3:** Implement per §12.9 and the agreed mockup (bar under the box: left `+`, mode, folder; right model, effort, ring slot). `+` is present and disabled with `title="Attachments come later"`. **The command id belongs to a pending send:** the composer keeps `{ commandId, prompt, settings }`, creates the id with `newCommandId()` when Send is pressed and no pending send matches the current prompt and settings, reuses it for any retry of that same send, and drops it on success or when the prompt or a setting changes. The CLI picker does the same for `setThreadHarness`.
- [ ] **Step 4:** typecheck, lint, test green. Commit `feat(web): CLI picker and composer bar (spec §12.9)`.

### Task W3: Context ring and observed model

**Files:**
- Create: `web/src/app/conversation/context-ring.tsx`, `web/src/app/conversation/usage.ts`
- Modify: `web/src/app/conversation/composer-bar.tsx` (mount the ring), `web/src/app/conversation/messages.tsx` (observed ≠ requested note), `web/src/app/conversation/use-conversation.ts` (latest invocation, limits from stream)
- Test: `web/src/app/conversation/usage.test.ts`, `web/src/app/conversation/context-ring.test.tsx`

**Interfaces:**
- Produces: `contextShown(inv: InvocationView | null): { used: number; window: number; percent: number } | null` (null unless both numbers are present; percent rounded); `formatTokens(n: number): string` (`81.4k`, `1M`); `resetIn(resetsAt: number, now: number): string` (`1h20m`, `3d21h`).

- [ ] **Step 1: Failing tests:**

```ts
it('shows context only when both numbers were reported', () => {
  expect(contextShown({ ...inv, context_used: 81400, context_window: 1_000_000 })).toEqual({ used: 81400, window: 1_000_000, percent: 8 })
  expect(contextShown({ ...inv, context_used: 81400, context_window: null })).toBeNull()
})
it('formats tokens and reset times', () => {
  expect(formatTokens(81400)).toBe('81.4k'); expect(formatTokens(1_000_000)).toBe('1M')
  expect(resetIn(1000 + 4800, 1000)).toBe('1h20m'); expect(resetIn(1000 + 3 * 86400 + 21 * 3600, 1000)).toBe('3d21h')
})
```

```tsx
it('hovering the ring shows context, limits and when they were observed; missing data says unavailable', async () => {
  // render ContextRing with invocation null and limits null → tooltip text includes 'Context unavailable'
  // render with numbers → includes '81.4k / 1M (8%)', '5h', 'Weekly', 'updated'
})
it('a reply whose observed model differs from the requested one says both', async () => {
  // operations answer with invocation { requested_model: 'haiku', observed_model: 'claude-sonnet-5' }
  // expect text 'Asked for haiku · ran on claude-sonnet-5'
})
```

- [ ] **Step 2–4:** implement, green, commit `feat(web): context ring, limits, observed model (spec §12.6, §12.9)`.

### Task W4: Message actions and allowed modes

**Files:**
- Create: `web/src/app/conversation/message-actions.tsx`, `web/src/app/project-modes.tsx`
- Modify: `web/src/app/conversation/messages.tsx`, `web/src/app/project-page.tsx`
- Test: `web/src/app/conversation/message-actions.test.tsx`, `web/src/app/project-modes.test.tsx`

- [ ] **Step 1: Failing tests:**

```tsx
it('copy is on every message and copies its text', async () => {
  const writes: string[] = []
  vi.stubGlobal('navigator', { clipboard: { writeText: async (t: string) => { writes.push(t) } } })
  // hover a message, click 'Copy' → writes = [its body]
})
it('fork shows only on the last message of an idle thread and opens the fork', async () => {
  // entries [u1, a1], operations [Completed]; 'Fork' button count === 1, on a1
  // click → POST /api/threads/t1/fork with at_entry_id a1 → router at /projects/p1/threads/<fork id>
})
it('fork is absent while a turn runs', async () => { /* operations [Running] → no 'Fork' button */ })
it('a refused fork shows the daemon message', async () => {
  // fork answers 422 FORK_POINT_NOT_SUPPORTED → ErrorLine shows it; URL unchanged
})
it('the project page edits allowed modes', async () => {
  // uncheck Auto → PATCH /api/projects/p1 with allowed_modes { 'claude-code': ['acceptEdits'] }
  // unchecking the last mode shows 'No mode left: turns cannot start' and still sends
})
```

- [ ] **Step 2–4:** implement (actions appear on hover and on keyboard focus; copy shows "Copied" for 1.5 s), green, commit `feat(web): copy, fork, and allowed modes (spec §12.7, §12.3)`.

---

## Task I: Integrate, review, run (controller)

- [ ] **Step 1: Merge.** On `milestone-1/harness-controls`: merge `m1-backend`, then `m1-web`. Conflict only possible in `api/openapi.json` (backend wins) and `docs/codebase/*` (backend owns).
- [ ] **Step 2: Reconcile the client.** In `web/`: `npm run gen:api`, `npm run typecheck`. Every type error is fixed in `web/` only, by a web agent dispatch that gets B6's difference list. Then `npm run lint && npm test && npm run build`.
- [ ] **Step 3: Full gate.** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (count recorded), web gate as above (count recorded).
- [ ] **Step 4: One whole-branch review** (opus): reviewer fixes what it finds, runs the gate, commits, reports (CLAUDE.md). The controller verifies the report — commits exist, diff matches, counts real — and does not re-review.
- [ ] **Step 5: Run it with Mohammed.** Build release, start `shadows serve --harness <claude> --debug`, start Vite, open the client. Walk §12.11 line by line with him; record the run in `docs/evidence/milestone1/ACCEPTANCE.md` (environment, gate counts, each acceptance line pass/fail, what is not established). Update `docs/status.md`.
- [ ] **Step 6:** Push, open the PR, CI green, merge, delete the branch (CLAUDE.md branches rule).
