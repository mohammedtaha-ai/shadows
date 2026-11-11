# Executor notices drawn as the Planner's activity

Mohammed, 2026-11-11. Runs tells the Planner how each executor attempt ended with a
`[Shadows]` notice, sent as the message of a Planner turn. Today the page draws it as the
person's own bubble. Mohammed wants it drawn the way Claude Code's desktop draws "Message from
subagent <name>": an item of the turn's activity, beside its tool calls, with a header such as
"Executor (sonnet) · T2 · attempt 3 committed" and the executor's full report under it as markdown, cut
with Show more when long. Never a user bubble, and never the person's waiting message while it
waits in the queue.

**Mohammed's rulings, 2026-11-11:**
1. The notice arrives **in the executor's name**, not the person's and not a plain "Shadows":
   the header reads `Executor ({card.model}) · T2 · attempt 3 committed`, as
   [the request](../../shared/requests/2026-11-11-shadows-notices.md) states after its
   amendment (`8b5a5a20`). Core refuses approvals in a turn with no message from the person
   (B-135); the web needs nothing for that.
2. Core refuses removing a waiting notice (409 `QUEUED_NOTICE_KEPT`); the web shows no Remove
   on it (F-112).
3. Each Planner turn is one collapsible box, as in Claude (F-113, after F-111). Cards the
   person acts on stay outside the folded box, under it (Mohammed confirmed).

**Mohammed's ruling, 2026-11-12** (after seeing F-113 in the browser):

4. A turn box holds **only** what comes to the Planner from others (the executor notices,
   F-111) and the Planner's tool groups. The Planner's own text never goes inside a box, the
   notes between tool calls included: it is drawn outside, in its place in the turn's order. The
   summary counts no notes. A turn with text between tool calls draws one box per run of tools
   and notices between two pieces of text, as Claude Code groups tool calls between its
   messages. Everything else F-113 decided stands.

Built by `frontend-builder`, one task at a time, from `docs/frontend/stages.md`.

Backend shape: [the shadows notices request](../../shared/requests/2026-11-11-shadows-notices.md)
(accepted), built by B-134 ([its plan](../../backend/plans/2026-11-11-marked-shadows-notices.md)).

## What Core gives

- A notice entry is a `UserMessage` whose `author` is `{ kind: "Executor", id: <the attempt's
  executor thread id> }`. The person's messages stay `User`/`local` with `card: null`. **The
  marker is `author.kind === "Executor"`**; the page never reads the body's text to decide.
- The header's name is `Executor ({card.model})`: an executor has no identity of its own, only
  its thread and the model it ran on.
- Its `card` is an `AttemptNotice`: `{ workflow_id, version, task, title, attempt, attempt_id,
  model, outcome: "committed" | "failed", commit, report, reason, last_addition }`. `task`,
  `attempt`, `outcome`, `title` and `model` are always set. `ThreadEntry.card`
  stays an open object in the schema, so the page reads it with a checked reader.
- `body` is the whole notice as the model read it, Shadows' instructions to the Planner included.
  The page shows it only when the card is missing or unreadable.
- `QueuedMessage.notice: { executor: <thread id>, card: AttemptNotice } | null`, null for the
  person's. Removing a notice row answers 409 `QUEUED_NOTICE_KEPT`.
- Old notices recorded before B-134 keep `User` and no card: they stay bubbles. Core does not
  rewrite them, and the page does not guess.

## Where the item sits

There is no single collapsible block per turn today: a turn's tool calls draw as tool groups
(`ToolGroup`, one collapsible line per run of calls of one turn), between the turn's text. A
notice is the **prompt** of the turn it starts, so its entry comes first in that turn, just
above the turn's first tool group. The notice item takes the tool group's look (left-aligned
`surface-panel rounded-xl`, a chevron header that opens and closes), so it reads as the first
item of the turn's activity, not as a message. A notice steered into a running turn (Send now)
is that turn's steering entry, so it sits among that turn's calls, where it arrived.

F-113 then folds the whole turn, the notice item included, into one box (ruling 3).

## Global constraints

- No `fetch`, no URL, no Core rule in a component (`docs/frontend/AGENTS.md` rule 0). The
  header's facts come from the card; the page computes nothing about the attempt.
