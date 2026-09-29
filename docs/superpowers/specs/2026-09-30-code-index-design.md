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
- Framework links, such as a route to its controller. §15.12 adds them to
  `vision.md` §9's open questions.
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
  - `scope.rs`: which projects and paths a question may read;
  - `watch.rs`: one active project's worker and watcher;
  - `active.rs`: which projects are active, in order of use;
  - `links.rs`: the links and the active limit, as commands;
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
  each grammar crate's `TAGS_QUERY`. TypeScript's query holds only what
  TypeScript adds, so the TypeScript and TSX entries use TypeScript's query
  followed by JavaScript's, as the grammar's own `tree-sitter.json` does. Where
  a query misses something we need, we add our own patterns next to the entry.
  Rust's misses `const`, `static` and calls through a path (`Storage::open`).
  Task 0 settles the full list (§15.10).
- **One tag per name.** When two patterns match the same name, such as a
  function inside an `impl`, `tree-sitter-tags` keeps the earlier pattern's
  tag, so that function is a `method`.
- **`extract(language, text) -> Vec<Tag>`.** A `Tag` holds:
  - `name`;
  - `kind`: the capture's kind, such as `function`, `method`, `class`,
    `interface`, `module`, `macro`, `type` or `constant`;
  - `role`: `Definition` or `Reference`;
  - `line`: the line the name is on, starting at 1;
  - `signature`: the definition's first line, which is that same line,
    trimmed, at most 200 characters. `extract` cuts it from the text itself,
    because `Tag::line_range` stops at 180 bytes. There is none for a
    reference.
- A file tree-sitter cannot fully parse, such as half-written code, still
  gives the tags it found. That is normal and is not an error.
- **Extraction is blocking work.** Parsing is CPU-bound, so `Code` runs
  `extract` on a blocking thread, never on the async runtime.
- **Versions** are pinned in `[workspace.dependencies]`. The grammar crates
  depend on `tree-sitter-language`, not on `tree-sitter`; what must agree is
  each grammar's ABI with the `tree-sitter` that `tree-sitter-tags` uses.
  Task 0 checks it.

## 15.4 Indexing

**The index is a copy derived from the files.**

- It can be deleted and rebuilt at any time.
- It has no `CommandId` and no journal events.
- Paths are stored relative to the project's folder, with `/`. On Windows, a
  path is normalised before it is used as a key, because `Foo.rs` and `foo.rs`
  are the same file.

**Tables (migration 0008):**

- `code_file`: `project_id`, `path_key` (the key above), `path`, `size`,
  `modified_ms`, `language`, `skipped_reason` (null when indexed).
- `code_tag`: `project_id`, `path_key`, `path`, `name`, `kind`, `role`,
  `line`, `signature`.
  - An index on `(project_id, name)` serves the name questions.
  - An index on `(project_id, path_key, line)` serves `outline`.
- `code_setting`: one row holding `active_limit`, default 5.
- `project_link`: `project_id`, `linked_project_id`, `created_at`.

**Scanning a project:**

1. Walk its folder with the `ignore` crate's `WalkBuilder`. It honours the
   folder's `.gitignore` files, whether or not it is a Git repository
   (`require_git(false)`). It reads no ignore file outside the folder:
   `parents(false)` and `git_global(false)`, whose defaults would read the
   folders above it and the user's global Git excludes. It follows no link.
   `target/` and `node_modules/` are never walked, with or without a
   `.gitignore`. Only files whose extension is in the language table are kept.
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
- Every changed path is filtered through the same rules as the walk and the
  language table. Then it goes through steps 3–5, or its rows are deleted if
  the file is gone. A changed `.gitignore` starts a full scan.
- **The watcher can lose changes without saying so.** On Windows, a folder
  full of changes, such as `target/` during a build, can overflow its buffer
  even though the files are ignored. `notify` 8.2.0 passes that overflow on
  silently, and on `ERROR_NOTIFY_ENUM_DIR` or a deleted folder it stops
  watching without telling the handler (its `windows.rs`). So the watcher is
  the fast path, and correctness does not rest on it:
  - **A periodic scan** runs every **60 s** on each active project: steps 1–6,
    which compare size and modified time and read only what changed. It opens
    no unchanged file, so it costs one directory walk; Task 0 measures it. A
    change the watcher lost is in the index within a minute.
  - **A question checks its own hits** (§15.5), so an answer never points into
    a file that changed since it was indexed.
  - A watcher error, or an event whose `need_rescan()` is true, starts a scan at
    once and writes a log line. A watcher that stopped is started again by the
    next periodic scan.
  - When a `notify` release reports the loss on Windows (its `main` branch
    already does), Shadows moves to it. The periodic scan stays, because it
    also covers a watcher that stopped.
- **A missing folder**, found by the watcher or the periodic scan, stops that
  project's watcher, and its status becomes `directory missing`. Its index is
  kept. When the folder returns it is scanned again.
- **A project's folder never changes, and a project is never deleted:**
  `Projects` has neither method, and the rows that reference a `project`
  forbid deleting it.

> **OPEN — a moved or deleted project.** Changing a project's folder must
> drop its index and scan the new one; deleting a project must delete its
> index, its links and the links to it. **Trigger:** the change that adds
> either method to `Projects`. **Why it does not block:** neither exists.

**Each active project has one worker.** It does the project's scans, the files
its watcher reports and the files a question re-checks, one after another, so
two writes of one file never race.

**A failed write of one file** is logged, and the file keeps its old row, so the
next scan does it again. The worker goes on. **The index never stops the
daemon:** a failure in it affects the index only, never turns or plans.

## 15.5 Questions

