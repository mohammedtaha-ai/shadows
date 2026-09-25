# Shadows — the vision

Mohammed told this on 2026-09-25, after Milestone 2 ran on Windows. He called
Milestone 2 good as a first version, but not yet the idea he has in mind.

This file says **what Shadows is for and where it is going**. It decides
nothing: every design decision lives in its owner spec
([`superpowers/specs/README.md`](./superpowers/specs/README.md)). When a part
below becomes a milestone, its design goes into a spec, and the part links to
that spec instead of repeating it.

Each part ends with a **Today** line saying how much of it exists.

---

## 1. The problem

These are from Mohammed's experience across many projects with AI agents
(Claude Code, Codex and others). None of them is specific to Shadows.

1. **Agents miss things, and more so as a project grows.** An agent misses a
   piece that already exists and writes it again in a new file, or puts it in
   the wrong file. The reasons:
   - it sees only the part of the project it opened;
   - its memory breaks between sessions and when a long session is compacted;
   - each tool (Claude, Codex, …) has its own memory;
   - the project grows but the context window does not.
2. **Memory is not enough for a new session.** Every session reads the code
   again to understand it before doing any work, and still misses *why* things
   are the way they are.
3. **Context gets polluted in a long session.** Running one fresh agent per task
   (as superpowers does) keeps the context clean, but every fresh agent starts
   cold and reads the code again. The same understanding is paid for again by
   every implementer and every reviewer.
4. **Existing memory tools do not fix it.** Mohammed tried MCP memory servers
   and `mx`, and stopped them. Most of their work was manual, and saving memory
   often cost more tokens than the work itself.

## 2. The core idea: Shadows knows the project, and agents ask it

1. **Clean planning from the start.** Before any code, the plan settles:
   - the contracts;
   - the languages;
   - the project structure;
   - the functions, and what each one must do.
2. **The plan becomes a clean workflow.** Every task is bounded, and it is known
   exactly what the task is.
3. **An executor works on its task without wandering:**
   1. Shadows shows it the project's overall structure.
   2. The task says which functions it must write and what they do.
   3. The executor writes them without exploring the rest of the code.
4. **"Where is X?"** When the executor needs to know how another function
   works, it asks Shadows. Shadows answers with the **name, file and line**,
   never the code. The executor opens that exact place itself if it needs to.
5. This holds on a project with earlier tasks done and code already written,
   not only on an empty one.

**What it solves:** the agent does not miss or duplicate code (§1.1), because
it asks before it writes. It does not re-read the project every session
(§1.2). And its context stays small and clean (§1.3).

**Today:** Milestone 2 has the planning half. The Planner writes a durable plan
of tasks and links, with versions, Approve and freezing. Tasks do not yet carry
contracts or function specifications. The MCP server that Milestone 2 built is
the channel that "Where is X?" would use. No code index answers it yet;
`docs/codebase/inventory.md` is a hand-run version of the idea, and `gcode` is
already named in `CLAUDE.md` as an optional code-search tool.

## 3. A failing test that belongs to another task

Say task 4's tests fail because the code they exercise belongs to task 8. For
example, task 4 built the frontend, and the backend contracts its tests call
are task 8's. Task 8 has not started, or its agent is still working.

- **Today's agents** go looking for why. They build the missing part themselves
  or write fake tests (mocks) so that the tests pass. That is wandering outside
  the task.
- **In Shadows**, the executor is told: *this failure is expected; the missing
  part is owned by task 8, whose state is …*. The executor builds nothing it does
  not own and writes no fake tests.
- **Task 4 finishes.** Its tests that wait on task 8 stay pending and linked to
  task 8. **When task 8 completes, Shadows runs them automatically.**

So a failing test is not always a defect. Shadows knows which failures are
"waiting on task N".

**Today:** not started.

## 4. Models: plan with a strong one, execute with the one that fits

Mohammed's view: planning is the most important part, and writing the code
can be done by any cheap model once the plan is good.

Agreed, with three caveats that came out of Milestone 2:

1. **Plans are proven wrong while coding.** Milestone 2 found that Claude
   ignores changed instructions after a session starts, that `rmcp` behaved
   unlike the plan assumed, and that the 5 s harness bound was too short. No
   plan predicted these. A cheap executor needs a way to say "the plan is wrong
   here" (§5) instead of improvising, and weak models improvise worse.
2. **Some tasks are hard in the code, not in the plan.** Concurrency is the
   example: Milestone 2's races at a turn's end came with a correct plan.
3. **Verification is not the place to save.** A cheap model may write the code,
   but something strong must check it: tests the plan itself specifies, or a
   strong reviewer.

So a strong model plans, takes the hard tasks, and verifies, and a cheaper
model takes the well-specified tasks. Shadows chooses the model per task.

**Today:** the person chooses the model per message for the Planner
(Milestone 1). Executors do not exist yet.

## 5. When the plan is wrong: the escalation ladder

What coding uncovers falls into five kinds:

1. the plan assumed something false about the outside world (a library, a
   harness), and it affects more than one task;
