# §23 PR 3b — Approval Authority (§24) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The Planner cannot act as the person: its session holds a fixed
tool set and none of the person's Claude settings, it approves through its own
MCP tools only, and every approval records who did it and at whose request.

**Architecture:** `SessionSetup` sends `tools`, `settingSources: []` and
`strictMcpConfig: true` on every session opening. Every approval method in the
core takes an `Approver` (the person, or the Planner of a running turn) and
writes `approved_principal`, `approved_agent` and the request entry onto the
approved row. Three MCP tools (`vision_approve`, `map_approve`,
`plan_approve`) call the core with the Planner as approver; a first map with
waivers and Create structure stay the person's. The web cards say who approved.

**Tech Stack:** Rust (SQLx 0.9 SQLite, rmcp, axum/utoipa), React + Vitest in
`web/`.

**Spec:** `docs/superpowers/specs/2026-10-08-approval-authority-design.md`
(§24.1–§24.5). Evidence:
`docs/evidence/2026-10-12-planner-tool-lock-probe.md`. Related:
`docs/superpowers/specs/2026-10-08-guided-planning-design.md` §23.5, and
`docs/superpowers/specs/2026-09-25-planner-workflow-design.md` §13.2, §13.8.

## Global Constraints

- Every cargo command is prefixed
  `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0`.
  Never build into the repo's `target/`.
- No Rust line over 100 columns. `cargo fmt --all` before each commit.
- Never edit a shipped migration (`0001`–`0023`). New schema is `0024_approval_agent.sql`.
- No new Rust integration test files: add tests to the files each task names.
- No prettier; no Python, sed or Node scripts editing source or docs.
- `npm run gen:api` (in `web/`) whenever `api/openapi.json` changes; commit both.
- Targeted tests while working. The full gate (CLAUDE.md steps 1–6, plus
  `npm run test`, `npm run typecheck`, `npm run lint`, `npm run build` in
  `web/`) runs once, in Task 6.
- A change to a service updates its `contract.yaml` in the same commit.
- Commits end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`
  (or your own model's line). Do not push.
- The Planner's built-in tools, exactly: `Read`, `Grep`, `Glob`, `WebSearch`,
  `WebFetch`, `ToolSearch`, `Agent`, `TodoWrite` (§24.3 rule 1).
- Agent strings: `human` for the web client's button; `planner:<harness_kind>`
  (today `planner:claude-code`) for the Planner. Principal: `user:local`.
- Shown text: "Approved by you"; "Approved by the Planner at your request",
  with "your message" linking to the request entry.

## Rulings already made (do not reopen)

1. **Command records keep their key and gain two audit columns.**
   `command_record.principal_kind/id` stays the idempotency key every store
   uses. Migration 0024 adds `audit_principal TEXT NULL` and
   `audit_agent TEXT NULL` (not in the key). `CommandContext` gains
   `pub agent: Option<String>`; `record_command` writes
   `audit_principal = 'user:local'` and `audit_agent` when it is `Some`.
   Every approval sets it (`human` or `planner:<harness>`); other writes
   leave `None` until they need it. The approved rows carry the same two
   fields plus the request (§24.1: "in the command record and on every
   approval"). A Planner approval's durable events and its PlanApproved entry
   use `writer.actor()`, never `Actor::user`. (Codex review, 2026-10-12.)
2. **The request entry is the turn's latest `UserMessage`** (`thread_entry`
   with `kind = 'UserMessage'` and `operation_id` = the running turn, highest
   `ordinal`), so a steered message (§20) counts. It is the approving
   Planner's own conversation, not the proposal's.
3. **Existing approvals are the person's.** The migration backfills
   `approved_agent = 'human'`, `approved_principal = 'user:local'` on every
   row approved before 3b; they have no request entry.
4. **A waiver refusal is `INVALID_COMMAND`** with the message
   `a first map with waivers is approved by the person's button`; no new
   error code.
5. **Subagents need nothing extra:** they share the Planner's grant and
   running turn (§24.3 rule 3), so their approvals record the same way.
6. **Authority is re-checked inside the write transaction.** A grant can be
   revoked, and a turn can end, between the core method's checks and the
   store's write. Inside the transaction, in this order: `check_writer`
   (the grant is live and in scope) before `classify` looks for a replay;
   then the turn is still running for that thread; then the request entry is
   read. Exactly as the `map_propose` store path does. (Codex review.)
