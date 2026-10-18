# NYLRA Vision, Documentation and Acceptance Transition Plan

Date: 2026-10-18. Status: documentation/migration plan; no file relocation,
source deletion or product migration has been executed.

**Goal:** Carry the existing NYLRA product, evidence and independently owned AYLUL
character into the agreed runtime architecture without losing verified behavior
or keeping contradictory active decisions.

**Design:** [AYLUL integration](../specs/2026-10-18-aylul-hermes-semantic-context-design.md).
**Service boundary:** [Independent code intelligence](../../code-intelligence/specs/2026-10-18-code-intelligence-service-design.md).
**Baseline:** [Pre-transition tracked inventory](../../evidence/aylul/2026-10-18-nylra-transition-inventory.json).
The inventory captures paths/hashes of 143 selected tracked files at NYLRA
`2b0427339a4e92d07cc7a0811dbafb2c5faecffd`; it is not a semantic audit or new test run.

## 1. Separate decisions from movement

Documentation routing was clarified on 2026-10-18: `docs/backend` and
`docs/frontend` remain Core-specific. New integration drafts are filed under
`docs/aylul`, independent service work under `docs/code-intelligence`, and
cross-runtime agreements are routed by `docs/integration`. See `docs/README.md`.
Only the task-authored new drafts were relocated during documentation cleanup;
NYLRA's existing historical/source/document trees have not been moved.

The human stopped continuation of the former backend-compatible NYLRA client
path. Preserve everything already completed; unfinished old relay/consumer work
is not completed merely to satisfy its superseded roadmap. The backend is being
saved as a documentation snapshot on a new branch, with no further implementation.

The runtime boundaries and AYLUL-first experience are agreed. A repository
consolidation remains a later operation with an exact source/destination manifest.
Do not move a whole folder merely because the sketch places it under one root.
Current source maps continue describing current files until the move occurs.

Use one action per item: **retain**, **amend in place**, **move unchanged**,
**supersede/archive**, or **delete after review**. An item being superseded does
not mean its evidence or tested implementation is invalid.

## 2. Initial disposition ledger

Paths below start in `E:/Globalprojects/nylra`; S means the Shadows repository.
Destinations are a concrete consolidation proposal to review, not existing paths.

| Current owner/family | Action now | Proposed action if consolidation is approved |
|---|---|---|
| `Vision.md` | Amend the selected runtime/primary experience; retain the full capability vision | Move to `S/docs/aylul/vision.md` as the sole AYLUL vision owner |
| `docs/superpowers/specs/2026-10-10-nylra-delivery-design.md` | Amend brain placement, memory owner and personal-first ordering | Move active owner to `S/docs/aylul/specs/` with redirects/reference updates |
| `docs/source-structure.md` | Mark old planned placements subject to the new runtime split; retain actual map link | Move/amend as `S/docs/aylul/source-structure.md` |
| `docs/codebase/README.md` | Keep accurate for current implementation | Move to `S/docs/aylul/codebase/README.md`, updating paths only after code moves |
| `docs/character/` | Retain its independent design/plan/stage ownership | Move intact to `S/docs/aylul/character/`; do not absorb it into Planner/native/brain docs |
| `docs/native/` | Retain native process/window/hook ownership; amend new runtime clients through its owner | Move to `S/docs/aylul/native/` with references adjusted |
| `docs/desktop-ui/` | Preserve existing scope; add AYLUL chat work only when its contract is ready | Move to `S/docs/aylul/ui/` |
| `docs/shared/` | Preserve accepted requests and their status/provenance | Resolve destination/ownership request-by-request, not by overwriting another side's request directory |
| Completed plans and old sequencing decisions | Keep as history; mark replaced ordering explicitly | Move to `S/docs/aylul/history/` only when no longer an active decision owner |
| `docs/evidence/` | Preserve dated facts and their acceptance limits | Move unchanged to `S/docs/evidence/nylra/`; provide a relocation index for former paths |
| `assets/aylul/README.md` | Preserve current procedural-renderer provenance and archive notes | Move with the character assets; do not substitute legacy PNGs for current source |
| `apps/desktop/src/features/companion/` | Retain source/contracts/studio separation | Move with the desktop app, preserving the character feature as its own owner |
| `apps/desktop/src-tauri/src/windows/companion/` | Retain native permission, placement and no-activation behavior | Move with native source; keep native window authority outside the renderer |
| Hook/sidecar/session/approval code and tests | Preserve the existing working path | Move with `apps/desktop/`; no rewrite as Core services |
| `apps/desktop/tests/`, hook and sidecar tests | Preserve assertions/fixtures and command provenance | Move with their owners and repair path-only imports before behavioral changes |
| Ignored `archive/`, `.runs/`, installed dependencies or credentials | Inventory presence only when needed, never auto-read/copy secrets | Handle selected irreplaceable art/evidence explicitly outside Git; regenerate dependencies |

This is a family-level starting ledger. Before any move, expand it to every
selected file with source commit/hash, destination, action, owner, references and
acceptance impact. Files not reviewed do not acquire a fictional approved disposition.

## 3. Decisions that change

