# Shared API Agreements — Stage 2 Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline. Mohammed
> authorized completing this stage, then stopping with a report. Preserve
> his instruction not to dispatch reviewers automatically.

**Goal:** Two plans bind exact versions of one HTTP agreement, review a
revision's actual impact, agree it without migrating either plan, then adopt
it explicitly in one Draft while retaining the other's old pin.

**Architecture:** Design owns agreement identities, immutable versions,
validation and impact review. Plans owns task bindings in version content.
Cross-service checks run in the existing serialized write transaction.
HTTP/MCP translate; Web shares the existing project event subscriptions.

**Tech Stack:** Existing SQLx/SQLite, axum/utoipa, React Query/Router.
`jsonschema` validates the official OAS 3.1 schema offline; serde_json is
the canonical content representation. No custom schema language or broker.

**Spec:** [§18.4–§18.12](../specs/2026-10-03-project-planning-workspace-design.md).
Baseline: `f45d490`. Stage 1 and 1b remain intact.

**Checkpoint, 2026-10-05:** Tasks overlap in the implemented end-to-end journey.
The [dated evidence](../../evidence/2026-10-05-shared-api-agreements.md) records
the completed gates, subsequent focused changes and isolated Windows trial.
Mohammed requested no further tests and a checkpoint commit. This plan remains
open: edits must identify the exact Draft version as well as its revision;
the stale-version regression and completion review are outstanding. Do not
start Stage 3 from this checkpoint.

## Global constraints

- Exact Agreed version pins; no latest-version substitution or automatic adoption.
- Same-project participants/bindings. Rename/move preserves stable IDs.
- One Draft per contract; nonblank reason after v1; expected revision on edits.
- Agreed content and Frozen bindings have storage guards, including direct SQL.
- Agreement is person HTTP only. MCP reads/proposes/edits under a live grant.
- New writes recheck live project/grant; replay returns the original result.
- Review identity covers content, declared part revisions, latest/historical
  pins, relevant plan states and revisions. Stale agreement is atomic conflict.
- Compatibility stays `Needs review`; execution stays `Not recorded`.
- Use migration 0017 onward, no unused placeholder migrations.
- Required complete Rust/Web gate before product commits; never touch live user DB.

## Review focus

1. Removed operation identity cannot be reused for a different operation.
2. Two starts/edits/approvals race: one Draft and no partial agreement.
3. Several consumers pin different old versions: compare each actual pin.
4. Replayed commands after later edits preserve their initial answer.
5. Invalid references, revoked grants and stale reviews write no content/event.

## Ownership and interface decisions

All names below are proposed new interfaces, not signatures already present.

- `design/agreement.rs`: public agreement values (`AgreementId`,
  `AgreementVersion`, `AgreementContent`, `AgreementParty`, `AgreementRole`).
- `design/agreement_validation.rs`: offline OAS validation, derived operation
  identities and JSON-pointer errors. Operations use UUID
  `x-shadows-operation-id`; copied versions preserve this extension.
- `design/agreements.rs`: Design's person/grant entry points.
- `design/store/agreements.rs`: snapshot version/list reads.
- `design/store/agreement_write.rs`: start/edit/freeze replay transactions.
- `design/store/agreement_review.rs`: participant snapshot and review identity.
- `plans/bindings.rs`, `plans/store/bindings.rs`: typed pins and persistence.
- `shadows-http/agreements.rs`: agreement route translation.
- `shadows-mcp/agreement_tools.rs`: grant-scoped proposal/read translation.
- `web/src/app/agreements/`: contract list/editor/review and task binding UI.

`AgreementContent` contains capability, purpose, behavior, acceptance,
declared parties and an OpenAPI JSON value. Drafts may be incomplete; reads
expose validation issues. Agreement requires zero issues and all providers.
`Design::agreement(project,id,version)` reads an exact version (omitted
version means the current one); `agreements(project)` lists identities.
Start/edit return an immutable `AgreementVersion` command result.
`AgreementReview` contains a service-derived review ID, structural changes,
declared parties and current/historical participants. Agree takes command ID,
agreement/version, expected revision and review ID.

## Task 1: Contract content and version lifecycle

- [ ] Write failing validation tests: valid login OAS; malformed response,
  missing/duplicate UUID identity, unresolved local/external reference,
  missing purpose/acceptance/provider and incomplete Draft diagnostics.