7. **"No tool policy" and "zero tools" differ.** `SessionSetup.tools` is
   `Option<Vec<String>>`: `None` sends no `tools` key (adapter default),
   `Some(vec![])` sends `"tools": []`. (Codex review.)

## Review Focus

1. A Planner approval replayed in the same turn (same args) returns the first
   result and appends nothing; a different turn approving an already-approved
   proposal is refused as the button is. Test in Task 3.
2. A turn whose person message was steered in mid-turn links the steered
   message, not the first one. Test in Task 3.
3. An outdated or superseded proposal is refused for the Planner exactly as
   for the button. Test in Task 3.
4. A session resumed after a daemon restart, and a fork's first session, still
   carry the locked options. Test in Task 1.
5. A pre-3b approved proposal or plan reads as "Approved by you", never as
   blank. Test in Task 2 (storage) and Task 5 (card).

---

### Task 1: Lock the Planner's session

**Files:**
- Modify: `crates/shadows-agent/src/acp.rs` (`SessionSetup`, `meta()`)
- Modify: `crates/shadows-core/src/harness/setup.rs` (where `SessionSetup` is built, line ~119)
- Modify: `crates/fake-acp/src/session.rs` (`Setup`, `setup_of`) and `crates/fake-acp/src/prompts.rs` (the report, line ~121)
- Test: `crates/shadows/tests/planner_mcp.rs`
- Contract: `crates/shadows-core/src/harness/contract.yaml`; spec §13.8 table row in `docs/superpowers/specs/2026-09-25-planner-workflow-design.md` (add the three options to the table)

**Interfaces:**
- Produces: `SessionSetup { mcp, append, allowed_tools: Vec<String>, tools: Option<Vec<String>> }`;
  `pub const PLANNER_TOOLS: [&str; 8]` in `setup.rs`.

- [ ] **Step 1: Write the failing test.** In `planner_mcp.rs`, extend
  `a_session_opens_with_shadows_mcp_instructions_and_the_pre_approval` and the
  resume test (line ~254, `second["how"] == "resume"`) and
  `a_forks_first_turn_gets_instruction_and_continue_plan_blocks` with:

```rust
fn assert_locked(r: &serde_json::Value) {
    assert_eq!(
        r["options"]["tools"],
        json!(["Read", "Grep", "Glob", "WebSearch", "WebFetch",
               "ToolSearch", "Agent", "TodoWrite"]),
        "{r}"
    );
    assert_eq!(r["options"]["settingSources"], json!([]), "{r}");
    assert_eq!(r["options"]["strictMcpConfig"], json!(true), "{r}");
    assert_eq!(r["options"]["allowedTools"], json!(["mcp__shadows__*"]), "{r}");
}
```

  Call `assert_locked(&r)` on each report (new, resume, fork). The
  "permission request is still refused" half of §24.5 test 2 is already
  pinned by the existing permission-refusal test (find it with
  `grep -rn PermissionRefused crates/shadows/tests`); leave it and name it in
  the contract.
- [ ] **Step 2: Report the options from fake-acp.** Add `options: Value` to
  `Setup`, filled from `meta.pointer("/claudeCode/options")` (`Value::Null`
  when absent), and `"options": s.setup.options` in the report beside
  `"allowed"`.
- [ ] **Step 3: Run it, watch it fail.**
  `cargo test -p shadows --test planner_mcp` → FAIL: `options.tools` is null.
- [ ] **Step 4: Implement.** In `acp.rs`:

```rust
    /// The built-in tools the session has at all (§24.3). `None` sets no
    /// policy (the adapter's default set); `Some(vec![])` means none at all.
    /// Either way a `Some` also keeps the person's own settings out.
    pub tools: Option<Vec<String>>,
```

  and in `meta()` replace the `allowed_tools` block with:

```rust
        let mut options = serde_json::Map::new();
        if !self.allowed_tools.is_empty() {
            options.insert("allowedTools".into(), serde_json::json!(self.allowed_tools));
        }
        if let Some(tools) = &self.tools {
            // §24.3: a fixed tool set, and none of the person's own settings,
            // MCP servers, plugins, skills or hooks.
            options.insert("tools".into(), serde_json::json!(tools));
            options.insert("settingSources".into(), serde_json::json!([]));
            options.insert("strictMcpConfig".into(), serde_json::json!(true));
        }
        if !options.is_empty() {
            meta.insert("claudeCode".into(), serde_json::json!({ "options": options }));
        }
```

  Unit tests in `acp.rs`'s own `#[cfg(test)]` module (add one if absent):
  `tools: None` → `meta()` has no `tools` key; `tools: Some(vec![])` →
  `"tools": []` with `settingSources: []` and `strictMcpConfig: true`.

  In `setup.rs` add

