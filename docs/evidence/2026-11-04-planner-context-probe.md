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

## 2. Measured on testing4, the one project with a whole design

Read on 2026-11-04 through the running daemon (`shadows serve --db
%LOCALAPPDATA%\shadows-dev\shadows.sqlite3`), with `GET /api/projects/{id}/design/vision`,
`/design/parts`,
`/design/outcomes`, `/standards`, `/workflows` and `GET /api/workflows/{id}`.

A first read of the same database file opened read-only with Python's `sqlite3` found only the
AYLUL project. It missed rows still in the write-ahead log, so a read beside a live daemon goes
through the daemon, not the file.

| Project | Vision (JSON chars) | Parts | Outcomes | Plans |
|---|---|---|---|---|
| testing4 (a to-do app) | 1 253 | 4, all top level | 4 | 1, frozen v1, 4 tasks |
| CSV Tool | 773 | 2 | — | 0 |
| shadows | 102 | 0 | — | 0 |

testing4, sizes as JSON characters (an upper bound for the text the context would carry),
tokens estimated as characters / 4:

| Piece | Characters | About tokens |
|---|---|---|
| **Index:** vision, part names and ids, outcome names and ids | 1 781 | 450 |
| Anchor `Backend`, its full content | 687 | 170 |
| The linked plan: title, goal, 4 tasks, change reason, links | 2 725 | 680 |
| Project standards in effect | 2 467 | 620 |
| **Detail total** | 5 879 | 1 470 |

testing4 has no agreements, and its plan has one version, so no older rationales. A bigger
project grows mainly with more parts in the index, and with agreements and plan versions in the
detail.
