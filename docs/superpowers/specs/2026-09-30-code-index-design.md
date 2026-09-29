# Section 15 — The Code Index (Milestone 3)

- **Date:** 2026-09-30
- **Status:** Designed with Mohammed on 2026-09-30. Not built.
- **Idea:** `docs/vision.md` §2.4, "Where is X?".

An agent asks Shadows where something is, and Shadows answers with the name,
its kind, the file, the line and the signature. It never answers with code.
The agent opens that exact place itself if it needs to. So it does not write
again what already exists, and it does not read the whole project to find out.

## 15.1 Scope

**In Milestone 3:**

- A new crate, `shadows-index`, turns one file into its definitions and
  references with tree-sitter. It starts with Rust, TypeScript/TSX and
  JavaScript, and a language is added without touching anything else (§15.3).
- A ninth service, `Code`, in `shadows-core`:
  - indexes the active projects and keeps them current by watching their files;
  - answers the three questions: where X is defined, who uses X, and what is
    in a file or folder.
- Three MCP tools for an external agent's project grant.
- HTTP routes a search page will use. The page itself is designed later (§15.9).
- Two settings a person changes from the web client later:
  - how many projects are active (default 5);
  - which projects a project may read (links, §15.6).

**Not in Milestone 3:**

- The Planner's access to the tools. Its column in §13.6 stays "—" (§15.7).
- Executors. They reach the same tools through their grant when they exist.
- Framework links, such as a route to its controller. They are left to
  `vision.md` §9.
- Type-aware references. "Who uses X" matches by name (§15.5).

## 15.2 Parts

```text
shadows-mcp  ─┐                             ┌─► shadows-index (tree-sitter, the language table)
shadows-http ─┼─► shadows-core ─── Code ────┤
              ┘                             └─► SQLite (Code's store)
```

- **`shadows-index`** has one job: a file's text in, its tags out. It knows
  nothing of SQLite, projects or the daemon. `shadows-core` depends on it, as
  it depends on `shadows-agent`.
- **`Code`**, in `crates/shadows-core/src/code/`, owns the index of every
  project: scanning, watching, the active set, links and the questions. It has
  its `contract.yaml` (§14.7). Its files:
  - `mod.rs`: the service's methods;
  - `model.rs`: its types;
  - `scan.rs`: walking a project and indexing what changed;
  - `watch.rs`: the file watcher of the active projects;
  - `store.rs`: its SQLite queries.
- **`shadows-mcp`** and **`shadows-http`** each call one `Code` method per tool
  or route (§14.5).

## 15.3 `shadows-index`

- **The language table.** `languages.rs` holds one entry per language:
  - its name;
  - its file extensions;
  - its tree-sitter grammar;
  - its tags query, which says what a definition and a reference are in
    that language.

  The first entries are Rust (`.rs`), TypeScript (`.ts`, `.mts`, `.cts`), TSX
  (`.tsx`) and JavaScript (`.js`, `.mjs`, `.cjs`, `.jsx`).
- **Adding a language** is one entry, one grammar crate, and its tags query.
  Nothing else changes: not `Code`, not its store, not the tools.
- **Tags come from `tree-sitter-tags`**, the standard tree-sitter tagging, using
  each grammar's own `tags.scm` where it has one. Where the query misses
  something we need, we add our own query next to the entry. Task 0 finds out
  which queries need this (§15.10).
- **`extract(language, text) -> Vec<Tag>`.** A `Tag` holds:
  - `name`;
  - `kind`: the capture's kind, such as `function`, `method`, `class`,
    `interface`, `module`, `macro`, `type` or `constant`;
  - `role`: `Definition` or `Reference`;
  - `line`, starting at 1;
  - `signature`: the first line of a definition, trimmed, at most 200
    characters. There is none for a reference.
- A file tree-sitter cannot fully parse, such as half-written code, still
  gives the tags it found. That is normal and is not an error.
- **Extraction is blocking work.** `TagsContext` is not `Send`, so `Code` runs
  `extract` on a blocking thread, never across an `await`.
- **Grammar versions** are pinned in `[workspace.dependencies]` with the
  `tree-sitter` and `tree-sitter-tags` versions they agree with.

## 15.4 Indexing

**The index is a copy derived from the files.**

- It can be deleted and rebuilt at any time.
- It has no `CommandId` and no journal events.
- Paths are stored relative to the project's folder, with `/`. On Windows, a
  path is normalised before it is used as a key, because `Foo.rs` and `foo.rs`
  are the same file.

**Tables (migration 0008):**

- `code_file`: `project_id`, `path`, `size`, `modified_at`, `language`,
  `skipped_reason` (null when indexed).
- `code_tag`: `project_id`, `path`, `name`, `kind`, `role`, `line`,
  `signature`.
  - An index on `(project_id, name)` serves the name questions.
  - An index on `(project_id, path, line)` serves `outline`.
- `code_setting`: one row holding `active_limit`, default 5.
- `project_link`: `project_id`, `linked_project_id`.

**Scanning a project:**

1. Walk its folder with the `ignore` crate's `WalkBuilder`. It honours
   `.gitignore`, so `target/` and `node_modules/` are not walked. Only files
   whose extension is in the language table are kept.
