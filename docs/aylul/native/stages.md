# Native (Tauri, sidecar, hook) stages

Where this side's work is now. `native-lead` keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

2026-10-18 direction change: preserve the completed companion/hook work below.
Do not continue the former shadows-backend-compatible relay/client roadmap.
AYLUL's new Hermes/Core/independent-service direction is documented in the
[transition plan](../plans/2026-10-18-nylra-transition.md).
No replacement native implementation task is activated by this documentation update.

Independent companion specification approved by the user on 2026-10-16:
[`2026-10-16-independent-companion-design.md`](specs/2026-10-16-independent-companion-design.md).
The implementation plan is approved; inline execution began on 2026-10-16.
Execution remains inline in
the original checkout; no worktrees.

## Tasks

The accepted cross-side interface is in
[`2026-10-16-independent-companion-window.md`](../shared/requests/2026-10-16-independent-companion-window.md).

Tasks follow the approved
[`2026-10-16-independent-companion-implementation.md`](plans/2026-10-16-independent-companion-implementation.md).
The exact file lists and test assertions live in each task's Files and Steps.

### N-1 · Own native companion lifecycle and IPC
- Status: done
- Plan: docs/aylul/native/plans/2026-10-16-independent-companion-implementation.md#task-1-native-companion-owner-and-restricted-ipc
- Read: approved native spec; windows and commands contracts; code map; accepted independent-companion shared request.
- Touch: exactly Task 1 Files, including its four named native companion source files and ACL/composition changes.
- Needs: accepted independent-companion shared request; implementation-plan review.
- Done when: Task 1 focused native tests and formatting pass; inspected contracts/map updated.

### N-2 · Synchronize native render permission
- Status: done
- Plan: docs/aylul/native/plans/2026-10-16-independent-companion-implementation.md#task-2-race-safe-platform-state-and-explicit-animation-permission
- Read: Task 1 state/commands; platform and companion contracts; approved spec section 4.
- Touch: exactly Task 2 Files.
- Needs: N-1 interfaces.
- Done when: Task 2 native-state Node tests, native-visibility browser tests and frontend build pass.

### N-3 · Present the independent character and gestures
- Status: done
- Plan: docs/aylul/native/plans/2026-10-16-independent-companion-implementation.md#task-3-compact-independent-entry-and-reliable-gestures
- Read: Task 2 interfaces; companion contract and renderer; approved spec section 3.
- Touch: exactly Task 3 Files.
- Needs: N-2 interfaces.
- Done when: gesture, desktop-window and production-graph checks pass; 240px visual inspected.

### N-4 · Connect main and tray controls
- Status: done
- Plan: docs/aylul/native/plans/2026-10-16-independent-companion-implementation.md#task-4-main-controls-and-tray-integration-without-duplicate-animation
- Read: Task 1 native owner; Task 2 wrappers; app/windows contracts; accepted shared request.
- Touch: exactly Task 4 Files.
- Needs: N-3 surface and N-1/N-2 interfaces.
- Done when: focused shell/native-owner checks and frontend build pass with account behavior preserved.

### N-5 · Verify native behavior and record remaining gates
- Status: done
- Plan: docs/aylul/native/plans/2026-10-16-independent-companion-implementation.md#task-5-product-native-acceptance-profiling-and-final-review
- Read: Tasks 1–4 inspected contracts; approved native spec section 6; existing dated Windows evidence.
- Touch: exactly Task 5 Files, including reproducible fixes inside Tasks 1–4 scopes.
- Needs: N-4 integrated product.
- Done when: final checks, actual Windows observations and fresh review are recorded; unperformed or failed device/performance gates remain explicitly open.

Implemented and locally verified 2026-10-17 in debug, per user instruction.
See [current acceptance](../../evidence/nylra/2026-10-16-independent-companion-acceptance.md)
for exact results and remaining optimized performance, hardware and hit-test gates.

## Next discussion: desktop laptop relay

The human requested collaborative planning on the same existing branch and
one commit per completed, verified task. The inspected starting points and
scope decisions are in the
[planning brief](specs/2026-10-17-laptop-relay-planning-brief.md).
The human subsequently requested merging their joint human/Claude hook work
into this branch. That local integration is recorded in
[merge evidence](../../evidence/nylra/2026-10-17-hook-companion-merge.md) and its
[accepted shared request](../shared/requests/2026-10-17-hook-companion-merge.md).
Cloud relay-consumer implementation tasks remain in planning, not started.
