# Shadows

Local-first AI orchestration: planning, workflow, context, execution and
verification, run by one local binary (`shadows serve`) with a React client.
Rust 1.94+, SQLx 0.9, SQLite.

## Start here

| Read | For |
|---|---|
| [docs/README.md](./docs/README.md) | Which folder holds which product's docs: Core, AYLUL, code intelligence |
| [docs/codebase/README.md](./docs/codebase/README.md) | **The code map.** What each crate and module owns. Read before writing code. |
| [docs/frontend/codebase/README.md](./docs/frontend/codebase/README.md) | The `web/src` code map |
| `<module>/contract.yaml` | What one module does, as the code behaves. Shape: [TEMPLATE.yaml](./docs/codebase/contracts/TEMPLATE.yaml) |
| [docs/superpowers/specs/README.md](./docs/superpowers/specs/README.md) | Which spec owns which decision |
| `docs/<side>/stages.md` | What is left on the current branch |
| [docs/future.md](./docs/future.md) · [docs/aylul/future.md](./docs/aylul/future.md) | Meant, not built. Decides nothing. |
| [docs/backend/AGENTS.md](./docs/backend/AGENTS.md) · [docs/frontend/AGENTS.md](./docs/frontend/AGENTS.md) | The two sides, their builders and fixers |

Where this file and an owner spec disagree, the spec is right and this file is the defect.

## Workspaces

| Workspace | Root | Holds |
|---|---|---|
| Core | `Cargo.toml` (`members = ["crates/*"]`) | Every `shadows*` crate and `fake-acp` |
| AYLUL desktop | `apps/desktop/src-tauri/` | The Tauri app ([its instructions](apps/desktop/AGENTS.md)) |
| AYLUL hook | `apps/desktop/hook/` | The desktop hook |
| Runtime (planned) | `runtime/` | Our own agent runtime, which replaces ACP step by step. Core calls it as a library. |

Each workspace has its own lockfile, `target/` and gate. A crate in one workspace
never reaches into another's source; it depends on it as a library.

```text
shadows ─┬─► shadows-http ─┐                 ┌─► shadows-agent ─► shadows-process
         ├─► shadows-mcp  ─┼─► shadows-core ─┤
         └────────────────►┘                 └─► shadows-index
```

Dependencies point one way. An adapter (`shadows-http`, `shadows-mcp`, the binary)
depends on `shadows-core` only. `shadows-index` depends on no Shadows crate.

| Crate | Sole owner of |
|---|---|
| `shadows` | The binary: configuration, tracing, binding, signals. Builds `AppCore` once. |
| `shadows-http` | The HTTP API and SSE (`/api/…`), and mounting `/mcp` |
| `shadows-mcp` | Shadows' MCP server at `/mcp` |
| `shadows-core` | The application. SQLite lives here only, in `db/` and the stores. |
| `shadows-agent` | The AI subprocess harness (the ACP `Connection`) |
| `shadows-index` | tree-sitter: a file's text in, its tags out |
| `shadows-process` | `tokio::process` / `process-wrap` |

## Code structure

> **STRICT RULE: AppCore is the map and the owner. Start there, every time.**
> Every task in Core begins at `crates/shadows-core/src/app.rs`. `AppCore` names
> every service, the one job it owns, and its contract. Find the service that
> owns what you are changing, go to that folder, read its `contract.yaml`, and
> change only there. You may not:
> - search the tree to find where something is done, instead of asking `AppCore`;
> - reach a service, a store, or live state (sessions, running turns) except
>   through `AppCore` and the service that owns it;
> - add a second way in: a second composition root, a second HTTP surface, or a
>   helper another service calls directly;
> - put a rule in a second place because the owner was hard to find.
>
> If no service in `AppCore` owns what you need, **stop and report**. A new owner
> is a design decision for the controller, never something to improvise. A change
> that breaks this rule is refused in review, whatever else it gets right.

One runtime object, services as classes, one owner per rule. New code follows
these rules. Existing code that breaks them is fixed service by service,
`harness` and `turns` first. Do not copy a violation because it is already there.

1. **One object at runtime.** `AppCore` is built once in `AppCore::start` and is
   the only way in: `app.turns().send(…)`, `app.harness().open(…)`. Every
   service is a struct with private fields, built only inside `AppCore::start`.
   An adapter sees `AppCore` and nothing else. `app.rs` is the one place the
   whole application is wired, so it is large by design.
2. **A service gets what it needs through its constructor.** It is handed its own
   store and the other services it calls, and the wiring is written once, in
   `app.rs`. A service never reads another service's tables: it calls that
   service's method. Live state (sessions, running turns) belongs to exactly one
   service and is reached through it.
3. **One rule, one method, one owner.** A rule such as "what a turn sends to the
   model" is one method on one service. Others supply facts to it and never
   assemble part of the answer. When two paths must give the same answer (open
   and switch, single and batch, Core and web), they are listed in `agreements`
   with the test that holds them.
4. **A fixed shape per service folder.**
   ```text
   <service>/
     mod.rs         the service struct and its constructor
     model.rs       its types; no persistence imports
     store.rs       its SQL only (or store/ split by domain)
     <job>.rs       one job's methods (`impl <Service>`) or private parts
     contract.yaml  the contract, per module, traced from the code
   ```
   No free `pub fn` in a service. A helper is a private function or a method.
