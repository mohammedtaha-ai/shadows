# Architecture ratchet

Owner: the binary's architecture integration test. The application architecture
is owned by the application-core design, section 14.12; this guard implements
the A-0 checks without changing product behavior.

Run `cargo test -p shadows --test architecture`. `main.rs` compares the scanner's
findings with four checked-in allowlists. Normal tests never rewrite them.
Fixture tests build temporary trees and use exactly the same scanner and
comparison. No mutation test edits the real product tree.

| File | Job |
|---|---|
| `scanner.rs` | Source discovery, registration, comment tokens, free public functions, finding identities |
| `resolution.rs` | Bounded Rust path and receiver classification |
| `sql.rs` | Literal SQLite table-reference classification |
| `table-owners.txt` | Explicit table ownership during service migration |
| `fixtures.rs` | Positive and negative mutation cases |

## Checks

- Ownership: `impl Storage` methods are indexed by resolved type and defining service; calls use
  imports, renamed imports, reexports, simple type aliases, typed parameters,
  struct fields, local bindings, clone wrappers and declared method returns.
  A known unrelated receiver is not a Storage call just because a name matches.
  An unresolved receiver with a Storage method name is a **review** finding.
  The seven foundation operations in `POOL_METHODS` are exempt from the
  cross-service Storage-call rule, not from SQL ownership checking.
- Literal SQL is scanned even when reached through an allowed pool/transaction
  operation. SELECT/FROM/JOIN, INSERT/REPLACE/INTO, UPDATE, DELETE, nested
  subqueries and comma-separated table lists are classified. SQL value strings
  are not identifiers. Another owner's table produces an ownership finding;
  unknown tables, interpolation, unsupported statements and dynamic/indirect
  `sqlx::query*`, `raw_sql` or `QueryBuilder` inputs produce review findings.
  Query aliases, query macros and generic builder paths are checked too.
  `QueryBuilder::push` inputs require the same classification, including
  direct calls, local bindings and fluent `push`/`push_bind` chains; bound values
  do not become dynamic SQL.
  Each statement in a literal batch must be classified. Quoted identifiers
  retain their table names without being mistaken for SQL clause keywords.
- Adapters cannot name/import/call stores or known live registry types.
  A service cannot depend on AppCore. Service constructors (`new`/`start` on
  structs implemented in a service entry module, plus calls to `default`) may
  only be invoked by `crates/shadows-core/src/app.rs`; explicit service struct literals
  are checked too. Publicly reexported value types defined in a store file are
  allowed as API values; stores and known registries remain forbidden.
  Existing test-support bypasses are
  listed individually rather than excluded by filename or cfg. Construction
  checks also cover Core root files and foundation folders. Extracted services
  may call their own store; the crate prefix identifies that owner.
- Free functions with any Rust visibility modifier are findings in service
  modules, including inline modules; impl/trait methods are not free functions.
- Only parser comment tokens are checked for section signs and history markers.
  Line comments, doc comments and each line of block comments count. Strings
  containing the same text are safe. Commit-looking hex tokens are conservative
  history candidates; this is a source-style guard, not commit validation.
- Every direct service folder under `shadows-core/src` and every extracted
  `crates/core-*/src` is discovered from disk, including empty folders. Each
  needs its entry contract and an AppCore doc-table row naming that contract.
  The explicit foundation list (`db`, `command`, `testing`, extracted `core-db`)
  instead needs a code-map row. Future service names have no blanket exemption.

The source corpus is `crates/*/src/**/*.rs`, including cfg-gated unit-test
modules. Separate product workspaces and integration-test fixture source trees
are outside Core's guard. Parse errors fail the test instead of skipping a file.

## Existing exceptions and maintenance

The initial baseline records current source at `41738fe`; it fixes none of the
audited architecture problems. `registration.allowlist` explicitly includes
the absent AppCore service map (A-1) and runtime's absent contract. The ownership
baseline includes both observed cross-owner accesses and individually identified
review cases, including indirect SQL constants and the SQLite migration/schema
catalog queries whose ownership is not assigned to an application service.
Public-function and comment baselines preserve existing source-style violations.
The ownership baseline also names the test-support `profiles` and `secrets`
constructors in `testing/mod.rs`; review exposed these existing bypasses when
construction checking was extended beyond service and adapter folders.

Each record is `file :: enclosing symbol + reason [evidence hash] #occurrence`.
The deterministic hash covers normalized source evidence. Repeated equal
evidence in the same symbol has separate occurrence numbers. Adding a second
identical call therefore creates a new record; removing a violation makes its
record stale. Line-number drift alone does not change identities.

When fixing a violation, remove its exact stale line in the same commit and
rerun the test. New violations must be fixed, not accepted by regenerating a
baseline. A pure file move can rename the same records one for one; it cannot
claim that moving a violation fixes ownership. Changes to table ownership or
scanner classification require reviewing the resulting baseline diff against
the real source, never automatically updating it during a test run.

## Detection limits

This is a bounded static guard, not rustc or a SQL engine. It does not expand
arbitrary macros, execute build scripts, resolve trait dispatch, monomorphize
generics, or track arbitrary higher-order/async data flow. SQL assembled behind
an unfamiliar execution API requires review beyond this guard. Known SQL entry
points with nonliteral inputs remain explicit findings even when a human knows
the referenced constant is safe. Unsupported SQL constructs remain review
records; passing does not prove their ownership. Glob imports produce review
records rather than pretending that their names have been resolved.

Module paths follow conventional Rust filenames. Inline declarations are parsed,
but nested module aliases, `#[path]` remapping and unusual constructor factories
need manual review. The constructor index covers entry-module implementations;
an extracted service must retain that documented shape. Method-return inference
uses the first type argument of wrappers; complex union/trait/generic return
flows are not a compiler proof. A service map row is a structural registration,
not proof that its accessor, job description or contract is semantically correct.

Compiler privacy, contracts, focused behavior tests and review continue to hold
the boundaries that this syntactic ratchet cannot prove.
