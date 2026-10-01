# Section 17 — Executable Task Contracts and Evidence

- **Date:** 2026-10-01
- **Status:** Draft for owner review.
- **Idea:** `docs/vision.md` §§2–5: agents work within explicit task boundaries,
  report evidence when the plan is wrong, and completion is established by
  verification rather than by an agent's claim.
- **Related:** §13 (Planner and task links), §15 (code index), §16 (project-owned
  plans and versions).

This section moves a plan from a list of tasks to a contract an executor can
follow and the runtime can check. Planning, execution, verification and human
decisions remain distinct kinds of state.

## 17.1 Scope

The work is delivered in two runnable stages. Stage 2 depends on Stage 1.

**Stage 1: task contract and rationale.**

- Enrich task content with implementation and interface constraints (§17.2).
- Project decisions with their rationale (§17.3). A version's reason and the
  writer of every edit are §16.3's.
- Keep plan approval separate from execution readiness (§17.4).
- Extend the plan and task views to show the contract, changed fields, and
  version rationale (§17.7).

**Stage 2: bounded execution and evidence.**

- Compile an immutable, bounded packet from a frozen version and one ready
  task (§17.5).
- Record a task run, deterministic verification, and review outcomes (§17.6).
- Show runs, evidence, failures and gate actions in the Web client (§17.7).

**Not here:** selecting multiple executor vendors, running overlapping tasks
in one working directory, worktree management, automatic waiting-test
scheduling, project teams, remote execution, and general-purpose semantic
memory. These need separate designs. In particular, parallel execution stays
off until write-scope conflict handling is designed and proven.

## 17.2 The task contract

`TaskContent` remains the canonical task content stored in a plan version.
Existing fields `number`, `title`, `goal`, `reads`, `writes`, and `acceptance`
remain. The following fields are added:

| Field | Meaning |
|---|---|
| `target_symbols` | Known functions, types, routes, or other symbols in scope. |
| `interface_contract` | Inputs, outputs, and externally visible behavior this task owns. |
| `implementation_steps` | Ordered, reviewable steps; executor choices inside a step stay local. |
| `invariants` | Properties that must remain true after the change. |
| `error_semantics` | Required error outcomes and the conditions that produce them. |
| `forbidden_changes` | Explicit changes the executor must not make. |
| `allowed_capabilities` | Narrow, typed runtime capabilities needed to perform the task. |

All fields are serialized in the versioned plan content and included in its
canonical digest. `reads` and `writes` are repository-relative paths; malformed
or escaping paths are refused. An empty `writes` list authorizes no file
changes. Unknown permission names fail closed. A task's writable paths are a
scope boundary, not a prompt suggestion. Capabilities come from a
server-owned typed registry; wildcard capabilities such as `filesystem` or
`shell` are not valid. An empty list grants no capabilities. A task is not
executable unless its selected adapter can enforce each requested capability
with an exact mapping; otherwise readiness is refused.

Each acceptance item remains a sentence that can be checked. The contract may
refer to a deterministic command, but merely naming a command does not prove
that it passed. Plan validation checks structural completeness; executable
readiness additionally checks that the task has a usable write boundary,
acceptance criteria, and all declared dependencies satisfied.

## 17.3 Version rationale and edit provenance

**A superseding version's `change_reason`** is §16.3's, built with §16: it
is required from v2 on, and a version from before it reads "Reason not
recorded". This section adds nothing to it.

Every `WorkflowEdited` event continues to record its actor and a concise
machine-readable change summary. A revision conflict still refuses the whole
edit; the editor must reread the latest revision and rebuild its command.

When an edit settles or reverses a project decision, its rationale must be
recorded as a project decision with status `Proposed`, `Accepted`, or
`Superseded`, its author, references, and rationale. An agent may propose a
decision but cannot accept or supersede one; those transitions require a
person's command. Until the person-authority boundary in §17.11 is closed,
decision records can be proposed and read, but transitions to `Accepted` or
`Superseded` are unavailable. Execution packets include only accepted
decisions selected as relevant to the task. Decision identity and history are
project-scoped and do not depend on a conversation's lifetime.

The Web version history shows the version number, writer, creation time, and
`change_reason`. Each edit event can be inspected for its actor, changed task
numbers, and summary. Detailed decision history is linked from the task or
version that references it.