5. **Comments say why, never history.** No "used to", no "before the routes did",
   no spec section numbers in code. The contract and the owner spec carry those.
   A comment that disagrees with the code is a defect in the comment.
6. **`runtime/` follows the same shape.** One `Runtime` object, crates that talk
   only through its event types, and one `Runtime` held by Core.

**A new operation is a new method on one service**, with its line in that
service's contract, in the same commit. An HTTP route and an MCP tool that do the
same thing call the same method. **An adapter translates and holds no rule:** it
reads a request, calls one method, and shapes the answer.

## Rules

- **The full gate runs once per branch, before its PR opens,** from the root. Each
  commit runs only the tests its change touches. CI (`.github/workflows/ci.yml`)
  runs steps 1–5:
  1. `cargo fmt --all --check`
  2. `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
  3. `cargo test --workspace`
  4. `cargo clippy --workspace -- -D warnings` (the build without `test-support`)
  5. `cargo tree -e features,no-dev --workspace | grep test-support` prints nothing
  6. `git diff --exit-code api/`: `api/openapi.json` changes only with a route

  The AYLUL desktop has its own gate (`.github/workflows/aylul.yml`), clippy included.
- **One build cache per workspace, built over in place.** Never a second copy: no
  `CARGO_TARGET_DIR`, no worktree build, no `cargo clean`, no `--release`,
  `--profile`, `--target` or changed `RUSTFLAGS`. Keep `rust-lld.exe` in
  `.cargo/config.toml`, `[lib] doctest = false`, `debug = "line-tables-only"` and
  dependencies at `debug = false`. When `target/debug/deps` holds old copies of a
  test binary, delete the older ones (`.exe`, `.pdb`, `.d`); never the dependency cache.
- **Libraries for infrastructure, our code for product logic.** SQLx, axum, tokio,
  aes-gcm, hkdf, hmac, keyring; no custom ORM, migration engine or crypto. Parsing,
  validation and domain rules are written here (`parse_dotenv` is ours, not
  `dotenvy`). A new dependency's commit says which of the two it is.
- **A file earns its size by having one job, not by its line count.** A file of
  1000 lines that does one clear thing stays whole. A file that mixes jobs
  splits, however short it is: name each part's job without "and". No
  `helpers.rs` or `utils.rs`, and no file that collects unrelated things, such as
  every service's tests in one file: tests live with the thing they test. The
  same holds at every level: a service with two jobs becomes two services, and
  likewise a crate or a workspace.
- **Watch the accretion points:** `shadows-core/src/db/mod.rs`, `shadows-http`, and
  `shadows-core/tests/storage_contract/`. They split by domain, never into a second facade.
- **Domain stays pure:** `model.rs` has no persistence imports (`sqlx`, `Row`, `Sqlite*`, `Pg*`).
- **Ordering is explicit:** an event sequence, an entry ordinal or another stable
  key. Never `rowid`, insertion order or an unordered `SELECT`.
- **Database-specific SQL** (FTS5, `PRAGMA`, `rowid`, `strftime`) stays in a store or `db/`.
- **Idempotency:** a mutating command carries `CommandId`, kind, schema version and
  a normalized fingerprint. A replay needs an equal fingerprint; a mismatch is `CommandConflict`.
- **`PLAN_BLOCKED` is an operation outcome,** not an HTTP error.
- **Run it before you document it.** A slice is done when a person has started
  `shadows serve`, used it in a browser and read its logs.
- **No layer before its first user,** and one path end to end before any abstraction.

## Documentation

- Every decision has one owner spec, amended in place. Others link to it. New specs
  and plans follow their product: `docs/backend/`, `docs/frontend/`, `docs/aylul/`,
  `docs/code-intelligence/`. `docs/superpowers/` is read and amended, never added to.
- No ADR, design note or plan for a struct shape, a library call or a test fix: amend the owner spec.
- An open question is an **OPEN** block in its owner spec, with the trigger that closes it.
- Evidence (`docs/evidence/`) is dated fact, never authority. A spike's code is
  deleted once its evidence is written; the evidence names the commit.
- The code map is written by hand and checked: `cargo test -p shadows --test codemap`
  refuses a module with no owner row, a missing path, or a job stated with "and".

## How agents work here

- **Start from `AppCore`** (the strict rule above): `app.rs` names the owning
  service, then its `contract.yaml`, then the files it names.
  `docs/codebase/README.md` maps the crates outside Core. Find symbols with the LSP (Rust) or
  Serena (TypeScript), never with text search. Grep is for docs, config and log text.
- **A change to a module updates its contract in the same commit.**
- **Compose a dispatch once.** Task, interfaces, rulings and constraints go in the
  first message. Only fix rounds get follow-ups.
- **A reviewer fixes what it finds,** runs the gate, commits, and reports everything,
  including what it left. It never silently contradicts a plan or spec.
- **The controller checks a report is true** (commits, diff, gate, counts) and does
  not repeat the review. One whole-branch review runs before the PR.
- **Branches are short-lived:** finish, open the PR, merge, delete the branch.
