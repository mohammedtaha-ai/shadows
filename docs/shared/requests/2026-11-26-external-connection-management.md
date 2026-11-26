# Named external connections

Status: accepted

## Need

Settings creates a named MCP credential and edits its allowed projects in place.

## Fields and errors

- `POST /api/external-connections` accepts a human-readable `name`. Older clients may
  omit it. Creation still requires at least one project.
- Connection responses add nullable `name` (null for unnamed credentials) and numeric
  `project_revision`, starting at zero.
- `PATCH /api/external-connections/{id}` accepts `command_id`,
  `expected_revision` and the complete `project_ids` list, which may be empty.
  It returns the current connection without credential material.
- An unknown connection or unavailable project returns 404. A stale revision,
  revoked connection or conflicting command replay returns 409. A blank supplied
  name returns 422.

## Backend answer

Accepted. Grants owns the edit, preserves the bearer and revokes removed scopes
in the edit transaction. A re-added project gets a fresh scoped grant.

## Frontend reply

Accepted. The form sends its starting revision and displays the saved connection.
It reloads server state on a conflict so the user can reopen the editor.
