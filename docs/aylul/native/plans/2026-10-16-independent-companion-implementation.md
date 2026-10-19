# Independent AYLUL Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking. The user already selected inline execution; do not dispatch implementer agents or create worktrees.

**Goal:** Run the accepted AYLUL renderer in a small independent Windows companion surface that survives main-panel closure without stealing focus or rendering while hidden.

**Architecture:** Native Rust owns both windows, the tray and revisioned companion state. A separate local React entry uses the existing Canvas renderer through a typed platform bridge; main contains account and companion controls. Keep native effects distinct from state calculation so failures and stale updates can be tested without a device.

**Tech Stack:** Existing React 19.3.0, TypeScript 7.0.2, Vite 8.3.4, Tauri 2.11.5 and windows-sys 0.61.2 pins; existing Node test runner, Playwright 1.64.1 and installed Chrome.

**Spec:** [Approved independent companion specification](../specs/2026-10-16-independent-companion-design.md).

## Global Constraints

- Original checkout and `codex/aylul-companion-in-window`; no worktrees, branch switching, push, merge or deployment.
- Read existing contracts before edits. Stage named files only. Preserve `ayl.html`, ignored archive and preexisting generated-schema changes.
- Native surface is `companion`, local entry `companion.html`, initially hidden, 240 by 240 logical pixels, transparent, undecorated, unresizable, shadowless, above ordinary windows and absent from taskbar.
- Main remains `main` with deliberate open/focus through the current tray, shortcut and single-instance routes. Closing either surface hides only that surface.
- Primary drag threshold is 8 logical pixels; short primary click opens main; dragging must suppress that click. Transparent padding remains a rectangular hit area.
- Native render permission begins false; updates use monotonic revision numbers; no scheduled or invalidation draw while permission is false.
- No new renderer, state library, global mouse hook, polling-based cursor tracking, sidecar or cloud transport. No account material reaches companion.
- No simulated operational states. Defaults remain idle, sound disabled. Studio remains development-only and isolated.
- Source contracts and code map describe actual implemented symbols/tests, not this proposed file map.
- Observe failing regression tests before implementing each behavior; final verification names actual commands and limits.

## Review Focus

1. Late subscription completion after unmount must immediately unlisten and must never report readiness (Task 2 test).
2. A newer hide event arriving before an older ready/snapshot reply must keep drawing disabled (Tasks 1–2 tests).
3. A construction/rendering failure must leave account, tray and an explicit reconstruction path usable (Tasks 1, 3–4 tests).
4. Native drag may consume the mouse-up event; the next click must work without reopening main for the preceding drag (Task 3 tests).
5. A zero-sized work area or recover callback caused by our own repositioning must not produce overflow or recursive relocation (Task 1 tests).

## File and ownership map

Paths below are intended additions/edits, not claims that they exist.

| Files | Job |
| --- | --- |
| `apps/desktop/src-tauri/src/windows/companion/{mod,state,placement,no_activate}.rs` | Window lifecycle, state decisions, physical fitting and isolated Windows show behavior |
| `apps/desktop/src-tauri/src/windows/companion/contract.yaml` | Inspected native companion behavior; parent windows contract excludes child ownership |
| `apps/desktop/src-tauri/src/commands/{mod,companion}.rs`, `src/lib.rs` | Explicit caller checks and runtime composition |
| `apps/desktop/src-tauri/{build.rs,permissions/companion.toml,capabilities/main.json,capabilities/companion.json}` | Exact local command/event grants, retaining account label checks |
| `apps/desktop/src/platform/{companion-state,companion}.ts` | Revisioned projection/subscription lifetime and typed Tauri operations |
| `apps/desktop/src/features/companion/{CompanionCanvas,DesktopCompanion,desktop-gesture}.tsx/ts`, `desktop.css` | Explicit drawing permission, compact presentation and pointer gesture arbitration |
| `apps/desktop/src/app/{companion,CompanionControls}.tsx`, `companion.html`, `vite.config.ts` | Separate entry, native controls and both production inputs |
| `apps/desktop/tests/companion/*`, `apps/desktop/tests/{companion-native.mjs,windows-companion.ps1}` | Model/browser/build checks and separately scoped native/device acceptance |

