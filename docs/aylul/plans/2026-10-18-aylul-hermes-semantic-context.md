# AYLUL Work Explanation Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` for approved inline
> execution, or `superpowers:subagent-driven-development` only if the human selects
> delegation. This draft schedules no agents and authorizes no implementation.

**Goal:** Explain a real agent interruption from bounded local evidence through
Hermes, then deliver the person's direction to the identified session.

Size review (over 300 lines): this file owns the AYLUL integration task sequence.
Its task interfaces remain together for execution consistency; independent service
evaluation and document migration have separate plans linked here rather than
expanding this file into their implementation owners.

**Architecture:** Retain the NYLRA Node sidecar and hook as independent local
components. Consume Core through its services/API, query source through a local
project-bound standalone code-intelligence service selected after evaluation,
and use one remote Hermes AYLUL profile as the brain. Its process/install lifecycle
is independent of Core, AYLUL and the editor; direct coding clients can use it too.
AYLUL remains the main NYLRA/Tauri experience; load the existing Shadows workspace
only when the person chooses it. This host choice was confirmed during planning.

**Tech Stack:** Existing Rust/Axum/SQLx Core, TypeScript/Node 24.15+ sidecar,
React/Vite workspace, Tauri 2 native host, external Hermes. Semantic dependency
selection is open: Serena, standalone SolidLSP or an existing native LSP client.

**Spec:** [Draft design](../specs/2026-10-18-aylul-hermes-semantic-context-design.md).
The [standalone service spec](../../code-intelligence/specs/2026-10-18-code-intelligence-service-design.md)
owns code-intelligence runtime requirements. This document owns AYLUL integration;
it does not build one application-wide `AppCore` or all independent backends at once.
The [NYLRA transition plan](2026-10-18-nylra-transition.md) owns document/test/asset
movement and decision retirement; it is part of readiness, not optional cleanup.
The design's OPEN gates are execution dependencies, not a claim that integrations
already work. Proposed new symbols/files below do not yet exist.

## Global constraints

- Personal Windows-first pilot; no production, signup, billing, mobile, or voice delivery in this slice.
- Existing Core services retain their rules/data. No cross-process SQLite access.
- One composition root per runtime: existing `AppCore` for Shadows, native/sidecar startup for AYLUL, an independent service entry point for code intelligence, upstream startup for Hermes.
- Hook/sidecar remain independent of AppCore; no TypeScript-to-Rust rewrite.
- The code-intelligence package is independent: stopping Core/AYLUL does not stop a separately launched service used by other clients. No duplicate LSP stack inside each client.
- Source queries stay bound to one selected worktree. If Serena is selected, use one worker per active worktree; first pilot one, concurrent-project acceptance two.
- Assessment is read-only: no Planner turn, approval call, shell/edit/REPL tool or write-capable MCP credential.
- Evidence bounds: 8 excerpts, 120 lines each, 64 KiB UTF-8 total, 10 semantic queries, 20 seconds retrieval deadline.
- Recheck sharing and binding before emitting results after asynchronous work.
- Explicitly addressed original user text can be sent directly; model rewrites stay drafts until the person sends them.
- Delivery is not completion. Ambiguous writes are not automatically replayed.
- Read contracts before code. Update the changed owner's contract and map in the same eventual code commit.
- Existing owner specs must be amended in place before implementation changes their decisions.
- No product source relocation/deletion, push, merge, deployment or ignored-secret reads are authorized. The human separately authorized documentation snapshot commits on new Shadows and shadows-backend branches on 2026-10-18.

## Review focus

| Failure condition | Required behavior | Owning task |
|---|---|---|
| Same symbol name in two worktrees | Correct root or explicit ambiguity, never a guessed match | T2 |
| Root opt-out/revocation during an awaited query | No result emitted after the policy changes | T2/T3 |
| Replayed Stop/terminal event after reconnect | No duplicate notification or extra model dispatch | T1/T5 |
| Connection lost after a direction may have been accepted | Receipt lookup or unknown; no silent resend | T6 |
| Existing Claude/Codex session belongs to another process | Unsupported until a live attachment/control route is proven | T0/T6 |

