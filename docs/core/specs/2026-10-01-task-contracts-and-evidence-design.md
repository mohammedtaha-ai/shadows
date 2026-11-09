# Section 17 — Executable Task Contracts and Evidence

- **Date:** 2026-10-01
- **Status:** Draft for owner review. Mohammed confirmed complete local-system
  scope on 2026-11-09. The written design in 17.0 still awaits his approval.
- **Idea:** `docs/core/vision.md` §§2–5: agents work within explicit task boundaries,
  report evidence when the plan is wrong, and completion is established by
  verification rather than by an agent's claim.
- **Related:** §13 (Planner and task links), §15 (code index), §16 (project-owned
  plans and versions).

This section moves a plan from a list of tasks to a contract an executor can
follow and the runtime can check. Planning, execution, verification and human
decisions remain distinct kinds of state.

## 17.0 Local delivery revision, 2026-11-09

This proposed revision incorporates Mohammed's current requirements and the
built executor path. Scope approval is not written-design approval. After
approval, 17.0 supersedes conflicting details in 17.1-17.11. Section 17.12
remains future production work. See the
[delivery program](../../backend/plans/2026-11-09-local-delivery-program.md).

### Intent and user decisions

The Planner plans against the real repository. Shadows shows files already
installed by the person. Once framework and destination are agreed, prepare
an absent scaffold before detailed planning continues. Layout follows that
framework and the person, never a universal folder template.

A task specifies files to read and why, files to create/modify, required
functions/classes, responsibility, inputs, outputs and behavior. The executor
writes the solution and its tests, not a supplied finished function body.
Shadows retains producer/consumer relationships; an executor receives the
contracts needed for its own task, not the whole Planner transcript.

Mohammed confirmed the complete local scope: discovery, detailed contracts,
execution context, independent checks/evidence, critic and bounded repair.
This adds no cloud/production, team, manager or editable specialist-dashboard
requirement to the local path.

### Baseline and approaches

Inspected at `cbf2b597` on `fix/first-run`: Code indexes Rust and JS/TS variants,
not PHP or a complete file inventory. TaskContent has goal, reads, writes,
acceptance and phases. Task context has selected layout and optional phase
dependencies, not captured source. Runs owns attempts, held writes, reported
commits and manual done marks. The executor commits and calls `task_report`;
Git/path validation is not independent behavioral verification. Critic,
execution packets and the repair ladder are not built in this path.

Two approaches were discussed:

1. Rich task prose plus a file browser: smaller change, but relationships,
   stale definitions and readiness remain interpreted from text.
2. Structured contracts over existing owners: more storage/API/UI work, but
   Core can validate references, capture inputs and explain blocked work.

The controller recommends approach 2 for written-design approval because it
matches the confirmed requirement that Shadows retain these relationships.
This is task-local detail, not a catalog planning every project file.

### Ownership

Start at AppCore. No new service, second composition root or cross-store
access is proposed. Planned responsibilities stay out of current contracts
until their implementation ships.

| Existing owner | Proposed responsibility |
|---|---|
| Code | Inventory, refresh state, file observations and scoped semantic queries through mature LSP clients/servers. |
| shadows-index | Its existing parser-only tags and explicitly labeled name-based fallback. |
| Design | Versioned project decisions and existing shared agreements. |
| Plans | Canonical task contracts, contract pins, dependency validation and owner-thread lookup. |
| Runs | Preparation attempts, execution packets, leases, checks, review, acceptance and repair. |
| Harness and Grants | Session policies, scoped tools and captured configuration. |
| Threads, Turns, Operations | Their existing conversation/turn/operation lifecycle. |
| HTTP, MCP, web | Translate owner operations and show owner results. |

Runs calls Plans, Design and Code through explicit dependencies wired by
AppCore. A compiler/verifier is a job inside Runs, not another facade.
Planning `task_context` remains a Plans read. A distinct execution-context
operation on Runs serves the compiled packet to executor/critic grants; do not
make Plans depend on Runs or put grant-kind business dispatch in an adapter.

### Repository discovery and preparation

Inventory is separate from symbol extraction. Code lists relative paths even
for unsupported languages, with kind, scan identity/time and exclusion reason.
Inventory is paged; incomplete/stale/unavailable scans remain visible. A failed
scan does not delete unseen entries. Paths cannot escape the project through
symlinks/reparse points. Dependency/build trees are summarized by default.
Secret-bearing entries expose redacted metadata, not their contents. Inventory
does not become an arbitrary host file-content API. Use a ready-made PHP
language server for PHP definitions. Preserve the existing parser/name-based
index as a labeled fallback; a second PHP analysis engine is not a prerequisite.

