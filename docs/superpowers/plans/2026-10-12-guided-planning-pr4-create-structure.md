# §23 PR 4 — Create Structure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The person presses **Create structure** on the Project map page and
Shadows writes the approved map into the project's empty directory: each
part's folders, its `contract.yaml`, the approved initial files, and a root
`SHADOWS.md`; a stopped write resumes; the stage moves to `roadmap`.

**Architecture:** A pure renderer turns the latest `design_map_version`
snapshot into a list of `(path, bytes)`. The Design service checks the gates,
records the intent (paths, hashes, map number, standards) in one write
transaction, writes each file to a temporary name and renames it, then
records success and a durable event in a second transaction. A resume
re-renders from the recorded map number. One HTTP route, no MCP tool.

**Tech Stack:** Rust (SQLx 0.9 SQLite, `sha2`, `yaml-rust2`, tokio fs), React + Vitest.

**Spec:** `docs/superpowers/specs/2026-10-08-guided-planning-design.md`
§23.6 (gate table), §23.7 (the write, and the 2026-10-12 decisions),
§23.10 test 4. Authority: `docs/superpowers/specs/2026-10-08-approval-authority-design.md`
§24.2 (Create structure is the person's only).

## Global Constraints

- Every cargo command is prefixed
  `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0`.
  Never build into the repo's `target/`. If a build fails with "Access is
  denied" on `shadows.exe`, the person's daemon holds it: stop and say so;
  do not kill it.
- No Rust line over 100 columns; `cargo fmt --all` before each commit.
- Never edit a shipped migration (`0001`–`0024`). New schema is `0025_structure_write.sql`.
- No new Rust integration test files: add modules under existing test
  directories (`crates/shadows-core/tests/design_parts/`) or tests to existing
  files.
- No prettier; no Python, sed or Node scripts editing source or docs.
- `npm run gen:api` (in `web/`) whenever `api/openapi.json` changes.
- Targeted tests while working; the full gate once, in Task 5.
- A change to a service updates its `contract.yaml` in the same commit.
- Commits end with your model's `Co-Authored-By` line. Do not push.
- No MCP tool writes to disk (§24.2). No Git: Shadows writes files only.
- Refusals are `STAGE_BLOCKED` (HTTP 409) with the reason, as §23.6.

## Rulings already made (do not reopen)

1. **Source of truth is the latest `design_map_version`.** Every approval and
   every hand edit inserts a version (`map_versions::insert_version`), so the
   latest row equals the live parts. Create structure renders that row's
   `parts_json`; a resume renders the recorded row's number, never the live one.
2. **Where a contract goes:** `<part's first folder>/contract.yaml`. A part
   with no folder refuses the write: `STAGE_BLOCKED: part has no folder: <title>`.
3. **Reserved paths.** `SHADOWS.md` at the root and `contract.yaml` directly
   in a part's first folder belong to Shadows. `map_rules::check_shape`
   refuses an initial file at either path (case-insensitive) from now on,
   and the write re-checks it, so a map approved before this PR refuses with
   `STAGE_BLOCKED: reserved path: <path>` and the Planner proposes a change.
4. **Windows device names.** `normalise_folder` also refuses a segment whose
   stem is `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9` or `LPT1`–`LPT9`
   (case-insensitive, with or without extension).
5. **contract.yaml content** (spec §23.7 decision): `title`, `kind`,
   `responsibility`, `serves`, `header`, `rules`, in that order, written with
   `yaml-rust2`'s emitter so quoting is correct, and a first comment line
   `# Written by Shadows from the approved map. Shadows is the source.`
6. **SHADOWS.md content:**

   ```markdown
   # <project name>

   Shadows wrote this structure from map version <n>.
   Standards: base v<base>, additions <id or "none">.
   Shadows is the source of truth for the vision, the map and each part's
   contract; these files are their output.
   ```
7. **One write at a time per daemon:** a `tokio::sync::Mutex<()>` held by the
   Design service for the whole Create structure call. Two clicks never
   interleave file writes.
8. **After success** the stage is `roadmap` (a new `Stage::Roadmap`), and a
   second Create structure refuses with `STAGE_BLOCKED: structure already
   written`. A replay of the same `command_id` returns the recorded result.
9. **Empty directory:** only `.git` (file or directory) may be present.
   Directories Shadows itself would create count as foreign on a first
   write, and as allowed on a resume only if they are recorded folders or
   ancestors of recorded paths.

## Review Focus

1. A map approved before this PR whose initial files include
   `backend/CONTRACT.yaml` (the trial project "CSV Tool" has one) refuses with
   the reserved path, writing nothing. Test in Task 2.
2. The daemon stops after some files: Create structure resumes, removes
   leftover temporary names, writes the rest and records success. Test in Task 2.
3. A file the person added after a stopped write, or a recorded file they
   changed, refuses the resume with the list and changes nothing. Test in Task 2.
4. The project directory is missing or is a file: refused with a clear
   reason, not a panic or a partial write. Test in Task 2.
5. Two concurrent Create structure calls: one writes, the other replays or
   refuses; files are written once. Test in Task 2.

---

### Task 1: Render the structure (pure)

**Files:**
- Create: `crates/shadows-core/src/design/structure.rs` — one job: turn a map
  snapshot into the files to write.
- Modify: `crates/shadows-core/src/design/map_rules.rs` (Rulings 3, 4),
  `crates/shadows-core/src/design/mod.rs` (`mod structure;`)
- Test: unit tests inside `structure.rs` and `map_rules.rs`
- Docs: `docs/codebase/README.md` owner row for `design/structure.rs`

**Interfaces:**
- Produces:

```rust
/// One file Create structure writes: a normalised relative path and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedFile { pub path: String, pub bytes: Vec<u8> }

pub(crate) struct Planned {
    /// Every folder, normalised, parents before children.
    pub folders: Vec<String>,
    /// SHADOWS.md, each part's contract.yaml, then the initial files.
    pub files: Vec<PlannedFile>,
}

pub(crate) struct RenderInput<'a> {
    pub project_name: &'a str,
    pub map_number: i64,
    pub parts: &'a [PartContent], // the snapshot's parts, map order
    pub standards_base: i64,
    pub standards_additions: Option<&'a str>,
}

/// `Err` carries the STAGE_BLOCKED reason text.
pub(crate) fn render(input: &RenderInput<'_>) -> Result<Planned, String>;
pub(crate) fn sha256_hex(bytes: &[u8]) -> String;
```

  Check how `parts_json` deserialises (`map_versions::rows` writes it): use
  the same row type to get each part's `PartContent`.

- [ ] **Step 1: Failing unit tests** in `structure.rs`:
  `renders_folders_contracts_files_and_shadows_md` (two parts; assert exact
  paths in order and the exact `contract.yaml` text of one part, Ruling 5);
  `a_part_without_a_folder_refuses`; `a_reserved_path_refuses` (initial
  file `backend/CONTRACT.yaml` with first folder `backend`);
  `shadows_md_names_map_and_standards`. In `map_rules.rs`:
  `reserved_initial_files_are_refused` and `windows_device_names_are_refused`
  (`nul`, `backend/con.txt`, `Com1`).
- [ ] **Step 2: Run, watch fail:** `cargo test -p shadows-core --lib structure map_rules`.
- [ ] **Step 3: Implement.** Re-run `normalise_folder` on every path inside
  `render` (defence: an older snapshot predates Ruling 4).
- [ ] **Step 4: Run** → PASS. `cargo test -p shadows-core --test design_parts` → PASS
  (Ruling 3 must not break existing map tests; fix fixtures that used a
  reserved path).
- [ ] **Step 5: Commit** `feat(design): render the approved map as files (§23.7)`.

### Task 2: Record and write the structure

**Files:**
- Create: `crates/shadows-core/migrations/0025_structure_write.sql`
- Create: `crates/shadows-core/src/design/store/structure.rs` — one job: the
  structure write's records (intent, success, reads).
- Create: `crates/shadows-core/src/design/structure_write.rs` — one job: put
  the planned files on disk safely (empty check, temp + rename, resume check).
- Modify: `crates/shadows-core/src/design/mod.rs` (the service method),
  `design/stage.rs` + `design/store/stage.rs` (`Stage::Roadmap`)
- Test: `crates/shadows-core/tests/design_parts/structure.rs` (new module in
  the existing directory; declare it in `design_parts.rs`)
- Contract: `crates/shadows-core/src/design/contract.yaml`

**Interfaces:**
- Consumes: Task 1's `render`, `Planned`, `sha256_hex`.
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StructureWritten {
    pub map_version: i64,
    pub paths: Vec<String>,
    pub completed_at: String,
    /// True when this call finished an earlier, stopped write.
    pub resumed: bool,
}

impl Design {
    /// §23.7: the person's Create structure. Never called from MCP.
    pub async fn create_structure(
        &self, command_id: String, project: &ProjectId,
    ) -> Result<StructureWritten, CoreError>;
}
```

  Migration:

```sql
-- §23.7: the intent is recorded before the first byte; success last.
CREATE TABLE design_structure_write (
    project_id TEXT PRIMARY KEY REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    map_number INTEGER NOT NULL,
    standards_version INTEGER NOT NULL,
    standards_additions_version_id TEXT NULL REFERENCES standards_additions_version(id),
    files_json TEXT NOT NULL CHECK (json_valid(files_json)), -- [{"path","sha256"}]
    folders_json TEXT NOT NULL CHECK (json_valid(folders_json)),
    started_at TEXT NOT NULL,
    completed_at TEXT NULL
);
```

  Order inside `create_structure` (Ruling 7 mutex held throughout):
  1. Replay: `classify` the command; a recorded one returns its result.
  2. Read in one reader transaction: project directory, project name, stage
     inputs, `map_revision`, the latest map version row, and the
     `design_structure_write` row.
  3. Gates (`STAGE_BLOCKED`): completed row → `structure already written`;
     `map_revision == 0` → `map not approved`; current missing mandatory
     kinds non-empty → `missing part: <kinds>` (§23.6: all kinds rechecked).
  4. No row: render from the latest version; check the directory exists and
     is a directory (else `STAGE_BLOCKED: project directory missing: <path>`);
     check emptiness (Ruling 9, else `directory not empty: <first 10 names>`);
     then in a write transaction re-read `map_revision` (changed → refuse
     `map changed; try again`) and insert the intent.
     Row exists, incomplete: render from its `map_number`; the resume check:
     recorded files whose hash matches are kept, leftover temporary names of
     recorded paths are removed, anything else (or a recorded path whose
     content differs) refuses with `STAGE_BLOCKED: unexpected files: <list>`
     and changes nothing.
  5. Create folders; for each file not already correct, write
     `<path>.shadows-tmp`, flush, rename onto `<path>`.
  6. Write transaction: set `completed_at`, append durable event
     `ProjectStructureWritten` (actor `Actor::user("local")`, payload
     `{project_id, map_version, paths}`), `record_command`.

  Every path joins the project directory after `normalise_folder`; assert the
  joined path `starts_with` the directory before touching it.

- [ ] **Step 1: Failing tests** in `design_parts/structure.rs` (temp dirs via
  the `tempfile` dev-dependency if present, else the fixture the file's
  siblings use; seed with `seed_map_for_tests`):
  1. `create_structure_writes_exactly_the_approved_files` (bytes equal the
     render; stage becomes `roadmap`; event appended).
  2. `a_directory_holding_only_git_is_accepted`; `a_non_empty_directory_refuses_and_writes_nothing`.
  3. `a_missing_or_file_directory_refuses`.
  4. `an_unapproved_map_or_missing_kind_refuses`.
  5. `a_second_call_refuses_and_a_replay_returns_the_result`.
  6. `a_stopped_write_resumes`: insert the intent and write half the files
     plus one `.shadows-tmp` through a test-support seam
     (`Design::create_structure_stopping_after(command, project, n)`,
     `#[cfg(feature = "test-support")]`), then call `create_structure`:
     `resumed == true`, temp gone, all files present.
  7. `a_foreign_or_changed_file_refuses_the_resume` (two cases, list named,
     nothing changed).
  8. `a_reserved_path_from_an_older_map_refuses` (Review Focus 1: seed a
     version whose part has initial file `backend/CONTRACT.yaml`).
  9. `concurrent_calls_write_once` (`tokio::join!` two calls with different
     command ids: one `Ok`, one `structure already written`).
- [ ] **Step 2: Run, watch fail:** `cargo test -p shadows-core --test design_parts structure`.
- [ ] **Step 3: Implement** migration, store, writer, service method,
  `Stage::Roadmap` (`stage::line`: `[Shadows] Stage: roadmap. The structure is
  written; outcomes and plans come next.`; the store returns `Roadmap` when a
  completed row exists). Update the storage schema inventory test for the
  new table.
- [ ] **Step 4: Run** → PASS; also `cargo test -p shadows-core --test design_vision --test contracts`.
- [ ] **Step 5: Contract and commit** `feat(design): Create structure writes the approved map, resumably (§23.7)`.

### Task 3: The route and the Planner's knowledge

**Files:**
- Modify: `crates/shadows-http/src/design.rs` (`POST /api/projects/{id}/structure`,
  body `{ "command_id": string }`, 200 `StructureWritten`, 409 `STAGE_BLOCKED`),
  `crates/shadows-http/src/openapi.rs` if routes are listed there
- Modify: `crates/shadows-core/src/harness/prompt.txt`: after the map lines add
  "Shadows writes each part's contract.yaml and a root SHADOWS.md itself; never
  propose files at those paths. Creating the structure is the person's button
  on the Project map page; you cannot do it."
- Test: `crates/shadows/tests/design_routes.rs`, `crates/shadows/tests/mcp_tools.rs`
  (still no tool containing `structure`), `crates/shadows/tests/openapi.rs`

- [ ] **Step 1: Failing route test** `create_structure_route_writes_and_refuses`:
  a seeded approved map in a temp project directory → 200 with paths; a
  second POST → 409 `STAGE_BLOCKED` "structure already written"; a project
  with a non-empty directory → 409 with "directory not empty".
- [ ] **Step 2: Run, watch fail:** `cargo test -p shadows --test design_routes`.
- [ ] **Step 3: Implement**; `npm run gen:api` in `web/`.
- [ ] **Step 4: Run** `cargo test -p shadows --test design_routes --test openapi --test mcp_tools --test planner_mcp` → PASS.
- [ ] **Step 5: Commit** `feat(http): POST /structure for the person's Create structure`.

### Task 4: The button

**Files:**
- Create: `web/src/app/design/create-structure.tsx` — one job: the Create
  structure control and its result.
- Modify: the Project map page component (find it from `web/src/app/design/`
  and the router; it renders `map-part-details.tsx`), and the stage header
  wherever `Stage` values are mapped to text (add `roadmap`).
- Test: `web/src/app/design/create-structure.test.tsx`

Behaviour: shown on the map page when the stage is `structure`. Disabled with
the reason while `missing` is non-empty. Pressing it opens a confirm naming
the project directory and the number of files; confirm POSTs with a fresh
command id. Success shows "Structure written · N files" and the paths in a
collapsible list; the page's stage refreshes (invalidate the stage and map
queries). A 409 shows the reason verbatim under the button. At stage
`roadmap` the control shows "Structure written" and no button.

