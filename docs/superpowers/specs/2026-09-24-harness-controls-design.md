# Section 12 — Harness Controls (Milestone 1)

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there.

- **Date:** 2026-09-24
- **Status:** Design agreed with Mohammed in conversation, revised after two Codex reviews; awaiting review of this file.
- **Builds on:** Milestone 0 (§11.1), merged at `58fb30e`.

Milestone 1 lets the person choose, per conversation, which agent CLI runs it,
and per message, which model, mode and effort it runs with; shows how full the
context is and how much of the account's limits are left; and adds copy and
fork to messages. It is built on Claude Code only. Codex is listed and not yet
runnable.

## 12.1 Scope

In:

1. **CLI choice** in the conversation header, locked once the conversation has run a turn (§12.4).
2. **Composer bar** under the message box: mode, folder, model, effort, context ring (§12.9).
3. **Per-harness catalogue** served by the daemon (§12.2) and **per-project allowed modes** (§12.3).
4. **Turn settings recorded** with every turn, in the same transaction that creates it (§12.5).
5. **Context and account limits** read from the harness stream (§12.6).
6. **Copy** on every message; **fork** from the last message, designed to widen to any message later (§12.7).
7. **`CommandId` on turn start**, with the user's message, the Operation and its invocation committed as one command (§12.5).

Out:

| Not in this milestone | Why |
|---|---|
| Running Codex | Its stream contract has not been measured the way Claude's was. Listed `available: false` (§12.2). |
| Claude's `plan` mode | Planning is the Planner's own job in Shadows (§11.6 slice 1). Claude's plan mode ends by asking a person to approve leaving it, which a `--print` turn cannot do. |
| Claude's `bypassPermissions` | Not needed; decided 2026-09-24. |
| Claude's `manual` mode | Needs an approval channel from the harness to a client and back (`--permission-prompt-tool`). A later slice. |
| Fork, edit, or retry from a message before the last | Needs the Context Compiler (§2.11) to rebuild a session up to that message. The fork request already names the entry (§12.7). |

## 12.2 The harness catalogue

Each harness declares the choices a turn may make on it, next to the flags it
translates them into (`agent/<harness>`). Clients read it from
`GET /api/harnesses` and build every menu from it; no client carries its own list.

Every choice has an `id`, a `label`, a one-line `description`, and `enabled`
with a `reason` when not. **Effort belongs to a model**: each model lists the
effort levels it accepts and its default, because the levels a model supports
differ by model.

**Claude Code** (measured against `claude` 2.1.278, 2026-09-24):

