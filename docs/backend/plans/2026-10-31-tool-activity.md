# Tool activity: the build

The rules are the spec's: [tool activity](../specs/2026-10-31-tool-activity-design.md).
This plan says where each goes and in what order. Owners, from `AppCore`
(`crates/shadows-core/src/app.rs`): `Turns` (turn execution lifecycle) builds and writes
the cards and holds the live state; `Threads` owns the entry kind; `Events` owns what a
subscriber gets. `shadows-agent` (outside `AppCore`, the ACP client library) only
translates. No new service, no new route.

Every task: read `docs/backend/AGENTS.md` first; find symbols with the LSP; run the task's
tests and AGENTS.md rule 5's checks; commit with `git add` per file. A task whose work needs
a file outside its `Touch:` stops and reports.

## B-19 · ACP facts reach `HarnessEvent`

Translation only; no rule about what a card shows.

- New `crates/shadows-agent/src/events/tool.rs`, re-exported from `events.rs`:
  - `ToolFacts { kind: Option<CallKind>, locations: Option<Vec<ToolLocation>>, input:
    Option<Value>, diffs: Option<Vec<ToolDiff>> }` (`Default`; not serialized).
  - `ToolLocation { path: String, line: Option<u32> }`,
    `ToolDiff { path: String, old_text: Option<String>, new_text: String }`.
  - The wire types, each `Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema`,
    enums `rename_all = "snake_case"`:
    `CallKind { Read, Edit, Delete, Move, Search, Execute, Think, Fetch, SwitchMode, Other }`,
    `CallStatus { Pending, InProgress, Completed, Failed, Stopped }`,
    `DiffOp { Context, Add, Remove }`, `DiffLine { op: DiffOp, text: String }`,
    `DiffHunk { path: String, old_start: Option<u32>, new_start: Option<u32>, lines: Vec<DiffLine> }`,
    `ToolCallCard { id, tool: Option<String>, kind: CallKind, status: CallStatus,
    target: Option<String>, lines: Option<[u32; 2]>, added: Option<u32>, removed: Option<u32>,
    diff: Option<Vec<DiffHunk>>, started_at: String, finished_at: String }`,
    `ToolLive { id, tool: Option<String>, kind: CallKind, title: String,
    target: Option<String>, lines: Option<[u32; 2]>, status: CallStatus, started_at: String }`,
    `TodoStatus { Pending, InProgress, Completed }`,
    `TodoItem { content: String, status: TodoStatus }`, `TodoListCard { entries: Vec<TodoItem> }`.
