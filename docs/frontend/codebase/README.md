# Frontend code map

**Read this before writing code in `web/`.** It answers one question: *where does my
new code go?* It names the one job each source file under `web/src` holds, and the
invariants no change may break. Each feature folder's `contract.yaml` sits next to
its code and follows [`docs/codebase/contracts/TEMPLATE.yaml`](../../codebase/contracts/TEMPLATE.yaml):
what must stay true of that folder. `web/src/codemap.test.ts` fails when a row here
names a file that does not exist, when a source file has no row, or when a folder
listed under Contracts has no `contract.yaml`. The map is checked on every
`npm test`, so it cannot go stale quietly. Paths are relative to the repository root.

## Architecture Invariants

What the code keeps true. Each one was checked by reading the code that enforces it.

- Feature HTTP calls go through `web/src/api/`. `client.ts` is the only file that
  calls `fetch` and the only one that creates the `openapi-fetch` client; every other
  API module uses this shared client.
- `web/src/api/schema.d.ts` is generated from the daemon's `api/openapi.json` by
  `npm run gen:api`. It is committed, its header says not to edit it, and CI fails
  on any difference after regeneration. A protocol change is made in the daemon and
  regenerated, never patched here.
- The event stream is read only through `web/src/stream/`. Only that folder opens an
  `EventSource`; the features bind to it through its hooks (`use-thread-stream.ts`,
  `use-project-events.ts`), so the replay-then-live rules live in one place.
- `web/src/components/ui/` holds shadcn/ui primitives that know nothing of Shadows.
  Nothing in it imports from `app/`, `api/`, or `stream/`.
- `web/src/api/` and `web/src/stream/` import nothing from `web/src/app/`. Data flows
  up from them into the features, never back down.
- Test-only code never reaches the product. `web/src/test/`, `web/src/app/test-app.tsx`
  and `web/src/stream/fake-event-source.ts` are imported only by `*.test.ts(x)` files.
- Feature folders reach shared code by path (`@/api/…`, `@/components/ui/…`); the
  shared pieces at the root of `web/src/app/` (`error-line.tsx`, `copy-button.tsx`,
  `daemon-status.tsx`, `mode-policy.ts`) are imported by the feature folders that need them.

## Contracts

Each folder below carries a `contract.yaml` next to its code, written to the template
above. `codemap.test.ts` checks that each one exists.

- `web/src/desktop`
- `web/src/app/desktop`
- `web/src/api`
- `web/src/stream`
- `web/src/app/agreements`
- `web/src/app/conversation`
- `web/src/app/design`
- `web/src/app/new-project`
- `web/src/app/project-settings`
- `web/src/app/settings`
- `web/src/app/agents`
- `web/src/app/sidebar`
- `web/src/app/workflows`

## What each file owns

One row per source file under `web/src`, in folder order. The job is stated without
"and": a conjunction means the file has quietly taken a second responsibility. The
reference file is the one to copy when adding something similar; for a feature folder
it is that folder's `contract.yaml`.

