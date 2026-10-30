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
| `crates/shadows-http/src/aylul_providers.rs` | provider HTTP translation | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-http/src/aylul_profiles_tests.rs` | AYLUL configuration route acceptance tests | `crates/shadows-http/src/contract.yaml` |
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
Constructor indexing preserves hyphenated crate paths separately from Rust module names;
the ProjectLifecycle fixture checks construction through AppCore and rejects other callers.

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
| `crates/core-plans/src/signal.rs` | live plan-show vocabulary | `crates/core-plans/contract.yaml` |
| `crates/core-harness/src/owners.rs` | harness configuration dependencies | `crates/core-harness/contract.yaml` |
| `crates/core-harness/src/live.rs` | live adapter opening | `crates/core-harness/contract.yaml` |
| `crates/core-providers/src/lib.rs` | provider configuration boundary | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/` | AYLUL provider configuration | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/model.rs` | public provider metadata | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/validation.rs` | provider input rules | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/presets.rs` | compiled provider presets | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/probe.rs` | bounded provider listing requests | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/listing.rs` | provider model response parsing | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/slug.rs` | generated configuration slug shape | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/error.rs` | provider refusals | `crates/core-providers/contract.yaml` |
| `crates/core-providers/src/store/` | provider configuration SQL | `crates/core-providers/contract.yaml` |
| `crates/core-agents/src/lib.rs` | agent configuration service | `crates/core-agents/contract.yaml` |
| `crates/core-agents/src/` | permanent AYLUL agent identities | `crates/core-agents/contract.yaml` |
| `crates/core-agents/src/model.rs` | public agent metadata | `crates/core-agents/contract.yaml` |
| `crates/core-agents/src/home.rs` | agent home seeding | `crates/core-agents/contract.yaml` |
| `crates/core-agents/src/error.rs` | agent refusals | `crates/core-agents/contract.yaml` |
| `crates/core-agents/src/store/` | agent identity SQL | `crates/core-agents/contract.yaml` |
| `crates/core-extensions/src/` | the shared extension library | `crates/core-extensions/contract.yaml` |
| `crates/core-extensions/src/model.rs` | public extension metadata | `crates/core-extensions/contract.yaml` |
| `crates/core-extensions/src/files.rs` | bounded library file copies | `crates/core-extensions/contract.yaml` |
| `crates/core-extensions/src/error.rs` | extension library refusals | `crates/core-extensions/contract.yaml` |
| `crates/core-extensions/src/store/` | extension configuration SQL | `crates/core-extensions/contract.yaml` |
| `crates/core-usage/src/` | measured AYLUL usage | `crates/core-usage/contract.yaml` |
| `crates/core-usage/src/model.rs` | usage report shapes | `crates/core-usage/contract.yaml` |
| `crates/core-usage/src/report.rs` | historical usage aggregation | `crates/core-usage/contract.yaml` |
| `crates/core-usage/src/store/` | usage row SQL | `crates/core-usage/contract.yaml` |
| `crates/core-person/src/` | the local person record | `crates/core-person/contract.yaml` |
| `crates/core-person/src/model.rs` | public person record | `crates/core-person/contract.yaml` |
| `crates/core-person/src/store/` | person record SQL | `crates/core-person/contract.yaml` |
| `crates/core-harness/src/extension_home.rs` | per-agent extension projection | `crates/core-harness/contract.yaml` |
| `crates/shadows-http/src/aylul_extensions.rs` | extension HTTP translation | `crates/core-extensions/contract.yaml` |
| `crates/shadows-http/src/aylul_extensions_tests.rs` | extension route acceptance | `crates/shadows-http/src/aylul_extensions.rs` |
| `crates/core-turns/src/store/usage.rs` | atomic turn usage insertion | `crates/core-turns/contract.yaml` |
| `crates/core-harness/src/profile_home.rs` | serialized agent home settings writes | `docs/aylul/specs/2026-10-19-aylul-claude-code-brain-design.md` |
| `crates/shadows-http/src/aylul_env.rs` | AYLUL environment HTTP translation | `crates/core-agents/contract.yaml` |
| `crates/shadows-http/src/aylul_env_tests.rs` | AYLUL environment route acceptance tests | `crates/shadows-http/src/aylul_env.rs` |
| `crates/shadows-agent/src/profile_tests.rs` | isolated profile environment tests | `crates/shadows-agent/src/profile.rs` |
| `crates/core-secrets/src/master_key.rs` | OS master key lifecycle | `crates/core-secrets/contract.yaml` |
| `crates/core-secrets/src/master_key_tests.rs` | master key lifecycle tests with in-memory credentials | `crates/core-secrets/contract.yaml` |
| `crates/shadows/src/config_tests.rs` | AYLUL startup configuration tests | `crates/shadows/src/config.rs` |
| `crates/core-secrets/src/lib.rs` | encrypted environment operations | `crates/core-secrets/contract.yaml` |
| `crates/core-secrets/src/error.rs` | secret operation failures | `crates/core-secrets/contract.yaml` |
| `crates/core-secrets/src/` | encrypted profile environment operations | `crates/core-secrets/contract.yaml` |
| `crates/core-secrets/src/model.rs` | secret environment metadata | `crates/core-secrets/src/model.rs` |
| `crates/core-secrets/src/store/` | encrypted environment SQL | `crates/core-secrets/src/store/` |
| `crates/core-secrets/src/crypto.rs` | authenticated environment encryption | `crates/core-secrets/src/crypto.rs` |
| `crates/core-grants/src/approval.rs` | transaction-local approval provenance | `docs/superpowers/specs/2026-10-08-approval-authority-design.md` |
| `crates/core-db/src/write_observer.rs` | test-only committed-write observation | `crates/shadows/tests/openapi/read_only.rs` |
| `crates/core-design/src/map_proposals.rs` | map proposal service entry points | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/map_proposals.rs` | recording map proposals | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/map_proposal_shape.rs` | assembling proposed map shapes | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/map_proposal_read.rs` | reading map proposal state | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/map_approval.rs` | committing approved map proposals | `crates/core-design/contract.yaml` |
| `crates/core-design/src/map_rules.rs` | pure map validation | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/map_versions.rs` | durable map snapshots | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/roadmap_gate.rs` | refusing work that needs an approved roadmap | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/roadmap_seed.rs` | test-only seeding of the plans stage | `crates/core-design/contract.yaml` |
| `crates/core-design/src/roadmap_proposals.rs` | roadmap proposal service entry points | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/roadmap_proposals.rs` | recording roadmap proposals | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/roadmap_shape.rs` | checking proposed roadmap shapes | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/roadmap_proposal_read.rs` | reading roadmap proposal state | `crates/core-design/contract.yaml` |
| `crates/core-design/src/store/roadmap_approval.rs` | committing approved roadmap proposals | `crates/core-design/contract.yaml` |
| `crates/shadows/src/main.rs` | the one binary Shadows ships | `crates/shadows/src/cli/mod.rs` |
| `crates/shadows/src/cli/` | daemon startup | `crates/shadows/src/cli/args.rs` |
| `crates/shadows/src/desktop_runtime.rs` | desktop launch identity | `docs/aylul/specs/2026-10-20-aylul-desktop-web-companion-design.md` |
| `crates/shadows/src/runtime_record.rs` | private runtime record publication | `crates/shadows/src/runtime_record.rs` |
| `crates/shadows/src/runtime_record_tests.rs` | private record replacement tests | `crates/shadows/src/runtime_record.rs` |
| `crates/shadows/src/config.rs` | startup configuration resolved once | `crates/shadows/src/config.rs` |
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
| `crates/shadows-http/src/code_tests.rs` | the tests of the code index routes | `crates/shadows-http/src/code_tests.rs` |
| `crates/shadows-http/src/sse.rs` | the replay-then-live stream's SSE framing | `crates/shadows-http/src/sse.rs` |
| `crates/shadows-http/src/fs.rs` | choosing a project directory on this machine | `crates/shadows-http/src/fs.rs` |
| `crates/shadows-http/src/openapi.rs` | the OpenAPI document describing this API | `crates/shadows-http/src/openapi.rs` |
| `crates/shadows-http/src/failure/` | the transport mapping of a failure | `crates/shadows-http/src/failure/mod.rs` |
| `crates/shadows-http/src/failure/storage.rs` | how a storage failure becomes a status | `crates/shadows-http/src/failure/storage.rs` |
| `crates/shadows-http/src/failure/cases.rs` | shared transport error expectations | `crates/shadows-core/src/error.rs` |
| `crates/shadows-http/src/failure/mapping_tests.rs` | HTTP error mapping acceptance | `crates/shadows-core/src/error.rs` |
| `crates/shadows-http/src/failure/other.rs` | how a start, core or directory failure becomes a status | `crates/shadows-http/src/failure/other.rs` |
| `crates/shadows-http/src/guard.rs` | refusing requests pages were made to send | `crates/shadows-http/src/guard.rs` |
| `crates/shadows-mcp/src/lib.rs` | Shadows' MCP server | `crates/shadows-mcp/src/lib.rs` |
| `crates/shadows-mcp/src/auth.rs` | refusing a `/mcp` request that holds no live grant | `crates/shadows-mcp/src/auth.rs` |
| `crates/shadows-mcp/src/server.rs` | the tools a grant's kind may see | `crates/shadows-mcp/src/server.rs` |
| `crates/shadows-mcp/src/plan_tools.rs` | plan MCP translation | `crates/shadows-mcp/src/contract.yaml` |
| `crates/shadows-mcp/src/agreement_tools.rs` | agreement tool translation | `crates/shadows-mcp/src/agreement_tools.rs` |
| `crates/shadows-mcp/src/workspace_tools.rs` | workspace read tool translation | `crates/shadows-mcp/src/workspace_tools.rs` |
| `crates/shadows-mcp/src/refusal.rs` | what an MCP tool call answers | `crates/shadows-mcp/src/refusal.rs` |
| `crates/shadows-core/src/lib.rs` | the application behind `AppCore` | `crates/shadows-core/src/app.rs` |
| `crates/shadows-core/src/app.rs` | the application's composition root | `crates/shadows-core/src/app.rs` |
| `crates/shadows-core/src/error.rs` | the stable failure taxonomy clients match on | `crates/shadows-core/src/error.rs` |
| `crates/core-db/src/command.rs` | external-command identity for idempotency | `crates/core-db/contract.yaml` |
| `crates/core-db/src/derive.rs` | command ids Shadows derives when a caller names none | `crates/core-db/contract.yaml` |
| `crates/core-db/src/journal.rs` | durable journal publication | `crates/core-db/contract.yaml` |
| `crates/core-db/src/id.rs` | the UUID id newtype pattern | `crates/core-db/contract.yaml` |
| `crates/core-db/src/embedded_migrations.rs` | Historical Windows profiles migration source selection | `docs/superpowers/specs/2026-09-21-persistence-api-and-migrations-design.md` |
| `crates/core-db/src/lib.rs` | shared SQLite lifetime | `crates/core-db/contract.yaml` |
| `crates/core-db/src/error.rs` | database failure vocabulary | `crates/core-db/contract.yaml` |
| `crates/core-runtime/src/` | the runtime instance's lifecycle | `crates/core-runtime/src/lib.rs` |
| `crates/core-runtime/src/lib.rs` | runtime registration boundary | `crates/core-runtime/contract.yaml` |
| `crates/core-runtime/src/store/` | runtime row persistence | `crates/core-runtime/contract.yaml` |
| `crates/core-runtime/src/model.rs` | runtime identity values | `crates/core-runtime/contract.yaml` |
| `crates/shadows-core/src/testing/` | the test apparatus every crate's tests share | `crates/shadows-core/src/testing/mod.rs` |
| `crates/core-projects/src/` | the projects, the folders a person picks them from included | `crates/core-projects/src/lib.rs` |
| `crates/core-projects/src/model.rs` | the project types callers meet | `crates/core-projects/src/model.rs` |
| `crates/core-projects/src/store/` | a project's rows | `crates/core-projects/src/store/` |
| `crates/core-projects/src/harness_policy.rs` | which harness a thread or a project's modes may name | `crates/core-projects/src/harness_policy.rs` |
| `crates/core-projects/src/browse.rs` | showing a person the directories they can choose from | `crates/core-projects/src/browse.rs` |
| `crates/core-threads/src/` | the planning threads, what they recorded included | `crates/core-threads/src/mod.rs` |
| `crates/core-threads/src/error.rs` | stable thread refusals | `crates/core-threads/contract.yaml` |
| `crates/core-threads/src/testing.rs` | test-only thread command contexts | `crates/core-threads/contract.yaml` |
| `crates/core-threads/src/store/records.rs` | snapshot-local thread facts | `crates/core-threads/contract.yaml` |
| `crates/core-threads/src/model.rs` | the planning thread types callers meet | `crates/core-threads/src/model.rs` |
| `crates/core-threads/src/title.rs` | the text a title taken from a conversation may read | `crates/core-threads/src/title.rs` |
| `crates/core-threads/src/store/` | private ThreadsStore persistence | `crates/core-threads/src/store/thread.rs` |
| `crates/core-threads/src/store/remove.rs` | atomic removal of a planning thread | `crates/core-threads/src/store/remove.rs` |
| `crates/core-operations/src/` | durable operation lifecycle | `crates/core-operations/contract.yaml` |
| `crates/core-operations/src/model.rs` | durable operation values | `crates/core-operations/contract.yaml` |
| `crates/core-operations/src/store/` | private operation persistence | `crates/core-operations/contract.yaml` |
| `crates/core-operations/src/fork.rs` | transactional fork eligibility | `crates/core-operations/contract.yaml` |
| `crates/core-operations/src/testing.rs` | test-only runtime stop entry | `crates/core-operations/contract.yaml` |
| `crates/core-turns/src/testing.rs` | test-only lifecycle composition | `crates/core-turns/contract.yaml` |
| `crates/core-turns/src/` | Planner turns, from start to stop | `crates/core-turns/src/lib.rs` |
| `crates/core-turns/src/model.rs` | the operation types callers meet | `crates/core-turns/src/model.rs` |
| `crates/core-turns/src/store/` | turn command persistence | `crates/core-turns/src/store/turn.rs` |
| `crates/core-turns/src/store/queue.rs` | the queue's SQLite queries | `crates/core-turns/src/store/queue.rs` |
| `crates/core-turns/src/store/steer.rs` | recording a steered message | `crates/core-turns/src/store/steer.rs` |
| `crates/core-turns/src/queue.rs` | the waiting messages of a conversation | `crates/core-turns/src/queue.rs` |
| `crates/core-turns/src/record.rs` | the checks that commit a turn's start | `crates/core-turns/src/record.rs` |
| `crates/core-turns/src/steer.rs` | a Send now inside the turn's watcher | `crates/core-turns/src/steer.rs` |
| `crates/core-turns/src/turn.rs` | the recorded ending of a live Planner turn | `crates/core-turns/src/turn.rs` |
| `crates/core-turns/src/entries.rs` | turning harness events into durable entries | `crates/core-turns/src/entries.rs` |
| `crates/core-turns/src/tools.rs` | tool card collection | `crates/core-turns/contract.yaml` |
| `crates/core-turns/src/hunks.rs` | edit hunk placement | `crates/core-turns/contract.yaml` |
| `crates/core-turns/src/subagents.rs` | gathering a subagent's tool calls into its card (§22) | `crates/core-turns/src/subagents.rs` |
| `crates/core-harness/src/` | the harnesses, each thread's open session included | `crates/core-harness/src/lib.rs` |
| `crates/core-harness/src/model.rs` | the harness shapes callers meet | `crates/core-harness/src/model.rs` |
| `crates/core-harness/src/store/` | harness persistence | `crates/core-harness/contract.yaml` |
| `crates/core-harness/src/assistant.rs` | choosing what a thread's adapter opens with, by harness kind | `crates/core-harness/src/assistant.rs` |
| `crates/core-harness/src/sessions.rs` | the live adapter connection each open thread holds | `crates/core-harness/src/sessions.rs` |
| `crates/core-harness/src/settings.rs` | setting an open session's options | `crates/core-harness/src/settings.rs` |
| `crates/core-harness/src/mode.rs` | durable conversation mode commands | `crates/core-harness/contract.yaml` |
| `crates/core-harness/src/fork.rs` | atomic conversation fork composition | `crates/core-harness/contract.yaml` |
| `crates/core-harness/src/store/mode.rs` | conversation permission persistence | `crates/core-harness/contract.yaml` |
| `crates/core-turns/src/store/remove.rs` | thread removal composition | `crates/core-turns/contract.yaml` |
| `crates/core-harness/src/setup.rs` | what a Planner session opens with | `crates/core-harness/src/setup.rs` |
| `crates/core-harness/src/standards.rs` | Planner standards rendering | `crates/core-harness/src/standards.rs` |
| `crates/core-harness/src/offers.rs` | the latest choices each open session offers | `crates/core-harness/src/offers.rs` |
| `crates/core-harness/src/commands.rs` | the latest `/` list each open session sent | `crates/core-harness/src/commands.rs` |
| `crates/core-harness/src/titles.rs` | handing the titles a harness sends to its thread | `crates/core-harness/src/titles.rs` |
| `crates/core-plans/src/` | project plans under the rules of §16 | `crates/core-plans/src/lib.rs` |
| `crates/core-plans/src/model.rs` | the plan types callers meet | `crates/core-plans/src/model.rs` |
| `crates/core-plans/src/dependencies.rs` | task dependency value shapes | `crates/core-plans/src/dependencies.rs` |
| `crates/core-plans/src/bindings.rs` | exact agreement binding value shapes | `crates/core-plans/src/bindings.rs` |
| `crates/core-plans/src/binding_edit.rs` | person adoption entry point | `crates/core-plans/src/binding_edit.rs` |
| `crates/core-plans/src/map.rs` | project plan map value shapes | `crates/core-plans/src/map.rs` |
| `crates/core-plans/src/dependency_graph.rs` | start/complete dependency ordering | `crates/core-plans/src/dependency_graph.rs` |
| `crates/core-plans/src/store/` | plans' SQLite queries | `crates/core-plans/src/store/edit.rs` |
| `crates/core-plans/src/store/edit.rs` | changing a plan version | `crates/core-plans/src/store/edit.rs` |
| `crates/core-plans/src/store/draft.rs` | starting a plan version | `crates/core-plans/src/store/draft.rs` |
| `crates/core-plans/src/store/plan.rs` | a plan's state lifecycle with its versions | `crates/core-plans/src/store/plan.rs` |
| `crates/core-plans/src/store/read.rs` | reading plan versions | `crates/core-plans/src/store/read.rs` |
| `crates/core-plans/src/store/task.rs` | a plan version's task graph rows | `crates/core-plans/src/store/task.rs` |
| `crates/core-plans/src/store/dependencies.rs` | persisted cross-plan links | `crates/core-plans/src/store/dependencies.rs` |
| `crates/core-plans/src/store/bindings.rs` | persisted agreement adoption | `crates/core-plans/src/store/bindings.rs` |
| `crates/core-plans/src/store/binding_participants.rs` | captured adoption participant reads | `crates/core-plans/src/store/binding_participants.rs` |
| `crates/core-plans/src/store/graph.rs` | reachable latest-version dependency checks | `crates/core-plans/src/store/graph.rs` |
| `crates/core-plans/src/store/linked.rs` | captured dependency preview reads | `crates/core-plans/src/store/linked.rs` |
| `crates/core-plans/src/store/map.rs` | captured latest-plan map reads | `crates/core-plans/src/store/map.rs` |
| `crates/core-plans/src/store/notifications.rs` | transactional dependency invalidations | `crates/core-plans/src/store/notifications.rs` |
| `crates/core-plans/src/store/view.rs` | showing a plan version in its conversation | `crates/core-plans/src/store/view.rs` |
| `crates/core-plans/src/ops.rs` | applying one batch of plan edits | `crates/core-plans/src/ops.rs` |
| `crates/core-plans/src/rules.rs` | what makes a plan invalid or unready | `crates/core-plans/src/rules.rs` |
| `crates/core-plans/src/scope.rs` | plan calls made under an MCP grant | `crates/core-plans/src/scope.rs` |
| `crates/core-plans/src/conversation.rs` | the plan in the conversation | `crates/core-plans/src/conversation.rs` |
| `crates/core-grants/src/` | grant authorization | `crates/core-grants/src/mod.rs` |
| `crates/core-grants/src/model.rs` | the grant types callers meet | `crates/core-grants/src/model.rs` |
| `crates/core-grants/src/store/` | an MCP grant's rows, from issue to revocation | `crates/core-grants/src/store/` |
| `crates/core-instructions/src/` | a project's numbered Planner instructions | `crates/core-instructions/src/lib.rs` |
| `crates/core-design/src/` | a project's design workspace | `crates/core-design/src/lib.rs` |
| `crates/core-design/src/structure.rs` | rendering an approved map as files | `crates/core-design/src/structure.rs` |
| `crates/core-design/src/structure_create.rs` | coordinating a person's structure write | `crates/core-design/src/structure_create.rs` |
| `crates/core-design/src/structure_write.rs` | publishing recorded files safely | `crates/core-design/src/structure_write.rs` |
| `crates/core-design/src/store/structure.rs` | structure write records | `crates/core-design/src/store/structure.rs` |
| `crates/core-design/src/vision_items.rs` | applying vision item changes | `crates/core-design/src/vision_items.rs` |
| `crates/core-design/src/store/vision_versions.rs` | stored vision versions | `crates/core-design/src/store/vision_versions.rs` |
| `crates/core-design/src/proposals.rs` | vision proposal entry points | `crates/core-design/src/proposals.rs` |
| `crates/core-design/src/store/proposals.rs` | recording vision proposals | `crates/core-design/src/store/proposals.rs` |
| `crates/core-design/src/store/proposal_read.rs` | reading proposal state | `crates/core-design/src/store/proposal_read.rs` |
| `crates/core-design/src/store/proposal_approval.rs` | approving vision proposals | `crates/core-design/src/store/proposal_approval.rs` |
| `crates/core-design/src/model.rs` | the workspace value shapes | `crates/core-design/src/model.rs` |
| `crates/core-design/src/agreement.rs` | shared agreement value shapes | `crates/core-design/src/agreement.rs` |
| `crates/core-design/src/agreement_validation.rs` | offline agreement content validation | `crates/core-design/src/agreement_validation.rs` |
| `crates/core-design/src/agreements.rs` | shared agreement service entry points | `crates/core-design/src/agreements.rs` |
| `crates/core-design/src/agreement_scope.rs` | grant attribution for agreement calls | `crates/core-design/src/agreement_scope.rs` |
| `crates/core-plans/src/agreement_review.rs` | agreement review value shapes | `crates/core-plans/src/agreement_review.rs` |
| `crates/core-design/src/store/agreement_binding.rs` | exact-version adoption checks | `crates/core-design/src/store/agreement_binding.rs` |
| `crates/core-plans/src/store/agreement_review.rs` | captured impact review | `crates/core-plans/src/store/agreement_review.rs` |
| `crates/core-design/src/store/agreements.rs` | captured agreement version reads | `crates/core-design/src/store/agreements.rs` |
| `crates/core-design/src/store/agreement_write.rs` | transactional agreement Draft mutations | `crates/core-design/src/store/agreement_write.rs` |
| `crates/core-design/src/parts.rs` | part service entry points | `crates/core-design/src/parts.rs` |
| `crates/core-design/src/workspace.rs` | a grant's read-only view of its project workspace | `crates/core-design/src/workspace.rs` |
| `crates/core-design/src/outcomes.rs` | outcome service entry points | `crates/core-design/src/outcomes.rs` |
| `crates/core-design/src/ops.rs` | normalization of workspace edits | `crates/core-design/src/ops.rs` |
| `crates/core-standards/src/` | project standards versions | `crates/core-standards/contract.yaml` |
| `crates/core-standards/src/model.rs` | public standards values | `crates/core-standards/contract.yaml` |
| `crates/core-standards/src/base.rs` | bundled base parsing | `crates/core-standards/contract.yaml` |
| `crates/core-standards/src/validation.rs` | project additions validation | `crates/core-standards/contract.yaml` |
| `crates/core-design/src/stage.rs` | a project's stage from its workspace | `crates/core-design/src/stage.rs` |
| `crates/core-standards/src/store/` | immutable additions persistence | `crates/core-standards/contract.yaml` |
| `crates/core-turns/src/store/standards.rs` | invocation standards selection | `crates/core-turns/contract.yaml` |
| `crates/core-design/src/store/stage.rs` | reading a project's stage inputs | `crates/core-design/src/store/stage.rs` |
| `crates/core-design/src/store/outcomes.rs` | snapshot reads of outcomes | `crates/core-design/src/store/outcomes.rs` |
| `crates/core-design/src/store/outcome_edit.rs` | transactional outcome mutations | `crates/core-design/src/store/outcome_edit.rs` |
| `crates/core-design/src/store/hierarchy.rs` | transactional containment ordering | `crates/core-design/src/store/hierarchy.rs` |
| `crates/core-design/src/store/parts.rs` | snapshot reads of parts | `crates/core-design/src/store/parts.rs` |
| `crates/core-design/src/store/part_edit.rs` | transactional part mutations | `crates/core-design/src/store/part_edit.rs` |
| `crates/core-design/src/store/` | persistence of the design workspace | `crates/core-design/src/store/vision.rs` |
| `crates/core-design/src/store/vision.rs` | atomic workspace edits with immutable replay results | `crates/core-design/src/store/vision.rs` |
| `crates/core-code/src/` | the code index of each project's folder | `crates/core-code/src/lib.rs` |
| `crates/core-code/src/model.rs` | the code index types callers meet | `crates/core-code/src/model.rs` |
| `crates/core-code/src/store/` | private CodeStore index persistence | `crates/core-code/src/store/mod.rs` |
| `crates/core-code/src/store/links.rs` | transactional code link persistence | `crates/core-code/src/store/links.rs` |
| `crates/core-code/src/store/reach.rs` | live one-way project reach in a snapshot | `crates/core-code/src/store/reach.rs` |
| `crates/core-code/src/scan.rs` | walking a project's folder into the index | `crates/core-code/src/scan.rs` |
| `crates/core-code/src/scope.rs` | what a code question may read | `crates/core-code/src/scope.rs` |
| `crates/core-code/src/watch.rs` | keeping one active project's index current | `crates/core-code/src/watch.rs` |
| `crates/core-code/src/active.rs` | which projects are active, in order of use | `crates/core-code/src/active.rs` |
| `crates/core-code/src/links.rs` | the commands over links or the active limit | `crates/core-code/src/links.rs` |
| `crates/core-events/src/` | what clients watch live | `crates/core-events/src/lib.rs` |
| `crates/core-events/src/model.rs` | journal replay cursor | `crates/core-events/src/model.rs` |
| `crates/core-events/src/subscription.rs` | one subscriber's replay-then-live stream | `crates/core-events/src/subscription.rs` |
| `crates/shadows-agent/src/lib.rs` | the ACP client for a harness adapter | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp/` | private ACP protocol implementation | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp/connection.rs` | ACP transport initialization | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp/session.rs` | ACP session configuration | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp/prompt.rs` | ACP prompt lifecycle | `crates/shadows-agent/src/acp.rs` |
| `crates/shadows-agent/src/acp/mapping.rs` | ordered ACP event translation | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/events/tool.rs` | tool activity reports | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/events/subagent.rs` | subagent metadata | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/events/limits.rs` | reported account limits | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/events/usage.rs` | reported turn usage | `crates/shadows-agent/src/events.rs` |
| `crates/shadows-agent/src/events/` | harness report definitions | `crates/shadows-agent/src/events.rs` |
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
| `crates/shadows-agent/src/claude.rs` | the launch spec of the pinned Claude ACP adapter | `crates/shadows-agent/src/claude.rs` |
| `crates/shadows-index/src/lib.rs` | one file's text turned into its tags | `crates/shadows-index/src/extract.rs` |
| `crates/shadows-index/src/languages.rs` | the table of languages Shadows indexes | `crates/shadows-index/src/languages.rs` |
| `crates/shadows-index/src/extract.rs` | one file's text in, its tags out | `crates/shadows-index/src/extract.rs` |
| `crates/shadows-process/src/lib.rs` | OS process ownership with whole-tree containment | `crates/shadows-process/src/lib.rs` |
| `crates/shadows-process/src/launch.rs` | child launch configuration | `crates/shadows-process/src/lib.rs` |
| `crates/shadows-process/src/persistent.rs` | independent process lifetime | `crates/shadows-process/src/lib.rs` |
| `crates/shadows-process/src/bin/` | test apparatus that no product code links | `crates/shadows-process/src/bin/tree_probe.rs` |
| `crates/fake-acp/src/main.rs` | test apparatus that no product code links | `crates/fake-acp/src/main.rs` |
| `crates/fake-acp/src/handlers.rs` | fake protocol handler registration | `crates/fake-acp/src/main.rs` |
| `crates/fake-acp/src/handlers/` | fake protocol request handling | `crates/fake-acp/src/main.rs` |
| `crates/fake-acp/src/handlers/session.rs` | fake session lifecycle requests | `crates/fake-acp/src/session.rs` |
| `crates/fake-acp/src/handlers/prompt.rs` | fake prompt requests | `crates/fake-acp/src/prompts.rs` |
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

| `crates/core-grants/src/writer.rs` | authenticated writer values | `crates/core-grants/contract.yaml` |
| `crates/core-grants/src/approver.rs` | approval provenance values | `crates/core-grants/contract.yaml` |
| `crates/core-grants/src/error.rs` | grant refusal mapping | `crates/core-grants/contract.yaml` |
| `crates/core-grants/src/tools.rs` | grant tool policy | `crates/core-grants/contract.yaml` |
| `crates/core-grants/src/testing.rs` | test-only grant entry points | `crates/core-grants/contract.yaml` |

| `crates/core-harness/src/access.rs` | live session owner operations | `crates/core-harness/contract.yaml` |
| `crates/core-harness/src/store/invocation.rs` | invocation transaction entry points | `crates/core-harness/contract.yaml` |
| `crates/core-turns/src/stage.rs` | shared prompt stage facts | `crates/core-turns/contract.yaml` |

| `crates/core-design/src/error.rs` | error boundary | `crates/core-design/contract.yaml` |
| `crates/core-design/src/proposal_context.rs` | proposal context boundary | `crates/core-design/contract.yaml` |
| `crates/core-plans/src/agreements.rs` | agreements boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/design_edit.rs` | design edit boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/design_ops.rs` | design ops boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/design_views.rs` | design views boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/error.rs` | error boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/project_links.rs` | project links boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/roadmap_approval.rs` | roadmap approval over plan-linked outcomes | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/store/design_links.rs` | design links boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/store/design_parts.rs` | design parts boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/store/project_links.rs` | project links boundary | `crates/core-plans/contract.yaml` |
| `crates/core-plans/src/testing.rs` | testing boundary | `crates/core-plans/contract.yaml` |
| `crates/core-turns/src/plan_show.rs` | plan show boundary | `crates/core-turns/contract.yaml` |