- Find symbols with Serena (`s.lsp` only). Open a file at the symbol you need.
- Run only the tests the task names plus `npm run typecheck`, `npm run lint`, `npm run build`
  and `npx vitest run src/codemap.test.ts src/architecture`, from `web/`. Never the whole
  suite. Never run vitest while a `cargo` process runs on this machine (check with
  `powershell -NoProfile -Command "Get-Process cargo -ErrorAction SilentlyContinue"`; empty
  output means none).
- `src/api/schema.d.ts` changes only through `npm run gen:api`, run first, after B-134 is
  committed (`api/openapi.json` holds `AttemptNotice`, `Notice` and `QueuedMessage.notice`).
- No browser journey: Mohammed tries it in the browser himself.
- One commit per task, `git add <each file>`, nothing under `.claude/`, no push.

## F-111 · An executor's notice draws as an item of the Planner's activity

**Reading the card.**
- `web/src/api/runs.ts`: export `type AttemptNotice = components['schemas']['AttemptNotice']`
  (Runs owns the type). Add it to the `runs:` block of `web/src/api/contract.yaml`.
- `web/src/stream/frames.ts`: `readAttemptNotice(value: unknown): AttemptNotice | null`, beside
  `readSubagentCard` and in its style: null unless `task` and `attempt` are numbers, `title` and
  `model` strings and `outcome` is `committed` or `failed`; `commit`, `report` and `reason` read as
  string or null; `last_addition` true only when it is `true`. Line in
  `web/src/stream/contract.yaml`.
- `web/src/app/conversation/tool-text.ts`: `noticeOf(entry: ThreadEntry): { card: AttemptNotice
  | null } | null`, beside `subagentOf`: null unless `entry.kind === 'UserMessage'` and
  `entry.author.kind === 'Executor'`; otherwise the card read by `readAttemptNotice` (null when
  missing or unreadable). `copyText` copies a notice's `report`, else its `reason`, else its body.

**The item.** New file `web/src/app/conversation/notice-item.tsx`:
- `noticeName(card: AttemptNotice | null): string`: `Executor ({card.model})`, or `Executor`
  when the card is null. `noticeTitle(card)`: `{noticeName} · T{task} · attempt {attempt}
  {outcome}`, or `Executor` when the card is null. Both exported: F-112 and F-113 use them.
- `NoticeItem({ body, card })`: a left-aligned `surface-panel rounded-xl p-3` block, like a
  tool group of several steps.
  - Header: a `button` with `aria-expanded`, a chevron (rotates when open), an icon
    (`Bell` or `MessageSquareShare` from lucide; pick one and keep it), the `noticeTitle` text,
    then, muted, the task's `title` (truncated), a `last addition` badge when `last_addition` is
    true, and the commit's first 7 characters in mono when `commit` is set. `outcome === "failed"`
    tints the outcome word `text-destructive`. Open by default; the header closes and reopens it.
  - Body (when open), under a `border-t` as the tool group's list:
    - `reason` (failed attempts), when set, as one line in the warning/destructive tone.
    - `report`, when set, as markdown with the Planner's markdown renderer, in a compact size:
      `ReplyText` (from `./reply-text`, loaded lazily the way `entry.tsx` loads it, with the
      plain text as the Suspense fallback) gains an optional `compact` prop that swaps
      `font-serif text-[15px] leading-7` for `text-sm leading-6`. Nothing else in `ReplyText`
      changes.
    - A long report (more than 1,200 characters or more than 16 lines) is shown in a box of
      `max-h-80 overflow-hidden` with a `Show more` button under it; `Show more` lifts the
      limit and becomes `Show less`. The full report is always in the DOM; only its height is cut.
    - Card null (unreadable or missing): the `body` as plain text (`whitespace-pre-wrap`,
      `dir="auto"`), under the header `Executor`.
    - `report` and `reason` both null: the header alone, with no chevron and no body.
- `web/src/app/conversation/entry.tsx`: the `UserMessage` branch first asks `noticeOf(entry)`;
  a notice returns `<NoticeItem body={entry.body} card={notice.card} />`. The person's bubble
  is unchanged.
- `web/src/app/conversation/messages.tsx`, `EntryRow`: the right-aligned footer
  (`justify-end`) is for the person's messages only; a notice's `ContextMark` and
  `MessageActions` sit left-aligned under the item, as under a tool group.

