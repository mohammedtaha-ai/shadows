# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

PR #78 (`261c1abe`) is on `main`: the delivery links on Roadmap, Map, State and the plan graph (F-150 to F-152), and
named external connections with an editable project scope (F-149). PR #77 (`3b17985f`) is on `main`: the Planner's
project switches (F-145 to F-148). PR #76 (`e7b9332`): the project recheck on the plan page (F-137, F-138) and the
screens that follow the stream instead of polling (F-139 to F-144). The project's state view, its ER diagram and screen
flow, and `code_where` in the web (F-130 to F-136) are on `main` in PR #75 (`4d69977d`).

Every task's checks run from `web/`: the tests it names, then `npm run typecheck`, `npm run lint`,
`npm run build` and `npx vitest run src/codemap.test.ts src/architecture`.

### Shipped

A shipped task's entry, and its plan once deleted, are in this file in Git at the commit named.

| PR | Work | Tasks | Entries at |
|---|---|---|---|
| #40 | The settings dialog | F-2 to F-12, F-14, F-15 | `1a2b601` |
| #41 | Architecture restructuring, web side | F-1, F-23 to F-25 | `c7e7bac4` |
| #43 | W-1 to W-8 | F-16, F-17, F-26, F-27, F-29 to F-32 | `ca618f89` |
| #44 | W-9, W-10, W-12, W-13 | F-20, F-21, F-33 to F-44 | `4d5c022d` |
| #45 | The context ring shows the summary only | F-22 | `29ddfd88` |
| #46 | Each settings form saves only what changed | F-28 | `e95b79c3` |
| #47 | W-11, with B-14 to B-18 | F-45 to F-49 | `4f23c717` |
| #48 | Web polish | F-50 to F-61 | `ce31036d` |
| #49 | AYLUL agents and panel, and the AI Studio UI migration | F-62 to F-76, F-77 (accepted 2026-11-03) | `b40e077b`, `a7b4009c` |
| #51 | The agent form starts empty; the background tasks panel | F-78, F-79 | `16da6b8c` |
| #52, #53 | Per-tool permissions in the agent form | F-80 | `9c7c7e5a` |
| #54 | Agent groups in the main window | F-81 to F-85 | `9e5b9ab2` |
| #57 | Planner context, the inspection view | F-86 to F-89 | `1b413fc4` |
| #60, #61 | Structure and depth, slices 2 to 5 (tried in the browser 2026-11-07) | F-90 to F-95 | `b64242a7`, `a4f89386` |
| #62 to #64 | Conversation order, Codex, the task page | F-96, F-97, F-98 | `3a443ea0`, `217d7540`, `c71b263d` |
| #65 | Task page review, the Diff tab, the Planner's notices | F-106 to F-113 | `617d806f` |
| #67 | Conversation images, the question card, Settings › Code | F-114 to F-116 | `db760593` |
| #68 | Attempt briefs and contract conflicts on the task page and in the Planner's notices | F-102, F-117 to F-120 (skeleton loading and task-page follow-ups) | `d73bb646` |
| #69 | Checks on the task's Report tab, the done reason under its title, and replies in the conversation | F-103 | `4031168f` |
| #70 | Interrupted and stopped attempt cards, the recheck card, decisions with Accept and in the attempt brief (shared request `docs/shared/requests/2026-11-16-slice-6-stop-control.md` at `e78e07dc`) | F-121 to F-124 | `449d4f95` |
| #71 | Executor choice on a task and in a new conversation | F-125 | `0e281585` |
| #72 | Settings › Code as one collapsible row, the plan page's real progress (its read is in `docs/backend/stages.md` at `753b26a4`), the visual acceptances recorded | F-126 to F-128 | `753b26a4` |
| #73 | The Report tab shows the `contract` check as Shadows' own (F-129; plan `docs/backend/plans/2026-11-19-code-record.md` at `7b7abec`) | F-129 | `7b7abec` |
| #75 | The project's state view, its ER diagram and screen flow, pinned elements in plan review and in the attempt brief, `code_where` and its contract inputs and return (the shared requests `docs/shared/requests/2026-11-19-project-state-and-diagrams.md` and `docs/shared/requests/2026-11-19-code-where.md`) | F-130 to F-136 | `4d69977d` |
| #76 | The project recheck on the plan page and its live terminal panel; the screens follow the project stream instead of polling (B-208 to B-210 in `docs/backend/stages.md` at `e7b9332`) | F-137 to F-144 | `e7b9332` |
| #77 | The Planner's project switches: the agent form's switch shared, the project's switch calls, the section "What the Planner may use", and a library change reaching the switches (plan `docs/frontend/plans/2026-11-25-planner-project-switches.md` at `3b17985f`; shared request `docs/shared/requests/2026-11-25-planner-project-switches.md`) | F-145 to F-148 | `3b17985f` |
| #78 | External connections: names and editable project scope (F-149; plan `docs/backend/plans/2026-11-26-external-connection-management.md` at `261c1abe`); related tasks on Roadmap, Map and State, approval requirements and return links, and real dependency status on the plan graph (F-150 to F-152; plan `docs/frontend/plans/2026-11-26-delivery-traceability.md` at `261c1abe`) | F-149 to F-152 | `261c1abe` |

