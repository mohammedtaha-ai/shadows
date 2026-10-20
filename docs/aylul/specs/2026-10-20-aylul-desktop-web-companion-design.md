# AYLUL desktop: shared web interface and a bound companion

Date: 2026-10-20.
Status: Proposed for Mohammed's review. No implementation is authorized by this
document alone.

This file owns the replacement of the desktop main interface with `web/`,
automatic local Core startup for that desktop, and the existing character's
binding to an AYLUL agent. The [brain design](2026-10-19-aylul-claude-code-brain-design.md)
owns conversations, providers, agents, profile homes and secret handling.
The [product host design](../../integration/2026-10-19-product-daemon-hermes-design.md)
keeps its other runtime responsibilities. Source contracts describe what is
implemented; this document describes the intended change.

## 1. Intent and decisions from Mohammed

One main interface serves both the browser and desktop. AYLUL and Shadows keep
the workspace switcher, conversation UI and settings already built in `web/`.
The previous desktop chat/workspace interface is retired.

Mohammed also wants the character connected to AYLUL: he writes to it, sees it
appear and react, and opens the conversation from it.

Confirmed in conversation on 2026-10-20:

- Create a branch and prepare this spec before implementation.
- Use the web interface as the desktop main interface.
- Preserve the desktop character and connect it to AYLUL.
- Text conversation and character reactions are in this slice; microphone,
  speech recognition and spoken replies come later.
- Select one agent in Settings for the character. The selection can be changed
  at any time; opening another agent's conversation does not change it.
- Start the configured local daemon automatically when opening the desktop.

Success is a real Windows journey: launch the desktop, use the existing dashboard,
choose an agent, send it a message, see the independent character react to that
actual turn, hide the main window and still receive the result, then reopen the
same conversation through the character.

## 2. Inspected starting point

Static inspection on `main` at `e259666`, before this document:

| Owner | Observed behavior |
|---|---|
| `web/` | Owns the Shadows/AYLUL workspace switcher, providers, agents, profile-scoped conversations, model selection and streamed replies. |
| `web/src/api/client.ts` | Uses `VITE_SHADOWS_URL`, defaulting to `http://127.0.0.1:4318`; it already makes HTTP requests across origins. |
| `web/src/stream/` | Owns browser SSE/replay. Hidden-page suspension belongs to that browser stream; it cannot establish native background presence. |
| `apps/desktop/vite.config.ts` | Builds a separate main entry and a companion entry. |
| `apps/desktop/src-tauri/src/windows/mod.rs` | Builds a local main webview, close-to-tray behavior, tray actions and a shortcut. |
| `apps/desktop/src/features/companion/DesktopCompanion.tsx` | Uses a constant Idle presentation, despite listening to native visibility/pause state. |
| `apps/desktop/src/features/companion/` | Already has expressions, motion, a Canvas renderer, message lifetime and an in-window speech bubble. |
| Native companion owner | Already provides a transparent independent window, revisioned drawing permission, no-activation show, drag and recovery. |
| Native main capabilities | Local main-only commands; the companion has a separate restricted command set. |
| `shadows serve` | Starts the AppCore HTTP/MCP runtime used by the dashboard, with explicit DB, Node, adapter, harness and AYLUL configuration. |
| `shadows daemon` | Retains the older Node assistant composition and an optional AppCore; it is not required to run the current AYLUL brain. |

The daemon currently serves APIs, not the web page. Its HTTP guard and CORS
allow exact configured origins. Existing source maps/contracts include historical
Hermes and desktop-workspace descriptions; they must be updated as their code is
replaced, without pretending those replacements already exist.

Navigation used Serena's file listing, source contracts and Rust/TypeScript LSP
symbols. No browser, native or provider acceptance was run for this design.

## 3. Selected approach and alternatives

