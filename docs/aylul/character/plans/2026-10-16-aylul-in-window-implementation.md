# AYLUL In-Window Companion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. The user must review this plan and choose the execution method first. Worktrees are explicitly forbidden.

**Goal:** Extract `ayl.html` into a typed Canvas companion in the existing NYLRA window, with a separate development studio using the same renderer.

**Architecture:** Keep drawing, instance-local animation and React lifecycle separate. Production consumes a controlled presentation and initially shows idle; the development studio alone owns synthetic commands, scenes and previews. Account, native and cloud behavior retain their current owners.

**Tech Stack:** Existing React 19.3.0, TypeScript 7.0.2, Vite 8.3.4 and Tauri 2. Node's built-in test runner covers pure TypeScript; Playwright covers real Canvas/DOM behavior. Only two development dependencies are planned: `@playwright/test@1.64.1` and `yaml@2.9.1` (registry versions checked 2026-10-16).

**Spec:** [Approved design](../specs/2026-10-16-aylul-in-window-design.md), including the three Astra review corrections.

## Global Constraints

- Work in `E:\Globalprojects\nylra`, branch `codex/aylul-companion-in-window`; never create a worktree, switch branches, stash, push or deploy.
- Preserve unrelated generated-schema changes and the existing Context7 configuration. Stage explicit task files only; one implementation commit per completed task.
- Preserve root `ayl.html` byte-for-byte: SHA-256 `103a5ba2765c4a0f81f9b93051d2883fef1dc79a7e49dc8cb4ee0d0ff76a8bae`. It remains the local extraction input; do not stage it incidentally or ship it as an app asset.
- Preserve the inspected geometry, palette, layer ordering and defaults. Canvas 2D simulates depth; introduce no 3D engine, runtime CDN, remote font or state-management package.
- Preserve all nine expressions, ten outfits and six direction presets from the spec. Initial production presentation is `idle`, default outfit and no message; tones start off.
- Interactions never manufacture operation outcomes, consent, microphone activity, speech playback or native notification delivery. No backend DTO copies, sidecar calls or `window.AYLUL` API.
- Same renderer for production, studio, thumbnails and transparent `512 x 512` PNG export. Existing PNG packs remain unchanged.
- One active scheduler per mounted main preview; no perpetual thumbnail loops. Clean callbacks, listeners, observers, timers and audio on disposal; support StrictMode and reduced motion.
- Shell/feature changes require an accepted Character/Desktop UI agreement before crossing that boundary. Native capabilities and CSP are unchanged.
- No product build or runtime evidence exists for this plan. Expected results below are future acceptance, never claims that tests already ran.

## Review Focus

1. A new message repeats the previous text: it gets a new lifetime; ordinary renders do not extend it, and poking preserves it. Task 3.
2. A demo is cancelled and the same expression is selected: stale callbacks cannot overwrite the new selection, message, outfit or tones. Task 4.
3. The canvas starts at zero width, then resizes or changes DPR: finite coordinates, correct hit testing and no clipped character. Tasks 2 and 3.
4. A controlled state changes while paused/reduced motion or hidden: current text and a fresh still render appear without restarting continuous motion. Tasks 2 and 3.
5. Canvas/audio/export fails: the shell's account and named hide action remain operable; export failure is visible rather than a false success. Tasks 3, 4 and 5.

## File structure and responsibilities

Paths below are planned, not an existing code map. `C` abbreviates
`apps/desktop/src/features/companion`; `T` abbreviates `apps/desktop/tests/companion`.
Every task expands the exact paths it owns. Do not add empty future modules.

| Files | Responsibility | Task |
| --- | --- | --- |
| `C/types.ts`, `C/catalog.ts` | Presentation/frame types, catalogs, source defaults and ranges | 1 |
| `C/renderer/{character,eyes,crystals,hands,outfits,effects,scene}.ts` | Character composition and focused drawing helpers; scene draws ambient effects/pedestal | 1 |
| `C/animation/{model,scheduler,geometry,messages}.ts` | Motion, one frame owner, viewport math and message expiry | 2 |
| `C/{Companion,CompanionCanvas,SpeechBubble}.tsx`, `C/audio.ts`, `C/companion.css` | React integration, accessible presentation and optional tones | 3 |
| `C/studio/{CompanionStudio,SceneControls,PreviewGrid,MockWindow}.tsx`, `C/studio/{simulation,export}.ts`, `C/studio/studio.css` | Development controls, mock scenes, cancellable demos, static previews and export | 4 |
| `C/contract.yaml`, `C/studio/contract.yaml` | Inspected behavior/evidence; the parent excludes the separately owned studio when it exists | 1–4 |
| `T/fixture.html`, `T/fixture.tsx`, `T/fakes.ts`, `T/renderer.spec.ts`, `apps/desktop/playwright.config.ts` | Test-only real-browser fixture and deterministic clock/random/scheduler helpers | 1 |
| `T/{model,scheduler,geometry,messages,contracts}.test.ts` | Pure Node tests and narrow YAML/source-reference checks | 1–2 |
| `T/{presence,studio,shell,visual}.spec.ts`, `T/production.test.ts` | DOM/browser behavior, extraction review artifacts and build exclusion | 3–6 |
| `apps/desktop/src/app/main.tsx`, `apps/desktop/src/app/contract.yaml` | Narrow shell composition and its inspected contract | 5 |
| `apps/desktop/tests/windows-shell.ps1` | Preserve named keyboard hide acceptance after companion buttons are added | 6 |

