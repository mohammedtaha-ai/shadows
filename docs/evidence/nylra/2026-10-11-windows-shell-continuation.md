# Windows shell and N0 continuation

Probe sources and raw artifacts named here were removed from the tree on
2026-10-13; `git show 41b8e84:<path>` retrieves any of them.

Date: **2026-10-11, Asia/Aden**. Started on `main` at
`94534bbfdeb141564b0f603bb7702422e6af663a`, with modified plan/spec and untracked
October 10/11 probe evidence. Continued on `codex/windows-shell-n0` in the existing
checkout. Existing evidence and work were preserved. The user subsequently
authorized local commits and confirmed his intentional deletion of the alternative
AYLUL design sheet. No push, deployment, AWS, phone or N2 conversation work.

## Acceptance ownership

| Gate | Owner | Current disposition |
| --- | --- | --- |
| Windows capabilities | NYLRA | Main shell and bounded native experiments; full animated-companion acceptance remains OPEN |
| Linux backend identity | shadows-backend | Independently owned B0/B1 evidence, not re-run here |
| Joint real login | Both | PENDING published owner wire contract, fixtures and runnable backend, then actual integrated login |

The [updated plan](../superpowers/plans/2026-10-10-nylra-initial-plan.md) separates
N1a main-shell/local native plumbing from N1b animated companion prerequisites.
It does not accept failed assets or silently close the aggregate N0 gate.

## AYLUL asset decision

The selected root `AYLUL Companion Character Reference Board.png` remains intact:
SHA-256 `a1ef31bc867e3905a5f40092032afc3c817dfe82c651060835fcf5f312785717`.
Only this board defines current art direction. The owner deleted the alternative
root sheet to avoid reference confusion; the historical original remains in Git.
Historical board comparison artifacts remain labelled evidence, not active assets.

Re-inspected the selected board, three-state v2 atlas and idle-v3 visually; read-only
alpha/geometry inspection retained at asset-readiness.json (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/asset-readiness.json` at `41b8e84`).
The atlas still has unequal body alignment and nonzero alpha touching top/bottom
edges (v2: 93/121/95 nonzero edge positions). Idle/listening eyes remain similar;
the blocked eyes differ. Idle-v3 retains the failed preparation recorded in the
previous report. **REJECT all existing generated trials as production assets.**
No original pixels were changed, no substitute character was shipped, and no fourth
generation was attempted after the three retained inadequate outputs.

Closing trigger: a consistent padded transparent three-state pack with authored
listening distinction, light/dark 64/96/128px inspection and owner provenance review.
Nine states and turning remain dependent asset work. The shell explicitly states
that the animated companion is unavailable.

## Release performance decision

Specific reason for new measurements: prior matched debug runs exceeded active CPU
at 144Hz; the retained canvas tick drew every display callback. The new regression
test first failed with **144 draws/second**, then passed after scheduling draws
through a cancellable timeout with a 30fps upper limit. Native hide cancels both
timeout and rAF. This results in approximately 24fps here because timeout and
display scheduling both contribute; it is an upper limit, not a promise of 30fps.

One locked/offline **Release** canvas run at 144Hz / scale 1.0, matched visible
640x480 main + 128x128 companion, five samples per stage:

| Stage | Mean CPU, one core | Maximum aggregate working set MiB |
| --- | ---: | ---: |
| Idle | 0.716 | 409.02 |
| Active | 6.944 | 422.27 |
| Hidden | 1.071 | 422.89 |
| Reshown | 8.125 | 421.03 |
| Reduced | 0.357 | 421.20 |

CPU/working-set pass the retained provisional thresholds: idle <=1%, active <=10%,
hidden <=idle+0.5 percentage points, aggregate working set <=512MiB. Stable hidden
samples have zero callbacks; the first boundary sample retains work performed
before hide and is not claimed as post-hide work. No battery/GPU-present-time result.
CPU is relative to one logical core; working set can double-count shared pages.

The original p95 callback interval <=20ms budget is **NOT passed**: capped active
samples have p95 around 48.8ms. Do not silently change it to accept the renderer.
Renderer adoption remains OPEN pending a reviewed motion/cadence target and actual
visual timing evidence. CPU improvement is bounded to this release run; no controlled
release CSS comparison or general renderer superiority is claimed.

Artifacts: CSV (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/performance-canvas.csv` at `41b8e84`),
summary (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/summary.json` at `41b8e84`),
runtime (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/runtime-canvas.jsonl` at `41b8e84`),
build log (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/build.log` at `41b8e84`).
Earlier matched passes were not repeated. CSS, old drag/hit testing and duplicate
launch acceptance were not re-run without a reason.

## Native focus, lifecycle and available hardware

Release hide/show-noactivate with explicit renderer visibility passed **20 paired
cycles at 100%**: foreground remained the main window; companion HWND remained
available. Observations (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/focus-observations.json` at `41b8e84`).
This extends the single earlier paired probe; it does not prove every application,
notification policy or mixed-DPI configuration.

