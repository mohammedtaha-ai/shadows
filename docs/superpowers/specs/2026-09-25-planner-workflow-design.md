# Section 13 — The Planner Writes a Plan (Milestone 2)

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there.

- **Date:** 2026-09-25
- **Status:** Designed with Mohammed on 2026-09-24/25, section by section, with
  an independent review of every section. Not yet built.
- **Builds on:** Milestone 1 (§12), on `main` at `24a2ea5`, run on Windows
  (`evidence/milestone1/WINDOWS_RUN.md`).

Milestone 2 is the first slice of §11.6: **a plan the Planner writes, a person
sees as a graph, and a person approves.** The Planner writes the plan through
Shadows' own MCP tools while it talks with the person; every change is checked
and saved as it happens. The same MCP server lets a Claude the person started
elsewhere read and edit plans. Nothing is executed: a plan approved here is
frozen as a plan, not declared runnable.

## 13.1 Scope

In:

1. **The plan** — a versioned `workflow` of numbered tasks and two kinds of
   link, owned by one planning conversation (§13.2, §13.3).
2. **Checks** on every edit and on approval, including cycles across both link
   kinds (§13.4).
3. **Commands that survive retries and concurrent writers** (§13.5).
4. **Shadows' MCP server** at `/mcp`, for the internal Planner and for an
   external agent (§13.6), under **grants** Shadows issues (§13.7).
5. **Planner instructions**: built into Shadows, plus per-project instructions
   edited in the web client (§13.8).
6. **Showing a plan inside the conversation** on request, and pointing at a
   task in the next message (§13.9).
7. **A Workflows section** in the web client with the plan drawn as a graph,
   approval, and project settings for instructions and external agents
   (§13.11).

Out:

| Not in this milestone | Where it goes |
|---|---|
| Running a task, task states beyond `Pending`, which CLI runs a task, per-task effort | §11.6 slice 2 (scheduler and execution) |
| Gates, `verification_check` rows, turning an acceptance item into a runnable check | §11.6 slice 3 (verification) |
| A roadmap above plans | A later slice, agreed 2026-09-24 |
| Phases grouping tasks | When plans grow large enough to need them |
| Editing a plan by hand in the web client | Later; edits go through the conversation |
| User-supplied MCP servers and plugins passed to agents | A later slice |
| A project-level event stream | When a screen needs changes live that polling cannot serve (§13.10) |
| Remote access to `/mcp` | §1's remote-access OPEN block |
| Restricting the Planner's Claude tools | Mohammed's ruling: the Planner keeps every tool Milestone 1 gives it |

## 13.2 The plan and its versions

