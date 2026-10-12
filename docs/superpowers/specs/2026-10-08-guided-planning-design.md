# Section 23 — Guided Planning From Scratch

- **Date:** 2026-10-08.
- **Status:** Accepted by Mohammed in conversation on 2026-10-08, section by
  section; this file awaits his review. Delivery follows the PR slices in §23.9;
  current progress and acceptance are recorded in `docs/status.md`.
- **Related owners:** §18 (the workspace: vision, parts, outcomes, agreements),
  §13.8 (Planner instructions and the session), §13.6 (Shadows' MCP server),
  §13.9 (showing a card in the conversation), §12.2 (what a turn's entries
  are), §17.5 (the bounded execution packet, later), §14 (core ownership).

This section owns how a new project is planned from an idea to a structure on
disk: a free conversation, a vision agreed item by item, a project map that
follows Shadows' standards, the folders and contracts written to an empty
directory, then the roadmap and plans. Each step is proposed by the Planner,
approved by the person, and gated in code: the Planner cannot skip a step,
and neither can the person.

Any AI asked "how do we build this?" proposes the quickest thing that runs —
one PHP file, a backend and database mixed together. A Shadows project never
starts that way. The standards are in the Planner's session from its first
message, and the code refuses a map that lacks a mandatory part.

## 23.1 The journey

1. **Idea.** The workspace is empty. The person talks about an idea; the
   Planner discusses it and asks questions. Nothing is written.
2. **Vision.** When the person asks how to start, the Planner walks the five
   vision fields (§23.3) one at a time. When an item is agreed it proposes it
   as a card; the person approves it and it is written.
3. **Map.** With the vision complete, the Planner proposes the project map:
   parts, folder structure, initial files and each part's contract head
   (§23.5). The person approves it.
4. **Structure.** The person presses **Create structure**; Shadows writes the
   approved folders, files and contracts into the project's empty directory
   (§23.7).
5. **Roadmap and plans.** Outcomes, then plans, each tied to the parts they
   serve (§18.3).

The vision and map keep growing after their first approval (§23.6). Every
project goes through this journey; there is no unguided mode. Existing
projects in a development database are test data and are deleted.

Out of this section, and named here so they are not mistaken for gaps:
importing an existing codebase (agents analyse it, map it into Shadows, and
the import is accepted only once the code meets the standards — the last
feature in this line); executors with bounded packets and enforced write
boundaries (§17.5); and focused context per part (roadmap Stage 3's context
compiler).

## 23.2 Standards

Standards come in two layers. The project's effective standards are the base
plus its additions; an addition never removes or weakens a base rule.

**Base standards** are one file in Shadows' code,
`crates/shadows-core/src/design/standards.yaml`, compiled into the binary. It
has a `version` and changes only by a reviewed PR. No route or tool edits it.
It is code rather than rows because it must be identical for every project,
must not be editable from the dashboard, and must not need a migration per
change (migrations are never edited).

It holds:

1. **Mandatory parts**, structured data the code checks:

   | Part | Owns | May be waived |
   |---|---|---|
   | `backend` | All logic: one core; every operation is a method of one service | Never |
   | `database` | The schema and storage: the only part that touches the database; migrations are append-only | Never |
   | `api` | The only path between the frontend and the backend, with a versioned contract (§18.4) | With the person's approval and a written reason |
   | `frontend` | The interface only, with no business rules | With the person's approval and a written reason |

   A waiver (a command-line tool has no frontend) is recorded with its reason
   and the person who gave it.

2. **Rules**, text the Planner reads and writes into contracts. Each has an
   id (`S1`, `S2`, …) and the parts it applies to. The first version holds at
   least: each service owns its tables and reads no other service's; the
   frontend reaches data only through the `api`; every part has a contract;
   migrations are never edited; ordering is explicit, never insertion order.

3. **The contract template**, adapted from Mohammed's generic source-contract
   template: rules (trace, gaps name symbols not lines, obligations name their
   test, reference external types, describe only what is owned) and the shape
   (header, shapes, functions, obligations, agreements, not_the_caller's,
   gaps, open_questions, tests).

**Project additions** live in the database, versioned like Planner
instructions (§13.8), edited from the workspace's Standards tab: extra rules and
extra mandatory parts. A new migration adds their table.

When a vision item, a map or a map change is approved, the base standards
version and the additions version it was built on are recorded with it.

## 23.3 The vision as items

This amends §18.2's vision: each of its five fields — purpose, users, goals,
boundaries, technical direction — becomes an ordered list of items. An item
has a stable id (`G1`, `U2`, …) that never changes or is reused, its text,
and the conversation it was approved from. A proposal adds, edits or removes
one item, so a card stays small however large the vision grows.

Each approval makes a new vision version; earlier versions stay readable.
The person can still edit items by hand on the Vision page; the same service
methods and revision checks apply.

The vision is **complete** when every field has at least one item.

Decided with Mohammed on 2026-10-12, for PR 2:

- **Ids.** Each field has a prefix:
  - purpose `PU`;
  - users `U`;
  - goals `G`;
  - boundaries `B`;
  - technical direction `T`.

  `PU` keeps purpose apart from a project rule's `P1` (§23.2). Numbers count
  per field from 1. An add gets its id when it is approved, so two pending
  adds never claim the same one. The next number is one past the highest
  ever issued in that field, so a removed id is never reused.
- **Versions.** Every change to the vision is a new version, whether an
  approval or a hand edit, and each version is stored. A page to read the
  history comes later, when the person or NYLRA needs to see what changed.
- **Origin.** On the Vision page each item names the conversation it was
  approved from, as a link, or says it was added by hand.
- **Migration.** A project's existing text fields become one item each; an
  empty field becomes an empty list.

## 23.4 The stage

A function of the `design` service computes the project's stage from the
database, never from the conversation:

```text
idea → vision (missing: users, goals…) → map (missing: database…)
     → structure (not written) → roadmap → plans
```

It also reports drift: vision items no part serves, and a map approved
against an older vision.

The Planner receives the stage in three ways:

1. **At session open,** the effective standards with their versions, inside
   the session's instructions (§13.8).
2. **When the standards change,** before the next message. A resumed session
   ignores a new `append`, so the session's instructions cannot carry a
   change. The base and additions versions join the versions §13.8's
   before-turn context block compares (`context_before_turn` in
   `harness/setup.rs`); when either differs from the thread's last recorded
   one, the selected effective standards accompany the next turn in its
   before-turn context block (§13.8). The invocation records that selected
   version; a save after recording is delivered on the following turn.
3. **With each of the person's messages,** one short line from Shadows naming
   the stage and what is missing or drifted (about 50 tokens).

The conversation's header shows the same stage to the person.

Drift arrives with map approval (PR 3). An unserved item is known only once
the map says which items each part serves, so with vision items alone (PR 2)
the stage still reports what is missing only. PR 1 ends at `structure`; later
stages require the recorded structure write.

From PR 3 the stage is `map` until a first map is approved, and `structure`
after. Drift is reported from then on, in the stage line and beside the goal
on the Project map page: each goal no part serves, and each part whose
served item changed or was removed since that part was last written. Drift is
per part, so editing one part never clears another's. A link to a removed
item is drift, not a refusal: it never blocks an edit of another part.
Renaming, reordering, moving or changing plan associations does not refresh
a part's served-item review baseline. A person content edit reviews its
responsibility, design or served-item links; an approved Planner change is
an explicit review of that part.
Paths are compared case-insensitively, as Windows does, and a part's initial
files lie inside its own folders.

## 23.5 Proposals and approval

The Planner never writes the workspace. It proposes; the person approves.
New MCP tools (§13.6):

| Tool | Proposes | Its card |
|---|---|---|
| `vision_propose` | Add, edit or remove one vision item | The item, with the old text beside an edit |
| `map_propose` | The first map whole; later, one change (add or change a part, its folders, files or contract) | The part tree and folders; each contract opens in the side panel |
| `roadmap_propose` | Outcomes and the parts they reference | The outcome list |

A map proposal carries, for each part: its title, responsibility, the vision
items it serves, its folders, its initial files with their content (manifests
and skeletons only, no logic) and its contract head — header, responsibility
and the standards rules that apply. Functions and tests are added when code
is written.

Rules:

1. A proposal is stored pending and drawn as a card (§13.9) from its
   structured entry (§23.8).
2. A newer proposal for the same item, or a newer map proposal, supersedes the
   pending one; its card shows **Superseded**.
3. There is no reject button: the person says what is wrong in the
   conversation and the Planner proposes again. The person approves with the
   card's button, or asks the Planner to approve; §24 records which.
4. A proposal records the workspace revision and the standards versions it
   was made against. Approval is refused if either changed since, and the
   Planner is asked for a fresh proposal.

Vision proposals: decided with Mohammed on 2026-10-12.

- **Several pending cards.** Several vision proposals may wait at once, each
  its own card, so the Planner can propose three goals in one reply. The
  person can approve two of them and ask for the third to change.
- **Supersession.** For an edit or a removal, a newer proposal for the same
  item supersedes the pending one. An add supersedes only the proposal it
  names as the one it replaces.
- **Staleness is per item.** For vision proposals this replaces rule 4.
  - **An add** is never stale.
  - **An edit or a removal** is refused when its item was edited or removed
    after the proposal. The card then shows **Outdated**, and the Planner
    proposes again on the current text.
  - **Other changes** do not make it stale: approving one card must not make
    its neighbours stale.
  - **Standards versions** are recorded with a vision proposal but not
    compared: the standards govern parts, not the vision.
  - **Map and roadmap** proposals keep rule 4.
- **Where they show.** Pending proposals are shown in the conversation only.
  The Vision page names how many are waiting, with a link to each
  conversation that has one.
- **Who approves.** In PR 2 only the card's button approves. Approval by the
  Planner at the person's request arrives with §24 in PR 3b.

Map proposals: decided with Mohammed on 2026-10-12, for PR 3.

- **The map is the parts.** There is no second map document. A part gains
  what the map needs: the vision items it serves, its folders, its initial
  files and its contract head (header and the standards rule ids that apply).
  Approving the first map writes every part in one write; every later change
  to the parts, approved or by hand, is a stored map version.
- **Who writes it.** The first map comes from the Planner only: before it is
  approved, a hand create, edit or move of a part is refused. After it, the
  person edits parts by hand on the Project map page, through the same gates.
- **Identity.** A part's id is issued when it is written. Two parts with the
  same title under the same parent are one identity and conflict.
- **Folders.** A folder belongs to one part. A child part's folders may sit
  inside its parent's; any other overlap is an ownership conflict and is
  refused.
- **Waivers.** A waiver of `api` or `frontend` travels inside the first map's
  card, with the Planner's written reason, shown apart so it is not missed.
  Approving the card approves the waiver, recorded with the person who
  approved. `backend`, `database` and a project's added parts are never
  waived.
- **Staleness.**
  - **The first map** is outdated when anything in the vision or the
    standards changed after it was proposed, or another map was approved
    first.
  - **A change to one part** is outdated when that part changed, when a
    vision item the change serves changed or was removed, or when the
    standards changed. A change to another part, or a goal it does not
    serve, leaves it valid: `notifications` serving G2 is outdated by an edit
    of G2, not by a new G4.
  - **Adding a part** is checked for conflicts (identity and folders) when
    proposed and again when approved, because the parts may change between.
- **Coverage.** Every goal (`G`) must be served by at least one part for the
  first map to be approved; the refusal names the unserved goals. After that,
  an unserved goal is drift, shown and not blocking. Purpose, users,
  boundaries and technical direction guide the map and need no part: a
  boundary may say what is not built. Serving a goal is planned coverage,
  never proof the goal is met.
- **Mandatory parts** are recognised by `kind` on a top-level part. Several
  parts may share a kind; one is enough.

## 23.6 Gates

The gates are in the methods of the service that owns each write, so HTTP
and MCP, the person and the Planner, meet the same ones. `design` owns the
stage and the vision, map and structure gates. Plans are `Plans`' writes:
every method that creates, edits or approves a plan calls `design`'s stage
check inside its own write transaction, a cross-service call declared in both
contracts (§14). No plan write path skips it — today's tools included.

| Operation | Refused when |
|---|---|
| Propose or approve a map | The vision is incomplete |
| Approve a map | A mandatory part is missing and not waived |
| Create structure | The map is not approved, or the directory is not empty |
| Propose or approve a roadmap or a plan | The structure has not been written |

A refusal is `STAGE_BLOCKED` with the reason (`vision incomplete: users,
goals`; `missing part: database`) — an operation outcome, not a transport
error, like `PLAN_BLOCKED`. The Planner reads the reason and returns to the
missing step. Over HTTP it is `409` with the code `STAGE_BLOCKED`; over MCP
it is a tool result whose text begins `STAGE_BLOCKED:` (§13.6). A hand edit
after the first map meets the same gate: it may not leave a mandatory,
unwaived kind without a top-level part.

**After approval.** Changing an approved vision deletes nothing. The stage
line names the change ("G3 added since the map; no part serves it"), the
Planner proposes a map change rather than a new map, and an unserved item is
shown as such in the workspace. A map change after the structure exists is
recorded; writing it to disk belongs to the executors (§17).

## 23.7 Writing the structure

**Create structure** on the map page writes exactly what the approved map
holds. It is a person's action in the web client; no MCP tool writes to
disk.

§24 owns who approved and makes the Planner unable to act as the person: it
has no shell to send the web client's request, so Create structure is
reached only from the web client. §24 is built before this write. What also
bounds the write is what it accepts: an approved map only, an empty directory
only, paths inside it only, the approved files only.

1. **What is written:** each part's folders; each part's `contract.yaml`,
   from its approved contract head and the template; the approved initial
   files; and a root `SHADOWS.md` naming the standards versions, the map
   version, and that Shadows is the source of truth.
2. **Empty directory:** the project directory may hold only `.git`;
   otherwise `STAGE_BLOCKED: directory not empty`.
3. **Paths:** every path must resolve inside the project directory. An
   absolute path or `..` refuses the whole write before anything is written.
4. **Intent first:** before the first byte, the write is recorded as started
   with every path and content hash, the map version and the standards
   versions. Each file is written to a temporary name beside it and renamed
   into place. Success is recorded last.
5. **Failure and retry:** if the write fails, or the daemon stops before
   success is recorded, the stage stays at structure and the person sees what
   is on disk. Create structure then resumes the recorded write: the
   directory may hold only `.git`, the recorded paths whose content matches
   their hash, and leftover temporary names of recorded paths, which are
   removed. It writes the missing files and records success. Any other file,
   or a recorded path whose content differs, refuses with the list of them;
   Shadows never deletes or overwrites such a file.

After the write, the contracts in Shadows remain the source; the files are
their output. Hand edits to those files are the executors' concern (§17).

## 23.8 Groundwork: what this section fixes first

The first PR changes no behaviour a person sees. It removes debt that §23
would otherwise grow:

1. **Structured cards in `thread_entry`.** This amends §12.2. Tool lines and
   subagent cards are stored today as agent messages with text bodies
   (`[tool: …]`, `[subagent: {json}]`). A new migration gives an entry a card
   kind and a JSON payload column and moves those bodies into them; readers
   read the columns. Plan cards are already structured — a `PlanView` entry
   with its workflow and task refs — and the migration keeps them as they
   are, parsing no plan text. §23's proposal cards are built on the new
   columns.
2. **The accretion points** named in CLAUDE.md are split by domain:
   `shadows-core/src/db/mod.rs`, `shadows-http`'s routes, and
   `shadows-core/tests/storage_contract.rs` — each service's share moves next
   to that service.

## 23.9 Delivery

Each PR is one slice a person can try in the browser.

| PR | Slice | Tried in the browser |
|---|---|---|
| 0 | Groundwork (§23.8) | Everything works as before; plan and subagent cards still show |
| 1 | Standards and the stage (§23.2, §23.4) | A Standards tab; the Planner proposes a separate backend from the first chat |
| 2 | Vision proposals (§23.3, §23.5) | Vision items are written from the conversation after approval |
| 3 | Map and contracts, with their gates (§23.5, §23.6) | A proposed map approved; a clear refusal when something is missing |
| 3b | §24: principal and agent, the Planner's fixed tools, its approval tools | "Approved by the Planner at your request" beside "Approved by you" |
| 4 | Writing the structure (§23.7) | The structure appears in the project directory |
| 5 | Roadmap and plans behind the map gate | No plan before the structure exists |

## 23.10 Tests and acceptance

Service tests, few and exact:

1. Each gate refuses with its reason: incomplete vision, missing part,
   non-empty directory, structure not written.
2. Waiving `api` or `frontend` needs a reason; `backend` and `database`
   cannot be waived.
3. Approving after the workspace changed is refused; a newer proposal
   supersedes the pending one.
4. The structure write refuses a path outside the directory and writes only
   the approved files. A write stopped after some files resumes and
   completes; a foreign file or a changed recorded file refuses the resume.
5. A plan write through any existing path is refused before the structure
   exists. A standards change reaches a running session before its next
   message, and a proposal made under older standards cannot be approved.
6. A vision item added after the map's approval appears in the stage as
   unserved.

Acceptance is the browser journey on a copy of the development database with
the real adapter: a new project; a conversation about an idea; vision items
approved; a map approved; the structure written into an empty directory; a
plan tied to a part.
