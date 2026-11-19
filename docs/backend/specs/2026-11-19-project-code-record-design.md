# The project's code record: the Planner asks, it never guesses

Status: approved by Mohammed, 2026-11-19: the direction (who writes which part) and the owner,
a new service (§8). Slice 1 is B-170. Whether the task page shows the gaps (§9) is open.

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

## 4. When it is made

When a task is marked done, Runs calls `CodeRecord::record_done` with the facts it owns (the
plan version, the task, and the judged attempt's commit). The record takes the task's contract
from Plans and the commit, asks the project's language server for each contract file's symbols (`documentSymbol`),
matches them to the contract by name and kind, and writes the file and symbol entries and the
gaps. A task marked done again later (a rework) replaces its own entries; a task marked not done
loses them (`CodeRecord::forget_done`). Nothing is written for a task without a contract. The
record is made after the mark is stored, off the mark's path, so a slow language server never
delays a done mark. The map part and layer of a file are found when a question is answered
(slice 2), not stored.

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

1. **The record on done:** entries and gaps written when a task is marked done; the task page
   shows the gaps. Tests: a done task with a contract writes its entries; a missing promised
   symbol is one gap; a rework replaces the entries.
2. **`code_where`:** the tool and its route, answered from the record and the language servers;
   the Planner's instructions ask it before naming a file. Acceptance: on a real provider, a
   second plan on a project names the first plan's files without reading the folder.
3. **Code no plan wrote** (§6): the live answer for such files.

**OPEN, for Mohammed:** whether slice 1 shows the gaps on the task page, or only to the Planner.
