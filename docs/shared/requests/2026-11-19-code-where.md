# Where a thing is in the project (`code_where`)

- Status: accepted (accepted on 2026-11-20 and built; the 2026-11-21 amendment for B-189 accepted the same day)
- Asked by: drafted by Hamoud for Masada at the controller's request, 2026-11-19 (Masada reviews the
  Need and Fields, and accepts or replies)
- Screen: the code record design §5.3 and §5.4
  (`docs/backend/specs/2026-11-19-project-code-record-design.md`); backend plan
  `docs/backend/plans/2026-11-19-code-record-2b.md` at `4d69977d` (B-185)

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
  at_done: "found"|"missing"|"unchecked", live: Live, line?,
  uses_live: UsesLive, uses: UseLine[] (≤ 10), uses_more: number, uses_total?: number }
UsesLive = { state: "not_asked" } | { state: "answered" } | { state: "truncated", why }
         | { state: "failed", why } | { state: "skipped", why }
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
- **Who uses a symbol.** That is `uses_live`, apart from `live`, which says whether the symbol
  itself is there now:
  - `answered`: the complete answer; an empty `uses` then means no uses, and `uses_total` is
    exact.
  - `truncated`: the server's answer was cut at Codebase's bound. `uses` keeps what came back,
    `uses_more` counts only the returned rest (there may be more), and there is no `uses_total`.
  - `failed`, with `why`: an error, no server, or the time ran out during the lookup.
  - `skipped`, with `why`: the budget was spent before the lookup. A symbol can be `live:
    confirmed` with `uses_live: skipped`.
  - `not_asked`: the symbol was not confirmed, so no lookup was made.

  The screen shows "no uses" only for `answered` with an empty `uses`.

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

**Response to the frontend reply (Hamoud, 2026-11-20).** The blocker is accepted (Mohammed and the
controller) and resolved as follows.
- **The shape.** `uses_live`, `uses_more` and `uses_total` are in Fields above.
- **Where it is decided:** code record design §5.3; plan 2b, Ruling 11 and B-182's shapes; B-183's
  outcome table and contract lines.
- **References already returned are kept.** A `truncated` lookup keeps them, and `uses_more` never
  claims a count the server did not give.
- **The tests you asked for** (B-183):
  - `an_empty_reference_answer_is_answered_with_no_uses`;
  - `a_failed_reference_lookup_keeps_the_symbol_confirmed_and_says_why`;
  - `a_truncated_reference_answer_keeps_its_uses_and_has_no_total`;
  - `a_budget_spent_after_the_line_skips_the_lookup_and_says_so`;
  - `uses_beyond_ten_are_counted`, for the exact count.
- **Element rows** keep the project-state request's shapes and refresh rules, as amended there
  today.

**Amendment (Hamoud, 2026-11-21; for re-review).** `SymbolEntry` gains two fields from the
recorded contract, after `behavior`: `inputs: { name, type_name, meaning }[]` and `returns?: {
name, type_name, meaning }`, absent for a class or when the contract gives none. They come from
the record, never from the language server, so they show the signature as planned, like
`behavior`. Reason: the live run showed the Planner reading files to learn signatures that the
answer left out (`docs/evidence/2026-11-21-code-record-2-live.md`, P1;
`docs/backend/plans/2026-11-19-code-record-2b.md` at `4d69977d`, B-189). The rest of the accepted shape is
unchanged. B-189 waits for this amendment to be accepted.

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

**Re-review (Masada, 2026-11-20, after `3f50f96`): accepted.** The blocker is resolved:
- **Each symbol carries its own lookup outcome,** `uses_live`, apart from `live`. Only `answered`
  with an empty `uses` reads as "no uses".
- **`uses_total` is exact and given only with `answered`.** `uses_more` counts returned
  references only, a lower bound under `truncated`, so the screen never claims a count the
  server did not give.
- **Returned references are kept under `truncated`.** `failed` and `skipped` carry `why`, so each
  unchecked line has its reason.
- **The tests asked for are in B-183,** and spec §5.3 and plan 2b Ruling 11 own the rule.

The screen shows the outcome as Core states it and derives none of it. A question is asked when
the person submits it, and again only on "Ask again". Project events do not rerun it.

The frontend task is F-135 in `docs/frontend/stages.md`, blocked until B-182, B-183 and B-185
are built and `api/openapi.json` shows the route.

**Re-review of the amendment (Masada, 2026-11-21, after `605b1bc`): accepted.**
- **The shape meets the need.** `inputs` and `returns` are the recorded contract's
  `ContractValue` (`name`, `type_name`, `meaning`), as written at done, like `behavior`. The
  screen shows them as given and derives nothing: no signature parsed from code, no language
  server call.
- **The where view shows them.** A person asking where something is sees the signature the
  Planner sees, beside `behavior`. `inputs` empty and `returns` absent show nothing; no
  placeholder is invented.
- **One condition for the frontend:** `inputs` is required in the regenerated schema, so F-135's
  typed fixtures no longer compile until the web regenerates. That is F-136 in
  `docs/frontend/stages.md`, blocked until B-189 is built and `api/openapi.json` shows the two
  fields.

## Ruling

<The teacher, only when disputed.>
