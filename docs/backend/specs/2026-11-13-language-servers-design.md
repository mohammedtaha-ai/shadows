# Language servers, the person's question card, and the Code settings (slice 2)

Status: approved by Mohammed, 2026-11-13. Replaces slice 2's B-97, B-98 and B-119 as
written in repository discovery (`docs/backend/plans/2026-11-09-repository-discovery.md` at `db760593`) (Tasks 2 and 3):
their per-person server configuration, `configure_server`, the PHP focus, the
thirteen-descriptor catalog and the `Stale`/`Ambiguous` machinery are not built.

## Why

The Planner plans on code it cannot follow. Today it has `project_files` and a read-only
look at files; `where_is`, `who_uses` and `outline` are given only to executors and external
agents (`core-grants/src/tools.rs`). Those three answer from a tree-sitter name index
(`shadows-index`) that knows Rust, TypeScript and JavaScript only, and finds a name where it
is written, not the function a call really reaches. On the Go projects of the trials
(`testing2`, `testing3`) they answered nothing.

Claude Code gets real answers from the official `*-lsp` plugins, but the Planner also runs
on Codex, which cannot. So Shadows itself runs the project's language servers and hands
their operations, as they are, to every Planner and executor through one tool of its own,
whichever program runs them.

## Goals

1. A project's language servers start on their own from its files' extensions, and only
   while a conversation in it is running.
2. The Planner and the executor ask the language servers through one tool, `code_lsp`,
   carrying the LSP operations as they are.
3. The Planner installs a missing server itself, through its own shell; its program
   (Claude Code or Codex) asks the person when it judges a command needs it, and that
   question reaches the person as a card in the conversation.
4. Shadows knows every project file the Planner changes in a turn, and asks it why when it
   did not say; the answer is recorded.
5. The person sees and controls the servers in a new **Code** section of the settings.
6. The name index is suspended, not deleted.

## Non-goals

- Shadows installs nothing on its own: Shadows runs no install command; the Planner does,
  under its program's approval.
- No server is chosen by the person per project: the files choose.
- No diagnostics, rename, formatting or completion: the read operations below only.
- No answer is mixed from two sources: a file whose language has no running server gets
  "no server", never a guess.
- The AYLUL main assistant configuring servers from a link the person pastes is later
  ([AYLUL future](../../aylul/future.md), added with this spec).

## Anti-goals (failure even if it works)

- A project file changes in a Planner turn and nothing records it.
- A question the Planner's program asks the person is refused unseen.
- A project nobody is working in keeps servers and watchers running.
- A Planner on Codex gets a worse answer than one on Claude Code.

## Decisions (Mohammed, 2026-11-12 and 2026-11-13)

1. **The catalog** is the twelve `lspServers` definitions of
   [claude-plugins-official](https://github.com/anthropics/claude-plugins-official/tree/main/plugins)
   (`*-lsp/.claude-plugin/plugin.json`), copied as one data file with the Apache-2.0
   notice. Each entry is enabled by default.
2. **The files choose the servers.** A server starts when the project holds a file with
   one of its extensions. A new extension (a setup task writing the first `.go` file) is
   seen by the project's watcher and starts its server. A project with several languages
   runs one server per language, and a question goes to the server of the file it names.
3. **The LSP operations as they are, not wrappers.** One tool, `code_lsp`, for the
   Planner and the executor. `where_is`, `who_uses` and `outline` leave every tool list
   (Planner, executor, external agents), and the executor's instructions name `code_lsp`.
4. **The name index is suspended, not removed.** Its code stays; nothing calls it, and the
   project worker stops parsing files for it. It returns only if a later need names it
   (an OPEN entry in [Core future](../../core/future.md), added with this spec).
   The file inventory (`project_files`, `project_refresh`) is a separate job and stays.
5. **Activity is a running conversation.** The "Active projects" limit goes. A project
   becomes active when a message is sent in one of its conversations (a new one or one
   continued), and stops (watcher and servers) after a short idle time with no use.
   Opening the project in the browser, creating it, or reading its thread list no longer
   activates it (today `Code::touch` fires on all of those).
6. **The person may switch a server off** in Settings › Code; the switch warns that
   Shadows' answers get worse. **The person may add a server** not in the catalog (its
   command, arguments and extensions), on their own responsibility, for community servers.
