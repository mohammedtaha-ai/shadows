# Section 21 — The `/` Menu

- **Date:** 2026-10-05.
- **Status:** Accepted in conversation by Mohammed on 2026-10-05; awaiting his
  review of this written text.
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

`available_commands_update` arrives right after `session/new` and again at
the first prompt. Each one is the **complete** list. An entry is
`{name, description, input?}`, with `input` as `{hint}`. Skills
(`superpowers:brainstorming`), plugin commands (`codex:review`) and built-in
commands (`compact`, `context`, `init` …) come mixed, with no field saying
which is which. A command runs when the prompt's text starts with
`/<name>`: the adapter runs it, not Shadows.

## 21.2 Where the list lives

In the daemon's memory, never in SQLite. A new file `harness/commands.rs`
has one job: the latest command list of each open session. It works as
`harness/offers.rs` does for choices:

- every `available_commands_update` replaces the thread's list and is
  published on the transient bus;
- the list is forgotten wherever the thread's choices are forgotten (the
  session closes, fails to open, or the thread is deleted);
- a restarted daemon has none until the session opens again, and the
  adapter then sends it again.

`shadows-agent` forwards the update as a new
`HarnessEvent::Commands(Vec<Command>)`, with `Command { name, description,
hint: Option<String> }`. It reaches `commands.rs` through the same intercept
that records options (`offers.rs::intercept`), because it also arrives
between turns.

## 21.3 How the browser gets it

- **`GET /api/threads/{id}/commands`** answers the current list, `[]` when the
  session has sent none. One `harness` method serves it.
- **SSE frame `commands`** — `{thread_id, commands}` on the thread stream,
  transient, whenever the list changes.

The web client subscribes first and then reads the list, so an update that
lands between the two is not lost. Neither the frame nor the route is
durable: nothing is replayed.

## 21.4 The composer

- The menu opens when the composer's text is exactly `/` followed by no
  whitespace, at the start of the box. A space, or a `/` elsewhere, closes
  it.
- **Filter**, on the text after `/`, case-insensitive, in this order: names
  that start with it; names whose part after the last `:` starts with it
  (`/br` finds `superpowers:brainstorming`); names that contain it. Within
  each group, the adapter's order.
- **Keys:** Up and Down move, Enter or Tab picks, Escape closes and leaves the
  text as it is. The mouse picks too.
- The highlighted entry's description shows beside the list.
- **Picking** replaces the text with `/<name> `. If the entry has a `hint`, it
  shows greyed as the composer's placeholder until the person types.
- The person sends it as any message, and it follows §20: while a turn runs it
  waits in the queue.
- With an empty list (no session yet, or the adapter sent none), `/` opens
  nothing and is typed as text.

## 21.5 Tests

- `fake-acp` sends a small `available_commands_update` after `session/new`.
- A daemon test: opening a session makes the list readable from the route,
  and the `commands` frame arrives.
- Web tests: the filter order, the keys, picking with and without a hint.
- The browser run: on a database copy, against the real adapter, the menu
  opens, `/br` finds brainstorming, and a picked `/compact` sent as a message
  runs.
