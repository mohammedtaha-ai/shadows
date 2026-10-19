# AYLUL in-window Canvas companion

Date: 2026-10-16.
Status: written specification approved for planning on 2026-10-16; implementation awaits plan review.
Owner: Character. Desktop UI owns the shell embedding described in section 7.

## 1. Intent and agreed scope

Turn the supplied root `ayl.html` into maintainable TypeScript/React source in
the existing desktop application. Preserve the character the user selected:
dark hood, purple lighting, expressive eyes, upper and hanging crystals, small
body and hands. The user chose presentation inside the existing NYLRA window.

Success means the existing shell displays the companion, development can review
its expressions and accessories using the same renderer, and the feature has
clear ownership and lifecycle boundaries. This is presentation work; simulated
commands in the source HTML do not become working agent or conversation features.

The user approved the in-chat design on 2026-10-16. This document makes that
design reviewable before the implementation plan. It does not authorize skipping
the written-spec and plan reviews. Work uses a normal Git branch in the existing
checkout; the user explicitly forbids worktrees.

## 2. Inspected baseline and sources

- Baseline: `origin/main` at `cf670bb`, inspected on 2026-10-16. The current task
  branch is `codex/aylul-companion-in-window`.
- Input: root `ayl.html`, 114148 bytes, SHA-256
  `103a5ba2765c4a0f81f9b93051d2883fef1dc79a7e49dc8cb4ee0d0ff76a8bae`.
  It is currently untracked. Preserve it during extraction; do not ship the
  monolithic page as an executable application asset.
- `renderAylulCharacter`, `drawTrue3DCrystal`, `drawExpressiveEye3D`,
  `draw3DShadowPaws`, `drawOutfitAccessory`, `drawStateEffects` and
  `draw3DHoloPedestal` draw through Canvas 2D. Depth is simulated through
  projection, gradients and layered drawing; no WebGL engine is required.
- `animateLoop` currently mixes physics, random blinking, speech expiry,
  document measurement and rendering twelve canvases when all previews are
  enabled. Some effects read the global `engine` directly.
- `runSimulatedTask`, `executeAssistantPrompt` and `triggerNotificationDemo`
  generate synthetic outcomes. The source has no implemented dialogue service,
  microphone capture or native notification delivery.
- `apps/desktop/src/app/main.tsx` composes account presentation and an unavailable
  AYLUL section. React StrictMode is enabled. Native account access goes through
  `apps/desktop/src/platform/`; the shell must remain usable independently.
- The app uses React, TypeScript, Vite and Tauri. Its CSP restricts resources to
  the application and existing IPC endpoints. Keep that policy intact.
- The user supplied screenshots of `ayl.html` in idle and speaking states.
  These support the chosen appearance, not animation, performance or comparison
  with the other HTML file. Live browser observation was blocked by the tool's
  `file://` policy; no runtime acceptance is claimed here.

Architecture and source-placement owners remain [Vision](../../vision.md),
[shared architecture](../../../../../shadows-backend/docs/architecture.md), and
[source-contract convention](../../../../../shadows-backend/docs/contracts/README.md).

## 3. Selected approach and alternatives

Extract a typed, instance-local Canvas 2D renderer and animation model, then
embed them through React. Reuse the inspected drawing paths and layer ordering
instead of redrawing the character. Local CSS replaces the page's Tailwind CDN
and Google Fonts dependencies; use the application's system-font stack.

Keeping the whole HTML page in an iframe would retain its global state, inline
handlers, CDN dependencies and synthetic assistant behavior. A WebGL/3D rewrite
would change the artwork and introduce a rendering stack without serving this
scope. Neither is selected.

## 4. Module boundaries

All paths in this table are proposed implementation destinations, not existing
modules. Create only the units used by the delivered feature.

