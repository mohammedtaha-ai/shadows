# The Planner's project switches (§23.9 PR 3c)

- **Date:** 2026-11-25. Agreed with Mohammed the same day.
- **Owner spec:** §24.3, "The person may widen the Planner's reach, never its hands"
  (`docs/core/specs/2026-10-08-approval-authority-design.md`). Its delivery is fixed by the
  probe `docs/evidence/2026-11-25-planner-switches-probe.md`.
- **Tasks:** B-230 to B-235 in `docs/backend/stages.md`. The web's section is the shared
  request `docs/shared/requests/2026-11-25-planner-project-switches.md`.

## What the person gets

A project's settings get a section, "What the Planner may use". It lists AYLUL's extension
library (the same skills and MCP servers AYLUL's agents switch), with one switch per skill
and per MCP server for this project, and one switch "Read the project's CLAUDE.md". Every
switch starts off. A Planner conversation of that project on Claude Code opened after a
change receives what is switched on. One already open keeps what it opened with until it is
opened again (a new conversation, a resume after idle, a fork).

## Reuse first

Nothing here is a second copy of AYLUL's machinery. Each part extends the AYLUL code that
already does the same job:

| Job | AYLUL today | This plan |
|---|---|---|
| The library | `Extensions` (`crates/core-extensions`) | unchanged |
| A switch over an item | `ExtensionsStore::toggle`, `agent_library` over `aylul_agent_extension`, `aylul_agent_mcp` | the same two functions take a `Holder` (agent or project); two new tables mirror the agent ones |
| Resolving switched-on MCP servers with their secrets | `Extensions::opening_snapshot`, `resolve_mcp_servers` | the same, for a `Holder`; a project has no tool rules |
| Copying a skill out of the library | `extension_home.rs` `copy`, `safe_name`, `Homes::check_local`, `Extensions::copy_tree` | reused by a new file `planner_skills.rs` |
| `McpServerConfig` → `McpServerSpec` | the `map` at the end of `Homes::write_extensions` | moved into one private function both openings call |
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
     A library server named `shadows` is skipped and logged
     (`harness.planner_mcp_name_taken`), since that name is Shadows' own server.
   - Skills: a plugin folder Harness writes (below), sent as `plugins`, with `Skill`
     appended to the tools and `skills` naming each skill `shadows-skills:<name>`. With no
     skill on, none of the three is sent and the tools are exactly `PLANNER_TOOLS`.
   - `CLAUDE.md`: read from the project directory and appended to the Planner's
     instructions (below).
6. **Claude Code only.** A Codex Planner (`harness == policy::CODEX`) opens exactly as today.
   Codex's delivery is a row in `docs/core/future.md`.
7. **Executors and AYLUL are unchanged.** An executor's setup and an agent's opening do not
   read the project switches, and every AYLUL test passes unchanged.
8. **No library, no switches.** When AYLUL is not configured (`home_root` is `None`), the
   library is empty: the project view lists nothing, a switch is `NotFound`, the opening adds
   no server or skill. The `CLAUDE.md` switch works without AYLUL.

## B-230 · Extensions: project switches over the library

**Migration** `crates/core-db/migrations/0080_planner_project_switches.sql`, exactly:

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
number. B-230 writes the file; B-231 only reads the column.)

**Model** (`crates/core-extensions/src/model.rs`): add

```rust
/// Who a switch belongs to: one AYLUL agent, or one project's Planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Holder {
    Agent(String),
    Project(core_projects::ProjectId),
}
```

`core-projects` moves from `[dev-dependencies]` to `[dependencies]` in
`crates/core-extensions/Cargo.toml` (check first that `core-projects` does not depend on
`core-extensions`; if it does, stop and report).

**Store** (`crates/core-extensions/src/store/mod.rs`):

