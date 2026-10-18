# Combine existing local hooks with the independent companion

- Status: accepted
- Asked by: human coordinating supervisor, 2026-10-17
- Owner: Native and Desktop UI integration
- Serves: the existing hook implementation and next relay planning brief

## Need

The human explicitly requested merging their and Claude's
`spec/aylul-hook-laptop` work into `codex/aylul-laptop-relay`, so subsequent
development uses one combined implementation. This authorizes preserving and
reconciling the existing local interfaces; it does not approve the still-planned
cloud consumer or install hooks into personal agent settings.

## Interface

| Existing interface | Resolution |
| --- | --- |
| `aylul_call` | Register the existing command and generated permissions; grant only the main capability. Existing sidecar owns method validation and outcomes. |
| Main presentation | Mount existing SessionsPanel/ApprovalsPanel alongside account actions, native companion controls and the development-only studio. |
| Native composition | Manage both companion state and the sidecar; retain the sidecar's existing stop-on-exit behavior. |
| Companion | Retain its independent entry and restricted capability; do not grant local approval/account methods. |

## Errors

Missing sidecar state uses the existing unavailable presentation while account
and companion controls remain usable. Other sidecar/approval/settings outcomes
retain their existing owners and stable codes. This merge changes no cloud wire
contract and creates no second local hook, store or approval authority.

## Answer

The coordinating implementation combines the existing owners, resolves the five
merge conflicts, registers the sidecar command in the current ACL system, and
verifies the result with isolated hook/sidecar tests and merged-shell checks.

## Reply

The human's direct merge request is the acceptance evidence. Local verification
and its limits are recorded in
[merge evidence](../../../evidence/nylra/2026-10-17-hook-companion-merge.md).
