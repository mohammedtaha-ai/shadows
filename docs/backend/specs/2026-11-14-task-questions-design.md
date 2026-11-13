# Slice 4: attempt context, task questions and parallel work

Status: draft for Mohammed's approval, 2026-11-14. No implementation is authorized by
this draft. The decisions in the next section were settled in the conversation; the
remaining choices are proposals for this revision.

This is the proposed slice 4 revision of [17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09).
It builds on the shipped [executor design](2026-11-08-executors-design.md) and
[language-server design](2026-11-13-language-servers-design.md). The
[delivery program](../plans/2026-11-09-local-delivery-program.md) owns ordering. The
old [execution-briefs plan](../plans/2026-11-09-execution-briefs.md) remains draft
material, not an executable plan for this revision.

## Decisions settled with Mohammed

1. A task names the exact work: its location, files to read and change, functions,
   arguments, returns and behavior. It identifies the outputs it uses and produces.
2. An executor asks Shadows whether a function exists, what it returns and which code
   or planned task consumes its result. Shadows answers from code and task contracts.
3. A contract conflict stops that task's attempt and notifies the Planner. Other tasks
   continue. The Planner decides which other tasks are affected, stops those tasks and
   corrects the plan. There is no automatic cancellation cascade, including when that
   correction creates a new Draft. This replaces the executor design's earlier whole-plan
   stop decision; its owner paragraph is amended with the same ruling.
4. Independent tasks may run together. Two tasks with overlapping write scopes do not:
   the first admitted task runs, and Start refuses the other with `overlap`. A task's
   scope stays held while its committed work waits for judgment.
5. The Planner should start compatible independent tasks together, rather than being
   instructed to follow only one `next` task after each done mark.

## Why

The executor already receives its task and resolved producer contracts through
`task_context`, looks at code through `code_lsp`, and reports its commit with
`task_report`. The missing question is the connection between those facts: a language
server cannot describe a consumer that exists only in an approved plan.

The task and its version are already immutable. Attempts already retain their model,
base commit, addition, report and resulting commit. Slice 4 must preserve that work.
What is missing is a recorded context for the attempt, including the layout and
standards supplied to it, and a way to ask and escalate against that context.

## Current code inspected

Baseline: `1efdf8d4`, before product changes on `feature/slice-4-task-questions`.

| Entry | What it does today |
|---|---|
| `crates/shadows-core/src/app.rs` | Wires Runs, Plans, Codebase, Harness and Turns once. |
| `core-runs/src/start.rs` | Checks plan/dependencies/git/dirty writes, records the attempt, starts its turn and watches it in the background. |
| `core-runs/src/store/mod.rs` | Serializes replay, held-scope checks and insertion. It does not recheck plan eligibility inside that write. |
| `core-plans/src/store/task_context.rs` | Reads a Frozen task, contracts and the current layout/standards in one read snapshot, without storing an attempt snapshot. |
| `core-runs/src/planner.rs` | Returns one never-attempted task as `next`; explicit Starts can already leave independent attempts running together. |
| `core-runs/src/notify.rs` | Sends an attempt's outcome to its plan's Planner through Turns' waiting notices. |
| `core-runs/src/stop.rs` | Stops an attempt and releases its executor session and held scope. |
| `core-grants/src/tools.rs` | Gives executors `task_context`, `code_lsp` and `task_report`. |

Paths in the table after the first are under `crates/`. These are inspected facts,
not acceptance evidence for this design.

## Goals

- Give an attempt one stable task context that it can read again and the person can inspect.
- Answer task questions using both real code and explicit producer/consumer contracts.
- Escalate a contract conflict durably without stopping unrelated work.
- Let the Planner discover and start a compatible group of independent tasks.
- Refuse known invalid required inputs and stale plan admission before work starts.

## Non-goals and failure conditions

- No new AI answerer, scheduler daemon, service, workspace, build cache or dependency graph.
- No parent conversation copied into an executor's brief; no source-copy store or packet hash.
- No universal comparison of arbitrary language signatures or return-type strings.
- No new restriction that makes `code_lsp` refuse legitimate project exploration outside
  a task's `reads`. Existing project/grant/path boundaries remain.
- No automatic edits of Frozen contracts, task acceptance, retries or dependent-task stops.
- A planned output must never be presented as implemented code. An unavailable server or
  incomplete search must never be reported as proof that a function or consumer is absent.
- An acknowledged conflict or its Planner notice must not disappear on restart.
- Recording a conflict must not release a scope while its executor can still write.

## Approaches

### Executor combines the two answers

