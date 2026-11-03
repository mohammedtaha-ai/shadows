# Per-agent MCP tool permissions

- Status: accepted
- Asked by: Mohammed, 2026-11-03; inline execution authorized in this chat.
- Screen: `docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md` §7.2.1.

## Need

The agent form lists a switched-on MCP item's tools and replaces that agent's rules.
Listing failures retain stored rules so the person can clear them.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/aylul/agents/{slug}/extensions/mcp/{id}/tools` | Agent slug and item id | `{tools: [{name, description}], list_error: string or null, rules: [{tool, rule}]}` |
| `PUT /api/aylul/agents/{slug}/extensions/mcp/{id}/tool-rules` | `{command_id, rules: [{tool, rule}]}` | Sorted stored rules; ask entries omitted |

## Errors

Missing agent or item returns 404; invalid rules return 400; changed command replay returns
409. Discovery failure returns 200 with a fixed value-free `list_error` and stored rules.

## Backend answer

As specified by the approved implementation plan, Tasks 1–4. Extensions owns both methods;
HTTP translates through AppCore. Contract: `crates/core-extensions/contract.yaml`.

## Frontend reply

Accepted under the user's instruction to execute the complete plan. Task 5 consumes the
generated API types and serializes all saves for one item's complete rule set.
