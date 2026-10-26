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

- Feature HTTP calls go through `web/src/api/`. `transport/index.ts` is the only file that
  calls `fetch` and the only one that creates the `openapi-fetch` client; every other
  API domain module uses this shared client.
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

## Architecture modules

The architecture test reads this table as its only ownership/dependency graph.
Paths are relative to `web/src`. `/**` covers a subtree; `/*` covers direct
children. Exact selectors take precedence over folder selectors so named test
infrastructure retains its owner. Each source has exactly one most-specific
owner. Entries are public import surfaces, including type imports and reexports.
Missing entries are exact baseline items until their migration task creates them.
`-` in Contract identifies a current module without a colocated contract; this
check does not invent contracts for those modules. Existing contracts must parse
as YAML and identify their mapped source folder.

The table maps today's folders onto the plan's section 6: `shared` covers the
root feedback/policy files; `navigation` owns the settings actions, and feedback
remains a target path. `api/*` exposes domain entries while keeping `api/transport/` private. `workflows` has its own row because its directory owns workflow presentation. These migration
boundaries do not exempt their sources from the other architecture rules.

| Owner | Sources | Entry | Contract | Allowed dependencies |
|---|---|---|---|---|
| `composition` | `main.tsx, app-root.tsx, router.tsx, env.d.ts, app/shell.tsx, app/home.tsx, app/page-pending.tsx` | `main.tsx, app-root.tsx, router.tsx, app/shell.tsx, app/home.tsx, app/page-pending.tsx` | `-` | `api, stream, desktop, ui, shared, agents, agreements, conversation, design, desktop-ui, new-project, project-settings, settings, sidebar, workflows, navigation` |
| `navigation` | `app/navigation/**` | `app/navigation/index.ts` | `app/navigation/contract.yaml` | `-` |
| `api` | `api/**` | `api/*` | `api/contract.yaml` | `-` |
| `stream` | `stream/**` | `stream/index.ts` | `stream/contract.yaml` | `api` |
| `desktop` | `desktop/**` | `desktop/index.ts, desktop/startup.ts` | `desktop/contract.yaml` | `api` |
| `ui` | `components/ui/**` | `components/ui/*` | `-` | `-` |
| `shared` | `app/error-line.tsx, app/copy-button.tsx, app/daemon-status.tsx, app/mode-policy.ts` | `app/error-line.tsx, app/copy-button.tsx, app/daemon-status.tsx, app/mode-policy.ts` | `-` | `api, ui` |
| `agents` | `app/agents/**` | `app/agents/index.ts, app/agents/home.tsx` | `app/agents/contract.yaml` | `api, ui, shared, desktop, navigation` |
| `agreements` | `app/agreements/**` | `app/agreements/index.ts, app/agreements/agreements-page.tsx` | `app/agreements/contract.yaml` | `api, ui, shared` |
| `conversation` | `app/conversation/**` | `app/conversation/index.ts, app/conversation/conversation.tsx, app/conversation/draft.tsx` | `app/conversation/contract.yaml` | `api, ui, shared, desktop, stream, design, workflows, desktop-ui, navigation` |
| `design` | `app/design/**` | `app/design/index.ts, app/design/workspace-page.tsx` | `app/design/contract.yaml` | `api, ui, shared, workflows` |
| `desktop-ui` | `app/desktop/**` | `app/desktop/index.ts, app/desktop/startup.ts, app/desktop/activity.tsx` | `app/desktop/contract.yaml` | `api, ui, shared, desktop, agents` |
| `new-project` | `app/new-project/**` | `app/new-project/index.ts` | `app/new-project/contract.yaml` | `api, ui, shared` |
| `project-settings` | `app/project-settings/**` | `app/project-settings/index.ts, app/project-settings/project-settings.tsx` | `app/project-settings/contract.yaml` | `api, ui, shared` |
| `settings` | `app/settings/**` | `app/settings/index.ts` | `app/settings/contract.yaml` | `api, ui, shared, desktop, agents, desktop-ui, navigation` |
| `sidebar` | `app/sidebar/**` | `app/sidebar/index.ts` | `app/sidebar/contract.yaml` | `api, ui, shared, desktop, stream, agents, new-project, workflows, navigation` |
| `workflows` | `app/workflows/**` | `app/workflows/index.ts, app/workflows/plan-state.ts, app/workflows/plan-page.tsx, app/workflows/plan-map.tsx` | `app/workflows/contract.yaml` | `api, ui, shared, stream, agreements` |
| `test-support` | `test/**, app/test-app.tsx, stream/fake-event-source.ts, architecture.test.ts, architecture.fixtures.ts, codemap.test.ts` | `test/**, app/test-app.tsx, stream/fake-event-source.ts, architecture.fixtures.ts` | `-` | `composition, api, stream, desktop, ui, shared` |

