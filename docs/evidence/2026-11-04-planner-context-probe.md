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

## 3. Measured on testing55, a bigger project planned for this probe

On 2026-11-04 Claude played the person in a new Planner conversation in `F:	esting55`
(Haiku 5.5, Claude Code). It described, step by step, a support system: a Telegram bot for
customers, a Laravel backend on MySQL with Redis queues, a Flutter app for agents and a web
admin dashboard. The Planner then took it through the vision field by field, a map, the created
structure, a roadmap and the Stage 1 plan. Sizes were read through the daemon as in §2.

| Piece | Characters | About tokens |
|---|---|---|
| Vision, whole: 1 purpose, 3 users, 5 goals, 5 boundaries, 7 technical directions | 4 516 | 1 130 |
| **Index** with the whole vision, 6 part names, 2 stage names | 5 042 | 1 260 |
| Anchor `Backend (Laravel core)`, its full content | 1 146 | 290 |
| The Stage 1 plan: title, goal, 10 tasks, links | 8 966 | 2 240 |
| Project standards in effect | 2 467 | 620 |
| **Detail total** | 12 579 | 3 140 |

- With the whole vision, the index is already near §4.2's 1 500 tokens after one plan. The
  design's rule, "the vision's title and first paragraph" in the index, is what keeps it small.
- The detail grows mostly with the plan's tasks: 10 tasks are about 2 240 tokens.
- No agreements were made; parts carry their own contracts.

Seen on the way, outside this probe's question:
- The map was refused three times (a file outside its part's folders, then two overlapping
  folder claims). The Planner fixed it alone each time.
- Its part `Serves` lists name IDs that do not match the vision's numbering for technical
  directions (the API part serves `TD1 TD2 TD3`).