**Tests**, new file `web/src/app/conversation/notice-item.test.tsx`, drawing `Entry` as
`todo-card.test.tsx` does:
- `a_notice_draws_as_an_activity_item_not_a_bubble`: an `Executor` entry with a committed card
  (task 2, attempt 3, model `sonnet`, a report holding `**done**` and a list): the header reads
  `Executor (sonnet) · T2 · attempt 3 committed`; the task title shows; `done` is drawn inside a `strong`; the body's
  instruction text is not on the page; no element carries the bubble's `surface-glow` class.
- `a_failed_notice_shows_its_reason_and_report`: outcome `failed`, `reason` and `report` set:
  both show; the header reads `attempt 1 failed`.
- `a_long_report_is_cut_with_show_more`: a report of 40 lines: `Show more` shows; after a
  click, `Show less`.
- `a_notice_without_a_readable_card_shows_its_body_under_executor`: author `Executor`, card
  `null`: header `Executor`, the body as plain text.
- `the_header_closes_and_reopens_the_report`: a click on the header hides the report
  (`aria-expanded` false); a second click shows it.
- `an_old_notice_from_the_person_stays_a_bubble`: author `User`/`local`, body starting
  `[Shadows]`, card null: drawn as the person's bubble (the page does not read the text).

**Contracts and map.** `web/src/app/conversation/contract.yaml`: one obligation, "An entry
whose author is an Executor draws as a notice item of the turn's activity, never as the person's
bubble; its header comes from its card, its report is drawn whole as markdown and cut only in
height (notice-item.test.tsx)". `docs/frontend/codebase/README.md`: a row for
`web/src/app/conversation/notice-item.tsx`, "An executor's notice drawn as an activity item".

## F-112 · A waiting notice is not drawn as the person's waiting message

