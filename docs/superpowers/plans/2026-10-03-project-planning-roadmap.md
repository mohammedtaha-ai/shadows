# Project planning — delivery roadmap

- **Date:** 2026-10-03
- **Status:** Draft for Mohammed's review.
- **Purpose:** The whole delivery sequence requested during brainstorming.
  This is a roadmap, not a task-by-task implementation plan or an execution
  authorization. Each stage gets its detailed plan after its written spec is
  reviewed.
- **Product intent:** [vision §9](../../vision.md#9-the-roadmap-shared-contracts-and-the-effect-of-a-change).
- **First release owner:** [§18](../specs/2026-10-03-project-planning-workspace-design.md).

## Starting point

§16 1a was squash-merged into `main` as `4d8774e`: project-owned plans,
version provenance, archive, continuation, conversation deletion and project
events. The planning branch starts there. §16 1b remains specified but has no
implementation plan; §17 remains a draft for task contracts and evidence.
No executors or executive manager are claimed to exist.

## Shared foundations

The first release establishes §18's element identity, free organization,
roadmap references, immutable agreed contract versions, explicit participant
adoption and work-state separation. Later stages reuse those identities.
The roadmap does not reproduce their decisions.

| Stage | Runnable result | Design owner / prerequisite |
|---|---|---|
| 1. Planning workspace | Write a vision, create nested parts, attach existing plans, build a stage from outcomes and navigate its related work | §18; existing §16 plans |
| 2. Shared API agreements | Link two plans' tasks to one API contract; propose a revision, inspect impact, approve it and track adoption | §18; Stage 1 |
| 3. Planner context | Start planning at an explicit part or stage with bounded, attributable context; inspect what was included | Extend §13's session owner and specify the planning context compiler after Stages 1–2 |
| 4. Executors and verification | Configure an executor and critic, run one bounded task serially, resolve a needed symbol and review actual results | §17 execution and §19 profiles/critic dispatch after review; Stages 2–3; adapter scope enforcement and person authority |
| 5. Executive manager and specialists | Investigate problems, direct repairs, dispatch selected specialists; configure extensions and additional tested provider connections | §19 profiles/extensions plus a later manager authority owner; Stage 4 evidence; separate runnable slices for manager, tools and providers |
| 6. Diagrams and views | Explore structured schema or screen design and regroup the same elements by architecture or domain | A later owner spec using Stage 1 identities; specialized diagrams are not an execution prerequisite |

Each stage ends with a Windows browser trial and a dated evidence record.
Readiness is measured on the running product, separately from automated tests.

**2026-10-03 refinement:** the core value is reusable understanding followed
by focused execution. Prove that loop before specialized visual editors.
Stage 1 retains simple map/navigation views. Preparation and coordination cost
count toward success. The person configures many roles but runs only tasks
that are ready and safe together; no large simultaneous swarm is required. The dated
[research record](../../evidence/2026-10-03-focused-agent-orchestration.md)
separates external reports and inspected code from unmeasured expectations.

## First release: the smallest complete contract-change journey

Deliver the minimal Stage 1 foundation followed by Stage 2. Do not implement
every future view or executor component in these PRs.

1. A person creates Backend and Frontend parts, with another level below one
   of them, without a mandatory template.
2. A roadmap outcome, "A user can log in", references both parts and their
   existing plans.
3. A project API contract contains a login operation. The person agrees v1.
4. Each plan's Draft binds its relevant task to that operation and exact v1,
   as provider or consumer; the person can approve the plans.
5. Frontend proposes v2 with a reason. The review shows what changed and
   the registered affected tasks and plan states.
6. The person agrees v2. Both old plan versions still refer to v1.
7. Continuing an affected plan creates a new Draft; adopting v2 is explicit.
   The person sees which participant still uses v1.
8. Restarting the daemon preserves the hierarchy, agreements, bindings and
   review history. Existing conversations and plans remain readable.

The first release's Planner may read and propose through project-scoped tools.
It does not yet get the automatic context compiler or task execution.

## Where §16 1b fits

§16.7's task-to-task links remain a separate feature. Their targets follow
the other plan's latest version. §18's agreement bindings name an exact
contract version. The two relations must not be conflated.

Deliver the remaining §16 1b work as a separately scoped plan/PR in Stage 2,
with §16 owning its checks and graph. Stage 2 delivery starts with that PR,
then delivers shared API agreements in their own scope. §16.9 owns the
migration sequence: Stage 1 uses 0013–0015, 1b ships 0016, and agreements start
at 0017. This orders schema delivery; the two relation kinds retain their
independent semantics. Amend §16.9 in place if unshipped numbering changes.
Never create unused schema merely to reserve a number.

## Stage exits

- **1:** identity survives rename/reparent; containment rejects cycles; old
  plans can be attached without rewriting Frozen content; one outcome can
  reference several parts; reload and restart retain the result.
- **2:** agreement and plan versions remain immutable; proposal/approval
  does not silently migrate consumers; stale impact review cannot be accepted;
  an archived plan remains readable and its bindings remain visible.
- **3:** browsing leaves planning scope fixed; the context manifest names its
  sources and revisions; a changed ancestor invalidates a reused summary;
  a too-large packet is refused or explicitly narrowed. Compare actual token
  use for a focused case and a whole-project case.
- **4:** write boundaries are enforced before a run; a wrong-plan report
  cannot rewrite a Frozen plan; execution evidence and review remain distinct;
  stop/retry/crash paths are demonstrated for the selected adapter. Demonstrate
  a targeted lookup, an unresolved dependency escalation and rejection of a
  stale location. Compare equivalent accepted work with and without prepared
  context, including preparation, retries, review, time and human corrections.
  A Dashboard-selected critic reviews the attempt's actual result, and failed
  review reaches the person/Planner until the manager exists (§19.3).
- **5:** a proposed contract revision does not indiscriminately stop work;
  affected runs have an explicit decision, reason and authority; escalation
  and recovery are demonstrated with a real executor. Show one local repair
  resolved by the manager, one plan-level issue resolved through the Planner,
  and one reserved decision reaching the person. Budget exhaustion stops the
  repair loop; restart neither loses a decision nor duplicates a dispatch.
  Demonstrate a selected researcher/library-reviewer profile, one plugin and
  one MCP connection enabled for a role, and one additional tested provider
  connection without mixing profile configuration or leaking credentials (§19).
- **6:** the diagram and detail editor describe one design; the same element
  keeps its identity in different views; intended design and observed code or
  schema are distinguishable. No inferred match is presented as proof.

## Work deliberately left for later designs

Parallel writes, team accounts, a company server and remote execution need
their own scope and proof. §19 introduces a staged path for profiles, extensions
and additional provider connections; it promises no universal CLI/model
compatibility. These capabilities do not exist merely because Stage 1 allows
arbitrary project structure.

The manager's enforceable authority policy, agreement deprecation, automated compatibility
classification and specialized diagrams are decided in their stage owners,
not in a detailed task list written prematurely here.

## Next artifact

The [Stage 1 implementation plan](./2026-10-03-planning-workspace-stage-1.md)
is now written for review; it has not been executed.

Review §18 together with the Stage 1 plan before choosing its execution
method. The plan specifies code ownership, migration allocation, public
interfaces, focused tests and the browser acceptance path. Stage 2 follows
the demonstrated Stage 1 baseline; later stages keep their separate design
and implementation reviews.
