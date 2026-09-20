# VerificationCheck uses CheckPhase

**Doc ID:** 988
**Status:** accepted
**Tags:** architecture, checks, phases, verification
**Source slug:** verificationcheck-uses-checkphase

---

# Decision — VerificationCheck uses CheckPhase

**Status:** accepted · 2026-09-20

## Rule

Checks are timing / requirement classifications on a
`VerificationCheck`, not types of `Gate`.

```rust
struct VerificationCheck {
    kind: CheckKind,
    phase: CheckPhase,
    // ...
}

enum CheckPhase {
    Task,                    // must pass now
    Regression,              // must not introduce regressions
    Deferred { gate_id },    // runs at gate_id
    Final,                   // must pass before workflow completion
}
```

## Roles

- Workflow / Gate decides when checks become runnable.
- Verifier receives checks, runs them, returns evidence / verdict.
- Verifier does not decide whether a check is `Task` or `Final`.

## Why

The four categories (Task / Regression / Deferred / Final) are timing
classifications. Putting them on `Gate` makes `Gate` mean too many
things. Putting them on `Check` keeps the categorization close to
what it classifies.

Related: [[planner-thinks-executor-implements]]
