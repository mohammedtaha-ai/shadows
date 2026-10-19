# AYLUL Windows probes - N0

Probe sources and raw artifacts named here were removed from the tree on
2026-10-13; `git show 41b8e84:<path>` retrieves any of them.

Run date: **2026-10-10**, Asia/Aden (UTC+03). Local Windows experiments only.
Documentation finalized: **2026-10-11**; no new device trial is implied by that date.
For actual additional October 11 trials and corrected comparison conditions, see
[the dated follow-up](2026-10-11-aylul-windows-probes.md). Results below describe
the October 10 run; later tray success is recorded in the follow-up.
**N0 was exercised; its acceptance exit remains OPEN. N1 has not started.**
There is no production application, production asset pack, approved performance
budget, or completed native-device acceptance in this change.

## Starting state and scope

- Repository: `E:\Globalprojects\nylra`; branch `main`.
- Starting HEAD: `94534bbfdeb141564b0f603bb7702422e6af663a`.
- `git status --short` was empty before work. Original PNG hashes were checked
  before and after the trials and are unchanged.
- Read AGENTS.md, README.md, Vision (including desktop presence), the delivery
  specification, initial plan, source structure, existing Coucou/library research,
  shared architecture and contract convention/template. Shared authority stayed
  in its existing owners; no cloud protocol or approval behavior was implemented.
- During the run Mohammed explicitly selected **AYLUL Companion Character
  Reference Board.png**. The richer sheet remains a preserved comparison input.
- All trial source, synthetic state, screenshots and measurements are under
  the retained probe directory (`docs/evidence/probes/2026-10-10-n0/README.md` at `41b8e84`). No daemon commands,
  daemon configuration, Shadows data, account content or cloud calls were used.
- No commit, staging, push, deployment, PR or later-task implementation occurred.
  Cargo downloaded dependencies needed for the native probe; no npm, PixiJS,
  Rive, React or product scaffold was installed.

## Machine and versions

Machine inventory (`docs/evidence/probes/2026-10-10-n0/artifacts/machine.json` at `41b8e84`) and
display inventory (`docs/evidence/probes/2026-10-10-n0/artifacts/displays.json` at `41b8e84`) preserve details.

| Item | Observed value |
| --- | --- |
| OS | Windows 11 Pro x64, 10.0.26200 / build 26200 |
| CPU / RAM | Intel Core i7-8750H 2.20 GHz, 12 logical processors; 34,278,969,344 bytes RAM |
| GPU / driver | NVIDIA GeForce GTX 1070; installed driver entry 582.28 |
| Display | One 1920x1080 display, 60 Hz, work area 1920x1032; Tauri scale 1.0; both HWNDs 96 DPI |
| WebView2 | Installed runtime 154.0.4258.37 |
| Rust / Cargo | rustc 1.98.1 (48a229cea); cargo 1.98.1 (797e8a9bc), stable x86_64-pc-windows-msvc |
| Other installed tools | Node v24.15.0; npm 11.12.1; Python 3.14.5; Pillow 12.3.0; PowerShell 7.6.5; Git 2.54.0.windows.1 |
| Native probe graph | Tauri 2.11.5; tauri-build/macros/codegen/plugin 2.6.3; utils 2.9.3; runtime 2.11.3; runtime-wry 2.11.4; wry 0.55.1; tao 0.35.3 |
| Official plugins | single-instance 2.4.5; global-shortcut 2.3.2; tray-icon 0.24.2 / muda 0.19.3 transitively |
| Asset tool | Built-in imagegen, two calls; internal model/version not exposed |

The exact dependency checksums and graph are retained in
Cargo.lock (`docs/evidence/probes/2026-10-10-n0/native/Cargo.lock` at `41b8e84`). This is the tested graph,
not a claim that these are the newest releases. No Rive editor/CLI, authored
`.riv` interaction, Inkscape, Blender, Photoshop, Krita or ImageMagick was found
through the limited installed-app/command checks. A browser-based Rive account
and authoring workflow were not established.