- `agent_library(slug)` becomes `holder_library(&Holder)`. The two `SELECT`s keep their
  shape; the joined table and its key column come from the holder:
  `aylul_agent_extension`/`agent_slug` and `aylul_agent_mcp`/`agent_slug` for an agent,
  `project_extension`/`project_id` and `project_mcp`/`project_id` for a project. Liveness:
  `Agents::require_live_in` for an agent, `Projects::require_live_in` for a project
  (`StorageError::NotFound("project")` maps to `ExtensionsError::NotFound`). For a project,
  `plugins` is returned empty. Write the table names as two `const` pairs selected by a
  `match`, never by formatting user input into SQL.
- `toggle(c, slug, kind, id, enabled)` becomes `toggle(c, &Holder, kind, id, enabled)`, the
  same body over the holder's tables. For a project, `kind == Plugin` returns
  `ExtensionsError::Invalid("plugins are not offered to the Planner".into())` before any
  write.
- `remove` already deletes the item; `ON DELETE CASCADE` removes the project rows. Add no code.

**Service** (`crates/core-extensions/src/lib.rs`, `mcp.rs`):

- `agent_extensions(slug)` and `set_agent_extension(..)` keep their signatures and call the
  holder versions with `Holder::Agent`. Their command kind and fingerprint stay
  (`extension.toggle`, `{slug,kind,id,enabled}`), so stored replays still match.
- New `pub async fn project_extensions(&self, project: &ProjectId) -> Result<AgentExtensions, ExtensionsError>`.
- New `pub async fn set_project_extension(&self, command_id: String, project: &ProjectId, kind: ExtensionKind, id: &str, enabled: bool) -> Result<AgentItem, ExtensionsError>`:
  takes the guard, command kind `extension.project_toggle`, params
  `{"project":project,"kind":kind,"id":id,"enabled":enabled}`.
- `opening_snapshot(slug)` keeps its signature; add
  `pub async fn project_snapshot(&self, project: &ProjectId) -> Result<AgentExtensionSnapshot, ExtensionsError>`.
  Both call one private `snapshot(&Holder)`. `resolve_mcp_servers` takes `&Holder`: for an
  agent it reads `tool_rules(slug, id)` as today; for a project `rules` is empty.

**Tests** in `crates/core-extensions/tests/library.rs`, through the public API, using the
file's existing fixtures:

- `a_project_switch_is_independent_of_agents`: one skill on for agent A and off for project P,
  and the reverse; each view reads its own.
- `a_project_switch_refuses_a_plugin`
- `a_project_switch_on_an_unknown_project_or_item_is_not_found`
- `a_project_switch_replays_and_conflicts`: same command id and body answers the first
  result; same id with `enabled` flipped is `CommandConflict`.
- `deleting_an_item_removes_its_project_switch`
- `a_project_snapshot_resolves_only_switched_on_servers_with_no_rules`
- Every existing test in the file passes unchanged.

**Contract** `crates/core-extensions/contract.yaml`: the header line names project switches;
`Holder` under `shapes`; the three new methods under the operations list; the rules above
with their test names; `dependencies` gains `projects: crates/core-projects/contract.yaml`.

**Touch:** the migration, `crates/core-extensions/{Cargo.toml,contract.yaml}`,
`src/{lib.rs,model.rs,mcp.rs,store/mod.rs}`, `tests/library.rs`.
**Done when:** `cargo test -p core-extensions` and
`cargo test -p shadows-core --test storage_contract` pass.

## B-231 · Projects: the CLAUDE.md switch

- `Project` (`crates/core-projects/src/model.rs`) gains
  `/// Whether the Planner on Claude Code receives the project directory's CLAUDE.md.`
  `pub planner_reads_claude_md: bool`. Each `SELECT` that builds a `Project`
  (`crates/core-projects/src/store/mod.rs`, the three at lines 98, 119 and 375 at `e7b9332`)
  adds the column.
