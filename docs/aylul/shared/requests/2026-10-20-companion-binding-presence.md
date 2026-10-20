# Selected agent companion binding and navigation

- Status: accepted
- Asked by: Desktop UI and Character scopes, 2026-10-20
- Owner: Native scope
- Serves: docs/aylul/specs/2026-10-20-aylul-desktop-web-companion-design.md

## Need

The character follows one explicitly selected changeable agent, independently of
the viewed route. Native owns persisted binding, validation and pending navigation.

## Interface

| Call / event | Input | Output |
|---|---|---|
| companion_binding | Main only | BindingView |
| select_companion_agent | agent slug or null; main only | Validated BindingView |
| target_companion_thread | threadId, expectedGeneration; main only | Validated BindingView |
| set_companion_previews | enabled; main only | BindingView |
| companion-binding | Native to main only | Latest revisioned BindingView |
| navigation_snapshot | Main only | PendingNavigation or null |
| ack_navigation | revision; main only | Clear only the matching pending revision |
| main-navigation | Native to main only | PendingNavigation |
| open_main_from_companion | Existing companion-only command | Explicit main open at validated closed destination |

BindingView uses camelCase: revision, generation, configurationId, agent, target,
previews. Target contains projectId and threadId. PendingNavigation contains
revision and destination, tagged by kind: companionSettings, agentDraft with agent,
or conversation with projectId/threadId. No arbitrary URL or raw settings path is
accepted. Native serializes the canonical JS fixture in web/src/test/native-presence.json.

Selecting a changed agent opts into showing the character without activation.
A later visibility intent wins even while selection validation is in flight.
Configuration change clears the old binding before publishing readiness.
Sending and the Task 7 watcher use this generation-scoped owner; viewing alone
does nothing. Background presentation extends the existing companion-state event
under spec section 8; it never grants render permission or conversation authority.

## Errors

Fixed codes only: companion_agent_unavailable, companion_thread_invalid,
companion_binding_changed, companion_core_unavailable/changed, and private
preference read/write errors. Raw HTTP bodies, secret values and conversation
content never enter these preferences or error strings.

## Answer

Accepted by the controller from Mohammed's approved specification and execution
instruction. No separate lead message is claimed. Native implementation precedes
UI consumption; the background watcher remains Task 7.

## Reply

UI consumes native revisions and ignores stale results. Character retains its
existing visibility, pause and rendering-recovery boundary.

## Background projection implementation

Task 7 extends CompanionState with presence: expression, status, preview.
Preview is null or {id, text, expiresAtMs}; the Rust-serialized fixture includes
both empty and active projections. It contains only sanitized presentation text,
never full history, raw errors or tool/user content. The six expressions are idle,
thinking, working, speaking, success and blocked; no microphone state is implied.
Render permission remains separately native-owned.

The native feed uses only runtime identity, selected agent/provider metadata,
target thread, operations, entries and subscribe read routes. Snapshot/liveness
work has a 10-second deadline, JSON a 16 MiB bound and events a 1 MiB bound.
A healthy SSE body has no total timeout. Replay/caught-up and durable sequence
govern live completion; transient turn-end cannot declare success. Metadata
liveness runs every 10 seconds. Ten failures without caught-up require explicit
Local Core Retry. Completion recovery makes at most five bounded entry snapshots.
Native timers clear a preview after eight seconds even when main is hidden.

Task 8: target_companion_thread acknowledges the scoped observer's snapshot and
caught-up for at most two seconds before returning. The ordinary send helper
catches that failure independently; it cannot cancel, duplicate or prevent
the person's command. This closes the fast-first-turn gap without celebrating
historical replies. A deliberate Retry companion connection uses existing
retry_desktop_connection, restarting reads against the verified runtime.
