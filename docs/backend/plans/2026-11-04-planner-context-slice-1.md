# Planner Context Slice 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Planner conversation can be anchored at a part or a roadmap stage. Its session then
receives a bounded, sourced context (an index, plus the anchor's detail) whenever that context
differs from the one it last carried. Each turn records what it carried. The same branch fixes
the three Planner defects found on 2026-11-04.

**Architecture:**
- `Design` stores a thread's anchor; it owns `DesignAnchor`, and Threads cannot depend on
  Design.
- `Turns::planning_context` compiles the context from `Design` and `Plans` reads, in
  `send_with`, and hands it to the turn.
- At prompt time the turn compares the context's digest with the last one `Harness` recorded
  for the thread. If they differ, it adds the block and asks `Harness` to record it.
- A command-only turn carries nothing and records nothing, so the context stays owed.

**Tech Stack:** Rust 1.94, SQLx 0.9 / SQLite, sha2 (already a workspace dependency), axum + utoipa.

**Spec:** `docs/backend/specs/2026-11-04-planner-context-design.md` (§4.1 to §4.4, §6 slice 1,
§7). **Defects:** `docs/evidence/2026-11-04-planner-session-defects.md` §1 to §3.

## Global Constraints

- CLAUDE.md "Code structure": start at `AppCore`; change only the owning service; a new
  operation is a new method with its contract line in the same commit.
- Migrations: `crates/core-db/migrations/`, next numbers `0043`, `0044`, `0045`. Each one is
  pinned in `crates/core-db/tests/fixtures/base_migrations.sha384` in the same task, with the
  count in `migration_checksums.rs` raised.
- Rust lines ≤ 100 characters (CI checks it); `cargo fmt`; clippy `-D warnings`.
- Never the full gate locally; each task runs its own crate's tests.
- Budgets, from spec §4.2, in one constant block in `core-turns/src/planning/budget.rs`:
  `INDEX_TOKENS = 1_500`, `DETAIL_TOKENS = 8_000`, tokens estimated as `chars / 4`.
- The index carries the vision's **purpose items only** (spec §7: the whole vision is about 1,130
  tokens on testing55), never the whole vision.
- AYLUL threads (`harness == "aylul"`) get no planning context.

## Review Focus

1. **The anchor is deleted after it was set.** The next turn's context says
   `[anchor removed: <kind> <id>]` and carries the index only. The turn does not fail.
2. **A command-only turn (`/compact`) on an anchored thread.** It carries no context and records
   none, and the next normal turn carries it.
3. **Two turns in a row with nothing changed.** The second carries no context block, and its
   record reads "unchanged".
4. **The detail is over budget.** The anchor's own text stays whole, and the block lists what was
   cut, in the spec's order: older rationales, then children's lines, then agreement bodies.
5. **A map refused for a rule.** The refusal names the part or parts, the paths, the rule, and
   what to do, and two different rules never read the same.

---

### Task B-47: A thread's planning anchor (Design)

**Files:**
- Create: `crates/core-db/migrations/0043_thread_anchor.sql`: table
  `design_thread_anchor(thread_id TEXT PRIMARY KEY REFERENCES planning_thread(id), kind TEXT NOT
  NULL CHECK(kind IN ('part','outcome')), item_id TEXT NOT NULL, set_at TEXT NOT NULL)`
- Create: `crates/core-design/src/anchors.rs` and its store file under `src/store/`
- Modify: `crates/core-design/src/lib.rs`, `contract.yaml`; the core-db checksum fixture
- Test: `crates/core-design/tests/anchors.rs`

**Interfaces:**
- Produces: `Design::set_thread_anchor(&self, command_id: String, thread: &ThreadId, anchor:
  Option<DesignAnchor>) -> Result<Option<DesignAnchor>, DesignError>`. Command
  `thread.anchor`, params `{thread, anchor}`.
  - It refuses an anchor whose part or outcome is not in the thread's project
    (`DesignError::NotFound`).
  - It refuses an AYLUL thread (`InvalidCommand`).
  - `None` clears the anchor.
- Produces: `Design::thread_anchor(&self, thread: &ThreadId) -> Result<Option<DesignAnchor>,
  DesignError>`.

- [ ] **Step 1: tests**
  - `an_anchor_is_set_read_and_cleared`
  - `an_anchor_from_another_project_is_refused`
  - `an_aylul_thread_cannot_be_anchored`
  - `a_replay_answers_the_first_anchor`
- [ ] **Step 2:** run `cargo test -p core-design --test anchors`; expect FAIL.
- [ ] **Step 3:** implement; pin the migration.
- [ ] **Step 4:** run `cargo test -p core-design` and
  `cargo test -p core-db --test migration_checksums`; expect PASS.