- [ ] Run `cargo test -p shadows-core --test agreement_validation`.
  Expected: FAIL before the public content validator exists.
- [ ] Implement library-backed validation and canonical values; retain
  schemas/examples without a lossy DTO round trip. Reject external retrieval.
- [ ] Write lifecycle tests for start/edit, one-Draft races, whole-result
  replay, conflict, foreign part and direct Agreed storage mutation guards.
- [ ] Add migration 0017, Design methods, HTTP read/start/edit routes and a
  browser Contracts entry/editor with a JSON view of the same content.
- [ ] Run lifecycle/route/UI tests; expected all PASS. Update contracts,
  generated API/types and code map; complete gate and scoped commit.

## Task 2: Exact plan bindings

- [ ] Write failing tests binding provider/consumer tasks to Agreed v1;
  wrong part/role/operation, Draft target and foreign project refuse atomically.
- [ ] Add `AgreementBinding {task, agreement_id, version, part_id, role,
  operations}`. `PlanOp::BindingPut` replaces that task/agreement/role pin;
  `BindingRemove` removes it. Task removal removes its bindings in the batch.
- [ ] Persist/copy pins with plan content; absent old bindings serialize as
  absent so old canonical digests remain compatible. Frozen guards cover
  insert/update/delete. Revalidate before plan approval.
- [ ] Expose task bindings and part-plan associations including derived
  bindings, distinguishing historical from current references.
- [ ] Run `cargo test -p shadows-core --test agreement_bindings`; expected
  PASS for edits, copy/restart, archive refusal and unchanged Frozen bytes.
  Update service contracts; complete gate and commit.

## Task 3: Impact review and person agreement

- [ ] Write failing tests for per-pin structural differences, shared behavior,
  declared unbound parties and latest versus historical bindings.
- [ ] Implement snapshot review and review identity; compare each pin's
  version, never assume all consumers use v1. Sort participant/diff output.
- [ ] Inside agreement's write transaction, validate content and recompute
  identity before freezing; stale content/part/pin/state refuses with 409.
- [ ] Add person review/agree HTTP routes. Agree changes no plan bindings.
  Journal agreement events in the same write; replay journals nothing new.
- [ ] Run `cargo test -p shadows-core --test agreement_review`; expected
  PASS including competing approvals, stale basis and old successful replay.
  Complete gate and commit.

## Task 4: Agent proposals and Web adoption

- [ ] Write failing real MCP tests for grant-scoped list/get/start/edit/review,
  revoked writes and no agreement approval tool.
- [ ] Add MCP tools using the same Design methods/policy; record grant writer
  and Planner provenance when available. No authority in adapters.
- [ ] Complete structured operation editor (method/path, parameters/body,
  responses/errors/security), version picker, party selection and review UI.
- [ ] Add task binding editor: choose exact Agreed version/role/operations.
  Frozen plans continue to a Draft before adoption; archived plans refuse.
- [ ] Test review conflict retaining input, adopting only one plan, old pins
  readable, Arabic content and UI execution/compatibility labels.
- [ ] Extend project SSE invalidation/reconnect for agreement/participant
  queries, without polling. Complete gate and commit.

## Task 5: Integrated Windows acceptance and report

- [ ] On an isolated DB, run the complete two-plan login journey: agree v1,
  bind both plans, freeze, propose v2, inspect both impacts, agree without
  repinning, continue/adopt only one plan, inspect old Frozen v1.
- [ ] Demonstrate stale review refusal, second-tab notification, daemon
  restart, persisted versions/pins and actionable validation errors.
- [ ] Run complete Rust/Web gate, Unicode width and generated API checks.
  Record dated evidence with actual counts and untested limitations.
- [ ] Commit; stop our temporary processes and report. Do not begin Stage 3.

## Pre-flight

Task 1 → Tasks 2–4: UUID operation IDs, exact version API and offline validation.
Task 2 → Task 3: snapshot bindings plus plan revisions/states enter review digest.
Task 3 → Task 4: only person approval; agent review is read-only.
Tasks 1–4 → Task 5: generated routes/types and project SSE events must match.
No conflicting ownership found; Design owns versions, Plans owns adoption.