Every question reads SQLite and never waits for indexing. **Before it
answers, it checks the size and modified time of the files in its hits** (at
most 50). A file that changed is indexed again first, and a file that is gone
has its rows deleted, so no hit points at a stale line. It opens no other file.

**The answer carries the status of each project it covers**, one of:

- `ready`;
- `indexing` with files done and files found, e.g. `indexing: 340/1200`, so an
  agent knows the answer may be incomplete;
- `inactive`: not in the active set, so the index is as it was when the
  project left it (§15.6);
- `not indexed: no directory`, for a project with no folder (only projects
  created before migration 0003 can have none);
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
  `INVALID_COMMAND` ("the path must be inside the project"). `outline` reads
  the index, so it never opens a path.
- **The scope** is the project asked about plus the projects it links to
  directly (§15.6); a link's own links are not in it. A question may name one
  project in that scope, by its slug. With none named, it covers them all. A
  project outside the scope is refused `GRANT_SCOPE` over MCP (§15.7) and
  `INVALID_COMMAND` over HTTP, which has no grant.

**A question about a project that is not active** makes it active (§15.6).
It answers from what is stored, with the status `indexing`.

## 15.6 Active projects and links

**Only the most recently used projects are active: indexed and watched.**

- **At startup,** the active projects are the `active_limit` projects with a
  folder whose last turn (the newest `operation.created_at` of its threads) is
  newest. A project with no turn yet counts from its creation.
- **The order of use is kept in memory, not stored.** After a restart it
  starts again from the last turns.
- **Using a project** makes it active at once. Using it means:
  - a turn starts in it: `Turns::send`;
  - the web client opens it, which lists its threads: `Threads::list`;
  - a code question names it;
  - it is created: `Projects::create`.

  `Turns`, `Threads` and `Projects` call `Code::touch(project)`, a public
  method (§14.4). It returns at once; the scan runs in the project's worker.
  `Code`'s contract names these callers under `touch`.
- **When the set is full,** the least recently used project leaves it. Its
  watcher stops and its index stays stored. When it comes back, a scan redoes
  only what changed while it was away.
- **`active_limit`** is a setting, default 5, from 1 to 20; another value is
  refused `INVALID_COMMAND`. Setting it is a command with a `CommandId` (kind
  `CodeActiveLimitSet`). Lowering it stops the least recently used projects'
  watchers at once.

**A project can read the index of the projects it is linked to.**

- A frontend in one folder and the backend it calls in another are two
  projects. Linking the frontend to the backend lets a question in the
  frontend answer from the backend too. Every hit names its project, e.g.
  `backend: src/api/orders.rs:42`.
- **A link goes one way.** Linking A to B does not let B read A.
- **Only a project can be linked.** A folder that is not a project is added as
  a project first. So every linked folder has its own index, status and
  watcher, and there is no second kind of index.
- **When a project becomes active, its linked projects do too,** just before
  it, so the project itself is always the most recent. They count toward
  `active_limit`. When the limit cannot hold them all, the links touched first
  leave, and answer with the status `inactive`. Activation is not transitive.
- Adding and removing a link are commands with a `CommandId` (kinds
  `ProjectLinkPut` and `ProjectLinkRemove`). A link to itself, or to a
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
  may name a linked project by its slug. A project outside the grant's project
  and its links is refused `GRANT_SCOPE`.
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
- The routes that change something take a `command_id` under §4's
  idempotency rule (§13.5): `PUT` in the body, `DELETE` in the query, as the
  grant revoke route does.
- The routes are added to `api/openapi.json`.

## 15.8 Startup and shutdown

- `AppCore::from_parts` builds `Code` before `Projects`, `Threads` and
  `Turns`, which hold it to call `touch`. Then `AppCore::start` has `Code`
  choose the active set and start the workers in the background. Tests that
  build the core with `assemble` start them only when they ask. **`serve`
  does not wait for indexing.**
- `AppCore::shut_down` first stops `Code`'s watchers and workers, then runs
  §8.5 unchanged. A worker stops after the file it is on: that file's
  transaction commits whole or not at all. The next start redoes what did not
  land.

## 15.9 The web client

The routes of §15.7 are what a search page will use. That page is not built in
Milestone 3.

> **OPEN — the search page and the settings.** What the search page looks
> like, and where the active limit and the links are set. **Trigger:** the
> web-client design session Mohammed holds after Milestone 3 runs. **Why it
> does not block:** every route the page needs exists and is tested.

## 15.10 Order of work

0. **Probe** (Task 0), before any product code. It answers:
   - which patterns each language needs beyond its grammar's query for
     §15.3, starting from the known gaps: Rust's `const`, `static`, calls
     through a path, and uses of a type;
   - that each grammar loads under the `tree-sitter` that `tree-sitter-tags`
     0.27 uses, and how much each adds to a clean build;
   - how `notify` behaves on Windows: a save through rename, a build writing
     into an ignored `target/`, and whether a watched folder can still be
     renamed;
   - how long the periodic scan (§15.4) takes on the Shadows repository on
     Mohammed's machine, with Windows Defender on. If it is heavy, the 60 s
     becomes longer or a setting.

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
   change made while the watcher is off is caught by the periodic scan.
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
- §14.4: `Code` in the service table, and `Code` stopping first in its
  shutdown bullet.
- §13.6: the three tools' rows.
- §6: the four tables of §15.4.
- `CLAUDE.md`: the crate, the diagram, the service list and the single-ownership
  table.
- `docs/codebase/README.md`: `shadows-index` and `code/`, and the invariant
  "`shadows-index` knows nothing of SQLite or projects".
- `vision.md` §2.4's **Today** line; in §9, the "Where is X?" open question,
  which this section answers, and a new one for framework links.
- `docs/status.md`.
