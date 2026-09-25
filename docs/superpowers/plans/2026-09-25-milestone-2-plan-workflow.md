# Milestone 2 — The Planner Writes a Plan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Planner writes a versioned plan of numbered, linked tasks through Shadows' own MCP server while it talks with the person; the person sees it as a graph, can call it into the conversation, and approves it; an external Claude can connect to the same server.

**Architecture:** A pure `workflow/` module holds the plan, its edit operations and one validator. Storage applies an edit batch, the revision check, the durable event and the command record in one transaction. `mcp/` serves `/mcp` with `rmcp` over Streamable HTTP on the daemon's listener; every request is authenticated by a grant, and the tool list depends on the grant's kind. The Planner's adapter session opens with Shadows' MCP server, appended instructions and a pre-approval of Shadows' tools; its grant lives as long as the adapter. The web client adds a Workflows section drawn with React Flow and dagre, plan cards in the conversation, and project settings.

**Tech Stack:** Rust (SQLx 0.9 on SQLite, axum 0.8, utoipa, `agent-client-protocol` 2.2.0, `rmcp` 3.4.1), Node running `@agentclientprotocol/claude-agent-acp` 0.81.1, React 19 + TypeScript (TanStack Query/Router, shadcn/ui, Tailwind v4, `@xyflow/react`, `@dagrejs/dagre`), Vitest.

**Spec:** `docs/superpowers/specs/2026-09-25-planner-workflow-design.md` (§13). Every task's requirements include the spec sections it names. Where this plan and §13 disagree, §13 is right and the plan is the defect: stop and report.

## Global Constraints

