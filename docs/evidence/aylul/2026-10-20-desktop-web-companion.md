# Shared desktop dashboard and bound companion — 2026-10-20

## Result

Tasks 1–9 implemented on codex/aylul-desktop-web-companion. Mohammed explicitly
waived the final reviewer and reserved the actual conversation/native trial for
himself. No reviewer was dispatched, and no push or merge was performed.

The actual debug executable is running with the unchanged app.nylra.desktop
identity and existing native profile. Core is ready on http://127.0.0.1:4318.
The read-only desktop-web-companion.ps1 -VerifyOnly driver passed: actual desktop
PID, Core process start identity, runtime/configuration IDs and DB receipt agree.
The API returns provider m with keySet=true, live agent ay using MiniMax-M3[1m],
and Book Catalogue, CSV Tool, shadows and the internal AYLUL project.
No credential value was read and no assistant Send was performed by the driver.

## Existing database and startup diagnosis

The launcher .claude/launch.json declares LocalAppData/shadows-dev/shadows.sqlite3.
Claude Windows packaging redirects its process to the existing physical file:

    %LOCALAPPDATA%/Packages/Claude_pzs8sxrjxfjjc/LocalCache/Local/shadows-dev/shadows.sqlite3

The controller initially selected the unvirtualized ordinary path too early.
It contained no user provider/history. Read-only inspection located the actual
provider, agent, projects, nine threads and 112 entries in the packaged path.
The controller stopped only its own empty-runtime process after checking its
runtime UUID, DB receipt, executable and exact process start time. It configured
the physical original file through the actual desktop UI. No database copy,
merge, provider recreation or credential migration was performed.

Actual startup then exposed SQLx VersionMismatch(29). The stored SHA-384 equals
exactly the CRLF variant of canonical migration 0029; its SQL content is the
same. db/embedded_migrations.rs selects that historical source through official
SQLx Migration/Migrator APIs only when its complete checksum matches. SQLx still
validates every migration and dirty state. Fresh databases retain canonical LF;
changed SQL or another migration mismatch still refuses startup. No SQL migration
file or applied record was rewritten, and migration 29 was not rerun.

The preserved actual checksum before and after startup is:

    6CB0D01E2BD42BFD1ACBF3A7AFED0E9E8659500AE66F577FE9D7B2F5151FDBD3F0E5668FC915D2E773B73A65D96345C5

PRAGMA quick_check returned ok before and after. Existing provider/agent/project
metadata remains present. Counts grew during live UI use (10 threads, 116
entries at the last observation), so a frozen-count assertion was corrected;
readiness does not forbid the person using their app. The controller sent no
real or synthetic assistant message in this actual-profile setup.

## Local verification

Windows, Rust 1.98.1, Node v24.15.0, npm 11.12.1. Existing target directories and
ordinary debug profile only; no clean/release/custom Cargo target or profile.

| Check | Observed result |
| --- | --- |
| Root format and CI Bash 100-character check | Passed |
| Workspace all-target clippy with fake-acp/test-support | Passed |
| Full workspace tests before startup compatibility fix | 631 passed, 1 ignored |
| Production workspace clippy and no-dev feature tree | Passed; no test-support |
| Canonical OpenAPI regeneration and TS generation | Passed; no generated drift |
| Web typecheck/lint/full tests | 63 files, 326 tests passed |
| Independent browser web build | Passed; existing large-chunk advisory |
| Shared desktop assembly | Passed |
| Retained desktop Node tests | 4 passed |
| Companion unit / production / Chrome browser | 15 / 1 / 41 passed |
| Native format / all-target clippy / debug build | Passed |
| Full native tests | 80 passed, 2 ignored |
| New profiles migration regression | 1 expected RED, then all 4 GREEN |
| Existing migration regression suite after the fix | 4 passed |
| Core all-target clippy after the fix | Passed |
| Codemap and actual Core debug rebuild after the fix | Passed |
| Actual default-profile readiness PowerShell driver | Passed, no message sent |
| Actual DB recorded checksum and quick_check | Preserved / ok |

The initial root debug_log failure came from inherited RUST_LOG=warn. Removing
that variable only in the validation shell, then restoring it, produced the
full passing run without product/test changes. Codemap initially rejected a
section suffix after a reference path; correcting the table cell passed its
check. Initial CDP inspection raced navigation; waiting for the final URL fixed
inspection. A later Start-Process request was rejected with only blocked by
policy; a subsequent explicit actual-program launch succeeded after the real
configuration was identified. These driver/tool incidents are not provider
acceptance results.

