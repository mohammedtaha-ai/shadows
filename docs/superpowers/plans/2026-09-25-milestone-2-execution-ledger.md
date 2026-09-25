# Milestone 2 — execution ledger and handoff

> Working file for executing `2026-09-25-milestone-2-plan-workflow.md`. **Delete it
> from the branch before the PR merges** (as Milestone 1's ledger was). It is here, and not
> in the git-ignored `.superpowers/sdd/`, so a session on another machine (cloud) can pick up.

## Handoff — where it stands (2026-09-25)

Branch `milestone-2/plan-workflow`, not pushed. Everything up to W2's implementation is
committed; the tree is clean.

| Task | State |
|---|---|
| Task 0 (probes) | done — `docs/evidence/milestone2/MCP_PROBE.md`; step 3 failed → §13.8 amended |
| B1–B8 (backend) | done and reviewed — 302 Rust tests |
| F1, F2 (pre-existing flakes) | done and reviewed — both were real product races |
| W1 (plan page + graph) | done and reviewed (one fix round) — 116 web tests |
| W2 (plan in the conversation) | done and reviewed (opus, cloud) — 125 web tests |
| W3 (project settings) | not started |
| Task I | not started: whole-branch review, then Mohammed's run on Windows (§13.14's nine steps), evidence, status, PR only when Mohammed says |

### Next steps, in order
1. **Review W2**: range `21e74b9..9476bb4`. Brief = plan's "Task W2"; implementer report
   summary below. Open questions for the reviewer (give a view on each): tool lines have no
   "· N changes" (needs a backend change — defer?); `turn-settings.ts` unchanged; new
   `tool-text.ts`; `messages.tsx` at 302 lines; one stream per thread; the legend may cover
   nodes on a zoomed compact card (fix if small).
2. **W3**: plan's "Task W3". Grants list includes revoked ones (show as revoked, no Revoke
   button). DELETE revokes project grants only.
3. **Final whole-branch review**: `git merge-base main HEAD`..HEAD against §13, pointed at the
   deferred minors below.
4. **Stop for Mohammed**: the Windows run (release build + `.claude/launch.json` `daemon`/`web`)
   and the PR are his.

### How this run works (Mohammed's rules)
- Implementers: fresh **opus** subagent per task; never a worktree; stage files by name;
  never push; commits end with the Co-Authored-By line.
