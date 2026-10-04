# Focused agent orchestration: source inspection

- **Date:** 2026-10-03.
- **Local baseline inspected:** `4bde8e1`, before this documentation revision.
- **Method:** static local source inspection and primary web documentation.
  No external agent system, Shadows executor, benchmark or browser trial ran.
- **Purpose:** ground the clarification in vision §§2, 5 and the planning
  roadmap. This record is evidence, not design authority or adoption of an
  external framework. No executable probe was created.

## Local source observations

The code map and service contracts were the entry points. Shadows MCP and
an LSP tool were not connected to this chat. A rust-analyzer executable was
present, but no LSP query was executed. Files named by the map/contracts were
read directly; bounded text searches supplied the line references below.

| Source at the inspected revision | Observation |
|---|---|
| `crates/shadows-core/src/app.rs:55` | `AppCore` composes nine services including `Code` and `Plans`; no execution or manager service appears in this composition. |
| `crates/shadows-core/src/code/mod.rs:150` | `definitions` delegates a name query to the index. |
| `crates/shadows-core/src/code/mod.rs:160` | `references` marks every returned hit `matched_by: name`; it does not disambiguate semantic callees. |
| `crates/shadows-core/src/code/mod.rs:175` | `outline` queries indexed definitions for a path and attempts rechecking. |
| `crates/shadows-core/src/plans/model.rs:135` | `TaskContent` currently contains number, title, goal, reads, writes and acceptance; §17's enriched task packet remains design. |
| `crates/shadows-core/src/harness/setup.rs:64` | Planner setup supplies MCP configuration and instructions; it is not a per-task execution packet compiler. |
| `crates/shadows-core/src/harness/setup.rs:115` | Turn context refreshes changed instructions; this is not evidence of task-specific source selection. |

These observations support targeted symbol navigation as an existing building
block. They do not establish executor containment, manager behavior, semantic
LSP integration, reduced tokens or accepted end-to-end execution.

## External primary sources read

### OpenClaw

[Sub-agents](https://docs.openclaw.ai/tools/subagents) describes separate child
sessions, their own token cost and configurable model selection.
[Nested sub-agents](https://docs.openclaw.ai/tools/subagents/nesting) describes
depth/admission limits and results returning through direct parents.
The useful observed mechanisms are isolated sessions and bounded delegation;
these pages do not prove cheaper software delivery for Shadows.

### Hermes Agent

[Subagent delegation](https://hermes-agent.nousresearch.com/docs/user-guide/features/delegation/)
describes fresh child conversations supplied with goal/context, with workspace
instructions also injected. It documents structured results and iteration
limits. It warns of collisions when children edit a shared repository and
offers optional worktree isolation. Context isolation and filesystem isolation
are distinct properties. This review did not run or audit Hermes enforcement.

### Cursor

[Scaling long-running autonomous coding](https://cursor.com/blog/scaling-agents),
published 2026-01-14, reports hundreds of concurrent agents. Its described
roles separate planners, focused workers and judging. The authors report
bottlenecks with flat shared coordination and emphasize role/model selection,
simplicity and remaining inefficiency. These are the authors' experiment
reports, not an independently reproduced throughput or cost benchmark here.

## Interpretation and unmeasured questions

The sources support the feasibility of role separation and focused delegation.
They do not establish that any weak model can implement every well-planned task,
that disjoint writes make a shared workspace safe, or that more agents reduce
total cost. Shadows still needs a controlled comparison of accepted work with
ordinary discovery versus prepared task context, including preparation,
coordination, review, retries, time and human interventions. The roadmap owns
that validation requirement; §17 owns packet and evidence semantics.

## Follow-up: configurable roles and extensions

Mohammed clarified that the goal is Dashboard-defined specialists and a small
number of independent tasks running together, not 50 concurrent workers.

[wshobson/agents](https://github.com/wshobson/agents) describes a marketplace
containing agent definitions, skills and commands, with harness-specific
installation formats. It is not itself an MCP server. Counts and supported
targets can change; none was installed or executed in this review.

[Claude Code plugins](https://code.claude.com/docs/en/plugins) documents bundles
of skills, agents, hooks and MCP servers. Enabled components can add context
metadata even when their full instructions are not invoked. This supports
selective per-role activation rather than loading a complete catalog.

[Claude Code gateways](https://code.claude.com/docs/en/llm-gateway) explicitly
states that routing Claude Code to non-Claude models through a gateway is not
supported by Anthropic. Protocol compatibility and a local successful trial
must not be reported as official support. No provider combination was tested.

Local `crates/shadows-agent/src/claude.rs` constructs the configured ACP adapter
process; `crates/shadows-core/src/harness/model.rs` exposes harness/model UI
shapes. Reading those files does not establish Dashboard-managed provider keys
or plugin installation. The new §19 is a design draft for that requested scope.