2. Skip a file whose size and modified time equal its `code_file` row.
3. Read a changed file. A file over **1 MB**, a binary one, or one that is not
   UTF-8 is not parsed. Its row records why.
4. Extract its tags on a blocking thread, outside any transaction.
5. Write the file in **one short transaction**: delete its old tags, insert
   the new ones, and update its row. A first index of a large project
   therefore never holds the single writer (§6.23) for long, and turns and plan
   edits are not delayed.
6. Delete the rows of files that are no longer there.

**Watching.**

- `notify-debouncer-full` watches each active project's folder recursively.
  Events are gathered for about 500 ms, because editors save by writing a
  temporary file and renaming it.
- Every changed path is filtered through the project's `.gitignore` (the
  `ignore` crate's `Gitignore`) and the language table. Then it goes through
  steps 3–5, or its rows are deleted if the file is gone.
- **A watcher error or lost events start a full scan** of that project, and
  write a log line. On Windows, a folder full of changes, such as `target/`
  during a build, can overflow the watcher's buffer even though the files are
  ignored. The full scan is what keeps the index true after that.
- **A missing folder** stops that project's watcher, and its status becomes
  `directory missing`. Its index is kept. When the folder returns, or the
  project's folder is changed, it is scanned again.

**Each active project has one worker.** It does the project's scans and the
files its watcher reports, one after another, so two writes of one file never
race.

**A failed write of one file** is logged, and the file keeps its old row, so the
next scan does it again. The worker goes on. **The index never stops the
daemon:** a failure in it affects the index only, never turns or plans.

## 15.5 Questions

Every question reads SQLite directly and never waits for indexing.

**The answer carries the project's status**, one of:

- `ready`;
- `indexing` with files done and files found, e.g. `indexing: 340/1200`, so an
  agent knows the answer may be incomplete;
- `not indexed: no directory`, for a project with no folder;
- `directory missing`.

It also says how many files were skipped and why.

| Method | Question | Answer, per hit |
|---|---|---|
| `definitions(scope, name)` | Where is X defined? | project, path, line, kind, signature |
| `references(scope, name)` | Who uses X? | project, path, line, kind, and `matched_by: name` |
| `outline(scope, path)` | What is in this file or folder? | the definitions under `path`: path, line, kind, name, signature |
| `status(project)` | How is the index? | the status above, the counts, the last update |

- **Names match exactly**, and case-sensitively. When nothing matches,
  `definitions` and `references` answer up to **10 names that contain** the
  text, case-insensitively, as suggestions.
- **At most 50 hits**, ordered by project, path and line, with a flag saying
  whether there are more.
- **`references` matches by name only.** Two functions with the same name in
  different files are not told apart. Every hit says `matched_by: name`, so no
  agent takes it for more than it is.
- **`outline`'s `path`** is relative to the project's folder. A path that is
  absolute, or that leaves the folder through `..`, is refused
  `INVALID_COMMAND` ("the path must be inside the project"), and nothing
  outside the folder is read.
- **The scope** is the project asked about plus the projects it links to
  (§15.6). A question may name one project in that scope. With none named, it
  covers them all.

**A question about a project that is not active** makes it active (§15.6).
It answers from what is stored, with the status `indexing`.

## 15.6 Active projects and links

**Only the most recently used projects are active: indexed and watched.**

- **At startup,** the active projects are the `active_limit` projects with a
  folder whose last turn is newest. A project with no turn yet counts from its
  creation.
- **Using a project** makes it active at once. Using it means:
  - a turn starts in it;
  - the web client opens it;
  - a code question names it.

  `Projects` and `Turns` call `Code::touch(project)`. These calls are declared
  under `called_by_other_services` in `Code`'s contract.
- **When the set is full,** the least recently used project leaves it. Its
  watcher stops and its index stays stored. When it comes back, a scan redoes
  only what changed while it was away.
- **`active_limit`** is a setting, default 5, from 1 to 20. Setting it is a
  command with a `CommandId` (kind `code_active_limit_set`). Lowering it stops
  the least recently used projects' watchers at once.
- **A new project, or a changed folder,** while the daemon runs: `Projects`
  calls `Code::touch`, which scans it and watches the new folder.

**A project can read the index of the projects it is linked to.**

- A frontend in one folder and the backend it calls in another are two
  projects. Linking the frontend to the backend lets a question in the
  frontend answer from the backend too. Every hit names its project, e.g.
  `backend: src/api/orders.rs:42`.
- **A link goes one way.** Linking A to B does not let B read A.
- **Only a project can be linked.** A folder that is not a project is added as
  a project first. So every linked folder has its own index, status and
  watcher, and there is no second kind of index.
- **When a project becomes active, its linked projects do too.** They count
  toward `active_limit`.
- Adding and removing a link are commands with a `CommandId` (kinds
  `project_link_put` and `project_link_remove`). A link to itself, or to a
  project that does not exist, is refused `INVALID_COMMAND`.
- **No path outside a project's folder is ever read,** linked or not.

## 15.7 Interfaces

**MCP tools**, a new row each in §13.6's table:

