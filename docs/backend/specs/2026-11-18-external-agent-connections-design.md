# Global external agent connections

Status — approved; built on `main` in PR #71 (`0e281585`). The live terminal and browser acceptance is Mohammed's.

An external Codex or Claude Code client connects once from Shadows Settings → External agents. At issue, the person selects one or more live projects. The token grants the existing external MCP capabilities for those projects. Future projects require an explicit grant. The Settings section owns project selection, Connect, setup instructions and Revoke. Project settings drop their External agents section.

## Scope and alternatives

A global screen could manage separate project tokens, but that would still require setting up each project. Giving every token all projects would remove the issuer's control over access. Use one token per connection, an explicit project allowlist at issue, and an explicit project argument on project operations. Discovery lists only live projects on that token's allowlist. A missing project or a project outside the allowlist is refused. A task, plan, agreement or draft reference must belong to the selected project when the operation requires same-project access. A draft reference is bound to the project where it was prepared.

## Authorization and cutover

Core Grants owns issue, list, bearer validation and revocation. Built: migration `0074_external_connections.sql` creates `external_connection` and `external_connection_project` and revokes every existing project-scoped grant. The connection's `project_list` tool returns its live projects; every other external tool takes a `project_id` argument and refuses `project`. Each allowed project has a scoped grant for attribution and transaction-local checks. The connection token selects one of those grants for each call. Scoped grants generated for a connection carry no usable standalone credential. Revoking a connection revokes all its scoped grants atomically. Existing plan and design services keep their transaction-local authority checks and validate target ownership. Planner and executor grant scopes keep their existing semantics. External connections retain the external tool inventory, with project discovery added. Approval and execution permissions remain with their existing principals.

The upgrade revokes project-scoped external credentials and retains the rows needed for history and authorship. An old token never gains global scope. The global UI lists global connections, and the project-specific issue and list routes are retired in the same cutover. Revoke stops future use and prevents writes still waiting to commit. Grant records contain token hashes. Issue responses display a token once, and command replays omit it.

## Setup and verification

Settings presents a client choice with copyable instructions for Claude Code and Codex. Both connect to the daemon's actual MCP endpoint. Codex reads the token from a named environment variable. The browser releases the issued secret when its setup view closes.

Automated checks cover migration of old credentials, token replay and revocation, a single token across two selected projects, denial for an unselected project, wrong and missing project arguments, draft reference isolation, external tool inventory, and Settings issue and revoke. The user owns live terminal and browser acceptance.