```rust
/// §24.3: the Planner's built-in tools. Anything else does not exist in its
/// session, in every mode; Shadows' MCP tools come on top.
pub const PLANNER_TOOLS: [&str; 8] = [
    "Read", "Grep", "Glob", "WebSearch", "WebFetch", "ToolSearch", "Agent", "TodoWrite",
];
```

  and set `tools: Some(PLANNER_TOOLS.iter().map(|t| t.to_string()).collect())`
  in the returned `SessionSetup`. Fix every other `SessionSetup { .. }`
  construction the compiler names (`tools: None`, or `..Default::default()`).
- [ ] **Step 5: Run.** `cargo test -p shadows --test planner_mcp` → PASS.
  Also `cargo test -p shadows-agent` → PASS.
- [ ] **Step 6: Contract and commit.** Add to the harness contract an
  obligation "every Planner session opening sends the §24.3 tool set,
  settingSources [] and strictMcpConfig true", naming the test. Commit
  `feat(harness): lock the Planner's tools and keep the person's settings out (§24.3)`.

### Task 2: Record who approved (principal, agent, request)

**Files:**
- Create: `crates/shadows-core/migrations/0024_approval_agent.sql`
- Create: `crates/shadows-core/src/command/approver.rs` (declared `pub mod approver;` in `command/mod.rs`, re-exported as `crate::command::{Approver, ApprovedBy}`)
- Modify: `crates/shadows-core/src/design/store/proposal_approval.rs`, `crates/shadows-core/src/design/store/map_approval.rs`, `crates/shadows-core/src/plans/store/edit.rs` (`approve_plan`), and the readers that build `VisionProposal`, `MapProposal` and the plan view (`plans/store/view.rs`)
- Modify: `crates/shadows-core/src/design/proposals.rs`, `design/map_proposals.rs`, `plans/mod.rs` (public methods pass `Approver::Person`)
- Test: `crates/shadows-core/tests/design_vision.rs`, `crates/shadows-core/tests/design_parts.rs`, `crates/shadows-core/tests/storage_contract/` (schema inventory, as PR 3 did), `crates/shadows/tests/plan_routes.rs`
- Contracts: `design/contract.yaml`, `plans/contract.yaml`

**Interfaces:**
- Produces:

```rust
/// Who performs an approval (§24.1). The core decides it from the call path,
/// never from a request's arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Approver {
    /// The person's button in the web client.
    Person,
    /// The Planner holding `writer` (a `Writer::Planner`) during `turn`. The
    /// store re-checks the grant and the turn inside its transaction and
    /// reads the request entry there (Ruling 6); nothing here is trusted.
    Planner { writer: Writer, turn: OperationId, harness: String },
}

/// What an approved row says about its approval; serialized on the API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ApprovedBy {
    /// Always `user:local` today.
    pub principal: String,
    /// `human`, or `planner:<harness>`.
    pub agent: String,
    /// The person's message the Planner acted on; none for the button.
    pub request: Option<ApprovalRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ApprovalRequest {
    pub thread_id: ThreadId,
    pub entry_id: ThreadEntryId,
    pub ordinal: i64,
}

impl Approver {
    pub fn agent(&self) -> String; // "human" | "planner:<harness>"
    /// The durable events' and PlanApproved entry's author: `Actor::user`
    /// for the person, `writer.actor()` for the Planner (Ruling 1).
    pub fn actor(&self) -> Actor;
}

/// Inside a write transaction (Ruling 6): for the Planner, `check_writer`,
/// then the turn is still running for the writer's thread, then that turn's
/// latest UserMessage id; `None` for the person. Called before `classify`.
pub(crate) async fn authorise_in(
    conn: &mut SqliteConnection, approver: &Approver,
) -> Result<Option<ThreadEntryId>, StorageError>;
```

  `authorise_in` lives in the store layer (`crates/shadows-core/src/db/` or
  the turns store, wherever `check_writer` and the running-turn query
  already live; it uses SQL, so not in `command/approver.rs`).

  - `VisionProposal`, `MapProposal` and the plan view (`Plan`) gain
    `pub approved_by: Option<ApprovedBy>` (`None` unless approved).
  - Store methods gain a trailing `approver: &Approver` parameter:
    `approve_vision_proposal_in`, `approve_map_proposal_in`, `approve_plan`.