## Interface decisions

Put the following presentation-only types in `C/types.ts`. Do not map them to
invented runtime events. Lowercase catalog identifiers are exactly those in spec
section 5; `SceneMode` is `studio | cursor | window | assistant` for preview use.

```ts
type Point = Readonly<{ x: number; y: number }>;
type Rect = Readonly<{ x: number; y: number; width: number; height: number }>;
type Viewport = Readonly<{ width: number; height: number; dpr: number }>;
type CompanionMessage = Readonly<{ id: string; text: string; durationMs?: number }>;
type CompanionPresentation = Readonly<{
  expression: Expression; statusText: string; message: CompanionMessage | null;
}>;
type CompanionSettings = Readonly<{
  depth3D: number; glowIntensity: number; floatSpeed: number;
  earringSwing: number; showHands: boolean; showPedestal: boolean;
  followPointer: boolean;
}>;
type Particle = Readonly<{
  x: number; y: number; z: number; radius: number;
  alpha: number; speedY: number; phase: number;
}>;
type SleepingMark = Readonly<{ offset: number; size: number }>;
type CharacterFrame = Readonly<{
  cx: number; cy: number; scale: number; lookX: number; lookY: number;
  tilt: number; time: number; earringAngle: number; blinkProgress: number;
  expression: Expression; outfit: OutfitId; mini: boolean; perchingHands: boolean;
  sleepingMarks: readonly SleepingMark[]; notificationPulse: number;
}>;
type SceneFrame = Readonly<{
  character: CharacterFrame; particles: readonly Particle[];
}>;
type MotionInput = Readonly<{
  viewport: Viewport; presentation: CompanionPresentation; settings: CompanionSettings;
  outfit: OutfitId; scene: SceneMode; pointer: Point | null;
  direction: DirectionId | null; perch: Rect | null;
}>;
```

All names above are exported; `Expression`, `OutfitId`, `DirectionId` and
`SceneMode` are string unions, not TypeScript enums. A message ID is an immutable
message occurrence: new occurrence means new ID, even for identical text. Omitting
`durationMs` means `4000`; `null` explicitly clears the message. Expiry is based
on the first receipt of that ID, not each React render. Settings bounds/defaults:

| Field | Default | Range |
| --- | --- | --- |
| `depth3D` | 1.35 | 0.5–2.0 |
| `glowIntensity` | 1.25 | 0.5–1.9 |
| `floatSpeed` | 1.0 | 0.3–2.5 |
| `earringSwing` | 1.25 | 0.3–2.2 |
| `showHands`, `showPedestal`, `followPointer` | true | boolean |

Use the HTML's `TURNAROUND_DIRS` numeric values unchanged. Numeric settings are
finite and clamped; invalid non-finite values use the corresponding default.
Clamp non-finite/negative viewport dimensions to zero and invalid DPR to one.

## Task 1: Extract the renderer with a real Canvas review fixture

**Files:** Create `C/types.ts`, `C/catalog.ts`, `C/renderer/character.ts`,
`C/renderer/eyes.ts`, `C/renderer/crystals.ts`, `C/renderer/hands.ts`,
`C/renderer/outfits.ts`, `C/renderer/effects.ts`, `C/renderer/scene.ts`,
`C/contract.yaml`, `T/fixture.html`, `T/fixture.tsx`, `T/fakes.ts`,
`T/renderer.spec.ts`, `T/contracts.test.ts`, `apps/desktop/playwright.config.ts`.
Modify `apps/desktop/package.json`, `apps/desktop/package-lock.json`,
`apps/desktop/tsconfig.json`, `.gitignore`, `docs/aylul/codebase/README.md`.

