# NYLRA Initial Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans for inline execution,
> or superpowers:subagent-driven-development when delegation is explicitly selected.

**Goal:** Deliver AYLUL on Windows connected to real Shadows through the cloud,
then complete voice, mobile continuity and operational incident experiences.
**Architecture:** React/Tauri desktop and later Flutter mobile; cloud dialogue and
project authority remain in their existing owners.
**Tech Stack:** React/TypeScript/Vite/Tauri 2, renderer decided by N0; Flutter later.
**Spec:** [NYLRA delivery](../../specs/2026-10-10-nylra-delivery-design.md).
**Status:** N0 experiments run on 2026-10-10 and 2026-10-11. Full N0 exit
OPEN. `codex/windows-shell-n0` merged as PR #1; historical probe sources are at `41b8e84`. The user
authorized local commits on 2026-10-11, but no push or deployment.
N1a main shell/local native primitives are implemented; joint login and N1b remain
unaccepted. See [continuation evidence](../../../evidence/nylra/2026-10-11-windows-shell-continuation.md).

## Global constraints

- N0 experiments decide libraries/assets; revise this plan before dependent work.
- Preserve original PNGs; create original production assets with explicit provenance.
- Do not copy Coucou's protected character, sounds or artwork.
- Do not make client state authoritative for project decisions or AWS operations.
- Native device evidence is distinct from browser previews and fake protocol tests.
- Paths below are planned; no placeholder source/contract folders are required.

## Review focus

1. Mixed DPI or unplugged monitor must not strand an invisible window (N1).
2. A hidden companion must not keep unnecessary frame work running (N0/N1).
3. Account switch must clear the previous account's evidence and input (N2).
4. A late stream response must not overwrite the newly selected topic (N2).
5. A changed proposal must not reuse an earlier confirmation (N3).

## Coordination

### Separate capability and identity acceptance

| Gate | Owner | Evidence required / dependency |
| --- | --- | --- |
| Windows capability acceptance | NYLRA N0/N1 | Actual native windows, foreground/tray actions, release CPU/memory, visibility, supported device scenarios and inspected assets. No Linux identity prerequisite for these local trials. |
| Linux backend identity acceptance | shadows-backend B0/B1 | OIDC/provider checks, backend-owned native-return contract and fixtures, persisted account/device/session validation and revocation. Windows hardware does not block backend acceptance. |
| Joint end-to-end login | NYLRA + shadows-backend | Both runnable sides; actual system browser, backend-mediated ticket return, native verifier redemption, protected persistence, restart validation and logout/revocation. Fixture evidence cannot close this gate. |

Native plumbing may be tested independently with an explicitly synthetic fixture.
The backend currently offers proposed service interfaces in its delivery spec,
not a versioned native-login HTTP schema or matching conformance fixtures. Do not
invent HTTP paths, error enums, ticket query rules or backend identity policy.
Pin and consume those owner artifacts when published; until then no login adapter
is enabled. The existing B0 public-client OIDC fixture is not the proposed B1
backend-mediated protocol.

### N1 dependency split for this authorized slice

N1a is the keyboard-accessible React/Tauri main shell, native tray/window lifecycle
and local browser/listener/verifier/protected-store plumbing. These do not depend
on shipping character art or a running Linux backend. Implement and verify them
without claiming a signed-in session from a stored handle.

N1b is the animated transparent companion. Failed alpha/alignment/state-readability
and performance prerequisites must close before this dependent implementation.
Missing real mixed-DPI/removal/suspend cases remain explicitly unaccepted; they
must not be disguised as passing by synthetic geometry or messages. N1a evidence
does not make N1b or the full Windows gate complete.

Implemented N1a uses React 19.3.0/React DOM 19.3.0, Vite 8.3.4, TypeScript 7.0.2,
frontend Tauri API 2.11.0 and the previously tested coherent native graph. Native
return/verifier/DPAPI primitives have fixture evidence only. Sign-in stays disabled
pending backend wire artifacts. Main tray activation and keyboard hide were tested.