- [ ] **Step 5:** commit `feat(core-design): a thread's planning anchor`.

### Task B-48: What a turn carried (Harness)

**Files:**
- Create: `crates/core-db/migrations/0044_invocation_context.sql`, table
  `invocation_context` with these columns:
  - `operation_id TEXT PRIMARY KEY REFERENCES operation(id)`
  - `thread_id TEXT NOT NULL`
  - `digest TEXT NOT NULL`
  - `text TEXT NOT NULL`
  - `sources_json TEXT NOT NULL`
  - `index_tokens INTEGER NOT NULL`
  - `detail_tokens INTEGER NOT NULL`
  - `cuts_json TEXT NOT NULL`
  - `created_at TEXT NOT NULL`
- Modify: `crates/core-harness/src/` (a new `context_record.rs` and its store), `model.rs`,
  `contract.yaml`; the checksum fixture
- Test: `crates/core-harness/tests/context_record.rs`

**Interfaces:**
- Produces (`model.rs`):
  - `CarriedContext { digest: String, text: String, sources: Vec<ContextSource>, index_tokens:
    i64, detail_tokens: i64, cuts: Vec<String> }`;
  - `ContextSource { kind: String, id: String, version: i64 }`.
  - Both serialize to JSON for the record and the route.
- Produces: `Harness::record_context(&self, op: &OperationId, thread: &ThreadId, carried:
  &CarriedContext) -> Result<(), StorageError>`.
- Produces: `Harness::last_context_digest(&self, thread: &ThreadId, before: &OperationId) ->
  Result<Option<String>, StorageError>`. It reads the digest of the thread's newest record
  whose operation started before `before`, ordered by the operation's start, never by `rowid`.
- Produces: `Harness::context_record(&self, op: &OperationId) ->
  Result<Option<CarriedContext>, StorageError>`.

- [ ] **Step 1: tests**
  - `a_recorded_context_is_read_back`
  - `the_last_digest_ignores_the_current_turn`
  - `a_thread_with_no_record_has_no_digest`
- [ ] **Step 2:** run them; expect FAIL.
- [ ] **Step 3:** implement; pin the migration.
- [ ] **Step 4:** run `cargo test -p core-harness`; expect PASS.
- [ ] **Step 5:** commit `feat(core-harness): record the context a turn carried`.

### Task B-49: Compile the planning context (Turns)

**Files:**
- Create: `crates/core-turns/src/planning/mod.rs` (the compiler), `planning/budget.rs` (the
  constants and the cut order), `planning/block.rs` (the text)
- Modify: `crates/core-turns/src/lib.rs`:
  - `Turns` gains `design: core_design::Design`, so `Turns::new` takes it;
  - AppCore passes it in `crates/shadows-core/src/app.rs`;
  - the contract is updated.
- Test: `crates/core-turns/tests/planning_context.rs`

**Interfaces:**
- Consumes:
  - `Design::thread_anchor` (B-47), `Design::vision`, `Design::parts(project, parent, None)`,
    `Design::outcomes(project, parent, None)`, `Design::agreements(project)`;
  - `Plans::design_part`, `Plans::design_outcome`, `Plans::list(project, false)`, `Plans::get`;
  - `CarriedContext`, `ContextSource` (B-48).
- Produces: `Turns::planning_context(&self, thread: &ThreadId, project: &ProjectId) ->
  Result<Option<CarriedContext>, TurnsError>`. It answers `None` for an AYLUL thread.
- **Index:**
  - the vision's purpose items, with their ids;
  - the part tree to depth 2 (`title` and id);
  - the outcome tree to depth 2 (`title` and id);
  - each agreement's name and agreed version.
- **Detail, anchored only,** from `Plans::design_part` or `design_outcome`:
  - the anchor's full content;
  - its ancestors' titles;
  - its children's titles, each with one line: `responsibility` or `intended_result`;
  - each linked plan's latest version (`title`, `goal`, and each task's number and title);
  - the `change_reason` of that version and of up to 2 earlier versions, by `previous`;
  - for an outcome, its parts' titles.
- **Not in the block:** standards. The existing "changes since the session last heard" block
  already carries them.
- **Every piece is a `ContextSource`:**
  - `("vision", project, revision)`;
  - `("part" | "outcome", id, item revision)`;
  - `("plan", workflow, version)`;
  - `("agreement", id, version)`.
- **Cuts:** in budget order. The anchor's content is never cut. Each cut is a line
  `[cut: <what>]` inside the block and an entry in `cuts`.
- **A missing anchor** (deleted after being set): the block holds
  `[anchor removed: <kind> <id>]` and the index only.
