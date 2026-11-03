# Planner context: how a compact is reported, and what can be measured: 2026-11-04

Dated facts for the Planner context design
(`docs/backend/specs/2026-11-04-planner-context-design.md`), which owns every decision.

## 1. A compact is reported, manual or automatic

Read, not run, from the adapter Shadows ships:
`harness/claude/node_modules/@agentclientprotocol/claude-agent-acp`, version 0.81.1
(`dist/acp-agent.js`, `dist/context-compaction.js`, `dist/context-compaction-meta.js`).

- **What the adapter watches.** Claude Code's SDK stream marks a compaction three ways: a status
  message `compacting`, a `compact_result` of `success` or `failed`, and a `compact_boundary`
  carrying `compact_metadata`. The adapter turns these into one lifecycle per compaction
  (`ContextCompactionLifecycle`).
- **Two presentations,** chosen by the client's capabilities
  (`clientSupportsCompactionUpdates`: `clientCapabilities.session.compaction` is an object):
  - **with the capability:** `sessionUpdate: "compaction_update"`, carrying a `compactionId`, a
    `status` (`in_progress`, `completed`, `failed` or `cancelled`), the retained summary, and
    `_meta.contextCompaction`;
  - **without it,** which is Shadows today: a `tool_call` with title `Compact conversation`,
    kind `think`, then `tool_call_update`s to its terminal status. Its `_meta` holds
    `contextCompaction: { version: 1, … }` and `claudeCode: { toolName: "compact" }`.
- **The facts carried** (`contextCompactionMetadataFromBoundary`):
  - `trigger`: `manual` (the person ran `/compact`) or `automatic` (Claude Code hit its limit);
  - `preTokens`, and when known `postTokens` and `durationMs`.
  - Without the capability they arrive as the terminal update's `rawOutput`.
- **Shadows today** declares no `session.compaction` capability, and has no code that reads a
  compaction (`grep -i compact` over `shadows-agent`, `core-turns` and `core-harness` finds only
  the `CLAUDE_CODE_AUTO_COMPACT_WINDOW` setting). A compaction reaches it as an ordinary tool call
  with that `_meta`.

Not measured: a live `/compact` and a live automatic compaction through `shadows serve`. The read
above is the adapter's own code at the version Shadows pins; a live run confirms it when slice 2
is built.

## 2. No project in the development database has a design to measure

- The development database (`%LOCALAPPDATA%\shadows-dev\shadows.sqlite3`, read-only) holds one
  project, `AYLUL`, and no rows in `design_vision_version`, `design_part`, `design_outcome`,
  `agreement`, `plan` or `planning_thread`.
- `F:\testing4` is a created project structure (`SHADOWS.md`, `api`, `backend`, `database`,
  `frontend`) with no design rows behind it in that database.
- So no real index or detail can be measured today. Any design written now for the measurement
  would be text written for the test, not a project's.
