# Desktop UI stages

Where this side's work is now. `desktop-ui-lead` keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

2026-10-19 new direction: the human requested a multi-bot workspace inspired by
Hermes Desktop's bots/chats/settings and selected one visible character for the
selected bot. The [written proposal](specs/2026-10-19-agent-workspace-design.md)
is ready for review. No workspace/bot-registry implementation has started; normal
chat below remains delivered. Backend management capabilities need their own proof.

Normal-chat H-2 is implemented. On 2026-10-18 the human selected
AYLUL as the primary UI with the existing Shadows workspace loaded on request.
The [integration draft](../plans/2026-10-18-aylul-hermes-semantic-context.md)
and [transition plan](../plans/2026-10-18-nylra-transition.md)
record that direction. The earlier backend chat/provider work remains its own
implemented owner. The human subsequently requested normal UI chat now, deferring
MCP/Core. The new assistant facade uses bounded snapshot polling.
Character design/renderer and native companion lifetime keep their separate owners.

## Tasks

### U-B1 · Review the agent-workspace design
- Status: design review
- Read: docs/aylul/ui/specs/2026-10-19-agent-workspace-design.md; current assistant/platform/companion contracts.
- Touch: this proposal and its existing vision/navigation/stage references only.
- Needs: human review of the written design before implementation planning.
- Done when: layout, bot/configuration ownership, staged feature inventory and selected-character semantics are accepted; then write an exact implementation plan.

### H-2 · Present one account-backend conversation
- Status: implemented and locally verified; physical desktop chat acceptance open
- Plan: docs/aylul/native/plans/2026-10-18-aylul-local-runtime-hermes.md#h-2--one-main-window-backend-conversation
- Read: generated backend assistant contract and local/native shared request once accepted.
- Touch: H-2 feature/platform/test files and their actual contracts only.
- Needs: account-scoped backend facade and native authenticated HTTP (satisfied); no direct Hermes credentials.
- Done when: selected conversation, reply/recovery and stop states work with local monitoring and optional character preserved.

See [normal-chat evidence](../../evidence/aylul/2026-10-19-normal-chat.md) for the
real backend/MiniMax browser journey, separate native HTTP checks and explicit
remaining physical GUI gate. No avatar redesign, code bridge or Core change.
