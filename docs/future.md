# Shadows — future list

What Shadows is meant to do and does not do yet, one line each. It decides nothing and copies
nothing: each line points to the part of [the vision](./vision.md) that says why, and a feature
that gets a design points to its owner spec. What is delivered is in its PR and in Git; the
order of delivery is in [the delivery roadmap](./superpowers/plans/2026-10-03-project-planning-roadmap.md).
AYLUL's list is [`aylul/future.md`](./aylul/future.md).

**Status:** `next` (chosen to start soon) · `planned` (has a spec or a plan) · `idea` (nothing
written beyond the vision). A feature leaves this list when it is delivered.

## Planning

| Feature | What it gives the person | Status | Why |
|---|---|---|---|
| Project switches (§23 PR 3c) | Connectors, skills and `CLAUDE.md` widen the Planner's reach per project | next | [§24.3](./superpowers/specs/2026-10-08-approval-authority-design.md) |
| Focused Planner context (§18 Stage 3) | Plan an explicitly chosen part, with a visible, bounded context manifest | planned | [vision §9](./vision.md#9-the-roadmap-shared-contracts-and-the-effect-of-a-change) |
| Decision records | The reasons behind a plan are kept, not only its edits | idea | [vision §8](./vision.md#8-the-plan-belongs-to-the-project-and-remembers-who-decided-what-and-why) |
| Clarifying messages | A new conversation asks the one that wrote a decision what it meant | idea | [vision §8](./vision.md#8-the-plan-belongs-to-the-project-and-remembers-who-decided-what-and-why) |
| Executable task contracts | Each task carries its contracts and the functions it must write | planned | [§17](./superpowers/specs/2026-10-01-task-contracts-and-evidence-design.md) |

## Execution

| Feature | What it gives the person | Status | Why |
|---|---|---|---|
| Executors | One bounded agent per task, writing only inside its task | planned | [vision §2](./vision.md#2-the-core-idea-shadows-knows-the-project-and-agents-ask-it), [§17](./superpowers/specs/2026-10-01-task-contracts-and-evidence-design.md) |
| The manager and the escalation ladder | Executor → manager → Planner → person; the plan never changes silently | idea | [vision §5](./vision.md#5-when-the-plan-is-wrong-the-escalation-ladder), [messaging probe](./evidence/2026-11-01-agent-messaging-probe.md) |
| Critic and specialists | A critic checks each result against its task; a researcher or library reviewer on demand | planned | [vision §6](./vision.md#6-executors-and-their-environment), [§19](./superpowers/specs/2026-10-03-agent-profiles-and-extensions-design.md) |
| A model per task | Strong model plans and verifies, a cheaper one takes well-specified tasks | idea | [vision §4](./vision.md#4-models-plan-with-a-strong-one-execute-with-the-one-that-fits) |
| Pending tests | A test waiting on task N stays linked to it and runs when N completes | idea | [vision §3](./vision.md#3-a-failing-test-that-belongs-to-another-task) |
| Two tasks at once | Small, conflict-checked concurrency after serial execution is proven | idea | [vision §6](./vision.md#6-executors-and-their-environment) |
| Codex environment | A tested Codex harness selectable from the Dashboard, with its MCP, plugins and skills | idea | [vision §10](./vision.md#10-a-complete-local-product) |

## Workspace and Dashboard

| Feature | What it gives the person | Status | Why |
|---|---|---|---|
| Execution Dashboard | What runs, what waits, missing evidence and the decision needed, not raw transcripts | idea | [vision §10](./vision.md#10-a-complete-local-product), [§12](./vision.md#12-clear-navigation-and-focused-work) |
| Design diagrams | ER diagrams for schemas, screen flows for interfaces, editing the same record as the details | idea | [vision §9](./vision.md#9-the-roadmap-shared-contracts-and-the-effect-of-a-change), [§12](./vision.md#12-clear-navigation-and-focused-work) |
| Shadows' own skills | Skills owned by Shadows, managed per project, the same under Claude and Codex | idea | [vision §13](./vision.md#13-shadows-own-skills-and-managing-them) |
| Extensions per project role | Skills, plugins and MCP connections enabled per role in the Dashboard | planned | [§19](./superpowers/specs/2026-10-03-agent-profiles-and-extensions-design.md) |
| Arabic interface | Arabic translations and right-to-left layout beside English | idea | [vision §12](./vision.md#12-clear-navigation-and-focused-work) |
| Installation and backups | Local install, explicit data location, recoverable backups, migration checks | idea | [vision §10](./vision.md#10-a-complete-local-product) |

## Beyond one machine

| Feature | What it gives the person | Status | Why |
|---|---|---|---|
| Teams and companies | A company workspace on a server; roadmap and contracts agreed at the top | idea | [vision §7](./vision.md#7-teams-and-companies--later), [§1.7](./superpowers/specs/2026-09-21-architecture-design.md#17-workspace-authority-and-deployment) |
| Production operations | Observe the app's AWS environment, propose and verify a remedy within authority | idea | [vision §10](./vision.md#future-production-operations), [§17.12](./superpowers/specs/2026-10-01-task-contracts-and-evidence-design.md#1712-production-operations-extension) |
| Linux containment | The adapter dies with a crashed daemon on Linux too | idea | [§1.5 OPEN](./superpowers/specs/2026-09-21-architecture-design.md) |

## Housekeeping

| Task | What it removes | Status | Why |
|---|---|---|---|
| Product workspaces (A-22) | `runtime/` and `code-intelligence/` as their own workspaces, each with its own lockfile, `target/` and CI job | planned | Deferred 2026-10-28: nothing new lives there yet, and a second build cache costs more than it protects. Starts at the first new crate of our own runtime (Phase 6), or `shadows-index` published on its own. [Plan](./backend/plans/2026-10-21-architecture.md#a-22--split-the-repository-by-product) |
| Remove mx | Its disabled hook scripts, `mx*` skills, permission lines, the `mxai-knowledge` MCP server and the mx block in the global `CLAUDE.md` | idea | [vision §1](./vision.md#1-the-problem) (memory tools did not help) |

## Adding to this list

A new idea goes first into [the vision](./vision.md) (its "Adding to this file" rules), then gets
one row here. A row changes status in place; a delivered feature is removed (its PR records it).
