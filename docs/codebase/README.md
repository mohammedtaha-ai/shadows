# Code map

**Read this before writing code in `shadows`.**

Core work starts at [the service table on `AppCore`](../../crates/shadows-core/src/app.rs),
then the owning service's contract and implementation. That table is the Core
navigation authority, including internal owners without an accessor. The rows
below retain checked module coverage; use them to locate modules within the
owner selected through `AppCore`, or to navigate crates outside Core. The target
and its staged migration are owned by [section 14.12](../superpowers/specs/2026-09-26-application-core-design.md#1412-service-ownership-and-staged-crate-extraction);
planned crates are not current owners until their implementation moves.

Two kinds of file, split by what can be checked:

| File | Kind | Answers |
|---|---|---|
| `crates/shadows-http/src/aylul_profiles.rs` | AYLUL configuration HTTP translation | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-http/src/aylul_profiles_tests.rs` | AYLUL configuration route acceptance tests | `crates/shadows-http/src/aylul_profiles.rs` |
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

`crates/shadows/tests/architecture/` owns the shrinking source-ownership ratchet.
Its [README](../../crates/shadows/tests/architecture/README.md) describes the
checks, explicit review cases, fixture mutations and baseline maintenance.
Run it with `cargo test -p shadows --test architecture`.

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
  write) or `called_by_other_services`. These are existing migration exceptions:
  new cross-service operations use the owner's service API, and each migration
  removes its old exception. Private stores do not by themselves prevent SQL
  against another owner's tables on a shared connection.

## What each crate and module owns

One job per row, stated without "and" — a conjunction here means the module
has quietly taken a second responsibility. A crate's row names its entry point
(`lib.rs` or `main.rs`); the rows after it are that crate's modules. The
reference file is the one to read before adding to that module: it is the
pattern to follow, not merely an example.

| Module | Its one job | Reference file |
|---|---|---|
| `crates/shadows-core/src/harness/live.rs` | live adapter opening | `crates/shadows-core/src/harness/contract.yaml` |
| `crates/shadows-core/src/providers/` | AYLUL provider configuration | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/providers/model.rs` | public provider metadata | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/providers/validation.rs` | provider input rules | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/providers/presets.rs` | compiled provider presets | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/providers/probe.rs` | bounded provider listing requests | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/providers/listing.rs` | provider model response parsing | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/providers/slug.rs` | generated configuration slug shape | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/providers/error.rs` | provider refusals | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/providers/store.rs` | provider configuration SQL | `crates/shadows-core/src/providers/contract.yaml` |
| `crates/shadows-core/src/agents/` | permanent AYLUL agent identities | `crates/shadows-core/src/agents/contract.yaml` |
| `crates/shadows-core/src/agents/model.rs` | public agent metadata | `crates/shadows-core/src/agents/contract.yaml` |
| `crates/shadows-core/src/agents/home.rs` | agent home seeding | `crates/shadows-core/src/agents/contract.yaml` |
| `crates/shadows-core/src/agents/error.rs` | agent refusals | `crates/shadows-core/src/agents/contract.yaml` |
| `crates/shadows-core/src/agents/store.rs` | agent identity SQL | `crates/shadows-core/src/agents/contract.yaml` |
| `crates/shadows-core/src/extensions/` | the shared extension library | `crates/shadows-core/src/extensions/contract.yaml` |
| `crates/shadows-core/src/extensions/model.rs` | public extension metadata | `crates/shadows-core/src/extensions/contract.yaml` |
| `crates/shadows-core/src/extensions/files.rs` | bounded library file copies | `crates/shadows-core/src/extensions/contract.yaml` |
| `crates/shadows-core/src/extensions/error.rs` | extension library refusals | `crates/shadows-core/src/extensions/contract.yaml` |
| `crates/shadows-core/src/extensions/store.rs` | extension configuration SQL | `crates/shadows-core/src/extensions/contract.yaml` |
| `crates/shadows-core/src/usage/` | measured AYLUL usage | `crates/shadows-core/src/usage/contract.yaml` |
| `crates/shadows-core/src/usage/model.rs` | usage report shapes | `crates/shadows-core/src/usage/contract.yaml` |
| `crates/shadows-core/src/usage/report.rs` | historical usage aggregation | `crates/shadows-core/src/usage/contract.yaml` |
| `crates/shadows-core/src/usage/store.rs` | usage row SQL | `crates/shadows-core/src/usage/contract.yaml` |
| `crates/shadows-core/src/person/` | the local person record | `crates/shadows-core/src/person/contract.yaml` |
| `crates/shadows-core/src/person/model.rs` | public person record | `crates/shadows-core/src/person/contract.yaml` |
| `crates/shadows-core/src/person/store.rs` | person record SQL | `crates/shadows-core/src/person/contract.yaml` |
| `crates/shadows-core/src/harness/extension_home.rs` | per-agent extension projection | `crates/shadows-core/src/harness/contract.yaml` |
| `crates/shadows-http/src/aylul_extensions.rs` | extension HTTP translation | `crates/shadows-core/src/extensions/contract.yaml` |
| `crates/shadows-http/src/aylul_extensions_tests.rs` | extension route acceptance | `crates/shadows-http/src/aylul_extensions.rs` |
| `crates/shadows-core/src/turns/store/usage.rs` | atomic turn usage insertion | `crates/shadows-core/src/turns/contract.yaml` |
| `crates/shadows-core/src/harness/profile_home.rs` | serialized agent home settings writes | `docs/aylul/specs/2026-10-19-aylul-claude-code-brain-design.md` |
| `crates/shadows-http/src/aylul_env.rs` | AYLUL environment HTTP translation | `crates/shadows-core/src/agents/contract.yaml` |
| `crates/shadows-http/src/aylul_env_tests.rs` | AYLUL environment route acceptance tests | `crates/shadows-http/src/aylul_env.rs` |
| `crates/shadows-agent/src/profile_tests.rs` | isolated profile environment tests | `crates/shadows-agent/src/profile.rs` |
| `crates/shadows/src/master_key.contract.yaml` | master key storage obligations | `crates/shadows/src/master_key.rs` |
| `crates/shadows/src/master_key.rs` | Windows master key storage | `crates/shadows/src/master_key.contract.yaml` |
| `crates/shadows/src/master_key_tests.rs` | master key lifecycle tests with in-memory credentials | `crates/shadows/src/master_key.contract.yaml` |
| `crates/shadows/src/aylul_tests.rs` | AYLUL startup configuration tests | `crates/shadows/src/aylul.rs` |
| `crates/shadows-core/src/secrets/` | encrypted profile environment operations | `crates/shadows-core/src/secrets/contract.yaml` |
| `crates/shadows-core/src/secrets/model.rs` | secret environment metadata | `crates/shadows-core/src/secrets/model.rs` |
| `crates/shadows-core/src/secrets/store.rs` | encrypted environment SQL | `crates/shadows-core/src/secrets/store.rs` |
| `crates/shadows-core/src/secrets/crypto.rs` | authenticated environment encryption | `crates/shadows-core/src/secrets/crypto.rs` |
| `crates/shadows-core/src/secrets/crypto_tests.rs` | envelope integrity tests | `crates/shadows-core/src/secrets/crypto_tests.rs` |
| `crates/shadows-core/src/grants/approval.rs` | transaction-local approval provenance | `docs/superpowers/specs/2026-10-08-approval-authority-design.md` |
| `crates/shadows-core/src/db/write_observer.rs` | test-only committed-write observation | `crates/shadows/tests/openapi/read_only.rs` |
| `crates/shadows-core/src/design/map_proposals.rs` | map proposal service entry points | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/map_proposals.rs` | recording map proposals | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/map_proposal_shape.rs` | assembling proposed map shapes | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/map_proposal_read.rs` | reading map proposal state | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/map_approval.rs` | committing approved map proposals | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/map_rules.rs` | pure map validation | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/map_versions.rs` | durable map snapshots | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/roadmap_gate.rs` | refusing work that needs an approved roadmap | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/roadmap_seed.rs` | test-only seeding of the plans stage | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/roadmap_proposals.rs` | roadmap proposal service entry points | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/roadmap_proposals.rs` | recording roadmap proposals | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/roadmap_shape.rs` | checking proposed roadmap shapes | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/roadmap_proposal_read.rs` | reading roadmap proposal state | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows-core/src/design/store/roadmap_approval.rs` | committing approved roadmap proposals | `crates/shadows-core/src/design/contract.yaml` |
| `crates/shadows/src/main.rs` | the one binary Shadows ships | `crates/shadows/src/cli/mod.rs` |
| `crates/shadows/src/cli/` | daemon startup | `crates/shadows/src/cli/args.rs` |
| `crates/shadows/src/desktop_runtime.rs` | desktop launch identity | `docs/aylul/specs/2026-10-20-aylul-desktop-web-companion-design.md` |
| `crates/shadows/src/runtime_record.rs` | private runtime record publication | `crates/shadows/src/runtime_record.rs` |
| `crates/shadows/src/config.rs` | startup configuration resolved once | `crates/shadows/src/config.rs` |
| `crates/shadows/src/daemon/` | product service composition | `docs/integration/2026-10-19-product-daemon-hermes-design.md` |
| `crates/shadows/src/daemon/bridge.rs` | bounded Node request ownership | `crates/shadows/src/daemon/bridge.rs` |
| `crates/shadows/src/daemon/http.rs` | native assistant request translation | `crates/shadows/src/daemon/http.rs` |
| `crates/shadows/src/aylul.rs` | AYLUL startup configuration | `crates/shadows/src/aylul.rs` |
| `crates/shadows/src/daemon/descriptor.rs` | private connection descriptor ownership | `crates/shadows/src/daemon/descriptor.rs` |
| `crates/shadows/src/tracing.rs` | tracing subscriber setup | `crates/shadows/src/tracing.rs` |
| `crates/shadows-http/src/runtime.rs` | runtime readiness HTTP projection | `crates/shadows/src/desktop_runtime.rs` |
| `crates/shadows-http/src/lib.rs` | the HTTP/SSE surface every client talks to | `crates/shadows-http/src/project.rs` |
| `crates/shadows-http/src/project.rs` | the routes over projects, their threads included | `crates/shadows-http/src/project.rs` |
| `crates/shadows-http/src/conversation.rs` | the routes over a thread's conversation | `crates/shadows-http/src/conversation.rs` |
| `crates/shadows-http/src/queue.rs` | the routes of a conversation's waiting messages | `crates/shadows-http/src/queue.rs` |
| `crates/shadows-http/src/harness.rs` | the routes over harnesses, their sessions included | `crates/shadows-http/src/harness.rs` |
| `crates/shadows-http/src/thread.rs` | the routes that change a planning thread itself | `crates/shadows-http/src/thread.rs` |
| `crates/shadows-http/src/workflow.rs` | the routes over plan versions | `crates/shadows-http/src/workflow.rs` |
| `crates/shadows-http/src/grants.rs` | the routes over external agents' MCP grants | `crates/shadows-http/src/grants.rs` |
| `crates/shadows-http/src/instructions.rs` | the routes over a project's Planner instructions | `crates/shadows-http/src/instructions.rs` |
| `crates/shadows-http/src/design.rs` | the routes over a project's design workspace | `crates/shadows-http/src/design.rs` |
| `crates/shadows-http/src/roadmap.rs` | the routes over roadmap proposals | `crates/shadows-http/src/roadmap.rs` |
| `crates/shadows-http/src/agreements.rs` | the routes over shared API agreements | `crates/shadows-http/src/agreements.rs` |
| `crates/shadows-http/src/code.rs` | the routes over the code index | `crates/shadows-http/src/code.rs` |
| `crates/shadows-http/src/sse.rs` | the replay-then-live stream's SSE framing | `crates/shadows-http/src/sse.rs` |
| `crates/shadows-http/src/fs.rs` | choosing a project directory on this machine | `crates/shadows-http/src/fs.rs` |
| `crates/shadows-http/src/openapi.rs` | the OpenAPI document describing this API | `crates/shadows-http/src/openapi.rs` |
| `crates/shadows-http/src/failure/` | the transport mapping of a failure | `crates/shadows-http/src/failure/mod.rs` |
| `crates/shadows-http/src/failure/storage.rs` | how a storage failure becomes a status | `crates/shadows-http/src/failure/storage.rs` |
| `crates/shadows-http/src/failure/other.rs` | how a start, core or directory failure becomes a status | `crates/shadows-http/src/failure/other.rs` |
| `crates/shadows-http/src/guard.rs` | refusing requests pages were made to send | `crates/shadows-http/src/guard.rs` |
| `crates/shadows-mcp/src/lib.rs` | Shadows' MCP server | `crates/shadows-mcp/src/lib.rs` |
| `crates/shadows-mcp/src/auth.rs` | refusing a `/mcp` request that holds no live grant | `crates/shadows-mcp/src/auth.rs` |
| `crates/shadows-mcp/src/server.rs` | the tools a grant's kind may see | `crates/shadows-mcp/src/server.rs` |
| `crates/shadows-mcp/src/tools.rs` | each MCP tool's one service call | `crates/shadows-mcp/src/tools.rs` |
| `crates/shadows-mcp/src/agreement_tools.rs` | agreement tool translation | `crates/shadows-mcp/src/agreement_tools.rs` |
| `crates/shadows-mcp/src/workspace_tools.rs` | workspace read tool translation | `crates/shadows-mcp/src/workspace_tools.rs` |
| `crates/shadows-mcp/src/refusal.rs` | what an MCP tool call answers | `crates/shadows-mcp/src/refusal.rs` |
| `crates/shadows-core/src/lib.rs` | the application behind `AppCore` | `crates/shadows-core/src/app.rs` |
| `crates/shadows-core/src/app.rs` | the application's composition root | `crates/shadows-core/src/app.rs` |
| `crates/shadows-core/src/error.rs` | the stable failure taxonomy clients match on | `crates/shadows-core/src/error.rs` |
| `crates/shadows-core/src/db/command.rs` | external-command identity for idempotency | `crates/shadows-core/src/db/contract.yaml` |
| `crates/shadows-core/src/db/derive.rs` | command ids Shadows derives when a caller names none | `crates/shadows-core/src/db/contract.yaml` |
| `crates/shadows-core/src/db/journal.rs` | appending to the durable journal | `crates/shadows-core/src/db/contract.yaml` |
| `crates/shadows-core/src/db/id.rs` | the UUID id newtype pattern | `crates/shadows-core/src/db/contract.yaml` |
| `crates/shadows-core/src/db/embedded_migrations.rs` | Historical Windows profiles migration source selection | `docs/superpowers/specs/2026-09-21-persistence-api-and-migrations-design.md` |
| `crates/shadows-core/src/db/` | the database no service owns | `crates/shadows-core/src/db/mod.rs` |
| `crates/shadows-core/src/runtime/` | the runtime instance's lifecycle | `crates/shadows-core/src/runtime/mod.rs` |
| `crates/shadows-core/src/testing/` | the test apparatus every crate's tests share | `crates/shadows-core/src/testing/mod.rs` |
| `crates/shadows-core/src/projects/` | the projects, the folders a person picks them from included | `crates/shadows-core/src/projects/mod.rs` |
| `crates/shadows-core/src/projects/model.rs` | the project types callers meet | `crates/shadows-core/src/projects/model.rs` |
| `crates/shadows-core/src/projects/store.rs` | a project's rows | `crates/shadows-core/src/projects/store.rs` |
| `crates/shadows-core/src/projects/harness_policy.rs` | which harness a thread or a project's modes may name | `crates/shadows-core/src/projects/harness_policy.rs` |
| `crates/shadows-core/src/projects/browse.rs` | showing a person the directories they can choose from | `crates/shadows-core/src/projects/browse.rs` |
| `crates/shadows-core/src/threads/` | the planning threads, what they recorded included | `crates/shadows-core/src/threads/mod.rs` |
| `crates/shadows-core/src/threads/error.rs` | stable thread refusals | `crates/shadows-core/src/threads/contract.yaml` |
| `crates/shadows-core/src/threads/testing.rs` | test-only thread command contexts | `crates/shadows-core/src/threads/contract.yaml` |
| `crates/shadows-core/src/threads/store/records.rs` | snapshot-local thread facts | `crates/shadows-core/src/threads/contract.yaml` |
| `crates/shadows-core/src/threads/model.rs` | the planning thread types callers meet | `crates/shadows-core/src/threads/model.rs` |
| `crates/shadows-core/src/threads/title.rs` | the text a title taken from a conversation may read | `crates/shadows-core/src/threads/title.rs` |
| `crates/shadows-core/src/threads/store/` | private ThreadsStore persistence | `crates/shadows-core/src/threads/store/thread.rs` |
| `crates/shadows-core/src/threads/store/remove.rs` | atomic removal of a planning thread | `crates/shadows-core/src/threads/store/remove.rs` |
| `crates/shadows-core/src/operations/` | durable operation lifecycle | `crates/shadows-core/src/operations/contract.yaml` |
| `crates/shadows-core/src/operations/model.rs` | durable operation values | `crates/shadows-core/src/operations/contract.yaml` |
| `crates/shadows-core/src/operations/store/` | private operation persistence | `crates/shadows-core/src/operations/contract.yaml` |
| `crates/shadows-core/src/operations/fork.rs` | transactional fork eligibility | `crates/shadows-core/src/operations/contract.yaml` |
| `crates/shadows-core/src/operations/testing.rs` | test-only runtime stop entry | `crates/shadows-core/src/operations/contract.yaml` |
| `crates/shadows-core/src/turns/testing.rs` | test-only lifecycle composition | `crates/shadows-core/src/turns/contract.yaml` |
| `crates/shadows-core/src/turns/` | Planner turns, from start to stop | `crates/shadows-core/src/turns/mod.rs` |
| `crates/shadows-core/src/turns/model.rs` | the operation types callers meet | `crates/shadows-core/src/turns/model.rs` |
| `crates/shadows-core/src/turns/store/` | turn command persistence | `crates/shadows-core/src/turns/store/turn.rs` |
| `crates/shadows-core/src/turns/store/queue.rs` | the queue's SQLite queries | `crates/shadows-core/src/turns/store/queue.rs` |
| `crates/shadows-core/src/turns/store/steer.rs` | recording a steered message | `crates/shadows-core/src/turns/store/steer.rs` |
| `crates/shadows-core/src/turns/queue.rs` | the waiting messages of a conversation | `crates/shadows-core/src/turns/queue.rs` |
| `crates/shadows-core/src/turns/record.rs` | the checks that commit a turn's start | `crates/shadows-core/src/turns/record.rs` |
| `crates/shadows-core/src/turns/steer.rs` | a Send now inside the turn's watcher | `crates/shadows-core/src/turns/steer.rs` |
| `crates/shadows-core/src/turns/turn.rs` | the recorded ending of a live Planner turn | `crates/shadows-core/src/turns/turn.rs` |
| `crates/shadows-core/src/turns/entries.rs` | turning harness events into durable entries | `crates/shadows-core/src/turns/entries.rs` |
| `crates/shadows-core/src/turns/subagents.rs` | gathering a subagent's tool calls into its card (§22) | `crates/shadows-core/src/turns/subagents.rs` |
| `crates/shadows-core/src/harness/` | the harnesses, each thread's open session included | `crates/shadows-core/src/harness/mod.rs` |
| `crates/shadows-core/src/harness/model.rs` | the harness shapes callers meet | `crates/shadows-core/src/harness/model.rs` |
| `crates/shadows-core/src/harness/store/` | harness persistence | `crates/shadows-core/src/harness/contract.yaml` |
| `crates/shadows-core/src/harness/assistant.rs` | choosing what a thread's adapter opens with, by harness kind | `crates/shadows-core/src/harness/assistant.rs` |
| `crates/shadows-core/src/harness/sessions.rs` | the live adapter connection each open thread holds | `crates/shadows-core/src/harness/sessions.rs` |
| `crates/shadows-core/src/harness/settings.rs` | setting an open session's options | `crates/shadows-core/src/harness/settings.rs` |
| `crates/shadows-core/src/harness/setup.rs` | what a Planner session opens with | `crates/shadows-core/src/harness/setup.rs` |
| `crates/shadows-core/src/harness/standards.rs` | Planner standards rendering | `crates/shadows-core/src/harness/standards.rs` |
| `crates/shadows-core/src/harness/offers.rs` | the latest choices each open session offers | `crates/shadows-core/src/harness/offers.rs` |
| `crates/shadows-core/src/harness/commands.rs` | the latest `/` list each open session sent | `crates/shadows-core/src/harness/commands.rs` |
| `crates/shadows-core/src/harness/context.rs` | reading a session's context breakdown on demand | `crates/shadows-core/src/harness/context.rs` |
| `crates/shadows-core/src/harness/titles.rs` | handing the titles a harness sends to its thread | `crates/shadows-core/src/harness/titles.rs` |
| `crates/shadows-core/src/plans/` | project plans under the rules of §16 | `crates/shadows-core/src/plans/mod.rs` |
| `crates/shadows-core/src/plans/model.rs` | the plan types callers meet | `crates/shadows-core/src/plans/model.rs` |
| `crates/shadows-core/src/plans/dependencies.rs` | task dependency value shapes | `crates/shadows-core/src/plans/dependencies.rs` |
| `crates/shadows-core/src/plans/bindings.rs` | exact agreement binding value shapes | `crates/shadows-core/src/plans/bindings.rs` |
| `crates/shadows-core/src/plans/binding_edit.rs` | person adoption entry point | `crates/shadows-core/src/plans/binding_edit.rs` |
| `crates/shadows-core/src/plans/map.rs` | project plan map value shapes | `crates/shadows-core/src/plans/map.rs` |
| `crates/shadows-core/src/plans/dependency_graph.rs` | start/complete dependency ordering | `crates/shadows-core/src/plans/dependency_graph.rs` |
| `crates/shadows-core/src/plans/store/` | plans' SQLite queries | `crates/shadows-core/src/plans/store/edit.rs` |
| `crates/shadows-core/src/plans/store/edit.rs` | changing a plan version | `crates/shadows-core/src/plans/store/edit.rs` |
| `crates/shadows-core/src/plans/store/draft.rs` | starting a plan version | `crates/shadows-core/src/plans/store/draft.rs` |
| `crates/shadows-core/src/plans/store/plan.rs` | a plan's state lifecycle with its versions | `crates/shadows-core/src/plans/store/plan.rs` |
| `crates/shadows-core/src/plans/store/read.rs` | reading plan versions | `crates/shadows-core/src/plans/store/read.rs` |
| `crates/shadows-core/src/plans/store/task.rs` | a plan version's task graph rows | `crates/shadows-core/src/plans/store/task.rs` |
| `crates/shadows-core/src/plans/store/dependencies.rs` | persisted cross-plan links | `crates/shadows-core/src/plans/store/dependencies.rs` |
| `crates/shadows-core/src/plans/store/bindings.rs` | persisted agreement adoption | `crates/shadows-core/src/plans/store/bindings.rs` |
| `crates/shadows-core/src/plans/store/binding_participants.rs` | captured adoption participant reads | `crates/shadows-core/src/plans/store/binding_participants.rs` |
| `crates/shadows-core/src/plans/store/graph.rs` | reachable latest-version dependency checks | `crates/shadows-core/src/plans/store/graph.rs` |
| `crates/shadows-core/src/plans/store/linked.rs` | captured dependency preview reads | `crates/shadows-core/src/plans/store/linked.rs` |
| `crates/shadows-core/src/plans/store/map.rs` | captured latest-plan map reads | `crates/shadows-core/src/plans/store/map.rs` |
| `crates/shadows-core/src/plans/store/notifications.rs` | transactional dependency invalidations | `crates/shadows-core/src/plans/store/notifications.rs` |
| `crates/shadows-core/src/plans/store/view.rs` | showing a plan version in its conversation | `crates/shadows-core/src/plans/store/view.rs` |
| `crates/shadows-core/src/plans/ops.rs` | applying one batch of plan edits | `crates/shadows-core/src/plans/ops.rs` |
| `crates/shadows-core/src/plans/rules.rs` | what makes a plan invalid or unready | `crates/shadows-core/src/plans/rules.rs` |
| `crates/shadows-core/src/plans/scope.rs` | plan calls made under an MCP grant | `crates/shadows-core/src/plans/scope.rs` |
| `crates/shadows-core/src/plans/conversation.rs` | the plan in the conversation | `crates/shadows-core/src/plans/conversation.rs` |
| `crates/shadows-core/src/grants/` | grant authorization | `crates/shadows-core/src/grants/mod.rs` |
| `crates/shadows-core/src/grants/model.rs` | the grant types callers meet | `crates/shadows-core/src/grants/model.rs` |
| `crates/shadows-core/src/grants/store.rs` | an MCP grant's rows, from issue to revocation | `crates/shadows-core/src/grants/store.rs` |
| `crates/shadows-core/src/instructions/` | a project's numbered Planner instructions | `crates/shadows-core/src/instructions/mod.rs` |
| `crates/shadows-core/src/design/` | a project's design workspace | `crates/shadows-core/src/design/mod.rs` |
| `crates/shadows-core/src/design/structure.rs` | rendering an approved map as files | `crates/shadows-core/src/design/structure.rs` |
| `crates/shadows-core/src/design/structure_create.rs` | coordinating a person's structure write | `crates/shadows-core/src/design/structure_create.rs` |
| `crates/shadows-core/src/design/structure_write.rs` | publishing recorded files safely | `crates/shadows-core/src/design/structure_write.rs` |
| `crates/shadows-core/src/design/store/structure.rs` | structure write records | `crates/shadows-core/src/design/store/structure.rs` |
| `crates/shadows-core/src/design/vision_items.rs` | applying vision item changes | `crates/shadows-core/src/design/vision_items.rs` |
| `crates/shadows-core/src/design/store/vision_versions.rs` | stored vision versions | `crates/shadows-core/src/design/store/vision_versions.rs` |
| `crates/shadows-core/src/design/proposals.rs` | vision proposal entry points | `crates/shadows-core/src/design/proposals.rs` |
| `crates/shadows-core/src/design/store/proposals.rs` | recording vision proposals | `crates/shadows-core/src/design/store/proposals.rs` |
| `crates/shadows-core/src/design/store/proposal_read.rs` | reading proposal state | `crates/shadows-core/src/design/store/proposal_read.rs` |
| `crates/shadows-core/src/design/store/proposal_approval.rs` | approving vision proposals | `crates/shadows-core/src/design/store/proposal_approval.rs` |
| `crates/shadows-core/src/design/model.rs` | the workspace value shapes | `crates/shadows-core/src/design/model.rs` |
| `crates/shadows-core/src/design/agreement.rs` | shared agreement value shapes | `crates/shadows-core/src/design/agreement.rs` |
| `crates/shadows-core/src/design/agreement_validation.rs` | offline agreement content validation | `crates/shadows-core/src/design/agreement_validation.rs` |
| `crates/shadows-core/src/design/agreements.rs` | shared agreement service entry points | `crates/shadows-core/src/design/agreements.rs` |
| `crates/shadows-core/src/design/agreement_scope.rs` | grant attribution for agreement calls | `crates/shadows-core/src/design/agreement_scope.rs` |
| `crates/shadows-core/src/design/agreement_review.rs` | agreement review value shapes | `crates/shadows-core/src/design/agreement_review.rs` |
| `crates/shadows-core/src/design/store/agreement_binding.rs` | exact-version adoption checks | `crates/shadows-core/src/design/store/agreement_binding.rs` |
| `crates/shadows-core/src/design/store/agreement_review.rs` | captured impact review | `crates/shadows-core/src/design/store/agreement_review.rs` |
| `crates/shadows-core/src/design/store/agreements.rs` | captured agreement version reads | `crates/shadows-core/src/design/store/agreements.rs` |
| `crates/shadows-core/src/design/store/agreement_write.rs` | transactional agreement Draft mutations | `crates/shadows-core/src/design/store/agreement_write.rs` |
| `crates/shadows-core/src/design/parts.rs` | part service entry points | `crates/shadows-core/src/design/parts.rs` |
| `crates/shadows-core/src/design/workspace.rs` | a grant's read-only view of its project workspace | `crates/shadows-core/src/design/workspace.rs` |
| `crates/shadows-core/src/design/outcomes.rs` | outcome service entry points | `crates/shadows-core/src/design/outcomes.rs` |
| `crates/shadows-core/src/design/ops.rs` | normalization of workspace edits | `crates/shadows-core/src/design/ops.rs` |
| `crates/shadows-core/src/standards/` | project standards versions | `crates/shadows-core/src/standards/contract.yaml` |
| `crates/shadows-core/src/standards/model.rs` | public standards values | `crates/shadows-core/src/standards/contract.yaml` |
| `crates/shadows-core/src/standards/base.rs` | bundled base parsing | `crates/shadows-core/src/standards/contract.yaml` |
| `crates/shadows-core/src/standards/validation.rs` | project additions validation | `crates/shadows-core/src/standards/contract.yaml` |
| `crates/shadows-core/src/design/stage.rs` | a project's stage from its workspace | `crates/shadows-core/src/design/stage.rs` |
| `crates/shadows-core/src/standards/store.rs` | immutable additions persistence | `crates/shadows-core/src/standards/contract.yaml` |
| `crates/shadows-core/src/turns/store/standards.rs` | invocation standards selection | `crates/shadows-core/src/turns/contract.yaml` |
| `crates/shadows-core/src/design/store/stage.rs` | reading a project's stage inputs | `crates/shadows-core/src/design/store/stage.rs` |
| `crates/shadows-core/src/design/store/outcomes.rs` | snapshot reads of outcomes | `crates/shadows-core/src/design/store/outcomes.rs` |
| `crates/shadows-core/src/design/store/outcome_edit.rs` | transactional outcome mutations | `crates/shadows-core/src/design/store/outcome_edit.rs` |
| `crates/shadows-core/src/design/store/hierarchy.rs` | transactional containment ordering | `crates/shadows-core/src/design/store/hierarchy.rs` |
| `crates/shadows-core/src/design/store/parts.rs` | snapshot reads of parts | `crates/shadows-core/src/design/store/parts.rs` |
| `crates/shadows-core/src/design/store/part_edit.rs` | transactional part mutations | `crates/shadows-core/src/design/store/part_edit.rs` |
| `crates/shadows-core/src/design/store/` | persistence of the design workspace | `crates/shadows-core/src/design/store/vision.rs` |
| `crates/shadows-core/src/design/store/vision.rs` | atomic workspace edits with immutable replay results | `crates/shadows-core/src/design/store/vision.rs` |
| `crates/shadows-core/src/code/` | the code index of each project's folder | `crates/shadows-core/src/code/mod.rs` |
| `crates/shadows-core/src/code/model.rs` | the code index types callers meet | `crates/shadows-core/src/code/model.rs` |
| `crates/shadows-core/src/code/store/` | private CodeStore index persistence | `crates/shadows-core/src/code/store/mod.rs` |
| `crates/shadows-core/src/code/store/links.rs` | transactional code link persistence | `crates/shadows-core/src/code/store/links.rs` |
| `crates/shadows-core/src/code/store/reach.rs` | live one-way project reach in a snapshot | `crates/shadows-core/src/code/store/reach.rs` |
| `crates/shadows-core/src/code/scan.rs` | walking a project's folder into the index | `crates/shadows-core/src/code/scan.rs` |
| `crates/shadows-core/src/code/scope.rs` | what a code question may read | `crates/shadows-core/src/code/scope.rs` |
| `crates/shadows-core/src/code/watch.rs` | keeping one active project's index current | `crates/shadows-core/src/code/watch.rs` |
| `crates/shadows-core/src/code/active.rs` | which projects are active, in order of use | `crates/shadows-core/src/code/active.rs` |
| `crates/shadows-core/src/code/links.rs` | the commands over links or the active limit | `crates/shadows-core/src/code/links.rs` |
| `crates/shadows-core/src/events/` | what clients watch live | `crates/shadows-core/src/events/mod.rs` |
| `crates/shadows-core/src/events/model.rs` | the event shapes the product records or signals | `crates/shadows-core/src/events/model.rs` |
| `crates/shadows-core/src/events/subscription.rs` | one subscriber's replay-then-live stream | `crates/shadows-core/src/events/subscription.rs` |
| `crates/shadows-agent/src/lib.rs` | the ACP client for a harness adapter | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/usage.rs` | session cost accounting | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/acp_tests.rs` | ACP boundary tests | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp.rs` | the ACP client connection to one adapter process | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/acp_tests.rs` | ACP boundary tests | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp.rs` | the ACP client connection to one adapter process | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/events.rs` | what a harness connection reports | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/policy.rs` | the modes Shadows allows per harness | `crates/shadows-agent/src/policy.rs` |
| `crates/shadows-agent/src/profile.rs` | the child environment of an isolated Claude Code profile | `crates/shadows-agent/src/profile.rs` |
| `crates/shadows-agent/src/choices.rs` | reading the harness's offered choices | `crates/shadows-agent/src/choices.rs` |
| `crates/shadows-agent/src/breakdown.rs` | reading Claude's `/context` answer | `crates/shadows-agent/src/breakdown.rs` |
| `crates/shadows-agent/src/claude.rs` | the launch spec of the pinned Claude ACP adapter | `crates/shadows-agent/src/claude.rs` |
| `crates/shadows-index/src/lib.rs` | one file's text turned into its tags | `crates/shadows-index/src/extract.rs` |
| `crates/shadows-index/src/languages.rs` | the table of languages Shadows indexes | `crates/shadows-index/src/languages.rs` |
| `crates/shadows-index/src/extract.rs` | one file's text in, its tags out | `crates/shadows-index/src/extract.rs` |
| `crates/shadows-process/src/lib.rs` | OS process ownership with whole-tree containment | `crates/shadows-process/src/lib.rs` |
| `crates/shadows-process/src/persistent.rs` | independent process lifetime | `crates/shadows-process/src/lib.rs` |
| `crates/shadows-process/src/bin/` | test apparatus that no product code links | `crates/shadows-process/src/bin/tree_probe.rs` |
| `crates/fake-acp/src/main.rs` | test apparatus that no product code links | `crates/fake-acp/src/main.rs` |
| `crates/fake-acp/src/session.rs` | a fake session's state, offered to the client | `crates/fake-acp/src/session.rs` |
| `crates/fake-acp/src/mcp.rs` | calling the session's MCP server | `crates/fake-acp/src/mcp.rs` |
| `crates/fake-acp/src/steer.rs` | steering a prompt that waits for it | `crates/fake-acp/src/steer.rs` |
| `crates/fake-acp/src/prompts.rs` | the scripted answers to a prompt | `crates/fake-acp/src/prompts.rs` |

The Web client in `web/` is a separate program outside this workspace and this
map; [`docs/frontend/codebase/README.md`](../frontend/codebase/README.md) is its
code map, and [`web/README.md`](../../web/README.md) says how to run it.

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
  each side's `stages.md` lists what is left. A third copy would record
  one fact in three places.
- **A list of declarations or signatures.** The LSP and `where_is` answer it
  exactly. A generated inventory did this until 2026-10-01, and cost every
  change a regeneration step.
- **Explanations of declarations.** Those live as doc comments on the
  declarations themselves.

| `crates/shadows-core/src/grants/writer.rs` | authenticated writer values | `crates/shadows-core/src/grants/contract.yaml` |
| `crates/shadows-core/src/grants/approver.rs` | approval provenance values | `crates/shadows-core/src/grants/contract.yaml` |
| `crates/shadows-core/src/grants/error.rs` | grant refusal mapping | `crates/shadows-core/src/grants/contract.yaml` |
| `crates/shadows-core/src/grants/tools.rs` | grant tool policy | `crates/shadows-core/src/grants/contract.yaml` |
| `crates/shadows-core/src/grants/testing.rs` | test-only grant entry points | `crates/shadows-core/src/grants/contract.yaml` |

| `crates/shadows-core/src/harness/access.rs` | live session owner operations | `crates/shadows-core/src/harness/contract.yaml` |
| `crates/shadows-core/src/harness/store/invocation.rs` | invocation transaction entry points | `crates/shadows-core/src/harness/contract.yaml` |
| `crates/shadows-core/src/turns/stage.rs` | shared prompt stage facts | `crates/shadows-core/src/turns/contract.yaml` |