- [ ] **Step 1: Failing tests:** enabled at `structure` with no missing;
  disabled with reason when missing; confirm → POST → success text and paths;
  409 reason shown; nothing rendered as a button at `roadmap`.
- [ ] **Step 2: Run, watch fail:** `npm run test -- create-structure`.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS; `npm run typecheck`, `npm run lint`.
- [ ] **Step 5: Commit** `feat(web): Create structure on the Project map page`.

### Task 5: Gate, acceptance, evidence

- [ ] **Step 1: Full gate** from the root, CLAUDE.md steps 1–6 with
  `RUST_LOG` unset, then `npm run test`, `typecheck`, `lint`, `build` in
  `web/`. Record counts.
- [ ] **Step 2: Browser acceptance on the person's own daemon** (port 4318
  via `.claude/launch.json`; if it must be rebuilt, ask the person to stop it
  first). Project "CSV Tool":
  1. Its approved map has `backend/CONTRACT.yaml`: Create structure refuses
     with the reserved path; the folder stays empty.
  2. Ask the Planner for a map change dropping that file; approve it.
  3. Create structure: the confirm names the directory; the files appear on
     disk (list them with `ls -R`); `backend/contract.yaml` matches Ruling 5;
     `SHADOWS.md` matches Ruling 6; the stage reads `roadmap`.
  4. Press again (reload first): no button; "Structure written".
- [ ] **Step 3: Evidence** `docs/evidence/2026-10-12-guided-planning-pr4.md`
  with the gate counts, the four steps' results and the file listing. Commit
  `docs: record PR 4 gate and acceptance`.