## Repository coordinates and change discipline

At planning time: `S = E:/Globalprojects/shadows` at `6c17249`;
`N = E:/Globalprojects/nylra` at `2b04273`;
`B = E:/Globalprojects/shadows-backend` at `e4d178a`;
`H = F:/projects_frok/hermes-agent-shallow` at `ccc695635094d7615373db267b77f894d978b1a0`.
Recheck branch/status at execution time. These are evidence snapshots, not immutable workspaces.

Implementation belongs to each owning repository/side. Keep contracts beside new
cohesive modules, not one contract per helper. Native-to-web changes need accepted
requests under the existing `docs/shared/requests/` workflow before they are built.
Stage files record planning readiness separately from product execution. Retain
completed native/character tasks and their evidence; no product task becomes done
because a new architecture document was written.

## Execution tracks and readiness

| Track | Next deliverable | Dependency |
|---|---|---|
| Vision/document continuity | Migration M-0/M-1 ledger, active owner updates and preserved acceptance | [NYLRA transition plan](2026-10-18-nylra-transition.md); physical moves remain separate |
| Standalone code intelligence | CI-1/CI-2 evidence, then one selected package plan | [Dedicated baseline plan](../../code-intelligence/plans/2026-10-18-code-intelligence-baseline.md); existing Serena smoke is only partial evidence |
| AYLUL local context | T1 observations and session/root binding | Native scope review; independent of the semantic implementation choice |
| Core context | T3 scoped projection | Existing section 1.7/24.6 caller contract and accepted frontend/native request |
| Remote brain | T0 Hermes protocol proof, then T4 | Pinned protocol and isolated connection; does not own local component lifetime |
| End-to-end explanation/control | T5/T6 | Above data paths plus each adapter's actual control capability |
| Primary UI and optional workspace | T7, then T8 acceptance | Stable native/sidecar contract and proven path; no source relocation prerequisite |

The first executable phase is the dedicated standalone baseline. Do not start
replacement package scaffolding or a repository merge while its implementation
choice is open. No product task is marked complete by these documentation changes.

No Hermes core modifications are planned. If transport adaptation requires Python,
ship a standalone Hermes plugin outside upstream, using the documented extension surface.
Do not silently route a tool-capable AYLUL conversation through the backend's current
text-only `ReplyPort`; that is a different integration.

## T0 - Prove external capabilities before committing to transports

Partial checkpoint: [2026-10-18 Serena baseline](../../evidence/code-intelligence/2026-10-18-serena-baseline.md)
records the completed isolated installation and Rust MCP smoke. This does not
complete the broader T0 matrix, real-project evaluation or replacement package.

**Owner:** native integration; isolated capability investigation first. The human
explicitly requested downloading/running Serena to learn its interface; this part
is authorized independently of implementing a replacement package.
**Files:** read `N/apps/desktop/sidecar/src/{handlers.ts,protocol.ts}`,
`H/apps/shared/src/{json-rpc-gateway.ts,gateway-contract.generated.ts}`,
`H/tui_gateway/{ws.py,methods_prompt.py,methods_session.py}`;
write dated evidence `S/docs/evidence/aylul/2026-10-18-capabilities.md` when actually run.

**Interfaces produced:** a versioned capability matrix with source-session identity,
observe/read-tail/send-idle/steer-busy/interrupt/approval support and exact ACK semantics.
Unsupported and untested are different values. Pin the Hermes wire schema artifact
and the selected semantic backend's installation/lock inputs; no dependency on moving `main`.

