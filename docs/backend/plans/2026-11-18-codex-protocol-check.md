# Codex protocol check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Shadows opens any Codex CLI at or above the tested version whose app-server protocol still
matches, in the parts Shadows uses, a tested protocol committed in the repo. Otherwise it refuses and
names what changed.

**Architecture:**
- `shadows-agent`'s codex module owns what Shadows uses of the protocol:
  - the list of used methods and answers;
  - the tested snapshot, `tested_protocol.json`;
  - the comparison;
  - running the installed CLI's `--version` and `app-server generate-json-schema`.
- Harness owns the verdict for one daemon run, kept in memory. It reports the verdict in the harness
  list's `reason` and passes the checked version to every Codex opening.
- `codex::Connection::open` refuses an app-server whose `initialize` reports a version other than the
  checked one. A CLI upgraded while the daemon runs is checked again.

**Tech Stack:** Rust, serde_json, shadows-process, fake-codex.

**Spec:** `docs/core/specs/2026-09-24-harness-controls-design.md`, the paragraph "The baseline is
`codex-cli 0.167.0`…". Its amendment is below. Evidence of the hand check: `docs/backend/stages.md`,
section "Codex CLI 0.169.0" (commit `027b8ebc`).

## Rulings (Mohammed, 2026-11-18, through the General discussion session)

1. **Versions:** a CLI below the tested version is refused without a check: "Codex CLI {tested} or later
   is required". A CLI at or above it opens only if its protocol check passes.
2. **Additions pass.** These pass: a new method, a new definition, a new optional property, and a new
   union variant. A new enum value also passes; the reason is in Review Focus.
3. **Changes refuse:** a removed method; a removed definition, property or union variant; a changed
   type or `$ref`; a removed enum value; a property that became or stopped being required; and a new
   property that is required.
4. **One check per CLI version per daemon run,** kept in memory. Nothing is stored in the database.
5. **Adopting a new version:** install it, then run the ignored test
   `adopt_the_installed_codex_protocol` with `SHADOWS_CODEX_ADOPT=1`, which rewrites
   `tested_protocol.json`. Then run the signed-in live Codex tests and commit the snapshot only if they
   pass. The snapshot's `version` is the floor, kept in one place.
6. **First proof:** the snapshot is adopted from the installed 0.169.0, already known compatible.

## Global Constraints

- No new dependency. Version parsing is ours: a dotted number triple, where a suffix (`-alpha.4`) sorts below the bare triple.
- The tested snapshot is an extract, not the whole schema. Its form is `{ "version": "0.169.0",
  "definitions": { <name>: <definition>, ... } }`, holding the used roots and every definition they
  reach through `$ref`. Keys are sorted, so a diff stays readable.
- The check never writes into the Codex home. Its scratch folder is under Harness's `catalog_root`
  (`codex-protocol/<uuid>`) and is removed after the check.
- The refusal message names at most five changes, then "and N more". Example:
  `Codex CLI 0.171.0 changed what Shadows uses: turn/steer removed; ThreadStartResponse.sandbox
  type changed`.
- The HTTP surface does not change. The list already carries `available` and `reason`, and an opening
  still fails as `HARNESS_START_FAILED`, as the pin does today. No shared request is needed.

## Review Focus

1. **A CLI upgraded while the daemon runs:** the next opening refuses once with "Codex CLI changed to
   X; it is checked again on the next opening", and the opening after that checks X.
   Test: `a_codex_upgraded_while_running_is_checked_again`.
2. **`generate-json-schema` fails, or writes no bundle** (an old or broken CLI): Codex is listed
   unavailable with the CLI's first stderr line, and nothing panics.
   Test: `a_schema_that_cannot_be_generated_is_a_reason`.
3. **Two openings at once on a fresh daemon:** one check runs and both wait on it.
   Test: `concurrent_openings_run_one_check`.
4. **A new enum value passes the check** (Ruling 2), e.g. a new `TurnStatus`. Shadows already fails
   such a turn visibly: `invalid Codex terminal status`. It does not pass silently. No new test; the
   existing mapping holds this.
5. **A deep change in a definition Shadows barely reads** (for example inside `Config`, reached from
   `config/read`) refuses as well. That is accepted: the message names the path, and adoption is one
   test run.

---

### Task B-160: The codex module checks a CLI's protocol against the tested one

Replaces B-160 "Codex CLI 0.169.0 is the pinned baseline" in `docs/backend/stages.md`.