## Commands and reproduction

Commands below were run from the repository root unless stated otherwise.
The probe README (`docs/evidence/probes/2026-10-10-n0/README.md` at `41b8e84`) gives the retained source map,
safe reproduction sequence and paths for the rejected variants.

```powershell
git rev-parse HEAD
git status --short
git branch --show-current
Get-FileHash -Algorithm SHA256 *.png
cargo --version
rustc --version
rustup show active-toolchain
node --version
npm --version
python --version
python -c "import PIL; print(PIL.__version__)"
Get-CimInstance Win32_OperatingSystem
Get-CimInstance Win32_Processor
Get-CimInstance Win32_ComputerSystem
Get-CimInstance Win32_VideoController
powercfg /a
python docs/evidence/probes/2026-10-10-n0/inspect-assets.py
```

Source/license snapshots were downloaded using `Invoke-WebRequest -Uri
($base+'/'+$upstreamPath) -OutFile $localPath`, where `$base` was exactly
`https://raw.githubusercontent.com/Louis-CFM/coucou/08393b657c79e99382eaf15185f480e6745997b0`.
The provenance map (`docs/evidence/probes/2026-10-10-n0/provenance/README.md` at `41b8e84`) enumerates every file.
Imagegen inputs, exact prompts, output IDs and copy destinations are in
asset-prompts.md (`docs/evidence/probes/2026-10-10-n0/asset-prompts.md` at `41b8e84`).

```powershell
cargo build --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml --target-dir docs/evidence/probes/2026-10-10-n0/target
cargo build --locked --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml --target-dir docs/evidence/probes/2026-10-10-n0/target
cargo build --offline --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml --target-dir docs/evidence/probes/2026-10-10-n0/target
cargo build --locked --offline --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml --target-dir docs/evidence/probes/2026-10-10-n0/target
& docs/evidence/probes/2026-10-10-n0/run-native.ps1
& docs/evidence/probes/2026-10-10-n0/measure.ps1
& docs/evidence/probes/2026-10-10-n0/supplemental-native.ps1
python docs/evidence/probes/2026-10-10-n0/summarize.py
cargo fmt --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml
cargo fmt --check --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml
cargo check --locked --offline --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml --target-dir docs/evidence/probes/2026-10-10-n0/target
python docs/evidence/probes/2026-10-10-n0/check-evidence.py
git diff --check
```

For the CSS trial the executable was launched with `Start-Process -PassThru
-WindowStyle Hidden`, `N0_LOG=.../artifacts/runtime-css.jsonl` and
`N0_CONTROL=.../artifacts/control.txt`. Its PID was saved to `owned-pid.txt`.
After four seconds, `Set-Content $env:N0_CONTROL 'css' -NoNewline` selected CSS;
one second later `measure.ps1` ran. run-renderer.ps1 (`docs/evidence/probes/2026-10-10-n0/run-renderer.ps1` at `41b8e84`)
retains that equivalent launch sequence. Earlier canvas runs used the default
canvas renderer. No probe changed OS display settings or put the computer to sleep.

## Asset experiments