**Interfaces:** Consumes root HTML rendering functions, not its startup script.
Produces `renderCharacter(ctx: CanvasRenderingContext2D, frame: CharacterFrame,
settings: CompanionSettings): void` in `character.ts`, and
`renderScene(ctx: CanvasRenderingContext2D, frame: SceneFrame,
settings: CompanionSettings): void` in `scene.ts`. `catalog.ts` exports
`EXPRESSIONS`, `OUTFITS`, `DIRECTIONS`, `DEFAULT_SETTINGS` and
`normalizeSettings(partial: Partial<CompanionSettings>): CompanionSettings`.
Helper signatures remain internal to the renderer; no later task imports them.

- [ ] **Step 1:** Add the two pinned development dependencies and the browser
  fixture needed by this deliverable. Use `npm install --save-dev --save-exact
  @playwright/test@1.64.1 yaml@2.9.1` from `apps/desktop`, then
  `npx playwright install chromium`. Node 24.15.0 is present at planning time;
  require Node 24.12+ for native TypeScript tests. Add
  `allowImportingTsExtensions: true`; pure source imports use explicit `.ts`
  extensions and type-only imports. No Node test imports `.tsx` or CSS.
  Ignore `apps/desktop/test-results/` and `apps/desktop/playwright-report/`;
  screenshots/logs stay there unless deliberately selected as dated evidence.
- [ ] **Step 2:** Write `renderer.spec.ts` tests named `all expressions and
  outfits draw without global engine`, `same inputs produce same pixels`,
  `character render preserves transparent corners`, and `defaults match source`.
  Use a real canvas in the fixture, fixed time/pose and empty ambient particles;
  assert nontransparent character pixels, transparent corner alpha, identical
  `getImageData` for repeated identical inputs, and exact defaults from the table.
  Add `contracts.test.ts`: YAML parses, source paths exist, declared public names
  resolve in source exports using the existing TypeScript parser, and named test
  references point to real files/tests. Keep this a small companion-specific check.
  For a prepared 256-square fixture canvas/frame, the determinism test includes:

  ```ts
  renderCharacter(ctx, frame, settings);
  const first = ctx.getImageData(0, 0, 256, 256).data.slice();
  ctx.clearRect(0, 0, 256, 256);
  renderCharacter(ctx, frame, settings);
  expect([...ctx.getImageData(0, 0, 256, 256).data]).toEqual([...first]);
  expect(first[3]).toBe(0);
  ```
- [ ] **Step 3:** Configure Playwright with `testDir: './tests/companion'`,
  `testMatch: '**/*.spec.ts'`, Chromium and a web server command
  `npm run dev -- --port 4173 --strictPort`, URL `http://127.0.0.1:4173`,
  `reuseExistingServer: false`. Fixture URL is `/tests/companion/fixture.html`.
  Add script `test:companion:browser` = `playwright test`. Run
  `npm run test:companion:browser -- renderer.spec.ts`; expect RED for missing
  renderer exports, not a broken browser install. Fix infrastructure failures
  before recording the feature's RED result.
- [ ] **Step 4:** Extract the existing draw paths into the named renderer files.
  Keep geometry/colors/layer ordering, including `drawFourPointStar`; pass sleep
  marks, pulse and pedestal flag explicitly instead of reading `engine`. Remove
  Canvas speech drawing. `renderScene` owns ambient effects and pedestal around
  `renderCharacter`; PNG callers use `renderCharacter` alone. Fill the contract
  from inspected code and add the first actual map row, without claiming animation
  or React functionality yet.
- [ ] **Step 5:** Run the renderer tests, `node --test
  tests/companion/contracts.test.ts`, and `npm run build`, all from
  `apps/desktop`. Expect PASS/exit 0. Save fixed-pose canvas captures as review
  artifacts; passing pixel invariants do not establish visual similarity to the
  user's screenshot. Verify the source HTML hash is unchanged.
- [ ] **Step 6:** Stage only this task's explicit files and commit
  `feat(companion): extract typed Canvas character renderer`. Do not add root HTML,
  schemas, browser binaries or generated test artifacts.

## Task 2: Add deterministic motion, geometry, scheduling and message expiry

**Files:** Create `C/animation/model.ts`, `C/animation/geometry.ts`,
`C/animation/scheduler.ts`, `C/animation/messages.ts`, `T/model.test.ts`,
`T/geometry.test.ts`, `T/scheduler.test.ts`, `T/messages.test.ts`.
Modify `C/contract.yaml`, `T/fakes.ts`.