| Destination under `apps/desktop/src/features/companion/` | One job |
| --- | --- |
| `types.ts` | Presentation state, settings and frame-input types; no backend DTO copies |
| `catalog.ts` | Typed expression, outfit and direction catalogs and setting bounds |
| `renderer/` | Draw a complete character frame from explicit inputs; focused files for character composition, eyes, crystals, hands, outfits and scene effects |
| `animation/` | Instance-local motion model, time advancement, blink and pointer-response state, and cancellable frame scheduling |
| `CompanionCanvas.tsx` | Own a canvas instance, element sizing, interaction, scheduling and cleanup |
| `SpeechBubble.tsx` | Render supplied text accessibly with correct wrapping and direction |
| `Companion.tsx` | Compose the production presence and its accessible controls |
| `audio.ts` | Optional synthesized interaction tones and their resource lifecycle |
| `studio/` | Development-only controls, mock scenes, simulation, preview sizes and PNG export |
| `companion.css` | Feature-scoped styles and responsive layout |

The renderer accepts a Canvas 2D context and a complete frame description. It
does not inspect the DOM, mutate animation state, start timers, choose work
outcomes, play audio or read a global `engine`. Render the same frame inputs
consistently; randomness is sampled by the model, with an injectable random
source for verification. Scene effects receive their particle and pulse inputs.

Animation state belongs to each mounted instance. React state holds controls,
messages and external presentation inputs; per-frame physics stays outside React
render state. Main presence and studio do not share mutable simulation state.
No `window.AYLUL`, inline event handlers or imperative HTML-string construction
is carried into the application.

## 5. Presentation and state flow

The production flow is: shell supplies presentation inputs -> companion updates
its local animation model -> scheduler advances motion -> renderer draws a frame.
Pointer actions affect local look, lean and a brief poke response. They cannot
authorize work or report a completed operation.

Retain all nine expression identifiers: `idle`, `happy`, `listening`, `speaking`,
`thinking`, `working`, `success`, `blocked`, `sleeping`. Retain the ten outfits:
`default`, `crown`, `moon`, `scarf`, `starry`, `winter`, `headphones`, `mask`,
`halo`, `special`. Preserve the six direction presets and the input HTML's
initial geometry, palette and setting values as the extraction baseline.

Production initially uses `idle`, the default outfit and no speech. No operational
event provider exists in the inspected shell, so this slice does not invent one.
Keep a typed controlled input boundary for a later owner-provided state/message;
do not add a speculative transport, backend schema or sidecar connection.

In future integration, `listening` requires actual capture, `speaking` requires
actual speech playback, `working` requires an owner-reported active operation,
and `success` requires its successful outcome. Visual animation is not evidence
of any of those activities. Queued, accepted and completed outcomes remain
distinct in the owner and its accompanying textual presentation.

The normal presence occupies a responsive, bounded area in the existing AYLUL
section. Default tracking changes gaze within that area; free movement following
the pointer is a studio scene. Give the poke interaction a keyboard-equivalent,
named control. Local interaction does not overwrite a controlled operational
expression. Provide pause/resume and sound controls; sound starts off and is
enabled only by an explicit user gesture. Preferences are in memory for this
slice, with no persistence or new settings service.

Move speech text out of the Canvas drawing routine into an HTML bubble. Use text
nodes, `dir="auto"` and suitable bidi isolation; do not render supplied text as
HTML. Wrap long text and keep it within the presence region. A new message
replaces the preceding message; expiry applies only to that message and never
changes work state. Maintain an accessible textual status independent of pixels;
announce message changes politely, not on animation frames. Do not manufacture
the prototype's hard-coded greeting whenever the expression is `speaking`.
The production poke response is visual and optionally tonal; it must not copy
`pokeAylul`'s `sayMessage('Hey! ...')` into the controlled message slot. Only the
supplied message or its own expiry may replace or clear that slot. Re-rendering
the same message must not restart its expiry; the input boundary must distinguish
a new message from a repeated render, including consecutive identical text.

## 6. Rendering, motion and failure behavior

- Measure the container on size changes rather than on every frame. Backing-store
  sizing accounts for device-pixel ratio without changing CSS-space pointer
  coordinates. Re-evaluate sizing when the effective DPR changes.
- Fit the character and its glow to the available region, including narrow
  windows. Preserve aspect ratio and visual registration. Handle zero-size or
  hidden containers without division by zero or allocating an invalid canvas.