**Selected proposal: bundle the web build into the local Tauri main webview.**
Keep `web/` as the sole source of main-window screens. Produce its build once
per desktop build and assemble it with the separately built companion assets.
Tauri loads local application assets, and the web client calls the selected local
Core API. Browser usage keeps its existing build and API behavior.

This preserves the local native capability boundary and allows a useful startup
screen even when the daemon cannot start.

Alternatives considered:

- Load a daemon-served HTTP page in the native main window. This would require
  adding static serving to the daemon and granting native authority to a network
  origin. It is unnecessary for this slice.
- Copy dashboard screens into the desktop React application. This creates two
  editable interfaces and is rejected.

No iframe, second dashboard source tree or new assistant execution loop is added.

## 4. Ownership and build

`web/` owns the main UI, routes, API calls, stream rendering and conversation
commands. A small desktop integration module supplies native startup status,
presence settings, account actions, hook Activity and native navigation. It is
loaded only in the native runtime; ordinary browser usage works without Tauri.

`apps/desktop/` remains the native shell and companion build owner. Its old
main React entry is replaced by the assembled web build. The companion retains
its own local HTML entry, styles, renderer, native permission lifecycle and
development studio. No companion source is copied into `web/`.

Assembly is a generated build artifact, not another source owner. It must not
overwrite a running executable, use a second persistent Cargo target directory,
or include development fixtures/studio modules in production. Preserve the
current application identifier and protected-store location; rebranding and
installer changes are outside this slice.

Main-route reloads and native navigation must resolve to the packaged main
entry. Missing assets must remain missing assets, never an HTML fallback.
Companion routes/assets cannot become main-window entry points.

## 5. Local daemon startup and connection

### Configuration and identity

A native-owned settings record holds absolute paths to the Shadows executable,
DB, Node, ACP adapter, harness and AYLUL config, plus the loopback API address.
These are references, never provider keys. Existing `aylul.json` remains
`{ "home_root": ... }`; this desktop launch record does not expand it.

Use the existing `shadows serve` runtime. Do not start the dormant Hermes
service or require account login. Validate paths before launch; no PATH
discovery, shell-built command strings or new empty DB as a fallback.

Use a private per-configuration launch record and single-owner lock. The record
identifies the runtime, configured DB and executable, actual endpoint and
process instance. Reuse the existing private descriptor-writing mechanism
where applicable; keep the old product-daemon descriptor format compatible.

A small binary-owned runtime readiness projection supplies a per-start public
runtime ID and an opaque configuration identity. It is configuration metadata,
not an AppCore business service or an authorization token. Compare it with the
native launch record before attaching. Its HTTP schema has one Rust owner and
generated consumers. Never expose DB paths, launch arguments, capabilities or
credentials in the projection.

A listener merely answering on the configured port is insufficient. A foreign
listener, mismatched DB/configuration, stale process instance or incompatible
readiness shape gives a named connection failure. Do not kill it or switch ports,
databases or sources automatically. Native attach additionally verifies the
recorded process instance; a readiness ID alone is not authentication.

### Startup and lifetime

On desktop startup, claim the configuration lock, attach to a matching healthy
runtime or start one hidden process with an argument vector. Concurrent launches
must not create two daemons. Wait for readiness with a bounded timeout, then
make the verified public API origin available before mounting API consumers.

Startup failure leaves a local connection screen with Retry and Edit
configuration, while tray and character controls remain usable. Retry may
reattach/relaunch a confirmed stopped runtime; it never resubmits a chat command.
Do not repeatedly spawn a failing daemon in a background loop.

The daemon remains independent after it starts. Closing main hides to tray.
Quitting the desktop stops its subscriptions and its existing owned hook sidecar,
but leaves the Core daemon and in-flight turns running. A daemon already running
before desktop startup is never treated as an owned child to terminate.
Daemon shutdown continues through its existing operator lifecycle.

### HTTP and native boundaries

