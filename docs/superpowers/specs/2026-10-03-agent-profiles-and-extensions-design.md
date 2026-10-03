# Section 19 — Agent Profiles and Extensions

- **Date:** 2026-10-03.
- **Status:** Draft for Mohammed's review; no product implementation authorized.
- **Intent:** [vision §6](../../vision.md#6-executors-and-their-environment).
- **Related owners:** §12 harness controls, §13 Planner sessions, §17 task
  execution/evidence, §18 shared agreements.

This section owns user-configured specialist profiles, extension selection,
provider connections and their dispatch bindings. It does not replace §17's
run lifecycle, implement a new general agent harness, or settle the manager's
full decision policy. Many saved profiles do not imply many concurrent runs.

## 19.1 Distinct concepts

| Concept | Responsibility |
|---|---|
| Agent profile | Reusable role, instructions, allowed tools, output expectations and resource limits. |
| Harness | The CLI or runtime that conducts the agent's tool loop. |
| Provider connection | Endpoint/protocol, model choices and a credential reference used by a compatible harness. |
| Extension | A versioned skill, agent template, plugin package or MCP connection enabled for selected profiles. |
| Dispatch rule | When a profile is invoked, what context it receives and where its result goes. |
| Run | One invocation against captured inputs and configuration revisions. |

A person can create an executor, critic, library reviewer or web researcher
from the Dashboard, write its instructions and select its connection and
extensions. The initial scope is project-owned profiles; cross-project
sharing/import is not an implicit grant to another project's data.
Descriptions and instructions cannot grant authority denied by the runtime.

ACP is Shadows' harness-control boundary: prefer an ACP-capable harness or a
tested ACP adapter for candidates such as jcode, OpenCode and Codex. Provider
and model routing remains that harness's responsibility. ACP compatibility
does not by itself prove a particular provider, tool policy or recovery path.

## 19.2 Profiles and captured configuration

A profile has stable identity and immutable numbered revisions. Editing uses
an expected revision; conflicts never overwrite another editor. A run captures
the profile revision, resolved instructions, harness/version, requested model,
observed model when reported, connection identity, extension versions and
effective capability set. Unknown observed values remain unknown.

Edits affect future dispatches, not running attempts. Disabling a profile blocks
new dispatches; stopping an existing run is a separate explicit action. Old
results retain enough configuration provenance to remain explainable without
retaining credentials in their records.

Effective permissions are the intersection of task/run authorization, profile
policy and the adapter's enforceable capabilities. Imported tools and prompts
cannot widen that intersection. Required unsupported capabilities block
dispatch with an actionable reason.

## 19.3 Critic and specialist workflow

The first automated rule invokes a selected critic after an implementation
attempt has recorded its changes and configured deterministic checks have
settled. A process exit or the executor saying "done" is not that event.
Failed/incomplete checks stay visible; the critic cannot turn them into passes.

The critic receives §17's task obligations, exact reviewed source/diff and
check evidence, plus relevant dependencies. It does not inherit the full
executor conversation. Its result uses §17.4's review outcomes and cites
concrete findings, affected acceptance items and evidence. A missing review
is not a pass. The critic is read-only by default and sends findings to the
manager, or the person/Planner before a manager exists. Repair is assigned as
authorized work; it is not silently performed by the critic.

A review is bound to one attempt and its result digest. Subsequent edits
require new verification/review; an old PASS never approves unseen changes.
Where review is required, task acceptance waits for both the configured
checks and review. The configured policy decides the treatment of
PASS_WITH_NOTES; absent such a policy it requires a person decision.

Library reviewers and web researchers can run manually or at an explicit
planning/review point. Their context names the question, relevant dependencies
and output expectations. Research records sources and uncertainty; advice is
not a change to an approved contract. They do not run after every task unless
the person deliberately selects such a rule.

Dispatch is durable and idempotent for the source attempt/result, rule revision
and profile revision. Duplicate completion events cannot start duplicate
reviews. Explicit retries get their own attempt identity. Reviewer completion
does not recursively trigger the same post-implementation rule. Failed or
interrupted specialists remain visible; no silent success or unbounded repair
loop. The controller's configured retry/cost limit bounds further dispatch.

## 19.4 Plugins and MCP in the Dashboard

The Dashboard separates browsing/importing, installing, enabling and updating.
It shows source, selected version or commit, included components, requested
tools and target-harness compatibility. A person selects which profiles use
which components. Updates are explicit; a run keeps its captured versions.
Historical results remain readable after an extension is disabled or removed.

An MCP connection exposes tools; a plugin may bundle skills, agent templates,
hooks and MCP configuration. A marketplace such as `wshobson/agents` is a
catalog/package source, not itself an MCP server. Importing a role template
creates a reviewable profile; it does not automatically execute its commands.
Use native harness formats when supported. Unsupported components are shown
as unsupported, never silently converted or reported as active.

Loading is selective: expose only enabled tools and relevant instruction
packages to a role. Account for descriptions and schemas that a harness adds
even before a skill runs. Extensions obey Shadows' task and approval
boundaries; hooks or nested agents that bypass these boundaries cannot be
enabled for bounded execution. Harness hooks are not the authoritative
delivery mechanism for Shadows' durable workflow rules.

## 19.5 Provider connections and credentials

The person can configure provider endpoint/protocol, a model and an API-key
reference, then select a compatible harness for a profile. CLI-managed login
is a separate connection mode; Shadows does not extract CLI OAuth tokens.
Secret values are resolved at launch and excluded from prompts, profile
exports, events and logs. UI writes must not echo a supplied secret back.
No credential implementation is introduced by this draft.

Compatibility is per harness/version, protocol and model configuration, not
just a successful text response. Before marking a combination tested, run a
disposable task demonstrating tool calls, streaming/result collection, scope
enforcement, cancellation and recovery. Record date, versions and limitations.
Display official support separately from local experimental evidence. Never
silently route to a different model/provider on failure.

Claude Code's gateway documentation explicitly does not support non-Claude
models through a gateway. Preserve the user's ability to use other providers
through an appropriate supported adapter; a third-party compatibility route
is experimental until demonstrated and is never labelled official support.

## 19.6 Delivery and acceptance

Build profile selection with the first real executor/critic caller in roadmap
Stage 4. Stage 5 adds manager-directed specialist dispatch, selected extension
installation and additional tested provider connections in separate runnable
slices. No complete marketplace, all-provider matrix or new service is a
prerequisite for the first serial task. Core/service/migration ownership is
assigned through §14 and the code map when implementation is planned.

Acceptance must demonstrate:

1. Dashboard profile creation and versioned editing; an in-flight run retains
   its captured settings while the next run uses the new revision.
2. One completed implementation invokes its critic once despite duplicate
   events/restart; findings reach the controller and a repair is reviewed anew.
3. A library reviewer or web researcher receives only its selected task context
   and returns attributable evidence without changing the approved plan.
4. One selected plugin and one MCP connection are enabled for a chosen profile;
   unsupported components and missing capabilities block affected dispatches.
5. A provider connection passes the real adapter trial; credentials remain out
   of prompt/event/export artifacts and failed setup never falls back silently.
6. Two saved profiles do not create two running agents. Concurrency is controlled
   by ready task dependencies and the independently validated conflict policy.

> **OPEN — credential backend and adapter configuration isolation.** Before
> saving the first credential or installing an extension, choose the secret
> resolver and per-project/per-profile configuration boundary. Prove that one
> profile's provider, tools or hooks cannot leak into another profile's run.
> This does not block a first profile using the existing CLI-owned login.

§17.5 owns the open manager-authority/recovery prerequisite. Shared-folder
concurrency remains outside §17.1 until its conflict design is proven. Profiles
and serial critic dispatch do not authorize either capability.
