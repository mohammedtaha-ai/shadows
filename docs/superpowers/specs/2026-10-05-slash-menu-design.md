# Section 21 — The `/` Menu

- **Date:** 2026-10-05.
- **Status:** Accepted by Mohammed on 2026-10-05, after an independent review
  (`a499a6c`), and amended by his ruling on `/model` after the browser run
  (21.4). Built on `next/slash-menu` and run in the browser against the real
  adapter (2026-10-05).
- **Evidence:** [`2026-10-05-steering-and-commands-probe.md`](../../evidence/2026-10-05-steering-and-commands-probe.md)
  finding 4 (adapter 0.81.1).
- **Related owners:** §12.4 (choices come from the harness; this section
  follows the same path), §2.10 (the thread stream), §20 (the composer while a
  turn runs).

This section owns the menu that opens when the person types `/` at the start
of the composer: the commands and skills the harness offers, filtered as they
type, each with its description. It copies Claude Code's desktop app. It shows
**only what the harness sends**. Shadows adds no commands of its own here:
model and effort already have their pickers (§12.11), and Shadows' own skills
and their management are future work (vision §13).

## 21.1 What the adapter gives

`available_commands_update` arrived right after `session/new` and again at
the first prompt (measured). The adapter's code (`dist/acp-agent.js`, 0.81.1,
read, not measured) also sends it right after `session/resume` and
`session/load`, and mid-session when Claude's list changes (skills found as
it works); not after `session/fork`, so a fork's first opening has no list
until its first prompt. It is sent just after the answer to the request, as a
notification. Each one is the **complete** list. An entry is
`{name, description, input?}`, with `input` as `{hint}`. Skills
(`superpowers:brainstorming`), plugin commands (`codex:review`) and built-in
commands (`compact`, `context`, `model`, `init` …) come mixed, with no field
saying which is which. A command runs when the prompt's text starts with
`/<name>`: the adapter runs it, not Shadows.

`agent-client-protocol` 2.2.0 types it:
`SessionUpdate::AvailableCommandsUpdate`, whose `available_commands` are `AvailableCommand { name, description, input }`
with `input` as `AvailableCommandInput::Unstructured { hint }`. The crate drops
an entry it cannot read rather than the whole update. Both types are
`non_exhaustive`.

## 21.2 Where the list lives

In the daemon's memory, never in SQLite. A new file `harness/commands.rs`
has one job: the latest command list of each open session. It works as
`harness/offers.rs` does for choices:

- every `available_commands_update` replaces the thread's list and is sent on
  `commands.rs`' own broadcast channel, as `offers.rs` sends on its own (not
  the turn's transient bus: the update also arrives between turns);
- the list is forgotten wherever the thread's choices are (`offers.forget`:
  the session closes or idles out, its adapter is found dead, it fails to
  open, the thread is deleted or switches harness). Forgetting publishes
  nothing;
- a restarted daemon has none until the session opens again, and the
  adapter then sends it again.

`shadows-agent`'s `forward` maps the update to a new
`HarnessEvent::Commands(Vec<SlashCommand>)`, with `SlashCommand { name,
description, hint: Option<String> }` (not `Command`: "command" already names
the idempotent `CommandId`). `commands.rs` takes it off the connection with
its own wrapper in the event chain `Sessions` builds at opening, as
`titles.rs`' `keep_titles` does, so it is recorded in the connection's own
dispatch, between turns too, and `offers.rs` keeps its one job. The new file
gets its code-map row and its line in the harness `contract.yaml`.
`sessions.rs` is at 500 lines and forgets offers and setups as a pair in six
places; the commands join one `forget` there instead of a seventh line each.

## 21.3 How the browser gets it

**SSE frame `commands`** — `{thread_id, commands}` on the thread stream
(`GET /api/subscribe?thread_id=…`), transient:

- sent right after `caught-up` when the thread has a list, and again whenever
  it changes. `Events::subscribe` takes the commands receiver with the
  options one, before anything is read, so an update that lands while the
  stream opens is delivered after the snapshot, never lost;
- every resubscription (a shown page again, §2.10; after `lagged`; after a
  restart) gets it again the same way. There is no route: nothing reads the
  list but the stream.

The client keeps the last list it was given for the thread. A stream that
sends none (the session idled out, the daemon restarted) leaves it, since the
next opening sends the list again; switching harness (§12.6) clears it.

## 21.4 The composer

- The menu is open while the composer's whole text is `/` followed by no
  whitespace (`/`, `/br`), from the first character. A space or a newline
  closes it, and so does a `/` anywhere but first. Escape closes it and leaves
  the text as it is; it opens again on the next change to the text that
  still qualifies.
- **Filter**, on the text after `/`, case-insensitive, in this order: names
  that start with it; names whose part after the last `:` starts with it
  (`/br` finds `superpowers:brainstorming`); names that contain it. Within
  each group, the adapter's order. With no match the menu closes.
- **Keys** while it is open: Up and Down move, Enter or Tab picks (Enter does
  not send), Escape closes. The mouse picks too. Keys during IME composition
  are the input method's, as Enter is today. Closed, Enter sends as it does
  now, so `/unknown` is sent as text.
- The highlighted entry's description shows beside the list. Names are
  laid out left to right; a description takes its own direction
  (`dir="auto"`), so an Arabic one reads right to left.
- **Picking** replaces the text with `/<name> ` and puts the caret at its end.
  If the entry has a `hint`, it shows greyed after the caret until the person
  types. It is not the textarea's `placeholder`, which shows only while the
  text is empty.
- The person sends it as any message, and it follows §20: while a turn runs it
  waits in the queue, and Send now steers it in. A steered command runs (the
  browser run: a queued `/context` sent with Send now answered its table and
  ended the turn, cutting the reply it pre-empted, as any steer does, §20.1).
- **`model` and `effort` act through the pickers, never as text.** The
  adapter changes its session for `/model` and sends no
  `config_option_update` (browser run: `/model sonnet` answered "Set model
  to Sonnet 5.5" while the picker kept Opus 5.5, after a reload too), so sent
  as text they would leave the picker showing a model the session no longer
  runs. So, decided by Mohammed on 2026-10-05:
  - picking `model` or `effort` from the menu, or sending `/model` or
    `/effort` with nothing after it, opens that picker and clears the box;
  - sending one with a name sets the picker to the enabled choice whose id or
    label is the name, or else the first whose id or label contains it,
    ignoring case; the picker then sets the session as a pick does (§12.7);
  - a name the session does not offer, or no session yet, is an error under
    the box; the text stays and nothing is sent.
- With an empty list (no session yet, or the adapter sent none), `/` opens
  nothing and is typed as text.

## 21.5 Tests

- `fake-acp` sends a small `available_commands_update` after `session/new`.
- A daemon test: opening a session sends a `commands` frame to a subscribed
  stream, and a stream subscribed afterwards gets the list after
  `caught-up`.
- Web tests: when the menu opens and closes, the filter order, the keys,
  picking with and without a hint.
- The browser run: on a database copy, against the real adapter, the menu
  opens, `/br` finds brainstorming, a picked `/context` sent as a message
  runs, the menu opens again after a reload, `/model <name>` moves the
  picker and sends nothing, picking `model` opens the picker, and
  `/context` queued during a turn and sent with Send now runs.
