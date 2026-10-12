# §23 PR 5 — Roadmap Proposals and Plan Gates Implementation Plan

> **For agentic workers:** execute task by task. Steps use checkbox (`- [ ]`)
> syntax. Commit at the end of each task.

**Goal:** After Create structure, the Planner proposes the roadmap (outcomes
and the parts each references) as a card; the person approves it with the
button, or the Planner does at their request. No plan is started, edited or
approved before a roadmap is approved, and no plan version is approved unless
its plan is linked to at least one part. The stage goes
`roadmap → plans`.

**Spec (read first, it is the authority):**
`docs/superpowers/specs/2026-10-08-guided-planning-design.md` §23.4 (stage),
§23.5 ("Roadmap proposals and plans: decided … for PR 5", and the map
decisions it mirrors), §23.6 (gate table), §23.10 test 5.
Authority: `docs/superpowers/specs/2026-10-08-approval-authority-design.md`
§24.2 (one Planner approval tool per thing it proposes).
Data model: `docs/superpowers/specs/2026-10-03-project-planning-workspace-design.md` §18.3.

**Read before code:** `CLAUDE.md`, `docs/codebase/README.md`,
`crates/shadows-core/src/design/contract.yaml`,
`crates/shadows-core/src/plans/contract.yaml`. Use the LSP / `rg` for
signatures; open only the files named.

**The template to mirror is the map proposal**, end to end:
`design/map_proposals.rs`, `design/store/map_proposals.rs`,
`design/store/map_proposal_read.rs`, `design/store/map_proposal_shape.rs`,
`design/store/map_approval.rs`, migration `0023_project_map.sql`,
`shadows-mcp/src/workspace_tools.rs` (`map_propose`, `map_approve`),
`shadows-http/src/design.rs` (`map_proposals`, `approve_map_proposal`),
`web/src/app/conversation/map-proposal-card.tsx`, and their tests
(`crates/shadows-core/tests/design_parts/`, `crates/shadows/tests/workspace_mcp/`,
`crates/shadows/tests/design_routes.rs`). Copy its structure, not its
part-specific rules.

## Global Constraints

- Linux container. Build in the repo's own `target/` (gitignored). No
  `CARGO_TARGET_DIR` override.
- No Rust line over 100 columns; `cargo fmt --all` before each commit.
- Never edit a shipped migration (`0001`–`0025`). New schema is
  `crates/shadows-core/migrations/0026_roadmap.sql`.
- No new Rust integration-test *files* at the top level of `tests/`: add
  modules under existing directories (`crates/shadows-core/tests/design_parts/`,
  `crates/shadows/tests/workspace_mcp/`) or tests to existing files.
- A change to a service updates its `contract.yaml` in the same commit
  (methods, obligations, agreements, tests). A new module gets its owner row
  in `docs/codebase/README.md` (the `codemap` test enforces it; one job, no
  "and").
- A file over 300 lines: the commit message says its one job; at 500 split by
  responsibility. `design/store/vision.rs` (327) and `map_rules.rs` (315) are
  already over 300: put new code in new files, not there.
- `cd web && npm run gen:api` whenever `api/openapi.json` changes; commit both.
- Targeted tests while working (`cargo test -p <crate> --test <name>`); the
  full gate once, in Task 6.
- Commits end with
  `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Do not push.
- Refusals for gates are `StorageError::StageBlocked(reason)` → `STAGE_BLOCKED`
  (HTTP 409; MCP text `STAGE_BLOCKED: …`). Reuse the existing mapping.

## Rulings already made (do not reopen)

1. **Stage.** Add `Stage::Plans` (serde `plans`). `project_stage` answers
   `roadmap` when the structure write is completed and
   `design_workspace.roadmap_revision = 0`, `plans` when it is `> 0`. Stage
   line for `Plans`: `"[Shadows] Stage: plans. The roadmap is approved; plan
   work for its outcomes, each plan tied to the parts it changes."`. Update
   the `Roadmap` line to: `"[Shadows] Stage: roadmap. The structure is
   written; propose the roadmap with roadmap_propose."`