Existing `Companion.tsx`, renderer and studio remain reusable. `DesktopCompanion`
does not import `Companion.tsx`, account code, shell CSS or studio. Keep changes
outside these owners limited to the contracts, stage ledger and evidence named
by each task.

## Task 1: Native companion owner and restricted IPC

**Files:** Create the four Rust companion files and child contract in the map;
create `src-tauri/src/commands/companion.rs`, `permissions/companion.toml` and
`capabilities/companion.json`. Modify `src-tauri/src/windows/mod.rs`, its contract,
`src-tauri/src/commands/mod.rs`, its contract, `src-tauri/src/lib.rs`,
`src-tauri/build.rs`, `capabilities/main.json`, this plan, native stages and code map.
All paths are below `apps/desktop/` unless prefixed `docs/`.

**Interfaces:**
- Produce serialized `CompanionState { revision: u64, visible: bool, renderAllowed: bool, paused: bool, error: Option<String> }`; frontend numbers must stay within JavaScript's safe integer range.
- Produce native `setup_companion(app: &AppHandle) -> Result<(), String>`, `snapshot(app: &AppHandle) -> CompanionState`, `set_visible(app: &AppHandle, visible: bool) -> Result<CompanionState, String>`, `set_paused(app: &AppHandle, paused: bool) -> Result<CompanionState, String>`, `ready(app: &AppHandle) -> Result<CompanionState, String>` and `drag(app: &AppHandle) -> Result<(), String>`.
- Expose the six exact command names in the [accepted interface](../../shared/requests/2026-10-16-independent-companion-window.md); companion state events are `companion-state`, targeted only to main and companion.
- `placement::fit_companion(size: (u32,u32), origin: Option<(i32,i32)>, areas: &[(i32,i32,u32,u32)]) -> Option<(i32,i32,u32,u32)>` returns a fully contained physical rectangle, preserving a valid origin; 16 physical pixel margin when space permits.
- Command helper `allowed_caller(label: &str, command: &str) -> bool` pins the calling-window matrix independently of the real IPC test.

Exact regression pins, expanded with the other scenarios below:
```rust
#[test]
fn companion_initial_position_uses_work_area_margin() {
    assert_eq!(fit_companion((240, 240), None, &[(0, 0, 1920, 1040)]),
               Some((1664, 784, 240, 240)));
}
#[test]
fn companion_cannot_mutate_visibility() {
    assert!(!allowed_caller("companion", "set_companion_visible"));
    assert!(allowed_caller("main", "set_companion_visible"));
}
```

- [x] **Write failing native tests** in `state.rs` and `placement.rs`. Assert: initial `{visible:false,renderAllowed:false,paused:false}`; ready causes only the latest requested visibility; hide before ready stays hidden; failed hide restores observed visibility with a stable error; failed construction can retry; denied caller changes nothing; initialization deadline expires into a retryable error; a stale generation cannot mark its replacement ready. Geometry: `(240,240),None,[(0,0,1920,1040)]` gives `(1664,784,240,240)`; preserve `(-500,300)` on `(-1920,0,1920,1080)`; fit inside `(0,0,100,80)`; zero areas return `None`; fitting the fitted result is unchanged.
- [x] **Observe RED:** `cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml companion`. Confirm a missing implementation/assertion failure rather than an unrelated dependency issue.
- [x] **Implement state and native effects.** Separate state decisions from OS calls, release shared state locks before native APIs/callbacks, and serialize transitions on the native UI thread. Emit render denial before hiding; read actual visibility after effects; advance revision for committed state/error changes. Readiness never reverses a later explicit hide. Keep unavailable/failed state recoverable through a subsequent explicit Show.
- [x] **Implement construction and placement.** Build `companion.html` only on initial setup or explicit reconstruction; use the spec's exact window properties and `.focused(false)`. Handle close, move, resize and scale changes without relocation recursion. A single 10-second readiness deadline per construction generation records `companion_unavailable` if initialization never completes; later Show reconstructs that failed surface. Cancel/ignore old deadlines and verify readiness comes from the currently owned WebView instance, not just the reused label. Initial construction failure records error but does not fail main/tray startup. Never extract code from the synthetic file-control probe.
- [x] **Implement `no_activate::show(window: &WebviewWindow) -> Result<(), String>`.** Isolate the Windows call on the native UI thread; use installed windows-sys `ShowWindow` with `SW_SHOWNOACTIVATE` and verify actual visibility. Its return value describes previous visibility, not a success boolean. Do not call `set_focus` or use ordinary `.show()` as a passive-restore fallback. The later physical test determines whether this combined implementation preserves foreground.
- [x] **Wire IPC and ACL.** Main alone changes visibility/pause; companion alone reports ready, starts drag and deliberately opens main; both may read state. Existing main-only account checks remain. Use `tauri_build::AppManifest` for explicit command permissions and only event listen/unlisten grants; avoid `core:default`. Add negative caller tests for every operation. During this task the frontend entry is not present yet; do not claim runnable product acceptance.
- [x] **Observe GREEN:** rerun the focused native command and `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check`. Update contracts/map/stages from inspected source and commit named task files as `feat(native): own independent companion lifecycle and IPC`.