- [ ] Record executable versions, process ownership and isolated test data locations; never use real credentials in the report.
- [ ] Compare a minimal fixture against Serena, standalone SolidLSP and an existing native LSP client integration. Record which is actually exercised, Windows/dependency cost and license boundaries; no three-provider implementation. For a Serena trial, pin commit `d9e4c796fae0207ba6848d1df4b331122a5567b5` or a deliberately selected release and use `tools` mode.
- [ ] On a synthetic repository, resolve an overloaded/same-named Rust symbol and a TS class method, then their callers; record results and startup/query latency.
- [ ] Prove the candidate's exposed read surface rejects writes and arbitrary execution. For Serena, explicitly test shell/REPL exclusions and activation-command refusal. Record language-server build/cache effects for every exercised candidate.
- [ ] Probe remote Hermes session create/resume, prompt receipt, event replay and interruption against one synthetic AYLUL profile. Verify its actual tools include only the bridge reads needed for assessment.
- [ ] Probe an already-running external Claude idle/busy session. Test the installed version's supported wake mechanism; latest `asyncRewake` documentation is a hypothesis until this passes. Do not fake a successful delivery by launching another conversation.
- [ ] Probe Codex against the actual owning app-server connection. `turn/steer` must name the expected active turn. A separate app-server cannot be assumed to own a running CLI or Desktop chat.
- [ ] Delete throwaway probe source after recording evidence, naming the source commit if any. Do not retain a sandbox crate or commit synthetic success as real-device acceptance.

**Gate:** T2/T4/T6 consume verified schemas and capabilities. If arbitrary external
idle control is unavailable, the human chooses a supported NYLRA-owned launch mode
(without using Planner), manual handoff, or a revised requirement before T6 enables it.
Do not downgrade the desired behavior silently.
G6 selects the semantic implementation before T2's integration; it does not
prevent T1/T3 or a clearly labeled map/contract/text explanation from progressing.

## T1 - Bind existing hook events to the correct workspace

**Files (N):** modify `apps/desktop/sidecar/src/{protocol.ts,handlers.ts,main.ts}`,
`hooks/route.ts`, `sessions/{session.ts,contract.yaml}`, `store/{store.ts,contract.yaml}`;
create `sessions/{binding.ts,binding.test.ts}`, `observations/{journal.ts,journal.test.ts,contract.yaml}`.
Keep existing hook and permission handling intact.

**Interfaces:** `SessionRef { sessionKey, agent, sourceSessionId, rootId, generation }`;
`bindSession(input): SessionRef | BindingRequired`;
`appendObservation(ref, observation): { seq: number }`;
`observationsAfter(after, limit): Observation[]`.
Optional Core project/thread/operation/task references are explicit bindings.

- [ ] RED: tests `project_root_survives_nested_cwd`, `two_worktrees_are_distinct`, `unknown_root_requires_selection`, `same_source_id_on_two_devices_does_not_collide`.
- [ ] RED: `replay_does_not_notify_twice`, `new_turn_with_same_text_is_not_discarded`, `stop_is_not_automatically_blocked`, `observation_cursor_survives_reopen`.
- [ ] Implement the smallest persistent observation journal in the existing SQLite owner. Use source event identity when available; otherwise document limited source deduplication and dedupe attention by known turn/state, not text alone.
- [ ] Keep `lastMessage` as a 2,000-character preview. Add an opaque message reference/digest plus explicit truncation; no arbitrary transcript path in cloud requests.
- [ ] Verify with `node --test src/sessions/binding.test.ts src/observations/journal.test.ts src/sessions/sessions.test.ts` from the sidecar, then `npm run typecheck`.
- [ ] Update contracts/map and review the scoped diff. Commit only when execution authorization includes commits.

## T2 - Connect to the independent code-intelligence service

**Files (N):** create `apps/desktop/sidecar/src/context/{policy.ts,files.ts,semantic.ts,types.ts,policy.test.ts,semantic.test.ts,contract.yaml}`;
modify `apps/desktop/sidecar/src/main.ts` only for composition and shutdown.

