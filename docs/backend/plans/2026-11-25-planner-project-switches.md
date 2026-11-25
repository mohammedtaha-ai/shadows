# The Planner's project switches (§23.9 PR 3c)

- **Date:** 2026-11-25. Agreed with Mohammed the same day.
- **Owner spec:** §24.3, "The person may widen the Planner's reach, never its hands"
  (`docs/core/specs/2026-10-08-approval-authority-design.md`). Its delivery is fixed by the
  probe `docs/evidence/2026-11-25-planner-switches-probe.md`.
- **Tasks:** B-230 to B-235 in `docs/backend/stages.md`. The web's section is the shared
  request `docs/shared/requests/2026-11-25-planner-project-switches.md`, which Masada writes
  (Mohammed, 2026-11-25).
  Only B-234 changes what the frontend sees; it waits for that request to be `accepted`.
- **Checked against the code at `e7b9332`** (2026-11-25 review): every path, signature and
  line named below was read there.

## What the person gets

A project's settings get a section, "What the Planner may use". It lists AYLUL's extension
library (the same skills and MCP servers AYLUL's agents switch), with one switch per skill
and per MCP server for this project, and one switch "Read the project's CLAUDE.md". Every
switch starts off. A Planner conversation of that project on Claude Code receives what is
switched on when its session is next opened:

- A skill or a server reaches every opening after the change: a new conversation, a resume
  after idle, a fork.
- `CLAUDE.md` rides in the appended instructions, which a resumed or forked Claude session
  keeps from its first opening (`crates/core-harness/src/setup.rs`, module comment and
  `keeps_parent_prompt`). So it reaches a new conversation; an existing one keeps what it
  opened with. (Mohammed, 2026-11-25: accepted for now.)

A session already open keeps what it opened with.

## Reuse first

Nothing here is a second copy of AYLUL's machinery. Each part extends the AYLUL code that
already does the same job:

| Job | AYLUL today | This plan |
|---|---|---|
| The library | `Extensions` (`crates/core-extensions`) | unchanged |
| A switch over an item | `ExtensionsStore::toggle`, `agent_library` over `aylul_agent_extension`, `aylul_agent_mcp` | the same two functions take a `Holder` (agent or project); two new tables mirror the agent ones |
| Resolving switched-on MCP servers with their secrets | `Extensions::opening_snapshot`, `resolve_mcp_servers` | the same, for a `Holder`; a project has no tool rules |
| A server named `shadows` | `Extensions::prepare_mcp` refuses the name (`mcp_input_is_checked`) | relied on; Harness adds no check |
| Copying a skill out of the library | `extension_home.rs` `copy` (checks the id and `safe_name`, then `Extensions::copy_tree`), `Homes::check_local` | `copy` becomes `Homes::copy_item`, called by the new `planner_skills.rs` |
| `McpServerConfig` → `McpServerSpec` | the `map` at the end of `Homes::write_extensions` | moved into `Homes::mcp_spec`, which both openings call |
| The AYLUL home root | `SessionsConfig::aylul` (`AylulConfig::home_root`), as `assistant.rs` passes it | the same value, handed to `Setups` |
| Route shapes | `aylul_extensions::agent`, `toggle`, `Toggle` | the same shapes under `/api/projects/{id}/…`, returning `AgentExtensions` and `AgentItem` |
| The web's switch list | the agent form's extensions section | reused by Masada (shared request) |

## Decisions

1. **One library.** The switches are over `Extensions`' library. Shadows never reads the
   person's own Claude settings. (Mohammed, 2026-11-25.)
2. **Skills and MCP servers only.** A library plugin is not offered to the Planner: a plugin
   runs its own hooks (probe R3, R7). `kind` `plugin` is refused with
   `ExtensionsError::Invalid("plugins are not offered to the Planner")`. A project view
   omits the `plugins` list (sent empty).
3. **No per-project tool rules.** A switched-on server's tools are not pre-approved and no
   rule is written: like `Bash`, each call goes through the conversation's mode (`auto`
   reviews it, `acceptEdits` asks the person). Per-tool rules for a project are a row in
   `docs/core/future.md`.
4. **Owners.** `Extensions` owns the project switches over its library, beside the agent
   switches. `Projects` owns the `CLAUDE.md` switch, a project setting. `Harness` applies
   both when it opens a Planner session. No new service.
