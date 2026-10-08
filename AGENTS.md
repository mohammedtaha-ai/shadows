# Shadows

> **AI Start Here** — Read these files to get started:
>
> | Document | Purpose |
> |----------|---------|
> | [CLAUDE.md](./CLAUDE.md) | Architecture, conventions, rules (this file) |
> | [docs/superpowers/specs/README.md](./docs/superpowers/specs/README.md) | **Index of the authoritative design sections and their owners.** |
> | [docs/codebase/README.md](./docs/codebase/README.md) | **The code map.** What each crate and module owns, and the Architecture Invariants. Read before writing code. |
> | `crates/shadows-core/src/<service>/contract.yaml` | **One contract per service, next to its code**: its methods, obligations, agreements and tests. Read before changing that service. Written to [docs/codebase/contracts/TEMPLATE.yaml](./docs/codebase/contracts/TEMPLATE.yaml). |
> | [docs/vision.md](./docs/vision.md) | **What Shadows is for and where it is going.** Read before specifying any new milestone. Decides nothing. |
> | [docs/status.md](./docs/status.md) | Where the project is right now. Decides nothing. |
> | [docs/evidence/](./docs/evidence/) | Dated measurement records. Facts, not decisions. |
>
> **Documentation rules:**
> - The specs are split by topic. Every decision has exactly one owner file and
>   is amended **in place**. Other files refer to it; they do not copy it.
> - Do not create an ADR, a design note, or a plan document for a struct shape,
>   a library call, a test correction, or an implementation detail. Amend its owner spec.
> - A question the project cannot answer yet is an **OPEN** block inside its owner spec,
>   at the point it bites, naming the trigger that closes it. An OPEN block with
>   no trigger is rot, not a question.
> - Evidence files are dated facts. They never expire and are never design
>   authority. When a measurement changes a decision, amend its owner spec.
> - **Spike code is deleted once its evidence file is written.** A probe's job is
>   to produce an answer, not a codebase. Its raw output moves into
>   `docs/evidence/`; its source stays in Git history and the evidence file names
>   the commit. There is no `sandbox/` directory and no throwaway crate anywhere in
>   the tree — anything you find under `crates/` is the product. Do not keep a spike
>   around because it might be useful later; that is how a rewrite acquires a second
>   codebase nobody maintains.
> - Earlier specs, the runtime draft, and the four consolidated ADRs were absorbed
>   into the topic specs and deleted. They remain in Git history and are not
>   active references.
> - **The ownership table is written by hand and checked.** What each module
>   owns lives in `docs/codebase/README.md`, and
>   `cargo test -p shadows --test codemap` refuses a module with no owner row, a
>   path that does not exist, or a job stated with "and". Signatures are not
>   written down anywhere: the Rust LSP, or `where_is` when the `shadows` MCP
>   server is connected, answers them from the code.
>   `crates/shadows/tests/codemap/main.rs` owns that decision.
> - CLAUDE.md stays compact: links + rules + architecture. No long backlogs.

## Project

- **Slug:** shadows
- **Stack:** Rust 1.94+ candidate floor from SQLx 0.9, a Cargo workspace of the crates below, plus a React client in `web/`
- **Status:** Milestones 0–3 are on `main` and ran on Windows; Milestone 3 added the code index (§15). See [docs/status.md](./docs/status.md) for where the project is, and [docs/vision.md](./docs/vision.md) for where it is going.
- **Purpose:** Local-first AI orchestration layer (planning + workflow + context + execution + verification + continuity). Clean rewrite of `shadow` avoiding patching pattern.

## Architecture

A Cargo workspace with a flat `crates/` directory (spec §14.3 owns the list), plus an independent browser client in `web/`. The one binary, `shadows serve`, serves the product locally but never opens a browser automatically.

```text
shadows ─┬─► shadows-http ─┐                 ┌─► shadows-agent ─► shadows-process
         ├─► shadows-mcp  ─┼─► shadows-core ─┤
         └────────────────►┘                 └─► shadows-index
```

