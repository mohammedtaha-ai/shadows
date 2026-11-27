# External connection management implementation plan

Implement inline on `feature/after-77`. Each task ends in a scoped commit.

**Spec** [External agent connections](../specs/2026-11-18-external-agent-connections-design.md)
and the [accepted HTTP request](../../shared/requests/2026-11-26-external-connection-management.md).

## B-236 · Name credentials and replace their project scope

- [x] Extend the Grants HTTP integration test to cover named issuance and the same bearer
  after adding, removing and re-adding projects. Pin down stale saves and command replay.
- [x] Append a Core DB migration for a nullable name and project revision. Preserve old
  rows and old nameless issue fingerprints. Own input normalization in Grants.
- [x] Implement an atomic scope replacement in the Grants store. Use existing grant
  issue/revocation and project liveness checks. Keep unchanged scope IDs and issue a fresh
  ID on re-add so in-flight writes cannot regain their old authority. Allow an empty list.
- [x] Expose PATCH through AppCore, register the OpenAPI route and regenerate the artifact.
  Update owner contracts and run focused Grants, migration and OpenAPI checks.
- [x] Commit the backend task.

Verification: 32 tests passed across Core, Grants, MCP transport and OpenAPI. The MCP
test used an existing connection to add one project and remove another, then called
tools with the same authenticated client.

## F-149 · Edit permissions and show portable setup

- [x] Regenerate the TypeScript API types. Add the name and PATCH calls to the existing
  external connection client.
- [x] Extend the existing Settings interaction test for named creation, default manual
  setup, both client commands, editing and revoking by connection.
- [x] Show a copyable MCP URL and bearer token in General setup. Keep Codex and Claude
  Code shortcuts optional. Use `shadows` as the registration name. Display an informative
  fallback for unnamed connections.
- [x] Add a project editor per live connection with Save and Cancel. Use the revision
  captured when opening it, and refresh after a stale save. Update the UI contracts.
- [x] Run the focused Settings test, typecheck, lint, build and frontend architecture checks.
- [x] Commit the frontend task.

Verification: 6 Settings and cache tests, 82 frontend architecture and code-map
tests, typecheck, lint and the production build passed.
