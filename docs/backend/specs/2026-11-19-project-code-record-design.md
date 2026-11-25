# The project's code record: the Planner asks, it never guesses

Status: approved by Mohammed, 2026-11-19: the direction (who writes which part), the owner, a new
service (§8), and that a task is verified against its contract before it is marked done (§4).
A missing symbol is returned to the executor as a failed check (§4). Amended by Mohammed on
2026-11-19 for slice 2: Shadows answers from its own knowledge and the code confirms (§5), and
code no plan wrote moves to importing a project (§6). Ruled again by Mohammed on 2026-11-19, after
the backend lead's review: tables and screens are Design's elements that a task pins, not contract
text (§5.1); the diagrams are read only in this slice and a later edit changes the same Design
record (§5.2); slice 2 is two plans, 2a and 2b, on one branch (§9); elements are tied to the map,
the roadmap and the standards, and the Planner writes them directly (§5.1). Slice 1 is on `main` in PR #73
(`7b7abec`); its plan was `docs/backend/plans/2026-11-19-code-record.md` at `7b7abec`, deleted once its tasks were done.

Builds on the [task contract](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#the-task-contract)
(§17), the [language-server design](2026-11-13-language-servers-design.md) (`code_lsp`) and the
[executors design](2026-11-08-executors-design.md) (attempts, commits, done marks).

## 1. Why

A Planner that plans the next change to a project must know what is already there: which file
holds the login logic, which functions it has, which database query and which screen use it.
Today it reads the folder itself or guesses. B-167 and B-169 (`docs/backend/stages.md` at `7b7abec`) refuse a guess once it is written (a
`modify` of a missing file, a `create` of a present one), but they only catch a wrong path, after
the fact. The Planner should know before it writes the task.

Shadows already holds the two halves of that knowledge, apart:

- **Why a file and a function exist.** Every task contract names its files with their purpose, and
  its symbols with their kind, inputs, return, behavior and errors. The plan that wrote them, its
  version and its task are recorded.
- **What the code is now.** The project's language servers answer where a symbol is (file, line),
  what a file holds, and who calls what (`code_lsp`).

The code record joins them, per project, the way `contract.yaml` does for Shadows' own code: the
intent from the plan, the facts from the code, and the gaps between them.

## 2. Who writes what (agreed 2026-11-19)

| Part of the record | Written by | Why |
|---|---|---|
| Purpose, behavior, inputs and return, errors, invariants (the intent) | The Planner, in the task contract, as today | It decided them; nobody else knows why |
| Where each symbol is now, its signature, who uses it (the facts) | Shadows, from the language servers | Written by hand they go stale at the next change |
| Checks and their results | The Planner names them in the contract; Shadows runs them and records the result, as today (slice 5) | |
| Gaps (promised and not found, found and not promised) | Shadows, comparing the two | The judge is neither the one who planned nor the one who built |
| The executor | Writes none of it | It is the one being checked; it reports what it did, as today |

## 3. What the record holds

One entry per file a done task created or modified, and under it one entry per symbol its
contract names:

- **File:** path; the map part and layer its place falls in; its purpose from each task that
  worked on it, newest first; the plan, version, task and attempt commit of each.
- **Symbol:** name and kind; inputs, return, behavior and errors from the contract; the task that
  promised it; whether the code at that commit has it (`found`, `missing`, or `unchecked` when no
  language server answers for the file).
- **Gaps:** a promised symbol missing from its file; a symbol the language server finds in a file
  the task created that no contract names. Each gap names the task, the file and the symbol.
- **Element pins** (slice 2, §5.1): each table or screen the done task's contract pins, with the
  pinned revision, the action, the file and the symbol id. The element's shape is Design's; its
  state is worked out when it is asked for (§5.2), from these rows, the symbol states above and the
  folder.

Lines are not stored: they move with every edit. A question answers them live (§5).

## 4. When it is made: verified before done (Mohammed, 2026-11-19)

A task is never marked done before Shadows has verified it. The comparison of code and contract
is part of that verification, beside the checks Shadows already runs (executors design §5.4); it
is not a record taken after the fact.

1. **The executor reports** its commit (`task_report`), as today.
2. **Shadows verifies the attempt,** after the report and before any done mark:
   - it runs the task's checks, as today;
   - it compares the code with the contract once the project is quiet (HEAD is the attempt's
     commit or a later one): for each contract file, the language server's classes, methods,
     constructors and functions (`documentSymbol`), matched to the contract's symbols by name (a
     bare name at any depth, or `Class.method`). A type-like kind (struct, interface, enum) counts
     as a class. A method's class is its type: the type an `impl` block names (`impl X`,
     `impl Trait for X`, generics stripped), or the receiver of a Go name `(*X).m` or `(X).m`;
     an `impl` block or a module is not itself a symbol. Each promised symbol is `found`, `missing` (not
     in a complete answer, or its file does not exist), or `unchecked` (no language server reads
     the file, or the answer was cut short and did not hold it). A top-level class or function in
     a file the task created that no contract names is `unpromised`.
   The result is stored with the attempt, beside its checks' results.