Sidecar/hook source is unchanged, and their separate physical acceptance was
not repeated. Ignored native external tests were not claimed as passing.

## Acceptance distinctions

The earlier measured native Origin/CORS/CSP probe is in
[origin evidence](2026-10-20-desktop-webview-origin.md). Task 4 separately proved
one Send/reload, native ACL refusal, unknown-asset rejection and Core persistence
using an isolated real Core with fake ACP. It predates Mohammed's actual-profile
instruction and does not establish the requested real-provider/tray journey.
Tasks 6–8 cover binding generations, replay, terminal order, reconnect, preview
filtering/expiry and the normal Send/Queue boundary. The 240px Arabic preview was
visually inspected; fixtures do not establish physical focus or fitting.

Pending Mohammed's trial: real assistant response, background completion with
main hidden, no-focus show, exact-chat restore, switching the binding during a
turn, hide/pause persistence, actual tray Quit/relaunch and retained account/hook
confirmation actions. Voice, production, mixed-DPI transitions and optimized
resource measurements remain outside this evidence.

## Source size responsibility

Changed handwritten source at or above 300 nonblank lines retains one job:

- presence/projection.rs: scoped operation presentation reduction.
- presence/watcher.rs: one cancellable bound-thread observation lifecycle.
- presence/tests.rs: deterministic projection regression coverage.
- presence/watcher_tests.rs: HTTP/SSE observation lifecycle regression coverage.
- crates/shadows/tests/desktop_runtime.rs: runtime identity acceptance.
- crates/shadows/tests/openapi.rs: canonical OpenAPI verification.
- web/src/api/client.ts: the existing typed Core HTTP boundary.
- web/src/app/conversation/composer.tsx: the existing conversation composer admission.

web/src/api/schema.d.ts is generated. The handwritten watcher remains below
500 lines. The new migration source selector and its tests each remain below
300 lines; they do not introduce a second migration engine.

## Tool state for handoff

A controller scripting mistake inserted into the list it was iterating and left
a Serena request running. The exact runaway Serena server process was identified
and stopped; the next tool call reported Transport closed. Remaining necessary
documentation work used Node under Mohammed's permitted fallback. MCP settings
and unrelated processes were not changed. Serena needs a client reconnection.
The actual desktop/Core remain separate and running.

## Execution rulings