- **`digest`:** a hex SHA-256 of `text`.

- [ ] **Step 1: tests**, over a design seeded with `Design::seed_map_for_tests` and
  `seed_roadmap_for_tests`:
  - `an_unanchored_thread_gets_the_index_only`
  - `a_part_anchor_adds_its_detail_and_sources`
  - `an_outcome_anchor_lists_its_parts_and_plans`
  - `the_index_carries_purpose_not_the_whole_vision`
  - `an_over_budget_detail_cuts_in_order_and_keeps_the_anchor` (lower the budget through a
    test-only constant)
  - `a_removed_anchor_says_so`
  - `an_aylul_thread_gets_nothing`
  - `the_same_design_gives_the_same_digest`
- [ ] **Step 2:** run `cargo test -p core-turns --test planning_context`; expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p core-turns`; expect PASS.
- [ ] **Step 5:** commit `feat(core-turns): compile a Planner's planning context`.

### Task B-50: Send it when it changed, and record it

**Files:**
- Modify:
  - `crates/core-turns/src/lib.rs` (`send_with` calls `planning_context` after `turn_context_in`);
  - `src/spawn.rs` (`PlannerTurnRequest` gains `planning: Option<CarriedContext>`);
  - `src/prompt.rs` (`PromptParts` gains `planning: Option<String>`);
  - `contract.yaml`.
- Test: `crates/shadows/tests/` (the Planner turn test that runs fake-acp: add
  `planning_context.rs`)

**Interfaces:**
- **In `spawn`, before compose:** read `Harness::last_context_digest(thread, op)`. The planning
  text goes into `PromptParts` only when the digest differs.
- **Block order:** text, images, changes, **planning**, focus, continue_plan, stage.
- **After compose:**
  - when the block was included and the prompt was not withheld (`withholds == false`, and not
    a command alone), call `Harness::record_context(op, thread, &carried)` before the prompt is
    sent;
  - a failed record logs `planner.context_record_failed` and does not fail the turn.
- **No new error:** a compile error refuses the turn the way a failed stage line does today
  (`FailureStage::Prepare`).

- [ ] **Step 1: tests,** through `AppCore` with fake-acp, reading the prompt fake-acp received:
  - `the_first_turn_carries_the_context_and_records_it`
  - `an_unchanged_context_is_not_sent_again`
  - `a_changed_anchor_sends_it_again`
  - `a_command_alone_carries_and_records_nothing_and_the_next_turn_does`
  - `the_planning_block_comes_after_changes_and_before_focus`
