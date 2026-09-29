# Section 1 — Architecture Overview

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there — many of them point into a different file.

`shadows` is a Cargo workspace: one application crate, `shadows-core`, with small crates around it that translate HTTP, MCP and the command line, and two infrastructure crates that run the harness and processes. §14.3 owns the crate list and the direction of their dependencies (decided 2026-09-27; it was one crate exposing a library and a binary until Milestone 2.5). The product client is an independent browser application that communicates only through the local protocol.

The binary has two primary modes:

```text
shadows serve    # long-running local daemon/runtime
shadows ...      # CLI client
```

`shadows serve` exposes the local API only. It prints the address and never opens a browser automatically; the user chooses which browser to use.

**The daemon does not serve or embed the client** (decided 2026-09-23, superseding the single-page `include_str!` recommendation in `docs/evidence/harness/SERVE_STREAM_SPIKE.md` Finding 1). The backend stays on the machine that owns the projects, the processes, and the harness; clients reach it over the protocol, the way a hosted web app reaches a locally running agent. The Web client is the first client, a desktop client is a later one, and a hosted Web client reaching a remote daemon is a later deployment of the same one. Consequences:

- The Web client lives in `web/` in this repository, built and deployed on its own. It is React + TypeScript on Vite, with TanStack Router and TanStack Query, shadcn/ui on Tailwind v4 (theme as CSS variables in one place), Motion for animation, and Streamdown for rendering streamed markdown. Anything else earns its place the day a screen needs it.
- **The protocol is described, not copied.** The daemon generates an OpenAPI document from its routes (`utoipa` + `utoipa-axum`), and every client's types and HTTP client are generated from it (`openapi-typescript` + `openapi-fetch`). A route change that the client has not followed fails the client's build. The daemon serves the document at `GET /api/openapi.json` and it is checked in at `api/openapi.json`, kept current by `crates/shadows/tests/openapi.rs` the way the code map is (`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` regenerates it); its keys are sorted so it changes only when the protocol does. Id newtypes appear as named `uuid`-format strings. The SSE stream (§2.10) is documented there too, but it is consumed by a hand-written hook, because its replay, then live, then dedupe-by-seq contract is ours.
- The daemon allows cross-origin requests only from origins listed in configuration: `shadows serve --allow-origin <origin>`, repeatable, an exact `scheme://host[:port]` checked at startup. Unset, the list is Vite's dev server under both of its names, `http://localhost:5173` and `http://127.0.0.1:5173`; given at all, it replaces them. No credentials are allowed. There is no `GET /`. CORS only decides whether a page may read an answer, so before any route the daemon also refuses (`403 ORIGIN_REFUSED`) a request whose `Host` is not `localhost` or an IP address (DNS rebinding), whose `Origin` is present and not listed, or that a browser marks as sent from another site (`Sec-Fetch-Site`) without an `Origin` (a no-cors request such as an `<img>` aimed at the disk routes). A request with none of those headers is not from a browser and passes.
- Filesystem browsing, for choosing or creating a project directory, is a daemon route: only the daemon can see the machine's disk. `GET /api/fs/dirs` lists one directory's subdirectories (or, with no path, the roots), flagging hidden ones rather than filtering them and skipping entries it cannot read; `POST /api/fs/dirs` creates one directory whose name is a single component Windows would accept. They expose the disk to every client the CORS list admits, which is sound only under the OPEN block below.

> **OPEN — remote access.** A client on another machine reaching this daemon needs authentication and transport security that do not exist yet. Milestone 0 binds to loopback and has neither. Trigger that closes it: the first deployment where the client and the daemon are not on the same machine. It does not block Milestone 0, whose clients are all local.

## 1.0 First runnable product boundary

Before the wider architecture is implemented, Shadows must prove one complete product path:

```text
shadows serve
  -> user manually opens the local Web client
  -> selects a local-directory project
  -> creates or resumes a PlanningThread
  -> starts one real Claude Planner turn
  -> receives live output
  -> stops the turn
  -> process layer confirms the complete child tree is gone
  -> Operation becomes Cancelled
  -> daemon restart restores the durable thread and terminal operation
```

Only modules and tables needed by this path are implemented initially. Workflow scheduling, execution DAGs, deterministic verification, MCP, ResearchArtifact, team sync, PostgreSQL, and the complete proposed schema remain later work and do not block the runnable milestone.

