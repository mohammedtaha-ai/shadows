# Local delivery program: repository to verified task

- Date: 2026-11-09.
- Status: **design-review draft**; not dispatchable implementation tasks.
- Scope: Mohammed confirmed the complete local system in conversation.
- Owner: [section 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-local-delivery-revision-2026-11-09).
- Base inspected: `cbf2b597`, `fix/first-run`, initially clean.

## Approach

Use the structured-contract approach described in the owner spec. Detailed
prose is cheaper, but cannot support the requested deterministic references and
readiness checks on its own. The proposal and controller choices await written
design approval; scope confirmation does not imply that approval.

## L1: Repository discovery

Result: the person and Planner see existing files, paths, exclusions and scan
state, with semantic PHP/Laravel queries through a tested ready-made LSP server.

- Owners: Code for inventory and the LSP gateway; existing shadows-index stays
  parser-only; adapters; web API/project view.
- Build on: core-code scan/watch/status/scope, a maintained LSP client library
  and independently installed language servers. Do not require Claude to run.
- Work: paged inventory independent of supported languages; safe path traversal;
  visible excluded/stale/incomplete states; bounded PHP/Laravel LSP trial;
  select client/server after freshness/isolation/lifecycle and dependency review;
  capability/configuration/status; Planner access; repository display through
  current project APIs and events. Preserve labeled fallback results.
- Proof: inspect a Laravel tree before planning; unsupported files are listed;
  secrets are metadata-only; incomplete scans do not invent deletions; reject
  escapes; handle case folding and watcher loss; resolve PHP methods by path;
  distinguish ambiguous/stale LSP results and same-named symbols. Verify the
  server integration with its own installed executable, not just plugin metadata.
- Needs: none. No independent code-intelligence workspace/cache.