**Interfaces:** Consumes Task 1 types/catalogs. `model.ts` exports
`createMotion(viewport: Viewport, random?: () => number): MotionModel`,
`advanceMotion(model: MotionModel, seconds: number, input: MotionInput): void`,
`pokeMotion(model: MotionModel): void`, and
`readFrame(model: MotionModel, input: MotionInput): SceneFrame`. `MotionModel`
is an exported type owned by `model.ts`; callers do not modify its fields.
`geometry.ts` exports `normalizeViewport(viewport: Viewport): Viewport`,
`backingSize(viewport: Viewport): {width: number; height: number}`,
`fitCharacter(viewport: Viewport): {center: Point; scale: number}` and
`localPointer(client: Point, bounds: Rect): Point | null`.

`scheduler.ts` exports `createFrameLoop(clock: FrameClock,
draw: (seconds: number) => void): FrameLoop`, where `FrameClock` has
`request(callback: (nowMs: number) => void): number` and
`cancel(id: number): void`; `FrameLoop` has `start(): void`, `stop(): void`,
`invalidate(): void`, `dispose(): void`. Start is idempotent; stop cancels pending
continuous work; invalidate requests one draw with delta zero while stopped.
Dispose makes all methods inert and cancels every owned callback.

`messages.ts` exports `createMessageSlot(): MessageSlot`,
`receiveMessage(slot: MessageSlot, message: CompanionMessage | null,
nowMs: number): void` and `visibleMessage(slot: MessageSlot, nowMs: number):
CompanionMessage | null`. `MessageSlot` is owned there. Invalid message durations
fall back to 4000 ms; zero expires immediately, text stays plain data.

- [ ] **Step 1:** Write focused Node tests using the controlled clock/random source
  in `T/fakes.ts`. Pin: viewport zero -> no invalid scale; pointer outside -> null;
  DPR 2 doubles backing dimensions without doubling local pointer coordinates;
  settings/resize keep the whole character fit. Assert input data is not mutated.
- [ ] **Step 2:** Write cadence tests comparing one second at 30/60/120 Hz with the
  same seed and target; frame coordinates differ by at most 0.01 CSS px and remain
  finite. Long-gap input advances at most 0.05 s. Write scheduler tests asserting
  double start leaves one pending callback, stop/dispose leave zero, and paused
  invalidation draws once with delta zero. Test message A at t=0 with default expiry
  at 4000, rerender at 3000 still expires at 4000, new ID/same text at 3500 expires
  at 7500, and replacement/clear cannot be undone by old expiry.
  The `same ID does not extend expiry` test includes:

  ```ts
  const slot = createMessageSlot();
  receiveMessage(slot, {id: 'a', text: 'hello'}, 0);
  receiveMessage(slot, {id: 'a', text: 'hello'}, 3000);
  assert.equal(visibleMessage(slot, 4000), null);
  receiveMessage(slot, {id: 'b', text: 'hello'}, 4000);
  assert.equal(visibleMessage(slot, 7999)?.id, 'b');
  ```
- [ ] **Step 3:** Run `node --test tests/companion/model.test.ts
  tests/companion/geometry.test.ts tests/companion/scheduler.test.ts
  tests/companion/messages.test.ts`; expect RED for missing exports/assertions.
- [ ] **Step 4:** Implement the declared operations. Use a fixed 1/120 s simulation
  accumulator for stable spring/pendulum evolution; clamp each external elapsed
  interval to 0.05 s. Convert prototype per-frame particle speed to seconds using
  its 60 Hz reference. Each model owns 28 ambient particles, three sleep marks,
  blink/poke/pulse state and sampled randomness. Freeze time/pose for zero-delta
  redraws; no DOM/global timers in the model. Reset scheduler timestamp on restart;
  ignore obsolete callbacks after stop/dispose. Fit based on the complete source
  silhouette/glow bounds, not only the face's center.
- [ ] **Step 5:** Run the four Node files, contract check and `npm run build`;
  expect PASS/exit 0. Update the owning contract with actual test references and
  known browser-lifecycle gaps.
- [ ] **Step 6:** Stage this task's listed files and commit
  `feat(companion): add isolated motion and lifecycle models`.

## Task 3: Build an accessible, lifecycle-safe React presence

**Files:** Create `C/Companion.tsx`, `C/CompanionCanvas.tsx`, `C/SpeechBubble.tsx`,
`C/audio.ts`, `C/companion.css`, `T/presence.spec.ts`. Modify `T/fixture.tsx`,
`C/contract.yaml`.

