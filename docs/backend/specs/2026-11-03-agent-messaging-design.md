# Agent messaging: how agents report, escalate and talk in groups

- Status: **accepted** by Mohammed on 2026-11-03, in conversation, with the new `Messages`
  owner (§7). Slice 1 (§8, groups) is **built** in PR #54 (`9e5b9ab2`); slices 2 to 4 wait for
  Core's executors (roadmap Stage 4).
- Evidence: the messaging probe (`docs/evidence/2026-11-01-agent-messaging-probe.md`
  at `f1459dcc`); Hermes group rooms, read from a local copy of its source
  (`apps/desktop/src/plugins/hermes-bots/group-chat.ts`,
  `apps/desktop/docs/canonical-groups.md`). Read, not run.
- Builds on, and does not replace:
  - Core vision §5, the escalation ladder (`docs/core/vision.md`);
  - §17.5, the execution packet and problem reports
    (`docs/core/specs/2026-10-01-task-contracts-and-evidence-design.md`);
  - §19.3, the critic (`docs/core/specs/2026-10-03-agent-profiles-and-extensions-design.md`);
  - §20, the queue (`docs/core/specs/2026-10-05-message-queue-design.md`);
  - §24, who approved (`docs/core/specs/2026-10-08-approval-authority-design.md`);
  - the AYLUL agents spec §7.8, asking before a tool runs
    (`docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md`).
- Answers in part §17.5's OPEN "manager authority and recovery" (§6 here).
  It is the "messaging design" the AYLUL agents spec §3 points to, for the
  agents Shadows starts. Watching sessions Shadows did not start stays with
  the hook design (`docs/aylul/native/specs/2026-10-13-aylul-hook-laptop-design.md`).

## 1. Intent and why

Shadows will run several agents on one plan: executors, a manager, the
Planner, later a critic. Mohammed wants to pick the harness per role: one
executor on Codex, another on Claude Code, another on Claude Code pointed at
MiniMax. When an executor finishes or hits a problem, someone above it must
hear about it, read what went wrong and answer, whatever harness each side
runs on.

Claude Code sessions can message each other today (the probe, §4), but only
inside one config directory, through an undocumented pipe, and Codex has no
counterpart. So agents cannot talk to each other directly across harnesses.

The same system serves AYLUL. Mohammed wants a group of AYLUL agents he can
write to, mentioning one or two of them, as Hermes does with its rooms. And
AYLUL's work profile needs to explain the agents' work and alert him; that
needs the same record of who said what to whom.

## 2. Goals

1. An agent on any harness Shadows runs can report "done" or "blocked" to the
   role above it, with evidence.
2. The role above reads the report, decides, and answers the same executor in
   the same session.
3. A problem climbs executor → manager → Planner → Mohammed, only as far as it
   needs to.
4. Mohammed can open a group of agents and write to them, mentioning some.
5. AYLUL sees every message and escalation as it happens, to explain and to
   alert.

## 3. Non-goals

- Agents speaking to each other without Shadows in the middle.
- Using Claude Code's session pipe or `SendMessage`; it is undocumented and
  Claude-only (probe §3, §4).
- Building the executor, the manager or the execution packet. §17 and Core
  stages 4 and 5 own them; this spec owns how they talk.
- Watching sessions Shadows did not start (the hook design owns that).
- Voice, the phone.

## 4. Anti-goals (a failure even if it works)

- A message, report or reply that exists only inside a harness's own history.
  If Shadows did not record it, it did not happen.
- A plan changed with no record of who changed it and why.
- Two agents mentioning each other with no end.
- An executor that gains files, tools or reads because a manager said so.
- A design that works for Claude Code only.

## 5. Decided by Mohammed (2026-11-03)

1. **Shadows is the broker.** Agents never talk to each other directly. Each
   talks to Shadows only. (Proposed by Claude, accepted.)
2. **The manager is an agent that thinks and decides**, not rules in code.
3. **Each role sees only what it needs.** The executor sees its task: the
   files, the related functions, the tools it may use. It does not see the
   whole plan. The manager sees the plan, the vision and the decisions, and
   links them. The Planner has its own planning context, wider than the plan.
4. **The manager answers the executor in the same session**, after adjusting
   the task, when it has a fix.
5. **A problem the manager judges too big goes to the Planner.**
6. **The Planner judges a change itself.** A small adjustment: it changes the
   plan and work continues. A large architectural decision: work stops and
   Mohammed is told.
7. **AYLUL uses the same system**: groups of agents with mentions, as in
   Hermes, and the work profile's explaining and alerting.
8. **Harnesses mix.** Executors may run on Codex, on Claude Code, or on Claude
   Code with another provider's API (MiniMax).
9. **The manager's adjustment is guidance only** (§6.4). It never widens an
   executor's files, tools or reads; a wider fix goes to the Planner.
