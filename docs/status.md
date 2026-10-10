# Project Status

**Updated:** 2026-10-10 (§23 PR 1 merged into `main`)

This file says where the project is. It decides nothing — the design and every
decision live in the topic owners indexed by
[`specs/README.md`](./superpowers/specs/README.md), and this file must never
restate them.

## Where we are

**§23 PR 1 is on `main`**, merged through PR #24 as `ced2095` on 2026-10-10.
Compiled-in base standards, versioned project additions, the Standards tab,
snapshot-derived stages, conversation/draft stage headers and per-turn
Planner standards updates are built. Windows verification passed 445 Rust
tests (one manual test ignored) and 231 Web tests, both clippy modes,
production feature isolation, formatting/width, generated API consistency,
typecheck, lint and build. A temporary MiniMax setup through the real ACP
adapter completed a turn and named the base parts plus the saved `billing`
addition; its observed model was `MiniMax-M3`. Credential-bearing trial
scripts were removed; normal daemon configuration was restored.
The browser trial remains partial: Standards save/reload and Vision save
were observed, while in-app browser-control policy prevented further UI
automation. The delayed cache-response race found during independent review
was fixed. The completed whole-branch review reported no new confirmed
defects and freshly passed 66 distinct Rust tests and 15 targeted Web tests;
it did not rerun the full gate because it made no code changes.
Post-merge CI on `ced2095` passed the Windows acceptance gate, the Web client
build gate and the Linux compile gate. Linux runtime acceptance is not claimed.
The merged branch was deleted locally and from the remote.
See [the dated record](./evidence/2026-10-10-guided-planning-pr1.md).
PRs 2 onward and §24 enforcement are not implemented by this slice.