**Interfaces:** Consumes Tasks 1–2. Export
`Companion(props: {presentation?: CompanionPresentation}): React.JSX.Element`
with idle/default/no-message fallback.
`CompanionCanvas(props: {input: MotionInput; paused: boolean;
onFrame?: (frame: SceneFrame) => void; onFailure: (reason: string) => void;
onPoke?: () => void}): React.JSX.Element` owns motion and canvas lifecycle.
`onFrame` supplies studio snapshots without per-frame React state updates.
`SpeechBubble(props: {message: CompanionMessage | null}): React.JSX.Element | null`
receives only the currently visible message.
`audio.ts` exports `createTones(): ToneController`, with
`enable(): Promise<void>`, `play(kind: 'poke' | 'state' | 'success'): void`,
`disable(): void`, `dispose(): void`; enabling is called only in a gesture handler.

- [ ] **Step 1:** Extend the fixture with React StrictMode, controlled presentation
  buttons, mount/unmount, pause and zero-size/resize cases. Keep fixture controls
  outside production source. Add browser tests `strict remount owns one loop`,
  `paused changes repaint without animation`, `reduced motion updates stay still`,
  `zero size then DPR resize`, `same text new message expires independently`,
  `poke preserves controlled message`, `plain mixed-direction text wraps`, and
  `canvas and audio failures stay local`.
- [ ] **Step 2:** In those tests, use a controlled browser clock plus RAF/listener
  counters installed before mounting. Assert zero continuous draws while paused,
  hidden or reduced-motion, then one updated still after controlled input changes.
  Assert no callbacks remain after unmount. Test `<img onerror=...>` text remains
  literal, mixed `Hey! جاهز لمساعدتك` stays inside the bubble, long unbroken text
  does not overflow at 320 px viewport width, and Enter/Space invokes the named
  poke control without changing the operational expression/message. Run
  `npm run test:companion:browser -- presence.spec.ts`; expect RED.
  Use `Poke AYLUL` as the accessible interaction name; in the fixture's
  owner-message case, pin the precedence rule with:

  ```ts
  const bubble = page.getByRole('status', {name: 'AYLUL message'});
  await expect(bubble).toHaveText('Owner message');
  await page.getByRole('button', {name: 'Poke AYLUL'}).press('Space');
  await expect(bubble).toHaveText('Owner message');
  ```
- [ ] **Step 3:** Implement the declared components with scoped CSS. Use
  ResizeObserver, visibility and intersection signals, DPR media-query rebinding,
  and dynamic reduced-motion observation. No per-frame element measurement.
  Defer draw invalidations while hidden; reveal the latest controlled inputs on
  restoration without replaying old frames or drawing in the hidden interval.
  Delegate to Task 2 scheduler and message slot; a single cancellable deadline timer
  invalidates the visible message without extending its lifetime. Hidden instances
  reconcile against current time before revealing. Bubble uses text nodes,
  `dir="auto"`, bidi isolation and wrapping; a separate polite status changes only
  on input/message changes. No synthetic greeting on `speaking`.
- [ ] **Step 4:** Implement lazy optional tones and local failure boundaries. A null
  Canvas context or render error disposes its loop and shows an accessible fallback;
  audio failure shows a sound-control message. Poking provides only visual/optional
  tonal feedback. Provide Pause/Resume and Enable/Mute sound controls. Render a
  frozen pose for input updates while paused/reduced-motion; do not restart motion.
- [ ] **Step 5:** Run `presence.spec.ts`, prior focused unit tests and
  `npm run build`; expect PASS. Contract changes must distinguish simulated browser
  visibility tests from unperformed native tray acceptance.
- [ ] **Step 6:** Stage only this task's files and commit
  `feat(companion): add accessible React companion presence`.

## Task 4: Rebuild the development studio on the shared renderer

**Files:** Create `C/studio/CompanionStudio.tsx`, `C/studio/SceneControls.tsx`,
`C/studio/PreviewGrid.tsx`, `C/studio/MockWindow.tsx`, `C/studio/simulation.ts`,
`C/studio/export.ts`, `C/studio/studio.css`, `C/studio/contract.yaml`,
`T/studio.spec.ts`. Modify `T/fixture.tsx`, `C/contract.yaml`,
`docs/aylul/codebase/README.md`.

**Interfaces:** Consumes renderer, catalogs and `CompanionCanvas` snapshot callback.
`CompanionStudio(): React.JSX.Element` is the development entry.
`simulation.ts` exports `createSimulation(clock: SimulationClock,
emit: (update: DemoUpdate) => void): Simulation`; `SimulationClock` has
`set(callback: () => void, delayMs: number): number` and `clear(id: number): void`.
`Simulation` has `run(kind: 'success' | 'blocked' | 'sleep'): void`,
`notify(): void`, `cancel(): void`, `dispose(): void`. `DemoUpdate` carries an
optional expression, message, outfit and tone and is owned/exported by this file.
Its exact shape is `{expression?: Expression; message?: CompanionMessage | null;
outfit?: OutfitId; tone?: 'poke' | 'state' | 'success'}`. `SceneControls`,
`PreviewGrid` and `MockWindow` are internal studio components; later tasks import
only `CompanionStudio`.
`export.ts` exports `exportCharacterPNG(frame: CharacterFrame,
settings: CompanionSettings): Promise<Blob>` and
`downloadCharacterPNG(blob: Blob, filename: string): void`.