5. **Delivery, as §24.3 states it.** `settingSources: []` and `strictMcpConfig: true` stay.
   - MCP: each switched-on server is passed in the session's `mcpServers` after `shadows`.
     No library server can be named `shadows`: `Extensions` refuses that name when the
     server is added or edited.
   - Skills: a plugin folder Harness writes (below), sent as `plugins`, with `Skill`
     appended to the tools and `skills` naming each skill `shadows-skills:<name>`. With no
     skill on, none of the three is sent and the tools are exactly `PLANNER_TOOLS`.
   - `CLAUDE.md`: read from the session's directory and appended to the Planner's
     instructions (below).
6. **Claude Code only.** A Codex Planner (`harness == policy::CODEX`) opens exactly as today.
   Codex's delivery is a row in `docs/core/future.md`.
7. **Executors and AYLUL are unchanged.** An executor's setup and an agent's opening do not
   read the project switches, and every AYLUL test passes unchanged.
8. **No AYLUL, no library switches.** When AYLUL is not configured, `project_extensions`
   answers an empty view, `set_project_extension` answers `NotFound`, and the opening adds no
   server or skill, whatever rows an earlier AYLUL run left. The `CLAUDE.md` switch works
   without AYLUL.

## B-230 · Extensions: project switches over the library

**Migration** `crates/core-db/migrations/0080_planner_project_switches.sql` (0080 is free:
the last is `0079_run_recheck_asked.sql`), exactly:

```sql
CREATE TABLE project_extension (
    project_id TEXT NOT NULL REFERENCES project(id),
    extension_id TEXT NOT NULL REFERENCES aylul_extension(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    PRIMARY KEY (project_id, extension_id)
);
CREATE TABLE project_mcp (
    project_id TEXT NOT NULL REFERENCES project(id),
    mcp_id TEXT NOT NULL REFERENCES aylul_mcp_server(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    PRIMARY KEY (project_id, mcp_id)
);
ALTER TABLE project ADD COLUMN planner_reads_claude_md INTEGER NOT NULL DEFAULT 0
    CHECK (planner_reads_claude_md IN (0, 1));
```

(The last statement is B-231's column, in the same migration so the two tasks share one
number. B-230 writes the file; B-231 only reads and writes the column.) A project is only
ever soft-removed (`removed_at`), so the `REFERENCES project(id)` rows stay with it.

The migration catalog is pinned: append
`<sha384sum of the file> 0080_planner_project_switches.sql` as the last line of
`crates/core-db/tests/fixtures/base_migrations.sha384` (the checksum is `sha384sum` of the
LF file, as for 0079), change `assert_eq!(expected.len(), 79)` to `80` in
`crates/core-db/tests/migration_checksums.rs`, and in `crates/core-db/contract.yaml` the test
line `All 74 versions` becomes `All 80 versions`.

The architecture ratchet names each table's owner: add `project_extension :: extensions`
and `project_mcp :: extensions` to `crates/shadows/tests/architecture/table-owners.txt`
(beside the `aylul_*` rows).

**Model** (`crates/core-extensions/src/model.rs`): add

```rust
/// Who a switch belongs to: one AYLUL agent, or one project's Planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Holder {
    Agent(String),
    Project(core_projects::ProjectId),
}
```

It is internal: no public signature names it, and `lib.rs` does not re-export it.

`core-projects` moves from `[dev-dependencies]` to `[dependencies]` in
`crates/core-extensions/Cargo.toml` (`core-projects` depends on `core-db` and
`shadows-agent` only, so there is no cycle). `Extensions` gets no `Projects` handle: it
calls the associated `Projects::require_live_in`, as it calls `Agents::require_live_in`.

**Store** (`crates/core-extensions/src/store/mod.rs`):

