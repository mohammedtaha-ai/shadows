# Delta Validation Result

## Versions tested

- SeaORM: `2.0.3` (`rust-version = 1.94.0`)
- SQLx: `0.9.0` (`rust-version = 1.94.0`)
- SeaQuery checked: `1.0.2`; deliberately not used by the SQLx candidate
- PostgreSQL: `16.15` (`postgres:16-alpine`)
- Rust: `rustc 1.96.0`, Cargo `1.96.0`, Windows x86_64 MSVC

## Evidence

| Criterion | SeaORM current | SQLx current | Notes |
|---|---|---|---|
| Real Postgres | PASS | PASS | Both ran the same atomic command, scoped cursor ordering, typed roundtrip, rollback, and v1-to-latest probes against PostgreSQL. |
| Atomic TX ergonomics | PASS, more glue | PASS, simpler | Both keep transaction types inside storage and need no public `Vec<MultiOp>`/DSL. Probe implementation: 335 SeaORM lines versus 263 SQLx lines plus 36 migration-SQL lines. |
| Official migrations | PARTIAL | PASS | Fresh and v1 fixture to latest pass for both. With four concurrent startups SeaORM produced 1 success/3 PostgreSQL `23505` failures; SQLx produced 4/4 successes using its built-in migration lock. |
| Domain isolation | PASS | PASS | Both consume the same persistence-free `shadows-domain`; no ORM/driver type crosses the adapter boundary. |
| Boilerplate | Higher | Lower | SeaORM raw statements require `Statement` construction and `QueryResult::try_get`; SQLx binds and row extraction are direct. |
| Compile/dependency cost | 89.18s cold, 0.71s hot; 232 packages; 1111 MiB target | 64.41s cold, 0.50s hot; 194 packages; 983 MiB target | Approximate local debug/test measurements with separate empty target directories and a warm crate-download cache. |
| SeaQuery needed? | N/A | No | The old SQLx spike declared SeaQuery but had no source usage; it contained 61 direct SQLx query call sites. The delta contract also needed no dynamic query builder. |

## Differences from the previous MiniMax spike

- SeaORM PostgreSQL is now executed end-to-end; it is no longer inferred from its SQLx internals.
- The old `Vec<MultiOp>` workaround is not required. A storage-owned use-case method can open a concrete transaction without leaking it to domain/application contracts.
- Both current releases require Rust 1.94; the old spike declared MSRV 1.87.
- SQLx 0.9 splits the old `runtime-tokio-rustls` feature into runtime and TLS features.
- SeaORM 2 raw SQL uses explicit `execute_raw` / `query_one_raw` / `query_all_raw` methods.
- SQLx official migrations remove the old custom-runner/manual-lock concern. SeaORM official migrations still need external startup serialization or a PostgreSQL advisory lock under concurrent startup.
- SeaQuery adds no demonstrated value for the current static-query workload.

## Recommendation

**`SQLx`**

1. It passed the complete PostgreSQL delta contract.
2. Its official migrator passed four concurrent startups without custom locking.
3. Its transaction stays inside the storage adapter; no SQLx type leaks into the domain.
4. The atomic use case is direct and does not require a custom transaction DSL.
5. It used less adapter glue for the same behavior.
6. Its measured cold build and dependency footprint were smaller on this machine.
7. SeaQuery is unnecessary until a real dynamic-query use case appears.
8. SeaORM 2 is viable, but it did not produce an offsetting benefit for this workload and adds migration-startup work.

## Architecture consequences

- Use SQLx's official `Migrator`/`migrate!`; do not build a migration framework.
- Keep SQLx types and SQL inside the storage backend; keep domain types unchanged.
- Start with SQLx alone. Add SeaQuery only if a concrete dynamic-composition query proves the need.
- Current SQLx 0.9 implies a Rust 1.94 floor; decide that toolchain floor explicitly before production adoption.
- This validation does not decide crate splits, public storage ports, SQLite runtime tuning, or whether cursor scope is global/project/thread. It proves only strict monotonic ordering within a supplied scope.