The two `startup.ts` entries expose only pre-readiness dependencies; their static
import graphs, and `main.tsx`'s, cannot reach `api/transport/index.ts`. Route modules listed
beside an `index.ts` remain separate lazy imports declared by that entry.
`app/workflows/plan-state.ts` is a second workflows entry so the sidebar's plan
labels do not statically pull the plan graph into the shell's chunk.

Named exemption **test fixture imports**: a `*.test.ts` / `*.test.tsx` source
may import `test-support` entries for fixtures. Test support may import its own
fixtures. This exempts only that edge from public-entry/dependency checks; tests
retain all other rules. Production runtime reachability from `main.tsx` to any
test or named test-support source always fails.

`architecture.baseline.txt` holds sorted, exact `rule :: importer :: target`
records with occurrence ordinals. New, stale, duplicate or unsorted entries fail.
Adding an identical violation fails too. Remove entries with their violations;
a pure rename relocates entries one-for-one and does not reduce the debt count.
The scanner never updates the baseline. In-memory fixtures exercise the same
TypeScript parser, configured module resolver, symbol checker and comparator.

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
- `web/src/app/navigation`
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
| `web/src/app/desktop/companion-controls.tsx` | Character visibility controls | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/activity.tsx` | Local activity page | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/use-activity.ts` | Visible activity polling | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/sessions-panel.tsx` | Local session presentation | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/approvals-panel.tsx` | Preview-bound person approvals | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/rule-preview.ts` | Rule preview identity | `web/src/app/desktop/contract.yaml` |
| `web/src/api/daemon-origin.ts` | One-time verified native origin | `web/src/api/contract.yaml` |
| `web/src/app-root.tsx` | Shared dashboard mounting, owns the app's one QueryClient | `web/src/desktop/contract.yaml` |
| `web/src/app/desktop/connection-settings.tsx` | Local Core connection form | `web/src/app/desktop/contract.yaml` |
| `web/src/desktop/runtime.ts` | Readiness ordering | `web/src/desktop/contract.yaml` |
| `web/src/desktop/native.ts` | Desktop invocation wrappers | `web/src/desktop/contract.yaml` |
| `web/src/app/desktop/index.ts` | Exposes native connection presentation | `web/src/app/desktop/contract.yaml` |
| `web/src/app/desktop/startup.ts` | Expose connection controls before API initialization | `web/src/app/desktop/contract.yaml` |
| `web/src/desktop/bootstrap.tsx` | Native startup gate with injected connection UI | `web/src/desktop/contract.yaml` |
| `web/src/desktop/bootstrap.test.tsx` | Readiness ordering fixtures | `web/src/desktop/contract.yaml` |
| `web/src/api/aylul-profiles.ts` | AYLUL configuration server state: providers, agents, keys, variables, agent threads | `web/src/api/contract.yaml` |
| `web/src/api/command-id.ts` | Idempotency key reused on each retry of a send | `web/src/api/contract.yaml` |
| `web/src/api/design.ts` | Project design server state | `web/src/api/contract.yaml` |
| `web/src/api/transport/index.ts` | Shared generated HTTP transport | `web/src/api/contract.yaml` |
| `web/src/api/error.ts` | Public daemon failure types | `web/src/api/contract.yaml` |
| `web/src/api/agreements.ts` | Agreement server state | `web/src/api/contract.yaml` |
| `web/src/api/code.ts` | Code index server state | `web/src/api/contract.yaml` |
| `web/src/api/directories.ts` | Filesystem directory server state | `web/src/api/contract.yaml` |
| `web/src/api/harnesses.ts` | Harness capabilities server state | `web/src/api/contract.yaml` |
| `web/src/api/native-activity.ts` | Native activity query identity | `web/src/api/contract.yaml` |
| `web/src/api/projects.ts` | Project server state | `web/src/api/contract.yaml` |
| `web/src/api/sessions.ts` | Harness session server state | `web/src/api/contract.yaml` |
| `web/src/api/threads.ts` | Thread server state | `web/src/api/contract.yaml` |
| `web/src/api/workflows.ts` | Workflow server state | `web/src/api/contract.yaml` |
| `web/src/api/saved-answer.ts` | A mutation's answer written to the cache over any earlier read | `web/src/api/contract.yaml` |
| `web/src/api/schema.d.ts` | Types generated from the daemon's OpenAPI document | `web/src/api/contract.yaml` |
| `web/src/app/agents/agent-env.tsx` | One agent's environment variables, values write-only | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/agent-settings.tsx` | Agent editing form | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/delete-zone.tsx` | Danger zone confirming the deletion of a provider or agent | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/form-field.tsx` | Labelled controls drawn for the AYLUL forms | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/home.tsx` | Main pane of the AYLUL workspace when no conversation is open | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/provider-form.tsx` | Form for one provider, its key write-only | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/provider-presets.ts` | Provider preset grouping | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/provider-picker.tsx` | Searchable provider choice | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/provider-models.tsx` | Provider model selection | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/model-picks.ts` | Provider model validation | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/character-colors.ts` | Shared character colour lookup | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/color-swatches.tsx` | Character colour controls | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/provider-test.ts` | Provider test outcome messages | `web/src/app/agents/contract.yaml` |
| `web/src/app/agreements/agreement-document.ts` | Reads OpenAPI object fields for the editor | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/agreement-editor.tsx` | Editor for one agreement version | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/agreements-page.tsx` | Page listing agreement identities with versions | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/binding-editor.tsx` | Binds a task to an exact Agreed contract | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/names.tsx` | Display names for a part id or a version writer | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/operations-editor.tsx` | Structured editor for an agreement's operations | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/party-picker.tsx` | Picks participant identities from a nested tree | `web/src/app/agreements/contract.yaml` |
| `web/src/app/agreements/index.ts` | Expose app/agreements public capabilities | `web/src/app/agreements/contract.yaml` |
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
| `web/src/app/conversation/index.ts` | Declare lazy route surfaces | `web/src/app/conversation/contract.yaml` |
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
| `web/src/app/design/use-outcome-pages.ts` | Outcome hierarchy pages from the daemon | `web/src/app/design/contract.yaml` |
| `web/src/app/design/vision-editor.tsx` | Vision as five item lists, edited one item at a time | `web/src/app/design/contract.yaml` |
| `web/src/app/design/vision-fields.ts` | Display names of the vision fields | `web/src/app/design/contract.yaml` |
| `web/src/app/design/workspace-page.tsx` | Design workspace page, opening on Vision | `web/src/app/design/contract.yaml` |
| `web/src/app/design/workspace-plans.tsx` | Project plan list, with its archive filter | `web/src/app/design/contract.yaml` |
| `web/src/app/design/index.ts` | Expose app/design public capabilities | `web/src/app/design/contract.yaml` |
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
| `web/src/app/new-project/index.ts` | Expose app/new-project public capabilities | `web/src/app/new-project/contract.yaml` |
| `web/src/app/page-pending.tsx` | Pane shown while a lazy page loads | `web/src/app/home.tsx` |
| `web/src/app/project-settings/code-index.tsx` | Code index status with its linked projects | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/external-agents.tsx` | External agents allowed on the project, with grants | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/instructions-editor.tsx` | Editor for the Planner's project instructions | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/project-modes.tsx` | Checklist of the project's allowed modes | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/project-settings.tsx` | Project settings page, assembled from its sections | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/remove-project.tsx` | Danger-zone control that removes the project | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/when.ts` | Formats a stored time for the settings page | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/project-settings/index.ts` | Declare lazy route surfaces | `web/src/app/project-settings/contract.yaml` |
| `web/src/app/settings/active-projects.tsx` | Editor for how many projects the index keeps active | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/settings-dialog.tsx` | URL-selected settings dialog | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections.ts` | Settings section availability | `web/src/app/settings/contract.yaml` |
| `web/src/app/navigation/index.ts` | Settings URL navigation hooks | `web/src/app/navigation/contract.yaml` |
| `web/src/app/settings/sections/general.tsx` | General settings content | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections/account.tsx` | Account state controls | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections/character.tsx` | Character settings content | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections/core.tsx` | Local Core settings content | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections/providers.tsx` | Provider settings navigation | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/sections/agents.tsx` | Agent settings navigation | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/account-actions.ts` | Account state action selection | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/you-card.tsx` | Person name editing | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/general-preferences.tsx` | General person preference editing | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/person-theme.tsx` | Person theme synchronization | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/desktop-preferences.tsx` | Native preference controls | `web/src/app/settings/contract.yaml` |
| `web/src/desktop/preferences.ts` | Desktop preference invocation wrappers | `web/src/desktop/contract.yaml` |
| `web/src/desktop/index.ts` | Expose desktop public capabilities | `web/src/desktop/contract.yaml` |
| `web/src/desktop/startup.ts` | Expose native bootstrap before API initialization | `web/src/desktop/contract.yaml` |
| `web/src/api/person.ts` | Person server state | `web/src/api/contract.yaml` |
| `web/src/api/usage.ts` | Usage report server state | `web/src/api/contract.yaml` |
| `web/src/api/extensions.ts` | Extension library server state | `web/src/api/contract.yaml` |
| `web/src/app/settings/sections/plugins.tsx` | Extension library navigation | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/extension-import.tsx` | Daemon folder import form | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/extension-list.tsx` | Shared extension removal controls | `web/src/app/settings/contract.yaml` |
| `web/src/app/agents/agent-extensions.tsx` | Per-agent extension enablement | `web/src/app/agents/contract.yaml` |
| `web/src/app/agents/index.ts` | Expose app/agents public capabilities | `web/src/app/agents/contract.yaml` |
| `web/src/app/settings/sections/usage.tsx` | Usage period selection | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/usage-period.ts` | Usage interval calculation | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/usage-tables.tsx` | Usage report presentation | `web/src/app/settings/contract.yaml` |
| `web/src/app/settings/index.ts` | Expose app/settings public capabilities | `web/src/app/settings/contract.yaml` |
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
| `web/src/app/sidebar/index.ts` | Expose app/sidebar public capabilities | `web/src/app/sidebar/contract.yaml` |
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
| `web/src/app/workflows/index.ts` | Expose app/workflows public capabilities | `web/src/app/workflows/contract.yaml` |
| `web/src/components/ui/alert.tsx` | Alert box with its title, body, action | `web/src/components/ui/input.tsx` |
| `web/src/components/ui/button.tsx` | Button, styled by its variant props | `web/src/components/ui/input.tsx` |
| `web/src/components/ui/dialog.tsx` | Modal dialog built on Base UI's Dialog | `web/src/components/ui/button.tsx` |
| `web/src/components/ui/dropdown-menu.tsx` | Dropdown menu built on Base UI's Menu | `web/src/components/ui/button.tsx` |
| `web/src/components/ui/input.tsx` | Text input styled for the app | `web/src/components/ui/button.tsx` |
| `web/src/components/ui/popover.tsx` | Popover built on Base UI's Popover | `web/src/components/ui/button.tsx` |
| `web/src/components/ui/tooltip.tsx` | Hover label built on Base UI's Tooltip | `web/src/components/ui/button.tsx` |
| `web/src/env.d.ts` | Declares the build-time environment variables | `web/src/main.tsx` |
| `web/src/main.tsx` | Composes native callbacks before app mounting | `web/src/router.tsx` |
| `web/src/router.tsx` | The app's routes, each open thing a URL | `web/src/main.tsx` |
| `web/src/stream/fake-event-source.ts` | Test-only EventSource driven frame by frame | `web/src/stream/contract.yaml` |
| `web/src/stream/frames.ts` | Typed reading of each subscribe frame | `web/src/stream/contract.yaml` |
| `web/src/stream/tab-id.ts` | This tab's id, sent with every turn | `web/src/stream/contract.yaml` |
| `web/src/stream/thread-stream.ts` | Client side of one thread's replay, then live feed | `web/src/stream/contract.yaml` |
| `web/src/stream/thread-registry.ts` | Share resumable connections within a cache scope | `web/src/stream/contract.yaml` |
| `web/src/stream/refresh.ts` | Refresh API-owned queries from stream events, once per shared stream | `web/src/stream/contract.yaml` |
| `web/src/stream/use-plan-events.ts` | Hold writer-thread plan refresh | `web/src/stream/contract.yaml` |
| `web/src/stream/use-project-events.ts` | Refetches project plan views on journal notices | `web/src/stream/contract.yaml` |
| `web/src/stream/use-thread-stream.ts` | Bind a registry subscription to a component | `web/src/stream/contract.yaml` |
| `web/src/stream/index.ts` | Expose stream public capabilities | `web/src/stream/contract.yaml` |
| `web/src/test/contract-fixtures.ts` | Sample daemon values typed against `schema.d.ts` | `web/src/test/fake-daemon.ts` |
| `web/src/test/fake-daemon.ts` | Route table of a fake one-project daemon | `web/src/test/contract-fixtures.ts` |
| `web/src/test/fake-resize-observer.ts` | Test ResizeObserver resized by hand | `web/src/test/fake-daemon.ts` |
| `web/src/architecture.test.ts` | Enforces the frontend ownership ratchet | `docs/frontend/plans/2026-10-25-web-architecture.md` |
| `web/src/architecture.fixtures.ts` | Supplies in-memory architecture rejection cases | `web/src/architecture.test.ts` |