| File | Its one job | Reference file |
|---|---|---|
| `web/src/desktop/companion-send.ts` | Independent companion send targeting | `web/src/desktop/contract.yaml` |
| `web/src/desktop/presence.ts` | Companion binding bridge | `web/src/desktop/contract.yaml` |
| `web/src/desktop/navigation.ts` | Closed native navigation delivery | `web/src/desktop/contract.yaml` |
| `web/src/app/desktop/companion-settings.tsx` | Companion binding controls | `web/src/app/desktop/contract.yaml` |
| `web/src/desktop/auth.ts` | Account invocation wrappers | `web/src/desktop/contract.yaml` |
| `web/src/desktop/activity-api.ts` | Sidecar invocation wrappers | `web/src/desktop/contract.yaml` |
| `web/src/app/desktop/native-settings.tsx` | Native settings sections | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/companion-controls.tsx` | Character visibility controls | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/activity.tsx` | Local activity page | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/use-activity.ts` | Visible activity polling | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/sessions-panel.tsx` | Local session presentation | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/approvals-panel.tsx` | Preview-bound person approvals | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/rule-preview.ts` | Rule preview identity | `web/src/app/desktop/contract.yaml` |
| `web/src/api/daemon-origin.ts` | One-time verified native origin | `web/src/api/contract.yaml` |
| `web/src/app-root.tsx` | Shared dashboard mounting | `web/src/desktop/contract.yaml` |
| `web/src/app/desktop/connection-settings.tsx` | Local Core connection form | `web/src/app/desktop/contract.yaml` |
| `web/src/desktop/runtime.ts` | Readiness ordering | `web/src/desktop/contract.yaml` |
| `web/src/desktop/native.ts` | Desktop invocation wrappers | `web/src/desktop/contract.yaml` |
| `web/src/desktop/bootstrap.tsx` | Native startup gate | `web/src/desktop/contract.yaml` |
| `web/src/desktop/bootstrap.test.tsx` | Readiness ordering fixtures | `web/src/desktop/contract.yaml` |
| `web/src/api/aylul-profiles.ts` | HTTP calls for AYLUL configuration: providers, agents, keys, variables, agent threads | `web/src/api/contract.yaml` |
| `web/src/api/client.ts` | The one door to the daemon's HTTP API | `web/src/api/contract.yaml` |
| `web/src/api/command-id.ts` | Idempotency key reused on each retry of a send | `web/src/api/contract.yaml` |
| `web/src/api/design.ts` | HTTP calls for the project design workspace | `web/src/api/client.ts` |
| `web/src/api/error.ts` | Every failed daemon call as one typed error | `web/src/api/contract.yaml` |
| `web/src/api/queries.ts` | TanStack Query keys for each piece of daemon state | `web/src/api/client.ts` |
| `web/src/api/schema.d.ts` | Types generated from the daemon's OpenAPI document | `web/src/api/client.ts` |
| `web/src/app/agents/agent-env.tsx` | One agent's environment variables, values write-only | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/agent-settings.tsx` | One agent's settings page, or a new agent's | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/delete-zone.tsx` | Danger zone confirming the deletion of a provider or agent | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/form-field.tsx` | Labelled controls drawn for the AYLUL forms | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/home.tsx` | Main pane of the AYLUL workspace when no conversation is open | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/provider-form.tsx` | Form for one provider, its key write-only | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/provider-settings.tsx` | Providers page: picks the provider its form edits | `web/src/app/agents/contract.yaml` |
| `web/src/app/agreements/agreement-document.ts` | Reads OpenAPI object fields for the editor | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/agreement-editor.tsx` | Editor for one agreement version | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/agreements-page.tsx` | Page listing agreement identities with versions | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/binding-editor.tsx` | Binds a task to an exact Agreed contract | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/names.tsx` | Display names for a part id or a version writer | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/operations-editor.tsx` | Structured editor for an agreement's operations | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/party-picker.tsx` | Picks participant identities from a nested tree | `web/src/app/agreements/contract.yaml` |
| `web/src/app/conversation/approved-by.tsx` | Attribution line linking to the approving conversation | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/bidi.ts` | Text direction for each block of a reply | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/carried-send.ts` | Carries a failed first message to its thread page | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/cli-picker.tsx` | Chooses the CLI a conversation runs on | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/composer-bar.tsx` | Flat row of composer menus under the message box | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/composer.tsx` | Message box that sends to the Planner, with Stop | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/context-ring.tsx` | Context-usage ring with its two popover levels | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/conversation.tsx` | Open conversation pane, assembled from its parts | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/deleted-conversation.tsx` | Readable history of a removed conversation | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/draft.tsx` | New conversation before its first message is sent | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/entry.tsx` | Renders one durable thread entry by its kind | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/focus-chip.tsx` | Chip naming the task a turn points at | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/map-proposal-card.tsx` | Card for an immutable map proposal, with approval state | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/message-actions.tsx` | Hover actions under one message: copy, fork | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/messages.tsx` | Ordered durable entries, then the running turn's text | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/operational-label.ts` | Maps a harness meta label to a person's line | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/plan-card.tsx` | Plan version shown as a card in the conversation | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/plan-side-panel.tsx` | Plan version shown beside the conversation | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/reply-text.tsx` | Renders the Planner's markdown in the reply voice | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/reply.ts` | Transient streamed reply text with its lifetime | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/roadmap-proposal-card.tsx` | Card for an immutable roadmap proposal, with approval state | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/session-commands.ts` | Reads the /model, /effort commands as picker choices | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/slash-filter.ts` | Decides which / menu entries a composer text lists | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/slash-menu.tsx` | Draws the / menu beside the composer | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/stage-chip.tsx` | Project stage as one line in the header | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/status-badge.tsx` | Badge for the latest turn's status | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/stream-banner.tsx` | Says the stream is not live, offering reconnect | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/subagent-card.tsx` | Card for one subagent in the conversation | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/subagent-panel.tsx` | Panel showing what one subagent did | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/subagent-text.ts` | Formats a subagent's figures for display | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/tool-text.ts` | Reads a tool call as one line for the conversation | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/turn-settings.ts` | Model, mode, effort the next turn runs with | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/turn-state.ts` | What the client knows of a thread's turn state | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/usage.ts` | Turns harness-reported context figures into readable figures | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/use-commands.ts` | Keeps each thread's slash-command list in the cache | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/use-conversation.ts` | Binds an open conversation to its stream, entries, turns | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/use-session.ts` | Harness session state the conversation screen reads | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/vision-proposal-card.tsx` | Card for a vision proposal, with approval state | `web/src/app/conversation/contract.yaml` |
| `web/src/app/conversation/waiting-messages.tsx` | Queued messages waiting to be sent or removed | `web/src/app/conversation/contract.yaml` |
| `web/src/app/copy-button.tsx` | Button that copies a text to the clipboard | `web/src/app/error-line.tsx` |
| `web/src/app/daemon-status.tsx` | Shows whether the daemon answers, with the reason | `web/src/app/error-line.tsx` |
| `web/src/app/design/create-structure.tsx` | Confirms the project's structure write | `web/src/app/design/contract.yaml` |
| `web/src/app/design/map-part-details.tsx` | Read-only details of one map part | `web/src/app/design/contract.yaml` |
| `web/src/app/design/outcome-editor.tsx` | Editor for one outcome, keeping unsaved input | `web/src/app/design/contract.yaml` |
| `web/src/app/design/part-editor.tsx` | Editor for one part, keeping unsaved input | `web/src/app/design/contract.yaml` |
| `web/src/app/design/part-references.tsx` | Picks the existing parts an outcome refers to | `web/src/app/design/contract.yaml` |
| `web/src/app/design/parts-view.tsx` | Bounded navigator through the part containment tree | `web/src/app/design/contract.yaml` |
| `web/src/app/design/roadmap-view.tsx` | Navigator for outcomes, without derived completion | `web/src/app/design/contract.yaml` |
| `web/src/app/design/standards-view.tsx` | Standards page: read-only base, editable additions | `web/src/app/design/contract.yaml` |
| `web/src/app/design/use-design-editor.ts` | Save lifecycle shared by the design editors | `web/src/app/design/contract.yaml` |
| `web/src/app/design/use-design-pages.ts` | Collects hierarchy pages while revisions agree | `web/src/app/design/contract.yaml` |
| `web/src/app/design/use-outcome-pages.ts` | Outcome hierarchy pages from the daemon | `web/src/app/design/contract.yaml` |
| `web/src/app/design/use-part-pages.ts` | Part hierarchy pages from the daemon | `web/src/app/design/contract.yaml` |
| `web/src/app/design/vision-editor.tsx` | Vision as five item lists, edited one item at a time | `web/src/app/design/contract.yaml` |
| `web/src/app/design/vision-fields.ts` | Display names of the vision fields | `web/src/app/design/contract.yaml` |
| `web/src/app/design/workspace-page.tsx` | Design workspace page, opening on Vision | `web/src/app/design/contract.yaml` |
| `web/src/app/design/workspace-plans.tsx` | Project plan list, with its archive filter | `web/src/app/design/contract.yaml` |
| `web/src/app/error-line.tsx` | Inline line saying why a daemon call failed | `web/src/app/copy-button.tsx` |
| `web/src/app/home.tsx` | Main pane shown when no project is open | `web/src/app/shell.tsx` |
| `web/src/app/mode-policy.ts` | Shadows' mode policy per harness, for display | `web/src/app/error-line.tsx` |
| `web/src/app/new-project/folder-browser.tsx` | Browses one daemon directory to pick a folder | `web/src/app/new-project/contract.yaml` |
| `web/src/app/new-project/folder-state.ts` | Current folder of the new-project browser | `web/src/app/new-project/contract.yaml` |
| `web/src/app/new-project/new-folder.tsx` | Creates a named folder in the shown directory | `web/src/app/new-project/contract.yaml` |
| `web/src/app/new-project/new-project-dialog.tsx` | Dialog that creates a project from its fields | `web/src/app/new-project/contract.yaml` |
| `web/src/app/new-project/new-project-provider.tsx` | Owns whether the New project dialog is open | `web/src/app/new-project/contract.yaml` |
| `web/src/app/new-project/open-new-project.ts` | Opens the New project dialog from any door | `web/src/app/new-project/contract.yaml` |
| `web/src/app/new-project/slug.ts` | Derives a project's slug from its name | `web/src/app/new-project/contract.yaml` |
| `web/src/app/page-pending.tsx` | Pane shown while a lazy page loads | `web/src/app/home.tsx` |
| `web/src/app/project-settings/code-index.tsx` | Code index status with its linked projects | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/external-agents.tsx` | External agents allowed on the project, with grants | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/instructions-editor.tsx` | Editor for the Planner's project instructions | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/project-modes.tsx` | Checklist of the project's allowed modes | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/project-settings.tsx` | Project settings page, assembled from its sections | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/remove-project.tsx` | Danger-zone control that removes the project | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/when.ts` | Formats a stored time for the settings page | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/settings/active-projects.tsx` | Editor for how many projects the index keeps active | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/settings-dialog.tsx` | URL-selected settings dialog | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections.ts` | Settings section availability | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/use-settings.ts` | Settings URL navigation hooks | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections/general.tsx` | General settings content | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections/account.tsx` | Account state controls | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/account-actions.ts` | Account state action selection | `web/src/app/settings/contract.yaml` |
| `web/src/app/shell.tsx` | Two-pane frame every screen sits in | `web/src/app/home.tsx` |
| `web/src/app/sidebar/agent-list.tsx` | Sidebar agent tree with their conversations | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/sidebar/delete-thread-dialog.tsx` | Confirms removing a conversation as one command | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/sidebar/open-projects.ts` | Remembers which projects the sidebar shows open | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/sidebar/project-list.tsx` | Sidebar project tree with its conversations, plans | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/sidebar/sidebar.tsx` | Left pane holding the sidebar's sections | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/sidebar/thread-list.tsx` | An open project's conversations in the sidebar | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/sidebar/workflow-list.tsx` | An open project's plans in the sidebar | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/sidebar/workspace-switcher.tsx` | Sidebar title as the workspace menu | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/sidebar/workspace.ts` | Which workspace the sidebar shows, the page deciding before the stored choice | `web/src/app/sidebar/contract.yaml` |
| `web/src/app/test-app.tsx` | Test harness running the app over a fake daemon | `web/src/test/fake-daemon.ts` |
| `web/src/app/workflows/approve-bar.tsx` | Approval control for a Draft plan, with refusals | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/inspect-panel.tsx` | Panel showing one task beside the graph | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/layout.ts` | Places a plan's graph with dagre | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/linked-layout.ts` | Plan-qualified identities for linked tasks | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/linked-node.tsx` | Graph node for a related plan's task | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/plan-camera.ts` | Where the plan graph's view looks | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/plan-edge.tsx` | Draws one link of the plan graph | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/plan-frames.ts` | Refetches plans when their thread's stream changes | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/plan-graph.tsx` | A plan version drawn as its graph canvas | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/plan-map.tsx` | Latest-plan map of a project | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/plan-page.tsx` | Workflows page for one plan version | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/plan-state.ts` | How a plan version's state reads to a person | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/task-node.tsx` | Draws one node of the plan graph | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/use-plan.ts` | One plan version, kept current for a component | `web/src/app/workflows/contract.yaml` |
| `web/src/app/workflows/version-line.tsx` | Line naming a version's writer, with its reason | `web/src/app/workflows/contract.yaml` |
| `web/src/components/ui/alert.tsx` | Alert box with its title, body, action | `web/src/components/ui/input.tsx` |
| `web/src/components/ui/button.tsx` | Button, styled by its variant props | `web/src/components/ui/input.tsx` |
| `web/src/components/ui/dialog.tsx` | Modal dialog built on Base UI's Dialog | `web/src/components/ui/button.tsx` |
| `web/src/components/ui/dropdown-menu.tsx` | Dropdown menu built on Base UI's Menu | `web/src/components/ui/button.tsx` |
| `web/src/components/ui/input.tsx` | Text input styled for the app | `web/src/components/ui/button.tsx` |
| `web/src/components/ui/popover.tsx` | Popover built on Base UI's Popover | `web/src/components/ui/button.tsx` |
| `web/src/components/ui/tooltip.tsx` | Hover label built on Base UI's Tooltip | `web/src/components/ui/button.tsx` |
| `web/src/env.d.ts` | Declares the build-time environment variables | `web/src/main.tsx` |
| `web/src/main.tsx` | Mounts the app, with its one QueryClient | `web/src/router.tsx` |
| `web/src/router.tsx` | The app's routes, each open thing a URL | `web/src/main.tsx` |
| `web/src/stream/fake-event-source.ts` | Test-only EventSource driven frame by frame | `web/src/stream/contract.yaml` |
| `web/src/stream/frames.ts` | Typed reading of each subscribe frame | `web/src/stream/contract.yaml` |
| `web/src/stream/tab-id.ts` | This tab's id, sent with every turn | `web/src/stream/contract.yaml` |
| `web/src/stream/thread-stream.ts` | Client side of one thread's replay, then live feed | `web/src/stream/contract.yaml` |
| `web/src/stream/use-project-events.ts` | Refetches project plan views on journal notices | `web/src/stream/contract.yaml` |
| `web/src/stream/use-thread-stream.ts` | Binds a thread stream to a component's lifetime | `web/src/stream/contract.yaml` |
| `web/src/test/contract-fixtures.ts` | Sample daemon values typed against `schema.d.ts` | `web/src/test/fake-daemon.ts` |
| `web/src/test/fake-daemon.ts` | Route table of a fake one-project daemon | `web/src/test/contract-fixtures.ts` |
| `web/src/test/fake-resize-observer.ts` | Test ResizeObserver resized by hand | `web/src/test/fake-daemon.ts` |