## 17.4 Plan approval and execution readiness

A plan version has the planning states already defined by §16: `Draft` and
`Frozen` (shown as Approved). Freezing approves immutable planning content; it
does not claim that a task has run or passed verification.

Execution is a separate record bound to one frozen plan version. A task run
advances only through valid transitions:

```text
Prepared → Running → Executed → Verified
                  ↘ Failed
```

`Failed` and `Verified` are terminal for that attempt. Retrying creates a new
run; the runtime never resets a failed run to `Prepared`. A crash or lost
external outcome is represented as unknown until reconciled, never reported as
a confirmed failure or success without evidence.

A gate is a durable pause over a declared set of tasks and checks. It becomes
eligible only when required task results and verification evidence exist. Once
the person-authority boundary is closed, a person chooses one explicit action:
`CONTINUE`, `REPLAN`, `PATCH`, `ROLLBACK`, or `ABORT`. The stored review outcome
is one of `PASS`, `PASS_WITH_NOTES`,
`FAIL_IMPLEMENTATION`, or `FAIL_PLAN`. `FAIL_PLAN` returns control to planning;
it does not authorize an executor to rewrite the frozen contract.

The first execution stage is serial. Readiness is checked immediately before a
run using the frozen version, current project binding, task dependencies, and
write scope. If two tasks declare intersecting write paths, they are not
eligible to run concurrently until a later design defines a safe conflict
policy.

## 17.5 Bounded execution packet

`TaskContextCompiler` produces an immutable `ExecutionPacket` for one ready
task. It contains:

- the project, plan version, task number, run identity, and source revision;
- the task contract and the dependency contracts required by its links;
- accepted project decisions selected for this task, each with provenance;
- the selected repository binding and base revision;
- bounded repository context selected from declared reads and relevant code
  index results, with path and content digests;
- the exact write scope, allowed capabilities, timeout/resource budget, and
  instructions;
- a canonical digest over every packet field.

The compiler refuses a mutable or unapproved plan, an unready task, unresolved
dependencies, a missing or ambiguous project binding, paths outside the
project, invalid scope patterns, unavailable required inputs, and context over
budget. It reads from one captured repository revision; if that revision
changes during compilation, it refuses rather than mixing snapshots.

The packet is persisted or referenced immutably by its digest. A retry uses the
same packet only when it is the same logical run; a new attempt compiles and
records a new packet. Provider-native conversation history is not canonical
task context.

## 17.6 Execution evidence and verification

The executor receives the packet and returns structured execution evidence:
run identity, changed paths, process outcome, and bounded artifact references.
Provider text saying “done” is not a successful run result. Diagnostics are
bounded and secret-redacted.

The adapter must enforce the packet's capabilities and write scope before it
starts. After execution, the verifier independently compares the observed
changes with the frozen write scope. If the adapter cannot enforce the exact
scope, the run is refused before launch; a post-run check alone is not treated
as containment.

`Verifier` checks the run against the frozen packet and a baseline captured at
the packet's base revision. It records:

- the packet and verification-contract digests;
- the baseline identity and changed paths;
- each deterministic check's command identity, exit status, and bounded result;
- scope violations, baseline discrepancies, and the resulting verdict;
- verifier identity and timestamp.

The verifier is deterministic and does not decide whether the plan itself was
correct. A separate Reviewer evaluates implementation evidence against the
task contract and can return the four review outcomes in §17.4. Review output
is attributable and immutable. Until a semantic Reviewer exists, the UI says
“verification passed” and does not imply “review passed.”

Verification or review failure does not mutate a frozen version. A plan
failure creates a proposed change for the Planner and, if accepted, a new plan
version with a required reason. An implementation failure leaves the run and
its evidence available and requires a new attempt or an explicit gate action.

## 17.7 The Web client

The project landing page is an overview of current work: active plans, ready,
running and blocked tasks, latest verification results, and items that need a
person's decision. It offers direct actions to continue planning, inspect a
task, or start an eligible task.

The plan page offers both a graph and a task list. Selecting a task opens its
contract, dependencies, accepted decisions, declared read/write paths, and
acceptance items. Version history shows the author and `change_reason`.

