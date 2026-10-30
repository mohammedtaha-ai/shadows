# Tool activity: what each tool call did, live and kept

- Status: accepted
- Asked by: Masada, 2026-10-31
- Screen: [docs/frontend/plans/2026-10-31-shell-and-activity.md](../../frontend/plans/2026-10-31-shell-and-activity.md#2-agent-activity), tasks F-53 to F-56

## Need

While the agent works, the person reads what it does the way Claude Code's own app shows
it: a run of tool calls is one line ("Read 3 files, ran 2 commands, edited a file +36",
the total time, a chevron) that opens into numbered steps; a step still running shows a
spinner and its elapsed time; an edit shows `+added −removed` and opens into its diff with
line numbers; the agent's task list (Claude's `TodoWrite`, ACP `plan`) is a checklist whose
items are done, in progress or pending. All of it must survive a reload.

Today a `ToolCall` entry carries only the tool's last title in `body`, and its `card` is
`null`; a call becomes visible only when its entry is written at its end; an ACP `plan`
update is dropped (`shadows-agent/src/acp/mapping.rs`, `other => None`). The web only draws
what Core gives: it reads no ACP, works out no diff, and decides no status. It counts and
words the steps of a group itself ("Read 3 files"), from the `kind` and `target` below.

## Fields

### 1. A `ToolCall` entry gains a `card`

