# Section 16 — Plans Belong to the Project

- **Date:** 2026-10-01
- **Status:** Designed with Mohammed on 2026-10-01.
- **Idea:** `docs/vision.md` §8, "The plan belongs to the project, and
  remembers who decided what and why".

A plan stops belonging to the conversation that wrote it. Any conversation on
the project reads it and carries it on, with the same model or another, and
every version records who wrote it: for the Planner, its conversation, model
and CLI; for an external agent, only its grant, since Shadows does not run
it and knows no conversation or model of its own. So a long
planning session can move to a fresh one without losing what was planned.
Because a plan no longer needs its conversation, a conversation can be
deleted.

## 16.1 Scope

The work is two pull requests, each one runnable on its own. 1b needs 1a.

**1a: the plan is the project's.**

- A plan is its own entity, owned by a project, Active or Archived (§16.2).
- Every version and every edit records who wrote it, and a new version says
  why it was started (§16.3).
- The Planner reads and edits every plan in its project (§16.4).
- A conversation can be deleted (§16.5).
- Each open session gets its own lock (§16.6).
- The web client, for all of the above (§16.8).

**1b: plans linked to plans.**

- A task can link to a task in another plan of the project, or of a project
  this one is linked to (§16.7).
- A plan's graph shows those tasks, and a project map shows its plans and
  their links (§16.8).

**Not here:**

- The decisions behind a plan, which is vision §8's second stage, and the
  history page that lists every edit with them. A version's reason (§16.3) is
  the one part of it here. §17 drafts the rest.
- A question left on a point of the plan, which is vision §8's third stage.
- Executors. §16.3 is laid out so that an executor's work is attributed the
  same way, from the same `agent_invocation` row, when executors exist.

## 16.2 The plan

**A plan is a row of a new table, `plan`, owned by one project.**

- A plan has no title of its own. Its name and goal are its latest version's
  `title` and `goal` (§13.2), so a new version can rename it.
- **A version** is still a `workflow` row (§13.2), with `version`,
  `revision`, `title` and `goal`, Draft or Frozen. It belongs to its plan
  instead of a thread: version numbers are consecutive within the plan, and
  `previous_version_id` stays within the plan.
- **A plan has at most one Draft.** It was one per thread, held only by the
  command's transaction. Now a unique index on the plan's Draft holds it in
  the database too, so two conversations starting a version at once make one.
  Two conversations that edit the same Draft are kept apart by the revision
  check (§13.5), as two writers already are.
- **A frozen version never changes** (§13.2), and the database now refuses
  it: a trigger rejects any update of a `Frozen` version's content, its tasks
  or its links. Until now only the store's code held that rule.
- **A project has any number of plans**, for example one for the backend and
  one for the web client.

**A plan's state is `Active` or `Archived`.**

- **Archiving** is a person's action in the web client (§16.8), like approval.
  It is a command with a `CommandId` (kind `PlanArchive`) and is undone by
  `PlanUnarchive`. No MCP tool archives.
- **An archived plan is read, never written.** `draft_start` and `plan_edit`
  on it are refused with `INVALID_COMMAND`, "plan X is archived; a person can
  unarchive it". A Draft it holds stays as it is.
- `workflow_list` and the sidebar leave archived plans out unless asked
  (§16.4, §16.8).

## 16.3 Who wrote it

**Every version records its writer, once, when it is created.**

| Writer | Recorded on the version |
|---|---|
| The internal Planner | `written_by_thread` (its conversation) and `written_by_operation` (the running turn) |
| An external agent | `written_by_grant` (its project grant) |
| A version from before migration 0012 | `written_by_thread` only: the thread it belonged to |

The running turn's `operation` has one `agent_invocation` (§12.7), which
already holds the harness, its version, the requested and observed model,
the effort and the role. So the model and CLI of a version are read from
there and are never copied.

**Every edit records its writer too.** `WorkflowEdited`, `WorkflowFrozen` and
`WorkflowDraftStarted` set the durable event's `thread_id` and
`operation_id` for the Planner, and the grant as the actor for an external
agent. A plan event's `project_id` is always set. A person's approval has no
conversation of its own: its `WorkflowFrozen` names the conversation that
wrote the version, and the "Plan v2 approved" entry (§13.9) goes there, unless
that conversation is deleted or the version came from an external agent, when
there is no entry.

