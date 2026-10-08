# §23 PR 0 — Groundwork Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Store tool lines and subagent cards as their own entry kinds with a structured payload instead of `[tool: …]` / `[subagent: {json}]` text, and split the two failure files every feature grows — with no behaviour a person can see changing.

**Architecture:** Migration 0020 adds `thread_entry.card_json` and moves the recognised text bodies into two new `ThreadEntryKind`s, `ToolCall` and `Subagent`. The store, the fork copy and the turn writer use them; the API exposes `card`; the web client branches on `kind` instead of parsing bodies. Separately, `StorageError` moves to its own file, `shadows-http`'s `failure.rs` splits by what it maps, and `storage_contract.rs` becomes one test binary with modules.

**Tech Stack:** Rust (SQLx 0.9 / SQLite, axum, utoipa), React + TypeScript (vitest), `npm run gen:api`.

**Spec:** `docs/superpowers/specs/2026-10-08-guided-planning-design.md` §23.8 (amends §12.2; §22.2's body format).

## Global Constraints

- Branch `next/guided-planning`; no worktrees.
- Every cargo command is prefixed `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0`.
- No Rust line over 100 columns.
- Never edit an existing migration; the new one is `0020_entry_cards.sql`.
- No new integration-test file: each one is another ~49 MB test binary. Add tests to existing files.
- Targeted tests while working; the full gate (CLAUDE.md steps 1–6) runs once, in Task 6.
- When `api/openapi.json` changes, run `npm run gen:api` in `web/`. No prettier.
- A service change updates its `contract.yaml` in the same commit; a new module gets its row in `docs/codebase/README.md`.
- Push only when Mohammed says "ارفع"; push with `git -c http.version=HTTP/1.1 push`.

## Review Focus

1. **A database written before 0020** — its `[tool: …]` and `[subagent: {…}]` rows must read back as `ToolCall` / `Subagent` with the same title and card. Test: Task 4, migration test.
2. **A `[subagent: …]` body whose JSON is not valid** — must stay an `AgentMessage` and never break listing the thread. Test: Task 4, migration test, the `broken` row.
3. **A fork of a thread with cards** — the fork's entries keep kind and card. Test: Task 4, fork test.
4. **A running turn** — a tool or subagent entry appended mid-turn must still move the streamed reply on, as an `AgentMessage` did (`agentOrdinal` in `use-conversation.ts`). Test: Task 5.
5. **Copy on a tool line or a subagent card** — still copies the title / the report, not JSON. Test: Task 5.

---

### Task 1: `StorageError` gets its own file

`db/mod.rs` owns the pool and transactions; the error enum gains a variant with most features (§23 adds `StageBlocked`). Move it whole; change nothing in it.

**Files:**
- Create: `crates/shadows-core/src/db/error.rs`
- Modify: `crates/shadows-core/src/db/mod.rs` (remove `StorageError`, its `problems` helper and `impl From<sqlx::Error>`; add `mod error; pub use error::StorageError;`)
- Modify: `docs/codebase/README.md` (row: `crates/shadows-core/src/db/error.rs` | the failures a store can return | `crates/shadows-core/src/db/error.rs`)

**Interfaces:**
- Produces: `shadows_core::StorageError` unchanged at every existing path (`crate::db::StorageError` re-exported).

- [ ] **Step 1:** Cut lines from `pub enum StorageError` through the end of `impl From<sqlx::Error> for StorageError` (today `db/mod.rs` ~35–152) into `db/error.rs`, with a header `//! One job: the failures a store can return.` and the `use` lines those items need (copy them from `mod.rs`; the compiler names any missing).
- [ ] **Step 2:** In `db/mod.rs` add `mod error;` and `pub use error::StorageError;` beside `mod command; mod journal;`.
- [ ] **Step 3:** Build and run the storage tests:
  `cargo test -p shadows-core --test storage_contract`
  Expected: all pass, no warnings.
- [ ] **Step 4:** Add the codemap row; run `cargo test -p shadows --test codemap`. Expected: PASS.
- [ ] **Step 5:** Commit: `refactor(core): StorageError in its own file`.

### Task 2: `failure.rs` splits by what it maps

`crates/shadows-http/src/failure.rs` is 402 lines and takes a match arm per feature. Split by responsibility; behaviour unchanged.

**Files:**
- Create: `crates/shadows-http/src/failure/mod.rs` — `Failure`, `Detail`, `ErrorBody`, `impl Failure`, `IntoResponse`, `rejections_as_error_bodies`.
- Create: `crates/shadows-http/src/failure/storage.rs` — `impl From<StorageError> for Failure` only.
- Create: `crates/shadows-http/src/failure/other.rs` — `impl From<StartError>`, `From<CoreError>`, `From<DirectoryError>` for `Failure`.
- Delete: `crates/shadows-http/src/failure.rs`
- Modify: `docs/codebase/README.md` (replace the `failure.rs` row with three: "what a failure answers", "how a storage failure becomes a status", "how a start, core or directory failure becomes a status").

- [ ] **Step 1:** `git mv crates/shadows-http/src/failure.rs crates/shadows-http/src/failure/mod.rs`.
- [ ] **Step 2:** Move the `impl From<StorageError> for Failure` block into `failure/storage.rs` with header `//! One job: how a storage failure becomes a status code.` and `use super::{Detail, Failure};`. Fields `Failure` uses stay private to `failure`; give them `pub(super)` where `storage.rs` needs them.
- [ ] **Step 3:** Move the three other `From` impls into `failure/other.rs` the same way. Add `mod storage; mod other;` to `failure/mod.rs`.
- [ ] **Step 4:** `cargo test -p shadows-http` and `cargo test -p shadows --test openapi`. Expected: PASS; `git diff --exit-code api/` clean.
- [ ] **Step 5:** Codemap rows; `cargo test -p shadows --test codemap`. Expected: PASS.
- [ ] **Step 6:** Commit: `refactor(http): failure mapping split by what it maps`.

### Task 3: `storage_contract` becomes one binary with modules

Same binary name, so no new exe. The seven tests split by job.

**Files:**
- Create: `crates/shadows-core/tests/storage_contract/main.rs` — shared `use`s and helpers, `mod schema; mod txn; mod journal;`
- Create: `.../storage_contract/schema.rs` — `fresh_database_migrates_and_applies_the_connection_policy`
- Create: `.../storage_contract/txn.rs` — `state_and_event_commit_atomically_or_not_at_all`, `concurrent_read_then_write_transactions_all_succeed`, `write_txn_recovers_after_a_panicking_transaction`, `write_txn_waits_out_an_external_writer_holding_begin_immediate`
- Create: `.../storage_contract/journal.rs` — `event_provenance_round_trips_through_append_event`, `the_committed_signal_moves_on_commit_and_never_on_rollback`
- Delete: `crates/shadows-core/tests/storage_contract.rs`

- [ ] **Step 1:** Create the folder; move each test body verbatim into its module; helpers used by more than one module go in `main.rs` as `pub(crate) fn`.
- [ ] **Step 2:** `cargo test -p shadows-core --test storage_contract`. Expected: 7 passed.
- [ ] **Step 3:** If CLAUDE.md or docs name the file path `storage_contract.rs`, update them to `storage_contract/`.
- [ ] **Step 4:** Commit: `refactor(core tests): storage_contract split by job, one binary`.

### Task 4: `ToolCall` and `Subagent` entries with `card`

**Files:**
- Create: `crates/shadows-core/migrations/0020_entry_cards.sql`
- Modify: `crates/shadows-core/src/threads/model.rs` (`ThreadEntryKind`, `ThreadEntry.card`, `NewThreadEntry.card`)
- Modify: `crates/shadows-core/src/threads/store/entry.rs` (insert, `EntryRow`, `into_entry`, select)
- Modify: `crates/shadows-core/src/threads/store/fork.rs` (copy `card_json`)
- Modify: `crates/shadows-core/src/turns/turn.rs` (`persist`)
- Modify: every `NewThreadEntry { … }` (17 sites; the compiler lists them) — add `card: None`
- Modify: `crates/shadows-core/src/threads/contract.yaml`, `crates/shadows-core/src/turns/contract.yaml`
- Modify: `crates/shadows-agent/src/events.rs` (doc of `SubagentCard`), `crates/shadows-http/src/sse.rs` (the `subagent` frame text)
- Test: `crates/shadows-core/tests/plan_migration.rs` (migration), `crates/shadows/tests/planner_turn.rs` (bodies), the fork test in `crates/shadows/tests/thread_contract.rs`

**Interfaces:**
- Produces: `ThreadEntryKind::ToolCall` (body = tool title, `card` = `None`), `ThreadEntryKind::Subagent` (body = card title, `card` = the `SubagentCard` as JSON); `ThreadEntry.card: Option<serde_json::Value>`; `NewThreadEntry.card: Option<&'a serde_json::Value>`. Wire names `"ToolCall"`, `"Subagent"`.

- [ ] **Step 1: The failing migration test.** In `plan_migration.rs`, generalise `migrated_to_0011` to `async fn migrated_to(db: &Path, last: i64) -> SqlitePool` (filter `m.version <= last`; keep the 0012 test calling `migrated_to(&db, 11)`), update the file's `//!` to "Migrations on a database written before them: 0012 (§16.9), 0020 (§23.8).", and add:

```rust
/// §23.8: tool and subagent bodies become their own kinds; anything else,
/// including a subagent body whose JSON is broken, is left as it was.
#[tokio::test]
async fn migration_0020_moves_tool_and_subagent_bodies_into_their_kinds() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("shadows.sqlite3");
    let pool = migrated_to(&db, 19).await;
    exec(&pool, r#"INSERT INTO project (id, slug, name, directory, created_at)
        VALUES ('P','p','P','C:/p','2026-09-01T00:00:00Z')"#).await;
    exec(&pool, r#"INSERT INTO planning_thread (id, project_id, title, status, created_at,
        title_source, next_entry_ordinal)
        VALUES ('T','P','t','Open','2026-09-01T00:00:00Z','Default',5)"#).await;
    for (ordinal, body) in [
        (1, "[tool: Read notes.md]"),
        (2, r#"[subagent: {"id":"a","title":"List files","status":"completed","steps":[]}]"#),
        (3, "[subagent: {broken]"),
        (4, "plain text"),
    ] {
        sqlx::query("INSERT INTO thread_entry (id, thread_id, ordinal, kind, author_kind,
            author_id, body, created_at) VALUES (?, 'T', ?, 'AgentMessage', 'Agent',
            'Planner', ?, '2026-09-01T00:00:00Z')")
            .bind(format!("E{ordinal}")).bind(ordinal).bind(body)
            .execute(&pool).await.unwrap();
    }
    pool.close().await;
    let storage = Storage::open(&db).await.unwrap();
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT kind, body, card_json FROM thread_entry ORDER BY ordinal")
        .fetch_all(storage.reader()).await.unwrap();
    assert_eq!(rows[0], ("ToolCall".into(), "Read notes.md".into(), None));
    assert_eq!((rows[1].0.as_str(), rows[1].1.as_str()), ("Subagent", "List files"));
    let card: serde_json::Value = serde_json::from_str(rows[1].2.as_deref().unwrap()).unwrap();
    assert_eq!(card["id"], "a");
    assert_eq!(rows[2].0, "AgentMessage", "broken JSON stays as it was");
    assert_eq!(rows[3], ("AgentMessage".into(), "plain text".into(), None));
}
```

  Adjust the `planning_thread` column list to what 0019's schema requires (copy it from the 0012 test's insert); the compiler of SQL is the database, so a missing NOT NULL column fails loudly.
