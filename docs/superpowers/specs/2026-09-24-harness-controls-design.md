# Section 12 — Harness Controls over ACP (Milestone 1)

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there.

- **Date:** 2026-09-24
- **Status:** Redesigned with Mohammed on 2026-09-24 around the Agent Client Protocol. The first draft (`2cc7930`) talked to `claude --print` directly and kept a hand-written catalogue; both are replaced here. Amended the same day after the Phase B run on the real harness, with Mohammed's rulings: what Accept edits allows (§12.5), a fork is locked to its harness from creation (§12.6, §12.9), and the model is set when it is chosen (§12.7).
- **Builds on:** Milestone 0 (§11.1), merged at `58fb30e`.

Milestone 1 runs the Planner's harness over the **Agent Client Protocol (ACP)**
through a maintained adapter instead of reading Claude Code's stream ourselves.
On that connection the person chooses, per conversation, which agent CLI runs it
and, per message, which model, mode and effort; sees how full the context is and
how much of the account's limits are left; and can copy any message and fork
from the last one. Model and effort lists come from the harness, so a new model
appears without a Shadows release. Claude Code only; Codex is listed and not yet
runnable.

## 12.1 Scope

In, in two phases:

- **Phase A — the connection.** Milestone 0's Planner turn, unchanged in what a
  person sees, now runs over ACP (§12.2, §12.3). Milestone 0's stream-json path
  is deleted, not kept beside it. A session opens in `acceptEdits` (§12.4's
  default) and otherwise at the harness's own model and effort; choosing them
  is Phase B.
- **Phase B — the controls,** on that connection:
  1. **CLI choice** in the conversation header, locked once the conversation has run a turn (§12.6).
  2. **Composer bar** under the message box: mode, folder, model, effort, context ring (§12.11).
  3. **Choices from the harness** (§12.4) filtered by Shadows' mode policy and the project's allowed modes (§12.5).
  4. **Turn start as one command** with its settings recorded (§12.7).
  5. **Context and account limits** (§12.8).
  6. **Copy** on every message; **fork** from the last message, designed to widen to any message later (§12.9).
  7. **A refused permission is shown** in the conversation (§12.3).

Out:

| Not in this milestone | Why |
|---|---|
| Running Codex | Its adapter (`@agentclientprotocol/codex-acp`) is not measured, and its modes are not decided (§12.4). Listed `available: false`. |
| Claude's `plan`, `default`, `dontAsk`, `bypassPermissions` modes | Not in Shadows' policy for Claude (§12.4); decided 2026-09-24. Planning is the Planner's own job (§11.6 slice 1). |
| Asking the person to approve a permission | The request reaches Shadows (§12.3), so this is a client feature on an existing path, not new plumbing. A later slice. |
| Shadows' own tools (an MCP server) | ACP carries them (`mcpServers` on `session/new`); the first user is the Planner's workflow tools, not this milestone. |
| Fork, edit, or retry from a message before the last | Needs the Context Compiler (§2.11). The fork request already names the entry (§12.9). |
| Packaging the adapter as one executable | Node runs it (§12.2). **OPEN** there. |

## 12.2 The harness connection

**The protocol.** Shadows is an ACP *client*. It uses the official Rust crate
`agent-client-protocol`, pinned to one version; it does not implement JSON-RPC
or the ACP message shapes itself (library-first, CLAUDE.md).

**The adapter.** Claude Code does not speak ACP. The adapter
`@agentclientprotocol/claude-agent-acp` (Apache-2.0, maintained by the ACP
project, built on the Claude Agent SDK) does. It is **installed, not copied**:

- `harness/claude/package.json` pins the exact adapter version, and
  `harness/claude/package-lock.json` is committed. `harness/` belongs to the
  daemon; it is not part of `web/` and no client imports it. A later desktop
  client talks to the daemon the same way the browser does.
- Updating the adapter is a commit that changes the pinned version, after its
  tests and a run. An upstream release never reaches Shadows on its own.
- Shadows does not patch the adapter. What it dislikes it fixes on its own side
  (§12.8 is the first case). A defect that can only be fixed inside the adapter
  is reported upstream; a local patch is a decision taken case by case and
  recorded where it applies.

**Running it.** The daemon starts `node <adapter entry>` through `process/`
(stdin and stdout piped, Job Object containment as every managed child, §1.5).
Configuration names three absolute paths, never resolved through `PATH` (§1.4):
Node, the adapter entry (`harness/claude/node_modules/@agentclientprotocol/claude-agent-acp/dist/index.js`),
and the Claude Code executable. The last is passed to the adapter as
`CLAUDE_CODE_EXECUTABLE`, so the Claude that runs is the one configured and not
the copy the Agent SDK can carry. Every invocation records the adapter's version
and Claude's (§12.7).

