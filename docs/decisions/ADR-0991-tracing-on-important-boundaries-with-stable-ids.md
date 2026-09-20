# Tracing on important boundaries with stable IDs

**Doc ID:** 991
**Status:** accepted
**Tags:** ids, logging, observability, tracing
**Source slug:** tracing-on-important-boundaries-with-stable-ids

---

# Decision — Observability, errors, idempotency, stable codes

**Status:** accepted · 2026-09-20

## Module-local typed errors

Each module owns its typed error:

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error { ... }
```

Modules do NOT know about HTTP, JSON-RPC, MCP wire shapes. The
domain is transport-neutral. `thiserror` provides the typed
boilerplate; we do not write custom `Display` / error boilerplate.

`anyhow` is used ONLY at binary / startup boundary (daemon startup,
config bootstrap, fatal initialization). It does NOT enter domain
APIs, persisted contracts, or wire envelopes.

## AppFailure — public-safe classification only

```rust
struct AppFailure {
    code: ErrorCode,
    class: FailureClass,
    retry: RetryClass,
    public_details: PublicDetails,   // typed enum
}
```

`AppFailure` is **public-safe**. It contains only code + class + retry
+ typed details. It is safe to map to HTTP, MCP, CLI diagnostic.

It does NOT carry the underlying source error. The source error
might contain paths, provider payloads, internal implementation
details — none of which should reach the wire by accident.

## FailureReport — internal only

```rust
struct FailureReport {
    public: AppFailure,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl FailureReport {
    fn into_log_fields(self) -> impl IntoIterator<Item = ...>;
}
```

`FailureReport` is internal. Its `source` field never enters a wire
envelope. It exists for server logs, tracing, and debug builds.

## ErrorCode registry — single place, stable identifiers

```rust
// error/codes.rs (single registry)
pub enum ErrorCode {
    WorkflowInvalidTransition,
    WorkflowValidationFailed,
    WorkflowFrozenImmutable,
    AgentAuthFailed,
    AgentUnsupportedProfile,
    AgentRateLimited,
    ProcessSpawnFailed,
    ProcessTerminated,
    StorageUnavailable,
    StorageMigrationFailed,
    CommandConflict,
    IdempotencyKeyRequired,
    InvalidCommand,
    // ... no string literals invented at call sites
}
```

Rules:

- `code` does not change when the human message changes.
- Clients depend on `code`, not on message.
- No module invents a string code at a call site. All codes come
  from the registry.
- Removing or changing the meaning of an existing code is a
  **protocol compatibility change** and must be called out in the
  spec.

## Mapping layers (no leak into domain)

```rust
protocol/error_map.rs      → HTTP response
mcp/error_map.rs           → MCP error
cli/diagnostics.rs         → human-facing diagnostic (using miette)
tracing                    → structured server log
```

Each transport layer reads `AppFailure` and renders its own shape.
Domain does not know any of them.

## Expected negative outcome ≠ system error

`Blocked` and `Rejected` are expected domain outcomes, not wire/system
errors. Invalid transitions remain command/domain failures, and upstream
rate limiting remains an agent/infrastructure failure with explicit retry
classification. Long-running domain outcomes are returned inside `Operation`:

```rust
enum OperationOutcome {
    Success   { result_ref },
    Blocked   { reason, missing_capability, alternatives },
    Rejected { reason },
}

enum OperationStatus {
    Pending,
    Running,
    Completed { outcome: OperationOutcome },  // success or structured refusal
    Failed    { stage, reason }, // actual failure
    Cancelled,
    Interrupted { reason, runtime_instance },
}
```

The wire returns:

```json
{
  "operation_id": "...",
  "status": "completed",
  "outcome": {
    "kind": "blocked",
    "reason": "Codex adapter cannot prove repository-relative write authority",
    "missing_capability": "RepositoryWrite",
    "alternatives": ["Use ClaudeCodeHarness for this role"]
  }
}
```

The client reads this from `GET /operations/:id`, SSE, or reconnect
snapshot. It is NOT a synchronous HTTP response failure.

## HTTP status mapping (synchronous commands only)

| Case | Status |
|---|---|
| Long-running command accepted | **202 + operation_id** |
| Invalid command, cannot even start | 400 |
| Conflict in synchronous command (cancel-completed, start-frozen, update-frozen) | 409 |
| Shadows client auth failure | 401 / 403 |
| Upstream agent auth failure (sync) | 502 / 503 |
| Infrastructure failure (sync) | 500 / 503 |

`POST /operations` returns `202 Accepted` with `operation_id`. The
client polls / streams. Synchronous command failures map to HTTP.
Operation outcomes live inside `Operation`. Blocked / Rejected is
NOT an HTTP status after the operation has started.

## Agent auth failure vs Shadows auth failure

Separate them. A `Claude API` auth failure is NOT a Shadows auth
failure.

```text
SHADOWS client auth failure  → HTTP 401 / 403
Upstream agent auth failure  → ErrorCode::AgentAuthFailed
                              → HTTP 502 / 503 if synchronous
                              → Operation::Failed if during a run
```

Clients depend on `ErrorCode`, not just HTTP status.

## Idempotency — durable atomic boundary

`CommandId` dedup must live in the SAME storage transaction as the state
mutation. Command identity includes principal, command scope, command ID,
command kind, command schema version, and a deterministic fingerprint of
the normalized typed request.

```rust
// Server side
async fn execute_mutating_command(
    storage: &Storage,
    command_id: CommandId,
    principal: Principal,
    command: Command,
) -> Result<OperationId, AppFailure> {
    storage.transaction(|tx| {
        // 1. Dedup check inside the same TX
        match tx.commands.find(command_id, principal.scope())? {
            Some(record) if record.matches(kind, schema, fingerprint) => {
                return Ok(record.outcome)
            }
            Some(_) => return Err(CommandConflict),
            None => {}
        }

        // 2. Perform state mutation
        let operation_id = tx.operations.create_pending(...)?;

        // 3. Append durable event
        tx.events.journal.append(CommandAccepted { command_id, operation_id })?;

        // 4. Record command → operation mapping
        tx.commands.insert(
            command_id,
            principal.scope(),
            kind,
            schema,
            fingerprint,
            operation_id,
        )?;

        Ok(operation_id)
    })
}
```

Scope: `(principal, command scope, command_id)`. Same key plus the same
kind/schema/fingerprint replays; a mismatch returns `CommandConflict`.
The fingerprint is not computed from arbitrary raw JSON bytes.

A retry of the same HTTP / MCP call → same `operation_id` from the
table, not a new ExecutionRun. No double execution.

Mutating commands:

- `planner.start`
- `execution.start`
- `workflow.propose`
- `decision.propose`
- `MCP mutation tools`

## Crash recovery with runtime instance id

```rust
struct RuntimeInstance {
    id: RuntimeInstanceId,
    started_at: Timestamp,
}
```

Every daemon has a `RuntimeInstanceId`. Every Operation records the
`RuntimeInstanceId` of the daemon that created it. On startup:

```text
Pending/Running at restart
   │
   ▼
reconcile:
   for op in operations(status in Pending/Running, runtime != current):
     TX:
       exact CAS on id + expected status + runtime_instance_id
       update op.status = Interrupted {
         reason:          RestartReconciliation,
         interrupted_at:  now,
         runtime_instance: previous_runtime_instance_id,
       }
       journal.append(OperationInterrupted { ... })
     COMMIT
```

No automatic retry from reconciliation. Workflow / Gate decides:

- retry (create new Operation)
- cancel (explicit policy)
- manual (human decision)

## Tracing pattern

```rust
#[instrument(skip_all, fields(operation_id = %op.id, thread_id = %op.thread_id))]
async fn handle(op: Operation) -> Result<...> {
    // ... at error boundary:
    tracing::error!(
        error.code  = %failure.public.code,
        error.class = ?failure.public.class,
        retry.class = ?failure.public.retry,
        "operation failed"
    );
}
```

We do NOT log:

- secrets
- raw provider payloads (may contain credentials)
- internal stack traces inside wire envelopes (kept in server logs
  only)

## Related

- `thiserror` for module errors
- `anyhow` only at binary / startup boundary
- `axum` + `tower` for HTTP
- `rmcp` for MCP
- `tokio::process` + `process-wrap` for process management
- evaluate `secrecy` + `zeroize` for secret wrapper
- evaluate `miette` for CLI diagnostics
- evaluate retry library vs `tokio::time` + policy

Related: [[library-first-with-selective-custom-implementation]], [[process-owns-child-process-primitive-with-processspec]], [[operation-lifecycle-with-two-phase-spawn]]
