# AYLUL desktop (imported from NYLRA)

This scope owns the existing React/Tauri desktop, Node sidecar and Rust hook.
It is independent of Shadows `AppCore` and of the root Cargo workspace.
Commands documented for `apps/desktop/` assume the Shadows repository root.

Read [AYLUL](../../docs/aylul/README.md),
[vision](../../docs/aylul/vision.md),
[source structure](../../docs/aylul/source-structure.md) and
[inspected code map](../../docs/aylul/codebase/README.md) before source edits.
Read the colocated contract before changing a module. Update its contract and
code-map row in the same change when its public behavior or ownership changes.
The [contract convention](../../../shadows-backend/docs/contracts/README.md)
and [template](../../../shadows-backend/docs/contracts/TEMPLATE.yaml) remain
externally owned references; do not invent a second contract format.

- Keep TypeScript/React presentation, Rust/Tauri platform integration, Node
  sidecar and `aylul-hook`; relocation is not a language or behavior rewrite.
- AYLUL is the main experience; loading the existing Shadows workspace remains
  planned integration. Hermes is the selected personal-pilot brain, not yet wired.
- Core owns its rules, plans and managed execution. The sidecar owns local agent
  observation and integration. Code intelligence has independent lifetime/state.
- UI calls typed Tauri boundaries; it does not access the sidecar or databases
  directly. Model output is never human confirmation. Preserve source scope,
  version and queued/accepted/completed distinctions; §1.7 and §24.6 remain binding.
- The character renderer/studio and native window are separate owners. Hiding or
  failing the character must preserve main, tray, monitoring and person actions.
- Keep completed auth/hook/companion behavior and tests. Do not continue the old
  backend-compatible relay/client roadmap or remove it as part of a path move.
- Explain edits first. Report actual local, browser-fixture, native and external
  acceptance separately. No personal hook installation, credential migration,
  push or deployment is implied by relocation.
- Use the current branch; no worktrees. Stage exact paths if a commit is requested.
  Preserve unrelated changes; do not switch, stash, reset or publish silently.

Owners: [UI](../../docs/aylul/ui/AGENTS.md),
[native/sidecar/hook](../../docs/aylul/native/AGENTS.md),
[character](../../docs/aylul/character/AGENTS.md).
Their internal agreements live in [AYLUL shared requests](../../docs/aylul/shared/README.md);
Core frontend/backend requests remain in `docs/shared/`.
Cloud wire schemas remain in `../shadows-backend`; import generated artifacts
through their contract, never sibling application source. Mobile is a future
direction; no mobile implementation was present in the imported snapshot.

Validate from the new paths: frontend build/focused Node and companion tests,
sidecar typecheck/tests, native/hook Cargo tests. Use debug native acceptance
until release validation is explicitly needed. The nine original Claude launcher
definitions remain at the preserved NYLRA checkout; no global agent configuration
was installed or overwritten by this import.
