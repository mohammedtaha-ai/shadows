# Tool activity: what each tool call did, live and kept

- Status: proposed by Hamoud, 2026-10-31; waits on the request's acceptance
- Request: [tool activity](../../shared/requests/2026-10-31-tool-activity.md)
- Frontend plan: [shell and activity](../../frontend/plans/2026-10-31-shell-and-activity.md)
- Plan: [tool activity](../plans/2026-10-31-tool-activity.md)
- Amends: harness-controls §12.3 (the entries row) and §12.10, subagent cards §22.2
  (`docs/superpowers/specs/`), which link here.

## What this decides

What Core keeps and streams of a turn's own tool calls and of the agent's task list
(Claude Code's `TodoWrite`, ACP `plan`). A subagent's calls stay steps of its card (§22.2).

| Rule | Owner | Where |
|---|---|---|
| Read what ACP says of a call and of a plan; decide nothing | `shadows-agent` | `acp/mapping.rs`, `events/tool.rs` |
| Build a call's card: target, lines, counts, hunks, status, times | `Turns` | `core-turns/src/tools.rs`, `hunks.rs` |
| When a call's entry and the task list's entry are written | `Turns` | `core-turns/src/entries.rs` |
| What a running turn shows now (open calls, current list) | `Turns` | live state in `LiveHandles` |
| The entry kind `TodoList` | `Threads` | `core-threads/src/model.rs` |
| Which live items a subscriber gets, and the re-send after `caught-up` | `Events` | `core-events/src/subscription.rs` |
| Frame names and JSON bodies | `shadows-http` | `sse.rs`, `openapi.rs` |

## 1. What the harness says

`shadows-agent` reads, from ACP `tool_call` and `tool_call_update`, besides what it reads
today (§22.2): `kind`, `locations` (path, line), `rawInput`, and the `diff` items of
`content` (path, `oldText`, `newText`). It hands them over as `ToolFacts`. On an update each
is `None` when the update does not carry it; when it does, it replaces the last one (ACP
replaces `content` and `locations` whole). It reads ACP `plan` as
`HarnessEvent::Todos(Vec<TodoItem { content, status }>)`; `priority` is dropped.

Measured against adapter 0.81.1 (`harness/claude/node_modules/@agentclientprotocol/claude-agent-acp`):

- `TodoWrite` and the `Task*` tools are never a `tool_call`: their list arrives only as `plan`,
  whole each time.
- An `Edit` starts with a diff of `old_string` / `new_string` and a location without a line.
  Claude's `PostToolUse` hook then sends an update whose `content` holds one `diff` per hunk
  of the real patch (with its context lines) and whose `locations` hold one
  `{ path, line: newStart }` per hunk. That update can come after the call's `completed`.
- A `Write` of a new file is one diff with `oldText` null.
- A `Read` names `{ path, line: offset }` and `rawInput` `offset` / `limit`.

## 2. A call's card

A `ToolCall` entry keeps `body` (the call's last title) and carries a `ToolCallCard`:

| Field | Rule |
|---|---|
| `id` | ACP `toolCallId` |
| `tool` | Claude's tool name, else ACP `name`, else null |
| `kind` | ACP `ToolKind` in snake case; `other` when absent |
| `status` | `completed`, `failed`, or `stopped` (still open when the turn ended) |
| `target` | The first location's path; else `rawInput`'s `file_path`, `notebook_path`, `path`, `command`, `pattern`, `url`, `query`, the first present; else null. A path under the turn's project directory is made relative to it, with `/` separators; any other is kept as given |
| `lines` | `[from, to]`, 1-based, inclusive. A read with `offset` and `limit`: `[offset, offset + limit - 1]`. An edit whose hunks are placed: the first hunk's `new_start` to the last line of the last hunk's new side. Else null |
| `added`, `removed` | Counted over every hunk; null when the call has no diff |
| `diff` | `DiffHunk[]`, one per ACP diff item, in order; null when there is none, or when the hunks hold more than 400 lines together (`added` and `removed` stay) |
| `started_at`, `finished_at` | Core's clock (RFC 3339): when it first saw the call, and when it saw it end (for `stopped`, the turn's end) |

`DiffHunk { path, old_start, new_start, lines: [{ op: context | add | remove, text }] }`.
Its lines come from diffing the item's `oldText` against its `newText` line by line
(`similar`; `oldText` null is all `add`). The context is whatever the harness sent; Core adds
none. `new_start` is the line of the location that names the same path at the same position
among that path's locations; `old_start` is `new_start` minus the lines the same path's
earlier hunks added net. With no such line both are null, and nobody invents them.

**When it is written.** A call that ends is held, not written at once: later updates of
the same call still change its card (so Claude's placed hunks replace the snippet). Held
calls are written, in the order they ended, before the next entry of another kind (a
message, a subagent card, a refused permission, the text cut by Send now, a `TodoList`) and
at the turn's end, where calls still open are written `stopped`. An update for a call
already written changes nothing. A call with no title is not written, as before.

Entries written before this change keep `card: null`.

## 3. The task list

The last `Todos` of a turn is kept. At the turn's end, whatever its ending, a turn that
received one writes one entry of kind `TodoList`, after its other entries: `card` is
`TodoListCard { entries: [{ content, status }] }`, `body` the list as Markdown task lines
(`- [x] ` for completed, `- [ ] ` otherwise), `operation_id` the turn's. An empty list,
or none, writes nothing.

## 4. Live

While a turn runs, its watcher publishes on the turn's bus:

- `HarnessEvent::Tool(ToolLive)` when a call starts and whenever its title, kind, target,
  lines or status changes. `ToolLive { id, tool, kind, title, target, lines, status,
  started_at }`, `status` being ACP's: `pending`, `in_progress`, `completed`, `failed`.
- `HarnessEvent::Todos` as it came from the harness.

Turns keeps, per running turn, its open calls (not yet ended) and its current list. Events
takes it after the replay (`Turns::live_activity`) and, right after `caught-up`, delivers one
`tool` frame per open call and one `todos` frame for the list. Items published between the
subscription and that read follow it, so the last frame for an id is current; a frame may
repeat one the snapshot already gave. A durable `ToolCall` entry for an id is final over
any frame for it.

Frames (transient, never replayed): `tool` `{op, id, tool, kind, title, target, lines,
status, started_at}` and `todos` `{op, entries}`.

## 5. Protocol

`ThreadEntryKind` gains `TodoList`. `ToolCallCard`, `DiffHunk`, `DiffLine`, `TodoListCard`,
`TodoItem`, `ToolLive` (the `tool` frame's body) and their enums are named schemas in `api/openapi.json`; `ThreadEntry.card`'s
description names which kind carries which. The subscribe route's description lists the two
frames. No route changes.

## Not here

A read's file contents, a command's output, a failed call's error text, a diff while the
call runs (the frame carries none), syntax colouring.