- New `pub async fn set_planner_claude_md(&self, command_id: String, project: &ProjectId, enabled: bool) -> Result<Project, ProjectsError>`
  in `lib.rs`, after `set_modes`, in its shape: command kind `project.planner_claude_md`,
  params `{"project":project,"enabled":enabled}`, a store method
  `set_planner_claude_md(&c, project, enabled)` that classifies, checks
  `require_live_in`, updates the column, records the command with the project as its
  output, and answers the project.
- New `pub async fn planner_reads_claude_md_in(conn: &mut SqliteConnection, project: &ProjectId) -> Result<bool, StorageError>`
  beside `require_live_in` in the store's `impl Projects`: the one read other services call,
  `NotFound("project")` for an unknown or removed project. Harness calls it (B-233).
- Tests in `crates/core-projects/tests/project_contract.rs`:
  `the_claude_md_switch_starts_off_and_is_set`, `the_claude_md_switch_replays_and_conflicts`,
  `the_claude_md_switch_on_a_removed_project_is_not_found`.
- Contract `crates/core-projects/contract.yaml`: the field, the method, its rules and tests.
- `Project` is serialized by the API: B-234 regenerates `api/openapi.json`; this task does not.

**Touch:** `crates/core-projects/{contract.yaml,src/lib.rs,src/model.rs,src/store/mod.rs,tests/project_contract.rs}`.
**Done when:** `cargo test -p core-projects` passes. Needs B-230's migration (run B-230 first,
or write the migration here if B-230 has not merged; never two migrations).

## B-232 · The agent's session options: plugins and skills

In `crates/shadows-agent/src/connection.rs`, `SessionSetup` gains:

```rust
/// Local plugin folders the session loads (`_meta.claudeCode.options.plugins`).
/// Empty sends none.
pub plugins: Vec<std::path::PathBuf>,
/// The only skills the session lists (`_meta.claudeCode.options.skills`).
/// `None` sends none.
pub skills: Option<Vec<String>>,
```

`meta()` in `crates/shadows-agent/src/acp/session.rs` inserts
`"plugins": [{"type":"local","path":<path as a string>}, …]` when `plugins` is not empty and
`"skills": [...]` when `skills` is `Some`. A path that is not valid UTF-8 is an
`AcpError` at session start, not a lossy string. Every construction of `SessionSetup` adds
`plugins: vec![], skills: None` (the compiler lists them: `setup.rs`, `assistant.rs`,
`executor.rs` in core-harness, tests, and `fake-acp` if it builds one).

Tests in `crates/shadows-agent/src/acp_tests.rs`, beside `aylul_meta_loads_only_user_settings`:
`plugins_and_skills_reach_the_options`, `no_plugins_or_skills_sends_neither`.

Contract `crates/shadows-agent/contract.yaml`: the two fields and the two rules.

**Touch:** those files and each `SessionSetup { .. }` the compiler names (only the two new
fields added there).
**Done when:** `cargo test -p shadows-agent` passes and the workspace builds.

## B-233 · Harness: the Planner opens with the project's switches

Needs B-230, B-231, B-232.

1. **Wiring.** `Setups::new` (`crates/core-harness/src/setup.rs`) takes one more argument,
   `extensions: Arc<core_extensions::Extensions>`, kept as a private field. `Sessions`
   passes `owners.extensions.clone()` (`crates/core-harness/src/sessions.rs`, the
   `Setups::new` call). The `CLAUDE.md` switch is read with
   `Projects::planner_reads_claude_md_in` on the read snapshot `for_opening` already opens
   for the standards; `Setups` gets no `Projects` handle.
2. **Signature.** `for_opening(thread, project, kind, harness)` gains `cwd: &Path`, the
   directory the session opens in; both calls in `crates/core-harness/src/live.rs` pass their
   `cwd`.
3. **Where.** Only the Planner branch of `for_opening` (`ThreadKind::Planning`) and only when
   `harness == policy::CLAUDE_CODE`. The executor branch and Codex are untouched.