- [ ] **Step 2:** `cargo test -p shadows-core --test plan_migration migration_0020`. Expected: FAIL (`no such column: card_json`).
- [ ] **Step 3: The migration.** `0020_entry_cards.sql`:

```sql
-- §23.8 (amends §12.2): a tool line and a subagent card are their own entry
-- kinds; a card's structured payload is a column, not text in the body.
ALTER TABLE thread_entry ADD COLUMN card_json TEXT NULL;

-- `[tool: <title>]` → ToolCall, body = title. '[tool: ' is 7 characters.
UPDATE thread_entry
   SET kind = 'ToolCall',
       body = substr(body, 8, length(body) - 8)
 WHERE kind = 'AgentMessage' AND body LIKE '[tool: %]';

-- `[subagent: <json>]` → Subagent, body = its title. '[subagent: ' is 11
-- characters. A body whose JSON is not valid is left as it was.
UPDATE thread_entry
   SET kind = 'Subagent',
       card_json = substr(body, 12, length(body) - 12),
       body = coalesce(json_extract(substr(body, 12, length(body) - 12), '$.title'), '')
 WHERE kind = 'AgentMessage' AND body LIKE '[subagent: %]'
   AND json_valid(substr(body, 12, length(body) - 12));
```

- [ ] **Step 4: The model.** In `ThreadEntryKind` add, after `AgentMessage`:

