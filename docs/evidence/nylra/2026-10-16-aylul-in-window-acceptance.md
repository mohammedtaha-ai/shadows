# AYLUL in-window implementation and acceptance

Date: 2026-10-16. Branch: `codex/aylul-companion-in-window`.
Status: product implementation complete; automated presentation and bounded native
shell checks passed. Remaining physical/runtime acceptance is explicitly open.

## Implemented scope

The existing AYLUL section now renders the procedural Canvas character extracted
from `ayl.html`. Rendering helpers, motion, scheduling, message lifetime, React
presentation, optional tones and studio simulations have separate owners under
`apps/desktop/src/features/companion/`. Account logic and native permissions were
not changed. Production starts idle and excludes the studio modules.

The development studio uses the same renderer for nine expressions, ten outfits,
six directions, true-size icon previews, stage-local movement/perching, synthetic
command/notification previews and transparent PNG export. No simulation executes
work, captures audio, delivers native notifications or manufactures confirmation.

Source HTML SHA-256 remains
`103a5ba2765c4a0f81f9b93051d2883fef1dc79a7e49dc8cb4ee0d0ff76a8bae`.
It remains untracked extraction input. At the user's later instruction, 68 legacy
asset-pack files and the reference board were moved to ignored local
`archive/aylul-legacy-2026-10-16/`; all moved SHA-256 values matched. Historical
commits were not rewritten. Fresh checkouts do not include that local archive.

## Local automated verification

Environment: Windows, Node 24.15.0, npm 11.12.1, existing React/TypeScript/Vite
versions, Playwright 1.64.1 using the installed Chrome channel. The managed
Chromium install attempt failed during headless-shell download; installed Chrome
provided the real Chromium engine used for these results.

Commands from `apps/desktop/` and observed results:

| Command | Observed result |
| --- | --- |
| `npm run test:companion:unit` | 10 passed, 0 failed after review corrections |
| `npm run test:companion:browser` | Final 34 passed, 0 failed, including review regressions and live DPR transition |
| `npm run test:companion:production` | 1 passed, 0 failed; emitted module IDs include Companion and exclude studio/test fixtures |
| `npm run build` | strict typecheck and Vite production bundle passed, 37 modules |

Tests cover real Canvas output for expression/outfit combinations, deterministic
frames, transparency, zero-size geometry, elapsed-time cadence, single scheduler
ownership, cancellation, message identity/deadlines, pause/reduced-motion still
updates, hidden-document suspension, StrictMode unmount/remount, text escaping and
wrapping, controlled-message precedence, local failure fallbacks, simulation
replacement, thumbnail stability and 512-square PNG download/alpha in Chromium.
Shell tests exercise the actual frontend platform wrappers against synthetic
Tauri IPC. They do not contact a real backend or validate real sign-in.

Observed RED before GREEN: renderer/presence/studio/shell missing before their
implementation; production assertion initially failed because Companion was not
included. A real unbound `requestAnimationFrame` invocation failed during browser
integration and was corrected with window-bound calls. A later still-direction
regression test failed with `0 !== 0.82` and passed after explicit preset projection.
The native keyboard-hide regression is recorded below. No runtime success is
inferred from source inspection alone.

Git whitespace and relative documentation links were checked. Companion source
contracts were parsed as YAML and their export/test references checked using the
installed TypeScript 7 scanner. This is a narrow lexical reference check, not a
general symbol/contract correctness proof. No remote CI ran.

## Visual inspection

The author viewed the generated gallery for all expressions and outfits, studio
layout/directions/icon previews, and narrow mixed Arabic/English text captures.
The hood, crystals, hands, eye expressions and palette match the selected
appearance shown in the user's screenshots. No pixel-perfect comparison against
the original standalone page is claimed; that page was not opened through a
browser-policy workaround.

Real-browser captures/assertions cover 320 CSS px width, DPR 1 and 2, light and
dark surroundings, plain text containing an HTML-like string, long unbroken text
and reduced motion. The bubble stayed within the presence bounds; image output
was nonblank and backing dimensions matched DPR. Light-background status text
was hard to read in the first capture; it now has an explicit dark background.

Review captures are local ignored artifacts under
`apps/desktop/test-results/visual-*/`. This is author visual review, not user
acceptance or a new approval of all animation combinations. PNG export uses scale
1.55 within the 512-square canvas instead of source 2.65 to preserve glow/shape
headroom; the drawing geometry itself is unchanged.

## Actual Windows shell acceptance

`cargo build --release --locked --manifest-path apps/desktop/src-tauri/Cargo.toml`
passed. The final rebuilt executable's SHA-256 was
`ab6673d628e1fe10bde380cf18ba4bae1afaff6b49a5f561b028bf12ac617c87`
after the shared perch correction. The native shell checks were rerun on that
rebuilt executable with RunName `aylul-in-window-reviewed-20261016`.