- Replace the proposed duplicated local/cloud custom `aylul-brain` runtime with
  the selected personal-first Hermes brain integration. Existing backend code/data
  remain until a separate compatibility/migration decision permits retirement.
- AYLUL is the primary desktop experience; the existing Shadows UI loads on request.
- `AppCore` composes Shadows only. Native AYLUL, code intelligence and Hermes
  retain independent composition/state/lifecycle owners.
- Code intelligence is independently usable by direct coding clients; Core and
  AYLUL consume it. Serena is a learning baseline, not implicitly the licensed
  source of the eventual package.
- Personal Windows acceptance comes before phone/server product expansion for
  this delivery. Earlier full-product capabilities remain in the vision unless
  the human explicitly removes them.

Record each changed decision in its existing owner, with a date and scope.
Do not rewrite a historical test report to make it agree with the new vision.
Delete an obsolete active decision only after its replacement owner is named,
all active references are updated, and its historical commit remains reachable.
Do not delete a test because it exposes an accidental regression from relocation.

## 4. Character/avatar remains independent

The current character owner is
`docs/character/specs/2026-10-16-aylul-in-window-design.md`; the native floating
surface is owned by `docs/native/specs/2026-10-16-independent-companion-design.md`.
Neither is replaced by the Hermes integration design.

Preserve the procedural Canvas renderer, animation model/scheduler, message
lifetime, scene/appearance options, development-only studio and current asset
provenance. The companion can be hidden or fail without disabling text chat,
session monitoring, approvals or opening Shadows.

The renderer receives a small presentation projection through the existing
native/UI boundaries. It does not import Core rules, Hermes credentials or
sidecar command dispatch. Native code remains authoritative for render permission,
window visibility/placement, pause and passive no-focus-steal behavior. Model text
cannot manufacture an approval or directly drive privileged native operations.

Keep actual chat/microphone/speaking/working status distinct from simulated studio
states. Do not infer work success from an animation. A redraw/asset/design change
needs its own character scope; infrastructure integration is not permission to redesign it.

## 5. Existing acceptance is carried forward with its limits

Inspected dated records include:

- `docs/evidence/2026-10-13-aylul-hook-probe.md`: actual Claude hook experiment
  and untested auto-mode details; retain the recorded version/environment limits.
- `docs/evidence/2026-10-16-aylul-in-window-acceptance.md`: renderer, studio,
  motion and automated presentation evidence; remaining physical/runtime limits stay open.
- `docs/evidence/2026-10-16-independent-companion-acceptance.md`: independent
  native window behavior with separate mocked and actual-device evidence.
- `docs/evidence/2026-10-17-hook-companion-merge.md`: recorded frontend build,
  7 focused tests, 15 companion unit tests, 1 production-graph test, 7 Chromium
  shell tests, 71 sidecar tests, 12 hook tests and 29 desktop Rust tests; the
  opt-in Docker native auth case remained ignored. These counts describe that
  dated run, not checks rerun during this planning turn.

Preserve test names, assertions, fixtures and acceptance reports before movement.
After relocation, rerun affected commands in their new location and write a new
dated report. A path-only move and a behavior change are separate reviewable steps.
CI is not physical Windows acceptance; browser fixtures are not real Claude/Codex
or real cloud-provider acceptance. Historical ignored tests do not become passes.

## 6. Execution order

### M-0 - Inventory and reference audit

- [ ] Expand the initial 143-file inventory with remaining selected tracked docs,
  build metadata, test entry points and explicit ignored-art/evidence needs.
- [ ] Map each active design section to one current owner; flag conflicts with
  the updated vision instead of copying both versions into the new tree.
- [ ] Record broken/cross-repository links and relative runtime/build paths before movement.

### M-1 - Amend vision and existing owners

- [ ] Review the dated changes in Vision/delivery/cloud architecture and ensure
  the old custom-brain placement is not still presented as the selected active path.
- [ ] Keep implemented code maps factual, completed stages historical and new
  task readiness explicit. Do not erase completed work because ordering changed.
- [ ] Confirm that character/native appearance owners remain authoritative.

### M-2 - Relocate one owner at a time after destination approval

- [ ] Produce exact absolute source/destination paths and verify destinations stay
  inside the approved target checkout before recursive moves.
- [ ] Preserve Git provenance; never reset the source checkout or overwrite
  unrelated destination files. Do not copy ignored credential/dependency stores.
- [ ] Move source with its contract/tests and update maps/imports/build roots.
  Relocate docs through the manifest; one canonical owner plus redirects, not two copies.
- [ ] Verify unchanged-file hashes against the inventory. Every hash difference
  must be classified as a reviewed path edit or a separately authorized behavior change.

### M-3 - Revalidate and retire replaced decisions

- [ ] Run path/contract/build checks and the affected existing suites first.
  Repeat real Windows/agent journeys where native/process boundaries moved.
- [ ] Write new dated evidence with environment, versions, outcomes and remaining gaps.
- [ ] Check the current vision and entrypoint docs for competing decision owners.
  Remove only explicitly reviewed obsolete material, retaining provenance/redirects.
- [ ] Retire a former repository/runtime only after the new location has passed
  its required acceptance and retained data/artifacts are accounted for.

No move, deletion, new test pass or visual redesign is claimed by this plan.