3. **Whoever marks done sees it first:** the person on the task page, the Planner in what it reads
   when it judges the attempt. A mark is made knowing what is missing.
4. **On done,** the verified result becomes the task's entries in the record (§3), written in the
   done mark's own write, so a mark and its record never disagree. A task marked done again (a
   rework) replaces its entries; a task marked not done loses them.

Nothing is compared for a task without a contract. The map part and layer of a file are found
when a question is answered (slice 2), not stored.

**A missing symbol goes back to the executor** (Mohammed, 2026-11-19), exactly as a failed check
does today (executors design §5.4), because it is one: the comparison is stored as the attempt's
check `contract`, beside the task's own checks, after them, on the same quiet project. So, with no
new path:
- a `contract` check that does not pass is returned to the same executor with its result ("T3
  promises `refresh_token` in `backend/auth/service.py`; it is not there"), within the same two
  returns as any check;
- after two returns it goes to the Planner with the results, which judges: an addition
  (`task_amend`), done with a reason, or a stop;
- a done mark while it has not passed needs a reason; the Planner's notice and the task page's
  Report tab show it as they show every check; a restart runs it again if it has no result.
Its outcome is passed (none missing), failed (one missing; its output names each in plain words,
and the unpromised ones), or `not_run` when nothing could be checked: that is shown on the task
page and in the Planner's notice with its reason, never silent (Mohammed, 2026-11-19): the notice gives the check's output after its line; it is
neither returned nor makes a done need a reason.

**The executor is told** in its brief that its contract's symbols are checked by name after its
commit, and to keep the names as written (Mohammed, 2026-11-19).

**A later change is caught** (Mohammed, 2026-11-19: in this slice): the project recheck after each
done mark runs the `contract` check of every done task too, so a task that removed a done task's
promised function is told to the Planner once, as a failing recheck is. A done task that a newer version of
its plan marks done too is not rechecked: the newest mark stands for the task (2026-11-19, from
the live run).

**Tests run a real language server** (Mohammed, 2026-11-19), beside fast tests over recorded
answers. Unpromised symbols alone do not fail it.

## 5. The project's state: Shadows answers first, the code confirms (Mohammed, 2026-11-19)

Shadows knows the project before it reads the code: its map, its roadmap, its plans and tasks, and
what each done task left (§3). A question is answered from that knowledge first; the language
servers then confirm it. `code_where` is the confirmation, not the source. When the two disagree,
the answer shows the gap; it never picks one silently.

```text
1. The Planner asks Shadows: "where is the login?"
2. Shadows answers from what it knows:
     part Backend, layer application, plan "Auth" v1, T3, backend/auth/service.py;
     the screen Login in part Frontend, web/src/login.tsx
3. The code confirms (language servers): login at line 12, used by LoginForm.submit,
   or: ⚠ refresh_token promised by T3, not in the file
```

### 5.1 Tables and screens: Design's elements, pinned by tasks

So that Shadows knows the project's shape and not only its files, the project's database tables
and its screens are **design elements**, owned by Design
([§18.13](../../core/specs/2026-10-03-project-planning-workspace-design.md#1813-tables-and-screens-design-elements-mohammed-2026-11-19)):
a stable id (`E3`), revisions, a part of the approved map (a table in a `database` part, a screen
in a `frontend` part, the standards' kinds), the roadmap outcomes it serves, and its content (a
table's columns with types, keys and the column each foreign key refers to; a screen's route and
the screens it leads to, and when). The Planner writes them directly in its running turn; the
person sees them in the plan's tasks that pin them.

A task contract **pins** the elements the task builds, changes or deletes
([§17 "The task contract"](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#the-task-contract)):
`{ id, revision, action, file, symbol_id? }`. It never copies columns or links. So two tasks
never disagree about a table's shape, a rename keeps the id, and a deletion is an element revision
that a task with action `delete` builds.

### 5.2 The diagrams are built from the design and the plans

The project's ER diagram (tables, columns, keys and relations) and its screen flow (screens and
where each leads; not their look) are **built from Design's elements and the tasks that pin
them**. They are read only in this slice. Editing a diagram later edits the same Design record
([vision §12](../../core/vision.md#12-clear-navigation-and-focused-work)): a new revision of the
element, by §18.13's rules.

An element's state comes from the pins of done tasks (the record, §3), the pins of tasks still to
do, and the folder. Revisions order them; no time or insertion order does.

| State | When |
|---|---|
| ⏳ `planned` | no done task pins it (whether or not a task of a live plan does yet) |
| ✅ `built` | the done pins at its **built revision** (the highest revision a done task pinned) all have their file in the folder, and each named symbol was `found` when its task was marked done |
| ⚠ `gap` | one of those pins' file is absent from the folder now, or its symbol was `missing` |
| ? `unchecked` | no gap, and a named symbol was `unchecked`, or the folder could not be read (the reason is given) |
| `removed` | its latest revision deletes it, and a done task pinned that revision with `delete`, or no done task ever built it |

`gap` outranks `unchecked`, which outranks `built`. Beside the state:
- **behind**: the built revision is older than the latest; the newer revision is shown as a planned
  change on the element;
- **confirmed**: `file_and_symbol` when every pin at the built revision names a symbol, else
  `file_only` (a table defined only in a SQL migration, which no language server reads);
- **done** and **planned**: the pins behind the state, each with its plan version, task, file and
  symbol. A planned pin is one of a task not done in the **live version** of an Active plan: its
  highest Frozen version. A Draft adds nothing until it is approved; an archived plan adds no
  planned pin, and its done pins stay in the record.

What is confirmed in this slice is the file and the symbol only. Columns and links are shown as
their revision says, marked "as planned", never as confirmed.

**OPEN (deeper confirmation):** confirming a table's columns against the migrations or the
database, and a screen's routes and links against the code. Closes when a task shows a `built`
table or screen that the code does not match. The states above stay; a check is added under them.

### 5.3 Asking

Shadows does not wait to be asked (Mohammed, 2026-11-21): every Planner turn's planning context
carries the record (files, purposes, symbols and their signatures, by part and layer), through
`CodeRecord::digest` (planner context spec §4.2, "Code record"). The questions below are for
what that block does not hold: a symbol's line now, its uses, and the live gaps.

Each question is an MCP tool for the Planner and a route for the web, calling the same method.

- **`project_state`** (slice 2a; argument `part_id`, optional): the overview from Shadows' own
  knowledge, read in one call, no language server asked. The project's stage and map revision;
  each part of the approved map in map order (at most 200, then a count of the rest) with its
  plans (at most 20 per part, then a count; a plan with no part is listed apart) and their tasks
  counted as done, started and not started from the plan's live version, and the counts of its
  elements by state; the elements in `gap` or `unchecked` with why (at most 100, then a count).
  With `part_id`, that part in full besides: its plans with each task (number, title, done, its
  latest attempt's state; at most 50 plans, then a count) and every element of the part with its
  content and state. A second route gives every element of the project with its state, for the
  diagrams (at most 500, Design's bound).
- **`code_where`** (slice 2b; one argument: a topic `login`, a path `backend/auth`, a symbol, an
  element id or name): Shadows' answer first, then the code's confirmation:
  1. The record's entries and the project's elements that match, ranked by a fixed order: an exact
     element id, element name or symbol name; then a path the topic is a prefix of; then a part
     whose title is the topic; then the words of the topic in a purpose or behavior. Ties by path,
     then element number. No fuzzy matching. At most 20 entries, then a count.
  2. For each matched symbol, its line now (`documentSymbol` of its file) and who uses it
     (`findReferences`, whose answer includes the callers, so `incomingCalls` is not asked and
     the budget goes further), grouped by the map part and layer of the using file (Design's
     `owners`), so "the database query" and "the screen" show as such. A topic that is not a path
     also asks `workspaceSymbol` once. A question asks the language servers at most 30 times and
     waits at most 20 s in all; what the budget does not reach is marked unchecked with that
     reason. A recorded file with no symbols is confirmed `file_only` when it exists.

     **Who uses a symbol is its own outcome** (2026-11-20, from the frontend's review), apart from
     the symbol's live confirmation (`live`). It is `uses_live`, one of:
     - `not_asked`: the symbol has no confirmed line;
     - `answered`: the server's complete answer, which may be empty;
     - `truncated`: the server's answer was cut at Codebase's bound;
     - `failed`, with why: an error, no server, or the time ran out during the lookup;
     - `skipped`, with why: the budget was spent before the lookup.

     `uses` keeps every reference the server returned, up to 10, also in a `truncated` answer.
     `uses_more` counts only the returned references beyond those 10, so it is a lower bound in a
     `truncated` answer and 0 when nothing was returned. `uses_total` is the exact count, given
     only when `answered`. An empty `uses` never stands for "no callers" unless `uses_live` is
     `answered`.
  3. The gaps on those entries.
  4. Files that match the topic in the project's file list and that no done task's contract
     names: their live facts (symbols, part and layer), marked "not in the record", never hidden.

Example answer:

```text
Login logic: backend/auth/service.py  (Backend · application; plan "Auth" v1, T3)
  login(email, password) -> Token      line 12   "checks the password, issues a token"
    calls  find_user        backend/db/users.py:30          (Database · queries)
    used by LoginForm.submit web/src/login.tsx:18           (Frontend · screens)
  verify_token(token) -> User          line 41
  ⚠ refresh_token promised by T3, not found in the file
Table E3 users ✅  db/migrations/0003_users.sql, model User (plan "Auth" v1, T2), r2 (behind: r3 planned)
Screen E7 Login ✅  /login, web/src/login.tsx → E8 Dashboard after a successful login
```

When a language server cannot answer, Shadows' part of the answer still comes, with its lines
marked unchecked and the reason; it never waits on the server beyond the budget or drops the
record's answer.

The Planner's instructions (slice 2a) say: read `project_state` before planning; write each table
and screen a task builds with `design_element_put` and pin it in the task's contract. Slice 2b
adds: before a contract names a file, ask `code_where`. B-167 and B-169 remain the guard when it
does not ask.

### 5.4 Seeing it

The web shows the project's state and its two diagrams, read only: the ER diagram for the
project's tables and the screen flow for its screens, each element with its state and, when
opened, its revisions, part, outcomes and the tasks that pin it, with file and symbol. A part's
page shows the elements of that part. The project event stream refreshes them on these events:
- `DesignElementChanged` (Design);
- `CodeRecordChanged` (CodeRecord, on every done or not-done mark's record write);
- `TaskProgressChanged` (Runs: an attempt starting, ending, stopped or interrupted, and every done
  or not-done mark; [executors design, the plan page](2026-11-08-executors-design.md#the-plan-page));
- the design, stage and plan events it carries today.

At caught-up, the client reads the state again. No view follows an executor's thread to learn a
task's progress (`docs/shared/requests/2026-11-19-project-state-and-diagrams.md`, amended
2026-11-20).

## 6. Code no plan wrote: moved to importing a project (Mohammed, 2026-11-19)

Shadows relies on its plans, contracts and standards. A project that existed before Shadows
enters it through **importing an existing codebase** (guided planning §23.1): agents analyse it,
map it into Shadows, and the import is accepted only once the code meets the standards. After an
import every file has a contract.

A project born in Shadows still holds files no contract names (scaffolding, generated files,
configuration, edits made without a contract). `code_where` gives their live facts marked "not in
the record" (§5.3 step 4); it records nothing for them.

**OPEN (import):** what the import writes for existing code (a contract per file, marked
described rather than decided, accepted by the person), its elements, and how `code_where` serves
the analysis. Closes when the import gets its design.

## 7. Not in scope

- Reading every file into a stored symbol table: the language servers stay the source of facts.
- Editing an element or a diagram by hand in this slice, and the screens' look (mockups).
- Confirming columns and links against the code (§5.2 OPEN).
- Changing what an executor reports.

## 8. Owner

| Owner | Owns | Why there |
|---|---|---|
| `Design` | the elements: identity, revisions, content, rules, writer (§18.13) | it holds the map, the roadmap and the decisions they are tied to |
| `Plans` | the contract's pins and their rules; which version of a plan is live | it owns the task contract and plan versions |
| `CodeRecord` (crate `code-record`) | the record, the pins of done tasks, each element's state (§5.2) and, in 2b, `code_where` | its one job is "whether the code holds what was promised" (Mohammed, 2026-11-19, over making it part of Plans) |
| `ProjectState` (crate `project-state`, new; approved by Mohammed, 2026-11-19) | `project_state`: composing the stage, the parts, the live plans with their task progress, and the element states | see below |
| `Harness` | the Planner's prompt (`prompt.txt`) and the executor's (`executor.md`) | it owns both texts; `Instructions` holds the person's own instructions, which this slice does not change |

Runs depends on CodeRecord and calls it when a mark is stored (Runs → CodeRecord → Codebase, and
Design and Plans for their types). Task progress is Runs' (`Runs::progress`), so a service that
answers "each part's plans and their tasks by status" sits above Runs. CodeRecord cannot: that
would be a cycle. Hence `ProjectState`, which holds no table and no rule of its own beyond the
composition and its bounds: it reads Design's stage, layout and elements, Plans' live versions,
Runs' progress, and asks CodeRecord for the element states, supplying the planned pins as facts.
(Changed on 2026-11-19 while planning 2a from "CodeRecord owns `project_state`", which assumed
CodeRecord could read task progress; approved by Mohammed the same day.)

`code_where` (2b) stays in CodeRecord: an element's state does not depend on the planned pins (they
only list the tasks to come), so CodeRecord answers it without Runs, calling Design's `elements`
and `layout` (CodeRecord → Design; Design depends on no service that depends on CodeRecord).

## 9. Slices

1. **Verification and the record:** the `contract` check after a report (§4), and the record's
   entries written on done and removed on not done. On `main` in PR #73 (`7b7abec`).
2. **The project's state, its diagrams and `code_where`** (Mohammed, 2026-11-19): two plans, both
   built and delivered on one branch (`ccr-9e1b02d8-b0ylm1`).
   - **2a** (plan `docs/backend/plans/2026-11-19-code-record-2a.md` at `4d69977d`): Design's elements; pins in the
     contract and the executor's task context; the record of pinned elements and their states;
     `project_state` (MCP tool and route); the routes and events the web's state view, ER diagram
     and screen flow need, after the shared request
     `docs/shared/requests/2026-11-19-project-state-and-diagrams.md` is accepted; the Planner told
     to read the state and write elements; acceptance on a real provider: a second plan pins the
     first plan's tables and screens by id from `project_state`, writes no duplicate, and the
     diagrams show them with their states in the browser.
   - **2b** (plan `docs/backend/plans/2026-11-19-code-record-2b.md` at `4d69977d`): `code_where` (§5.3), owned by
     CodeRecord, its MCP tool and route (after the shared request
     `docs/shared/requests/2026-11-19-code-where.md` is accepted), the Planner told to ask it
     before naming a file, and its acceptance: a second plan names the first plan's files without
     reading the folder.
3. **Code no plan wrote:** moved to importing a project (§6).

Each plan is in `docs/backend/plans/` before any of its tasks is dispatched.
