# Task 11 report — durable replay with a no-gap handoff to live

Commit: `b22d916` (branch `milestone-0/product-path`, base `e521423`).
Gate: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings`
clean, `cargo test` 53 passed / 0 failed, `cargo build` clean with
`test-support` off. No `#[allow(...)]` added anywhere.

## What I implemented

**`src/storage/sqlite/events_read.rs`** (new, 74 lines) — the replay half.
`Storage::current_cursor()` and `Storage::read_events_after(EventCursor,
&ThreadId, i64) -> Vec<StoredEvent>`. Thread-scoped, `WHERE thread_id = ? AND
seq > ?`, `ORDER BY seq` explicitly, `LIMIT ?`. `StoredEvent.operation_id` is
`Option<OperationId>`, rebuilt with `OperationId::from_stored` — no bare
`String` id crosses the boundary and no new conversion was added. It is its own
file rather than an addition to `events.rs`: writing an event is private by
cross-cutting rule 10, reading has no such hazard and no such caller, and one
file that both writes and reads the journal is two jobs.

**`src/protocol/`** (new module) split on the way in, as CLAUDE.md's accretion
rule and the brief both require:

| File | Its one job | Lines |
|---|---|---|
| `mod.rs` | wiring: `AppState`, `router()`, the `Failure` → HTTP mapping, `index` | 95 |
| `handlers.rs` | what each route does: six handlers, their request structs, the `ctx` helper | 151 |
| `sse.rs` | the durable-replay-then-live stream | 127 |

No `types.rs` and no `utils.rs`. Everything in `handlers.rs` is `pub(super)`;
`router()` is the only thing that names any of it. `sse` is `pub` because the
no-gap test calls `subscribe` directly (see below). Path parameters are the id
newtypes (`Path<ProjectId>`, `Path<ThreadId>`, `Path<OperationId>`), so the
§4.1 close holds at the transport edge too.

**`src/cli/mod.rs`** — `serve` now opens storage, starts the runtime, builds
`AppState` (the first construction of `LiveHandles` in the tree), mounts
`router(state)`, and installs the §8.5 graceful-shutdown closure the brief
specifies. It still prints exactly one address line and never opens a browser.

**`src/protocol/index.html`** — the two-line stub per ledger Ruling 1.

**Code map** — `docs/codebase/README.md` gained the one-job row for
`src/protocol/`; `docs/codebase/inventory.md` regenerated with
`UPDATE_CODEMAP=1 cargo test --test codemap` in the same commit.

## TDD evidence

### RED (before any implementation)

    $ cargo test --test resync
    error[E0433]: cannot find `protocol` in `shadows`
    error[E0432]: unresolved import `shadows::protocol`
    error[E0599]: no method named `current_cursor` found for struct `shadows::storage::Storage`
    error[E0599]: no method named `read_events_after` found for struct `shadows::storage::Storage`

Expected: `current_cursor`, `read_events_after` and the whole `protocol` module
did not exist.

### GREEN

    $ cargo test --test resync
    running 3 tests
    test a_zero_cursor_replays_the_whole_thread ... ok
    test reading_after_a_cursor_returns_the_unseen_tail_in_order ... ok
    test an_event_published_during_the_replay_survives_the_handoff ... ok
    test result: ok. 3 passed; 0 failed

    $ cargo test            # whole suite, after the codemap regeneration
    test result: ok. (53 passed; 0 failed across 16 targets)

### Each test shown to bite

A compile error is not proof that an assertion has teeth, so each of the three
was failed a second time by breaking the implementation under it and restored.

1. **`reading_after_a_cursor_...`** — `AND seq > ?` → `AND seq >= ?`:

       assertion `left == right` failed: exactly the events after the cursor
         left: 4   right: 3

   It also exposed something worth recording: with `>=`, the SSE replay loop in
   `sse.rs` never terminates, because each batch re-delivers its own last row.
   `seq > last_seq` is what makes that loop finite, not an incidental detail.

2. **`a_zero_cursor_replays_the_whole_thread`** — `ORDER BY seq` → `ORDER BY seq DESC`:

       assertion `left == right` failed
         left: "ThreadEntryAppended"   right: "PlanningThreadCreated"

3. **`an_event_published_during_the_replay_survives_the_handoff`** — moved
   `state.bus.subscribe()` from before the spawn to after the replay, which is
   exactly the gap §2.10 forbids:

       panicked at tests/resync.rs:183:
       the live subscription must exist before the replay is read:
       SendError((OperationId("op-live"), Delta { text: "during-replay" }))