The Planner reads inventory and relevant manifests before detailed planning.
Existing Laravel is adopted, not installed again. Refresh after scaffold or
external file changes before accepting exact file references. Content reads
record observations/digests; cached paths alone do not prove current content.

**Controller proposal:** the Planner requests a bounded preparation attempt
through Runs after the person confirms stack, destination and operation. A
separate preparation worker performs the setup; ordinary planning sessions
keep their tool policy. No approved product plan or fake task is needed.
Capture request identity, destination, model, inventory, output manifest, Git
result and outcome. Start requires an empty/missing destination and no lease
conflict. A nonempty destination is inspected, never overwritten. Retry checks
partial results before doing anything. Cancellation/restart leave inspectable
state, not automatic cleanup. Refresh inventory before detailed planning resumes.
Only the worker's own product files enter its commit, never runtimes/installers.
Normal package caches may be used; this is not OS-level shell containment.

### Task knowledge and ready-made language servers

**Mohammed's direction, 2026-11-09:** reuse mature LSP implementations alongside
Shadows' task knowledge. Language-server results can be incomplete or wrong;
they do not replace the approved contracts, source observations or read scope.

The official Claude `php-lsp` plugin configures `intelephense --stdio`; the
plugin does not contain that server. Intelephense documents an independent LSP
client interface, so the same installed server can be a candidate for Shadows
without requiring Claude. Use a maintained LSP client library, not a custom
protocol implementation. Install/configure servers separately, detect existing
executables explicitly, capture their versions/capabilities and test them before
reporting availability. Do not attach to Claude's private running process or
hard-code its plugin-cache paths. Server licensing and free/premium capability
availability remain distinct from the Claude plugin's license.

