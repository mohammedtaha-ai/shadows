# Events split into Durable Journal and Live Bus

**Doc ID:** 994
**Status:** accepted
**Tags:** architecture, events, journal, streaming
**Source slug:** events-split-into-durable-journal-and-live-bus

---

# Decision — Event durability: journal + bus, atomic, gap-free resync

**Status:** accepted · 2026-09-20

## Rule

`events/` is not one append-only log. It is two related concerns:

```text
              events/
                 │
        ┌────────┴────────┐
        │                 │
 Durable Journal    Live Event Bus
        │                 │
      SQLite            SSE
```

A durable event may also be published on the live bus. A transient
event never enters the journal.

## Atomicity: commit then publish

Every state mutation + its durable event commit in the SAME storage
transaction:

```text
storage.transaction {
    mutate state
    journal.append(DurableEvent)
}
    ↓ COMMIT succeeds
bus.publish(durable_event)   // best effort
```

We do NOT publish a durable event before commit. A client must not
see a truth that is not yet durable.

If bus publish fails:

- the journal is the truth
- the client catches up via `events_since(cursor)`

## subscribe_after: gap-free resync

External clients subscribe via `events::subscribe_after(cursor, scope)`
which guarantees:

```text
durable replay after cursor
+
transition to live without gap
+
deduplication by durable_seq
```

Both `protocol/` and `mcp/` use this API. They do not invent their
own algorithm.

A simple `events_since(cursor)` followed by a live subscription is
NOT enough on its own because commits can occur between replay and
subscribe. The API hides that race.

## Durable vs transient classification

| Event | Journal? | Bus? |
|---|---|---|
| `OperationStarted` / `OperationCompleted` / `OperationCancellationRequested` / `OperationCancelled` | yes | yes |
| `DecisionAccepted` / `WorkflowChanged` / `WorkflowSuperseded` | yes | yes |
| `TaskCompleted` / `GateReached` / `CheckRan` | yes | yes |
| `AgentOutputDelta` / `stdout chunk` / `progress heartbeat` | no  | yes |

## ClientDisconnected is transient

A transport disconnect (browser / CLI / WebSocket) is NOT a durable
domain event. Logging every disconnect in the journal would pollute
project truth with transport noise.

In v1:

- disconnect → transient / observability only
- operation continues
- on reconnect, the client sends its `last_durable_cursor`
- a future audit subsystem can be added separately

## External clients via protocol only

External clients (CLI, future Web, future Mobile) subscribe to the
event bus only through `protocol/`. They never reach into the bus
directly.

```text
CLI ──HTTP/SSE──▶ protocol/ ──▶ events::bus
future-Web ──HTTP/SSE──▶ protocol/ ──▶ events::bus
```

`mcp/` may subscribe to the bus for its own MCP notifications but is
also an internal adapter, not an external client.

Architecture test:

```text
cli importing events::bus directly → fail
mcp importing rusqlite directly    → fail
```

## Subscribe path summary

```text
cli:        protocol/ → events::bus.subscribe_after(cursor) → SSE
future-Web: protocol/ → events::bus.subscribe_after(cursor) → SSE
mcp:        mcp/      → events::bus.subscribe_after(cursor) → MCP notification (if supported)
replay:     storage.load(journal_events) for historical
```

Transient events between disconnect and reconnect MAY be lost by
design. Durable state has no gap.

Related: [[external-clients-via-protocol-was-archived]], [[reconnect-cursor-was-archived]], [[state-mutation-atomic-was-archived]], [[architecture-tests-are-mechanical-defense-in-depth]]