7. **The Planner has a shell** (Mohammed, 2026-11-13; this replaces "the Planner itself
   writes no file" of [executors design §5.2](2026-11-08-executors-design.md#52-setup-before-detailed-planning-mohammed-2026-11-11)).
   On Claude Code it gets `Bash` and runs in `auto` mode; on Codex its sandbox lets it write
   and its approval policy asks on request ("approve for me"). The program decides when to
   ask the person; Shadows does not second-guess it (accepted: an install the program
   judges safe runs without a question). Its instructions say: the shell is for installing
   and the environment (a language server, a container, a version check); do not change the
   project's files, which executors change through commits; if you must, call
   `project_change_note` with the paths and the reason.
8. **The program's questions reach the person.** A permission request from a Planner
   session (Claude Code's, through ACP; Codex's `requestApproval`) is held and shown as a
   permission card with its options, as AYLUL's are today, never refused unseen.
9. **Shadows knows the Planner's changes.** At the end of each Planner turn, Shadows takes
   the project files that changed during the turn without a commit (its edit tool calls,
   which carry their path, and the project's watcher, which sees a shell's writes too).
   Paths the Planner noted with `project_change_note` in that turn are recorded with their
   reason. The rest are recorded as "no reason given", and Shadows sends the Planner a
   notice naming them and asking why; its answer is recorded the same way. A change the
   person makes by hand during a Planner turn is rare and accepted as asked about.
10. **The question card is general.** The Planner writes the question and its options
   (buttons), and the person's choice returns to the Planner as its answer, for any choice
   the person must make. It grants nothing. No session restarts: the tools exist from the
   start.
11. **A missing server:** `code_lsp` answers "not installed" with the catalog's install line;
   the Planner asks the person with the question card whether it installs it or the person
   does, then runs the line in its shell or leaves it.
12. **One slice**, backend and web together.
13. **`core-code` becomes `codebase`** (`Code` becomes `Codebase`), named for what it
   owns, without `core-`; the other `core-*` crates keep their names until touched. This
   document names today's code (`core-code`, `Code`) where it describes what exists.

## How it fits the code

| Need | Built on | New |
|---|---|---|
| Servers per project | `core-code`'s one `Worker` per active project (`watch.rs`), its watcher and inventory | the worker owns the project's server clients; it stops parsing for the name index |
| Activity | `Code::touch` and the active set (`active.rs`) | a sent message activates; idle time replaces `active_limit` |
| Talking to a server | `shadows-process` (spawn, stdio, kill on drop) | `shadows-lsp`: one client for every server |
| Code tools | `where_is`, `who_uses`, `outline` (`shadows-mcp/src/code_tools.rs`) | `code_lsp` in their place, for the Planner and the executor |
| Waiting for the person | the look of the permission card (`permission-card.tsx`) | `person_ask`: a stored question in Turns, and its card |
| The program's permission requests | AYLUL's held requests and their card (`core-harness/permissions.rs`) | held for Planner sessions too, on Claude Code and Codex |
| The Planner's file changes | tool cards with kind, path and diff (`ToolCallCard`); the project watcher | the changes of one turn, `project_change_note`, the end-of-turn notice |
| Telling the Planner | the waiting notice, `Turns::queue_notice` (B-125) | a sender that is not an executor: Shadows asking about a change |
| Settings | the settings dialog and its sections (`web/src/app/settings`) | the Code section; "Active projects" removed |

The executor works in the project's own folder (the commit is the checkpoint), so the
server reads the same files the executor writes; there is no worktree to follow.

### The `shadows-lsp` crate

A library beside `shadows-index`, with no Shadows dependency but `shadows-process`. It
starts one server from a command, arguments and a folder; initializes it; tells it which
files changed on disk (`workspace/didChangeWatchedFiles`, which the project's watcher
feeds: a server such as gopls keeps its own copy of closed files and reads them again only
when told, Codex's trial on gopls v0.23.0, 2026-11-13); opens the asked file with its
current text for one request, then closes it; and shuts the server down (kill on drop). It knows no project, grant, task or table. One
client serves every server (`async-lsp`, infrastructure dependency). A server gets code of
its own only when it needs it.

### `code_lsp`

- Input: `operation`, `file` (project-relative), `line` and `character` (one-based, as an
  editor shows them; `documentSymbol` and `workspaceSymbol` take none, `workspaceSymbol`
  takes `query`), and `project` as the code tools take it today.
- Operations: `goToDefinition`, `findReferences`, `hover`, `documentSymbol`,
  `workspaceSymbol`, `goToImplementation`, `incomingCalls`, `outgoingCalls`, the same set
  as Claude Code's LSP tool, so a Planner reads the same answers on Codex and on Claude Code.
- The server of the file's extension answers; `workspaceSymbol` asks every running server
  of the project. Before a request, the file's current text on disk is sent to the server.
- Locations come back project-relative with one-based lines. A location outside the
  project and its linked projects is dropped. Results are bounded (count and bytes).
- No server for the file: the answer says so with the server's state (`not_installed`,
  `off`, `failed`, `starting`) and, when not installed, its install command. Nothing
  falls back to the name index.
- The scope rules of today's code tools hold: an executor sees what its grant allows.

### Code owns the servers

- `Code` reads the catalog, the person's added servers and switched-off servers, and the
  project's extensions, and decides which servers the project's worker runs. The rule
  is one method on `Code`.
- A server's state per project: `running`, `starting`, `not_installed` (its command is not
  found), `off` (the person switched it off), `failed` (it exited or did not initialize,
  with its last error line).