2. **Schema (0026).** `design_workspace.roadmap_revision INTEGER NOT NULL
   DEFAULT 0 CHECK (>= 0)`; table `design_roadmap_proposal` shaped like
   `design_map_proposal` with `action IN ('first','add','change','remove')`,
   `outcome_id TEXT NULL`, `payload_json`, `roadmap_revision INTEGER NOT NULL`,
   `expected_outcome_revision INTEGER NULL`, `expected_parts_json TEXT NOT NULL
   DEFAULT '{}'` (part id → part revision at proposal time), `status IN
   ('Pending','Approved','Superseded')`, `approved_principal`,
   `approved_agent`, `approved_request_entry` columns as the map proposal
   gained in `0024_approval_agent.sql`, `created_at`, `seq` unique per project.
   New thread entry kind `RoadmapProposal` (`threads/model.rs`).
3. **Proposal shape** (`design/roadmap_proposals.rs`):

   ```rust
   pub enum RoadmapAction { First, Add, Change, Remove }
   pub struct ProposedOutcome {
       pub key: String,                    // slug, unique in the proposal
       pub parent_key: Option<String>,     // another key in this proposal
       pub parent: Option<OutcomeId>,      // an existing outcome (add/change)
       pub content: OutcomeContent,        // title, intended_result, acceptance
       pub parts: Vec<PartId>,             // non-empty; parts of this project
   }
   pub struct RoadmapPropose {
       pub action: RoadmapAction,
       pub outcomes: Vec<ProposedOutcome>, // first: ≥1; add/change: exactly 1; remove: 0
       pub outcome_id: Option<OutcomeId>,  // change/remove: required
       pub replaces: Option<String>,
   }
   pub struct RoadmapProposal { approved_by, id, thread_id, action, outcomes,
       outcome_id, old: Option<OutcomeContent>, status, outdated, created_at }
   ```

   Refused (`INVALID_COMMAND` for shape, `STAGE_BLOCKED` for stage):
   structure not written (`structure not written: press Create structure
   first`); `first` when `roadmap_revision > 0` (`a roadmap is approved;
   propose a change`); non-first when it is 0; an outcome with no part, an
   unknown part, a blank title, a duplicate key, a parent cycle; change/remove
   of an unknown outcome.
4. **Supersession and staleness** (spec §23.5). A newer `first` supersedes a
   pending `first`; a newer change/remove of the same outcome supersedes the
   pending one; `replaces` supersedes the one it names. On approval, refuse as
   outdated (card `outdated: true`, `STAGE_BLOCKED: proposal outdated; propose
   again`) when: any referenced part's revision differs from
   `expected_parts_json` or the part is gone; for `first`,
   `roadmap_revision > 0`; for change/remove, the outcome's revision differs
   from `expected_outcome_revision` or it is gone. Nothing else outdates it
   (vision and standards changes do not).
5. **Approval** writes in one transaction: the outcomes (`design_outcome`
   rows, with ordinals in proposal order and parents resolved from keys),
   their `design_outcome_part` rows, (for remove: deletes the outcome only if
   it has no children and no plan links, else `STAGE_BLOCKED: outcome has
   children or plans`), bumps `roadmap_revision`, marks the proposal
   Approved with §24 provenance via `crate::db::approval::authorise_in` (copy
   `map_approval.rs`), rewrites the card JSON, appends `RoadmapApproved` and
   the usual `DesignChanged` signal the map approval emits, records the
   command. Planner approval: `approve_roadmap_proposal_as_planner`; person:
   `approve_roadmap_proposal`. No waiver concept here.
6. **Hand outcome edits** (`DesignOp::Outcome*`, and `PlanLink*` with an
   `Outcome` anchor) through the workspace edit are refused while
   `roadmap_revision = 0` with `STAGE_BLOCKED: the first roadmap comes from
   the Planner: ask it to propose one` — same place and pattern as the map's
   `touches_parts` check in `design/store/vision.rs`, but put the new check in
   a small function in a new file (`design/store/roadmap_gate.rs`) and call it
   from there. Part-anchored plan links stay allowed once a map exists (that
   is how the person ties a plan to a part).