- [ ] **Step 1:** Write browser tests `all catalogs and scene controls work`,
  `manual same-expression selection cancels notification`, `scene close cancels
  every demo callback`, `new run supersedes old run`, `studio cannot change main
  presence`, `thumbnails draw on changes only`, `mock perch stays inside stage`,
  `PNG is 512 square with transparent corners`, and `export failure shows error`.
  Freeze the clock and test late callbacks after replacement, including the
  prototype's notification transition at 1800 ms and identical manual `listening`
  selection before it. Assert the selected expression/text/outfit and tones remain
  unchanged after cancellation. Run `studio.spec.ts`; expect RED.
  With `page.clock` installed before the fixture and the catalog buttons scoped
  to the studio, the regression assertion is:

  ```ts
  await studio.getByRole('button', {name: 'Preview notification'}).click();
  await studio.getByRole('button', {name: 'Listening', exact: true}).click();
  await page.clock.runFor(1801);
  await expect(studio.getByRole('status', {name: 'Preview expression'}))
    .toHaveText('listening');
  ```
- [ ] **Step 2:** Build catalog-driven controls and preview scenes. Scene selection,
  manual expression selection, replacement run and closure invalidate a generation
  token and clear all owned timers; never rely on current-expression equality.
  Reproduce the prototype's visible demo sequences in this module only; label
  synthetic assistant input and alerts explicitly. Keep all effects local to this
  studio instance; no native notification, real capture or task call is added.
- [ ] **Step 3:** Implement six static direction thumbnails and true 256/128/64/32/16
  pixel icon previews, redrawn on relevant control changes using a frozen snapshot.
  Keep one animated main preview, stage-bounded pointer following, draggable mock
  window/perching hands, physics sliders and hands/pedestal switches. Mock dragging
  uses pointer capture and releases on pointerup/cancel. Remove Tauri conversion
  modal/global API snippets. Optional audio is independently owned by this instance.
- [ ] **Step 4:** Export through `renderCharacter` with a cloned current frame,
  512-square canvas, center `(256,275)` and source scale `2.65`; do not render scene
  particles, pedestal, bubble, mock window or UI. Test extreme directions/outfits
  for clipping and fit within the export canvas if bounds require it. Reject null
  context/blob or encoding errors; revoke object URLs after download handoff.
  A generated Blob is not proof that WebView saved the image.
- [ ] **Step 5:** Run `studio.spec.ts`, `renderer.spec.ts`, `presence.spec.ts`,
  contract checks and `npm run build`; expect PASS. Update the parent contract to
  exclude studio rules and add its independent contract/map entry from actual code.
- [ ] **Step 6:** Stage only this task's files and commit
  `feat(companion): add isolated development studio and PNG export`.

## Task 5: Agree the boundary and embed the feature in the existing shell

**Files:** Create
`docs/aylul/shared/requests/2026-10-16-companion-shell-embedding.md`,
`T/shell.spec.ts`, `T/production.test.ts`.
Modify `apps/desktop/src/app/main.tsx`, `apps/desktop/src/app/contract.yaml`,
`T/fixture.tsx`, `apps/desktop/package.json`.

**Interfaces:** Consumes `Companion` and `CompanionStudio` exactly as above.
The shell renders `<Companion />` in its existing AYLUL section. Development-only
import is `import.meta.env.DEV ? import('../features/companion/studio/CompanionStudio')
: null`, wrapped in lazy/Suspense loading plus a local error boundary. Opening
the studio is explicit, and production has no entry/imported studio module.

- [ ] **Step 1:** Write the Character request from the existing shared template
  with these imports, idle default, state/message ownership and failure isolation.
  Obtain the Desktop UI answer and accepted status through the repository process
  before changing shell code. Do not fabricate an acceptance or message another
  chat without user authorization. If no lead session is available, bring this
  concrete agreement to the user acting as teacher; isolated feature work from
  Tasks 1–4 can already be reviewed independently.