L1 includes the complete [13-entry upstream catalog](../../core/specs/2026-09-30-code-index-design.md#proposed-ready-made-server-catalog)
requested by Mohammed: clangd, C#, Go, Java, Kotlin, Liquid, Lua, PHP, Python,
Ruby, Rust, Swift and TypeScript/JavaScript. Implementation tasks cover catalog
registration, per-project language/root selection, executable/runtime detection,
on-demand installation guidance and per-backend handshake/query acceptance.
PHP/Laravel, Rust and TypeScript are the first end-to-end trial; the remaining
entries retain explicit untested/platform-unavailable states until measured.
Do not equate adding a descriptor with working support, or require all language
toolchains to be installed on every machine.

## L2: Planning-time preparation

Result: request framework setup from planning at the agreed empty destination,
then continue detailed planning against refreshed real files.

- Owners: Runs for preparation attempts/Git; Harness/Grants for worker policy;
  Threads/Turns for the session; Code for refresh; conversation/status UI.
- Work: exact request and confirmation identity; destination/lease validation;
  preparation lifecycle; output manifest/Git result; stop/restart/retry;
  inventory refresh before planning resumes.
- Proof: adopt existing Laravel without reinstalling; refuse nonempty targets;
  replay creates one attempt; interrupted installs remain inspectable; retry
  checks partial files; commit no unrelated file, runtime or installer.
- Needs: L1. The Planner orchestrates a bounded preparation worker; ordinary
  planning mode keeps its current shell/edit policy.

## L3: Task contracts and decisions

Result: inspect exact file operations, required symbols, behavior, inputs,
outputs/errors, accepted decisions and producer/consumer pins on a task.

- Owners: Plans; Design for decisions/shared agreements; adapters and task UI.
- Work: canonical typed content; validation/serialization; exact contract pins;
  dependency/phase graph integration; versioned decision provenance; MCP edits;
  API generation and contract editing/display.
- Proof: one producer/consumer contract; missing outputs and cycles refused;
  Frozen content unchanged; revision conflicts atomic; exact action/path
  scopes; existing shared agreement operations referenced without duplication.
- Needs: L1. Can be planned independently of L2 after L1 ships.

## L4: Execution packets and readiness

Result: Start explains missing inputs; inspect the exact captured context and
scope received by a particular attempt.

- Owners: Runs with public Plans/Design/Code dependencies; Harness/Grants;
  task-context/attempt APIs; packet/readiness UI.
- Work: canonical packet/digest; selected source observations; decision/contract
  pins; exact derived paths; read/write leases; bounded questions/usage facts;
  task-authorized semantic questions joining Code results with Plans' contract
  pins; capability checks; legacy version/adoption behavior.
- Proof: stale/missing/ambiguous inputs block dispatch; unrelated changes do
  not invalidate all tasks; conflicting writes/required reads cannot overlap;
  evidence stays attributable across retries; unknown usage stays unknown;
  LSP cannot disclose outside task reads, fabricate a planned function as
  existing, or silently replace a conflicting approved signature.
- Needs: L2 and L3. Same folder/cache; no OS shell-containment claim.

## L5: Independent verification and evidence

Result: inspect baseline/post checks linked to the candidate commit, including
failed, cancelled, timed-out and inconclusive results.

- Owners: Runs; shadows-process for children; adapters and evidence UI.
- Work: typed check definitions; input/output leases; same-folder comparison;
  redacted bounded output/artifact digests; diagnostics; restart reconciliation.
- Proof: new regression versus unchanged failure; equal exit codes with
  different failures are not equivalent; unexpected input drift invalidates
  attribution; restart/lost response cannot duplicate checks or manufacture PASS.
- Needs: L4. task_report remains the executor's report, not correctness proof.

## L6: Critic and acceptance

Result: read independent findings per acceptance item, required check evidence
and real progress against the exact implementation result.

- Owners: Runs; Harness/Grants; task-review MCP; task/plan progress UI.
- Work: immutable review input; fixed critic policy/model selection; no
  arbitrary critic edits/shell; per-item outcomes; evidence-backed produced
  contracts; pending status; PASS_WITH_NOTES person action.
- Proof: failed checks cannot become PASS by assertion; review dispatch
  deduplicates; no review recursion; manual marks labeled; no old review
  approving unseen changes; pending work not displayed fully verified.
- Needs: L5. No editable specialist/profile dashboard required.

## L7: Bounded repair and pending checks

Result: failures reach the actual Planner, retries are bounded, exact pending
dependencies trigger durable rechecks and plan changes stop affected work.

- Owners: Runs; Plans owner-thread/Draft transitions; Threads/Turns delivery;
  current Design/Grants authority; reports and review-gate UI.
- Work: failure/problem records; busy/missing owner behavior; retry,
  investigation and directed third attempt; pending rechecks; replan blocking;
  person gates; interruption/lost-session reconciliation.
- Proof: third failure stops; agent cannot authorize fourth attempt; busy owner
  waits durably; missing owner stops visibly; Draft blocks/stops plan work;
  cross-plan pins invalidate explicitly; no destructive automatic rollback.
- Needs: L6. No manager or cross-chat speculative authority.

## L8: Full local acceptance

Result: disposable Laravel journey works in the real browser and harness, and
documentation records the shipped behavior and limits.

- Owners: controller across existing owners; each fixes its scoped defects.
- Work: existing/new scaffold; producer/consumer task pair; forced check,
  critic and plan failures; interrupted preparation/execution/check/review;
  disjoint concurrency and read/write conflict; documentation reconciliation;
  whole-branch review and delivery gate.
- Proof: observe shadows serve, browser outcomes and logs; record environment,
  harness/model and results. Windows/Linux automated tests are distinct from
  physical/provider acceptance. Preserve the person's existing testing1;
  exercise destructive/failure cases only on disposable fixtures.
- Needs: L7.

## Delivery rules

`L1 -> (L2, L3) -> L4 -> L5 -> L6 -> L7 -> L8`.

The dependency shape does not authorize overlapping writers/builds. Each slice
has a short-lived branch, focused per-task checks, a separate commit for each
completed verified task, one branch review and one full gate before its PR.
Browser/log acceptance precedes recording the slice as delivered.

Detailed plans belong to docs/backend/plans and docs/frontend/plans, with
accepted shared requests. They must expand current symbols into exact callers,
contracts, tests and API shapes, and put blocked/ready tasks in the owning
stages files. This program reserves no B/F numbers, starts no product work and
marks no current feature as delivered.

## Written-design checkpoint

Review the owner amendment, including preparation-worker, ownership, exact-path
and evidence-model choices. A brainstorming builder check checks that the design
can be planned without material guesses. After Mohammed approves that written
design, invoke writing-plans and produce the dispatchable implementation plans.
The owner design also resolves the builder check's five material questions:
attributed preparation authorization, idle repair-session/attempt binding,
exact contract-shape compatibility, output-specific evidence associations and
immutable source artifacts for critic reads.
Independent read-only builder check completed on 2026-11-09. After the five
resolutions were added, the fix round reported no important unanswered
questions. This is a design-completeness check, not automated product testing
or Mohammed's approval of the written design.
The subsequent LSP/task-knowledge amendment is a further design revision; its
candidate-server trial remains planned, not a claim that LSP works in Shadows.

## Coverage of the earlier section 17

| Earlier obligation | Delivery |
|---|---|
| 17.1-17.2 contract and rationale | L3, using repository facts from L1/L2. |
| 17.3 decisions and provenance | L3; pinning in L4. |
| 17.4 readiness, lifecycle and gates | L4, L6 and L7. |
| 17.5 packets, bounded questions and problems | L4; problem routing in L7. |
| 17.6 independent evidence and review | L5/L6; repair in L7. |
| 17.7 web views | Included with the API in each runnable slice. |
| 17.8 persistence, API and immutable history | Included in each owning slice, with legacy-version handling in L3/L4. |
| 17.9 staged delivery and 17.10 tests | L1-L8; focused checks per task and real journey in L8. |
| 17.11 authority and owner-document changes | Current section 24 policy; slice-local amendments and final reconciliation in L8. |
| 17.12 production operations | Outside the confirmed local-system scope. |