- `agent_library(slug)` becomes `holder_library(&Holder)`. Liveness first: for an agent
  `Agents::require_live_in` as today; for a project `Projects::require_live_in`, with
  `StorageError::NotFound(_)` mapped to `ExtensionsError::NotFound` and any other error
  kept (`.into()`). Each `SELECT` is written twice as a literal, one per holder, picked by a
  `match` on the holder; never `format!` a table name into SQL (the architecture ratchet
  refuses interpolated SQL). The project pair, binding `project.as_str()`:

  ```sql
  SELECT e.*,COALESCE(p.enabled,0) AS enabled FROM aylul_extension e
   LEFT JOIN project_extension p ON p.extension_id=e.id AND p.project_id=?
   ORDER BY e.name,e.id
  SELECT e.*,COALESCE(p.enabled,0) AS enabled FROM aylul_mcp_server e
   LEFT JOIN project_mcp p ON p.mcp_id=e.id AND p.project_id=? ORDER BY e.name,e.id
  ```

  For a project, plugin rows are dropped, so `plugins` is empty.
- `toggle(c, slug, kind, id, enabled)` becomes `toggle(c, &Holder, kind, id, enabled)`, the
  same body: classify, liveness as above, then the item check and the upsert, each upsert a
  literal per holder:

  ```sql
  INSERT INTO project_mcp(project_id,mcp_id,enabled) VALUES (?,?,?)
   ON CONFLICT(project_id,mcp_id) DO UPDATE SET enabled=excluded.enabled
  INSERT INTO project_extension(project_id,extension_id,enabled) VALUES (?,?,?)
   ON CONFLICT(project_id,extension_id) DO UPDATE SET enabled=excluded.enabled
  ```
- `remove` already deletes the item, and `ON DELETE CASCADE` removes the project rows
  (foreign keys are on: `Storage::open`). Add no code.

**Service** (`crates/core-extensions/src/lib.rs`, `mcp.rs`):

- `agent_extensions(slug)` and `set_agent_extension(..)` keep their signatures and call the
  holder versions with `Holder::Agent`. Their command kind and fingerprint stay
  (`extension.toggle`, `{slug,kind,id,enabled}`), so stored replays still match.
- New `pub async fn project_extensions(&self, project: &ProjectId) -> Result<AgentExtensions, ExtensionsError>`:
  `AgentExtensions::default()` when `home_root` is `None`, else `holder_library`.
- New `pub async fn set_project_extension(&self, command_id: String, project: &ProjectId, kind: ExtensionKind, id: &str, enabled: bool) -> Result<AgentItem, ExtensionsError>`,
  in this order: `kind == Plugin` returns
  `ExtensionsError::Invalid("plugins are not offered to the Planner".into())`;
  `home_root` `None` returns `ExtensionsError::NotFound`; then it takes the guard and builds
  the command, kind `extension.project_toggle`, params
  `{"project":project,"kind":kind,"id":id,"enabled":enabled}`, and calls `toggle`.
- `opening_snapshot(slug)` keeps its signature; add
  `pub async fn project_snapshot(&self, project: &ProjectId) -> Result<AgentExtensionSnapshot, ExtensionsError>`.
  Both call one private `snapshot(&Holder)` (guard, `holder_library`,
  `resolve_mcp_servers`). `resolve_mcp_servers` takes `&Holder`: for an agent it reads
  `tool_rules(slug, id)` as today; for a project `rules` is empty.

**Tests** in `crates/core-extensions/tests/library.rs`, through the public API, using the
file's `world()` and `source()`. A project comes from
`Projects::new(w.storage.clone()).create(cmd, slug, name, dir)` with `dir` a folder under
`w._tmp`; an MCP item from the file's `mcp_input` helper and `add_mcp`.

- `a_project_switch_is_independent_of_agents`: one skill on for agent `agent` and off for
  project P, then the reverse; `agent_extensions` and `project_extensions` each read their
  own.
- `a_project_switch_refuses_a_plugin`: `Invalid`, and the project view is unchanged.
- `a_project_switch_on_an_unknown_project_or_item_is_not_found`: an unknown
  `ProjectId::from_recorded`, a removed project (`Projects::mark_removed_in` in a
  `write_txn`) and an unknown item id are each `NotFound`, for both `project_extensions`
  (projects only) and `set_project_extension`.
- `a_project_switch_replays_and_conflicts`: same command id and body answers the first
  result; same id with `enabled` flipped is `ExtensionsError::Storage(StorageError::CommandConflict)`.
- `deleting_an_item_removes_its_project_switch`: a skill and a server switched on for P,
  then deleted; `SELECT COUNT(*)` of `project_extension` and `project_mcp` is 0.
- `a_project_snapshot_resolves_only_switched_on_servers_with_no_rules`: two servers, one on
  for P; the agent has a tool rule on it; `project_snapshot(P).mcp` holds only that server,
  with its value resolved and `rules` empty.
