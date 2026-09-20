# Library-first with selective custom implementation

**Doc ID:** 977
**Status:** active
**Tags:** architecture, foundation, library-policy
**Source slug:** adr-0001-library-first-with-selective-custom-implementation

---

# Decision — Library-first with selective custom implementation

**Status:** active · 2026-09-20

## Rule

If a mature library serves our need correctly, use it. Do not rebuild
existing things. Categories covered (non-exhaustive): async runtime,
HTTP server, SQLite access, process management, serialization, CLI
parsing, logging / tracing, graph algorithms, retries / backoff,
testing utilities, cryptography / hashing, config parsing.

Library selection criteria:

- quality and project maintenance
- Windows / Linux / macOS support
- clear and stable API
- good tests
- does not impose a strange architecture
- appropriate license
- acceptable performance
- no hidden global state or behavior that is hard to isolate between workers

Do not wrap every library without reason. The abstraction layer exists
only when there is a real reason: expected change, multiple
provider/harness backends, hard testability, or a boundary we must
protect.

## When we build our own

If no library serves the semantics we need, we build our own
implementation. This is normal, not a failure. The custom
implementation must be:

- small and single-responsibility
- with a clear API
- documented
- covered by unit + integration tests
- without mixing business logic with infrastructure
- extractable into its own crate later if it becomes truly reusable

## Mantra

> Use libraries where they help. Build our own where the product
> genuinely needs something different.

## ADR discipline

An ADR is created only when the choice has real alternatives, has
long-term consequences, and someone is likely to ask later "why did
we do it this way?"

A small struct shape, a function signature, choosing one well-known
library, or an implementation detail does NOT need an ADR.

We do NOT create a new ADR for every correction. Related decisions
are grouped into families. If two ADRs end up saying the same thing
in different words, the smaller is archived as superseded.

## Library-first vs abstractions

The library-first rule does not mean "no abstractions". When we
build our own we still apply the same separation-of-concerns rules:

- infrastructure does not leak into domain modules
- small modules with clear responsibilities
- replaceable through small seams

Absence of an abstraction is a problem when the same complexity
appears in two places, not when it appears in one place.

Related: [[modular-architecture-without-over-engineering-was-archived]]
