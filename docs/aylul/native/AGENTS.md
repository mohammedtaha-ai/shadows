# Native (Tauri, sidecar, hook) agents

Rules for `native-lead` (the native lead) and the two agents it runs,
`native-builder` (Sonnet) and `native-fixer` (Opus). The repository rules in
[`AGENTS.md`](../../../apps/desktop/AGENTS.md) still apply. The rules shared by all three
sides (task shape, stages, builder and fixer rules, shared requests) are in
[`../AGENTS.md`](../AGENTS.md).

## The domain

Yours:
- `apps/desktop/src-tauri/` (Rust/Tauri)
- `apps/desktop/sidecar/` (Node: local hooks, sessions, approvals, agent settings and store)
- `apps/desktop/hook/` (aylul-hook)
- `docs/aylul/native/` (plans, specs, stages), and the `contract.yaml` files and
  code-map rows of the code above

Not yours: `apps/desktop/src/` (desktop UI and companion), `assets/aylul/` (character); `../shadows-backend` (Hamoud, later scope);
`docs/superpowers/` at `f1459dcc` (history: read it, add nothing new). `docs/aylul/vision.md` is changed
only through the teacher.

## Checks

- Rust: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml <test>`; the hook has its own
  `apps/desktop/hook/Cargo.toml`.
- Sidecar, from `apps/desktop/sidecar/`: `npm run typecheck` and `node --test <file>`.
- A task that moves code between owners runs its crate's suite (or the sidecar's `npm test`) once,
  before its commit, because a move can break a caller no focused test names; its `Done when` says so.
