# The code record, slice 2b: `code_where`

Design: [`../specs/2026-11-19-project-code-record-design.md`](../specs/2026-11-19-project-code-record-design.md)
§5, §5.3 (`code_where`), §6 (files no contract names), §8, §9 (2b).
Shared request for the route: [`../../shared/requests/2026-11-19-code-where.md`](../../shared/requests/2026-11-19-code-where.md).
Branch: `ccr-9e1b02d8-b0ylm1`, after plan 2a ([`2026-11-19-code-record-2a.md`](2026-11-19-code-record-2a.md)).
Dispatch: the controller runs the builders and fixers (Mohammed, 2026-11-19); the backend lead
prepares the tasks and checks the reports.

## Rulings for every task

From Mohammed and the design, 2026-11-19, unless marked otherwise.

1. **Owner:** `CodeRecord` owns `code_where` (design §8): one method, `CodeRecord::code_where`. The
   MCP tool and the route call it and nothing else.
2. **Dependencies:**
   - CodeRecord gains the `Design` service: `elements` and `layout`, and the path-ownership rule
     `ProjectLayout::owners`. It does not call Plans, Runs or ProjectState.
   - An element's state comes from `element_states(project, &elements, &[])`. The planned pins
     only fill the `planned` list, which `code_where` does not show; `project_state` shows it.
3. **Shadows answers first.** The record's half of the answer is built from CodeRecord's tables and
   Design's elements, with no language server. It always comes back, even when every server
   fails or the budget runs out.
4. **Ranking is fixed** (design §5.3). Four tiers, in order: `exact`, `path`, `part`, `words`. Ties
   go by path, then element number, then symbol name. No fuzzy matching and no scores. At most 20
   entries, then a count.
5. **Budget:**
   - at most 30 language-server asks per question (`documentSymbol`, `findReferences`,
     `workspaceSymbol` each count one; `missing_inputs` and `inventory` do not);
   - at most 20 s for the live half in all;
   - at most 10 uses listed per symbol, then a count;
   - what the budget does not reach is `unchecked` with the reason;
   - no retry waits for a starting server: a `NoServer` is `unchecked` at once, unlike the
     `contract` check's 2-minute wait.
6. **`incomingCalls` is not asked.** `findReferences` includes the callers, and one ask per symbol
   keeps the budget for more entries (design §5.3, amended while planning 2b).
7. **Not in the record:** files that match the topic and that no done task's contract names are
   listed with their live facts, marked `not_in_record`. Nothing is recorded for them (design §6).
8. **`file_only`:** a recorded file with no recorded symbol that exists now is confirmed
   `file_only`. An element keeps the `confirmed` of its state (2a).
9. **No migration.** 2b reads the tables 0076 and 0078 made.
10. **The frontend sees nothing before its request is accepted.** The route (B-185) waits for
    `docs/shared/requests/2026-11-19-code-where.md`. The MCP tool (B-184) is not frontend-visible.

## What is reused