- `without_aylul_a_project_view_is_empty_and_a_switch_not_found`: an `Extensions` built with
  `home_root` `None` over the same storage, after an item exists.
- Every existing test in the file passes unchanged.

**Contract** `crates/core-extensions/contract.yaml`: the header line names project switches;
`Holder` under `shapes` (crate-private); `project_extensions`, `set_project_extension`,
`project_snapshot` under `functions`; `project_snapshot` under `called_by_other_services`
(Harness, the Planner opening); the rules above under `obligations` with their test names;
a `dependencies` entry `projects: crates/core-projects/contract.yaml`.

**Touch:** `crates/core-db/migrations/0080_planner_project_switches.sql`,
`crates/core-db/tests/fixtures/base_migrations.sha384`,
`crates/core-db/tests/migration_checksums.rs`, `crates/core-db/contract.yaml`,
`crates/shadows/tests/architecture/table-owners.txt`,
`crates/core-extensions/{Cargo.toml,contract.yaml}`,
`crates/core-extensions/src/{lib.rs,model.rs,mcp.rs,store/mod.rs}`,
`crates/core-extensions/tests/library.rs`.
**Done when:** `cargo test -p core-extensions`, `cargo test -p core-db --test migration_checksums`,
`cargo test -p shadows-core --test storage_contract` and
`cargo test -p shadows --test architecture` pass.

## B-231 · Projects: the CLAUDE.md switch

No wire change here: `Project` gains its field in B-234, with the routes, once the shared
request is accepted.

- New `pub async fn set_planner_claude_md(&self, command_id: String, project: &ProjectId, enabled: bool) -> Result<Project, ProjectsError>`
  in `crates/core-projects/src/lib.rs`, after `set_modes`, in its shape: command kind
  `project.planner_claude_md`, params `{"project":project,"enabled":enabled}`, then the
  store's new `set_planner_claude_md(&c, project, enabled) -> Result<Project, StorageError>`
  in `crates/core-projects/src/store/mod.rs`, a copy of the store's `set_modes` flow:
  classify (scope `"Project"`, the id; a replay answers `load_project`) →
  `Projects::require_live_in` → `UPDATE project SET planner_reads_claude_md = ? WHERE id = ?`
  → `record_command` (scope `"Project"`, output kind `"Project"`, the id) →
  `load_project`. No journal event, like the extension switches.
- New `pub async fn planner_reads_claude_md_in(conn: &mut SqliteConnection, project: &ProjectId) -> Result<bool, StorageError>`
  in the store's `impl Projects`, beside `require_live_in`: `require_live_in` first
  (`NotFound("project")` for an unknown or removed project), then the column. It is the one
  read other services call; Harness calls it (B-233).
- Tests in `crates/core-projects/tests/project_contract.rs`, with the file's `dir()`, over
  `Projects::create`:
  - `the_claude_md_switch_starts_off_and_is_set`: a new project reads `false` through
    `planner_reads_claude_md_in` on a reader connection; after `set_planner_claude_md(.., true)`
    it reads `true`, and after `false`, `false`.
  - `the_claude_md_switch_replays_and_conflicts`: the same id and body answers the same
    project; the same id with `enabled` flipped is
    `ProjectsError::Storage(StorageError::CommandConflict)`, and
    the column is unchanged.
  - `the_claude_md_switch_on_a_removed_project_is_not_found`: after
    `Projects::mark_removed_in` in a `write_txn`, the setter is
    `ProjectsError::Storage(StorageError::NotFound("project"))` and the read is
    `StorageError::NotFound("project")`.
- Contract `crates/core-projects/contract.yaml`: `set_planner_claude_md` under `service`,
  the store's `set_planner_claude_md` and `planner_reads_claude_md_in` under `store`,
  `planner_reads_claude_md_in` under `shared_in_transaction` and `called_by_other_services`
  (Harness, the Planner opening), the rules with the three tests under `obligations` and
  `tests`; the `Replay identity` rule names `"project.planner_claude_md"` params
  `{ project, enabled }`.

**Touch:** `crates/core-projects/{contract.yaml,src/lib.rs,src/store/mod.rs,tests/project_contract.rs}`.
**Done when:** `cargo test -p core-projects` passes. Needs B-230 (its migration).