```rust
    /// A tool the harness ran; its body is the tool's title (§23.8).
    ToolCall,
    /// A subagent's card (§22.2); its body is the card's title and `card`
    /// the card itself (§23.8).
    Subagent,
```

  and add both to `as_str` (`"ToolCall"`, `"Subagent"`) and to `parse`'s array. Add to `ThreadEntry`, after `refs`:

```rust
    /// A card's structured payload (§23.8): a `Subagent`'s card. `None` for
    /// every other kind.
    #[schema(value_type = Option<Object>, required)]
    pub card: Option<serde_json::Value>,
```

  and to `NewThreadEntry`: `pub card: Option<&'a serde_json::Value>,`.
- [ ] **Step 5: The store.** In `append_entry_in`: bind `card_json` (`entry.card.map(serde_json::to_string).transpose()?`) as an 11th column in the `INSERT`, and set `card: entry.card.cloned()` in the returned `ThreadEntry`. Extend `EntryRow` with a trailing `Option<String>`, add `card_json` at the end of the `SELECT` in `list_thread_entries` and its doc comment, and in `into_entry`: `card: r.10.as_deref().map(serde_json::from_str).transpose()?`. In `append_thread_entry`, carry `entry.card.cloned()` into the closure and pass `card: card.as_ref()`.
- [ ] **Step 6: The fork.** In `fork.rs`'s copy: add `card_json` to the `SELECT`, to `CopiedRow`, to the `INSERT` column list and its binds.
- [ ] **Step 7: The turn writer.** In `turns/turn.rs` `persist`, make the tuple `(kind, author, body, card)`:

```rust
            Durable::Tool(title) => (ThreadEntryKind::ToolCall, planner(), title, None),
            // §22.2: drawn as a card; §23.8: its payload is a column.
            Durable::Subagent(card) => (
                ThreadEntryKind::Subagent,
                planner(),
                card.title.clone(),
                serde_json::to_value(&*card).ok(),
            ),
```

  with `fn planner() -> Actor { Actor { kind: "Agent".into(), id: "Planner".into() } }` beside `persist`, `card: None` for `Message` and `PermissionRefused`, and `card: card.as_ref()` in the `NewThreadEntry`.
- [ ] **Step 8:** Add `card: None` at every other `NewThreadEntry { … }` the compiler names.
- [ ] **Step 9: The fork test.** In `crates/shadows/tests/thread_contract.rs`, beside the existing fork test, add one that appends a `Subagent` entry (`card: Some(&json!({"id":"a","title":"t"}))`) before the fork point, forks, and asserts the fork's entry has `kind == ThreadEntryKind::Subagent` and `card == Some(json!({"id":"a","title":"t"}))`. Copy the setup lines from the existing fork test in that file.
- [ ] **Step 10:** In `crates/shadows/tests/planner_turn.rs:227` the expected bodies become `["two-messages", "first", "Read notes.md", "second"]`; if that test compares kinds, the third is `ToolCall`.
- [ ] **Step 11:** Update the doc of `SubagentCard` in `shadows-agent/src/events.rs` and the `subagent` frame text in `shadows-http/src/sse.rs`: "Its entry, kind `Subagent` with this card as `card`, is written when it ends."
- [ ] **Step 12: Contracts.** `threads/contract.yaml`: an obligation "A `ToolCall` entry's body is its title and it has no card; a `Subagent` entry's body is its card's title and `card` is the card; a fork copies `card`", tested by `migration_0020_moves_tool_and_subagent_bodies_into_their_kinds` and the new fork test. `turns/contract.yaml`: the persist obligation names the new kinds.
- [ ] **Step 13:** Run:
  `cargo test -p shadows-core --test plan_migration`, `cargo test -p shadows-core --lib turns::`, `cargo test -p shadows --test thread_contract --test planner_turn --test contracts`.
  Expected: all PASS.
