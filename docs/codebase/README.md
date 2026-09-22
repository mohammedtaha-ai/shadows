# Code map

**Read this before writing code in `shadows`.** Two files, split by what can be
checked:

| File | Kind | Answers |
|---|---|---|
| [`inventory.md`](./inventory.md) | generated from `src/` | *Does this already exist?* Every reachable declaration with its full signature. |
| this file | written by hand | *Where does my new code go?* The one job each module holds. |

`cargo test --test codemap` fails when either has drifted from the tree, so
neither can go stale quietly. Regenerate the inventory with
`UPDATE_CODEMAP=1 cargo test --test codemap` in the same commit as the code
change that moved it.

## What each module owns

One job per module, stated without "and" — a conjunction here means the module
has quietly taken a second responsibility. The reference file is the one to read
before adding to that module: it is the pattern to follow, not merely an example.

| Module | Its one job | Reference file |
|---|---|---|
| `src/agent/` | the AI subprocess harness contract | `src/agent/claude.rs` |
| `src/bin/` | test apparatus that no product code links | `src/bin/tree_probe.rs` |
| `src/cli/` | the daemon's entry point | `src/cli/mod.rs` |
| `src/command/` | external-command identity for idempotency | `src/command/mod.rs` |
| `src/config.rs` | startup configuration resolved once | `src/config.rs` |
| `src/error.rs` | the stable failure taxonomy clients match on | `src/error.rs` |
| `src/events/` | the durable event record's shape | `src/events/mod.rs` |
| `src/process/` | OS process ownership with whole-tree containment | `src/process/mod.rs` |
| `src/project/` | project identity | `src/project/mod.rs` |
| `src/runtime/` | the runtime instance's lifecycle | `src/runtime/mod.rs` |
| `src/storage/` | persistence | `src/storage/sqlite/project.rs` |
| `src/thread/` | the planning thread's shape | `src/thread/mod.rs` |
| `src/tracing.rs` | tracing subscriber setup | `src/tracing.rs` |

A module absent from this table is a module that does not exist yet. The fifteen
planned modules are listed in [`CLAUDE.md`](../../CLAUDE.md); this table is not a
second copy of that list, and the tree is what decides which of them are real.

**This table is a working summary, not authority.** `CLAUDE.md` owns the five
single-ownership invariants and the file-size rules; the specs indexed by
[`specs/README.md`](../superpowers/specs/README.md) own the design. Where this
file disagrees with either, they are right and this file is the defect.

## What is deliberately not here

- **Line numbers.** Wrong at the first line inserted above them, with nothing
  failing when they lie. One field that rots silently costs the reader their
  trust in every field beside it.
- **Built / not-built status.** The tree answers it and
  [`docs/status.md`](../status.md) narrates progress. A third copy would record
  one fact in three places.
- **Explanations of declarations.** Those live as doc comments on the
  declarations themselves, which is why the inventory strips them.