| Experiment / inputs | Expected | Observed / evidence | Status |
| --- | --- | --- | --- |
| Inspect/hash both root PNGs | Preserve both inputs; establish whether they are atlases | Both 1536x1024, all 1,572,864 pixels opaque; boards contain text and background. Hashes below match start/end | PASS preservation; reject boards as shipping sprites |
| Equal-size board excerpts at 64/96/128 CSS pixels, actual DPR 1 | Judge eyes and silhouette with aspect ratio preserved | Both retain eyes; simpler board has less ornament detail competing at 64px. Richer cloth/crystals crowd the lower silhouette. This is a visual judgment, not an automated readability score | PASS comparison; selected reference is the user's choice |
| Imagegen transparent idle/listening/blocked atlas v1 | Equal canvas, pivot, scale; readable expressions; clear alpha edges | Real transparency, three 724x724 cells, but threshold-alpha body centers shift about 54-57 source pixels between cells (~9.5-10px at 128px). Nonzero alpha reaches top/bottom boundaries. Listening remains close to idle | FAIL production geometry/readability |
| Targeted v2 alpha/alignment correction | Repair v1 without changing identity | Alignment and boundary-alpha problems remain; 93/121/95 top-or-bottom positions per cell have nonzero alpha. No production acceptance | FAIL correction |
| Native light/dark contact sheets at 64/96/128px | No obvious backdrop rectangle; meaningful state distinction | Edges composite without the original opaque board rectangle; crystal halos visibly differ on light/dark. Blocked differs; idle/listening remain subtle. No rigorous halo color/contrast threshold was established | PASS basic compositing; FAIL expression/alignment gate |

Original SHA-256:

- Selected reference: `a1ef31bc867e3905a5f40092032afc3c817dfe82c651060835fcf5f312785717`.
- Richer comparison: `7479d63424c01a66a7cd6d58ad684f97d7762fc0936ec12f1623b19a1d8553e3`.

The asset metrics (`docs/evidence/probes/2026-10-10-n0/artifacts/asset-metrics.json` at `41b8e84`) include full
alpha counts and cell bounds. Many partial-alpha pixels are nearly opaque:
v1 has 532,662 pixels at alpha 240-254, and sampled hood pixels were alpha 252-253.
The partial-alpha total alone does **not** demonstrate a translucent body.
The production rejection rests on alignment, clipped halo/padding and expression
distinction. A declared equal pivot in the trial manifest (`docs/evidence/probes/2026-10-10-n0/asset-manifest.json` at `41b8e84`)
does not establish that the drawn subjects obey it.

Screenshots: v1 contact sheet (`docs/evidence/probes/2026-10-10-n0/artifacts/asset-comparison.png` at `41b8e84`),
v2 contact sheet (`docs/evidence/probes/2026-10-10-n0/artifacts/asset-comparison-v2.png` at `41b8e84`).
Original generated PNGs remain byte-for-byte intact in
v1 (`docs/evidence/probes/2026-10-10-n0/native/ui/trial-atlas.png` at `41b8e84`) and
v2 (`docs/evidence/probes/2026-10-10-n0/native/ui/trial-atlas-v2.png` at `41b8e84`). These are derivative trial
art from the supplied AYLUL reference, with no Coucou art input. Original-board
distribution rights and final generated-asset terms are not documented; close
that provenance gate before shipping.

Mohammed observed the live motion and changing expressions and said they looked
fine so far. He then noted that turning and other motion remained. This is useful
preliminary visual feedback on this machine, **not** acceptance of the nine-state
pack, directional animation, production assets, or native lifecycle. The trial
only bobs a raster and swaps three states once a second; it has no turn animation,
rig, lip sync or real listening/product state. Motion snapshots:
1 (`docs/evidence/probes/2026-10-10-n0/artifacts/motion-1.png` at `41b8e84`),
2 (`docs/evidence/probes/2026-10-10-n0/artifacts/motion-2.png` at `41b8e84`),
3 (`docs/evidence/probes/2026-10-10-n0/artifacts/motion-3.png` at `41b8e84`).

## Rendering and performance experiments

Provisional budgets (`docs/evidence/probes/2026-10-10-n0/budgets.json` at `41b8e84`) were written before the
performance trials: mean idle <=1% of one core, active <=10%, hidden no more than
idle+0.5 percentage points; working set <=512 MiB; rAF p95 <=20ms; no steady-state
hidden callbacks. These were experiment thresholds, not a user-approved SLA.
Do not relax them silently or label a rejected run accepted.