- The person's added and switched-off servers are stored in `codebase`'s store, for every
  project (my call: settings are the person's, not a project's); the catalog is data in
  the crate.
- A linked project runs its own servers when it is active itself; touching a project no
  longer activates its links (my call).

### The question card (`person_ask`)

- MCP tool for a Planner conversation only (a thread grant; an executor or an external
  agent cannot call it): a question and two to five options (each a short label). It
  returns at once with the question's id; the card waits in the conversation. It grants
  nothing.
- The question is stored, a thread entry of its own kind, so it survives a restart. The
  click is the person's own message to the Planner (`Answer to "<question>": <label>`),
  recorded with the choice in one transaction; the card then reads its choice. A question
  is answered once; a card copied into a fork is history, with no buttons.
- **Owner: Turns** (my call). Turns records the person's messages and sits above Harness
  and Codebase (`core-turns` depends on both).

### The Planner's shell and its changes

- **Shell:** `Bash` joins `PLANNER_TOOLS` on Claude Code, in `auto` mode. A Codex
  conversation has two modes, the person's choice (Mohammed, 2026-11-13, B-146 in `docs/backend/stages.md` at `db760593`):
  "Approve for me" (`ask`, the default; renamed from "Ask to approve" by Mohammed,
  2026-11-13: sandbox `workspace-write`, approval `on-request`, approvals reviewer
  `auto_review`, so Codex's own reviewer allows or denies each request and none
  reaches the person as a held card, since codex-cli 0.167.0's reviewer answers only
  allow or deny; the id stays `ask`, the one stored projects and threads hold) and "Read only" (`read-only`: sandbox `read-only`, approval
  `never`, reviewer `user`, nothing asked). A thread opens in the default mode when its project allows it,
  otherwise in the project's first allowed mode: a project made before B-146 (`docs/backend/stages.md` at `db760593`) allows
  `read-only` only (migration 0050) and keeps it until the person adds `ask`. The picked
  mode is sent
  with every `turn/start` as `sandboxPolicy`, `approvalPolicy` and `approvalsReviewer`,
  and the opening refuses a thread whose answer does not echo the default mode's three
  (`shadows-agent/src/codex/session.rs`). In Read only the Planner cannot install a
  missing server, so its `person_ask` offers only "you install it" (with the install
  line) or "leave it"; the prompt states this against the Planner's own sandbox.
- **Permission cards:** `Harness::waiting_permissions` holds and lists requests from Planner
  sessions as well as AYLUL's; Codex's `requestApproval` is held with its options instead
  of declined. The person's answer goes back through the route that exists
  (`/api/threads/{id}/session/permissions/{request_id}`).
- **`project_change_note { paths, reason }`:** a Planner MCP tool that records why it
  changed project files; each note is a thread entry.
- **The end-of-turn check:** while a Planner turn runs, the project's `ActiveHold`
  (Codebase) collects the paths its watcher sees change, `.git/` left out. When the turn
  settles, Turns compares them with the turn's notes: unnoted paths are recorded as a
  thread entry "changed with no reason given", and a Shadows notice asks the Planner why.
  One notice per turn; the answer turn's own changes are checked the same way.