Concrete continuation status:

- [x] Local React/Tauri main shell, close/hide/shortcut and real tray/menu actions.
- [x] Client verifier, loopback listener, reviewed-origin browser launcher and DPAPI primitives with fixture-based process restart/local clear evidence.
- [x] Fix DPI-expanded main-window size; actual 150% and 175% final-source trials.
- [x] Consume auth-v1 1.1.0 fixtures in Rust; wire session checks, enrollment reuse, cancellation and logout; verify native consumer against local Docker and a fresh process.
- [ ] Complete full system-browser/app UI login, restart, cancellation and logout acceptance with browser trust configured correctly. See joint native evidence; automated form-driver acceptance alone does not close this gate.
- [ ] Accept assets/motion and implement N1b; complete unavailable mixed-DPI/removal and 200% cases when hardware/settings permit.

[Backend plan](../../../../shadows-backend/docs/superpowers/plans/2026-10-10-backend-initial-plan.md)
owns J1 wire artifacts and names J0-J3. N0/B0 can proceed independently. N1 is a
local shell; N2 waits for J1/B3 and real Shadows context for J2 acceptance. N3 waits
for B4 and actual Shadows §24.6 enforcement before J3 acceptance. Never hide that
dependency by making NYLRA approve locally.

### N0: AYLUL and native capability experiments

**Inputs:** root reference-board and design-sheet PNGs.
**Output:** `docs/evidence/nylra/<run-date>-aylul-windows-probes.md`; amend this plan/spec.
**Temporary artifacts:** isolated asset/render/native trial, not product scaffolding.
**Evidence:** [Windows probe report](../../../evidence/nylra/2026-10-10-aylul-windows-probes.md).
The [dated follow-up](../../../evidence/nylra/2026-10-11-aylul-windows-probes.md) corrects
historical comparison conditions and records matched-window trials at 144Hz.
**Retention:** keep probe source (`docs/evidence/nylra/probes/2026-10-10-n0/README.md` at `41b8e84`)
until reviewer inspection; no deletion of the only reproduction.

- [x] Preserve originals and compare the two silhouettes at 64/96/128 logical pixels.
- [ ] Prepare representative original transparent idle/listening/blocked states
  with equal pivot/scale; inspect alpha edges on light and dark backgrounds.
- [ ] Compare browser raster animation with PixiJS only if layered animation needs
  it; evaluate one Rive interaction if a rig remains a credible alternative.
- [ ] Measure idle/active/hidden CPU, frame-time and memory on the same machine;
  establish acceptance budgets in the evidence before accepting the renderer.
- [ ] Prove transparent Tauri window, drag, restricted click-through, tray, single
  instance, shortcut, focus and suspend/resume. Test 100/150/200 percent DPI and
  display removal with main/companion surfaces.
- [x] Review exact Coucou source/license before any reuse; record borrowed code
  provenance/notices separately from original AYLUL art. No blanket fork.
- [ ] Select dependencies and asset pipeline from evidence, pin versions, replace
  later task assumptions and record any unavailable physical-device scenario.

**Exit:** accepted asset/render/native approach and revised tasks. A static board
preview cannot close animation or native-window acceptance.

**Observed status:** user selected the simpler reference board. Two original
transparent asset trials were produced and inspected but failed alignment/padding
and expression distinction; keep the asset checklist open. Canvas and CSS raster
trials have 150 retained CPU/memory samples and rAF cadence measurements, but both
exceed the provisional active CPU budget; final renderer acceptance is open.
No layered requirement justified PixiJS; no authored Rive interaction was available.

Continuation: reject retained assets as production inputs. The capped Release
canvas run passed provisional CPU/memory/hidden-work thresholds but failed the
original 20ms cadence budget. Renderer acceptance stays OPEN. Paired release focus
and a real sleep/resume passed on this device; 150% cases have separate evidence.
No second monitor is available now; mixed DPI/removal remain UNTESTED.