With the user's permission, a real Windows Sleep attempt used SetSuspendState
after enabling SeShutdownPrivilege. Call succeeded with error 0. Kernel-Power
events **42 and 107** were recorded; the call returned after 19.87 seconds. Same
companion HWND **69734**, position (760,200), 128x128 and DPI 96 before/after;
owned process alive and active samples continued after return. The user confirmed:
"I woke it and the face was still present." Accept this single current-device
suspend/resume scenario; no hibernate, repeated sleep or hidden-during-sleep claim.
Observation (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/suspend-observations.json` at `41b8e84`),
runtime (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/runtime-suspend.jsonl` at `41b8e84`).
Kernel event timestamps alone do not establish full sleep duration; native elapsed
time and the user's wake observation supplement them.

Current hardware exposes one 1920x1080 GTX 1070 display. The user clarified that
there is **no second monitor available now**. Mixed DPI, physical removal, and
transfer between displays remain UNTESTED for that precise reason. Synthetic
placement tests prove rectangle recovery logic only. Modern Standby S0, S1 and S2
are unavailable according to the retained powercfg inventory; S3 and hibernate are
available, but hibernate was not tested. Real single-monitor 150% testing is recorded
below after the user changed Windows Scale. The user reports that the exposed
Scale choices stop at **175%**; 200% is unavailable in the current standard options.
No custom scaling override was requested or set. Its acceptance remains UNTESTED.

