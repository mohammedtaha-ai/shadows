# NYLRA delivery design

Date: 2026-10-10. Updated: 2026-10-18. Status: N0 measured; acceptance exit OPEN.
Windows first, mobile afterward. Libraries and rendering choices are evaluated at
the start; initial plans must change to follow the evidence before dependent work.

## 1. Ownership

This file owns AYLUL delivery, client capability evaluation and acceptance gates.
Size review (over 300 lines): delivery decisions and their dated gate history stay
in this owner; character implementation, native windows and transition execution
refer to their own specs/plans instead of duplicating those responsibilities here.
[Vision](../vision.md) owns the full experience;
[source structure](../source-structure.md) owns client placement;
[shared architecture](../../../../shadows-backend/docs/architecture.md) owns cloud
data and authority. Backend J1 owns the wire protocol. Shadows §1.7, §24.6 and
§17.12 own workspace placement, external approval and Production respectively.

The product is an unobtrusive companion that explains real work and brings the
person's decisions to the owner. It must remain useful as text with the character
hidden or animation reduced. AYLUL is optional presence, not a condition for access.

### Independent companion

Decided with Mohammed on 2026-10-13; runtime allocation amended 2026-10-18.
AYLUL is an independent assistant and the primary desktop experience. It observes and talks to the agents the person works with
(Shadows, Claude Code and Codex now, others later) and is none of them.
[Coucou](../../evidence/nylra/2026-10-10-coucou-and-library-research.md) is the
reference shape: a companion with its own model that shows live agent sessions
and carries the person's decisions to them.

- **Its own brain.** The personal-first direction uses a Hermes AYLUL profile on
  the server. Its model context/memory are separate from coding-agent sessions.
  Existing `aylul-brain`/account-chat code remains implemented work until an explicit
  retirement/migration task; it is not a second engine for the same AYLUL turn.
- **Local runtime.** The tested Node sidecar owns local session integration,
  bounded context, dispatch receipts and the brain connection. It is independent
  of Shadows AppCore. The current local runtime does not silently become a second
  model loop. Phone/cloud continuation is later work with its own acceptance.
- **Memory.** The selected pilot uses Hermes profile memory under the existing
  content/source/disclosure rules. No automatic transfer or deletion of backend
  data is implied. Project content stays with its owner and is read on demand;
  a new brain does not authorize retaining complete project transcripts/code.