## Task 2: Race-safe platform state and explicit animation permission

**Files:** Create `src/platform/companion-state.ts`, `src/platform/companion.ts`,
`tests/companion/native-state.test.ts`, `tests/companion/native-visibility.spec.ts`.
Modify `src/platform/contract.yaml`, `features/companion/CompanionCanvas.tsx`, its
contract, `tests/companion/fixture.tsx`, `package.json`, native stages and this plan.

**Interfaces:**
- Consume Task 1 state and commands exactly. TypeScript `CompanionState` has the same fields; reject malformed/nonfinite/unsafe revisions and invalid field types before use.
- Produce `connectCompanion(onState: (value: CompanionState) => void, onError: (error: string) => void, ready?: boolean): () => void`. It listens before snapshot; `ready=true` reports readiness after the listener exists. Both returned readiness state and snapshot pass through the same revision filter. Cleanup prevents callbacks/readiness after disposal and cleans late listeners.
- Its fourth optional test parameter is `transport?: CompanionTransport`, with `listen(handler:(value:unknown)=>void):Promise<()=>void>`, `snapshot():Promise<unknown>`, `ready():Promise<unknown>`. Produce pure `acceptState(current:CompanionState|null, raw:unknown):CompanionState`, rejecting malformed input and retaining current for an older/equal revision.
- Produce typed `setCompanionVisible(visible:boolean): Promise<CompanionState>`, `setCompanionPaused(paused:boolean): Promise<CompanionState>`, `dragCompanion(): Promise<void>`, `openMainFromCompanion(): Promise<void>`.
- Add `renderAllowed?: boolean` to `CompanionCanvasProps`, default true, and optional `onReady?:()=>void`, called after successful context/resource/viewport initialization without requiring a draw. Permission is distinct from `paused` and browser visibility.

Pin the race as `test("older ready reply cannot reverse newer native hide", ...)`:
```ts
const hidden = {revision:8, visible:false, renderAllowed:false, paused:false, error:null};
const oldReady = {...hidden, revision:7, visible:true, renderAllowed:true};
assert.strictEqual(acceptState(hidden, oldReady), hidden);
assert.throws(() => acceptState(hidden, {...oldReady, revision:NaN}));
```

- [x] **Write failing state tests** using an injected fake subscription/invoke transport, never clocks with arbitrary sleeps. Test `revision 8 hidden event` then `revision 7 ready reply` preserves denial; same revision is idempotent; late unlisten is called once after disposal; no ready call after disposed subscription; failed listener keeps initial permission false; malformed event reports local error without enabling drawing.
- [x] **Write failing browser checks** with the existing controlled RAF fixture. Keep `document.visibilityState === 'visible'`; render denial cancels all pending callbacks and causes zero draws after input/resize changes. Successful Canvas initialization reports ready even while drawing is denied, preventing a readiness/show deadlock; failed context never reports ready. Restore while paused draws exactly once with current inputs; restore under reduced motion does not restart continuous frames; unmount leaves no listeners/frames.
- [x] **Observe RED:** from `apps/desktop`, `node --test tests/companion/native-state.test.ts` and `npx playwright test native-visibility.spec.ts`.
- [x] **Implement the platform and Canvas changes.** Subscription transport is injectable for tests, production defaults use current Tauri API. Preserve the existing studio/browser consumers without native calls. Add the Node test to `test:companion:unit`.
- [x] **Observe GREEN:** rerun those focused commands and `npm run build`. Update actual contracts/stages, then commit named files as `feat(companion): synchronize native rendering permission safely`.