## B-232 · The agent's session options: plugins and skills

In `crates/shadows-agent/src/connection.rs`, `SessionSetup` gains:

```rust
/// Local plugin folders the session loads (`_meta.claudeCode.options.plugins`),
/// each an absolute path as text. Empty sends none.
pub plugins: Vec<String>,
/// The only skills the session lists (`_meta.claudeCode.options.skills`).
/// `None` sends none.
pub skills: Option<Vec<String>>,
```

The paths are text so `meta()` cannot fail: Harness, which builds the folder, refuses a
path that is not valid UTF-8 (B-233).

`SessionSetup::meta()` in `crates/shadows-agent/src/acp/session.rs` inserts into `options`
`"plugins": [{"type":"local","path":<path>}, …]` when `plugins` is not empty and
`"skills": [...]` when `skills` is `Some`. Nothing else changes.

`SessionSetup` derives `Default`; the three literals that name every field add
`plugins: vec![], skills: None`: `crates/core-harness/src/assistant.rs` (`aylul_setup`) and
`crates/core-harness/src/setup.rs` (`for_opening` and `for_executor`). The others use
`..Default::default()` and need no change.

Tests in `crates/shadows-agent/src/acp_tests.rs`, beside `aylul_meta_loads_only_user_settings`:

- `plugins_and_skills_reach_the_options`: a setup with `tools: Some(vec![])`, one plugin
  path and `skills: Some(vec!["shadows-skills:hello"])`; `meta()`'s
  `claudeCode.options.plugins == [{"type":"local","path":<it>}]` and
  `skills == ["shadows-skills:hello"]`.
- `no_plugins_or_skills_sends_neither`: a setup with `tools: Some(vec![])` only; the options
  have no `plugins` and no `skills` key.

Contract `crates/shadows-agent/contract.yaml`: the two fields and the two rules with their
tests.

**Touch:** `crates/shadows-agent/{contract.yaml,src/connection.rs,src/acp/session.rs,src/acp_tests.rs}`,
`crates/core-harness/src/{assistant.rs,setup.rs}` (only the two new fields in the literals).
**Done when:** `cargo test -p shadows-agent` passes and
`cargo check --workspace --all-targets --features fake-acp/test-support` builds.

## B-233 · Harness: the Planner opens with the project's switches

Needs B-230, B-231, B-232.

1. **Wiring.** `Setups::new` (`crates/core-harness/src/setup.rs`) takes two more
   arguments, kept as private fields: `extensions: Arc<core_extensions::Extensions>` and
   `aylul_root: Option<PathBuf>`. `Sessions::with_owners`
   (`crates/core-harness/src/sessions.rs`) binds `let extensions = owners.extensions.clone();`
   before its `Self { .. }` literal (which moves `owners`) and passes it with
   `config.aylul.as_ref().map(|a| a.home_root.clone())`, the root `assistant.rs` already
   hands `write_extensions`. `Setups` gets no `Projects` handle and `Extensions` no new
   accessor.
2. **Signature.** `for_opening(thread, project, kind, harness)` gains a last argument
   `cwd: &Path`, the directory the session opens in; both calls in
   `crates/core-harness/src/live.rs` (the Codex one and the Claude one) pass `&cwd`.
3. **Where.** Only the Planner branch of `for_opening` (`ThreadKind::Planning`), and only
   when `harness == policy::CLAUDE_CODE`. The executor branch returns before it, as today,
   and Codex never enters it.
4. **When.** After the standards read and before the grant is issued, so a failure leaves
   nothing issued (`forget` already ran at the top). Any error fails the opening with its
   text (`map_err(|e| e.to_string())`), as the other steps do.
5. **The `CLAUDE.md` switch** is read with `Projects::planner_reads_claude_md_in` on the
   reader snapshot `for_opening` already opens for `Standards::current_in`, before its
   `commit`.
6. **MCP.** Only when `aylul_root` is `Some`:
   `let snapshot = self.extensions.project_snapshot(project).await?` (hold it until the
   skills folder is written, then drop it). Each server converts with `Homes::mcp_spec`:
   move the `map` at the end of `Homes::write_extensions`
   (`crates/core-harness/src/extension_home.rs`) into
   `impl Homes { pub(super) fn mcp_spec(server: core_extensions::McpServerConfig) -> McpServerSpec }`
   and call it from both. Append the specs to `mcp` after `shadows`.
