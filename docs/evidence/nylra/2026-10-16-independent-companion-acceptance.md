# Independent AYLUL development acceptance

Executed: 2026-10-17, Asia/Aden. Original checkout, branch
`codex/aylul-companion-in-window`; no worktree, push, merge or deployment.

## Delivered behavior

Native Tauri now owns a transparent, undecorated, topmost 240 by 240 logical
pixel companion window independently of main. The compact React entry reuses the
existing procedural renderer without loading account or development studio code.
Main contains companion visibility/motion controls and retains its account actions.
Tray contains Open NYLRA, Show/Hide AYLUL and Quit NYLRA.

Closing main leaves the character alive. A short character click deliberately
opens main; an eight-pixel-threshold native drag consumes that click. Hidden native
state explicitly cancels drawing even when the WebView document remains visible.
The companion has only presentation/readiness/failure/drag/navigation permissions;
account operations remain restricted to main. Rendering failure becomes a native
retryable state; transient operation errors leave Canvas mounted.

## Source and environment

- Windows 11 Pro, 10.0.26200; current single-display trials at 96 DPI.
- Native implementation commits: `504450c`, `e0b79e9`, `d0c7a58`, `9068133`,
  `207f5ac`, plus the final correction commit containing this report.
- Tested **debug** executable:
  `apps/desktop/src-tauri/target/debug/nylra-desktop.exe`.
- SHA256: `B94035495B0A7E83C83B882B458691EF609CA72E4A98FB3E81E09449C16B422F`.
- User explicitly selected continued development/debug work on 2026-10-17.
  An earlier optimized compilation completed before that correction but was
  never launched or published. No optimized performance acceptance is claimed.
- Root `ayl.html` remains untracked and unchanged, SHA256
  `103a5ba2765c4a0f81f9b93051d2883fef1dc79a7e49dc8cb4ee0d0ff76a8bae`.
  Existing generated desktop/windows schema changes remain outside these commits.

## Local automated checks

| Command | Observed result |
| --- | --- |
| `npm run test:companion:unit` from desktop | 15 passed |
| `npm run test:companion:browser` / final `npx playwright test --reporter=list` | 43 passed in 57.3 seconds, installed Chrome |
| `npm run test:companion:production` | 1 passed; both HTML entries and emitted dependency graphs inspected |
| `npm run build --prefix apps/desktop` | Strict TypeScript and Vite build passed, 46 modules |
| `cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml` | 24 passed, 1 intentionally ignored external integration test |
| `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check` | Passed |
| `cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml` | Development build passed |

The browser suite includes revision ordering, late subscription disposal, explicit
native render denial, reduced motion, desktop gesture arbitration, action-error
recovery, after-readiness drawing failure, account isolation and studio exclusion.
Mocked IPC tests are frontend evidence; the separate device checks below use the
actual native bridge. No real account login, CI or backend deployment is implied.

## Actual Windows observations

Ignored reproducible outputs are in `apps/desktop/tests/.runs/`:

| Run | Observed checks |
| --- | --- |
| `independent-debug-final-20261017-l` | Native state/IPC, 10 hide/show foreground cycles, hidden drawing suspension, rendering-failure reconstruction, actual tray Quit |
| `independent-debug-pointer-20261017-n` | Real native drag, short character click, close-main independence, actual tray Hide/Show and Quit, native recovery; focus cycle count intentionally 0 because the preceding final run covers it |
| `independent-debug-shell-20261017-d` | Main fits work area, actual tray left click, 10 close/shortcut cycles, actual Tab/Space Hide to tray |
| `independent-debug-menu-20261017-c` | Actual tray Open restores/focuses main; actual Quit exits the owned application |

Native companion extent was 240 by 240 physical pixels at 96 DPI, initially
`(1664, 776)`. Its client rectangle equals its full outer extent; no native
non-client frame is visible and the topmost extended style is present. Tao retains
caption style bits internally while removing chrome through `WM_NCCALCSIZE`;
checking those bits alone was an invalid decoration test.

In final run `l`, Canvas count stayed **91 -> 91** during the hidden interval while
`document.visibilityState` was **visible**. It resumed after restore. All 10 paired
foreground HWND comparisons remained equal to the owned, responsive witness window.
Native IPC rejected main-only account/hide/pause/visibility actions from companion,
and companion-only ready/failure/drag/open actions from main. No credentials were
retrieved into the test report.

