# Executor notices marked in the conversation

- Status: accepted (the amended shape confirmed by Masada, 2026-11-11)
- Asked by: Hamoud for Mohammed's wish, 2026-11-11 (Masada plans the web item on this shape)
- Screen: the Planner conversation; backend [plan](../../backend/plans/2026-11-11-marked-shadows-notices.md)

## Need

Runs tells the Planner how each executor attempt ended with a `[Shadows]` notice, sent as the
message of a Planner turn. Today it is stored as an ordinary `UserMessage` from `User/local`,
so the page draws it as the person's own bubble. Mohammed wants it drawn as an item inside the
Planner turn's activity, beside its tool calls, the way Claude Code shows "Message from
subagent <name>": a header such as "Executor (sonnet) · T2 · attempt 3 committed", and the full report
under it as markdown, cut with Show more when long. Never a user bubble. The page needs a
marker it reads without guessing from the text, and the header's facts.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/threads/{id}/entries` and the stream's entry frames | nothing new | a notice entry: `kind: "UserMessage"`, `author: { kind: "Executor", id: <the attempt's executor thread id> }`, `body` the full text the model read, `card` an `AttemptNotice` |
| `AttemptNotice` (the `card`) | | `{ workflow_id, version, task, title, attempt, attempt_id, model, outcome: "committed" \| "failed", commit: string \| null, report: string \| null, reason: string \| null, last_addition: boolean }` |
| `GET /api/threads/{id}/queue` (the waiting messages) | nothing new | `QueuedMessage` gains `notice: { executor: <thread id>, card: AttemptNotice } \| null`; null for the person's own |
| `DELETE /api/threads/{id}/queue/{qid}` on a notice row | as today | 409 `QUEUED_NOTICE_KEPT`; the row stays. The person's own rows are removed as today |

- **The marker** is `author.kind == "Executor"`. The person's messages stay `User`/`local` with
  `card: null`. `author.id` is the executor's thread, the conversation the task page opens.
- **The name in the header** is "Executor ({card.model})": an executor has no agent identity of
  its own, only its thread and the model it ran on.
- **The header** comes from the card: `T{task}`, `attempt {attempt}`, `{outcome}`. `task`,
  `attempt`, `outcome`, `title` and `model` are always set; `title` is the task's title, or
  `T{task}` when the plan could not be read; `last_addition` true means this attempt carried the task's third and last
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
| an `Executor` entry whose card is missing or unreadable | the body as plain text in the same item, with the header "Executor" |
| removing a waiting notice (409 `QUEUED_NOTICE_KEPT`) | no Remove is drawn on a notice row; if the call is made anyway, Core's message |
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

**Amended after Mohammed's rulings (2026-11-11), after Masada accepted.** The table above is
the amended shape. (1) The notice arrives in the executor's name: `author` is `Executor` and
the executor's thread id, not `Shadows/runs`, and the card gains `model` for the header's name;
the turn the notice starts runs under the executor's principal, not the person's, and Grants
refuses a vision, map, roadmap or plan approval in a turn that holds no message from the person
(B-135, no change for the web). (2) Core refuses removing a waiting notice:
`DELETE /api/threads/{id}/queue/{qid}` on a notice row is 409 `QUEUED_NOTICE_KEPT`. The web
changes: F-111 and F-112 read `author.kind == "Executor"` instead of `"Shadows"`, name the
header from `card.model`, read `QueuedMessage.notice.executor` instead of `from`, and draw no
Remove on a notice row.

## Frontend reply

**The amended shape is accepted** (Masada, 2026-11-11, after `8b5a5a20`): the marker is
`author.kind == "Executor"`, the header's name is `Executor ({card.model})`,
`QueuedMessage.notice.executor` replaces `from`, and a notice row draws no Remove (Core answers
409 `QUEUED_NOTICE_KEPT`). F-111, F-112 and F-113 read this shape. The reply below is the first
acceptance, kept as written; where it says `Shadows` it is superseded by this paragraph.

Accepted as answered (Masada, 2026-11-11). B-134 may be built. The web plan is
[`docs/frontend/plans/2026-11-11-shadows-notices.md`](../../frontend/plans/2026-11-11-shadows-notices.md),
tasks F-111 (the entry as an activity item) and F-112 (the waiting notice).

- The page reads the marker from `author.kind == "Shadows"` only, and the card with a checked
  reader, since `ThreadEntry.card` stays open. Keep `title`, `task`, `attempt` and `outcome`
  always set on the card: the header needs them.
- `QueuedMessage.notice.card` is the same `AttemptNotice`; the page reads both with one reader.
- The two open questions in the backend plan go to Mohammed with these recommendations:
  1. **A notice turn and approval:** run the notice turn under a Shadows principal
     (`Shadows/runs`), not the person's, and let Grants refuse `plan_approve` (and any other
     approval the person alone may give) for a non-person principal. The record then says who
     really asked, and the refusal sits in its one owner. The web needs nothing for it.
  2. **Removing a waiting notice:** Core refuses it (`unqueue` of a row with a notice answers a
     refusal), so every client holds the same rule; the web draws no Remove on a notice row,
     because the row is Shadows' and not the person's (F-112). If Mohammed rules the person may
     remove it, Core changes nothing and F-112 keeps the button.

## Ruling

<The teacher, only when disputed.>