10. **Some changes always stop for Mohammed**, by a rule in code (§6.5).
11. **The bounds** in §6.5 and §6.6, as starting values that become settings
    later.
12. **Groups are built first** (§8).

Decisions 9 to 12 were proposed by Claude and accepted.

## 6. The design

### 6.1 Two channels, both existing

| Direction | Channel | Carries |
|---|---|---|
| Shadows → agent | ACP (`shadows-agent`) | Start the session, send a turn, see every event, see the turn end |
| Agent → Shadows | MCP (`/mcp`, `shadows-mcp`) | Report, ask, reply: tools called under the agent's grant |

Both already work for the Planner. ACP runs `claude-code` and `codex` today
(`core-harness/src/model.rs`); MiniMax runs inside Claude Code through its
provider settings. No new protocol is added. (Claude's call: no
agent-to-agent protocol such as A2A; nothing needs one while Shadows starts
every agent.)

### 6.2 The message

Every report, reply, escalation and group line is one durable **message**:

| Field | Meaning |
|---|---|
| id | Its identity |
| channel | Where it belongs: a task run's channel, an escalation, or a group |
| sequence | Its order in the channel; explicit, never insertion order |
| from | A role and agent (`executor:<run>`, `manager`, `planner:<harness>`, `aylul:<slug>`), the person, or Shadows itself |
| to | One or more addressees; in a group, the mentioned members |
| kind | `report` (done), `blocked`, `question`, `reply`, `escalation`, `decision`, `notice`, `chat` |
| body | Text the sender wrote |
| refs | What it is about: task, run, packet digest, plan version, agreement version, evidence ids |