Retain the shared HTTP client and browser SSE implementation. Desktop bootstrap
selects one verified loopback origin; feature code cannot change it. Browser
`VITE_SHADOWS_URL` behavior is unchanged. No generic native HTTP proxy is added.

Allow only the exact native main origin in Core's existing origin guard/CORS,
preserving configured browser origins. Use a CSP connection allowlist for the
selected loopback endpoint. Determine and assert the actual WebView2 origin in
native acceptance; do not allow wildcard origins, null origins or arbitrary
remote hosts to make the connection work.

> OPEN: The actual packaged WebView2 Origin and its fetch/EventSource behavior
> have not been measured for the shared main. Trigger: before enabling the Core
> origin/CSP changes, run a local Windows probe of packaged GET, JSON mutation
> and SSE, recording the exact Origin and capability behavior. Close this block
> with that evidence. If the selected local-asset boundary cannot support them,
> amend this section for review rather than broadening origin permissions.

Main remains a packaged local webview. External links use a deliberate system
browser action; block navigation of the privileged surface to remote content.
Companion remains local with its existing narrower permission set. This slice
does not introduce remote Core access or change its existing authentication scope.

## 6. Main UI replacement and retained desktop functions

The packaged main interface shows the same two workspaces and the same
conversations as the browser against the configured DB. No history is copied
between stores and no cloud/Hermes conversation is silently imported.

Desktop-only additions fit the existing web visual system:

- Settings: daemon connection/configuration, companion binding and visibility,
  motion pause, reply-preview preference, and the existing native account actions.
- AYLUL Activity: the existing local hook sessions, requests and person actions.
  These remain observed external-agent activity, distinct from the chosen
  character agent's conversation.
- Native hide-to-tray action and existing tray/shortcut/window recovery behavior.

Keep working sign-in, sign-out and revoke actions reachable through Settings,
without using account login as a prerequisite for local AYLUL. Preserve native
protected session storage and confirmation semantics. Account actions do not
switch the local chat to the retired backend chat path.

Remove the old desktop main shell, Hermes/local-versus-account chat selector,
old bot inspector and replaced presentation dependencies once their live
functions above have been transferred. Remove disabled placeholder navigation.
Keep native auth, hook, sidecar and character owners. Remove retired assistant
IPC exposure only after proving it has no retained caller.

Do not delete `packages/hermes-client`, `services/assistant`, account backend
APIs or historical evidence as incidental cleanup; those have separate consumers
or ownership. Update both source maps and changed contracts with implementation.

## 7. Companion settings and conversation target

The companion represents **one selected live AYLUL agent**, independent of
workspace navigation. Settings lists agents from the existing Profiles API,
showing their names and model/provider metadata. Save the immutable agent slug.

Persist the selection and optional target thread in native desktop preferences,
scoped to the configured installation/DB identity. Keep window visibility/pause
under the existing native window owner. No provider key or conversation content
is persisted in companion preferences.

Selecting an agent is explicit opt-in to its companion presence. Enable/show the
character without stealing focus; the user can hide it or pause motion afterward.
First launch without a selection is unbound and offers Choose an agent. It never
creates a provider, agent or conversation automatically.

Changing the selected agent detaches the prior watcher, clears its preview and
target thread, and advances a binding generation. The old conversation and its
running turn are preserved. Late events from the old generation cannot affect
the new character. Changing the configured DB also clears the binding.

A target thread is established when the person sends from a conversation of the
selected agent, including a draft's first successful creation, or deliberately
chooses Use with companion on such a conversation. Validate that the thread is
AYLUL and its profile matches the selected live agent. Merely viewing a thread,
switching workspace or receiving another agent's reply does not retarget it.

The latest such explicit action wins. Observe one target thread at a time;
other conversations may continue without controlling the character. A failed
send admission must not appear as running. A successfully created thread remains
the target even if its subsequent turn fails. A failed companion-binding update
is reported separately and must not duplicate, cancel or prevent the ordinary
chat send; the person can retry the binding through Use with companion.

