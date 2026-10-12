# Guided planning PR 4 (Create structure) — gate and acceptance, 2026-10-12

Branch `next/guided-planning-pr4`. A measurement record, not design
authority. Codex executed Tasks 1–4 and the Task 5 gate (commits `1bae95e`
through `30008e6`); no independent review was dispatched. Claude reviewed
the writer and the service coordination, ran the browser acceptance and
fixed what it found.

## Gate (Codex)

`cargo fmt --check`, both Clippy runs, the `test-support` tree check and the
committed API passed; Rust 510 passed, 1 ignored; Web 264 passed, typecheck,
lint, build. Codex's rulings in its ledger: service coordination in
`design/structure_create.rs`; files published by hard link then unlink, so a
destination added after the check is never replaced; collision-free
temporary names; the total file count is read when the confirm opens.

## Review (Claude)

`structure_write.rs` reads the whole directory before deleting any
temporary file, refuses reparse points (symlinks, junctions), joins every
path through `normalise_folder` and checks it stays under the canonical
root. `structure_create.rs` holds the per-daemon lock for the whole call,
records the intent before the first byte, refuses a resume whose recorded
hashes or folders differ from the re-render, and records success last.

## Browser, on the person's own daemon

Port 4318 rebuilt from this branch; project "CSV Tool" (scratchpad
`csv-tool`, empty directory) from the PR 3b run.

1. Its approved map held `backend/CONTRACT.yaml` and
   `database/CONTRACT.yaml`. Create structure's confirm read "Write 11
   files"; confirming answered `reserved path: backend/CONTRACT.yaml` under
   the button; the directory stayed empty.
2. Asked to remove both files one part at a time, the Planner's two change
   proposals were each refused for the *other* part's file
   (`reserved path: database/CONTRACT.yaml` for the backend change, and the
   reverse). **Defect:** `map_rules::check_shape` applied the reserved-path
   check to every part, not only the written ones, so an older map could not
   be repaired change by change. Fixed in `447db7f` (the check now applies to
   written parts, like unknown items and rules); the unit test failed before
   the fix (`left: Err("reserved path: backend/CONTRACT.yaml")`) and passes
   after; `design_parts` 32/32, `design_vision` 21/21, `design_routes` 8/8,
   `workspace_mcp` 7/7, Clippy clean.
3. On the rebuilt daemon the Planner proposed both changes and approved
   them at the person's request (`planner:claude-code`). Create structure's
   confirm read "Write 9 files"; it wrote:

   ```text
   ./SHADOWS.md
   ./backend/contract.yaml
   ./backend/converter/__init__.py
   ./backend/converter/cli.py
   ./backend/converter/service.py
   ./backend/pyproject.toml
   ./database/contract.yaml
   ./database/storage/__init__.py
   ./database/storage/files.py
   ```

   `SHADOWS.md` names map version 3 and standards base v1, additions none.
   `backend/contract.yaml` starts with the "Written by Shadows" comment and
   holds title, kind, responsibility, serves (PU1, U1, G1, B1, T1), header
   and rules (S1, S3, S5, S6). The stage route answered `roadmap`.
4. After a reload the map page shows "Structure written" and no Create
   structure button.