- Use elapsed time for physics and particles. Correct frame-count-dependent
  motion while preserving the apparent source behavior at its reference cadence.
  Clamp long time gaps and reset the frame timestamp after suspension; do not
  replay a backlog or jump the character across the stage.
- Each active canvas owns at most one scheduled frame. Unmount, pause, hidden
  document or a non-visible presence region cancels continuous scheduling.
  Clean observers, listeners, animation callbacks and audio resources when their
  owner ends. StrictMode mount/cleanup/remount must not duplicate them.
- Respect `prefers-reduced-motion`, including changes while mounted. In reduced
  motion, draw a still pose on input/size changes; stop continuous blinking,
  bobbing, particles, shaking and movement. State and text remain available.
- Manual pause stops motion, not presentation updates. While paused or in reduced
  motion, redraw once for a changed expression, outfit or size using a stable
  pose, and update message/status immediately without restarting continuous frames.
  Returning to visibility must draw the latest inputs before resuming motion.
- On resume, reconcile message expiry with its deadline. Timers belonging to a
  superseded message or simulation may not mutate the new one. No deferred work
  result is inferred from elapsed time in production.
- Acquire audio lazily after opt-in. Suspended/unsupported audio stays silent;
  report failure near the sound control without breaking the character or shell.
  These are interaction tones, not voice or microphone features.
- If Canvas 2D is unavailable or renderer initialization fails, show an accessible
  companion-unavailable message and preserve account and tray actions. Bound the
  failure to this feature rather than continuously throwing from the frame loop.

## 7. Development studio and shell integration

Expose an explicitly labeled development studio through a development-only entry
inside the existing app. Guard its import with Vite's development condition so
the production bundle has no studio controls or simulation code. It uses the same
renderer as the normal companion; opening or closing it cannot alter account state.

Retain manual expression/outfit/direction controls, pointer tracking, stage-local
cursor following, a draggable mock-window perch, an assistant simulation scene,
tone opt-in, physics/lighting sliders, hands and pedestal toggles, six-direction
previews and icon-size previews. Label synthetic commands and alerts as previews.
No command from those scenes executes project work, records approval, captures
audio or delivers an operating-system notification. Remove the prototype's
"convert to Tauri" modal and global API installation snippets: this app is already
Tauri and uses typed composition.

Manual expression selection, scene changes and studio closure invalidate pending
simulation transitions and notification-demo timers. A new simulation replaces
the previous one. Include `triggerNotificationDemo`'s standalone timeout in that
ownership; checking only the current expression is insufficient because a later
selection can have the same value. A cancelled run cannot update text, expression,
outfit or tones after its scene or selection has been replaced.

Size previews must actually render at the stated pixel sizes. Share one main
preview scheduling owner; render direction and icon thumbnails on relevant
changes instead of running eleven extra permanent animation loops. Frame advances
in a deliberately animated preview must stop when that preview is inactive.

PNG export is a studio-only action using the same renderer and a frozen current
frame/settings snapshot. Export a transparent 512 x 512 image containing the
character, without the surrounding studio, mock window, DOM bubble or status UI.
Report download-generation errors honestly. No native filesystem permission or
Tauri capability is added for export; the eventual browser/WebView download path
must be checked rather than assumed to work.

Desktop UI owns the narrow change to `src/app/main.tsx`: replace the unavailable
AYLUL content with the companion and add the development-only studio entry.
Character owns the feature's implementation and public presentation interface.
Before code crosses this boundary, record and accept an agreement in
`docs/aylul/shared/requests/` under the existing request process. The agreement must
cover imports, the idle default, message/state ownership, development-only entry
and independent failure handling. This spec does not claim that agreement exists.

No authentication behavior, native permissions, account types, tray behavior or
cloud connection is changed by this feature. No separate overlay window, global
mouse hook, always-on-top behavior or actual window perching is included.

## 8. Verification and documentation acceptance

The implementation plan must name actual test paths and commands before building.
Use the existing desktop toolchain. Add only the test support justified by these
checks; no new rendering engine or general state-management package is required.

