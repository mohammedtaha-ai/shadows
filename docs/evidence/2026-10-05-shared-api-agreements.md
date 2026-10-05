# Shared API agreements — implementation checkpoint

Date: 2026-10-05. Branch: `codex/full-project-vision`, above `f45d490`.
This saves Stage 2 work; it does not establish Stage 2 completion.
Mohammed requested stopping further tests and committing the current work.

## Delivered in this checkpoint

Design owns agreement identities, Draft revisions, immutable Agreed versions,
offline OpenAPI 3.1 validation, participant impact review and person approval.
Plans owns exact version bindings, explicit adoption and Frozen storage guards.
HTTP and grant-scoped MCP expose those services; MCP cannot approve agreements.
The browser has Contracts, operation editing, impact review, version browsing,
task adoption and project-event invalidation. Migrations 0017/0018 add storage.

## Executed verification

- Complete Rust gate: format, both clippy modes, workspace tests and production
  build passed; **411 tests passed, one ignored**. Production feature graph
  contained no `test-support`. Unicode width check passed.
- Complete Web gate: typecheck, lint, **188 tests** and production build passed.
  The existing bundle-size warning remains.
- Subsequently, the strengthened lifecycle test for competing Draft starts
  and revoked grant writes/replays passed. Final UI operation editing and
  readable binding labels passed typecheck, lint and production preview build.
  The complete suites were not repeated after those final edits.
- Focused real HTTP/MCP and integrated two-plan tests passed, including stale
  review refusal, per-pinned-version impact and retained Frozen bindings.

## Native Windows trial

An isolated daemon on port 4319 and production Web preview on 5174 exercised
real HTTP/MCP calls and an agent-operated browser. No model turn was submitted.

The browser agreed v1; both plans pinned v1. MCP proposed v2. Changing a
participant after review produced the expected HTTP 409 refusal. A fresh review
allowed agreement of v2 without repinning either plan. The browser explicitly
adopted v2 in a continued frontend Draft. Readback showed backend pin 1,
old frontend pin 1 and continued frontend pin 2. After daemon restart the
same readback passed, and the browser opened immutable Agreed v1 successfully.
The fresh post-restart browser reported zero errors and warnings; the earlier
deliberate stale-review refusal generated an expected network error.

An online backup of the actual dev database was migrated on a separate daemon
on port 4320, from migration 15 to 18. It retained four projects, one Frozen
version and 12 Frozen tasks, with zero foreign-key violations. A digest of
Frozen workflow identity/title/goal/version/revision and task identity,
workflow/number/contract/scope matched before and after. This is a bounded
content comparison, not proof of byte-for-byte database equality. The original
database was not changed.

Raw gate logs, browser snapshots and private database copies remain under
`.superpowers/sdd/2026-10-04-shared-api-agreements/`; they are not committed.
No independent review, human acceptance or Linux run is claimed.

## Remaining implementation gate

Static source inspection found that `edit_agreement` accepts only an expected
revision and loads the latest version. Each new Draft begins at revision zero.
A delayed edit for v1/rev0 can therefore match v2/rev0 and overwrite the newer
Draft. The frozen older version remains protected, but the edit target is
ambiguous. No regression test or fix for this finding has been executed.

Next authorized work must carry the exact Draft version with its revision
through core, HTTP, MCP, Web and command fingerprint, then verify refusal of
that stale request. Stage 3 has not started.
