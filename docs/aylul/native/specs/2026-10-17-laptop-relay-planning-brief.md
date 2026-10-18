# Desktop laptop relay - planning brief

Date: 2026-10-17.
Status: discussion brief; local hook merge authorized, cloud-consumer design and task plan are not approved.

2026-10-18 disposition: historical/deferred client direction. The human stopped
continuation of this backend-compatible path after selecting the new AYLUL/Hermes
architecture. Preserve completed work and its evidence; do not finish this brief's
unfinished consumer tasks. See the [transition plan](../../plans/2026-10-18-nylra-transition.md).

Priority update: the human selected direct account-backed text conversation
through BYOK providers as the next slice, before cloud hook relay. Its
[backend-owned written design](../../../../../shadows-backend/docs/backend/specs/2026-10-17-account-chat-and-providers-design.md)
records the subsequent conversation. The human explicitly clarified that current
preparation is backend-only. This relay brief remains deferred client context;
it is not the active implementation plan and owns no backend chat decisions.

## Intent and workflow

The combined hook/companion work was merged into `main` through
[PR #13](https://github.com/mohammedtaha-ai/nylra/pull/13) at `0d26ece`, with
GitHub desktop CI passing. Following the human's merge confirmation, the local
`main` was fast-forwarded, the merged local branches and old remote PR branch
were deleted, and `codex/aylul-relay-client` was created from that updated main.
Continue planning there in the original checkout, without worktrees. Keep a
separate commit after each completed, verified task. Plan the next slice together
before implementation; this brief records inspected starting points and choices,
not a new wire contract. The integrated hook history is recorded below.

The intended next outcome is a desktop sidecar that uses the backend's generated
relay contract and resumes safely after a disconnect. Which agent is included in
the first end-user journey is the first discussion decision below.

## Inspected starting points

| Owner | Current evidence |
| --- | --- |
| NYLRA receiving baseline | `93d787f` before planning, then planning commit `cb89a1a`; independent companion and native authentication are present. |
| Integrated Native baseline | Human-authorized merge of local `spec/aylul-hook-laptop` at `0d086b6`: hook executable, Node sidecar, SQLite store, local session/step projection, approval/grant handling and Tauri lifecycle, alongside the independent companion. |
| Backend | `codex/aylul-laptop-relay` at `7d99ab3`: implemented pairing, fenced WSS ingress, durable acknowledgements and HTTP/SSE. Dated backend evidence records 88 passing Docker tests and the actual synthetic Compose relay journey. |
| Local event capture | Baseline `hooks/route.ts` saves session and step separately. Relay enqueueing needs an explicit atomic boundary; a crash must not silently lose one half. |
| Local content | Baseline `sessions/steps.ts` can put shell commands in `target` and `summary`; `Session.lastMessage` can hold assistant text. Serializing these local records wholesale is unsuitable for the reviewed cloud projection. |
| Missing Native path | No cloud `link/` module or durable outgoing relay/checkpoint queue exists in the inspected baseline. Current native auth exposes a credential-free view; it has no sidecar credential handoff. |

Sources: [current code map](../../codebase/README.md),
[source placement](../../source-structure.md),
[accepted backend request](../../../../../shadows-backend/docs/shared/requests/2026-10-17-laptop-relay.md),
[backend acceptance](../../../../../shadows-backend/docs/evidence/2026-10-17-laptop-relay-acceptance.md).
Initial observations came from `git show spec/aylul-hook-laptop:<path>`.
The human then requested integration into the current checkout; see
[merge evidence](../../../evidence/nylra/2026-10-17-hook-companion-merge.md) and the
[accepted integration request](../../shared/requests/2026-10-17-hook-companion-merge.md).

## Recommended direction for discussion

1. Build on the combined Native/companion baseline, preserving current auth and
   local approval behavior. Reuse one hook and one local SQLite owner.
2. Pin generated HTTP and frame artifacts with provenance/freshness checks;
   consumers never import backend source or handwrite endpoint schemas.
3. Project only the reviewed session/step fields. Keep raw hook bodies, commands,
   local summaries, assistant messages and approval authority outside this relay.
4. Persist the outgoing sequence, producer identity and acknowledgement checkpoint
   with the existing SQLite owner. Specify transaction, bounds, overflow, expiry,
   replay and account/device/origin-switch behavior before writing the sender.
5. Keep credentials under native auth ownership; pass a validated session only
   over owned process IPC. Expose connection/pairing state to the UI without
   exposing credentials, and stop delivery on surrender or refused authority.
6. Verify the real Windows process boundary against Docker using an isolated
   synthetic data directory, including disconnect/restart, replay and revocation.

The human chose a normal merge preserving the joint hook history, with explicit
reconciliation of its UI/CI and the current companion/auth/ACL changes. The
remaining cloud consumer extends those integrated owners; another hook/store
would duplicate their responsibility.

## Decisions to settle together

- First agent: start with the existing Claude Code hook path, or require Codex
  capture in the first slice as well? Recommendation: Claude Code first; a real
  Codex adapter needs its own inspected capture interface and acceptance evidence.
- Minimal product presentation: where pairing and connection status belong;
  agree the Native/Desktop UI interface before either side changes source.
- Credential and outbox lifecycle: exact stop/resume/reset behavior on logout,
  reauthentication, account change, unlink, overflow and missing replay history.
- Data scope: synthetic integration is currently allowed. Real local history and
  identity content remain behind the backend's separate lifecycle acceptance;
  do not enable upload of an existing personal database implicitly.

Each unresolved decision closes through the agreed Native design and relevant
shared request before its implementation task. An API mismatch goes back to the
backend's canonical code, never a locally invented wire variant.

## Planning output and delivery discipline

After agreeing the scope, write the Native design under `docs/aylul/native/specs/`,
then the concrete file-scoped implementation tasks under `docs/aylul/native/plans/`
and register them in `docs/aylul/native/stages.md`. Each task names its checks and
ends with an explicit-path commit after verification. A broken or incomplete
task is not reported as complete merely because it has a commit.

No model selection, paid provider calls, brain/voice implementation, mobile app,
actual agent-settings installation, real-history upload, publication or merge
is performed by this planning task. MiniMax remains a trial provider.