7. **Plan gates** (spec §23.6). Design declares, in
   `design/store/roadmap_gate.rs`:
   - `pub(crate) async fn require_roadmap_in(conn, project) -> Result<(), StorageError>`:
     `STAGE_BLOCKED: no roadmap approved: propose the roadmap first` when
     `roadmap_revision = 0` (or the workspace row is missing).
   - `pub(crate) async fn plan_has_part_in(conn, project, plan) -> Result<bool, StorageError>`.
   - `pub(crate) async fn link_plan_parts_in(conn, project, plan, parts: &[PartId])`:
     checks each part is in the project (`StorageError::Constraint` otherwise),
     inserts `design_part_plan` rows `ON CONFLICT DO NOTHING`, emits nothing
     itself (the plan write's event covers it).
   Plans calls `require_roadmap_in` inside the transaction of
   `start_draft`, `edit_plan`, `approve_plan_with` and the binding edit
   (`plans/binding_edit.rs` → its store write), after the writer/authority
   check and **after** the replay `classify` (a recorded replay still
   answers). `approve_plan_with` then refuses with `STAGE_BLOCKED: plan has
   no part: link it to a part on the Project map page, or start it with
   parts` when `plan_has_part_in` is false. Archive, unarchive, reads and
   `plan_show` are not gated. Declare the cross-service calls in both
   contracts (`agreements`/`shared_in_transaction`, as `check_design_plan`
   is declared).
8. **`draft_start` carries parts.** `DraftStart` gains
   `#[serde(default)] parts: Vec<PartId>`, part of the fingerprint only when
   non-empty (so existing replays keep their fingerprint). `start_draft`
   calls `link_plan_parts_in` for the (new or continued) plan inside its
   transaction. The MCP description says: "parts: the part ids this plan
   changes; a plan cannot be approved until it is tied to at least one part."
   Prompt (`harness/prompt.txt`): name `roadmap_propose`/`roadmap_approve`,
   that plans come after the roadmap, and that `draft_start` takes `parts`.
9. **Tools.** `roadmap_propose` and `roadmap_approve` join `THREAD_TOOLS`
   (19) in `shadows-mcp/src/server.rs` and the Planner's fixed tool list
   wherever `map_propose` is listed (`harness/contract.yaml`, harness
   settings/tests — find every place with `rg map_propose`). External
   (project-grant) agents do not get them.
10. **HTTP.** `GET /api/projects/{id}/design/roadmap-proposals` and
    `POST /api/projects/{id}/design/roadmap-proposals/{proposal}/approve`
    (body `{command_id}`), mirroring the map proposal routes' paths and
    shapes exactly (check the real map route paths and copy their pattern).
11. **Test fixtures.** Existing tests that start/edit/approve plans must keep
    passing by reaching the `plans` stage, not by weakening the gate. Add a
    `test-support` seed next to `seed_map_for_tests`:
    `Storage::seed_roadmap_for_tests(project)` /
    `Design::seed_roadmap_for_tests` — seeds the map if absent (reuse
    `seed_map_for_tests`), sets `roadmap_revision = 1`, and returns the id
    of one seeded part. Plus `Storage::link_plan_part_for_tests(project,
    plan, part)`. Call them from the shared fixtures
    (`crates/shadows/tests/fixtures/app.rs` project setup,
    `crates/shadows/tests/fixtures/plan.rs`,
    `crates/shadows-core/tests/fixtures/cross_plan.rs` — raw SQL fixture: add
    the equivalent rows in its SQL). Only tests that exist to prove the gate
    run without the seed. Do **not** seed in production code paths. Note:
    the structure-write gate is not part of the plan gate (the roadmap
    proposal already requires it), so the seed need not fake a structure
    write.
12. **Web.** `RoadmapProposalCard` in `web/src/app/conversation/` mirroring
    `map-proposal-card.tsx`: outcome list (title, intended result,
    acceptance items, referenced part titles), old content beside a change,
    Approve button, Superseded/Outdated/Approved states with the §24
    "Approved by you" / "Approved by the Planner at your request" line exactly
    as the map card does. Wire it in `entry.tsx` for the `RoadmapProposal`
    kind, the API in `web/src/api/design.ts`/`queries.ts`. The stage header
    shows `plans`. A plan approval refused with `STAGE_BLOCKED` shows its
    reason (check the existing approve error display; add only if missing).

## Review Focus

1. A roadmap proposal before the structure write is refused; after it, a
   first roadmap is proposed, approved by the button, and the stage is
   `plans`.
2. `roadmap_approve` by the Planner records `planner:<harness>` with the
   request entry; without a running turn it is refused as the map's is.
3. Staleness: editing a referenced part (hand edit) outdates the pending
   first roadmap; a vision edit does not.
4. Every plan write path — MCP `draft_start`, `plan_edit`, `plan_approve`,
   HTTP approve, binding edit, external project-grant agents — is refused
   with `STAGE_BLOCKED` before the roadmap; a replay of a command recorded
   before still answers its recorded result.
5. A plan with no part link cannot be approved; `draft_start` with `parts`
   links it and approval then succeeds.
6. Hand outcome edits before the first roadmap are refused; after it they
   work.

---

### Task 1: Stage, schema and the plan gate (core)

**Files:** migration `0026_roadmap.sql`; `design/stage.rs`,
`design/store/stage.rs`; new `design/store/roadmap_gate.rs`; `plans/store/draft.rs`,
`plans/store/edit.rs`, the binding edit store; `plans/model.rs` (`DraftStart.parts`),
`plans/mod.rs`; `design/store/vision.rs` (one call into `roadmap_gate`);
`design/store/map_versions.rs` or a new test-support file for the seeds;
contracts of `design` and `plans`; `docs/codebase/README.md`.

- [ ] Write failing tests first (in `crates/shadows-core/tests/design_parts/`
      new module `roadmap_gate.rs`, registered in `design_parts.rs`):
      start/edit/approve refused before the roadmap; approve refused with no
      part; `parts` on start links and approval passes; hand outcome op
      refused before roadmap; replay still answers.
- [ ] Implement Rulings 1, 2 (schema only), 6, 7, 8, 11.
- [ ] Fix the existing tests through the fixtures (Ruling 11). Run
      `cargo test -p shadows-core` and `cargo test -p shadows` for the plan,
      design and MCP test binaries you touched.
- [ ] Commit.

### Task 2: Roadmap proposals (core)

**Files:** new `design/roadmap_proposals.rs` (values + service entry points),
new `design/store/roadmap_proposals.rs` (propose),
`design/store/roadmap_proposal_read.rs` (load/list/outdated),
`design/store/roadmap_approval.rs` (approve); `threads/model.rs`
(`RoadmapProposal` kind); `events` contract for `RoadmapProposed` /
`RoadmapApproved`; `design/contract.yaml`; README owner rows.

- [ ] Failing tests first in `crates/shadows-core/tests/design_parts/roadmap.rs`:
      Rulings 3, 4, 5 and Review Focus 1–3.
- [ ] Implement. Keep each new file under 300 lines with one job.
- [ ] Commit.

### Task 3: Adapters (MCP, HTTP, prompt, tool lock)

**Files:** `shadows-mcp/src/workspace_tools.rs`, `server.rs`;
`shadows-http/src/design.rs`, `lib.rs` (routes, OpenAPI);
`harness/prompt.txt`, `harness/contract.yaml`, harness tool list;
tests in `crates/shadows/tests/workspace_mcp/` (new module `roadmap.rs`),
`design_routes.rs`, `mcp_tools.rs`, `mcp_server.rs`, `openapi.rs`,
`planner_mcp.rs` as the tool counts require.

- [ ] Rulings 8 (MCP description), 9, 10. Regenerate `api/openapi.json` the
      way the repo does (see `crates/shadows/tests/openapi.rs`), then
      `cd web && npm run gen:api`.
- [ ] Tests: a Planner proposes and approves a roadmap over MCP; an external
      agent has no `roadmap_*` tool; HTTP list + approve; a plan tool before
      the roadmap answers text starting `STAGE_BLOCKED:`.
- [ ] Commit.

### Task 4: Web

**Files:** `web/src/app/conversation/roadmap-proposal-card.tsx` (+ `.test.tsx`),
`entry.tsx`, `web/src/api/design.ts`, `queries.ts`; stage header component
(find where `structure`/`roadmap` stage labels render).

- [ ] Ruling 12. Tests mirror `map-proposal-card.test.tsx`.
- [ ] `cd web && npm test -- --run`, `npm run typecheck`, `npm run lint`,
      `npm run build` (check `web/package.json` for the exact script names).
- [ ] Commit.

### Task 5: Docs

- [ ] Every touched service's `contract.yaml` is complete (the `contracts`
      test passes); `docs/codebase/README.md` rows; the `codemap` test passes.
- [ ] Commit.

### Task 6: Full gate

From the repo root, in order, all must pass:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
3. `cargo test --workspace`
4. `cargo clippy --workspace -- -D warnings`
5. `cargo tree -e features,no-dev --workspace | grep test-support` prints nothing
6. `git diff --exit-code api/` after regenerating (committed API is current)
7. Web: tests, typecheck, lint, build.
8. No Rust line over 100 columns in changed files.

- [ ] Record the commands and counts (Rust passed/ignored, Web passed) in
      your report. Commit any fix.

## Report

List commits, the gate counts, every ruling you could not follow and why,
and anything you found that the spec does not decide (leave it, report it).
