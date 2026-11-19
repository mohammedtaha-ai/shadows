# The project's code record: the Planner asks, it never guesses

Status: approved by Mohammed, 2026-11-19: the direction (who writes which part), the owner, a new
service (§8), and that a task is verified against its contract before it is marked done (§4).
A missing symbol is returned to the executor as a failed check (§4). Slice 1's plan is
`docs/backend/plans/2026-11-19-code-record.md`.

Builds on the [task contract](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#the-task-contract)
(§17), the [language-server design](2026-11-13-language-servers-design.md) (`code_lsp`) and the
[executors design](2026-11-08-executors-design.md) (attempts, commits, done marks).

## 1. Why

A Planner that plans the next change to a project must know what is already there: which file
holds the login logic, which functions it has, which database query and which screen use it.
Today it reads the folder itself or guesses. B-167 and B-169 refuse a guess once it is written (a
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

## 5. Asking it: `code_where`

A new MCP tool for the Planner (and a route for the web), one argument: a topic (`login`), a path
(`backend/auth`) or a symbol name. It answers from the record first, then the code:

1. The record's entries whose path, part, purpose, symbol name or behavior match the topic,
   best first.
2. For each matched symbol, its line now (`workspaceSymbol` or `documentSymbol`), and who uses it
   (`findReferences`, `incomingCalls`), grouped by the map part and layer of the using file, so
   "the database query" and "the screen" show as such.
3. Gaps on those entries.

Example answer:

```text
Login logic: backend/auth/service.py  (Backend · application; plan "Auth" v1, T3)
  login(email, password) -> Token      line 12   "checks the password, issues a token"
    calls  find_user        backend/db/users.py:30          (Database · queries)
    used by LoginForm.submit web/src/login.tsx:18           (Frontend · screens)
  verify_token(token) -> User          line 41
  ⚠ refresh_token promised by T3, not found in the file
```

The Planner's instructions say: before a contract names a file, ask `code_where`; B-167 and B-169
remain the guard when it does not.

## 6. Code no plan wrote

A project that existed before Shadows has files no contract describes. For them `code_where`
answers the facts only (symbols, lines, uses, part and layer from the map), marked "no plan says
why". Recording an intent for such code (the Planner describing a module it read) is left for a
later slice.

## 7. Not in scope

- Reading every file into a stored symbol table: the language servers stay the source of facts.
- A record for files outside any done task's contract, beyond §6's live answer.
- Changing what an executor reports.

## 8. Owner

A new service, `CodeRecord` (crate `code-record`), owns the record's tables and the rule "what a
done task left in the code" (Mohammed, 2026-11-19, over making it part of Plans). It depends on
Plans (the contract) and Codebase (the language servers), and on Design for the map in slice 2.
Runs depends on it and calls it when a mark is stored, supplying the commit; so dependencies stay
one way (Runs → CodeRecord → Plans, Codebase). `code_where` is its method.

## 9. Slices

1. **Verification and the record:** the `contract` check after a report (§4), and the record's
   entries written on done and removed on not done.
2. **`code_where`:** the tool and its route, answered from the record and the language servers;
   the Planner's instructions ask it before naming a file. Acceptance: on a real provider, a
   second plan on a project names the first plan's files without reading the folder.
3. **Code no plan wrote** (§6): the live answer for such files.

Each slice gets its plan in `docs/backend/plans/` before any task is dispatched.
