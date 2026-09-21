# THROWAWAY PROBE — do not build on this

Measurement code, not product code. It exists only to have answered two
questions, and it is scheduled for deletion.

Findings: [`docs/evidence/persistence/WAL_VALIDATION.md`](../../docs/evidence/persistence/WAL_VALIDATION.md)
Raw output of the accepted run: [`results.txt`](./results.txt)

## What it measures

Twelve scenarios against file-backed SQLite in WAL mode, each running writers
whose transactions read current state and then write — the shape that forces a
lock upgrade and makes the question meaningful.

- deferred `BEGIN` vs `BEGIN IMMEDIATE` vs one serialized write connection
- `busy_timeout` at 5000 ms and at 0, to separate upgrade failure from timeout
  tolerance
- 8, 16, and 32 concurrent writers
- concurrent readers polling for visibility inversions in `durable_seq`
- a direct, contention-free probe of what a rollback does to the sequence

## Running it

```
cargo run --release
```

It writes its databases to `%TEMP%\wal-validation-probe` and leaves them for
inspection. Delete that directory between runs; the program also removes each
scenario file before using it.

## Known correction

The first version deadlocked after all twelve scenarios: the rollback probe held
the only connection of a `max_connections(1)` pool in a live binding and then
awaited `pool.close()`, which waits for every connection to be returned. Fixed by
dropping the connection first.

One measurement in this probe is **not trustworthy and its output should be
ignored**: the `assign_order == commit_order` comparison increments its
commit-order counter after the transaction result is matched, not at `COMMIT`, so
tokio scheduling can reorder the records. The reader visibility-inversion probe
is the measurement that answers that question, and it is correct. The findings
report explains both.
