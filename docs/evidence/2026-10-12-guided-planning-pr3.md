# Guided planning PR 3 local acceptance — 2026-10-12

Branch: `next/guided-planning-pr3`. Implementation baseline `75f90ca`;
Tasks 1–6 commits `30f5919` through `a3c4133`; browser refresh fix `1266b23`.
This is a measurement record, not design authority. Final checks remain in
progress until the final section below is completed.

## Implemented slice

First whole-map proposal; later one-part add/change cards; person-only HTTP
approval; highlighted first-map waiver consent; invocation standards pins;
item/part revision staleness; ownership and first-map coverage gates;
atomic map snapshots and command replay; per-part drift; Project map editing
and read-only contract/starter-file inspection. Nothing writes the skeletons
to the project directory in this slice.

## Real browser journey

Independent daemon `127.0.0.1:4326`, Vite `127.0.0.1:5196`, profile `pr3`.
The existing user daemon PID 13972 was not stopped or rebuilt in place.
Online SQLite backup of the source development DB was stored in this plan's
ignored acceptance workspace. Source migration was 12; acceptance copy
migrated to 23. The trial used a new synthetic project:
`cec54209-2f1d-4e06-85ea-ceb838d68cc1`, conversation
`a1a16dab-f387-42c6-a679-83cd954defde`.

Actual Planner: Claude Code 2.1.292 through ACP adapter 0.81.1; UI offered
Opus 5.5 with Medium effort. No MiniMax call or deployment was performed.

1. Five vision fields were entered through the browser. A real SSE/save
   refresh race produced `CancelledError`; its regression failed, then
   passed after sharing active refreshes instead of cancelling them.
   Subsequent browser field saves succeeded without that error.
2. Real Planner proposed backend/API with a frontend waiver, deliberately
   missing database. Browser approval returned
   `STAGE_BLOCKED: missing part: database`; no map was written.
3. Planner proposed the whole corrected map with backend/database/API.
   Previous card became Superseded. Clicking **Approve map and waivers**
   changed the new card to Approved and stage to structure.
4. Project map showed three stable-id parts, G1 links, folders and starter
   files. Backend contract opened in the read-only side panel.
5. Adding an independent goal G2 produced `G2 is served by no part` drift.
   The approved map remained present.
6. Planner proposed a Backend design change serving G1 only. It remained
   pending despite unserved G2. Editing G1 by hand to include ISBN made the
   card Outdated and removed its Approve button.

Observed DB: three parts, one map version, frontend waiver with reason
`This is a terminal tool.` and approver `local`; foreign-key check returned
zero faults. Server logs showed actual requests and completed Planner turns.
Three real Planner turns completed. DB proposal states were one Approved
first map, one Superseded first map, and one Pending change computed Outdated.
Seven vision versions were stored. After rebuilding the final profile and
restarting on the same copy, all three parts, their per-part drift, the
unserved G2 and the outdated card survived; there were zero Approve map
buttons. Trial processes were then stopped: neither 4326 nor 5196 had a
listener; user daemon PID 13972 was still running. Trial tab was closed.

Screenshots (default narrow in-app viewport, not resized for presentation):
[refusal](./2026-10-12-guided-planning-pr3/refused.png),
[contract](./2026-10-12-guided-planning-pr3/contract.png),
[map drift](./2026-10-12-guided-planning-pr3/map-drift.png),
[outdated](./2026-10-12-guided-planning-pr3/outdated.png).
Browser AX/DOM observations establish the statuses; screenshots do not
show every item in the scrollable conversation at once.

## Review and verification

Targeted core: parts/proposals 19/19, vision 19/19, migration 4/4,
contracts 1/1; existing outcomes and agreement suites passed. HTTP routes
7/7, OpenAPI 4/4; MCP transport 6/6, workspace MCP 3/3, Planner 19/19.
Web map/SSE 9/9, card 5/5, refresh regression suite 6/6.

First full Web gate: 250/250 tests, types, lint and production build passed.
After browser refresh fix: 251/251 across 48 files, types/lint/build passed.
Vite retained its existing large syntax-highlighting chunk warning.

First Rust full gate stopped at a preexisting terminal-turn test whose
global journal cursor advanced because shutdown appends events
asynchronously. The assertion now measures only VisionProposed events and
proposal rows; targeted test passed. Strict Clippy modes passed before
review fixes. No failed full run is counted as acceptance.
The review gate then reached the storage schema inventory and failed because
that test's allowed table list omitted the three new 0023 map tables. Root
updated the explicit inventory (without changing migration behavior) and
reran the storage contract and whole workspace. Final results follow below.