## 1.1 Core top-level modules

There are **15 core domain/application/adapter modules**:

```text
project/
thread/
command/
runtime/

agent/
planner/
workflow/
operation/

scheduler/
execution/
verification/
events/

storage/
protocol/
cli/
```

These name responsibilities, not folders of one crate. Since Milestone 2.5 the
application ones are services and modules inside `shadows-core` (§14.4), and the
adapters are crates: `protocol/` is `shadows-http`, `cli/` is the `shadows`
binary, `agent/` is `shadows-agent` (§14.3). `scheduler/`, `execution/` and
`verification/` are not built yet.

## 1.2 Cross-cutting / infrastructure modules

```text
process/
mcp/
config/
secrets/
error/
tracing/
```

`process/` is the `shadows-process` crate and `mcp/` the `shadows-mcp` crate; `config/` and `tracing/` are modules of the `shadows` binary, and `error/` is `shadows-core`'s (§14.3). `secrets/` is not built yet. “Cross-cutting” describes responsibility, not a separate architectural layer.

## 1.3 Ownership rules

| Owner | Sole owner of |
|---|---|
| `shadows-agent` | Agent-harness abstraction and harness-specific translation from `AgentInvocation` to `ProcessSpec` |
| `shadows-process` | OS process primitives: `tokio::process`, process trees/groups/job objects, `ProcessSpec`, `ProcessHandle` |
| `shadows-core`: `db/`, each service's `store` and `runtime/store.rs` | SQLx, SQLite schema/query code, migrations, future backend adapters (§2.9) |
| `shadows-http` | HTTP/SSE transport |
| `shadows-mcp` | MCP adapter and MCP request/response translation |
| `secrets` (planned) | `SecretRef` resolution; secret values are resolved only at spawn |
| `scheduler` (planned) | Pure scheduling decision logic; no I/O |
| `shadows-core`'s `events` | Durable-event vocabulary, cursor semantics, transient live-event abstraction |

### Mechanical boundary examples

- `shadows-agent` may construct `ProcessSpec`, but only `shadows-process` may call `tokio::process`.
- `verification` may construct a deterministic process check, but process spawning still goes through `shadows-process`.
- `shadows-core`'s stores (each service's `store`, `runtime/store.rs`) and `db/` may import SQLx; a service's `model.rs` and every other crate's product code may not.
- `shadows-http` and `shadows-mcp` call `AppCore`'s services; they cannot reach storage, which is private to `shadows-core` (§14.6).

## 1.4 Agent abstraction

The fundamental model is:

```text
Role != Harness != Provider != Model
```

Examples:

```text
Role: Executor
Harness: Claude Code CLI
Provider: MiniMax
Model: MiniMax-M3
```

```text
Role: Executor
Harness: Codex CLI
Provider: OpenAI
Model: configured OpenAI model
```

The same executable may be launched as multiple isolated workers with different environment/profile/model settings.

Conceptual runtime seam:

```rust
trait AgentHarness {
    async fn start(
        &self,
        invocation: AgentInvocation,
    ) -> Result<AgentRunHandle, agent::Error>;
}
```

`AgentRunHandle` is runtime-only. Durable lifecycle is represented by `Operation`.

### Harness identity is explicit configuration

A harness executable is resolved from explicit configuration, never from `PATH`.
The resolved path and the harness's self-reported version are read when an
Operation starts and recorded with it.

This is not defensive habit. The measured harness stream contract
(`docs/evidence/harness/SERVE_STREAM_SPIKE.md`) is the contract of one
installation at one version, and a machine can carry several: on the validation
machine the Claude desktop application bundles its own copy, at more than one
version, entirely separate from whatever `PATH` resolves. An auto-update can
therefore change the output contract underneath a running install. Without a
recorded path and version the first symptom is a blank page rather than an error,
and nothing in the durable record says which binary produced which turn.

## 1.5 Process boundary

```text
AgentInvocation
      ↓
agent/<harness>
      ↓ translates
ProcessSpec
      ↓
process::spawn(ProcessSpec)
      ↓
ProcessHandle
```

`process/` knows nothing about `Role`, `Claude`, `Codex`, planning, workflows, or verification.

Child environment is built per process. The daemon's own global environment is never mutated.

