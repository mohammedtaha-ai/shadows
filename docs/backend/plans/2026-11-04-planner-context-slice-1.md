# Planner Context Slice 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** A Planner conversation can be anchored at a part or a roadmap stage. Its session then
receives a bounded, sourced context (an index, plus the anchor's detail) whenever that context
differs from the one it last carried. Each turn records what it carried. The same branch fixes
the three Planner defects found on 2026-11-04.

**Architecture:**
- `Design` stores a thread's anchor; it owns `DesignAnchor`, and Threads cannot depend on
  Design.
- `Turns::planning_context` compiles the context from `Design` and `Plans` reads, in
  `send_with`, and hands it to the turn.
- At prompt time the turn compares the context's digest with the last one **this harness
  session** heard. If they differ, it adds the block, and `Harness` records it as prepared.
- Only a turn that ends normally marks it heard. A stopped or failed turn, or a command-only
  turn, leaves it owed.

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
- A new module updates the code map. New tables update the architecture table-owner inventory
  and the foundation schema test. Focused compatibility and contract checks cover existing callers.

## Review Focus

1. **A new harness session on a thread that heard the context before.** The new session gets
   the context on its first turn (B-50 test).
2. **The anchor is deleted after it was set.** The next turn's context says
   `[anchor removed: <kind> <id>]` and carries the index only. The turn does not fail.
3. **A command-only turn on an anchored thread with context owed.** It carries no context and
   records none, and the next normal turn carries the owed context. Invalidating a context
   already heard after compaction belongs to spec §6 slice 2.
4. **Two turns in a row with nothing changed.** The second carries no context block, and its
   turn has no record.
5. **The detail is over budget.** The anchor's own text stays whole, and the block lists what was
   cut, in the spec's order: older rationales, then children's lines, then agreement bodies.
6. **A map refused for a rule.** The refusal names the part or parts, the paths, the rule, and
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
    (`DesignError::Storage(StorageError::NotFound)`).
  - It refuses an AYLUL thread (`InvalidCommand`).
  - `None` clears the anchor.
- Produces: `Design::thread_anchor(&self, thread: &ThreadId) -> Result<Option<DesignAnchor>,
  DesignError>`.

- [x] **Step 1: tests**
  - `an_anchor_is_set_read_and_cleared`
  - `an_anchor_from_another_project_is_refused`
  - `an_aylul_thread_cannot_be_anchored`
  - `a_replay_answers_the_first_anchor`
- [x] **Step 2:** run `cargo test -p core-design --test anchors`; expect FAIL.
- [x] **Step 3:** implement; pin the migration.
- [x] **Step 4:** run `cargo test -p core-design` and
  `cargo test -p core-db --test migration_checksums`; expect PASS.
- [x] **Step 5:** commit `feat(core-design): a thread's planning anchor`.

### Task B-48: What a turn carried, per session (Harness)

**Files:**
- Create: `crates/core-db/migrations/0044_invocation_context.sql`: table `invocation_context`,
  with these columns:
  - `operation_id TEXT PRIMARY KEY REFERENCES operation(id)`;
  - `thread_id TEXT NOT NULL`;
  - `session_id TEXT NOT NULL`: the harness session the prompt went to;
  - `start_seq INTEGER NOT NULL`: the operation journal sequence captured at preparation;
  - `digest TEXT NOT NULL`;
  - `text TEXT NOT NULL`;
  - `sources_json TEXT NOT NULL`;
  - `index_tokens INTEGER NOT NULL`;
  - `detail_tokens INTEGER NOT NULL`;
  - `over_tokens INTEGER NOT NULL`;
  - `cuts_json TEXT NOT NULL`;
  - `heard INTEGER NOT NULL DEFAULT 0`;
  - `created_at TEXT NOT NULL`.
- Modify: `crates/core-harness/src/` (a new `context_record.rs` and its store), `model.rs`,
  `contract.yaml`; the checksum fixture
- Test: `crates/shadows/tests/context_record.rs`

**Interfaces:**
- **Types** (`model.rs`; both serialize to JSON for the record and the route):
  - `CarriedContext { digest: String, text: String, sources: Vec<ContextSource>, index_tokens:
    i64, detail_tokens: i64, over_tokens: i64, cuts: Vec<String> }`;
  - `ContextSource { kind: String, id: String, version: i64 }`.
- **Two states, so a context that never arrived is never taken as heard:**
  - `Harness::prepare_context(&self, op: &OperationId, thread: &ThreadId, session: &str,
    carried: &CarriedContext) -> Result<(), StorageError>` writes the row with `heard = 0`;
  - `Harness::mark_context_heard(&self, op: &OperationId) -> Result<(), StorageError>` sets
    `heard = 1`.
- **What the session heard, by session, not by thread:**
  `Harness::heard_context_digest(&self, session: &str) -> Result<Option<String>, StorageError>`
  reads the digest of the newest `heard = 1` row for that harness session. The newest is by the
  captured operation journal sequence, never `rowid`. A new session on an old thread has heard nothing.
- **The record a person reads:** `Harness::context_record(&self, op: &OperationId) ->
  Result<Option<(CarriedContext, bool)>, StorageError>` answers the row and its `heard`.

- [x] **Step 1: tests**
  - `a_prepared_context_is_read_back_unheard`
  - `only_a_heard_context_counts`
  - `a_new_session_on_the_same_thread_has_heard_nothing`
  - `the_newest_heard_digest_wins`
- [x] **Step 2:** run them; expect FAIL.
- [x] **Step 3:** implement; pin the migration.
- [x] **Step 4:** run `cargo test -p core-harness` and
  `cargo test -p shadows --test context_record`; expect PASS.
- [x] **Step 5:** commit `feat(core-harness): record the context a turn carried, per session`.

### Task B-49: Compile the planning context (Turns)

**Files:**
- Create: `crates/core-turns/src/planning/mod.rs` (the compiler), `planning/budget.rs` (the
  constants and the cut order), `planning/block.rs` (the text)
- Modify: `crates/core-turns/src/lib.rs`:
  - `Turns` gains `design: core_design::Design`, so `Turns::new` takes it;
  - AppCore passes it in `crates/shadows-core/src/app.rs`;
  - the contract is updated.
- Test: `crates/core-turns/tests/planning_context.rs`, budget tests in `planning/block.rs`,
  and the pinned-agreement, removed-anchor and AYLUL cases in
  `crates/shadows/tests/planning_context.rs`.

**Interfaces:**
- Consumes:
  - `Design::thread_anchor` (B-47), `Design::vision`, `Design::parts(project, parent, cursor)`,
    `Design::outcomes(project, parent, cursor)`, `Design::agreements(project)`,
    `Design::agreement(project, id, Some(version))`;
  - `Plans::design_part`, `Plans::design_outcome`, `Plans::list(project, false)`, `Plans::get`;
  - `CarriedContext`, `ContextSource` (B-48).
- Produces: `Turns::planning_context(&self, thread: &ThreadId, project: &ProjectId) ->
  Result<Option<CarriedContext>, TurnsError>`. It answers `None` for an AYLUL thread.
- **Index**, in this order:
  - the vision's **purpose items** with their ids;
  - the part tree to depth 2 (`title` and id);
  - the outcome tree to depth 2 (`title` and id);
  - each agreement's name and agreed version.

  Read one page, at 50 items, for each parent visited through depth 2. A non-empty `next`
  is stated as `[more parts available]` or `[more outcomes available]`. No total is
  inferred from a cursor, and the read does not follow additional pages for that parent.
- **Detail, anchored only,** from `Plans::design_part` or `design_outcome`:
  - the anchor's full content;
  - its ancestors' titles;
  - its children's titles, each with one line: `responsibility` or `intended_result`;
  - each linked plan's latest version (`title`, `goal`, and each task's number and title);
  - the `change_reason` of that version and of up to 2 earlier versions, by `previous`;
  - for an outcome, its parts' titles;
  - **the agreements those plan versions bind**, each read at its **pinned version** from the
    version's `bindings` (`AgreementBinding { agreement_id, version, .. }`), through
    `Design::agreement(project, id, Some(version))`, never the latest. Each one is a source
    `("agreement", id, pinned version)`.