- [ ] **Step 1: Migration.**

```sql
-- §24.1: who approved, on whose behalf, at which request. Rows approved
-- before this migration were approved by the person's button.
ALTER TABLE design_vision_proposal ADD COLUMN approved_principal TEXT NULL;
ALTER TABLE design_vision_proposal ADD COLUMN approved_agent TEXT NULL;
ALTER TABLE design_vision_proposal ADD COLUMN approved_request_entry TEXT NULL
    REFERENCES thread_entry(id) ON DELETE RESTRICT;
ALTER TABLE design_map_proposal ADD COLUMN approved_principal TEXT NULL;
ALTER TABLE design_map_proposal ADD COLUMN approved_agent TEXT NULL;
ALTER TABLE design_map_proposal ADD COLUMN approved_request_entry TEXT NULL
    REFERENCES thread_entry(id) ON DELETE RESTRICT;
ALTER TABLE workflow ADD COLUMN approved_principal TEXT NULL;
ALTER TABLE workflow ADD COLUMN approved_agent TEXT NULL;
ALTER TABLE workflow ADD COLUMN approved_request_entry TEXT NULL
    REFERENCES thread_entry(id) ON DELETE RESTRICT;
UPDATE design_vision_proposal SET approved_principal = 'user:local',
    approved_agent = 'human' WHERE status = 'Approved';
UPDATE design_map_proposal SET approved_principal = 'user:local',
    approved_agent = 'human' WHERE status = 'Approved';
UPDATE workflow SET approved_principal = 'user:local',
    approved_agent = 'human' WHERE frozen_at IS NOT NULL;
-- Audit only; the key stays (principal_kind, principal_id, scope, command_id).
ALTER TABLE command_record ADD COLUMN audit_principal TEXT NULL;
ALTER TABLE command_record ADD COLUMN audit_agent TEXT NULL;
```

  Also add `pub agent: Option<String>` to `CommandContext`
  (`command/mod.rs`); set `None` in every existing constructor the compiler
  names (`user_command` in `app` included); `record_command` writes
  `audit_principal = 'user:local'`, `audit_agent = agent` when `Some`.

  Check the `workflow` column is `frozen_at` before relying on it (it is set
  in `approve_plan`, `plans/store/edit.rs` ~line 145).
- [ ] **Step 2: Failing tests.**
  - `design_vision.rs`: a person approval (the existing approve helper)
    yields `approved_by == Some(ApprovedBy { principal: "user:local",
    agent: "human", request: None })`; a pending proposal has `None`.
  - `design_parts.rs`: the same for a first map approval.
  - `plan_routes.rs`: after `POST .../approve`, `GET` of the workflow returns
    `approved_by.agent == "human"`.
  - `crates/shadows-core/tests/design_migration.rs`: a database migrated
    from 23 with one Approved vision proposal reads `agent == "human"` after
    0024 (follow how the file already seeds a pre-migration database).
