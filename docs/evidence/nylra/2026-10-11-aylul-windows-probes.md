# AYLUL Windows probes - N0 follow-up

Probe sources and raw artifacts named here were removed from the tree on
2026-10-13; `git show 41b8e84:<path>` retrieves any of them.

Actual run date: **2026-10-11**, Asia/Aden. This continues N0 only, without a
reviewer agent. No N1, commit, push, deployment, cloud call or Shadows operation.
**N0 acceptance remains OPEN wherever the evidence below fails or is unavailable.**

## Starting state and machine

HEAD remains `94534bbfdeb141564b0f603bb7702422e6af663a` on `main`. Starting worktree
had the amended delivery spec/initial plan and untracked report/probe directory
from October 10. No existing user source changes were overwritten.

The workstation/toolchain is described in the
original inventory (`docs/evidence/probes/2026-10-10-n0/artifacts/machine.json` at `41b8e84`). Current GPU/display
inventory is here (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/display-current.json` at `41b8e84`):
GTX 1070, 1920x1080, **144Hz**, rather than the original 60Hz. Native snapshots
confirm scale 1.0. The same locked Tauri 2.11.5/internal graph, single-instance
2.4.5, global-shortcut 2.3.2 and WebView2 baseline were used. No new dependency
was added. No OS display setting was changed by the probe.

## Correction to historical evidence

The October 10 CSS companion was 180px, while the canvas companion was 128px.
The corrected canvas main was minimized during its measurement; CSS main remained
visible. Those historical values are preserved but **cannot support a controlled
cross-renderer comparison**. The [October 10 report](2026-10-10-aylul-windows-probes.md)
has been corrected to state these differences.

## Matched-window measurement

Retained driver: controlled-renderers.ps1 (`docs/evidence/probes/2026-10-10-n0/controlled-renderers.ps1` at `41b8e84`).
Both variants use the same v1 atlas, a 128x128 logical companion and a visible,
unminimized 640x480 main. The driver restores these conditions and asserts them
against native snapshots for every sample; both use DPR 1 on the current display.
Each stage has five CPU-delta samples about two seconds apart, after startup and
stage warm-up. It measures aggregate owned Tauri/WebView2 process CPU and working
set, as in the original report. No GPU-present-time or battery result is inferred.

Exact commands, repository-root working directory:

```powershell
cargo fmt --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml
cargo build --locked --offline --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml --target-dir docs/evidence/probes/2026-10-10-n0/target
& docs/evidence/probes/2026-10-10-n0/controlled-renderers.ps1
& docs/evidence/probes/2026-10-10-n0/controlled-renderers.ps1 -Renderers css
& docs/evidence/probes/2026-10-10-n0/close-native.ps1
```

| Renderer | Idle CPU | Active CPU | Hidden CPU | Reshown CPU | Reduced CPU | Maximum working set |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Canvas | 0.95% | 34.36% | 1.66% | 53.69% | 0.47% | 437.97 MiB |
| CSS raster | 1.06% | 27.50% | 6.53% | 23.65% | 1.18% | 450.81 MiB |

CPU percentages are relative to one logical core; divide by 12 for whole-machine
CPU. Shared working-set pages may be counted repeatedly. These are short debug
trials, sequential in a shared desktop session, not a statistically established
performance ranking. CSS's lower active mean in this controlled trial does not
establish production superiority. Both **FAIL** the provisional active <=10%
one-core budget; hidden allowance also fails. Both **PASS** the provisional
512MiB aggregate working-set threshold. Do not silently increase the budget.

The 144Hz run must be kept separate from the older 60Hz run. Rendering was not
frame-rate capped; a cap/release profile remains a closing prerequisite rather
than a retrospectively claimed improvement. Native visibility cancels steady-state
hidden callbacks; callback cadence remains a proxy, not GPU presentation time.

Evidence: canvas CSV (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/performance-canvas.csv` at `41b8e84`),
CSS CSV (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/performance-css.csv` at `41b8e84`),
summary (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/summary.json` at `41b8e84`),
canvas runtime (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/runtime-canvas.jsonl` at `41b8e84`),
CSS runtime (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/runtime-css.jsonl` at `41b8e84`).