- **Install lines** are written by us per catalog entry (the upstream manifests carry
  none), from each plugin's README, and shown by `code_lsp` and Settings › Code. Windows
  only in this slice; the data has a field per system:

  | Server | Command | Install (Windows) |
  |---|---|---|
  | clangd | `clangd --background-index` | `winget install LLVM.LLVM` |
  | csharp-ls | `csharp-ls` | `dotnet tool install --global csharp-ls` |
  | gopls | `gopls` | `go install golang.org/x/tools/gopls@latest` |
  | jdtls | `jdtls` | none known |
  | kotlin-lsp | `kotlin-lsp --stdio` | none known |
  | lua | `lua-language-server` | `winget install LuaLS.lua-language-server` |
  | intelephense | `intelephense --stdio` | `npm install -g intelephense` |
  | pyright | `pyright-langserver --stdio` | `npm install -g pyright` |
  | ruby-lsp | `ruby-lsp` | `gem install ruby-lsp` |
  | rust-analyzer | `rust-analyzer` | `rustup component add rust-analyzer` |
  | sourcekit-lsp | `sourcekit-lsp` | none known |
  | typescript | `tsc --lsp --stdio` | `npm install -g typescript` |

  The winget ids are mine and are checked by the builder before they go in. The first
  column is the server's id as the upstream catalog writes it: Lua's is `lua` (its
  command is `lua-language-server`), kept so the catalog matches upstream (Mohammed,
  2026-11-13).

  **typescript departs from upstream** (Mohammed, 2026-11-13). Upstream runs
  `typescript-language-server --stdio`, which needs TypeScript's `lib/tsserver.js`.
  TypeScript 7, what `npm install -g typescript` now installs, ships no `tsserver.js`, so
  that server refused to start in any project without its own TypeScript 6 or older
  ("Could not find a valid TypeScript installation"). TypeScript 7's own language server,
  `tsc --lsp --stdio` (`typescript-go`), runs instead; the entry keeps upstream's id and
  extensions. If it falls short (an operation the Planner needs that it does not answer),
  the fallback is upstream's server with an older TypeScript:
  `npm install -g typescript-language-server typescript@6`.

### Settings › Code (web)

A new section, meant to grow. It starts with:
- **Language servers:** every catalog and added server with its extensions and state in
  the open project; a switch per server (switching off warns that answers get worse); for
  a missing one, its install command.
- **Add a server:** command, arguments, extensions, with a line saying it is on the
  person's responsibility.
- "Active projects" is removed from General.

### Instructions

- **Planner:** `code_lsp` and its operations; the shell is for installing and the
  environment, not for the project's files; a file it must change is noted with
  `project_change_note`; a language with no running server is offered to the person with
  `person_ask` ("I install it" / "you install it" / "leave it"), then installed in the
  shell on the first. `person_ask` for any other choice the person must make.
- **Executor:** `code_lsp` replaces "ask `where_is`, `who_uses` and `outline`"
  (`core-harness/src/executor.md`).

## Tests

Real, per the program's rulings: a real temporary folder, the real installed `gopls`
and `rust-analyzer`, the live Planner (ignored unless its key or Codex home is set). The
new `shadows-lsp` client is tested against `gopls`. A server that is not installed is
tested with a command that does not exist. Existing `fake-acp` tests stay; tests of the
three removed tools are removed with them.

Acceptance in the browser on `testing3` (Go): a conversation starts gopls; the Planner's
`code_lsp` `findReferences` answers from it; adding a `.ts` file makes the Planner ask about
the TypeScript server on a card; choosing "install it" makes the Planner run the
install line (with Claude Code's or Codex's permission card when it asks) and the server
starts; a file the Planner edits without a note brings Shadows' question; switching gopls off in Settings › Code shows the warning, and `code_lsp` then
answers "no server (off)".

## Left to the builder

- The idle time (a few minutes; Mohammed: "short").
- The server time limits for start and for one answer, and the result bounds.
- The exact MCP and HTTP shapes, following the existing tools and routes.
- How a server is found on the machine (PATH, and the Windows `.cmd` shims for npm).
- How the name index is switched off in the worker while its code stays compiled.