Keep only `task_context` and `code_lsp`, and ask the executor to correlate producers,
consumers and code itself. This uses fewer product operations, but leaves the connection
the person asked Shadows to supply to each model. Planned consumers are still a separate
discovery job, and conflict reports have no structured delivery path.

### Runs answers in the attempt's context — proposed

Runs reads the attempt's saved brief, asks Plans for contract relationships and asks
Codebase for source facts. It returns each source distinctly. A separate conflict command
records evidence and initiates the existing stop/notice lifecycle. This fits the requested
behavior and existing dependency direction. `code_lsp` remains available for ordinary code
exploration; task questions add the planning facts it cannot supply.

## Ownership

| Owner | Responsibility |
|---|---|
| Runs | Attempt brief, joining task answers, conflict record, conflict stop/delivery, ready-group selection and Planner-authorized stops. |
| Plans | Approved task context and producer/consumer references, including reverse references derived from its stored contracts; transaction-local plan eligibility. |
| Codebase | Real files, canonical containment, language-server questions and the completeness of their answers. |
| Harness / Grants | Session policy and authenticated tool reach. |
| Turns / Threads / Operations | Their existing turn, queued-notice, entry and operation lifecycles. |
| HTTP / MCP / web | Translate operations and display Core's results. |

Runs already depends on the owners it needs. Plans and Codebase do not depend on Runs.
Plans supplies one context assembler for both current task reads and snapshot capture;
the task-contract rules are not duplicated in Runs. Stores stay private; shared writes
use each owner's transaction-local methods.

## One brief per attempt

Proposed operation: `task_brief`, an executor read whose attempt is obtained from its
authenticated grant and thread, never a client-selected attempt id.

- Reuse `TaskContext` for the approved task, resolved contracts, selected parts and path
  ownership, standards versions and map revision. Store that supplied context once, keyed
  by the existing attempt id. Existing attempt fields remain the source of model, base
  commit, addition, plan/version/task identity and timestamps.
- Capture the context in the same admission transaction as a new attempt, after rechecking
  plan eligibility. A failed or replayed Start must not leave another brief.
- A fresh Start and each addition create a new attempt and a new context. The preceding
  attempt's context never changes. The addition remains recorded in its existing field.
- Repeated `task_brief` calls return the saved context, even if the project map or standards
  later change. The executor's first prompt and instructions require this read.
- Keep `task_context` for Planner/external plan inspection. Replace it with `task_brief`
  in the executor's grant so an executor cannot accidentally substitute a fresh layout
  read for the context its attempt received.
- Historical attempts without a saved context are explicitly `context_not_recorded`.
  Do not synthesize a historical brief from today's layout. A new addition to such an
  attempt captures its own new context normally.
- The person can read the same brief from the attempt page; copied thread history does
  not confer authority to read or mutate another attempt.

The brief records the task context supplied by Shadows. It is not a claim to capture
every byte of the provider's conversation, tools or internal model state.

## Task questions

Proposed operation: `task_question`, an executor read bound to its current Running attempt.
The detailed plan will define typed selectors for contract lookup, producer lookup,
consumer lookup and an optional native LSP question. Backend code does not interpret an
arbitrary natural-language question or ask another model to answer it.

An answer separates:

1. **Code facts:** the language server's result and project-relative locations, or a clear
   unavailable/incomplete answer. Native LSP operation semantics stay with Codebase.
2. **Planning facts:** the relevant approved symbol/output contract, producer and consumer
   references, and whether the associated task is done or still pending. A done mark and
   a recorded attempt commit are identified separately; neither proves that an LSP lookup
   found the symbol.
3. **Uncertainty:** an unresolved reference, missing contract detail, unavailable server,
   ambiguous target, truncated answer or an unrecorded consumer relationship.

Planned producers and consumers are derived from `produces` and `uses` by Plans in the
attempt's exact Frozen version. Current `ContractTarget` supports local tasks only. That
version identity is already retained by the attempt; later plans and later versions do
not enter its planned-consumer answer. Do not save a second dependency graph or copy all
sibling tasks into the brief. The answer explicitly limits its completeness claim to
that version. Consumers in another plan are not yet represented by these contracts; an
empty result does not mean the project has no other consumers. Broader planned-consumer
discovery waits for explicit cross-plan contract references and a real user of them.

Results from live code and live attempt/done states can differ from the approved
promises; show their current observation separately from the saved context. State a
consumer that the plan records and a caller that the language server finds distinctly.