**Files:**
- Create: `crates/shadows-agent/src/codex/protocol.rs`. Job: what Shadows uses of the app-server
  protocol, and comparing a CLI's against it.
- Create: `crates/shadows-agent/src/codex/tested_protocol.json`, adopted from the installed 0.169.0 in
  Step 6.
- Modify: `crates/shadows-agent/src/codex.rs`:
  - `VERSION` goes;
  - add `mod protocol`;
  - add `CodexProgram::check`;
  - `Connection::open` takes the checked version.
- Modify: `crates/shadows-agent/src/connection.rs`: `ManagedConnection::open`'s `codex: bool` becomes
  `codex: Option<&str>`, the checked version.
- Modify: `crates/shadows-agent/tests/codex.rs`, line 50, to pass the version.
- Modify: `crates/fake-codex/src/main.rs`:
  - `--version` prints `codex-cli {FAKE_CODEX_VERSION or 0.169.0}`;
  - `app-server generate-json-schema --out <dir>` writes `codex_app_server_protocol.schemas.json`;
  - `initialize` answers `codex/{same version}`.
- Modify: `crates/shadows-agent/contract.yaml`: the `codex::Connection::open` line, plus new lines for
  `CodexProgram::check` and the `protocol` module.

**Interfaces:**
- Produces, in `shadows_agent::codex`:
  - `pub struct CheckedCodex { pub version: String }`
  - `impl CodexProgram { pub async fn check(&self, scratch: &Path) -> Result<CheckedCodex, ConnectionError> }`.
    It runs `--version` and then `app-server generate-json-schema --out <scratch>` through
    shadows-process, with `catalog_spec`'s environment and no arguments that carry a grant. It reads
    `<scratch>/codex_app_server_protocol.schemas.json`. A refusal is
    `ConnectionError::Rpc(<the message>)`.
  - `pub async fn Connection::open(handle, events, version: &str) -> Result<Self, ConnectionError>`.
    A `userAgent` with another version answers `Rpc("Codex CLI changed to {found}; it is checked again
    on the next opening")`.
- Private to `protocol.rs`:
  - `const USED: Used`, the roots, and nothing else:
    - Client requests: `initialize`, `thread/start`, `thread/resume`, `thread/fork`, `turn/start`,
      `turn/steer`, `turn/interrupt`, `model/list`, `account/read`, `account/rateLimits/read`,
      `config/read`, `mcpServerStatus/list`.
    - Their answers: `InitializeResponse`, `ThreadStartResponse`, `ThreadResumeResponse`,
      `ThreadForkResponse`, `TurnStartResponse`, `ModelListResponse`, `GetAccountResponse`,
      `GetAccountRateLimitsResponse`, `ConfigReadResponse`, `ListMcpServerStatusResponse`.
    - Server requests and their answers: `item/commandExecution/requestApproval`,
      `item/fileChange/requestApproval`, `item/permissions/requestApproval`.
    - Notifications: `turn/completed`, `item/started`, `item/completed`, `item/agentMessage/delta`,
      `thread/tokenUsage/updated`, `account/rateLimits/updated`.
    - The builder confirms each answer's definition name against the 0.169.0 bundle, where all of them
      live under `definitions` and `definitions.v2`, and records the names in the module's doc comment.
  - `fn extract(bundle: &Value, version: &str) -> Result<Value, String>`. It takes each method's
    variant from `ClientRequest`, `ServerRequest` or `ServerNotification` (matched by its `method`
    enum), each named answer, and every `$ref` they reach, and returns the snapshot form.
  - `fn compare(tested: &Value, found: &Value) -> Vec<String>`, implementing Rulings 2 and 3. Each
    change reads `<Definition>.<property path> <what>`, or `<method> removed`. Union variants are matched
    by their `type` or `method` enum value, otherwise by position.
  - `fn version_at_least(found: &str, floor: &str) -> bool`.

Order: the snapshot must exist before the tests and fake-codex can read it. So do Step 5 and Step 6
first: write `USED`, `extract` and the adoption test, and adopt 0.169.0. Then do Steps 1 to 4, 7 and 8.

