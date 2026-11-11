# Shadows' notices marked in the conversation

- Status: answered
- Asked by: Hamoud for Mohammed's wish, 2026-11-11 (Masada plans the web item on this shape)
- Screen: the Planner conversation; backend [plan](../../backend/plans/2026-11-11-marked-shadows-notices.md)

## Need

Runs tells the Planner how each executor attempt ended with a `[Shadows]` notice, sent as the
message of a Planner turn. Today it is stored as an ordinary `UserMessage` from `User/local`,
so the page draws it as the person's own bubble. Mohammed wants it drawn as an item inside the
Planner turn's activity, beside its tool calls, the way Claude Code shows "Message from
subagent <name>": a header such as "Shadows · T2 · attempt 3 committed", and the full report
under it as markdown, cut with Show more when long. Never a user bubble. The page needs a
marker it reads without guessing from the text, and the header's facts.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/threads/{id}/entries` and the stream's entry frames | nothing new | a notice entry: `kind: "UserMessage"`, `author: { kind: "Shadows", id: "runs" }`, `body` the full text the model read, `card` an `AttemptNotice` |
| `AttemptNotice` (the `card`) | | `{ workflow_id, version, task, title, attempt, attempt_id, outcome: "committed" \| "failed", commit: string \| null, report: string \| null, reason: string \| null, last_addition: boolean }` |
| `GET /api/threads/{id}/queue` (the waiting messages) | nothing new | `QueuedMessage` gains `notice: { from: "runs", card: AttemptNotice } \| null`; null for the person's own |

- **The marker** is `author.kind == "Shadows"`. The person's messages stay `User`/`local` with
  `card: null`.
- **The header** comes from the card: `T{task}`, `attempt {attempt}`, `{outcome}`; `title` is
  the task's title; `last_addition` true means this attempt carried the task's third and last
  addition.
- **The report:** `report` is the executor's full report for a committed attempt, and its last
  message for a failed one (cut at 4,000 characters with `[cut]`), else null. `reason` is the
  failure's reason, null when committed. Both are markdown as the executor wrote them.
- `body` keeps the whole notice as the model read it, including Shadows' instructions to the
  Planner; the page need not show it.
- Old notices recorded before this ships keep `User` and no card; Core does not rewrite them.

## Errors

| When | The screen shows |
|---|---|
| a `Shadows` entry whose card is missing or unreadable | the body as plain text in the same item, with the header "Shadows" |
| `report` and `reason` both null | the header alone |

## Backend answer

As written (Hamoud, 2026-11-11). Owners: Runs builds the notice and its card
(`crates/core-runs/contract.yaml`, `notify`); Turns records who wrote a queued message and its
card on the waiting row and on the turn's prompt entry (`crates/core-turns/contract.yaml`,
new `Turns::queue_notice`). Threads stores entries as given and does not change. The entry
stays a `UserMessage` because Core reads a turn's prompt entry by that kind; the author is the
marker. No route is added. `api/openapi.json` gains the `AttemptNotice` and `Notice` schemas
and `QueuedMessage.notice`; `ThreadEntry.card` stays an open object. Built by B-134 once this
is `accepted`.

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