Each run used the same workstation, two native WebViews and the same v1 atlas.
Initial/corrected canvas companion size was 128 logical pixels; CSS used a fresh 180px companion. The corrected canvas main became minimized during measurement (runtime records position -32000 and size 0x0), while the CSS main remained 640x480. These historical runs are not a controlled cross-renderer comparison.
Each stage has ten CPU-delta samples about two seconds apart, after a two-second
stage warm-up. Owned Tauri and recursive WebView2 descendants were aggregated
(nine processes); other application processes and the PowerShell sampler were
excluded. CPU is summed processor seconds / elapsed wall time, expressed relative
to **one** logical core; divide by 12 for whole-machine percentage. Working-set
sums can count shared pages more than once and are not exclusive physical RAM.
Runs were sequential, short, unoptimized debug builds in a shared desktop session;
no statistical significance, battery claim or GPU attribution follows.

| Variant | Idle CPU | Active CPU | Hidden CPU | Reshown CPU | Reduced CPU | Peak working set across stages |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Initial canvas; resize every frame; rely on document.hidden | 1.17% | 19.49% | 16.96% | 18.57% | 1.67% | 421.67 MiB |
| Canvas; resize on size change; explicit native visibility | 1.32% | 18.28% | 0.35% | 17.40% | 0.89% | 413.00 MiB |
| CSS background atlas + Web Animations; explicit native visibility | 0.63% | 18.27% | 1.82% | 11.64% | 0.70% | 440.21 MiB |

Raw inputs: initial CSV (`docs/evidence/probes/2026-10-10-n0/artifacts/performance-initial.csv` at `41b8e84`),
corrected canvas CSV (`docs/evidence/probes/2026-10-10-n0/artifacts/performance-canvas.csv` at `41b8e84`),
CSS CSV (`docs/evidence/probes/2026-10-10-n0/artifacts/performance.csv` at `41b8e84`),
aggregate summary (`docs/evidence/probes/2026-10-10-n0/artifacts/summary.json` at `41b8e84`).
Corresponding runtime logs: initial (`docs/evidence/probes/2026-10-10-n0/artifacts/runtime-initial.jsonl` at `41b8e84`),
canvas (`docs/evidence/probes/2026-10-10-n0/artifacts/runtime-canvas.jsonl` at `41b8e84`),
CSS (`docs/evidence/probes/2026-10-10-n0/artifacts/runtime-css.jsonl` at `41b8e84`).

- **FAIL initial hidden behavior:** native hide did not change document.hidden;
  full frame callbacks continued. The initial run had active full bins while hidden.
- **PASS corrected steady-state cancellation:** both renderers' subsequent full
  hidden bins have zero callbacks after explicit native visibility signaling.
  The first partial bin contains pre-hide callbacks (257 canvas / 146 CSS); there
  are no event timestamps proving an exact cancellation-latency bound.
- **FAIL active CPU budget in all three runs.** Canvas size reuse did not provide
  a sufficient improvement. Different companion sizes and main-window states
  prevent a valid CSS-versus-canvas performance comparison. Those historical
  means must not be interpreted as equal-workload renderer evidence.
- **FAIL corrected canvas idle mean; FAIL CSS hidden CPU allowance:** CSS hidden
  1.82% exceeds its idle 0.63% + 0.5. Attribution needs a longer release trial.
- **PASS measured memory threshold.** This is aggregate debug working set only.
- **PASS rAF cadence proxy:** maximum p95 across full active bins was 17ms in each
  run; observed maximum intervals were 18.30/17.90/18.20ms. These are callback
  intervals, not GPU presentation timings or measured per-frame drawing costs.
  CSS retains a rAF loop for measurement only. End-to-end frame-time acceptance
  remains unproven.
- **PASS synthetic reduced mode stopping steady-state callbacks.** OS preference
  was false; toggling a real Windows reduced-motion setting was not tested.

PixiJS was not installed: this trial has one raster and no independently animated
layers. Failed CPU thresholds alone do not justify adding a rendering engine
without profiling. Rive remains an authored-rig alternative, not a PNG converter;
no authored interaction/tool workflow was available, so its runtime/fidelity trial
is **BLOCKED**, not passed. Final renderer acceptance remains OPEN.

