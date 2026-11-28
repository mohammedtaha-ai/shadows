# External agents like the Planner

- Status: accepted (Masada, 2026-11-28), with the two additions in "Frontend reply"
- Asked by: Hamoud, for Mohammed, 2026-11-28 (the backend needs these screens, so it writes first)
- Screen: Settings → External agents; Design's vision, map and roadmap proposals; the task page.
  Spec: [external connections, "An external agent works like the Planner"](../../backend/specs/2026-11-18-external-agent-connections-design.md#an-external-agent-works-like-the-planner-mohammed-2026-11-28)

## Need

An agent connected from outside (Claude Code, or the Codex desktop app) now proposes the
vision, the map and the roadmap, writes plans and runs tasks as the Planner does. It never
approves: the person approves its proposals with the button. Shadows never wakes it: the agent
waits for an attempt's report, through its client's Stop hook on an HTTP route or through the
MCP tool `task_wait`. When no one waits, the result waits unjudged and Shadows shows it so.

The web needs four things:

1. **Settings → External agents** gives, per connection and per selected project, ready to
   copy:
   - the Claude Code `Stop` hook with `asyncRewake` (it runs in the background after each turn);
   - the Codex `Stop` hook, its `timeout` raised to 600 seconds in its entry.
   The Settings section owns setup instructions (spec, "Global external agent connections"), so
   the web writes the hook text; the backend serves the route the hooks call and its address.
2. **Design proposals with no conversation.** A vision, map or roadmap proposal from an external
   agent has no thread. It shows where the person approves proposals, marked as from an
   external agent, with Approve.
3. **An executor chosen by an external agent.** The task page's "Chosen by" has a third value.
4. **A result waiting for its author.** A task whose latest attempt ended and whose external
   author has not acknowledged its report says so.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `POST /api/external-connections`, `GET /api/external-connections`, `PATCH /api/external-connections/{id}` | as today | each connection gains `wait_url: string`, e.g. `http://127.0.0.1:7420/api/external/task-wait` |
| `POST /api/external/task-wait` (hooks only, not the browser) | header `Authorization: Bearer <connection token>`; body `{ project_id: string, seconds?: number, ack?: string[] }` (`seconds` default 300, held to 540) | `{ outcome: "ended" \| "nothing_yet" \| "none_running", reports: [{ report_id: string, attempt_id, workflow_id, task: number, kind: "ended" \| "conflict", report: string }] }` |
| `GET /api/projects/{id}/vision-proposals`, `…/map-proposals`, `…/roadmap-proposals` | as today | each proposal's `thread_id` becomes `string \| null`, and it gains `grant_id: string \| null` (set when an external agent proposed it) |
| the task run (`TaskRun`) | as today | `executor.chosen_by` may be `"external"` beside `"person"` and `"planner"`; new `author_waiting: boolean` |

What a hook does with the route (so the web can write it):

- It makes **one** call per run, with the connection's token from the environment
  (`SHADOWS_TOKEN`, as the Codex registration already uses), its project's id, `seconds`
  under its own timeout (Codex's hook `timeout` covers the whole hook, so the hook does not
  call again inside it), and `ack`: the `report_id`s its previous run handed to the session,
  read from a small file the hook keeps beside its script. It then empties that file.
- `none_running`: no attempt of this connection's plans is pending and no report is open: the
  hook ends at once (exit 0), so a turn with no task running stops as usual.
- `nothing_yet`: the bound passed with an attempt still pending: the hook ends (exit 0, no
  decision). The report stays for the next wait.
- `ended`: Claude Code's hook writes the reports to stderr and exits with code 2 (this wakes
  the session); Codex's hook prints `{"decision": "block", "reason": "<the reports>"}` and
  exits 0 (Codex takes the reason as its next prompt). Each report's text carries its
  `report_id`. Only after it has written them out does the hook save their ids to its file, so
  its next run acknowledges them. It never acknowledges a report it has not handed over, and
  there is no acknowledge-only call.
- A report not acknowledged within two minutes is answered again by a later wait, with the same
  `report_id`, so an answer that never reached the session never loses it; a repeat is told by
  its `report_id`.
- A connection revoked, or a project removed from it, while a call waits: the call answers 401
  and the report stays for that project's next valid wait.

## Errors

| When | The screen shows |
|---|---|
| The hook's token is missing or revoked, before or during the wait (401) | nothing in the browser; the hook prints the status and exits 0 |
| The project is no longer on the connection (404) | nothing in the browser; the hook exits 0 |
| An approve of an external proposal is stale (409, as today) | as today |

## Backend answer

As above; the backend proposes these shapes. Services and contracts:

- `wait_url` and the bearer check: Grants (`crates/core-grants/contract.yaml`), task B-260.
- The wait and its acknowledgement: Runs (`crates/core-runs/contract.yaml`),
  `Runs::author_wait`, served as `POST /api/external/task-wait` (B-260) and the MCP tool
  `task_wait` (B-259).
- Proposals with no conversation: Design (`crates/core-design/contract.yaml`), B-250. Approving
  one writes no conversation entry.
- `chosen_by: "external"`: Runs, B-252. `author_waiting`: Runs, B-261; the project stream's
  `TaskProgressChanged` comes when it changes, so no poll is needed.

B-250, B-252, B-260 and B-261 wait for this request to be `accepted`.

## Frontend reply

Accepted, with two additions. Tasks: F-153 to F-156 in `docs/frontend/stages.md`.

1. **Hooks in Settings: accepted as written.** The hook text holds no token (it reads
   `SHADOWS_TOKEN`), so the web offers it on every live connection row, for one of its
   projects, not only once at creation. The web writes, per client, a wait script and the hook
   entry that runs it: PowerShell on Windows, `sh` with `curl` on macOS and Linux. The files go
   in the project's own folder (`Project.directory`), so only sessions opened there wait:
   Claude Code's entry in `.claude/settings.local.json`, Codex's in `.codex/hooks.json` (a
   trusted project), each script and its ack file beside the entry, under `hooks/`. Both
   entries set `"timeout": 600`, and the script sends `seconds: 540`. Codex is the desktop app only: no
   `codex --remote` setup.
2. **Proposals with no conversation: accepted, plus `connection_id`.** Each vision, map and
   roadmap proposal also gains `connection_id: string | null`, the connection whose grant
   proposed it (Grants maps a grant to its connection in `external_connection_project`). Why:
   `grant_id` appears in no read the web has, so with two connections the person could not tell
   which agent proposed; the web names it from `GET /api/external-connections`
   ("from Laptop Codex"), and shows "an external agent" when the connection is gone. The web
   does not use `grant_id`. `VisionProposed`, `MapProposed` and `RoadmapProposed` stay on the
   project stream for an external proposal, with `thread_id: null`; the web refreshes on them
   as today.
3. **`chosen_by: "external"`: accepted as written.** The page shows "Chosen by an external
   agent".
4. **`author_waiting`: accepted, with one condition.** The `TaskProgressChanged` journaled when
   a report is saved or acknowledged carries the task's `workflow_id`, `version` and `task`, as
   today's do: the web refreshes only that task's run on those three, so without them the page
   would not change until reloaded.

Errors: accepted; nothing in the browser.

## Ruling

<The teacher, only when disputed.>