**A new version says why it was started.** Every version after v1 is
created with a `change_reason`, one or two sentences on what made the plan
change, such as "the backend's API changed shape". `draft_start` from an
existing plan refuses an empty one with `INVALID_COMMAND`. v1 has none: its
goal is why it exists. It is stored on the version, written once, and copied
nowhere. A version from before migration 0012 has none, and reads "Reason not
recorded": none is invented for it. A `Draft` answered again by `draft_start`
(§16.4) keeps the reason it was started with.

This is the first step of vision §8's decisions. A reason is per version, not
per edit, and it is not a decision record: §17 drafts those.

**What a person reads** (§16.8): one line per version, "v2 · from *Web fixes*
· Opus 5.5 · Claude Code", with its reason under it. The model is the
observed one, or the requested one before it is known. A version from a
deleted conversation names it with "(deleted)", and one from an external
agent reads "External agent". A version from before the migration names its
conversation only.

## 16.4 The Planner and the project's plans

**The Planner's tools reach every plan of its project.** The grant still
names its thread (§13.7), and the thread fixes the project.

| Tool | Before | Now |
|---|---|---|
| `workflow_list` | external only | also the Planner: the project's Active plans, or with `archived: true` the archived ones too |
| `workflow_get`, `task_get` | its thread's plan | any version of any plan in the project |
| `draft_start` | in its thread | `plan_id` absent: a new plan with v1 from a title and a goal. `plan_id` present: that plan's Draft if it has one, which is answered and changed nothing, or a new version copied from its latest, which needs a `reason` (§16.3) |
| `plan_edit` | its thread's Draft | the Draft of any Active plan in the project |
| `plan_show` | its thread's plan | any version in the project; the entry is written in the Planner's own conversation (§13.9) |

- **`plan_id` replaces `from_workflow_id`** in `draft_start` for both kinds of
  grant. §13.6's rule that a Planner's source must be its thread's latest
  version goes: a plan's next version always starts from its latest.
- **An external agent's `draft_start` from scratch creates no thread.** The
  plan and its v1 are created together, written by the grant, and no
  conversation is made to hold them. Its `draft_ref` (§13.5) is unchanged,
  and so is the requirement of one when it starts from an existing plan.
- **Command identity is unchanged** (§13.5): the Planner's principal is its
  thread, and a derived command id still uses the running turn.
- **A plan in another project is never written.** Reading one is §16.7's.

**Shadows' instructions** (`prompt.txt`, §13.8) say:

1. the project's plans are shared by every conversation on it;
2. when the person names a plan, find it with `workflow_list`, read it with
   `workflow_get`, and carry it on; start a new plan only when the person
   asks for one or none fits;
3. an archived plan is read only, and unarchiving is the person's;
4. a new version needs its reason: say in a sentence what changed the plan,
   as the person and the conversation settled it, not a summary of the edit.

A changed `prompt.txt` reaches every conversation through §13.8's context
block, so no conversation needs reopening.

**"Continue this plan."** A plan's page opens a new draft conversation
(§13.11) with the plan chosen. Its first message carries the plan, and the
turn's prompt gets one context block after the person's text: `[Shadows] The
person opened this conversation to continue the plan "Web" (plan_id …). Read
it with workflow_get before you plan.` The conversation is not bound to the
plan: later messages may move to another plan in words.

## 16.5 Deleting a conversation

**A conversation can be deleted** (`DELETE /api/threads/{id}`, command
`thread.remove`, with a `CommandId` as project removal has). It is a soft
remove: the row stays with `removed_at` set, because the durable log and its
plans' versions refer to it.

In order, so that no new work can start once the delete has begun:

1. **One write** sets `removed_at`, **revokes the thread's Planner grants**
   (§13.7), and journals `ThreadRemoved`. From that commit on, the
   conversation is removed: every write that starts a turn (`Turns::send`) or
   forks it checks `removed_at IS NULL` inside its own write transaction, and
   every plan write by its Planner is refused `GRANT_INVALID` by the check
   every grant write already makes inside its transaction (`check_writer`).
   The single writer (§6.23) orders each of them before or after this one,
   never across it.
