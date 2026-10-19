# Independent AYLUL desktop window

Date: 2026-10-16.
Status: approved; implemented in development/debug on 2026-10-17. Device and optimized performance limits are recorded in the dated acceptance report.
Owner: Native, with Character and Desktop UI integration.

## 1. Intent and existing decisions

The user wants the accepted procedural AYLUL character to live outside the
main application panel as a desktop companion. They approved the recommendation
to use a separate transparent window within the existing Tauri application and
instructed the implementer to proceed. The earlier Windows probes are evidence,
not a product implementation to copy or an obligation to retain their mechanism.

Success means the character remains usable after closing the main panel, can be
moved and deliberately hidden/restored, does not interrupt typing in another
application when restored passively, and stops animation work when hidden.
The main account panel remains usable if the character fails.

This extends the existing two-surface direction in the
delivery design (deleted 2026-10-19; Git `226d0a3`).
The [in-window specification](../../character/specs/2026-10-16-aylul-in-window-design.md)
still owns the extracted artwork and studio. This specification supersedes its
exclusion of a floating surface only for this newly authorized integration.
It does not declare the historical N0/N1b acceptance gates closed.

Work stays in the original checkout on `codex/aylul-companion-in-window`.
The user prohibits worktrees and previously selected inline implementation.
No branch switch, push, merge or deployment is included.

## 2. Approach selection

| Approach | Consequences | Decision |
| --- | --- | --- |
| A second local WebView window in the current Tauri application | Reuses the renderer, native lifetime, tray and account owner; allows separate capabilities and failures | Selected |
| A separate assistant executable | Duplicates startup, instance coordination, account handoff and packaging for the same user journey | Unnecessary for this slice |
| Reuse the experimental probe as product source | Retains synthetic control files, polling and experimental global state without product boundaries | Rejected; reuse observations only |

Use React/TypeScript for the surface and the current pinned Tauri/Rust stack for
native operations. A small Windows adapter owns any required no-activation API.
Prefer existing native APIs that satisfy the observed focus requirement; prove
the combined product behavior rather than selecting an API solely from an old
passing probe. Do not add another rendering engine, global mouse hook, or service.

## 3. Windows and user interaction

Retain `main` for account and application controls. Build `companion` from a
separate local HTML entry so loading the character cannot mount the account UI,
start an account request, or import the development studio. Production bundles
contain both required entries; only the explicit development studio entry may
contain studio code.

The companion starts hidden during construction. Its background is transparent,
with no title bar, native shadow, resizing, or taskbar entry; it stays above
ordinary application windows. Start at 240 by 240 logical pixels, preserving
the full character, crystal, hands and glow within that surface. Fit it to the
primary monitor work area with a margin near the lower right corner. Main startup
and its deliberate tray/shortcut/single-instance open behavior remain unchanged.
Show the companion after its local presentation has mounted successfully.

Closing main hides main only. Closing the companion hides that surface only.
The tray provides Open NYLRA, Show/Hide AYLUL, and Quit NYLRA. Main offers the same
explicit Show/Hide AYLUL control and motion pause control. Tray Show restores
AYLUL without activating main or changing the foreground application. Quit exits
the application, including both surfaces and any already-owned background work;
this slice adds no sidecar or cloud transport.

A short primary click on the character opens/focuses main as an explicit user
action. A primary drag crossing an 8 logical pixel threshold starts native
window dragging once and suppresses the corresponding click. Cancellation,
pointer loss and release reset the gesture. Ignore secondary-button gestures.
Do not chase the cursor or inspect other applications' windows.

The initial native hit area is the compact rectangular surface, including its
transparent padding. Transparency is not click-through. Main and tray must always
provide a keyboard-accessible hide/restore route. Exact silhouette hit testing
and automatic click-through are deferred; neither cursor polling nor unverified
alpha-shaped behavior is part of this slice. Record this limitation in acceptance.

Remember position in memory for the application lifetime. On restore and native
move/resize/display-related callbacks, validate placement against current work
areas and recover a stranded companion. Keep valid negative-monitor coordinates.
No preference-file persistence is added. Use native physical coordinates for
placement and logical coordinates for surface layout; account for DPI changes.
Hardware monitor removal remains unverified unless actually exercised.

## 4. Native ownership and presentation boundary

Native owns requested show/hide, effective render permission, window placement,
pause state, tray labels and the application lifetime. React owns drawing and
gesture presentation. The native state is a local UI projection, not a cloud
domain model, identity projection or approval state.

Agree the narrow interface in the
[shared request](../../shared/requests/2026-10-16-independent-companion-window.md)
before dependent source edits. Its proposed operations cover surface readiness
and state, companion visibility, pause, native drag and explicit main open.
Use typed wrappers in `src/platform/`; feature rendering never invokes raw IPC.

Each state update has a monotonic revision within the process. Subscribe before
requesting the initial snapshot and discard older revisions. Initial render
permission is false until synchronized. Hidden state changes must remain effective
even if a WebView missed an earlier event; querying current state recovers them.
Listeners and pending asynchronous subscriptions are cleaned up on unmount.

