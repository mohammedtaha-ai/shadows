# Character (AYLUL) agents

Rules for `aylul-lead` (the AYLUL character lead) and the two agents it runs,
`character-builder` (Sonnet) and `character-fixer` (Opus). The repository rules in
[`AGENTS.md`](../../../apps/desktop/AGENTS.md) still apply. The rules shared by all three
sides (task shape, stages, builder and fixer rules, shared requests) are in
[`../AGENTS.md`](../AGENTS.md).

## The domain

Yours:
- `apps/desktop/src/features/companion/` (AYLUL presence; created by this side), except the
  colour values below
- `assets/aylul/README.md` (appearance sources; the artwork is now drawn by code, see there)
- `docs/aylul/character/` (plans, specs, stages), and the `contract.yaml` files and
  code-map rows of the code above

The colour table and palette belong to the desktop UI side. The ruling and its owner are in
[settings dialog backend §4](../../backend/specs/2026-10-21-settings-dialog-backend-design.md).

Not yours: `apps/desktop/src/` outside `features/companion/` (desktop UI), `src-tauri/`, `sidecar/`, `hook/` (native); `../shadows-backend` (Hamoud, later scope);
`docs/superpowers/` at `f1459dcc` (history: read it, add nothing new). `docs/aylul/vision.md` is changed
only through the teacher.

## Checks

From `apps/desktop/`: `npm run build` and the tests the task names (`node --test <file>`).
An asset change names the review it needs in `assets/aylul/README.md`.
