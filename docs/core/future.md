# Shadows — future list

What Shadows is meant to do and does not do yet, one line each. It decides nothing and copies
nothing: each line points to the part of [the vision](./vision.md) that says why, and a feature
that gets a design points to its owner spec. What is delivered is in its PR and in Git; the
order of delivery is in [the delivery roadmap](roadmap.md).
AYLUL's list is [`aylul/future.md`](../aylul/future.md).

**Status:** `next` (chosen to start soon) · `planned` (has a spec or a plan) · `idea` (nothing
written beyond the vision). A feature leaves this list when it is delivered.

## Planning

| Feature | What it gives the person | Status | Why |
|---|---|---|---|
| Project switches (§23 PR 3c) | Connectors, skills and `CLAUDE.md` widen the Planner's reach per project | next | [§23.9](specs/2026-10-08-approval-authority-design.md), [§24.3](specs/2026-10-08-approval-authority-design.md) |
| Focused Planner context (§18 Stage 3) | Plan an explicitly chosen part, with a visible, bounded context manifest | planned | [vision §9](./vision.md#9-the-roadmap-shared-contracts-and-the-effect-of-a-change), [planner context spec §6](../backend/specs/2026-11-04-planner-context-design.md#6-order-of-building); slices 1 to 3 are in PR #55, PR #56 (`024432fc`) and PR #57 (`1b413fc4`); slice 4 waits for messaging slice 3 |
| Decision records | The reasons behind a plan are kept, not only its edits. Plan versions already keep their reasons; the rest is not built | planned | [vision §8](./vision.md#8-the-plan-belongs-to-the-project-and-remembers-who-decided-what-and-why), [§16 1a](specs/2026-10-01-project-plans-design.md) |
| Clarifying messages | A new conversation asks the one that wrote a decision what it meant | idea | [vision §8](./vision.md#8-the-plan-belongs-to-the-project-and-remembers-who-decided-what-and-why) |
| Executable task contracts | Each task carries its contracts and the functions it must write | planned | [§17](specs/2026-10-01-task-contracts-and-evidence-design.md) |
| A plan's architecture stored and checked | The plan holds its architecture, and a builder's change is checked against it | planned | [§13.16 OPEN](specs/2026-09-25-planner-workflow-design.md) |
| Research artifacts and search | Research kept as dated artifacts, found by full-text search | planned | [§6.7, §6.22](specs/2026-09-21-sqlite-schema-design.md), [§4.2](specs/2026-09-21-domain-model-design.md) |

## Execution

| Feature | What it gives the person | Status | Why |
|---|---|---|---|
| Executors | One bounded agent per task, writing only inside its task; independent tasks together | partly built: slices 0, 1, 1b and 3 in PRs #64 and #65 (`c71b263d`, `617d806f`); slice 4, tasks together, is planned: [executors design](../backend/specs/2026-11-08-executors-design.md) | [vision §2](./vision.md#2-the-core-idea-shadows-knows-the-project-and-agents-ask-it), [§17](specs/2026-10-01-task-contracts-and-evidence-design.md) |
| The manager and the escalation ladder | Executor → manager → Planner → person; the plan never changes silently | designed: messaging spec (`docs/backend/specs/2026-11-03-agent-messaging-design.md`); its slice 1, groups, built in PR #54 | [vision §5](./vision.md#5-when-the-plan-is-wrong-the-escalation-ladder), messaging probe (`docs/evidence/2026-11-01-agent-messaging-probe.md` at `f1459dcc`) |
| Critic and specialists | A critic checks each result against its task; a researcher or library reviewer on demand | replaced for the critic: the Planner judges each result ([executors design §5.1](../backend/specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11)); a specialist reviewer on demand is planned | [vision §6](./vision.md#6-executors-and-their-environment), [§19](specs/2026-10-03-agent-profiles-and-extensions-design.md) |
| Earlier attempts' diffs | The task page's Diff shows each attempt's commit, not only the latest, so an addition's change is read on its own | idea (Masada, deferred 2026-11-11) | `docs/shared/requests/2026-11-11-planner-review.md` at `617d806f`; `GET /api/attempts/{id}/diff` already answers any attempt |
| A tool call's input and output | Each tool the Planner or an executor calls opens to show what it was given and what it answered (a command line and its output, `plan_edit`'s operations), like Claude Code's tool rows | idea (Mohammed, 2026-11-12) | Today an entry keeps only the tool's name and status, though ACP's `rawInput`/`rawOutput` and Codex's `command`/`aggregatedOutput` reach Core. Store them with the entry, capped; when outputs grow large, keep the full output as a file in Shadows' data folder and only its head and path in the database, as Claude Code and Codex keep sessions on disk |
| A model per task | Strong model plans and verifies, a cheaper one takes well-specified tasks | idea | [vision §4](./vision.md#4-models-plan-with-a-strong-one-execute-with-the-one-that-fits) |
| Pending tests | A test waiting on task N stays linked to it and runs when N completes | idea | [vision §3](./vision.md#3-a-failing-test-that-belongs-to-another-task) |
| Two tasks at once | Small, conflict-checked concurrency after serial execution is proven | idea | [vision §6](./vision.md#6-executors-and-their-environment) |
| Codex environment | A tested Codex harness selectable from the Dashboard, with its MCP, plugins and skills | planned: selectable in the conversation since PR #63 (`217d7540`), with images and its two modes since PR #67 (`db760593`); its real Planner acceptance (B-88, `docs/backend/stages.md`) and the Dashboard are open | [vision §10](./vision.md#10-a-complete-local-product) |
| Deterministic verification | Checks that run without AI review, with each run's verdict recorded | planned | [§2.8](specs/2026-09-21-data-flow-design.md), [§6.12, §6.16, §6.17](specs/2026-09-21-sqlite-schema-design.md) |
| Context Compiler and cross-harness continuity | The Compiler chooses and budgets what an agent receives; a conversation continues on another harness | planned | [§2.11](specs/2026-09-21-data-flow-design.md), [§11.5](specs/2026-09-21-rules-scope-and-milestones-design.md) |
| Our own runtime replaces ACP | Core's agent connection moves from ACP to Shadows' own runtime, one capability at a time | planned | [architecture plan, Phase 6](../backend/plans/2026-10-21-architecture.md#phase-6--our-own-runtime-replaces-acp) |
| The name index | `where_is`, `who_uses` and `outline` from the tree-sitter name index (`shadows-index`: Rust, TypeScript, JavaScript) | suspended (Mohammed, 2026-11-13): its code stays, nothing calls it; returns only if a need names it, such as a language with no language server | [language servers design](../backend/specs/2026-11-13-language-servers-design.md): `code_lsp` answers from the project's language servers instead |
| Single-file adapter | The adapter runs without Node installed on the machine | idea | [harness controls, OPEN](specs/2026-09-24-harness-controls-design.md) |

## Workspace and Dashboard

| Feature | What it gives the person | Status | Why |
|---|---|---|---|
| Execution Dashboard | What runs, what waits, missing evidence and the decision needed, not raw transcripts | idea | [vision §10](./vision.md#10-a-complete-local-product), [§12](./vision.md#12-clear-navigation-and-focused-work) |
| Design diagrams | ER diagrams for schemas, screen flows for interfaces, editing the same record as the details | idea | [vision §9](./vision.md#9-the-roadmap-shared-contracts-and-the-effect-of-a-change), [§12](./vision.md#12-clear-navigation-and-focused-work) |
| Shadows' own skills | Skills owned by Shadows, managed per project, the same under Claude and Codex | idea | [vision §13](./vision.md#13-shadows-own-skills-and-managing-them) |
| Extensions per project role | Skills, plugins and MCP connections enabled per role in the Dashboard | planned | [§19](specs/2026-10-03-agent-profiles-and-extensions-design.md) |
| Arabic interface | Arabic translations and right-to-left layout beside English | idea | [vision §12](./vision.md#12-clear-navigation-and-focused-work) |
| Installation and backups | Local install, explicit data location, recoverable backups, migration checks | idea | [vision §10](./vision.md#10-a-complete-local-product) |
| Bounded resync snapshot | A reconnecting client gets a bounded snapshot, then older history on request | planned | [§2.10](specs/2026-09-21-data-flow-design.md), [§5.7](specs/2026-09-21-persistence-api-and-migrations-design.md) |

## Beyond one machine

| Feature | What it gives the person | Status | Why |
|---|---|---|---|
| Teams and companies | A company workspace on a server; roadmap and contracts agreed at the top | idea | [vision §7](./vision.md#7-teams-and-companies--later), [§1.7](specs/2026-09-21-architecture-design.md#17-workspace-authority-and-deployment) |
| Production operations | Observe the app's AWS environment, propose and verify a remedy within authority | idea | [vision §10](./vision.md#future-production-operations), [§17.12](specs/2026-10-01-task-contracts-and-evidence-design.md#1712-production-operations-extension) |
| Linux containment | The adapter dies with a crashed daemon on Linux too | idea | [§1.5 OPEN](specs/2026-09-21-architecture-design.md) |
| Remote approvals and external authority | A person confirms an approval away from the machine, and the proof is checked outside the Planner | planned | [§24.6 OPEN](specs/2026-10-08-approval-authority-design.md#246-external-human-confirmation-and-assessment) |
| Independent code-intelligence package | A code index any MCP client can use without Core or AYLUL running | planned | [service design](../code-intelligence/specs/2026-10-18-code-intelligence-service-design.md), [stages](../code-intelligence/stages.md) |

## Housekeeping

| Task | What it removes | Status | Why |
|---|---|---|---|
| Product workspaces (A-22) | `runtime/` and `code-intelligence/` as their own workspaces, each with its own lockfile, `target/` and CI job | planned | Deferred 2026-10-28: nothing new lives there yet, and a second build cache costs more than it protects. Starts at the first new crate of our own runtime (Phase 6), or `shadows-index` published on its own. [Plan](../backend/plans/2026-10-21-architecture.md#a-22--split-the-repository-by-product) |
| Real-provider journey suite | Most Core tests run against `fake-acp`, so they prove Shadows agrees with itself, not that it works with real Claude. A small required suite runs the real daemon, adapter and Claude through one journey (Idea → Vision → Map → Roadmap → Plans, a part move, phases, `task_context`), asserts on stored state rather than reply text, and reports what passed and the tokens spent. One local command, run before each PR and each adapter upgrade; about 10–15 minutes per run, about a day to set up. `fake-acp` stays for internal rules, storage and refusals | idea | Asked by Mohammed on 2026-11-07: an adapter upgrade and the map-move refusal were both invisible to the fake-backed tests |
| Remove mx | Its disabled hook scripts, `mx*` skills, permission lines, the `mxai-knowledge` MCP server and the mx block in the global `CLAUDE.md` | idea | [vision §1](./vision.md#1-the-problem) (memory tools did not help) |

## Adding to this list

A new idea goes first into [the vision](./vision.md) (its "Adding to this file" rules), then gets
one row here. A row changes status in place; a delivered feature is removed (its PR records it).