2. **A running turn is stopped**, as Stop stops it (§12.3), and its terminal
   state is waited for. That turn's own last writes, its terminal state and
   the entries it already produced, are still recorded: they end work that
   began before the delete.
3. **Its adapter is closed** through its session slot (§16.6). Opening a
   session holds the same slot and reads the thread, `removed_at` included,
   once it holds it. So an opening already under way finishes first and is
   closed here, and one that takes the slot after this is refused as not
   found. The slot is not held through step 2: a Stop closes the adapter
   through the slot too, and would wait on it.

A replay answers what the first one answered.

**If the daemon stops between steps 1 and 3,** the delete has already taken
effect: the conversation is removed and its grant revoked, so nothing it
leaves behind can write. At start, recovery ends every turn left running as
`Interrupted` (§8). Whether the adapter itself is gone depends on how the
daemon ended: a clean stop closes every session; a crash on Windows kills it
through the Job Object (§1.5); a crash on Linux may leave it running, which is
§1.5's open risk and not this section's. Such an adapter can still read the
project's files, but it cannot write a plan or a turn.

**Afterwards:**

- It is not listed, and nothing acts on it: a turn, opening its session, a
  fork from it or a focus naming it is refused as not found.
- **It can still be read.** `GET /api/threads/{id}` and its entries answer,
  with `removed_at`, so a version's "from *X* (deleted)" opens it read only
  (§16.8). There is no restore.
- **Its plans are untouched.** They are the project's (§16.2), and their
  versions still name it.
- A fork of it keeps working: a fork holds copies of its source's messages
  (§12.9).
- §4.2's removal of a project is refused while the project holds a thread
  that is **not removed**. So a project is removed by deleting its
  conversations first.

## 16.6 One lock per open session

`Sessions` (`harness/sessions.rs`) holds one `Mutex` over every live session
and keeps it through an adapter's start (3 to 6 s on Windows, at most 20 s)
and through each termination wait. So opening one conversation delays Stop on
another. It is latency, not a correctness defect.

**The fix:**

- The shared map holds one slot per thread. Its lock is held only to find,
  add or remove a slot, never across an `await` on a process.
- Each slot has its own lock. Opening, closing and stopping one thread hold
  only its slot.
- An action on the same thread still waits for that thread's own opening,
  within the same 20 s bound.
- Behaviour is otherwise unchanged: the idle close, the dead-adapter check and
  the count of live sessions work as before.

## 16.7 Links between plans (1b)

**A task can link to a task of another plan** with the two kinds and the label
of §13.3: `Web T4 needs Backend T3`.

- **The other end** names a plan and a task number, never a version. It
  always means that task in the plan's **latest** version. A task keeps its
  number across versions (§13.3), so a new version of Backend does not break
  the link.