**Actual 150% evidence:** after the user changed Windows Scale, native
GetDpiForWindow returned **144**. Initial shell snapshot was 1102x1016 at (38,38)
and extended behind the taskbar. A failing geometry regression test reproduced
the physical-size problem. `fit_inner` now subtracts native decoration size and
margins from the current monitor's physical work area; startup also calls the
same explicit-open recovery path. Final Release at 150% measured **1102x944 at
(228,64)**, entirely within work area (0,0)-(1920,1008). Driver asserts that
containment, actual tray activation, ten hide/restore cycles and keyboard hide.
Final observations (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-dpi150-final/shell-observations.json` at `41b8e84`),
screenshot (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-dpi150-final/shell-main.png` at `41b8e84`).
Twenty release probe paired no-activate cycles also passed at **144 DPI**;
observations (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-dpi150/focus-observations.json` at `41b8e84`).
This is actual single-monitor DPI, not resized 96-DPI evidence and not a transition
between simultaneous mixed-DPI displays. The pre-fix screenshot/observations and
size regression failure are retained separately.

**Actual 175% evidence:** Windows native DPI returned **168**. Final shell
measured 1284x932 at (176,0) in work area (0,0)-(1920,996). Native work-area
containment, actual tray activation, ten close/hide/shortcut cycles and Tab/Space
hide passed. Observations (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-dpi175/shell-observations.json` at `41b8e84`),
screenshot (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-dpi175/shell-main.png` at `41b8e84`).
This additional available setting does not replace the unavailable 200% case.

## Implemented Windows slice

React/TypeScript main shell and Rust/Tauri integration, pinned frontend dependencies
and Cargo graph, minimal main capability and CSP. Close hides to tray; an explicit
tray left click, shortcut, second-launch callback or Open menu restores/focuses it.
The shell has a keyboard Hide to tray button with a local error alert. Physical
work-area recovery on explicit open handles an inaccessible title bar, including
a one-pixel monitor sliver; hardware-removal acceptance remains pending.

There is no product companion, notification background show, conversation, provider
SDK or Shadows connector. The synthetic app icon is not AYLUL artwork. Behavior
owners and colocated source contracts are mapped in [the code map](../codebase/README.md).

Actual Release shell at **96 DPI** passed owned tray icon left-click at (1606,836),
restoring visible foreground; ten close/hide/shortcut cycles; keyboard Tab+Space
hide through React/platform/IPC. Main-process-only idle was 0% CPU and 26.88MiB;
this excludes WebView2 descendants and is not another aggregate renderer test.
Observations (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/shell-observations.json` at `41b8e84`),
native screenshot (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/shell-main.png` at `41b8e84`).

The first tray driver could not locate overflow because the actual Button name was
`Show Hidden Icons`; exact-case correction and the owned `NYLRA` Button produced
the actual click above. The earlier failed observation is retained. One build
failed with access denied when the owned test app locked its EXE; it was rebuilt
after that test exited. Failed build log (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/shell-build-locked.log` at `41b8e84`)
and earlier shell evidence (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/shell-before-ipc.json` at `41b8e84`)
remain explicit, rather than mixed into final-source acceptance.
The subsequent DPI size fix changes the executable hash; final 150% observations
identify the latest tested binary. Earlier 100% observations are attributed to
their preceding binary rather than silently described as final-source acceptance.
One menu test overlapped the probe's registered shortcut and could not start the
shell normally; those UI runs must be sequential. Menu keyboard selection and
owned-item availability failures are kept as failed evidence, not menu acceptance.
The final focused driver locates the actual visible MenuItem by its exact owned
name and clicks its native rectangle. **Open NYLRA restored/focused the main HWND
22024398; Quit NYLRA exited the owned process within five seconds. Both PASS.**
Menu observations (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/tray-menu-observations.json` at `41b8e84`).
This closes actual native menu activation on the current single-display setup;
prior keyboard-selection failures do not contradict the corrected owned-item trial.

## Native authentication plumbing: fixture-based only

Read the backend architecture, source-contract convention and B1 identity design.
The available design proposes backend-mediated OIDC with native S256, IPv4 loopback
and `/oidc-return`; it does not yet publish a versioned native-login HTTP contract
or matching B1 fixtures. Earlier B0 direct-provider public-client fixtures differ
from that proposed flow and are not adopted as NYLRA authentication.

Implemented native-only primitives: BCryptGenRandom verifier, S256 challenge,
ephemeral IPv4 loopback listener with fixed path and exact Host validation,
bounded header/read/deadline and cancellation, no-store/no-referrer return response,
reviewed HTTPS-origin browser launcher, user-scoped DPAPI, atomic protected-session
replacement, bounded load and idempotent local clear. Neither callback/query nor
verifier/session material is registered in webview IPC, logged or serializable.
No account identity is trusted from the callback; no backend identity rules copied.

**Fixture results:** real default system browser reached the owned native loopback
listener. Synthetic opaque material was saved, loaded by a fresh native process,
cleared by another process, then confirmed absent by a further process.
Fixture log (`docs/evidence/probes/2026-10-10-n0/artifacts/2026-10-11-release/native-fixture.log` at `41b8e84`).
The browser fixture intentionally uses its owned HTTP loopback test utility; the
production launcher rejects HTTP and non-reviewed origins. This does not test
backend-mediated identity, ticket redemption, authenticated restart or remote logout.

Unit tests exercise real TCP and DPAPI rather than simulated OS encryption:
client binding, foreign/duplicate Host, method/path/fragment refusal, listener
cancellation, protected round-trip/tamper refusal, replacement/corruption, local
clear and browser refusal. Initial four tests failed on missing behavior before
implementation. Geometry's recovery case also failed before its implementation.