- [ ] **Step 3: Run, watch fail** (`cargo test -p shadows-core --test design_vision --test design_parts --test design_migration`; `cargo test -p shadows --test plan_routes`).
  - In the same three tests: the approval's `command_record` row has
    `audit_agent = 'human'`, `audit_principal = 'user:local'` (read it through
    the test's storage handle as the file already reads rows).
- [ ] **Step 4: Implement.** Each approve store calls `authorise_in` first
  (before `classify`), sets `ctx.agent = Some(approver.agent())`, and writes
  the three columns in the same `UPDATE` that sets the approved status
  (`approver.agent()`, `"user:local"`, the entry `authorise_in` returned).
  Events and the PlanApproved entry use `approver.actor()`. Read them back in each view
  (join `thread_entry` for `ordinal` and `thread_id` when the request is set).
  The public person methods pass `&Approver::Person`. Update the storage
  schema inventory test for the new columns if it lists columns.
- [ ] **Step 5: Run** the same commands → PASS. Then
  `cargo test -p shadows --test openapi`; if `api/openapi.json` changed run
  `npm run gen:api` in `web/` and `npm run typecheck`.
- [ ] **Step 6: Contracts and commit.** Design and Plans contracts: each
  approve method "records principal, agent and request (§24.1)". Commit
  `feat(core): record who approved, on whose behalf, at which request (§24.1)`.

### Task 3: The Planner's approval tools

**Files:**
- Modify: `crates/shadows-core/src/design/proposals.rs`, `design/map_proposals.rs`, `plans/mod.rs` (new methods)
- Create: `crates/shadows-core/src/turns/request.rs` if Turns has no reader for "the running turn's latest UserMessage"; otherwise extend the existing turns store reader. One job: find the request entry of a running turn.
- Modify: `crates/shadows-mcp/src/workspace_tools.rs` (`vision_approve`, `map_approve`), `crates/shadows-mcp/src/tools.rs` (`plan_approve`)
- Modify: `crates/shadows-core/src/harness/prompt.txt` (lines 5, 27–33, 57–58)
- Test: `crates/shadows/tests/workspace_mcp.rs`, `crates/shadows/tests/planner_mcp.rs` (prompt text), `crates/shadows/tests/mcp_tools.rs` (tool list)
- Contracts: `design/contract.yaml`, `plans/contract.yaml`, `turns/contract.yaml` if touched

**Interfaces:**
- Consumes: `Approver`, `ApprovedBy` (Task 2); `self.handles.running_for(thread)` as `propose_map` uses it (`design/map_proposals.rs` ~line 72); `super::agreement_scope::writer(grant)`.
- Produces:

```rust
impl Design {
    pub async fn approve_vision_proposal_as_planner(
        &self, grant: &Grant, proposal: &str,
    ) -> Result<DesignChange, CoreError>;
    pub async fn approve_map_proposal_as_planner(
        &self, grant: &Grant, proposal: &str,
    ) -> Result<DesignChange, CoreError>;
}
impl Plans {
    pub async fn approve_as_planner(
        &self, grant: &Grant, workflow: &WorkflowId, expected_revision: i64,
    ) -> Result<Approved, CoreError>;
}
```

  Each: `writer(grant)` must be `Writer::Planner { thread, .. }` (else
  `GrantScope`, as `propose_map`); the proposal or workflow must belong to the
  grant's project (else `NotFound`, the same answer as an unknown id); a
  running turn must exist (else `InvalidCommand` "no turn is running"); it
  builds `Approver::Planner { writer, turn, harness }` with `harness` = the
  thread's `harness_kind` and passes it to the Task 2 store method, whose
  `authorise_in` re-checks the grant and the turn and reads the request
  entry inside the transaction (Ruling 6). The command id is
  `derived_id(Anchor::Operation(&turn), &fingerprint(kind, params))` with the
  `CommandContext` built from `writer.principal()`, exactly as `propose_map`
  does, so a replay in the same turn returns the first result.
  `approve_map_proposal_as_planner` loads the proposal first and refuses a
  `MapAction::First` proposal with non-empty `waivers` with
  `CoreError::Refused { code: InvalidCommand, message: "a first map with waivers is approved by the person's button" }`.
  MCP tools: `vision_approve { proposal_id }`, `map_approve { proposal_id }`,
  `plan_approve { workflow_id, expected_revision }`, each answering like the
  neighbouring tools (`answer(...)`, refusals through `refusal.rs`).

- [ ] **Step 1: Failing tests in `workspace_mcp.rs`** (use the file's
  existing pattern for a Planner turn calling MCP tools through fake-acp):
  1. `the_planner_approves_a_vision_card_at_the_persons_request`: person
     sends a message; in that turn the Planner calls `vision_propose`, then
     `vision_approve`; the proposal reads `approved_by.agent ==
     "planner:claude-code"` and `approved_by.request.entry_id` is that
     turn's UserMessage id.
  2. `a_steered_message_is_the_request`: a turn with a steered second
     message (§20 steer route, as `message_queue.rs` does) links the steered
     entry.
  2a. `a_revoked_grant_cannot_approve` and `an_ended_turn_cannot_approve`:
     call the core store method directly (`approve_vision_proposal_in` with an
     `Approver::Planner`) after revoking the grant, and again after the turn
     has ended; both refused, the proposal stays Pending, no
     `command_record` row is written. These go in
     `crates/shadows-core/tests/design_vision.rs`, beside Task 2's tests.
  3. `the_planner_cannot_approve_a_first_map_with_waivers`: `map_propose`
     first map with a frontend waiver, `map_approve` → refusal text
     contains "person's button"; the proposal stays Pending; the person's
     `POST .../map-proposals/{id}/approve` then succeeds with
     `agent == "human"`.
  4. `the_planner_approves_a_map_without_waivers` → Approved,
     `planner:claude-code`.
  5. `a_replayed_planner_approval_appends_nothing`: calling `vision_approve`
     twice in one turn returns the same result and the durable event count
     rises once.
  6. `an_outdated_card_is_refused_for_the_planner_as_for_the_button`: edit the
     item by hand after the proposal, `vision_approve` → refused.
  7. In `planner_mcp.rs` or `plan_in_conversation.rs`: `plan_approve` freezes
     a blocker-free draft; the PlanApproved entry body reads
     `Plan v1 approved by the Planner at your request`; the plan's
     `approved_by.agent == "planner:claude-code"`.
  8. In `mcp_tools.rs`: the tool list contains `vision_approve`,
     `map_approve`, `plan_approve`, and no tool name contains `structure`
     (§24.5 test 4).
- [ ] **Step 2: Run, watch fail** (`cargo test -p shadows --test workspace_mcp --test mcp_tools --test planner_mcp`).
- [ ] **Step 3: Implement** the request reader, the three core methods, the
  three MCP tools. In `approve_plan`, the PlanApproved body is
  `Plan v{n} approved` for the person and
  `Plan v{n} approved by the Planner at your request` for the Planner.
- [ ] **Step 4: Prompt.** Replace in `prompt.txt`:
  - line 5's "The person approves it; you never approve it." with
    "The person approves it with the card's button, or asks you to approve it with vision_approve."
  - add tool lines: `vision_approve`, `map_approve`, `plan_approve`: "approve
    only when the person asked you to in this conversation; the approval is
    recorded in your name with their message".
  - line 32's waiver sentence: append "A first map with waivers is approved
    only by the person's button; map_approve refuses it."
  - lines 57–58 heading "Approval belongs to the person" → "Who approves":
    "The person approves with the button, or asks you to approve. Never
    approve unasked. Never call Shadows' HTTP API. Creating the structure on
    disk is always the person's."
  Update `planner_mcp.rs` assertions on `append` if any quote the old lines.
- [ ] **Step 5: Run** the Step 2 command → PASS; also
  `cargo test -p shadows-core --test design_vision --test design_parts` → PASS.
- [ ] **Step 6: Contracts, `gen:api` if needed, commit**
  `feat(mcp): the Planner approves at the person's request, never a waiver map (§24.2)`.

### Task 4: A read never writes

**Files:**
- Modify: `crates/shadows-core/src/testing/mod.rs` (behind `test-support`):
  `pub async fn database_writes(storage: &Storage) -> i64`, reading SQLite's
  `PRAGMA data_version` from a connection of its own. It changes whenever
  any other connection commits a write — an `UPDATE` included, which row
  counts miss. The PRAGMA stays inside `shadows-core` (CLAUDE.md's
  DB-specific syntax rule).
- Test: `crates/shadows/tests/openapi.rs`

**Interfaces:** consumes `api/openapi.json` and the test fixtures the file
already uses to start an app.

- [ ] **Step 1: Write the test** `no_get_route_changes_state`. Seed an app so
  that every path parameter has a real id: a project with a thread holding a
  completed turn, a plan with an approved version, a vision item, a pending
  vision proposal and map proposal, an approved map part, and an agreement
  (reuse the seeding helpers of `design_routes.rs`, `plan_routes.rs` and
  `agreement_routes.rs`, and `seed_map_for_tests`). Build a map from each
  `{param}` name in `api/openapi.json` to its seeded id; the test fails,
  naming the parameter, if a GET path uses a name the map lacks, so a new
  route cannot slip through with a dummy id. For every `get` operation whose
  200 response is not `text/event-stream`: read `database_writes`, call it,
  assert the status is 2xx (a 404 means the seed is wrong, not that the route
  is safe), read `database_writes` again, and assert it is unchanged, naming
  the path.
  Not covered, and the test's doc comment says so: SSE routes (they hold the
  connection open; they only read the journal), and GETs with query
  parameters other than the defaults.
