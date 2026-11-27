# Global external agent connections

The initial global connection flow shipped in PR #71 (`0e281585`).
Named credentials and project edits extend that flow.

A client that supports MCP over HTTP connects once from Shadows Settings → External agents. At issue, the person names the credential and selects one or more live projects. The token grants the existing external MCP capabilities for those projects. The Settings section owns creation, project edits, setup instructions and Revoke.

## Scope and alternatives

Use one token per connection, an explicit project allowlist and an explicit project argument on project operations. Discovery lists only live projects on that token's allowlist. A missing project or a project outside the allowlist is refused. A task, plan, agreement or draft reference must belong to the selected project when the operation requires same-project access. A draft reference is bound to the project where it was prepared.

The person can replace an existing connection's project list, including clearing it.
The edit keeps the bearer and is guarded by the project revision from when the edit
opened. Removing a project revokes its scoped grant in the same write. Re-adding it
creates a fresh scoped grant. Unchanged projects retain their scoped grants. Revoked
connections cannot be edited. Existing credentials stay valid and display a fallback
label if they have no name.

## Authorization and cutover

Core Grants owns issue, list, bearer validation and revocation. Built: migration `0074_external_connections.sql` creates `external_connection` and `external_connection_project` and revokes every existing project-scoped grant. The connection's `project_list` tool returns its live projects; every other external tool takes a `project_id` argument and refuses `project`. Each allowed project has a scoped grant for attribution and transaction-local checks. The connection token selects one of those grants for each call. Scoped grants generated for a connection carry no usable standalone credential. Revoking a connection revokes all its scoped grants atomically. Existing plan and design services keep their transaction-local authority checks and validate target ownership. Planner and executor grant scopes keep their existing semantics. External connections retain the external tool inventory, with project discovery added. Approval and execution permissions remain with their existing principals.

The upgrade revokes project-scoped external credentials and retains the rows needed for history and authorship. An old token never gains global scope. The global UI lists global connections, and the project-specific issue and list routes are retired in the same cutover. Revoke stops future use and prevents writes still waiting to commit. Grant records contain token hashes. Issue responses display a token once, and command replays omit it.

## Setup and verification

Settings defaults to manual setup with server name `shadows`, the daemon's MCP HTTP
endpoint and the bearer credential. Optional Codex and Claude Code selections show
their registration commands using that same server name. Codex reads the token from
a named environment variable. The browser releases the issued secret when its setup
view closes.

Automated checks cover migration of old credentials, token replay and revocation, a single token across two selected projects, denial for an unselected project, wrong and missing project arguments, draft reference isolation, external tool inventory, and Settings issue and revoke. The user owns live terminal and browser acceptance.