Backend dependencies and closing triggers: export/pin native-login request/return,
session-validation and logout schemas plus refusal fixtures and reviewed deployment
origin; implement the consuming native adapter; exercise real joint browser login,
fresh-process validation, expired/revoked sessions and local+remote logout. A stored
handle alone never creates a signed-in UI; local deletion never proves revocation.
The sign-in button remains disabled. In-memory material has no explicit zeroization
yet, recorded in the inspected contract. No end-to-end login acceptance is claimed.

## Commands and verification

Executed from `E:\Globalprojects\nylra` (retained outputs under `2026-10-11-release`):

```powershell
node --test docs/evidence/probes/2026-10-10-n0/animation.test.cjs
cargo build --release --locked --offline --manifest-path docs/evidence/probes/2026-10-10-n0/native/Cargo.toml --target-dir docs/evidence/probes/2026-10-10-n0/target
& docs/evidence/probes/2026-10-10-n0/controlled-renderers.ps1 -Renderers canvas -Profile release -RunName 2026-10-11-release -ShowCommand show-noactivate
& docs/evidence/probes/2026-10-10-n0/release-focus.ps1
& docs/evidence/probes/2026-10-10-n0/real-suspend.ps1
npm install --prefix apps/desktop --no-audit --no-fund
npm run build --prefix apps/desktop
cargo test --locked --offline --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo build --release --locked --offline --manifest-path apps/desktop/src-tauri/Cargo.toml --target-dir docs/evidence/probes/2026-10-10-n0/target --bin nylra-desktop --example native_fixture
& apps/desktop/tests/native-fixture.ps1
& apps/desktop/tests/windows-shell.ps1
& apps/desktop/tests/windows-shell.ps1 -RunName 2026-10-11-dpi150-final
& apps/desktop/tests/windows-shell.ps1 -RunName 2026-10-11-dpi175
& docs/evidence/probes/2026-10-10-n0/release-focus.ps1 -RunName 2026-10-11-dpi150
& apps/desktop/tests/tray-menu.ps1
```

Read-only asset inspection and retained evidence syntax/hash/sample/link checks
used bundled Python, not an inferred historic test pass. `npm run build` checks
TypeScript and Vite. Source contracts explicitly distinguish missing hardware and
backend evidence. No remote CI, installer-signing, clean-machine distribution,
voice, battery or GPU-present-time acceptance was run.
The native suite passed **11 tests**, with zero failures; three scheduler tests
passed. The added DPI regression first failed, then passed with the complete suite.
Final verification also passed Rust formatting, six YAML contract/source paths,
current report/plan/spec links, PowerShell driver syntax, retained JSON/JSONL,
final 150/175% outcome assertions, 40 paired focus-cycle assertions and final
Open/Quit outcome assertions. Failed diagnostic observations are retained and
excluded from accepted runs. Existing 200 historic samples were checked for syntax
and completeness, not re-run. The new release run has 25 samples.

Verification-only YAML 2.8.1 was installed under git-ignored
`.superpowers/validation`, not in desktop product dependencies. Checker:
`node docs/evidence/probes/2026-10-10-n0/check-continuation.cjs`.
No native test app was left running after the drivers completed. Windows Scale
was changed by the user, not programmatically; he was asked to restore 100%
after 175%. Restoration was not verified and is not claimed.

Primary references inspected for native implementation:
[Tauri tray](https://v2.tauri.app/learn/system-tray/),
[Microsoft DPAPI](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata),
[SetSuspendState](https://learn.microsoft.com/en-us/windows/win32/api/powrprof/nf-powrprof-setsuspendstate),
[RFC 8252](https://www.rfc-editor.org/rfc/rfc8252).

**Full N0/N1 animated-companion and joint-login acceptance remain OPEN.** The
implemented independent shell and local plumbing do not close those dependent gates.