Dependencies point one way, and Cargo refuses a cycle. An adapter depends on
`shadows-core` only; a `shadows-agent` type it serializes, such as
`SessionChoices`, is re-exported by `shadows-core`. `fake-acp` is the test
adapter binary; no product crate links it. `shadows-index` depends on no
Shadows crate (spec §15.2). Inside `shadows-core`, `AppCore` holds nine
services, one folder each: `projects`, `threads`, `turns`, `harness`, `plans`,
`grants`, `instructions`, `events`, `code` (spec §14.4). Planned,
and not created until their first user exists: the `scheduler`, `execution`
and `verification` services, and `secrets`.

### Single-ownership rules

| Crate | Sole owner of |
|---|---|
| `shadows` | The binary: configuration, tracing, binding, signals; builds `AppCore` once |
| `shadows-http` | The HTTP API and SSE (`/api/…`), and mounting `/mcp` |
| `shadows-mcp` | Shadows' MCP server at `/mcp`, a separate interface from the HTTP API (spec §13.6) |
| `shadows-core` | The application: every operation is a method of one service. SQLite lives here only, in `db/` and the stores |
| `shadows-agent` | The AI subprocess harness (the ACP `Connection`) |
| `shadows-index` | tree-sitter: a file's text in, its tags out, knowing nothing of SQLite or projects (spec §15.3) |
| `shadows-process` | `tokio::process` / `process-wrap` (private to this crate) |
| `secrets` (planned) | Secret value resolution (config holds refs only) |

### Persistence (spike outcome)

**Winner: SQLx 0.9 only.** The delta validated SQLx and SeaORM 2.0.3 end-to-end against PostgreSQL 16.15; SQLx retained the edge and SeaQuery had no demonstrated use case. See `docs/evidence/persistence/DELTA_VALIDATION.md` at `92e6dae`.

### External tools

`gcode` is an optional external executable. Shadows may invoke it through the normal process/tool boundary when code search is added, but does not vendor or depend on `gobby-cli`, `gcore`, PostgreSQL, FalkorDB, or Qdrant. `ghook` and `gwiki` are not planned.

## Rules (project-specific)