- **Which plans.** A plan of the same project, or a plan of a project this
  project is linked to (§15.6's one-way link). Any other is refused with
  `INVALID_COMMAND`, which names the project link that is missing.
- **Written** by `plan_edit`'s `link_put` and `link_remove`, whose `parent`
  may be `{ plan_id, task }`. It is copied into a new version with the other
  links, as §13.2 copies links.
- **Read** with the plan: `workflow_get` answers each link to another plan
  with the plan's name, the project's name if it is another project, and the
  task's title, state, goal and acceptance items. So a writer sees what it
  depends on without reading the other plan whole. The Planner and an external agent may call
  `workflow_list` and `workflow_get` with `project` set to a linked project.
  That is read only.

**A link breaks** when its task is not in the plan's latest version, its plan
is in a project no longer linked, or that project was removed.

- A broken link is kept and shown in red (§16.8). It is never deleted
  silently.
- On a Draft, a broken link is in the list of what blocks approval (§13.4),
  naming the plan and the task.
- On a frozen version, a link that breaks later is only shown. A frozen
  version never changes (§13.2).

**A cycle through other plans is refused.** `Web T4 needs Backend T3` and
`Backend T3 needs Web T2` make a cycle no plan holds alone. §13.4 finds a
cycle inside one version. Approval also follows the links into other plans'
latest versions, and a cycle found there is in the list of what blocks
approval, naming each task on it. The order is §13.4's, across both kinds:
`needs` and `completes_after` count alike.

## 16.8 The web client

**The sidebar,** under a project:

- **Workflows** lists the project's Active plans, with their latest version's
  title and state. Under them, a folded "Archived (N)" lists the rest.
  **Map** opens the project map (1b).
- **Each conversation** has a `⋯` menu with **Delete**. It asks first:
  "Delete "*X*"? It leaves the list for good. Its plans stay in the project,
  and their history still names it." Deleting the open conversation goes to
  the project's draft.

**The Workflows page.** The graph, its nodes, edges, layout, legend, Inspect
and Approve stay exactly as §13.11 has them. The header gains:

- the line of §16.3 for the version shown. Its conversation's name opens that
  conversation;
- **Versions**: every version of the plan, each with its line;
- **Continue this plan**, which opens the draft of §16.4;
- **Archive**, or **Unarchive** on an archived plan, with "Archived · read
  only" in place of Approve.

**In the graph (1b),** a task of another plan is drawn with the same node,
headed with its plan's name, "Backend · T3". One from another project is the
same node with a dashed border and the project's name, "shadows-api · Auth
API · T2". A broken link's node is red and says what broke. Links into this
plan from other plans are drawn the same way. Clicking such a node opens its
plan.

**The project map (1b),** `/projects/{id}/map`:

- Each Active plan is a node in the same style as a task node: title, goal,
  latest version and its state, and its number of tasks.
- An edge joins two plans that have links between their tasks, labelled with
  how many.
- A plan of another project that is linked either way is a node with a
  dashed border and its project's name.
- The same `@xyflow/react` and `@dagrejs/dagre` as the plan graph, left to
  right. Clicking a node opens its plan.

**A deleted conversation** opened from a version's line shows a banner, "This
conversation was deleted. You can read it, but not write in it.", its
messages, and no composer.

**Project settings → Remove project** is enabled once every conversation is
deleted. Until then it reads "This project has N conversations. Delete them
from the sidebar first."

**How the pages stay current.** A plan's events go to the stream of the
conversation that wrote them (§13.10). A plan page, a project map and a
`PlanView` card shown in any other conversation refetch after the person's own
actions and every 10 seconds while shown, as the project's plan list already
does.

## 16.9 Schema

**Migration 0012 (1a):**

- `plan`: `id`, `project_id` (references `project`), `state` (`Active` or
  `Archived`), `created_at`, `archived_at`. `archived_at` is set exactly when
  the plan is archived.
- `workflow` is rebuilt, by SQLite's table rebuild, so that:
  - `thread_id` gives way to `plan_id` (references `plan`) and to
    `written_by_thread`, `written_by_operation` and `written_by_grant`;
  - `UNIQUE (plan_id, version)`, and `previous_version_id` references a
    version of the same plan;
  - a version has a thread or a grant as its writer, never neither;
  - `change_reason`, `NULL` for v1 and for every version before 0012, and
    otherwise text that is not blank. The API answers `null` for none, and a
    reader tells v1 from an unrecorded reason by the version number;
  - a unique index on `plan_id` where the state is `Draft` (§16.2).
- Triggers refuse an update of a `Frozen` version's content, and an insert,
  update or delete of its tasks and links (§16.2). Freezing itself, the update
  from `Draft` to `Frozen`, is allowed.
- **The data:** each thread's chain of versions becomes one Active plan of
  the thread's project. Each version keeps its id, so `task`, `task_parent`,
  `draft_intent`, `PlanView` entries and every event that names a version
  stay valid. `written_by_thread` is the old `thread_id`.
- `planning_thread` gains `removed_at`.

The migration is run first on a copy of Mohammed's dev database and checked:
every version is in a plan, and none is lost.

**Migration 0013 (1b):** one table for a link whose parent is in another plan.
It holds the version and task the link starts from, the target plan and task
number, the kind, the label and the waiting items, with §13.3's rules on
them. Triggers refuse an insert, update or delete of such a link on a `Frozen`
version, as 0012's refuse its other links.

## 16.10 Interfaces

**HTTP routes** (in `api/openapi.json`):

| Route | Purpose |
|---|---|
| `GET /api/projects/{id}/workflows` | The project's plans: id, state, and the latest version's id, number, state and title. Archived ones with `?archived=true` |
| `GET /api/plans/{id}` | One plan: state and every version with its writer (§16.3) |
| `POST /api/plans/{id}/archive`, `/unarchive` | `{ command_id }` |
| `GET /api/workflows/{id}` | As today, plus `plan_id`, the plan's state and the version's writer; in 1b, the links to and from other plans with their state |
| `DELETE /api/threads/{id}` | §16.5 |
| `GET /api/threads/{id}` | New: one thread, removed or not, with `removed_at`. The list leaves removed threads out, so a deleted conversation's page reads it here |
| `GET /api/projects/{id}/plan-map` | 1b: the map of §16.8 |

`POST /api/threads/{id}/turns` gains an optional `plan` (§16.4's "Continue
this plan"). It joins the fingerprint, as `focus` does (§13.10).

**MCP:** the tool table of §16.4, and `project` on `workflow_list` and
`workflow_get` in 1b.

**Each change goes to the owning service's contract** in the same commit
(CLAUDE.md): `plans`, `threads`, `harness`, `grants`, `projects`.

## 16.11 Order of work

**1a:**

1. The lock (§16.6), with its test. It touches nothing else.
2. Migration 0012 and the `plans` store: plans, writers, archive.
3. The tools and `prompt.txt` (§16.4).
4. The HTTP routes and the writer of each version (§16.10).
5. Deleting a conversation (§16.5), and project removal's new rule.
6. The web client (§16.8, without the map and the cross-plan nodes).

**1b:**

7. Migration 0013, links between plans in `plan_edit` and `workflow_get`,
   and the approval check.
8. The plan graph's nodes for other plans.
9. The project map.

## 16.12 Tests and acceptance

Few tests, each on a rule a person would notice broken:

1. **The lock:** one conversation's adapter is slow to start, using
   `fake-acp`. A Stop on another conversation finishes without waiting for
   it.
2. **The migration:** on a database with two threads, one with v1 and v2,
   every version is in a plan, ids are unchanged, and `written_by_thread` is
   the old thread.
3. **A shared plan:** conversation B starts v2 of a plan conversation A
   wrote. Without a reason it is refused. With one, v2 names B and its
   reason, and v1 still names A.
4. **One Draft per plan:** two `draft_start`s on the same plan at once make
   one Draft. A write to a frozen version's task is refused by the database
   itself.
5. **Deletion:** a deleted conversation leaves the list, a turn in it is
   refused, its plan is still listed, and it still reads. While the delete
   is stopping a running turn, a new turn is refused, and a `plan_edit` by
   that turn's Planner is refused `GRANT_INVALID` and writes nothing. No
   adapter is left open afterwards.
6. **A broken link (1b):** removing Backend's T3 in a new version turns Web
   T4's link red and puts it in Web's Draft's approval list.
7. **A cycle across plans (1b):** Web T4 needs Backend T3 and Backend T3
   needs Web T2. Web's Draft cannot be approved, and the list names all three
   tasks.

**Acceptance on Windows,** on a copy of the dev database, then on the
database itself:

- After 0012, both existing conversations' plans are in Workflows.
- A new conversation continues a plan with "Continue this plan" and starts
  its next version with a reason. The version line names the new
  conversation, its model and CLI, with the reason under it.
- A conversation is deleted. Its plan stays, and its version line opens it
  read only.
- Stop on one conversation while another opens does not wait.
- 1b: a Web task linked to a Backend task shows in both graphs and on the
  map.

## 16.13 Changes to other documents

Made in the commit of the step that makes them true:

- **§13.2:** a plan is §16.2's, not one thread's chain; one Draft per plan.
- **§13.6:** the tool table and `draft_start` become §16.4's, and an external
  draft from scratch creates no thread.
- **§13.10, §13.11:** the plan list, the Workflows section, and the Workflows
  page header refer to §16.8 and §16.10.
- **§4.2:** a thread can be removed (§16.5), and project removal counts only
  threads that are not removed.
- **§6.8:** a version's lineage is within its plan.
- **`vision.md` §8:** links here, and its Today line says what exists.
- **`docs/status.md`:** the lock leaves Next, and so does deleting a
  conversation.
