# Section 1 — Architecture Overview

`shadows` starts as one Rust crate exposing a library and a binary. The product client is an independent browser application that communicates only through the local protocol.

The binary has two primary modes:

```text
shadows serve    # long-running local daemon/runtime
shadows ...      # CLI client
```

`shadows serve` exposes the local API and product Web client at one local address. It prints the address and never opens a browser automatically; the user chooses which browser to use.

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

## 1.2 Cross-cutting / infrastructure modules

```text
process/
mcp/
config/
secrets/
error/
tracing/
```

These are still normal modules in the same crate; “cross-cutting” describes responsibility, not a separate architectural layer.

## 1.3 Ownership rules

| Module | Sole owner of |
|---|---|
| `agent/` | Agent-harness abstraction and harness-specific translation from `AgentInvocation` to `ProcessSpec` |
| `process/` | OS process primitives: `tokio::process`, process trees/groups/job objects, `ProcessSpec`, `ProcessHandle` |
| `storage/` | SQLx, SQLite schema/query code, migrations, future backend adapters |
| `protocol/` | HTTP/SSE transport |
| `mcp/` | MCP adapter and MCP request/response translation |
| `secrets/` | `SecretRef` resolution; secret values are resolved only at spawn |
| `scheduler/` | Pure scheduling decision logic; no I/O |
| `events/` | Durable-event vocabulary, cursor semantics, transient live-event abstraction |

### Mechanical boundary examples

- `agent/` may construct `ProcessSpec`, but only `process/` may call `tokio::process`.
- `verification/` may construct a deterministic process check, but process spawning still goes through `process/`.
- `storage/` may import SQLx; application/domain modules may not.
- `protocol/` and `mcp/` call application/domain operations; they do not access SQL directly.

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