| Tool | Planner (thread grant) | External agent (project grant) | Writes |
|---|---|---|---|
| `where_is` | — | ✓ | no |
| `who_uses` | — | ✓ | no |
| `outline` | — | ✓ | no |

- A tool takes no project id. **The project comes from the grant.** The tool
  may name a linked project by name. A project outside the grant's project and
  its links is refused `GRANT_SCOPE`.
- Each tool calls one `Code` method, and its text answer lists one hit per line:
  `project: path:line kind name — signature`.

> **OPEN — the Planner's code tools.** The Planner's grant gets the same three
> tools. Its scope is its thread's project and that project's links.
> **Trigger:** the milestone that has the Planner write tasks naming real
> functions. **Why it does not block:** the index, the tools and the scope
> rule are the same; the change is a column in §13.6.

**HTTP routes**, under `/api/projects/{id}/code`:

| Route | Method |
|---|---|
| `GET …/code/definitions?name=` | `definitions` |
| `GET …/code/references?name=` | `references` |
| `GET …/code/outline?path=` | `outline` |
| `GET …/code/status` | `status` |
| `GET …/code/links`, `PUT …/code/links/{linked_id}`, `DELETE …/code/links/{linked_id}` | the links |

- The active limit is `GET` and `PUT /api/code/settings`.
- The routes that change something take a command id, as the others do (§2).
- The routes are added to `api/openapi.json`.

## 15.8 Startup and shutdown

- `AppCore::start` builds `Code` before `Projects` and `Turns`, which hold it
  to call `touch`. `Code` then chooses the active set and starts the workers in
  the background. **`serve` does not wait for indexing.**
- `AppCore::shut_down` stops the watchers and workers **before** the storage
  closes, in §8.5's order. A write that was running finishes, or is dropped
  whole with its transaction. The next start redoes what did not land.

## 15.9 The web client

The routes of §15.7 are what a search page will use. That page is not built in
Milestone 3.

> **OPEN — the search page and the settings.** What the search page looks
> like, and where the active limit and the links are set. **Trigger:** the
> web-client design session Mohammed holds after Milestone 3 runs. **Why it
> does not block:** every route the page needs exists and is tested.

## 15.10 Order of work

0. **Probe** (Task 0), before any product code. It answers:
   - whether the Rust, TypeScript, TSX and JavaScript tags queries capture what
     §15.3 needs. For example, whether TypeScript's query needs JavaScript's
     with it, and whether Rust's catches `const` and `static`;
   - how much each grammar adds to a clean build;
   - how `notify` behaves on Windows: a save through rename, a build writing
     into an ignored `target/`, and whether a watched folder can still be
     renamed.

   Its output goes to `docs/evidence/milestone3/`, and its code is deleted
   (CLAUDE.md). If it proves a part of this section wrong, this section is
   amended before Task 1.
1. `shadows-index`, with its language table and `extract`.
2. `Code`'s tables, the scan and the questions.
3. The watcher and the active set.
4. The links and the active-limit setting, with their commands.
5. The MCP tools and the HTTP routes.
6. The documents of §15.12.

## 15.11 Tests and acceptance

**Few tests: one per behaviour.** A test is broken on purpose, to prove it
fails, only for the scope rule of §15.5–§15.7.

1. **One sample file per language** gives the definitions, with names, lines
   and signatures, and the references it should.
2. **Indexing:** a temporary project is scanned. Then a file changes, one is
   deleted, one is added, and one over 1 MB is skipped. The index follows each.
3. **Watching:** a file written on disk is in the index within a bound, and a
   simulated lost-events error starts a full scan.
4. **The active set:** of six projects, five are watched. Using the sixth
   brings it in and the oldest leaves.
5. **The scope:** a linked project answers, an unlinked one is `GRANT_SCOPE`,
   a path with `..` is refused, and a link goes one way. This one is broken on
   purpose.
6. `Code`'s contract passes `contracts.rs`, and every existing test still
   passes.

**Acceptance: Mohammed's run on Windows.** Milestone 3 is done after it.

1. `shadows serve` on the Shadows repository itself. The log shows how long
   the first index took.
2. Claude Code, connected over `/mcp`, asked "where is `start_turn` defined?",
   answers with the file, the line and the signature, without reading the file.
3. A function is edited and saved. Asking again shows the change.
4. A second project is linked, and a question about it answers.
5. After a restart, the index is not rebuilt from nothing.

The run is recorded in `docs/evidence/milestone3/WINDOWS_RUN.md`.

## 15.12 Changes to other documents

- `specs/README.md`: §15's row.
- §14.3: `shadows-index` in the crate list and the dependency diagram.
- §14.4: `Code` in the service table.
- §13.6: the three tools' rows.
- §6: the four tables of §15.4.
- `CLAUDE.md`: the crate, the diagram, the service list and the single-ownership
  table.
- `docs/codebase/README.md`: `shadows-index` and `code/`, and the invariant
  "`shadows-index` knows nothing of SQLite or projects".
- `vision.md` §2.4's **Today** line, and the "Where is X?" open question in §9,
  which this section answers.
- `docs/status.md`.