- **Not in the block:** standards. The existing "changes since the session last heard" block
  already carries them.
- **Budgets:**
  - **Index over budget:** first drop the depth-2 children of parts, then of outcomes, each
    replaced by `[N children]`, then the agreement list, replaced by `[N agreements]`.
  - **Detail over budget:** cut in the spec's order: older rationales, then children's lines,
    then agreement bodies. Bodies are cut to the agreement's name and version.
  - **Still over after every allowed cut:** the cap is soft. The anchor's text and the plans'
    task lists are never cut, the block is sent as it is, and `over_tokens` records how far it
    is over. Each cut is a line `[cut: <what>]` and an entry in `cuts`.
- **A missing anchor** (deleted after being set): the block holds
  `[anchor removed: <kind> <id>]` and the index only.
- **`digest`:** a hex SHA-256 of `text`.

- [x] **Step 1: tests**, over a design seeded with `Design::seed_map_for_tests` and
  `seed_roadmap_for_tests`:
  - `an_unanchored_thread_gets_the_index_only`
  - `a_part_anchor_adds_its_detail_and_sources`
  - `an_outcome_anchor_lists_its_parts_and_plans`
  - `the_index_carries_purpose_not_the_whole_vision`
  - `a_bound_agreement_is_read_at_its_pinned_version`: the plan binds v1, the latest is v2, and
    the detail holds v1's text and the source `("agreement", id, 1)`
  - `an_over_budget_detail_cuts_in_order_and_keeps_the_anchor` (lower the budget through a
    test-only constant)
  - `an_over_budget_index_drops_children_first`
  - `a_detail_still_over_after_every_cut_is_sent_and_says_by_how_much`
  - `parts_past_one_page_are_announced_not_listed`
  - `a_removed_anchor_says_so`
  - `an_aylul_thread_gets_nothing`
  - `the_same_design_gives_the_same_digest`