## Actual Windows capability experiments

`run-native.ps1` sends native control operations through a probe-owned local file,
uses actual Win32 mouse/keyboard input, and records HWND rectangles/DPI/foreground
handles. `supplemental-native.ps1` tests alternatives without any Shadows access.
Expected outcomes are below; API success is separated from behavioral proof.

| Capability / exact input | Expected | Observed | Status |
| --- | --- | --- | --- |
| Transparent, undecorated, always-on-top companion over synthetic main checkerboard | Main pixels show through alpha outside character | Observed in native screenshot; both WebViews loaded real 2172x724 PNG at DPR 1 | PASS current Windows configuration |
| `overlap`, `restricted`; click (205,205), then (290,290), each after 500ms cursor dwell | Padding click reaches main; central target reaches companion | Main pointer (137,114), companion pointer (90,90); ignore flag toggled | PASS slow rectangular target only; fast crossing/alpha-shaped hit testing unresolved |
| `drag`, mouse-down at companion+(80,80), move +(60,30), release | Native window follows and stops | HWND origin moved (760,200) -> (820,230) | PASS simple native drag; rapid drag/click races untested |
| `focus-main`, `hide`, `show` | Companion reappearance preserves main foreground | Foreground changed from main to companion | FAIL default Tauri hide/show focus policy |
| `nonfocusable`, repeat hide/show | Removing focusability prevents activation | Foreground still changed to companion | FAIL this attempted mitigation |
| `offscreen`, `recover` while main has focus | Position-only parking/restore preserves focus | Foreground handle unchanged in supplemental runs | PASS isolated primitive; full parking lifecycle unresolved |
| Hide, Win32 `ShowWindow(hwnd,4)` (SW_SHOWNOACTIVATE) | Reappearance preserves main focus | Foreground unchanged; previously-visible result false | PASS isolated primitive; animation restoration/production wrapper untested |
| Tray builder; `tray-hide`, `tray-show`; shell input attempts | Icon lifetime works and actual user click reaches handler | Creation and visibility APIs returned success; tooltip/native rectangle observed. Click attempts did not produce a recorded `tray_input` | PASS API construction only; FAIL/UNRESOLVED end-to-end click delivery, no library defect attributed |
| Start second identical executable | Second process exits, first receives invocation | Second PID exited; first recorded second_instance | PASS one duplicate launch; burst/recovery untested |
| Registered Ctrl+Alt+Shift+F12; actual key-down/up | Shortcut handler opens/focuses main | shortcut_pressed recorded; foreground HWND matched main | PASS current shortcut; collisions/remapping not tested |
| Main focused, companion hidden; Tab then Space | Main UI remains usable with companion hidden | click counter 1 recorded; keyboard screenshot | PASS narrow keyboard interaction, not a complete accessibility audit |
| `size100`, `size150`, `size200` | Supplementary raster size samples | 128/192/256 physical HWND pixels, all at **96 DPI** | PASS size change only; these are NOT 100/150/200% DPI tests |
| `offscreen` at (-10000,-10000), `recover` | Exercise offscreen placement and explicit restoration | Both positions observed | PASS primitive only; automatic unplug recovery untested |
| Real 150/200% OS DPI and mixed-DPI main/companion migration | Correct physical/logical placement, readable inputs | Only one 100% display/session available; no isolated scale-changing session exercised | BLOCKED |
| Physical display removal | Both surfaces remain reachable | No second removable display available in this run | BLOCKED |
| Actual suspend/resume | Reachable surfaces, resumed rendering/shortcut/tray lifecycle | powercfg reports S3/hibernate support; no attended wake/resume harness available, computer was not suspended | BLOCKED; capability listing is not a resume test |