Worker isolation may include:

```text
HOME / USERPROFILE
APPDATA / config dirs
temporary directory
provider-specific environment
secret values
working directory / worktree
```

`process/` receives only OS-level intent:

```text
executable
argv
cwd
explicit environment
stdio policy
timeout
containment policy
resource limits when supported
```

The child environment is built explicitly; secret values are resolved only at
spawn and never persisted, traced, or placed in command-line arguments.

`HOME`, `USERPROFILE`, `APPDATA`, temporary directories, and provider
configuration locations follow the selected isolation profile rather than leaking
from the daemon by accident. Clearing the environment wholesale is not the same
as isolating it: on Windows a child that loses `SystemRoot`, `SystemDrive`,
`ComSpec`, or `PATHEXT` fails in ways that never appear on Linux.

Persisted diagnostics about a spawn may include executable identity and version,
argument count, workspace identity, profile name, and environment key names. They
must not include prompt text, model output, secret values, complete environments,
or sensitive argument values.

A child's stdin is closed unless the harness contract requires streaming input.
An open stdin that never receives data costs a fixed stall on every turn.


### Managed-process containment invariant

A managed child process must not survive loss of its owning Shadows runtime
indefinitely. `process/` owns this OS-level guarantee for the complete managed
process tree, including grandchildren.

The contract is capability-based rather than tied to one signal or API:

```text
Windows:
  Job Object kill-on-owner-close semantics, with breakaway prevented,
  or an equivalent mechanism proven by tests

Linux:
  parent-death/supervisor containment plus process-tree cleanup,
  or an equivalent mechanism proven by tests
```

A Unix process group alone is not proof that descendants die when the daemon
crashes. The implementation plan must verify what `process-wrap` provides and
add platform-specific support where it does not satisfy this invariant.

That verification has now been done, and it split: Windows satisfies the
invariant and Linux does not.

- **Windows — satisfied.** `process-wrap`'s `JobObject` creates the child
  suspended, assigns it to the job, and only then resumes it, so no child code
  runs uncontained. The job carries `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, which
  is the kill-on-owner-close half: when the daemon dies its job handle closes
  and the tree goes with it. Breakaway is not permitted, because the job never
  opts into it.
- **Linux — NOT satisfied.** `ProcessSession` is `setsid` plus `killpg`. That
  kills the tree when Shadows asks, which is §8.3's requirement, and it is what
  `crates/shadows-process/tests/containment.rs` proves. It supplies no parent-death containment
  whatsoever: kill the daemon with `SIGKILL` and the harness keeps running,
  which is precisely the case the paragraph above refuses to accept a process
  group as proof of.

> **OPEN — Linux has no parent-death containment, and nothing fails because of
> it.** The mechanism is a choice between `PR_SET_PDEATHSIG` on the direct child
> (which does not reach grandchildren), a cgroup v2 scope the whole tree lives
> in, or a `pidfd`-watching supervisor. Picking one needs to be done against a
> real Linux daemon, and this project's development and acceptance target is
> Windows, so measuring it here would be measuring the wrong thing.
>
> **Trigger:** the first time Shadows is run as a daemon on Linux by anyone,
> or the first Linux acceptance claim — whichever comes first. Until then the
> Linux job is a compile-and-portable-test gate and is not evidence for this
> invariant.
>
> **Why it does not block Milestone 0:** the milestone's vertical slice runs on
> Windows, where the invariant holds. Cancellation and shutdown both go through
> `terminate_tree`, which Linux does satisfy; only daemon *death* is uncovered
> there.

## 1.6 Persistence decision

Persistence selection is closed:

```text
SQLx 0.9
SQLite v1
PostgreSQL compatibility is a design requirement
SeaQuery is NOT included initially
```

The final delta validation compared current candidates rather than relying only on the original spike:

- SeaORM `2.0.3`
- SQLx `0.9.0`
- PostgreSQL 16
- Rust 1.96

Both passed the required PostgreSQL semantic contract. SQLx remained preferred because it required less adapter glue, had a smaller measured dependency/build footprint for this workload, and its official migrator handled concurrent PostgreSQL startup in the validation without custom locking. SeaQuery demonstrated no necessary value for the current static-query workload.

This decision does **not** imply separate storage/domain crates, generic repository traits, or a custom transaction DSL.

---
