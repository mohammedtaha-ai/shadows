# Shadows' notices drawn as the Planner's activity

Mohammed, 2026-11-11. Runs tells the Planner how each executor attempt ended with a
`[Shadows]` notice, sent as the message of a Planner turn. Today the page draws it as the
person's own bubble. Mohammed wants it drawn the way Claude Code's desktop draws "Message from
subagent <name>": an item of the turn's activity, beside its tool calls, with a header such as
"Shadows · T2 · attempt 3 committed" and the executor's full report under it as markdown, cut
with Show more when long. Never a user bubble, and never the person's waiting message while it
waits in the queue.

Backend shape: [the shadows notices request](../../shared/requests/2026-11-11-shadows-notices.md)
(accepted), built by B-134 ([its plan](../../backend/plans/2026-11-11-marked-shadows-notices.md)).

## What Core gives

- A notice entry is a `UserMessage` whose `author` is `{ kind: "Shadows", id: "runs" }`. The
  person's messages stay `User`/`local` with `card: null`. **The marker is `author.kind ===
  "Shadows"`**; the page never reads the body's text to decide.
- Its `card` is an `AttemptNotice`: `{ workflow_id, version, task, title, attempt, attempt_id,
  outcome: "committed" | "failed", commit, report, reason, last_addition }`. `ThreadEntry.card`
  stays an open object in the schema, so the page reads it with a checked reader.
- `body` is the whole notice as the model read it, Shadows' instructions to the Planner included.
  The page shows it only when the card is missing or unreadable.
- `QueuedMessage.notice: { from: "runs", card: AttemptNotice } | null`, null for the person's.
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

If Mohammed wants one collapsible block holding a whole turn's activity (notice, every tool
group and the text between them), that is a redesign of `Messages` and is not planned here.

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

## F-111 · A Shadows notice draws as an item of the Planner's activity

**Reading the card.**
- `web/src/api/runs.ts`: export `type AttemptNotice = components['schemas']['AttemptNotice']`
  (Runs owns the type). Add it to the `runs:` block of `web/src/api/contract.yaml`.
- `web/src/stream/frames.ts`: `readAttemptNotice(value: unknown): AttemptNotice | null`, beside
  `readSubagentCard` and in its style: null unless `task` and `attempt` are numbers, `title` a
  string and `outcome` is `committed` or `failed`; `commit`, `report` and `reason` read as
  string or null; `last_addition` true only when it is `true`. Line in
  `web/src/stream/contract.yaml`.
- `web/src/app/conversation/tool-text.ts`: `noticeOf(entry: ThreadEntry): { card: AttemptNotice
  | null } | null`, beside `subagentOf`: null unless `entry.kind === 'UserMessage'` and
  `entry.author.kind === 'Shadows'`; otherwise the card read by `readAttemptNotice` (null when
  missing or unreadable). `copyText` copies a notice's `report`, else its `reason`, else its body.

**The item.** New file `web/src/app/conversation/notice-item.tsx`:
- `noticeTitle(card: AttemptNotice | null): string`: `Shadows · T{task} · attempt {attempt}
  {outcome}`, or `Shadows` when the card is null. Exported: F-112 uses it.
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
      `dir="auto"`), under the header `Shadows`.
    - `report` and `reason` both null: the header alone, with no chevron and no body.
- `web/src/app/conversation/entry.tsx`: the `UserMessage` branch first asks `noticeOf(entry)`;
  a notice returns `<NoticeItem body={entry.body} card={notice.card} />`. The person's bubble
  is unchanged.
- `web/src/app/conversation/messages.tsx`, `EntryRow`: the right-aligned footer
  (`justify-end`) is for the person's messages only; a notice's `ContextMark` and
  `MessageActions` sit left-aligned under the item, as under a tool group.

**Tests**, new file `web/src/app/conversation/notice-item.test.tsx`, drawing `Entry` as
`todo-card.test.tsx` does:
- `a_notice_draws_as_an_activity_item_not_a_bubble`: a `Shadows` entry with a committed card
  (task 2, attempt 3, a report holding `**done**` and a list): the header reads `Shadows · T2 ·
  attempt 3 committed`; the task title shows; `done` is drawn inside a `strong`; the body's
  instruction text is not on the page; no element carries the bubble's `surface-glow` class.
- `a_failed_notice_shows_its_reason_and_report`: outcome `failed`, `reason` and `report` set:
  both show; the header reads `attempt 1 failed`.
- `a_long_report_is_cut_with_show_more`: a report of 40 lines: `Show more` shows; after a
  click, `Show less`.
- `a_notice_without_a_readable_card_shows_its_body_under_shadows`: author `Shadows`, card
  `null`: header `Shadows`, the body as plain text.
- `the_header_closes_and_reopens_the_report`: a click on the header hides the report
  (`aria-expanded` false); a second click shows it.
- `an_old_notice_from_the_person_stays_a_bubble`: author `User`/`local`, body starting
  `[Shadows]`, card null: drawn as the person's bubble (the page does not read the text).

**Contracts and map.** `web/src/app/conversation/contract.yaml`: one obligation, "An entry
whose author is Shadows draws as a notice item of the turn's activity, never as the person's
bubble; its header comes from its card, its report is drawn whole as markdown and cut only in
height (notice-item.test.tsx)". `docs/frontend/codebase/README.md`: a row for
`web/src/app/conversation/notice-item.tsx`, "A Shadows notice drawn as an activity item".

## F-112 · A waiting notice is not drawn as the person's waiting message

`web/src/app/conversation/waiting-messages.tsx`: a row whose `notice` is not null draws as a
notice, not as the person's message:
- left-aligned (no `ml-auto`), in the same muted panel as the notice item;
- one line: the icon of `NoticeItem`, `noticeTitle(readAttemptNotice(m.notice.card))`, and
  `Waiting for the Planner` muted; the `prompt` text is **not** shown (it is the model's text);
- `Send now` / `Send` stays as for every row (Core steers it with its author and card);
- **no `Remove` button** (the queue question below); `last_error`, when set, shows as for
  every row.
A row whose `notice` is null is drawn exactly as today.

**Tests**, in `web/src/app/conversation/waiting-messages.test.tsx`:
- `a_waiting_notice_is_not_drawn_as_the_persons_message`: the queue answers a notice row
  (task 2, attempt 3, committed) then the person's row. The notice row shows `Shadows · T2 ·
  attempt 3 committed` and `Waiting for the Planner`, not its prompt text, offers `Send now`
  and no `Remove`; the person's row shows its text with `Send now` and `Remove`, as today.

**Contract.** `web/src/app/conversation/contract.yaml`: the waiting messages line says a
notice waits as a notice line, with no Remove (`a_waiting_notice_is_not_drawn_as_the_persons_message`).

**If Mohammed rules that the person may remove a waiting notice** (the request's second open
question), the `Remove` button stays on the notice row and the test checks it is there.
