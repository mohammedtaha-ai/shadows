# Code map

**Read this before writing code in `shadows`.** Two kinds of file, split by
what can be checked:

| File | Kind | Answers |
|---|---|---|
| this file | written by hand | *Where does my new code go?* The one job each crate and module holds, and the invariants no change may break. |
| each service's `contract.yaml` | written by hand, next to its code in `crates/shadows-core/src/<service>/` | *What must stay true of this service?* Its methods, obligations, agreements and tests, in the shape of [`contracts/TEMPLATE.yaml`](./contracts/TEMPLATE.yaml). |

`cargo test -p shadows --test codemap` fails when this file names a path that
does not exist or misses a module that does, and
`cargo test -p shadows-core --test contracts` when a contract names what its
service does not have (spec §14.7), so neither can go stale quietly. The map
spans the whole workspace: every path below is relative to the repository
root. *Does this already exist, and with what signature?* is not written down:
the Rust LSP, or `where_is`, `who_uses` and `outline` when the `shadows` MCP
server is connected, answer it from the code.

## Architecture Invariants

What the code keeps true, in rust-analyzer's style. Spec §14.6 owns them.

- `shadows-http` knows HTTP; nothing below it does. `shadows-core`,
  `shadows-agent` and `shadows-process` depend on no web framework and name no
  status code; the types an adapter serializes derive `utoipa::ToSchema`, which
  describes a shape and names no route.
- `shadows-core` never imports `axum` or `rmcp`.
- `shadows-index` knows nothing of SQLite or projects. It depends on no
  Shadows crate, nor on a database library; spec §15.2 owns this one.
- Another service calls a service's `store` only through a function or method
  that store's contract declares, under `shared_in_transaction` (inside one
  write) or `called_by_other_services`.

## What each crate and module owns

One job per row, stated without "and" — a conjunction here means the module
has quietly taken a second responsibility. A crate's row names its entry point
(`lib.rs` or `main.rs`); the rows after it are that crate's modules. The
reference file is the one to read before adding to that module: it is the
pattern to follow, not merely an example.