| Existing piece | Where | How 2b uses it |
|---|---|---|
| The record's tables `code_record_file`, `code_record_symbol` (contract text in `contract_json`, done state) | migration `0076_code_record.sql`, `crates/code-record/src/store/mod.rs` | The record's half: files with purpose, plan, task and commit; symbols with behavior and their state at done |
| `code_record_element` and `CodeRecord::element_states` (2a, B-178) | `crates/code-record/src/elements.rs` | Element entries and their state, `behind` and `confirmed` |
| `Design::elements` (2a, B-175) | `crates/core-design/src/elements.rs` | Element matching by id or name and words of the purpose |
| `Design::layout` and `ProjectLayout::owners` | `crates/core-design/src/layout.rs` | Part and layer of every file and every use; the part tier (a part's folder) |
| `CodeRecord::document_symbols`, `matches`, `owner`, `receiver` (the parser of a `documentSymbol` answer) | `crates/code-record/src/verify.rs` | The live line of each recorded symbol; a recorded symbol now absent is a live gap. `Found` gains the one-based `line` and `character` that Codebase's rewrite puts on every symbol (`located`) |
| `ask`'s recorded-answer seam and its `LspAnswer` to `FileAnswer` translation | `crates/code-record/src/verify.rs`, `RecordedAnswer` | Extracted into two private helpers shared by `ask` and the new no-wait ask. The seam gains two answers for tests |
| `Codebase::lsp` (`DocumentSymbol`, `FindReferences`, `WorkspaceSymbol`), its one-based `{ path, line, character }` locations, 100 items / 32 KiB bound | `crates/codebase` | The live half |
| `Codebase::missing_inputs` | `crates/codebase` | Absent recorded files, in one call, outside the ask budget |
| `Codebase::inventory` with a prefix | `crates/codebase` | "Not in the record" files under a path topic, at most one page of 200 |
| The MCP tool pattern (`decision_tools.rs`), `Grants::tools_for`, the refusal mapping | `crates/shadows-mcp/src/`, `crates/core-grants` | The `code_where` tool |
| The decisions routes' shape | `crates/shadows-http/src/design.rs` | The `code_where` route |
| Real-server tests marked `#[ignore = "needs pyright-langserver on PATH"]` | `crates/code-record/tests/verify.rs` | The live tests of 2b |

**Not reused:**
- **The name index's `where_is`, `who_uses` and `outline`** (`Codebase::definitions`,
  `references`, `outline`): they are off in `AppCore` (`name_index: false`, "the language servers
  answer instead") and listed in no grant.
- **The `contract` check's `ask`, as is:** it waits up to 2 minutes for a starting server, which a
  question may not.

**New, and why nothing existing fits:**
- **`store/lookup.rs`:** the record has no read method yet (slice 1 left it for `code_where`).
- **`ranking.rs`:** a pure rule with one job, the tiers.
- **`code_where.rs`:** the question.
- **Two `RecordedAnswer` variants:** tests need recorded `findReferences` and `workspaceSymbol`
  answers, as they have recorded `documentSymbol` ones.

## Gate files every task here touches

No migration and no table. A new module gets its row in `docs/codebase/README.md`. Test modules name
their imports.

Every task's Done when includes:
- `cargo test -p shadows --test architecture --test codemap`;
- `cargo test -p shadows-core --features test-support --test contracts`;
- `cargo fmt --all --check`;
- both clippy commands of `docs/backend/AGENTS.md` rule 5;
- the 100-character line check there.

The LSP was not available while this plan was written: callers of `CodeRecord::new` are named by
the compiler (B-182 says which ones are expected).

---

## Task B-182 · `code_where`, Shadows' half: the record read and ranked

**Files:**
- **Create:**
  - `crates/code-record/src/code_where.rs`;
  - `crates/code-record/src/ranking.rs`;
  - `crates/code-record/src/store/lookup.rs`;
  - `crates/shadows-core/tests/code_where.rs` (through `AppCore`, whose fixture has projects, a
    Planner grant with a running turn, Design's seed and `app.code_record()`'s recorded answers).
- **Modify:**
  - `crates/code-record/src/lib.rs` (field `design: Design`; `new(storage, code, design)`; modules;
    re-exports);
  - `crates/code-record/src/model.rs`, `src/error.rs`, `src/store/mod.rs` (module line);
  - `crates/code-record/Cargo.toml` (`core-design` becomes a service dependency, not types only:
    edit its comment);
  - `crates/code-record/contract.yaml`;
  - `crates/shadows-core/src/app.rs` (`CodeRecord::new(storage.clone(), code.clone(),
    design.clone())`; the doc row's job stays "whether the code holds what a contract promised");
  - `crates/shadows-core/src/lib.rs` (re-exports);
  - `crates/shadows-core/src/error.rs` (`CodeRecordError::InvalidTopic` is `INVALID_COMMAND`);
  - every other `CodeRecord::new` call the compiler names. Expected: the fixtures in
    `crates/code-record/tests/verify.rs`, `tests/record.rs` and `tests/elements.rs`. Each passes
    `Design::new(storage.clone(), core_operations::Operations::new(storage.clone()))`, as
    `AppCore::design_for_tests` builds one, and `crates/code-record/Cargo.toml`'s
    `[dev-dependencies]` gains `core-operations = { path = "../core-operations" }`;
  - `docs/codebase/README.md` (rows below);
  - the architecture gate (`code-record` may depend on `core-design`, already allowed by B-178).
- **Not:** any language server ask (B-183); MCP, HTTP; Design; Plans; Runs.
- **Needs:** B-178.

**Shapes** (`model.rs`; `Serialize, utoipa::ToSchema`; enums `rename_all = "snake_case"`):

```rust
pub enum Rank { Exact, Path, Part, Words }

/// The answer to one question, record first.
pub struct WhereAnswer {
    pub topic: String,
    pub entries: Vec<WhereEntry>,          // ranked, at most ENTRIES_MAX (20)
    pub entries_more: u32,
    pub not_in_record: Vec<UnrecordedFile>, // B-183; empty here
    pub not_in_record_more: u32,
    pub asked: u32,                        // language-server asks spent (B-183); 0 here
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<String>,            // why the live half stopped early, when it did
}

#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WhereEntry { File(FileEntry), Element(ElementEntry) }

pub struct Owner { pub part_id: PartId, pub part_title: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub layer: Option<String> }

pub struct FileEntry {
    pub rank: Rank,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub owner: Option<Owner>,
    pub tasks: Vec<FileTask>,              // newest first (below)
    pub symbols: Vec<SymbolEntry>,         // by name
    pub live: Live,                        // the file now (B-183 fills it; `not_asked` here)
    #[serde(skip_serializing_if = "Option::is_none")] pub confirmed: Option<Confirmation>,
}
pub struct FileTask { pub workflow_id: WorkflowId, pub version: i64, pub task: u32,
    pub action: String, pub purpose: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub commit: Option<String> }

pub struct SymbolEntry {
    pub symbol_id: String, pub name: String, pub kind: String,
    pub workflow_id: WorkflowId, pub version: i64, pub task: u32,
    pub behavior: Vec<String>,             // from contract_json
    pub at_done: SymbolState,              // the stored state
    pub live: Live,
    #[serde(skip_serializing_if = "Option::is_none")] pub line: Option<u32>,
    pub uses: Vec<UseLine>, pub uses_more: u32,
}
pub struct UseLine { pub path: String, pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")] pub owner: Option<Owner> }

/// What the code says now, for a file or a symbol.
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Live { NotAsked, Confirmed, Gap { why: String }, Unchecked { why: String } }

pub struct ElementEntry { pub rank: Rank, pub element: DesignElement, pub state: ElementState }

pub struct UnrecordedFile { pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub owner: Option<Owner>,
    pub symbols: Vec<UnrecordedSymbol> }  // from workspaceSymbol (B-183)
pub struct UnrecordedSymbol { pub name: String, pub line: u32 }
```

**Error** (`error.rs`): `InvalidTopic(String)` (`#[error("{0}")]`).

**Method** (`code_where.rs`):

```rust
/// Where `topic` is in `project`: the record's entries and elements, ranked, then (B-183) the code.
pub async fn code_where(&self, project: &ProjectId, topic: &str)
    -> Result<WhereAnswer, CodeRecordError>;
```

**Steps of the record's half:**
1. Trim `topic`. Fewer than 1 or more than 200 characters is `InvalidTopic("a topic needs 1 to 200
   characters")`.
2. `self.design.layout(project)`, then `self.design.elements(project)`. A `DesignError` becomes
   `CodeRecordError::Design(#[from] DesignError)`, a new variant mapped by its inner error.
3. The lookup reads (`store/lookup.rs`, each one query bounded by `LIMIT 21`, on the project):
   - `files_by_path(project, topic)`: `code_record_file` rows whose `path = topic` or `path LIKE
     topic || '/%' ESCAPE '\'`, with `%`, `_` and `\` escaped in `topic`;
   - `files_under(project, folders: &[String])`: the same for a part's folder;
   - `symbols_named(project, name)`: `code_record_symbol` rows whose `name = topic`, or whose name
     ends with `'.' || topic` (a method by its bare name);
   - `rows_with_words(project, words)`: file rows whose `lower(purpose)` and symbol rows whose
     `lower(contract_json)` contain every word, one `instr(...) > 0` per word.

   Every read orders by `path, workflow_id, version DESC, task DESC` and is completed per path
   with the file's tasks and symbols by `files_detail(project, paths)` (no limit beyond the paths).
4. `ranking.rs`: `Ranking::entries(topic, layout, elements, rows) -> (Vec<Ranked>, u32)`, pure:

   | Tier | Matches |
   |---|---|
   | `exact` | an element whose id equals the topic ignoring case, or whose name equals it ignoring case; a recorded symbol from `symbols_named` (its file becomes the entry) |
   | `path` | a recorded file from `files_by_path`; an element pinned (a done pin) in such a file |
   | `part` | the topic equals a part title ignoring case: the recorded files under that part's folder (`files_under`) and the part's elements |
   | `words` | the topic split on any character that is not a letter or a digit, lowercased, words of 3 or more characters; at least one word; a file or symbol from `rows_with_words`; an element whose name or purpose holds every word |

   An entry keeps its best tier and appears once. Within a tier: files by path, elements by
   number, a file before an element at equal keys. At most 20, then `entries_more`.
5. Each file entry:
   - `tasks`: every task row of the path, newest first: `version DESC`, then `task DESC`, then
     `workflow_id`. These are explicit keys, never `recorded_at` or rowid.
   - `symbols`: each recorded symbol of the path from its newest task, by name.
   - `owner`: from `layout.owners(path)`. The part is the first owner whose `via` is `Folder`; the
     layer is the first owner whose `via` is `Layer(id)`; `part_title` comes from the layout.
6. Element entries: `self.element_states(project, &matched_elements, &[])`.
7. `live` is `not_asked` on every file and symbol, and `asked: 0`.

**Contract** (`crates/code-record/contract.yaml`):
- dependencies: `design: crates/core-design/contract.yaml # elements, layout and owners`; drop
  "types only";
- shapes: the types above;
- `code_where` with steps 1 to 7;
- `ranking` (private) with the tier table;
- the four lookup reads (private);
- obligations with the tests below;
- modules: `code_where.rs: answering where a topic is`, `ranking.rs: ranking a question's
  matches`, `store/lookup.rs: reading the record for a question`.

The same rows go in `docs/codebase/README.md`.

**Tests:**

Unit tests in `src/ranking.rs`:
- `an_exact_name_ranks_before_a_path_and_a_path_before_words`
- `an_entry_appears_once_at_its_best_tier`
- `ties_go_by_path_then_element_number`
- `short_words_and_punctuation_are_not_words`: `"a b"` gives no words tier; `"user-login"`
  gives `user` and `login`.
- `twenty_entries_then_a_count`

`crates/shadows-core/tests/code_where.rs`. Its fixture is built as
`crates/shadows-core/tests/design_elements.rs` (B-175) builds one: `seed_elements_for_tests`, then
elements put by a Planner grant with a running turn. The record is written with
`CodeRecord::record_done_in` inside the fixture storage's `write_txn`, as
`crates/code-record/tests/record.rs` builds its `DoneRecord`:
- `a_symbol_name_finds_its_file_with_its_tasks_newest_first`
- `a_path_prefix_finds_the_files_under_it_and_escapes_like_wildcards`: a topic `db_x` does not
  match `dbax/...`.
- `a_part_title_finds_the_files_in_its_folder`
- `an_element_id_or_name_finds_the_element_with_its_state`
- `the_owner_is_the_folder_part_and_the_first_layer`
- `a_blank_or_long_topic_is_refused`
- `nothing_matching_answers_no_entries`

**Done when:**
- `cargo test -p code-record --features test-support`;
- `cargo test -p shadows-core --features test-support --test code_where`;
- `cargo test -p shadows-core --features test-support --test runs_attempts` (CodeRecord's new
  constructor);
- the gate list.

---

## Task B-183 · `code_where`, the code's half: confirmed within a budget

**Files:**
- **Modify:**
  - `crates/code-record/src/code_where.rs`;
  - `crates/code-record/src/verify.rs`:
    - extract `ask`'s recorded-answer lookup into `fn recorded(&self, key: &str) ->
      Option<RecordedAnswer>` (test-support) and its `LspAnswer::Answered` translation into
      `fn file_answer(result: &Value, truncated: bool) -> FileAnswer`; `ask` calls both, and its
      behavior is unchanged;
    - `Found` gains `line: Option<u32>` and `character: Option<u32>`, read from the symbol's
      one-based `line` and `character` keys (Codebase's rewrite); none in a raw recorded answer
      without them;
  - `crates/code-record/src/lib.rs` (`RecordedAnswer` gains two variants);
  - `crates/code-record/contract.yaml`;
  - `crates/shadows-core/tests/code_where.rs`.
- **Not:** Codebase; Design; any other crate.
- **Needs:** B-182.

**Recorded answers** (test-support only):
- `RecordedAnswer::References(serde_json::Value)`: a `findReferences` result, recorded under the
  key `"{path}#references@{line}"`.
- `RecordedAnswer::Workspace(serde_json::Value)`: a `workspaceSymbol` result, recorded under
  `"#workspace:{query}"`.

`record_answer_for_tests(key, answer)` keeps its signature (its first parameter is renamed `key`).
The contract says which key each answer takes.

**The live half, after the record's half and in this order.** Constants in `code_where.rs`:
`MAX_ASKS = 30`, `LIVE_FOR = Duration::from_secs(20)`, `USES_MAX = 10`, `UNRECORDED_MAX = 10`. One
deadline (`Instant::now() + LIVE_FOR`) bounds every ask through
`tokio::time::timeout_at(deadline, ...)`.

1. **Files absent now.** `self.code.missing_inputs(Asker::Person(project), &entry_paths)` once, not
   counted. An absent file is `live: gap { why: "{path} is absent now" }` and its symbols
   `gap { why: "its file is absent now" }`; it is asked nothing more. An error makes every file
   `unchecked { why: "the folder could not be read: {error}" }`, and no ask follows.
2. **The topic is not a path** (it has no `/` and does not equal a recorded or inventory path):
   one `WorkspaceSymbol` ask with `query: topic`. Each answered symbol whose `path` is not a
   recorded file of the project (one `files_detail` read) becomes an `UnrecordedFile`, grouped by
   path, by path then line, at most `UNRECORDED_MAX`, then a count.
3. **The topic has a `/`**: `self.code.inventory(Asker::Person(project), InventoryQuery { prefix:
   topic, after: None, limit: 200 })`, not counted. Every `File` entry not recorded becomes an
   `UnrecordedFile` with no symbols, at most `UNRECORDED_MAX`, then a count. An inventory error
   leaves `not_in_record` empty and sets `budget` to `"the file list could not be read:
   {error}"`.
4. **Each file entry in rank order**, while asks remain and the deadline has not passed:
   - **No recorded symbol:** the file exists, so `live: confirmed`, `confirmed: file_only`, and no
     ask.
   - **Otherwise one `DocumentSymbol` ask**, with no wait for a starting server:
     - an `Answered` result is parsed by `document_symbols`; each recorded symbol `matches` one →
       `live: confirmed` with its `line`;
     - in a complete answer with no match → `live: gap { why: "{name} is not in {path} now" }`;
     - in a truncated answer with no match → `unchecked { why: "the server's answer was cut
       short" }`;
     - the file is `confirmed` with `confirmed: file_and_symbol`.
     - `NoServer` → the file and its symbols are `unchecked { why: "no language server reads
       {extension}" }`; when `install` is known, `" (not installed)"` is appended.
     - An error → `unchecked { why: "{error}" }`.
   - **Then, for each confirmed symbol that has a line,** one `FindReferences` ask at its `line`
     and `character`. The result is a list of locations. Drop the symbol's own location. Sort the
     rest by path, then line. List up to `USES_MAX` with their `owner` (the same rule as files);
     the rest is `uses_more`. A failure leaves `uses` empty and adds nothing else.
5. **When asks or time run out,** everything not yet asked is `unchecked { why: "the question's
   budget ran out (30 asks, 20 s)" }`, and `budget` says the same once.
6. `asked` is the number of asks made.

Element entries keep the state `element_states` gave them (B-182). They are not asked again.

**Contract:**
- `code_where`'s live steps, the constants and the keys;
- the extracted helpers (private);
- `Found`'s new fields;
- obligations with the tests below.

The `contract` check's obligations are unchanged.

**Tests** (`crates/shadows-core/tests/code_where.rs`, recorded answers through
`app.code_record().record_answer_for_tests`):
- `a_found_symbol_has_its_line_and_its_uses_grouped_by_part`
- `a_symbol_gone_from_its_file_is_a_live_gap`
- `an_absent_file_is_a_gap_and_is_not_asked`
- `a_file_without_symbols_is_confirmed_file_only_without_an_ask`
- `no_server_makes_the_file_unchecked_and_the_record_still_answers`
- `the_budget_stops_at_thirty_asks_and_says_so`: 25 recorded files of 2 symbols each; `asked ==
  30`, and the later entries are `unchecked` with the budget text.
- `uses_beyond_ten_are_counted`
- `a_word_topic_lists_files_not_in_the_record_from_workspace_symbols`
- `a_path_topic_lists_files_not_in_the_record_from_the_file_list`

The extraction from `ask` adds no test of its own. Slice 1's `contract` tests must pass unchanged:
`cargo test -p shadows-core --features test-support --test runs_attempts contract_check` and
`cargo test -p code-record --features test-support --test verify`.

Real pyright, `#[ignore = "needs pyright-langserver on PATH"]`, in the same file, on a temporary
Python project folder registered with the fixture and a written record:
- `live_code_where_gives_a_functions_line_and_its_caller`
- `live_a_file_outside_the_record_is_listed_as_not_in_the_record`

**Done when:**
- `cargo test -p code-record --features test-support`;
- `cargo test -p shadows-core --features test-support --test code_where`, and the same with
  `-- --include-ignored` here (pyright); the controller records the result in the status line;
- the gate list.

---

## Task B-184 · The `code_where` tool, and the Planner asks first

**Files:**
- **Create:** `crates/shadows-mcp/src/record_tools.rs`.
- **Modify:**
  - `crates/shadows-mcp/src/server.rs` (combine `record_tool_router()`), `src/lib.rs`,
    `src/refusal.rs` (`CodeRecordError`: `InvalidTopic` is `INVALID_COMMAND`, `Design` by its
    inner error, `Storage` as storage), `src/contract.yaml`;
  - `crates/core-grants/src/` (`tools_for`'s Thread list) and `crates/core-grants/contract.yaml`
    (45 becomes 46);
  - `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`;
  - `crates/shadows/tests/mcp_server.rs`, `crates/shadows/tests/mcp_tools.rs`,
    `crates/shadows/tests/element_mcp.rs`;
  - `docs/codebase/README.md` (`record_tools.rs: CodeRecord service MCP translation`).
- **Not:** HTTP; CodeRecord's rules.
- **Needs:** B-183, B-179 (the tool count and the prompt's `project_state` line come first).

**Tool** (Planner only):
- `code_where`:
  - **args:** `{ topic: String }`.
  - **calls:** `self.core.code_record().code_where(&grant.project_id, &args.topic)`.
  - **description:** "Ask where something is in this project before you name a file, a table or
    a screen: a topic (login), a path (backend/auth), a symbol, or an element id or name.
    Shadows answers from its record first: each file with the plans and tasks that made it, its
    symbols and their contracts, the elements and their states. Then the language servers
    confirm: each symbol's line now and who uses it, by part and layer. Within 30 asks and 20
    seconds; what they did not reach is marked unchecked. Files no contract names are listed as
    not in the record."

**Prompt** (`prompt.txt`):
- In the "Shadows' tools" list, after the `project_state` line, exactly:

  ```text
  - code_where: before a contract names a file, a symbol, a table or a screen, ask code_where with a topic, a path or a name. It answers from Shadows' record first and the language servers confirm it. Use its paths, names and element ids as they are; a gap means the code no longer holds what a done task promised; a file not in the record has no contract behind it, so read it before you change it.
  ```

- In the "Look before you write a path" section, a new first line, exactly:

  ```text
  - Ask code_where first. When it answers a file from the record with no gap, use that answer: you need not read the file. Read the files it does not answer for: a file not in the record, a file with a gap or an unchecked symbol, or a project whose record is empty.
  ```

  The next line (today the section's first, "Before a task names a file, ...") becomes "Otherwise,
  before a task names a file, ..." with the rest of its text unchanged. The section's other lines
  stay as they are.

**Tests:**
- `element_mcp::code_where_tool_answers_the_method`: the tool's JSON equals
  `serde_json::to_value` of the method's answer.
- `mcp_server::the_tool_list_depends_on_the_grant_kind`: only the Planner's list has
  `code_where`.
- `mcp_tools::the_planner_is_told_to_ask_code_where_first`: both new prompt lines, and the
  "Otherwise, before a task names a file" wording.
- `code_scope::the_planner_is_told_to_look_at_the_real_files` passes unchanged: it asserts the
  section's title and "Never write a framework's layout from memory", which both stay.

**Done when:**
- `cargo test -p shadows --test element_mcp --test mcp_server --test mcp_tools --test code_scope`;
- the gate list.

---

## Task B-185 · The `code_where` route

**Status at writing:** blocked until `docs/shared/requests/2026-11-19-code-where.md` is accepted.

**Files:**
- **Create:** `crates/shadows-http/src/code_record.rs`;
  `crates/shadows/tests/code_where_routes.rs`.
- **Modify:**
  - `crates/shadows-http/src/lib.rs`, `src/openapi.rs`, `src/contract.yaml`;
  - `crates/shadows-http/src/failure/cases.rs` (only if `CodeRecordError` has no mapping yet:
    `InvalidTopic` is 422 `INVALID_COMMAND`);
  - `api/openapi.json` (regenerated);
  - `docs/codebase/README.md` (`code_record.rs: CodeRecord service HTTP translation`).
- **Needs:** B-183; the request accepted.

**Route:** `GET /api/projects/{id}/code-where?topic=` calls `CodeRecord::code_where` and answers
`WhereAnswer`.
- An unknown or removed project is 404, as the decisions routes answer.
- A blank or long topic is 422 `INVALID_COMMAND`.
- The person asks as `Asker::Person`, which is how `code_where` already asks.

**Tests:**
- `code_where_routes::the_route_answers_the_method`
- `code_where_routes::a_blank_topic_is_422`

Then `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.

**Done when:**
- those tests;
- `cargo test -p shadows --test openapi`;
- the gate list.

---

## Task B-186 · Acceptance on a real provider (2b)

**Needs:** B-184 (the route is not needed for it), and 2a's B-181 run on the same project, so its
record holds the first plan.

The controller runs it with `shadows serve` and a real provider, with the keys as B-181 reads them.
It uses the project B-181 left: its first plan done, its tables and screens built.

1. **New conversation.** Ask the Planner for "users can change their email".
   - **Expected:** before writing a task, it asks `project_state` and `code_where` (for example
     `code_where("users")` and `code_where("Sign in")`). Its contract names the existing files,
     symbols and element ids those answers gave. It modifies, never re-creates, and makes no `Read`,
     `Grep`, `Glob` or shell read of the project's files before its first `plan_edit`.
   - **Record:** the tool calls in order, from the server log and the conversation.
2. **Delete one function** the first plan built, in the folder by hand, and ask
   `code_where(<its name>)` over MCP.
   - **Expected:** the record still answers the entry, with `live: gap`.
   - **Record:** the answer.
3. **Stop the language server** (Settings › Code) and ask again.
   - **Expected:** the record's half comes, with `unchecked` and the reason.
   - **Record:** the answer.

**Evidence:** `docs/evidence/2026-11-2x-code-record-2b-live.md` (the date of the run), with the
commit.

**Done when:** the evidence records the three steps as they happened, failures included.
