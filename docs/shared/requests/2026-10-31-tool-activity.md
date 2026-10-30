# Tool activity: what each tool call did, live and kept

- Status: asked
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

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
