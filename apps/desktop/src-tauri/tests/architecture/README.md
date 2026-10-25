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
The `OWNERS` table maps the D-0 owners to current modules; `EDGES` is the D-0 graph.
Account includes its private assistant and native-login primitives. Preferences
is distinct from MainWindow even while it lives under `windows/`.

The checks hold:

- Owner dependencies, including imports, aliases, qualified paths, type aliases
  and named reexports. Cycles are reported by participating file and owner edge.
- Managed-state method/UFCS lookups and Tauri `State` injection outside `lib.rs`.
- Construction of the explicitly inventoried lifetime owners outside `lib.rs`,
  including `new`, `start`, `resolve`, `default`, tuple calls and struct literals.
  Ordinary DTO construction is allowed.
- Cross-owner implementation-submodule access and private owner stores.
- Core endpoint literals outside Companion/CoreConnection, including literals
  inside macros; reqwest paths outside those owners or retained Account HTTP.
- Owner contract existence and code-map entry coverage. Existing standalone
  owners use their current map/spec references rather than invented contracts.
- Equal command-name sets in `generate_handler!`, build metadata and every
  permission file's allow/deny commands, including handwritten groups.

The resolution is bounded source analysis, not Rust type checking. Foreign
owner glob imports require explicit review rather than hiding their dependency
surface. Native behavior tests remain responsible for focus, windows, races,
HTTP behavior and permissions at runtime.

`baseline.txt` contains only exact `rule :: file :: target` exceptions. Source
findings carry an occurrence ordinal so adding an identical reference in the
same file fails too. Both NEW and STALE entries fail; duplicate baseline records
fail. Tests never rewrite the baseline. Remove an entry with its violation; a
pure move may relocate it one-for-one. Review changes per file/owner rather than
approving a regenerated inventory. Tests have no blanket exemption.

The command baseline deliberately holds the two build omissions:
`companion_input` and `companion_send`. It does not repair command registration
or generated permissions.

`fixtures.rs` supplies in-memory sources to the same scanner/comparator:
forbidden/allowed edges, cycles, import aliases, qualified paths, reexports,
type aliases, foreign globs, `#[path]`, managed state (`state`, `try_state`, UFCS,
State injection), constructors, private submodules/stores, Core HTTP, contract/map
coverage, three-way command omissions and extra permission commands, new/stale
exceptions, repeated violations and duplicate baseline entries.