New [asset continuation](../../../evidence/nylra/2026-10-11-aylul-asset-continuation.md):
user prefers idle-v3; its alpha-only derivative now passes outer padding while
preserving RGB. Listening-v5 and blocked-v4 have a browser preview and passing
padding diagnostics. Mohammed subsequently accepted their shown appearance and
lighting variation; canonical sources are pinned in assets/aylul/states/v1.
The [six remaining expression candidates](../../../evidence/nylra/2026-10-12-approved-aylul-and-six-expressions.md)
are prepared from this master and await separate user review. Native animation
acceptance remains open; appearance approval
does not imply runtime/event acceptance. Earlier raw v1/v2/v3 remain historical.

Adopt the tested native baseline pins recorded in the spec/report: Tauri 2.11.5
with its coherent locked internal graph, official single-instance 2.4.5 and
global-shortcut 2.3.2. This is capability evidence, not a production dependency
approval. Explicit native animation visibility is required; default show/hide and
the nonfocusable mitigation stole focus. Parking/no-activate show remain alternatives
until complete device lifecycle proof. Actual tray Left Down/Up delivery and an
immediate paired no-activate visibility trial passed in the follow-up; tray menu
activation, actual 150/200% and mixed DPI, display removal and suspend/resume remain
unresolved. No broad native checklist
item is marked complete from partial trials.

**Closing trigger before N1b:** reviewer accepts consistent/provenanced assets,
release performance and budgets, full tray activation and one focus-preserving
visibility method with the missing device scenarios exercised. Turning and other
expressions remain asset work; no N1 source was scaffolded by these experiments.
The subsequent continuation implements independent N1a only; N1b remains gated.

### N1: Working Windows companion shell

**Intended files:** `apps/desktop/package.json`, `apps/desktop/src/app/`,
`apps/desktop/src/features/companion/`, `apps/desktop/src/platform/`,
`apps/desktop/src-tauri/src/{lib,main}.rs`, native `windows/` and `commands/`,
`apps/desktop/src-tauri/capabilities/`, `assets/aylul/manifest.json`,
`apps/desktop/tests/companion-lifecycle.test.ts`.
**Consumes:** reviewer-accepted N0 choices; currently blocked by the open N0 exit.
This consumption applies to N1b. N1a consumes only inspected native primitives and
the owner's proposed native return location/verifier responsibilities; its transport
adapter waits for the versioned backend contract. No nine-state pack is required
or claimed by a main-only shell.
**Produces:** companion/main surfaces and pure visual state transitions; fix exact
interface names in the reviewed N1 design after N0 choices close. Probe controls
are not product interfaces. Use the selected reference board, explicit rendering
visibility and a proven focus policy; do not inherit an untested renderer or tray
activation method from the probe.

- [ ] Write failures for hidden animation, rapid drag/click, missing display,
  stale visual event, keyboard navigation and reduced motion.
- [ ] Implement composition and the minimal native/window wrappers using selected
  official plugins first. Scope capabilities per surface.
- [ ] Produce the nine original states with a manifest; missing assets fail visibly
  or fall back to an explicit neutral state, never an unrelated success animation.
- [ ] Verify no focus stealing during ordinary notifications; main panel remains
  keyboard-accessible without the companion.
- [ ] Run Windows acceptance, source-contract checks and scoped tests; record and commit.

### N2: Real topic conversation and sources

**Intended files:** `apps/desktop/src/features/{connections,conversations,sources}/`,
`apps/desktop/src/data/`, `apps/desktop/src/api/generated/`,
`apps/desktop/src-tauri/src/local_shadows/`,
`apps/desktop/tests/topic-reconnect.test.ts`, `protocol-fixtures/`.
**Consumes:** J1 and B1-B3. **Produces:** J2 integrated Windows journey.

