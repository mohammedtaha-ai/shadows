# Global external agent connections

The initial global connection flow shipped in PR #71 (`0e281585`).
Named credentials and project edits extend that flow.

A client that supports MCP over HTTP connects once from Shadows Settings → External agents. At issue, the person names the credential and selects one or more live projects. The token grants the existing external MCP capabilities for those projects. The Settings section owns creation, project edits, setup instructions and Revoke.

## Scope and alternatives

Use one token per connection, an explicit project allowlist and an explicit project argument on project operations. Discovery lists only live projects on that token's allowlist. A missing project or a project outside the allowlist is refused. A task, plan, agreement or draft reference must belong to the selected project when the operation requires same-project access. A draft reference is bound to the project where it was prepared.

The person can replace an existing connection's project list, including clearing it.
The edit keeps the bearer and is guarded by the project revision from when the edit
opened. Removing a project revokes its scoped grant in the same write. Re-adding it
creates a fresh scoped grant. Unchanged projects retain their scoped grants. Revoked
connections cannot be edited. Existing credentials stay valid and display a fallback
label if they have no name.

## Authorization and cutover

Core Grants owns issue, list, bearer validation and revocation. Built: migration `0074_external_connections.sql` creates `external_connection` and `external_connection_project` and revokes every existing project-scoped grant. The connection's `project_list` tool returns its live projects; every other external tool takes a `project_id` argument and refuses `project`. Each allowed project has a scoped grant for attribution and transaction-local checks. The connection token selects one of those grants for each call. Scoped grants generated for a connection carry no usable standalone credential. Revoking a connection revokes all its scoped grants atomically. Existing plan and design services keep their transaction-local authority checks and validate target ownership. Planner and executor grant scopes keep their existing semantics. External connections retain the external tool inventory, with project discovery added. Approval and execution permissions remain with their existing principals.

The upgrade revokes project-scoped external credentials and retains the rows needed for history and authorship. An old token never gains global scope. The global UI lists global connections, and the project-specific issue and list routes are retired in the same cutover. Revoke stops future use and prevents writes still waiting to commit. Grant records contain token hashes. Issue responses display a token once, and command replays omit it.

## Setup and verification

Settings defaults to manual setup with server name `shadows`, the daemon's MCP HTTP
endpoint and the bearer credential. Optional Codex and Claude Code selections show
their registration commands using that same server name. Codex reads the token from
a named environment variable. The browser releases the issued secret when its setup
view closes.

Automated checks cover migration of old credentials, token replay and revocation, a single token across two selected projects, denial for an unselected project, wrong and missing project arguments, draft reference isolation, external tool inventory, and Settings issue and revoke. The user owns live terminal and browser acceptance.

## An external agent works like the Planner (Mohammed, 2026-11-28)

Status: decided in conversation; not built. It replaces "Approval and execution permissions
remain with their existing principals" above for execution only: approval stays as it is.

An agent connected from outside, such as Claude Code or Codex, can take a project from its
idea to finished tasks, as the Planner does in a Shadows conversation. Today it cannot: it
reads the workspace but cannot write the vision, the map or the roadmap, so `draft_start`
answers `STAGE_BLOCKED: no roadmap approved` (tried on `testing10`, 2026-11-28).

### Its tools

The external inventory (`PROJECT_TOOLS` in Core Grants) grows to the Planner's
(`THREAD_TOOLS`) except for three groups:

| Group | Tools | For an external agent |
|---|---|---|
| Propose, design and read | `vision_propose`, `map_propose`, `roadmap_propose`, `decision_propose`, `decision_list`, `design_element_put`, `design_element_delete`, `design_elements`, `project_state`, `code_where`, `project_files`, `project_refresh`, `project_change_note` | Granted |
| Run tasks | `task_ready`, `task_runs`, `task_start`, `task_start_status`, `task_executor_set`, `executor_catalog`, `task_stop`, `task_done`, `task_amend`, `task_lessons`, `task_diff`, `task_file`, `task_checks`, `project_recheck`, `project_git_init`, `message_agent` | Granted, with the Planner's limits: scopes, checks and the executor's own approvals do not change |
| Approve | `vision_approve`, `map_approve`, `roadmap_approve`, `plan_approve` | **Never.** The person approves with the button. §24.2 lets the Planner approve at the person's request because the approval links to the person's message in the same conversation; an external agent has no such message, so its approval could not show the person asked |
| Bound to a conversation | `plan_show`, `person_ask` | Not granted: they draw in a Shadows conversation the external agent does not have. An external agent asks the person directly in its own client (Mohammed, 2026-11-28) |

### How the plan's author hears of its tasks

The Planner hears as today: a new turn in its conversation. An external agent is never woken
by Shadows: it waits (below), so Shadows keeps no address for it. The external clients are
Claude Code and the Codex desktop app, which the person opens (Mohammed, 2026-11-28). A second
client on the person's own `codex app-server` can start a turn in a `codex --remote`
conversation (proved live on 2026-11-28), but the person does not run Codex that way, so
Shadows does not build it.

### Waiting for a task

One method waits until an attempt of the project ends and its checks have run, or until a
bound time passes, and answers the attempt's report or "nothing yet". Two adapters call it:

- the MCP tool `task_wait`, for any client. The client is busy while it waits, and Codex
  stops a tool after its `tool_timeout_sec` (about 60 seconds by default), so the bound
  stays under that;
- an HTTP route with the connection's bearer, which Claude Code calls from a background
  command and is woken when it ends, free in the meantime.

The client's own hooks call the same route, so a wait needs no model turn and no one to
remember it. Settings → External agents gives each hook ready to register:

| Client | Hook | While it waits |
|---|---|---|
| Claude Code | a `Stop` hook with `asyncRewake`: it runs in the background after each turn and exits with code 2 and the attempt's report, which wakes the session even when it is idle. Optional: a background command does the same | free |
| Codex, including its desktop app | a `Stop` hook: it answers `{"decision": "block", "reason": "<the attempt's report>"}`, which Codex takes as the next prompt; its `timeout` (600 seconds by default) is raised in the hook's entry | busy, spending no tokens; the person can stop it |

Each hook first asks whether the connection has an attempt still running in the project,
and ends at once when it has none, so a turn with no task running stops as usual.

Codex Desktop can only wait this way: its conversations accept no turn from another program
(`codex exec resume` is refused while the app holds the thread, and its
`send_message_to_thread` exists only inside the app). Claude Code's own cross-session
`SendMessage` reaches a session from another Claude Code session on the same configuration
folder, but not from a program that is not one (each session's pipe is authenticated), and
Claude Code channels are a stdio-only research preview; Shadows uses neither.

When an attempt of an externally written plan ends and its author is neither woken nor
waiting, the result waits unjudged, shown as such in Shadows. No Planner takes over: that
would spend tokens the person did not ask for and put two writers on one plan. The next
wait answers at once with every attempt that ended meanwhile (Mohammed, 2026-11-28).
