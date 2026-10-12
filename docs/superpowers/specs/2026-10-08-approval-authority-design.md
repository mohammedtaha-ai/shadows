# Section 24 — Who Approved: Principal and Agent

- **Date:** 2026-10-08.
- **Status:** Accepted by Mohammed in conversation on 2026-10-08. The
  real-adapter probe ran on 2026-10-12 and §24.2–§24.3 were amended with
  Mohammed the same day. Enforcement is §23.9 PR 3b, before §23's structure
  write (PR 4).
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

**Two approvals stay the person's only**, by the button in the web client:

- **Create structure.** It writes to disk; no MCP tool calls it (§23.7).
- **A first map that carries a waiver** (§23.5). A waiver gives up a
  mandatory part of the person's project; the Planner's map approval tool
  refuses any proposal that carries one, and says the person must approve it.
  A map without waivers, and a later one-part change, the Planner may approve
  at the person's request.

Decided with Mohammed on 2026-10-12.

## 24.3 What must be impossible: the Planner as `human`

The one forgery that matters is an approval recorded with agent = `human`
that the person did not click. The Planner could do it only by sending the web
client's own HTTP request from a shell (`curl`). The earlier `shadow` did not
stop this; this section does, by giving the Planner no hand to send it:

1. **The Planner's tools are fixed by Shadows** when it opens the session.
   Its built-in tools are exactly Read, Grep, Glob, WebSearch, WebFetch,
   ToolSearch, Agent and TodoWrite, plus Shadows' MCP tools. ToolSearch is
   how Claude Code loads the deferred MCP tools; Agent starts the Planner's
   subagents (§22); TodoWrite keeps a list inside the session and writes no
   file. No shell, no Write, no Edit, no other MCP server. Sent as
   `_meta.claudeCode.options.tools` on every session opening; a tool not in
   the list does not exist in the session, so no mode can run it.
2. **The person's own Claude settings stay out**: `settingSources: []` and
   `strictMcpConfig: true`. That drops their MCP connectors, plugins, skills
   and hooks (a hook runs a shell command, the very hand this section
   removes), and also the project's `CLAUDE.md`; the project's instructions
   reach the Planner through Shadows' own (§13.8). Decided with Mohammed on
   2026-10-12.
3. **The Planner's subagents are advisers with the Planner's authority.**
   They inherit the same tools, Shadows' MCP tools included, over the same
   grant: what they propose or approve is recorded as `planner:<harness>`,
   with the same limits (no waiver map, no Create structure). They never run
   or write code. Executors are a different matter (the OPEN below). Decided
   with Mohammed on 2026-10-12.
4. **Every permission request is still refused**, as today: defence in depth
   for any tool the list lets through.
5. **A read never writes.** Every HTTP route that changes state is a `POST`,
   `PUT` or `DELETE`; a `GET` changes nothing, so a fetch tool cannot approve.
6. **The core is the boundary.** An approval method checks its agent itself;
   the tools and the client are defence in depth.

Losing the shell costs the Planner running commands such as `git log` or a
build. It plans; reading files is enough for that.

The probe ([evidence](../../evidence/2026-10-12-planner-tool-lock-probe.md),
code at `d8d3465`) showed on `claude-agent-acp` 0.81.1: without these options
the Planner's session held Bash, PowerShell, Write and Edit and the person's
claude.ai connectors (Google Drive, Claude Docs); with them, in both modes
Shadows offers (`acceptEdits`, `auto`) and in a subagent, none of those
existed, and a planted project hook, skill and `CLAUDE.md` that plain Claude
Code loaded did not reach the Planner. Bypass is not in Shadows' mode policy (§12.4) and was not run; the
tool list does not depend on the mode.

**The person may widen the Planner's reach, never its hands.** Planned for
§23.9 PR 3c: a project settings page with switches for what is safe to give
the Planner — the person's own MCP connectors, their skills, and the
project's `CLAUDE.md`. Write, Edit, Bash and PowerShell are never offered:
a shell is the forgery this section removes, and writing code is the
executors' (§17). Decided with Mohammed on 2026-10-12.

> **OPEN — how each switch is delivered without the settings it rides on.**
> `settingSources: ["project"]` would bring the project's hooks with its
> `CLAUDE.md`, and a hook runs a shell command. Shadows may instead read
> `CLAUDE.md` itself into the appended instructions, pass chosen MCP servers
> in `mcpServers`, and pass skills through the SDK's own options. **Closes
> when** a probe like 2026-10-12's shows each switch brings its item and no
> hook runs, and this paragraph names the delivery.

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
2. Every Planner session opening (new, resume, fork) sends the fixed `tools`
   list, `settingSources: []` and `strictMcpConfig: true`, and a permission
   request is still refused.
3. No `GET` route changes state (one test walks the router).
4. Create structure has no MCP tool.
5. The Planner's map approval tool refuses a proposal carrying a waiver, and
   the same proposal approved by the person's button passes.

## 24.6 External human confirmation and assessment

Decided 2026-10-10 for NYLRA integration; not implemented by the local §23 slices.
§24.1's `user:local` is the current local identity, not a remote authentication
scheme. Before external commands, bind an authenticated account/device to an
authorized principal of the specific workspace. §1.7 owns workspace placement;
the [cloud architecture](../../../../shadows-backend/docs/architecture.md#8-decision-and-approval-integrity)
owns the confirmation delivery path. The runtime derives the actor from that
trusted path, never from a JSON field or forwarded model text.

There are two distinct effects:

- **Planner approval:** §24.2 continues to permit it and records the Planner as
  agent. It never satisfies an operation reserved for explicit human confirmation.
- **Human confirmation:** binds authenticated person/device, workspace, operation,
  exact proposal digest, source revisions, expiry and stable command identity.
  The owner validates both authority and current basis before accepting it, and
  records the intermediary/device separately without crediting the model as human.

Create structure remains human-only. NYLRA cannot turn a generic conversation,
preference, Planner approval or inferred "yes" into this authorization. The
trusted client shows the exact operation and proposal independently of model
output and requires a deliberate confirmation action. Voice may prepare a
proposal; voice-only approval remains unavailable until a separately reviewed
confirmation mechanism proves equivalent binding and origin. Optional future
delegated policies are explicit owner-enforced grants, never learned preferences.

An assessment request through NYLRA runs with a capability set that cannot edit
or approve plans/agreements/proposals, create structure, or dispatch execution.
Durable assessment/conversation records are allowed; project adoption is not.
The core enforces these restrictions for each operation. A normal write-capable
Planner session with an "evaluate only" prompt does not satisfy this contract;
reject an assessment request when an enforced assessment context is unavailable.
Link the assessment to the intended work and source versions. Subsequent adoption
is a separate command and may require a fresh assessment if its basis changed.

An identical authenticated retry returns the original recorded outcome without
another effect; the same identity with different normalized content conflicts.
A stale first submission is refused. Replay does not bypass current authorization
to disclose a stored result, nor start new work after revocation.

> **OPEN — external authority proof.** Close before accepting remote mutations
> with a reviewed account/device binding protocol and executable checks for
> forged origins, stale proposals, revoked membership/devices, exact retries,
> cross-workspace misuse and assessment attempts to write. Existing loopback
> Origin checks and the §24.3 adapter probe are not proof of remote authority.