Clicking the character or pressing Enter/Space deliberately restores main and
opens its target conversation. With a selected agent but no target, open that
agent's existing new-conversation route; create no thread until Send. Unbound
or deleted selection opens companion Settings. Native navigation is a closed
destination with validated IDs, never an arbitrary URL. The main router receives
the latest pending navigation after it is ready; older acknowledgements cannot
clear a newer request.

Dragging consumes the click as it does today. Showing or updating the character
never focuses main. Explicitly hiding the character wins over sends/replies
until the person enables it again.

## 8. Presence feed and reactions

### Background ownership

A native-owned read-only watcher observes the bound thread's existing Core
operations, entries and SSE. It survives main-route changes, webview reloads and
hide-to-tray. Browser hidden-page stream behavior is unchanged.

Use the existing Core API and durable event semantics. No second turn, model
request, bot history, daemon conversation store or global per-agent polling is
added. Maintain one subscription for the target and bounded snapshot recovery.
Use a maintained SSE implementation rather than creating another framing engine.

Subscribe and reconcile a snapshot/cursor before announcing a live update.
Durable sequence and operation/entry identity prevent reordering and duplicate
completion. On reconnect or app restart, reconcile the current state without
replaying old celebrations or reply bubbles. A failed stream can retry with
bounded backoff; only reads retry.

The watcher publishes a bounded presentation through the existing native
revisioned companion state boundary. Native visibility/recovery remains a
separate authority: conversation state never grants drawing permission. Native
commands are restricted to their owning window; the companion cannot send,
stop, approve, select arbitrary endpoints or read provider credentials.

Binding/retarget commands validate against the chosen daemon before mutation.
Include binding generation and native revision in late-response checks. An agent
deleted or thread removed while subscribed clears the target preview and shows
a named unavailable state. Keep the historical thread readable through the
normal web UI; never select a replacement agent automatically.

### State mapping

Presentation is derived from actual Core status, not model-written claims.

| Observation for the target | Existing expression | Visible meaning |
|---|---|---|
| Unbound, or bound and idle | idle | Choose an agent, or Ready |
| Accepted pending/running turn before reply text | thinking | Thinking |
| Running turn with tool progress | working | Working |
| Live reply text arrives | speaking | Replying; no audio is implied |
| Newly observed live Completed operation | success briefly, then idle | Reply ready |
| Failed or Interrupted operation, missing key/provider, unavailable agent | blocked | Fixed actionable status; full detail stays in main |
| Cancelled operation | idle | Stopped |
| Lost daemon connection or unresolved snapshot | blocked | Reconnecting or Unavailable; do not announce completion |

For simultaneous observations, terminal operation state wins; a newer accepted
operation supersedes the old one. Reply text takes precedence over tool-progress
decoration while text is arriving. Completion is acknowledged once, holds its
success expression for two seconds, then returns to idle if no newer operation
has begun. Unknown statuses show an unavailable state, never success.

Do not use the listening expression to imply an active microphone. Existing
idle motion remains; new art, automatic perching, screen tracking and agent
monitoring alerts are outside this slice.

### Reply preview and window behavior

Show one plain-text preview of a newly completed live assistant reply, with a
Settings switch to turn previews off. Default on after explicit agent binding.
Use at most 240 Unicode characters, clamp visually to two lines, strip markup
and render as text. No tool arguments, user prompts, raw errors or hidden
reasoning go into that preview. The full answer remains in the main chat.

Use the final durable assistant entry for the completed operation, not a guessed
transient tail. If it is not available yet, wait for reconciliation before
showing a preview. Failure to obtain it must not synthesize an answer.

Identify a preview by binding generation, thread, operation and final entry ID.
It expires after eight seconds; receiving the same ID cannot extend that time.
Do not show historical previews after reload/reconnect or newly reveal an
expired preview after unhide. Keep preview data in memory; do not log it.