Extend `CompanionCanvas` with an explicit render-permission input, defaulting to
true for existing browser/studio consumers. Effective drawing requires native
permission, document visibility, intersection and nonzero viewport dimensions.
No permission means no scheduled animation or one-shot invalidation. User pause
and reduced motion stop continuous motion but permit a single current-state draw
when visible. Resuming uses the existing bounded simulation step; it does not
simulate all elapsed hidden time.

Signal render denial before native hiding. If hiding fails, reconcile against the
actual native state and report failure; do not report that the window is hidden.
For showing, restore the current presentation and permit rendering as part of the
native transition; do not call `set_focus` on the companion. A show error leaves
the controller conservative and recoverable through main/tray. Apply explicit
native permission to the retained main preview if it remains mounted during an
intermediate implementation step; the final main surface uses controls instead
of running a duplicate production character animation.

Reuse `CompanionCanvas` and the existing motion/renderer directly for the compact
surface. Keep its background transparent and place account, sound and studio
controls in main. The production character remains idle without a real owning
service presentation; no simulated listening, speaking or operational success.
Native errors are local plain text. Any visual fallback must fit the small
surface and retain the tray/main recovery route.

## 5. Capabilities and failure handling

Use separate window capabilities with only the exact event subscription and
native operations each surface needs. Keep the current self-only content policy
and local entry origins. Avoid broad default capability grants. Existing account
commands retain their main-label checks; companion cannot call account actions,
read credentials, cancel login, or use main-only hide operations. Each new command
validates its allowed calling window, including negative tests from the other
real surface. Model text never selects commands or changes native authority.

Failure to construct or initialize the companion must leave main and tray usable
with an honest error/status and an explicit retry route. Canvas failure suspends
the companion drawing loop and leaves main controls usable. Native drag and
visibility failures report stable public errors without secrets or OS internals.
No error silently quits the application or claims a successful operation.

## 6. Verification and acceptance

User amendment, 2026-10-17: use development/debug native builds while developing.
Release profiling below is deferred to an explicitly requested later step.
Debug rendering/lifecycle evidence does not establish optimized performance.

The implementation plan must assign exact file scopes, test names and commands
before code. Update inspected source contracts and the actual code map with the
corresponding implementation; create no placeholder source contracts now.

Focused automated checks must cover:

- Revision ordering, subscribe/snapshot races, cleanup after late subscription,
  conservative initial permission and hidden/restore transitions.
- Animation callbacks cancelled by native denial even when `document.hidden`
  remains false; no draw while hidden; paused/reduced-motion updates when restored.
- Click versus drag threshold, one drag per gesture and no panel open after drag.
- Synthetic placement at negative coordinates, undersized work areas and DPI
  changes; no assertion that these prove physical device transitions.
- Both production HTML entries, transparent surface CSS, no account mounting or
  studio modules in the companion's dependency graph, and studio exclusion from
  all production chunks.
- Explicit account-command rejection from companion, visibility-command caller
  restrictions and failure recovery without changing identity state.
- Existing account/hide UI behavior and independent main usability when companion
  initialization or Canvas rendering fails.

Build with `npm run build --prefix apps/desktop` and the pinned native release
graph. Run the affected companion tests and focused Rust tests; perform complete
affected suites at integration, without repeating passed checks unnecessarily.

Run the actual Windows release executable for separate device evidence:

1. Transparent character, full silhouette/glow, usable desktop placement, actual
   native drag, short click, close-main independence and tray restoration.
2. Repeated companion hide/show while another owned test window has foreground;
   its foreground HWND stays unchanged. Explicit character click may focus main.
3. Renderer frame activity stops while natively hidden even if the document
   reports visible, and resumes without catch-up or duplicate schedulers.
4. Actual tray Show/Hide/Open/Quit and shortcut behavior; negative native IPC from
   companion and recovery after a controlled local presentation failure.
5. Release profiling including WebView renderer processes, refresh rate, surface
   size, duration and CPU normalization. Use the existing provisional <=10% of one
   core active budget as a comparison; record failure rather than changing the
   budget to fit a result. Hidden renderer activity must be demonstrably stopped.

Real mixed-DPI transitions, display removal and sleep/resume remain explicitly
open if hardware or a safe test opportunity is unavailable. Browser screenshots,
mock IPC, synthetic rectangles and historical probes cannot close those gates.
Record exact source revision, commands, device context and results in a new dated
evidence report. Do not relabel the old probe results as product acceptance.

## 7. Review boundary and deferred work

The user approved the conversational approach and instructed work to begin.
The user subsequently approved this written specification with an instruction
to begin. Prepare the implementation plan and its stage tasks before product
edits. Preserve the previously requested inline execution method and
no-worktree constraint; do not ask the user to choose these again.

| Deferred decision | Owner and closing trigger |
| --- | --- |
| Silhouette click-through | Native + Character; separately agree on a non-disruptive hit-test strategy and prove fast pointer crossings and drag races |
| Cross-restart position/preferences | Native + Desktop UI; add only with an explicit preference lifecycle and restore tests |
| Real operational expressions/messages | Owning service + Character; connect inspected service outcomes through an accepted interface |
| Physical lifecycle gates not exercised here | Native evidence report; close only with the named hardware/device observations |