- [ ] **Step 1: Write the failing unit tests** in `protocol.rs` `#[cfg(test)] mod tests`. Each test
  builds `found` by changing a copy of the committed snapshot:
  - `the_tested_protocol_matches_itself`: `compare(t, t)` is empty.
  - `an_added_optional_property_passes`: a property `extra` without `required` added to
    `ThreadStartResponse` → empty.
  - `an_added_method_passes`: a new `ClientRequest` variant `modelProvider/list` → empty.
  - `an_added_enum_value_passes`: `TurnStatus` gains `paused` → empty.
  - `a_removed_method_refuses_and_names_it`: drop `turn/steer` → `["turn/steer removed"]`.
  - `a_removed_property_refuses_and_names_it`: drop `approvalsReviewer` from `ThreadStartResponse` →
    one change containing `ThreadStartResponse.approvalsReviewer`.
  - `a_changed_type_refuses`: `ModelListResponse.data` becomes `"type": "object"` → one change
    containing `ModelListResponse.data`.
  - `a_newly_required_parameter_refuses`: `TurnSteerParams` gains a required `mode` → one change
    containing `TurnSteerParams.mode`.
  - `a_removed_union_variant_refuses`: the `ThreadItem` variant `agentMessage` goes → one change
    containing `ThreadItem` and `agentMessage`.
  - `versions_compare_by_number`: `version_at_least` holds for `0.170.0` and `0.169.0` against floor
    `0.169.0`, and fails for `0.168.9` and `0.169.0-alpha.1`.
  - `the_snapshot_holds_every_used_method`: every root in `USED` is in `tested_protocol.json`.
  - `the_refusal_names_five_changes_then_counts`: seven changes → the message lists five, then "and 2
    more".

- [ ] **Step 2: Run them.**
  `cargo test -p shadows-agent --lib codex::protocol`. Expected: FAIL to compile, because `protocol`
  does not exist yet.

- [ ] **Step 3: Implement** `protocol.rs` (`USED`, `extract`, `compare`, `version_at_least`, the
  message) and `CodexProgram::check`, which in order:
  1. reads the version;
  2. refuses below the snapshot's `version`;
  3. generates the bundle;
  4. extracts it;
  5. compares it;
  6. removes the scratch folder.

  Then the `open(…, version)` refusal and `ManagedConnection::open(…, codex: Option<&str>, …)`.

- [ ] **Step 4: Extend fake-codex.** It gets `--version`, `generate-json-schema` and the `initialize`
  version. With `FAKE_CODEX_PROTOCOL` it writes the committed snapshot in bundle form:
  - `same` (the default) writes it unchanged;
  - `added` adds an optional property and a method;
  - `removed` drops `turn/steer`;
  - `broken` exits 1 with `error: unknown subcommand` on stderr and writes nothing.

  The snapshot is read with `include_str!("../../shadows-agent/src/codex/tested_protocol.json")`.
  `FAKE_CODEX_VERSION_FILE`, when set, is read on each start, so a test can change the version while a
  daemon runs.

- [ ] **Step 5: Write the adoption test** in `crates/shadows-agent/tests/codex.rs`:
  `#[ignore] adopt_the_installed_codex_protocol`. It needs `SHADOWS_CODEX_ADOPT=1` and
  `SHADOWS_CODEX_PATH`. It runs the real CLI's `--version` and generator, extracts the bundle, and
  writes `src/codex/tested_protocol.json` with sorted keys. Without the variable it returns at once.

- [ ] **Step 6: Adopt 0.169.0.**
  1. Run `SHADOWS_CODEX_ADOPT=1 SHADOWS_CODEX_PATH=<installed codex.exe> cargo test -p shadows-agent
     --test codex adopt_the_installed_codex_protocol -- --ignored`.
  2. Check that the file's `version` is `0.169.0`.

- [ ] **Step 7: Run.**
  - `cargo test -p shadows-agent --lib codex::protocol`: all 12 pass.
  - `cargo test -p shadows-agent --test codex`: passes.

- [ ] **Step 8: Commit.** Scoped to the files above: `feat(agent): B-160 check Codex's protocol against
  the tested 0.169.0`.

### Task B-161: Harness opens Codex on the checked version and lists the verdict

**Files:**
- Create: `crates/core-harness/src/codex_check.rs`. Job: the Codex protocol verdict for one daemon run.
- Modify:
  - `crates/core-harness/src/assistant.rs`: `harness_available` and `unavailable_reason` for Codex read
    the verdict;
  - `crates/core-harness/src/lib.rs`: `list` passes the verdict's message as `reason`;
  - every Codex opening: `crates/core-harness/src/sessions.rs`, `crates/core-harness/src/catalog.rs`,
    and the executor opening. Each awaits the check and passes `Some(&checked.version)` to
    `ManagedConnection::open`.
- Modify: `crates/core-harness/contract.yaml`, which gets a Codex check paragraph and an updated list
  `reason`.
