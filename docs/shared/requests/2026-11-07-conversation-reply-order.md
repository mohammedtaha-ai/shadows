# Project conversation reply order

- Status: accepted
- Requested by: Mohammed, 2026-11-07, in this implementation chat
- Screen: docs/core/specs/2026-09-25-planner-workflow-design.md#1311-web-client

## Need

Project conversations appear in descending order of their latest received assistant
reply. Sending a user message, renaming a conversation or recording tool activity
does not promote it. This is the user's corrected and authorized requirement.

## Fields

`GET /api/projects/{id}/threads` keeps its existing response shape. Threads owns
the ordering using the durable sequence of the latest `AgentMessage` append.
Conversations with no assistant reply follow those with replies; creation sequence
descending, then id, resolves their order. No timestamps or per-thread fetches are
added to the client. Agent-profile conversation lists retain their existing policy.

## Errors

Existing errors are unchanged.

## Backend answer

Supported by `Threads::list`, through `crates/core-threads/contract.yaml`, without
a database migration or response schema change. Durable event sequence gives
arrival order independently of timestamp text.

## Frontend reply

Display the returned order. Refresh thread lists when the open thread receives
an `AgentMessage` append; the existing ten-second polling covers background
conversations whose thread stream is not open. Manual browser acceptance belongs
to Mohammed.
