# Product daemon Hermes implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline, task by task.

**Goal:** A local conversation through a Rust-supervised TypeScript assistant,
with durable recovery and an explicit choice of local or account-backed service.

**Architecture:** Extract the existing native Hermes client into a packed npm
package. Keep conversation state in a separate Node SQLite service supervised by
the Rust binary. Tauri remains the protected desktop client.

**Tech Stack:** Rust, Node 24, TypeScript, node:sqlite, existing Hermes REST API.

**Spec:** [Product daemon design](2026-10-19-product-daemon-hermes-design.md).
The human instructed inline execution on 2026-10-19 after discussing local/remote
selection. Preserve that authorization; no worktrees or publication.

**Delivery:** completed locally on 2026-10-19. See the
[verification record](../evidence/aylul/2026-10-19-product-daemon-hermes.md) and
[startup handoff](2026-10-19-local-assistant-handoff.md). Physical GUI and real
provider conversation through the new local path remain separate acceptance.

## Global constraints

- No Rust rewrite of the Hermes client and no protocol switch to ACP.
- Keep backend accounts/history, native login and existing Planner startup.
- A local assistant does not require a cloud account or Claude executable.
- Commit each tested task separately; stage explicit files in both repositories.
- No secret values in generated artifacts, webviews, logs or this document.
- No automatic retries/failover after uncertain writes or source changes.
- Generate local wire artifacts from one canonical schema.

## Review focus

Lost submission reply; restart with a prepared intent; revoked/stale source;
forged localhost requests; child-process death during an outstanding request.
Tasks below explicitly exercise these cases. A local fixture is not physical
desktop or real-provider acceptance.

## Task 1: Portable TypeScript client

Files: `packages/hermes-client/{package.json,package-lock.json,tsconfig.json,src/*,tests/*,contract.yaml}`;
backend `vendor/`, package manifests/Dockerfile, existing Hermes adapter forwarding
modules, its contract and code-map row. Record original backend commit provenance.

- [x] Watch the existing HTTP-fixture tests pass as the extraction baseline.
- [x] Add an artifact/exports test and observe failure before the package exists.
- [x] Move transport logic without behavior changes; build declarations and pack.
- [x] Install the immutable artifact into backend using Docker; keep old public
  imports as rule-free forwarding modules so callers do not fork behavior.
- [x] Run package tests plus Docker client/readiness/assistant/typecheck/contracts.
- [x] Commit each repository's completed extraction.

## Task 2: Durable local assistant service

Files: `services/assistant/{package*,tsconfig.json,src/*,tests/*,contract.yaml}`,
generated local wire and export command. Register actual source owners in the map.

Interfaces: a closed JSON request union for status/list/create/snapshot/send/stop/
recover; JSON-line input/output with one bounded response per request. Backend
0.5 projections remain UI-compatible. Only Node resolves the Hermes key-file ref.

- [x] Write fixture tests for persistence/reopen, one active turn, changed-input
  conflict, lost-ACK recovery under the same key, expiry refusal and bounded reads.
- [x] Observe RED, implement SQLite store plus service, observe GREEN.
- [x] Generate request schema; reject unknown fields and path escapes before I/O.
- [x] Test actual child stdio, malformed/oversized input, EOF and redacted errors.
- [x] Typecheck/build/test the package; update contracts and commit.

## Task 3: Rust daemon supervisor

Files: `crates/shadows/src/{daemon/*,cli/args.rs,lib.rs}`, Cargo manifests/lock,
daemon tests, `docs/codebase/README.md` and CLI handoff. Reuse `shadows-process`.

Interfaces: an explicit assistant-only CLI subcommand with operator-selected Node,
entry, config and protected descriptor paths. Authenticated loopback POST forwards
the closed request to the child; descriptor reports origin plus installation ID.

- [x] RED: assistant startup without Planner flags; missing/wrong capability and
  browser Origin refused; actual fixture conversation; child death is unavailable.
- [x] Implement bounded serialized stdio, tree containment and shutdown; no replay.
- [x] Keep key material in protected native runtime files, never stdout.
- [x] GREEN: focused supervisor/process/codemap tests and format/clippy.
- [x] Commit; combined Planner hosting remains a later composition extension if
  it requires changing the current AppCore contract beyond this first caller.

## Task 4: Native source selection and desktop conversation

Files: native assistant/local transport/commands/capability declarations; platform
assistant client; conversation hook; shell/settings/view; focused native/browser
tests and respective contracts/maps. Remote calls retain the existing auth owner.

Interfaces: native source metadata contains public source/installation identity
only. Explicit local/remote routing is captured per call. Local operation reads
the protected descriptor natively; remote operation uses the account session.

- [x] RED: signed-out local chat, switching source clears drafts/history, delayed
  old-source replies are ignored, companion cannot invoke local assistant calls.
- [x] Implement source selector and scoped controller without a second poller.
- [x] GREEN: native tests, frontend build and relevant browser fixtures.
- [x] Commit; no implicit history migration or raw upstream selectors.

## Task 5: Composed verification and handoff

Review ruling, 2026-10-19: the reviewer found the original-deadline recovery gap
also exists in the retained remote assistant service. Fix the same unsafe replay
condition in that owner rather than leave source selection with different safety
guarantees. Additional exact Touch in shadows-backend: one new ordered recovery
deadline migration, `src/platform/database/schema.ts`, Assistant service/store/
contract, its existing integration test and owner spec,
`src/platform/database/contract.yaml`, `tests/integration/migrations.test.ts`.
The migration test must prove legacy-data preservation plus ordered rollback,
including updated existing migration-count assertions. Do not change account
authority, public wire, provider configuration or existing conversation IDs.
Legacy uncertain intents without a proven deadline must refuse resubmission.
Require a RED/GREEN regression for an expired intent followed by a longer newly
advertised retention, plus focused Docker assistant and schema/contract checks.

Files: `scripts/` launcher/configuration examples without secrets, fixture journey,
dated evidence, stages and routing documentation. Amend old placement statements
to distinguish current implementation from previous checkpoints.

- [x] Start real daemon + Node against an owned Hermes HTTP fixture, send, stop,
  recover/restart and verify durable identities and no duplicate dispatch.
- [x] Probe installed Hermes readiness only without a model call; any actual
  provider/physical desktop acceptance is recorded separately if performed.
- [x] Run affected regression gates once, obtain one fresh review, fix important
  findings, record exact verification and commit. Do not push or deploy.