- Modify: `docs/core/specs/2026-09-24-harness-controls-design.md`, applying the amendment below.
- Test: `crates/shadows-core/tests/codex_session.rs`, the new tests below.

**Interfaces:**
- Consumes: `CodexProgram::check`, `CheckedCodex`, `ManagedConnection::open(…, Option<&str>, …)` from
  B-160.
- Produces:
  - `pub(crate) async fn Sessions::codex_checked(&self) -> Result<CheckedCodex, HarnessError>`.
    - The verdict is held in a `tokio::sync::Mutex<Option<Result<CheckedCodex, String>>>`, so
      concurrent callers wait on one check.
    - A refusal is `HarnessStartFailed(<message>)`; Codex not configured stays `HarnessUnavailable`.
    - When an opening's `open` reports a changed version, the cached verdict is cleared.
  - `#[cfg(feature = "test-support")] pub fn Harness::codex_checks_for_test(&self) -> usize`.

- [ ] **Step 1: Write the failing tests** in `codex_session.rs` (fake-codex):
  - `a_matching_newer_codex_opens`: `FAKE_CODEX_VERSION=0.170.0` and protocol `added` → `open_session`
    succeeds; the list shows codex `available: true`, `reason: None`.
  - `a_changed_codex_is_listed_with_what_changed`: protocol `removed` → the list shows codex
    `available: false`, with `reason` containing `turn/steer removed`. `open_session` and
    `catalog("codex")` refuse with `HarnessStartFailed` carrying the same text.
  - `an_older_codex_is_refused_without_a_check`: `FAKE_CODEX_VERSION=0.168.0` → `reason` is
    `Codex CLI 0.169.0 or later is required`.
  - `a_schema_that_cannot_be_generated_is_a_reason`: protocol `broken` → `available: false`, with
    `reason` containing `unknown subcommand`.
  - `the_protocol_is_checked_once_per_run`: list, open and catalog run `codex_checks_for_test() == 1`.
  - `concurrent_openings_run_one_check`: two `open_session` calls on two threads, joined → both succeed,
    and checks `== 1`.
  - `a_codex_upgraded_while_running_is_checked_again`:
    1. Open on 0.169.0.
    2. Close the thread.
    3. Write 0.170.0 to `FAKE_CODEX_VERSION_FILE`.
    4. The next `open_session` refuses with `changed to 0.170.0`.
    5. The one after succeeds, and checks `== 2`.

- [ ] **Step 2: Run them.**
  `cargo test -p shadows-core --test codex_session`. Expected: the new tests FAIL. The existing ones
  compile against B-160's signatures.

- [ ] **Step 3: Implement.** `codex_check.rs`; the list's `available` and `reason`; the openings pass the
  checked version; the cache is cleared on a changed version.

- [ ] **Step 4: Run the fake suites.**
  `cargo test -p shadows-core --test codex_session`, `--test harness_catalog`, `--test executor_session`
  and `--test harness_choices` (`-p shadows`). All pass.

- [ ] **Step 5: Run the first proof, live, on the installed 0.169.0** (signed in):
  - `codex_session` `live_a_codex_planner_reads_a_file_and_writes_none`;
  - `codex_session` `live_a_codex_turn_sees_an_image`;
  - `harness_catalog` `live_the_codex_catalog_lists_each_models_efforts`.

  Each must pass. Their results go in the commit message.

- [ ] **Step 6: Amend the spec** as below. Update the contract.

- [ ] **Step 7: Commit.** Scoped: `feat(harness): B-161 open Codex on its checked protocol; 0.169.0
  proven live`.

## Spec amendment

In `docs/core/specs/2026-09-24-harness-controls-design.md`, replace the paragraph that begins "The
baseline is `codex-cli 0.167.0`…" with:

> **Codex versions.**
> - The tested protocol is `crates/shadows-agent/src/codex/tested_protocol.json`. It is the extract of
>   the app-server schema that Shadows uses: the methods it calls, the requests it answers, the
>   notifications it reads, and every definition they reach.
> - Its `version` is the floor. A CLI below it is refused.
> - A CLI at or above it is checked once per daemon run, with `codex app-server generate-json-schema`.
>   Additions pass. A removed or changed part of the extract refuses, and the harness list's reason
>   names each change.
> - A new version is adopted by running `adopt_the_installed_codex_protocol` and then the signed-in
>   live Codex tests. The new snapshot is committed only if those tests pass.
> - The first snapshot is 0.169.0. The 0.167.0 baseline's evidence stays in
>   `docs/evidence/2026-11-07-codex-app-server-probe.md`.
