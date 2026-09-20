# Reviewer is separate from Verifier

**Doc ID:** 1006
**Status:** active
**Tags:** architecture, reviewer, separation, verifier
**Source slug:** reviewer-is-separate-from-verifier

---

# Decision — Reviewer is separate from Verifier; no ReviewCheck kind in v1

**Status:** active · 2026-09-20

## Rule

`Verifier` (deterministic) and `Reviewer` (AI judgement) are
different things. They must NOT be conflated.

```text
verification/  → deterministic checks (build, test, contract, baseline)
                  NO AI / NO Reviewer logic

review/        → (future) AI Reviewer operation, separate role
```

In v1, Reviewer is deferred. It does not exist yet.

## VerificationCheck.kind in v1

```rust
enum CheckKind {
    BuildCheck,
    TestCheck,
    ContractCheck,
    BaselineCheck,
    // NO ReviewCheck in v1
}
```

We delete `ReviewCheck` entirely from `VerificationCheck.kind` in
v1. It does NOT remain as a "deferred runner". A future Reviewer is
its own `Operation` (kind = ReviewRun) with its own lifecycle and
its own evidence — it is not a verifier call.

## Why this is not "the same thing"

These have different lifecycles, evidence shapes, and authority:

- deterministic verifier → reproducible machine verdict
- AI reviewer → judgement a human must read and reply to

Shadow's W4b is exactly this reviewer and is deliberately a separate
milestone.

## What stays

- `VerificationCheck.phase` (Task / Regression / Deferred / Final)
  remains. Workflow / Gate decides when checks become runnable.
- `verifier::run(...)` does NOT know about Reviewer.

## When Reviewer lands

It will be its own Operation kind, started by `operation::start(kind=
ReviewRun, ...)`, going through `agent::start(role: Reviewer, ...)`
with its own context and permissions. The verdict it produces is
`ReviewVerdict`, not `Verifier::Verdict`.

Related: [[agent-seam-role-agentharnessstart-agentrunhandle]], [[operation-lifecycle-with-recovery]]
