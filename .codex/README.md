# Shadows Codex agents

Project-scoped agents for this repository. `shadows_backend`,
`shadows_frontend` and `shadows_reviewer` use `gpt-6.1-terra`; the other roles use
`gpt-6.1-sol`. `shadows_reviewer` uses `model_reasoning_effort = "high"`; every other
role uses `"medium"`. The project config sets
`gpt-6.1-sol` and `medium` as defaults
for unnamed subagents. It does not change the parent chat's model or personal
Codex configuration. `.claude/agents/` belongs to Claude Code and is unchanged.

## Roles

| Name | Use for | Allowed work |
|---|---|---|
| `shadows_architect` | Ownership, dependency analysis, specifications and task preparation | Read code; edit explicitly assigned planning documents |
| `shadows_backend` | One Core/backend implementation task or targeted fix | Its assigned Rust/backend scope, contracts and tests |
| `shadows_frontend` | One Core web implementation task or targeted fix | Its assigned `web/` scope, contracts and tests |
| `shadows_aylul` | One desktop UI, native, sidecar, hook or character task | One explicitly selected AYLUL ownership scope |
| `shadows_reviewer` | Independent review of a named task or commit range | Review and fix within assigned scope; read-only when explicitly requested |
| `shadows_docs` | Synchronize maps, contracts, stages or post-merge documentation | Assigned documentation only |

The parent chat remains the controller. Builders fix their own failed changes;
there is no separate fixer profile for each side. An architect is not a second
controller and does not silently authorize work or accept a shared request.

## Dispatch protocol

Use these roles when the user or applicable project/skill instructions authorize
delegation. Installing these definitions does not itself start implementation or
authorize autonomous delegation. Send one complete assignment containing:

- Task ID and stages/plan path, or a precise standalone objective.
- Base commit and current checkout, owning service/contract, relevant decisions.
- Exact allowed paths, required checks, dependencies and acceptance criteria.
- Read-only or implementation/review-and-fix mode, and commit ownership.

A task already containing these details can be referenced instead of copied.
Do not dispatch a blocked task as executable work. Missing owners, unresolved
dependencies or extra required paths return to the controller. Follow-ups are
for fix rounds; do not resend the entire assignment or duplicate finished work.

Select the named custom agent using the host's supported role selector. Some
hosts expose `agent_type`; the collaboration API in other hosts exposes only
`task_name`, `message`, `model`, `reasoning_effort` and `fork_turns`. In that case,
read the selected TOML and include its `developer_instructions` in the assignment,
using the selected TOML's `model`, `reasoning_effort = "medium"` and
`fork_turns = "none"`. A task name alone does not load a role. Never invent an
unsupported selector or claim a profile was loaded without evidence.

## Shared working rules

1. Read root `AGENTS.md`, the applicable side instructions, and the assigned task.
   For Core start at `crates/shadows-core/src/app.rs`, then its owner contract.
   Follow current owner specs when an older instruction disagrees. Locate Rust
   symbols through the LSP and TypeScript symbols through Serena. Text search is
   for docs, config and logs; report unavailable semantic navigation honestly.
2. Stay within `Touch:`. Preserve others' work. No branch switch, stash, reset,
   worktree build, push, merge or deployment is implied. Do not start other agents
   or message other chats; report to the parent through the agent result.
3. The controller serializes mutations and build commands in a shared checkout.
   Never run cargo and vitest in parallel. Reuse each workspace's existing cache;
   no `CARGO_TARGET_DIR`, `cargo clean`, release/custom target/profile or changed
   `RUSTFLAGS`. A locked running executable is reported, not killed.
4. Keep rules with their service, stores private, and API changes behind the
   accepted shared request. Update affected contracts/maps with the change.
   Generate public API artifacts using the repository commands; never hand-edit
   generated schemas. Do not copy planned ownership into current contracts.
5. Run affected checks for an ordinary task. A full gate runs only when the
   controller assigns the branch acceptance gate. Keep one whole-branch review;
   fix rounds review the delta unless a new finding justifies a broader pass.
6. Follow the task's commit ownership. Default completed implementation tasks get
   a scoped local commit with explicit-path staging after required checks. An
   explicitly read-only review edits and commits nothing. Report blocked checks;
   do not mark a task done or claim verification when its required check did not run.
7. Code, prompts, comments and test assertions are English. Reports to the parent
   name the result, commit/diff, exact checks and remaining issues. User-facing
   summaries can be concise Arabic. Separate static inspection, automated tests,
   browser fixtures, native acceptance, provider tests and production evidence.

There are at most three spawned agents concurrently. This is a capacity limit,
not a requirement to use three: one builder followed by one reviewer is the
normal implementation sequence. Parallel read-only analysis is useful when the
assignments are independent. Do not run overlapping writers or build commands.

## Loading and verification

Custom agents are standalone TOML files in `.codex/agents/`, with `name`,
`description`, `developer_instructions`, `model` and `model_reasoning_effort`.
Project configuration must be enabled/trusted by the host. A running chat may
need a fresh session to discover new roles; the explicit-dispatch fallback above
does not depend on hot reload. Permissions and connected tools come from the
parent; these instructions are not a separate sandbox.

Reference: [OpenAI custom-agent documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents#custom-agents).