Evidence: native observations (`docs/evidence/probes/2026-10-10-n0/artifacts/native-observations.json` at `41b8e84`),
initial observations (`docs/evidence/probes/2026-10-10-n0/artifacts/native-observations-initial.json` at `41b8e84`),
supplemental observations (`docs/evidence/probes/2026-10-10-n0/artifacts/supplemental-observations.json` at `41b8e84`),
supplemental runtime (`docs/evidence/probes/2026-10-10-n0/artifacts/runtime-supplemental.jsonl` at `41b8e84`),
power states (`docs/evidence/probes/2026-10-10-n0/artifacts/power-states.txt` at `41b8e84`).
Screenshots: transparency (`docs/evidence/probes/2026-10-10-n0/artifacts/transparency.png` at `41b8e84`),
keyboard main (`docs/evidence/probes/2026-10-10-n0/artifacts/keyboard-main.png` at `41b8e84`),
128px (`docs/evidence/probes/2026-10-10-n0/artifacts/size100.png` at `41b8e84`),
192px (`docs/evidence/probes/2026-10-10-n0/artifacts/size150.png` at `41b8e84`),
256px (`docs/evidence/probes/2026-10-10-n0/artifacts/size200.png` at `41b8e84`).
Arabic/English text rendered in the synthetic main surface; full RTL layouts,
screen readers and production navigation were not tested.

## Build failures and limitations preserved

1. Initial build lacked the required Windows `icons/icon.ico`; the icon generator
   now produces a synthetic purple square. Initial log (`docs/evidence/probes/2026-10-10-n0/artifacts/build.log` at `41b8e84`).
2. Tauri 2.11.5 with broadly resolved runtime/runtime-wry 2.12.1 and macros 2.7.1
   failed with monitor signature and menu macro errors. Matching internal versions
   were constrained explicitly; no dependency source was patched.
   Failure (`docs/evidence/probes/2026-10-10-n0/artifacts/build-retry.log` at `41b8e84`),
   pinned success (`docs/evidence/probes/2026-10-10-n0/artifacts/build-pinned.log` at `41b8e84`).
3. The screenshot helper initially marshalled a null class as an empty string.
   Enumeration confirmed owned windows existed; `FindWindow(IntPtr.Zero,title)`
   repaired the helper. Failed-helper runtime (`docs/evidence/probes/2026-10-10-n0/artifacts/runtime-failed-helper.jsonl` at `41b8e84`).
4. One rebuild while the measured probe was alive failed with Windows error 5
   because its executable was locked. Waited for owned-probe exit before rebuilding;
   no other process was stopped. Locked executable log (`docs/evidence/probes/2026-10-10-n0/artifacts/build-optimized.log` at `41b8e84`).
5. Shell UI Automation initially did not expose the tray icon; a later tooltip
   match was insufficient. The final attempt used Tauri's native icon rectangle,
   but no callback was observed. Keep end-to-end tray activation unresolved.

## Decisions and exact provenance

| Area | Decision | Evidence / closing trigger |
| --- | --- | --- |
| Selected character direction | ADOPT simpler reference board | Explicit user choice; small-size comparison; preserve richer board |
| Generated v1/v2 as shipping assets | REJECT | Alignment, alpha padding and listening distinction failed |
| Production asset pipeline | UNRESOLVED | Obtain consistent source artwork/manual cleanup workflow, manifest/rights and approved three-state samples, then nine states |
| Browser raster family | ADOPT as next probe baseline only | Adequate simple state display without renderer dependency; user saw motion; production acceptance still gated |
| Final CSS versus canvas renderer | UNRESOLVED | No active CPU win; both fail budget; close after representative accepted art, release profiling and reviewer-approved budgets |
| PixiJS dependency now | REJECT for N0 | No layered animation need; no runtime evaluation or claim about its performance |
| Rive dependency now | UNRESOLVED; do not install for N1 by assumption | Author one original interaction with an available rig workflow before selection |
| Tauri 2.11.5 coherent graph | ADOPT tested native baseline | Locked builds and real native primitives; upgrade requires another compatibility trial |
| Official single-instance 2.4.5 / global-shortcut 2.3.2 | ADOPT tested API baseline | Actual duplicate launch and actual shortcut input passed; broader lifecycle checks remain |
| Tauri core tray | UNRESOLVED complete behavior | Construction works; resolve actual shell activation before N0 exit |
| Default show/hide and nonfocusable mitigation | REJECT as ordinary attention policy | Both steal focus |
| Explicit native rendering visibility | ADOPT requirement | document.hidden alone failed; full hidden bins stopped after explicit signaling |
| Position parking versus native no-activate show | UNRESOLVED | Both isolated focus probes passed; choose after full hidden/frame/monitor/suspend lifecycle, including main accessibility |
| Coucou implementation reuse | REJECT blanket fork; no executable code reused | Pinned inspected sources and notices retained solely as research snapshots |

