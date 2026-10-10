# Section 24 — Who Approved: Principal and Agent

- **Date:** 2026-10-08.
- **Status:** Accepted by Mohammed in conversation on 2026-10-08; awaits his
  review of this file. Enforcement is planned for §23.9 PR 3b, before §23's
  structure write (PR 4), after the real-adapter probe in §24.3.
- **Evidence:** the earlier `shadow`'s
  `crates/domain/shadow-actor/src/lib.rs` (two fields per write) and
  `crates/runtime/shadow-core/src/authority.rs` (the core as the boundary),
  with its design
  `docs/superpowers/specs/2026-09-10-shadow-backend-authentication-and-bootstrap-design.md`,
  whose threat table states that a process running as the user was not
  stopped. Read, not run.
- **Related owners:** §13.2 (approval is the person's; its OPEN), §13.6
  (Shadows' MCP server), §13.7 (grants), §13.8 (the Planner's session),
  §1 (the request guard), §17 (executors), §23 (proposals and the structure).

This section owns how Shadows records who approved something and enforces
that a person's approval cannot be forged. It closes §13.2's OPEN and §23.7's
for the Planner.

## 24.1 Two fields, not one

Every approval, and every write that §23 gates, records two things:

| Field | Answers | Values |
|---|---|---|
| **principal** | On whose behalf | The local person (`user:local`); one person until teams exist |
| **agent** | What performed it | `human` (the person in the web client), `planner:<harness>`, `external:<grant>`, later `executor:<harness>` |

One field cannot tell "the person decided this" from "a model did it during
the person's session", and that is a safety distinction. Today's `Writer`
(`Person`, `Planner`, `External`) is the agent; the principal is added beside
it, in the command record and on every approval.

The agent comes from the connection, never from a request's arguments: an
MCP call is the grant's holder, whatever its arguments say; an HTTP write is
`human` only on the web client's routes (§24.3).

## 24.2 Three kinds of approval

| Kind | Recorded as | Shown |
|---|---|---|
| The person clicks **Approve** | principal = person, agent = `human` | "Approved by you" |
| The person asks the Planner in the conversation, and it approves with a tool | principal = person, agent = `planner:<harness>`, with the person's message of that turn | "Approved by the Planner at your request", linked to the message |
| The Planner approves without being asked | the same record; the linked message shows no such request | The same line; the person reads the message and judges |

So the Planner gets approval tools — one per thing it proposes (§23.5) and
for plans (§13.2) — and every approval through them is recorded in its name.
The code cannot read whether the person truly asked; it links the approval to
the person's message in the same turn, so the person can see it.

**Create structure stays the person's only.** It writes to disk; no MCP tool
calls it (§23.7).

## 24.3 What must be impossible: the Planner as `human`

The one forgery that matters is an approval recorded with agent = `human`
that the person did not click. The Planner could do it only by sending the web
client's own HTTP request from a shell (`curl`). The earlier `shadow` did not
stop this; this section does, by giving the Planner no hand to send it:

1. **The Planner's tools are fixed by Shadows** when it opens the session:
   reading files (Read, Grep, Glob), web search and fetch (reads only), and
   Shadows' MCP tools. No shell, no Write, no Edit, no other MCP server.
2. **Every permission request is refused outside that list**, in every mode
   the person can choose, bypass included. Shadows already answers each
   permission request; this makes the answer fixed for the Planner.
3. **A read never writes.** Every HTTP route that changes state is a `POST`,
   `PUT` or `DELETE`; a `GET` changes nothing, so a fetch tool cannot approve.
4. **The core is the boundary.** An approval method checks its agent itself;
   the tools and the client are defence in depth.

Losing the shell costs the Planner running commands such as `git log` or a
build. It plans; reading files is enough for that.

> **OPEN — what the adapter actually honours.** Before building, a probe on
> the real adapter must show: (a) the fixed tool list holds in every mode,
> bypass included; (b) whether a session loads MCP servers, plugins or hooks
> from the person's own Claude settings, such as browser tools — if it does,
> Shadows must keep them out of the Planner's session. **Closes when** the
> probe's evidence file is written and this section is amended with what it
> showed.

> **OPEN — executors.** §17's executors need a shell, so §24.3's rule cannot
> protect them. **Closes when** §17 specifies their sandbox: a shell that
> cannot reach the daemon's port.

## 24.4 What this does not stop

Another program the person runs, or malware running as the same user, can
still send the web client's request. On a single-user machine that is inside
the trust boundary: it can read the database too. This section is about the
agents Shadows launches, which it controls.

## 24.5 Tests

1. An approval through an MCP tool records agent = `planner:<harness>` and
   the turn's person message, whatever its arguments claim.
2. The Planner session refuses a shell, Write or Edit permission request in
   every mode, bypass included.
3. No `GET` route changes state (one test walks the router).
4. Create structure has no MCP tool.