- [ ] **Step 2:** run them; expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p shadows --test planning_context` and
  `cargo test -p core-turns`; expect PASS.
- [ ] **Step 5:** commit `feat(core-turns): send the planning context when it changed`.

### Task B-51: Routes for the anchor and the record

**Files:**
- Modify: the `crates/shadows-http/src/` thread routes, `openapi.rs`, `contract.yaml`;
  `api/openapi.json`
- Test: `crates/shadows/tests/planning_routes.rs`

**Interfaces:**

| Route | Body | Calls | Answers |
|---|---|---|---|
| `PUT /api/threads/{id}/anchor` | `{command_id, anchor: {kind: "part" \| "outcome", id} \| null}` | `Design::set_thread_anchor` | the anchor |
| `GET /api/threads/{id}/anchor` | | `Design::thread_anchor` | the anchor or `null` |
| `GET /api/operations/{id}/context` | | `Harness::context_record` | `CarriedContext`, or 404 `CONTEXT_NOT_CARRIED` |

- [ ] **Step 1: tests**
  - `an_anchor_is_set_over_http_and_read_back`
  - `a_turns_carried_context_is_read_over_http`
  - `a_turn_that_carried_nothing_answers_not_carried`
- [ ] **Step 2:** run `cargo test -p shadows --test planning_routes`; expect FAIL.
- [ ] **Step 3:** implement; regenerate `api/openapi.json`.
- [ ] **Step 4:** run them and `cargo test -p shadows --test openapi`; expect PASS;
  `git diff api/` shows only the three routes.
- [ ] **Step 5:** commit `feat(shadows-http): planning anchor and carried-context routes`.

### Task B-52: Map refusals say which rule, which parts and what to do (defect §1)

**Files:**
- Modify:
  - `crates/core-design/src/map_rules.rs` (each rule returns a `MapRefusal`);
  - `src/error.rs` (`DesignError::MapRefused(MapRefusal)`);
  - `src/map_proposals.rs`;
  - `crates/shadows-core` `ErrorCode` (adds `MapRefused`, wire `MAP_REFUSED`);
  - `crates/shadows-mcp/src/refusal.rs` and the HTTP failure mapping;
  - the contracts.
- Test: `crates/core-design/tests/map_rules.rs` (or the existing map rules tests)

**Interfaces:**
- `MapRefusal { rule: &'static str, message: String }`, wired as
  `MAP_REFUSED: <rule>: <message>`. Each message names:
  - the part titles;
  - the paths;
  - one sentence saying what is allowed.
- `rule` is one of: `duplicate_part`, `duplicate_sibling`, `missing_parent`,
  `unknown_vision_item`, `unknown_rule`, `file_outside_part`, `duplicate_file`,
  `file_is_folder`, `folder_claimed_twice`, `reserved_file`, `invalid_path`.
- Exact text for the three seen on 2026-11-04:
  - `file_outside_part: part "<title>" lists <file>, which is outside its folders <folders>.
    Move the file into one of them, or add its folder to the part.`
  - `folder_claimed_twice: parts "<a>" and "<b>" both claim <folder>. Give the folder to one
    part, or make one part the other's child.`
  - The same rule covers a folder inside another part's folder: `parts "<a>" (<folder>) and
    "<b>" (<path>) overlap…`.

- [ ] **Step 1: tests**
  - `a_file_outside_its_part_names_the_part_and_its_folders`
  - `the_same_folder_twice_names_both_parts`
  - `a_nested_folder_names_both_parts`
  - `two_rules_never_share_a_message`, which runs one map per rule and checks that the `rule`
    values are distinct
- [ ] **Step 2:** run them; expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p core-design`, then `cargo test -p shadows-mcp`; expect PASS.
- [ ] **Step 5:** commit `fix(core-design): map refusals name the rule, the parts and the fix`.

### Task B-53: Vision IDs are given when proposed (defect §2)

**Files:**
- Create: `crates/core-db/migrations/0045_vision_reserved_id.sql`:
  `ALTER TABLE design_vision_proposal ADD COLUMN reserved_item_id TEXT`
- Modify: the propose path in `crates/core-design/src/store/` and
  `store/proposal_approval.rs`; the proposal view; the contract; the checksum fixture
- Test: `crates/core-design/tests/` (the vision proposal tests)

**Interfaces:**
- **Proposing an `Add`:** takes the next ID from the counters at proposal time
  (`VisionItems::next_id`), stores it in `reserved_item_id`, and returns it in the proposal's
  answer and card.
- **Approval uses `reserved_item_id`.** A row without one (proposed before this migration) takes
  the next ID at approval, as today.
- A superseded or refused proposal leaves its ID unused; IDs are never reused.

- [ ] **Step 1: tests**
  - `an_added_item_keeps_the_id_it_was_proposed_with`: propose TD1, then TD2; approve TD2
    first; the stored IDs are still TD1 and TD2
  - `an_old_proposal_without_a_reserved_id_still_approves`
- [ ] **Step 2:** run them; expect FAIL.
- [ ] **Step 3:** implement; pin the migration.
- [ ] **Step 4:** run `cargo test -p core-design`; expect PASS.
- [ ] **Step 5:** commit `fix(core-design): a vision item keeps the ID it was proposed with`.

### Task B-54: A map proposal lists vision items no part serves (defect §3)

**Files:**
- Modify: `crates/core-design/src/map_proposals.rs` (the proposal's answer), `model.rs`, the
  contract; `shadows-mcp`'s `map_propose` answer if it shapes it
- Test: `crates/core-design/tests/` (the map proposal tests)

**Interfaces:**
- The proposal answer gains `unserved: Vec<String>`: the vision item ids that no part in the
  proposed map lists in `serves`, in vision order.
- This is information, not a refusal: the Planner sees it and decides.

- [ ] **Step 1:** test `a_map_answer_lists_the_unserved_vision_items`.
- [ ] **Step 2:** run it; expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p core-design`; expect PASS.
- [ ] **Step 5:** commit `feat(core-design): a map proposal lists unserved vision items`.

---

## Acceptance

1. CI passes on the branch.
2. Mohammed runs `shadows serve` and opens testing55 in the browser:
   - he anchors its conversation at `Backend (Laravel core)` with
     `PUT /api/threads/{id}/anchor`, then sends a turn;
   - `GET /api/operations/{id}/context` shows the index and the Backend detail, about 1 300 and
     3 100 tokens on that project;
   - a second turn records nothing.
3. On a new project, a map with a folder claimed twice is refused with a message naming both
   parts.

## Spec amendments

Made with this plan: spec §4.1 and §5 name `Design` as the anchor's owner, and §6 moves
"re-send on change" into slice 1.