| `crates/core-turns/src/remove.rs` | thread removal orchestration | `crates/core-turns/contract.yaml` |
| `crates/core-turns/src/error.rs` | turn refusal mapping | `crates/core-turns/contract.yaml` |

| `crates/core-threads/src/attachment.rs` | bounded image validation through every animation frame | `crates/core-threads/contract.yaml` |
| `crates/core-threads/src/store/attachment.rs` | immutable image persistence | `crates/core-threads/contract.yaml` |
| `crates/shadows-http/src/attachments.rs` | image HTTP translation | `crates/core-threads/contract.yaml` |

| `crates/core-turns/src/store/images.rs` | neutral image read composition | `crates/core-turns/contract.yaml` |

| `crates/core-project-lifecycle/src/` | cross-owner project lifecycle | `crates/core-project-lifecycle/contract.yaml` |
| `crates/core-project-lifecycle/src/error.rs` | project removal refusal | `crates/core-project-lifecycle/contract.yaml` |
| `crates/core-project-lifecycle/src/store.rs` | atomic project removal composition | `crates/core-project-lifecycle/contract.yaml` |

| `crates/core-plans/src/store/event_projection.rs` | identifier-only plan event projection | `crates/core-plans/contract.yaml` |
| `crates/core-events/src/store.rs` | private journal replay persistence | `crates/core-events/contract.yaml` |
| `crates/shadows-http/src/aylul_agents.rs` | agent HTTP translation | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-http/src/aylul_person.rs` | person settings HTTP translation | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-http/src/aylul_usage.rs` | usage HTTP translation | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-http/src/aylul_threads.rs` | agent conversation listing | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-http/src/plan_design.rs` | plan association HTTP translation | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-http/src/standards.rs` | standards HTTP translation | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-http/src/transport.rs` | shared HTTP request handling | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-mcp/src/code_tools.rs` | code query MCP translation | `crates/shadows-mcp/src/contract.yaml` |
| `crates/shadows-mcp/src/turn_tools.rs` | plan display MCP translation | `crates/shadows-mcp/src/contract.yaml` |

| `crates/shadows-http/src/adapter_agreements_tests.rs` | adapter entry point agreements | `crates/shadows-http/src/contract.yaml` |
| `crates/shadows-http/src/contract.yaml` | HTTP adapter obligations | `crates/shadows-http/src/lib.rs` |
| `crates/shadows-mcp/src/contract.yaml` | MCP adapter obligations | `crates/shadows-mcp/src/lib.rs` |

| `crates/core-projects/src/lib.rs` | project registration boundary | `crates/core-projects/contract.yaml` |
| `crates/core-projects/src/error.rs` | project refusals | `crates/core-projects/contract.yaml` |
| `crates/core-projects/src/directory.rs` | project directory validation | `crates/core-projects/contract.yaml` |

| `crates/core-usage/src/lib.rs` | measured usage boundary | `crates/core-usage/contract.yaml` |
| `crates/core-usage/src/error.rs` | usage refusals | `crates/core-usage/contract.yaml` |

| `crates/core-person/src/lib.rs` | person preference boundary | `crates/core-person/contract.yaml` |
| `crates/core-person/src/error.rs` | person refusals | `crates/core-person/contract.yaml` |

| `crates/core-instructions/src/lib.rs` | numbered instructions boundary | `crates/core-instructions/contract.yaml` |
| `crates/core-instructions/src/model.rs` | public saved instructions | `crates/core-instructions/contract.yaml` |
| `crates/core-instructions/src/error.rs` | instructions failures | `crates/core-instructions/contract.yaml` |
| `crates/core-instructions/src/store/` | private instructions SQL | `crates/core-instructions/contract.yaml` |

| `crates/core-standards/src/lib.rs` | project standards boundary | `crates/core-standards/contract.yaml` |
| `crates/core-standards/src/error.rs` | standards refusals | `crates/core-standards/contract.yaml` |
| `crates/core-standards/src/standards.yaml` | bundled standards definition | `crates/core-standards/contract.yaml` |

| `crates/core-code/src/lib.rs` | code indexing boundary | `crates/core-code/contract.yaml` |
| `crates/core-code/tests/recheck/` | question refresh race regression | `crates/core-code/tests/recheck/invariants.rs` |
| `crates/core-code/src/error.rs` | index refusals | `crates/core-code/contract.yaml` |
| `crates/core-code/src/testing.rs` | gated index inspection | `crates/core-code/contract.yaml` |

| `crates/core-agents/assets/aylul-profile/CLAUDE.md` | bundled agent home instructions | `crates/core-agents/contract.yaml` |
| `crates/core-agents/assets/aylul-profile/settings.json` | bundled agent home settings | `crates/core-agents/contract.yaml` |

| `crates/core-extensions/src/lib.rs` | extension library boundary | `crates/core-extensions/contract.yaml` |

| `crates/core-threads/src/lib.rs` | thread command boundary | `crates/core-threads/contract.yaml` |

| `crates/core-grants/src/lib.rs` | grant authority boundary | `crates/core-grants/contract.yaml` |

| `crates/core-operations/src/lib.rs` | durable lifecycle boundary | `crates/core-operations/contract.yaml` |
| `crates/core-operations/src/error.rs` | operation failure vocabulary | `crates/core-operations/contract.yaml` |

| `crates/core-harness/src/error.rs` | harness failure vocabulary | `crates/core-harness/contract.yaml` |
| `crates/core-harness/src/prompt.txt` | Planner opening instructions | `crates/core-harness/contract.yaml` |
| `crates/core-harness/src/schema_descriptions/` | harness wire description metadata | `crates/core-harness/contract.yaml` |

| `crates/core-design/src/openapi-3.1-schema.json` | bundled OpenAPI document schema | `crates/core-design/contract.yaml` |

| `crates/core-design/src/openapi-3.1-schema-base.json` | bundled OpenAPI schema-base validation | `crates/core-design/contract.yaml` |

| `crates/core-design/src/openapi-3.1-meta.json` | bundled OpenAPI vocabulary schema | `crates/core-design/contract.yaml` |

| `crates/core-design/src/openapi-3.1-dialect.json` | bundled OpenAPI schema dialect | `crates/core-design/contract.yaml` |

| `crates/core-turns/src/handles.rs` | live turn registry | `crates/core-turns/contract.yaml` |

| `crates/core-turns/src/prompt.rs` | prompt composition | `crates/core-turns/contract.yaml` |

| `crates/core-turns/src/shutdown.rs` | live shutdown orchestration | `crates/core-turns/contract.yaml` |

| `crates/core-turns/src/spawn.rs` | live registration | `crates/core-turns/contract.yaml` |

| `crates/core-events/src/error.rs` | event subscription failures | `crates/core-events/contract.yaml` |

| `crates/core-runtime/src/error.rs` | runtime startup failures | `crates/core-runtime/contract.yaml` |