**§23 and §24 are specified, and §23's PR 0 is merged**
(2026-10-08, `d963fc4`, PR #23). The owners are
[`§23`](./superpowers/specs/2026-10-08-guided-planning-design.md), guided
planning from scratch, and
[`§24`](./superpowers/specs/2026-10-08-approval-authority-design.md), who
approved. PR 0 is §23.8's groundwork: tool lines and subagent cards are their
own entry kinds, `ToolCall` and `Subagent`, with the card in a column
(migration 0020); `StorageError`, the HTTP failure mapping and
`storage_contract` are split by job. The full gate ran once: **430 Rust
tests** and **225 web tests** passed, with both clippy modes, the feature
check and the 100-column check clean. The browser run used a copy of §22's
test database: its old card and 29 tool lines came through the migration
unchanged on screen; a live turn wrote a `ToolCall` and a `Subagent`, both
survived a reload, and a fork kept both cards; the daemon log held no error.
The whole-branch review has not run; Mohammed will say when.

**§22, subagent cards, is merged** (2026-10-08, `ce52f1f`, PR #22); CI's
Windows job runs again. The owner is
[`§22`](./superpowers/specs/2026-10-05-subagent-cards-design.md). The full
gate ran: **428 Rust tests** passed, and the web suite with typecheck and lint
clean. The browser run used a copy of the dev database and the real adapter:
a subagent showed as one card, Running with its count of finished tools, then
"Done · general-purpose · Sonnet 5.5 · 12s · 32k tokens · 4 tools"; its panel
showed the prompt, three steps and the report; the card survived a reload, and
no inner tool wrote a line of its own. The branch also sets the build: `rust-lld`
on Windows, line tables only, no doctests — a clean build takes 222 s, the
full test run 304 s, a one-file rebuild 116 s, and the target directory stays
at 9.2 GB. The branch review found one bug, fixed: a late update to a card
already written opened a second, empty card.

**§21, the `/` menu, is merged** (2026-10-05, `8efcea1`, PR #21). The owner is
[`§21`](./superpowers/specs/2026-10-05-slash-menu-design.md). The full gate
ran: **425 Rust tests** and **217 web tests** passed, with both clippy modes,
the feature check and the 100-column check clean. The browser run used a copy
of the dev database and the real adapter: `/` listed Claude's skills and
commands, `/br` found brainstorming, a picked entry showed its hint greyed
after it, `/context` ran both sent and queued with Send now, and the daemon log
held no error. It found three things, fixed on the branch: Chrome's
`scrollIntoView` returning a Promise crashed the conversation when the menu
opened; the description ran off a narrow window; and `/model` sent as text
left the picker on the old model, so `/model` and `/effort` now act through
the pickers (Mohammed's ruling, §21.4).

**§20, writing while a turn runs, is merged** (2026-10-05, `e02a6ad`, PR #19). The owner is
[`§20`](./superpowers/specs/2026-10-05-message-queue-design.md). The full gate
ran once on `fb2246f`: **422 Rust tests** and the web suite passed, with both
clippy modes, the feature check and the 100-column check clean. The browser
run used a copy of the dev database and the real adapter (Opus 5.5 low, then
Sonnet 5.5 medium):

1. Two messages queued during a running turn showed as Waiting; when the turn
   completed each was sent in order as its own turn.
2. Send now during a streamed story: the story stopped mid-word, the message
   sat below the text streamed before it, and the reply ("BANANA") ended the
   same turn (thread entries 8, 9, 10 on one operation).
3. Stop with a message waiting: the turn ended Stopped, the message stayed
   with **Send**, and Send ran it.
4. Two conversations in two projects ran at the same time; both completed.
5. The daemon log held no error besides the Stop's own `cancelled` end.

The run found one bug, fixed in `7079652`: a second message typed while the
first was being queued was wiped. With two tabs and the daemon's own page open,
a send stalled with "The daemon is not reachable" while the daemon logged no
request; closing the extra tabs cleared it. Cause: the tabs' event streams
filled the browser's six-connection pool for the daemon's host. Fixed in
`838ddf7`: a hidden page holds no stream (§2.10); measured, the same request
went from an 8 s stall to 4 ms.

**Project plans, §16 1a, are on `main`**, squash-merged through PR #14 as
`4d8774e`. The owner is
[`§16`](./superpowers/specs/2026-10-01-project-plans-design.md).
Plans are shared across a project's conversations, versions show their
writers and reasons, and the web client supports Archive, Continue and
Delete with retained read-only conversation history. Project events refresh
plan views, and each open session has its own lock.

The recorded Task 6b gate on 2026-10-02 passed **368 Rust tests and 161 web
tests** (commit `51f6a2b`). Its Windows browser check used fake ACP and a
scratch database: Delete left the list and opened the draft, the plan stayed,
and its deleted writer opened history without a composer. A fresh ordinary
debug daemon build from that commit also passed. Mohammed then confirmed
that **Open plan works with the rebuilt daemon** and attributed the earlier
error to the old daemon. This is a partial human check; the full §16.12
acceptance run remains pending
(Windows record: `docs/evidence/project-plans/WINDOWS_RUN.md` at `92e6dae`).

The whole-branch review fixed archive transaction checks, removal of a
Pending turn before live registration, shared-plan task focus, removed-session
reuse, Continue retries and Web action feedback/cache refresh. Its full gate
ran once: **371/372 Rust and 162/164 Web tests passed**. The three failures were
test synchronization issues; the affected files passed **3/3 Rust and 11/11
Web** after correction. The full suites were not rerun. Formatting, both
clippy modes, production feature isolation, API consistency, Web typecheck
and lint passed. The controller clarified §16.3 to match the existing
contract: approval of a version whose writer conversation is deleted emits
`WorkflowFrozen` without a thread and writes no conversation entry; the
version keeps its writer. No runtime behavior changed.

On 2026-10-03 the ordinary debug daemon was rebuilt from review commit
`fbbdde55`. Mohammed then reported that he tried the application and believed
everything was ready. This records his successful Windows trial of the
reviewed build; it does not claim that Codex independently executed every
§16.12 scenario (Windows record: `docs/evidence/project-plans/WINDOWS_RUN.md` at `92e6dae`).
It merged as PR #14.

**Code-index settings, effort at once, and a Linux watcher fix** (PR #12,
2026-10-01; 358 Rust, 156 web tests). The changes:
- Project settings has a Code index section (status, links, Link a
  project…) and a Remove project zone; a global Settings page sets the
  active limit (§15.9, §13.11);
- a picked effort is set on the session at once, the effort is remembered
  per model, and Claude's `default` is no longer offered: Shadows advertises
  the adapter's `recommendedValue` (§12.4, §12.7, migration 0011,
  `docs/evidence/harness/EFFORT_DEFAULT_PROBE.md` at `92e6dae`);
- the code watcher ignores file reads, which Linux reports: before, an idle
  daemon re-scanned forever at 148% CPU (§15.4,
  `docs/evidence/milestone3/LINUX_WATCHER_READS.md` at `92e6dae`).

It ran in a Linux cloud container, in a browser against the real adapter
and Claude Code, on a copy of a scratch database. On 2026-10-01 it ran on
Windows against a copy of Mohammed's dev database, and then on the database
itself. Migration 0011 applied, and a remembered `low` effort came through
it. The effort menu offered Low to Max without `default`, and High was set
on the session at once (`PUT …/session/effort`, 200). The Code index section
read "Ready · 296 files". Remove project was refused with 2 conversations.
Active projects saved 8, and 8 was still there after a reload. The web test "saves the active limit" failed on Windows
only. Its fake daemon answered 5 to every read, so the refetch after a save
brought 5 back. It now answers what was saved, and 156 web tests pass on
Windows.

**The web client after Mohammed used it** (PR #11, 2026-10-01; 350 Rust,
142 web tests). The changes:
- the sidebar folds like the Claude app, with a `+` per project;
- a new conversation is a draft until its first message;
- a conversation is titled from its first message, then by the title Claude
  Code generates (§4.2, §12.3);
- Arabic and English each read in their own direction (§13.11);
- the allowed modes moved into Project settings (§12.11).

It ran against a copy of the dev database. That run found "New session", a
placeholder title Claude Code sends, which is now ignored.

**A project can be removed** (PR #10, 2026-10-01; 344 Rust tests): a soft
removal, refused while the project has threads (spec §4.2), which drops its
code index, links and grants (§15.4). It ran on the dev daemon on Windows: a
throwaway project was indexed, linked both ways, removed, and replayed; it
left the list, its links and its active slot (`code.removed`), and a question
naming it answered 404. Its web control came with PR #12.

**Milestone 3 (§15, the code index) is on `main`** (PR #9, 340 Rust tests)
and ran on Windows on 2026-09-30:
§15.11's five acceptance steps passed
(`docs/evidence/milestone3/WINDOWS_RUN.md` at `92e6dae`).
It adds the crate `shadows-index`, the ninth service `Code`,
and the MCP tools `where_is`, `who_uses` and `outline` for an external agent's
grant, with their HTTP routes. The probe's measurements are in
`docs/evidence/milestone3/PROBE.md` at `92e6dae`.

**Milestone 2.5 (§14, one application core) is on `main`** (PR #7,
2026-09-29; 328 Rust tests). Shadows is now a Cargo workspace under `crates/`:
the HTTP and MCP adapters are their own crates and reach the application only
through `AppCore` in `shadows-core`, whose eight services each own their
operations and carry a `contract.yaml` that
`crates/shadows-core/tests/contracts.rs` keeps current. Behaviour did not
change, and `api/openapi.json` is byte-identical. Mohammed ran it on Windows:
six Planner turns completed and the Planner used Shadows' MCP server; Stop, a
restart, Approve and Revoke were not exercised in that run
(`docs/evidence/milestone2_5/WINDOWS_RUN.md` at `92e6dae`).
Its execution ledger was deleted after the merge.

**Milestone 2 (§13, the Planner writes a plan) is on `main`** (PR #6,
2026-09-25; 304 Rust, 135 web tests), and Mohammed ran it on Windows. A
12-task plan, Approve freezing v1 while an edit made v2, project instructions
reaching both a new and an existing conversation, and Connect and Revoke from
an external Claude Code all worked. The run found that a harness opening took
up to 5.7 s against a 5 s bound, which is now fixed:
`docs/evidence/milestone2/WINDOWS_RUN.md` at `92e6dae`.
Its execution ledger was removed from the branch before merge.

**Milestone 1 is implemented, Phase A and Phase B, and both have run on
Linux against the real harness.** A Planner turn is an ACP `session/prompt` on
an adapter each open thread keeps (spec §12). The person chooses the CLI per
conversation and the model, mode and effort per message, all read from the
harness. The composer shows context and limits. Any message can be copied, and
the last one forked. The Phase B run found four defects, all fixed and run
again: `docs/evidence/milestone1/PHASE_B_RUN.md` at `92e6dae`.
The Phase A run is `docs/evidence/milestone1/PHASE_A_RUN.md` at `92e6dae`.
**Mohammed ran it on Windows:** send, Stop (56 ms to `Cancelled`) and a
daemon restart with the conversation remembered all worked:
`docs/evidence/milestone1/WINDOWS_RUN.md` at `92e6dae`.
Fork, the permission-refused line and the breakdown were not checked item by
item.

- Milestone 0 is complete on Windows and on `main` (PRs #1-#3).
  `docs/evidence/milestone0/ACCEPTANCE.md` at `92e6dae`.
- Milestone 1 is on `main` (PRs #4 and #5). The
  whole-branch review ran before the merge.
- Mohammed's three rulings after the run are built and ran on the real
  harness (spec §12.5, §12.6/§12.9, §12.7): the mode menu says what Accept
  edits allows, a fork is locked to its harness, and a chosen model is set at
  once.
- The daemon serves an API only; the React client in `web/` is a separate
  application (spec §1). Its types are generated from `api/openapi.json`.
- 202 Rust tests and 95 web tests.

The PR is #4; its execution ledger was removed from the branch before merge.

## What has been measured

- **Milestone 0 acceptance on Windows.** Gates green; Stop terminates only the
  turn's own tree while 19 unrelated `claude.exe` processes survive; a daemon
  killed mid-turn takes the harness and its grandchild with it through the Job
  Object, and restart records the turn `Interrupted` with every entry intact.
  `docs/evidence/milestone0/` at `92e6dae`.
- **Phase A over ACP, on Linux.** Adapter 0.81.1 over Claude Code 2.1.281:
  the harness itself confirmed a Stop (`cancelled`) and the adapter lived on,
  and a restarted daemon resumed the recorded session. Read-only shell commands
  are allowed by Claude Code without asking. `docs/evidence/milestone1/` at `92e6dae`.
- **Phase B on Linux.** The model list, each model's efforts and the modes all
  come from the session. The adapter needs `PATH` and `HOME` from the daemon, or
  no shell command runs. Accept edits runs file commands in the project folder,
  `rm` included, without a permission request. `docs/evidence/milestone1/` at `92e6dae`.
- **The harness does spawn a tree.** `claude --print` starts an MCP proxy as a
  child. This settles the question the serve/stream spike left open; whether a
  `Bash` call adds more descendants was not measured separately.
- **Persistence.** SQLx 0.9 + SQLite chosen; the delta validated SQLx and
  SeaORM 2.0.3 against PostgreSQL 16. `docs/evidence/persistence/` at `92e6dae`.
- **Harness stream contract.** Measured against Claude Code 2.1.278: four stream
  classes, of which only `assistant`, `user`, and `result` are durable; turn end
  is an explicit `result` line; `--session-id`/`--resume` give continuity across
  processes. `docs/evidence/harness/` at `92e6dae`.
- **Harness binary identity.** The machine carries more than one `claude-code`
  installation at different versions; spec §1.4 requires an explicit path and a
  recorded version.
- **SQLite writer strategy and `durable_seq` ordering.** One write connection
  plus `BEGIN IMMEDIATE`; no visibility inversion. Spec §6.23 and §6.18.
  `docs/evidence/persistence/` at `92e6dae`.

## Next

**The next guided-planning slice is §23 PR 2, vision proposals** (§23.9).
PR 1's browser acceptance remains partial as recorded above. §18 Stage 3,
focused Planner context, has not started and remains later work in the full
delivery roadmap. The four
items below are on `main`, merged through PRs #15 and #18 (`c080fba`,
`92e6dae`); no branch is open. The
[delivery roadmap](./superpowers/plans/2026-10-03-project-planning-roadmap.md)
retains the full local, concurrency and company sequence. Executors, the
manager and specialized diagrams are still unimplemented.

1. **Structure hygiene, merged.** Four changes that
   preserve product behaviour, specified in
   [`2026-10-04-structure-hygiene-design.md`](./superpowers/specs/2026-10-04-structure-hygiene-design.md):
   a line-length check for the format gate's blind spot, one hook shared by
   the part and outcome editors, one dead export removed, and six merged plans
   deleted. All four are on `main`, and CI runs the line-length check. The shared editor retains
   the existing retry and revision-conflict behavior. OpenAPI documentation
   and generated client declarations were refreshed together after rewrapping
   a route's comment. Final Windows verification passed: **385 Rust tests
   and 180 Web tests**, formatting, both clippy modes, production build,
   production feature isolation, Web typecheck, lint and build. The manual
   database-copy migration test remains explicitly ignored in the default
   suite. The line-length step accepted 100 Unicode characters with CRLF,
   rejected 101 with file/line/width, and accepted the repaired file. No
   independent branch review or new human browser acceptance is claimed.
2. **Planning workspace Stage 1, merged.** The [full delivery roadmap](./superpowers/plans/2026-10-03-project-planning-roadmap.md)
   and [§18 draft](./superpowers/specs/2026-10-03-project-planning-workspace-design.md)
   are written, together with the [Stage 1 implementation plan](./superpowers/plans/2026-10-03-planning-workspace-stage-1.md).
   Its Vision, nested parts, Roadmap outcomes and existing-plan associations
   are implemented. Tasks 1–3 received independent review; Task 4 added
   migration/stream integration checks, corrected PartEditor retry and ran an
   agent-operated Windows browser trial on an isolated database. Rust passed
   385 tests (one manual backup test ignored by default, separately executed);
   Web passed 179 tests. The [dated record](./evidence/2026-10-04-planning-workspace-stage-1.md)
   distinguishes the empty actual dev-DB copy from populated synthetic legacy
   fixtures. No separate record of Mohammed's acceptance or a whole-branch
   review was written before the merge.
3. **§16 1b, merged:** the [scoped plan](./superpowers/plans/2026-10-04-cross-plan-links-1b.md)
   is executed. Latest-task dependencies, grant-scoped related reads,
   project map, broken-link graph navigation and dependency notifications
   are implemented above the user checkpoint `dfe6070`. The final gate passed:
   **402 Rust tests (one ignored) and 187 Web tests**, both clippy modes,
   production build/feature isolation, Rust format/Unicode width, Web
   typecheck/lint/build and generated API consistency. The
   [dated evidence](./evidence/2026-10-04-cross-plan-links-1b.md) records an
   isolated Windows production-preview trial with real HTTP/MCP requests,
   reciprocal graph navigation, refused broken-link approval and retained
   state after restart. It does not establish human acceptance, Linux
   containment or an independent branch review. §17 remains a draft for later
   task contracts and execution evidence; the executive manager needs its
   later design after Mohammed explains the rest of its responsibilities.

4. **§18 Stage 2, merged, 2026-10-05:** shared agreement lifecycle,
   validation, impact review, exact task pins, HTTP/MCP and browser adoption
   are implemented with migrations 0017/0018. The
   [implementation plan](./superpowers/plans/2026-10-04-shared-api-agreements.md)
   remains open. The [dated evidence](./evidence/2026-10-05-shared-api-agreements.md)
   records 411 Rust tests (one ignored), 188 Web tests, later focused checks,
   the native two-plan journey/restart and migration of an actual dev-DB copy.
   Complete suites were not rerun after the final focused changes: Mohammed
   requested stopping tests and saving a checkpoint. An agreement edit now
   checks the exact Draft version and the expected revision together, so a
   delayed request across a version rollover is refused
   (`crates/shadows-core/src/design/store/agreement_write.rs`). Stage 3 has
   not started.

## Standing risks

- **Linux parent-death containment is not implemented.** The Linux run and CI
  stop the daemon cleanly; neither proves that the adapter dies with a crashed
  daemon (spec §1.5 OPEN block). Never generalize a Windows
  result into a Linux claim.
- **A harness that stops answering but keeps running** was the Milestone 0 risk.
  Stop no longer waits on a stream: after `cancel_wait` it terminates the
  adapter's tree (§12.3). `fake-acp`'s `ignore-cancel` covers this; the real
  harness was not driven into that state.
- **Accept edits lets Claude Code delete files in the project folder without
  asking** (spec §12.5, measured in the Phase B run). This is the harness's
  behaviour, kept by decision; an uncommitted file it deletes is lost.
- **No authentication.** The daemon binds to loopback and refuses cross-site
  browser requests, but any local process can call it. Remote access is an OPEN
  block in spec §1 with its trigger.
- **Remove mx.** Its hooks are disabled, not deleted. The hook scripts, `mx*`
  skills, permission lines, the `mxai-knowledge` MCP server, and the mx block in
  the global `CLAUDE.md` still need removing.