- [ ] Generate the client from pinned J1 artifacts; consume shared conformance
  examples without importing backend source.
- [ ] Test topic/account switches during a stream, duplicate/out-of-order events,
  revoked links, cursor expiry, dropped connection and restored cached sources.
- [ ] Implement selected-project navigation, durable topic view, source/version
  display and explicit offline/freshness state. Decide cache/query libraries from
  these tests rather than adding an unused global store.
- [ ] Keep local Shadows navigation separate from cloud chat; verify closing NYLRA
  leaves the daemon-owned connector alive.
- [ ] Run direct Planner conversation -> NYLRA explanation with actual sources,
  provider stream, restart and recovery. Record J2 only after the real path works.
- [ ] Update contracts/evidence and commit this complete slice.

### N3: Assessment and trusted confirmation

**Intended files:** `apps/desktop/src/features/confirmations/`,
`apps/desktop/tests/confirmation-basis.test.ts`.
**Consumes:** B4 and Shadows §24.6. **Produces:** J3 user-confirmed owner outcome.

- [ ] Test changed proposal while the panel is open, expired/revoked authority,
  repeated clicks, two devices and model text claiming that an action succeeded.
- [ ] Render trusted operation/proposal basis outside generated response markup;
  bind explicit user action to that exact basis using J1.
- [ ] Display queued, delivered, accepted, rejected and completed according to
  owner evidence. Assessment requests never silently become adoption requests.
- [ ] Verify real owner records and refused stale submissions; update contracts,
  record the native trial and commit.

### N4: Voice and attention

**Intended files:** `apps/desktop/src/platform/audio/`,
`apps/desktop/src/features/conversations/`, native `notifications/` as needed.
**Consumes:** B5, N2. **Produces:** cancellable voice with text fallback.

- [ ] First probe actual microphone/playback/provider behavior and choose maintained
  libraries against permission denial, interruption and device changes.
- [ ] Amend this task with selected library/version and exact commands before code.
- [ ] Test topic switch during speech, muted output, denied capture, sleep/resume,
  duplicate notifications and quiet hours. No stale playback after cancellation.
- [ ] Map listening to active capture and speaking to playback; never infer human
  confirmation from a transcript. Run actual Windows voice acceptance and commit.

## Later slices retained in the full plan

| Slice | Prerequisite | Intended paths | Acceptance exit |
| --- | --- | --- | --- |
| N5 Mobile capability proof | Windows J2, B6 protocol readiness | `apps/mobile/`, evidence | Select physical Android/iOS targets, prove login, stream/reconnect, notifications and audio |
| N6 Mobile product journey | N5 selections, J3 for confirmation | Flutter `lib/ui/`, `lib/data/`, `integration_test/` | Same topic across desktop/phone, scoped cache, stale refusal, real lock-screen/background limits |
| N7 Production companion | Backend B8/B9 and Shadows §17.12 | New incident feature only with first user | Sourced pressure alert, remedy explanation, confirmed owner outcome and measured result |
| N8 Team and richer presence | Rust team owner, B7 | Existing connection/source views plus required feature units | Workspace isolation, multiple incidents, original accessories, discoverable attention controls |

Advanced memory, more channels, non-Shadows integrations and enterprise use stay
in Vision. Expand later slices only after their preceding evidence exists, keeping
the full direction without speculative module scaffolding.

## Verification and amendment

N0 records the exact package-manager commands for typecheck, test, production build
and native checks for selected versions. Add native Windows acceptance steps to
the task they prove. UI tests alone do not establish tray/audio/background behavior.
Every behavior task follows failing test -> implementation -> passing checks ->
contract/evidence update -> scoped commit. Artifact-only steps use visual inspection
and manifest validation rather than artificial implementation-mirroring tests.

A failed probe updates the spec, dependency decision and affected tasks in both
plans. Maintain one chosen path, not a parallel rejected renderer or client engine.
No implementation or successful capability trial is claimed by writing this plan.