- `HarnessEvent::ToolCall` gains `facts: ToolFacts`. New variants:
  `Tool(ToolLive)` (sent by the Planner's watcher, as `Subagent` is) and
  `Todos(Vec<TodoItem>)`.
- `acp/mapping.rs`: `ToolCall` fills every `facts` field (`kind` from ACP's `ToolKind`,
  `Other` for an unknown one; `diffs` from the `Diff` items of `content`, other items
  skipped; paths with `Path::display`). `ToolCallUpdate` fills each from its `Option`.
  `SessionUpdate::Plan(p)` becomes `Todos`, `priority` dropped.
- `core-turns` only compiles on: `entries.rs` adds `..` to its `ToolCall` pattern and puts
  `Tool(_)` and `Todos(_)` in its ignored arm; `tests/entries.rs` builds `facts: ToolFacts::default()`.
- Code map row: `crates/shadows-agent/src/events/tool.rs` | `tool activity reports` |
  `crates/shadows-agent/src/events.rs`.
- Tests in `crates/shadows-agent/src/acp_tests.rs`, through `mapping::Mapping::forward`:
  `a_tool_call_carries_its_kind_locations_input_and_diffs` (an `Edit` `tool_call` with a
  location, `rawInput` and one diff), `an_update_carries_only_what_it_names` (a title-only
  update has every `facts` field `None`; one with `content` and `locations` has both),
  `a_plan_update_becomes_the_task_list` (three entries, statuses kept, in order).

## B-20 · A `ToolCall` entry carries its card

- Dependency `similar` (infrastructure: the line diff), in the root `[workspace.dependencies]`
  and `core-turns/Cargo.toml`; the commit says so.
- New `crates/core-turns/src/hunks.rs` (job: `edit hunk placement`): private
  `struct Hunks` with `fn from_diffs(diffs: &[ToolDiff], locations: &[ToolLocation]) ->
  Placed`, where `Placed { hunks: Vec<DiffHunk>, added: u32, removed: u32 }`; the
  400-line cap is applied by the caller. Line diff with `similar::TextDiff::from_lines`;
  numbering per spec §2.
- New `crates/core-turns/src/tools.rs` (job: `tool card collection`), the counterpart of
  `subagents.rs`: `pub(crate) struct Tools { root: Option<PathBuf>, open: Vec<OpenCall>,
  ended: Vec<OpenCall>, written: HashSet<String> }`. Methods:
  `take(&mut self, id, title, status, tool, facts: &ToolFacts, now: &str)` (an id in
  `written` is ignored), `ended(&mut self) -> Vec<(String, ToolCallCard)>` (title and card of
  each held call, in end order, clearing them into `written`), `stop_all(&mut self, now) ->
  Vec<(String, ToolCallCard)>` (held, then open as `Stopped`). Target, lines, counts and cap
  per spec §2.
- `entries.rs`: `Durable::Tool { title: String, card: Box<ToolCallCard> }`. `Collector` gets
  `tools: Tools` in place of `tools: Vec<(String, String)>` and is built by
  `Collector::new(root: Option<PathBuf>)`. A `ToolCall` that is not a subagent's goes to
  `tools.take`; `tools.ended()` is written before any other `Durable` in `push` and `cut`;
  `finish` writes `stop_all`. The clock is `core_db::Storage::now()`.
- `PlannerTurnRequest` gains `root: Option<PathBuf>`; `Turns::send` sets it from
  `context.project_directory`; `TurnWatch` carries it to `Collector::new`. Test constructors
  set `root: None`.
- `turn.rs` `persist`: `Durable::Tool` writes kind `ToolCall`, body the title, `card` the
  serialized card.
- `shadows-core/src/lib.rs` re-exports `ToolCallCard, DiffHunk, DiffLine, DiffOp, CallKind,
  CallStatus`; `shadows-http/src/openapi.rs` lists them in `components(schemas(...))`.
  `core-threads/src/model.rs`: `ToolCall`'s doc says it carries a `ToolCallCard`;
  `ThreadEntry.card`'s doc names `ToolCallCard` for `ToolCall`. Regenerate `api/openapi.json`.
- Contracts: `core-turns` (modules `tools.rs`, `hunks.rs`; the §23.8 obligation rewritten to
  the held-call rule; the new tests), `core-threads` (its "A tool line ... has no card"
  obligation). Code map rows for `tools.rs` and `hunks.rs`.
- `fake-acp` prompt `tool-activity`: chunk `m1`; an `Edit` `tool_call` (`kind: edit`,
  location without line, a snippet diff) → `completed` → an update with two placed diff
  items and their located lines; a `Read` with `offset: 10, limit: 5` → `completed`; an
  `execute` call → `failed`; chunk `m2`; an `execute` call left `in_progress`.
- Tests: in `crates/core-turns/tests/entries.rs`:
  `a_tool_entry_carries_its_card_with_target_lines_and_status`,
  `a_placed_diff_after_the_end_replaces_the_snippet_before_the_next_text`,
  `a_diff_over_400_lines_keeps_its_counts_and_drops_its_hunks`,
  `a_new_file_is_one_hunk_of_added_lines_without_numbers`,
  `a_failed_call_is_failed_and_an_open_one_is_stopped`,
  `a_target_under_the_root_is_relative`, `an_update_for_a_written_call_writes_nothing`;
  the existing tests kept, rewritten to the new `Durable::Tool`. In
  `crates/shadows/tests/planner_turn.rs`: `tool_entries_carry_their_cards` (the `tool-activity`
  prompt over HTTP: entry order, both edit hunks placed, counts, `lines`, `failed`, `stopped`).

## B-21 · The task list is kept, and a running turn's activity is readable

- `core-threads/src/model.rs`: `ThreadEntryKind::TodoList` (doc, `as_str`, `parse`);
  `ThreadEntry.card`'s doc names `TodoListCard`. `core-threads/contract.yaml` lists it.
- `entries.rs`: `Collector` keeps the last `Todos`; `Durable::TodoList(Vec<TodoItem>)`
  last in `finish` when the list is not empty. `changed_tools(&mut self) -> Vec<ToolLive>`:
  each call whose live view changed since the last call (from `Tools`).
  `live(&self) -> (Vec<ToolLive>, Option<Vec<TodoItem>>)`: open calls in start order, and the list.
- `turn.rs`: `persist` writes `TodoList` (author planner, body per spec §3, card
  `TodoListCard`). `accept` publishes each `changed_tools()` item as `HarnessEvent::Tool`,
  after `changed_cards`, then stores `collector.live()` in the turn's activity.
- Live state, Turns': `model.rs` `pub struct LiveActivity { op: OperationId, tools:
  Vec<ToolLive>, todos: Option<Vec<TodoItem>> }`. `handles.rs`: `LiveTurn.activity:
  Arc<std::sync::Mutex<(Vec<ToolLive>, Option<Vec<TodoItem>>)>>` and
  `pub(crate) async fn activity_for(&self, thread: &ThreadId) -> Option<LiveActivity>`.
  `spawn.rs` creates it for `LiveTurn` and `TurnWatch`. `lib.rs`: `pub async fn
  live_activity(&self, thread: &ThreadId) -> Option<LiveActivity>`; `LiveActivity`
  re-exported.
- `core-turns/contract.yaml`: `live_activity` under `service` and `called_by_other_services`
  (Events), `LiveActivity` shape, the task-list obligation and tests.
- `shadows-core/src/lib.rs` re-exports `TodoListCard, TodoItem, TodoStatus, ToolLive`;
  `shadows-http/src/openapi.rs` adds them to `components(schemas(...))` (`ToolLive` is the
  `tool` frame's body, named so the web's generated types have it).
- `fake-acp` prompt `todo-list`: two `plan` updates (the second marks one item completed),
  chunk, end.
- Regenerate `api/openapi.json`.
- Tests: `crates/core-turns/tests/entries.rs`:
  `the_last_plan_update_becomes_one_todo_list_entry_at_the_end`,
  `a_turn_without_a_plan_writes_no_todo_list`,
  `changed_tools_reports_a_call_once_per_change`;
  `crates/shadows/tests/planner_turn.rs`: `a_turns_task_list_is_kept_as_a_todo_list_entry`;
  `crates/shadows/tests/thread_contract.rs`: `TodoList` added to
  `every_entry_kind_round_trips_as_its_stored_name`.

## B-22 · A subscriber gets `tool` and `todos`, and a reload gets them back

- `core-events/src/subscription.rs`: `Delivery::Tool { op, call: ToolLive }` and
  `Delivery::Todos { op, entries: Vec<TodoItem> }`; `transient` maps `HarnessEvent::Tool`
  and `HarnessEvent::Todos`. A thread subscription holds a `Turns` clone; on its first
  `CaughtUp` it reads `Turns::live_activity` and queues one `Tool` per open call, then one
  `Todos` if a list exists, delivered before the commands list.
- `core-events/src/lib.rs`: `subscribe` passes the `Turns` clone; `subscribe_project` passes
  none.
- `shadows-http/src/sse.rs`: frames `tool` (`{op, ...ToolLive}` flattened) and `todos`
  (`{op, entries}`); both in `STREAM_DESCRIPTION`. Regenerate `api/openapi.json`.
- `core-events/contract.yaml`: `Delivery`, `transient`, `next`, the re-send rule and tests.
- `fake-acp` prompt `activity-hold`: an `execute` `tool_call` `in_progress`, one `plan`
  update, then waits for cancel as `hang` does.
- Tests in `crates/shadows/tests/stream_frames.rs`:
  `tool_and_todos_frames_reach_a_running_turns_stream` (`tool-activity` and `todo-list`:
  a `tool` frame per change, with `failed`; a `todos` frame per update),
  `a_stream_opened_mid_turn_gets_its_open_calls_and_list_after_caught_up` (`activity-hold`,
  subscribe after the frames went out: after `caught-up`, one `tool` `in_progress` and one
  `todos`; then Stop: the entry is `stopped`).