- **The gate before every commit**, from the root; CI (`.github/workflows/ci.yml`)
  runs steps 1–5:
  1. `cargo fmt --all --check`
  2. `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
     (`fake-acp` builds only with its `test-support`)
  3. `cargo test --workspace`
  4. `cargo clippy --workspace -- -D warnings`, without `--all-targets`: the build
     without `test-support`, where the core's boundary is checked
  5. `cargo tree -e features,no-dev --workspace | grep test-support` prints nothing
  6. `git diff --exit-code api/`: `api/openapi.json` changes only with a route
- **Library-first:** no custom ORM, no custom migration engine. Use Rust ecosystem crates.
- **A file earns its size.** At **300 lines** a file stops being free: the change that
  pushes it over states, in its commit message or report, what that file's single
  responsibility is and why the new code shares it. At **500 lines** it splits. Split by
  responsibility, never by line count — a `helpers.rs` or `utils.rs` is the same pile
  under a new name, and so is a split that leaves two files that must be read together.
  The test: name each resulting file's one job in a short phrase without using "and".
  These thresholds are a trigger for judgment, not a lint; a 520-line file with one
  genuine responsibility survives review by saying so.
- **Known accretion points.** Three places take a change from nearly every task, so they
  rot first and must be watched by name: `shadows-core/src/db/mod.rs` (the pool and
  write transaction every store shares), `shadows-http` (one route per feature,
  forever), and `shadows-core/tests/storage_contract/`. When one of them grows, the
  split goes by domain — each service's `store` already does this — not into a second
  facade.
- **A new operation is a new method on one service,** with its line in that service's
  contract. An HTTP route and an MCP tool that do the same thing call the same method.
- **An adapter translates and holds no rule.** `shadows-http`, `shadows-mcp` and the
  binary read a request, call one service method, and shape its answer (spec §14.5).
- **Domain stays pure:** a service's `model.rs` carries no persistence imports (`sea_orm`, `sqlx`, `sea_query`, `Row`, `Entity`, `ActiveModel`, `Pg*`, `Sqlite*`); queries live in its `store`.
- **Ordering is explicit:** durable event sequence, thread-entry ordinal, or another explicit stable domain key. No `rowid`, physical insertion order, or implicit `SELECT` order.
- **DB-specific syntax isolation:** backend-specific SQL (FTS5, tsvector, `PRAGMA`, `rowid`, `strftime`) stays inside a `store` (each service's, and `runtime/store.rs`) and `shadows-core/src/db/`, never outside `shadows-core`. Enforced by crate and module boundaries, contract tests, and review — not by a keyword blacklist scanned across the tree (spec §2.9).
- **Idempotency:** mutating commands carry `CommandId`, command kind, schema version, and normalized request fingerprint. Replay requires fingerprint equality; mismatch is `CommandConflict`.
- **PLAN_BLOCKED = Operation outcome, NOT HTTP error.** Structured refusal, not transport failure.

## How agents work here

These override the defaults of any execution skill. Token cost is a real
constraint on this project, and every rule below exists because a round trip,
a crawl, or a re-read was paid for and bought nothing.

- **The code map is the entry point, not the source tree.** Read
  `docs/codebase/README.md` (what each module owns) and the service's
  `contract.yaml` FIRST, then ask the LSP or `where_is` for signatures and
  callers, and open only the files they name. Reading the tree to discover
  what a signature is means you skipped them — say so in your report.
- **Ask the code index before searching the tree.** When the `shadows` MCP
  server is connected, `where_is`, `who_uses` and `outline` answer where a
  name is defined, where it is used, and what a file declares, with file,
  line and signature. Use them before Grep or opening files to find a
  signature; open the file only at the line they name. They answer where,
  not how or why: that is the contract, the owner spec, and the code itself.
  If the server is not connected, say so in your report and fall back to the
  Rust LSP.
- **The contract is the entry point for a service.** Read
  `crates/shadows-core/src/<service>/contract.yaml` before changing that
  service; open its code for what the contract points to.
- **A change to a service updates its contract in the same commit:** its
  methods, obligations, agreements and tests, following
  `docs/codebase/contracts/TEMPLATE.yaml`. `contracts.rs` catches a missing
  name; the reviewer catches a false rule.
- **A reviewer fixes what it finds.** A review dispatch is one seat: find it,
  fix it, run the gate, commit, and report what changed and why — not a findings
  list that costs another dispatch to act on. It still reports everything it
  found, including what it chose not to change and why. The controller reads the
  resulting diff; that is the second pair of eyes. What a reviewer may NOT do
  silently is contradict the plan or a spec — those it reports and leaves.
- **Compose the dispatch once.** Everything a subagent needs — the task, the
  interfaces, the rulings, the constraints — goes in the first message. A
  follow-up message to steer an agent mid-task is a controller planning failure
  and is paid for in full context re-read. Fix rounds are the exception, because
  the findings did not exist yet.
- **Branches are short-lived.** Finish the tasks, open the PR, merge it, delete
  the branch. Do not carry a second long-lived branch alongside `main` and do
  not leave merged branches on the remote. A branch nobody is committing to is
  either merged or abandoned; both cases end with it deleted.
- **The controller verifies a review; it does not repeat it.** After a reviewer
  reports, the controller checks that the report is true — the commits exist,
  the diff says what the report says, the gate and test count are real — and
  rules on what was left to it. It does not re-read the code for a second deep
  review. One whole-branch review runs before the PR, and that is the only other.

## Lessons from `shadow`

`shadow` spent 29 days, 413 commits and 67k lines of Rust and ended with
nothing a person could run: no web client, and a socket on which no business
method could execute. These three rules are what that cost.

- **Run it before you document it.** A slice is not done until a person has
  started `shadows serve`, used it in a browser, and read its logs. 35% of
  `shadow`'s commits were docs about software nobody had run.
- **One path end to end before any abstraction.** Every `shadow` method was
  tested alone; the first test that crossed the layers found swapped arguments
  in minutes. Build the path through every layer first, then widen it.
- **No layer before its first user.** Protocol versions, nine authorities and
  six crates existed before one request succeeded. A seam, trait, or version is
  added when the second caller needs it, not when it might.

## Decisions

[`docs/superpowers/specs/README.md`](./docs/superpowers/specs/README.md) maps every design section to its sole owner file. Together those owner files hold the complete architecture semantics. The rules above are a working summary; where this file and an owner spec disagree, the owner spec is right and this file is the defect.