The plan's legacy `powershell -NoProfile -File ...` command was refused because
that host disabled scripts. No execution policy was modified. The existing
PowerShell 7 host (`pwsh`, local RemoteSigned policy) executed the repository's
local test normally:

```powershell
pwsh -NoProfile -File apps/desktop/tests/windows-shell.ps1 -RunName aylul-in-window-before-keyboard-fix-pwsh-20261016
pwsh -NoProfile -File apps/desktop/tests/windows-shell.ps1 -RunName aylul-in-window-named-keyboard-20261016
pwsh -NoProfile -File apps/desktop/tests/windows-shell.ps1 -RunName aylul-in-window-reviewed-20261016
```

The first run recorded the old first-Tab keyboard hide assumption as
`Passed: false`; tray click and ten restore cycles passed. The driver now focuses
the real UI Automation button named `Hide to tray` inside the owned window, sends
Space and fails if the native window remains visible. It does not directly invoke
the hide callback. The final run exited 0 and recorded:

| Actual scenario | Result |
| --- | --- |
| Window fits native work area | passed |
| Native startup, 736 x 679 outer pixels at DPI 96 | visible |
| Actual notification-area icon click | passed |
| Close hides and Ctrl+Alt+Shift+F12 restores | 10 cycles passed |
| Named UI button focused, Space hides native window | passed |

Run records and bounded screenshots remain under ignored
`apps/desktop/tests/.runs/aylul-in-window-reviewed-20261016/`.
The script sampled only the main process for about five seconds (0% of one core
in that reviewed run, 29.45 MiB working set). These figures exclude WebView2 descendants
and are not a claim of rendering efficiency or zero hidden rendering work.

## Remaining owner gates

- **Native/Character:** verify actual WebView frame suspension during tray hide,
  aggregate renderer CPU while visible/paused/hidden, and real DPI/monitor changes.
  Synthetic document visibility and DPR tests do not close these gates. If current
  signals prove insufficient, agree on a native interface before extending scope.
- **Character/Native:** actual WebView2 studio download and audio-device opt-in/
  mute observations remain open. Chromium export/negative audio tests are not
  those device observations; production intentionally has no studio export entry.
- **User:** review the resulting character/motion and the per-outfit appearance.
  The supplied idle/speaking screenshots guided extraction but do not approve all
  derived runtime presentation automatically.
- **Owning services:** real work/capture/playback state mapping remains future
  integration. No operational provider was invented in this slice.

Task C-6 therefore records partial acceptance with these closing triggers rather
than claiming every physical/runtime gate is complete. No push, merge, deployment,
worktree or history rewrite was performed.

## Independent whole-branch review and correction pass

A fresh `gpt-6.1-sol` reviewer inspected `cf670bb..4f087aa` read-only, reran the
nine then-existing Node tests and reproduced scene geometry numerically. It found
two P2 defects; both were corrected with observed failing regression tests:

1. Export preferred a stale visible-canvas snapshot after offscreen selection
   changes. The new browser test failed byte comparison against the requested
   blocked/winter/right-preset export. Export now retains the frozen motion pose
   but overlays current controlled expression, outfit and direction. That
   regression passed after correction. A second failing case covered clearing a
   previously visible locked direction while offscreen; snapshots now also track
   which controlled direction produced their pose so a cleared preset cannot leak
   into the new export.
2. Mock position bounds conflicted with the character's normal-stage bounds.
   The Node regression reported hands baseline 193 against mock top 100; a browser
   regression observed the initial mock exceeding a narrow stage. Perch scale and
   center now leave hood headroom and align the hands with the mock top. Mock
   bounds are shared by initial placement, dragging and parent resize, including
   borders. Both regression tests passed after correction.

The reviewer explicitly did not establish physical WebView/DPI/audio/download
acceptance, user visual approval, remote CI or real backend authentication. Those
remain the owner gates above. Its noted lack of live DPR transition coverage was
addressed with a Chromium metrics-change test while paused; it still does not
stand in for moving a physical Windows window between monitors. No deferred minor
findings were reported. The author verifies the correction pass with tests; no
second review seat or review pass was dispatched.

Verification encountered one 15-second page-load timeout in the export-error
test while the concurrent native build was running; 32 other tests passed. The
unchanged test passed alone in 11.5 seconds. The browser test budget now uses the
standard 30 seconds, with no automatic retries or weaker assertions. Subsequent
full-suite results below supersede that interrupted run without hiding it.

Final complete browser rerun: 34/34 passed in 54.3 seconds. Focused Node checks
passed 10/10 and the production-module check passed 1/1. No known P1/P2 findings
remain from the independent review after this correction pass. The remaining
physical/runtime gates above are still open, not reclassified as completed.