7. **Skills.** New file `crates/core-harness/src/planner_skills.rs`, one job: a Planner
   session's skills folder. It holds `impl Setups` with two private methods:
   - `fn write_skills(&self, root: &Path, thread: &ThreadId, skills: &[Item]) -> Result<Option<String>, HarnessError>`:
     with no skill, `None` and nothing written. Otherwise the folder is
     `<root>/planner-skills/<thread id>/`; `Homes::check_local` on `<root>/planner-skills`
     and on the folder; it writes `.claude-plugin/plugin.json` =
     `{"name":"shadows-skills","version":"0.0.0","description":"Skills this project gives its Planner"}`
     and copies each skill to `skills/<name>/` with `Homes::copy_item(root, target, item)`.
     That is the private `copy` of `extension_home.rs`, made
     `impl Homes { pub(super) fn copy_item(..) }` (a free function with a visibility
     modifier is an architecture finding); it already refuses an id that is not a UUID and
     a name `safe_name` refuses, and its AYLUL callers switch to it. On a failure the
     folder is removed. It answers the folder's path as text, or
     `HarnessError::ProfileInvalid("the skills folder path is not valid UTF-8")`.
   - `fn remove_skills(&self, thread: &ThreadId)`: removes `<root>/planner-skills/<thread id>/`
     when `aylul_root` is `Some`, ignoring any error. `Setups::forget` calls it, so the
     folder goes when its session closes and before every new opening of the thread; two
     conversations of one project never share a folder.
8. **Setup.** When the folder is `Some(dir)`: `plugins = vec![dir]`, `tools` =
   `PLANNER_TOOLS` + `"Skill"`, `skills = Some(names.map(|n| format!("shadows-skills:{n}")))`.
   Otherwise the three are as today.
