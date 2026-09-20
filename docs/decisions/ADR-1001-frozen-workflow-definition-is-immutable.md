# Frozen workflow definition is immutable

**Doc ID:** 1001
**Status:** accepted
**Tags:** architecture, immutability, versioning, workflow
**Source slug:** frozen-workflow-definition-is-immutable

---

# Decision — Frozen workflow is immutable; supersede replaces it

**Status:** accepted · 2026-09-20

## Rule

Once a workflow reaches `Frozen` state, its **definition** is
immutable. There is no `unfreeze`.

If the design must change, the workflow is **superseded** by a new
version:

```text
Workflow v1 (Frozen)
   ▲
   │ previous_version_id
Workflow v2 (Draft) {
    reason,
    trigger,
    author,
    previous_version_id: v1.id
}
```

Execution status does not change the definition itself.

## Supersede chain

Workflow lineage has one stored direction:

```text
new.previous_version_id = old.id
```

The old workflow does not store `superseded_by`. A successor is derived
by querying for a workflow whose `previous_version_id` equals the old
workflow id. In v1, `UNIQUE(previous_version_id)` allows at most one
successor.

- The scheduler MUST return no new ready tasks for a version with a
  derived successor.
- The dispatch transaction MUST re-check successor absence so a stale
  scheduling decision cannot dispatch after supersession.
- Existing runs either complete on the old version or are cancelled by
  an explicit policy decided at supersede time.

`unfreeze` is the typical patch pattern that shadows' history warns
against. It encourages ad-hoc edits to a definition that other
agents and verifiers were already reasoning about.

## State machine

```text
Draft ─approve──▶ Approved ─freeze──▶ Frozen ─start──▶ Running ─finish──▶ Completed
 │
 └─▶ Failed
```

A new workflow references the previous via `previous_version_id`.
Operations against the old version continue using the old definition.
They do not auto-migrate.

## Why this matters

`unfreeze` is exactly the kind of rework that compounds over years.
Each unfreeze undoes a frozen contract. We freeze definitions for
the same reason we freeze task contracts — anyone reasoning about
the system (agent, verifier, reviewer, future Web) needs that
immutability to make correct decisions.

Related: [[scheduler-is-pure-decision-engine]]