| Module | Its one job | Reference file |
|---|---|---|
| `crates/shadows/src/main.rs` | the one binary Shadows ships | `crates/shadows/src/cli/mod.rs` |
| `crates/shadows/src/cli/` | daemon startup | `crates/shadows/src/cli/args.rs` |
| `crates/shadows/src/config.rs` | startup configuration resolved once | `crates/shadows/src/config.rs` |
| `crates/shadows/src/tracing.rs` | tracing subscriber setup | `crates/shadows/src/tracing.rs` |
| `crates/shadows-http/src/lib.rs` | the HTTP/SSE surface every client talks to | `crates/shadows-http/src/project.rs` |
| `crates/shadows-http/src/project.rs` | the routes over projects, their threads included | `crates/shadows-http/src/project.rs` |
| `crates/shadows-http/src/conversation.rs` | the routes over a thread's conversation | `crates/shadows-http/src/conversation.rs` |
| `crates/shadows-http/src/harness.rs` | the routes over harnesses, their sessions included | `crates/shadows-http/src/harness.rs` |
| `crates/shadows-http/src/thread.rs` | the routes that change a planning thread itself | `crates/shadows-http/src/thread.rs` |
| `crates/shadows-http/src/workflow.rs` | the routes over plan versions | `crates/shadows-http/src/workflow.rs` |
| `crates/shadows-http/src/grants.rs` | the routes over external agents' MCP grants | `crates/shadows-http/src/grants.rs` |
| `crates/shadows-http/src/instructions.rs` | the routes over a project's Planner instructions | `crates/shadows-http/src/instructions.rs` |
| `crates/shadows-http/src/design.rs` | the routes over a project's design workspace | `crates/shadows-http/src/design.rs` |
| `crates/shadows-http/src/code.rs` | the routes over the code index | `crates/shadows-http/src/code.rs` |
| `crates/shadows-http/src/sse.rs` | the replay-then-live stream's SSE framing | `crates/shadows-http/src/sse.rs` |
| `crates/shadows-http/src/fs.rs` | choosing a project directory on this machine | `crates/shadows-http/src/fs.rs` |
| `crates/shadows-http/src/openapi.rs` | the OpenAPI document describing this API | `crates/shadows-http/src/openapi.rs` |
| `crates/shadows-http/src/failure.rs` | the transport mapping of a failure | `crates/shadows-http/src/failure.rs` |
| `crates/shadows-http/src/guard.rs` | refusing requests pages were made to send | `crates/shadows-http/src/guard.rs` |
| `crates/shadows-mcp/src/lib.rs` | Shadows' MCP server | `crates/shadows-mcp/src/lib.rs` |
| `crates/shadows-mcp/src/auth.rs` | refusing a `/mcp` request that holds no live grant | `crates/shadows-mcp/src/auth.rs` |
| `crates/shadows-mcp/src/server.rs` | the tools a grant's kind may see | `crates/shadows-mcp/src/server.rs` |
| `crates/shadows-mcp/src/tools.rs` | each MCP tool's one service call | `crates/shadows-mcp/src/tools.rs` |
| `crates/shadows-mcp/src/refusal.rs` | what an MCP tool call answers | `crates/shadows-mcp/src/refusal.rs` |
| `crates/shadows-core/src/lib.rs` | the application behind `AppCore` | `crates/shadows-core/src/app.rs` |
| `crates/shadows-core/src/app.rs` | the application's composition root | `crates/shadows-core/src/app.rs` |
| `crates/shadows-core/src/error.rs` | the stable failure taxonomy clients match on | `crates/shadows-core/src/error.rs` |
| `crates/shadows-core/src/id.rs` | the UUID id newtype pattern | `crates/shadows-core/src/id.rs` |
| `crates/shadows-core/src/command/` | external-command identity for idempotency | `crates/shadows-core/src/command/mod.rs` |
| `crates/shadows-core/src/command/derive.rs` | command ids Shadows derives when a caller names none | `crates/shadows-core/src/command/derive.rs` |
| `crates/shadows-core/src/db/` | the database no service owns | `crates/shadows-core/src/db/mod.rs` |
| `crates/shadows-core/src/runtime/` | the runtime instance's lifecycle | `crates/shadows-core/src/runtime/mod.rs` |
| `crates/shadows-core/src/testing/` | the test apparatus every crate's tests share | `crates/shadows-core/src/testing/mod.rs` |
| `crates/shadows-core/src/projects/` | the projects, the folders a person picks them from included | `crates/shadows-core/src/projects/mod.rs` |
| `crates/shadows-core/src/projects/model.rs` | the project types callers meet | `crates/shadows-core/src/projects/model.rs` |
| `crates/shadows-core/src/projects/store.rs` | a project's rows | `crates/shadows-core/src/projects/store.rs` |
| `crates/shadows-core/src/projects/browse.rs` | showing a person the directories they can choose from | `crates/shadows-core/src/projects/browse.rs` |
| `crates/shadows-core/src/threads/` | the planning threads, what they recorded included | `crates/shadows-core/src/threads/mod.rs` |
| `crates/shadows-core/src/threads/model.rs` | the planning thread types callers meet | `crates/shadows-core/src/threads/model.rs` |
| `crates/shadows-core/src/threads/rules.rs` | which harness a thread may name | `crates/shadows-core/src/threads/rules.rs` |
| `crates/shadows-core/src/threads/title.rs` | the text a title taken from a conversation may read | `crates/shadows-core/src/threads/title.rs` |
| `crates/shadows-core/src/threads/store/` | threads' SQLite queries | `crates/shadows-core/src/threads/store/thread.rs` |
| `crates/shadows-core/src/threads/store/remove.rs` | atomic removal of a planning thread | `crates/shadows-core/src/threads/store/remove.rs` |
| `crates/shadows-core/src/turns/` | Planner turns, from start to stop | `crates/shadows-core/src/turns/mod.rs` |
| `crates/shadows-core/src/turns/model.rs` | the operation types callers meet | `crates/shadows-core/src/turns/model.rs` |
| `crates/shadows-core/src/turns/store/` | turns' SQLite queries | `crates/shadows-core/src/turns/store/turn.rs` |
| `crates/shadows-core/src/turns/turn.rs` | the recorded ending of a live Planner turn | `crates/shadows-core/src/turns/turn.rs` |
| `crates/shadows-core/src/turns/entries.rs` | turning harness events into durable entries | `crates/shadows-core/src/turns/entries.rs` |
| `crates/shadows-core/src/harness/` | the harnesses, each thread's open session included | `crates/shadows-core/src/harness/mod.rs` |
| `crates/shadows-core/src/harness/model.rs` | the harness shapes callers meet | `crates/shadows-core/src/harness/model.rs` |
| `crates/shadows-core/src/harness/store.rs` | the rows kept per harness | `crates/shadows-core/src/harness/store.rs` |
| `crates/shadows-core/src/harness/sessions.rs` | the live adapter connection each open thread holds | `crates/shadows-core/src/harness/sessions.rs` |
| `crates/shadows-core/src/harness/settings.rs` | setting an open session's options | `crates/shadows-core/src/harness/settings.rs` |
| `crates/shadows-core/src/harness/setup.rs` | what a Planner session opens with | `crates/shadows-core/src/harness/setup.rs` |
| `crates/shadows-core/src/harness/offers.rs` | the latest choices each open session offers | `crates/shadows-core/src/harness/offers.rs` |
| `crates/shadows-core/src/harness/context.rs` | reading a session's context breakdown on demand | `crates/shadows-core/src/harness/context.rs` |
| `crates/shadows-core/src/harness/titles.rs` | handing the titles a harness sends to its thread | `crates/shadows-core/src/harness/titles.rs` |
| `crates/shadows-core/src/plans/` | project plans under the rules of §16 | `crates/shadows-core/src/plans/mod.rs` |
| `crates/shadows-core/src/plans/model.rs` | the plan types callers meet | `crates/shadows-core/src/plans/model.rs` |
| `crates/shadows-core/src/plans/store/` | plans' SQLite queries | `crates/shadows-core/src/plans/store/edit.rs` |
| `crates/shadows-core/src/plans/store/edit.rs` | changing a plan version | `crates/shadows-core/src/plans/store/edit.rs` |
| `crates/shadows-core/src/plans/store/draft.rs` | starting a plan version | `crates/shadows-core/src/plans/store/draft.rs` |
| `crates/shadows-core/src/plans/store/plan.rs` | a plan's state lifecycle with its versions | `crates/shadows-core/src/plans/store/plan.rs` |
| `crates/shadows-core/src/plans/store/read.rs` | reading plan versions | `crates/shadows-core/src/plans/store/read.rs` |
| `crates/shadows-core/src/plans/store/task.rs` | a plan version's task graph rows | `crates/shadows-core/src/plans/store/task.rs` |
| `crates/shadows-core/src/plans/store/view.rs` | showing a plan version in its conversation | `crates/shadows-core/src/plans/store/view.rs` |
| `crates/shadows-core/src/plans/ops.rs` | applying one batch of plan edits | `crates/shadows-core/src/plans/ops.rs` |
| `crates/shadows-core/src/plans/rules.rs` | what makes a plan invalid or unready | `crates/shadows-core/src/plans/rules.rs` |
| `crates/shadows-core/src/plans/scope.rs` | plan calls made under an MCP grant | `crates/shadows-core/src/plans/scope.rs` |
| `crates/shadows-core/src/plans/conversation.rs` | the plan in the conversation | `crates/shadows-core/src/plans/conversation.rs` |
| `crates/shadows-core/src/grants/` | MCP grants, from issue to revocation | `crates/shadows-core/src/grants/mod.rs` |
| `crates/shadows-core/src/grants/model.rs` | the grant types callers meet | `crates/shadows-core/src/grants/model.rs` |
| `crates/shadows-core/src/grants/store.rs` | an MCP grant's rows, from issue to revocation | `crates/shadows-core/src/grants/store.rs` |
| `crates/shadows-core/src/instructions/` | a project's numbered Planner instructions | `crates/shadows-core/src/instructions/mod.rs` |
| `crates/shadows-core/src/design/` | a project's design workspace | `crates/shadows-core/src/design/mod.rs` |
| `crates/shadows-core/src/design/model.rs` | the workspace value shapes | `crates/shadows-core/src/design/model.rs` |
| `crates/shadows-core/src/design/parts.rs` | part service entry points | `crates/shadows-core/src/design/parts.rs` |
| `crates/shadows-core/src/design/outcomes.rs` | outcome service entry points | `crates/shadows-core/src/design/outcomes.rs` |
| `crates/shadows-core/src/design/ops.rs` | normalization of workspace edits | `crates/shadows-core/src/design/ops.rs` |
| `crates/shadows-core/src/design/store/outcomes.rs` | snapshot reads of outcomes | `crates/shadows-core/src/design/store/outcomes.rs` |
| `crates/shadows-core/src/design/store/outcome_edit.rs` | transactional outcome mutations | `crates/shadows-core/src/design/store/outcome_edit.rs` |
| `crates/shadows-core/src/design/store/hierarchy.rs` | transactional containment ordering | `crates/shadows-core/src/design/store/hierarchy.rs` |
| `crates/shadows-core/src/design/store/parts.rs` | snapshot reads of parts | `crates/shadows-core/src/design/store/parts.rs` |
| `crates/shadows-core/src/design/store/part_edit.rs` | transactional part mutations | `crates/shadows-core/src/design/store/part_edit.rs` |
| `crates/shadows-core/src/design/store/` | persistence of the design workspace | `crates/shadows-core/src/design/store/vision.rs` |
| `crates/shadows-core/src/design/store/vision.rs` | atomic workspace edits with immutable replay results | `crates/shadows-core/src/design/store/vision.rs` |
| `crates/shadows-core/src/code/` | the code index of each project's folder | `crates/shadows-core/src/code/mod.rs` |
| `crates/shadows-core/src/code/model.rs` | the code index types callers meet | `crates/shadows-core/src/code/model.rs` |
| `crates/shadows-core/src/code/store/` | the code index's SQLite queries | `crates/shadows-core/src/code/store/mod.rs` |
| `crates/shadows-core/src/code/store/links.rs` | the rows a person's code choices write | `crates/shadows-core/src/code/store/links.rs` |
| `crates/shadows-core/src/code/scan.rs` | walking a project's folder into the index | `crates/shadows-core/src/code/scan.rs` |
| `crates/shadows-core/src/code/scope.rs` | what a code question may read | `crates/shadows-core/src/code/scope.rs` |
| `crates/shadows-core/src/code/watch.rs` | keeping one active project's index current | `crates/shadows-core/src/code/watch.rs` |
| `crates/shadows-core/src/code/active.rs` | which projects are active, in order of use | `crates/shadows-core/src/code/active.rs` |
| `crates/shadows-core/src/code/links.rs` | the commands over links or the active limit | `crates/shadows-core/src/code/links.rs` |
| `crates/shadows-core/src/events/` | what clients watch live | `crates/shadows-core/src/events/mod.rs` |
| `crates/shadows-core/src/events/model.rs` | the event shapes the product records or signals | `crates/shadows-core/src/events/model.rs` |
| `crates/shadows-core/src/events/subscription.rs` | one subscriber's replay-then-live stream | `crates/shadows-core/src/events/subscription.rs` |
| `crates/shadows-agent/src/lib.rs` | the ACP client for a harness adapter | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp.rs` | the ACP client connection to one adapter process | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/events.rs` | what a harness connection reports | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/policy.rs` | the modes Shadows allows per harness | `crates/shadows-agent/src/policy.rs` |
| `crates/shadows-agent/src/choices.rs` | reading the harness's offered choices | `crates/shadows-agent/src/choices.rs` |
| `crates/shadows-agent/src/breakdown.rs` | reading Claude's `/context` answer | `crates/shadows-agent/src/breakdown.rs` |
| `crates/shadows-agent/src/claude.rs` | the launch spec of the pinned Claude ACP adapter | `crates/shadows-agent/src/claude.rs` |
| `crates/shadows-index/src/lib.rs` | one file's text turned into its tags | `crates/shadows-index/src/extract.rs` |
| `crates/shadows-index/src/languages.rs` | the table of languages Shadows indexes | `crates/shadows-index/src/languages.rs` |
| `crates/shadows-index/src/extract.rs` | one file's text in, its tags out | `crates/shadows-index/src/extract.rs` |
| `crates/shadows-process/src/lib.rs` | OS process ownership with whole-tree containment | `crates/shadows-process/src/lib.rs` |
| `crates/shadows-process/src/bin/` | test apparatus that no product code links | `crates/shadows-process/src/bin/tree_probe.rs` |
| `crates/fake-acp/src/main.rs` | test apparatus that no product code links | `crates/fake-acp/src/main.rs` |

The Web client in `web/` is a separate program outside this workspace and this
map; [`web/README.md`](../../web/README.md) describes it.

A module absent from this table is a module that does not exist yet. The
planned services are named in [`CLAUDE.md`](../../CLAUDE.md); this table is not
a second copy of that list, and the tree is what decides which of them are
real.

**This table is a working summary, not authority.** `CLAUDE.md` owns the
crates' single-ownership rules and the file-size rules; the specs indexed by
[`specs/README.md`](../superpowers/specs/README.md) own the design. Where this
file disagrees with either, they are right and this file is the defect.

## What is deliberately not here

- **Line numbers.** Wrong at the first line inserted above them, with nothing
  failing when they lie. One field that rots silently costs the reader their
  trust in every field beside it.
- **Built / not-built status.** The tree answers it and
  [`docs/status.md`](../status.md) narrates progress. A third copy would record
  one fact in three places.
- **A list of declarations or signatures.** The LSP and `where_is` answer it
  exactly. A generated inventory did this until 2026-10-01, and cost every
  change a regeneration step.
- **Explanations of declarations.** Those live as doc comments on the
  declarations themselves.