`web/src/app/conversation/waiting-messages.tsx`: a row whose `notice` is not null draws as a
notice, not as the person's message:
- left-aligned (no `ml-auto`), in the same muted panel as the notice item;
- one line: the icon of `NoticeItem`, `noticeTitle(readAttemptNotice(m.notice.card))`, and
  `Waiting for the Planner` muted; the `prompt` text is **not** shown (it is the model's text);
- `Send now` / `Send` stays as for every row (Core steers it with its author and card);
- **no `Remove` button** (ruling 2: Core refuses it with 409 `QUEUED_NOTICE_KEPT`; if the call
  is made anyway, the existing error line shows Core's message);
- the row reads `m.notice.card`; `m.notice.executor` is not shown; `last_error`, when set, shows as for
  every row.
A row whose `notice` is null is drawn exactly as today.

**Tests**, in `web/src/app/conversation/waiting-messages.test.tsx`:
- `a_waiting_notice_is_not_drawn_as_the_persons_message`: the queue answers a notice row
  (task 2, attempt 3, committed) then the person's row. The notice row shows `Executor (sonnet) · T2 ·
  attempt 3 committed` and `Waiting for the Planner`, not its prompt text, offers `Send now`
  and no `Remove`; the person's row shows its text with `Send now` and `Remove`, as today.

**Contract.** `web/src/app/conversation/contract.yaml`: the waiting messages line says a
notice waits as a notice line, with no Remove (`a_waiting_notice_is_not_drawn_as_the_persons_message`).

## F-113 · Each Planner turn is one collapsible box

Rulings 3 and 4. Like Claude: a Planner turn's notices and tool calls fold into boxes, one per
run between two pieces of the Planner's text; the text itself stays outside every box.

**Needs from Core: nothing.** Every entry carries `operation_id` (its turn), and the person's
prompt is the turn's `UserMessage`. The streamed rows of the running turn belong to the newest
turn while `running` is true. No shared request.

**What goes where, per turn** (entries with the same `operation_id`; entries with none are
drawn as today, outside any box):
- **Above the boxes:** the person's prompt bubble (a `UserMessage` that is not a notice).
- **In a box, in order:** one run of notice items (F-111), tool groups and the permission
  answered/refused lines among them.
- **Outside every box, in its place (ruling 4, 2026-11-12):** the Planner's own text
  (`AgentMessage` entries and streamed text), whether between tool calls or the final answer.
  Each piece of text ends the run before it, so the run after it is a new box.
- **Under the box of the run it came in, visible:** cards the person reads or acts on
  (`PermissionAsked`, the vision, map and roadmap proposals, `PlanView`, `PlanApproved`,
  `Subagent`, `TodoList`), in their order, so an approval or a question is never folded away.
  A card does not split the run.
- A run with no notice and no tool call draws no box.

**The box.** New file `web/src/app/conversation/turn-box.tsx`, `TurnBox({ summary, live,
children })`: left-aligned `surface-panel rounded-xl p-3`, a header `button` with
`aria-expanded`, a chevron, and the summary line. **Open while its turn runs** (the newest
turn while `running`; only the turn's last box, the earlier ones have ended), **folded once it
ends**; the person's click wins over both, as
`ToolGroup` does (`chosen ?? live`). A link to an entry inside (`#entry-<ordinal>`) opens it.
- The summary line: the parts that apply, joined by ", ", first letter capital: `received a
  notice from {noticeName(card)}` (F-111's `noticeName`: `Executor (sonnet)`, or `Executor`
  with no readable card), `ran N tools` (`ran 1 tool`). It counts no text (ruling 4). Example:
  "Received a notice from Executor (sonnet), ran 5 tools". While it runs, a
  spinner replaces the icon, as `ToolGroup` does.

**The grouping.** New file `web/src/app/conversation/turn-blocks.ts`:
`turnBlocks(rows: readonly Row[], runningOp: string | null): Block[]`, a pure function over
the `activityRows`/`withLiveSteps` output. `Block` is `{ kind: 'row', row }` for a row drawn as
today, or `{ kind: 'turn', op, key, inside: Row[], summary parts, live }`. A row's turn is its
entry's `operation_id`, a tool row's steps' operation, and a streamed text row's the newest
turn while `running`. The text rows stay `row` blocks in their place, between the turn blocks;
a run's outside cards come right after its turn block as `row` blocks. `messages.tsx` maps the blocks: a `turn` block draws `TurnBox` with its inside
rows drawn exactly as `Messages` draws them today (`ToolRow`, `EntryRow`, `ReplyText`); the
reply tail, live lists, subagent cards, Thinking and the operational label stay where they are.
Applies to Planner conversations only: `Messages` skips the boxes when `harness ===
AYLUL_HARNESS`.

**Tests:**
- `web/src/app/conversation/turn-blocks.test.ts` (new):
  `a_finished_turn_folds_its_notice_and_tools_and_keeps_its_final_answer` (notice, 2 tool rows,
  final text: one turn block holding the first three, the final text outside);
  `the_planner_text_between_tools_stays_outside_and_splits_the_box` (notice, tool, text, 2
  tools, final text: box, text, box, text; ruling 4); `only_the_last_box_of_a_running_turn_is_live`;
  `a_turn_with_no_activity_has_no_box`; `a_proposal_card_stays_outside_the_box`;
  `the_person_prompt_stays_above_the_box`; `a_message_steered_in_mid_turn_stays_where_it_was_sent`.
- `web/src/app/conversation/turn-box.test.tsx` (new):
  `a_finished_turn_shows_one_summary_line_and_opens_on_click` (folded: "Received a notice from
  Executor (sonnet), ran 5 tools", tool rows absent; after a click, present);
  `the_planner_text_between_tools_stays_outside_and_splits_the_box` (two summaries, no "wrote",
  the note visible while both are folded); `a_running_turn_is_open_and_folds_when_it_ends`.
- Existing tests that read a finished turn's tool rows or in-between text now open its box
  first: `conversation.test.tsx`, `streamed-reply.test.tsx`, `tool-group.test.tsx`,
  `plan-in-conversation.test.tsx`. If another test file breaks, stop and report it: Masada
  widens the task.

**Contracts and map.** `web/src/app/conversation/contract.yaml`: one obligation, "A turn box holds only the notices
and tool groups, with a summary that counts no text; the Planner's text stays outside in its
place and splits the runs into boxes; the prompt stays above, the cards the person acts on stay
visible under their run's box; only a running turn's last box is open
(turn-blocks.test.ts, turn-box.test.tsx)".
`docs/frontend/codebase/README.md`: rows for `turn-blocks.ts` ("Groups a turn's activity rows
into one box") and `turn-box.tsx` ("One Planner turn's activity as one collapsible box").