- [x] **Step 2:** run `cargo test -p core-turns --test planning_context`; expect FAIL.
- [x] **Step 3:** implement.
- [x] **Step 4:** run `cargo test -p core-turns`; expect PASS.
- [x] **Step 5:** commit `feat(core-turns): compile a Planner's planning context`.

### Task B-50: Send it when the session has not heard it

**Files:**
- Modify:
  - `crates/core-turns/src/lib.rs` (`send_with` calls `planning_context` after
    `turn_context_in`);
  - `src/spawn.rs` (`PlannerTurnRequest` gains `planning: Result<Option<CarriedContext>, String>`
    so compile failures become a recorded Prepare failure);
  - `src/prompt.rs` (`PromptParts` gains `planning: Option<String>`);
  - `src/store/operation.rs` (acknowledges context atomically with successful completion);
  - `core-harness` gains `mark_context_heard_in` for that transaction;
  - direct request constructors in `shadows-core/src/testing/turn.rs` and
    `shadows/tests/{turn_command.rs,plan_boundaries.rs,planner_mcp/standards.rs}`;
  - `contract.yaml`.
- Test: `crates/shadows/tests/planning_context.rs` (new; a Planner turn through `AppCore` with
  fake-acp, reading the prompt fake-acp received)

**Interfaces:**
- **In `spawn`, before compose:** read `Harness::heard_context_digest(&opened.session_id)`. The
  planning text goes into `PromptParts` only when the digest differs or nothing was heard.
- **Block order:** text, images, changes, **planning**, focus, continue_plan, stage.
- **After compose:** when the block was included and the prompt is not withheld (a command
  alone), call `Harness::prepare_context(op, thread, &opened.session_id, &carried)`. A failed
  write logs `planner.context_record_failed`; the turn goes on, and the context stays owed.
