# AYLUL — future list

What AYLUL is meant to do and does not do yet, one line each. It decides nothing and copies
nothing: each line points to the part of [the vision](./vision.md) that says why, and a feature
that gets a design points to its spec. Where the long-form vision names NYLRA, Hermes or a cloud
brain, [the current direction](./vision.md#current-direction-2026-10-19) replaces it: AYLUL's
brain is Claude Code run by Shadows Core. Shadows' own list is [`../future.md`](../future.md).

**Status:** `next` (chosen to start soon) · `planned` (has a spec or a plan) · `idea` (nothing
written beyond the vision). A feature leaves this list when it is delivered.

## Now and next

| Feature | What it gives Mohammed | Status | Why |
|---|---|---|---|
| Settings dialog: what is left | Usage per provider and model, plugins and skills per agent, General | planned | [settings spec](../frontend/specs/2026-10-20-settings-dialog-design.md) |
| Character colour on the companion | The character wears its agent's colour, repainted at once on save | planned | [settings spec](../frontend/specs/2026-10-20-settings-dialog-design.md), [native request](./shared/requests/2026-10-21-settings-dialog-native.md) |
| Start with Windows, hide to tray | Desktop switches in General | planned | [native request](./shared/requests/2026-10-21-settings-dialog-native.md) |
| Work profile | Watches the agents through hooks, explains their work, alerts | next | [current direction](./vision.md#current-direction-2026-10-19) |
| Personal profile | Email, YouTube and personal preferences | idea | [current direction](./vision.md#current-direction-2026-10-19) |
| Voice | Speech in and out, separate from the reasoning model, falls back to text | idea | [vision §14](./vision.md#14-voice-must-be-decoupled-from-reasoning) |

## Working with Shadows

| Feature | What it gives Mohammed | Status | Why |
|---|---|---|---|
| Liaison for the Planner | Explains a blocker with its sources, carries his direction back as a proposal | idea | [vision §4](./vision.md#4-the-core-vision), [§11](./vision.md#11-context-engine) |
| Decision Gateway | Approval bound to the exact proposal, confirmed in a trusted surface | idea | [vision §9](./vision.md#9-human-in-the-loop-decision-gateway), [§23](./vision.md#23-approval-integrity) |
| Relay while away | The Planner's question and his answer travel when he is not at the machine | idea | [current direction](./vision.md#current-direction-2026-10-19) |
| Answers with sources | "Why?" shows the plans, contracts and decisions behind a summary; fact vs. inference | idea | [vision §25](./vision.md#25-explainability-and-provenance), [§26](./vision.md#26-trust-and-confidence) |
| Decision replay | "What happens if I choose A?" simulated by the Planner without changing the plan | idea | [vision §42](./vision.md#42-replay-and-decision-simulation), [§43](./vision.md#43-counterfactual-explanations) |
| Decision feedback | Compares what a decision expected with what happened | idea | [vision §44](./vision.md#44-decision-quality-feedback) |

## Attention and channels

| Feature | What it gives Mohammed | Status | Why |
|---|---|---|---|
| Notification policy | Severity, quiet hours and impact decide whether to log, notify, speak or call | idea | [vision §16](./vision.md#16-notification-and-escalation-policy) |
| Batching and attention budget | Related events explained together; small decisions grouped | idea | [vision §35](./vision.md#35-smart-notification-batching), [§45](./vision.md#45-attention-budget) |
| Scheduled briefings | Morning and evening summaries, text or voice | idea | [vision §34](./vision.md#34-scheduled-briefings) |
| Remote alerts | Telegram or similar first, then a mobile app with Explain / Approve / Reject | idea | [current direction](./vision.md#current-direction-2026-10-19), [vision §17](./vision.md#17-mobile-experience) |
| Critical calls | A rare phone call whose first sentence says what, how urgent, and whether to decide | idea | [vision §36](./vision.md#36-critical-voice-calls) |
| Wearables | Watch notification with quick actions | idea | [vision §48](./vision.md#48-wearables) |

## The companion and the agents

| Feature | What it gives Mohammed | Status | Why |
|---|---|---|---|
| AYLUL's own MCP tools | Core gives each agent tools for AYLUL's screen: set the character's state, show a task list | idea | [current direction](./vision.md#current-direction-2026-10-19) |
| Glanceable desktop presence | Live agent status, quick approvals, drag-and-drop context, short spoken briefings | idea | [vision §18](./vision.md#18-desktop-companion) |
| Personal memory with controls | Preferences remembered, correctable and forgettable; never used as project truth | idea | [vision §10](./vision.md#10-two-kinds-of-memory), [§37](./vision.md#37-the-liaison-as-a-personal-representative) |
| Cost and token visibility | Model spend and token savings shown beside a decision | idea | [vision §12](./vision.md#12-token-efficiency-as-a-product-feature), [§46](./vision.md#46-cost-and-budget-awareness) |
| Delegated policies | Explicit, scoped, reversible permission to approve small things without him | idea | [vision §38](./vision.md#38-delegated-policies) |

## Beyond one machine

| Feature | What it gives Mohammed | Status | Why |
|---|---|---|---|
| Account server and devices | Sign-in, his devices, the same daemon locally and on the server | idea | [vision §20](./vision.md#20-pairing-model), [§21](./vision.md#21-secure-relay-architecture) |
| Teams | Roles and approval routing to the right person | idea | [vision §39](./vision.md#39-multi-user-and-team-mode) |
| Systems beyond Shadows | Adapters for CI, issue trackers and other agent systems | idea | [vision §41](./vision.md#41-integration-beyond-shadows) |
| Production companion | Explains incidents from linked AWS environments and follows up after the action | idea | [vision §46](./vision.md#46-cost-and-budget-awareness) |

## Adding to this list

A new idea goes first into [the vision](./vision.md), then gets one row here. A row changes
status in place; a delivered feature is removed.