The execution view groups runs by plan and task. It shows the executor, current
state, stop/retry availability, changed paths, checks, verifier verdict,
review outcome, and gate actions. Color is never the only indication of state.
Arabic and English content keeps the existing per-block RTL/LTR behavior.

Approval and gate actions are issued only through the person-facing API
authority. An agent cannot approve a plan, accept a decision, or choose a gate
action. These authority checks are enforced by the backend, not only hidden in
the UI.

## 17.8 Interfaces and persistence

The plan/task contract changes update the owning `plans` contract, MCP schemas,
HTTP/OpenAPI schemas, generated Web types, canonical serialization, and digest
tests together.

The migration preserves existing plan, workflow, task, event, and thread IDs.
Legacy task fields receive explicit empty defaults only where that remains a
valid non-executable Draft. Existing frozen versions remain readable; they do
not become execution-ready until the required contract fields are supplied in
a new version. How a version with no recorded reason is stored and read is
§16.9's.

New persistence is project-scoped for decisions and plan versions, and
version-scoped for packets, runs, checks, and review outcomes. Foreign project
or version references are refused before writes. Retention does not delete the
evidence needed to explain a terminal run or a gate decision.

## 17.9 Order of work

**Stage 1:**

1. Add typed task-contract fields, canonical serialization, validation, and
   migration defaults.
2. Add a project-scoped, person-accepted decision record with provenance
   (`change_reason` is built with §16).
3. Update MCP and HTTP contracts, generated Web types, plan/task pages, and
   focused acceptance coverage.

**Stage 2:**

4. Add execution, task-run, packet, check, and review persistence with
   immutable transitions and idempotent commands.
5. Add the bounded context compiler and packet digest verification.
6. Add one serial executor path and backend-enforced task write scope.
7. Add deterministic verification and separate review outcomes.
8. Add project overview, task/run views, evidence display, and explicit gate
   actions.

Each stage ends with a runnable path and independent review before the next
stage begins.

## 17.10 Tests and acceptance

1. A malformed task contract, escaping path, unknown capability, or empty
   executable write scope is refused before persistence or execution.
2. A superseding version keeps the old frozen content byte for byte (its
   reason and writer are tested with §16, §16.12).
3. A stale `expected_revision` refuses the entire batch with no partial task,
   link, decision, or event write.
4. Before the person-authority boundary is closed, acceptance and supersession
   are refused for every caller. After it closes, an agent can propose but
   cannot accept or supersede a decision; an authorized person can, and the
   transition is durable and replayable.
5. The same frozen task and repository snapshot produce the same packet digest;
   changing any contract, decision, selected file, scope, or base revision
   changes the digest.
6. Packet compilation refuses a dirty or changing snapshot, unresolved
   dependency, invalid binding, and over-budget context.
7. A run cannot skip lifecycle states, be claimed twice, or be reset after
   `Failed`; process loss with unknown outcome is not silently retried.
8. A write outside the frozen task scope is detected and cannot be reported as
   verified.
9. A deterministic verifier records a repeatable verdict from the same
   baseline, packet, and check results. Reviewer outcomes remain distinct from
   verifier verdicts.
10. The Web client distinguishes plan approval, execution readiness,
    verification, and review; it shows reasons, evidence, and gate choices
    without relying on color alone.

Acceptance covers Windows and Linux with disposable repositories and
databases. Credentialed CLI acceptance is recorded separately from automated
tests; no platform or real-user run is claimed without its own evidence.

## 17.11 Changes to other documents

After this specification is approved, update the canonical owners in the same
implementation stage that makes each statement true:

- **§13:** task contract and version rationale, with references here;
- **§16:** plan-owned task contracts and the project overview, with references
  here;
- **§4, §6 and §7:** decision, packet, run, verification, and review records;
- **`docs/vision.md`:** Today lines and links to the implemented stages;
- **`docs/status.md`:** verified progress only.

This file owns the design in §17. Other documents link here instead of copying
its decisions.

> **OPEN — person authority for decisions and gates.** The current local HTTP
> boundary does not prove that a request came from the person rather than a
> process running as the same user (§13.2). **Trigger:** before a decision can
> become `Accepted` or `Superseded`, or a gate action is enabled. Until then,
> agents may propose decisions, but the person-only transitions and gate actions
> stay unavailable. This does not block task-contract or packet work.
