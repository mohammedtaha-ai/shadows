# Section 3 — Errors, Dependencies, and Testing

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there — many of them point into a different file.

## 3.1 Module-local errors

Each module owns typed errors using `thiserror`.

Storage example:

```rust
pub enum storage::Error {
    TransitionConflict { /* typed state */ },
    NotFound(EntityKind),
    InvalidCursor { reason: CursorError },
    Serialization(serde_json::Error),
    Constraint(sqlx::Error),
    Database(sqlx::Error),
    Migration(sqlx::migrate::MigrateError),
}
```

No God Error inside the core.

## 3.2 AppFailure

`AppFailure` is transport-neutral and public-safe:

```rust
struct AppFailure {
    code: ErrorCode,
    class: FailureClass,
    retry: RetryClass,
    public_details: PublicDetails,
}
```

The internal causal chain is retained separately in `FailureReport` and cannot accidentally serialize.

Retry classes:

```text
Never
Immediate
Backoff
AfterReconfiguration
AfterUserAction
```

Retry classification does not itself authorize automatic retry.

## 3.3 Negative domain outcome != system error

Long-running commands normally return:

```text
HTTP 202 + operation_id
```

The operation later reaches a terminal outcome.

```rust
enum OperationOutcome {
    Success {
        result_ref: Option<EntityRef>,
    },
    Blocked {
        reason: BlockReason,
        missing_capability: Option<Capability>,
        alternatives: Vec<Alternative>,
    },
    Rejected {
        reason: RejectionReason,
    },
}
```

`Blocked` and `Rejected` are domain outcomes, not HTTP/protocol failures.

Reading a completed operation may return HTTP 200 with that outcome.

## 3.4 ErrorCode registry

Clients pattern-match on stable codes, not human text.

Representative error codes:

```text
WorkflowInvalidTransition
WorkflowFrozenImmutable
WorkflowValidationFailed

AgentAuthFailed
AgentUnsupportedProfile
AgentRateLimited

ProcessSpawnFailed
ProcessTerminated

StorageUnavailable
StorageMigrationFailed
StorageConstraintViolation

CommandConflict
IdempotencyKeyRequired

InvalidCommand
InvalidCursor

PathInvalid          -- not absolute, not UTF-8, or not a single valid name
PathNotFound
PathNotADirectory
PathAccessDenied
PathAlreadyExists
PathUnavailable      -- any other I/O failure reading the disk
```

The `Path*` codes answer the daemon's disk routes (§1) and a project's
directory (§4.2). They are refusals of a request, so each maps to a 4xx except
`PathUnavailable`, which is the daemon failing to read its own disk.

`Blocked`/`Rejected` outcome kinds do **not** appear in the error-code registry.

Changing/removing an existing public code is a protocol compatibility concern.

## 3.5 Upstream auth vs Shadows auth

Do not conflate:

```text
Shadows client authentication/authorization
```

with:

```text
upstream provider / harness authentication failure
```

The first maps to normal client auth semantics. The second is infrastructure/agent failure and is reported synchronously or through an asynchronous `Operation` failure depending on when it occurs.

## 3.6 Dependency strategy

| Need | Library | Decision |
|---|---|---|
| Typed errors | `thiserror` | USE |
| Startup/binary boundary | `anyhow` | USE only at bootstrap/top-level boundaries |
| Async runtime | `tokio` | USE |
| HTTP | `axum` | USE |
| Middleware | `tower`, `tower-http` | USE |
| Persistence | `SQLx 0.9` | USE |
| Migrations | SQLx official `Migrator` / `migrate!` | USE |
| Process runtime | `tokio::process` + `process-wrap` | USE behind `process/` |
| MCP | `rmcp` | USE |
| Serialization | `serde`, `serde_json` | USE |
| Tracing | `tracing`, `tracing-subscriber` | USE |
| CLI | `clap` | USE |
| Secrets hardening | `secrecy`, `zeroize` | EVALUATE when actual secret-lifetime benefit is demonstrated |
| Rich CLI diagnostics | `miette` | EVALUATE |
| Retry helper | library or tokio primitives | EVALUATE from concrete retry policy |
| DAG library | none initially | ADD only if it materially reduces scheduler complexity |
| Dynamic SQL builder | none initially | SeaQuery only if real dynamic composition appears |