> **OPEN — the adapter needs Node at run time.** A person running the daemon from
> source has Node already (the web client needs it). A packaged desktop client
> cannot assume it. **Trigger that closes this:** the first packaged desktop
> build. The candidate is compiling the adapter to one executable (the Codex
> adapter already ships that way); only how the process is started changes, not
> the connection.

**One adapter per open conversation.**

1. **Opening.** A client opens a conversation's harness session with
   `POST /api/threads/{id}/session` (§12.10) when it shows the conversation, so
   the menus are ready before the first message; a turn start on a thread whose
   session is not open opens it first (§12.7). The daemon starts that thread's
   adapter if none is live, sends `initialize`, then `session/new` (a thread
   with no harness session), `session/resume` (a thread with one), or
   `session/fork` (a fork's first opening, §12.9), with the project directory
   as `cwd`. The answer carries the choices the harness offers (§12.4).
2. **Idle.** An adapter with no turn running for **15 minutes** is closed. The
   next opening starts a new one and resumes the same harness session if its id
   was recorded by a finished turn (§12.3). Before the first turn finishes,
   closing the adapter discards its unrecorded session id; the next opening
   starts a new session.
3. **Switching harness** before the first turn (§12.6) closes the thread's
   adapter.
4. **Shutdown** (§8.5) stops every running turn, then closes every adapter.
   Closing is terminating the tree through its containment handle and reaping it.
5. **Restart.** Adapters do not outlive the daemon. Recovery (§8.6) is
   unchanged: a turn a previous runtime left non-terminal becomes `Interrupted`,
   and the conversation continues by resuming its harness session.

What `session/new` is given in this milestone: the project directory, no MCP
servers, and no `_meta.claudeCode.options`. Milestone 0 ran Claude with its
default setting sources and tool set, and so does this one.

**Every opening sets the mode.** A session starts in the mode of the person's
own Claude settings, and a resumed one comes back at those defaults rather than
at the settings it last ran with (`docs/evidence/harness/ACP_PROBE.md` at `92e6dae` §1, §4).
So after `session/new`, `session/resume` or a fork's opening, Shadows sets the
mode to the policy's default before anything else, and a turn sets model,
effort and mode again whenever they differ from what the session reports
(§12.7).

The adapter writes diagnostics to stderr; the daemon forwards each line to its
debug log (target `harness.stderr`, which `--debug` enables).

**An adapter has 20 s to open its session** (`setup_wait`): starting, ACP
`initialize`, `session/new`, `session/resume` or the fork, and the opening
settings. Past that the opening fails and its tree is terminated. Claude Code
reads the person's own configuration when it starts, so its startup has no
fixed cost: on Windows an opening took 2.6–5.7 s, nearly all of it the SDK's
`sdk-initialize` phase (4.2 s in one), and the first bound of 5 s refused
openings in Milestone 1's and Milestone 2's runs
(`docs/evidence/milestone2/WINDOWS_RUN.md` at `92e6dae`).

**Permission requests are refused.** The adapter asks the client before a tool
runs that the mode does not already allow (`session/request_permission`). In
this milestone Shadows answers every such request with the harness's reject
option, and writes a durable `PermissionRefused` entry to the thread naming
what was asked ("run `npm install`"), so the person sees why the turn went the
way it did and that `auto` is the mode that would have allowed it. `auto` itself
does not ask. This keeps Milestone 0's behaviour, where
`--permission-prompts none` denied the same requests without a trace.

## 12.3 A turn over the connection

This section replaces what Milestone 0's stream-json reader decided about a
turn. §2.3, §2.7 and §8.3–§8.5 still hold; where they say "process", read the
table below.

| Milestone 0 (a process per turn) | Milestone 1 (a prompt on a live connection) |
|---|---|
| spawn `claude --print` | the thread's adapter is open (§12.2), then `session/prompt` is sent |
| handle registered, then `Running` | the prompt is registered in `LiveHandles`, then `Running` |
| `stream_event` text deltas | `session/update` chunks: transient, rendered, never stored |
| `assistant` / `user` lines → entries | one durable entry per agent message, keyed by the update's `messageId` and written when that message is complete (the next message begins, or the turn ends); a tool call is its own entry, written when its `tool_call_update` reports `completed` or `failed` (or the turn ends), under the last title it was given — the first title is generic ("Terminal") and the command arrives later |
| `result` line + exit status | the `session/prompt` response: `stopReason` `end_turn` is success; `max_tokens`, `max_turn_requests` and `refusal` are recorded as failures naming the reason; a JSON-RPC error is a failure carrying its message |
| session recorded at turn-end | unchanged: the harness session id is recorded on the thread when its first turn ends, never earlier |
| the adapter exits mid-turn | `Failed { stage: Run }`, "the harness exited during the turn" |

**Stop.** `PlannerTurn::stop` keeps §2.3's rule that `Cancelled` means confirmed:

1. Record the cancellation request (TX #1), as today.
2. Send `session/cancel`.
3. **The harness confirms:** the prompt answers `stopReason: cancelled` within
   **10 seconds**. The adapter owns the Claude process and has ended the turn;
   write `Cancelled`, with the harness's answer as the confirmation.
4. **The harness does not confirm** in time: terminate the adapter's tree
   through its containment handle, reap it, then write `Cancelled` — Milestone
   0's path. The thread's next opening starts a new adapter.
5. **Termination fails:** §8.4 case 6 unchanged; nothing terminal is written.

§8.4 case 4 (the turn ended on its own first) keeps its meaning: a prompt that
answered before the cancel took ownership keeps the ending it answered with.

**What is deleted.** `agent/claude.rs`'s stream-json `classify`, the
`--print` argument builder, `StreamItem`'s line classes that only stream-json
produced, and `src/bin/fake_claude.rs`. Tests run against a fake ACP agent
(the `fake-acp` crate since Milestone 2.5, built on `agent-client-protocol`'s agent side) instead.

**Measured on 2026-09-24** against adapter 0.81.1 and Claude Code 2.1.281
(`docs/evidence/harness/ACP_PROBE.md` at `92e6dae`): chunks carry `messageId`; a cancel
answers `cancelled` in about 60 ms; `session/resume` after the adapter was
killed continues the conversation; a rejected permission ends the tool call
`failed` and the turn `end_turn`. This section rests on those findings.

**The session's title.** At a turn's end the adapter asks Claude Code to
generate a title for the session (`generate_session_title`, at most once per
session, in the background) and sends it as a `session/update` of kind
`session_info_update` carrying `title` (and `updatedAt`). When it cannot
generate one yet, it may send the stored summary, which is the raw first prompt.
The same kind also arrives with only `_meta` (a goal, a file-change report);
Shadows reads the `title` and nothing else. Because the title is generated
after `session/prompt` has answered, it arrives when no turn reads the thread's
events: the connection's own dispatch hands it to Threads, which applies §4.2's
title rule and journals `ThreadRetitled`. Titles from one adapter are written in
the order it sent them.

## 12.4 Choices come from the harness

The model, mode and effort lists are the harness's, read from the ACP session:
`configOptions` on `session/new`, `session/resume` and `session/fork`, on every
`session/set_config_option` answer, and on every `config_option_update`
notification. Each answer is the complete set, so choosing a model also
replaces the effort levels on offer. Shadows keeps no list of Claude's models,
nor of their efforts.

**No `default`.** Unasked, the adapter offers an effort `default` and a model
`default`. The effort `default` is no level: Claude Code picks one per model
(medium for some, high or xhigh for others) and the session goes on reporting
`default`, so a person could not see what a turn ran at
(`docs/evidence/harness/EFFORT_DEFAULT_PROBE.md` at `92e6dae` §1). So at `initialize`
Shadows advertises the adapter's `recommendedValue` extension in the client
capabilities' `_meta`:
`{"jetbrains":{"air":{"version":1,"capabilities":["recommendedValue"]}}}`.
With it the adapter offers neither `default`, and a session's effort starts at
a real level the adapter chooses and reports (`medium` when the model offers
it, measured in §2 of the same file). Shadows removes nothing from the menus
itself: they stay exactly what the harness reports.

| ACP option | Shadows reads it as |
|---|---|
| `category: model` | the model menu, in the harness's order |
| `category: thought_level` | the effort menu for the current model; **absent** for a model that offers no effort (Haiku 4.5 on 2026-09-24), and then the turn's effort is none |
| `category: mode` | the mode menu, **after** the filter below |
| anything else (`fast`, a `model_config`) | ignored in this milestone |

**Listed is not usable.** The harness lists models the account may not run:
when this section was written, Fable 5.1 was listed and selecting it was refused
with "Usage credits are required for this model". Access belongs to the account
and changes: in the Phase B run the same model was accepted and answered a turn
(`docs/evidence/milestone1/PHASE_B_RUN.md` at `92e6dae`). Shadows shows the list as the harness gives it and
reports the harness's refusal, in its own words, when a model is chosen
(`SettingNotOffered` carrying the message, §12.7).

**Shadows' mode policy** is per harness, because modes are:

| Harness | Modes Shadows allows | Default of a new conversation |
|---|---|---|
| Claude Code | `acceptEdits`, `auto` | `acceptEdits` |
| Codex | decided when Codex is enabled | — |

A mode reaches the menu only if it is in that policy, the harness lists it, and
the project allows it (§12.5).

**`auto` per model.** The harness keeps `auto` listed for every model, but a
model without auto mode moves the session to `acceptEdits` when it is chosen
(Haiku 4.5, measured). So after a model change Shadows reads the mode the
session reports: if it is not the mode the person had, the menu shows the
session's mode and the client says the model moved it. A turn that still asks
for `auto` on such a model fails at `Prepare` with the harness's message.

**Remembered settings.** Efforts belong to a model, so they are remembered per
model. The daemon keeps, per harness, the model of the last turn it started
(`harness_preference`), and per harness and model the effort of the last turn
started on that model (`harness_model_effort`, migration 0011, which carried
each harness's earlier remembered effort over to its remembered model, except
an effort `default`, which was no level, and any effort of the model
`default`). Both
are written by the turn start's own transaction; choosing a model or an effort
without sending records nothing. They are applied:

- when a session opens: it is set to the remembered model, then to the
  remembered effort of the model it then holds;
- when a person picks a model (§12.7) and the session moves to it: to that
  model's remembered effort.

A remembered value the harness no longer offers (a model `default` remembered
before `recommendedValue`, say) or refuses is dropped, and the harness's
current value stands; so a model never used starts at the effort the adapter
reports for it: `medium` in a new session, and inside a session the effort it
last had, which the adapter carries over to a model that offers it (same
evidence file, §3). `GET /api/harnesses` answers the remembered model and that
model's remembered effort. The mode is not remembered: every new conversation
starts at the policy's default.

**The harness list** itself — which CLIs exist, which run — is Shadows': Claude
Code (available when its three paths are configured), Codex (not yet).

## 12.5 Allowed modes per project

A mode is a permission decision, not a display preference: `auto` lets the
harness act on its own judgement where `acceptEdits` does not. Each project
states which modes its turns may use:

- `project_mode(project_id, harness_kind, mode_id)`, primary key over all three. A project is created with every mode of its harness's policy (§12.4); the migration gives existing projects the same. An empty set for a harness means no turn can start on it, and the client says so.
- The daemon checks the requested mode against the project's set at turn start, before anything durable is written (`ModeNotAllowed`). What the client offered or disabled is not the check.
- Changing the set is its own command (`PATCH /api/projects/{id}` with a `CommandId`), and only modes in the harness's policy are accepted.
- The set is a list of named modes per harness, not an ordering, so it claims nothing about how one mode's authority compares with another's or with a second harness's.

> **OPEN — Shadows imposes no boundary of its own.** §8.2 freezes "effective
> read / write / network permissions" with every invocation. In this milestone
> those are exactly the harness's mode: Shadows does not confine the Planner to
> the project directory, the network, or a tool set beyond what the harness
> enforces, and must not claim it does. The invocation records the mode as the
> whole of it. The place to enforce more already exists — every
> `session/request_permission` reaches the daemon (§12.2) — and so do the
> options to narrow the tool set at `session/new`. **Trigger that closes this:**
> the first role or mode whose limits Shadows must enforce itself — at the
> latest §11.6 slice 2, where an Executor runs against a declared write scope.
> It does not block this milestone: the Planner is a person's own agent, run in
> the directory that person chose, in one of two modes that person allowed.

**What Accept edits allows is Claude Code's, and the client says so.** Measured
in the Phase B run (`docs/evidence/milestone1/PHASE_B_RUN.md` at `92e6dae`): in `acceptEdits`,
Claude Code runs file commands inside the project directory without a
permission request — `rm -rf ./README.md` deleted a file that was never
committed, and Shadows never saw a request. A command outside that, such as
`curl` to the network, is asked, and Shadows refuses it (§12.2). Decided with
Mohammed on 2026-09-24: Shadows keeps the harness's behaviour and does not add
a boundary of its own here (the OPEN block above still names when it must).
The mode menu states, on the mode itself, what each mode lets the harness do
(§12.11), so the choice is made knowing it.

## 12.6 A conversation's harness

`PlanningThread.harness` is chosen when the thread is created, defaults to
`claude-code`, may be changed while the thread has no Operation, and is fixed
from its first Operation on (`HarnessLocked`). A fork is fixed from creation
(§12.9): it continues its source's harness session, which only that harness
can continue. A harness session belongs to one
harness; a thread whose turns alternated harnesses would have no session to
continue. Moving a conversation between harnesses is §2.11's continuity, not a
setting. A trigger enforces the lock in storage, below the application.

## 12.7 Starting a turn as one command

`POST /api/threads/{id}/turns` carries `{ command_id, prompt, model, mode, effort }`;
`effort` is `null` exactly when the chosen model offers none (§12.4).

**Validation, all before any write:** the thread's harness is available
(`HarnessUnavailable`); its project's directory exists (`PathNotFound`, 409,
whose message is the reason — it is about a directory the user chose, so it is
public-safe, §3.2); its session is open, and is opened here if it is not
(`HarnessStartFailed` when that fails — opening starts a process and issues
the thread's MCP grant, §13.7, but writes nothing of the turn's, §12.2); model, mode and effort are among the choices the session
offers now, the effort is one the chosen model offers, and the mode passes §12.4's
policy (`SettingNotOffered`); the mode is in the project's allowed set
(`ModeNotAllowed`); no turn is running on the thread (`ThreadBusy`).
Efforts belong to a model, so a turn naming another model than the session
holds sets it before these checks; when a later check or the transaction
refuses the turn, the session's model is set back, so a refused turn leaves the
session as it found it.

**One transaction** then commits, together or not at all (§2.1, §6.20):

```text
TX
  user ThreadEntry (the prompt)
  + Operation(Pending)
  + agent_invocation (frozen, below)
  + durable events
  + CommandRecord(command_id, kind, fingerprint, result = operation id + entry id)
COMMIT
```

This replaces Milestone 0's two writes (the entry in `protocol/conversation.rs`,
then `Pending` in `planner/spawn.rs`), which a retried request could turn into a
second message and a second run.

**The model is set during validation.** Efforts belong to a model, so a turn
that names another model than the session holds sets it with
`session/set_config_option` before the checks, and checks the effort against
the answer. The harness refusing the model (an account without access to it)
is `SettingNotOffered` with the harness's message. That changes the session, not the durable record; a later refusal
leaves the session on the new model, and clients see it as an `options` frame.

**Choosing a model sets it at once.** Efforts belong to a model, and the
session only reports a model's efforts once it holds that model. So the client
does not wait for Send: `PUT /api/threads/{id}/session/model` with `{ model }`
opens the session if needed, sets the model when it differs, and answers the
choices as `POST /session` does, the new model's efforts included. The change
writes nothing durable and carries no `CommandId`: setting the same model twice is
the same state, and a turn records its model in its own invocation. An opening
it causes issues the thread's grant, as any opening does (§13.7). It is
`SettingNotOffered` for a model not on offer or refused by the harness (with
the harness's message), and `ThreadBusy` while a turn runs, since a running
turn's session is not changed under it. It does not write the remembered
settings, which follow turns started (§12.4); when the session moves to the
model, that model's remembered effort is set, if it is still offered. Turn
start keeps setting the model during validation, for any client that did not.

**Choosing an effort sets it at once,** for the same reason: the session then
reports the effort a turn will run at. `PUT /api/threads/{id}/session/effort`
with `{ effort }` mirrors the model route: it opens the session if needed, sets
the effort with `session/set_config_option` when it differs, and answers the
choices as `POST /session` does. It writes nothing durable and carries no
`CommandId`. It is `SettingNotOffered` for an effort the current model does not
offer (a model with no effort offers none) or one the harness refuses (with its
message), and `ThreadBusy` while a turn runs. Measured: a chosen effort reaches
Claude Code (`docs/evidence/harness/EFFORT_DEFAULT_PROBE.md` at `92e6dae` §2).

**Then, before the prompt is sent,** the session is set to the turn's effort
and mode, one call per value that differs from what the session holds — for
any client that did not set the effort at once. A
refusal there fails the turn at `Prepare`, naming the setting; nothing was sent
to the model.

**Replay.** A request whose `command_id` has a record and whose fingerprint
matches returns the recorded result and starts nothing — answered before any
other check, including the daemon stopping, and without opening a session,
because the command already happened. A matching `command_id` with a different
fingerprint is `CommandConflict`. The fingerprint covers the thread id, prompt,
model, mode and effort, and from Milestone 2 the turn's `focus` (§13.10).

**`agent_invocation`** is created in that transaction, before anything is sent
(§2.7, §8.2). It answers §6.15's OPEN block with the explicit columns §6.15
already preferred:

```text
agent_invocation
id                  TEXT PRIMARY KEY
operation_id        TEXT NOT NULL UNIQUE FK operation(id) ON DELETE RESTRICT
role                TEXT NOT NULL
harness_kind        TEXT NOT NULL
harness_path        TEXT NOT NULL      -- the adapter entry
harness_version     TEXT NOT NULL      -- the adapter's package version
agent_path          TEXT NOT NULL      -- the CLI the adapter runs (CLAUDE_CODE_EXECUTABLE)
agent_version       TEXT NOT NULL      -- that CLI's --version
requested_model     TEXT NOT NULL
requested_mode      TEXT NOT NULL      -- the permission mode requested of the harness
requested_effort    TEXT NULL          -- NULL when the model offers no effort
profile_json        TEXT NOT NULL      -- '{}' until profiles exist
native_session_id   TEXT NULL
observed_model      TEXT NULL
context_used        INTEGER NULL
context_window      INTEGER NULL
created_at          TEXT NOT NULL
```

`requested_mode` is what Shadows asked the harness for, not a record of the
rules and settings the harness then applied. §8.2's "effective read / write /
network permissions" stay unrecorded and are §12.5's OPEN block.

The `requested_*`, path and version columns never change. The rest is written
once, in the transaction that records the turn's terminal transition, from what
the harness reported during the turn. Nothing it did not report is estimated;
it stays NULL and a client shows it as unavailable.

- `observed_model` is the model the harness names in `_meta["_claude/model"]` on the turn's last usage report — a model id such as `claude-sonnet-5`, where `requested_model` holds the option value the person chose (`sonnet`). It can name another model than the one asked for: on 2026-09-24 a turn that asked for `haiku` through `claude --print` was answered by `claude-sonnet-5`.

**Every entry names the turn it belongs to.** `thread_entry` gains
`operation_id TEXT NULL FK operation(id) ON DELETE RESTRICT`: the user entry the
turn command writes, and every entry the turn produces, carry that turn's
operation id. Milestone 0's entries carry none (NULL), since which turn wrote
them can only be inferred from timing, and an inference is not recorded as
fact. This is the durable link §12.9's fork rule reads.

## 12.8 Context and account limits

**Context.** The adapter reports `usage_update { used, size }` during and after a
turn. Shadows keeps the latest for the session and writes the turn's last one
into its invocation (`context_used`, `context_window`). Only the last one of a
turn is trusted: the adapter first reports its guess of the window (200k) and
corrects it after the turn's result (1M, measured). A report with no usable
`size` is not shown as a percentage.

**Account limits.** The adapter forwards Claude's rate-limit report in
`_meta["_claude/rateLimit"]` on a `usage_update`: five-hour and seven-day
utilisation, each with its reset time. The limits are account-wide, so the
daemon keeps the latest per harness with the time it was observed
(`harness_limit`, one row per harness, latest wins; not journal data). The
adapter drops a report that arrives before the session's first answer; the
kept row is what covers that gap.

**The ring never waits.** It shows the last figures the daemon holds and the
time they were observed, opens at any moment, and says "no figures yet" when it
has none. There is no loading state. (The adapter's context breakdown, read
through a control request that the adapter's own source says can stall a
session for tens of seconds, is why a ring elsewhere was seen spinning.)

**Two levels, as agreed on 2026-09-24:**

1. **Summary:** context `used / size (percent)`; five-hour and weekly bars with
   reset times; "updated <time>". No credit balance: that belongs to the claude.ai
   account and does not reach the CLI.
2. **Breakdown:** messages, system tools, MCP tools, skills, memory files, system
   prompt, custom agents, autocompact buffer, free space.

**The breakdown is read on demand.** Sending `/context` as a prompt answers
the breakdown as one markdown message, costs no model tokens, and takes 0.6 s
on a session that has already answered once — but 17–23 s as a fresh session's
first call (ACP_PROBE §5). So the breakdown is fetched only when the person opens
the ring's second level (`GET /api/threads/{id}/context`), only on an open,
idle session that has run a turn in this adapter, with a 5-second limit. It is
parsed from the "Estimated usage by category" table, shown, and not stored.
While a turn runs, before the session's first turn, or past the limit, the
second level says why it has nothing, and the summary still shows. The
`/context` exchange is not a Planner turn: it writes no Operation and no entry,
and its chunks never reach the conversation.

> **OPEN — does `/context` enter the conversation Claude remembers?** The probe
> did not establish whether the exchange is written into the session transcript
> that `session/resume` replays to the model. **Trigger that closes this:** the
> Phase B run (§12.13). Read the session transcript after a `/context` and
> resume; if the exchange is there, the breakdown route is removed and the ring
> ships with the summary only. It does not block Phase A.

Delivery: the thread snapshot carries each operation's invocation; the
operation's terminal event carries the observed values; `GET /api/harnesses`
carries the latest limits; a transient `usage` SSE frame pushes a new context
or limits report as it arrives.

## 12.9 Fork

`POST /api/threads/{id}/fork` with `{ command_id, at_entry_id }` creates, as one
command, a new thread in the same project on the same harness, titled after the
source with "(fork)":

- It copies the source's entries up to and including `at_entry_id` under new ids and ordinals. It copies no Operation: an Operation belongs to the thread that ran it.
- **Copied entries keep their `operation_id` and `refs` unchanged.** Both still name the source thread's operation; that is provenance, read-only history, and is shown as such. Nothing in the fork can stop, retry, or otherwise act on it.
- It records `forked_from_thread`, `forked_from_entry`, and `fork_session_id` (the source's harness session at that moment). The three are all NULL or all set, and are written only by the creating insert. The source is not changed.
- **The fork is locked to the source's harness from creation** (§12.6): `fork_session_id` is a session of that harness, and no other can continue it. Changing it is `HarnessLocked`, and storage refuses it as it refuses a thread with an Operation.
- The fork's first opening (§12.2) calls `session/fork` on `fork_session_id`, then `session/resume` on the id it returns: the adapter answers a fork's id without making it live, and a prompt to it before the resume is "Session not found" (ACP_PROBE §6). The fork remembers the source's messages and the source is unaffected. The session is recorded as the fork's own `harness_session_id` when the fork's first turn ends, by the rule of §12.3.
- **Valid fork point in this milestone**, decided only from durable rows: `at_entry_id` is the source's highest-ordinal entry; its `operation_id` is set; that operation is `Completed`; and the source's `harness_session_id` is set. A turn running on the source is `ThreadBusy`; anything else is `ForkPointNotSupported`, including an entry with no `operation_id` and a turn that was stopped or failed. Widening it to any entry is a server change only, once the Context Compiler exists.

## 12.10 Protocol changes

| Route | Change |
|---|---|
| `GET /api/harnesses` | new: each harness's kind, label, availability with a reason, remembered model and effort, latest limits |
| `POST /api/threads/{id}/session` | new: open the thread's harness session (§12.2); answers the choices on offer — models, efforts of the current model, modes after §12.4's filter, current values. Idempotent: an open session answers what it holds. No `CommandId`; an opening issues the thread's MCP grant, a durable event (§13.7) |
| `POST /api/threads/{id}/turns` | body becomes `{ command_id, prompt, model, mode, effort }` (§12.7) |
| `PUT /api/threads/{id}/session/model` | new: `{ model }`; sets the open session's model when it is chosen and answers the choices as `POST /session` does (§12.7). The change is not durable and has no `CommandId`; an opening it causes issues the grant (§13.7) |
| `PUT /api/threads/{id}/session/effort` | new: `{ effort }`; sets the open session's effort when it is chosen and answers the choices as `POST /session` does (§12.7). Not durable, no `CommandId`, as the model route |
| `PATCH /api/threads/{id}` | new: `{ command_id, harness }`; `HarnessLocked` for a new command once an Operation exists, or at once for a fork (§12.6); a replay answers the thread as it stands |
| create thread | gains optional `harness` (default `claude-code`) |
| `GET /api/projects/{id}` | carries the allowed modes per harness |
| `PATCH /api/projects/{id}` | new: `{ command_id, allowed_modes }` |
| `POST /api/threads/{id}/fork` | new, §12.9 |
| `GET /api/threads/{id}/context` | new: the context breakdown read on demand (§12.8); answers the categories, or none with the reason |
| thread snapshot, operation events | carry the invocation's requested and observed values |
| entries | a new kind, `PermissionRefused` |
| SSE | new transient frames: `usage` (context and limits), `options` (the session's choices changed) |
| thread stream | a durable `ThreadRetitled` { title, source } when §4.2's rule changes the title; the web client reads the project's threads again on it. No project-level stream exists, and the harness's title usually lands after the person has left the conversation, so the web client also polls each open project's threads every 10 s, as it does its plans |

New stable error codes (§3.4):

```text
HarnessUnavailable     422  the thread's harness is listed but not runnable
HarnessStartFailed     502  the adapter did not start or did not answer initialize / session setup
SettingNotOffered      422  a model, mode or effort the session does not offer, or a mode outside the policy
ModeNotAllowed         403  the project does not allow this mode
HarnessLocked          409  the thread already ran a turn on its harness
ThreadBusy             409  the thread has a turn running
ForkPointNotSupported  422  fork from anything but the last completed entry
```

## 12.11 Web client

Matches the mockup agreed on 2026-09-24:

- **Opening a conversation** opens its session. Until the answer arrives the bar reads "Connecting to Claude Code…"; a failure shows the daemon's message and a retry. Never a spinner without words.
- **Header:** CLI picker (`cli-picker.tsx`). Unavailable harnesses shown disabled as "coming". Changeable until the first turn, then shown with a lock.
- **Under the message box**, flat, no border (`composer-bar.tsx`): left `+`, mode, folder; right model, effort, context ring. Every menu is built from the session's answer. Modes the project does not allow are shown disabled with the reason. Each mode carries one line saying what it lets the harness do; for Accept edits, that it edits, creates and deletes files in the project folder without asking and other commands are refused (§12.5). Choosing a model sets it at once (§12.7): the effort menu waits for the answer and then shows that model's efforts, at the effort the model route's answer reports (its remembered one, else the one the adapter carried over, §12.4) — an `options` frame streamed during the move may still carry the previous effort and does not settle it; a refusal puts the session's model back and shows the harness's message. Choosing an effort sets it at once too; a refusal puts the session's effort back and shows the message on the error line. When the observed model differs from the requested one, the reply shows both.
- **Context ring** (`context-ring.tsx`): §12.8's two levels; hover or click opens it at any time.
- **Message actions** (`message-actions.tsx`): copy on every message, fork on the last one when the thread is idle; fork opens the new thread.
- **A refused permission** renders as a quiet line: what was asked, that `acceptEdits` refused it, and that `auto` would allow it.
- **Allowed modes:** a per-harness checklist in Project settings (§13.11).
- New UI pieces are shadcn's `DropdownMenu` and `Tooltip` on the existing Base UI. No new library.

## 12.12 How the work is split

**Phase A** is one backend agent. The HTTP contract does not change in Phase A
except the `PermissionRefused` entry kind, so the web client is untouched.
Phase A ends with Mohammed running a conversation, a Stop, and a daemon restart
on the real adapter before Phase B starts.

**Phase B** is built by two agents that do not touch each other's tree; the
contract between them is `api/openapi.json`.

1. **Contract first.** The controller writes the draft `api/openapi.json` by hand from §12.10, including the SSE frame shapes, and commits it alone before any code.
2. **Backend agent** owns `src/`, `tests/`, `migrations/`, `harness/`, and at the end regenerates `api/openapi.json` (`UPDATE_OPENAPI=1 cargo test --test openapi`). Its report lists every difference between what it generated and the draft, with the reason for each. It never opens `web/`.
3. **Web agent** owns `web/` only. It generates its types from the draft and tests against a fake daemon built from it. It never runs the daemon or opens `src/`.
4. **A contract that is wrong or missing something** stops the agent that found it; it reports to the controller and changes neither the contract nor the other side.
5. **Integration.** Each agent works in its own worktree. The controller merges both into one branch; the generated `api/openapi.json` replaces the draft; the web types are regenerated, and any type error is fixed in `web/` only.

## 12.13 Acceptance

```text
Phase A
[ ] §12.3's measurements are recorded in docs/evidence/harness/ACP_PROBE.md at 92e6dae
[ ] a Planner turn runs over ACP on the pinned adapter; its reply streams live and its
    messages are durable, one entry per message
[ ] Stop ends a running turn: Cancelled after the harness confirms, or after the tree is
    reaped when it does not; never before either
[ ] a daemon restart leaves the conversation readable, and its next turn remembers it
[ ] a permission request is refused and shows as a PermissionRefused entry
[ ] the stream-json path and fake_claude are gone; tests run on fake_acp
[ ] Mohammed runs a conversation, a Stop and a restart on the real adapter

Phase B
[ ] GET /api/harnesses lists Claude Code (runnable) and Codex (not)
[ ] opening a session lists the models Claude offers today, with each model's efforts,
    and only the modes §12.4 lets through
[ ] a turn runs with the chosen model, mode and effort; its entry, operation, invocation and
    command record commit in one transaction; the invocation holds both paths and versions,
    requested and observed values
[ ] a replayed turn start with the same CommandId returns the first result and starts nothing,
    even after the project's allowed modes changed; a changed body is CommandConflict
[ ] a mode the project does not allow is refused by the daemon, not only hidden by the client
[ ] the harness can be changed before the first turn and not after, enforced in storage;
    a fork's cannot be changed at all
[ ] a new conversation starts at Accept edits and at the last model and effort used
[ ] the ring shows figures or "no figures yet" and never spins; limits show their observed time
[ ] the ring's second level shows the breakdown, or says why it has none; §12.8's OPEN
    block is closed by reading the transcript after a /context
[ ] a model the account cannot use is refused with the harness's message;
    a model with no effort (Haiku 4.5) runs with none
[ ] copy works on every message; fork from the last message opens a thread whose next turn
    remembers the conversation, and the source is unchanged
[ ] one whole-branch review before the PR
[ ] Mohammed runs the real daemon with the real client and exercises every item above
```
