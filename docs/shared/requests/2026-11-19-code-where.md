# Where a thing is in the project (`code_where`)

- Status: disputed
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

Reviewed as Masada, 2026-11-20, against the owner design §5.3 and §8 and plan 2b
(B-182, B-183, B-185). CodeRecord ownership, the single read route, server ranking, historical
`at_done` versus live confirmation, owner metadata, explicit remainder counts, and 404/422
errors meet the frontend need. Results can be shown through a generated API domain hook without
reconstructing Core's matching or state rules. A submitted search can be rerun explicitly; this
request does not require an automatic live search on each project event.

**Blocker: an empty caller list cannot distinguish no uses from an unanswered lookup.** B-183's
FindReferences step explicitly says a failure leaves `uses` empty and adds nothing else. After
DocumentSymbol succeeds, `SymbolEntry.live` is already `confirmed`. The response thus looks the
same when there are no callers and when caller lookup fails, times out, is skipped for budget,
or returns an incomplete answer. `asked` and an optional answer-level `budget` do not identify
which confirmed symbol's uses were actually checked. This contradicts the need to show who uses
it and the Errors promise to show unchecked lines with their reason.

Before acceptance, agree a per-symbol reference-lookup outcome and reason, separate from the
symbol's own live location confirmation (for example `uses_live`, with a way to mark partial
answers). Specify the meanings of `uses_more` for truncated upstream results: it must not imply
an exact count when the server did not supply one. Preserve any references already returned,
while visibly marking the incomplete lookup. Add tests covering a successful empty lookup,
failure, a truncated answer, and budget exhaustion after the symbol's line is confirmed, so the
frontend can distinguish these cases without guessing. Amend B-183 and the owning shape/contract
before B-185 exposes it.

Element rows reuse the project-state shapes; their interpretation must stay aligned with that
request once its refresh dispute is resolved. This is an agreement review, not confirmation of
implementation. The request remains disputed for the controller's ruling under the
shared-request cycle.

## Ruling

<The teacher, only when disputed.>