### 3.6.1 External developer tools

`gcode` is integrated, if needed, as an optional external executable. Shadows
invokes it through its ordinary tool/process boundary and consumes a bounded,
version-checked output contract. The Shadows repository does not vendor or take
a Rust dependency on `gobby-cli` or `gcore`, and does not inherit Gobby's
PostgreSQL, FalkorDB, Qdrant, home-directory, daemon, or configuration model.

An unavailable or incompatible `gcode` disables that optional search
capability; it does not prevent the Planner from starting. `ghook` and `gwiki`
are outside the first runnable milestone. Any future hook adapter is a small
Shadows-owned protocol adapter rather than a copied Gobby dispatcher.

## 3.7 Testing strategy

These layers describe the eventual system. They activate only when the feature
they protect exists. The first runnable milestone uses focused unit tests plus
one real Windows browser/Claude Start/stream/Stop/restart acceptance path; it
does not wait for scheduler property tests, MCP compatibility, research FTS,
or mutation testing. Linux process-containment and core gates are required
before calling the runtime cross-platform, not before the first Windows debug
run.

### Layer 1 — Unit tests

Colocated under Rust source modules.

Use for:

```text
state machines
scheduler decisions
context-selection rules
parsers
scope comparisons
error mapping
```

### Layer 2 — Black-box / cross-module integration tests

Use `tests/` for:

```text
storage contracts
process behavior
protocol flows
resync
cross-module application behavior
```

### Layer 3 — Property tests

Use `proptest` where invariants matter:

```text
DAG scheduling
cursor rules
scope relations
state transitions
idempotency keys
```

### Layer 4 — Fault injection / crash recovery

Inject failures around transaction boundaries and run real daemon-process recovery tests.

Use a platform-neutral unclean-termination helper rather than assuming Unix `kill -9`.

Critical crash points include:

```text
before TX
during TX
after TX #1 / before spawn
after spawn / before TX #2
during terminal transition
```

Required cross-platform containment probes use a real hierarchy:

```text
daemon -> long-lived child -> long-lived grandchild
uncleanly terminate daemon
assert child and grandchild do not remain alive indefinitely
restart daemon
assert old-runtime Pending/Running Operation becomes Interrupted by exact CAS
```

Run this on Windows and Linux. A direct-child-only test is insufficient.

### Layer 5 — Concurrency tests

Cover:

```text
duplicate external commands
parallel scheduler decisions
active ExecutionRun uniqueness
event resync handoff
SQLite writer contention
cancellation races
cancel command committed + daemon crash before process exit
spawn succeeds + daemon crash before Running commit
```

### Layer 6 — Architecture tests

Mechanical only:

```text
SQLx import containment
process primitive containment
no persistence types in domain signatures
```

Do not use architecture scanners as a substitute for semantic tests.

### Layer 7 — Protocol compatibility

Test stable public:

```text
ErrorCode
event envelopes
cursor encoding
command envelopes
operation outcomes
```

### Layer 8 — Migration tests

Required:

```text
fresh DB -> latest
supported old fixture -> latest
failed migration safety
concurrent startup behavior
```

Reversible/down migrations are not a blanket requirement.

### Layer 9 — Real CLI acceptance

Scheduled/manual/release or harness-change gated tests against actual Claude/Codex binaries.

Assertions target observable contracts, not exact natural-language output.

Credentials are dedicated CI credentials only.

### Layer 10 — Mutation testing

Use `cargo-mutants` selectively on deterministic critical code:

```text
scheduler
idempotency
state transitions
cursor logic
scope enforcement
```

Not the entire codebase.

### CI

If Windows is a supported runtime, **Linux and Windows core suites are both required merge/PR gates**.

Additional scheduled jobs:

```text
property-heavy suites
mutation tests
stress/concurrency tests
real CLI acceptance
```

`cargo-nextest` is preferred for normal CI execution. Retries are disabled for the core correctness suite.

A requirement-test matrix may point from requirements/ADRs to executable evidence. There is no rule requiring “one test per ADR”.

---