**Interfaces:** `CodeQuery { rootId, operation, symbol?, relativePath? }`;
`queryCode(ref: SessionRef, query: CodeQuery, signal: AbortSignal): Promise<EvidenceResult>`;
`EvidenceResult { items: SourceExcerpt[], omissions: string[], status }`;
`SourceExcerpt { rootId, relativePath, symbol?, startLine, endLine, digest, capturedAt, text }`.
Root paths resolve from local registration, never a model-supplied absolute path.
`context/types.ts` also declares `ContextBundle { session: SessionRef, cause,
messages, planRefs, evidence, basisDigest, omissions }`; T4 consumes the type,
and T5 implements its builder. Before T5, T4 tests use bounded synthetic bundles.

- [ ] RED: `same_named_symbol_stays_in_selected_root`, `symlink_and_junction_escape_refused`, `ignored_secret_file_is_never_read`, `dependency_reference_is_omitted`, `revocation_during_query_releases_no_result`.
- [ ] RED: `read_surface_rejects_writes_and_arbitrary_execution`, `timeout_cancels_only_own_request`, `core_shutdown_leaves_shared_service_running`, `excerpt_and_total_budgets_are_enforced`, `changed_file_invalidates_cached_excerpt`.
- [ ] Close G6/G7. `semantic.ts` is a client of the selected standalone package, not a second LSP supervisor. Use a standard MCP/client library; do not implement a generic protocol stack or language semantics. Replacement package implementation requires its accepted standalone task plan.
- [ ] `policy.ts` owns disclosure consent; `files.ts` owns bounded local context-document reads; `semantic.ts` owns the client connection. The independent service owns source-query policy and language-server workers. Preserve Windows-native paths. A Serena baseline can use a client-owned stdio process; shared service shutdown is a distinct ownership case.
- [ ] Existing Shadows name-index results remain labeled discovery, not semantic references. Core can expose its own authorized client view over the independent service; the service remains the language-server owner. No duplicate LSP supervisor is built inside Core or the sidecar.
- [ ] Verify `node --test src/context/policy.test.ts src/context/semantic.test.ts` plus `npm run typecheck`; run the selected backend's real synthetic integration separately and retain provenance.

## T3 - Expose the Core context promised by its owner spec

**Files (S):** create `crates/shadows-core/src/projects/nylra.rs`,
`projects/store/nylra.rs`, `crates/shadows-http/src/nylra.rs`,
`crates/shadows-core/migrations/0027_nylra_sharing.sql`,
`crates/shadows-core/tests/nylra_context.rs`, `crates/shadows/tests/nylra_context.rs`;
modify `projects/{mod.rs,model.rs,contract.yaml}`, existing `projects/store.rs`
for its private child module declaration, HTTP router/OpenAPI, and explicit
read ports/contracts in `threads`, `plans`, `design` as required for a consistent read.
If migration 0027 exists at execution time, allocate the next number before editing.

**Interfaces:** `Projects::set_nylra_sharing(command_id, project, enabled)`;
`Projects::nylra_context(project, thread, limits) -> NylraContext`;
`NylraContext` carries `nylra-context/1`, project revision, versioned source items,
and explicit omissions. Define the serialized schema in Rust, export through OpenAPI.
Proposed routes: `PUT /api/projects/{id}/nylra-sharing`,
`GET /api/projects/{id}/nylra-context?thread_id=...`.

- [ ] Before code, amend existing architecture 1.7 with the reviewed caller/authority design. A local-only read is not authorization to expose mutation routes remotely.
- [ ] RED: `sharing_defaults_off`, `opt_out_refuses_next_read`, `other_project_thread_refused`, `projection_is_bounded_and_versioned`, `read_is_one_consistent_snapshot`, `projection_cannot_mutate_plans`.
- [ ] RED: `message_append_notice_resolves_to_committed_agent_entry`, `tail_includes_relevant_message_beyond_preview`, `stale_or_removed_source_is_explicit`.
- [ ] Implement one Projects operation boundary. Other services declare transaction-scoped read ports where necessary; do not bypass their ownership by querying arbitrary tables in an adapter. Include roadmap/plan state and the named thread's bounded recent person/Planner messages only.
- [ ] Verify `cargo test -p shadows-core --test nylra_context` and `cargo test -p shadows --test nylra_context`; update contracts/code map and regenerate canonical OpenAPI/types using existing exporter commands.
- [ ] Keep existing `/api/subscribe` semantics. Sidecar reads entries on committed notices and persists its cursor; a transient turn-end frame is not the only trigger.