A plan is the project-owned entity of [§16.2](./2026-10-01-project-plans-design.md#162-the-plan).
Its chain of `workflow` rows holds its **versions**; §6.8's lineage stays
within that plan.

- **`version`** — the plan's position in its chain: 1, 2, 3, unique and
  consecutive within the plan. "Plan v2" always means this number.
- **`revision`** — a counter of edits to one version, starting at 0 when the
  version is created and increased by every successful edit. Writers send the
  revision they read as `expected_revision` (§13.5). A revision is never shown
  as a version.
- **`title`** and **`goal`** — the plan's name and what it is for. They head
  the graph (§13.11).

States used in Milestone 2 are `Draft` and `Frozen` (§2.5's lifecycle, §4
`WorkflowState`):

- A plan has **at most one `Draft`** at a time (§16.2).
- **Approval is one step:** a person's approval moves a `Draft` to `Frozen` and
  sets `frozen_at`. `Approved` is not used in this milestone; it stays in the
  state set for the day execution needs a state between the two.
- **The web client calls `Frozen` "Approved".** The stored state and the word a
  person reads are deliberately different; nothing else maps them.
- **A frozen version never changes.** A later change starts a new version with
  `draft_start` (§13.6), which copies the frozen version's tasks and links with
  the same task numbers into a new `Draft` whose `previous_version_id` is the
  frozen one.
- **`Frozen` means frozen as a plan.** It does not mean every `TaskContract`
  holds what execution will need. When execution is built, its own readiness
  check runs; whatever a contract lacks is added in a new version, because a
  frozen version is never unfrozen.
- **Approval is a person's action in the web client** (§13.10). No MCP tool
  approves, for the internal Planner or for an external agent, and the
  Planner's instructions say approval is the person's (§13.8).

> **OPEN — the server cannot tell a person from a local process.** The
> approval route, and the routes that issue grants and save instructions, are
> HTTP API routes, and §1's request guard admits any request without browser
> headers (`curl`, a script). The Planner keeps every Claude tool (§13.1),
> shell commands included, so a Planner that runs a command could approve its
> own plan or issue itself a project grant. "Only a person approves" is
> therefore a rule of the tools and the client, not a guarantee the daemon
> enforces.
>
> **Why Milestone 2 accepts it (Mohammed, 2026-09-25):** a frozen plan runs
> nothing, and a wrong approval is undone by drafting the next version. A grant
> a Planner issues itself still cannot approve.
>
> **Trigger that closes this:** whichever comes first — the execution slice,
> where approving a plan starts real work, or §1's remote-access OPEN block.
> Either needs requests the daemon can attribute to a person; the likely shape
> is a person token the web client obtains by entering a code the daemon
> prints in its own terminal.

## 13.3 Tasks and links

**A task** (§6.9) gains a **`number`**: a positive integer, unique within its
version, shown as `T4`. A new version keeps every copied task's number, so T4
in v2 is the task that was T4 in v1. Writers choose the number (§13.5); tools
name tasks by number, not by id.

In Milestone 2 a task holds, in the two places §6.9 already gives it:

| Field | Stored in | Meaning |
|---|---|---|
| `title` | `contract_json` | Short name shown on the graph |
| `goal` | `contract_json` | What done means |
| `acceptance` | `contract_json` | See below |
| `reads` | `scope_json` (§4 `DeclaredScope`) | Paths the task reads |
| `writes` | `scope_json` | Paths the task may change |

The paths are the task's declared scope and have exactly one owner,
`scope_json`, so what the graph shows is what execution will read.
`acceptance` is a list of items, each with a number within the task and a
sentence someone can check.

§4's `TaskContract` also lists expected outputs, network capability,
timeout/budget and required checks. They are not asked for here, so the Planner
is never pushed to fill fields with filler; execution adds them (§13.2).

**A link** (§6.10's `task_parent`, one row per link) gains:

- **`kind`** — `needs` or `completes_after`:
  - `B needs A`: B does not start until A is complete.
  - `B completes_after A`: B can be built and finished now, but the acceptance
    items the link names only hold once A is complete. The link lists those
    item numbers; it names at least one.
- **`label`** — a few words saying what passes from A to B. Drawn on the edge.

A task may have at most one link of each kind to the same other task.

Acceptance items that wait are not `verification_check` rows: a check there is
a runnable build or test bound to a task or a gate (§6.12), and turning an
acceptance item into one is the verification slice's job.

## 13.4 Checks

One validator in `workflow/` answers both levels, so the web client's list of
what blocks approval and the approval route's refusal never disagree.

**Every edit** (the final state of a `plan_edit`, §13.6) must hold:

1. every link names two tasks of the same version, and not the same task twice;
2. every `completes_after` link names acceptance items that exist on the task
   that waits;
3. task numbers are unique;
4. **no cycle across both kinds.** Each task is two events, *start* and
   *complete*, with start before complete. `B needs A` orders complete(A)
   before start(B); `B completes_after A` orders complete(A) before
   complete(B). The combined order must be acyclic. Example refused: `T2 needs
   T4` together with `T4 completes_after T2` — complete(T4) → start(T2) →
   complete(T2) → complete(T4).

**Approval** additionally requires a title, at least one task, and on every task
a goal and at least one acceptance item. A `Draft` may lack these while it is
worked on; the list of what is missing is part of the plan read (§13.10).

**A batch** (`plan_edit`'s list of operations) is applied in order, and the
final state is checked, so a task can be added and linked in one call. Two
operations on the same task, or on the same link, in one list are refused
rather than resolved by order. Removing a task that still has links is refused
("T4 is still linked from T2"); the links are removed explicitly, in the same
list if wanted. Nothing is removed implicitly.

## 13.5 Commands, retries and concurrent writers

Every write — an MCP write tool or an HTTP route — is a command under §4's
idempotency rule: principal, command scope and command id identify it; the
fingerprint (command kind, command schema version, normalised arguments)
decides whether a repeat is a replay.

**Order of work for a write:**

1. the grant is valid and in scope (§13.7), for MCP calls;
2. a recorded command with the same key is looked for. Same fingerprint:
   its recorded outcome is returned, before any revision check — the
   revision moved because of that very command. An explicit `command_id` with
   a different fingerprint is `CommandConflict`;
3. `expected_revision` must equal the version's current revision, or the
   command is refused with `REVISION_CONFLICT` carrying `current_revision` and
   a summary of the changes since. The summary helps; a writer may still need
   `workflow_get` before rebuilding its edit;
4. the change, the revision increase, its durable event, the command record
   and its outcome commit in one transaction, and the grant is checked again
   inside it (§13.7).

**The revision check is strict, with no exception for a writer's own earlier
edits.** Two calls from one writer based on the same revision may still
overwrite each other (each `task_update` carries the whole task). Parallel changes
go in one `plan_edit`, which moves the revision once.

**Who names the command.** A model is not asked to invent ids or to reuse one
on a retry. When a caller sends no `command_id`, Shadows derives it:

| Principal | Command | Derived command id |
|---|---|---|
| The thread (internal Planner) | a write with `expected_revision` | the expected revision + the fingerprint |
| The grant (external agent) | a write with `expected_revision` | the expected revision + the fingerprint |
| The thread | `plan_show`, `draft_start` | the running turn's `operation_id` + the fingerprint |
| The grant | `draft_start` | the `draft_ref` it passes, alone |

The principal is the **thread** for the internal Planner, not its grant, because
that grant is replaced whenever an adapter opens (§13.7) and a retry after a
resume must still find the first result.

A derived id carries the fingerprint, so the same request repeated is a replay
and a different one is a different command: a second edit on a stale revision
goes on to step 3's `REVISION_CONFLICT`, and a second `plan_show` in one turn
for another task shows it. A `draft_ref` is the one derived id without the
fingerprint: it names one intended plan, so the same ref with different
arguments is `CommandConflict`, like an explicit `command_id`.

**`draft_ref`.** An external agent has no turn, and Streamable HTTP in MCP
2026-07-28 has no protocol sessions, so nothing in the request tells a retry
from a second, intended plan with the same name. `draft_prepare` returns a
`draft_ref` issued by Shadows; `draft_start` takes it. The same ref returns the
same plan; a new plan takes a new ref. A ref is bound to the grant that asked
for it and expires unused after one hour. `draft_prepare` itself is not
deduplicated: calling it again issues another ref, and refs never used simply
expire. The guarantee against a duplicate plan starts at `draft_start`.

## 13.6 Shadows' MCP server

- Served by the daemon at **`/mcp`** on the daemon's own listener, with `rmcp` (the
  official Rust SDK) over Streamable HTTP on the existing axum router. MCP is a
  separate interface from the HTTP API (§2.12): the two share the application
  commands and the validator, not a protocol.
- Claude Code 2.1.281 opens with `server/discover` at MCP `2026-07-28` and,
  from a server that does not know it, falls back to `initialize` at
  `2025-11-25`. `rmcp` 3.4.1 serves both: the first always statelessly, the
  second statelessly with `legacy_session_mode: false` (`MCP_PROBE.md` §4).
  Without protocol sessions, every request stands alone.
- §1's request guard applies unchanged: loopback only, `Host` checked, and a
  present, unlisted `Origin` refused with 403 `ORIGIN_REFUSED`, which the MCP
  transport requires.
- **Every request carries `Authorization: Bearer <token>`.** No token, an
  unknown one or a revoked one is answered **HTTP 401** before any tool runs.

**The tool list depends on the grant kind.**

The plan tools' reach and `draft_start` arguments are owned by
[§16.4](./2026-10-01-project-plans-design.md#164-the-planner-and-the-projects-plans).

| Tool | Planner (thread grant) | External agent (project grant) | Writes |
|---|---|---|---|
| `workflow_list` | ✓ (§16.4) | ✓ (§16.4) | no |
| `workflow_get` | ✓ (§16.4) | ✓ (§16.4) | no |
| `task_get` | ✓ | ✓ | no |
| `draft_prepare` | — | ✓ | issues a `draft_ref` |
| `draft_start` | ✓ (§16.4) | ✓ (§16.4), with a `draft_ref` | yes |
| `plan_edit` | ✓ | ✓ | yes |
| `plan_show` | ✓ | — | a conversation entry (§13.9) |
| `where_is` | — (§15.7) | ✓ the project and its links (§15.5) | no |
| `who_uses` | — (§15.7) | ✓ the project and its links (§15.5) | no |
| `outline` | — (§15.7) | ✓ the project and its links (§15.5) | no |

- **`draft_start`** uses `plan_id`, with its new-plan and existing-plan rules
  in §16.4 and its new-version reason in §16.3. `from_workflow_id` is no longer
  an argument. An external caller still requires a `draft_ref` (§13.5).
- **`plan_edit`** takes `expected_revision` and a list of operations —
  `plan_put` (title and goal), `task_add`, `task_update`, `task_remove`,
  `link_put`, `link_remove` — and answers the new revision. `task_add`
  refuses a number already in use and `task_update` one not in use, so a
  writer that meant to add a task never overwrites another by reusing its
  number; both carry the whole task.
- **An external `draft_start` from scratch creates no thread** (§16.4).
  Its writer is attributed under §16.3.
- The Planner's grant names its thread, which fixes its project. Plan reads,
  edits and shows follow §16.4; command identity remains §13.5's.

**Errors come in three layers:**

1. **HTTP 401** — authentication failed before a tool ran.
2. **A tool result with `isError`** and a symbolic code, once a tool has
   started: `GRANT_SCOPE` (a plan outside the grant's project),
   `GRANT_INVALID` (revoked while the call was in flight), `REVISION_CONFLICT`,
   `WORKFLOW_FROZEN_IMMUTABLE`, `WORKFLOW_VALIDATION_FAILED` (with the
   validator's list), `COMMAND_CONFLICT` (§13.5), and §3.4's
   `INVALID_COMMAND` for a request that cannot be done as asked — a task or
   plan that does not exist, an archived plan write (§16.2), a new version
   without a reason (§16.3), a Planner call
   that needs a running turn outside one. The text says what to
   do, e.g. "T9 does not exist in this plan".
3. **HTTP API status codes** (409, 422) belong to the HTTP API only (§13.10).

A tool the grant's list does not hold is unknown to that client.

## 13.7 Grants

A grant is Shadows' answer to "who may do what" on `/mcp` (§2.12's
`GrantedMcpContext`). Claude's own approval of a tool (§13.8) never widens it.

- **Stored as a hash.** The token is shown once and never stored in clear.
- **Checked twice**: when a request arrives, and inside the transaction that
  writes (§13.5), so a write that started before a revocation does not commit
  after it.

**The Planner's grant lives as long as its adapter.**

- It is issued before `session/new` or `session/resume` and passed with it
  (§13.8). If the session does not open, it is revoked at once.
- It is revoked when that adapter closes for any reason: the idle close, a
  forced stop, the daemon stopping. Starting the daemon revokes every
  internal grant left from before (recovery, §8).
- The web client does not manage grants. Opening a conversation can start an
  adapter (§12.2), and a grant comes with it.
- A change of project instructions (§13.8) touches neither the session nor its
  grant. A new adapter's grant reaches a resumed session because `mcpServers`
  is read on every resume (`MCP_PROBE.md` §3).

**An external agent's grant is bound to one project.**

- Project settings → **Connect** issues one and shows, once, a command to copy:
  `claude mcp add --transport http shadows http://127.0.0.1:4318/mcp --header "Authorization: Bearer …"`.
- **Revoke** makes Shadows refuse it immediately. It does not remove the
  server from the person's Claude configuration. Claude Code keeps configured
  headers in plain text in `~/.claude.json`; the settings page says so. This is
  accepted for a loopback-only daemon, a token bound to one project and unable
  to approve, that the person can revoke.
- A server that answers 401 with an empty body and no `WWW-Authenticate` is
  reported by Claude Code 2.1.281 as failed to connect with a 401. It tried
  twice and made no OAuth discovery request (`MCP_PROBE.md` §5). That was
  observed when a session opens; acceptance step 6 records what Claude Code
  shows when a token is revoked during a session.
- Issuing and revoking are durable events, recorded without the token.
- Issuing is an HTTP API route, open to any local process like approval is
  (§13.2's OPEN block).

## 13.8 Planner instructions and the session

**Shadows' instructions** are `crates/shadows-core/src/harness/prompt.txt`, compiled in with
`include_str!`. Their version is a hash of the compiled text, computed once when the daemon starts.
They tell the Planner:

1. its main job — analyse, discuss, and build the plan with Shadows' tools; no
   Claude tool is taken away from it;
2. how to use the tools — read with `workflow_get` before editing, put every
   change of one step in a single `plan_edit`, choose task numbers;
3. what the two link kinds and a label mean;
4. that an acceptance item is a sentence someone can check;
5. that `REVISION_CONFLICT` means read the plan again, then rebuild the edit;
6. that approving is the person's: when asked to approve, it asks the person to
   press Approve, and never reaches the daemon's HTTP API itself (§13.2's OPEN
   block).

**Project instructions** are edited in project settings. Each save is a new row
of `planner_instructions_version` (§13.15) with the next `number`; nothing is
overwritten, and a project's current instructions are its highest number, so
no pointer can name another project's row. They follow
Shadows' instructions under a "Project instructions" heading.

**What a session opens with.** `session/new` and `session/resume` carry, where
the installed adapter reads them (0.81.1, `acp-agent.js`):

| Field | Value |
|---|---|
| `mcpServers` (a field of the request) | `shadows`, type http, the `/mcp` URL, `Authorization: Bearer <token>` |
| `_meta.systemPrompt` | `{ append: Shadows' instructions + project instructions }` — appended to Claude Code's own prompt, which is kept |
| `_meta.claudeCode.options.allowedTools` | `["mcp__shadows__*"]` |

`allowedTools` pre-approves Shadows' tools only. Milestone 1 refuses every
permission request (§12.3), and Accept edits (§12.5's default) does not
pre-approve MCP tools, so without it the Planner could not write a plan in the
default mode. It restricts nothing; the grant remains the authority.

**What a Claude session keeps.** Claude Code writes the appended instructions
into the session's transcript when the session is created, and restores them
on every `session/resume`, from the same adapter process or a new one. A
changed `append` on resume is ignored. `mcpServers` is read again on every
resume, so a new grant's token takes effect (`MCP_PROBE.md` §3). So `append`
reaches Claude once, when its session is created. Shadows never rebuilds a
session to change instructions.

**When instructions change.** Before each turn starts, Shadows compares the
current versions with those recorded by the thread's latest `agent_invocation` whose turn started (the route records the new turn's invocation before this check, and a turn refused before its prompt went out must not swallow a change).
The turn's prompt then carries, after the person's text, one context block with
what differs:

- the project instructions version differs: `[Shadows] The project's Planner
  instructions changed. They replace the project instructions you were given
  before:` followed by the new body, or `[Shadows] The project's Planner
  instructions were removed.`;
- `prompt_version` differs or was never recorded (a conversation from before
  this milestone, or a Shadows upgrade that changed `prompt.txt`): the block
  also carries `prompt.txt`, under `[Shadows] Shadows' instructions for you:`.

A thread with no started turn whose session opened with `session/new` is compared
with the versions that `append` carried, so instructions saved between opening a
conversation and its first message still arrive. It gets a block with both when
its session opens by fork (§12.2), because the parent's session holds versions
Shadows did not record for this thread. The person's text is always the first
content block. A session is never touched during a turn.

Each `agent_invocation` records `prompt_version` and the
`planner_instructions_version` current when it started (§13.15), so a change is
sent once.

## 13.9 Showing a plan in the conversation

The Workflows page (§13.11) is where a plan is reviewed. Inside a conversation,
the person can ask to see it — "show me the plan", "show me T4", "open it on
the side", "take me to its page" — and keep talking about what they see.

**`plan_show`** (internal Planner only) takes the plan version, an optional
task, and where: `inline` in the conversation, `side` in a panel beside it, or
`page`. It changes no plan. It writes a thread entry of kind `PlanView` with
the version, the task and the place, and a `Workflow` reference (§4
`EntryRef`), under the running turn's command identity (§13.5). An external
agent can read plans but cannot move a person's screen.

- **The entry renders the real graph**, with the component the Workflows page
  uses. It is live: it follows its version's edits, headed by that version and
  its current revision. It is pinned to its version and never jumps to the
  next one.
- **`side` and `page` act only in the tab that sent the turn, and only live.**
  Each tab makes a random id per page load and sends it with the turn; the
  daemon keeps it in memory for that turn (§2.10: a client connection is
  transport state, not stored) and attaches it to the `PlanView` event's live
  delivery only (§2.4's transient bus); the durable event does not carry it.
  A tab opens the panel or navigates only for a live event carrying its own
  id. A replayed event, or another tab, shows the card only.
- **Pointing at a task.** Clicking a task in a card or panel puts a chip above
  the composer ("T4 · Login screen ×"). The next turn carries `focus: {
  workflow_id, task_id, revision }` (§13.10) — the task's id, with its number
  shown. Shadows checks that the task belongs to that version, then gives the
  Planner the focus as a content block separate from the person's text, so the
  Planner knows what "this" means and reads the current plan before editing.
  The Planner does not see the screen.
- **The focus is kept with the message.** The person's `UserMessage` entry
  carries `Workflow` and `Task` references (§4 `EntryRef`), so "change this"
  still says which task when the conversation is read later.
- Approval's `PlanApproved` entry ("Plan v2 approved") goes to the version's
  writer conversation when §16.3 permits it.

The client branches on these kinds, as Milestone 1's already does on
`UserMessage` and `PermissionRefused`; that is the trigger of §4's
`ThreadEntryKind` OPEN block, which §4 closes.

## 13.10 Protocol changes

**HTTP routes** (described in `api/openapi.json`, like every route):

| Route | Purpose |
|---|---|
| `GET /api/projects/{id}/workflows` | The project's plan list (§16.10) |
| `GET /api/workflows/{id}` | One version: tasks, links, revision, `previous`/`next` version ids, what blocks approval, and the last `plan_edit`'s summary with the task numbers it changed |
| `POST /api/workflows/{id}/approve` | `{ command_id, expected_revision }` |
| `GET` / `PUT /api/projects/{id}/planner-instructions` | Read and save project instructions |
| `GET` / `POST /api/projects/{id}/mcp-grants` | List external grants; issue one (the token is in this answer only) |
| `DELETE /api/mcp-grants/{id}` | Revoke |

The last edit's summary comes from the stored event, so a page opened later
still marks what changed.

The plan/version, archive, conversation deletion and project-stream interfaces
are owned by [§16.10](./2026-10-01-project-plans-design.md#1610-interfaces).

**`POST /api/threads/{id}/turns`** (§12.7's `StartTurn`) gains two optional
fields: `focus` (§13.9) and `client_tab` (the tab id, kept in memory only).
The optional `plan` for Continue this plan is owned by §16.4 and §16.10.
`focus` joins the command's fingerprint, so the same `command_id` and text
with T3 instead of T4 is `CommandConflict`, not a replay; `client_tab` does
not, being transport state.

**Errors.** Two codes §3.4 already registers are used as they stand; three are
added there:

| Code | HTTP | When |
|---|---|---|
| `WORKFLOW_FROZEN_IMMUTABLE` (existing) | 409 | A change to a frozen version |
| `WORKFLOW_VALIDATION_FAILED` (existing) | 422 | A check of §13.4 failed; carries the list |
| `REVISION_CONFLICT` (new) | 409 | `expected_revision` is stale; carries `current_revision` |
| `GRANT_SCOPE`, `GRANT_INVALID` (new) | — | MCP tool results only (§13.6) |

**Events.** Draft started, plan edited (with revision and summary), plan
frozen, plan shown, grant issued, grant revoked — each a durable event
committed with its change. The plan events reach a client on the thread's
existing stream (`/api/subscribe?thread_id=`, §2.10). Plan views across
conversations stay current through the project stream of
[§16.8](./2026-10-01-project-plans-design.md#168-the-web-client), without timer
polling. Grant lists refetch after the person's own actions and every
10 seconds while project settings is open.

## 13.11 Web client

- **Sidebar**, under a project: Conversations, **Workflows** (the project's
  plans under §16.8) and Project settings. With no plan yet, the
  section invites the person to ask the Planner for one. Each project folds
  open or closed from its row, as a tree, and several may be open at once;
  folding never navigates, so the open conversation stays open. The project
  the URL is in opens when the URL comes to it, and the open set is
  remembered in the browser across reloads. A `+` on the row is the only way
  to add a conversation.
- **A new conversation is a draft until its first message.** The `+` opens
  `/projects/{id}/new`: the empty conversation with its composer, and nothing
  on the daemon. The draft offers what is known before a session exists: the
  CLI, and the modes the project allows for it (§12.5); the model reads "Set on
  send" and is not chosen, and no effort shows: both are the session's, known
  once Send opens it (§12.4). Send creates the thread with the
  chosen CLI, opens its session and starts the turn with the chosen mode and
  the session's model and effort, then replaces the draft's URL with the
  thread's, leaving no history entry. A failed create stays in the draft with
  the text, and a retry reuses its command id. A turn that fails once the
  thread exists goes to the thread with its text and error, so a retry there
  never makes a second thread. Leaving the draft leaves nothing behind; one
  left while its send is under way finishes that send, and the new thread
  shows in the sidebar without taking the person back to it.
  `/projects/{id}` has no page of its own and redirects to the draft, which is
  also where a newly created project lands: a project is opened to talk in.
- **Text direction:** Arabic and English each read in their own direction.
  Every block of a reply (paragraph, list item, heading, table cell) takes the
  direction of its first strong character, as do the person's messages and
  every conversation, plan and project name; a quote or list takes the
  direction of its first letter. Inline code is a left-to-right island and a
  fenced code block is always left to right.
- **Workflows page** `/projects/{id}/workflows/{workflowId}`:
  - Header: the version, writer, version list and plan actions of §16.8,
    backed by §16.10. Zoom and fit stay here. **Approve** on an Active plan's
    draft shows the validator's list above it. Approve stays enabled; pressed
    with something missing, it shows the `422` list.
  - The graph: `@xyflow/react` for the canvas, minimap and controls;
    `@dagrejs/dagre` for automatic left-to-right layout, recomputed on every
    change; nodes are not dragged. A start node shows the title and goal. A
    task node shows `T4`, its title, two lines of goal, the paths it writes,
    its dependencies and "1 part waits for T8". `needs` edges are solid,
    `completes_after` edges dashed, each labelled. Tasks the last edit changed
    carry a colour and "changed". A legend explains both.
  - **Inspect** opens a task's goal, reads, writes and acceptance items; an
    item that waits says for which task.
  - An approved version is read-only, under "Approved v1 · editing creates
    draft v2", or "Approved v1 · v2 is its next version" once v2 exists.
- **In a conversation:** Shadows' tool calls read as sentences ("Plan edited ·
  3 changes") with **Open plan**; `PlanView` cards and panels (§13.9); the
  focus chip.
- **Project settings:** Planner instructions (text and Save, with when they
  last changed); the allowed modes (§12.11); External agents (**Connect**, the one-time command with Copy
  and the `~/.claude.json` notice, the list of grants with **Revoke**).
  - **Code index** (§15.6): the project's index as one coloured line —
    `● Ready · 277 files · updated 2 min ago`, or indexing with its count,
    inactive, no folder, folder missing. **Reads code from** lists the linked
    projects by name and folder, each with its own index line and `×` to
    unlink, under one line saying that questions here also search them and
    that a link goes one way. **Link a project…** offers the other projects
    not yet linked.
  - **Remove project**, last, in red. With no conversation, Remove asks
    first: the project is hidden, its index and links are dropped, its folder
    on disk is untouched and its slug stays taken; then the page goes home.
    The live-conversation count and deletion copy are §16.5 and §16.8's.
    A `PROJECT_HAS_THREADS` that races in shows in the dialog.
- **Settings** `/settings`, linked at the foot of the sidebar beside the
  connection line: the daemon's settings that belong to no one project, one
  section each. Its first is **Active projects** (§15.6), 1 to 20, with the
  line that this many recently used projects are watched and kept fresh and
  the others keep their index and answer as inactive. A number out of range
  is refused before it is sent.
- Colours are Shadows' own theme tokens. The reference picture's glow and
  gradients are not used.

## 13.12 Where the code goes

- **`workflow/`** (new top-level module): the plan's domain types, the batch
  operations and the validator. Pure: no storage or protocol imports.
- **`mcp/`** (the cross-cutting module §1 names): the `/mcp` server, the tool
  list per grant kind and the mapping of tool calls to commands.
- **`storage/sqlite/`**: one file per new entity (§13.15).
- **`planner/`**: `prompt.txt`, the instructions check before a turn, the
  grant handed to a session.
- **`protocol/`**: the routes of §13.10, one file per resource.

## 13.13 Work order

1. **Task 0 — probes on the real harness,** with a throwaway MCP server,
   deleted after its evidence file is written:
   1. `_meta.systemPrompt.append` reaches Claude and Claude Code's own tools
      remain;
   2. with `allowedTools`, a tool of that server runs in Accept edits with no
      permission request;
   3. `session/resume` with changed instructions: Claude follows the new ones
      **and** still remembers the conversation (the "amber" check of
      `ACP_PROBE.md` §4);
   4. the MCP protocol version Claude Code 2.1.281 and `rmcp` agree on.

   A probe that fails amends this section before anything is built. **Ran
   2026-09-25 (`docs/evidence/milestone2/MCP_PROBE.md`):** 1, 2 and 4 pass.
   3 failed: the conversation is kept but the new `append` is not, which
   amended §13.8 ("What a Claude session keeps").
2. The plan: domain, validator, storage, commands, HTTP routes.
3. `/mcp` and grants, tested with an `rmcp` client.
4. The Planner: instructions, session fields, `fake_acp` calling `/mcp`.
5. The web client.
6. Mohammed's run (§13.14).

## 13.14 Tests and acceptance

**Tests:**

1. **Validator** (no database): the start/complete cycle across kinds, dangling
   and self links, missing acceptance items, two operations on one object in a
   batch, removal of a linked task, approval's requirements.
2. **Storage contract**: the version chain, `revision` from 0 per version,
   `version` consecutive per thread, the event in the change's transaction,
   replay by derived and explicit command id, a grant revoked inside the
   transaction.
3. **MCP end to end** with an `rmcp` client against the daemon: each grant's
   tool list and scope, no approval tool, no `plan_show` for an external
   grant, `draft_start` from scratch, `draft_ref` replay after a lost answer,
   two intended plans with one name.
4. **`fake_acp`** takes `mcpServers` and the token and calls `/mcp` as a
   Planner would, so a whole turn that edits a plan runs without Claude.
5. **Web** (vitest): the graph, the list above Approve, `409` and `422`, a
   `PlanView` card, the focus chip, navigation only in the target tab,
   instructions, Connect and Revoke.

**Acceptance — Mohammed's run on Windows:**

1. Talking with the Planner, a plan of four or more tasks with both link kinds
   appears, and the graph updates while it is written.
2. "Show me T4" shows it in the conversation; clicking it and saying "change
   this" changes T4.
3. A cycle is refused with its reason.
4. Approve: v1 reads Approved. Asking for a change creates draft v2; v1 is
   unchanged.
5. Changing project instructions takes effect at the next message, and the
   conversation is remembered.
6. A Claude Code started outside Shadows connects with the copied command,
   reads a plan, edits it and starts a new plan from scratch; after Revoke it
   is refused, and what Claude Code shows is recorded.
7. External `draft_start`: the same `draft_ref` after a dropped connection
   returns the same plan; two refs with one name make two plans.
8. Two tabs on one conversation: "open it on its page" from the first moves
   only the first; reopening the conversation shows the card without moving.
9. Restarting the daemon keeps every plan, and the internal grants from before
   are refused.

## 13.15 Schema changes

Milestone 2's migration, in the tables of §6. Each §6 table this touches points
here, as §6.15 points to §12.7 for Milestone 1's columns.

The `workflow` block below records Milestone 2's original migration.
Migration 0012 replaces its thread ownership, version uniqueness and Draft
constraint under [§16.9](./2026-10-01-project-plans-design.md#169-schema).

**`workflow`** (§6.8) gains:

```text
version    INTEGER NOT NULL      -- 1, 2, 3 … within the thread
revision   INTEGER NOT NULL DEFAULT 0
title      TEXT NOT NULL
goal       TEXT NOT NULL

UNIQUE(thread_id, version)
```

At most one `Draft` per thread is enforced by the command that creates a draft,
inside its transaction.

**`task`** (§6.9) gains `number INTEGER NOT NULL`, `UNIQUE(workflow_id, number)`.
`contract_json` holds title, goal and acceptance, and `scope_json` the reads and writes (§13.3). `state` stays `Pending`.

**`task_parent`** (§6.10) gains:

```text
kind              TEXT NOT NULL      -- 'needs' | 'completes_after'
label             TEXT NOT NULL
waiting_items     TEXT NULL          -- JSON array of acceptance item numbers

PRIMARY KEY(workflow_id, task_id, parent_id, kind)
CHECK kind IN ('needs', 'completes_after')
CHECK (kind = 'completes_after') = (waiting_items IS NOT NULL)
```

**`agent_invocation`** (§6.15) gains `prompt_version TEXT NULL` and
`planner_instructions_version_id TEXT NULL`.

**New tables:**

```text
planner_instructions_version
  id           TEXT PRIMARY KEY
  project_id   TEXT NOT NULL FK project(id)
  number       INTEGER NOT NULL      -- 1, 2, 3 … within the project
  body         TEXT NOT NULL
  created_at   TEXT NOT NULL
  UNIQUE(project_id, number)

mcp_grant
  id           TEXT PRIMARY KEY
  kind         TEXT NOT NULL          -- 'thread' | 'project'
  thread_id    TEXT NULL FK planning_thread(id)
  project_id   TEXT NOT NULL FK project(id)
  token_hash   TEXT NOT NULL UNIQUE
  created_at   TEXT NOT NULL
  revoked_at   TEXT NULL
  CHECK kind IN ('thread', 'project')
  CHECK (kind = 'thread') = (thread_id IS NOT NULL)
  FK (thread_id, project_id) → planning_thread(id, project_id)
  UNIQUE(id, project_id)

draft_intent
  draft_ref    TEXT PRIMARY KEY
  grant_id     TEXT NOT NULL FK mcp_grant(id)
  workflow_id  TEXT NULL FK workflow(id)   -- set when draft_start uses it
  created_at   TEXT NOT NULL
  expires_at   TEXT NOT NULL
```

`planning_thread` gains `UNIQUE(id, project_id)` so a grant's thread and
project cannot disagree. A `draft_intent`'s plan is in its grant's project;
that pair crosses two tables, so the command that sets `workflow_id` checks it
inside its transaction.

`thread_entry.kind` gains the values `PlanView` and `PlanApproved` (§13.9), and
§4 types the column's values from here on.