- [ ] **Step 2: Run** `cargo test -p shadows --test openapi` → PASS. It should
  pass today; if a GET writes, that is a finding: fix the route and say so in
  the commit message.
- [ ] **Step 3: Commit** `test(http): no GET route changes state (§24.3 rule 5)`.

### Task 5: Show who approved

**Files:**
- Modify: `web/src/app/conversation/vision-proposal-card.tsx`, `web/src/app/conversation/map-proposal-card.tsx`, `web/src/app/conversation/plan-card.tsx` (line ~121), `web/src/app/conversation/messages.tsx` (line ~120: give each entry `id={`entry-${entry.ordinal}`}`)
- Create: `web/src/app/conversation/approved-by.tsx` — one job: the "Approved by …" line.
- Test: `web/src/app/conversation/vision-proposal-card.test.tsx`, `map-proposal-card.test.tsx`, a new `approved-by.test.tsx`

**Interfaces:** consumes `approved_by` from the generated API types (Task 2).

The request may sit in another conversation than the card (the Planner of
conversation B approved a card drawn in A). So the link names the request's
own thread, never a bare `#entry-N` (Codex review):

```tsx
/** §24.2: who approved. The Planner's line links the person's message, in
 * the conversation where it was written. */
export function ApprovedBy(
  { by, projectId }: { by: ApprovedBy | null | undefined; projectId: string },
) {
  if (by == null) return null
  if (by.agent === 'human') return <span>Approved by you</span>
  return (
    <span>
      Approved by the Planner at your request
      {by.request && (
        <> · <Link
          to="/projects/$projectId/threads/$threadId"
          params={{ projectId, threadId: by.request.thread_id }}
          hash={`entry-${by.request.ordinal}`}
        >your message</Link></>
      )}
    </span>
  )
}
```

  `Link` is TanStack Router's, as `message-actions.tsx:44` uses it. In
  `messages.tsx`, after entries load, scroll the element whose id equals the
  location hash into view (once per hash), so the link lands on the message
  whether the thread was already open or not.

