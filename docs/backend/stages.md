# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

No Core product implementation is in progress. The new AYLUL integration has
its own [plan](../aylul/plans/2026-10-18-aylul-hermes-semantic-context.md).
The independent service baseline is tracked in
[code-intelligence stages](../code-intelligence/stages.md), not as a Core task.
Existing AppCore remains the composition root for this runtime only.

## Tasks

### B-A2 · Prepare Core's scoped context contract
- Status: blocked
- Plan: docs/aylul/plans/2026-10-18-aylul-hermes-semantic-context.md#t3---expose-the-core-context-promised-by-its-owner-spec
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
