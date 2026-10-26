# Native architecture ratchet

Run from the repository root:

```powershell
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test architecture
```

Cargo discovers `architecture/main.rs` as an integration test. The existing AYLUL
workflow's native `cargo test` step includes it without another CI entry.

`scanner.rs` parses every Rust file under `src/` with `syn`, including test and
fixture sources and inline test modules. It follows module declarations and
`#[path]` attributes, with directory-derived names for unattached fixture sources.
Parent-relative paths are normalized. Missing explicit path targets and multiple
mounts of one source fail rather than selecting an owner implicitly. Unknown
inline owners fail coverage too.
The `OWNERS` table maps the D-0 owners to current modules; `EDGES` is the D-0 graph.
`app.rs` is composition, alongside the lifecycle adapters in `lib.rs`. Commands
may reference the root. Window references require exact `owner_edge` exceptions,
and every lookup still needs a `managed_state` exception until D-5/D-7 remove it.
Accessor return signatures in `DesktopApp` identify the underlying owners:
outside commands/composition, those accesses retain owner, cycle and private-store
checks. Commands may call owner accessors but still cannot obtain private stores.
Accessor names are matched conservatively without receiver type inference,
including renamed accessors; an unrelated method with the same name needs review.
Only the top-level
composition module may name its stored lifetime types without private-access
exceptions; nested owner modules retain all checks. Implementation-submodule
workflow references remain ratcheted even in composition.
Account includes its private assistant and native-login primitives. Preferences
is the `desktop_preferences/` owner, distinct from MainWindow. Its lifetime
constructor replaces the Store constructor in the service inventory; its private
Store remains subject to cross-owner submodule checks. The OS-adapter constructor
remains inventoried at its private module path and is injected by composition.

The checks hold:

- Owner dependencies, including imports, aliases, qualified paths, type aliases
  and named reexports. Cycles are reported by participating file and owner edge.
- Managed-state method/UFCS lookups (including UFCS function values) and Tauri
  `State` injection outside the root composition modules in `lib.rs` and `app.rs`.
- Construction of the explicitly inventoried lifetime owners and OS login-item
  adapter outside that composition module,
  including `new`, `start`, `resolve`, `default`, tuple calls and struct literals.
  Associated factory signatures returning the inventoried type or `Self`
  (including wrappers such as `Arc<Self>`) are discovered before checking calls,
  so names such as `with_timing` and `from_environment` cannot bypass the rule.
  Ordinary DTO construction is allowed.
  `Self::new` resolves through the enclosing implementation. A factory's `Self`
  field initialization is allowed so it can implement the public constructor.
- Cross-owner implementation-submodule access and private owner stores.
- Core endpoint literals outside Companion/CoreConnection, including literals
  inside macros; reqwest paths outside CoreConnection, retained Account HTTP and,
  within Companion, its private `presence::core_client` module.
- Owner contract existence and code-map entry coverage. Existing standalone
  owners use their current map/spec references rather than invented contracts.
- Equal command-name sets in `generate_handler!` passed to `invoke_handler`,
  build metadata and every
  permission file's allow/deny commands, including handwritten groups.
  Registration and build metadata are collected from their own files. Direct
  `#[cfg(test)]` modules/functions and `#[test]` functions do not contribute
  production inventory; unused handler lists cannot fill registration omissions.

The resolution is bounded source analysis, not Rust type checking. Foreign
owner glob imports require explicit review rather than hiding their dependency
surface. Native behavior tests remain responsible for focus, windows, races,
HTTP behavior and permissions at runtime.
Rust macro arguments are parsed as expressions, statements, or expression
fragments around DSL separators; nested groups are scanned. Macro expansion,
generated identifiers and runtime dispatch still require code review. Constructor
types are explicitly inventoried; named return types supplement the conventional factory names;
arbitrary factories returning erased types need review. Method names `state`
and `try_state` are conservative without receiver type inference.

`baseline.txt` contains only exact `rule :: file :: target` exceptions. Source
findings carry an occurrence ordinal so adding an identical reference in the
same file fails too. Both NEW and STALE entries fail; duplicate baseline records
fail. Tests never rewrite the baseline. Remove an entry with its violation; a
pure move may relocate it one-for-one. Review changes per file/owner rather than
approving a regenerated inventory. Tests have no blanket exemption.

The command inventory has no baseline exceptions. All 27 registered commands
have build metadata and generated permissions.

N-19 also compares each registered command's native admission, local capability
allow grants, and wrapper surfaces pairwise. `deny` names count only as metadata,
never as grants. Permission identifiers resolve through source TOML; local JSON
capabilities map those allows to their window labels. External plugin permissions
are outside the application command inventory.