## Task 3: Compact independent entry and reliable gestures

**Files:** Create `companion.html`, `vite.config.ts`, `src/app/companion.tsx`,
`src/features/companion/DesktopCompanion.tsx`, `desktop-gesture.ts`, `desktop.css`,
`tests/companion/desktop-gesture.test.ts`, `desktop-window.spec.ts`.
Modify `features/companion/contract.yaml`, `tests/companion/fixture.tsx`,
`tests/companion/production.test.ts`, `package.json`, code map, native stages and plan.

**Interfaces:**
- `DesktopCompanion(): React.JSX.Element` uses Task 2 bridge and `CompanionCanvas`; starts denied. Its successful Canvas `onReady` starts `connectCompanion(...,true)`, which subscribes before reporting native readiness; do not wait for the first drawn frame while native drawing is denied. Local Canvas failure stays recoverable through main/tray and the native initialization deadline.
- `createDesktopGesture(actions: {drag:()=>Promise<void>; open:()=>Promise<void>; fail:(reason:string)=>void})` returns `down(event)`, `move(event)`, `up(event)`, `cancel()` and `dispose()` handlers accepting `{pointerId:number,button:number,x:number,y:number}`. It owns one gesture, consumes native drag completion and suppresses the associated browser click.
- Separate HTML entry mounts only `DesktopCompanion`. Use Vite production inputs `index.html` and `companion.html`; preserve default local dev URLs and existing test fixture handling.

Pin `test("eight pixel drag consumes panel click", ...)` with actions that increment
`dragCalls`/`openCalls` and `gesture = createDesktopGesture(actions)`:
```ts
gesture.down({pointerId:1,button:0,x:20,y:20});
gesture.move({pointerId:1,button:0,x:28,y:20});
gesture.up({pointerId:1,button:0,x:28,y:20});
assert.equal(dragCalls, 1);
assert.equal(openCalls, 0);
```

- [x] **Write failing gesture tests:** a displacement of `7.99` pixels opens once on release; exactly `8` starts drag once and never opens; ignored secondary input does neither; cancelling resets; resolving native drag without a mouse-up followed by a new primary gesture allows the next short click; rejection reports one error and recovers.
- [x] **Write failing browser/build checks:** independent 240x240 entry has a nonempty transparent Canvas, no account heading/auth calls/studio import, no scrollbars, local fallback on Canvas failure, and click/drag call only their intended wrapper. Production output has both HTML files; traverse each emitted chunk's imports/dynamic imports to prove the companion graph excludes account/shell/studio and every production chunk excludes studio/tests. Replace the old assertion requiring `Companion.tsx` in production with the actual desktop entry and renderer symbols.
- [x] **Observe RED:** `node --test tests/companion/desktop-gesture.test.ts tests/companion/production.test.ts`; `npx playwright test desktop-window.spec.ts` from desktop.
- [x] **Implement compact composition and entry.** Force root/body/canvas sizing and transparent CSS without shell CSS. Keep idle settings and all artwork paths; no extra gradient panel/status/control rows in the desktop surface. Add `aria-label` for the character action; keyboard-equivalent show/hide/pause remains in main/tray. Use pointer capture/cancellation deliberately and avoid a second independent DOM click handler after gesture arbitration.
- [x] **Observe GREEN:** focused tests and `npm run build`; inspect the actual 240px screenshot for head/glow/hand clipping. Add the gesture Node test to the unit script. Update contracts/map/stages; commit named files as `feat(companion): add independent transparent desktop surface`.

## Task 4: Main controls and tray integration without duplicate animation

