# Architecture tests are mechanical defense-in-depth

**Doc ID:** 990
**Status:** accepted
**Tags:** architecture, defense-in-depth, guard, testing
**Source slug:** architecture-tests-are-mechanical-defense-in-depth

---

# Decision — Architecture tests are mechanical defense-in-depth

**Status:** accepted · 2026-09-20

## Rule

`tests/architecture_tests.rs` walks the source tree and fails on
mechanical violations.

### Current mechanical rules

```
sqlx imports outside storage/                               → fail
axum outside protocol/                                      → fail
tokio::process outside process/                             → fail   # STRICT
process-wrap outside process/                               → fail
AgentInvocation outside agent/                              → fail
ProcessSpec outside agent/ | verification/ | process/      → fail
cli importing storage internals                             → fail
cli importing events::bus directly                          → fail
mcp importing storage drivers directly                      → fail
```

The `process/` rule is strict (no carve-outs). The storage rules catch
observable dependency/import violations only. See
[[storage-ports-are-capability-specific-not-a-generic-database-trait]].

## What they are NOT

These tests are not the primary architectural proof. They do not prove
semantic portability and intentionally avoid a growing keyword blacklist
for `rowid`, FTS5, PRAGMA, backend functions, or locking assumptions.
Storage containment, contract/parity tests, and code review cover those
semantic risks. Architecture tests are defense-in-depth, not the main guard.

## Primary defense: visibility itself

Where possible, prefer:

```rust
pub(crate)          // visible inside the crate only
private modules     // no pub at all
small public surface
```

so that the architecture tests are a backstop, not the only thing
standing.

Related: [[process-owns-child-process-primitive-with-processspec]], [[agent-seam-role-agentharnessstart-agentrunhandle]], [[storage-ports-are-capability-specific-not-a-generic-database-trait]]