- **Agents.** Preserve the tested Claude Code `aylul-hook` path and its permission
  behavior. Other hooks/live control paths are enabled only after their actual
  version/capability trials. The hook carries events to the local sidecar; it is
  not itself proof of arbitrary prompt submission. Shadows is read through its local API's NYLRA projection
  ([Shadows §1.7](../../superpowers/specs/2026-09-21-architecture-design.md#17-workspace-authority-and-deployment)).
- **The cloud link belongs to AYLUL desktop.** The sidecar owns the connection to
  the selected Hermes brain/relay. Its reviewed transport may use a private route
  or an outbound relay; neither Core nor raw Serena is exposed publicly. Existing
  backend relay code is preserved, and remote control/sharing gates remain open.
- **Optional workspaces.** AYLUL opens the existing Shadows UI on request. Code
  intelligence is a separate install/runtime usable without Core or AYLUL. The
  character and its design remain independently owned and optional.

**Delegation.** A grant is a permission rule the person approved on the laptop,
in Claude Code's syntax (`Edit(/src/app.ts)`, `Bash(npm test)`), per agent and
project, with an expiry. For Claude Code, AYLUL writes it into the project's
`.claude/settings.local.json` and Claude Code enforces it; AYLUL never allows a
call by a hook decision under a grant (amended 2026-10-13, see
[step 2(a)](2026-10-13-aylul-hook-laptop-design.md#5-rules)). Model output, file
content and session text never create or match a grant. A request no rule
covers goes to the person, on the laptop and the phone. Every decision records
who made it: `person`, or `aylul_grant` with the grant's id for a call a grant
allowed. The phone and the cloud can revoke a grant, never create or widen one.
Codex follows once its hooks are tested. Grants over Shadows operations are
Shadows' own (§24.6) and are outside the first release. Create structure is
never delegable.

**Failure is not consent.** If AYLUL is off, the link is lost or a request
times out, the hook hands control back to the agent's own prompt. Nothing is
approved by default.

### Current personal-first delivery (2026-10-18)

The [integration plan](../plans/2026-10-18-aylul-hermes-semantic-context.md)
starts from the existing hook/native/character work, proves code/context retrieval
and Hermes integration, then proves addressed direction delivery. Independent
code intelligence has its own baseline/package gate. The
[transition plan](../plans/2026-10-18-nylra-transition.md)
preserves documents, tests, assets and evidence while retiring replaced decisions.
The earlier phone-first sequence below is historical ordering, not the active
task list. Character/native owner specs and their recorded acceptance still apply.

### Historical first-release ordering (2026-10-13)

The phone has three screens: chat, live sessions with their steps, and pending
approvals. Grants are managed on the laptop. Session steps sent to the cloud
carry the tool name, file path and a one-line summary, never diffs or file
contents, until architecture §5's lifecycle closes.

0. **Cleanup.** Each probe's findings go into a dated evidence file naming its
   commit; probe code and probe branches are then deleted. One review of the
   kept authentication code against the project's structure rules, fixing what
   it finds.
1. **Contract.** nylra-v1 0.2 with fixtures and a mock server that replays them,
   SSE included; the backend's J1 owns it. Both tracks start after it is agreed.
2. **In parallel.** Mohammed's friend: AYLUL's avatar states, which can start
   now, and the Flutter app's three screens against the mock server. Mohammed:
   (a) `aylul-hook` with sessions and approvals on the laptop against Claude Code
   on Windows, no cloud; (b) the backend relay, session and approval routes, and
   push; (c) `aylul-brain`, memory, chat and the Shadows read; (d) grants.
3. **Integration** of the real phone, backend and laptop.

Outside the first release: AWS, voice, Shadows approvals through AYLUL, ACP.

## 2. AYLUL asset direction

Current appearance/renderer owner: [Character's Canvas specification](../character/specs/2026-10-16-aylul-in-window-design.md).
The [asset provenance index](../../../assets/aylul/README.md) records the extraction
and archived PNG references. The raster-selection history below describes the
earlier accepted images; it does not replace the accepted procedural renderer
or require a new avatar design. Keep its dated evidence and remaining gates distinct.

Mohammed selected the **AYLUL Companion Character Reference Board.png** during N0.
He later preferred idle-v3 and approved cleaned idle-v3, listening-v5 and blocked-v4
after the live preview, including their measured state-lighting variation.
The historical canonical pack, `assets/aylul/states/v1/manifest.json` at `04b1231`,
records the accepted source hashes; its idle sprite was the appearance reference
for that raster stage.
The [asset continuation](../../evidence/nylra/2026-10-11-aylul-asset-continuation.md)
establishes non-destructive alpha=1 cleanup with exact RGB preservation and zero
alpha in the outer8% margin. Do not substitute the unselected idle-v4 redraw.
Appearance review is closed for these three images; native motion/performance and
event-driven presentation remain separate N1b gates. The six remaining expressions
were prepared in `assets/aylul/candidates/2026-10-11/manifest.json` at `04b1231`
and await their own review; [dated evidence](../../evidence/nylra/2026-10-12-approved-aylul-and-six-expressions.md)
records the selected corrected versions and measurements.
Preserve that original unchanged. On 2026-10-11 he intentionally deleted the
alternative Shadow Companion Design Sheet to avoid confusing the references;
its historical original remains in Git. Use only the selected simpler silhouette.
Both were compared at 64/96/128 logical pixels on the current 100% Windows display.
They are reference boards, not animation atlases; do not ship the board as a sprite.

Prepare original transparent production assets with fixed canvas/pivot/scale,
consistent lighting and silhouette, no text/background, and an asset manifest of
origin/version/license. Initial expressions: idle, happy, listening, speaking,
thinking, working, success, blocked and sleeping. Directional views and accessories
follow only once base states are consistent. State changes use actual product events:
success needs an observed outcome; disconnected/pending are separate status labels.

Maintain scene/presentation state separately from business state. Speech animation
follows playback, not token arrival; listening means capture is active. A blocked
project does not imply all projects are blocked. Multiple incidents retain explicit
scope; the character may indicate attention while the panel identifies the cause.

## 3. Experiment gate N0

| Experiment | Acceptance evidence | Result-dependent choice |
| --- | --- | --- |
| Raster states with browser drawing | Readable eyes/silhouette at small sizes; clean alpha edges; no distortion or missing state | Start here if sufficient |
| Layered animation | Compare visual fidelity, authoring effort, frame-time, memory and idle CPU against the same states | Evaluate PixiJS for raster layers; do not add it for a single static image |
| Rive alternative | Author one representative interaction and verify its fidelity/runtime/tooling requirements | Select only if rig workflow benefits the desired art; never promise PNG-to-rig conversion |
| Tauri Windows surface | Transparency, drag, focus, click-through bounds, tray, single instance, shortcuts, sleep/resume and display removal | Prefer official plugins; isolate only necessary native Windows code |
| DPI and accessibility | 100/150/200 percent scaling, mixed-DPI displays, keyboard access, reduced motion and Arabic/English layout | Repair behavior before declaring the shell ready |

Use identical representative assets/interactions for renderer comparisons. Record
versions, measurements and screenshots with scenario and machine. Define an agreed
performance budget from measured baseline before accepting the renderer; do not
claim battery/performance benefit from an idle screenshot. Stop frame work while
hidden and test that behavior. No global animation framework is required by default.

The code from Coucou is a reference, not our product base. Its Windows manifests
and selected state logic were inspected; it was not run. Before any reuse, review
the exact source at a pinned revision, preserve applicable MIT notices and record
provenance. Never copy its protected Mochi character, branding, sounds or animations.
Use native/window patterns only where they satisfy NYLRA's capabilities and trust.

**Exit:** choose renderer/native libraries from results, amend this spec and plan,
pin dependencies and record rejected alternatives. Mohammed approved removing
probe sources on 2026-10-13: each probe's evidence names the commit that holds it.

### N0 observations and remaining gates

[Dated evidence](../../evidence/nylra/2026-10-10-aylul-windows-probes.md) records the
2026-10-10 run; [follow-up evidence](../../evidence/nylra/2026-10-11-aylul-windows-probes.md)
records actual 2026-10-11 trials. N0's exit is **OPEN**.
No product source or later task was implemented.
That statement describes the earlier dated probes. The later authorized
[continuation](../../evidence/nylra/2026-10-11-windows-shell-continuation.md) implements
the independent N1a main shell and local login primitives; N1b/joint login stay OPEN.

- Browser raster remains the minimal experiment baseline. Canvas and CSS atlas
  animation both displayed three synthetic states, but active CPU exceeded the
  provisional budget. The historical comparison had different window conditions;
  the matched follow-up still exceeded the budget on the current 144Hz display.
  Final renderer selection closes only after accepted art, release profiling,
  presentation-timing evidence and a reviewer-approved performance budget.
- Two generated transparent trials failed alignment/padding and listening-state
  distinction. Neither is a shipping asset. Close asset authoring/provenance with
  approved consistent three-state samples before producing the complete nine-state
  pack. Turning/directional animation and other expressions remain untested.
- No layered need justified PixiJS. Rive remains gated on an available authoring
  workflow and one original interaction; its runtime was not tested.
- Tested native baseline: Tauri 2.11.5, build/macros/codegen/plugin 2.6.3, utils
  2.9.3, runtime 2.11.3 and runtime-wry 2.11.4; official single-instance 2.4.5 and
  global-shortcut 2.3.2. Preserve the probe lockfile; broad internal version ranges
  resolved a graph that failed compilation. These pins do not select untested
  React/Vite dependencies or establish production readiness.
- Native hide did not update document.hidden in the trial. Explicit native
  visibility signaling is required to cancel animation work. Default hide/show
  stole focus even after set_focusable(false); reject that attention policy.
  Parking and SW_SHOWNOACTIVATE preserved focus in isolated probes. Select a final
  method only after combined rendering/visibility/monitor/suspend lifecycle proof.
- Transparency, simple drag, stationary rectangular click-through, duplicate
  launch, shortcut and one keyboard action passed narrow native trials. Tray API
  construction and actual owned Left Down/Up click delivery passed in the follow-up.
  Tray menu activation remains unverified. No-activate show paired with explicit
  rendering visibility preserved foreground in the immediate follow-up test;
  complete display/suspend lifecycle still gates final native adoption.
- Real 150/200% and mixed DPI, physical display removal and suspend/resume are
  still gated on corresponding device evidence. Resizing a 96-DPI window to
  128/192/256 pixels is not a DPI test. Full RTL/accessibility and real OS reduced
  motion remain unverified.
- Coucou source snapshots and complete license notices are retained at the pinned
  revision in the report. No Coucou implementation was imported into the probe and
  no protected character/media was copied.

The animated companion implementation (N1b) waits for closure of its N0
asset/render/native gates. The authorized independent N1a main shell and native
login plumbing can proceed without shipping rejected character art. This is not
closure of the full N0 or Windows acceptance gate.
Unresolved choices must not become default libraries, fake passing checks or
speculative product interfaces.

### Identity acceptance ownership

Windows capability acceptance is NYLRA-owned. Linux identity/provider/session
acceptance is shadows-backend-owned. Joint login acceptance follows only after
both sides run together. NYLRA owns system-browser launch, a listener bound only
to IPv4 127.0.0.1 with an ephemeral port and `/oidc-return`, a locally generated
S256 return verifier, OS-protected opaque-session persistence, restart validation
and logout behavior. The verifier and session handle must stay out of webview IPC.
Backend identity validation, ticket validity/replay, account/device mapping and
session revocation remain in shadows-backend.

NYLRA now consumes backend auth-v1 1.1.0 and its safe fixtures in native Rust.
The main surface invokes a fixed action set and receives only a credential-free
account projection. reqwest 0.12.28 uses verified HTTPS with an explicit public
local CA; requests have a 15-second bound, no redirect following and bounded bodies.
The current accepted endpoint is the local shadows-local realm; production issuer
configuration remains separate. DPAPI atomically protects an origin-bound envelope
in the application's data directory, retaining session and enrollment independently.
On restart, stored bytes never establish identity without GET session acceptance.
401 refuses the identity projection while preserving enrollment; 503 permits retry.
Only first enrollment or explicit user-selected new enrollment sends mode new;
ordinary reauthentication proves reuse. No failure silently falls back to new.
Logout clears a handle only after backend confirmation, preserves enrollment,
and reports provider completion as pending after browser handoff. Device revoke
clears proof only after successful backend surrender. Native cancellation closes
the listener, with bounded in-flight requests and best-effort revoke if already issued.
The [joint evidence](../../evidence/nylra/2026-10-11-joint-native-auth.md) separates actual
native/Docker acceptance, system-browser launch and remaining full UI acceptance.

## 4. Windows architecture and interaction

Retain React/TypeScript/Vite presentation and Tauri Rust platform integration.
Provide a companion surface plus a main topic/source/settings surface. Give each
window minimal capabilities. Generated remote text never becomes privileged HTML
or a native command. A confirmation view renders trusted proposal data independently
of the generated explanation and reports queued/accepted/completed separately.

Feature state/actions use a data layer for server projections and narrow platform
wrappers for native calls. Do not duplicate server authorization in a UI store.
Use ordinary local state for simple view interaction; add a state-machine library
only if the measured transition complexity benefits. Select server-cache/UI utility
libraries at N0/N2 against actual reconnect and accessibility needs, not by inventory.

Chat goes to the laptop brain, or to the cloud brain when the laptop is off.
AYLUL desktop owns the cloud link (§1); closing the main window must not close
it, only quitting AYLUL does. Local Shadows access is the NYLRA projection plus
presentation/navigation.
Workspace/account switches isolate cached evidence and pending input. Reconnect
resumes through J1, with explicit resnapshot when needed; a cached view is not fresh.

## 5. Scope of our own modules

Own AYLUL visual-state mapping, native window behavior, source/freshness presentation,
topic navigation and trusted confirmation interaction. Use maintained libraries for
rendering, routing/cache when justified, protocol generation and native plugins.
No custom transport stack, model SDK, ORM or queue belongs in the desktop app.
Do not publish a custom component framework before another actual consumer exists.
Use the shared source-contract convention for behavior-bearing units.

## 6. Voice, mobile and Production

Text remains the fallback. Voice acceptance proves permission denial, interruption,
capture stopped, playback stopped and no stale audio after topic switch. STT and TTS
remain separate from reasoning; the client owns capture/playback, not cloud dialogue.
Voice-only human approval remains gated by Shadows §24.6.

Flutter is built in parallel with Windows against the same versioned protocol,
its fixtures and its mock server (§1); it generates Dart from the backend's
OpenAPI file and never edits a copy.
Android/iOS delivery order and available physical devices are decided at the mobile
gate, not assumed here. Native lock-screen/background limitations need per-platform
evidence; a WebView preview is not phone acceptance. No forced TS/Dart code sharing.

Production adds scoped incident cards with source time, facts versus hypotheses,
proposed remedy, cost assumptions, required authority and post-action result.
The companion never independently scales AWS resources. Teams, multiple instances,
advanced memory, richer accessories, external integrations and enterprise hosting
remain in the Vision inventory and need their own complete slices.

## 7. Acceptance

The first release is accepted by a real run on Windows and a real phone:

1. Claude Code asks to edit a file; the phone gets a push; the person allows it
   there; Claude Code continues.
2. A grant covers `npm test`; Claude Code runs it without asking, and AYLUL's
   record says it ran on the person's behalf under that grant.
3. The laptop is turned off; the phone shows it offline and chat continues with
   the cloud brain.
4. The grant is revoked from the phone; the next such request goes to the person.

Few, exact tests: grant matching with its refusals; a failed or absent AYLUL
approves nothing; both sides conform to the contract; a request already decided
refuses with 409. A native Windows trial must include display/focus behavior.
Shadows approval claims wait for backend B4 and real Shadows enforcement.

Research: [Coucou and library record](../../evidence/nylra/2026-10-10-coucou-and-library-research.md).
Backend: [foundations owner](../../../../shadows-backend/docs/backend/specs/2026-10-10-backend-foundations.md).