**Files:** Create `src/app/CompanionControls.tsx`. Modify `src/app/main.tsx`, its
contract, `src-tauri/src/windows/mod.rs`, its contract and companion owner files
from Task 1 for tray state/error integration; `tests/companion/shell.spec.ts`,
`fixture.tsx`, `tests/windows-shell.ps1`, `tests/tray-menu.ps1`, native stages/plan.

**Interfaces:**
- `CompanionControls(): React.JSX.Element` consumes Task 2 state and wrappers; buttons name actual actions: `Show AYLUL`, `Hide AYLUL`, `Pause motion`, `Resume motion`, and `Retry AYLUL` when initialization failed.
- Tray item ID `companion-toggle` uses the same native owner as main controls and updates its Show/Hide AYLUL label from actual state. Existing Open NYLRA, Quit NYLRA and hotkey behavior retain their ownership.
- Main removes its continuously animated production `<Companion />`; the development studio remains lazy and isolated. Its explicit development preview may include the original full presence and optional synthesized sound controls inside main, default off; lazy-load this preview under the same Vite DEV guard. Production's compact surface has no sound interaction in this slice. No native audio permission or cross-WebView autoplay promise is introduced.

Pin `test("main remains usable when independent companion is unavailable", ...)`
using the shell fixture's native error snapshot:
```ts
await expect(page.getByRole("button", {name:"Retry AYLUL",exact:true})).toBeVisible();
await page.getByRole("button", {name:"Check session",exact:true}).click();
await expect(page.getByRole("heading", {name:"Account",exact:true})).toBeVisible();
await expect(page.locator("canvas")).toHaveCount(0);
```

- [x] **Write failing shell checks:** toggling native state changes main's button labels from actual revisioned replies/events; a rejected hide does not claim hidden; companion error offers retry while account/session and Hide to tray still work; main contains no active production Canvas; studio changes never invoke account/visibility controls. Adjust existing idle assertions to the independent surface rather than preserving duplicate production animation.
- [x] **Write tray/native-owner checks:** failed companion construction leaves Open/Quit routes installed; Show retries construction; main close does not hide/quit companion; companion close does not close main. Ensure old keyboard driver targets Hide to tray by accessible name, and tray driver selects exact menu item text rather than ordinal.
- [x] **Observe RED:** `npx playwright test shell.spec.ts` and `cargo test --locked --manifest-path src-tauri/Cargo.toml companion` from desktop.
- [x] **Implement composition and tray wiring** without modifying account actions or cancellation behavior. Preserve independent error boundaries around studio and companion controls. Main controls remain usable by keyboard without activating the companion.
- [x] **Observe GREEN:** rerun focused commands and `npm run build`; inspect script syntax without launching simultaneous test apps. Update contracts/stages and commit named files as `feat(desktop): control independent AYLUL from main and tray`.

## Task 5: Product-native acceptance, profiling and final review

**User amendment, 2026-10-17:** Continue in development/debug. All executable
builds and native drivers below use `cargo build --locked` and `target/debug`,
superseding release instructions in this plan. Optimized performance acceptance
is deferred to an explicitly requested later profiling step; debug measurements
are diagnostic only and cannot close the provisional release CPU budget.

**Files:** Create `tests/companion-native.mjs`, `tests/windows-companion.ps1`,
`docs/evidence/nylra/2026-10-16-independent-companion-acceptance.md`. Modify relevant
actual contracts, `apps/desktop/README.md`, root `README.md`, `docs/aylul/codebase/README.md`,
native stages/plan and the Task 1–4 source/tests only to fix reproduced findings.
The native test helpers in `tests/windows-support.ps1` are included for observed
notification-icon targeting and cleanup of orphaned test icons.
Keep failed diagnostic runs and successful evidence distinctly labeled.

**Interfaces:**
- `pwsh -NoProfile -File apps/desktop/tests/windows-companion.ps1 -RunName <unique-name>` launches only its owned release executable and writes ignored `tests/.runs/<unique-name>/` observations. Reuse the native pointer/tray helpers; never kill unrelated processes or change machine policy/preferences.
- `node apps/desktop/tests/companion-native.mjs --port <test-owned-loopback-port> --output <absolute-json-path>` connects Playwright CDP only to that owned WebView2 runtime. Debugging arguments exist only in the test launch environment; no debug port or test IPC ships in product defaults.
- The JSON report includes `focusCycles: {before:number,after:number}[]`, `hiddenFrames:{before:number,after:number}` and separately named success/failure observations. Pin the final native checks without treating an empty report as success:

```js
assert.ok(report.focusCycles.length >= 10);
for (const cycle of report.focusCycles) assert.equal(cycle.after, cycle.before);
assert.equal(report.hiddenFrames.after, report.hiddenFrames.before);
```

- [x] **Write the native driver with assertions before a run.** Locate both owned HWNDs by process/title; verify visibility/style and that closing main leaves companion visible. Perform real pointer drag and short click separately. Drive exact tray items and check both actual visibility and foreground HWND for at least 10 paired hide/show cycles with another owned test window foreground. Do not infer focus preservation from builder flags.
- [x] **Instrument the test-owned WebView externally** before reload to count Canvas render calls, without adding product globals. Native hide must stop drawing even while `document.hidden` is false; restore resumes one scheduler. Exercise actual IPC denial from companion for `auth_action`, `cancel_login`, `hide_main`, visibility and pause changes, and from main for companion-only ready/drag/open commands. Assert rejection without account mutation; no credential retrieval/output. Test the actual IPC bridge, not just the label helper.
- [x] **Exercise recovery** by failing only the owned companion's Canvas context during reload; main/tray remain functional, and explicit retry reconstructs/reinitializes companion without creating a second instance. If restart is needed, report that limitation rather than calling it recovery.
- [x] **Run final local integration checks sequentially:** from desktop, `npm run test:companion:unit`, `npm run test:companion:browser`, `npm run test:companion:production`, `npm run build`; from repo, `cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml` and `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check`; then `cargo build --release --locked --manifest-path apps/desktop/src-tauri/Cargo.toml`. Success requires zero failures; investigate failures without weakening assertions.
- [x] **Run actual Windows drivers sequentially** via existing `pwsh`, including `windows-shell.ps1`, `tray-menu.ps1` and the new companion driver against the final release binary. Do not bypass execution policy. Record binary hash, source commit, work area, DPI, refresh rate, screenshots and per-scenario results. A browser-only run is not native acceptance.
- [ ] **Deferred by user amendment: profile visible idle, paused and hidden** for five 10-second samples each using owned Rust and recursively owned WebView2 process CPU deltas, normalized as `100 * cpuSeconds / wallSeconds` for percent of one core. Capture stable process identity and new descendants during sampling; record incomplete samples if processes exit. Compare active CPU to provisional <=10% of one core; hidden Canvas count must stay constant. Do not report Rust-main-only CPU as renderer performance or silently change the threshold.
- [x] **Record remaining hardware evidence** honestly: available DPI observations are distinct from mixed-monitor transitions; do not suspend the user's machine or change display scaling unprompted. Retain unresolved gates with concrete closing triggers. Update README's current renderer/window description so it no longer points to archived PNGs as the live implementation.
- [x] **Request one fresh whole-branch review** through the review skill permitted by inline execution. No Astra reviewer unless explicitly requested anew. Give the approved spec/plan, base `5c35bf8` and current immutable review commit/diff, test evidence and unclosed gates. Fix actionable findings with regression tests, run affected checks, and document deferred issues precisely. Do not launch per-task implementer agents.
- [x] **Commit inspected acceptance and fixes** with named paths as `test(desktop): verify independent companion native behavior`. Mark tasks implemented only for proven behavior; retain open device/performance gates. Leave the branch local for the user's next integration instruction.

## Coverage and execution boundary

Spec sections 1–2 map to Tasks 1–4; section 3 to Tasks 1, 3–5; section 4 to
Tasks 1–4; section 5 to Tasks 1 and 5; section 6 to Tasks 2–5; section 7 to
this boundary and the final evidence report. Every Review Focus item has an
explicit owning regression test above.

The user approved the written spec and this implementation plan. Inline source
implementation and local development acceptance completed on 2026-10-17; see the
dated acceptance report for results and explicit remaining gates. No repeat choice
of branch, worktree or execution method is needed.