| Kind | id (= flag value) | Label | Notes |
|---|---|---|---|
| mode | `acceptEdits` | Accept edits | **default of every new conversation** |
| mode | `auto` | Auto | |
| model | `sonnet` | Sonnet | alias; **default** (Milestone 0's hard-coded value) |
| model | `opus` | Opus | alias |
| model | `haiku` | Haiku | alias |
| effort | from `low` `medium` `high` `xhigh` `max` | Low … Max | `--effort`; the subset and default per model are measured first (§12.7 box) |

The mode is passed as `--permission-mode`; `--permission-prompts none` stays,
so anything either mode would ask a person about is denied, never left waiting.

**Codex** is listed with `available: false` and no choices.

**The catalogue states what Shadows supports, not what the account may use.**
Aliases move as models are released, and a provider or organisation may refuse
or reroute one: on 2026-09-24 a turn asked for `--model haiku` and the stream
reported `claude-sonnet-5`. The catalogue is never presented as an availability
check. What a turn ran on is recorded (§12.5) and shown whenever it differs
from what was requested.

**Remembered settings.** The daemon keeps, per harness, the **model and effort**
of the last turn it started and returns them with the catalogue as the next
conversation's starting values. The **mode is not remembered**: every new
conversation starts at the catalogue's default mode. They live in the daemon so
every client, including a later desktop client, starts from the same place.

## 12.3 Allowed modes per project

A mode is a permission decision, not a display preference: `auto` lets the
harness act on its own judgement where `acceptEdits` does not. Each project
states which modes its turns may use:

- `project_mode(project_id, harness_kind, mode_id)`, primary key over all three. A project is created with every mode its harness's catalogue enables; the migration gives existing projects the same. An empty set for a harness means no turn can start on it, and the client says so.
- The daemon checks the requested mode against the project's set at turn start, before anything durable is written (`ModeNotAllowed`). What the client offered or disabled is not the check.
- Changing the set is its own command (`PATCH /api/projects/{id}` with a `CommandId`).
- The set is a list of named modes per harness, not an ordering, so it claims nothing about how one mode's authority compares with another's or with a second harness's.

> **OPEN — Shadows imposes no boundary of its own.** §8.2 freezes "effective
> read / write / network permissions" with every invocation. In this milestone
> those are exactly the harness's mode: Shadows does not confine the Planner to
> the project directory, the network, or a tool set outside what the harness
> itself enforces, and must not claim it does. The invocation records the mode
> as the whole of it. **Trigger that closes this:** the first role or mode whose
> limits Shadows must enforce itself — at the latest §11.6 slice 2, where an
> Executor runs against a declared write scope. It does not block this
> milestone: the Planner is a person's own agent, run in the directory that
> person chose, in one of two modes that person allowed.

## 12.4 A conversation's harness

`PlanningThread.harness` is chosen when the thread is created, defaults to
`claude-code`, may be changed while the thread has no Operation, and is fixed
from its first Operation on (`HarnessLocked`). A harness session belongs to one
harness; a thread whose turns alternated harnesses would have no session to
continue. Moving a conversation between harnesses is §2.11's continuity, not a
setting. A trigger enforces the lock in storage, below the application.

## 12.5 Starting a turn as one command

`POST /api/threads/{id}/turns` carries `{ command_id, prompt, model, mode, effort }`.

**Validation, all before any write:** the thread's harness is available
(`HarnessUnavailable`); model, mode and effort are in its catalogue, enabled,
and the effort is one that model accepts (`SettingNotOffered`); the mode is in
the project's allowed set (`ModeNotAllowed`); no turn is running on the thread
(`ThreadBusy`).

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

**Replay.** A request whose `command_id` has a record and whose fingerprint
matches returns the recorded result and starts nothing — without re-validating
against the catalogue or the allowed modes as they are now, because the command
already happened. A matching `command_id` with a different fingerprint is
`CommandConflict`. The fingerprint covers the thread id, prompt, model, mode and
effort.

**`agent_invocation`** is created in that transaction, before anything spawns
(§2.7, §8.2). It answers §6.15's OPEN block with the explicit columns §6.15
already preferred:

```text
agent_invocation
id                  TEXT PRIMARY KEY
operation_id        TEXT NOT NULL UNIQUE FK operation(id) ON DELETE RESTRICT
role                TEXT NOT NULL
harness_kind        TEXT NOT NULL
harness_path        TEXT NOT NULL
harness_version     TEXT NOT NULL
requested_model     TEXT NOT NULL
requested_mode      TEXT NOT NULL      -- the permission mode requested of the harness
requested_effort    TEXT NOT NULL
profile_json        TEXT NOT NULL      -- '{}' until profiles exist
native_session_id   TEXT NULL
observed_model      TEXT NULL
observed_models     TEXT NULL          -- JSON array
context_used        INTEGER NULL
context_window      INTEGER NULL
output_tokens       INTEGER NULL
created_at          TEXT NOT NULL
```

`requested_mode` is what Shadows asked the harness for, not a record of the
rules and settings the harness then applied. §8.2's "effective read / write /
network permissions" stay unrecorded and are §12.3's OPEN block.

The `requested_*` columns never change. The rest is written once, in the
transaction that records the turn's terminal transition, from what the stream
reported. Nothing the stream did not report is estimated; it stays NULL and a
client shows it as unavailable.

- `observed_model` is the `message.model` of the turn's **last `assistant` line** — the model that produced the answer. The `system`/`init` line names the model the session started on, which a fallback can change, so it is not the evidence.
- `observed_models` is every key of the `result` line's `modelUsage`, because a turn can use more than one model (a fallback, or a small model for background work).

**Every entry names the turn it belongs to.** `thread_entry` gains
`operation_id TEXT NULL FK operation(id) ON DELETE RESTRICT`: the user entry the
turn command writes, and every agent entry the turn's stream produces, carry
that turn's operation id. Milestone 0's entries carry none (NULL), since which
turn wrote them can only be inferred from timing, and an inference is not
recorded as fact. This is the durable link §12.7's fork rule reads.

## 12.6 Context and account limits

**Context used** is taken from the turn's `result` line: the **last** element of
`usage.iterations`, summing `input_tokens`, `cache_read_input_tokens` and
`cache_creation_input_tokens`. Output tokens are not context. The top-level
`usage` sums every request the turn made and so overstates how full the context
is; it is not used. **Context window** is `modelUsage[observed_model].contextWindow`.
If either is missing, the ring shows "unavailable" rather than a guess. The
figure describes the context after the last turn; it does not move while a turn
runs, and a compaction shows up at the next turn.

**Account limits** come from the `rate_limit_event` stream line
(`rate_limit_info.unifiedWindows.five_hour` and `.seven_day`: `utilization` and
`resetsAt`). They are account-wide, so the daemon keeps the latest per harness
with the time it was observed (`harness_limit`, one row per harness, latest
wins; not journal data). A client always shows that time: the figure is only as
fresh as the last turn.

Delivery: the thread snapshot carries each operation's invocation; the
operation's terminal event carries the observed values so a live client updates
without refetching; `GET /api/harnesses` carries the latest limits, and a
transient `limits` SSE frame pushes a new report as it arrives.

## 12.7 Fork

`POST /api/threads/{id}/fork` with `{ command_id, at_entry_id }` creates, as one
command, a new thread in the same project on the same harness, titled after the
source with "(fork)":

- It copies the source's entries up to and including `at_entry_id` under new ids and ordinals. It copies no Operation: an Operation belongs to the thread that ran it.
- **Copied entries keep their `operation_id` and `refs` unchanged.** Both still name the source thread's operation; that is provenance, read-only history of how the message came about, and is shown as such. Nothing in the fork can stop, retry, or otherwise act on it.
- It records `forked_from_thread`, `forked_from_entry`, and `fork_session_id` (the source's harness session at that moment). The three are all NULL or all set, and are written only by the creating insert. The source is not changed.
- The fork's first turn runs `--resume <fork_session_id> --fork-session`; the new session id the stream reports is recorded as the fork's own `harness_session_id` by the existing rule (§4.2): at turn-end, once.
- **Valid fork point in this milestone**, decided only from durable rows: `at_entry_id` is the source's highest-ordinal entry; its `operation_id` is set; that operation is `Completed` (the state the watcher records only after the harness's turn-end); and the source's `harness_session_id` is set. A turn running on the source is `ThreadBusy`; anything else is `ForkPointNotSupported`, including an entry with no `operation_id` and a turn that was stopped or failed. Widening it to any entry is a server change only, once the Context Compiler exists.

> **To measure first** (the plan's first task, recorded in `docs/evidence/`):
> 1. `--resume X --fork-session` leaves session X usable and reports a new session id on the stream.
> 2. Which effort levels each catalogue model accepts, and what a refused one looks like.
> 3. Whether `rate_limit_event` appears on every turn or only past a threshold.
> 4. The `haiku` → `claude-sonnet-5` observation: what the last `assistant` line and `modelUsage` say for it.

## 12.8 Protocol changes

| Route | Change |
|---|---|
| `GET /api/harnesses` | new: catalogue, defaults, remembered model and effort, latest limits |
| create thread | gains optional `harness` (default `claude-code`) |
| `PATCH /api/threads/{id}` | new: `{ command_id, harness }`; `HarnessLocked` for a new command once an Operation exists; a replay answers the thread as it stands |
| `GET /api/projects/{id}` | carries the allowed modes per harness |
| `PATCH /api/projects/{id}` | new: `{ command_id, allowed_modes }` |
| `POST /api/threads/{id}/turns` | body becomes `{ command_id, prompt, model, mode, effort }` (§12.5) |
| `POST /api/threads/{id}/fork` | new, §12.7 |
| thread snapshot, operation events | carry the invocation's requested and observed values |
| SSE | new transient `limits` frame |

New stable error codes (§3.4):

```text
HarnessUnavailable     422  the thread's harness is listed but not runnable
SettingNotOffered      422  a model, mode or effort the harness does not list, has disabled,
                            or (effort) the chosen model does not accept
ModeNotAllowed         403  the project does not allow this mode
HarnessLocked          409  the thread already ran a turn on its harness
ThreadBusy             409  the thread has a turn running
ForkPointNotSupported  422  fork from anything but the last completed entry
```

## 12.9 Web client

Matches the mockup agreed on 2026-09-24:

- **Header:** CLI picker (`cli-picker.tsx`). Unavailable harnesses shown disabled as "coming". Changeable until the first turn, then shown with a lock.
- **Under the message box**, flat, no border (`composer-bar.tsx`): left `+`, mode, folder; right model, effort, context ring. Modes the project does not allow are shown disabled with the reason. The effort menu follows the chosen model. When the observed model differs from the requested one, the reply shows both.
- **Context ring** (`context-ring.tsx`): hover shows context used / window and percent, last turn's output tokens, five-hour and weekly utilisation with reset times, and "updated <time>".
- **Message actions** (`message-actions.tsx`): copy on every message, fork on the last one when the thread is idle; fork opens the new thread.
- **Allowed modes:** a per-harness checklist on the project page.
- New UI pieces are shadcn's `DropdownMenu` and `Tooltip` on the existing Base UI. No new library.

## 12.10 How the work is split

Backend and web client are built by separate agents that do not touch each
other's tree. The contract between them is `api/openapi.json`.

1. **Contract first.** The controller writes the draft `api/openapi.json` by hand from §12.8, including the SSE frame shapes, and commits it alone before any code.
2. **Backend agent** owns `src/`, `tests/`, `migrations/`, and at the end regenerates `api/openapi.json` (`UPDATE_OPENAPI=1 cargo test --test openapi`). Its report lists every difference between what it generated and the draft, with the reason for each. It never opens `web/`.
3. **Web agent** owns `web/` only. It generates its types from the draft and tests against a fake daemon built from it. It never runs the daemon or opens `src/`.
4. **A contract that is wrong or missing something** stops the agent that found it; it reports to the controller and changes neither the contract nor the other side.
5. **Integration.** Each agent works in its own worktree. The controller merges both into one branch; the generated `api/openapi.json` replaces the draft; the web types are regenerated, and any type error is fixed in `web/` only.

## 12.11 Acceptance

```text
[ ] the measurements in §12.7 are recorded in docs/evidence/
[ ] GET /api/harnesses lists Claude Code (runnable) and Codex (not), with Claude's catalogue as in §12.2
[ ] a turn runs with the chosen model, mode and effort; its entry, operation, invocation and
    command record commit in one transaction; its invocation holds path, version, requested
    and observed values
[ ] a replayed turn start with the same CommandId returns the first result and starts nothing,
    even after the project's allowed modes changed; a changed body is CommandConflict
[ ] a mode the project does not allow is refused by the daemon, not only hidden by the client
[ ] the harness can be changed before the first turn and not after, enforced in storage
[ ] a new conversation starts at Accept edits and at the last model and effort used
[ ] the context ring shows the last-iteration figure, or "unavailable"; limits show their observed time
[ ] copy works on every message; fork from the last message opens a thread whose next turn
    remembers the conversation, and the source is unchanged
[ ] one whole-branch review before the PR
[ ] Mohammed runs the real daemon with the real client and exercises every item above
```
