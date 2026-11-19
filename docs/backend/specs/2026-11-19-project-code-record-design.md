# The project's code record: the Planner asks, it never guesses

Status: approved by Mohammed, 2026-11-19: the direction (who writes which part), the owner, a new
service (§8), and that a task is verified against its contract before it is marked done (§4).
A missing symbol is returned to the executor as a failed check (§4). Amended by Mohammed on
2026-11-19 for slice 2: Shadows answers from its own knowledge and the code confirms (§5), tables
and screens join the contract and the ER and screen-flow diagrams are built from the plans
(§5.1, §5.2), and code no plan wrote moves to importing a project (§6). Slice 1 is on `main` in PR #73
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
- **Table and screen** (slice 2, §5.1): each one the done task's contract names, with its shape
  from the contract (columns and keys; route and where it leads), its file and symbol, and its
  state from the verdict (§5.2).

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

### 5.1 Tables and screens in the task contract

So that Shadows knows the project's shape and not only its files, a task contract names the
database tables and the screens a task builds, beside its files and symbols (§17 "The task
contract", amended 2026-11-19). The Planner writes them as it writes symbols today; nobody else
does.

- **A table:** id, name, action (`create`, `modify`, `delete`), purpose, the contract file that
  defines it (a migration or a model), optionally the id of the contract symbol that models it
  (a class), and its columns as they are after the task: name, type as the project writes it,
  meaning, primary key, and the `table.column` a foreign key refers to.
- **A screen:** id, name, purpose, its route when it has one (`/login`), the contract file that
  holds it, optionally the id of the contract symbol that is it (a component), and where it leads:
  the screens it goes to and when ("after a successful login").

A table's or screen's file must be one of the task's `create` or `modify` files; its symbol, when
given, one of the task's symbols. Ids are unique within the task. Names are the project's: one
table or screen name means one thing across the project's plans.

### 5.2 The diagrams are built from the plans

The project's ER diagram (tables, columns, keys and relations) and its screen flow (screens and
where each leads; not their look) are **built from the roadmap, the plans and their tasks**,
never drawn or edited by hand. To change a diagram, the plan changes; the diagram follows.

Each element has one state, from its tasks:

| State | When |
|---|---|
| ⏳ planned | a task of a live plan names it and is not done |
| ✅ built | its latest done task's file exists and its symbol, when named, is `found` |
| ⚠ gap | its latest done task's file is absent, or its symbol is `missing` |
| ? unchecked | done, and no language server could confirm it |

When several tasks name one table or screen, the latest done task gives its shape now, and the
not-done tasks that name it show as planned changes on it. A task of an older plan whose
contract has no tables or screens adds nothing: it is not rewritten.

What is confirmed in this plan is the file and the symbol only. A table's columns and a screen's
arrows are shown as the plan says, marked "as planned", never as confirmed.

**OPEN (deeper confirmation):** confirming a table's columns against the migrations or the
database, and a screen's routes and links against the code. Closes when a task shows a
"✅ built" table or screen that the code does not match. The states above stay; a check is added
under them.

### 5.3 Asking

Two questions, one owner (§8), each an MCP tool for the Planner and a route for the web, calling
the same method:

- **`project_state`** (no argument): the overview from Shadows' own knowledge. The project's
  stage on the roadmap; each map part with its plans and their tasks by status; its tables and
  screens with their states; the gaps. Bounded: a large project is summarised per part, and a
  part is asked for in full by its id.
- **`code_where`** (one argument: a topic `login`, a path `backend/auth`, a symbol, a table or a
  screen): Shadows' answer first, then the code's confirmation:
  1. The record's entries, and the planned tables and screens, whose path, part, purpose, name or
     behavior match the topic, best first.
  2. For each matched symbol, its line now (`workspaceSymbol` or `documentSymbol`) and who uses it
     (`findReferences`, `incomingCalls`), grouped by the map part and layer of the using file
     (Design's `owners`), so "the database query" and "the screen" show as such.
  3. The gaps on those entries.

Example answer:

```text
Login logic: backend/auth/service.py  (Backend · application; plan "Auth" v1, T3)
  login(email, password) -> Token      line 12   "checks the password, issues a token"
    calls  find_user        backend/db/users.py:30          (Database · queries)
    used by LoginForm.submit web/src/login.tsx:18           (Frontend · screens)
  verify_token(token) -> User          line 41
  ⚠ refresh_token promised by T3, not found in the file
Table users ✅  migrations/0003_users.sql, model User (plan "Auth" v1, T2)
Screen Login ✅  /login, web/src/login.tsx → Dashboard after a successful login
```

When a language server cannot answer, Shadows' part of the answer still comes, with its lines
marked unchecked and the reason; it never waits on the server or drops the record's answer.

The Planner's instructions say: before a contract names a file, a table or a screen, ask
`project_state` or `code_where`; write the tables and screens a task builds into its contract.
B-167 and B-169 remain the guard when it does not ask.

### 5.4 Seeing it

The web shows the project's state and its two diagrams, read only: the ER diagram for the
project's tables and the screen flow for its screens, each element with its state and, when
opened, its plan, task, file and symbol. A part's page shows the elements of that part.

## 6. Code no plan wrote: moved to importing a project (Mohammed, 2026-11-19)

Shadows relies on its plans, contracts and standards. A project that existed before Shadows
enters it through **importing an existing codebase** (guided planning §23.1): agents analyse it,
map it into Shadows, and the import is accepted only once the code meets the standards. After an
import every file has a contract, so the earlier "facts only, no plan says why" answer is not
built.

**OPEN (import):** what the import writes for existing code (a contract per file, marked
described rather than decided, accepted by the person) and how `code_where` serves the analysis.
Closes when the import gets its design.

## 7. Not in scope

- Reading every file into a stored symbol table: the language servers stay the source of facts.
- Drawing or editing a diagram by hand, and the screens' look (mockups).
- Confirming columns and arrows against the code (§5.2 OPEN).
- Changing what an executor reports.

## 8. Owner

A new service, `CodeRecord` (crate `code-record`), owns the record's tables and the rule "what a
done task left in the code" (Mohammed, 2026-11-19, over making it part of Plans). It also owns
the project's state, the diagrams built from it and `code_where` (Mohammed, 2026-11-19): one
owner for "what Shadows knows of the project's code, and whether the code agrees". It depends
on Plans (the contracts, and the tasks of live plans), Codebase (the language servers) and Design
(the map and the roadmap stage). Runs depends on it and calls it when a mark is stored,
supplying the commit; so dependencies stay one way (Runs → CodeRecord → Plans, Codebase, Design).

Plans owns the contract's tables and screens and their rules (§5.1); the Planner's instructions
are Instructions'.

## 9. Slices

1. **Verification and the record:** the `contract` check after a report (§4), and the record's
   entries written on done and removed on not done. On `main` in PR #73 (`7b7abec`).
2. **The project's state, its diagrams and `code_where`** (one plan, Mohammed, 2026-11-19), in
   this order:
   1. tables and screens in the contract (§5.1), and the Planner told to write them;
   2. the record keeps a done task's tables and screens with their states, and `project_state`
      answers (§5.2, §5.3);
   3. `code_where`: Shadows' answer confirmed by the language servers, and the Planner told to ask
      first (§5.3);
   4. the web's state view, ER diagram and screen flow (§5.4);
   5. acceptance on a real provider: a second plan on a project names the first plan's files,
      tables and screens from `project_state` and `code_where` without reading the folder, and the
      diagrams show them with their states in the browser.
3. **Code no plan wrote:** moved to importing a project (§6).

Each slice gets its plan in `docs/backend/plans/` before any task is dispatched.
