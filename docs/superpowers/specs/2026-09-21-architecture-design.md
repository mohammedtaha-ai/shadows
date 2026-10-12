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

**The daemon does not serve or embed the client** (decided 2026-09-23, superseding the single-page `include_str!` recommendation in `docs/evidence/harness/SERVE_STREAM_SPIKE.md` at `92e6dae` Finding 1). The backend stays on the machine that owns the projects, the processes, and the harness; clients reach it over the protocol, the way a hosted web app reaches a locally running agent. The Web client is the first client, a desktop client is a later one, and a hosted Web client reaching a remote daemon is a later deployment of the same one. Consequences:

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
(`docs/evidence/harness/SERVE_STREAM_SPIKE.md` at `92e6dae`) is the contract of one
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

## 1.7 Workspace authority and deployment

Decided 2026-10-10 for the personal/team boundary. This section owns workspace
authority, runtime placement, synchronization semantics and execution ownership.
It specifies future integration; it does not claim a server runtime, connector,
PostgreSQL product store or remote worker is implemented.
The [shared cloud architecture](../../../../shadows-backend/docs/architecture.md)
owns accounts, cloud delivery, NYLRA data and sharing lifecycle. §24 owns approval.

### One authority per workspace

| Workspace | Authoritative runtime | Authoritative data | Disconnected use |
| --- | --- | --- | --- |
| Personal | Shadows on the owning computer | Local SQLite plus local project files | Full delivered local functionality |
| Team | Server deployment of the same Rust application rules | Server PostgreSQL plus managed artifacts | Cached reads, draft proposals and explicitly authorized bounded work |

An account is not a workspace. Signing in, installing NYLRA, or linking a project
does not move workspace ownership. A person can use a personal workspace while
participating in several team workspaces. Team work on their laptop remains team
work; its checkout is not a new personal authority. Private personal projects,
conversations and preferences are not made visible to a team by membership.

The owning runtime alone accepts changes to its plans, agreements, approvals and
execution state. Clients submit commands against expected revisions. Another
device's cache is never a second writable master. Conflicting proposals require
review and resubmission; last-write-wins cannot decide domain conflicts.

### One implementation of project rules

`shadows-core` remains the owner of project rules in both deployments. Local
SQLite and future server PostgreSQL stores live behind its service boundaries.
Add storage variations when implementing the server; do not add unused generic
repositories now. The same semantic contract suite must exercise both stores.

The TypeScript application in `shadows-backend` owns cloud accounts, membership,
devices, delivery and NYLRA. It calls the Rust runtime through an authenticated
service protocol. It neither reimplements plan validation nor writes Rust-owned
tables. Rust and TypeScript stores have separate ownership and database access
roles even if hosted by the same PostgreSQL installation. There is no distributed
transaction spanning them: durable requests, deduplication and recorded outcomes
bridge that boundary.

Cloud identity/membership establishes who may access a workspace. The Rust owner
enforces the authenticated scope and domain authorization on every operation;
an arbitrary header or model-supplied actor is not authentication. The detailed
identity protocol closes the remote-access OPEN before any external mutation.

### What synchronization means

1. **NYLRA continuity:** its cloud-owned topics and conversations are available on
   desktop and phone. This does not synchronize Shadows databases.
2. **Workspace views:** selected versioned snapshots plus ordered changes build
   read caches. Initial snapshot and event position must share a consistent
   boundary. A lost/expired cursor forces resnapshot; reconnect must not skip a
   change between snapshot and stream. Commands carry stable identity and expected
   versions; the owner answers replay or conflict using §2's command rules.
3. **Code and artifacts:** Git commits/branches carry source changes; explicit
   manifests and digests identify required non-Git inputs and result artifacts.
   Dirty files, ignored files and large artifacts require explicit inclusion.
   A plan sync neither uploads a repository nor proves files are available.
4. **Backup:** an explicit recoverable snapshot, with its own retention and restore
   policy. A cloud context cache is not a backup. Restoring a copy does not authorize
   two runtimes to write the same workspace identity.
5. **Move ownership:** an explicit later migration/export-import operation with
   quiescence, verified data, cutover and fencing of the former owner. Linking a
   second computer never performs migration or automatic failover.

Personal workspace access from a second device routes to its owning computer.
When that computer is offline, the device can read permitted cached evidence or
queue proposals, but cannot start its Planner or claim acceptance. A separate
personal workspace on another computer has a separate identity.

### NYLRA on the owning computer

Decided with Mohammed on 2026-10-12: the Shadows–NYLRA integration starts with
NYLRA desktop on the computer that owns the workspace. It reads project context
from this daemon's local API; the cloud never reads the daemon on this path. The
backend's [same-machine context path](../../../../shadows-backend/docs/architecture.md#same-machine-context-path)
owns the cloud half.

Shadows owns two things on this path. **The sharing switch:** a project is
readable by NYLRA only after the person turns it on for that project, and
turning it off refuses the next read. **The projection:** one read returns the
bounded `nylra-context/1` items, each with its version, and the project's
revision. Nothing outside the projection is reachable through it, and what it
returns grants nothing: approvals and other writes keep their own routes and
§24's rules. Both are methods of one service, with their lines in its contract.

The connector, for the phone and other computers, carries the same projection
when it is built; it is not needed for this slice.

> **OPEN — who may call the projection.** Today the daemon knows a caller only
> by its browser `Origin`; any local process can call it, as it can call every
> route. That is accepted for a read the person switched on, and recorded as a
> limit. **Closes when** the connector slice adds an authenticated caller, or
> earlier if a local write is added to this path.

### Execution is assigned, not synchronized

A team runtime may assign a bounded attempt to an enrolled local worker or, later,
a cloud worker. The runtime retains plan and acceptance authority; the worker owns
only its assigned process tree and isolated checkout. An assignment binds the
workspace, task version, input digests, capabilities, expiry and attempt identity.
One writable attempt has one active assignment generation. Reassignment fences
older results; evidence cannot be accepted under a different generation.

Offline workers start no new assignments or privileged external effects. They may
finish only work explicitly permitted by an unexpired offline grant, then retain
results for revalidation. Without such a grant they stop safely on loss of lease.
Revocation cannot instantly stop an unreachable machine: the UI shows stop pending
until acknowledged or expiry, and the owner rejects stale results. Reassignment
must not duplicate non-idempotent external effects; those wait for reconciliation.

Moving execution stops/checkpoints the old attempt and starts a new attempt from
verified inputs. It does not move a live process, CLI login or opaque harness
session. A resumed conversation is not proof that execution moved.

> **OPEN — server and worker implementation.** Close before Stage 9/server or
> remote-worker implementation by specifying and testing tenant isolation, the
> authenticated identity/revocation protocol, PostgreSQL semantic parity,
> snapshot/replay recovery, lease expiry/fencing and artifact transfer. Linux
> server acceptance also closes §1.5's containment OPEN. This does not block the
> full local release, which remains the first delivery objective.

---