## T4 - Connect the local bridge to one Hermes AYLUL profile

**Files (N):** create `apps/desktop/sidecar/src/brain/{hermes.ts,hermes.test.ts,contract.yaml}`,
`apps/desktop/sidecar/src/bridge/{assessment-tools.ts,assessment-tools.test.ts,contract.yaml}`;
modify native connection storage/commands under `apps/desktop/src-tauri/src/commands/`
with a separate reviewed connection contract. Generated Hermes wire types carry
upstream commit and generator provenance; never hand-edit a second API definition.

**Interfaces:** `explain(bundle: ContextBundle, signal): AsyncIterable<ExplanationPart>`;
`ExplanationPart = textDelta | done | unavailable`;
read-only bridge tools `read_work_context`, `query_code`, `read_source_excerpt`.
Connection configuration selects one URL/profile; secrets stay native-side, not in the webview.

- [ ] RED: `auth_material_never_enters_renderer`, `assessment_tool_call_cannot_dispatch_action`, `profile_routing_does_not_bleed`, `reconnect_resumes_without_duplicate_prompt`.
- [ ] Implement the pinned protocol proven in T0. Reuse the transport client where distributable; otherwise a narrow adapter uses the upstream-generated schema and the standard WebSocket implementation. No Hermes core fork.
- [ ] Personal network path: authenticated HTTPS/WSS to Hermes; a private route for its read-only MCP access to the local bridge. Bind the bridge to loopback/private ingress only. Do not expose the raw Serena process or Core to the internet.
- [ ] Recheck local consent for every tool call. Return stable unavailable/refused outcomes when laptop, project or the semantic backend is offline; never fall back to searching the server's unrelated filesystem.
- [ ] Verify `node --test src/brain/hermes.test.ts src/bridge/assessment-tools.test.ts` and `npm run typecheck`, then a synthetic two-machine round trip separately.

## T5 - Explain an attention item without querying the Planner

**Files (N):** create `apps/desktop/sidecar/src/assessment/{bundle.ts,attention.ts,explain.ts,assessment.test.ts,contract.yaml}`;
create `apps/desktop/sidecar/src/shadows/{client.ts,events.ts,client.test.ts,contract.yaml}`;
modify existing composition only.

**Interfaces:** consume `ContextBundle` from T2 and `ExplanationPart` from T4;
`buildContext(ref, cause, signal): Promise<ContextBundle>`;
`assess(ref, cause, signal): AsyncIterable<ExplanationPart>`;
`onObservation(event): Promise<void>` schedules at most one active assessment per attention basis.

- [ ] RED: `explanation_uses_contract_then_targeted_symbols`, `no_plan_binding_still_explains_external_session`, `ambiguous_task_is_not_guessed`, `assessment_never_starts_planner_turn`.
- [ ] RED: `duplicate_replay_reuses_attention`, `normal_completion_stays_quiet`, `changed_plan_or_code_invalidates_explanation`, `offline_brain_keeps_local_attention_visible`, `evidence_budget_returns_omissions`.
- [ ] Resolve full last-message evidence locally, without copying unlimited transcripts. The transcript reader verifies the T0 transcript schema/session identity and limits before reading. Raw hook tool input is not automatically sent to Hermes.
- [ ] Present facts, inference and missing evidence separately. Use a bounded evidence cache keyed by source digests/revision vector. No permanent third project-memory system or automatic full repository onboarding.
- [ ] Verify `node --test src/assessment/assessment.test.ts src/shadows/client.test.ts` plus typecheck. A fake model test asserts exact zero calls to Core prompt endpoints during explanation.