Native admission follows the command's first statement into the helper it calls.
Call paths resolve from the calling module, so a same-named function elsewhere is
never read in its place, and each helper parameter binds positionally to the
caller's literal or `label()` argument; an unbound name is unsupported. The bounded
`syn` evaluator supports early-return label checks, Result-valued helper branches,
boolean predicate helpers, `match` on the bound policy key, boolean comparisons and
`matches!` label alternatives. Policy-table rows alone confer no admission. A
missing or late guard means any window granted the command can call it; that is
compared as unrestricted admission, not a standalone violation. Unsupported guard
expressions produce `command_guard` findings. Label candidates include source
literals, capability labels, wrapper surfaces and an unknown-label sentinel, so a
guard-only extra label or unrestricted admission cannot disappear.

The architecture test runs `node ../tests/native-acl.test.mjs --wrapper-inventory`
to parse TypeScript text with the installed TypeScript compiler (shared web
installation preferred, desktop fallback). CI already installs these dependencies.
The two roots are `web/src/desktop/` (main) and `apps/desktop/src/platform/`
(companion); `.ts` and `.tsx` files are read, test/spec files are excluded.
Callers are `invoke` imported from `@tauri-apps/api/core` (named, aliased or through
a namespace) and `nativeCall` imported from whichever relative path resolves to the
`native.ts` bridge. Every use of a caller must be a direct call with a literal
command: a caller stored, passed as a value or re-exported, a string constant, a
template or any other dynamic argument fails, except the bridge's own
`invoke(command, args)` forwarding call. Comments, strings and unrelated functions
are not calls. This is bounded AST analysis, not TypeScript type checking or runtime
IPC acceptance; a new bridge form needs explicit scanner coverage.

Negative fixtures assert the exact finding for every pairwise disagreement,
granted commands without wrappers, wrappers without grants, deny-only grants,
removed/late guards, unsupported guard expressions, the actual helper policy key
(including guard-only labels) and module-resolved helpers. Node fixtures exercise
wrapper aliases, namespaces, generic calls, relative bridge imports from `.tsx`,
ignored test sources, comment/string decoys, and the refusal of nonliteral commands
and of callers used indirectly or re-exported.

`fixtures.rs` supplies in-memory sources to the same scanner/comparator:
forbidden/allowed edges, cycles, import aliases, qualified paths, reexports,
type aliases, foreign globs, `#[path]`, managed state (`state`, `try_state`, UFCS,
State injection), constructors, private submodules/stores, Core HTTP, contract/map
coverage, three-way command omissions and extra permission commands, new/stale
exceptions, repeated violations and duplicate baseline entries. Alias resolution
also covers private parent imports, public type aliases and block-local aliases
inside macros. Private access checks retain the written module surface so a
reexport cannot make an implementation submodule a public owner entry.
The review fixtures also cover shadowing of absolute paths, private import
aliases in reexports, inline declarations, Tauri globs, statement/DSL macros,
handler aliases, inventory separation, path escape/multiple mounts, OS adapter
construction, move relocation and deterministic output. Only top-level composition in `lib.rs` and `app.rs`
has construction/state exemptions; neither inline nor file-backed tests do.
The inventory's test-only filtering does not exempt tests from ownership rules.

`desktop_registers_one_private_composition_root` checks that the native sources
register one managed value, start `DesktopApp` once and retain the exact private owner names and types below.
The N-10 fixtures prove that window root lookups still count as managed-state
and owner-edge violations, accessor calls retain owner edges/private access/cycles,
and nested owners cannot inherit composition exemptions.

Accessor return-type checks cover private Stores independently of the
service-construction inventory. Preference fixtures cover command accessor
privacy and OS-adapter construction through the owner's reexport.

## DesktopApp owner inventory

The composition assertion compares every private field and its complete Rust type
against this table. Arc wrappers are part of the stored type. Renamed, extra and
mistyped fields are rejected by an adversarial in-memory source fixture.

| Field | Type path |
| --- | --- |
| `core_connection` | `Arc<core_connection::Connection>` |
| `companion` | `Arc<presence::Presence>` |
| `companion_window` | `windows::companion::Store` |
| `main_window` | `windows::MainWindow` |
| `navigation` | `Arc<navigation::Navigation>` |
| `desktop_preferences` | `desktop_preferences::DesktopPreferences` |
| `account` | `Arc<auth::AuthController>` |
| `activity` | `Arc<sidecar::SidecarState>` |
| `app_paths` | `app_paths::AppPaths` |