- [ ] **Step 2:** In the test-only fixture's shell mode, install Tauri `mockIPC`
  before dynamically importing the real `src/app/main.tsx`; no shell copies or
  production mocking flags. Return synthetic `AuthView` values using the existing
  platform types and inspect calls in the fixture only. Test `companion failure
  preserves account and hide actions`, `studio changes never change auth`, and
  `production starts idle without synthetic operations`. Assert Check session and
  Hide to tray still invoke their actual platform wrappers; fake IPC is not native
  or backend acceptance. Expect RED before replacing unavailable presence.
  The fixture handles `auth_action` with `{action: 'status'}` and `hide_main`,
  matching the inspected wrappers; other commands fail closed. Use a synthetic
  signed-out view, never real credentials. Assert the observed invocation after
  the named action, rather than testing a copy of the wrapper:

  ```ts
  await page.getByRole('button', {name: 'Hide to tray', exact: true}).click();
  await expect(page.getByTestId('fixture-ipc-calls')).toContainText('hide_main');
  await expect(page.getByRole('heading', {name: 'Account', exact: true})).toBeVisible();
  ```
- [ ] **Step 3:** Add `production.test.ts` invoking Vite's build API with
  `write:false`, no broad deletion of `dist`, and a small `generateBundle` inspection
  plugin. Assert the emitted production chunks include `/companion/Companion.tsx`
  and exclude all module IDs from `/companion/studio/`, after normalizing path
  separators. Assert no test fixture is an emitted entry and emitted HTML/CSS
  references no external CDN/font. The presence browser test separately records
  requests and rejects non-local resources. Do not use minified function-name
  greps as evidence of dead-code exclusion. Run this test RED before shell embedding
  because the production companion module is not included yet; an absence-only
  studio check would already pass and is not evidence of RED.
- [ ] **Step 4:** Make the narrow shell changes without refactoring auth or moving
  its state. Use feature-scoped styles so account sections/buttons retain existing
  semantics. Development studio load/render failure stays local; main companion
  remains usable when the studio is closed. Update the shell contract using the
  browser tests' exact limits.
- [ ] **Step 5:** Run `shell.spec.ts`, all companion browser tests,
  `node --test tests/companion/production.test.ts`, and `npm run build`. Expected:
  PASS, no production studio modules and unchanged native config/capabilities.
  Add `test:companion:unit` listing the four pure Node test files plus the contract
  check; add `test:companion:production` for the production test. Do not add a
  blanket repository-wide test command or claim remote CI.
- [ ] **Step 6:** Stage only this task's files and commit
  `feat(desktop): embed AYLUL with a development-only studio`.

## Task 6: Close visual and Windows acceptance with truthful evidence

**Files:** Create `T/visual.spec.ts`,
`docs/evidence/nylra/2026-10-16-aylul-in-window-acceptance.md`.
Modify `apps/desktop/tests/windows-shell.ps1`, `apps/desktop/README.md`,
`assets/aylul/README.md`, `docs/aylul/codebase/README.md`, `C/contract.yaml`,
`C/studio/contract.yaml`, `apps/desktop/src/app/contract.yaml`,
`docs/aylul/character/stages.md`. Test artifacts remain outside commits unless selected
as bounded evidence; do not add large generated galleries or overwrite old runs.

**Interfaces:** Consumes all earlier public interfaces. Existing
`windows-shell.ps1 -RunName <unique-name>` remains the native shell acceptance
entry; its assertions must target the actual named Hide to tray button rather
than assume the first Tab stop is the only enabled action.

- [ ] **Step 1:** Add real-browser visual cases for all expressions, outfits,
  direction presets, 320 px width and DPR 1/2, light/dark surroundings, long mixed
  Arabic/English bubbles, reduced motion and transparent export. Freeze time/random
  for captures. Assert component bounds contain bubble and canvas, nonblank character
  pixels and no unexpected page errors; collect review captures. An auto-approved
  screenshot baseline does not prove source fidelity: review them against the
  selected HTML appearance/screenshots before accepting new baselines.
  For each bubble capture, assert its bounding box is inside the presence region,
  not merely inside the screenshot. With boxes measured in CSS pixels:

  ```ts
  expect(bubble.x).toBeGreaterThanOrEqual(presence.x);
  expect(bubble.x + bubble.width).toBeLessThanOrEqual(presence.x + presence.width);
  expect(bubble.y + bubble.height).toBeLessThanOrEqual(presence.y + presence.height);
  ```
- [ ] **Step 2:** Update the desktop UI shared agreement to cover its native test
  file if necessary. Exercise keyboard-hide with initial focus on an enabled
  companion control and record whether the old first-Tab assumption fails. Do
  not label an observed PASS as RED. Then explicitly focus the real UI Automation button named
  `Hide to tray` and use Space, verifying its native window becomes hidden. Do not
  invoke the Rust hide callback directly or weaken an assertion to get PASS.
