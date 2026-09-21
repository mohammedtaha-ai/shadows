# SQLite WAL Concurrency Validation

**Date:** 2026-09-21
**Status:** Complete. Both questions answered; probe code is throwaway.
**Probe code:** `sandbox/wal-validation/` — throwaway, not a workspace member.
**Raw output:** `sandbox/wal-validation/results.txt`

## Questions

Two OPEN blocks in the spec pointed at this experiment.

**Q1 — writer strategy** (spec §6.23). With file-backed SQLite in WAL mode and
several concurrent writers running read-then-write transactions through SQLx,
does the default deferred `BEGIN` produce lock-upgrade failures that
`busy_timeout` cannot rescue? Does `BEGIN IMMEDIATE` fix it? Is some other
serialization needed instead?

**Q2 — `durable_seq` ordering** (spec §6.18). `durable_seq` is
`INTEGER PRIMARY KEY AUTOINCREMENT`, which assigns at INSERT, not at COMMIT. The
resync contract (§2.10) requires durable replay with a no-gap handoff to live.
Can a reader observe sequence N+1 as visible while N is not, which would let a
cursor skip an event permanently?

## Environment

| Component | Version |
|---|---|
| rustc / cargo | 1.96.0 |
| sqlx | 0.9 (sqlite) |
| OS | Windows 11 Pro 26200 |
| journal mode | WAL, file-backed |
| synchronous | NORMAL |

Every writer transaction reads current state, then inserts one entity row **and**
one `durable_event` row, then commits. That read-then-write shape is what makes
Q1 meaningful; a write-only transaction cannot produce a lock upgrade.

Scenarios with "think time" sleep a random 0–3 ms between the read and the
write, widening the window in which another writer can take the write lock.

## Results

| # | Mode | Writers | `busy_timeout` | Readers | Succeeded | Failed | Elapsed |
|---|---|---|---|---|---|---|---|
| A | deferred, no think time | 8 | 5000 | 0 | 322 / 1200 | 878 | 175 ms |
| B | deferred | 8 | 5000 | 0 | 120 / 1200 | 1080 | 2.36 s |
| C | deferred | 16 | 5000 | 0 | 85 / 1600 | 1515 | 1.57 s |
| D | **IMMEDIATE** | 16 | 5000 | 0 | **1575 / 1600** | 25 | 25.01 s |
| E | IMMEDIATE | 16 | **0** | 0 | 34 / 1600 | 1566 | 497 ms |
| F | deferred | 8 | **0** | 0 | 112 / 1200 | 1088 | 2.37 s |
| G | IMMEDIATE | 8 | 5000 | 4 | 1192 / 1200 | 8 | 17.77 s |
| H | deferred | 8 | 5000 | 4 | 124 / 1200 | 1076 | 2.36 s |
| I | IMMEDIATE | 32 | 5000 | 4 | 1865 / 1920 | 55 | 22.65 s |
| J | deferred | 32 | 5000 | 4 | 62 / 1920 | 1858 | 987 ms |
| K | **single write connection** | 16 | 5000 | 4 | **1600 / 1600** | **0** | 18.46 s |
| L | **single write connection** | 32 | 5000 | 4 | **1920 / 1920** | **0** | 20.84 s |

Error codes, exactly as SQLite returned them:

```text
code 5    SQLITE_BUSY            all scenarios that failed
code 517  SQLITE_BUSY_SNAPSHOT   scenarios A B C F H J only
```

`SQLITE_BUSY_SNAPSHOT` is the lock-upgrade failure. **It appears in every
deferred scenario and in no other scenario.** That is the hypothesised failure
mode, observed directly rather than inferred.

## Q1 finding — deferred `BEGIN` fails, and `busy_timeout` does not rescue it

Deferred mode succeeded on 3–27 % of transactions. The decisive comparison is
**B against F**: identical except `busy_timeout` 5000 ms versus 0 ms.

```text
B  deferred, busy_timeout = 5000    120 / 1200 succeeded
F  deferred, busy_timeout = 0       112 / 1200 succeeded
```

Five seconds of busy-wait tolerance bought eight transactions out of 1200. A
busy handler cannot wait on a lock upgrade, because the transaction already
holds a read snapshot that would have to be abandoned. This is not a tuning
problem, and no `busy_timeout` value fixes it.

**`BEGIN IMMEDIATE` alone is also not enough.** Compare D against E, identical
except the timeout:

```text
D  IMMEDIATE, busy_timeout = 5000   1575 / 1600 succeeded
E  IMMEDIATE, busy_timeout = 0        34 / 1600 succeeded
```

`IMMEDIATE` moves contention to `BEGIN`, where a busy handler *can* wait — so the
two must be used together. Either alone fails.

**Serializing writes through a single write connection beat both, on reliability
and on throughput.**

```text
K  single write connection, 16 writers   1600 / 1600   86.7 txn/s
L  single write connection, 32 writers   1920 / 1920   92.1 txn/s
D  IMMEDIATE, 16 writers                 1575 / 1600   63.0 txn/s
I  IMMEDIATE, 32 writers                 1865 / 1920   82.3 txn/s
G  IMMEDIATE, 8 writers + readers        1192 / 1200   67.1 txn/s
```

Zero failures instead of 8–55, and faster. The reason is that SQLite's busy
handler resolves contention by sleeping and retrying, which throws away work,
while an application-level write serialization queues callers and wastes none.
The residual 8–55 failures under `IMMEDIATE` also matter beyond the rate: each
one is a transaction the application would have to detect and retry, so choosing
`IMMEDIATE` alone means writing retry logic that the single-connection approach
does not need.