2. the plan left something out that the task needs, which is a small gap;
3. two tasks assume different contracts for the same thing;
4. a surprise that shows only when the software runs;
5. the task is harder than planned and needs a stronger model.

**Rule: an executor never changes the plan silently.** It stops at the edge of
its task and reports: what it found, the evidence, and which tasks it affects.

The report climbs only as far as it has to:

```text
Executors (one agent per task)
   ↓ finds a problem
Manager (an agent above the executors, who understands the whole plan)
   ↓ when unsure of its own decision
Planner
   ↓ when the problem is very large
Mohammed (the user)
```

- **Executor:** decides nothing outside its task. It reports to the manager.
- **Manager:**
  - decides whether to stop the executor, and any other executor affected, such
    as a task 8 already working on the old contract, or to let them go on. It is
    the only one who sees every running task.
  - When the fix is easy and clear from its understanding of the plan, it
    proposes the fix and has the executor apply it.
  - When it doubts its own decision, it takes its proposal to the Planner.
- **Planner:** accepts the manager's proposal, amends it, or chooses a better
  one. It changes the plan as a new version, as Milestone 2's v1 → v2 does. When
  the problem is very large, it stops the work and alerts the user.
- **The user** hears only about what is truly large. Everything else is settled
  below.

A good plan also knows where it is unsure. It marks the unknowns and probes
them before coding, as Milestone 2's Task 0 probed MCP and found the frozen
instructions early.

**Today:** by hand. In Milestone 2 the controller amended the spec in place
whenever the code proved it wrong, and recorded it in the ledger. Shadows is to
do this itself.

## 6. Executors and their environment

- **The executor is chosen per task.** For example, MiniMax runs task 4 and
  Codex runs task 7.
- **Tasks that do not conflict run at the same time, in the same project
  folder, without worktrees.** The plan knows which files and functions each
  task owns, so Shadows can judge which tasks conflict.
- **How each executor runs:**
  - Claude and Codex through their own CLIs, with the login already on the
    machine. Shadows never touches their OAuth tokens. This is how Shadows
    runs Claude today.
  - Everything else, such as MiniMax, through
    [jcode](https://github.com/1jehuang/jcode), with an API key chosen per
    executor.

**jcode, as read on 2026-09-25:**
- Rust, MIT, about 20k stars, actively developed.
- Supports MiniMax by name: `jcode login --provider minimax` or
  `MINIMAX_API_KEY`. Also Gemini, Ollama and OpenAI-compatible endpoints.
- Runs non-interactively (`jcode run "…"`) and as a server (`jcode serve`).
- Supports MCP, so a jcode executor could ask Shadows "Where is X?" over the
  same server.
- **Does not mention ACP**, the protocol Shadows drives Claude through (send a
  turn, stream the answer, stop, resume). Its server protocol is internal and
  undocumented, and the docs describe a Unix socket.

**Today:** only the Planner runs, and only on Claude Code.

## 7. Teams and companies — later

Personal use stays exactly as above. For companies:

- **A company workspace:** a backend on a server with PostgreSQL or similar.
  Each person's personal Shadows stays local.
- **Planning comes down in levels:**
  1. Two or three people write a large roadmap and agree the contracts.
  2. Each team takes its part and splits it however it likes.
  3. Each person takes their piece and makes their own workflow however they
     like.
- **Without teams:** for example, five members agree the roadmap, and each takes
  their part and splits it however they like.
- **The contracts are the link.** They are agreed at the top, and everyone below
  is free in how they build, as long as they keep them.

This is a large topic and likely its own backend, maybe its own repository. It
waits until personal use is complete.

**Today:** the storage layer was chosen with PostgreSQL in mind. SQLx was
validated against PostgreSQL 16, and `CLAUDE.md` names a future PostgreSQL
adapter. Nothing else exists.

## 8. Open questions

Each one closes when the part it belongs to becomes a milestone.

- **Shared-folder parallelism (§6).** Two executors in one folder share builds
  and tests. One may see the other's half-written code and fail for a reason
  that is not its own. In Rust, the `target` directory is locked by one build at
  a time.
- **Driving jcode (§6).** It has no ACP. A probe must show whether `jcode run`
  gives output a program can read, and whether it can be stopped and resumed.
  Whether Codex has a usable ACP adapter also needs checking.
- **Pending tests (§3).** How a test is marked as waiting on task N, and who
  owns it when it fails after task N completes.
- **The "Where is X?" index (§2.4).** Where it comes from: generated from the
  code as `inventory.md` is, a parser, `gcode`, or another tool. And how it
  stays current while executors write code.
- **The manager (§5).** What it sees, and whether it is one agent for the
  whole workflow or one per group of tasks.

---

## Adding to this file

- **A new feature or idea** gets a new numbered part before §8, and the parts
  keep their numbers. Write it as:
  - what the person does or sees;
  - which problem in §1 it solves, or a new problem added to §1;
  - a **Today** line.
- **A changed idea** is edited in place, with a dated line saying what changed.
- **A part that becomes a milestone** gets its design in an owner spec. The part
  keeps its idea and links to the spec.
- **An answered open question** leaves §8, and its answer goes where it belongs.
