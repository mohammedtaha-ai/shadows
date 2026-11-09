# Codex App-server Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task by task.
> Steps use checkbox syntax. Keep this branch and the existing workspace cache.

**Goal:** Make Codex usable for project Planner conversations through its direct
app-server interface, including Shadows tools, enforced permissions and persistent sessions.

**Architecture:** AppCore wires the configured programs. Core Harness owns the session
and grant lifecycle; Shadows Agent translates ACP and Codex into shared choices/events.
Process creation remains in Shadows Process. The web client uses the existing Core API.

**Tech Stack:** Rust 1.94+, Tokio, serde/serde_json, installed Codex CLI 0.167.0,
React/TypeScript, the existing HTTP MCP server. No new product transport framework.

**Spec:** [Harness controls §12.14](../../core/specs/2026-09-24-harness-controls-design.md#1214-codex-over-app-server).

## Global constraints

- Base: `codex/codex-app-server`, forked from `codex/acp-upgrade`; plan inspection at `41dd19ef`.
- No conversation transfer, AYLUL Codex profile, vendor patch, global Codex config change,
  second build cache, `cargo clean`, release build or changed `RUSTFLAGS`.
- Installed CLI 0.167.0 is the measured baseline. Use its generated schemas, not a guessed
  schema copied from online examples. The earlier standalone checks are evidence, not completion.
- Preserve command replay, ownership boundaries, event ordering, live-generation fencing
  and confirmed cancellation. Update contracts/maps only with the implementation they describe.
- Grant values stay in child environment; never arguments, thread config, settings files,
  prompts, database records, logs or Codex rollouts. Use existing sign-in without copying credentials.
- Work task by task with focused checks. The full workspace gate belongs to CI per Mohammed's
  instruction; do not restart repeated local full-workspace compilation.

## Review focus

1. An inherited MCP/plugin/hook exposes extra tools or authority: B-85 must prove the inventory and refusal.
2. A rotated/revoked grant is reused after resume: B-86 tests the fresh grant and denies the old one.
3. Stop races a final delta/tool result or a restarted process: B-86 tests one terminal result and generation fencing.
4. A listed model is inaccessible or has different efforts: B-87 keeps the error and per-model effort rules.
5. Replayed history/usage produces duplicate text or inflated context: B-87 and F-97 pin normalization.

## Delivery order

| Task | Deliverable | Starts when |
|---|---|---|
| B-85 | Real MCP and permission proof, exact launch policy | Ready now; only this task is initially executable |
| B-86 | First Core-to-Codex Planner path with lifecycle and tools | B-85 closes the spec's permission OPEN block |
| B-87 | Models, efforts, usage and existing conversation actions | B-86 passes |
| F-97 | Existing web flow uses the enabled Codex harness correctly | B-87 and the shared agreement are satisfied |
| B-88 | Real daemon/browser acceptance and delivery record | F-97 passes |

Keep task dependencies in stages. Do not dispatch a blocked task as though its evidence exists.

## B-85: Prove tools and permissions first

**Execution status, 2026-11-08:** verified on Windows with native `0.167.0`.
The [signed-in isolation probe](../../evidence/2026-11-07-codex-app-server-probe.md#signed-in-shadows-home-on-2026-11-08)
used Mohammed's ChatGPT account in a dedicated Shadows Codex home. It excluded
the regular profile and untrusted project layers, including a project MCP
server added after preflight. The same launch passed the Planner draft,
approval, filesystem, Stop, resume and bearer checks. B-86 can start with
the policy fixed in §12.14.

**Files:** Modify this plan, the owner spec §12.14, `docs/backend/stages.md`, and
`docs/evidence/2026-11-07-codex-app-server-probe.md`. Throwaway probe sources live in a
temporary directory and are deleted after evidence is recorded. No product source changes.

**Interfaces:** Consumes the installed `codex app-server`, its generated schema, and a
disposable Shadows project through the existing HTTP/MCP surface. Produces a measured
Codex launch configuration, a complete tool/mode mapping, and sanitized acceptance evidence.
Use AppCore's existing owners/routes; do not access a store or the SQLite file directly.

- [x] Prepare a disposable project and a genuine scoped grant through the existing public
  Shadows surface. Use child-environment injection; do not print/read the token into model context.
  Keep the command IDs and issue/revoke results so uncertain writes are not blindly replayed.
- [x] Connect 0.167.0 with the Shadows HTTP MCP server marked required and its bearer supplied
  by `bearer_token_env_var`. Inspect effective tools, including inherited MCPs/plugins/hooks.
  Prove the configuration excludes everything outside the current Planner policy without
  changing the person's global config. A required server outage must fail setup.
- [x] Run a bounded real-provider round trip: read project facts, create one planning draft,
  then read it back through Shadows. Use a runtime-random marker so the response proves the
  tool result was consumed. Verify the saved draft through the public API.
- [x] Exercise forbidden authority separately: file writes outside the selected policy,
  a planning write with a revoked grant, and person-only approval. Assert no forbidden state
  change; a refusal must be visible. Do not accept "the model declined" as enforcement proof.
- [x] Cancel while a controllable MCP call is pending; resume with a fresh process and fresh
  grant. Confirm terminal cancellation, old-grant refusal and successful authorized continuation.
- [x] Record exact mode IDs, default, config keys and effective behavior in §12.14, plus the
  evidence commit. Retain `read-only`/`never` for the initial proof. If an equivalent supported
  policy cannot be enforced, report the measured gap and leave B-86 blocked.

**Verification:** All asserted tool writes/refusals are independently read back over the API;
the marker round trip, required-server failure, Stop, resume and secret-absence scan pass.
Scan artifacts locally by comparing the runtime token without printing it. No Cargo gate for
this throwaway proof. Commit only the planning/evidence documents.

## B-86: First real Planner path

**Verification ruling, 2026-11-08:** Mohammed chose the smallest product path
with real daemon and provider checks and asked to retain the fixtures already
written. Validate the actual signed-in Codex executable against the running
Shadows daemon, with project-grant and Planner-turn state read back through
the existing public routes.

**Files to create:**
`crates/shadows-agent/src/connection.rs`, `crates/shadows-agent/src/codex.rs`,
`crates/shadows-agent/src/codex/connection.rs`, `crates/shadows-agent/src/codex/session.rs`,
`crates/shadows-agent/src/codex/prompt.rs`, `crates/shadows-agent/src/codex/mapping.rs`,
`crates/shadows-agent/tests/codex.rs`, `crates/shadows-core/tests/codex_session.rs`,
`crates/fake-codex/Cargo.toml`, `crates/fake-codex/src/main.rs`,
`crates/fake-codex/contract.yaml`.

**Files to modify:**
`crates/shadows-agent/src/lib.rs`, `src/acp.rs`, `src/acp/connection.rs`, `src/acp/session.rs`,
`src/acp/prompt.rs`, `src/acp/mapping.rs`, `src/events.rs`, `src/choices.rs`, `src/policy.rs`,
`contract.yaml` (all under shadows-agent);
`crates/core-harness/src/lib.rs`, `src/access.rs`, `src/assistant.rs`, `src/live.rs`,
`src/sessions.rs`, `src/setup.rs`, `src/settings.rs`, `src/offers.rs`, `src/error.rs`, `contract.yaml`
(all under core-harness);
`crates/core-turns/src/turn.rs`, `src/steer.rs`, `src/prompt.rs`, `contract.yaml`
(all under core-turns);
`crates/shadows/src/config.rs`, `src/config_tests.rs`, `src/cli/args.rs`, `src/cli/mod.rs`,
`crates/shadows-core/src/app.rs`, `src/testing/mod.rs`,
`crates/shadows-core/tests/design_migration.rs`, `crates/shadows-http/src/aylul_env_tests.rs`,
`crates/shadows-core/tests/acp_connection.rs`, `tests/aylul_session/permissions.rs` (ACP regression callers),
`crates/shadows/tests/tool_rules_routes.rs`, `crates/shadows/tests/harness_config.rs`,
`crates/shadows-core/Cargo.toml`, `Cargo.lock`, `docs/codebase/README.md`, this plan/stages/spec.
The StartConfig-literal callers above were found through the Rust LSP. Before dispatch,
expand any additional constructor/test callers introduced since this plan; do not guess paths.

**Interfaces:**
- Add `Config::codex_path: Option<PathBuf>` and matching optional absolute `--codex` input.
  Pair it with an explicit absolute `--codex-home` for the dedicated signed-in home.
  Pass both paths through `StartConfig`. AppCore builds `codex::CodexProgram` once and puts
  `Option<Arc<CodexProgram>>` in `SessionsConfig::codex`.
- `CodexProgram::new(executable: PathBuf, home: PathBuf) -> Self` holds the validated paths.
  `CodexProgram::process_spec(&self, cwd: &Path, setup: &SessionSetup)` returns
  `Result<ProcessSpec, ConnectionError>` with B-85's verified launch/configuration and
  in-memory grant environment. It does not spawn or own the child.
- Add `connection::ManagedConnection`, a cloneable finite ACP/Codex enum. Its operations
  retain the current connection signatures for session start, prompt blocks, cancel, options,
  capabilities, permissions, steering and task stop; errors use `ConnectionError`, not `AcpError`.
- Move shared session/prompt values to the neutral boundary, retaining ACP re-exports for
  existing low-level tests. `Opened.options`, `set_option` results and `HarnessEvent::Options`
  carry `choices::Offered`. Parse ACP options only at the ACP boundary. `Offers::record`
  consumes `&Offered`; the frontend's serialized choices shape is unchanged.
- The Codex branch keeps its active native thread/turn IDs and normalizes matching terminal
  events to the existing `PromptResult`. The configured program supplies a `ProcessSpec`
  using B-85's verified policy. ProcessHandle is still owned by Sessions.

- [x] Fixture tests `codex_streams_before_completing`,
  `codex_interrupt_waits_for_the_matching_terminal_event`,
  `codex_eof_fails_pending_work`, `codex_unknown_notifications_are_ignored`,
  `codex_server_requests_do_not_deadlock`, and
  `codex_cannot_finish_a_replacement_generation` cover partial/multiple JSONL frames,
  ordered deltas, RPC error vs EOF, and exactly one ending. `fake-codex` is a gated test
  binary, never a shipping dependency; its fixtures carry no secret.
- [x] Implement the version-matched JSONL transport with existing Tokio/serde dependencies
  and the smallest typed request/result subset needed.
- [x] Wire the second connection through Harness and AppCore with no new composition root.
  Prepare/revoke the authenticated setup across every spawn/handshake/session failure. Keep
  the existing idle/cancel/setup durations and generation/lease rules.
- [x] Core tests `codex_uses_shadows_mcp`, `codex_resume_rotates_the_grant`,
  `codex_failed_setup_revokes_the_grant`, `codex_refuses_forbidden_authority`, and
  `codex_stop_during_a_tool_call` run through AppCore and real MCP routing with the fake
  peer and assert persisted entries and unchanged forbidden state.
- [x] Keep public availability disabled until B-87 completes. Run the first real Core-to-Codex
  tool path with the explicit test configuration and repeat B-85's allowed/refused operations.
  Update affected contracts and code-map rows with their actual implemented behavior.

**Verification:** `cargo build -p fake-codex --features test-support`, then
`cargo test -p shadows-agent --test codex`,
`cargo test -p shadows-core --test codex_session --test sessions --test acp_connection --test aylul_session`,
`cargo test -p shadows --test harness_config --test planner_mcp`, and clippy for the
changed crates. Then build the daemon in the existing cache, run the real signed-in
provider through the configured daemon, and observe streamed text, authorized MCP
tools, one terminal outcome, refusals and same-thread continuation. Record the
verified state and commit explicit paths.

## B-87: Complete controls and enable the harness

**Files:** Agent `codex/session.rs`, `codex/prompt.rs`, `codex/mapping.rs`, `choices.rs`,
`policy.rs`, and its contract/tests; Core Harness `assistant.rs`, `settings.rs`, `offers.rs`,
`model.rs`, `fork.rs`, `store/limits.rs`, and its contract;
`crates/core-db/migrations/0050_codex_modes.sql`, `crates/core-db/contract.yaml`,
`crates/core-projects/src/store/mod.rs`, `crates/core-projects/contract.yaml`,
`crates/shadows-core/tests/codex_session.rs`, `crates/shadows/tests/harness_choices.rs`,
`crates/shadows/tests/harness_effort.rs`, `crates/shadows/tests/harness_observation.rs`,
`docs/codebase/README.md`, the shared request, this plan/stages/spec.
Check the next unused migration number before dispatch and record any renumbering here.
On this branch 0050 was unused and is `0050_codex_modes.sql`; the unmerged `feature/executors` branch also adds a `0050_executor_sessions.sql`, so whichever merges second renumbers.

**Interfaces:** Preserve `HarnessInfo`, `SessionChoices`, `TurnSettings` and existing API
routes. `Harness::list` enables Codex only when configured and reports its configuration
reason otherwise. The mode list/default must be the measured B-85 policy. Use existing
mode/effort storage keys; introduce no second preference store or session-ID column.

- [x] Add tests `codex_choices_follow_the_model_catalog`,
  `codex_efforts_belong_to_the_selected_model`, `codex_unavailable_model_reports_its_error`,
  and `codex_project_modes_are_enforced`. Fetch the catalog, preserve order, select only offered
  efforts, keep busy checks and remember only settings belonging to committed turn starts.
- [x] Add the new modes for ordinary existing projects with an explicit migration; never
  broaden an existing project's authority automatically. New projects use the declared policy.
  The first conversation default is the least-privileged mode proven in B-85. Cover
  `codex_modes_migrate_without_granting_write_access` and
  `codex_new_projects_use_the_declared_mode_policy` in `codex_session.rs`.
- [x] Cover `codex_resume_does_not_replay_entries`,
  `codex_usage_is_last_request_not_cumulative`, and `codex_usage_is_not_double_charged_on_resume`.
  Pin an input with last=29130, total=54203, cached=24960 and window=665000: report
  29130/665000, not 54203 or 54090. Missing/zero windows stay unknown.
- [x] Map `account/rateLimits/read` and update notifications to existing account limits.
  Map actual provider errors without credentials/raw config in user-facing messages.
- [x] Preserve the existing fork and Send now actions using `thread/fork` and `turn/steer`.
  Add `codex_fork_keeps_the_source_unchanged` and `codex_steer_targets_the_active_turn`.
  If a visible action cannot be supported, resolve its explicit capability contract in the
  shared request before enabling Codex; never fabricate a successful result. Images can
  remain explicitly unsupported via the existing image capability.
- [x] Enable the configured harness, update contracts, and run the focused checks below.

**Verification:** `cargo test -p shadows-core --test codex_session --test thread_session`,
`cargo test -p shadows --test harness_choices --test harness_effort --test harness_observation`,
`cargo test -p shadows-agent --test codex`, and
`cargo test -p core-db embedded_migrations_match_the_base_catalog`. Regenerate OpenAPI only if public
types change for the accepted request; do not edit artifacts by hand. Commit the task.

## F-97: Use the existing conversation UI

**Files:** `web/src/app/conversation/cli-picker.tsx`, `draft-session.ts`, `use-session.ts`,
`operational-label.ts`, `entry.tsx`, `composer-bar.tsx`, `draft.test.tsx`,
`conversation.test.tsx`, `composer-bar.test.tsx`, `context-ring.test.tsx`,
`background-tasks.test.tsx`, `contract.yaml` (all under conversation);
`web/src/test/fake-daemon.ts`,
`docs/frontend/codebase/README.md`, `docs/frontend/stages.md`, the shared request.

**Interfaces:** Uses the existing harness/session API and stream through their domain hooks;
no direct provider calls, provider-specific permission inference, or hard-coded model list.

- [ ] Add tests for selecting configured Codex in a draft, the first-send handoff retaining
  `harness: codex`, the harness lock after a turn, and the server's missing-configuration reason.
- [ ] Cover per-model effort refresh and rejected model errors. Keep text/settings on a failed
  send and preserve command IDs for equal retries; changing settings makes a new command.
- [ ] Make labels/refusals describe Core's reported state without a Claude-only Auto hint on
  Codex. Test streamed text and tool results once each, unsupported images, no phantom background
  tasks, and the context/rate-limit values supplied by B-87. Keep the existing layout.
- [ ] From `web/`, run `npx vitest run src/app/conversation/draft.test.tsx
  src/app/conversation/conversation.test.tsx src/app/conversation/composer-bar.test.tsx
  src/app/conversation/context-ring.test.tsx src/app/conversation/background-tasks.test.tsx`,
  `npm run typecheck`, `npm run lint`, `npm run build`, and
  `npx vitest run src/codemap.test.ts src/architecture`. Update only the affected
  contract/map/stage facts and commit.

## B-88: Acceptance and delivery

**Files:** Existing Codex evidence record, this plan, both stages files and owner spec §12.14.

- [ ] Start `shadows serve` with the explicitly configured Codex executable. Respect the
  running process: report a locked executable instead of killing it or making another cache.
- [ ] In the browser, select Codex, create/read a planning draft through its tools, observe a
  forbidden operation refused, Stop an active tool turn, restart the backend and continue.
  Exercise model/effort selection, same-harness fork and Send now. Inspect server logs.
- [ ] Check a Claude conversation still runs and existing grants/session IDs remain scoped.
  Record bounded live calls, observed values, browser results and any unsupported behavior.
- [ ] Keep the local full gate delegated to CI. Run one whole-branch review only when assigned
  for delivery; no builder/reviewer is started by saving this plan. Mark acceptance complete
  only after the real browser run; open a PR only when requested.