**How that third test observes a gap.** It calls `subscribe` directly and then,
with no `.await` between, publishes on the bus. On `#[tokio::test]`'s
current-thread runtime the spawned replay task provably has not been polled at
that instant, so the publication lands squarely inside the replay window: a
correct handler already holds a receiver and buffers it, a handler that
subscribes after the replay has no receiver and loses it outright — the send
itself fails. The test then drains the real SSE body and asserts
`event: durable` (×3, the thread's whole history exactly once) precedes
`event: caught-up`, which precedes the live item. This is deliberate: a test
that published *after* `caught-up` would pass against both implementations and
would not be a test of this guarantee at all.

I also verified, with a throwaway test that was deleted rather than committed,
that `Path<ProjectId>` deserialization and `GET /` both work end to end over a
real socket (`GET /api/projects/<id>/threads` → `200 []`, `GET /` → the stub
page). See the coverage concern below.

## Files changed

- new `src/storage/sqlite/events_read.rs`, `src/protocol/{mod,handlers,sse}.rs`,
  `src/protocol/index.html`, `tests/resync.rs`
- modified `src/lib.rs`, `src/storage/mod.rs`, `src/storage/sqlite/mod.rs`,
  `src/cli/mod.rs`, `src/tracing.rs`, `docs/codebase/README.md`,
  `docs/codebase/inventory.md`

## Deviations from the brief, and why

1. **The harness version probe does not use `tokio::process`.** The brief's Step
   6 calls `tokio::process::Command::new(path).arg("--version")` inside
   `cli/mod.rs`. CLAUDE.md makes `process/` the sole owner of that API, and that
   rule is binding over the brief's Rust. `harness_version` now goes through
   `process::spawn` + `take_stdout_lines` + `wait`, which needed no new API in
   `process/` and so did not give that module a second job. Behaviour is the
   same, including `"unknown"` when the executable cannot be run.

2. **The tracing subscriber now writes to stderr** (`src/tracing.rs`, one line
   plus its reason). This was not optional: the brief's `serve` logs
   `"startup recovery complete"` *before* printing the address, and
   `tracing_subscriber::fmt()` defaults to stdout, so `tests/serve_smoke.rs`
   failed with the log line as the daemon's first stdout line. Spec §1.0 gives
   stdout one job. Moving the log after the `println!` would not have been a
   fix — `Runtime::start` logs recovery transitions of its own before `serve`
   ever reaches that line.

3. **Signatures adapted to the post-`edc2dd2` / post-Task-10 tree**, as the
   dispatch directed: newtype ids throughout, `NewThreadEntry` instead of the
   brief's positional `append_thread_entry`, `PlannerTurnRequest` instead of
   eight positional arguments, `bus: broadcast::Sender<(OperationId, StreamItem)>`.
   Requirements, spec references, event kinds, SQL and assertions are verbatim.

4. **`src/protocol/index.html` stub** per ledger Ruling 1.

## Concerns

1. **§8.5 is not actually kept by the shutdown I was told to wire — reported,
   not redesigned.** The brief's Step 6 decides `StopKind` from
   `PlannerTurn::stop(...).is_err()`. `stop` returns `Ok(())` for three
   different outcomes — cancelled, declined because the turn already ended, and
   no live handle — so `all_confirmed` stays true in cases where the operation
   is *not* terminal, and `stop_kind = Graceful` gets written anyway. §8.5 is
   explicit that Graceful "is a claim, and it is only written when it is true".
   Two further holes in the same closure: it enumerates `LiveHandles`, not the
   operations this runtime *owns*, so a `Pending` operation that never
   registered is invisible to it; and there is no second-stop-signal escalation
   path at all. I implemented what the brief specifies and did not invent a
   design beyond it. The controller decides whether this lands here or in Task 13.
   Until then, §8.6's anomaly detection is the only thing that will notice.

2. **`PlannerTurn::stop`'s three-outcomes-one-`Ok` problem reaches a route now.**
   `POST /api/operations/{id}/stop` returns the operation row afterwards, so a
   client can infer the outcome from `status_kind` — but the handler itself
   cannot distinguish "cancelled" from "declined" from "no handle". Known
   deferred minor; reporting rather than redesigning `stop`.

3. **Durable events committed *after* `caught-up` are never streamed.** The live
   phase forwards transient `StreamItem`s off the bus and nothing else, so a
   client that stays connected sees the assistant's text as a transient `entry`
   and never sees the `ThreadEntryAppended` row that was durably written for it.
   That is survivable in Milestone 0 (the client can re-subscribe with `after`),
   and §2.10's own wording is about reconnect, but "de-duplication by durable
   sequence" only has something to de-duplicate if live events eventually carry
   a `seq`. Worth a ruling before Task 12 builds its client against the current
   shape.

4. **Six of the seven routes have no test.** The brief's tests cover the replay
   query and I added the no-gap test; nothing in this task exercises
   `create_project`, `create_thread`, `list_*`, `start_turn` or `stop_turn`, nor
   the `Failure` → status mapping. I verified the riskiest part by hand (path
   newtype deserialization, which would have 400'd every id-bearing route) and
   deleted the probe. If Task 12 does not cover these, the mapping in
   `protocol/mod.rs` has branches no case reaches.

5. `start_turn` appends the user's message durably *before* `PlannerTurn::start`,
   as the brief specifies and comments. If the start then fails, the message
   remains. That is the intended trade (a restart mid-turn still shows what was
   asked), but it is a visible asymmetry, not an oversight.

## Did the code map serve me?

Yes, and it was the entry point. `docs/codebase/inventory.md` gave me every
signature this task consumes — `PlannerTurn::start`/`stop`, `PlannerTurnRequest`,
`LiveHandles`, `NewThreadEntry`, `Storage::*`, `EventCursor`, `StorageError`'s
variants — without opening the modules that declare them, and the ownership
table told me `protocol/` did not exist yet.

Three things I had to read the tree for, all of them cases the map states it
excludes rather than failures of it:

- **`newtype_id!`-generated items are invisible.** `ProjectId`, `ThreadId` and
  `OperationId` appear in the inventory only as *field types*; their own
  declarations, and in particular `from_stored`'s `pub(crate)` visibility and
  `from_literal`'s `test-support` gate, are produced by a macro the generator
  does not expand. I read `src/id.rs` for those, and the task brief pointed me
  there anyway. This is the one place where "every declaration that exists" is
  not literally true, and a future reader will hit it too.
- **Derives are stripped**, so whether `Operation`/`Project`/`ThreadEntry` are
  `Serialize` (every handler `json!`s them) and whether `EventCursor` is `Copy`
  (it is — `.clone()` on it would have been a clippy failure) needed a one-line
  grep each. Reasonable exclusion; cheap to check.
- **Private items are absent by design**, so the durable-event column list came
  from `migrations/0001_milestone0.sql` and `events.rs`.

I did not crawl the tree to discover what existed.