- [ ] **Step 14:** Commit: `§23.8: tool lines and subagent cards are their own entry kinds`.

### Task 5: The API and the web client read kinds, not bodies

**Files:**
- Modify: `api/openapi.json` (regenerated by `cargo test -p shadows --test openapi`, as the repo does today), `web/src/api/schema.d.ts` (`npm run gen:api`)
- Modify: `web/src/app/conversation/tool-text.ts`, `entry.tsx`, `messages.tsx`, `conversation.tsx`, `deleted-conversation.tsx`, `use-conversation.ts`
- Modify: `web/src/test/contract-fixtures.ts`
- Test: `web/src/app/conversation/message-actions.test.tsx`, `plan-in-conversation.test.tsx`, `subagent-card.test.tsx`, and the stream test that covers the reply (`use-conversation` or `reply` test — `grep -rl agent-entry web/src`)

**Interfaces:**
- Consumes: `ThreadEntry.kind` `'ToolCall' | 'Subagent'`, `ThreadEntry.card`.
- Produces: `toolTitle(entry: ThreadEntry): string | null`, `subagentOf(entry: ThreadEntry): SubagentCard | null`; fixtures `toolEntry(id, title)`, `subagentEntry(id, card)`.

- [ ] **Step 1:** Regenerate `api/openapi.json` and run `npm run gen:api` in `web/`. `ThreadEntryKind` gains `"ToolCall" | "Subagent"`; `ThreadEntry` gains `card`.
- [ ] **Step 2: Fixtures and failing tests.** In `contract-fixtures.ts` add:

```ts
export const toolEntry = (id: string, title: string) => entryOfKind(id, 'ToolCall', title)
export const subagentEntry = (id: string, card: Record<string, unknown>) => ({
  ...entryOfKind(id, 'Subagent', String(card.title)),
  card,
})
```

  (if `entryOfKind` builds `card`, default it to `null`). Replace `agentEntry('…', '[tool: X]')` with `toolEntry('…', 'X')` in `message-actions.test.tsx` and `plan-in-conversation.test.tsx`, and the `[subagent: …]` entry in `subagent-card.test.tsx` with `subagentEntry('e1', CARD)`. Add to `message-actions.test.tsx` a test that Copy on a `toolEntry('t1', 'npm test')` copies `npm test`, and to `subagent-card.test.tsx` that Copy on the card copies its report.
- [ ] **Step 3:** `npx vitest run src/app/conversation`. Expected: FAIL — tool lines and cards render as plain text.
- [ ] **Step 4: `tool-text.ts`.** Replace the two parsers and the predicates:

```ts
/** A tool call is a `ToolCall` entry whose body is its title (§23.8). */
export function toolTitle(entry: ThreadEntry): string | null {
  return entry.kind === 'ToolCall' ? entry.body : null
}

/** A subagent is a `Subagent` entry carrying its card (§22.2, §23.8). */
export function subagentOf(entry: ThreadEntry): SubagentCard | null {
  if (entry.kind !== 'Subagent' || entry.card == null) return null
  try {
    return readSubagentCard(entry.card)
  } catch {
    return null
  }
}

export const isTool = (entry: ThreadEntry) => entry.kind === 'ToolCall'

export function silent(entry: ThreadEntry): boolean {
  const tool = toolTitle(entry)
  return tool !== null && toolText(tool) === null
}

export function copyText(entry: ThreadEntry): string {
  const card = subagentOf(entry)
  if (card !== null) return card.report ?? card.title
  const tool = toolTitle(entry)
  return tool === null ? entry.body : (toolText(tool) ?? tool)
}
```

  Update the file's header comment: entries are told apart by kind.
- [ ] **Step 5: Callers.** `entry.tsx`: replace the `AgentMessage` branch's card/tool parsing with two branches before it — `if (entry.kind === 'Subagent')` (render `SubagentCardView` when `subagentOf(entry)` is non-null, else nothing) and `if (entry.kind === 'ToolCall')` (the existing tool line, using `entry.body` as `tool`); the `AgentMessage` branch becomes `return <ReplyText text={entry.body} />`. `messages.tsx`, `conversation.tsx`, `deleted-conversation.tsx`: `subagentOf(entry.body)` → `subagentOf(entry)`.
- [ ] **Step 6: The running reply.** In `use-conversation.ts`, `agentOrdinal` accepts the three kinds a turn's agent writes:

```ts
const AGENT_KINDS = new Set(['AgentMessage', 'ToolCall', 'Subagent'])
// …
  return typeof kind === 'string' && AGENT_KINDS.has(kind) && typeof ordinal === 'number'
    ? ordinal
    : null
```

  and its doc says so. Add to the reply's stream test a case where a `ThreadEntryAppended` with `kind: 'ToolCall'` dispatches `agent-entry` as `AgentMessage` does (copy the existing `AgentMessage` case and change the kind).
- [ ] **Step 7:** `npm run typecheck`, `npm run lint`, `npx vitest run src/app/conversation src/stream`. Expected: PASS.
- [ ] **Step 8:** Commit: `§23.8 web: tool lines and subagent cards read from their kinds`.

### Task 6: Specs, the gate, the browser

**Files:**
- Modify: `docs/superpowers/specs/2026-10-05-subagent-cards-design.md` §22.2 (the entry is kind `Subagent` with `card`, per §23.8 — amend in place), and §12.2's owner file (find it: `grep -n "^## 12.2" docs/superpowers/specs/*.md`) with the two kinds.
- Modify: `docs/status.md` (one paragraph: PR 0 built on `next/guided-planning`).

- [ ] **Step 1:** Amend §22.2 and §12.2 in place.
- [ ] **Step 2: The full gate, once,** from the root: CLAUDE.md steps 1–6 and the 100-column check (`git diff --name-only main | grep '\.rs$' | xargs awk 'length > 100 {print FILENAME": "FNR}'` prints nothing), and in `web/`: `npm run typecheck`, `npm run lint`, `npx vitest run`.
- [ ] **Step 3: The browser,** on a copy of the dev database (copy `shadows.sqlite3` to `run-23.sqlite3`, point the `daemon` entry of `.claude/launch.json` at it, restore after): open a conversation that has tool lines and a subagent card from before 0020 — they read as before; send a turn that runs a tool and a subagent — the line and the card appear live and after a reload; fork it — the fork shows them; the daemon log holds no error.
- [ ] **Step 4:** Update `docs/status.md`; commit: `§23 PR 0: groundwork built and run`.
- [ ] **Step 5:** Stop. Push and the PR wait for "ارفع".