Preserve the compact transparent window, dragging, no-activation show, fitting,
pause and native render denial. Fit the two-line preview inside its existing
logical bounds; do not introduce a second popup or expand beyond monitor bounds.
Hide cancels rendering but does not cancel the turn or the background watcher.
Reduced-motion preference suppresses decorative motion while retaining status.

## 9. Delivery boundaries and verification

The implementation plan must break work into verified commits in this order:

1. Native Core connection/configuration, single-start/readiness and a packaged
   main displaying the existing dashboard; prove an actual local conversation.
2. Transfer retained native controls/Activity/account entry points into the
   shared main UI, then remove replaced desktop-main code and its dependencies.
3. Agent binding, native background watcher, closed navigation and character
   reactions using existing expressions and the existing Core turn path.
4. Full relevant gates and a real Windows journey, with dated evidence.

This is ordering, not an executable task plan. Before builders start, turn the
cross-owner interfaces into accepted shared requests and explicit Touch lists.
Changed Core endpoints use canonical OpenAPI generation; desktop IPC stays typed
with parity checks. Contracts describe implemented behavior in the same commits.

Automated checks must cover:

- Browser web build remains independent of Tauri; desktop assembly contains the
  shared main plus the companion and excludes the retired main/studio fixtures.
- Connection retry, invalid paths, wrong listener/DB, stale record, simultaneous
  startup and desktop relaunch never create or attach to an unintended daemon.
- Core origin guards accept the exact native origin plus configured browser
  origins and still refuse foreign/null origins; production CSP is enforced.
- Existing model selection, first send, replay and thread/profile scoping work
  through the packaged main against a real Core fixture.
- Binding switches, DB changes, deleted agents/threads and stale generations
  never leak another conversation's state or preview.
- Operation ordering, replay/reconnect, preview deduplication/expiry, cancellation,
  transport failure and hidden-main completion produce the declared states.
- Character input is read-only, IPC/window restrictions remain enforced, drag
  cannot open chat, and pending native navigation survives main reload.
- Retained auth, hook approvals, tray, rendering denial, drawing failure recovery
  and optional companion failure remain independent of one another.

Actual Windows acceptance on an isolated DB/profile copy must demonstrate:

1. Open desktop with a saved launch config and no daemon; exactly one matching
   daemon starts and the packaged dashboard connects without account login.
2. Use Shadows and AYLUL, edit provider/agent settings and complete a real AYLUL
   turn through the existing model choices. Record provider use separately.
3. Bind agent A, send a message, observe thinking/working/replying as reported by
   actual events, then a completion preview. Do not manufacture an absent tool event.
4. Hide main to tray during a turn; character reacts to the result; clicking it
   restores the exact target chat. Closing main never cancels the turn.
5. Switch binding to agent B while A is running; A's late result cannot affect B.
   Changing only the viewed conversation also cannot change the binding.
6. Hide/pause the character, send again, and verify those choices hold while chat
   continues. Character failure leaves main, tray and account actions usable.
7. Restart desktop and attach to the same still-running daemon/DB; no duplicate
   process, historical reply bubble or repeated send is produced.
8. Exercise a retained hook request and native account action with their existing
   confirmation rules, and inspect logs for errors and unintended content.

Fixture tests do not establish native focus, tray suspension, actual origins,
physical fitting, provider response or microphone behavior. Record observed
automated/browser/native/provider results separately. Existing mixed-DPI and
resource-performance gaps stay open unless directly measured.

## 10. Out of scope

Voice/microphone/TTS, a mini-chat composer in the character window, new character
art or animation systems, autonomous approvals/actions, screen surveillance,
cross-device settings, remote daemon mode, cloud-chat parity, model-or-provider
changes, personal/work-monitoring capabilities and installer/rebranding work.

No personal credentials/configuration are migrated by writing this spec.
No implementation plan, product changes, reviewer dispatch, push or deployment
is included in this documentation task.