- Branch: rename `milestone-2/plan-spec` to `milestone-2/plan-workflow` before Task B1 (`git branch -m`). One branch for the milestone. **No git worktrees** (Mohammed's rule). Commit per task; push only when Mohammed asks.
- Every subagent runs on opus, stated explicitly in the dispatch.
- Read `docs/codebase/README.md` and `docs/codebase/inventory.md` first; open only the files a task names (CLAUDE.md "How agents work here").
- `rmcp = { version = "=3.4.1", default-features = false, features = ["server", "macros", "transport-streamable-http-server"] }`. **Never** enable `transport-child-process` or `which-command`: `tokio::process` stays private to `process/`. The test client features (`client`, `transport-streamable-http-client-reqwest`) are enabled only through the `test-support` feature.
- Rust dependencies added in this milestone, besides `rmcp`: `petgraph = "0.8"` (B1, the cycle check) and `schemars = "1"` (B6, the same major `rmcp` 3.4.1's `server` feature pulls in, for tool input schemas). Library-first: a hand-written graph algorithm or JSON Schema is a defect.
- Web dependencies added in this milestone, exactly: `@xyflow/react` and `@dagrejs/dagre`. Not `dagre`.
- Link kinds are the strings `needs` and `completes_after`. Plan states used: `Draft`, `Frozen`; the client shows `Frozen` as "Approved".
- Error codes: add `WorkflowFrozenImmutable` (409), `WorkflowValidationFailed` (422), `RevisionConflict` (409), `GrantScope`, `GrantInvalid` (MCP tool results only) to `src/error.rs`. Wire names are the existing SCREAMING_SNAKE rendering.
- MCP errors: HTTP 401 before a tool runs; after it starts, a tool result with `isError: true` whose text begins with the code (`REVISION_CONFLICT: …`).
- MCP tool names, exactly: `workflow_list`, `workflow_get`, `task_get`, `draft_prepare`, `draft_start`, `plan_edit`, `plan_show`. Edit operations, exactly: `plan_put`, `task_add`, `task_update`, `task_remove`, `link_put`, `link_remove`.
- Session fields: `mcpServers` on the ACP request; `_meta.systemPrompt = { "append": … }`; `_meta.claudeCode.options.allowedTools = ["mcp__shadows__*"]`.
- The Planner keeps every Claude tool. Nothing in this milestone restricts them.
- Test polling interval for plan lists and grants: 10 seconds while the page is open.
- Files: 300 lines needs a stated reason, 500 splits (CLAUDE.md). `src/planner/sessions.rs` is at 459 lines: every change to it in this plan is a call into a new file, never new logic in it.
- A new module gets its one-job line in `docs/codebase/README.md`; every signature change regenerates the code map in the same commit (`UPDATE_CODEMAP=1 cargo test --test codemap`). Every route change regenerates `api/openapi.json` (`UPDATE_OPENAPI=1 cargo test --test openapi`) and `web/src/api/schema.d.ts` (`npm --prefix web run gen:api`) in the same commit.
- The gate before every commit: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; for web changes also `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web test`. Report the test counts (at the start: 202 Rust, 95 web).
- Test helpers live in `tests/fixtures/` (Rust) and `web/src/test/` (TypeScript). Reuse the existing helper of a name where one exists; the task that first needs a missing one adds it there.

## Review Focus

1. **The Planner fires two `plan_edit` calls at once on the same revision** (Claude runs tools in parallel). The person expects both changes to land, not one lost silently. One succeeds, the other gets `REVISION_CONFLICT` with the current revision and a summary, and a retry on the new revision succeeds. (Task B6 test `parallel_edits_on_one_revision_conflict_and_the_loser_can_retry`.)
2. **The Planner adds "T4" when T4 already exists.** The person expects the existing task untouched. `task_add` refuses a number in use. (Task B1 test `task_add_refuses_a_number_in_use`.)
3. **Arabic titles, goals, labels and acceptance items.** Mohammed writes in Arabic; the text must survive MCP → storage → HTTP → graph unchanged, and render right-to-left inside nodes. (Task B1 test `arabic_text_survives_an_edit`; Task B6 test `arabic_plan_round_trips_through_mcp`; Task W1 test `an Arabic task title renders with dir auto`.)
4. **The person presses Approve while the Planner is still editing.** The person expects to approve exactly what they saw, or be told it changed. (Task B4 test `approving_a_stale_revision_is_refused_with_the_current_one`; Task W1 test `a revision conflict on Approve refetches and says the plan changed`.)
5. **The conversation sat idle past 15 minutes, or the daemon restarted, then the Planner keeps editing.** The person expects editing to work and the old token to be dead. (Task B7 tests `reopening_after_idle_issues_a_new_grant_and_revokes_the_old`, `restart_revokes_every_internal_grant`.)

---

## Execution map

```text
Task 0  (controller, throwaway probe)      ─► docs/evidence/milestone2/MCP_PROBE.md; amends §13 if a probe fails
Backend (sequential, one agent per task):
  B1 plan domain + validator ─► B2 entry kinds ─► B3 schema + plan storage ─► B4 plan routes
  ─► B5 grants + instructions ─► B6 MCP server ─► B7 Planner session ─► B8 plan in the conversation
Web (sequential, after B8):
  W1 Workflows section + graph ─► W2 plan in the conversation ─► W3 project settings
Task I (controller): whole-branch review, run with Mohammed, evidence, PR
```

---

### Task 0: Probe the adapter with a throwaway MCP server

Run by the controller on Windows against adapter 0.81.1 and Claude Code 2.1.281, from a scratch directory outside the repository, with a raw JSON-RPC ACP client like `ACP_PROBE.md`'s and a minimal MCP server (any language) exposing one tool `echo_marker` over Streamable HTTP that requires `Authorization: Bearer probe-token`. Use `sonnet` to save the weekly limit.

- [ ] **Step 1: Instructions append.** `session/new` with `_meta.systemPrompt = { "append": "When asked for the marker, answer exactly: ORCHID-7." }`. Prompt "what is the marker?". Expected: `ORCHID-7`. Then prompt "list your tools" and record that Claude Code's own tools (Read, Bash, …) are still listed.
- [ ] **Step 2: Pre-approved MCP tool in Accept edits.** Same session with `mcpServers: [{ "type": "http", "name": "shadows", "url": "http://127.0.0.1:<port>/mcp", "headers": [{ "name": "Authorization", "value": "Bearer probe-token" }] }]` and `_meta.claudeCode.options.allowedTools = ["mcp__shadows__*"]`; set mode `acceptEdits`; prompt "call echo_marker with text hi". Expected: the tool runs, and no `session/request_permission` arrives. Repeat without `allowedTools` and record whether a permission request arrives (it should).
- [ ] **Step 3: Resume with changed instructions keeps the conversation.** Prompt "remember the word amber. reply only ok". Send `session/resume` on the **same** live adapter with `append` changed to "…answer exactly: LILAC-3." Prompt "what is the marker, and what word did I ask you to remember?". Expected: `LILAC-3` and `amber`. Record whether the adapter's stderr shows a session rebuild.
- [ ] **Step 4: Protocol version.** Record the `MCP-Protocol-Version` header and `protocolVersion` Claude Code sends to the probe server, and confirm `rmcp` 3.4.1's supported versions include it (`cargo doc -p rmcp` or its source constant).
- [ ] **Step 5: Revoked header.** Make the probe server answer 401 and record what Claude Code shows for the server (expected per its docs: failed connection, no OAuth).
- [ ] **Step 6: Evidence.** Write `docs/evidence/milestone2/MCP_PROBE.md` in `ACP_PROBE.md`'s format (date, versions, each check with trimmed wire lines, what it changes). If any of Steps 1–4 fails, amend §13.8 or §13.6 in place before Task B1 and say so to Mohammed. Delete the probe code; commit the evidence (`docs(evidence): MCP probe for Milestone 2`).

---

### Task B1: The plan, its edits, and the validator (pure)

**Files:**
- Create: `src/workflow/mod.rs` (ids, states, content types, re-exports), `src/workflow/ops.rs` (edit operations and `apply`), `src/workflow/check.rs` (validator)
- Modify: `src/lib.rs` (`pub mod workflow;`), `docs/codebase/README.md` (three owner lines)
- Test: `tests/workflow_rules.rs`

**Interfaces:**
- Produces (all `pub`, all `serde::{Serialize, Deserialize}` + `utoipa::ToSchema` + `Debug, Clone, PartialEq`):

```rust
// src/workflow/mod.rs
newtype_id! { /// Spec §13.2. One version of a plan.
    WorkflowId }
newtype_id! { /// Spec §13.3.
    TaskId }

pub enum WorkflowState { Draft, Frozen }            // serialised "Draft" / "Frozen"
#[serde(rename_all = "snake_case")]
pub enum LinkKind { Needs, CompletesAfter }         // "needs" / "completes_after"

pub struct AcceptanceItem { pub number: u32, pub text: String }
pub struct TaskContent {
    pub number: u32, pub title: String, pub goal: String,
    pub reads: Vec<String>, pub writes: Vec<String>,
    pub acceptance: Vec<AcceptanceItem>,
}
/// `task` waits for `after`.
pub struct Link {
    pub task: u32, pub after: u32, pub kind: LinkKind,
    pub label: String,
    #[serde(default)] pub waiting_items: Vec<u32>,  // empty for `needs`
}
pub struct PlanContent {
    pub title: String, pub goal: String,
    pub tasks: std::collections::BTreeMap<u32, TaskContent>,
    pub links: Vec<Link>,
}

// src/workflow/ops.rs
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PlanOp {
    PlanPut { title: String, goal: String },
    TaskAdd { task: TaskContent },
    TaskUpdate { task: TaskContent },
    TaskRemove { number: u32 },
    LinkPut { link: Link },
    LinkRemove { task: u32, after: u32, kind: LinkKind },
}
pub struct Applied { pub content: PlanContent, pub changed_tasks: Vec<u32>, pub summary: String }
pub fn apply(current: &PlanContent, ops: &[PlanOp]) -> Result<Applied, Vec<Problem>>

// src/workflow/check.rs
pub struct Problem { pub message: String }
pub fn edit_problems(content: &PlanContent) -> Vec<Problem>
pub fn approval_problems(content: &PlanContent) -> Vec<Problem>   // includes edit_problems
```

Rules (§13.3, §13.4), each one a `Problem` whose message names tasks as `T4`:
- `apply` refuses, before checking the final state: an empty `ops`; two ops on one object (object = `Plan` for `plan_put`; task number for `task_*`; `(task, after, kind)` for `link_*`); `task_add` of a number in use; `task_update` / `task_remove` of a number not in use; `link_remove` of a link that does not exist; a task number of 0.
- `edit_problems`: a link naming a missing task; a task linked to itself; `needs` with `waiting_items`; `completes_after` with none, or naming an acceptance number the waiting task lacks; acceptance numbers not unique within a task; a cycle over start/complete events (below).
- `approval_problems` adds: empty title; no tasks; a task with an empty goal; a task with no acceptance item.
- `summary`: `"<n> changes: added T4, updated T2, removed T7, linked T4 → T2, unlinked T3 → T1, renamed the plan"` (only the parts that occurred, in op order). `changed_tasks`: sorted, deduplicated task numbers named by `task_*` ops and by both ends of `link_*` ops.

- [ ] **Step 1: Write the failing tests** in `tests/workflow_rules.rs`:

```rust
use shadows::workflow::*;

fn task(n: u32, title: &str) -> TaskContent {
    TaskContent { number: n, title: title.into(), goal: format!("goal {n}"),
        reads: vec![], writes: vec![format!("src/t{n}")],
        acceptance: vec![AcceptanceItem { number: 1, text: format!("T{n} works") }] }
}
fn empty() -> PlanContent {
    PlanContent { title: "Login".into(), goal: "people can log in".into(),
        tasks: Default::default(), links: vec![] }
}
fn needs(task: u32, after: u32) -> Link {
    Link { task, after, kind: LinkKind::Needs, label: "api".into(), waiting_items: vec![] }
}
fn messages(p: Vec<Problem>) -> Vec<String> { p.into_iter().map(|p| p.message).collect() }

#[test]
fn adding_and_linking_in_one_edit_is_valid() {
    let a = apply(&empty(), &[
        PlanOp::TaskAdd { task: task(1, "table") },
        PlanOp::TaskAdd { task: task(2, "api") },
        PlanOp::LinkPut { link: needs(2, 1) },
    ]).unwrap();
    assert_eq!(a.content.tasks.len(), 2);
    assert_eq!(a.changed_tasks, vec![1, 2]);
    assert_eq!(a.summary, "3 changes: added T1, added T2, linked T2 → T1");
}

#[test]
fn task_add_refuses_a_number_in_use() {
    let base = apply(&empty(), &[PlanOp::TaskAdd { task: task(4, "screen") }]).unwrap().content;
    let err = apply(&base, &[PlanOp::TaskAdd { task: task(4, "other") }]).unwrap_err();
    assert_eq!(messages(err), vec!["T4 already exists; use task_update to change it"]);
    assert_eq!(base.tasks[&4].title, "screen");
}

#[test]
fn task_update_refuses_an_unknown_number() {
    let err = apply(&empty(), &[PlanOp::TaskUpdate { task: task(9, "x") }]).unwrap_err();
    assert_eq!(messages(err), vec!["T9 does not exist in this plan"]);
}

#[test]
fn two_operations_on_one_task_in_one_edit_are_refused() {
    let err = apply(&empty(), &[
        PlanOp::TaskAdd { task: task(1, "a") },
        PlanOp::TaskUpdate { task: task(1, "b") },
    ]).unwrap_err();
    assert_eq!(messages(err), vec!["two operations on T1 in one edit"]);
}

#[test]
fn removing_a_linked_task_is_refused_until_its_links_go() {
    let base = apply(&empty(), &[
        PlanOp::TaskAdd { task: task(1, "a") }, PlanOp::TaskAdd { task: task(2, "b") },
        PlanOp::LinkPut { link: needs(2, 1) },
    ]).unwrap().content;
    let err = apply(&base, &[PlanOp::TaskRemove { number: 1 }]).unwrap_err();
    assert_eq!(messages(err), vec!["T1 is still linked from T2"]);
    let ok = apply(&base, &[
        PlanOp::LinkRemove { task: 2, after: 1, kind: LinkKind::Needs },
        PlanOp::TaskRemove { number: 1 },
    ]);
    assert!(ok.is_ok());
}

#[test]
fn a_cycle_across_both_kinds_is_refused() {
    // T2 needs T4, and part of T4 waits for T2: complete(T4) → start(T2) → complete(T2) → complete(T4).
    let base = apply(&empty(), &[
        PlanOp::TaskAdd { task: task(2, "a") }, PlanOp::TaskAdd { task: task(4, "b") },
    ]).unwrap().content;
    let err = apply(&base, &[
        PlanOp::LinkPut { link: needs(2, 4) },
        PlanOp::LinkPut { link: Link { task: 4, after: 2, kind: LinkKind::CompletesAfter,
            label: "mail".into(), waiting_items: vec![1] } },
    ]).unwrap_err();
    assert_eq!(messages(err), vec!["these links form a cycle: T2, T4"]);
}

#[test]
fn completes_after_alone_in_both_directions_is_a_cycle() {
    let base = apply(&empty(), &[
        PlanOp::TaskAdd { task: task(1, "a") }, PlanOp::TaskAdd { task: task(2, "b") },
    ]).unwrap().content;
    let wait = |t, a| Link { task: t, after: a, kind: LinkKind::CompletesAfter, label: "x".into(), waiting_items: vec![1] };
    let err = apply(&base, &[PlanOp::LinkPut { link: wait(1, 2) }, PlanOp::LinkPut { link: wait(2, 1) }]).unwrap_err();
    assert_eq!(messages(err), vec!["these links form a cycle: T1, T2"]);
}

#[test]
fn a_task_after_a_cycle_is_not_named_in_it() {
    let base = apply(&empty(), &[
        PlanOp::TaskAdd { task: task(1, "a") }, PlanOp::TaskAdd { task: task(2, "b") },
        PlanOp::TaskAdd { task: task(3, "c") },
    ]).unwrap().content;
    let err = apply(&base, &[
        PlanOp::LinkPut { link: needs(1, 2) }, PlanOp::LinkPut { link: needs(2, 1) },
        PlanOp::LinkPut { link: needs(3, 2) },
    ]).unwrap_err();
    assert_eq!(messages(err), vec!["these links form a cycle: T1, T2"]);
}

#[test]
fn completes_after_one_way_with_needs_the_other_way_is_valid() {
    // T3 needs T2; part of T3 waits for T8 — no cycle.
    let base = apply(&empty(), &[
        PlanOp::TaskAdd { task: task(2, "api") }, PlanOp::TaskAdd { task: task(3, "reset") },
        PlanOp::TaskAdd { task: task(8, "mail") },
    ]).unwrap().content;
    let ok = apply(&base, &[
        PlanOp::LinkPut { link: needs(3, 2) },
        PlanOp::LinkPut { link: Link { task: 3, after: 8, kind: LinkKind::CompletesAfter,
            label: "email sender".into(), waiting_items: vec![1] } },
    ]);
    assert!(ok.is_ok());
}

#[test]
fn waiting_items_must_exist_on_the_waiting_task() {
    let base = apply(&empty(), &[
        PlanOp::TaskAdd { task: task(1, "a") }, PlanOp::TaskAdd { task: task(2, "b") },
    ]).unwrap().content;
    let err = apply(&base, &[PlanOp::LinkPut { link: Link { task: 2, after: 1,
        kind: LinkKind::CompletesAfter, label: "x".into(), waiting_items: vec![5] } }]).unwrap_err();
    assert_eq!(messages(err), vec!["T2 has no acceptance item 5"]);
}

#[test]
fn a_link_to_a_missing_task_or_to_itself_is_refused() {
    let base = apply(&empty(), &[PlanOp::TaskAdd { task: task(1, "a") }]).unwrap().content;
    assert_eq!(messages(apply(&base, &[PlanOp::LinkPut { link: needs(1, 7) }]).unwrap_err()),
        vec!["the link from T1 names T7, which does not exist"]);
    assert_eq!(messages(apply(&base, &[PlanOp::LinkPut { link: needs(1, 1) }]).unwrap_err()),
        vec!["T1 cannot be linked to itself"]);
}

#[test]
fn a_draft_may_lack_goals_but_approval_lists_what_is_missing() {
    let mut t = task(3, "bare");
    t.goal = String::new();
    t.acceptance.clear();
    let a = apply(&empty(), &[PlanOp::TaskAdd { task: t }]).unwrap();
    assert!(edit_problems(&a.content).is_empty());
    assert_eq!(messages(approval_problems(&a.content)),
        vec!["T3 has no goal", "T3 has no acceptance item"]);
}

#[test]
fn arabic_text_survives_an_edit() {
    let op: PlanOp = serde_json::from_str(
        r#"{"op":"task_add","task":{"number":1,"title":"جدول المستخدمين","goal":"تسجيل الدخول",
            "reads":[],"writes":["db/migrations"],"acceptance":[{"number":1,"text":"يظهر خطأ عند كلمة سر خاطئة"}]}}"#
    ).unwrap();
    let a = apply(&empty(), &[op]).unwrap();
    assert_eq!(a.content.tasks[&1].title, "جدول المستخدمين");
    assert_eq!(a.content.tasks[&1].acceptance[0].text, "يظهر خطأ عند كلمة سر خاطئة");
}
```

- [ ] **Step 2: Run** `cargo test --test workflow_rules`. Expected: compile failure (`workflow` not found).
- [ ] **Step 3: Implement.** The cycle check in `check.rs` uses `petgraph` (library-first; `petgraph = "0.8"` added to `Cargo.toml` in this task):

```rust
use petgraph::{algo::tarjan_scc, graph::DiGraph};

/// §13.4: each task is two events, start and complete, start before complete.
/// `B needs A`: complete(A) → start(B). `B completes_after A`: complete(A) → complete(B).
/// A cycle is a strongly connected set of more than one event; its tasks are
/// named, never a task that merely comes after it. When several exist, the one
/// whose sorted task list is smallest is reported, so the message is stable.
fn cycle(content: &PlanContent) -> Option<Vec<u32>> {
    let mut g = DiGraph::<u32, ()>::new();
    let events: std::collections::BTreeMap<u32, _> = content.tasks.keys()
        .map(|&n| { let s = g.add_node(n); let c = g.add_node(n); g.add_edge(s, c, ()); (n, (s, c)) })
        .collect();
    for l in &content.links {
        if l.task == l.after { continue; } // reported as "cannot be linked to itself"
        let (Some(&(b_start, b_complete)), Some(&(_, a_complete))) = (events.get(&l.task), events.get(&l.after))
            else { continue }; // reported as a missing task
        let to = match l.kind { LinkKind::Needs => b_start, LinkKind::CompletesAfter => b_complete };
        g.add_edge(a_complete, to, ());
    }
    tarjan_scc(&g).into_iter()
        .filter(|scc| scc.len() > 1)
        .map(|scc| { let mut t: Vec<u32> = scc.iter().map(|&i| g[i]).collect(); t.sort_unstable(); t.dedup(); t })
        .min()
}
```

Its problem message: `format!("these links form a cycle: {}", tasks.iter().map(|n| format!("T{n}")).collect::<Vec<_>>().join(", "))`. Order the other problems deterministically (tasks ascending, then links in stored order) so messages are stable.

- [ ] **Step 4: Run** `cargo test --test workflow_rules`. Expected: all pass. Then the full gate.
- [ ] **Step 5: Commit.** `UPDATE_CODEMAP=1 cargo test --test codemap`; add to `docs/codebase/README.md`: `src/workflow/` — "a plan's content under the rules of §13"; `src/workflow/ops.rs` — "applying one batch of plan edits"; `src/workflow/check.rs` — "what makes a plan invalid or unready". `git commit -m "feat(workflow): the plan, its edit operations and the validator (§13.3–§13.4)"`.

---

### Task B2: Typed entry kinds and task references

**Files:**
- Modify: `src/thread/mod.rs` (`ThreadEntryKind`, `EntryRef`), `src/planner/turn.rs`, `src/storage/sqlite/turn.rs`, `src/storage/sqlite/entry.rs`, every other writer of `NewThreadEntry` the compiler finds
- Test: `tests/thread_contract.rs` (extend)

**Interfaces:**
- Produces:

```rust
// src/thread/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub enum ThreadEntryKind { UserMessage, AgentMessage, PermissionRefused, PlanView, PlanApproved }
impl ThreadEntryKind { pub fn as_str(self) -> &'static str; pub fn parse(s: &str) -> Option<Self> }

pub enum EntryRef {
    Operation(OperationId),
    Decision(String),
    Research(String),
    Workflow(WorkflowId),   // was String
    Task(TaskId),           // new, §13.9
}
// ThreadEntry.kind: ThreadEntryKind; NewThreadEntry.kind: ThreadEntryKind
```

Stored text does not change: each variant is stored as its name. Reading an unknown stored kind is `StorageError::Constraint("unknown thread entry kind: …")`.

- [ ] **Step 1: Failing tests** in `tests/thread_contract.rs`:

```rust
#[tokio::test]
async fn every_entry_kind_round_trips_as_its_stored_name() {
    let app = test_app().await;
    let thread = create_thread(&app, json!({ "command_id": fresh_command(), "title": "t", "harness": "claude-code" })).await;
    let thread_id = ThreadId::from_literal(thread["id"].as_str().unwrap());
    for kind in [ThreadEntryKind::UserMessage, ThreadEntryKind::AgentMessage,
                 ThreadEntryKind::PermissionRefused, ThreadEntryKind::PlanView, ThreadEntryKind::PlanApproved] {
        app.storage.append_thread_entry(&thread_id, NewThreadEntry {
            kind, author: Actor::system(), body: "b", refs: &[], operation_id: None }).await.unwrap();
    }
    let kinds: Vec<_> = entries_on(&app, &thread_id).await.into_iter().map(|e| e.kind).collect();
    assert_eq!(kinds.len(), 5);
    let stored: Vec<String> = sqlx::query_scalar("SELECT kind FROM thread_entry ORDER BY ordinal")
        .fetch_all(app.storage.reader()).await.unwrap();
    assert_eq!(stored, ["UserMessage", "AgentMessage", "PermissionRefused", "PlanView", "PlanApproved"]);
}

#[tokio::test]
async fn a_task_reference_round_trips() {
    let app = test_app().await;
    let thread = create_thread(&app, json!({ "command_id": fresh_command(), "title": "t", "harness": "claude-code" })).await;
    let thread_id = ThreadId::from_literal(thread["id"].as_str().unwrap());
    let refs = [EntryRef::Workflow(WorkflowId::from_literal("w1")), EntryRef::Task(TaskId::from_literal("t1"))];
    app.storage.append_thread_entry(&thread_id, NewThreadEntry {
        kind: ThreadEntryKind::UserMessage, author: Actor::user("local"), body: "change this",
        refs: &refs, operation_id: None }).await.unwrap();
    assert_eq!(entries_on(&app, &thread_id).await[0].refs, refs.to_vec());
}
```

(Use the fixture names that exist in `tests/fixtures/app.rs`; `entries_on` exists.)

- [ ] **Step 2: Run** `cargo test --test thread_contract`. Expected: compile failure.
- [ ] **Step 3: Implement.** Replace every `kind: "…"` literal for entries with the enum (`turn.rs` persist, `sqlite/turn.rs` user message, `PermissionRefused`). Keep `Actor.kind` strings as they are: they are authors, not entry kinds. `Workflow(WorkflowId)` serialises exactly as the old `Workflow(String)` (the id newtype is a string), so stored `refs_json` still reads.
- [ ] **Step 4: Run** the full gate. Regenerate code map, `api/openapi.json` and `web/src/api/schema.d.ts` (the entry schema's `kind` becomes an enum); `npm --prefix web run typecheck` must still pass — fix any web narrowing the new enum type exposes.
- [ ] **Step 5: Commit** `refactor(thread): typed entry kinds and task references (§4, §13.9)`.

---

### Task B3: Plan schema and storage

**Files:**
- Create: `migrations/0007_plan_workflow.sql`, `src/storage/sqlite/workflow.rs` (writes), `src/storage/sqlite/workflow_read.rs` (reads), `src/command/derive.rs`, `src/mcp/mod.rs`, `src/mcp/grant.rs` (only `GrantId` here; B5 fills it)
- Modify: `src/storage/sqlite/mod.rs` (`StorageError` variants, `mod` lines), `src/command/mod.rs` (`pub mod derive; pub enum Writer`), `src/workflow/mod.rs` (`Plan`, `PlanListing`, `LastEdit`), `src/lib.rs` (`pub mod mcp;`)
- Test: `tests/plan_storage.rs` (new; not `storage_contract.rs`, an accretion point)

**Interfaces:**
- Consumes: B1's `PlanContent`, `PlanOp`, `apply`, `edit_problems`, `approval_problems`, `Problem`; B2's `ThreadEntryKind::PlanApproved`, `EntryRef`.
- Produces:

```rust
// src/mcp/grant.rs
newtype_id! { /// Spec §13.7.
    GrantId }

// src/command/mod.rs — who writes (§13.5). The principal of the command record.
pub enum Writer {
    Person,                                        // principal ("User", "local")
    Planner { thread: ThreadId, grant: GrantId },  // principal ("Thread", thread)
    External { grant: GrantId },                   // principal ("Grant", grant)
}
impl Writer { pub fn principal(&self) -> (&'static str, String); pub fn actor(&self) -> Actor }

// src/command/derive.rs — §13.5's derived command ids.
pub enum Anchor<'a> { Revision(i64), Operation(&'a OperationId), DraftRef(&'a str) }
pub fn derived_id(anchor: Anchor<'_>, fingerprint: &str) -> String
// "rev:7:<first 16 hex of fingerprint>", "op:<op id>:<16 hex>", "ref:<draft_ref>"

// src/workflow/mod.rs (domain, no storage imports)
pub struct LastEdit { pub revision: i64, pub summary: String, pub changed_tasks: Vec<u32> }
pub struct PlanTask { pub id: TaskId, #[serde(flatten)] pub content: TaskContent }
pub struct Plan {
    pub id: WorkflowId, pub thread_id: ThreadId, pub project_id: ProjectId,
    pub version: i64, pub revision: i64, pub state: WorkflowState,
    pub title: String, pub goal: String,
    pub previous: Option<WorkflowId>, pub next: Option<WorkflowId>,
    pub tasks: Vec<PlanTask>, pub links: Vec<Link>,
    pub blockers: Vec<Problem>,          // approval_problems, for a Draft; empty for Frozen
    pub last_edit: Option<LastEdit>,
    pub frozen_at: Option<String>, pub created_at: String,
}
/// What a command did, fixed when it committed, so a replay answers exactly
/// that and not the plan as it is later (§13.5).
pub struct EditOutcome { pub workflow_id: WorkflowId, pub version: i64, pub revision: i64,
    pub summary: String, pub changed_tasks: Vec<u32> }
pub struct DraftStarted { pub workflow_id: WorkflowId, pub thread_id: ThreadId, pub version: i64 }
pub struct Approved { pub workflow_id: WorkflowId, pub version: i64, pub revision: i64, pub frozen_at: String }
pub struct PlanListing { pub id: WorkflowId, pub thread_id: ThreadId, pub title: String,
    pub version: i64, pub state: WorkflowState, pub updated_at: String }

// src/storage/sqlite/mod.rs — StorageError gains
RevisionConflict { current: i64, summary: String },
WorkflowFrozen,
PlanInvalid(Vec<Problem>),
GrantInvalid,
GrantScope,

// src/storage/sqlite/workflow.rs
impl Storage {
    /// §13.6 draft_start in a thread: v1 from `fresh` when the thread has no plan;
    /// a copy of the latest frozen version otherwise (`fresh` ignored);
    /// the existing Draft, unchanged, when there is one.
    pub async fn start_draft(&self, ctx: &CommandContext, writer: &Writer, thread: &ThreadId,
        fresh: Option<(&str, &str)>, draft_ref: Option<&str>) -> Result<DraftStarted, StorageError>;
    /// §13.6 external draft_start from scratch: a planning thread titled `title`
    /// and its v1, in `project`, harness `claude-code`.
    pub async fn start_thread_with_draft(&self, ctx: &CommandContext, writer: &Writer,
        project: &ProjectId, title: &str, goal: &str, draft_ref: Option<&str>) -> Result<DraftStarted, StorageError>;
    pub async fn edit_plan(&self, ctx: &CommandContext, writer: &Writer, workflow: &WorkflowId,
        expected_revision: i64, ops: &[PlanOp]) -> Result<EditOutcome, StorageError>;
    pub async fn approve_plan(&self, ctx: &CommandContext, workflow: &WorkflowId,
        expected_revision: i64) -> Result<Approved, StorageError>;
}
// src/storage/sqlite/workflow_read.rs
impl Storage {
    pub async fn get_plan(&self, workflow: &WorkflowId) -> Result<Plan, StorageError>;
    pub async fn list_plans(&self, project: &ProjectId) -> Result<Vec<PlanListing>, StorageError>; // latest version per thread
    pub async fn thread_plan(&self, thread: &ThreadId) -> Result<Option<WorkflowId>, StorageError>; // latest version
}
```

Durable events (kind, payload), each with the plan's thread and project and the writer's actor:
`WorkflowDraftStarted {workflow_id, version}`, `WorkflowEdited {workflow_id, version, revision, summary, changed_tasks}`, `WorkflowFrozen {workflow_id, version}`.

- [ ] **Step 1: The migration** `migrations/0007_plan_workflow.sql`:

```sql
-- Milestone 2 (§13.15). workflow/task/task_parent did not exist before this.
CREATE UNIQUE INDEX planning_thread_id_project ON planning_thread(id, project_id);

CREATE TABLE workflow (
    id                  TEXT PRIMARY KEY,
    thread_id           TEXT NOT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    state               TEXT NOT NULL CHECK (state IN ('Draft','Approved','Frozen','Running','Completed','Failed')),
    previous_version_id TEXT NULL,
    source_plan_json    TEXT NULL,
    version             INTEGER NOT NULL CHECK (version >= 1),
    revision            INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
    title               TEXT NOT NULL,
    goal                TEXT NOT NULL,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    frozen_at           TEXT NULL,
    UNIQUE (id, thread_id),
    UNIQUE (previous_version_id),
    UNIQUE (thread_id, version),
    FOREIGN KEY (previous_version_id, thread_id) REFERENCES workflow(id, thread_id),
    CHECK (state NOT IN ('Frozen','Running','Completed','Failed') OR frozen_at IS NOT NULL)
);

CREATE TABLE task (
    id            TEXT PRIMARY KEY,
    workflow_id   TEXT NOT NULL REFERENCES workflow(id) ON DELETE RESTRICT,
    number        INTEGER NOT NULL CHECK (number >= 1),
    contract_json TEXT NOT NULL,
    scope_json    TEXT NOT NULL,
    state         TEXT NOT NULL DEFAULT 'Pending'
                  CHECK (state IN ('Pending','Ready','InProgress','Completed','Failed','Blocked')),
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    UNIQUE (id, workflow_id),
    UNIQUE (workflow_id, number)
);

CREATE TABLE task_parent (
    workflow_id   TEXT NOT NULL,
    task_id       TEXT NOT NULL,
    parent_id     TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK (kind IN ('needs','completes_after')),
    label         TEXT NOT NULL,
    waiting_items TEXT NULL,
    PRIMARY KEY (workflow_id, task_id, parent_id, kind),
    FOREIGN KEY (task_id, workflow_id) REFERENCES task(id, workflow_id) ON DELETE RESTRICT,
    FOREIGN KEY (parent_id, workflow_id) REFERENCES task(id, workflow_id) ON DELETE RESTRICT,
    CHECK (task_id != parent_id),
    CHECK ((kind = 'completes_after') = (waiting_items IS NOT NULL))
);
CREATE INDEX task_parent_by_parent ON task_parent(workflow_id, parent_id);

CREATE TABLE planner_instructions_version (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id),
    number     INTEGER NOT NULL CHECK (number >= 1),
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (project_id, number)
);

CREATE TABLE mcp_grant (
    id         TEXT PRIMARY KEY,
    kind       TEXT NOT NULL CHECK (kind IN ('thread','project')),
    thread_id  TEXT NULL,
    project_id TEXT NOT NULL REFERENCES project(id),
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    revoked_at TEXT NULL,
    CHECK ((kind = 'thread') = (thread_id IS NOT NULL)),
    FOREIGN KEY (thread_id, project_id) REFERENCES planning_thread(id, project_id)
);

CREATE TABLE draft_intent (
    draft_ref   TEXT PRIMARY KEY,
    grant_id    TEXT NOT NULL REFERENCES mcp_grant(id),
    workflow_id TEXT NULL REFERENCES workflow(id),
    created_at  TEXT NOT NULL,
    expires_at  TEXT NOT NULL
);

ALTER TABLE agent_invocation ADD COLUMN prompt_version TEXT NULL;
ALTER TABLE agent_invocation ADD COLUMN planner_instructions_version_id TEXT NULL
    REFERENCES planner_instructions_version(id);
```

Before writing it, check the SQLite version bundled with SQLx accepts `ALTER TABLE … ADD COLUMN … REFERENCES` with a NULL default (it does since 3.x with foreign keys on, for a NULL default). If `0005_harness_controls.sql` shows a different pattern for adding columns, follow it.

- [ ] **Step 2: Failing tests** in `tests/plan_storage.rs` (a thread created through the existing fixture; `ops` built with B1's types):
  - `a_new_draft_is_version_one_at_revision_zero`
  - `start_draft_answers_the_existing_draft_unchanged`
  - `an_edit_moves_the_revision_once_and_records_one_event` — one `edit_plan` with three ops → revision 1; exactly one `WorkflowEdited` event for the thread with `changed_tasks` and `summary`.
  - `an_edit_on_a_stale_revision_is_refused_with_the_current_one_and_a_summary` — expect `StorageError::RevisionConflict { current: 1, summary }` where `summary` is the first edit's summary.
  - `a_replayed_edit_returns_its_own_outcome_after_a_later_edit` — edit A on revision 0 (→ 1), edit B on revision 1 (→ 2), then A's exact `ctx` again → `EditOutcome { revision: 1, summary: A's }`, not revision 2; still two `WorkflowEdited` events.
  - `a_replayed_draft_start_answers_the_same_version_after_edits` — `start_draft`, an edit, then the same `start_draft` ctx → the same `DraftStarted`.
  - `a_draft_ref_binds_in_both_paths` — `start_thread_with_draft(…, Some(r1))` and, after freezing another plan, `start_draft(…, that plan's thread, None, Some(r2))` each set `draft_intent.workflow_id` in their transaction; a ref bound to a plan outside the grant's project, an expired ref and another grant's ref are each refused `GrantScope`.
  - `the_same_explicit_command_id_with_different_ops_is_a_command_conflict`
  - `an_invalid_edit_writes_nothing` — `PlanInvalid(problems)`; revision unchanged; no event.
  - `a_frozen_version_refuses_edits` → `WorkflowFrozen`.
  - `approval_freezes_and_writes_a_plan_approved_entry` — state `Frozen`, `frozen_at` set, a `PlanApproved` thread entry "Plan v1 approved" with `EntryRef::Workflow`, a `WorkflowFrozen` event.
  - `a_replayed_approval_answers_its_own_outcome_after_v2_exists` — approve v1, `start_draft` v2, replay the approval's `ctx` → the same `Approved` as the first answer; v1's `get_plan` now has `next = v2` but the replay does not carry it.
  - `approval_with_blockers_is_refused_with_the_list` → `PlanInvalid`.
  - `after_approval_start_draft_copies_tasks_and_links_with_their_numbers` — v2 `Draft`, `previous` = v1, same numbers, new task ids, revision 0; v1 unchanged.
  - `an_edit_by_a_revoked_grant_writes_nothing` — insert an `mcp_grant` row, revoke it (`UPDATE … SET revoked_at`), call `edit_plan` with `Writer::External` → `GrantInvalid`, revision unchanged. This is the in-transaction check of §13.7.
  - `start_thread_with_draft_creates_both_and_records_the_grant_as_actor` — the `WorkflowDraftStarted` event's actor is `("Grant", id)`.
  - `reads_and_writes_are_stored_in_scope_json` — read the row back: `contract_json` has title/goal/acceptance only; `scope_json` has `reads`/`writes`.

- [ ] **Step 3: Run** `cargo test --test plan_storage`. Expected: fails to compile.
- [ ] **Step 4: Implement.** Follow `storage/sqlite/project.rs` for the command pattern. `edit_plan` inside `write_txn`, in this order (§13.5):
  1. if the writer holds a grant: `SELECT revoked_at FROM mcp_grant WHERE id = ?` — missing or revoked → `GrantInvalid`;
  2. `classify(conn, ctx, "Workflow", workflow)` — `Some(event_id)` → replay: read that `WorkflowEdited` event's payload and return it as the `EditOutcome`, never the plan as it is now; a fingerprint mismatch already surfaces as `CommandConflict` from `classify`;
  3. load the version; `Frozen` → `WorkflowFrozen`; `revision != expected_revision` → `RevisionConflict { current, summary }` where `summary` joins the `summary` of every `WorkflowEdited` event of this workflow with revision > expected (read from `durable_event` by `thread_id` and `kind`, ordered by `seq`);
  4. `workflow::apply(&content, ops)` — `Err(p)` → `PlanInvalid(p)`;
  5. write the diff: delete removed tasks' rows and links, upsert tasks by `(workflow_id, number)` (new ones get `TaskId::generate()`), replace the version's `task_parent` rows with the new links (task ids resolved by number); `revision = revision + 1`, `updated_at`;
  6. append `WorkflowEdited` whose payload is the `EditOutcome`; `record_command(conn, ctx, "Workflow", workflow, "WorkflowEdited", <that event's event_id>, &ts)` — the outcome reference is the event, which never changes.

  `start_draft` and `start_thread_with_draft` record the version's id as their outcome and answer `DraftStarted`, which does not change as the draft is edited. With `draft_ref: Some(r)`, both, inside their transaction: `UPDATE draft_intent SET workflow_id = ? WHERE draft_ref = ? AND grant_id = ? AND ((workflow_id IS NULL AND expires_at > ?) OR workflow_id = ?)` — expiry stops only a ref's first use; a ref bound to a plan stays good for replaying that plan's start forever, and check the plan's project equals the grant's; zero rows or another project → `GrantScope`. `approve_plan` writes a `WorkflowFrozen` event whose payload is the `Approved`, records that event's id as its outcome, and a replay answers the event's payload — never `get_plan`, whose `next` changes once v2 exists. Every command here answers only what was fixed when it committed.

  Callers build `ctx` with `command::derived_id(Anchor::Revision(expected_revision), &fingerprint)` when no explicit id was given; the storage method does not derive ids itself. Add a helper for tests: `tests/fixtures/plan.rs` with `edit_ctx(writer: &Writer, workflow: &WorkflowId, expected: i64, ops: &[PlanOp]) -> CommandContext` that fingerprints `{"workflow": id, "expected_revision": n, "ops": ops}` with `command::fingerprint("PlanEdit", …)` and derives the id.

- [ ] **Step 5: Run** the gate. **Step 6: Commit** with code map and README lines (`src/storage/sqlite/workflow.rs` — "writing plan versions"; `workflow_read.rs` — "reading plan versions"; `src/command/derive.rs` — "command ids Shadows derives when a caller names none"; `src/mcp/` — "Shadows' MCP server"; `src/mcp/grant.rs` — "who may do what on /mcp"): `feat(storage): plan versions, edits, approval (§13.2–§13.5, §13.15)`.

---

### Task B4: Plan routes and errors

**Files:**
- Create: `src/protocol/workflow.rs`
- Modify: `src/error.rs` (five codes), `src/protocol/failure.rs` (mapping), `src/protocol/mod.rs` (routes), `docs/superpowers/specs/2026-09-21-errors-and-testing-design.md` only if a name differs from §3.4 (it must not)
- Test: `tests/plan_routes.rs`

**Interfaces:**
- Consumes: B3's `Storage::{get_plan, list_plans, approve_plan}`, `StorageError` variants.
- Produces routes (utoipa-documented like `protocol/project.rs`):
  - `GET /api/projects/{id}/workflows` → `Vec<PlanListing>`
  - `GET /api/workflows/{id}` → `Plan`
  - `POST /api/workflows/{id}/approve` body `ApprovePlan { command_id: String, expected_revision: i64 }` → `Approved` (the client refetches the plan)
- Error mapping: `RevisionConflict` → 409 `REVISION_CONFLICT`, message `"the plan changed; current revision is {current}: {summary}"`, and the body carries `current_revision` (extend `ErrorBody` with an optional `current_revision: Option<i64>` field, skipped when `None`); `WorkflowFrozen` → 409 `WORKFLOW_FROZEN_IMMUTABLE`; `PlanInvalid(p)` → 422 `WORKFLOW_VALIDATION_FAILED` with the problems joined by `"; "` in `message` and listed in an optional `problems: Option<Vec<String>>` field; `GrantInvalid`/`GrantScope` never reach HTTP routes (map to 500 with a `tracing::error!` if they do).

- [ ] **Step 1: Failing tests** in `tests/plan_routes.rs` using `test_app`, `post`, `get_json` and a plan seeded through `app.storage` (B3 fixtures):
  - `the_project_lists_each_threads_latest_version`
  - `a_plan_read_lists_its_blockers_and_its_last_edit`
  - `approving_a_stale_revision_is_refused_with_the_current_one` — 409, `code == "REVISION_CONFLICT"`, `current_revision == 1`.
  - `approving_with_blockers_is_422_with_the_list`
  - `a_replayed_approval_answers_the_first_result` — same `command_id` twice → both 200, one `WorkflowFrozen` event.
  - `approving_a_frozen_version_again_with_a_new_command_is_409_frozen`
- [ ] **Step 2: Run**, expect failures. **Step 3: Implement.** **Step 4: Gate**, regenerate `api/openapi.json` and `web/src/api/schema.d.ts`. **Step 5: Commit** `feat(protocol): plan routes and errors (§13.10)`.

---

### Task B5: Grants and Planner instructions

**Files:**
- Modify: `src/mcp/grant.rs` (kinds, token, hash)
- Create: `src/storage/sqlite/grant.rs`, `src/storage/sqlite/instructions.rs`, `src/protocol/grants.rs`, `src/protocol/instructions.rs`
- Modify: `src/protocol/mod.rs` (routes), `src/cli/mod.rs` (startup revokes internal grants)
- Test: `tests/grants.rs`, `tests/planner_instructions.rs`

**Interfaces:**
- Produces:

```rust
// src/mcp/grant.rs
pub enum GrantKind { Thread, Project }                         // "thread" / "project"
pub struct Grant { pub id: GrantId, pub kind: GrantKind, pub project_id: ProjectId,
    pub thread_id: Option<ThreadId>, pub created_at: String, pub revoked_at: Option<String> }
pub struct Token(String);                                      // never Debug-printed in full
impl Token { pub fn generate() -> Self; pub fn as_str(&self) -> &str; pub fn hash(&self) -> String }
pub fn hash_token(raw: &str) -> String                          // sha256 hex
pub struct IssuedGrant { pub grant: Grant, pub token: Option<Token> }
// Token::generate: "shd_" + two v4 UUIDs without hyphens (244 random bits).

// src/storage/sqlite/grant.rs
impl Storage {
    /// `token` is `Some` on the first issue and `None` on a replay: the token is shown once.
    pub async fn issue_project_grant(&self, ctx: &CommandContext, project: &ProjectId) -> Result<IssuedGrant, StorageError>;
    pub async fn issue_thread_grant(&self, thread: &ThreadId) -> Result<(Grant, Token), StorageError>;
    pub async fn revoke_grant(&self, ctx: &CommandContext, id: &GrantId) -> Result<Grant, StorageError>;
    pub async fn revoke_thread_grant(&self, id: &GrantId) -> Result<(), StorageError>;   // internal, no command
    pub async fn revoke_all_thread_grants(&self) -> Result<u64, StorageError>;           // startup
    pub async fn grant_for_token(&self, raw: &str) -> Result<Option<Grant>, StorageError>; // live only
    pub async fn list_project_grants(&self, project: &ProjectId) -> Result<Vec<Grant>, StorageError>;
    pub async fn prepare_draft(&self, grant: &GrantId) -> Result<String, StorageError>;   // draft_ref, expires in 1 h
    /// The plan a draft_ref already started (bound refs never expire), or None
    /// if unused and not expired; Err if unused and expired, unknown, or issued
    /// to another grant. Expiry stops only a first use (§13.5).
    pub async fn draft_intent(&self, grant: &GrantId, draft_ref: &str) -> Result<Option<WorkflowId>, StorageError>;
}
// src/storage/sqlite/instructions.rs
pub struct InstructionsVersion { pub id: String, pub number: i64, pub body: String, pub created_at: String }
impl Storage {
    pub async fn save_planner_instructions(&self, ctx: &CommandContext, project: &ProjectId, body: &str) -> Result<InstructionsVersion, StorageError>;
    pub async fn current_planner_instructions(&self, project: &ProjectId) -> Result<Option<InstructionsVersion>, StorageError>;
}
```

B3 already binds a `draft_ref` when a draft starts; this task only issues and reads refs.

Routes:
- `GET /api/projects/{id}/planner-instructions` → `{ number, body, created_at } | null`
- `PUT /api/projects/{id}/planner-instructions` body `{ command_id, body }` → the new version
- `GET /api/projects/{id}/mcp-grants` → project grants, newest first, never a token
- `POST /api/projects/{id}/mcp-grants` body `{ command_id }` → `{ grant, token, command }` where `command` is `claude mcp add --transport http shadows http://<bind>/mcp --header "Authorization: Bearer <token>"`. A replayed `command_id` answers the grant **without** the token (`token: null`) — the token is shown once.
- `DELETE /api/mcp-grants/{id}?command_id=<id>` → the revoked grant. The router has no DELETE route yet; this is its first. The idempotency key rides in the query because a DELETE has no body.

Events: `McpGrantIssued {grant_id, kind}`, `McpGrantRevoked {grant_id}`, `PlannerInstructionsSaved {number}` — none carries a token or the instructions body.

- [ ] **Step 1: Failing tests:**
  - `tests/grants.rs`: `a_token_is_stored_only_as_its_hash` (no row contains the raw token); `a_revoked_grant_no_longer_resolves`; `a_replayed_issue_does_not_show_the_token_again`; `grant_events_never_carry_the_token` (search every `payload_json`); `startup_revokes_every_thread_grant_and_keeps_project_grants`; `a_draft_ref_expires_and_belongs_to_its_grant`; `a_thread_grant_cannot_name_a_thread_of_another_project` (the composite FK refuses the insert).
  - `tests/planner_instructions.rs`: `each_save_is_a_new_numbered_version`; `current_instructions_are_the_highest_number`; `the_saved_event_does_not_carry_the_body`.
- [ ] **Step 2–4:** run failing, implement, gate (regenerate openapi + schema).
- [ ] **Step 5: Commit** `feat(mcp): grants and Planner instructions (§13.7, §13.8)`.

---

### Task B6: The MCP server

**Files:**
- Create: `src/mcp/auth.rs` (bearer → grant, 401), `src/mcp/server.rs` (the `rmcp` handler, tool list per grant kind), `src/mcp/tools.rs` (argument types and the mapping to storage), `tests/fixtures/listening.rs` (a daemon on a real loopback port for MCP clients)
- Modify: `Cargo.toml` (`rmcp`, test-support features), `src/protocol/mod.rs` (mount `/mcp` under the same guard), `src/planner/handles.rs` (`running_for`)
- Test: `tests/mcp_server.rs`

**Interfaces:**
- Consumes: B3, B5 storage; `LiveHandles`.
- Produces:

```rust
// src/mcp/mod.rs
pub fn service(state: McpState) -> axum::Router;      // mounted at /mcp by protocol::router
pub struct McpState { pub storage: Arc<Storage>, pub handles: Arc<LiveHandles>, pub bind: SocketAddr }
// B8 adds: pub ui: tokio::sync::broadcast::Sender<UiSignal>
// src/planner/handles.rs
impl LiveHandles { pub async fn running_for(&self, thread: &ThreadId) -> Option<OperationId> }
// tests/fixtures/listening.rs
pub struct Listening { pub app: App, pub base: String /* http://127.0.0.1:<port> */ }
pub async fn listening_app() -> Listening;
pub async fn mcp_client(base: &str, token: &str) -> rmcp::service::RunningService<rmcp::RoleClient, ()>;
```

Cargo:

```toml
rmcp = { version = "=3.4.1", default-features = false, features = ["server", "macros", "transport-streamable-http-server"] }
schemars = "1"   # the major rmcp 3.4.1's `server` feature uses; tool input schemas are derived, never hand-written
reqwest = { version = "0.12", optional = true, default-features = false, features = ["json", "rustls-tls"] }

[features]
test-support = ["rmcp/client", "rmcp/transport-streamable-http-client-reqwest", "dep:reqwest"]
```

(Match `reqwest`'s version to the one `rmcp` 3.4.1 depends on — `cargo tree -p rmcp -i reqwest` — so one copy is built.)

Behaviour (§13.6):
- `auth.rs` is an axum middleware on the `/mcp` router: `Authorization: Bearer <t>` → `storage.grant_for_token(t)`; none or `None` → **401** with an empty body; otherwise insert the `Grant` into the request extensions. The existing `refuse_foreign_pages` guard runs first (403 `ORIGIN_REFUSED` for a foreign `Origin`).
- `server.rs`: the handler reads the `Grant` from the request extensions `rmcp` passes to the tool context (`RequestContext` → `extensions` → `http::request::Parts` → `extensions`); confirm the exact path in `rmcp` 3.4.1's `transport::streamable_http_server` docs before writing it, and state in the task report which it was. `list_tools` returns only the grant kind's tools (table in §13.6; `plan_show` arrives in B8). A call to a tool outside the list answers the JSON-RPC "tool not found" error `rmcp` produces for an unknown tool.
- Stateless: `StreamableHttpServerConfig { legacy_session_mode: false, json_response: true, .. }` (Task 0: Claude Code 2.1.281 opens with `server/discover` at `2026-07-28`, which `rmcp` always serves statelessly, and falls back to `initialize` at `2025-11-25` for servers without it; `legacy_session_mode: false` keeps the fallback stateless too). Do not enable the `transport-streamable-http-server-session` feature. `rmcp`'s own `allowed_hosts` default is loopback-only, which matches the daemon's bind; leave its `allowed_origins` empty — the daemon's `refuse_foreign_pages` guard owns Origin.
- Tools are declared with `rmcp`'s `#[tool_router]`/`#[tool]` macros and `Parameters<T>`; argument types derive `schemars::JsonSchema`, and so do the workflow content types they embed (`PlanOp`, `TaskContent`, `Link`, `LinkKind`, `AcceptanceItem` — add the derive in `src/workflow/` in this task; `domain stays pure` concerns persistence imports only).
- `tools.rs`:
  - `workflow_get { workflow_id? }` — thread grant: its thread's latest version (`thread_plan`), `workflow_id` ignored if given and different → `GRANT_SCOPE`; project grant: required, must be in the grant's project → else `GRANT_SCOPE`.
  - `task_get { workflow_id?, number }`.
  - `workflow_list {}` — project grant only.
  - `draft_prepare {}` — project grant only → `{ draft_ref }`.
  - `draft_start { title?, goal?, from_workflow_id?, draft_ref? }` — thread grant: `start_draft` with the running turn's derived id (`Anchor::Operation(running_for(thread))`; no running turn → `isError` "no turn is running for this conversation"); project grant: `draft_ref` required and passed to storage in both paths; with `from_workflow_id` → `start_draft(…, None, Some(ref))` on that plan's thread; without → `start_thread_with_draft(…, Some(ref))`; id `Anchor::DraftRef(ref)`. `plan_edit` answers the stored `EditOutcome`.
  - `plan_edit { workflow_id?, expected_revision, ops: Vec<PlanOp>, command_id? }` → `{ workflow_id, revision, summary }`.
  - Results are JSON text content; errors are `CallToolResult::error` with text `"<CODE>: <message>"`, codes from `StorageError`: `RevisionConflict` → `REVISION_CONFLICT: the plan changed; current revision is N: <summary>. Read it with workflow_get, then rebuild your edit.`; `WorkflowFrozen` → `WORKFLOW_FROZEN_IMMUTABLE: …Start a new version with draft_start.`; `PlanInvalid` → `WORKFLOW_VALIDATION_FAILED: <problems joined by "; ">`; `GrantInvalid` → `GRANT_INVALID`; scope → `GRANT_SCOPE`; `CommandConflict` → `COMMAND_CONFLICT`.

- [ ] **Step 1: Failing tests** in `tests/mcp_server.rs` (all through `listening_app()` and `mcp_client`, except the raw-HTTP ones through `reqwest`):
  - `a_request_without_a_token_is_401` and `a_revoked_token_is_401`
  - `a_foreign_origin_is_403`
  - `the_tool_list_depends_on_the_grant_kind` — thread grant: `workflow_get, task_get, draft_start, plan_edit`; project grant: `workflow_list, workflow_get, task_get, draft_prepare, draft_start, plan_edit`. Neither lists an approval tool.
  - `a_thread_grant_edits_only_its_own_threads_plan` — `workflow_id` of another thread → `GRANT_SCOPE`.
  - `a_project_grant_cannot_reach_another_project` → `GRANT_SCOPE`.
  - `parallel_edits_on_one_revision_conflict_and_the_loser_can_retry` — two `plan_edit` calls joined with `tokio::join!` on revision 0 with different ops: exactly one `REVISION_CONFLICT`; retrying it with `expected_revision: 1` succeeds; the plan has both changes. (Review Focus 1.)
  - `a_repeated_edit_returns_the_first_result` — same call twice → same revision, one event.
  - `editing_a_frozen_plan_is_refused`
  - `a_draft_ref_replays_for_a_new_version_of_a_frozen_plan` — freeze a plan; `draft_prepare` → `draft_start{ref, from_workflow_id}` → v2; edit v2; the same `draft_start` again answers v2's id.
  - `a_bound_draft_ref_still_replays_after_its_hour` — for both paths (from scratch, and from a frozen plan): bind a ref, set its `expires_at` in the past directly in the database, repeat the same `draft_start` → the same plan, nothing new created; an unused ref past its hour → `GRANT_SCOPE`.
  - `draft_ref_replays_and_distinct_refs_make_distinct_plans` — `draft_prepare` → `draft_start{ref, "Payments"}` twice → one plan; a second ref with the same title → a second plan and thread; the first ref with a different goal → `COMMAND_CONFLICT`.
  - `arabic_plan_round_trips_through_mcp` — `plan_edit` with Arabic title/goal/label/acceptance, then `workflow_get` returns them byte-identical. (Review Focus 3.)
- [ ] **Step 2–4:** failing run, implement, gate.
- [ ] **Step 5: Commit** `feat(mcp): the /mcp server, tools and grant checks (§13.5–§13.6)`.

---

### Task B7: The Planner's session opens with Shadows

**Files:**
- Create: `src/planner/prompt.txt`, `src/planner/setup.rs` (what a session opens with; the grant each adapter holds)
- Modify: `src/agent/acp.rs` (`SessionSetup`, `start_session` signature, `reapply`), `src/planner/sessions.rs` (call sites only), `src/planner/spawn.rs` (instructions check before a turn), `src/storage/sqlite/turn.rs` + `NewTurn` (two invocation columns), `src/cli/mod.rs` (MCP URL, startup revocation from B5 wired), `src/bin/fake_acp.rs` (record setup in `report`, an `mcp` prompt)
- Test: `tests/planner_mcp.rs`

**Interfaces:**
- Consumes: B5 grants and instructions; B6 `/mcp`.
- Produces:

```rust
// src/agent/acp.rs
pub struct SessionSetup {
    pub mcp: Option<McpServerSpec>,      // Shadows' server
    pub append: Option<String>,          // appended to Claude Code's prompt
    pub allowed_tools: Vec<String>,
}
pub struct McpServerSpec { pub name: String, pub url: String, pub bearer: String }
impl Connection {
    pub async fn start_session(&self, cwd: &Path, how: SessionStart, setup: &SessionSetup) -> Result<Opened, AcpError>;
    /// session/resume on a live session with new settings (§13.8).
    pub async fn reapply(&self, session: &str, cwd: &Path, setup: &SessionSetup) -> Result<Opened, AcpError>;
}
// src/planner/setup.rs
pub(crate) struct Setups { /* per-thread stamp: grant id + instructions version id */ }
impl Setups {
    pub(crate) fn new(storage: Arc<Storage>, mcp_url: Option<String>) -> Self;
    /// Issues the thread's grant (revoking any previous one) and builds the setup.
    pub(crate) async fn for_opening(&self, thread: &ThreadId, project: &ProjectId) -> Result<SessionSetup, String>;
    /// Before a turn: Some(setup) with the same grant when the project's instructions changed.
    pub(crate) async fn changed_before_turn(&self, thread: &ThreadId, project: &ProjectId) -> Result<Option<SessionSetup>, String>;
    /// The adapter closed: revoke its grant.
    pub(crate) async fn forget(&self, thread: &ThreadId);
    pub(crate) fn stamp(&self, thread: &ThreadId) -> Option<(GrantId, Option<String>)>;
}
pub fn prompt_version() -> &'static str;   // sha256 hex (first 16) of prompt.txt, computed once
// src/planner/sessions.rs gains one field and one accessor:
//   setups: Setups,   pub(crate) fn setups(&self) -> &Setups
// SessionsConfig gains: pub mcp_url: Option<String>   (None in tests that need no MCP)
// NewTurn gains: pub prompt_version: Option<&'a str>, pub instructions_version: Option<&'a str>
```

The ACP request fields (Task 0 confirmed them; §13.8 table): `mcp_servers` = one HTTP server `{ name: "shadows", url, headers: [Authorization: Bearer …] }`; `_meta.systemPrompt = { "append": prompt.txt + "\n\n## Project instructions\n\n" + body }` (the heading only when the project has instructions); `_meta.claudeCode.options.allowedTools = ["mcp__shadows__*"]`. Use the `agent-client-protocol` 2.2.0 builders for `mcp_servers` and `meta`; if a builder is missing, construct the request from `serde_json` into the typed request.

`src/planner/prompt.txt` says, in plain English, the six points of §13.8, including: "You never approve a plan and never call Shadows' HTTP API; when asked to approve, ask the person to press Approve." Keep it under 60 lines.

Wiring (keep `sessions.rs` under 500 lines — only calls):
- `open_live`: `let setup = self.setups.for_opening(thread, &context.project_id).await.map_err(OpenError::Start)?;` before `start_session`; on any setup failure after the grant was issued, `self.setups.forget(thread).await`.
- every path that removes a `Live` entry (idle close, `terminate`, `terminate_adapter`, `close_all`, dead-adapter replacement): `self.setups.forget(thread).await`.
- `spawn.rs`, after the session is leased and before `prepare_turn`: `if let Some(setup) = sessions.setups().changed_before_turn(..)? { opened.connection().reapply(..).await }` — a failure fails the turn at `Prepare` with the adapter's words.
- `start_turn` records `prompt_version()` and the current instructions version id.
- `fake_acp`: store the received `mcp_servers`, `_meta.systemPrompt.append` and `allowedTools` per session and include them (`"mcp"`, `"append"`, `"allowed"`) in the JSON its existing `report` prompt echoes — the bearer only as `"bearer_hash"` (sha256 hex, the same function as `mcp::grant::hash_token`), never the token, because a reply is stored as an `AgentMessage` — plus `"blocks"`: the text of every content block of the current prompt; a prompt `mcp <tool> <json args>` calls that tool on the session's MCP server with its bearer (using `rmcp`'s client, `test-support` only) and replies with the tool result's text.

- [ ] **Step 1: Failing tests** in `tests/planner_mcp.rs` (on `listening_app()`, whose `SessionsConfig.mcp_url` points at itself):
  - `a_session_opens_with_shadows_mcp_instructions_and_the_pre_approval` — `report` shows the URL, a `bearer_hash` equal to the `token_hash` of this thread's live grant in `mcp_grant`, an `append` containing prompt.txt's first line, and `allowed == ["mcp__shadows__*"]`.
  - `a_turn_edits_the_plan_through_mcp` — `mcp draft_start {"title":"Login","goal":"g"}` then `mcp plan_edit {...}` → the plan exists with the task; the edit event's actor is `("Thread", thread)`.
  - `reopening_after_idle_issues_a_new_grant_and_revokes_the_old` — shorten `idle_after`; after the close, the grant whose `token_hash` is the first `bearer_hash` has `revoked_at` set; the next turn's `report` shows a different `bearer_hash` whose grant is live, and `mcp workflow_get {}` through the fake succeeds. (401 for a revoked token is B6's test.) (Review Focus 5.)
  - `restart_revokes_every_internal_grant` — shut the app down and start another on the same database: every `kind = 'thread'` grant has `revoked_at` set; project grants do not. (Review Focus 5.)
  - `changed_instructions_apply_at_the_next_turn_not_during_one` — start a `wait-for-release` turn (existing: it ends when a file named `release` appears in the project directory), save new instructions via the route while it runs, then release it; a following `report` turn shows the new `append` and the same `bearer_hash`, and the adapter received exactly one `session/resume` (count them in `report`'s JSON as `"resumes"`), sent after the first turn ended.
  - `an_invocation_records_the_prompt_and_instructions_versions`
- [ ] **Step 2–4:** failing run, implement, gate.
- [ ] **Step 5: Commit** `feat(planner): sessions open with Shadows' MCP server and instructions (§13.7–§13.8)`.

---

### Task B8: The plan in the conversation

**Files:**
- Modify: `src/protocol/conversation.rs` (`StartTurn.focus`, `StartTurn.client_tab`, fingerprint), `src/storage/sqlite/turn.rs` (focus refs on the `UserMessage`), `src/agent/acp.rs` (`prompt` with a context block), `src/planner/handles.rs` (`LiveTurn.client_tab`), `src/mcp/tools.rs` + `src/mcp/server.rs` (`plan_show`), `src/protocol/mod.rs` (`AppState.ui`), `src/protocol/sse.rs` (`plan-show` frame)
- Create: `src/protocol/ui_signal.rs` (the live-only signal type)
- Test: `tests/plan_in_conversation.rs`

**Interfaces:**
- Produces:

```rust
// StartTurn gains (both optional):
pub struct Focus { pub workflow_id: WorkflowId, pub task_id: TaskId, pub revision: i64 }
focus: Option<Focus>, client_tab: Option<String>
// The fingerprint covers thread id, prompt, model, mode, effort and focus (§12.7 amended); not client_tab.

// src/agent/acp.rs
pub async fn prompt(&self, session: &str, text: &str, context: Option<&str>) -> Result<TurnEnd, AcpError>
// context, when present, is a second text content block after the person's text.

// src/protocol/ui_signal.rs
#[derive(Clone, serde::Serialize)]
pub struct UiSignal { pub thread_id: ThreadId, pub target_tab: Option<String>,
    pub workflow_id: WorkflowId, pub version: i64, pub task_number: Option<u32>,
    pub place: Place }
#[serde(rename_all = "snake_case")] pub enum Place { Inline, Side, Page }
// AppState gains: pub ui: broadcast::Sender<UiSignal>
```

Behaviour (§13.9):
- Turn start with `focus`: in the same transaction as the turn, check the task belongs to that workflow version (else 422 `INVALID_COMMAND` "the chosen task is not in that plan"); the `UserMessage` entry's refs are `[Workflow(w), Task(t)]`. The prompt's context block: `"[Shadows] The person is pointing at task T{n} (\"{title}\") of plan {workflow_id}, revision {revision}. Read the plan with workflow_get before changing it."` — read the task's number and title at turn start.
- `client_tab` is stored in `LiveTurn` only (memory).
- `plan_show { workflow_id?, task_number?, place }` (thread grant only; added to the thread grant's tool list): needs a running turn (else `isError`); derived id `Anchor::Operation(op)`; writes a `PlanView` entry (body `"Plan v{version}"` or `"T{n} · {title}"`, refs `[Workflow, Task?]`, `operation_id` = the turn) and a `PlanShown` event; then sends one `UiSignal` with `target_tab` = the turn's `client_tab`. A replayed call writes nothing and sends nothing.
- SSE: while live, a `UiSignal` for the thread becomes a `plan-show` frame with the signal as JSON. It is never written to, or replayed from, the journal. Workflow events already reach the stream as `durable` frames.

- [ ] **Step 1: Failing tests** in `tests/plan_in_conversation.rs`:
  - `a_focused_turn_stores_the_task_with_the_message_and_tells_the_planner` — a focused `report` turn's `blocks` hold the person's text and then the context block; the `UserMessage` refs hold `Workflow` and `Task`.
  - `the_same_command_with_another_focus_is_a_command_conflict` — same `command_id` and prompt, T3 then T4 → 409 `COMMAND_CONFLICT`.
  - `a_focus_on_a_task_of_another_plan_is_refused`
  - `plan_show_writes_a_card_and_signals_only_the_sending_tab` — subscribe twice; the turn carries `client_tab: "tab-a"`; `mcp plan_show {"place":"page"}` (sent as a focused turn with `client_tab: "tab-a"`) → both streams get the `durable` event for `PlanShown`, and each gets one `plan-show` frame whose `target_tab` is `"tab-a"` (the web client acts only on its own).
  - `a_replayed_subscription_gets_the_card_but_no_plan_show_frame` — subscribe with `after: 0` after the turn: the `PlanView` entry's event arrives as `durable`; no `plan-show` frame.
  - `plan_show_twice_in_one_turn_for_different_tasks_writes_two_cards`
  - `an_external_grant_has_no_plan_show`
- [ ] **Step 2–4:** failing run, implement, gate (regenerate openapi + schema).
- [ ] **Step 5: Commit** `feat: show a plan in the conversation and point at a task (§13.9)`.

---

### Task W1: The Workflows section and the plan graph

**Files:**
- Modify: `web/package.json` (`@xyflow/react`, `@dagrejs/dagre`), `web/src/router.tsx` (route `/projects/$projectId/workflows/$workflowId`), `web/src/app/sidebar/*` (Workflows section), `web/src/api/client.ts`, `web/src/api/queries.ts`, `web/src/test/fake-daemon.ts`
- Create: `web/src/app/workflows/plan-page.tsx`, `web/src/app/workflows/plan-graph.tsx` (the shared graph component), `web/src/app/workflows/layout.ts` (dagre), `web/src/app/workflows/task-node.tsx`, `web/src/app/workflows/inspect-panel.tsx`, `web/src/app/workflows/approve-bar.tsx`, `web/src/app/workflows/use-plan.ts`
- Test: `web/src/app/workflows/plan-page.test.tsx`, `web/src/app/workflows/layout.test.ts`

**Interfaces:**
- Consumes: `GET /api/projects/{id}/workflows`, `GET /api/workflows/{id}`, `POST /api/workflows/{id}/approve`, the thread stream's `durable` frames of kind `WorkflowEdited`, `WorkflowFrozen`, `WorkflowDraftStarted`.
- Produces:

```ts
// plan-graph.tsx — used by the page (W1), cards and the side panel (W2)
export function PlanGraph(props: {
  plan: Plan                       // schema type
  compact?: boolean                // a card: no minimap, fixed height
  focusTask?: number               // centre and outline this task
  onSelectTask?: (task: PlanTask) => void
}): JSX.Element
// layout.ts
export function layoutPlan(plan: Plan): { nodes: Node[]; edges: Edge[] }   // dagre, rankdir LR
// use-plan.ts
export function usePlan(workflowId: string): UseQueryResult<Plan>          // refetches on the thread's Workflow* durable frames
```

Page behaviour (§13.11): header with title, `Draft vN`/`Approved vN`, conversation link, previous/next, zoom/fit (React Flow `Controls`), Approve on a draft only with `blockers` listed above it; Approve sends `{ command_id, expected_revision: plan.revision }` (new command id per press, reused on a network retry, as `api/command-id.ts` does) and, on success, refetches the plan (the answer is an `Approved`, not the plan); `409 REVISION_CONFLICT` → refetch and show "The plan changed while you were looking; review it again"; `422` → show the problems. Start node: title + goal. Task node: `T{n}`, title, two lines of goal, writes, deps, "N part(s) wait for T8" from `completes_after` links; every text element has `dir="auto"`. Edges: `needs` solid, `completes_after` dashed, label = link label. Tasks in `last_edit.changed_tasks` get the accent colour and "changed". Frozen: read-only banner "Approved vN · editing creates draft vN+1". Sidebar lists `list_plans` polled every 10 s while visible.

- [ ] **Step 1: Failing tests:**
  - `layout.test.ts`: `lays out left to right with each task right of what it needs`; `lays out 60 tasks without overlapping nodes` (boxes pairwise disjoint).
  - `plan-page.test.tsx` (fake daemon): `shows Draft v2 with its blockers above Approve`; `approve sends the revision it showed`; `a revision conflict on Approve refetches and says the plan changed` (Review Focus 4); `an approved plan has no Approve and shows the banner`; `changed tasks are marked`; `a completes_after edge is dashed and labelled`; `an Arabic task title renders with dir auto` (Review Focus 3); `a Workflow durable frame refetches the plan`.
- [ ] **Step 2–4:** failing run, implement, web gate.
- [ ] **Step 5: Commit** `feat(web): the Workflows section and plan graph (§13.11)`.

---

### Task W2: The plan in the conversation

**Files:**
- Create: `web/src/app/conversation/plan-card.tsx`, `web/src/app/conversation/plan-side-panel.tsx`, `web/src/app/conversation/focus-chip.tsx`, `web/src/stream/tab-id.ts`
- Modify: `web/src/app/conversation/messages.tsx` (`PlanView`, `PlanApproved` entries; `mcp__shadows__*` tool lines as sentences), `web/src/app/conversation/composer.tsx` + `turn-settings.ts` (focus, client_tab), `web/src/stream/frames.ts` + `thread-stream.ts` (`plan-show` frame), `web/src/app/conversation/conversation.tsx` (side panel slot)
- Test: `web/src/app/conversation/plan-in-conversation.test.tsx`

**Interfaces:**
- Consumes: W1's `PlanGraph`, `usePlan`; B8's `StartTurn.focus`/`client_tab`, `plan-show` frame, `PlanView` entries.
- Produces: `tabId(): string` — one random id per page load (`crypto.randomUUID()`), sent as `client_tab` on every turn.

Behaviour (§13.9, §13.11): a `PlanView` entry renders `PlanGraph compact` of its referenced version (live via `usePlan`), headed "Plan vN · revision R", with **Open plan**; clicking a task sets the focus chip "T4 · title ×"; the next Send includes `focus: { workflow_id, task_id, revision }` and clears the chip. A `plan-show` frame whose `target_tab === tabId()` and `place === "side"` opens the side panel on that version; `"page"` navigates to the plan page; any other tab, and any replayed entry, only shows the card. Tool lines titled `mcp__shadows__plan_edit` read "Plan edited", `…draft_start` "Plan started", `…plan_show` nothing extra (its card is the entry), `…workflow_get` "Read the plan". `PlanApproved` renders as a quiet system line.

- [ ] **Step 1: Failing tests:** `a PlanView entry renders the graph of its version`; `clicking a task adds the focus chip and the next send carries it`; `a plan-show frame for this tab and page navigates`; `a plan-show frame for another tab only shows the card` (acceptance step 8); `a replayed PlanView entry does not open the side panel`; `plan tool lines read as sentences`.
- [ ] **Step 2–4:** failing run, implement, web gate.
- [ ] **Step 5: Commit** `feat(web): plans in the conversation and pointing at a task (§13.9)`.

---

### Task W3: Project settings — Planner instructions and external agents

**Files:**
- Create: `web/src/app/project-settings/project-settings.tsx`, `web/src/app/project-settings/instructions-editor.tsx`, `web/src/app/project-settings/external-agents.tsx`
- Modify: `web/src/router.tsx` (`/projects/$projectId/settings`), sidebar (Project settings link), `web/src/api/client.ts`, `web/src/test/fake-daemon.ts`
- Test: `web/src/app/project-settings/project-settings.test.tsx`

Behaviour (§13.7, §13.8, §13.11): instructions textarea with Save (`PUT` with a command id) and "Last changed …"; External agents: **Connect** → shows the returned `command` once in a code block with **Copy** and the notice "Claude Code stores this token in plain text in ~/.claude.json. Revoking it here stops Shadows accepting it; it does not remove it from Claude's settings."; the list polls every 10 s; **Revoke** per grant. A replayed Connect that answers `token: null` shows "This connection was already created; its command is no longer shown. Revoke it and connect again if you need it."

- [ ] **Step 1: Failing tests:** `saving instructions sends the body with a command id`; `connect shows the command once with the notice`; `a replayed connect without a token explains why`; `revoke removes the grant from the list`.
- [ ] **Step 2–4:** failing run, implement, web gate.
- [ ] **Step 5: Commit** `feat(web): project settings for Planner instructions and external agents (§13.7–§13.8)`.

---

### Task I: Whole-branch review, run with Mohammed, merge

- [ ] **Step 1: Whole-branch review** — one opus reviewer over `main..milestone-2/plan-workflow` against §13, which fixes what it finds, runs the gate, commits and reports (CLAUDE.md "A reviewer fixes what it finds"). The controller verifies the report (commits exist, diff matches, test counts real) and does not re-review.
- [ ] **Step 2: Build and start** the release daemon and the web client (`.claude/launch.json` configurations `daemon` and `web`) with Mohammed on Windows.
- [ ] **Step 3: Mohammed's run** — the nine acceptance steps of §13.14, in order. Record each as it happens, with the log lines and API reads that show it.
- [ ] **Step 4: Evidence** — `docs/evidence/milestone2/WINDOWS_RUN.md` in `milestone1/WINDOWS_RUN.md`'s format, including what Claude Code showed after Revoke (§13.7). Update `docs/status.md` (where the project is; next). Commit.
- [ ] **Step 5: PR** — push the branch and open the PR only when Mohammed says so; after merge, delete the branch (CLAUDE.md "Branches are short-lived").