The first CSS follow-up stopped when concurrent writes interleaved JSONL records.
The logger was corrected with a process-local mutex and one complete-line write;
the CSS run was repeated. The failed partial CSV and
raw interleaved log (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/runtime-css-interleaved.raw` at `41b8e84`)
are retained and excluded from the completed sample set. Native event records now
include Unix millisecond timestamps. This repairs the probe, not a product service.

## Native lifecycle and tray investigation

The probe now pairs explicit renderer visibility with a Windows
`ShowWindow(hwnd, SW_SHOWNOACTIVATE)` experiment. This is a minimal Windows-only
FFI probe, not a scaffolded platform module. Earlier paired focus preserved the
main foreground in one run; a long-interval rerun had a foreground change and did
not establish attribution. The final driver compares foreground immediately
around show, after the hidden observation interval, to reduce that ambiguity.
See native observations (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/native-observations.json` at `41b8e84`)
and runtime (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/runtime-native.jsonl` at `41b8e84`).
This does not prove actual suspend/resume, monitor removal or mixed DPI.

Final paired result: foreground HWND **5704632 before and after show**; PASS this
immediate no-activation trial. The hidden interval and subsequent resumed callbacks
are recorded in the timestamped runtime; this is current-machine lifecycle proof,
not complete device acceptance.

Tray investigation found that the native icon rectangle pointed at the hidden-icon
overflow arrow. The rectangle screenshot (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/tray-icon-rectangle.png` at `41b8e84`)
shows the arrow, so previous clicks were not a valid actual-icon delivery test.
A probe-owned menu item `N0 Show Main` was added using Tauri's existing menu API,
with event recording. The final driver opens overflow and targets only a matching
owned Button, never a tooltip, before selecting the owned menu item. Pass/fail
depends on actual recorded events; a menu definition alone is not acceptance.

Final actual icon click **PASS**: pointer (1606,835) inside the owned icon rectangle
(1586,815), 40x40; Left Down and Left Up were recorded at Unix millisecond
1791671349398. The menu item was not exposed and no menu action was recorded:
**UNRESOLVED menu activation**, distinct from the now-proven actual icon click.

## Further asset-production attempt

Built-in imagegen was exercised once more, now requesting one padded idle sprite
as a master rather than an equal-cell atlas. Input: preserved user-selected
`AYLUL Companion Character Reference Board.png`; transparent_background=true.
Output copied intact to idle-v3.png (`docs/evidence/probes/2026-10-10-n0/native/ui/idle-v3.png` at `41b8e84`).
Output ID `exec-b2ff78ca-8ecd-4572-ac42-4b8c072e211c`; internal model/version
not exposed. Exact prompt is appended to the retained asset provenance file.

Actual output is 1254x1254. Significant alpha (>32) is padded, but low-alpha noise
still reaches image boundaries, including 40 top/bottom positions. See
metrics (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11/idle-v3-metrics.json` at `41b8e84`).
**FAIL strict production-alpha preparation; do not ship this master.** Do not
pretend that the requested padding or subsequent equal-state geometry was achieved.
No pixel cleanup, additional matching states, turning, or six remaining expressions
were produced from it. Manual asset cleanup/authoring and distribution provenance
remain open; no protected Coucou art was used.

## Decisions and N1 prerequisites

| Decision | Status / closing trigger |
| --- | --- |
| User-selected simpler AYLUL board | ADOPT; originals preserved |
| Browser raster without another renderer dependency | Retain experiment baseline; final renderer UNRESOLVED until agreed budgets and release/frame-rate evidence |
| New third image output | REJECT as production master; consistent alpha/state assets still required |
| Explicit native visibility and serialized probe recording | ADOPT measured probe requirements; no product implementation claim |
| Default Tauri focus-stealing show | REJECT; no-activate lifecycle remains conditional on full native proof |
| Tauri core tray icon click | ADOPT tested click baseline; owned Left Down/Up observed. Menu activation remains unresolved |
| PixiJS/Rive | No extra installation; original conditional justification/authoring gates remain |
| DPI, display removal, real suspend/resume | BLOCKED on corresponding controlled device scenarios, not replaced by resizing or synthetic messages |

Coucou provenance and complete applicable notices remain in the
research snapshots (`docs/evidence/probes/2026-10-10-n0/provenance/README.md` at `41b8e84`); no executable Coucou
code or protected media was reused. See the October 10 report for source links,
licenses and initial commands. Probe source remains retained and non-product.

The spec and initial plan reflect these supported observations. N1 remains
unstarted. Full N0 acceptance cannot be declared while performance/assets fail or
device/native scenarios remain untested. No reviewer agent was restarted.

## Checks and retained artifacts

Locked/offline native builds passed after adding the visibility/tray experiments
and serialized logger. Rust formatting, YAML/source-path checks, Python/embedded
JavaScript syntax, local report links and trial hashes were checked. The evidence
checker validates retained JSONL, the original 150 complete samples and 50 matched
follow-up samples, including required window conditions. Failed partial CSV/raw
logs are retained separately. No release build, GPU-present-time test or real
sleep/DPI/device-removal check is claimed. Drivers leave no probe process running.