Automated acceptance must cover:

1. Bounded motion and finite coordinates for normal, zero and long time steps;
   pointer bounds after resize; reproducible blink/particle behavior with a fixed
   random source and equivalent elapsed-time behavior across frame cadences.
2. Single-owner scheduling, cleanup, pause/visibility/reduced-motion transitions,
   and remount behavior. Use a controlled clock/scheduler to observe cancelled
   callbacks and stale simulation/message updates, not arbitrary sleeps.
   Cover manual selection of the same expression during a pending demo, scene
   changes, and controlled input updates while paused or in reduced motion.
3. Independence of main and studio state; simulation never drives the main
   production presence; the production build excludes studio simulation code.
4. Canvas-failure fallback and non-fatal audio/export failures. Verify that account
   controls remain usable rather than only testing an error helper in isolation.
5. Real text rendering for mixed Arabic/English, long messages, keyboard controls
   and accessible status. A snapshot of TypeScript objects cannot prove layout.
   Verify that poke leaves supplied messages intact, ordinary re-renders do not
   extend expiry, and a new message with identical text has its own lifecycle.

Run `npm run build --prefix apps/desktop` and the focused tests selected in the
plan. Source inspection and a passing build do not constitute visual acceptance.
Compare the supplied character at a fixed pose/time with the extracted renderer,
then visually review all expressions, outfits, direction and icon previews.
Review light and dark surroundings, a narrow app window, and multiple DPR values.
Check bubbles, crystal/glow clipping, hands, transparent export and reduced motion.

Windows WebView2 acceptance must separately inspect hide-to-tray/reopen, actual
visibility behavior, resizing and DPI changes, download behavior and idle rendering
cost. Browser document visibility alone does not prove native tray-hide behavior.
Record measurements with the scenario and environment; no CPU or real-device
performance guarantee is established by this specification.

With actual source, add the feature's colocated source contract and code-map row,
update the shell contract for its inspected composition, and record exact checks
and missing evidence. Do not create placeholder contracts before implementation.
Keep existing PNG assets and animation packs unchanged. Their approval remains
historical evidence; this renderer's acceptance is recorded separately with input
provenance. Update asset-facing documentation if its current-appearance description
would otherwise misleadingly imply this renderer uses those PNGs.

User amendment during implementation, 2026-10-16: move those unchanged legacy
packs and the reference board to ignored local `archive/aylul-legacy-2026-10-16/`,
removing their tracked copies from the current tree. This supersedes keeping them
at their original paths; preserve complete pack contents and verify their hashes.
Historical evidence retains its original, dated path references.

## 9. Deferred decisions and closing triggers

| Decision | Owner and closing trigger |
| --- | --- |
| Concrete operational event mapping | Owning service plus Character; close when a real capture/playback/work event consumer is implemented against its owner's types |
| Shell/companion public interface agreement | Desktop UI and Character in `docs/aylul/shared/requests/`; close with an accepted written agreement before dependent shell code |
| Native hide/restore render suspension | Native and Character; close with a Windows WebView2 hide/reopen observation; if existing signals are insufficient, agree on the required native interface before extending scope |
| WebView PNG download compatibility | Character's studio; close with an observed export and transparent-image inspection; keep native save integration outside this slice if a separate native change is needed |
| Original root HTML retention after extraction | Character and user; close after the extracted renderer is visually accepted and its provenance is recorded; preserve `ayl.html` until then |
| Floating desktop companion | Native plus Character; requires a separate design and user instruction; not a prerequisite for this in-window feature |

## 10. Current review boundary

The user approved preparation of the implementation plan after the requested
`gpt-6-astra`/`medium` static review. Its corrections protect controlled messages,
invalidate superseded demo timers, and keep paused presentation current. The
[implementation plan](../plans/2026-10-16-aylul-in-window-implementation.md) carries
the next review boundary. Product source and source contracts remain unchanged.
Implementation starts after plan review and selection of execution method. No
worktree, push or deployment is part of this workflow.