- Ruling: Latest user instruction enables one final fresh-context reviewer on the session's own model, overriding the plan's previous no-reviewer wording — explicit user choice — cost if wrong: one review dispatch.
- Ruling: Work in this existing branch, without a worktree or dependency reinstall — user's saved in-place choice plus apps/desktop/AGENTS.md — cost if wrong: changes share this checkout.
- Ruling: Focused baselines and touched tests per commit, full gate once at the end — repository rules override skill full-suite defaults — cost if wrong: unrelated regressions discovered at final gate.
- Task 1: Ruling: Add temporary JavaScript initializer and Node collector beside the planned Rust/PowerShell probe — observes actual HTTP Origin without trusting the URL and uses real Core APIs with unused harness paths — cost if wrong: native behavior is rechecked in Task 9.
- Task 1: Ruling: Run the Node driver directly because Windows PowerShell 5.1 refuses unsigned scripts; no policy change — same driver and assertions — cost if wrong: PowerShell wrapper remains separately unverified.
- Task 1: Ruling: Temporarily link Tauri resource.lib to the probe example, then restore build.rs — executable exited 0xC0000139 because Cargo rustc-link-arg-bins excluded examples and their Common Controls v6 manifest; verified by PE imports/build output and native pass after the link — cost if wrong: native-main acceptance still checks the normal binary.
- Task 2: Ruling: Add ProcessIdentity::is_current as the shared safe verifier and isolate Windows FFI in one allowed function — native attach must reject reused process IDs without duplicating OS logic — cost if wrong: one infrastructure API to maintain. Serde/Windows bindings are infrastructure; serde_json remains test-support-only.
- Task 3: Ruling: Include the actual canonical DB path in the private receipt, while keeping public readiness IDs only — a caller-supplied configuration UUID alone cannot prove which DB Core opened — cost if wrong: one private receipt field and native validation to maintain. Test receipt_identifies_actual_database observed RED before this addition.
- Task 3: Ruling: Run the existing shadows/cors integration tests instead of shadows-http guard::, which selects zero tests — actual origin coverage lives in cors.rs — cost if wrong: none; the intended behavior is exercised.
- Task 3: Ruling: Missing desktop identity returns the repository standard ErrorBody with RUNTIME_NOT_CONFIGURED, not the plan's empty 404 — OpenAPI and route-presence contracts require structured errors; read-only test observed RED before the fix — cost if wrong: one additional stable wire error code.
- Task 4: Ruling: Retain a private pending process identity after readiness timeout — a second desktop or explicit Retry must not spawn again while the first Core is still starting — cost if wrong: one private startup record to reconcile.
- Task 4: Ruling: Simplify canonical Windows paths with dunce only at the launch argument boundary — real Arabic-path test failed because SQLx parses the verbatim prefix as URL query syntax; normalization and receipt comparison remain canonical — cost if wrong: unsupported long path fails explicitly rather than selecting a different DB.
- Task 4: Ruling: Make the existing frontend code-map reader normalize CRLF — Serena edits use Windows line endings and the old contract parser retained the carriage return in each folder name — cost if wrong: none, ownership checks retain their meaning.
- Task 4: Ruling: Debug acceptance supplies separate explicit CDP ports for the isolated main/companion WebView profiles — a single WebView2 environment port reaches only one of their independent browser processes — cost if wrong: debug-only acceptance argument, absent in release.
- Task 7: Ruling: Use pulldown-cmark 0.13.4 for CommonMark parsing, with product-owned filtering/truncation; eventsource-stream 0.2.3 for SSE framing; async reqwest/Tokio/futures for transport, axum only for HTTP/SSE test fixtures — infrastructure follows library-first and avoids inventing Markdown/framing parsers — cost if wrong: maintained parser/transport dependencies in the independent native lockfile. Versions/APIs verified against crates.io and official docs.
- Task 8: Ruling: Native target_companion_thread waits up to two seconds for the exact generation/configuration/thread feed to reconcile before returning — otherwise a fast first turn can finish before the initial SSE subscription, becoming historical replay and losing its reaction — cost if wrong: bounded companion setup latency, independently reported without blocking/cancelling/replaying ordinary sends. Added a native fast-completion regression and readiness channel; this is the first caller needing it.
- Task 9: Ruling: Extend maintained fake-ACP with a code-block response for packaged CSP acceptance, plus owned Node/Win32 journey helpers beside the planned PowerShell wrapper — exercise the actual lazy code renderer and physical owned windows without real provider credentials or personal hooks — cost if wrong: a test-only adapter case and reusable native acceptance drivers.
- Task 9: Ruling: Latest human instruction replaces isolated/synthetic hands-on acceptance with the actual built application, its existing database/profile and real assistant. Removed the just-prepared fixture refactor and unused fake code-block prompt; no new runtime/database copy was launched. Existing automated regression fixtures remain tests — cost if wrong: none, this follows explicit user direction.
- Task 9: Ruling: Claude's packaged process virtualized the normal launcher DB into Packages/Claude_pzs8sxrjxfjjc/LocalCache/Local/shadows-dev; readonly inspection found the real provider m/agent ay, 3 visible projects, 9 threads and 112 entries, quick_check ok. Correct native connection to that exact existing physical database; no copy or data merge. Initial ordinary-path connection was the controller's wrong selection and its own empty-runtime PID was stopped after process-time/runtime/DB verification — cost if wrong: pointing to the wrong personal history, so actual metadata is verified before use.
- Task 9: Ruling: Actual database startup exposed migration 29's stored checksum matching exactly canonical SQL converted LF→CRLF. Add db/embedded_migrations.rs source selection through official SQLx Migration/Migrator APIs only; preserve the record, SQL contents and user rows; retain strict checks for every other mismatch. Owner §7/map and regression test added to Task 9 scope as necessary startup compatibility — cost if wrong: startup acceptance of altered SQL; tests cover preserved checksum/data, canonical fresh startup, changed 29 SQL refusal and other-version refusal.
- Final: Ruling: Mohammed explicitly waived the final reviewer and reserved the conversation trial for himself. Do not dispatch a reviewer or another test message; hand off the ready program and record remaining manual acceptance — cost if wrong: human acceptance remains unproven until he tries it.
- Final: Ruling: Keep the current branch/check-out for Mohammed to continue; no push/merge/reviewer, as explicitly requested. Retain this plan workspace with the local logs until his manual acceptance rather than presenting a waived review as a clean review — cost if wrong: only retained ignored diagnostic disk usage. Task 9 commit title describes actual startup/data preservation rather than claiming the deferred physical journey.