`GET /api/threads/{id}/entries` (and the fork's copied entries): a `ToolCall` entry keeps
`body` (the call's last title) and its `card` becomes a `ToolCallCard` instead of `null`.
Entries written before this change keep `card: null`; the web shows their title as today.

```text
ToolCallCard {
  id:          string            ACP toolCallId; the same id the live `tool` frame carries
  tool:        string | null     Claude's tool name: "Read", "Edit", "Bash", "Grep", "WebFetch"...
  kind:        "read" | "edit" | "delete" | "move" | "search" | "execute" | "think"
               | "fetch" | "other"                ACP ToolKind, lower case; "other" when absent
  status:      "completed" | "failed" | "stopped" "stopped": the turn ended before the call did
  target:      string | null     the one thing the call acted on: the first `locations` path,
                                 else the command (execute), pattern (search) or URL (fetch)
                                 from rawInput; null when there is none. A path inside the
                                 project's directory is relative to it; any other stays whole.
  lines:       [number, number] | null   first and last line read or acted on, 1-based and
                                 inclusive (a Read's offset/limit); null when not named
  added:       number | null     lines added, from the call's ACP `diff` content; null if no diff
  removed:     number | null     lines removed, same
  diff:        DiffHunk[] | null the edit, in hunks with 3 lines of context; null when there is
                                 no diff, or when it is longer than 400 lines (then `added` and
                                 `removed` stay set)
  started_at:  string            RFC 3339; when Core first saw the call
  finished_at: string            RFC 3339; when it saw the call end
}

DiffHunk {
  old_start: number | null       file line number of the hunk's first old line; null when Core
                                 cannot place the change in the file
  new_start: number | null       same, for the new file
  lines:     { op: "context" | "add" | "remove", text: string }[]   no trailing newline
}
```

A new file (ACP `oldText` null) is one hunk of `add` lines; a deleted file one of `remove`
lines. When `old_start` and `new_start` are null the web draws the diff without line numbers;
it never invents them.

Please declare `ToolCallCard`, `DiffHunk` and `TodoListCard` (below) as named schemas in
`api/openapi.json` and name them in `ThreadEntry.card`'s description, so `npm run gen:api`
gives the web their types.

### 2. A live `tool` frame on `GET /api/subscribe`

```text
event: tool
data: { op, id, tool, kind, title, target, lines, status, started_at }
```

- `op`: the running turn's operation id. `id`, `tool`, `kind`, `target`, `lines`,
  `started_at`: as in the card. `title`: the call's current title.
- `status`: `"pending" | "in_progress" | "completed" | "failed"` (ACP's), the call's current one.
- Sent when a call starts and each time its title, kind, target, lines or status changes,
  including the change to `completed` or `failed`. The durable entry still comes when the
  call ends, as today.
- Only the turn's own calls: a call that belongs to a subagent (it has a parent) stays in that
  subagent's card, as today.
- Transient, like `delta`: never replayed. After `caught-up`, one frame for every call of a
  running turn that has not ended yet, so a reload during a long command still shows it.
- Documented with the other frames in the subscribe route's description.

### 3. The agent's task list: a live `todos` frame and a `TodoList` entry

```text
event: todos
data: { op, entries: [{ content: string, status: "pending" | "in_progress" | "completed" }] }
```

- Sent on each ACP `plan` update of a running turn, whole (ACP replaces the list each time).
  `priority` is not needed.
- Transient. After `caught-up`, one frame with the current list of a running turn that has one.
- When a turn that received a `plan` update ends (whatever its status), Core writes one entry of
  a new `ThreadEntryKind` `TodoList`, after the turn's other entries, holding the last list:
  - `card`: `TodoListCard { entries: [{ content, status }] }`, same item shape as the frame.
  - `body`: the list as Markdown task lines, one per item (`- [x] ` for completed, `- [ ] `
    otherwise), which is what Copy takes.
  - `operation_id`: the turn's.
  A turn with no `plan` update writes no `TodoList` entry.

## Errors

| When | The screen shows |
|---|---|
| A `ToolCall` card is `null` or cannot be read | The step reads the entry's title, as today; no throw |
| A `TodoList` card cannot be read | Its `body` as plain text |
| A `tool` or `todos` frame cannot be read | The stream's existing rule: it fails with the frame in `problem` |
| A turn ends while a call is still running | Core writes that call's entry with `status: "stopped"`; the step stops spinning |
| No `diff`, but `added` / `removed` set (over 400 lines) | The step shows `+added −removed` and "Diff too long to show" |

## Backend answer

Hamoud, 2026-10-31. **As asked, with five small changes** (marked *change*). Owner spec:
[tool activity](../../backend/specs/2026-10-31-tool-activity-design.md); tasks B-19 to B-22
in `docs/backend/stages.md`. Owners: `shadows-agent` translates ACP and decides nothing;
`Turns` (`crates/core-turns/contract.yaml`) builds the cards, writes the entries and holds
a running turn's activity; `Threads` (`crates/core-threads/contract.yaml`) owns the kind
`TodoList`; `Events` (`crates/core-events/contract.yaml`) delivers the frames and the re-send;
`shadows-http/src/sse.rs` shapes them. No route changes.

**1. `ToolCallCard`: as asked, and:**
- *change* `kind` also takes `switch_mode` (ACP has it; the adapter uses it for plan mode).
- *change* `DiffHunk` carries `path` too, because one call's diff items may name more than one file.
  `lines` items are `DiffLine { op, text }` as you wrote.
- `target`: Core makes a path under the turn's project directory relative, with `/`.
- **Line numbers: yes, for Claude's edits, once the call has run.** Measured on adapter
  0.81.1: an `Edit` starts with a snippet diff (`old_string`/`new_string`) and a location
  without a line. Claude's `PostToolUse` hook then sends one diff item per hunk of the real
  patch, and one location `{path, line: newStart}` for each. Core numbers `new_start` from that
  line, and `old_start` from it less the net lines the file's earlier hunks added. A snippet
  that was never placed, or another harness's diff without a line, has both `null`, as you allowed.
- *change* The context is whatever the harness sent. Claude's patch hunks carry 3 lines. Core adds none.
- `added` / `removed` count the `add` / `remove` lines over all hunks. Over 400 hunk lines, `diff` is `null` and the counts stay.
- *change, timing* The placed diff can come **after** `completed`. So an ended call's entry is
  held. Later updates of the same call still change the card, and it is written before the
  turn's next entry of another kind (a message, a subagent card, a refused permission), or at
  the turn's end. Calls are written in the order they ended. Between `completed` and the entry,
  the step is drawn from its last `tool` frame.
- **`stopped` fits.** Today `Collector::finish` already writes every call still open at the
  turn's end, under its last title. It now writes them with `status: "stopped"` and
  `finished_at` the turn's end.
- `started_at` / `finished_at` come from Core's clock, RFC 3339.

**2. `tool` frame: as asked.** Its data is `{op, id, tool, kind, title, target, lines, status,
started_at}`. It is sent on start and on each change of title, kind, target, lines or status,
for the turn's own calls only. After `caught-up` a reload gets one frame per call not yet ended.
The body is the named schema `ToolLive` (plus `op`).

Rule for the web: the bus items sent between your subscribe and that snapshot are delivered after
it. So a frame can repeat or briefly lag the snapshot, and the last frame for an `id` is
current. **A durable `ToolCall` entry for an `id` is final over any frame for that `id`.**

**3. `todos` frame and `TodoList` entry: as asked.** `todos` is `{op, entries}`, re-sent after
`caught-up` when a running turn has a list. The entry is written after the turn's other entries
with `TodoListCard`, the Markdown `body` and the turn's `operation_id`. An empty list, or none,
writes nothing. Each turn that got a `plan` writes its own entry, so a later turn's list is a
later entry. Measured: Claude's `TodoWrite` (and `TaskCreate`/`TaskUpdate`) never arrive as a
`tool_call`, only as `plan`, so no step duplicates the checklist.

**Named schemas:** `ToolCallCard`, `DiffHunk`, `DiffLine`, `TodoListCard`, `TodoItem`, `ToolLive`,
and the enums `CallKind`, `CallStatus`, `DiffOp`, `TodoStatus`. `ThreadEntry.card`'s description
names which kind carries which. `card` itself stays a free-form object in the schema, as the
`Subagent` card is. The web narrows it by `kind`.

**Not given:** a call's output or error text, and a diff inside the live frame (it comes with
the entry).

## Frontend reply

Masada, 2026-10-31. **Accepted**, all five changes. The plan and F-53 to F-56 follow them:
`switch_mode` gets its own words ("switched mode"); the diff shows a file heading where a
hunk's `path` changes; a step between `completed` and its entry is drawn from its last `tool`
frame; a durable `ToolCall` entry is final over every frame of its `id`; and F-53 aliases all
ten named schemas, narrowing `card` by `kind`. F-53 stays blocked until B-19 to B-22 are built
and `api/openapi.json` shows them.

## Ruling

<The teacher, only when disputed.>
