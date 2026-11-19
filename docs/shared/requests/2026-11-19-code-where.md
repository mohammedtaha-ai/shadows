# Where a thing is in the project (`code_where`)

- Status: answered
- Asked by: drafted by Hamoud for Masada at the controller's request, 2026-11-19 (Masada reviews the
  Need and Fields, and accepts or replies)
- Screen: the code record design §5.3 and §5.4
  (`docs/backend/specs/2026-11-19-project-code-record-design.md`); backend plan
  `docs/backend/plans/2026-11-19-code-record-2b.md` (B-185)

## Need

**What the person does.** On a project, the person types a topic: `login`, a path such as
`backend/auth`, a symbol name, or a table or screen id or name. They see where it is, Shadows'
answer first:
- the matching files, each with the plans and tasks that made it and its part and layer;
- each file's symbols with their contract behavior;
- the matching tables and screens with their states.

**Then the code's confirmation.** Each symbol's line now and who uses it, grouped by part and
layer, and the gaps. Files that no contract names are listed apart, as "not in the record".
Nothing is written.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/projects/{id}/code-where` | query `topic: string` (1 to 200 characters) | `WhereAnswer` (below) |

```text
WhereAnswer { topic, entries: WhereEntry[] (≤ 20), entries_more,
  not_in_record: UnrecordedFile[] (≤ 10), not_in_record_more, asked: number, budget? }
WhereEntry = { kind: "file", rank, path, owner?, tasks: FileTask[], symbols: SymbolEntry[],
               live: Live, confirmed?: "file_and_symbol"|"file_only" }
           | { kind: "element", rank, element: DesignElement, state: ElementState }
rank: "exact"|"path"|"part"|"words"
Owner { part_id, part_title, layer? }
FileTask { workflow_id, version, task, action, purpose, commit? }   newest first
SymbolEntry { symbol_id, name, kind, workflow_id, version, task, behavior: string[],
  at_done: "found"|"missing"|"unchecked", live: Live, line?, uses: UseLine[] (≤ 10), uses_more }
UseLine { path, line, owner? }
Live = { state: "not_asked" } | { state: "confirmed" } | { state: "gap", why } | { state: "unchecked", why }
UnrecordedFile { path, owner?, symbols: { name, line }[] }
```

`DesignElement` and `ElementState` are the shapes of the project-state request
(`docs/shared/requests/2026-11-19-project-state-and-diagrams.md`).

**Reading the answer:**
- **Order.** Entries come in rank order, then by path. The order is the answer's; the screen does
  not sort them again.
- **`asked` and `budget`.** `asked` is how many language-server questions were spent, at most 30.
  `budget`, when present, says why some lines are `unchecked`.
- **Timing.** The call returns within about 20 s, even when servers are slow.

## Errors

| When | The screen shows |
|---|---|
| unknown or removed project (404) | the project's not-found state |
| blank or longer than 200 characters (422 `INVALID_COMMAND`) | the field's error line with the message |
| no entry matches | "Shadows has no record of this", with the not-in-the-record files when there are some |
| a server could not answer | the record's lines as they are, each unchecked line with its reason |

## Backend answer

As written above. `CodeRecord::code_where` (B-182, B-183) answers both this route and the Planner's
`code_where` tool. The route is B-185 and waits for this request. Contracts touched:
`crates/code-record/contract.yaml`, `crates/shadows-http/src/contract.yaml`.

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