A message is written once and never edited. It is published on the journal,
so `Events` subscribers (the web client, AYLUL) see it live. An agent's
message comes from its grant, never from a tool argument naming the sender
(§24.1's rule, applied here).

### 6.3 Delivery: waking the addressee

A message to an agent is delivered as a turn in that agent's session:

- **Idle session:** Shadows starts a turn whose prompt is the message and the
  new messages in its channel since that agent's last turn.
- **Busy session:** the delivery waits in `Messages`' own durable delivery
  record and starts when the agent's running turn ends, however it ends. §20's
  queue is not used: it sends the next message only after `Completed` and then
  waits for a person (§20.3), which would leave agents stuck. An agent's
  running turn is never interrupted by another agent.
- **Turn ends are read from the journal**, after a cursor `Messages` keeps,
  and a reply is recorded in the same transaction that moves the cursor, so a
  restart loses no reply.
- **Dispatch and terminal consumption share one worker.** It replays prepared
  starts with their stored inputs and links their accepted operation before
  reading terminal events. The model's offered effort is chosen by `Turns`
  when starting a new operation.
- **A busy lease can clear without a journal event.** Prepared deliveries can
  retry in that case. Shutdown stops new dispatch and uses Turns' confirmation
  bound. An unfinished link can be recovered from the same start command.
- **No session** (closed, crashed): Shadows opens one for that role. For an
  executor this is the same run only if the run still holds; otherwise the
  manager is told and decides (a new attempt is §17.4's rule).
- **A turn that ends with no report.** If an executor's turn ends without
  `report` or `blocked`, Shadows writes a `notice` from itself to the manager:
  "the executor stopped without reporting", with its last message. ACP's turn
  end is the event; the executor saying "done" in text is not (§19.3).

A message to the person is not a turn; it is shown, and AYLUL alerts (§6.7).

### 6.4 What each role sees and may do

The grant is the boundary. Shadows refuses a tool outside it, whatever the
instructions say.

| Role | Sees | May |
|---|---|---|
| Executor | Its packet (§17.5): task contract, files, related symbols, allowed tools; its run channel | `report`, `blocked`, `question` to the manager |
| Manager | The frozen plan, vision, decisions, agreements, every task's state, reports, evidence; every run channel | `reply` to an executor; stop or hold a run; add guidance to a run; `escalation` to the Planner; `notice` to the person |
| Planner | Its planning thread and everything the manager sees | `decision` to the manager; a new plan version; stop all runs; `escalation` to the person |
| Person | Everything | Answer any escalation; write to any channel |

**Guidance is not scope.** The manager's "adjust the task" is guidance added
to the run: what to try, what it found. It cannot widen the files, tools or
reads; §17.5 already refuses that ("a manager reply cannot silently expand
them"). A fix that needs more scope is a plan change and goes to the Planner.
(Decision 9.)

### 6.5 The ladder

1. **Executor** finds a problem inside its task: fixes it. At the edge of its
   task: calls `blocked` with §17.5's report (task, packet and agreement
   versions, obstacle, evidence, tried remedies, affected tasks, proposal).
2. **Manager** is woken with the report. It reads what it needs through its
   tools and decides:
   - a fix inside scope → `reply` with guidance; the executor continues in
     the same session;
   - other runs affected (a task already working on the old contract) → hold
     them;
   - unsure, or the fix changes scope, a contract or ownership → `escalation`
     to the Planner.
3. **Planner** is woken with the escalation and decides:
   - **small adjustment** → a new plan version, approved by the Planner itself
     (§24.2 records it as `planner:<harness>`); work resumes and Mohammed can
     read and revert it;
   - **large architectural decision** → stop the affected runs, `escalation`
     to Mohammed, wait.
4. **Mohammed** answers in the channel; the answer climbs back down the same
   way.

**Always Mohammed's, whatever the Planner judges** (decision 10, a rule in
code, not in the Planner's instructions):

- a change to an approved API agreement (§18; it needs his approval today);
- widening a task's write scope or capabilities beyond its approved version;
- anything §24.2 reserves for the person (create structure, waivers).

The Planner's tool refuses these and turns them into an escalation to the
person. A model's sense of "small" decides the rest.

**Bounds** (decision 11; one place in code, so they can become settings
later):

- one problem gets at most **2 manager replies**; the third failure escalates
  to the Planner by itself;
- a problem that reaches the Planner twice goes to Mohammed;
- a problem has an identity, so a repeat is counted, not treated as new.

### 6.6 Groups (AYLUL)

A **group** is a channel whose members are AYLUL agents and Mohammed.

- **Mentions route.** Mohammed writes `@coder @reviewer …`; only the mentioned
  members get a turn. A message of Mohammed's with no recognized address reaches every
  member. An explicit `@mohammed` alone wakes no member. A member's reply wakes
  only the other members it mentions; a reply
  with no member mention wakes no one.
- **A member's reply is the text of its turn.** It needs no tool, so it works
  on any provider.
- **A member sees what is new.** Its turn carries the group's messages since
  its last turn, up to a size limit, the oldest dropped first and the cut
  stated.
- **Members may mention each other**, inside bounds: one message from Mohammed
  allows at most **3 rounds** and **10 member deliveries** (Hermes' numbers,
  learned from a real mention loop), reserved when each delivery is scheduled. When the bound is reached the group stops
  and says so.
- **`@mohammed` means "needs you"**: the group is marked, and AYLUL alerts.
- **Permission requests** of a member (§7.8 of the AYLUL agents spec) show in
  the group, answered in place.
- **Each member has one session per group** (Claude's call), apart from its
  one-to-one conversation with Mohammed, so a group does not fill that
  conversation and each group keeps its own memory.
- **Members may run on different harnesses**, because Shadows is the broker.
- The log is Shadows' from the first day (Hermes began in the browser's
  storage and had to move it).

### 6.7 AYLUL watches

AYLUL subscribes to messages through `Events`, as the web client does.
Each `MessageAdded` payload includes the group id, the group-local sequence and
`calls_person`, set when an agent addresses the person. Older journal rows can
omit the flag. The first desktop replay establishes its baseline silently.

- **Alert** when a message is addressed to Mohammed, an escalation reaches
  him, a run is stopped or fails, or a group says `@mohammed`.
- **Explain**: an AYLUL agent can read a channel and tell Mohammed, in his
  words, what happened. How it explains belongs to the work profile design;
  this spec gives it the record.

## 7. Owners

No service in `AppCore` owned messages. Mohammed approved a new owner on
2026-11-03:

| Rule | Owner |
|---|---|
| Channels, messages, their order, delivery, bounds | a new service `Messages` (`core-messages`) |
| What a role's grant reaches | `Grants` (new grant kinds: executor, manager) |
| A plan version and its approval | `Plans`, `Grants` (unchanged) |
| Starting a turn for a delivered message | `Turns::send`, called by `Messages` from its delivery record |
| Reading which turns ended, after a cursor | `Events` (the journal) |
| The MCP tools (`task_report`, `task_blocked`, `message_reply`, …) | `shadows-mcp`, calling `Messages` only |
| Groups' members and settings | `Messages` |

## 8. Order of building

Each slice runs end to end before the next.

1. **Groups for AYLUL agents.** Needs no executor; uses `Messages`, delivery,
   mentions, bounds and AYLUL's alert. Proves the system on what exists.
2. **Executor reports to the person** (§17.5's first serial release): needs
   Core stage 4's executor.
3. **The manager**: replies, holds, guidance, escalation to the Planner.
4. **The Planner's decisions** and the rules that are always Mohammed's.

(Decision 12: groups first, because executors are not built and groups test
the same parts.)

## 9. Open

> **OPEN — the manager's harness and context size.** The manager reads the
> plan, vision and decisions; how much is sent per turn and how much is
> fetched by tool is not decided. Closed by the manager slice's plan, with a
> measured run.

> **OPEN — one manager per plan or per project.** Closed before slice 3.

## 10. Left to the builder

- Table names and columns, beyond the fields in §6.2.
- The exact MCP tool names and their argument shapes.
- How the delivered turn's prompt is worded.
- The web client's views of channels; AYLUL's group screen belongs to the
  AYLUL UI plan.