## T6 - Deliver a person's direction through the actual session owner

**Files (N):** create `apps/desktop/sidecar/src/control/{dispatch.ts,receipts.ts,dispatch.test.ts,contract.yaml}`;
create proven adapters `control/{core.ts,claude.ts,codex.ts}` only as their T0 gates pass;
modify `handlers.ts` and native allowed-method tables.
**Files (S):** add a scoped companion entry point in `turns/companion.rs`, its service
contract and HTTP adapter, with `crates/shadows-core/tests/companion_send.rs` and
`crates/shadows/tests/companion_send.rs`. Reuse Turns' existing lifecycle internally.

**Interfaces:** `sendDirection(ref: SessionRef, command: UserDirection): Promise<DeliveryReceipt>`;
`UserDirection { commandId, body, expectedGeneration, expectedTurnId?, origin }`;
`DeliveryReceipt { commandId, state, ownerReceipt?, reason? }`;
states are `queued | accepted | delivered | completed | failed | unknown | unsupported`.
The trusted native action path sets origin; model JSON cannot supply human authority.

- [ ] Before Core mutation code, close the section 24.6 caller/actor gate for this exact local-person delivery path. Keep privileged approvals separate. Do not call an existing User/local route from the remote model and relabel it human.
- [ ] RED: `wrong_project_or_generation_refused`, `same_command_same_body_replays`, `same_command_changed_body_conflicts`, `lost_ack_is_unknown_until_receipt_lookup`, `model_text_cannot_approve_permission`, `unsupported_adapter_has_no_send_control`.
- [ ] Core-managed adapter maps idle send/busy queue/explicit Send now to their existing owners. External adapters use only T0-proven native control APIs. Never launch a second writer against the same live session as a resume workaround.
- [ ] A typed original user direction can dispatch directly. A suggested rewrite requires the person's Send action; no separate approval dialog for an ordinary already-authored message. Voice approval remains outside this slice.
- [ ] Verify `node --test src/control/dispatch.test.ts`; run focused Core tests above and native allowed-method tests. Then prove actual idle/busy delivery and observe the resulting agent turn, including an unrelated second session remaining untouched.

## T7 - Keep AYLUL primary and load the existing Shadows workspace on demand

**Files (N):** create `apps/desktop/src/features/assistant/{AssistantPage.tsx,AttentionCard.tsx,SourceLinks.tsx,DirectionComposer.tsx,contract.yaml}`,
`apps/desktop/src/platform/assistant.ts`, `apps/desktop/scripts/stage-shadows-ui.mjs`,
`apps/desktop/src-tauri/src/windows/shadows.rs`, and
`apps/desktop/tests/companion/assistant.spec.ts`;
modify `apps/desktop/src/app/main.tsx`, app/features/platform contracts, native
window/command/capability owners, `tauri.conf.json`, and the NYLRA map/shared request.
The parent features contract excludes the new assistant child contract explicitly.
**Files (S):** modify `web/src/router.tsx` for the packaged hash-history option,
`web/vite.config.ts` only if required for a separate asset base, frontend map and
the accepted packaging request. Preserve the existing Planner/plan components.

**Interfaces:** `AylulHost { listAttention(), explain(id), sendDirection(ref, text), subscribe(after) }`.
Typed bootstrap capabilities expose this only to the trusted main window, not arbitrary
web pages or the companion renderer. Cloud credentials are never returned.
`openShadowsWorkspace()` is a separate native action: create/focus the `shadows`
window on explicit user request only; it does not inherit AYLUL's privileged IPC.

