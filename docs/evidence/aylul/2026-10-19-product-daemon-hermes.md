# Product daemon and local Hermes assistant

Date: 2026-10-19. Scope: local implementation and retained remote adapter in
Windows development. No production, physical GUI or paid model acceptance.

## Delivered source

| Commit | Result |
| --- | --- |
| `681c2dd` | Portable TypeScript Hermes transport extracted from backend `4dc5280` |
| `4a61199` | Node SQLite assistant application with closed stdio operations |
| `93e6497` | Rust product daemon supervision with optional existing Planner |
| `7e22868` | Native local/server source selection |
| `ea337b0` | Persist original recovery guarantee; conservative legacy migration |
| `71b9866` | Captured remote account/session authority and local source generations |
| `e4185a8` | Atomic private connection descriptor, Windows owner-only DACL |
| `dd6634c` | Composed restart/recovery/stop journey and Windows launcher |

Backend `81c6cb0` consumes an immutable npm archive with lockfile integrity.
The source/declarations inside match the canonical client. Docker image
`shadows-backend-local:dev` built from its own checkout and loaded the package
without mounting Shadows. Backend `846c60e` adds migration 012 and preserves
verified original recovery deadlines. No live account database was migrated;
disposable test databases exercised upgrade and rollback.

## Observed checks

- Portable package: `npm test`, 9/9 including original native HTTP fixtures.
- Local assistant: `npm test`, 9/9 covering persistent identity/history, lost ACK,
  explicit recovery, original retention, prepared-intent restart, conservative
  v1 upgrade, stop acknowledgement/restart and explicit turn ordinal ordering.
- Root focused tests: 5/5 for assistant-only CLI, private descriptor replacement,
  authenticated native listener, browser-origin refusal, child death and code map.
- Root library Clippy passed with warnings denied after review fixes.
- Native desktop library: 34 passed, 2 explicitly ignored live-account cases.
- Frontend production build passed. Root desktop Node tests 7/7; companion unit
  tests 15/15; production graph test 1/1.
- Complete Chromium suite: 65/65, including local signed-out chat and stale replies
  after local-to-remote-to-local selection. IPC is an explicit synthetic boundary.
- Backend final Docker typecheck/contracts passed. Assistant/migration/native
  client/readiness tests 32/32; generated auth/NYLRA wire checks 12/12; docs and
  secrets checks passed. Earlier focused architecture/native tests passed 28/28.
- Native debug executable rebuilt successfully; it was not launched by automation.

The actual composed `scripts/assistant-daemon-journey.mjs` passed through a real
Rust daemon and Node process to an owned Hermes HTTP fixture. It restarted the
daemon, recovered a lost admission reply without duplicate execution, and
observed stopping before cancellation. Two intended messages caused exactly two
executions; model calls: zero.

The launcher ran against synthetic configuration in both PowerShell Core and
Windows PowerShell 5.1, including paths containing spaces. No GUI or model call
was involved. The test used process-only execution policy settings, not a machine
policy change. The temporary launcher probe was removed.

The installed Hermes readiness probe first returned unavailable; the existing
backend host helper then started the isolated chat home from the Windows
installation. Both the local Node service and actual running local product daemon
returned ready against that installation. The authenticated local daemon response
was HTTP 200. No provider prompt or personal credential value was projected.

## Review and environment

One fresh reviewer performed the branch review, fixing original-retention replay,
descriptor privacy, remote account capture and stale source-generation defects.
The same reviewer received one targeted fix continuation for the identical remote
retention defect; this was not a second whole-branch review.

The first broad Rust linking attempt exhausted drive E. Automatic review rejected
direct removal of build symbols, so Cargo's own scoped package clean was used.
Only generated build outputs were removed. A later run used
`--config profile.test.debug=0` to avoid large test PDBs. The inherited
`RUST_LOG=warn` hid an existing debug-log test's expected lines; clearing it only
in the test process made that test pass. An existing concurrent plan-show SSE test
failed once, then its full target passed 8/8 with test threads serialized. Final
broad-gate results are recorded below rather than treating these attempts as green.

The first fresh Docker build hit registry.npmjs.org EIDLETIMEOUT; a later identical
build succeeded. No test deadline or assertion was weakened for these conditions.

## Final broad gate

`cargo test --workspace --no-fail-fast --config profile.test.debug=0 --jobs 2 --
--test-threads=1`, with inherited `RUST_LOG` cleared only for this process,
finished with exit 0: **534 passed, 0 failed, 1 ignored**. No source/test assertion
was changed to obtain that result. Format passed. Workspace all-target Clippy
with `fake-acp/test-support` and warnings denied passed. Generated Core `api/`
remained unchanged. The local request schema regenerated with no diff; eight
affected source contracts parsed successfully.

Final native debug build succeeded. The running local product daemon returned
HTTP 200 / ready against installed Hermes using the protected per-user descriptor.
The backend checkout is clean at `8a026c3`, after its retained-route recovery fix.
Source and delivery documents are committed independently; no push or PR is
included in this task.

Raw logs remain git-ignored in `.superpowers/sdd/product-daemon-hermes/` and
backend `.local-runtime/`. This dated record contains the observed results, not
unrestricted application logs or runtime credentials.

## Remaining acceptance

The local daemon is loopback-only. Server deployment through a new authenticated
backend-to-daemon adapter remains future work; current remote account chat still
uses its retained backend-to-Hermes path. The same runtime can run on another
host, but no server rollout was performed.

Hook-sidecar lifetime remains under Tauri. No bot CRUD, model configuration UI,
semantic project bridge, computer control, history migration or public streaming
was delivered by this move. Physical desktop rendering/sign-in and an actual
provider conversation through the new local path remain person-operated acceptance.
Startup instructions: [local handoff](../../integration/2026-10-19-local-assistant-handoff.md).
