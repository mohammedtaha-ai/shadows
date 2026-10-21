# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

Branch `context-meter`: the composer's Context ring shows the size of the
conversation, not the last model call (Mohammed's ruling, 2026-10-21;
[plan](plans/2026-10-21-context-meter.md), owner spec harness controls §12.8).
B-1 is a probe and changes nothing the frontend sees. B-2 waits for B-1 and for
a shared request to be `accepted`.
Live keys: only `OPENROUTER` and `API_KEY_MINIMAX`, in `.env`. Read only the
variable names, never the values, and never print, log or commit a key.

The settings dialog's backend tasks were delivered by PR #40 (merged at `1a2b601`).

## Tasks

### B-1 · Probe: `/context` as the conversation's size, and live turn tokens
- Status: todo
- Plan: docs/backend/plans/2026-10-21-context-meter.md#b-1--probe-context-as-the-conversations-size-and-live-turn-tokens
- Read: docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.8; docs/evidence/2026-10-21-acp-usage-probe.md; the B-6 probe at `0bf351f` (`git show 0bf351f:crates/shadows-agent/src/acp.rs`, `b6_live_usage_probe`); `Sessions::context` in crates/shadows-core/src/harness/context.rs; `parse` in crates/shadows-agent/src/breakdown.rs
- Touch: crates/shadows-agent/src/acp.rs (probe commit only, reverted in the evidence commit), docs/evidence/2026-10-21-context-meter-probe.md (new), docs/backend/stages.md (this task's Status only)
- Needs: nothing. Uses `OPENROUTER` and `API_KEY_MINIMAX` from `.env`, loaded into the one cargo command's environment. Build only `-p shadows-agent`: `target/debug/shadows.exe` is locked by Mohammed's running Core, so stop and report if a build needs it. Do not stop `shadows.exe` or `nylra-desktop.exe`.
- Done when: `set -a; . ./.env; set +a; cargo test -p shadows-agent --lib b1_context_meter_probe -- --ignored --nocapture` has run every step of the plan. The evidence file records the raw figures and answers Q1-Q4, with no key, prompt text or generated text except step 6's number, and names the probe commit. After the evidence commit, `git diff 1a2b601 -- crates/shadows-agent/src/acp.rs` is empty.

### B-2 · The ring shows the conversation's size
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-context-meter.md#b-2--the-ring-shows-the-conversations-size
- Read: §12.8 as amended after B-1; crates/shadows-core/src/harness/contract.yaml; crates/shadows-core/src/turns/contract.yaml
- Touch: written when B-1 closes
- Needs: B-1 done, its answer supporting option A, and §12.8 amended; a shared request for `context_source` and stored categories, `accepted`
- Done when: written when B-1 closes

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