- [ ] **Step 3:** Run `npm run test:companion:unit`,
  `npm run test:companion:browser`, `npm run test:companion:production`,
  `npm run build` from `apps/desktop`. Expected PASS/exit 0. Build the current native
  app with `cargo build --release --locked --manifest-path
  apps/desktop/src-tauri/Cargo.toml`, then run
  `powershell -NoProfile -File apps/desktop/tests/windows-shell.ps1 -RunName
  aylul-in-window-<unique-run-id>` from the repository root. Native acceptance
  requires MSVC/WebView2 and an available desktop; if unavailable, record the exact
  blocked check and leave that acceptance open.
- [ ] **Step 4:** On Windows observe hide-to-tray/reopen, zero rendering work while
  hidden, correct latest still/state on restore, real resize/DPI transitions,
  tones opt-in/mute, and saved PNG transparency. Record how WebView renderer and
  app processes were measured separately while visible/paused/hidden; the existing
  script's main-process-only CPU sample cannot establish renderer cost. If hidden
  WebView rendering does not suspend through existing signals, stop closure of
  this acceptance and request the Native-owned interface; do not add Rust changes
  within this plan. WebView PNG failure is recorded as a studio limitation requiring
  a separate native-save decision, not a claim of successful export.
- [ ] **Step 5:** Write dated evidence separating static inspection, Node checks,
  Chromium presentation tests, native Windows observations and remaining gates.
  Record source hash/settings, actual commands/results, screenshot review and
  download outcome. Update contracts/code map and READMEs to describe the Canvas
  feature without implying it uses the historical PNGs or has real voice/cloud
  integration. Populate/update `docs/aylul/character/stages.md` with actual task statuses;
  do not mark visual/native acceptance done if its evidence is missing.
- [ ] **Step 6:** Stage only this task's listed files and selected evidence; commit
  `test(companion): record in-window visual and Windows acceptance`. A blocked
  native check may be documented in a clearly partial evidence commit, but does
  not complete the task or the feature's acceptance.

## Execution records and final review

After this plan is approved, the implementer populates `docs/aylul/character/stages.md`
using its required `Status`, `Plan`, `Read`, exact `Touch`, `Needs`, `Done when`
fields for these six tasks before product edits. Task updates belong in their
own task commit. Task 5's shared agreement is a real prerequisite, not implied
by approval of this plan. No implementation is active at plan-writing time.

Before each commit, check the staged paths/diff, run the task's named checks and
verify the original HTML hash. Never stage the unrelated generated schemas. Each
task carries its own contract changes; do not postpone all contracts to Task 6.
On completion, request one whole-branch review under the selected execution
method and report its findings with the exact local evidence. Do not claim merge,
CI, device continuity or production acceptance.

## Plan self-review and coverage

- Spec sections 1–4 -> Task 1 ownership/extraction and Task 5 composition.
- Spec section 5 -> Tasks 1–3 catalogs, controlled presentation, text and tones;
  Tasks 4–5 keep synthetic outcomes out of production.
- Spec section 6 -> Tasks 2–3 geometry, clocks, cleanup, reduced motion and local
  failures; Task 6 verifies native visibility rather than inferring it.
- Spec section 7 -> Task 4 studio/export; Task 5 agreement and production exclusion.
- Spec section 8 -> named tests in all tasks; Task 6 visual/native evidence and
  historical asset documentation. No runtime claims are made by writing this plan.
- Spec section 9 -> owner/closing triggers retained; Task 6 must leave unsupported
  native observations open. No hidden expansion into native/backend work.
- The five Review Focus items have explicit owner tests. Public signatures/types
  are defined once above and consumed by name. No task requires an undefined
  source service, a guessed backend event, or an automatically accepted agreement.

## Tooling references checked during planning

- [Node 24 TypeScript support](https://nodejs.org/docs/latest-v24.x/api/typescript.html):
  native type stripping supports erasable `.ts`, not TSX; explicit extensions and
  `allowImportingTsExtensions` support the pure Node tests.
- [Playwright web server](https://playwright.dev/docs/test-webserver),
  [browser mocks](https://playwright.dev/docs/mock-browser-apis),
  [clock](https://playwright.dev/docs/clock): tests start the local dev server and
  control time/browser APIs without arbitrary waits.
- [Tauri API mocking](https://v2.tauri.app/develop/tests/mocking/): test-only IPC
  substitution checks frontend calls; it is separate from actual Rust/WebView work.

The earlier browser policy denial for direct `file://` access remains in effect.
This plan does not authorize a UI workaround to inspect that blocked page. Its
software tests target the new application feature; the original file is a static
extraction input, and visual acceptance uses supplied/approved review artifacts.
