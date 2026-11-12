# Language servers, the person's question card, and the Code settings (slice 2)

Status: approved by Mohammed, 2026-11-13. Replaces slice 2's B-97, B-98 and B-119 as
written in [repository discovery](../plans/2026-11-09-repository-discovery.md) (Tasks 2 and 3):
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
3. A missing server is installed only after the person chooses it on a question card the
   Planner writes.
4. The person sees and controls the servers in a new **Code** section of the settings.
5. The name index is suspended, not deleted.

## Non-goals

- Shadows installs nothing on its own; an install always follows the person's choice.
- No server is chosen by the person per project: the files choose.
- No diagnostics, rename, formatting or completion: the read operations below only.
- No answer is mixed from two sources: a file whose language has no running server gets
  "no server", never a guess.
- The AYLUL main assistant configuring servers from a link the person pastes is later
  ([AYLUL future](../../aylul/future.md), added with this spec).

## Anti-goals (failure even if it works)

- A model installs or runs anything the person did not choose on the card.
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
7. **A missing server:** the Planner asks the person on a question card whether Shadows
   installs it or the person does. On "Shadows installs it" the Planner calls
   `code_server_install`; Shadows runs the catalog's install command itself, never the
   Planner's text, and only after that choice.
8. **The question card is general.** The Planner writes the question and its options
   (buttons), and the person's choice returns to the Planner as its answer. Install is its
   first user, not its only one. No session restarts: the tools exist from the start.
9. **One slice**, backend and web together.

## How it fits the code

| Need | Built on | New |
|---|---|---|
| Servers per project | `core-code`'s one `Worker` per active project (`watch.rs`), its watcher and inventory | the worker owns the project's server clients; it stops parsing for the name index |
| Activity | `Code::touch` and the active set (`active.rs`) | a sent message activates; idle time replaces `active_limit` |
| Talking to a server | `shadows-process` (spawn, stdio, kill on drop) | `shadows-lsp`: one client for every server |
| Code tools | `where_is`, `who_uses`, `outline` (`shadows-mcp/src/code_tools.rs`) | `code_lsp` in their place, for the Planner and the executor |
| Waiting for the person | the look of the permission card (`permission-card.tsx`) | `person_ask`: a stored question in Turns, and its card |
| Telling the Planner | the waiting notice, `Turns::queue_notice` (B-125) | a sender that is not an executor: an answer, an install's outcome |
| Settings | the settings dialog and its sections (`web/src/app/settings`) | the Code section; "Active projects" removed |

The executor works in the project's own folder (the commit is the checkpoint), so the
server reads the same files the executor writes; there is no worktree to follow.

### The `shadows-lsp` crate

A library beside `shadows-index`, with no Shadows dependency but `shadows-process`. It
starts one server from a command, arguments and a folder; initializes it; keeps the files
it is asked about open with their current text; sends one request and returns its answer;
and shuts the server down (kill on drop). It knows no project, grant, task or table. One
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
- The person's added and switched-off servers are stored in `core-code`'s store, for every
  project (my call: settings are the person's, not a project's); the catalog is data in
  the crate.
- A linked project runs its own servers when it is active itself; touching a project no
  longer activates its links (my call).

### The question card (`person_ask`)

- MCP tool for a Planner conversation only (a thread grant; an executor or an external
  agent cannot call it): a question, two to five options (each a short label), and an
  optional `consent` on an option (below). It returns at once with the question's id; the
  card waits in the conversation.
- The question is stored, a thread entry of its own kind, so it survives a restart. The
  answer is a command with a command id, replayed as usual. A question is answered once.
- The person's click reaches the Planner as a waiting notice, as an executor's notice
  does. `Notice` names its sender today as `executor: ThreadId`; it becomes a sender that
  is either an executor thread or the person's answer / an install's outcome.
- **Owner: Turns** (my call). Turns already holds the waiting notice and sits above Harness
  and Code (`core-turns` depends on both), so Harness or Code could not queue the notice
  without a dependency cycle. The question store lives in Turns; the install is Turns
  calling `Code`, then queueing its outcome. Permission requests are live, in memory and
  AYLUL-only, so they are a model for the card's look, not for its storage.

### Install (`code_server_install`)

- MCP tool for a Planner conversation, taking a catalog server id.
- **Consent:** the Planner marks the install option with `consent: "server_install:<id>"`.
  The card shows the person what that option allows. `code_server_install` runs only when
  the person's latest answered question in this conversation chose an option with that
  consent for that id, and that consent was not used before (my call: one consent, one
  install).
- Shadows runs the catalog's install command for the machine's system through
  `shadows-process`, with a time limit, and records its outcome. The Planner gets the
  outcome as a notice; on success the server starts in the project.
- A catalog entry with no install command for this system cannot be installed by
  Shadows; the Planner offers only "I'll install it" (the card shows the command when one
  is known).
- Install commands are written by us per entry (the upstream manifests carry none), from
  each plugin's README. This slice writes Windows commands only, the system Shadows runs
  on today; on Linux and macOS every entry has no known install command, so the card
  offers only "I'll install it" there (my call). The data has a field per system so the
  others can be filled later:

  | Server | Command | Install (Windows) |
  |---|---|---|
  | clangd | `clangd --background-index` | `winget install LLVM.LLVM` |
  | csharp-ls | `csharp-ls` | `dotnet tool install --global csharp-ls` |
  | gopls | `gopls` | `go install golang.org/x/tools/gopls@latest` |
  | jdtls | `jdtls` | none known |
  | kotlin-lsp | `kotlin-lsp --stdio` | none known |
  | lua-language-server | `lua-language-server` | `winget install LuaLS.lua-language-server` |
  | intelephense | `intelephense --stdio` | `npm install -g intelephense` |
  | pyright | `pyright-langserver --stdio` | `npm install -g pyright` |
  | ruby-lsp | `ruby-lsp` | `gem install ruby-lsp` |
  | rust-analyzer | `rust-analyzer` | `rustup component add rust-analyzer` |
  | sourcekit-lsp | `sourcekit-lsp` | none known |
  | typescript | `typescript-language-server --stdio` | `npm install -g typescript-language-server typescript` |

  The winget ids are mine and are checked by the builder before they go in.

### Settings › Code (web)

A new section, meant to grow. It starts with:
- **Language servers:** every catalog and added server with its extensions and state in
  the open project; a switch per server (switching off warns that answers get worse); for
  a missing one, its install command.
- **Add a server:** command, arguments, extensions, with a line saying it is on the
  person's responsibility.
- "Active projects" is removed from General.

### Instructions

- **Planner:** `code_lsp` and its operations; when a language the project uses has no
  running server, ask the person with `person_ask` whether Shadows installs it, and call
  `code_server_install` only after that choice. `person_ask` for any other choice the
  person must make.
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
typescript-language-server on a card; choosing "Shadows installs it" installs it and
starts it; switching gopls off in Settings › Code shows the warning, and `code_lsp` then
answers "no server (off)".

## Left to the builder

- The idle time (a few minutes; Mohammed: "short").
- The server time limits for start and for one answer, and the result bounds.
- The exact MCP and HTTP shapes, following the existing tools and routes.
- How a server is found on the machine (PATH, and the Windows `.cmd` shims for npm).
- How the name index is switched off in the worker while its code stays compiled.