- **At the turn's end:** only an `Ended` turn acknowledges its context, through
  `Harness::mark_context_heard_in` in the same transaction that writes its terminal row.
  - A `Cancelled`, `Refused`, `Failed` or interrupted turn, or one whose ACP connection closed,
    leaves its row `heard = 0`, so the next turn sends the context again.
  - A repeat is preferred to a context that never arrived.
- **No new error:** a compile error refuses the turn the way a failed stage line does today
  (`FailureStage::Prepare`).

- [x] **Step 1: tests**
  - `the_first_turn_carries_the_context_and_marks_it_heard`
  - `an_unchanged_context_is_not_sent_again`
  - `a_changed_anchor_sends_it_again`
  - `a_new_session_on_an_old_thread_gets_it_again`: the thread's session is closed and a new one
    is opened, and the context is sent although the thread has a heard row
  - `a_stopped_turn_leaves_it_owed`: a turn stopped before its end leaves `heard = 0`, and the
    next turn sends the context
  - `a_failed_turn_leaves_it_owed` (fake-acp refuses the prompt)
  - `a_command_alone_carries_and_records_nothing_and_the_next_turn_does`
  - `the_planning_block_comes_after_changes_and_before_focus`
- [x] **Step 2:** run them; expect FAIL.
- [x] **Step 3:** implement.
- [x] **Step 4:** run `cargo test -p shadows --test planning_context` and
  `cargo test -p core-turns`; expect PASS.
- [x] **Step 5:** commit `feat(core-turns): send the planning context until the session hears it`.

### Task B-51: Routes for the anchor and the record

**Request:** `docs/shared/requests/2026-11-04-planning-context-routes.md` must be **accepted**
before this task starts (`docs/backend/AGENTS.md`, rule 4).

**Files:**
- Modify: the `crates/shadows-http/src/` thread routes, `openapi.rs`, `contract.yaml`;
  `api/openapi.json`
- Test: `crates/shadows/tests/planning_routes.rs`

**Interfaces:**

| Route | Body | Calls | Answers |
|---|---|---|---|
| `PUT /api/threads/{id}/anchor` | `{command_id, anchor: {kind: "part" \| "outcome", id} \| null}` | `Design::set_thread_anchor` | the anchor |
| `GET /api/threads/{id}/anchor` | | `Design::thread_anchor` | the anchor or `null` |
| `GET /api/operations/{id}/context` | | `Harness::context_record` | `{ carried: CarriedContext, heard: bool }`, or 404 `CONTEXT_NOT_CARRIED` |

- [x] **Step 1: tests**
  - `an_anchor_is_set_over_http_and_read_back`
  - `a_turns_carried_context_is_read_over_http`
  - `a_turn_that_carried_nothing_answers_not_carried`
- [x] **Step 2:** run `cargo test -p shadows --test planning_routes`; expect FAIL.
- [x] **Step 3:** implement; regenerate `api/openapi.json`.
- [x] **Step 4:** run them and `cargo test -p shadows --test openapi`; expect PASS;
  `git diff api/` shows only the three routes.
- [x] **Step 5:** commit `feat(shadows-http): planning anchor and carried-context routes`.

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
  `file_is_folder`, `folder_claimed_twice`, `reserved_file`, `invalid_path`, `invalid_part`.