4. **MCP.** `self.extensions.project_snapshot(project)` (hold the snapshot until the skills
   folder is written, then drop it). Each server converts with the function the AYLUL opening
   uses; move that `map` out of `Homes::write_extensions` into
   `fn mcp_spec(config: McpServerConfig) -> McpServerSpec` in `extension_home.rs` and call it
   from both. Append to `mcp` after `shadows`, skipping one named `shadows` with
   `tracing::warn!(target: "harness", event = "planner_mcp_name_taken", project = %project)`.
   `ExtensionsError::NotConfigured` (no AYLUL) counts as an empty snapshot; any other error
   fails the opening with its message, after `self.forget(thread)`.
5. **Skills.** New file `crates/core-harness/src/planner_skills.rs`, one job: write a
   project's skills plugin folder.
   - Root: `<home_root>/planner-skills/<project id>/`, where `home_root` is the AYLUL home
     root Extensions holds. Add `pub fn library_root(&self) -> Option<PathBuf>` to
     `Extensions` only if Harness cannot already reach it; prefer the root `assistant.rs`
     already uses.
   - Each opening writes a fresh generation `<root>/<uuid>/` holding
     `.claude-plugin/plugin.json` = `{"name":"shadows-skills","version":"0.0.0","description":"Skills this project gives its Planner"}`
     and `skills/<name>/` copied with the existing `copy(root, target, item)` from
     `extension_home.rs` (make it `pub(super)`), each name checked with `safe_name`, each path
     with `Homes::check_local`. Then it removes the other generations under `<root>`
     (ignoring a removal error). An empty skill list writes nothing and removes them all.
   - Returns `Option<PathBuf>`, the generation folder when a skill is on.
6. **Setup.** When the folder is `Some(dir)`: `plugins = vec![dir]`, `tools` =
   `PLANNER_TOOLS` + `"Skill"`, `skills = Some(names.map(|n| format!("shadows-skills:{n}")))`.
   Otherwise the three are as today.
7. **CLAUDE.md.** When `Projects::planner_reads_claude_md_in` answers true: read
   `cwd.join("CLAUDE.md")` only if `symlink_metadata` says it is a regular file of at most
   65 536 bytes and it is valid UTF-8. Then append, after the project instructions:
   `\n\n## The project's CLAUDE.md\n\n{text}`. Otherwise append nothing and log
   `tracing::info!(target: "harness", event = "claude_md_skipped", reason)` with `reason`
   one of `missing`, `not_a_file`, `too_large`, `not_utf8`, `unreadable`.
8. **Held setup.** `Held` (what the session opened with) records nothing new: a switch
   change reaches the next opening, not a live session.

**Tests**, a new file `crates/shadows/tests/planner_switches.rs` (fake-acp `report` and
`report-mcp`, an app with AYLUL configured, as the AYLUL route tests build it):

- `with_nothing_switched_on_the_planner_opens_as_before`: `assert_locked`'s exact options,
  no `plugins`, no `skills`, only `shadows` in `mcpServers`.
- `a_switched_on_skill_reaches_the_planner_as_a_plugin`: options carry `plugins` with one
  folder holding `.claude-plugin/plugin.json` and `skills/<name>/SKILL.md`, `tools` ends with
  `Skill`, `skills == ["shadows-skills:<name>"]`.
- `a_switched_on_server_reaches_the_planner_after_shadows`
- `a_server_named_shadows_is_skipped`
- `a_switched_off_item_and_a_plugin_never_reach_the_planner`
- `the_claude_md_switch_appends_the_file`: the append ends with the file's text under the
  heading; with the switch off it does not contain it.
- `a_claude_md_that_is_too_large_or_a_symlink_is_skipped`
- `a_codex_planner_and_an_executor_ignore_the_switches`: on a project with everything on,
  an executor opening's options equal today's.
- `a_switch_change_reaches_the_next_opening_only`: switch on, the open session's report is
  unchanged; after a new conversation it carries the item.