- [ ] **Step 1: Failing tests.** `approved-by.test.tsx`: human → "Approved by
  you"; planner with request in thread `t2` → text and a link whose href is
  `/projects/p1/threads/t2#entry-7`, even when rendered inside thread `t1`;
  null → nothing. A `messages` test: rendering thread entries with location
  hash `#entry-3` calls `scrollIntoView` on the third entry.
  Card tests: an Approved card with `approved_by` planner shows the Planner
  line; an Approved card from before 3b (`agent: 'human'`) shows "Approved by
  you".
- [ ] **Step 2: Run, watch fail:** `npm run test -- approved-by vision-proposal-card map-proposal-card` (in `web/`).
- [ ] **Step 3: Implement** the component, use it under each card's status
  line when Approved and in the plan card beside "Approved", add the entry
  ids.
- [ ] **Step 4: Run** → PASS; `npm run typecheck`, `npm run lint`.
- [ ] **Step 5: Commit** `feat(web): show who approved each card and plan (§24.2)`.

### Task 6: Gate, acceptance, evidence

**Files:**
- Create: `docs/evidence/2026-10-12-guided-planning-pr3b.md`
- Modify: `docs/codebase/README.md` if a new module was created (owner rows)

- [ ] **Step 1: Full gate** from the root, CLAUDE.md steps 1–6, with
  `RUST_LOG` unset (`env -u RUST_LOG`), then in `web/`: `npm run test`,
  `npm run typecheck`, `npm run lint`, `npm run build`. All pass; record the
  counts.
- [ ] **Step 2: Browser acceptance on the person's own daemon** (port 4318,
  `.claude/launch.json` `daemon` and `web`; rebuild it from this branch; do
  not create a second database). In project "Book Catalogue":
  1. Ask the Planner to list its tools: no Bash, PowerShell, Write, Edit, no
     Google Drive or Claude Docs.
  2. Ask it to propose a vision edit and approve it: the card reads
     "Approved by the Planner at your request · your message"; the link
     scrolls to the message.
  3. Press Approve on another card yourself: "Approved by you".
  4. A new project's first map with a waiver: asking the Planner to approve is
     refused with the button text; the button approves it.
  5. Old cards approved before 3b read "Approved by you".
- [ ] **Step 3: Evidence file**: what ran, counts, the five steps' results,
  anything that did not match. Commit
  `docs: record PR 3b gate and acceptance`.
