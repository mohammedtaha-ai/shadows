# Section 22 — Subagent Cards

- **Date:** 2026-10-05.
- **Status:** Accepted by Mohammed on 2026-10-05 (the inner tools show only
  inside the card; the card is stored as a conversation entry). Built on
  `next/subagent-cards`.
- **Evidence:** [`2026-10-05-subagent-over-acp-probe.md`](../../evidence/2026-10-05-subagent-over-acp-probe.md)
  (adapter 0.81.1), and the adapter's `dist/acp-agent.js` (read, not measured).
- **Related owners:** §12.2 (what a turn's entries are), §2.10 (the thread
  stream), §13.9 (the side panel the plan uses), §19 (a per-project list of
  background work, later).

This section owns how a subagent the harness launches is shown: one card in
the conversation with its task, model, state, duration and tokens, which opens
what it did in a side panel. It copies Claude Code's desktop app.

## 22.1 What the adapter gives

A subagent is one tool call on the parent session whose
`_meta.claudeCode.toolName` is `Agent` (or `Task`); its first `tool_call`
also carries `subagent: true`. Its `rawInput` fills in over later updates:
`description`, `subagent_type`, `model`, `prompt`. Its title starts as `Task`
and becomes the description.

The subagent's own tool calls stream on the same session, each first seen with
`_meta.claudeCode.parentToolUseId` naming the Agent call. Its last one is
`SubagentHandback`, whose input is the report. Its text is never streamed.

The Agent call's numbers come in an update the adapter sends from Claude's
`PostToolUse` hook, after the call's `completed`:
`_meta.claudeCode.toolResponse` with `resolvedModel`, `totalDurationMs`,
`totalTokens`, `totalToolUseCount` and `handbackReport.text`. A failed call
may never get one.

## 22.2 The daemon

`shadows-agent`'s `forward` adds to `HarnessEvent::ToolCall` what it read:
`tool` (`toolName`), `parent` (`parentToolUseId`), and, for an Agent call,
`agent: AgentFacts` (the input fields and the `toolResponse` numbers, each
`None` when absent). It translates; the rules are the collector's.

`turns/entries.rs`' collector keeps each open subagent as a
`SubagentCard { id, title, agent_type, model, status, prompt, steps,
report, duration_ms, tokens, tool_count }`:

- a call with `agent` facts, or one already known as a subagent, updates the
  card; `status` is `running`, then `completed` or `failed`;
- a call whose `parent` is an open card's id is that subagent's step: never
  an entry of its own (decided by Mohammed). When it ends, its title is
  appended to `steps`. `SubagentHandback` is not a step: it is the report;
- the card is written once, as a `Subagent` entry whose body is the card's
  title and whose `card` is the card, the way a tool is a `ToolCall` entry
  whose body is its title. §23.8 made both their own kinds (Mohammed,
  2026-10-08); before migration 0020 they were `AgentMessage` bodies
  `[subagent: <card JSON>]` and `[tool: <title>]`. It is written when its call has
  ended and its numbers have arrived; or, ended without numbers, when the
  parent's next text arrives, so the card stays before that text; or when the
  turn ends. A card still `running` when the turn ends is written `stopped`.

While it runs, every change to a card is published on the turn's bus as
`HarnessEvent::Subagent(card)`, which the watcher sends, not the connection,
as it sends `TurnEnd`.

## 22.3 The stream

**SSE frame `subagent`** — `{op, card}`, transient: the whole card each time,
so a stream opened mid-run has it complete from its next change. Its entry,
when written, arrives as any entry does.

## 22.4 The browser

- A `Subagent` entry, or a live card whose id no entry has yet, is drawn
  as a card: the task, `agent_type · model`, the state (running, done,
  failed, stopped), and `duration · tokens · steps`. A live card is dropped
  when its turn ends; the entry has replaced it.
- Clicking it opens a side panel, as a plan does: what it was asked (the
  prompt), its steps in order, and its report as Markdown.

## 22.5 Not here

The subagent's streamed text (the adapter does not send it), the AIR
`nativeSubagentSessions` capability (not advertised), stopping one subagent
alone, and a per-project list of background work (§19).

## 22.6 Tests

- The collector: an Agent call with two inner tools and a handback becomes
  one entry with two steps and its numbers; an inner tool writes no entry; a
  card ended without numbers is written before the next text; a running one
  is written `stopped` at the turn's end.
- `forward`: the facts are read from `_meta.claudeCode` and `rawInput`.
- Web: the card is drawn from an entry and from a live frame; the panel shows
  the prompt, the steps and the report.
- The browser run: on a database copy, against the real adapter, a prompt
  that launches one subagent shows a running card, then the finished card,
  which opens the panel, and still does after a reload.
