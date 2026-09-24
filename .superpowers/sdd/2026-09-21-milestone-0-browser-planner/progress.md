# SDD ledger — plan: docs/superpowers/plans/2026-09-21-milestone-0-browser-planner.md

Branch: milestone-0/product-path (Tasks 5+). Tasks 1-4 were done on
milestone-0/browser-planner (branched from a46b921 on main) and are merged into
main at 68dc2ff via PR #1; milestone-0/product-path branched from there.
Spec: docs/superpowers/specs/README.md (read — binding authority)
Implementer model: sonnet (user's instruction)

## Pre-flight scan

Every pair of tasks sharing a file or an interface, plus each task against itself.

| Pair | Shared file / interface | Finding |
|---|---|---|
| 1 -> 2 | `Config` produced / consumed | clean (`db_path: PathBuf` -> `&Path`) |
| 2 -> 3 | `src/storage/sqlite/mod.rs`; `Storage` | clean (T3 adds `write_txn` to T2's facade) |
| 3 -> 4,5,6,9 | private `append_event` | clean, with a note: `events.rs` is `storage::sqlite::events`, its callers are siblings, so "private" must be `pub(super)`, not `pub(self)`. Carried into each dispatch. |
| 4 -> 9 | `RuntimeInstanceId` | clean |
| 5 -> 6 | `pub(super) classify` / `record_command` | clean (same `storage::sqlite` parent) |
| 7 -> 8 | `ProcessSpec`, `ProcessHandle`, `spawn` | clean, signatures match verbatim |
| 8 -> 9 | `AgentInvocation` | not consumed by any T9 signature. Not a conflict: this is the plan's declared gap (agent_invocation is not persisted in M0). |
| 9 -> 10 | `src/storage/sqlite/operation.rs`, `tests/operation_lifecycle.rs` | clean (T10 modifies) |
| 10 -> 11 | `LiveHandles`, `PlannerTurn`, bus type | clean (`broadcast::Sender<(String, StreamItem)>` both sides) |
| 1 -> 11 | `src/cli/mod.rs` | clean (T11 mounts the router inside T1's `serve`) |
| 2,3 | `tests/storage_contract.rs` | clean (additive appends). Tasks 5 and 6 were redirected to `tests/project_contract.rs` and `tests/thread_contract.rs` — see Ruling 19. |
| **11 -> 12** | **`src/protocol/index.html`** | **CONFLICT — see Ruling 1** |

Each task against its own text: clean, except Task 7 and Task 12.

- Task 7 creates `src/bin/tree_probe.rs` while Global Constraints say "a library and a
  single binary". Reconciled, not a conflict: the constraint governs product binaries,
  and the task justifies `tree_probe` in its own text as test apparatus carrying no
  product logic. No ruling needed.
- Task 12: see Ruling 1.

Ruling 1: Task 11 line 3702 compiles `include_str!("index.html")`, but that file is
created by Task 12 — Task 11 cannot build. Task 11's implementer creates
`src/protocol/index.html` as a two-line stub so the crate compiles, and Task 12 replaces
its entire content. Task 12's Step 2 expected failure changes from "the file does not
exist" to "the `<title>Shadows</title>` assertion fails".
Why: the spec requires the page be embedded, so the alternative (serving from disk in
T11) would contradict it, and reordering the tasks would put an untested page before its
routes.
Cost if wrong: a test's failure text differs from the plan's prose for one step.

## Progress

Task 1: implemented by sonnet (agent a5815ab), commit 1e235b3, BASE a46b921.
  Reported DONE; cargo test green, clippy -D warnings clean, fmt clean.
  Implementer flagged: ran `cargo fmt`, which reformatted whitespace in 3 files
  the brief gave as literal blocks. Handed to the reviewer to judge.
Task 1: review dispatched (sonnet) on review-a46b921..1e235b3.diff

Ruling 2 (CI, outside the plan): the user asked whether to open a PR and run CI after
several tasks. Decision — add `.github/workflows/ci.yml` immediately after Task 1's
review passes, not after several tasks, and run it on windows-latest AND ubuntu-latest
from the start with different jobs: Windows is the acceptance gate (fmt + clippy -D
warnings + full test suite), Linux is the COMPILE gate only (fmt + clippy + `cargo test`
with platform-gated tests allowed to be `#[ignore]`d). Milestone 0 claims Windows only
(plan Task 13), so a Linux job that claimed acceptance would be a lie; a Linux job that
does not exist lets Linux rot silently for 13 tasks.
Why now: CI added after six tasks tests six tasks' assumptions at once, and when it goes
red nobody knows which task broke it. After Task 1 there is a crate that builds, which is
all CI needs.
PR cadence: two PRs — the first after Task 4 (already a spec checkpoint, §10.2
"Persistence Foundation Ready"), the second after Task 13. Not one giant PR.
Cost if wrong: a workflow file that needs editing as later tasks add platform-specific
tests — cheap, and visible on every push.

Task 1 review: spec ❌ — one Important finding, labeled plan-mandated. Adjudicated below.

Ruling 3: the reviewer flagged the axum Router + `/health` in `src/cli/mod.rs` against the
constraint "Task 1 must not introduce any HTTP surface". That constraint was MINE, written
into the reviewer's prompt; it is not in the plan and not in the spec. The spec's actual
rule is "Only `protocol/` owns HTTP and SSE types. They do not appear in domain or
application signatures." A `Router` held as a local inside `cli::serve` appears in no
signature, and `cli/` is the composition root, not a domain or application module. Task 11
modifies this exact file to mount the real router. So the `axum::serve` call STANDS — and
it must, or the daemon would bind, print, and exit.
The finding is still half right, on other grounds: `/health` is an endpoint no test
exercises, no client calls, and no spec section asks for. That is YAGNI, and an operational
decision the spec has not made. It goes; `Router::new()` with no routes stays.
Cost if wrong: if a later task wants a health endpoint it re-adds one line, in `protocol/`
where it belongs.

Task 1: minor (deferred): `--harness` default is the bare string "claude"; confirm in
Task 8 that it is not silently turned into a PATH lookup at spawn time.
Task 1: fix round 1/5 dispatched (resumed implementer a5815ab), commit dacd294
  "fix: drop the unspec'd /health route from shadows serve".
  Controller verified independently before the round: `cargo clippy --all-targets
  -- -D warnings` and `cargo fmt --check` both clean — this closes the reviewer's
  one "cannot verify from diff" item.
  Implementer noted the workspace is gitignored so its report was not committed.
  That is correct and by design; not a finding.
  Scoped re-review dispatched (haiku) on review-1e235b3..dacd294.diff.
Task 1: re-review clean — /health removal ADDRESSED, axum::serve retained, no new breakage.
Task 1: complete (commits a46b921..dacd294, review clean after 1 fix round)

CI (Ruling 2) added: .github/workflows/ci.yml. Written by the controller, not a
plan task. Verified by running its exact three commands locally first: fmt clean,
clippy -D warnings clean, cargo test 1/1 passing.

BLOCKER for the PR half of Ruling 2: `git remote -v` is EMPTY. There is no GitHub
remote, so no PR can be opened and CI cannot actually run. Creating a remote
repository is an outward-facing action and is the user's to authorize — surfaced
to them, not decided here. Execution of the plan continues meanwhile; the workflow
file is ready and will run on the first push whenever a remote exists.
Task 2: dispatched (sonnet, agent aa17b4b), BASE d7eae19.
Task 2: implemented by sonnet (agent aa17b4b), commit 9d7f154. Reported DONE.
  Implementer flagged its own `#[allow(dead_code)]` on Storage::write.
Ruling 4: the allow is acceptable in THIS instance and only in it. It is scoped to one
  field (not a module, not a crate), carries a comment naming the task that consumes it,
  and that task is the very next one. The predecessor project's failure was a crate-level
  allow over a 609-line module whose consumer never arrived.
  Condition attached: Task 3's dispatch MUST remove it, and a Task 3 diff that still
  contains it is an Important finding, not an oversight. Carried into Task 3's dispatch.
  Cost if wrong: one suppressed warning lives for exactly one task.
Task 2: review dispatched (sonnet) on review-d7eae19..9d7f154.diff

Remote created (user authorized: name `shadows`, private):
  https://github.com/mohammedtaha-ai/shadows — main and milestone-0/browser-planner pushed.
  First CI run started on the branch push.

Task 2 review: spec ❌ — three Important findings, ALL labeled plan-mandated, i.e. all
three are defects in MY plan, not in the implementer's work. Verified each against the
spec text myself before ruling; the reviewer's citations were accurate.

Ruling 5 (finding 1, `synchronous = NORMAL`): reviewer is RIGHT, finding upheld.
  Spec 6.23 (2026-09-21-sqlite-schema-design.md:1002-1010) names `synchronous` in a list
  of settings not to lock without evidence, and ends "Stay conservative by default."
  NORMAL is less durable than FULL — it is the opposite of conservative — and
  WAL_VALIDATION.md carries it only in the Environment table as the fixed backdrop of the
  Q1/Q2 experiments, never as a result. The plan locked a durability knob on the strength
  of an experiment that was not about it. Drop the `.synchronous(...)` call entirely.
  Cost if wrong: writes are more durable and slightly slower until someone measures it.

Ruling 6 (finding 2, `sqlx` in tests/storage_contract.rs): code STANDS.
  Spec 2.9 (2026-09-21-data-flow-design.md:324) already names storage contract tests as
  one of the mechanisms that establish semantic portability. A contract test asserting
  PRAGMA state is that mechanism working. The reviewer's alternative — adding
  `journal_mode()` accessors to `Storage` — would move SQLite vocabulary out of a test,
  where it is contained, and into the product API, where it is permanent. That is worse.
  The reviewer was right that nothing said so, so 2.9 is AMENDED IN PLACE to say it,
  including the limit: a contract test may read backend state, and may never become the
  reason a backend-specific accessor is added to a product API.
  Cost if wrong: one paragraph in the spec to retract.

Ruling 7 (finding 3, premature `write` field): reviewer is RIGHT, finding upheld, and
  this REPLACES Ruling 4 — which was mine and was the weaker call.
  Task 2's own Produces contract is `open` and `reader`; nothing in it needs a write
  connection. The field existed only to be unused, and the `#[allow(dead_code)]` existed
  only to hide that. The right fix is not a better-scoped suppression — it is removing
  the thing being suppressed. The field arrives in Task 3, with its consumer, suppressing
  nothing.
  Consequence carried forward: Task 3's brief (line 230) USES `self.write` but never
  declares it. Task 3's dispatch must carry the instruction to add the field.
  Cost if wrong: Task 3 adds two lines it would otherwise have inherited.
Task 2: fix round 1/5 (2 addressed, 0 open; commit 93a84ac). Re-review clean.
Task 2: complete (commits d7eae19..93a84ac, review clean after 1 fix round)
  Spec amendment for Ruling 6 committed separately as 661ab80.
Task 2: minor (deferred): the contract test asserts journal_mode and foreign_keys but
  not busy_timeout; and `Storage::reader` returns a raw `sqlx::SqlitePool` from the
  public facade, which will re-open the module-boundary question for the first caller
  outside storage/. Both for the final whole-branch review to triage.

CI: first run green on BOTH jobs — Linux compile gate 2m36s, Windows acceptance 5m01s.
  Run 35642093210.
Task 3: dispatched (sonnet), BASE 661ab80. Carries Ruling 7's consequence: the brief
  uses `self.write` at line 230 but never declares it, so Task 3 must add the field.

Ruling 8 (user-requested, outside the plan): deleted `sandbox/` — both throwaway probe
  crates. SERVE_STREAM_SPIKE.md already promised this in its own header and it had not
  happened. 12 tracked files, 3353 lines, plus ~827MB of untracked build artifacts.
  NOT a straight delete: `sandbox/wal-validation/results.txt` was tracked and cited by
  WAL_VALIDATION.md as its raw output — the measurement behind the writer policy Task 3
  is implementing right now. Deleting it would have left an evidence file pointing at
  nothing, the exact failure the predecessor's `.gitattributes` taught. It moved to
  `docs/evidence/persistence/wal-validation-results.txt` (git recorded a 100% rename).
  Both evidence files now name the commit holding the deleted source.
  CLAUDE.md gains the general rule so the next spike is not re-argued.
  Cost if wrong: `git revert 0834bb7`.

CORRECTION for the Task 3 review package: BASE is 0834bb7, NOT the 661ab80 recorded at
  dispatch. The sandbox deletion landed after Task 3 was dispatched but before it
  committed; using 661ab80 would show the reviewer 3353 lines of deletions as if they
  were Task 3's work.
Task 3: implemented by sonnet (agent af5175c), commit c9288fd. Reported DONE.
  Self-disclosed deviation: `append_event` is `pub(in crate::storage)` not the brief's
  `pub(super)`, because the brief's own `test_support` module is a sibling of `sqlite`.
  Sent to the reviewer WITHOUT a verdict from me, and pointed at the sharper question
  behind it: whether a `test_support` module in the PRODUCT tree that can append an
  event without writing state is itself the hole the "no public raw append_event" rule
  exists to close. That question is bigger than the visibility keyword.
Task 3: review dispatched (sonnet) on review-0834bb7..c9288fd.diff, BASE corrected.

Ruling 9 (user instruction, standing): structure is a continuous check, not a cleanup
  pass. Acted on three ways rather than one.
  (a) CLAUDE.md gains a TRIGGER, not a wish: 300 lines is where a file must state its
      single responsibility, 500 is where it splits, and the split is by responsibility —
      a utils.rs is the same pile renamed.
  (b) The three accretion points are named by name in CLAUDE.md before they accrete:
      storage/mod.rs, protocol/, tests/storage_contract.rs.
  (c) Plan Task 11 AMENDED: protocol/ lands as mod.rs (wiring) + handlers.rs (routes) +
      sse.rs (stream) instead of one ~300-line file. The code in its steps is unchanged,
      only its destination. Doing this now costs an edit; doing it at Task 11 costs a
      rewrite of a file every later milestone touches.
  Measured before deciding: whole tree is 554 lines, largest product file 93. Nothing has
  accreted yet — this is prevention, not repair.
  Cost if wrong: a threshold that annoys, and one extra file in protocol/.

Ruling 10 (process, fixes my own failure mode): reviewer dispatches will no longer carry
  a hand-retyped constraints block. Ruling 3 exists only because I typed a constraint
  from memory that was in neither the plan nor the spec, and a reviewer then correctly
  flagged compliant code against my invention. Future dispatches point reviewers at
  CLAUDE.md and the plan's `## Global Constraints` section directly.
  This is also the user's own rule applied to my process: a retyped constraint block IS
  the same decision recorded twice, and the copy drifts.
  Cost if wrong: reviewers read two files instead of one prompt.

Task 3 review: spec ❌ — six findings, ALL plan-mandated. Adjudicated individually.

Ruling 11 (test_support unconditionally pub): UPHELD. `#[doc(hidden)] pub mod` hides
  docs, not visibility. It ships in every build and appends an event without state —
  the exact hole rule 10 closes. Its doc comment claims it is "not compiled into the
  library for consumers", which is FALSE, and a hole with a comment denying it is worse
  than the hole. Fix: Cargo feature `test-support`, enabled via dev-dependencies so
  `cargo test` needs no flag and `cargo build` does not ship it.

Ruling 12 (write_txn has no panic/cancel safety): UPHELD, and it is the serious one.
  Closure panic or future drop leaves the ONE write connection inside an open
  BEGIN IMMEDIATE; every later write_txn then fails its own BEGIN and all durable
  writes are dead until restart. A pool discards a broken member; one connection has
  nothing to discard. Drop cannot run async SQL, so the fix is recovery-at-entry, which
  covers panic and cancellation with one mechanism. Folded in the discarded ROLLBACK
  error (was Minor) — given this finding, a failed rollback IS the poisoned signal.
  Required a test that leaves a transaction open abnormally and proves the next
  write_txn still works. Without it the fix is a claim.

Ruling 13 (provenance columns never written): UPHELD, merged with the reviewer's ⚠️.
  `causation_kind`/`causation_ref` are on no struct; `correlation_id` is on the struct
  with no builder and absent from the INSERT. Four later tasks would write NULL
  provenance forever into the table whose whole job is provenance. Required causation as
  ONE `Option<Causation>` so the table's paired CHECK cannot be violated from Rust.

Ruling 14 (doc comment overclaims): UPHELD and RAISED from the reviewer's Minor.
  The concurrency test proves the single-connection shape; it does NOT prove
  BEGIN IMMEDIATE, because the mutex already serializes our own writers. The comment
  implying otherwise is a claim the suite does not back — and an overclaiming comment
  on the write path is how the next person stops testing the thing. BEGIN IMMEDIATE is
  load-bearing against writers this process does not own (a second daemon, a CLI
  client). Required a second independent connection test, which is the shape
  WAL_VALIDATION.md actually measured.

Ruling 15 (String ids vs spec 4.1): NOT sent to the implementer — deferred with an OPEN
  block committed as ed0b991. Reason is structural, not cost: DurableEvent references
  entities whose modules do not exist yet, and defining ProjectId outside project/ would
  break the costlier rule that no module is created before the task that fills it.
  Trigger named: Task 9, the last id-owning module and the first signature to place two
  different id kinds adjacent where the compiler cannot tell them apart.
  Cost if wrong: Milestone 0 ships with String ids and Task 9 does a wider sweep.

Task 3: fix round 1/5 dispatched (resumed implementer af5175c) with findings 1-4.

Controller error, self-caught and corrected in a724e3f: 0834bb7's message claimed both
  evidence files now name the commit holding the deleted probe source. They did not. The
  `git add -A CLAUDE.md docs/evidence sandbox` named `sandbox`, which that same command
  had just removed, so the add failed for EVERY path in it — and I had piped stderr to
  /dev/null, so nothing said so. The deletion still landed because `git rm` had staged it
  itself. Net effect: the worse half shipped alone — two evidence files pointing at
  directories that no longer existed, which is precisely the rot the commit existed to
  prevent. CLAUDE.md's spike rule was swept into 1895e3b by accident for the same reason.
  Lesson recorded, not just the fix: do not pipe git stderr to /dev/null, and never name
  a path in `git add` that an earlier command in the same line removed.

Task 3: re-review re-dispatched (sonnet) after the first attempt died on a session rate
  limit, not on a finding. Same diff package, ed0b991..60f75e4.
Task 3: fix round 1/5 re-review — findings 1, 2, 3 ADDRESSED and independently checked
  (feature gate confirmed by `cargo build --lib`; panic test confirmed to genuinely
  strand the connection rather than simulate it). Finding 4 REMAINS OPEN.
  The re-reviewer caught that the fix for an overclaim was itself an overclaim: the new
  external-writer test issues a write-only transaction, and WAL_VALIDATION.md:37-39 says
  plainly that "a write-only transaction cannot produce a lock upgrade". So the test
  passes identically under a deferred BEGIN and proves nothing about BEGIN IMMEDIATE.
  Caught by citing the project's own evidence file back at the code. That is the review
  working as intended.
Task 3: fix round 2/5 dispatched (resumed af5175c) — finding 4 only: give the closure the
  read-then-write shape that actually creates the snapshot a deferred BEGIN must upgrade.
Task 3: minor (deferred): when recovery-at-entry's ROLLBACK itself fails, `txn_open` stays
  true and no fresh BEGIN is attempted, so a genuinely poisoned connection retries the
  same failing ROLLBACK forever. It is logged and it fails loudly rather than proceeding
  on a broken connection, which is the right direction — but nothing ever reconnects.
  For the final whole-branch review to triage.

Ruling 16 (process, from the user stopping a resumed agent): STOP RESUMING LARGE-CONTEXT
  IMPLEMENTERS FOR SMALL FIXES. The skill says rounds 1-3 resume the original implementer,
  on the reasoning that its context is intact and therefore cheap. That reasoning inverts
  once the context is large: this implementer had already spent 167k tokens across the
  task and round 1, so resuming it for a five-line test change re-sent all of it and cost
  ~190k more. The user stopped it, correctly.
  New rule for this plan: resume only when the fix genuinely needs the implementer's own
  reasoning about choices it made. Otherwise dispatch a FRESH cheap agent carrying the
  brief path, the report path, and the finding — which is what the skill already
  prescribes for rounds 4-5, just applied on cost grounds instead of capability grounds.
  Cost if wrong: a fresh agent re-reads two files it would have remembered.

Task 3: fix round 2/5 — the killed agent HAD completed the work; it died before the
  commit step, so the diff was sitting uncommitted in the tree. Controller verified it
  rather than trusting it, and rather than re-dispatching work already done:
    - `cargo test --test storage_contract` 6/6 pass
    - the discriminating claim checked EMPIRICALLY: changed `BEGIN IMMEDIATE` to `BEGIN`
      at src/storage/sqlite/mod.rs:143, ran the suite, and
      `write_txn_waits_out_an_external_writer_holding_begin_immediate` alone failed with
      SqliteError code 517 (SQLITE_BUSY_SNAPSHOT) while the other five passed. Reverted.
    - all three CI gates clean.
  Committed as 51f74a3. The verification is in the commit message, not just here.
Task 3: fix round 2/5 re-review clean — test discriminates AND doc comment claims only
  what the suite establishes. Both had to hold; both do.
Task 3: complete (commits 0834bb7..51f74a3, review clean after 2 fix rounds)
Task 4: dispatched (sonnet, fresh agent), BASE 51f74a3.
Task 4: implemented by sonnet (agent aecbffe), commit 2e14db4, DONE_WITH_CONCERNS.
  11/11 tests pass, three CI gates clean, and the `append_event` dead_code warning is
  confirmed gone — Task 4 added its first product caller, as predicted.

Ruling 17 (implementer concern 1 — `src/runtime/recovery.rs`): the IMPLEMENTER was right
  and MY PLAN was wrong. Its Files block listed the file; no step ever defined its
  contents. The implementer declined to create an empty module rather than guess, which
  is exactly what CLAUDE.md demands. There is nothing for it to hold: §8.6 puts the
  ownership scan in SQL, so `Storage::reconcile_orphans` IS the recovery logic and
  `Runtime::start` only calls it. Plan amended, line removed, reason written where it
  stood (c7dd02c). Cost if wrong: a future task re-adds a file with a real purpose.

Ruling 18 (implementer concern 2 — Runtime::start not wired into cli::serve): NOT a
  defect. Verified against the plan: Task 11 Step 6 calls `Runtime::start(storage.clone())`
  inside `serve` when AppState assembles. Deliberate, not forgotten.

Ruling 19 (my own structural rule, applied to myself): tests/storage_contract.rs is at
  361 lines, past the 300 threshold I added to CLAUDE.md this session, and Tasks 5 and 6
  were both aimed at it — it would have crossed 500 by Task 6. Amended both tasks to
  write tests/project_contract.rs and tests/thread_contract.rs instead. Split by domain,
  which is what CLAUDE.md prescribes for this named accretion point. Two lines now versus
  untangling a 500-line file later. A rule I wrote and then let slide would have been
  worse than no rule.
Task 4: review dispatched (sonnet) on review-51f74a3..2e14db4.diff, with both rulings
  carried so the reviewer does not re-litigate them.
Task 4: review APPROVED, no Critical and no Important findings — the first task this plan
  has passed on the first pass.
  The reviewer's one ⚠️ resolved by the controller: `runtime_instance.stop_kind`,
  `operation.status_kind`, `interrupt_reason`, and `finished_at` are all present in
  migrations/0001_milestone0.sql, and the table's CHECK constraints enforce the state
  machine itself — `status_kind = 'Interrupted'` requires `interrupt_reason IS NOT NULL`,
  so recovery could not have written a malformed row even if it tried.
Task 4: complete (commits 51f74a3..2e14db4, review clean, zero fix rounds)
Task 4: minor (deferred): `reconcile_orphans` ORDER BY o.id sorts by UUID string, which
  carries no domain meaning — worth a comment saying it is for determinism only. And the
  three recovery paths are each tested alone; none exercises all three in one
  reconcile_orphans call. Both for the final whole-branch review.

PR #1 opened: https://github.com/mohammedtaha-ai/shadows/pull/1 — tasks 1-4, frozen at
  c7dd02c. This is Ruling 2's first checkpoint and it is spec 10.2's own
  "Persistence Foundation Ready", so the gate is the spec's, not one I invented.
Ruling 20: work continues on a NEW branch `milestone-0/product-path` from c7dd02c.
  Reason: had tasks 5-13 kept pushing to `milestone-0/browser-planner`, PR #1 would have
  grown under the reviewer and stopped being a checkpoint — a PR that keeps moving is a
  branch with a URL. Frozen, it can be read, merged, or rejected on its own.
  Cost if wrong: PR #2 shows PR #1's commits until PR #1 merges, which is ordinary
  stacked-PR behaviour and resolves itself.
Task 5: dispatched (sonnet, fresh agent) on milestone-0/product-path, BASE c7dd02c.
  Carries the Ruling 19 override: tests go in tests/project_contract.rs, and the brief's
  own Files header and git-add line are wrong on that point.
Task 5: implemented by sonnet (agent a46362f), commit 7cfa63d, DONE. 14/14 tests pass,
  three CI gates clean. tests/project_contract.rs created with all 3 of the brief's tests;
  storage_contract.rs untouched at 361 lines, which is the Ruling 19 split working.
  Implementer's "five tests" note checked by the controller against the brief rather than
  handed on as doubt: the brief defines exactly 3, and the "five" counted two tests that
  lived in storage_contract.rs when the plan was written. Stale arithmetic, not a gap.
Task 5: review dispatched (sonnet) on review-c7dd02c..7cfa63d.diff, with both rulings
  carried so the reviewer does not re-litigate them, and with the idempotency contract
  called out as load-bearing for Tasks 6, 9, 10 and 11.

PR #1 MERGED (68dc2ff) into main, on the user's instruction, after CI went green on both
  Windows and Linux. main now carries tasks 1-4. `milestone-0/browser-planner` kept, not
  deleted — its commits are also the base of product-path, and deleting it buys nothing.

USER INSTRUCTION: stop after the Task 5 reviewer reports. Do NOT dispatch Task 6.
  Resume point when work continues: Task 5's review outcome, then Task 6 (PlanningThread
  and ThreadEntry with transactional ordinal allocation), BASE = whatever Task 5's fix
  rounds leave as head of milestone-0/product-path. Task 6's dispatch must carry the
  Ruling 19 override: its tests go in tests/thread_contract.rs, NOT storage_contract.rs,
  and its brief's Files header and git-add line are wrong on that point.

Task 5 review: spec mostly ✅ — atomicity, interfaces, domain purity, ordering, file sizes
  and TDD evidence all verified clean. ONE Important finding, brief-mandated, OPEN.

  The reviewer CONSTRUCTED a fingerprint collision rather than suspecting one.
  `canonical()` in src/command/mod.rs:23-32 joins `"{key}:{value}"` pairs with "," and
  escapes nothing, so:
      {"a": 1, "bc": 2}   -> "a:1,bc:2"
      {"a:1,bc": 2}       -> "a:1,bc:2"
  Same digest, same command_kind. `classify` would treat a structurally different request
  as a replay of the first. Not exploitable today — `project.create`'s keys are Rust
  literals — but this exact function is reused verbatim by Tasks 6, 9, 10 and 11, and the
  first command with user-influenced key names inherits the hole. Copied verbatim from the
  brief's Step 3, so it is my plan's defect.
  Verified by reading, not taken on trust: format!("{}:{}", "a:1,bc", 2) == "a:1" + "," +
  "bc:2". The collision is arithmetic, not a hypothesis.

Ruling 21 (the reviewer's one ⚠️, scope_kind/scope_key): RESOLVED, code is correct.
  Spec 6.x (2026-09-21-sqlite-schema-design.md:737-743) states Global scope verbatim as
  command_scope_kind = 'Global', command_scope_key = '', and says non-global scopes use a
  non-empty canonical entity key. `project.create` creates the project, so no project
  scope exists to key on at the time the command runs. No finding.

STOPPED HERE ON USER INSTRUCTION. Task 5 is NOT complete — one Important finding open,
  zero fix rounds spent. Resume by dispatching fix round 1/5 for the collision, then the
  scoped re-review, then Task 6.
Task 5: fix round 1/5 — fresh sonnet agent (ab42bf8), NOT a resume, per Ruling 16.
  Commit 916cbe4. Three lines: object keys are now emitted via serde_json::to_string, so
  quotes and escapes delimit them. The explicit keys.sort() survived, so the fix does not
  lean on serde_json's default BTreeMap ordering and a `preserve_order` feature switched
  on elsewhere in the dependency graph cannot silently change future fingerprints.
  16/16 tests. RED confirmed before the fix: the collision test failed with two identical
  SHA-256 digests printed, which is the evidence that it guards something.
  Scoped re-review dispatched (haiku), told to reason about injectivity itself rather
  than trusting two test cases, and to check the keys.sort() survival specifically —
  because removing it would pass every test today and fail silently much later.
Task 5: re-review clean. The re-reviewer reasoned injectivity out itself rather than
  resting on the two tests, and checked the three cases I named — a string value
  impersonating an encoded key, nested recursion, object-vs-array — finding none open.
  It confirmed keys.sort() survived at src/command/mod.rs:24 and does not depend on Map
  iteration order, which was the failure that would have passed every test today.
  The earlier Minor about no test pinning a single value change is now CLOSED, not
  deferred: `the_fingerprint_changes_when_a_value_changes` covers it.
Task 5: complete (commits c7dd02c..916cbe4, review clean after 1 fix round)

READY FOR TASK 6. BASE = 916cbe4 on milestone-0/product-path.
  Task 6 = PlanningThread and ThreadEntry with transactional ordinal allocation.
  Its dispatch MUST carry the Ruling 19 override: tests go in tests/thread_contract.rs,
  NOT storage_contract.rs, and the brief's Files header and git-add line are wrong there.
  It consumes `classify` and `record_command` from Task 5 — now collision-free.

---

Ruling 22: `classify` must compare all three fields spec §6.19 names, not two.

The user read the Task 5 completion and found what both my re-review and I had
missed: `classify` compared `command_kind` and `request_fingerprint` and ignored
`command_schema_ver`, though §6.19 states the comparison as three fields.

I fixed it myself rather than dispatching (31efa30). RED confirmed first: the
test got back the stored Project instead of `CommandConflict`.

**What it costs if wrong.** Every later mutating command reuses `classify`. With
the version dropped, bumping a command's schema version replays an outcome
computed under normalisation rules that no longer apply, the caller reads a
stale entity as success, and nothing fails anywhere. Four remaining tasks were
one seam away from inheriting it.

**What this says about the review loop.** Two reviewers passed over `classify`'s
body — the Task 5 spec reviewer and the collision re-reviewer — and neither
compared the field list against §6.19's sentence. A reviewer asked to check a
fix checks the fix. Nothing was watching the function the fix lived in. The
whole-branch review before PR #2 is where that gets caught, and it now carries
this as a named instance rather than a general worry.

Docs corrected in the same pass (be95b1c), none of them a decision change:
Task 5 and Task 6 bodies pointed at `storage_contract.rs` while their own
headers said otherwise (Ruling 19 was applied to the headers only); the ledger
header named the wrong branch; `status.md` still said no product code existed.

Scoped re-review of 916cbe4..be95b1c: clean on the fix, and it **found one
defect in my own doc commit** — `status.md` claimed five test suites where the
tree has four, because I counted the two empty unit-test binaries `cargo test`
prints. Fixed in 77d8a0b. A reviewer that only confirms is not reviewing.

Task 5: complete (commits c7dd02c..77d8a0b).

## READY FOR TASK 6

BASE: 77d8a0b on milestone-0/product-path.
Plan: Task 6, PlanningThread + ThreadEntry with transactional ordinal allocation.
Consumes `classify` and `record_command` from Task 5 — now schema-version-correct.
The Task 6 body in the plan is now self-consistent: tests go in
`tests/thread_contract.rs` and the `git add` line names it. No override needed
in the dispatch any more; the earlier warning is discharged.

User instruction: run on opus, fall back to fable if opus refuses.
Task 6 dispatched (opus, agent a7823017), BASE 77d8a0b — killed mid-task before
committing (tree stayed clean). Re-dispatched on fable per user instruction.
Task 6 implementer (fable, agent add6b493d), commit 1240b2d, DONE.
  19/19 tests green; clippy/fmt clean. RED → GREEN TDD evidence recorded.
  Raised three concerns; the implementer's `src/storage/mod.rs` interpretation
  (modified `src/storage/sqlite/mod.rs` to match Tasks 4-5) was verified by
  the reviewer as correct, not a deviation. The "seven tests" line in the
  brief is stale prose; only two are defined and only two were written.
Task 6 review dispatched (sonnet, agent aa2c5296) — 403 error: this key only
  admits fable and opus. Re-dispatched on fable (agent a16d5fa5).
Task 6 review verdict: spec compliant, task quality approved, no Critical or
  Important findings. Four Minor deferred:
  - M1: `ThreadEntry` and `list_thread_entries` SELECT omit `refs_json` (the
    column exists in the schema). Brief didn't list it; stays out of scope.
    Resurfaces when an entry-rendering task needs it.
  - M2: `Row` type alias inside the function body (clippy-driven, non-behavioral).
  - M3: `ThreadEntryAppended` payload omits `body` (body lives in
    `thread_entry.body`; payload is event-stream metadata only).
  - M4: the concurrent test would silently lose its meaning if ordinal
    allocation ever moved out of `write_txn`. Worth a one-line guard comment
    when the allocation is next touched.
Task 6: minor (deferred): see M1-M4 above, for the final whole-branch review.
Task 6: complete (commits 77d8a0b..1240b2d, review clean, zero fix rounds)

User standing instruction: ALL subsequent dispatches in this plan use fable.
  Recorded for the rest of the plan; no per-task reminder needed.

Ruling 23 (process, from user correcting the Task 6 reviewer prompt): reviewers
read the SPEC CONTENT, not just the README. The README is an index that maps
section numbers to owner files; a reviewer who reads only the README is
checking section numbers against implementation, not section CONTENT against
implementation. Carried into every reviewer dispatch from here on: the prompt
must list the owner files for the task's sections by name and tell the
reviewer to Read them, not just the README.
Why: a reviewer operating only on the README is one indirection away from the
binding text, and that indirection is where reviewers stop finding things.
Cost if wrong: a reviewer approves a task that contradicts an owner file the
controller would have caught by reading it. The whole-branch review is the
last net.

## READY FOR TASK 7

BASE: 1240b2d on milestone-0/product-path.
Plan: Task 7, the managed process primitive and process-tree containment.
Carries the Ruling 23 spec-content requirement: the reviewer prompt will name
the relevant owner files (§1 architecture, §3 errors-and-testing, §8 runtime).
Plus the two Global-Constraints rules that bind this task specifically: only
`process/` may call `tokio::process` or `process-wrap`, and `process/` knows
nothing about Role, Claude, planning, workflows, or verification.

Task 7 implementer (fable, agent a0964701), commit 5c07eee, DONE.
  21/21 tests green; clippy/fmt clean. Containment verified on Windows with
  a real three-level tree (daemon → tree_probe → grandchild). Brief deviation:
  `Win32_Security` added to `windows` features because `CreateJobObjectW` is
  gated behind it in `windows` 0.58. Two cosmetic clippy-driven removals
  (unused `mut`, dead trait import) and one `#[allow(clippy::zombie_processes)]`
  on the grandchild spawn (intentional — `.wait()` would defeat the test).
  Unix parent-death half is honestly documented as unimplemented in Milestone 0.
Task 7 review dispatched (fable, agent a4e10223) with Ruling 23 applied:
  reviewer Read the three owner files (architecture §1.5, errors §3.7
  Layer 4, runtime §8.3) before judging the diff.
Task 7 review verdict: spec compliant, task quality approved with one
  Important fix recommended before the runtime path ships.

  Finding 1 (Important, plan-mandated): the spawn-to-attach window on Windows
  can leak an orphan. `src/process/mod.rs:102-106`: between `cmd.spawn()?`
  succeeding and `containment::attach(pid)?` returning Ok, `AssignProcessToJobObject`
  can fail (process already in another job, broken handle, etc.). If it does,
  `?` propagates the error, `child` is dropped, and `tokio::process::Child::drop`
  does NOT terminate the process on Windows — the child becomes uncontained.
  Spec §1.5 ("the tree does not outlive its owning runtime") is violated in
  exactly the failure mode this safety primitive exists to close. Same fix
  covers a panic in that window.
  Finding 2 (Minor): `stdout_lines` returns by `&mut` rather than by value
  (deviation from the brief, not in the report). Functionally equivalent and
  arguably more correct; amend the brief or document the deviation.
  Finding 3 (Minor): Windows `attach` leaks the job handle on `SetInformationJobObject`
  or `OpenProcess` failure (benign in practice — OS reaps the job at daemon
  exit — but a leak).

USER INSTRUCTION (mid-iteration): stop after Task 7 is ready; do not dispatch
Task 8. Applying per the user's direction.

Ruling 24: Finding 1 is load-bearing and real. Task 8 (Claude harness) spawns
  through `process::spawn` and Task 10 (cancellation) terminates through
  `terminate_tree`. If `spawn` can leak an orphan in any failure path, those
  two tasks inherit the defect, and the cancellation path inherits the
  "process no longer in our containment" problem on its very first turn.
  Therefore the fix MUST ship before Task 8's implementer starts writing
  code that depends on the invariant.
  Smallest fix: RAII guard around the OS process handle, terminated explicitly
  on any error between `cmd.spawn()` and the successful `attach`. Same guard
  covers a panic in that window. Add a regression test that mocks
  `AssignProcessToJobObject` failure (e.g., a pre-existing job assignment) and
  asserts the orphan does not survive.
  Decision: parked with ruling. The fix is dispatched as Task 7's fix round 1/5
  before Task 8 begins. Carry the brief bug (`Win32_Security` brief omission)
  into the same fix if cheap, otherwise let the next plan amendment absorb it.

Task 7: minor (deferred): M2 stdout_lines signature drift; M3 Windows job
  handle leak on `attach` error path. For the final whole-branch review.
Task 7: NOT YET COMPLETE — fix round 1/5 required before next task. Resume
with the spawn-to-attach RAII guard + regression test. BASE for that fix:
5c07eee. After fix round 1 lands, re-review, then Task 8.

Task 7 fix round 1 — controller correction in progress, superseding the
suggested direct-child RAII guard after root-cause analysis. A guard around
`tokio::process::Child` can kill only the leader after `attach` fails; a
grandchild may already have escaped during the spawn-to-attach window, so that
fix would not establish the complete-tree invariant. The spec already selected
`process-wrap`. Version 10's Windows JobObject temporarily adds
CREATE_SUSPENDED, assigns the process to the Job, and resumes it only after
assignment; its failure paths terminate the still-suspended child and close
owned handles. The hand-written containment files were removed.

During verification, a stronger probe found that process-wrap's job wait can
block on a long-lived grandchild or observe a completion notification before
the full tree is gone. Shadows therefore owns the product semantic explicitly:
`ProcessHandle::wait` polls leader exit, terminates any remaining managed tree,
then returns the leader status. A real leader-exits/grandchild-lives regression
test covers it, alongside the existing three-level explicit-termination test.

Task 6 correction included in the same bounded pass at the user's instruction:
the review's M1 was a real spec omission, not deferred work. `ThreadEntry` now
carries typed `EntryRef` values, append persists `refs_json`, and reads decode
it. A new failure-path test installs a rejecting INSERT trigger and proves that
ordinal allocation rolls back rather than leaving a gap. M2 remains cosmetic;
M3 remains intentionally absent because entity tables are current truth and the
durable event payload is ordering/provenance metadata; M4 is closed by the new
rollback test rather than by a comment.

---

Ruling 25: the code map is generated and build-enforced, not written and trusted.

The user asked why `docs/codebase/` had never been built. The honest answer is
that I proposed it, got no answer, and let it drop — and then, while correcting
`status.md` earlier in this session, I demoted it from "step 2 before
implementation" to "proposed, not yet approved" without deciding anything. That
is how a commitment dies: not by being cancelled but by being reworded.

Built as `docs/codebase/` + `tests/codemap/` (commit 46555ea), and the shape was
settled with the user before any code:

- `inventory.md` GENERATED from `src/`, full signatures, no line numbers.
- `README.md` HAND-WRITTEN, only what no generator can derive: one job per
  module plus its reference file.
- `tests/codemap` regenerates and diffs, and checks the hand-written half's
  mechanical claims. `cargo test` is already the Windows acceptance gate, so a
  stale map cannot reach `main`.

**What it costs if wrong.** A map that can rot is read with the same trust as a
true one, so it is worse than no map. The enforcement is the whole point; the
document is incidental.

**The user overruled part of their own request, and was right to be asked.**
They originally specified `Class::method(argType) -> file:lineStart`. I dropped
`lineStart` — a line number is wrong at the first line inserted above it and
nothing fails when it lies — and proposed full signatures instead. They chose
full signatures without line numbers.

**It paid for itself in its first output.** The generated inventory shows
`append_thread_entry(&self, thread_id: &str, kind: &str, author_kind: &str,
author_id: &str, body: &str, ...)` — five consecutive `&str` the compiler cannot
tell apart. `shadow` died partly on a reversed argument pair that 413 commits
missed. This is the same shape, now visible on one line. It is not yet fixed;
Task 9's newtype sweep (domain spec section 4.1 OPEN block) is where it belongs,
and that block should be read as covering this too.

All three checks were confirmed to fail before being trusted: a removed module
row, a job phrased with "and", and a changed `tracing::init` signature.

## OPEN — Task 7 has no recorded re-review

The ledger's Task 7 entry says "NOT YET COMPLETE — fix round 1/5 required", and
commit 45256e3 landed that fix. No re-review and no completion line follows it,
so on the ledger Task 7 is still open while the tree has moved past it. Either
the re-review happened and was never written down, or it was skipped. Both are
bad in the same way: the record no longer matches the work.

**Trigger that closes this:** before Task 8 is dispatched. Task 8 spawns the
Claude harness through `process/`, which is exactly what Task 7 built and what
the fix changed, so dispatching Task 8 over an unreviewed containment fix is the
one ordering this milestone cannot afford. Ruling 24 already says Finding 1 there
is load-bearing.

Task 8: dispatched (sonnet, fresh agent) on milestone-0/product-path, BASE 46555ea,
brief at task-8-brief.md.

User instruction: dispatch Task 8 now, over the open Task 7 re-review. Ruling 24's
actual requirement was that the containment FIX ship before Task 8's implementer
writes code, and it did (45256e3). What is missing is the re-review record, not the
fix. I had framed this more strongly than the ledger supports; corrected here.
The Task 7 OPEN block above stays open and is closed before PR #2.

Four pre-flight findings carried into the brief as rulings:

A. The plan's Task 8 Interfaces block contradicts its own Step 4 code —
   Interfaces lists `AgentInvocation { harness_path, harness_version }`, the
   Step 4 struct has neither and has `session_id` instead, and the Step 2 tests
   build the Step 4 shape. Ruled: Step 4 + the tests are authoritative.
   Consequence for Task 9: spec §8.2 freezes the harness version per Operation
   and it now lives on `ClaudeHarness.version`, so Task 9 reads it from the
   harness. The implementer must state this in the report.

B. Step 1's capture command carries `--safe-mode`, which `to_process_spec` never
   emits and the measured invocation in SERVE_STREAM_SPIKE.md does not contain.
   Ruled: capture with exactly the flags the product emits. **What it costs if
   wrong:** a fixture captured under flags the product never sends is not
   evidence about the product's invocation, so all four tests would pass while
   proving nothing — and a flag living only in the capture is a divergence
   nobody decided.

C. `< /dev/null` is a POSIX redirect on a Windows host, and closing the child's
   stdin is load-bearing (evidence Finding 5.1: three seconds per turn
   otherwise). Brief requires the Bash tool plus four explicit checks on the
   captured fixture, and requires STOPPING rather than hand-writing one.

D. First task under Ruling 25: `src/agent/` needs a row in
   docs/codebase/README.md and a regenerated inventory in the same commit, or
   `cargo test` fails. The enforcement is now doing the reminding, which is the
   point of building it.

Task 8: implemented by sonnet (agent a3339fd), commit 9ee9255. Verified by me, not
taken on report: commit present, tree clean, 30 tests passing, fmt and clippy clean,
no new `allow` attributes, `src/agent/` row present in docs/codebase/README.md with
inventory regenerated, `--safe-mode` absent from src/ and tests/. Fixture is a real
20-line turn: 1 assistant, 1 result, 3 content_block_delta, system/init with a
session id, plus rate_limit_event.

The implementer found one defect the brief missed and fixed it rather than
suppressing it: the plan's Step 2 test code uses `.filter(..).next_back()`, which
fails `clippy::filter_next` under `-D warnings`. The plan shipped test code that
cannot pass the project's own gates.

Ruling 26: a counted-but-discarded observation is not a test.

Task 8's suite counted delta lines into `deltas` and ended with `let _ = deltas;`.
I disabled the `content_block_delta` arm of `classify` outright — **all four tests
stayed green.** The transient class, which SERVE_STREAM_SPIKE.md calls the main
result of the whole spike, had no assertion anywhere.

Fixed in ec963f4, RED confirmed before GREEN by breaking that arm again:
`deltas >= 1`, plus a new test asserting the evidence report's decisive property
that nothing in the suite compared — the concatenated delta text equals the
durable assistant entry's text (both are "fixture" on the fixture).

**What it costs if wrong.** If that equality stops holding, the daemon drops text
the user watched stream past while the durable history claims otherwise. Task 11
builds durable replay with a no-gap handoff to live on top of exactly this
property. It would have been an invisible divergence between what was displayed
and what was stored, and every other test in the file would still have passed.

The plan's Step 2 is amended so it no longer ships the discard or the clippy
failure. Its stated count is now five tests.

Task 8: minor (deferred): `src/bin/tree_probe.rs:24` carries
`#[allow(clippy::zombie_processes)]` from Task 7 — the only suppression in the
tree. Test apparatus, not product, but it is a suppression and belongs in the
final whole-branch review with Task 7's M2 and M3.

Task 8: review dispatched (sonnet) on review-46555ea..ec963f4.diff, pointed at
CLAUDE.md, the plan's Global Constraints, the evidence report, and the spec index
rather than a retyped constraint block (Ruling 10). It is asked, per point 5, to
hunt for more tests of the shape Ruling 26 found.

Task 8: review APPROVED (sonnet). No Critical, no Important. It verified the
invocation flag-by-flag against SERVE_STREAM_SPIKE.md:52-59, confirmed the four
stream classes including that `rate_limit_event` is a top-level type rather than
a system subtype, and argued the fixture's authenticity from something I had not
thought of: it carries `system/hook_started`, `hook_response`, `status` and
`post_turn_summary` subtypes that the evidence report never illustrates, and a
hand-written fixture would not have invented them.

Ruling 27: §1.4 was violated in the product, under a comment claiming compliance.

The review raised it as out-of-diff, which it was. It is also the most dangerous
shape this class of defect takes: `src/main.rs` carried
`#[arg(long, default_value = "claude")] harness: PathBuf` directly beneath the
doc comment "Spec §1.4 forbids PATH lookup". A false claim of compliance sitting
on top of the violation reads as already reviewed, so nobody looks again.

Fixed in bd1fa43 before Task 9 rather than deferred, and the ordering is the
argument: Task 9 is the first task that records the harness path and version per
Operation. **What it costs if wrong:** the record says nothing, because the path
means "whatever PATH resolved at spawn time" — and this machine carries several
claude-code installations at different versions, so the recorded version can
belong to a different binary than the one that produced the turn. That is
verbatim the failure §1.4 exists to prevent, and §1.4 says the first symptom is
a blank page rather than an error.

`--harness` is now required with no default, takes `SHADOWS_HARNESS` (an
environment variable is still explicit configuration, unlike PATH), and goes
through `config::harness_path`, which refuses any non-absolute path. Refusing
only bare names would not do: a relative path resolves against the daemon's
working directory, which is not stable either.

Review Minor (point 6) closed in the same commit rather than deferred. The
fixture is a pure-text turn, so `render_content`'s `tool_use`, `tool_result` and
`thinking` arms had no test whatsoever — the same shape as Ruling 26. A swap
between them would mislabel every tool call in the durable history with all of
`tests/harness_stream.rs` green, and Task 9 runs a Planner turn, which invokes
tools. Tested in `tests/harness_config.rs` from the shapes the evidence report
measured (SERVE_STREAM_SPIKE.md:95-104), not from invention. Both new tests were
confirmed to bite before being trusted.

Caught by running the full suite rather than the new file: making `--harness`
required broke `tests/serve_smoke.rs`, which invoked the binary without one.
Fixed in the same commit.

Task 8: complete (commits 46555ea..bd1fa43, review approved, 1 self-found fix
round). 33 tests.

## READY FOR TASK 9

BASE: bd1fa43 on milestone-0/product-path.
Plan: Task 9, Operation lifecycle — two-phase spawn for a Planner turn.

Task 9's dispatch MUST carry:
1. The domain spec §4.1 OPEN block. Task 9 creates the last of the three id
   modules and is the first task whose signatures place two ids of different
   kinds adjacent — `mark_operation_started(op_id, expected_runtime)` takes two
   `String`s the compiler cannot tell apart. The newtypes land with Task 9 and
   sweep the earlier signatures. The block names Task 9 as its own trigger.
2. Ruling 27's consequence: the harness version is frozen per Operation from
   `ClaudeHarness.version`, not from `AgentInvocation`, which has no such field.
   The plan's Task 8 Interfaces block is stale on this and Task 9's Interfaces
   block inherits the error.
3. Ruling 26 as a standing check, not a past event: a counted-but-discarded
   observation is not a test. Task 9's tests must each be shown failing.
4. The same §4.1 sweep should take in `append_thread_entry`, which takes five
   consecutive `&str` — found by the code map on its first run.

Ruling 28: §4.1's newtype close is split into two reviewable changes.

The OPEN block instructed that the newtypes land with Task 9 and sweep the
earlier signatures. I am departing from that, so I amended the block in place
(7f3a0f4) rather than deviating silently.

Task 9 lands `OperationId` and `RuntimeInstanceId` only — that pair is the
adjacency the block actually names, and Task 9's own API is where a swapped
argument first becomes reachable. The `ProjectId`/`ThreadId`/`ThreadEntryId`
sweep follows after Task 9's review, before Task 10.

**Why.** A tree-wide rename folded into the operation lifecycle produces one diff
in which a reviewer cannot reject the rename while approving the lifecycle, or
the reverse. The plan's own task right-sizing rule is that a task is the smallest
unit worth a fresh reviewer's gate. **What it costs if wrong:** almost nothing —
the worst case is that `mark_operation_started`'s hazard is closed one change
earlier than the rest, which is the ordering I want anyway.

Also recorded in the amendment: the block's *original* reason for deferring was
structural — `ProjectId` and `ThreadId` had no module to live in. Tasks 5 and 6
created those modules, so that argument is spent and only sequencing remains. An
OPEN block whose stated reason has expired but which still reads as current is
the rot the documentation rules exist to prevent.

Task 9: dispatched (sonnet, fresh agent) on milestone-0/product-path, BASE
7f3a0f4, brief at task-9-brief.md.

Brief carries five rulings. Beyond A (scope) and B (harness version from
`ClaudeHarness.version`, not `AgentInvocation` — the plan's Task 9 Interfaces
block inherits Task 8's error), the load-bearing one is C: Ruling 26 restated as a
standing check with a concrete instruction — for each test, name the one-line
implementation change that would break it, and if none exists the test is
decoration. The CAS tests specifically must prove refusal AND that the stored row
is unchanged after the refusal, not merely that a happy path works.

Also carried: run the whole suite, because ruling A changes
`register_runtime_instance`'s return type and `tests/recovery.rs` calls it. Task 8
shipped a green new file over a broken suite because only the new file was run;
that is now an explicit instruction rather than a hope.

Task 9: implemented by sonnet (agent a3b722c), commit 970f2c8.

Verified by me, not taken on report:
- `mark_operation_started(&OperationId, &RuntimeInstanceId)`. I wrote a throwaway
  test that passes them swapped: it **fails to compile** — "expected
  `&OperationId`, found `&RuntimeInstanceId`" and the reverse on the same call.
  The hazard §4.1's OPEN block was opened for is closed by the compiler, not by a
  convention. Probe deleted.
- The newtypes cannot be bypassed: no `From<String>`, no `Deref`, no `AsRef`, and
  `from_stored` is `pub(crate)`. Outside code cannot build one from an arbitrary
  string, so the types are not decorative.
- Ruling A scope held: no `ProjectId`/`ThreadId`/`ThreadEntryId`.
- CAS SQL read directly: `WHERE id = ? AND status_kind = 'Pending' AND
  runtime_instance_id = ?` on the Running transition.
- `src/operation/` row present in docs/codebase/README.md, inventory regenerated.
- No new `allow` attributes. `clippy::type_complexity` on the 13-column row was
  fixed by extracting a type alias rather than suppressed — the right instinct.
- Largest file in the tree is still tests/storage_contract.rs at 361.
- fmt clean, clippy clean, 0 failures.

Correction to the report: it claims 43 tests. I measure **38**
(codemap 2, containment 3, harness_config 2, harness_stream 5,
operation_lifecycle 5, project_contract 6, recovery 4, serve_smoke 1,
storage_contract 6, thread_contract 4). The suite is green either way, but the
number in the report is wrong.

Ruling 29: ruling B's premise was wrong, and the implementer was right to say so.

The brief told the implementer to record the harness path and version per
Operation, and to report rather than invent a column if one was missing. They
refused to add it and gave the reason: §8.2 freezes that on `AgentInvocation`, not
on `Operation`, and `agent_invocation` is not among the seven tables Milestone 0
persists. Checked both — correct on both counts.

But the refusal surfaced something neither of us was looking for: **§6.15's
`agent_invocation` has no column for the resolved path or the version either.** It
carries `harness_kind` and `profile_json`. So §8.2 and §1.4 require a fact that
the schema has nowhere to store, in the table §8.2 names as its owner.

Recorded as an OPEN block on §6.15 (e302ac4), with the trigger at the point it
bites: the task that first creates that table cannot be written without answering
it. Not fixed now — `agent_invocation` is out of Milestone 0 and inventing columns
for a table nobody is building is how a schema acquires fields nothing fills.

This is the second time a brief of mine has been wrong in its premise and an
implementer has been right to push back (the first was the invented "no HTTP
surface" constraint in Task 1). Both times the value came from the instruction to
report rather than work around.

Task 9: review dispatched (sonnet) on review-7f3a0f4..e302ac4.diff. Point 7 asks
it to name, per test, the single change that would break it, and to check whether
the refusal tests prove the stored row UNCHANGED rather than merely that an error
came back. Point 9 asks for a column-by-column check of `get_operation`'s wide row
against the struct — a silent misalignment there is the worst available outcome
and no test would necessarily catch it.

Task 9: review APPROVED (sonnet). Two findings, both closed in 39f3b02.

It verified §8.6 by checking all five public functions rather than the ones the
tests exercise, confirmed `get_operation`'s 13 columns line up field-by-field with
the struct, and independently counted 38 tests — matching my count and not the
implementer's reported 43.

Ruling 30: fixing an instance is not fixing the class.

Finding 1 (Important): `tests/operation_lifecycle.rs` captured a durable_event
count into `before` and ended with `let _ = before;`. That is Ruling 26 verbatim,
**reproduced in the very task whose brief names it as a standing check.** Ruling 26
was earned in Task 8 and I fixed Task 8's instance. The plan was still carrying
another one, so nothing I did in Task 8 prevented this.

So I swept the plan for the pattern instead of patching the line. Results:
- Task 9's discard: amended in place.
- Task 3's `let _ = conn.execute("ROLLBACK")`: still in the plan, though the
  shipped code fixed it during Task 3's review. Plan staleness, now amended.
- Three in Tasks 10 and 11 ignore the result of a storage call
  (`let _ = reader_runtime...` twice, `let _ = handle.wait().await`). NOT touched:
  they may be deliberate in an SSE reader's shutdown path. **Carried as a named
  pre-flight check for those two dispatches** rather than guessed at now.
- The rest are deliberate best-effort calls in production paths (CloseHandle,
  tx.send on a closed channel, ctrl_c) and are not discarded observations.

**What it costs if wrong.** A discarded observation is a test that reads as
thorough and asserts nothing. The new assertion is also scoped where the old was
not: `kinds` filters to this operation's rows, so an event written against the
wrong operation — or against none — leaves it exactly right while the journal grew
by more than three. Confirmed RED by renaming one event kind.

Finding 2 (Minor) was half wrong, and checking it was worth more than accepting
it. The reviewer read the `Default` impls as surface beyond ruling A. Removing them
fails `clippy::new_without_default`, whose only escape is the `#[allow]` this
project forbids — so `Default` was not gratuitous, the name `new` was. A
no-argument `new` that mints a random UUID reads like a cheap empty value, and
`Default` for it means `OperationId::default()` silently yielding a different id
on every call. Renamed to `generate()`; the lint does not apply and the name is
honest. Unused `Display` impls removed.

The code map earned its keep unprompted: `cargo test` failed on the stale
inventory after the rename, before I thought to regenerate it.

Task 9: complete (commits 7f3a0f4..39f3b02, review approved, 1 fix round). 38 tests.

## READY FOR THE §4.1 SWEEP, THEN TASK 10

BASE: 39f3b02 on milestone-0/product-path.

Next is NOT Task 10. Ruling 28 sequenced the remainder of §4.1's newtype close
before it: `ProjectId`, `ThreadId`, `ThreadEntryId` across `project/`, `thread/`,
`events/`, `storage/` and their tests, taking in `append_thread_entry`'s five
consecutive `&str` parameters. Its dispatch carries:
- `OperationId`/`RuntimeInstanceId` in `src/operation/mod.rs` and
  `src/runtime/mod.rs` are the pattern to copy: private inner field, `generate()`
  not `new()`, `as_str()`, `pub(crate) from_stored`, and no `From<String>`,
  `Deref` or `AsRef` — those would reopen the hazard from the back door.
- The §4.1 OPEN block closes with this change. Amend it; do not leave it reading
  as current.
- The code map will fail until regenerated, and that is the signal, not a chore.

Task 10's own dispatch then carries the three ignored storage-call results above.

§4.1 SWEEP: done by me, not dispatched (user's instruction). Commit edc2dd2.

Ruling 31: the pattern lives once, because drift is the failure mode.

Five newtypes meant five copies of the same 25 lines. Ruling A had said "no macro
for two types" — right at two, wrong at five. **What it costs if wrong:** one copy
quietly gains a `From<String>`, a `Deref` or an `AsRef<str>` a year from now, the
swap hazard reopens in that one type, and nothing anywhere fails. So `src/id.rs`
holds the macro and nothing else; it knows nothing about projects or threads, so
it is the pattern and not a `utils.rs` under a better name.

Three hazards, probed rather than asserted (throwaway test, deleted after):
- `mark_operation_started` — closed by Task 9.
- `list_thread_entries` took any `String`, so a project id reached it silently.
  Now a compile error.
- `DurableEvent::with_project/with_thread/with_operation` took
  `impl Into<String>`, so `.with_project(thread_id)` compiled. One event row is
  visible through several scopes (§4.2), so that is a row in the wrong replay and
  absent from the right one, with nothing failing. **This one nobody had named** —
  it surfaced only because typing the ids made the builders type-check.

One hazard is REDUCED, NOT CLOSED, and I said so in the spec instead of implying
otherwise. `append_thread_entry`'s five `&str` became one named struct, so a swap
must be written out as `kind: <body text>` rather than happening by position —
but `kind` and `body` are both `&str` and the compiler cannot refuse it. The
complete fix is `ThreadEntryKind`, whose variants §4.2 never enumerates. Recorded
as an OPEN block with a trigger (the first feature that branches on entry kind),
not invented: a wrong early enum costs more than text, because migrating stored
values costs more than adding the type later.

Two decisions recorded rather than taken silently:
- `ThreadEntry.author` is `events::Actor`. §4.2 names the type `Principal`; a
  second identical struct would record one decision twice.
- The ids are not `sqlx` types — CLAUDE.md keeps persistence imports out of domain
  types, so each converts to a column at the `storage/sqlite/` boundary only.

Correction to Ruling 30's fix: `Display` is back. I removed it there as unused
surface on the reviewer's word; it was unused only because the code that logs
these ids had not been typed yet, and `tracing`'s `%field` requires it. Unlike
`Deref` or `AsRef<str>` it cannot be applied implicitly where a `&str` is
expected, so it prints an id without ever standing in for one. **The lesson is
narrower than "the reviewer was wrong": "no call sites today" is not evidence of
"not needed", when the change that needs it is the next one queued.**

Correction to what I told the user after Task 9: I said outside code cannot build
an id from an arbitrary string. `Deserialize` is derived and can — it is required
to read `EntryRef` back out of `refs_json`. It grants no ability to pass one kind
of id where another is expected, which is what these types are for, so the hazard
claim stands and the construction claim was too broad. Stated in `src/id.rs`.

`from_literal` is gated behind the existing `test-support` feature so tests can
name a seeded id while product code cannot.

§4.1's OPEN block is CLOSED in place. §4.2 gained the two notes above.

The code map refused the commit twice — once for the stale inventory, once for
`src/id.rs` having no owner row. Both times before I thought of it.

## READY FOR TASK 10

BASE: edc2dd2 on milestone-0/product-path. 38 tests.
Task 10: cancellation — request, confirmed termination, terminal Cancelled.

Its dispatch carries:
1. The three ignored storage-call results in the plan's Tasks 10-11 code
   (`let _ = reader_runtime...` twice, `let _ = handle.wait().await`). Examine, do
   not copy. Ruling 30: a discarded result in a shutdown path may be deliberate; a
   discarded observation in a test never is.
2. Task 7's deferred minors M2 and M3, and the `#[allow(clippy::zombie_processes)]`
   in `src/bin/tree_probe.rs` — the only suppression in the tree.
3. The Task 7 OPEN block above: its fix has no recorded re-review, and Task 10
   terminates through `terminate_tree`, which that fix rewrote.

## TASK 7 RE-REVIEW — the OPEN block above is now CLOSED

Done by me on 2026-09-22, not dispatched. Subject: `45256e3` (the containment
fix) read against §1.5, §8.3 and the library it now delegates to. Verdict:
**the fix is sound, the guarantee it rests on is not tested, and one half of
§1.5 was never implemented at all.**

**The race is genuinely closed, and by the library rather than by us.**
`process-wrap`'s `JobObject::pre_spawn` adds `CREATE_SUSPENDED`, `wrap_child`
assigns the process to the job, and only then `resume_threads` runs. No child
code executes before containment, which is exactly what the manual
spawn-then-`attach` could not promise. M3 (the Windows job-handle leak on the
`attach` error path) is **moot**: `attach` no longer exists, and every failure
path in `wrap_child` terminates the child before returning.

Ruling 32: a guarantee no test can lose is not a guarantee, it is a coincidence.

`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` — the whole of §1.5's kill-on-owner-close
half on Windows — is set by `make_job_object(handle, kill_on_drop)` **only when
a `KillOnDrop` wrapper is registered alongside `JobObject`**. Nothing in our
tree said so, and `wrap(KillOnDrop)` reads like a redundant nicety next to a
Job Object that already kills its tree.

Probed, not asserted: I deleted `wrap(KillOnDrop)` and its import, and ran the
Windows gate. `clippy -D warnings` clean. All three containment tests green.
`cargo test` green. **The one line that makes a killed daemon take its harness
with it can be removed and nothing in this repository objects.** Restored, with
a comment at the wrap site that states the coupling and records the measurement.

**What it costs if wrong:** every orphan story in §8.6 assumes the OS already
killed what the dead runtime owned. If that flag silently goes, `reconcile_orphans`
still marks the operation `Interrupted` on the next start — while the real
harness is still running, still writing, still spending tokens. The database
would say interrupted and the machine would say otherwise, and the test suite
would agree with the database.

**§1.5's Linux half was never implemented.** `ProcessSession` is `setsid` plus
`killpg`: it satisfies §8.3 (Shadows asks, the tree dies) and `tests/containment.rs`
proves that much. It supplies no parent-death containment at all — `SIGKILL` the
daemon and the harness keeps running. §1.5 names this case and refuses a process
group as proof of it, so the spec was stating as decided something the code does
not do, with the gap living only in `status.md`, which decides nothing.
Now an **OPEN block in §1.5** with the three candidate mechanisms
(`PR_SET_PDEATHSIG`, a cgroup v2 scope, a `pidfd` supervisor), the trigger (the
first Linux daemon run or the first Linux acceptance claim), and why it does not
block a Windows milestone.

**Carried into Task 10, unchanged in substance from the READY block above:**

- M2 is real and it bites: `stdout_lines(&mut self) -> Option<&mut Lines<..>>`
  hands out a borrow, but the plan's Task 10 code does
  `handle.stdout_lines().take()` and then moves `handle` into `LiveHandles`
  while the reader task holds those lines for `'static`. That does not compile.
  Task 10 needs an ownership-taking accessor (`take_stdout_lines`), and
  `tests/containment.rs` follows it.
- The three discarded storage results, and the `#[allow(clippy::zombie_processes)]`
  in `src/bin/tree_probe.rs`.

Also fixed in this pass, from reading the same code: `ProcessHandle::wait`'s
10 ms `try_wait` poll had no comment saying why it is not `child.wait().await`.
Both wrappers' `wait` drain the entire job or group before returning, so the
obvious simplification would hang any turn whose harness leaves a helper behind
— the exact case `waiting_for_the_leader_reaps_any_remaining_grandchild` exists
to cover. Comment added at the loop.

Separately, the Linux CI failure that had been red since 46555ea was a test
truth bug, not a containment bug: `is_alive` read the existence of `/proc/<pid>`,
which a killed-but-unreaped process keeps. Fixed in `7fbd5cb`; both CI jobs green.

Task 7: COMPLETE. The OPEN block is closed. 38 tests.

## TASK 10 DISPATCHED

BASE: 9b4b095 on milestone-0/product-path. 38 tests. CI green on both jobs.
Implementer: sonnet, from task-10-brief.md. Report: task-10-report.md.

Four rulings carried in the dispatch, all made because the brief predates
edc2dd2 and nobody can transcribe it as written:

Ruling 33: the brief's Rust code is intent, not transcription. Its spec
references, status strings, event kinds, SQL and assertions stay verbatim; its
signatures do not, because §4.1's newtypes landed after it was written. **What
it costs if wrong:** an implementer that treats the whole brief as literal
writes code that cannot compile, burns a round discovering it, and may "fix" it
by reintroducing `String` ids — reopening the exact hazard §4.1 closed.

Ruling 34: `stdout_lines` becomes ownership-taking, and `tests/containment.rs`
follows it. Task 7's M2 was booked as a deferred minor; it is load-bearing here,
because the reader task needs `'static` lines while the handle moves into
`LiveHandles`. Closed inside Task 10 rather than deferred again. **What it costs
if wrong:** nothing silently — it is a compile error either way. The risk is
only that the fix is done badly, e.g. leaving a second accessor behind.

Ruling 35: the three `let _ =` discards are judged individually, not copied.
Named the reader's `append_thread_entry` discard as the one that is certainly
wrong — it is the durable write of the conversation, not a shutdown path, and
swallowing it loses a message with nothing anywhere saying so. **What it costs
if wrong:** if I am wrong and it is deliberate, we get an unnecessary log line;
if I had let it stand and it is not, the product loses conversation entries
silently, which is the milestone's whole point.

Ruling 36: `#[allow(clippy::zombie_processes)]` in `src/bin/tree_probe.rs` stays
out of Task 10 and goes to the whole-branch review, against the earlier READY
block that carried it here. It has nothing to do with cancellation, and the
probe's shape is exactly what the containment tests depend on. **What it costs
if wrong:** the only suppression in the tree survives one task longer and could
be forgotten — so it is written into the final review's list, not dropped.

Task 10 implementer (sonnet) reported DONE, commit 0e6ffe7. Verified rather than
relayed: `git log 9b4b095..HEAD` is that one commit, the tree is clean, and
`cargo test` counts 45 — the report's number, confirmed independently.

The implementer's own concern is the interesting one and went into the review
dispatch verbatim: TDD surfaced a race the brief's code did not anticipate —
killing a process closes its stdout, the background reader reads that as a
natural exit, and it races `stop()`'s Cancelled write. It says it made the
live-handle map the single arbitration point for who writes the terminal state.

Diff also adds `src/bin/fake_claude.rs`, a second test-support binary. The plan's
File Structure names no file under `src/bin/`. Not pre-judged in the dispatch —
the constraints went to the reviewer verbatim and it decides.

Task 10 review dispatched (opus — the diff is 770 lines and the change is
concurrency arbitration, not transcription). Package:
review-9b4b095..0e6ffe7.diff.

Task 10 review verdict: spec ❌, quality Needs fixes. No Critical; six Important,
five Minor, two ⚠️. The review is good — it verified the arbitration claim
instead of accepting it, and found that the fix for the race introduced a
different §8.4 violation.

Rulings before fix round 1:

Ruling 37: finding 5 (a child that dies without a TurnEnd is recorded
`Completed`) was labelled brief-mandated, and the brief loses. §8.4 case 4 says
in as many words: "Persist `Completed` or **`Failed`** from the real exit." The
brief's unconditional `mark_operation_completed` contradicts the spec it cites,
and `FailureStage::Run` exists in `src/operation/mod.rs` with no caller for
exactly this path. Fixed in this round, and the plan text amended so the defect
does not outlive it. **What it costs if wrong:** a crashed harness is recorded
as a completed turn — the database says the work finished and the machine says
it died, which is the failure class this whole project is built to refuse.

Ruling 38: finding 4's uuid does NOT go into `Actor.id`, and it does not get a
column either. `StreamItem::Entry`'s own doc calls the uuid "the entry's
harness-side identity", and `thread_entry` has no place for an identity — the
same gap as `agent_invocation` and the harness version, recorded in e302ac4.
So: the author becomes a stable identity, the uuid is dropped deliberately with
the reason in the code, and §6's `thread_entry` gains an OPEN block whose
trigger is the first feature that must match a stored entry to a harness-side
line — resume dedupe or replay, i.e. plausibly Task 11. **What it costs if
wrong:** if Task 11 needs the uuid we pay one migration on a schema no product
data has ever used. Inventing the column now on a guess costs the same migration
plus a column nothing reads.

Ruling 39: ⚠️1 — `AgentInvocation.operation_id` stays a `String` today and is
typed in this round. §4.1's close claims outside code cannot pass one kind of id
where another is expected; a public `String` field on the struct that names the
operation is a hole in that claim, and it is two lines. **What it costs if
wrong:** nothing but churn — it is a compile-time change with no stored form.

Ruling 40: ⚠️2 — `src/bin/fake_claude.rs` is accepted under the `tree_probe.rs`
precedent; a harness contract measured against a real `claude` cannot be tested
without a fake one. The Minor it exposes is real and belongs to both binaries at
once, not to this task: test-support binaries build in `cargo build --release`
while `storage::test_support` is feature-gated for exactly that reason. Booked
for the whole-branch review as ONE fix for both. **What it costs if wrong:** two
test binaries ship in a release build of a product that has no release yet.

Ruling 41: the `TurnEnd` minor is promoted into this fix round. The reviewer
graded it Minor and by its own rubric that is right, but this project has
already shipped a whole stream class that could be disabled with every test
green (Task 8, commit ec963f4), and this is the same shape: delete the branch
and the suite still passes. One assertion on the persisted outcome. **What it
costs if wrong:** one cheap assertion nobody needed.

Deferred minors (for the whole-branch review): `stop` returns `Ok(())` for
three different outcomes and the HTTP layer cannot tell them apart;
`request_cancellation` is silently `Ok(())` for a nonexistent operation id;
test-support binaries in release builds (Ruling 40); `storage/sqlite/operation.rs`
at 296 lines against the 300 trigger.

Task 10 fix round 1/5: the sonnet implementer was stopped by the user mid-round
before it wrote anything (tree still clean at 0e6ffe7). Re-dispatched as a FRESH
implementer on opus at the user's instruction, carrying the brief, the prior
report file, all eight findings, and rulings 37-41. Normally the skill reserves
the capability bump for rounds 4-5; taking it at round 1 is the user's call and
costs nothing but tokens.

FIX_BASE for the scoped re-review: 0e6ffe7.

Task 10 fix round 1/5 landed: commit 558171a, "the interlock stops guessing
which ending a turn had". Verified independently: one commit, tree clean,
`cargo test` counts 48 (45 + 3 new) — the report's number.

Checked the report's riskiest claim myself before dispatching the re-review:
the new `termination_fails` seam in `src/process/mod.rs` is gated at all four
points — the field (:38), the branch in `terminate_tree` (:93), the setter
(:104) and the initializer (:173) each carry `#[cfg(feature = "test-support")]`.
So the claim "absent from cargo build" holds. Whether a production type should
carry a test-only failure switch at all is a judgment, and it went to the
re-reviewer as a named question rather than as my verdict.

Three implementer concerns went into the re-review dispatch as claims to weigh,
not accept. The first is the one that matters: `stop` now declines to terminate
once the turn produced its own ending, so a harness that emits `TurnEnd` and
then hangs cannot be killed by `stop`. That is finding 2's prescribed tiebreak
taken to its conclusion, and it may be a faithful reading of §8.4 case 4 or a
new hole against case 3 ("cancel after Running terminates and reaps the
registered tree"). I did not pre-judge it.

Scoped re-review dispatched (opus) on review-0e6ffe7..558171a.diff.

Task 10 fix round 1/5 re-review: all EIGHT findings ADDRESSED, each with
file:line evidence and each test checked for whether it can actually fail. One
Important new hole introduced by fix 2, plus two minors and a process gap.

Ruling 42: the new hole is paid now, in round 2, not deferred.

The hole: `stop` now declines to terminate once `turn_end_seen` is set, so a
harness that emits its turn-end result and then hangs cannot be killed, and
`stop` returns `Ok(())` having terminated nothing. §8.4 case 4's precondition is
"*the process exits naturally*" — `has_exited` is that fact; `turn_end_seen` is
not, and the fix's own test proves it, because its whole premise is a process
that is demonstrably still alive after emitting `TurnEnd`. While it is alive and
the operation is `Running`, a cancel is case 3: terminate and reap the
registered tree.

The re-reviewer's shape is right and is what I am ordering: the OR fuses two
decisions that belong apart. `has_exited` decides whether there is anything to
terminate; `turn_end_seen` decides only who writes the outcome. The second flag
the implementer wanted to avoid — the reader must know its own exit was caused
by our kill, so it records the turn-end outcome it already holds instead of
reading the kill's status as a `Run` failure — is the cost of case 3, not an
argument against it.

**What it costs if wrong:** one more bool in the subtlest part of the interlock,
and the implementer's judgment that this is the worse machinery may prove right
under some case nobody has thought of. What deferring costs is worse and is not
hypothetical: `stop` reports success having stopped nothing, the operation stays
`Running` forever, and §8.5 says shutdown *reuses the cancellation path* — so
whoever wires `LiveHandles` into `cli/` in Task 12 or 13 inherits a stop that
silently declines, with the acceptance gate green. A claim that outruns reality,
which is the one defect class this project exists to refuse.

Ruling 43: the `TurnEnd`-with-failure-subtype minor joins round 2. `ended_cleanly`
consults only the exit status, so `{"subtype":"error_max_turns"}` with exit 0
persists as `Completed` carrying an error outcome. The re-reviewer graded it
Minor and out of finding 5's scope, which is correct — but `error_max_turns` is
a real subtype of the measured harness contract, not a hypothetical, and the
code is already being edited in that exact match. Same class as ruling 37: the
record must not say the turn succeeded when the harness said it did not.

Ruling 44: the re-reviewer's process gap is closed in round 2 too — nothing in
the tree fails if `test-support` ever becomes a default feature, and the whole
safety of three test-only seams (`storage::test_support`, `from_literal`,
`termination_fails`) rests on it not being one. A `cargo build` step with the
feature off, in the CI gate. This is the Task 7 lesson applied before it bites:
a guarantee no test can lose is a coincidence.

Deferred minors (whole-branch review), added to the list: the reader holds the
registration across `wait()`, so a stdout-closed-but-alive harness is unreachable
by `stop` until recovery — honest about the ending, a behavioural change worth
a second look with §8.5 in hand; `stop`'s hardcoded `Actor::user("local")`;
`cli/` does not construct `LiveHandles` yet, so §8.5's shutdown guarantee has no
implementation to check.

Task 10 fix round 2/5: commit 4d7a13e (a97ea37 was recommitted with the
regenerated code map). Verified independently: two fix commits on 0e6ffe7, tree
clean, 49 tests, `src/planner/mod.rs` at 527 lines.

Status DONE_WITH_CONCERNS. Round 2's items were shown RED first, including the
one that matters: re-fusing the two branches left the 120-second harness alive
with the operation never terminal, which is ruling 42's hazard reproduced rather
than argued.

Three concerns are open and are NOT adjudicated — the user stopped the loop here
before the scoped re-review of this round:

1. `src/planner/mod.rs` is 527 lines, past CLAUDE.md's 500 split threshold. The
   implementer stated the judgment instead of assuming it: the only seam that
   gets it under 500 without cutting elsewhere is `LiveHandles`/`LiveTurn` apart
   from `PlannerTurn`, which is the forbidden split, since `LiveTurn`'s field
   comments ARE the arbitration rule and `stop`'s branches are that rule
   executed. It proposes instead a `spawn` file owning "turn a request into a
   registered, Running operation", leaving `mod.rs` with "decide and persist how
   a live turn ends". That reads like two jobs nameable without "and", so it is
   probably the right cut — mine to rule on, not ruled yet.
2. **My round-2 message was wrong on a fact.** I told it `error_max_turns` is a
   real subtype "in the measured harness contract in docs/evidence/harness/".
   It checked: nothing in `docs/` contains that string, and `SERVE_STREAM_SPIKE.md`
   carries exactly one `result` subtype, `success`. So it branched on the one
   measured value and treats everything else as failing, which is the honest
   shape and better than the list I implied. Recorded because a controller
   citing evidence it did not open is the same defect this project hunts in
   implementers, and the remedy is the same: if that subtype is known from
   outside, it belongs in the evidence file before any code branches on it.
3. `stop`'s `has_exited` branch has no full-stack test — `slow-exit` now stays
   alive so it no longer traverses it. Storage-layer coverage exists
   (`a_natural_exit_wins_over_an_in_flight_cancellation`).

STOPPED AT USER INSTRUCTION after this report. Task 10 is NOT complete: round 2
has had no scoped re-review. FIX_BASE for that re-review is 558171a. Nothing is
pushed; the branch is two commits ahead of origin.

## THE THREE OPEN ITEMS, CLOSED BY ME (commit e521423)

User instruction: finish these myself rather than dispatching. 50 tests, full
gate green, feature-off `cargo build` clean.

Ruling 45: `src/planner/mod.rs` splits, and by the implementer's seam not the
obvious one. It was right that separating `LiveHandles`/`LiveTurn` from
`PlannerTurn` is the forbidden split — the field comments ARE the §8.4
arbitration rule and `stop`'s branches are that rule executed. So `spawn.rs`
owns "turn a request into a registered, running operation" and `mod.rs` owns
"decide and persist how a live turn ends"; the watcher and `stop` stay together
because they are the two sides of one arbitration. Neither job needs "and".
447 + 160 lines. **What it costs if wrong:** one more file to open when
reading the spawn path, against a 527-line file that CLAUDE.md said to split.

Ruling 46: `has_exited` gets a test where the fact is defined, and the
full-stack window stays uncovered on purpose, with the reason written into the
test. Entering "exited, but the watcher has not yet claimed the registration"
deterministically needs a pause knob inside the interlock, and machinery in the
arbitration costs more than it proves. Proved RED by inverting the predicate.
**What it costs if wrong:** the window stays covered only by the storage-level
case-4 test, so a future change to `stop`'s branch order could pass the suite.
Said so in the test rather than leaving it to be discovered.

My own correction, carried out rather than only confessed: the evidence report
now records what it did NOT measure — no failing turn was provoked, no other
`result` subtype was observed, and a subtype learned from outside belongs in
that file before any code branches on it. I had asserted `error_max_turns` was
measured; it is not, anywhere in `docs/`.

Task 10 status unchanged: fix round 2 still has no scoped re-review. FIX_BASE
for it is 558171a; the range now also carries e521423, which is mine and was
not reviewed either.

Task 10: COMPLETE (commits 0e6ffe7..e521423, 50 tests).

Closed on the user's decision, not on a clean re-review. Fix round 2 (4d7a13e)
and my own e521423 were never scoped-re-reviewed; the user ruled that green CI
on both platforms stands in their place. CI run 35755115340 on e521423: Linux
1m43s, Windows 3m53s, both green.

Recorded as theirs, and recorded for what it is: CI proves the tree compiles on
both platforms and that 50 tests pass. It does not read the §8.4 arbitration.
The unreviewed surface is exactly the interlock's two-decision split and the
file split around it.

Branch pushed: 9b4b095..e521423 on origin/milestone-0/product-path.
Next in the plan: Task 11 (durable replay with a no-gap handoff to live).

## TASK 11

BASE e521423. Implementer: opus (standing instruction from the user — every
dispatch on opus from now on, stated explicitly, never inherited).

Reported DONE_WITH_CONCERNS, commit b22d916. Verified rather than relayed: one
commit, tree clean, 53 tests. 836 insertions across protocol/ (three files as
the plan names them), events_read.rs, cli wiring, tracing, and tests/resync.rs.

Review dispatched (opus) under the NEW review rule from CLAUDE.md: the reviewer
fixes what it finds in its own seat and reports what it chose not to change.
What it may not do silently is contradict the plan or a spec — those come back
to me. This is the first task run under that rule.

Five implementer concerns went into the dispatch as claims to weigh:

1. §8.5 shutdown gap, forewarned in the dispatch and confirmed by the
   implementer: the brief decides `StopKind` from `stop(...).is_err()`, but
   `stop` returns `Ok(())` for declined and no-handle, so `Graceful` can be
   written over a non-terminal Operation. It implemented the brief and reported,
   which is exactly right. Mine to rule on — Task 13 or here.
2. Two forced deviations: the harness version probe now goes through
   `process::spawn` (CLAUDE.md owns `tokio::process`), and `src/tracing.rs`
   writes to stderr because the brief's `serve` logged before printing the
   address and broke `serve_smoke`'s §1.0 one-line guarantee.
3. **The one that matters.** The live phase forwards only transient
   `StreamItem`s, so durable events committed after `caught-up` are never
   streamed, and "de-duplication by durable sequence" has nothing to
   de-duplicate while connected. If true, §2.10's no-gap handoff may be kept by
   the tests and not by the code. Told the reviewer to say so plainly and name
   the smallest faithful fix, and NOT to redesign the bus.
4. Six of seven routes untested; it hand-verified two and deleted the probe.
5. `newtype_id!`-generated items never appear in `inventory.md` — if true the
   code map lies by omission about every id type.

Task 11 review, first attempt: died on the session rate limit before changing
anything (tree verified clean at b22d916). Re-dispatched on opus with the same
brief.

Controller's own independent check while it runs (user asked me to verify),
read-only, `src/protocol/sse.rs`:

A. CONFIRMED — concern 3 is real. The bus is `(OperationId, StreamItem)`; no live
   event carries a `seq`. Durable events committed after the replay's last empty
   read (OperationStarted, OperationCompleted, OperationCancelled, thread entries)
   are never delivered while connected. The code comment says "the client
   discards any live event whose `seq` it has already applied" — there is no
   `seq` on a live event to compare, so that comment describes a mechanism that
   does not exist. Subscribing first buffers only the transient bus, which is not
   where durable events travel. §2.10's no-gap handoff is not kept by this code.

B. NEW, not in the implementer's concerns — the live phase is not scoped to the
   thread. The loop forwards every `(op_id, item)` from the bus regardless of
   `q.thread_id`, so a client subscribed to thread A receives thread B's deltas,
   entries and turn ends. This is the "SSE events are not isolated between
   conversations" defect Codex named in its interrupted transcript; it was right.

Holding both for comparison against the reviewer's report before ruling. Not
sent to the reviewer mid-task (CLAUDE.md: compose the dispatch once).

Task 11 review (retry, opus) finished: commit a17d5e8 on b22d916. Controller
verified independently: diff read (shutdown watch in AppState, live loop selects
on it, tests/protocol.rs, codemap expands newtype_id!), `cargo test` = 58 passed
(summed per binary), tree clean. Reviewer independently reached A and B.

Ruling 47 — A (durable events never live) is fixed NOW, in a Task 11 fix round,
not deferred. Grounds: data-flow §2.4 already specifies "live publication after
commit"; this is conformance, not new design, and Task 12's client must not be
built against the current live shape. Shape: storage raises a committed-seq
signal after each committing write; `subscribe` takes it before the replay; the
live phase re-reads `read_events_after(last_seq)` on each signal and emits
`durable` events with seq. The transient StreamItem bus stays. The durable
`entry` duplicate on reconnect goes away because entries arrive as durable.

Ruling 48 — B (cross-thread leakage) is fixed in the same round: the bus payload
carries the thread id and the live loop filters on `q.thread_id`.

Ruling 49 — §8.5 gap (Graceful over a live tree / unwritten outcome / Pending
without handle / start_turn during stop) is DEFERRED to Task 13, which owns the
shutdown acceptance run. Runtime-design line 341 is the rule it must meet.
Reviewer's minors (NotFound code, Constraint never built, raw sqlx message,
lagged continues, harness_version no timeout, `sse` pub) go to the whole-branch
review list.

User decisions 2026-09-22 (evening):
- YES to three "Lessons from shadow" lines in CLAUDE.md (run before documenting;
  end-to-end path before abstraction; no layer before a user of it), added with
  the "controller verifies, doesn't re-review" line after the fix round lands.
- Mutation-proof question ("break the test" for critical tests only vs all) left
  UNANSWERED — keep current practice until the user rules.
- NEW requirement: a debug mode and logs before the first manual run. Today:
  RUST_LOG EnvFilter to stderr, ~10 log points, no request spans, no operation-
  transition or planner-lifecycle logs, no log file. Planned as a small task
  after the fix round (so it doesn't collide with the agent's tree): `serve
  --debug` (debug level + log file under the data dir, path printed at start),
  HTTP request spans, one log line per operation transition and per planner
  lifecycle step (spawn+pid, turn-end, stop, reap) carrying op_id/thread_id.
- The user runs the product WITH me: when ready to run, WAIT for the user. Do
  not run `shadows serve` manual acceptance alone.

Fix round (opus) finished: a81204c. Controller verified: diff read (storage
signal raised after COMMIT under the write lock, never on rollback; bus now
(ThreadId, OperationId, StreamItem); sse live re-reads journal on signal),
cargo test = 62, tree clean. Commit message of a81204c overclaims "every
operation transition" arrives — false, see ruling 50.

Ruling 50 — operation events are thread-scoped: only OperationCreated carries
thread_id today, so Started/Completed/Failed/CancellationRequested/Cancelled
never reach a thread's stream (replay or live). Fix in the next dispatch:
every operation event row carries its operation's thread_id. Makes a81204c's
message true.
Ruling 51 — ThreadEntryAppended keeps its {ordinal, kind} payload for M0; the
client refetches /entries on a durable entry event. No payload change.
Known limit accepted: the exactly-once handoff test catches "twice", not
"zero" (a commit needs an await, so it cannot be forced into the gap).
CLAUDE.md rule + "Lessons from shadow" committed and pushed with the branch.
Next dispatch (one agent, opus): ruling 50 + debug mode and logs.

2026-09-22 20:36Z — FIRST REAL RUN (user present). Commits fde7db7, e22ff1c
verified (diff stat, build.rs + migration 0002 read, 66 tests) and pushed.
`shadows serve --debug` (release) on 127.0.0.1:4318, db under %TEMP%\shadows-run.
Created project + thread over the API, one real `claude` turn: "Reply with
exactly: shadows is alive" -> AgentMessage "shadows is alive" in ~8s; entries
ordinal 1,2; log file shows every step (http, transitions Pending->Running->
Completed, spawn pid, first_output, turn_end success, exit 0).
Found by running:
1. The turn's detached task inherits the HTTP request span, so every planner
   line is prefixed http{POST .../turns} long after the 202 returned. Cosmetic
   but misleading; fix: spawn the watcher with the planner.turn span only.
2. The turn's cwd is the DAEMON's cwd (E:\Globalprojects\shadows), not a chosen
   project directory — a project has no path. The milestone says "select a
   local project". Must be ruled before/with Task 12.
3. index.html is a 2-line placeholder: nothing to use in a browser until Task 12.

2026-09-23: PR #2 (tasks 5-11, backend) MERGED into main as 5434de0 on the
user's call ("backend is ready: PR, merge, then plan the interface"). CI green
on both runs (Windows acceptance + Linux compile). Branch product-path deleted.
Whole-branch review did NOT run before this PR — user chose to merge; deferred
minors carry to the Task 12/13 branch. Decision: each project picks or creates
a folder; turns run in it. Next: plan the web client with the user before any
dispatch.

2026-09-23 — Web client direction (user decisions): React; the daemon does NOT
serve or embed a client (backend stays local, clients reach it over the
protocol, like a hosted web app reaching a local agent; desktop client later).
Stack approved: Vite, TanStack Router+Query, shadcn/ui (Base UI) + Tailwind v4,
Motion, Streamdown, utoipa/utoipa-axum -> OpenAPI -> openapi-typescript +
openapi-fetch; hand-written SSE hook. Spec §1 amended, plan Task 12 replaced by
12a/12b/12c (commit f087d7d on branch milestone-0/web-client). Mockup approved:
two panes, folder browser, near-black + dark purple.

Ruling 52 — conversation continuity is broken today: the thread never records
its harness session id; `start_turn` takes `resume_session_id` from the client
but the client is never told one, so every turn starts a fresh Claude session.
SERVE_STREAM_SPIKE Finding 3 recommended recording it on the thread. Fixed in
12a: the thread records its session id from the first turn; later turns resume
it; the request field goes away.

Task 12a (opus) finished: 5a9cff6, 78dab79, 792ff0b, ab7ef56, 1004dca. Controller
verified: 5 commits, 51 files +3801/-355, cargo test = 84 (sum), tree clean;
spec §1 amended in place by the agent (CORS flag, fs routes, openapi file).
Pushed branch milestone-0/web-client for CI. Agent freed ~14 GB (deleted
target/debug/incremental) because E: had 41 MB free; E: now 13.9 GB free.
Rulings on the agent's "decisions to review" — all ACCEPTED:
- Old projects: null directory, turn accepted then Failed at Prepare with reason.
- Session id recorded at the first turn's turn-end (not spawn), so a failed
  first turn never leaves an id --resume rejects.
- CORS refused-origin test checks only Access-Control-Allow-Origin (tower-http
  sends methods/headers anyway; the browser decides on origin).
- POST /api/fs/dirs has no command_id: no DB write, retry gets 409. Accepted.
- utoipa ToSchema derives on domain types: not persistence, purity rule holds.
Carried to whole-branch review: create thread under missing project -> 500;
axum pre-handler rejections are plain text; planner/mod.rs at 481 lines (split
next time it grows); fs hidden/unreadable paths untested.
No separate per-task reviewer for 12a (usage limit): the whole-branch review
before the PR is the review for 12a-12c.

Task 12b (opus) finished: cea351d, 0b6b371, 6fb37c2. Controller verified: 3
commits, tree clean, no stray node process, vitest 14 passed, tsc -b 0.
Rulings on 12b's report:
- Plan says web CI "before Rust"; it runs in parallel. ACCEPTED (parallel is
  fine; plan wording was loose).
- Oxlint instead of ESLint (template default). ACCEPTED.
- Daemon gaps the agent found, all go to 12c's brief (daemon + client), since
  12c is their first user (ruling 53):
  1. `durable` SSE frame lacks operation_id/thread_id -> client can't tell which
     operation ended; a stopped turn sends no turn-end.
  2. entries route returns no journal cursor (§2.10 snapshot+cursor) -> hook
     replays from 0 each open. Acceptable for M0; note it.
  3. no route to read an operation / a thread's operations -> reload can't know
     a turn is running.
  4. payload JSON-in-JSON; caught-up plain text; ErrorCode type-only. Minor.
User asked for a reviewer after 12b: dispatching a 12b reviewer (opus) now.
12b review (opus): f5aa2b0 (Important: refetch storm after reconnect fixed; lifecycle tests under StrictMode), 50441b3 (coverage). Controller verified: 2 commits, vitest 19 passed, tree clean; pushed. Reviewer carried to 12c: reply flicker between turn-end and refetched entry.

Task 12c (opus) finished: a7fae80 (daemon: durable frames carry operation_id/
thread_id, payload is an object, caught-up is {seq}, GET threads/{id}/operations),
db66105 (turn state, no-blink reply, command-id reuse), 69afd79 (the two screens).
Controller verified: 3 commits, 62 files +4777/-152, cargo test 86, vitest 46,
tsc 0, tree clean; pushed. Noted for whole-branch review: main JS chunk 1.27 MB
(@streamdown/code) -> lazy-load; projects/threads ordered by created_at text
(same sub-second sort flaw the agent avoided for operations); agent entries
don't record their operation; Escape in new-folder box may close dialog.
Next: first manual run WITH the user (fresh db).

2026-09-23 — FIRST MANUAL RUN OF THE FULL PRODUCT (user drove it): new project
via folder browser, conversation, streamed reply, Stop mid-turn, reload during a
run — user reports all steps work ("تمام التمام"). User asked about a native
Windows folder dialog: explained browsers can't yield a path; offered a
daemon-side "Browse…" (rfd) as a local-only option — not requested yet.
Remaining for Milestone 0: Task 13 (process-tree confirmation, restart recovery,
§8.5 shutdown gap, Linux gap report), whole-branch review, PR, merge.

Task 13 code (opus): 0cea4c0 (§8.5: Graceful only over terminal ops; stop returns
5 outcomes; start refused during shutdown 503 RUNTIME_STOPPING; second Ctrl-C
escalates; bounded 10s confirmation wait -> Escalated), 560663c (bystander,
tree-vs-tree, turn-vs-turn isolation tests). Controller verified: cargo test 97,
tree clean; pushed.
Ruling 54 — §8.5 tension: the bounded wait adds a timeout constant §8.5 argues
against. KEEP the bound (it can only produce Escalated, never a false Graceful;
an unbounded wait would hang a Ctrl-C forever when a reader is stuck); amend
§8.5 in place to say so. Given to the whole-branch reviewer.
Given to the reviewer to verify+fix: client disconnect during start_turn/stop may
drop the handler future mid-work (§8.4 case 7); web Stop can't be retried after
PROCESS_TERMINATION_FAILED.
User chose: ONE whole-branch reviewer now (12a was never reviewed; 12c only by
its author) instead of a Task-13-only reviewer.

Whole-branch review (opus): 12 commits cc781fc..061f0ab. CRITICAL fixed: any
website could make the daemon act (CORS doesn't stop the request) -> origin/host/
Sec-Fetch-Site guard, 403 ORIGIN_REFUSED (spec §1, §3.4). Important fixed:
disconnect strands work (§8.4 case 7), raw sqlx text leaked, stderr pipe never
read (could hang a turn; from main), web Stop retry. Controller verified: 12
commits, cargo test 107, vitest 48, tree clean; pushed.
Ruling 55 — left-open Important items go to the acceptance record as known gaps
for the next milestone, not fixed now: (1) a harness that closes stdout but keeps
running can't be stopped (reader holds the handle); (2) start_turn carries no
command id (CLAUDE.md idempotency).
Next: manual acceptance run with the user (fresh db), then ACCEPTANCE.md,
status.md, PR, merge.

ACCEPTANCE RUN (2026-09-23 ~21:47Z, user driving, Windows, db %TEMP%\shadows-accept):
STOP PROOF — op 5ea0b0f5 thread 660960d4, cwd F:\abdo. Log: spawn pid=15928 ->
CancellationRequested -> planner.stop -> process.terminate pid=15928 -> exit
code 1 -> "tree reaped" -> OperationCancelled. Independent check after: CIM
query for pid 15928 or ParentProcessId 15928 = (empty). Bystanders: user's
claude.exe snapshot before = 19, alive after = 19 of 19. Durable row (via GET
/api/threads/{id}/operations; no sqlite3 CLI on machine): status_kind=Cancelled,
cancel_requested_at 21:47:50.48, finished_at 21:47:50.58.
RECOVERY PROOF — op d9310aac (thread 660960d4). Turn tree 6s in: 31868/claude.exe
+ 32892/mxMCPProxy.exe (a real grandchild: claude spawns its MCP proxy — first
measured answer to SERVE_STREAM_SPIKE's open "does claude spawn a tree?").
Daemon pid 13176 force-killed (Stop-Process -Force, not Ctrl-C) at
2026-09-23T21:51:12Z; 2s later CIM query for 31868 or its children = (empty):
Job Object kill-on-close took the grandchild too. Restart: recovery.reconcile
interrupted=1 anomalies=0; row status_kind=Interrupted, interrupt_reason=
PreviousRuntimeEndedDuringRun. Entries 32, ordinals 1..32 contiguous. First
attempt did not count: turn completed (26s) before the kill.
