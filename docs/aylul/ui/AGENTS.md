# Desktop UI agents

Rules for **Masada** (the desktop UI lead) and the two agents she runs,
`frontend-builder` (Sonnet) and `frontend-fixer` (Opus). Since 2026-10-19 (the
human's ruling) Masada leads this side as well as the Shadows web client
([`docs/frontend/AGENTS.md`](../../frontend/AGENTS.md)); the former
`desktop-ui-lead`/`-builder`/`-fixer` roles are retired. The repository rules in
[`AGENTS.md`](../../../apps/desktop/AGENTS.md) still apply. The rules shared by all three
sides (task shape, stages, builder and fixer rules, shared requests) are in
[`../AGENTS.md`](../AGENTS.md).

## The domain

Yours:
- `apps/desktop/src/` except `features/companion/`
- `apps/desktop/tests/`, `apps/desktop/companion.html` and `apps/desktop/studio.html` (the
  main window is the shared `web/` build; there is no `index.html`)
- `docs/aylul/ui/` (plans, specs, stages), and the `contract.yaml` files and
  code-map rows of the code above
- the AYLUL panel (`features/companion/panel/`), whose tasks are U- tasks; the character side
  owns only the drawing inside it (agents and panel design §7.8)
- the character colour table and palette: their ruling and owner are in
  [settings dialog backend §4](../../backend/specs/2026-10-21-settings-dialog-backend-design.md);
  the table is `assets/aylul/character-colors.json`, the palette is
  `features/companion/renderer/palette.ts`

Not yours: `assets/aylul/` and `features/companion/` (character), `src-tauri/`, `sidecar/`, `hook/` (native); `../shadows-backend` (Hamoud, later scope);
`docs/superpowers/` at `f1459dcc` (history: read it, add nothing new). `docs/aylul/vision.md` is changed
only through the teacher.

## Checks

From `apps/desktop/`: `npm run build` (typecheck + bundle), `npm run typecheck:tests` and the tests
the task names (`node --test <file>`). The desktop CI gate runs `npm test`,
`npm run test:companion:unit` and `npm run test:companion:production`; a separate browser job runs
`npm run test:companion:browser`. `RECORD_PURPLE`, `EXPORT_COLORS` and native acceptance scripts
stay opt-in. Never the whole suite while working.