### Readers are never blocked

Scenarios G, I, K, L ran 4 concurrent readers continuously while writers worked:

```text
reader errors: 0 in every scenario
reader polls:  21,798 total across the five scenarios that ran readers
```

WAL delivers what it promises here: readers do not block on writers and writers
do not block on readers.

### Recommendation for Q1

`storage/` serializes all write transactions through **one write connection**,
and opens every write transaction with **`BEGIN IMMEDIATE`**, with
`busy_timeout` set as a backstop.

The single write connection is what earns the zero failure rate. `IMMEDIATE`
costs nothing when uncontended and keeps the guarantee from depending on there
being exactly one write connection forever — migrations, a maintenance task, or
a future backend could introduce a second one, and `IMMEDIATE` is what stops
that from silently reintroducing `SQLITE_BUSY_SNAPSHOT`. Reads use a separate
pool and are unaffected.

## Q2 finding — no visibility inversion was observed, and none is structurally possible on SQLite

The probe measured this two ways, and **the two measurements disagreed**. Only
one of them is trustworthy, and the reason matters.

**The trustworthy measurement.** Four concurrent readers repeatedly select all
sequence numbers and record any *newly appearing* sequence lower than one they
had already seen — exactly the event that would let a cursor skip a row
permanently.

```text
visibility inversions observed: 0
across:                         21,798 reader polls, 5 scenarios
including:                      32 concurrent writers under high contention
```

**The untrustworthy measurement.** The probe also compared "assignment order"
against "commit order" and reported them unequal in scenarios A, D, G, and I.
That result is a measurement artifact and should be disregarded. Reading the
probe source, the commit-order counter is incremented after the transaction's
result has been matched and the success branch entered — not at `COMMIT`. Tokio
can deschedule a task in that window, so a transaction that committed first can
record its counter second. The pattern in the results confirms this: the
discrepancy appears only in scenarios with many concurrent in-flight tasks, and
never in K or L, where a single write connection keeps one task in the write
section at a time.

**Why SQLite cannot invert here.** The empirical zero matches the mechanism.
SQLite permits one write transaction at a time. The sequence number is assigned
by the INSERT, which can only execute while the transaction holds the write
lock, and the commit happens before that lock is released. A lower sequence is
therefore always committed before a higher one, and a reader cannot see N+1
before N.

### Rollback behaviour

A direct single-connection probe inserted and committed, inserted and rolled
back, then inserted and committed again:

```text
seq of committed row      = 1
seq of rolled-back row    = 2
seq of next committed row = 2      <- the rolled-back number was reused
rows in table             = 2
resulting gap             = 0
```

**A rolled-back transaction's sequence number is reused.** `AUTOINCREMENT`
tracks its high-water mark in the ordinary `sqlite_sequence` table, whose update
is itself part of the transaction and is rolled back with it. The probe's inline
comment asserting that `AUTOINCREMENT` should not reuse is wrong: that guarantee
concerns rows that were *deleted*, not transactions that never committed.

This is the convenient outcome for §2.10 — rollbacks leave no holes for a
replaying cursor to trip over. All twelve scenarios also reported
`max_seq - row_count = 0`, so no gap appeared anywhere.

### Recommendation for Q2

Keep `durable_seq` as `INTEGER PRIMARY KEY AUTOINCREMENT` for the SQLite
backend. The §2.10 no-gap handoff is sound on SQLite.

**This does not transfer to PostgreSQL.** A PostgreSQL sequence is
non-transactional and is assigned outside any commit ordering, so two
transactions can take sequence values in one order and commit in the other, and
a reader *can* observe a higher value before a lower one. Since §1.6 makes
PostgreSQL compatibility a design requirement, the PostgreSQL adapter must
either assign the ordering key inside the commit-ordered section or use a
different ordering mechanism entirely. That is a separate design question this
experiment does not answer and does not attempt to.

## What this does not establish

- **Linux.** Everything here ran on Windows 11. SQLite's locking primitives
  differ by platform. Nothing in this report is evidence about Linux.
- **Rollback gaps under contention.** The rollback probe ran single-connection
  with no contention. In the concurrent scenarios, failures occurred at `BEGIN
  IMMEDIATE` or at the first INSERT — before any sequence was assigned — so a
  rollback *after* a successful INSERT was never exercised concurrently. The
  `max_seq - row_count = 0` results are therefore weaker evidence than they look.
- **Inversion windows shorter than 500 µs.** Readers polled at 500 µs. A window
  briefer than that would have been missed. The structural argument above is what
  carries this conclusion; the measurement corroborates it rather than proving it
  alone.
- **Durability tuning.** `synchronous = NORMAL` throughout. No comparison against
  `FULL` was made, and no crash-durability testing was performed.
- **Realistic transaction sizes.** Each transaction wrote two small rows. Larger
  writes hold the write lock longer and would change the throughput figures,
  though not the ordering of the three strategies.

## Note on the probe itself

The first run of this probe hung after completing all twelve scenarios. The
cause was in the rollback probe, not in the measurement: a pool with
`max_connections(1)` had its only connection checked out into a live variable,
and `pool.close().await` waits for every connection to return, so it waited on a
connection that could not be returned until the function exited. Fixed by
dropping the connection before closing. The scenario results above come from a
clean re-run after that fix.