9. **CLAUDE.md.** When the switch is on: read `cwd.join("CLAUDE.md")` only if
   `std::fs::symlink_metadata` says it is a regular file (not a symlink) of at most
   65 536 bytes and its bytes are valid UTF-8. Then append, after the project instructions:
   `\n\n## The project's CLAUDE.md\n\n{text}`. Otherwise append nothing and log
   `tracing::info!(project_id = %project, reason, "planner.claude_md_skipped")` with
   `reason` one of `missing`, `not_a_file`, `too_large`, `not_utf8`, `unreadable`
   (the harness's `"planner.…"` message style, as `planner.grant_revoke_failed`).
10. **Held setup.** `Held` records nothing new: a switch change reaches the next opening,
    not a live session.

**Tests**, a new file `crates/shadows/tests/planner_switches.rs`, over `fake_acp`'s
`report` and `report-mcp` prompts (no fake-acp change: `report` already echoes
`_meta.claudeCode.options` whole). Its fixture:

- `#[path = "fixtures/app.rs"] mod app;`, then
  `app::test_app_with(tmp.path(), SessionsConfig { mcp_url: Some(acp::MCP_URL.into()), aylul: Some(AylulConfig { home_root: tmp.path().join("aylul"), project_slug: "aylul".into() }), ..acp::test_config() }, acp::MCP_URL).await.owning(tmp)`
  (with `aylul` set the fixture gives `Secrets` a key, so a server's values resolve).
- A skill: `SKILL.md` = `"---\nname: hello\ndescription: Test greeting\n---\nsay hi\n"` in a
  folder under the temp dir, added with `app.core.extensions().add_extension(..)`. A server:
  `add_mcp` with a stdio transport (`command` `"node"`, one env value). Switches through
  `set_project_extension` and `projects().set_planner_claude_md` (the routes come in B-234).
- `report`: `start_and_finish(&app, "report", default_settings())`, then
  `last_agent_entry(&app).await.body` parsed as JSON, as `planner_mcp.rs`'s `report` does;
  `report-mcp` likewise. A new conversation: `app::create_thread(&app, json!({"command_id":..,"title":..}))`
  and `start_and_finish_on` on it.

Tests:

- `with_nothing_switched_on_the_planner_opens_as_before`: with AYLUL configured and an item
  in the library, `options` equals exactly
  `{"allowedTools":["mcp__shadows__*"],"tools":[the nine PLANNER_TOOLS],"settingSources":[],"strictMcpConfig":true}`
  (no `plugins`, no `skills`), only `shadows` in `report-mcp`, and no
  `aylul/planner-skills/<thread>` folder.
- `a_switched_on_skill_reaches_the_planner_as_a_plugin`: `options.plugins` names one folder
  holding `.claude-plugin/plugin.json` and `skills/hello/SKILL.md`, `options.tools` ends
  with `"Skill"`, `options.skills == ["shadows-skills:hello"]`; after
  `app.sessions.close_session(&thread)` the folder is gone.
- `a_switched_on_server_reaches_the_planner_after_shadows`: `report-mcp` lists `shadows`
  then the server, with its env value present.
- `a_switched_off_item_and_a_library_plugin_never_reach_the_planner`: a skill and a server
  switched on then off, and a plugin in the library (refused for the project); the options
  are as in the first test and `report-mcp` lists only `shadows`.
- `the_claude_md_switch_appends_the_file`: with `CLAUDE.md` in the project directory and the
  switch on, `append` ends with `"\n\n## The project's CLAUDE.md\n\n<text>"`; with it off
  (a new conversation), `append` does not contain the text.
- `a_claude_md_that_is_too_large_or_a_symlink_is_skipped`: a 65 537-byte file is not
  appended; on Unix only (`#[cfg(unix)]` on that half), a symlink named `CLAUDE.md` is not
  appended either.
- `a_switch_change_reaches_the_next_opening_only`: a server switched on while the session
  is open: the next `report-mcp` on that thread lists only `shadows`; a new conversation
  lists both.

Codex and executors are held by the branch in step 3, not by a new test; their existing
tests (`codex_session`, `executor_session`) must pass unchanged (Mohammed, 2026-11-25).

Contract `crates/core-harness/contract.yaml`: the module `planner_skills.rs: Planner
session skills folder` under `modules`; the opening rules above with these test names; the
`planner.claude_md_skipped` log; `Homes::copy_item` and `Homes::mcp_spec`. Code map
`docs/codebase/README.md`: a row
`| \`crates/core-harness/src/planner_skills.rs\` | Planner session skills folder | \`crates/core-harness/contract.yaml\` |`.

**Touch:** `crates/core-harness/{contract.yaml,src/lib.rs,src/setup.rs,src/sessions.rs,src/live.rs,src/extension_home.rs,src/planner_skills.rs}`,
`crates/shadows/tests/planner_switches.rs`, `docs/codebase/README.md`.
**Done when:** `cargo test -p shadows --test planner_switches --test planner_mcp --test codemap --test architecture`
and `cargo test -p shadows-core --test aylul_session --test aylul_extensions --test executor_session --test codex_session`
pass.

## B-234 · Routes, and the switch on `Project`

Only after the shared request is `accepted`: this task changes what the frontend sees.

**`Project` gains the switch.** `Project` (`crates/core-projects/src/model.rs`) gains, after
`assistant`:
`/// Whether the Planner on Claude Code receives the project directory's CLAUDE.md.`
`pub planner_reads_claude_md: bool`. In `crates/core-projects/src/store/mod.rs`, the three
`SELECT`s that build a `Project` (lines 98, 119 and 375 at `e7b9332`) add
`planner_reads_claude_md` after `assistant`, `ProjectRow` gains a seventh `bool`, and
`project()` sets the field. `crates/core-projects/contract.yaml`: the field under
`shapes.Project`.

**Routes.** In a new file `crates/shadows-http/src/planner_switches.rs` (one job: the
project's Planner switch routes), declared `mod planner_switches;` and registered with
`routes!` in `crates/shadows-http/src/lib.rs` beside the project routes, tag `projects`, in
`aylul_extensions.rs`'s shape and reusing its `Toggle` body (its two fields become
`pub(super)`; the struct already is):

| Route | Calls | Answers |
|---|---|---|
| `GET /api/projects/{id}/planner-extensions` | `extensions().project_extensions` | 200 `AgentExtensions`; 404 |
| `PUT /api/projects/{id}/planner-extensions/{kind}/{item}` with `Toggle` | `extensions().set_project_extension` | 200 `AgentItem`; 400 a plugin; 404; 409 |
| `PUT /api/projects/{id}/planner-claude-md` with `Toggle` | `projects().set_planner_claude_md` | 200 `Project`; 404; 409 |

`{id}` is a `Path<ProjectId>`, `{kind}` an `ExtensionKind`. Each handler calls one method and
shapes its answer; the error mapping is the existing `From<ExtensionsError>` and
`From<ProjectsError>` for `Failure` (`crates/shadows-http/src/failure/other.rs`):
`Invalid` → 400, `NotFound` → 404, `CommandConflict` → 409.

Tests in `crates/shadows/tests/planner_switches.rs` (the same file, over `app::call`):
`the_routes_list_and_switch_a_project_s_items`,
`the_routes_refuse_a_plugin_and_an_unknown_project`,
`the_claude_md_route_sets_the_project` (the answer's `planner_reads_claude_md` is `true`,
and `GET /api/projects` lists it). Regenerate
`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`. Contract
`crates/shadows-http/src/contract.yaml`: the three routes. Code map
`docs/codebase/README.md`: a row
`| \`crates/shadows-http/src/planner_switches.rs\` | Planner switch HTTP translation | \`crates/core-extensions/contract.yaml\` |`.

**Touch:** `crates/core-projects/{contract.yaml,src/model.rs,src/store/mod.rs}`,
`crates/shadows-http/{src/contract.yaml,src/lib.rs,src/planner_switches.rs,src/aylul_extensions.rs}`,
`crates/shadows/tests/planner_switches.rs`, `api/openapi.json`, `docs/codebase/README.md`.
**Done when:** `cargo test -p core-projects` and
`cargo test -p shadows --test planner_switches --test openapi --test codemap --test architecture`
pass.

## B-235 · Live acceptance on Claude

A live test `crates/shadows/tests/planner_switches_live.rs`, `#[ignore]` like the other
`*_live.rs`, on `fixtures/live.rs`'s `live_adapter()` (the pinned adapter on MiniMax) through
`app::test_app_on`, with AYLUL configured as in B-233. Switches are set through the service
methods (`set_project_extension`, `set_planner_claude_md`), so it does not need B-234. It
plants the probe's fixtures: a `CLAUDE.md` with a code word; a project
`.claude/settings.json` with a `SessionStart` hook that touches a file; a library skill
whose `SKILL.md` holds a second word; a library stdio MCP server named `kestrel` running
`crates/core-extensions/tests/fixtures/mcp.cjs` (`command` `"node"`, the script's absolute
path as its argument; its tools are `read` and `write`). It switches the skill, the server
and `CLAUDE.md` on, opens a new Planner conversation with `planner_turn`, and asks for the
code word, its skills and its MCP tools (ToolSearch "kestrel"). It asserts the reply names
`shadows-skills:<skill>` and `mcp__kestrel__read` and quotes the code word, and that the
hook's file does not exist. It then switches all three off, opens a new conversation, and
asserts none of the three is named. Run here with `API_KEY_MINIMAX` read as `stages.md`'s
rule says, `SHADOWS_NODE_PATH` and `SHADOWS_HARNESS_PATH` set to this machine's `node` and
`claude`:
`cargo test -p shadows --test planner_switches_live -- --ignored --test-threads=1`. Its
result is recorded in `docs/evidence/2026-11-25-planner-switches-probe.md` under a new
heading "Through Shadows".

**Touch:** `crates/shadows/tests/planner_switches_live.rs`, that evidence file.
**Done when:** the live test passes here, with its output in the evidence.

## Order and checks

B-230 and B-232 first; B-231 after B-230; B-233 next; B-234 once the request is accepted;
B-235 last (it needs B-233 only). Per task, only the tests it names, plus what
`docs/backend/AGENTS.md` rule 5 runs before every commit. Before the PR, the full gate from
CLAUDE.md and one branch review. The browser check of the page waits for Mohammed's
machine. With the tasks in `stages.md`, Hamoud adds the three rows below to
`docs/core/future.md`.

## Left out, on purpose

- Per-project tool rules for a server (decision 3), Codex (decision 6), plugins
  (decision 2): rows in `docs/core/future.md`.
- A live session picking up a switch change without reopening, and `CLAUDE.md` reaching a
  resumed or forked conversation.
- A skills folder left by a daemon that stopped with a session open stays until that thread
  opens again; no cleanup at start (Mohammed, 2026-11-25).