- [ ] Implement the confirmed host choice: keep AYLUL at `index.html` in the main window, stage the built Shadows UI under `dist/shadows/`, and open `shadows/index.html` in the optional `shadows` window only on request. This copies build artifacts, not source/business rules. Closing that window leaves AYLUL and its sidecar running.
- [ ] Use desktop hash history for the staged entry; preserve normal browser routing. Windows Core access permits the exact `http://tauri.localhost` origin with an explicit CSP endpoint; never wildcard CORS. Test the packaged origin, not only Vite.
- [ ] RED: `aylul_is_default_and_shadows_is_lazy`, `closing_shadows_keeps_aylul_alive`, `stop_opens_attention_without_focus_steal`, `explanation_has_source_links_and_omissions`, `send_targets_selected_session`, `workspace_cannot_call_main_only_ipc`, `core_offline_does_not_break_external_observation`.
- [ ] Keep existing Planner routes working both in a browser and the packaged workspace. AYLUL remains independent of Core availability; no mock-success controls or eager loading of the full Shadows workspace at AYLUL startup.
- [ ] Preserve the independently owned character renderer/studio and native companion. Pin `hidden_character_preserves_text_and_monitoring`, `renderer_failure_preserves_main_and_tray`, `workspace_window_cannot_grant_render_permission` and `production_excludes_studio`; run affected existing character/native tests without rewriting their old evidence.
- [ ] Verify `npx playwright test tests/companion/assistant.spec.ts` and `npm run build` in N/apps/desktop, typecheck/lint in S/web, and native command/window tests. Then physically check packaged Windows source links, tray, companion, lazy workspace loading and two concurrent sessions.

## T8 - Measure the whole journey and select the migration boundary

**Files:** add a persistent integration test driver in
`N/apps/desktop/tests/aylul-context-journey.test.mjs` and dated results under
`S/docs/evidence/aylul/`; no new throwaway product crate.

- [ ] Exercise external Claude interruption -> selected root -> contract/symbol evidence -> Hermes explanation -> user direction -> actual target continuation. Run the same journey for a Core-managed session.
- [ ] Exercise Codex using the transport that passed T0; record separately any existing-Desktop/CLI attachment limits. Failure is not a pass for the broader requirement.
- [ ] Repeat through restart/reconnect, an opted-out root, missing language server, busy target, duplicate command, same-named symbols and two worktrees. Use synthetic content until disclosure/lifecycle gates are satisfied.
- [ ] Compare five fixed explanation questions with the same model/settings: bounded map/contract/text retrieval versus the selected semantic backend. Record input/output tokens, tool result bytes, cold/warm latency, source correctness and whether a Planner call occurred. No savings claim from a single attractive example.
- [ ] Report the five-question result as a pilot sample, including failures and setup cost; do not generalize it into a guaranteed saving for all projects/models.
- [ ] Run focused checks during tasks. Once per finished branch, run the repository's final gates. For Core: fmt, both required Clippy modes, workspace tests, no production test-support features and canonical API freshness. For web/native: existing required build/lint/type/test gates plus physical Windows use. Hermes/Serena upstream tests are not replaced by these client checks.
- [ ] Select one active brain path. Keep backend account/relay/chat work intact until a separate migration decision says which responsibilities remain. Do not dispatch one turn through both engines.
- [ ] Execute physical consolidation only through the reviewed migration manifest: source/contracts/tests together, one active document owner, retained historical reports and new post-move evidence. Retire a replaced decision/runtime only after its successor and retained data/artifacts are accounted for.

## Completion evidence and handoff

The original draft used static source inspection and official documentation.
The subsequently authorized Serena installation/Rust smoke is recorded separately
in the linked T0 evidence. No model call, network deployment, product code change
or AYLUL integration has been performed. The screenshot's VS Code extension is not
evidence about the independent Serena installation.

Implementation starts after the human reviews the design/plan and resolves the
authority decisions; the AYLUL-first host choice is already recorded. T0 then produces the facts needed to commit to the
remaining adapter interfaces. A source-backed explanation is the first delivery;
the complete requested feature additionally requires proven direction delivery.
