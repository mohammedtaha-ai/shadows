# Desktop UI stages

Where this side's work is now. `desktop-ui-lead` keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

No desktop implementation task is in progress. On 2026-10-18 the human selected
AYLUL as the primary UI with the existing Shadows workspace loaded on request.
The [integration draft](../plans/2026-10-18-aylul-hermes-semantic-context.md)
and [transition plan](../plans/2026-10-18-nylra-transition.md)
record that direction. The earlier backend chat/provider work remains its own
implemented owner; it does not activate a desktop chat task automatically.
Character design/renderer and native companion lifetime keep their separate owners.

## Tasks

### H-2 · Present one account-backend conversation
- Status: blocked on backend/native contract and H-1
- Plan: docs/aylul/native/plans/2026-10-18-aylul-local-runtime-hermes.md#h-2--one-main-window-backend-conversation
- Read: generated backend assistant contract and local/native shared request once accepted.
- Touch: H-2 feature/platform/test files and their actual contracts only.
- Needs: account-scoped backend facade and native handoff; no direct Hermes credentials.
- Done when: selected conversation, reply/recovery and stop states work with local monitoring and optional character preserved.