The question may inspect code within the grant's existing project/link reach, including
files outside `reads`. It cannot discover unrelated projects, secret file contents or
paths escaping through symlinks/junctions. Answers are bounded, with visible truncation;
large result sets must not silently become a claim that no other consumers exist.

Example: the task expects `findCustomer(id) -> Customer`, but code lookup exposes
`findCustomer(email) -> Option<Customer>`. Shadows returns the expected contract and the
observed declaration. The executor reports why that difference prevents its work; Rust
code in Shadows does not attempt general semantic equivalence across languages.

## Contract conflicts

Proposed operation: `task_conflict`, a mutating executor command. It is separate from the
read-only question and from a successful `task_report`.

- It identifies the expected contract or missing instruction, the observed fact, the
  affected project-relative paths/symbols and bounded evidence. It refers to the current
  attempt and its brief; a caller cannot nominate another executor's attempt.
- Validate grant, current attempt and evidence bounds before accepting it. Persist the
  conflict, command replay identity and durable delivery intent in one owner transaction.
  One accepted conflict ends this attempt's work; repeats replay, and contradictory input
  under the same command id is `CommandConflict`.
- Acceptance prevents a later commit report from racing past the conflict. The report
  write and conflict write arbitrate inside their transactions: one wins; the other
  returns the already accepted outcome. A committed attempt is not converted into a
  conflict-stopped attempt by a late tool call.
- The tool answers promptly. Runs then requests termination through Turns outside the
  tool handler's synchronous call chain, so stopping the same executor cannot deadlock
  the response. Keep the scope until the turn/session can no longer write. If stopping
  fails, preserve the held scope and surface that failure; do not claim the task stopped.
- Use the existing Stopped outcome with a contract-conflict reason and its associated
  conflict record; no general pause/resume state machine is introduced.
- The plan's actual Planner receives the conflict through the existing executor-attributed
  waiting-notice path. It remains an executor's report, never a person's approval message.
  Persist the delivery intent before acknowledging the conflict and retry delivery with
  one stable conflict-derived command id and frozen enqueue payload: retries must not
  change the command's parameters by rereading newer Planner model/mode settings. The
  watcher must not append a second generic
  "ended without a commit" notice for the same conflict.
- Restart preserves the conflict and completes the pending stop/recovery and notice work;
  it never restarts the executor. If the Planner conversation was removed or no Planner
  owns the plan, retain the undeliverable conflict visibly on the attempt for the person.
- Keep changes already left in the working tree. Do not silently reset, delete or commit
  them. Existing dirty-scope checks continue to govern the next Start.

The Planner decides the effect on other tasks. Runs does not stop siblings or traverse
dependency links to cancel them automatically. Existing `needs` readiness still prevents
starting a task whose required predecessor is not done.

Add a Planner `task_stop` operation over Runs' existing Stop: same-project Thread grant,
exact attempt identity, confirmed stop and the existing held-scope rules. It gives the
Planner the operation needed to stop affected tasks; a prompt alone cannot provide it.

After reviewing a conflict, the Planner may keep independent work running, explicitly
stop affected attempts, and correct the plan. Changing a Frozen contract requires a new
version and normal approval. Creating that Draft does not itself stop unaffected running
attempts. They report against their original version, and the Planner judges and marks
that work against the same original contract, even after a replacement version is approved.
Do not transfer their attempts or done marks to the new version automatically. The Planner
accounts for already-implemented outputs when writing the corrected plan. New Starts use
the current eligible approved version; a newer Draft still prevents new Starts on the
superseded version. This changes the old cancellation policy, not Frozen history.

A stopped, uncommitted attempt is not a valid target for
`task_amend`, which currently continues a Committed attempt. Corrections begin a new
attempt after the appropriate contract is approved. No automatic retry is added.

## Parallel groups

Runs continues to admit one Start at a time through its existing transaction; it does
not need a batch-start engine. Proposed `task_ready` is a Planner read for one exact
Frozen plan version. After a done mark, the existing `next` answer becomes a group using
the same selection method.

- Consider never-attempted, not-done tasks in task-number order, retaining today's
  no-automatic-retry rule. Check dependencies, inputs, plan state and already-held scopes.
- Build a deterministic compatible group using the existing conservative write matcher.
  A candidate must also be disjoint from every task already selected in that group.
- Return task identities and their suggested models, with stable ordering. This is an
  observed ready group, not a reservation; Start always revalidates admission.
- When the person asks to run the plan, the Planner asks for the first group and starts
  its tasks without waiting for one executor to finish. A request for one named task
  still starts only that task. After a done mark it starts the next returned group.