Coucou revision: `08393b657c79e99382eaf15185f480e6745997b0`.
Inspected [desktop.rs](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/windows/src-tauri/src/desktop.rs)
defines transparent/nonfocusable windows, position parking, gated cursor polling,
body hit testing and display geometry. Its geometry tests and comments are source
evidence, **not** our Windows acceptance. The
[desktop lifecycle](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/windows/src/mochi/desktop-logic.ts)
and [island FSM](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/windows/src/island/fsm.ts)
were inspected for lifecycle/port separation. No character-dependent rules were
ported. The synthetic rectangular hit target and drivers were authored independently.

The source MIT notice (`docs/evidence/probes/2026-10-10-n0/provenance/LICENSE` at `41b8e84`) and
separate asset restrictions (`docs/evidence/probes/2026-10-10-n0/provenance/LICENSE-ASSETS.md` at `41b8e84`)
are retained intact with the source snapshots. MIT source permission does not
cover Coucou/Mochi branding, character, animations, artwork, icons or sounds.
No such media were downloaded or copied. Future code reuse needs exact per-file
review and applicable notices, not a blanket inference from this experiment.

Official plugin documentation was consulted for setup syntax:
[single-instance](https://v2.tauri.app/plugin/single-instance/) and
[global-shortcut](https://v2.tauri.app/plugin/global-shortcut/).
Runtime claims above rely on local trials, not those pages.

## Required specification/plan changes and N1 prerequisites

The owning spec and plan now record the selected reference, explicit visibility
requirement, rejected default focus behavior, pinned probe graph and OPEN exit.
They retain the whole Windows/mobile direction and do not create product modules.
The spec's earlier instruction to delete probes has been replaced: retain these
sources until the reviewer has inspected them and approved archival/removal.

Before N1: reviewer must close production asset authoring/provenance and expression
distinction; select/approve a performance baseline with release/presentation-timing
evidence; resolve full tray activation and the focus-preserving visibility method;
exercise real 150/200% and mixed DPI, display removal and actual suspend/resume.
Fast hit-test transitions, drag/click races, real reduced-motion preference and
screen-reader/RTL layouts remain additional native acceptance work. Turning,
directional views and the other six expressions need original consistent assets;
this probe does not imply they exist. No phone or voice requirement is passed.

## Checks and reviewer attention

Locked/offline native builds passed after graph correction, including the final
source build. `cargo check --locked --offline` and `cargo fmt --check` passed.
Asset metrics and summaries were run. `check-evidence.py` checks JSON/JSONL,
Python/embedded JavaScript syntax, trial hashes, 150 CSV samples and changed-document
local links. `git diff --check` checks tracked documentation whitespace. PowerShell
drivers were executed against the actual owned process. No npm typecheck/test/build
command exists because no product package was scaffolded; no fabricated test-suite
or release-build result is claimed. The native probe is debug-only, not a product
production build.

Reviewer attention: **N0 exit stays OPEN**, especially active CPU budgets, aligned
transparent assets, tray input, visibility/focus strategy and missing device trials.
Inspection/parse checks cannot close those gates. Final-source and documentation
check logs are retained in the artifacts directory; probe sources are retained.