Contract `crates/core-harness/contract.yaml`: the module `planner_skills.rs`, the opening
rules above with these test names, and the two log events. Code map
`docs/codebase/README.md`: a row for `planner_skills.rs`.

**Touch:** `crates/core-harness/{contract.yaml,src/setup.rs,src/sessions.rs,src/live.rs,src/extension_home.rs,src/planner_skills.rs,src/lib.rs}`,
`crates/core-extensions/src/lib.rs` (only `library_root` if needed, with its contract line),
`crates/shadows/tests/planner_switches.rs`, `docs/codebase/README.md`.
**Done when:** `cargo test -p shadows --test planner_switches`, `--test planner_mcp`,
`--test codemap`, and `cargo test -p shadows-core --test aylul_session --test executor_session`
pass.

## B-234 · Routes

Only after the shared request is `accepted`. In a new file
`crates/shadows-http/src/planner_switches.rs` (one job: the project's Planner switch routes),
registered in `lib.rs` beside the project routes, tag `projects`, in
`aylul_extensions.rs`'s shape and reusing its `Toggle` body (make it `pub(super)` there):

| Route | Calls | Answers |
|---|---|---|
| `GET /api/projects/{id}/planner-extensions` | `extensions().project_extensions` | 200 `AgentExtensions`; 404 |
| `PUT /api/projects/{id}/planner-extensions/{kind}/{item}` with `Toggle` | `extensions().set_project_extension` | 200 `AgentItem`; 400 a plugin; 404; 409 |
| `PUT /api/projects/{id}/planner-claude-md` with `Toggle` | `projects().set_planner_claude_md` | 200 `Project`; 404; 409 |

Each handler calls one method and shapes its answer; the error mapping is the existing one
for `ExtensionsError` and `ProjectsError`. Tests in `crates/shadows/tests/planner_switches.rs`
(the same file): `the_routes_list_and_switch_a_project_s_items`,
`the_routes_refuse_a_plugin_and_an_unknown_project`, `the_claude_md_route_sets_the_project`.
Regenerate `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`. Contract
`crates/shadows-http/contract.yaml`: the three routes.

**Touch:** `crates/shadows-http/{contract.yaml,src/lib.rs,src/planner_switches.rs,src/aylul_extensions.rs}`,
`crates/shadows/tests/planner_switches.rs`, `api/openapi.json`, `docs/codebase/README.md`
(the new file's row).
**Done when:** `cargo test -p shadows --test planner_switches --test openapi --test codemap`.

## B-235 · Live acceptance on Claude

A live test `crates/shadows/tests/planner_switches_live.rs`, `#[ignore]` like the other
`*_live.rs`, using `fixtures/live.rs`'s pinned adapter, run here with
`cargo test -p shadows --test planner_switches_live -- --ignored`. It plants the probe's
fixtures (a `CLAUDE.md` code word, a project `.claude/settings.json` with a `SessionStart`
hook that touches a file, a library skill with a word, a library stdio MCP server with one
tool), switches the skill, the server and `CLAUDE.md` on, opens a Planner conversation, and
asserts the reply names the skill and the server's tool and quotes the code word, and the
hook's file does not exist. It then switches all off, opens a new conversation, and asserts
none of the three is named. Its result is recorded in
`docs/evidence/2026-11-25-planner-switches-probe.md` under a new heading "Through Shadows".

**Touch:** `crates/shadows/tests/planner_switches_live.rs`, that evidence file.
**Done when:** the live test passes here, with its output in the evidence.

## Order and checks

B-230, B-231 (after B-230's migration) and B-232 first; B-233 next; B-234 once the request is
accepted; B-235 last. Per task, only the tests it names. Before the PR, the full gate from
CLAUDE.md and one branch review. The browser check of the page waits for Mohammed's machine.

## Left out, on purpose

- Per-project tool rules for a server (decision 3), Codex (decision 6), plugins
  (decision 2): rows in `docs/core/future.md`.
- A live session picking up a switch change without reopening.