Code owns the local semantic gateway and worker lifecycle behind AppCore.
This does not select the standalone package's runtime or release architecture.
Keep its separate CI-3/package decision deferred. L1 starts with a bounded
PHP/Laravel server/client trial and the already supported Rust/TS behavior;
choose the concrete client/server combination from measured freshness,
same-name resolution, lifecycle, licensing and task-scope results before
committing implementation to it. Mohammed requested the other official catalog
choices too; [section 15's catalog](2026-09-30-code-index-design.md#proposed-ready-made-server-catalog)
pins all 13 entries, requirements and enablement checks. Register them as
on-demand candidates; each backend becomes ready only after its own trial.
Additional languages use the same backend capability contract, not a new
product-specific analyzer per language.

An executor's code question names its attempt/packet, file or qualified symbol,
and the operation needed. Runs checks the task's allowed reads and supplies
only authorized query scope to Code through its public API. Code returns
semantic candidates and provenance; Runs joins them to the packet's source
observations and Plans' pinned contracts. Keep the dependency one-way: Code
does not acquire Plans/Runs or a second task-authority database. The planning
read path is project-scoped under the Planner's existing grant.

Three facts stay distinct in every answer:

- What checked source currently contains, with file/symbol and content digest.
- What the selected language server resolved, with backend/version, source
  observation, capability, candidates and any stale/unsupported state.
- What the approved task promises, including producer task and implementation
  evidence. A planned function is not described as an existing definition.

For example, a task allowed to read an account repository may ask for a named
method before using it. Check the file scope first, resolve through LSP, verify
the returned location/signature against source, then return the bounded relevant
definition and its pinned contract. An out-of-scope hit does not widen the
task; withhold its source and request scope resolution. If the method is only
planned, identify the producer and readiness state. If LSP and checked source
or the approved signature disagree, return the discrepancy and evidence rather
than choosing a silent winner. A verified source lookup can be a labeled
fallback when semantic resolution is unavailable; no result is not proof of
absence. Name-only and semantic references never share an unlabeled result.

A server may need an authorized whole-workspace view, including dependencies,
for analysis. The executor's disclosure/read scope remains narrower and is
enforced by Shadows on results and source excerpts. Do not claim per-file OS
read isolation for the LSP process itself. Refresh/synchronize server document
state for changed files and reject answers from an older workspace generation.
Critic review still reads captured artifacts, not a live LSP answer substituted
for the exact candidate source that was checked.

Primary sources checked on 2026-11-09:
[Claude code intelligence](https://code.claude.com/docs/en/plugins/code-intelligence),
[official PHP configuration](https://github.com/anthropics/claude-plugins-official/blob/main/.claude-plugin/marketplace.json),
[Intelephense integration and limitations](https://intelephense.com/docs).
These establish the candidate integration, not executed Shadows acceptance.

### Canonical task contract

Extend TaskContent rather than maintaining an unversioned parallel brief:

- File operation: task-local identity, exact relative path, action
  read/create/modify/delete, purpose and optional existing symbol observation.
  Creation names an intended symbol without claiming it exists.
- Symbol obligation: owning file operation, qualified function/class name,
  responsibility, inputs, output shape, behavior, errors and invariants.
  The executor chooses the internal algorithm. Signature text is not a proof
  of semantic/type compatibility.
- Produced contract: stable task-local identity for an operation, value,
  schema/table or configuration artifact. Not every artifact is a function.
- Consumed contract: inspected existing source with digest, an exact shared
  agreement operation, or a named output from a task. Within a plan version
  use task/output identity; across plans pin exact workflow/version/task/output.
  There is no implicit latest-version contract pin.
- Ordered implementation steps, forbidden changes, required capabilities and
  executable checks, linked to acceptance items.

Plans validates identities, paths/actions, missing outputs, shared-agreement
operations and task/phase cycles. A required consumed output contributes an
edge through the existing dependency graph, not a second readiness system.
Normal executable reads/writes derive from file operations, with exact paths;
they are not independently editable conflicting fields. Legacy globs remain
readable. Unknown generated paths require preparation or a reviewed generation
output resolved before execution, not a broad Modules/** grant.

Readiness requires accepted evidence for a produced contract or explicit
inspected adoption of existing implementation with checks. A manual done mark
does not invent verified source. Existing shared agreements remain the owner
of their operations and are referenced, not duplicated. This permits task-local
symbols while retaining the structure spec's rejection of a global element model.

Design owns project decisions with immutable revisions, rationale and
proposed/accepted/superseded states. Tasks pin relevant accepted revisions.
Approval provenance follows current section 24, including attributed Planner
approvals where allowed. Executor/critic grants never gain approval tools.

### Packets, readiness and bounded questions

Runs compiles an immutable packet through the owners: frozen contract, contract
pins, decisions, relevant rules, map/standards revisions, selected source
observations/digests, session configuration, exact scopes, checks and resource
limits. Canonical serialization uses an established digest library.
Missing/ambiguous/stale inputs, unresolved dependencies, unsupported required
capabilities or excess context refuse dispatch with a reason. Read-only tasks
may have empty writes; this grants no edit permission.

Acquire required read/write leases atomically: reads share; writes exclude
writes and required reads. Confirm process end before release. Keep concurrent
disjoint tasks in the same folder, without a second checkout/build cache.
Unmanaged user edits can still occur; changed captured inputs invalidate evidence.
Targeted questions record path, symbol, digest and matching method within the
packet's budget. No match means uncertainty. Follow-up evidence does not mutate
the packet. New attempts get new packets, retaining links to prior evidence and
intentional prior-attempt edits. Unreported model usage/cost stays unknown;
enforceable byte/result/turn limits are distinct from token estimates.

Retain the newer executor boundary honestly: file-tool rules and reported-commit
scope checks, with unrestricted shell side effects outside OS containment.
A task requiring unsupported stronger isolation is blocked. Do not claim that
a prompt or tool permission flag contains every shell filesystem/network effect.

### Verification, critic and acceptance

Runs stores structured check identity/revision, executable and arguments, cwd,
nonsecret environment references, declared input/output scopes, timeout,
expected result and acceptance associations. Run through shadows-process;
record bounded redacted output, exit/timeout/cancel and artifact digests.

Baseline/post checks use the same folder under input/output leases. Record
candidate commit and captured inputs. Unexpected input changes are inconclusive.
Compare pass, regression, pre-existing failure, fixed or unknown only on
comparable evidence; equal nonzero exit codes do not prove equal failures.
If a tool cannot identify stable failures, retain inconclusive attribution.
If a new task creates the test/check itself, record baseline-unavailable rather
than inventing an earlier result. A passing post-check is still evidence; a
failing one cannot be excused as pre-existing without a comparable baseline.

Keep Committed as implementation outcome. Checks, review and acceptance are
separate durable records tied to the attempt/result digest. Dispatch a critic
after checks settle, with the exact diff, contracts, evidence and report, not
the executor conversation. Critic has no arbitrary shell/edit tools; checked
commands are run by Runs. Deduplicate review by attempt/result/policy.
Each acceptance item has evidence and met/not-met/pending status. Overall
review is PASS, PASS_WITH_NOTES, FAIL_IMPLEMENTATION or FAIL_PLAN. Failed checks
cannot be erased by a critic claim; absent review is not PASS.

Required checks plus qualifying review establish acceptance. PASS_WITH_NOTES
defaults to a person decision. Pending items pin a task/check and durable rerun
trigger. Dependants proceed only if their required produced contracts have
passed; the task is not displayed as fully verified while items remain pending.
Prevent dependency/recheck deadlocks. Legacy done marks stay labeled manual.
Old Frozen plans remain readable; rich execution uses a new approved version or
explicit inspected adoption. Distinguish automated, browser, OS and provider proof.

### Bounded repair, gates and recovery

Use the approved executors ladder: initial attempt, retry in the same session,
Planner investigation, directed third attempt, then stop for Mohammed. Runs
owns counters and dispatch, so tools cannot authorize a fourth attempt.
Problem records retain evidence, affected contracts and Planner direction.
Use Plans::owner_thread; a busy owner waits durably, a missing owner stops with
a reason. Do not introduce the later manager system or invent cost measurements.

FAIL_PLAN proposes a Draft changing affected tasks. Runs/Plans coordinate
stopping attempts of that plan and blocking starts; affected cross-plan
consumers are blocked without rewriting their frozen pins. Approval never
restarts work automatically. Continue the designated executor session only
when it still exists; lost sessions remain explicit recovery cases.
Keep the dependency one-way: Plans records the execution hold with the Draft
transition and publishes its durable event; Runs observes it, stops attempts,
then acknowledges the hold through Plans' public API. New Starts check that
hold in their serialized admission. Draft edits/approval wait until affected
turns have confirmed their end. A restart resumes this handshake; no
Plans-to-Runs reference or best-effort event alone provides the barrier.

Person gates bind CONTINUE/REPLAN/PATCH/ROLLBACK/ABORT to exact scope/evidence.
CONTINUE does not erase a failed required check. PATCH is new bounded work.
ROLLBACK proposes compensating work, never automatic reset/deletion or an
assumption that database changes are reversible. Follow section 24's local
trust boundary: task tools cannot approve, but same-user shell access to HTTP
is not claimed impossible. Stronger person-origin proof is a distinct prerequisite
when an operation requires it; an actor field is not proof by itself.

Recovery reconciles attempts/checks/reviews before further dispatch. Lost
responses cannot duplicate review or acceptance. Unconfirmed process end holds
leases. Preserve evidence, interruption and unknown outcomes through restart.

### Resolved builder questions

The independent builder check raised five material choices. The following are
the controller's proposed resolutions for written-design approval:

1. **Preparation authorization.** Use attributed conversation authorization,
   not a mandatory new confirmation button. The person must have agreed to
   framework preparation at the exact destination; choosing a technology in
   isolation is not a generic command grant. Record the source person message,
   stack/version, destination and preparation operation. The Planner issues the
   scoped preparation request in its own name under section 24. A replay uses
   that immutable request; a changed destination or operation is a new request.
   Inspect/adopt an existing scaffold without running the installer again.
   An ambiguous request pauses preparation, not read-only discovery/planning.

2. **Repair-session lifetime.** Retain the executor session idle through checks
   and review, closing it on terminal acceptance, Stop, escalation or loss of
   the live session. Its stable session credential has no active mutation
   binding after the turn ends. Runs uses Grants' public transaction method to
   bind the next attempt/packet only while admitting that retry. Every new
   executor mutation tool requires attempt identity and packet digest, checked
   against the active binding. Late reports from a retired attempt are refused;
   changing active attempts never relabels an old report as a new one. This
   avoids assuming that an adapter can replace its MCP token inside a session.
   Old slice-1 sessions retain their current close-on-release behavior until
   the rich repair path is introduced and tested. Lost sessions require an
   explicit recovery/new session, never a claim of preserved native history.

3. **Contract compatibility.** A produced operation has one canonical input,
   output and error shape, expressed with JSON Schema and optional
   language-qualified type names. Use a maintained schema library for schema
   validation. Consumers reference that exact operation revision and use its
   shapes; they do not independently override them. Existing shared OpenAPI
   operations retain their current authority and exact operation/version pins.
   If a separate expected shape is declared, require an exact canonical shape
   fingerprint, conservatively; no inferred subtyping, coercion or semantic
   equivalence is claimed. A different representation needs an explicitly
   planned adapter/contract change. Missing pins or unequal declared shapes
   block approval/readiness. Checking that the actual PHP/Rust implementation
   satisfies that shape belongs to executable contract tests and review.

4. **Output-specific acceptance.** Each produced contract declares the task's
   acceptance-item identities and required check identities that establish it.
   At least one acceptance item and one deterministic check are required for
   an automatically accepted output. The declared checks must pass and the
   critic must mark every associated item met against this result digest.
   No pending/failed item can establish that output. Other task items may stay
   pending without withholding an independently established output. An
   inspected existing implementation uses the same evidence association;
   historical manual marks cannot substitute. Plans validates the combined
   task/phase/output/check readiness graph, including cycles such as an output
   whose proof waits for the consumer that needs it. Split producer conformance
   checks from later consumer integration checks to resolve such a cycle.

5. **Source lifetime for the critic.** Before execution leases release, Runs
   captures bounded relevant source bytes for declared reads/writes, candidate
   diff and check artifacts, with digests and source identity. Persist them as
   immutable run artifacts in Core's data storage, not a second checkout or
   build cache in the person's project. The critic reads captured source via
   a Runs operation, not live project Read/Bash tools. Later tasks may edit the
   live files after capture without changing what was reviewed. A missing
   required artifact makes review incomplete; supplying it requires a new
   attributable capture/review and refreshed checks when source equivalence
   cannot be established. Retention keeps artifacts referenced by acceptance,
   pending repair or a person gate. Redacted/missing secret source is not
   silently presented as complete evidence.

### Delivery, builder choices and approval

Eight runnable slices: L1 inventory/LSP, L2 preparation, L3 contracts/decisions,
L4 packets/readiness, L5 checks/evidence, L6 critic/acceptance, L7 repair/pending
checks/gates, L8 real local acceptance. L2 and L3 depend on L1; L4 needs both;
then deliver in order. Each slice gets exact backend/frontend task plans,
accepted shared requests, focused tests, browser/log acceptance and one branch
review/full gate. Commit each verified task separately.

The controller proposes the preparation worker, existing-owner allocation,
exact paths, conservative leases and separate evidence records. These choices
await written-design approval. Builders choose concrete naming, storage indexes,
pagination limits and UI placement within current views, not new ownership or
authority. This program does not add a new workspace or another Cargo cache.

Prove existing/new Laravel discovery, producer/consumer contracts, missing/stale
symbols, refused broad writes, check/critic/plan failures, pending reruns,
bounded repair and interruption during each lifecycle. Report automated tests,
browser fixtures and real harness/OS evidence separately. Detailed implementation
plans follow written-design approval; no product code is authorized by this draft.

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

These are this section's two delivery slices, not the numbering of the
project-planning roadmap. That roadmap schedules this owner at its execution
stage; specialized diagrams are not a prerequisite.

**Not here:** selecting multiple executor vendors, running overlapping tasks
in one working directory, worktree management, automatic waiting-test
scheduling, project teams, remote execution, and general-purpose semantic
memory. These need separate designs. (Amended 2026-11-08: concurrent tasks in one folder are
owned by the [executors design](../../backend/specs/2026-11-08-executors-design.md), D2, D3.) In particular, parallel execution stays
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

The [executors design](../../backend/specs/2026-11-08-executors-design.md) supersedes "serial" below (2026-11-08): independent
tasks with disjoint write scopes run together. The first execution stage is serial. Readiness is checked immediately before a
run using the frozen version, current project binding, task dependencies, and
write scope. If two tasks declare intersecting write paths, they are not
eligible to run concurrently until a later design defines a safe conflict
policy.

## 17.5 Bounded execution packet

The packet replaces repeated project discovery. It gives the executor the
local purpose and approved obligations needed to implement one task, not the
whole plan, sibling tasks, or the Planner's conversation history. A compact
ownership overview explains where this task fits without copying other scopes.

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

Locations pair a repository-relative path and symbol with the captured source
revision and file digest; line numbers are hints at that revision. For a new
symbol, the packet names the owning module and insertion responsibility.
The executor validates the surrounding declaration before editing. Stale or
ambiguous locations require resolution, never a guessed edit at the old line.

The compiler refuses a mutable or unapproved plan, an unready task, unresolved
dependencies, a missing or ambiguous project binding, paths outside the
project, invalid scope patterns, unavailable required inputs, and context over
budget. It reads from one captured repository revision; if that revision
changes during compilation, it refuses rather than mixing snapshots.

The packet is persisted or referenced immutably by its digest. A retry uses the
same packet only when it is the same logical run; a new attempt compiles and
records a new packet. Provider-native conversation history is not canonical
task context.

### Focused questions during a run

Within its declared reads, capabilities and budget, an executor may ask for a
named definition, relevant references or a bounded outline without escalating
each lookup. It opens the relevant source region, widening to necessary types,
invariants and tests when the question requires them. It must understand its
change; a narrow packet is not permission to guess. Repository-wide discovery
is not the default response to a missing answer.

§15's index answers locations, not code. Name-matched references remain
labelled as such; they are not a proven call graph. Semantic LSP results may
be used when an adapter supplies them with provenance, but no LSP integration
is claimed by this draft. No match, incomplete indexing or ambiguity means
uncertainty, not proof that an implementation is absent.

Additional reads must resolve against the packet's captured source, or the
attempt's recorded edits to it. A live index hit is only a candidate until
checked against that source. Each supplied result records the question,
path/symbol, source digest and bounded response in the run's evidence; it does
not mutate the packet. If the source cannot be reconciled, stop for refresh.
Expanding declared reads, write scope or capability requires a newly approved
task version and packet; a manager reply cannot silently expand them.

The context budget includes tool results, repeated turns and inherited harness
instructions/history, not just the initial packet. An adapter must expose how
these inputs are supplied and measured; unavailable usage is labelled unknown.
On budget exhaustion or a missing out-of-scope dependency, return a bounded
problem report instead of crawling the repository. Read minimization is a
context policy, not a claim of OS-level read isolation.

### Problems returned to the controller

A report names task/attempt, packet and agreement versions, the precise
obstacle, evidence, attempted remedies, known affected dependencies and a
proposed action if available. The first serial release returns it to the
person/Planner. A later agentic manager can investigate and direct repairs
within delegated authority; it cannot alter frozen obligations or certify its
own proposed plan change. §17.4 and §18 retain approval ownership.

> **OPEN — manager authority and recovery.** Before an autonomous manager
> dispatches repairs, accepts escalations or controls runs, its owner spec must
> define allowed actions, cost/retry limits, durable problem identities,
> stop/resume behavior and escalation to Planner/person. The role direction is
> in `docs/core/vision.md` §5. This does not block serial execution with a person
> controlling it; no manager service is introduced by this section.

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

§19 owns the Dashboard-selected critic profile and durable post-implementation
dispatch rule. Its review binds to this attempt's exact result; a later repair
does not inherit the earlier verdict. Profile configuration does not change
the verification or person-authority rules in this section.

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
11. A task starts from bounded context without the parent transcript; a needed
    in-scope lookup reads targeted source and records its provenance.
    Ambiguous/name-only results are not presented as semantic proof; a moved
    symbol cannot direct an edit to a stale line.
12. An out-of-scope question, unreconciled source or exhausted context budget
    returns a problem report without silently widening access or changing the
    packet. A fresh attempt retains the previous evidence.

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
- **`docs/core/vision.md`:** Today lines and links to the implemented stages;
- **`docs/core/future.md`:** a stage's row leaves the list once it is delivered.

This file owns the design in §17. Other documents link here instead of copying
its decisions.

> **OPEN — person authority for decisions and gates.** The current local HTTP
> boundary does not prove that a request came from the person rather than a
> process running as the same user (§13.2). **Trigger:** before a decision can
> become `Accepted` or `Superseded`, or a gate action is enabled. Until then,
> agents may propose decisions, but the person-only transitions and gate actions
> stay unavailable. This does not block task-contract or packet work.

## 17.12 Production operations extension

Product direction confirmed 2026-10-10; detailed implementation remains draft.
This section owns the future production action lifecycle. It extends execution
and verification rather than creating another executor. §1.7 owns workspace and
worker placement; §24.6 owns external confirmation. Competition participation
does not define or narrow this capability.

### Environments and resources

A project can have Development, Staging and Production environments independently
of personal/team workspace ownership. Each environment binds explicit provider,
account, region, resource identities, deployed revision and operating policy.
Deploying an application does not migrate its Shadows workspace to the cloud.
Application resources are separate from resources hosting Shadows or NYLRA.

AWS is the first intended integration. Resource selection follows inspected
application needs and measured workload: RDS for a required managed relational
database, S3 for object storage when needed, plus suitable compute and observability.
No mode switch automatically provisions a fixed AWS bundle. Existing resources
are discovered and associated explicitly; tags/names alone do not grant ownership.
Infrastructure code belongs to the target application's repository, with reviewed
changes and resource replacement effects visible before execution.

### Observe, diagnose, propose, act, verify

1. Collect scoped metrics, logs, traces, deployment changes and available usage
   evidence. Preserve timestamps, windows, units, source references and missing
   data. Application latency/errors and dependencies matter alongside CPU/load.
2. Correlate a sustained symptom with the project's topology and recent changes.
   A high CPU alarm is a symptom, not proof that scaling is the correct remedy.
   Distinguish observed facts, suspected causes and confidence; gather more evidence
   when a bottleneck could be application code, queries, traffic or a dependency.
3. Propose a bounded action: resource targets, expected benefit, alternatives,
   cost assumptions, prerequisites, risks, observation window and recovery path.
   Options include scale out/in, a configuration change, a code fix, rollback,
   investigation or no action. Resource-specific constraints remain explicit.
4. Obtain exact human confirmation or match a previously explicit, bounded policy.
   An inferred preference or generated recommendation is not an execution grant.
5. Revalidate current resource state and authority, execute through an assigned
   worker, and record the provider's operation identity. A timeout means unknown
   outcome until reconciliation, not permission to repeat a non-idempotent action.
6. Verify both provider state and application behavior over the declared window.
   Report success, failure, no demonstrated improvement or insufficient evidence.
   Recovery itself follows its approved boundary. Database/data changes are not
   assumed reversible merely because an application release can be rolled back.

The action records its evidence basis, proposal, approver/policy, attempt and
observed result. Implementation chooses exact states under this lifecycle; no
new service/module exists until its first runnable operation needs it.

### Authority and continuous operation

Observation and mutation use separate capabilities. Scope each to account,
environment and resources. Mutations require explicit allowed actions, capacity
and spending bounds, expiry and cooldown. Automatic policies must include a stop
condition and escalation after ineffective attempts; avoid scale-up/scale-down
oscillation. Native autoscaling remains authoritative for its configured loop;
Shadows must inspect and coordinate with it, not compete with another controller.
Destructive actions and policy-boundary changes need separate explicit approval.

Continuous monitoring runs on an explicitly assigned always-on collector. For a
personal local owner, an optional cloud collector may observe and send scoped
alerts while the computer is off, but cannot adopt new plans or execute owner
commands. Owner-dependent diagnosis may wait; cached advice shows its age.
Unattended remediation requires an always-on authoritative runtime or a separately
specified durable grant under §1.7; a cloud NYLRA conversation is not such a grant.

NYLRA explains the incident and action, asks the person when required, and reports
observed outcomes. It never directly acquires unrestricted AWS execution power.
Its message should answer what changed, which service is affected, why the proposed
action helps, what it costs under stated assumptions, and whether it actually helped.

### Delivery and proof

Start with read-only observation for one explicit environment and a sourced NYLRA
incident explanation. Then prove one reversible operation in a disposable test
environment with confirmation and post-action checks. Expand to reviewed automated
policies and resource types only after that path works. Production is a future
product extension, not a prerequisite for completing the local roadmap.

> **OPEN — operational implementation.** Close before live mutation by specifying
> provider adapters, resource inventory, least-privilege boundaries, durable incident
> ownership, always-on collector lifecycle, action deduplication/reconciliation,
> cost estimation freshness, policy limits and recovery. Acceptance must cover stale
> evidence, revoked authority, duplicate alerts, ambiguous provider timeouts,
> competing autoscaling, lack of improvement and loss of the owning runtime.
> A successful demo deployment does not establish production readiness.
