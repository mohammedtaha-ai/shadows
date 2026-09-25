# Code map

**Read this before writing code in `shadows`.** Two files, split by what can be
checked:

| File | Kind | Answers |
|---|---|---|
| [`inventory.md`](./inventory.md) | generated from `src/` | *Does this already exist?* Every reachable declaration with its full signature. |
| this file | written by hand | *Where does my new code go?* The one job each module holds. |

`cargo test --test codemap` fails when either has drifted from the tree, so
neither can go stale quietly. Regenerate the inventory with
`UPDATE_CODEMAP=1 cargo test --test codemap` in the same commit as the code
change that moved it.

## What each module owns

One job per module, stated without "and" — a conjunction here means the module
has quietly taken a second responsibility. The reference file is the one to read
before adding to that module: it is the pattern to follow, not merely an example.

| Module | Its one job | Reference file |
|---|---|---|
| `src/agent/` | the AI subprocess harness contract | `src/agent/acp.rs` |
| `src/agent/acp.rs` | the ACP client connection to one adapter process | `src/agent/acp.rs` |
| `src/agent/events.rs` | what a harness connection reports | `src/agent/events.rs` |
| `src/agent/policy.rs` | the modes Shadows allows per harness | `src/agent/policy.rs` |
| `src/agent/choices.rs` | reading the harness's offered choices | `src/agent/choices.rs` |
| `src/agent/breakdown.rs` | reading Claude's `/context` answer | `src/agent/breakdown.rs` |
| `src/planner/offers.rs` | the latest choices each open session offers | `src/planner/offers.rs` |
| `src/planner/context.rs` | reading a session's context breakdown on demand | `src/planner/context.rs` |
| `src/protocol/harness.rs` | the routes over harnesses, their sessions included | `src/protocol/harness.rs` |
| `src/protocol/thread.rs` | the routes that change a planning thread itself | `src/protocol/thread.rs` |
| `src/planner/sessions.rs` | the live adapter connection each open thread holds | `src/planner/sessions.rs` |
| `src/planner/settings.rs` | setting an open session's options | `src/planner/settings.rs` |
| `src/planner/turn.rs` | the recorded ending of a live Planner turn | `src/planner/turn.rs` |
| `src/planner/entries.rs` | turning harness events into durable entries | `src/planner/entries.rs` |
| `src/bin/` | test apparatus that no product code links | `src/bin/tree_probe.rs` |
| `src/cli/` | daemon startup | `src/cli/args.rs` |
| `src/command/` | external-command identity for idempotency | `src/command/mod.rs` |
| `src/command/derive.rs` | command ids Shadows derives when a caller names none | `src/command/derive.rs` |
| `src/config.rs` | startup configuration resolved once | `src/config.rs` |
| `src/error.rs` | the stable failure taxonomy clients match on | `src/error.rs` |
| `src/events/` | the durable event record's shape | `src/events/mod.rs` |
| `src/id.rs` | the UUID id newtype pattern | `src/id.rs` |
| `src/mcp/` | Shadows' MCP server | `src/mcp/mod.rs` |
| `src/mcp/grant.rs` | who may do what on `/mcp` | `src/mcp/grant.rs` |
| `src/operation/` | the operation lifecycle's shape | `src/operation/mod.rs` |
| `src/planner/` | the Planner turn's spawn-through-termination lifecycle | `src/planner/mod.rs` |
| `src/process/` | OS process ownership with whole-tree containment | `src/process/mod.rs` |
| `src/project/` | the project: its identity, the directory it owns | `src/project/mod.rs` |
| `src/protocol/` | the HTTP/SSE surface every client talks to | `src/protocol/project.rs` |
| `src/runtime/` | the runtime instance's lifecycle | `src/runtime/mod.rs` |
| `src/storage/` | persistence | `src/storage/sqlite/project.rs` |
| `src/storage/sqlite/workflow.rs` | changing a plan version | `src/storage/sqlite/workflow.rs` |
| `src/storage/sqlite/workflow_draft.rs` | starting a plan version | `src/storage/sqlite/workflow_draft.rs` |
| `src/storage/sqlite/workflow_read.rs` | reading plan versions | `src/storage/sqlite/workflow_read.rs` |
| `src/storage/sqlite/task.rs` | a plan version's task graph rows | `src/storage/sqlite/task.rs` |
| `src/storage/sqlite/grant.rs` | what a writer's MCP grant permits a write | `src/storage/sqlite/grant.rs` |
| `src/thread/` | the planning thread's shape | `src/thread/mod.rs` |
| `src/tracing.rs` | tracing subscriber setup | `src/tracing.rs` |
| `src/workflow/` | a plan's content under the rules of §13 | `src/workflow/mod.rs` |
| `src/workflow/ops.rs` | applying one batch of plan edits | `src/workflow/ops.rs` |
| `src/workflow/check.rs` | what makes a plan invalid or unready | `src/workflow/check.rs` |

The Web client in `web/` is a separate program outside this crate and this map;
[`web/README.md`](../../web/README.md) describes it.

A module absent from this table is a module that does not exist yet. The fifteen
planned modules are listed in [`CLAUDE.md`](../../CLAUDE.md); this table is not a
second copy of that list, and the tree is what decides which of them are real.

**This table is a working summary, not authority.** `CLAUDE.md` owns the five
single-ownership invariants and the file-size rules; the specs indexed by
[`specs/README.md`](../superpowers/specs/README.md) own the design. Where this
file disagrees with either, they are right and this file is the defect.

## What is deliberately not here

- **Line numbers.** Wrong at the first line inserted above them, with nothing
  failing when they lie. One field that rots silently costs the reader their
  trust in every field beside it.
- **Built / not-built status.** The tree answers it and
  [`docs/status.md`](../status.md) narrates progress. A third copy would record
  one fact in three places.
- **Explanations of declarations.** Those live as doc comments on the
  declarations themselves, which is why the inventory strips them.