- Exact text for the three seen on 2026-11-04:
  - `file_outside_part: part "<title>" lists <file>, which is outside its folders <folders>.
    Move the file into one of them, or add its folder to the part.`
  - `folder_claimed_twice: parts "<a>" and "<b>" both claim <folder>. Give the folder to one
    part, or make one part the other's child.`
  - The same rule covers a folder inside another part's folder: `parts "<a>" (<folder>) and
    "<b>" (<path>) overlap…`.

- [x] **Step 1: tests**
  - `a_file_outside_its_part_names_the_part_and_its_folders`
  - `the_same_folder_twice_names_both_parts`
  - `a_nested_folder_names_both_parts`
  - `two_rules_never_share_a_message`, which runs one map per rule and checks that the `rule`
    values are distinct
- [x] **Step 2:** run them; expect FAIL.
- [x] **Step 3:** implement.
- [x] **Step 4:** run `cargo test -p core-design`, then `cargo test -p shadows-mcp`; expect PASS.
- [x] **Step 5:** commit `fix(core-design): map refusals name the rule, the parts and the fix`.

### Task B-53: Vision IDs are given when proposed (defect §2)

**Files:**
- Create: `crates/core-db/migrations/0045_vision_reserved_id.sql`:
  `ALTER TABLE design_vision_proposal ADD COLUMN reserved_item_id TEXT`
- Modify: the propose path in `crates/core-design/src/store/` and
  `store/proposal_approval.rs`; the proposal view; the contract; the checksum fixture
- Test: `crates/shadows/tests/vision_proposal_ids.rs`, through AppCore; existing vision assertions in `crates/shadows-core/tests/design_vision/proposals.rs`

**Interfaces:**
- **Proposing an `Add`:** takes the next ID from the counters at proposal time
  (`VisionItems::next_id`), stores it in `reserved_item_id`, and returns it in the proposal's
  answer and card.
- **Approval uses `reserved_item_id`.** A row without one (proposed before this migration) takes
  the next ID at approval, as today.
- A superseded or refused proposal leaves its ID unused; IDs are never reused.

- [x] **Step 1: tests**
  - `an_added_item_keeps_the_id_it_was_proposed_with`: propose TD1, then TD2; approve TD2
    first; the stored IDs are still TD1 and TD2
  - `an_old_proposal_without_a_reserved_id_still_approves`
- [x] **Step 2:** run them; expect FAIL.
- [x] **Step 3:** implement; pin the migration.
- [x] **Step 4:** run `cargo test -p core-design`; expect PASS.
- [x] **Step 5:** commit `fix(core-design): a vision item keeps the ID it was proposed with`.

### Task B-54: A map proposal lists vision items no part serves (defect §3)

**Files:**
- Modify: `crates/core-design/src/map_proposals.rs` (the proposal's answer), the
  contract; `src/map_rules.rs` and `src/store/map_proposals.rs`
- Test: `crates/shadows/tests/map_proposal_coverage.rs`, through AppCore

**Interfaces:**
- The proposal answer gains `unserved: Vec<String>`: the vision item ids that no part in the
  proposed map lists in `serves`, in vision order.
- This is information, not a refusal: the Planner sees it and decides.
- **Its limit, stated:** it finds a vision item that **no** part serves. It does not find a part
  that forgot an item another part already serves. testing55's case was this: Telegram did not
  list TD4, and Backend did not serve it either, so B-54 would have caught it. Had another part
  served TD4, it would not have. Whether a part's text matches its `serves` is a judgement for
  the Planner and the person, not a rule in code.

- [x] **Step 1:** test `a_map_answer_lists_the_unserved_vision_items`.
- [x] **Step 2:** run it; expect FAIL.
- [x] **Step 3:** implement.
- [x] **Step 4:** run `cargo test -p core-design`; expect PASS.
- [x] **Step 5:** commit `feat(core-design): a map proposal lists unserved vision items`.

---

## Acceptance

1. CI passes on the branch (pending push; the full gate is not run locally).
2. Run `shadows serve` with the saved development database and open testing55 in the browser:
   - anchor its conversation at `Backend (Laravel core)` with
     `PUT /api/threads/{id}/anchor`, then sends a turn;
   - `GET /api/operations/{id}/context` shows the index and Backend detail, their measured
     character-based estimates, sources, cuts and digest;
   - a second turn records nothing.
3. On a new project, a map with a folder claimed twice is refused with a message naming both
   parts.

## Spec amendments

Made with this plan: spec §4.1 and §5 name `Design` as the anchor's owner, and §6 moves
"re-send on change" into slice 1.