Same-model fresh-context review found file/folder prefix collisions,
non-normalized proposal retry identities, and hidden missing mandatory
kinds after standards changes. Each has a failing regression and a fix;
the review's final gate and commit are pending recording below.
The user then explicitly stopped the reviewer to avoid concurrent work.
The reviewer was interrupted; root retained its fixes and took over final
verification/commit. The already-running read-only Cargo gate continued.
This is an interim independent review with root completion, not a claimed
completed reviewer dispatch.
Review fixes' targeted checks passed: core parts 21/21, contracts 1/1,
stage rendering 1/1, map UI 7/7. The final Web gate after those fixes passed
252/252 tests across 48 files, types, lint and production build.

## Limits and rulings

- Current checkout reused, per user request; no new worktree.
  Tradeoff: no second checkout isolates unrelated future edits.
- Metadata-only edits preserve served-item review baselines. Content review
  or an approved Planner change refreshes the reviewed part only.
  If that proves too implicit, a separate explicit review action is needed.
- Historical removed links and rule ids on unwritten content do not block
  unrelated repairs; global identity/path ownership checks still apply.
  Tradeoff: obsolete references remain visible until their part is repaired.
- A standards-added mandatory gap permits no-worsening incremental repairs;
  the gap stays visible and future structure writing must recheck all kinds.
  Tightening approval later needs a migration/repair path for existing maps.
- Existing pre-PR3 parts survive migration but require explicit adoption.
  First-map approval refuses silently replacing them; §23.5 owns the OPEN.
  Cost: an existing parts workspace cannot adopt its first map through this
  slice until the explicit identity-preserving adoption path is implemented.
- Test fixtures use the test-support-only Storage seed with a Design wrapper;
  it has no production caller.
  Cost: one test-only convenience method rather than a fake Planner session
  in every old part test.
- No structure-on-disk acceptance, plan gates, map-history UI, §24 authority
  enforcement, production or Linux runtime acceptance is claimed.
- A transient maximum-update-depth browser error appeared while Vite HMR
  replaced source during active streaming; stable post-reload acceptance
  worked. No confirmed branch-specific streaming defect was established.

## Final gate and cleanup

Final gate at `474a5df`, run by Claude after Codex was stopped:
`cargo fmt --all --check`, both Clippy runs, the `test-support` feature
check and `git diff --exit-code api/` passed. `cargo test --workspace
--no-fail-fast` with `RUST_LOG` unset: 481 passed, 0 failed, 1 ignored.
With an inherited `RUST_LOG` the debug-log test fails; that is the
environment, not the code. Web: 251 of 252 passed in the full run; the one
failure (`project-settings` connect notice) passed 8/8 alone, so it is
timing under load. Typecheck, lint and build passed.

The person's own daemon (port 4318) was rebuilt from this branch and
restarted on a fresh database. The old one could not start: it had applied a
pre-merge draft of migration 0020 during PR 0 development. It and two older
trial databases were moved to `shadows-dev-backup-2026-10-12`. On that
daemon, with the real Planner (Claude Code, Opus 5.5 Medium), project
"Book Catalogue":

1. Before a map, Project map shows "The first map comes from the Planner"
   and no add-part button.
2. A first map leaving G2 unserved: approval answered
   `STAGE_BLOCKED · unserved goals: G2` under the card; nothing written.
3. A corrected map without `frontend`, waived with a reason: the waiver sat
   in its own highlighted block, the button read "Approve map and waivers",
   approval passed, the first card showed Superseded, stage `structure`.
4. A hand edit of the Backend part's kind to `service` was refused with
   `missing part: backend`.
5. Two change cards: Backend serving G1, API serving G2. A hand edit of G1
   made only the Backend card Outdated; the API card stayed valid and was
   approved. Adding G3 by hand showed drift: `Backend: G1 changed since the
   part was reviewed` and `G3 is served by no part`; the page counted one
   pending map proposal, not two.

The `Maximum update depth exceeded` error from `thread-stream.ts` was not an
HMR artefact. It came during live turns that call a tool, not on page load
and not in a long tool-free reply. It predates this PR: `use-conversation.ts`
mirrored each streamed delta into the reply reducer from an effect. The reply
now follows the streamed text during render. After the fix, two tool-calling
turns on the same conversation logged no error; before it, one in two did.
`use-conversation.test.tsx` streams 120 deltas and pins the reply text, but
it passed before the fix too: under happy-dom the error did not reproduce, so
the browser runs are the evidence. Web after the fix: 253/253, typecheck, lint
and build passed.
