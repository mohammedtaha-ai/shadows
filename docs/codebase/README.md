# Code map

**Read this before writing code in `shadows`.** Two files, split by what can be
checked:

| File | Kind | Answers |
|---|---|---|
| [`inventory.md`](./inventory.md) | generated from each `crates/*/src` | *Does this already exist?* Every reachable declaration with its full signature. |
| this file | written by hand | *Where does my new code go?* The one job each module holds. |

`cargo test -p shadows --test codemap` fails when either has drifted from the
tree, so neither can go stale quietly. The map spans the whole workspace: every
path below is relative to the repository root. Regenerate the inventory with
`UPDATE_CODEMAP=1 cargo test -p shadows --test codemap` in the same commit as
the code change that moved it.

## What each module owns

One job per module, stated without "and" — a conjunction here means the module
has quietly taken a second responsibility. The reference file is the one to read
before adding to that module: it is the pattern to follow, not merely an example.

| Module | Its one job | Reference file |
|---|---|---|
| `crates/shadows-agent/src/lib.rs` | the AI subprocess harness contract | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp.rs` | the ACP client connection to one adapter process | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/events.rs` | what a harness connection reports | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/policy.rs` | the modes Shadows allows per harness | `crates/shadows-agent/src/policy.rs` |
| `crates/shadows-agent/src/choices.rs` | reading the harness's offered choices | `crates/shadows-agent/src/choices.rs` |
| `crates/shadows-agent/src/breakdown.rs` | reading Claude's `/context` answer | `crates/shadows-agent/src/breakdown.rs` |
| `crates/shadows-agent/src/claude.rs` | the launch spec of the pinned Claude ACP adapter | `crates/shadows-agent/src/claude.rs` |
| `crates/shadows-core/src/planner/offers.rs` | the latest choices each open session offers | `crates/shadows-core/src/planner/offers.rs` |
| `crates/shadows-core/src/planner/context.rs` | reading a session's context breakdown on demand | `crates/shadows-core/src/planner/context.rs` |
| `crates/shadows-http/src/harness.rs` | the routes over harnesses, their sessions included | `crates/shadows-http/src/harness.rs` |
| `crates/shadows-http/src/thread.rs` | the routes that change a planning thread itself | `crates/shadows-http/src/thread.rs` |
| `crates/shadows-http/src/workflow.rs` | the routes over plan versions | `crates/shadows-http/src/workflow.rs` |
| `crates/shadows-http/src/grants.rs` | the routes over external agents' MCP grants | `crates/shadows-http/src/grants.rs` |
| `crates/shadows-http/src/instructions.rs` | the routes over a project's Planner instructions | `crates/shadows-http/src/instructions.rs` |
| `crates/shadows-core/src/planner/sessions.rs` | the live adapter connection each open thread holds | `crates/shadows-core/src/planner/sessions.rs` |
| `crates/shadows-core/src/planner/settings.rs` | setting an open session's options | `crates/shadows-core/src/planner/settings.rs` |
| `crates/shadows-core/src/planner/setup.rs` | what a Planner session opens with | `crates/shadows-core/src/planner/setup.rs` |
| `crates/shadows-core/src/planner/turn.rs` | the recorded ending of a live Planner turn | `crates/shadows-core/src/planner/turn.rs` |
| `crates/shadows-core/src/planner/entries.rs` | turning harness events into durable entries | `crates/shadows-core/src/planner/entries.rs` |
| `crates/fake-acp/src/main.rs` | test apparatus that no product code links | `crates/fake-acp/src/main.rs` |
| `crates/shadows-core/src/testing.rs` | the test apparatus every crate's tests share | `crates/shadows-core/src/testing.rs` |
| `crates/shadows-process/src/bin/` | test apparatus that no product code links | `crates/shadows-process/src/bin/tree_probe.rs` |
| `crates/shadows/src/cli/` | daemon startup | `crates/shadows/src/cli/args.rs` |
| `crates/shadows-core/src/command/` | external-command identity for idempotency | `crates/shadows-core/src/command/mod.rs` |
| `crates/shadows-core/src/command/derive.rs` | command ids Shadows derives when a caller names none | `crates/shadows-core/src/command/derive.rs` |
| `crates/shadows/src/config.rs` | startup configuration resolved once | `crates/shadows/src/config.rs` |
| `crates/shadows-core/src/error.rs` | the stable failure taxonomy clients match on | `crates/shadows-core/src/error.rs` |
| `crates/shadows-core/src/events/` | the event shapes the product records or signals | `crates/shadows-core/src/events/mod.rs` |
| `crates/shadows-core/src/id.rs` | the UUID id newtype pattern | `crates/shadows-core/src/id.rs` |
| `crates/shadows-mcp/src/lib.rs` | Shadows' MCP server | `crates/shadows-mcp/src/lib.rs` |
| `crates/shadows-core/src/grant/` | who may do what on `/mcp` | `crates/shadows-core/src/grant/mod.rs` |
| `crates/shadows-mcp/src/auth.rs` | refusing a `/mcp` request that holds no live grant | `crates/shadows-mcp/src/auth.rs` |
| `crates/shadows-mcp/src/server.rs` | the tools a grant's kind may see | `crates/shadows-mcp/src/server.rs` |
| `crates/shadows-mcp/src/tools.rs` | each MCP tool's storage call | `crates/shadows-mcp/src/tools.rs` |
| `crates/shadows-mcp/src/refusal.rs` | what an MCP tool call answers | `crates/shadows-mcp/src/refusal.rs` |
| `crates/shadows-core/src/operation/` | the operation lifecycle's shape | `crates/shadows-core/src/operation/mod.rs` |
| `crates/shadows-core/src/planner/` | the Planner turn's spawn-through-termination lifecycle | `crates/shadows-core/src/planner/mod.rs` |
| `crates/shadows-process/src/lib.rs` | OS process ownership with whole-tree containment | `crates/shadows-process/src/lib.rs` |
| `crates/shadows-core/src/project/` | the project: its identity, the directory it owns | `crates/shadows-core/src/project/mod.rs` |
| `crates/shadows-http/src/lib.rs` | the HTTP/SSE surface every client talks to | `crates/shadows-http/src/project.rs` |
| `crates/shadows-http/src/project.rs` | the routes over projects, their threads included | `crates/shadows-http/src/project.rs` |
| `crates/shadows-http/src/conversation.rs` | the routes over a thread's conversation | `crates/shadows-http/src/conversation.rs` |
| `crates/shadows-http/src/sse.rs` | the replay-then-live stream | `crates/shadows-http/src/sse.rs` |
| `crates/shadows-http/src/fs.rs` | choosing a project directory on this machine | `crates/shadows-http/src/fs.rs` |
| `crates/shadows-http/src/openapi.rs` | the OpenAPI document describing this API | `crates/shadows-http/src/openapi.rs` |
| `crates/shadows-http/src/failure.rs` | the transport mapping of a failure | `crates/shadows-http/src/failure.rs` |
| `crates/shadows-http/src/guard.rs` | refusing requests pages were made to send | `crates/shadows-http/src/guard.rs` |
| `crates/shadows-core/src/runtime/` | the runtime instance's lifecycle | `crates/shadows-core/src/runtime/mod.rs` |
| `crates/shadows-core/src/storage/` | persistence | `crates/shadows-core/src/storage/sqlite/project.rs` |
| `crates/shadows-core/src/storage/sqlite/workflow.rs` | changing a plan version | `crates/shadows-core/src/storage/sqlite/workflow.rs` |
| `crates/shadows-core/src/storage/sqlite/workflow_draft.rs` | starting a plan version | `crates/shadows-core/src/storage/sqlite/workflow_draft.rs` |
| `crates/shadows-core/src/storage/sqlite/workflow_read.rs` | reading plan versions | `crates/shadows-core/src/storage/sqlite/workflow_read.rs` |
| `crates/shadows-core/src/storage/sqlite/task.rs` | a plan version's task graph rows | `crates/shadows-core/src/storage/sqlite/task.rs` |
| `crates/shadows-core/src/storage/sqlite/grant.rs` | an MCP grant's rows, from issue to revocation | `crates/shadows-core/src/storage/sqlite/grant.rs` |
| `crates/shadows-core/src/storage/sqlite/instructions.rs` | a project's numbered Planner instructions | `crates/shadows-core/src/storage/sqlite/instructions.rs` |
| `crates/shadows-core/src/storage/sqlite/plan_view.rs` | showing a plan version in its conversation | `crates/shadows-core/src/storage/sqlite/plan_view.rs` |
| `crates/shadows-core/src/thread/` | the planning thread's shape | `crates/shadows-core/src/thread/mod.rs` |
| `crates/shadows/src/tracing.rs` | tracing subscriber setup | `crates/shadows/src/tracing.rs` |
| `crates/shadows-core/src/workflow/` | a plan's content under the rules of §13 | `crates/shadows-core/src/workflow/mod.rs` |
| `crates/shadows-core/src/workflow/ops.rs` | applying one batch of plan edits | `crates/shadows-core/src/workflow/ops.rs` |
| `crates/shadows-core/src/workflow/check.rs` | what makes a plan invalid or unready | `crates/shadows-core/src/workflow/check.rs` |
| `crates/shadows-core/src/workflow/conversation.rs` | the plan in the conversation | `crates/shadows-core/src/workflow/conversation.rs` |

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