- Reviewer: **Codex `gpt-6-sol`**, which reviews, fixes, runs the gate and commits
  (`codex exec -m gpt-6-sol -s danger-full-access -C <repo> -o <last.md> - < prompt`; the
  codex-companion plugin forces a sandbox that fails on Mohammed's Windows). **If Codex is not
  available (cloud, or its limit), an opus reviewer does the same job**; back to Codex when it
  returns.
- The controller verifies each review (commit exists, diff matches, test counts) and reads
  Codex's fix; a wrong Codex fix goes back to an implementer (W1 needed this).
- Before a ruling, get Codex's view when available; record both.
- Amend spec/plan in place when the code proves them wrong; prefer a library over hand code.
- Gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`;
  web `npm --prefix web run typecheck|lint|test`; regenerate code map / openapi / schema when
  signatures or routes move.
- On Mohammed's Windows machine E: fills up: builds use
  `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/Temp/claude/E--Globalprojects-shadows/e86a610a-4c5b-4114-884d-faf050ce6a96/scratchpad/target`
  and `CARGO_INCREMENTAL=0`. Irrelevant elsewhere.

## Ledger (copied from `.superpowers/sdd/…/progress.md`)

# SDD ledger — plan: docs/superpowers/plans/2026-09-25-milestone-2-plan-workflow.md

Spec: docs/superpowers/specs/2026-09-25-planner-workflow-design.md (§13).
Branch: milestone-2/plan-workflow (renamed from milestone-2/plan-spec after 72ec33b). Branch start: 72ec33b. main = 24a2ea5.

## Setup rulings

- Ruling: no worktree; work happens on milestone-2/plan-workflow in the main folder — Mohammed's standing rule overrides the skill's worktree setup — cost if wrong: none.
- Ruling: implementers run on opus — Mohammed's instruction and project memory override the skill's cheap-tier selection — cost if wrong: token spend only.
- Ruling: the task reviewer is Codex gpt-6-sol (Mohammed: "codex sol 6"), run write-capable through `codex-companion.mjs task --write --model gpt-6-sol`. It fixes what it finds, runs the gate, commits and writes its report to a file. The controller verifies that report (commits exist, the diff says what it says, the gate and test counts are real). There is no implementer fix loop unless Codex leaves a finding that contradicts the plan or spec — Mohammed's instruction plus CLAUDE.md "A reviewer fixes what it finds" — cost if wrong: a Codex fix is itself unreviewed, and controller verification is the only net.
- Ruling: Task I's whole-branch review also goes to Codex gpt-6-sol, not to the opus reviewer the plan names — Mohammed named Codex as the reviewer — cost if wrong: the reviewer for Task I gets swapped back.

## Pre-flight scan

### Task pairs that share a file or an interface

| Pair | Produces → consumes | Found |
|---|---|---|
| B1 → B2 | `WorkflowId`, `TaskId` → `EntryRef::{Workflow,Task}` | agrees |
| B1 → B3 | `PlanContent`, `PlanOp`, `apply`, `edit_problems`, `approval_problems`, `Problem` → storage | agrees |
| B2 → B3 | `ThreadEntryKind::PlanApproved`, `EntryRef::Workflow` → the approval entry | agrees |
| B3 → B4 | `get_plan`, `list_plans`, `approve_plan -> Approved`, `StorageError` variants → routes | agrees (fixed in the round-2 plan review) |
| B3 ↔ B5 | B3 creates `src/mcp/grant.rs` holding only `GrantId`, and binds `draft_intent`; B5 fills in grant.rs and issues and reads refs | B3's tests need `mcp_grant` and `draft_intent` rows before B5's issuing API exists → R1 |
| B3/B5 → B6 | storage, grants → tools | B6 says both "plan_edit → {workflow_id, revision, summary}" and "plan_edit answers the stored EditOutcome" → R2 |
| B5 ↔ B7 | both edit `src/cli/mod.rs` for startup revocation | overlap → R3 |
| B5 → B6 | the grant route's `command` needs the bind address; B6 `McpState.bind` | the implementer finds where the bind address lives in AppState; no conflict |
| B6 ↔ B8 | B6's test `the_tool_list_depends_on_the_grant_kind` lists the thread tools without `plan_show`; B8 adds `plan_show` to that list | B8 must update B6's test → R4 |
| B6 ↔ B7 | `test-support` gains the rmcp client; `fake_acp` (required-features test-support) uses it | agrees (fake_acp is already test-support-only) |
| B7 ↔ B8 | both edit `src/agent/acp.rs` and `src/storage/sqlite/turn.rs` | sequential, so no conflict |
| B8 → W2 | `plan-show` frame, `StartTurn.focus`/`client_tab`, `PlanView` → cards, chip | agrees |
| W1 → W2 | `PlanGraph`, `usePlan` | agrees |
| B4/B5 → W1/W3 | routes → client | agrees after the openapi and schema regeneration |

### Each task against its own text

| Task | Found |
|---|---|
| 0 | controller-run; no code enters the repo |
| B1 | Traced the DFS by hand for `a_cycle_across_both_kinds_is_refused` → [2,4] ✓ and `a_task_after_a_cycle_is_not_named_in_it` → [1,2] ✓. The test `removing_a_linked_task_is_refused_until_its_links_go` expects "T1 is still linked from T2", a message the rules list never names → R5 |
| B2 | the tests name `Actor::system()`/`Actor::user("local")`, and `NewThreadEntry` fields may differ; the plan says to use the existing fixture names — agrees |
| B3 | the migration's composite FK needs `planning_thread_id_project`, which is created first ✓; the test list agrees with the interface |
| B4 | agrees |
| B5 | the composite-FK test inserts by SQL (stated) ✓ |
| B6 | R2 |
| B7 | agrees |
| B8 | R4 |
| W1–W3 | agree |

### Pre-flight rulings

- R1 Ruling: B3's tests seed `mcp_grant` and `draft_intent` rows with direct SQL inserts in a `tests/fixtures/plan.rs` helper, because B5's `prepare_draft`/issue API does not exist yet — the plan orders B3 before B5, and binding belongs to the draft start — cost if wrong: B5 may replace the helper with its API.
- R2 Ruling: `plan_edit` answers the whole stored `EditOutcome` as JSON (`workflow_id, version, revision, summary, changed_tasks`), which is a superset of the three fields — a replay must answer the fixed outcome (§13.5) — cost if wrong: two extra fields in a tool result.
- R3 Ruling: B5 wires `revoke_all_thread_grants` at daemon startup in `src/cli/mod.rs`; B7 only adds the MCP URL to `SessionsConfig` and relies on B5's wiring — B5's test `startup_revokes_every_thread_grant_and_keeps_project_grants` needs the wiring — cost if wrong: none.
- R4 Ruling: B8 updates B6's `the_tool_list_depends_on_the_grant_kind` so the thread grant lists `plan_show` — §13.6's table gives the thread grant `plan_show` — cost if wrong: none.
- R5 Ruling: when a batch leaves a link naming a task that `task_remove` removed in the same batch, `apply` reports "T{removed} is still linked from T{other}" (the removed task is the link's `after`) or "T{removed} is still linked to T{other}" (the removed task is the link's `task`), not the generic missing-task message, which stays for tasks that never existed — the test is the only statement of the message — cost if wrong: one message string.

## Progress

Task 0: probes ran (3 runs). Steps 1, 2, 4 and 5 pass. Step 3 FAILS: Claude Code stores the appended system prompt in the session transcript and restores it on every resume, warm or cold, so a changed `append` is ignored. The conversation is kept, and on a cold resume the new bearer in `mcpServers` IS used. → §13.8 amended: changed instructions reach the next turn as a context block; `reapply` is removed from B7.
- Ruling: B7's context block also carries prompt.txt when the thread's latest invocation recorded a different (or NULL, i.e. pre-M2) prompt_version — the same fact (append frozen at session creation) makes pre-M2 threads resume without Shadows' instructions — cost if wrong: a longer first turn on old threads.
- Ruling: `prompt(session, text, context: &[String])` moves from B8 to B7 (B7 is its first user); B8 adds the focus block to the list — cost if wrong: none.
Library check (Mohammed asked): petgraph 0.8 replaces B1's hand DFS; schemars 1 (rmcp server feature) derives B6 tool schemas; rmcp legacy_session_mode:false = stateless. No other hand-rolled piece found a library worth adding (bearer middleware = axum from_fn; tokens = uuid v4; hash = sha2, all present). Plan amended in 25eac7e.
Task B1: dispatched (opus, background), BASE 25eac7e.
Task 0: complete (evidence d742c44; probe code and probe sessions deleted)
Note: B1 review BASE is d742c44 (the Task 0 docs commit landed after B1 was dispatched).
Task B1: implementer DONE_WITH_CONCERNS, commit 5339d1e (218 tests: 216 pass, 2 fail in harness_observation, pre-existing).
- Ruling: `#[allow(dead_code)]` on `from_stored` in src/id.rs stays until B3 reads the ids; B3 removes it — cost if wrong: one lint suppression for two tasks.
- Ruling: the harness_observation / thread_routes flake fails on a clean 25eac7e too (usage-update race in the fake adapter), so it is not B1's. It gets fixed as a separate small task (F1) before B2, so every later gate is green — cost if wrong: one extra dispatch.
Task B1: Codex review attempt 1 failed before reading anything (companion forced workspace-write; Windows sandbox CryptUnprotectData). run-review.sh now calls codex exec -s danger-full-access, as Mohammed's Codex config does.
Task B1: complete (commits 5339d1e..c64d76e, Codex review fixed 2 Important: T0 in stored content, duplicate links). F1 BASE c64d76e.
Task F1: implementer DONE (6e31ede): real product race — offers.rs relay task between connection and turn could hold the last update when the prompt answered; now a callback inside the connection's dispatch loop. 15/20 and 6/20 failures -> 0/30. E: was full; controller ran cargo clean (34.9 GiB of target/, regenerable).
Task F1: complete (commit 6e31ede, Codex review clean, 220/220). Task B2: dispatched, BASE 6e31ede.
Task B2: implementer DONE, commit 5b1d9cd (223 Rust, 95 web).
- Ruling: domain §4.2's "the migration checks the database holds no other kind" goes into B3's 0007 migration (a CHECK on a temp table fed by SELECT DISTINCT kind), not B2 — B2 adds no migration and B3 creates the next one; plan amended in the next docs commit — cost if wrong: none.
- Ruling: the web flake (clock-dependent `context-ring` "Resets in 4h18m", plus one run with 25 failures) is pre-existing; it becomes task F2 before W1 — cost if wrong: one dispatch.
Task B2: complete (commits 5b1d9cd..f22c766, Codex strengthened the legacy refs_json test). Task B3: dispatched, BASE f22c766.
Task B3: implementer DONE, 86d5d16 (243 Rust). Splits: workflow.rs / workflow_draft.rs / task.rs / grant.rs; tests plan_storage.rs + plan_grants.rs.
- Ruling: `#[expect(dead_code)]` on `pub mod grant;` stays until B5 adds GrantId::from_stored's caller; B5 removes it (expect fails loudly once it is used) — cost if wrong: none.
- Ruling: the in-transaction scope check (GrantScope for a write outside the grant's thread/project) stays — §13.5 step 1 says "valid and in scope" — cost if wrong: none.
- Ruling: `start_thread_with_draft` with Writer::Person and no ref stays callable but unused; its only caller is B6's external path, which always passes a ref — §13.6 makes from-scratch external-only — cost if wrong: an unused path until someone needs it.
Note: B3 review package range dd5d86c..86d5d16 includes B2's review commit f22c766 (controller used the wrong base; harmless).
Task B3: complete (commits 86d5d16..6819a7c, Codex fixed 1 Important: external draft creation without draft_ref). Task B4: dispatched, BASE 6819a7c.
Task B4: implementer DONE, 454a56f (251 Rust, 95 web after one known F2 flake). (First run hit the Claude session limit before writing; resumed.)
- Ruling: GrantInvalid/GrantScope reaching an HTTP route answer 500 with the existing internal code and a tracing::error! — no route writes with a grant, so this is unreachable; a GRANT_* code would advertise a path that does not exist — cost if wrong: a misleading 500 code on an impossible path.
Task B4: complete (commits 454a56f..bca5a2f, Codex: 2 Minor test gaps fixed). Task B5: dispatched, BASE bca5a2f.
Task B5: implementer DONE, fb95c9e (264 Rust, 95 web). AppState.mcp_url added (actual bound address) — B7 reuses it.
- Ruling: DELETE /api/mcp-grants/{id} revokes project grants only — §13.7: the web client does not manage thread grants — cost if wrong: none.
- Ruling: GET mcp-grants lists revoked grants too (with revoked_at); W3 shows them as revoked, without a Revoke button — cost if wrong: a longer list.
Task B5: complete (commit fb95c9e, Codex review clean, 264 Rust). F2 moved ahead of B6: the web flake reached 24/95 failures in one run. F2 BASE fb95c9e.
Task F2: implementer DONE, d61d7cf: first startApp now loads before tests (was inside a 5 s timeout under load), fixed clock in context-ring, product race in ProjectModes (stale modes after save), act() wraps. 0/20 idle, 0/30 under load.
Task F2: complete (commits d61d7cf..1325da8, Codex added a deterministic ProjectModes race test; 264 Rust, 96 web).
Task F2: minor (deferred): startApp does not unmount if app startup itself throws, so a startup failure still cascades through its file.
Task B6: dispatched, BASE 1325da8.
Mohammed (before sleeping): if a Codex fix is not right, get it corrected (fix dispatch); take Codex's view on rulings when available. Continue through W3 + final review; no push/PR/merge.
Task B6: implementer DONE_WITH_CONCERNS, 4aff7f2 (280 Rust). E: full again: controller ran cargo clean (34.1 GiB) and all builds now share CARGO_TARGET_DIR on C: (scratchpad/target) with CARGO_INCREMENTAL=0. rmcp feature correction committed d252f04. Codex asked for its view on 5 rulings (task-B6-questions.md).
Task B6: complete (commits 4aff7f2..b125336, Codex fixed 2 Important: project grant without/unknown workflow_id -> GRANT_SCOPE; external draft_start from a Draft source refused). Codex agreed with all 5 rulings. Controller read the fix: sound.
Task B6: minor (deferred): external draft_start from a Draft source is refused with GRANT_SCOPE, but it is a state error, not scope; INVALID_COMMAND would name it better (src/mcp/tools.rs ~287).
Task B7: dispatched, BASE b125336.
Task B7: implementer DONE_WITH_CONCERNS, ace118a (293 Rust). 5 departures (see report) sent to Codex for its view; concerns A (bearer in debug logs) and B (grant events late via second Storage) sent to Codex to investigate and fix.
Task B7: complete (commits ace118a..9f19786). Codex fixed 1 Important (bearer reached ACP debug logs: ACP crate capped at INFO in src/tracing.rs + test) and 2 Minor.
- Ruling (Codex agrees): latest_invocation_versions counts only started turns; a thread with no started turn compares with what its session opened with; context_before_turn(thread) without project; Setups::grant() dropped; prompt hash computed once at startup from compiled text. Spec §13.8 amended to match — cost if wrong: none.
- Ruling (Codex agrees): McpGrant* events written via Sessions' own Storage reach live SSE late; nothing consumes them live (grants poll every 10 s). Left.
Task B7: minor (deferred): a save between the route's version record and spawn's context read can repeat one version's block on the following turn (not lost) — src/protocol/conversation.rs:227, src/planner/setup.rs:122.
Controller read 9f19786's log cap: global filter_fn layer, ACP targets capped at INFO — sound. Task B8: dispatched, BASE 48cc617.
Task B8: implementer DONE, 48d66d5 (301 Rust, 96 web). Departures + open question (plan_show older version) sent to Codex.
Task B8: complete (commits 48d66d5..2109a8c). Codex fixed 1 Important: thread grant can read/show any version of its own thread (edits still latest only). Codex agreed with rulings 1-4.
- Ruling (Codex agrees): thread grants read/show any version of their own thread; §13.6 amended — cost if wrong: none.
Backend (B1-B8) complete: 302 Rust, 96 web.
Task W1: dispatched, BASE 0c07e48.
Task W1: implementer DONE_WITH_CONCERNS, fddc3f6 (109 web). Controller viewed the screenshot: layout, edges, labels and Arabic render as designed; text is small at fit. Sent to Codex: 4 rulings for its view, and A (readable fit) and B (panel overlap) to fix.
Task W1: Codex review 44c9e55 fixed 3 Important (readable fit, refit on Inspect, list error line); agreed with 4 rulings. Controller read the fix: minZoom={1} on ReactFlow locks zoom-out (fit button can't show a large plan — regression of the brief's zoom/fit), and key-remount on panel toggle loses pan/zoom. Per Mohammed ("if a Codex fix isn't right, fix it"): fix round 1 sent to the W1 implementer (readable initial fit via fitViewOptions.minZoom, low ReactFlow minZoom, fitView on toggle without remount). FIX_BASE 44c9e55.
Task W1: fix round 1/5 (2 addressed — zoom lock, remount; commits 44c9e55..7dd3951); Codex re-review ADDRESSED both, and added: fit floor for 60 linear tasks, wide plan opens at the start column (Codex agreed with the implementer's open point) — 21e74b9.
Task W1: complete (commits fddc3f6..21e74b9, 116 web).
- Ruling (Codex agrees): a wide plan's first view opens at the start column at readable zoom — cost if wrong: one camera setting.
Task W2: dispatched, BASE 21e74b9.
Task W2: implementer DONE_WITH_CONCERNS, 9476bb4 (125 web). Controller viewed the screenshot: card, Arabic focus chip and Open plan render. Rulings and the legend overlap sent to Codex.
Session moved to the cloud (no Codex there): the desktop WIP commit held only `.claude/launch.json` (Windows paths) — dropped from the branch, kept untracked. Branch pushed as `milestone-2/plan-workflow-ggu7l6`. Reviewer from here on: opus (the ledger's fallback).
Task W2: complete (commits 9476bb4..1dadd4a, opus review: legend moved under the compact card; messages.tsx split by job into messages.tsx + entry.tsx; 125 web, controller re-ran 125/125).
Task W2: minor (deferred): tool lines lack §13.11's "· N changes" and Open plan — tool entries store bare `[tool: title]` with no workflow ref or result; needs a backend change (src/planner/turn.rs:88, web/src/app/conversation/tool-text.ts). Spec/brief gap, not a W2 defect.
Task W2: minor (deferred): Fork disappears when a turn's last entry is the hidden plan_show tool line (tool lines batch after the card); the daemon forks only from the real last entry (src/storage/sqlite/mod.rs:66). Needs a ruling on fork points or on hiding that line.
Task W3: dispatched, BASE 1dadd4a.