Dropped without being built: F-99 and F-101 (covered by F-102's brief panel and F-116's server list; the rest is in [future](../core/future.md)), F-100 (with B-102), F-104 and F-105 (with the repair plan; slice 6 replanned them as F-121 to F-124). Their entries and plan are at `d997393a`.

PRs #59 and #66 have no `F-` task. Two composer fixes in PR #61 have none either: `1b85b6e6`
(the textarea grows to 192px) and `e6e6e9cf` (`dir="auto"`, so Arabic runs right to left).

### Known and left

- The light palette and F-114's composer tiles are recorded in `docs/evidence/2026-11-18-visual-acceptance.md`.
  Left to Mohammed: the tiles inside a sent message and the desktop login item (start with Windows).
- After PR #79 (`6680cfd8`), web `architecture.test.ts` runs near its 60 s limit; no task yet.

### Before a new task

Its shared request must be accepted and its backend response must exist. Expand the API hooks,
routing, event refresh, contracts and code-map paths first. The old mockup's `Built` label is not
evidence of completion.

## External agents like the Planner (F-153 to F-156)

Spec: [external connections, "An external agent works like the Planner"](../backend/specs/2026-11-18-external-agent-connections-design.md#an-external-agent-works-like-the-planner-mohammed-2026-11-28).
Shared request: [`2026-11-28-external-agents-like-the-planner.md`](../shared/requests/2026-11-28-external-agents-like-the-planner.md)
(accepted 2026-11-28, with `connection_id` on proposals). Backend: B-249 to B-262 in
`docs/backend/stages.md`. Every decision is Mohammed's (2026-11-28). This section is the plan.

Rules for every task here:
- An external agent proposes; it never approves. The person approves with the button.
- Shadows never wakes an external agent; the agent's client waits through its Stop hook. The
  clients are Claude Code and the Codex desktop app. No `codex --remote` setup anywhere.
- After the backend task a task needs is built, run `npm run gen:api` first; never edit
  `schema.d.ts` by hand.

### F-153 · Settings → External agents gives each connection's Stop hooks to copy

- Status: blocked (needs B-260: `wait_url` on `ExternalConnection`)
- Plan: this section, and the request's "What a hook does with the route".
- Read: `web/src/app/settings/contract.yaml`; `web/src/app/settings/sections/external-agents.tsx`
  (`ConnectionRow`, `CopyLine`); `web/src/api/external-connections.ts`; `Project.directory` in
  `web/src/api/schema.d.ts`.
- What it shows: on each connection row that is not revoked, a **Wait hooks** button opens a
  panel with a project choice (the connection's `project_ids`, by name), a client choice
  (Claude Code, Codex desktop app) and a platform choice (Windows, or macOS and Linux; it starts
  on Windows when `navigator.userAgent` names Windows), then three `CopyLine`s: the wait
  address (`wait_url`), the script with its file path, and the hook entry with its file path.
  A note says the files hold no token: the script reads `SHADOWS_TOKEN`, the connection's token
  saved at creation, from the environment that starts the client (as Codex's registration
  already does; now for Claude Code too). On macOS and Linux it says the script needs `curl`
  and `jq`. A project with no `directory` shows "This project has no folder recorded" and no
  hook.
- The text, written by one pure function `hookSetup({ client, platform, waitUrl, projectId,
  directory })` → `{ scriptPath, script, entryPath, entry }` in a new file `external-hooks.ts`:
  - Paths, under the project's folder `D`: Claude Code `D/.claude/hooks/shadows-wait.{ps1|sh}`,
    entry in `D/.claude/settings.local.json`; Codex `D/.codex/hooks/shadows-wait.{ps1|sh}`,
    entry in `D/.codex/hooks.json` (Codex reads it in a trusted project). The ack file is
    `shadows-wait.ack` beside the script. Windows paths use `\`.
  - The run command: Windows `powershell -NoProfile -ExecutionPolicy Bypass -File "<script>"`;
    macOS and Linux `sh "<script>"`.
  - Claude Code entry: `{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"<run>",
    "asyncRewake":true,"timeout":600}]}]}}`. Codex entry: `{"hooks":{"Stop":[{"hooks":
    [{"type":"command","command":"<run>","timeout":600}]}]}}`. Both pretty-printed.
  - The script (PowerShell, or `sh` with `curl` and `jq`; one logic): with `SHADOWS_TOKEN`
    empty, print `Shadows: SHADOWS_TOKEN is not set` and exit 0. Read the ack file's ids (one
    per line; none if it is missing). Make **one** `POST <wait_url>` with `Authorization: Bearer
    $SHADOWS_TOKEN`, body `{"project_id":"<id>","seconds":540,"ack":[…]}`, client timeout
    570 s. A status other than 200: print `Shadows wait: <status>` and exit 0, keeping the ack
    file. On 200: empty the ack file; `none_running` or `nothing_yet`: exit 0. `ended`: the
    text is each report, oldest first, as `Shadows report <report_id> (task <task>, <kind>)`
    with its `report` on the next lines, reports separated by a blank line. Claude Code: write
    the text to stderr. Codex: print `{"decision":"block","reason":<text>}` to stdout (built by
    `ConvertTo-Json` / `jq -n --arg`). Only then write the reports' ids to the ack file, and
    exit (Claude Code 2, Codex 0).
- Touch: `web/src/app/settings/sections/external-agents.tsx`, new
  `web/src/app/settings/sections/external-hooks.ts`, new
  `web/src/app/settings/sections/external-hooks.test.ts`,
  `web/src/app/settings/sections/external-agents.test.tsx`, `web/src/test/fake-daemon.ts`
  (`wait_url` on its connections), `web/src/app/settings/contract.yaml`,
  `docs/frontend/codebase/README.md` (the new file's row).
- Needs: the request (accepted); B-260 built.
- Done when: `npx vitest run src/app/settings` passes with
  `the_claude_code_hook_wakes_with_async_rewake` (entry path, `asyncRewake: true`,
  `timeout: 600`; the script exits 2 with the reports on stderr),
  `the_codex_hook_blocks_with_the_reports_as_reason`,
  `each_script_acknowledges_only_after_writing_the_reports` (in both languages the ack-file
  write comes after the output),
  `the_script_sends_540_seconds_and_the_project_and_reads_the_token_from_the_environment`
  (no token in any generated text), `windows_paths_and_the_powershell_run_command`,
  `a_connection_row_offers_wait_hooks_for_its_projects`,
  `a_revoked_connection_offers_no_hooks`, `a_project_with_no_folder_offers_no_hook`. B-262
  runs both hooks live.

### F-154 · Vision, map and roadmap proposals from an external agent, approved on their page

- Status: todo (B-250 is built: `10bf056e`)
- Plan: this section.
- Read: `web/src/app/design/contract.yaml`, `web/src/app/conversation/contract.yaml`
  (the proposal cards' agreements); `vision-editor.tsx` (the pending aside), `parts-view.tsx`
  (the same, and "The first map comes from the Planner"), `roadmap-view.tsx`;
  `vision-proposal-card.tsx`, `map-proposal-card.tsx`, `roadmap-proposal-card.tsx`,
  `use-proposal-approval.ts`.
- What it shows: a proposal whose `thread_id` is null is an external one. The pending asides
  count and link only proposals with a conversation, as now. Below them, a new section
  "From external agents" lists each pending, not outdated, external proposal of that page's
  kind as its card with **Approve**, headed "From <connection name>" (the name from
  `connectionsQuery` by `connection_id`; "an external agent" when it is not found). Roadmap
  shows this section too (today it shows no proposal). Map's empty line becomes "The first map
  comes from the Planner or an external agent." An outdated external proposal's status reads
  "Outdated — ask its agent to propose again" (the Planner's wording stays for a thread).
  Approved and superseded external proposals are not listed.
- How: each of the three cards takes either `entry` (as now) or `proposal` (the live row) and
  renders the same body and approval from it; a new `external-proposals.tsx` in
  `app/conversation`, exported from its `index.ts`, lists one kind's external proposals; the
  three design views render it. If the architecture test reports a cycle, stop and report.
- Touch: `web/src/api/schema.d.ts` (by `npm run gen:api` only; it also brings B-252's
  `external` chooser for F-155), `web/src/app/conversation/vision-proposal-card.tsx`, `map-proposal-card.tsx`,
  `roadmap-proposal-card.tsx`, `use-proposal-approval.ts`, new
  `web/src/app/conversation/external-proposals.tsx` and its test,
  `web/src/app/conversation/index.ts`, `web/src/app/conversation/contract.yaml`,
  `web/src/app/design/vision-editor.tsx`, `parts-view.tsx`, `roadmap-view.tsx`, their tests,
  `web/src/app/design/contract.yaml`, `web/src/test/fake-daemon.ts`,
  `docs/frontend/codebase/README.md`.
- Needs: the request (accepted); B-250 built.
- Done when: `npx vitest run src/app/conversation src/app/design src/architecture` passes
  with `an_external_vision_proposal_is_approved_on_the_vision_page` (posts the existing
  approve route once; a retry reuses its command id), `an_external_proposal_names_its_connection`,
  `an_external_proposal_of_a_removed_connection_says_an_external_agent`,
  `the_pending_aside_links_only_proposals_with_a_conversation`,
  `the_roadmap_page_lists_external_roadmap_proposals`,
  `an_outdated_external_proposal_asks_its_agent`, and the existing card tests unchanged.

### F-155 · The task page says an executor was chosen by an external agent

- Status: todo (B-252 is built: `2157fcbf`)
- Plan: this section.
- Read: `web/src/app/tasks/contract.yaml`; `executor-settings.tsx` (the "Chosen by" line).
- What it shows: "Chosen by you" for `person`, "the Planner" for `planner`, "an external agent"
  for `external`; each value matched by name, none by default.
- Touch: `web/src/app/tasks/executor-settings.tsx`, `web/src/app/tasks/task-page.test.tsx`,
  `web/src/app/tasks/contract.yaml`.
- Needs: the request (accepted); B-252 built.
- Done when: `npx vitest run src/app/tasks` passes with
  `the_executor_line_names_each_chooser` (all three values).

### F-156 · A task's result waiting for its external author shows so

- Status: blocked (needs B-261)
- Plan: this section.
- Read: `web/src/app/tasks/contract.yaml`; `task-header.tsx`, `use-task-run.ts`;
  `web/src/stream/contract.yaml` (`TaskProgressChanged` refreshes that task's run).
- What it shows: while `author_waiting` is true, the task header shows a status line "Result
  waits for its external author" with a muted note "It is judged when the agent that wrote this
  plan waits again." It goes when the field turns false, through the project stream, with no
  poll. No button: Shadows wakes no one.
- Touch: `web/src/app/tasks/task-header.tsx`, `web/src/app/tasks/task-page.test.tsx`,
  `web/src/app/tasks/contract.yaml`, `web/src/test/fake-daemon.ts`.
- Needs: the request (accepted); B-261 built.
- Done when: `npx vitest run src/app/tasks src/stream` passes with
  `a_result_waiting_for_its_author_says_so` and
  `the_waiting_line_goes_when_the_stream_reports_the_acknowledgement` (a `TaskProgressChanged`
  with that task's `workflow_id`, `version` and `task` refetches the run).