Controlled Canvas failure after readiness produced native retry state; explicit
Show/Retry destroyed the failed surface, waited for native destruction/label
removal, then built one healthy replacement without restarting the application.
There was exactly one companion page after recovery. Screenshots of the actual
WebView and the 240px browser view were inspected for character/head/glow/hand
clipping and transparent corners.

The main-only process sample in the legacy shell driver excludes WebView2
descendants and is **not** renderer performance evidence.

## Failures found and corrected

Failed diagnostic runs are retained separately, not counted as accepted runs.

1. Targeted native events do not reach a generic Any-target listener. The bridge
   now uses the current WebviewWindow listener, preserving targeted delivery.
2. Direct `SW_SHOWNOACTIVATE` bypasses Tao's cached visibility flags. A subsequent
   Tao hide could be skipped; an actual trial counted 113 -> 259 frames while
   intended hidden. The adapter now pairs direct no-activate show with `SW_HIDE`
   and verifies actual Win32 visibility after each operation.
3. Immediate destroy/rebuild raced Tauri's asynchronous label removal. Recovery
   now waits for Destroyed and queues reconstruction after event dispatch.
4. A fresh final reviewer found that an action error permanently replaced Canvas,
   and after-readiness rendering failure was invisible to native Retry. Action and
   Canvas errors are separate; the fixed companion-only `companion_failed`
   operation reports a stable error without accepting caller error text.
   Two browser regressions and the native model regression were observed RED,
   then GREEN; the complete post-fix suites above passed.

Device-driver corrections preserved the intended assertions: distinguish the
notification icon by the observed `NotifyItemIcon` UI Automation ID instead of the
same-named taskbar button; reacquire icons after Explorer removes orphaned test
icons; keep the focus witness message pump responsive to prevent ghost HWNDs;
select an ephemeral test-only loopback debug port; and traverse native Tab focus
before Space rather than relying only on the WebView accessibility focus anchor.
Pointer trials move the character left before testing the tray popup, keeping the
rectangular companion hit region out of that interaction. Main-only drivers
independently hide the optional companion before testing main controls.

## Review and implementation rulings

One fresh read-only reviewer inspected immutable `5c35bf8..207f5ac`. Its two P2
findings are corrected above. No second review seat or re-review was launched;
the author verified the correction pass with regression and complete suite runs.
No minor findings were deferred.

- User's normal branch/no-worktree choice overrides workspace creation. No extra
  checkout is created; unrelated schema/root-HTML changes remain unstaged.
- Generated ACL manifests/command permissions travel with their native source
  configuration. The older dirty desktop/windows schemas remain outside commits;
  confusing those artifacts could lose unrelated workspace changes.
- Production HTML is checked after Vite finishes emission rather than in an early
  user generateBundle hook; otherwise a valid build would be rejected prematurely.
- Actual client/outer dimensions establish frame removal for this Tao version;
  mistaking internal style bits for visible chrome would drive an unnecessary OS fix.
- Development validation supersedes optimized profiling by explicit user request;
  treating debug measurements as release evidence would give a false budget claim.
- Destruction-aware reconstruction and the narrow failure report implement the
  existing native recovery owner; a regression there would leave Retry unavailable.
- Reviewer set aside possible stale readiness across document replacement because
  concrete dispatch ordering was not established. Current caller/owned-HWND checks,
  generation-scoped deadlines and revision filtering stand; add a controlled
  document-replacement/queued-IPC test before claiming that race exhaustively covered.
- Reviewer found no concrete placement oscillation/deadlock. Current fitting is
  idempotent and compares actual coordinates before mutation; physical monitor
  transitions still close only through the named hardware evidence below.
- Late subscription disposal, stale reply ordering, next click after consumed
  mouse-up and zero work areas have inspected regression coverage; no extra finding
  was set aside for those behaviors.

## Remaining gates and closing triggers

These are explicit limits of current development acceptance, not passing results:

- Optimized renderer CPU/memory/frame timing, including all WebView2 descendants:
  Native owns a later explicitly requested release profiling session and budget review.
- Real mixed-DPI transitions, monitor removal and suspend/resume: Native records
  actual hardware/device observations when available; synthetic rectangles and
  historical probes do not close them.
- The compact surface has a rectangular pointer area including transparent
  padding. Exact silhouette click-through and interactions over system popups need
  their separately agreed hit-test work; no global cursor polling is introduced.
- The optional development studio's native PNG/audio acceptance remains separately
  open from the earlier in-window feature; this slice adds no production audio.
- Full screen-reader, 200% scaling and arbitrary foreground application combinations
  remain unverified beyond the named local keyboard and 96-DPI trials.