- A failure or confirmed stop can free a scope without any done mark. While carrying out
  an existing request to run the plan, the Planner may ask `task_ready` again and continue
  never-attempted independent work after reviewing that outcome. For a contract conflict,
  assess its impact first and explicitly stop affected work before choosing another group.
  A failed task itself is never automatically retried. A new Draft still blocks new Starts
  on the old version; after correction, normal approval is required for the new version.
- A Start refused because state changed does not undo already-started independent tasks.
  The Planner reports/handles the refusal and asks for current readiness instead of
  looping the same stale list. No task that already ran is automatically retried.
- A conflicting candidate stays eligible for a later group after held scopes are freed.
  Two different functions in the same file still overlap. Do not introduce range locks.
- No arbitrary new concurrency setting or global cross-plan scheduler is introduced.
  The group concerns the named plan; existing Starts in other plans still participate
  in the project's held-scope check.

## Admission and required inputs

Plan archival, Frozen state, supersession/newer Draft and required done marks must be
rechecked through the owning services inside the attempt-admission transaction, alongside
the held-scope checks. Replay remains first, before current-state refusals.

Required existing inputs are the exact files a rich task contract names as read, modify
or delete, and its explicit existing-source references. Canonical path/grant checks apply
before inspecting them. A create target need not exist. A broad `reads` glob is a permitted
search area, not a claim that every possible match must exist. Legacy tasks without rich
contracts remain supported and expose that limitation.

Do not infer a missing function from empty/incomplete LSP results or introduce a universal
signature validator at Start. File existence is an admission check; unresolved symbols and
contract mismatches use the question/conflict path. This is a proposed refinement of
17.0's earlier blanket wording about missing required symbols.

Filesystem observations describe when the check ran; they do not create read locks or
prevent an external process from changing an input afterwards. A later obstacle can still
be reported as a conflict.

## What the person sees

The attempt page can open its saved brief: approved task and version, contracts and their
producers/consumers, layout/standards versions, base commit, model and addition. Historical
attempts without a snapshot show that fact rather than a reconstructed answer.

A conflict appears on that attempt and in the Planner's conversation: expected behavior,
observed fact, evidence, stop outcome and delivery state. Core supplies those states;
the client does not decide which other tasks must stop. Retain source/plan references so
the person and Planner can inspect the claim.

The implementation plan must first reconcile the shared API/model agreement and define
the brief/conflict reads and the changed `next` wire shape. Generate OpenAPI and the web
schema from the implementation. Current contracts are not rewritten to claim this exists.

## Acceptance

Use real temporary repositories and the installed language servers; use live adapters for
provider acceptance. Keep automated checks distinct from browser/provider evidence.

1. Repeated reads within one attempt preserve its context after map/standards changes;
   an addition gets its own context, and a replay creates no second record.
2. A question identifies an existing function and an unimplemented approved consumer,
   with distinct code and planning provenance. Missing/incomplete servers do not invent
   absence. Check project, linked-project, path and secret boundaries.
3. A reported contract mismatch stops only its attempt and reaches a busy Planner once.
   An independent executor continues, including across the corrective Draft and approval,
   and is judged against its original contract. The Planner can explicitly stop an affected
   task. No old attempt or done mark is silently copied to the new version.
4. Exercise conflict versus commit-report races, failed termination, disconnect, restart
   before notice delivery and removed Planner threads. A held scope is never freed while
   the executor may write, and no conflict/notice is silently lost or duplicated.
5. A real first ready group starts two independent tasks before either finishes. Two
   overlapping candidates do not run together; a scope waiting for judgment still blocks.
   A conflict-stop frees a scope without a done mark; after impact review, the Planner
   can discover a previously withheld, unaffected task without retrying the failed one.
6. A plan/Draft change racing Start cannot admit work from the old eligible observation.
   Missing existing inputs refuse admission; an intended new file and unrelated dirty
   paths do not. Invalid inputs write no attempt or brief.
7. The browser shows the saved brief and conflict on a disposable project. The Planner
   receives the conflict, leaves unrelated work running, and corrects the affected work
   through normal plan approval before another attempt.

## Left to the implementation plan and builder

The implementation plan must name exact public types/signatures, normalized command
fingerprints, private stores/migrations, caller changes, grants, HTTP/MCP payloads,
frontend reads/events, focused tests and one commit per independently verified task.
It must specify bounded payload sizes and retry/cancellation behavior using existing
owner patterns, including how the conflict survives the stop/report race.

The builder chooses function bodies and appropriate infrastructure calls. No product
behavior above, new authority boundary or new owner is left for it to invent.
