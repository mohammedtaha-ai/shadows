# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

No Core product implementation is in progress. AYLUL's brain will run on Core
inside the same daemon ([brain design](../aylul/specs/2026-10-19-aylul-claude-code-brain-design.md));
its plan comes next.
The independent service baseline is tracked in
[code-intelligence stages](../code-intelligence/stages.md), not as a Core task.
Existing AppCore remains the composition root for this runtime only.

## Tasks

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
