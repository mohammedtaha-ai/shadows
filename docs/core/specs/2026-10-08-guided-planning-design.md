# Section 23 — Guided Planning From Scratch

- **Date:** 2026-10-08.
- **Status:** Accepted by Mohammed in conversation on 2026-10-08, section by
  section. Delivery follows the PR slices in §23.9;
  acceptance is recorded in each PR and its dated evidence.
- **Related owners:** §18 (the workspace: vision, parts, outcomes, agreements),
  §13.8 (Planner instructions and the session), §13.6 (Shadows' MCP server),
  §13.9 (showing a card in the conversation), §12.2 (what a turn's entries
  are), §17.5 (the bounded execution packet, later), §14 (core ownership).

This section owns planning from an idea to approved roadmap and plans:
a free conversation, a vision agreed item by item, and a project map that
records parts and layers in the chosen stack's layout. Each planning step
is proposed and approved through its owning service. Framework tools and
building agents create the files; Shadows records the layout.

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
   parts, one folder each, layers and each part's contract head
   (§23.5). The person approves it.
4. **Roadmap and plans.** After map approval, outcomes, then plans, each
   tied to the parts they serve (§18.3).

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
`crates/core-standards/src/standards.yaml`, compiled into the binary. It
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
  - technical direction `TD`.

  `PU` keeps purpose apart from a project rule's `P1` (§23.2); `TD` keeps technical
  direction apart from a plan task's `T1` (user ruling, 2026-10-31). Numbers count
  per field from 1. An add gets its id when it is proposed: the proposal
  reserves the next number, its answer and card show it, and approval uses
  it, so the Planner can cite it at once, two pending adds never claim the
  same one, and the order of approval does not change it. A superseded add
  leaves its id unused. An add proposal stored without a reserved id takes
  the next number when it is approved. The next number is one past the
  highest ever issued in that field, so a removed id is never reused.
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
     → roadmap → plans
```

The stage is `roadmap` once the map is approved, and `plans`
once a first roadmap is approved.

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

Before map approval, the stage reports missing vision fields or mandatory
part kinds. Every nesting depth counts toward a required kind. Map approval
opens Roadmap directly. Drift is reported from then on, in the stage line and beside the goal
on the Project map page: each goal no part serves, and each part whose
served item changed or was removed since that part was last written. Drift is
per part, so editing one part never clears another's. A link to a removed
item is drift, not a refusal: it never blocks an edit of another part.
Renaming, reordering, moving or changing plan associations does not refresh
a part's served-item review baseline. A person content edit reviews its
responsibility, design or served-item links; an approved Planner change is
an explicit review of that part.
Paths are compared case-insensitively, as Windows does. Each part has one
folder; layer places follow the chosen stack and may lie outside that folder.

## 23.5 Proposals and approval

The Planner never writes the workspace. It proposes; the person approves.
New MCP tools (§13.6):

| Tool | Proposes | Its card |
|---|---|---|
| `vision_propose` | Add, edit or remove one vision item | The item, with the old text beside an edit |
| `map_propose` | The first map whole; later, one change (add or change a part, its folder, layers or contract) | The part tree and folders; each contract opens in the side panel |
| `roadmap_propose` | The first roadmap whole; later, one outcome added, changed or removed | The outcome list, each with the parts it references |

A map proposal carries each part's title, responsibility, served vision ids,
one folder, layers with places or not-used reasons, and its contract head.
Starter files are not part of the map. The scaffold creates manifests in the
framework's layout; agents learn the recorded layout from Shadows.

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
  Planner at the person's request arrives with §24 in PR 3b; a first map
  carrying a waiver stays the button's only (§24.2).

Map proposals: decided with Mohammed on 2026-10-12, for PR 3.

- **The map is the parts.** There is no second map document. A part gains
  what the map needs: the vision items it serves, its folder, its layers
  and its contract head (header and the standards rule ids that apply).
  Approving the first map writes every part in one write; every later change
  to the parts, approved or by hand, is a stored map version.
- **Who writes it.** The first map comes from the Planner only: before it is
  approved, a hand create, edit or move of a part is refused. After it, the
  person edits parts by hand on the Project map page, through the same gates.
  **Legacy adoption: there is none** (decided with Mohammed on 2026-10-12).
  Pre-PR3 parts and plans are test data; first-map approval still refuses
  replacing their identities, and nothing adopts them. A person's existing
  codebase enters Shadows only through the import named in §23.1, which
  reshapes it until it meets the standards or refuses it.
- **Identity.** A part's id is issued when it is written. Two parts with the
  same title under the same parent are one identity and conflict.
- **Folders.** A folder belongs to one part. A child part's folder sits
  inside its parent's; any other overlap is an ownership conflict and is
  refused.
- **Rule refusals.** A map, part, path or folder that breaks a map rule is
  refused as `MAP_REFUSED: <rule>: <message>`: HTTP `422` with the code
  `MAP_REFUSED`; over MCP a tool result whose text begins `MAP_REFUSED:`. The
  rule is one of `duplicate_part`, `duplicate_sibling`, `missing_parent`,
  `unknown_vision_item`, `unknown_rule`, `one_folder`,
  `folder_outside_parent`, `folder_claimed_twice`, `missing_layer`,
  `unknown_layer`, `layer_not_used_without_reason`, `layer_place_shared`,
  `invalid_path` and `invalid_part`. The message names the
  parts and the paths involved and says in one sentence what to change.
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
- **Unserved items.** A map proposal's answer and card carry `unserved`: the
  vision item ids, of every field and in vision order, that no part of the
  proposed map serves. It informs the Planner and refuses nothing. Its limit:
  it finds an item no part serves, not a part that left out an item another
  part already serves; whether a part's text matches its `serves` is for the
  Planner and the person to judge.
- **Mandatory parts** are recognised by `kind` on a part at any nesting depth. Several
  parts may share a kind; one is enough.

Roadmap proposals and plans: decided with Mohammed on 2026-10-12, for PR 5.

- **The roadmap is the outcomes** (§18.3). There is no second roadmap
  document. `roadmap_propose` mirrors `map_propose`: the first roadmap is
  proposed whole, as one card; after its approval each later proposal adds,
  changes or removes one outcome. Each proposed outcome carries its title,
  intended result, acceptance items, parent and the parts it references; a
  first roadmap with no outcome, or an outcome referencing no part, is
  refused. A change replaces its outcome's content, parts and parent: naming
  another parent, or none, moves it, and the card shows the move.
- **Who writes it.** The first roadmap comes from the Planner only: before
  it is approved, a hand create, edit, move or part reference of an outcome
  is refused. After it, the person edits outcomes by hand on the Roadmap
  page, through the same gates.
- **Who approves.** The card's button, or the Planner at the person's
  request through `roadmap_approve` (§24.2).
- **Staleness** follows the map's per-item rule, not rule 4: the first
  roadmap is outdated when a part it references changed or was removed, or
  another roadmap was approved first; a change to one outcome is outdated
  when that outcome's content or parent changed, or a part it references
  changed or was removed. A part "changed" means its content: tying a plan to
  it or reordering it does not outdate a card.
- **A plan comes after the roadmap.** No plan is started, edited or approved
  before the first roadmap is approved.
- **A plan is tied to a part.** A plan version is approved only when its plan
  is linked to at least one part. The link is Design's existing plan link
  (§18.3); an outcome link is optional. The Planner links when it starts a
  new plan: `draft_start` carries the part ids, and Plans writes the links in
  the same transaction through Design's declared function. The person links
  and unlinks on the Project map page as before.

## 23.6 Gates

The gates are in the methods of the service that owns each write, so HTTP
and MCP, the person and the Planner, meet the same ones. `design` owns the
stage and the vision and map gates. Plans are `Plans`' writes:
every method that creates, edits or approves a plan calls `design`'s stage
check inside its own write transaction, a cross-service call declared in both
contracts (§14). No plan write path skips it — today's tools included.

| Operation | Refused when |
|---|---|
| Propose or approve a map | The vision is incomplete |
| Approve a map | A mandatory part is missing and not waived |
| Propose or approve a roadmap | The map is not approved |
| Start, edit or approve a plan; edit its bindings | No roadmap is approved |
| Approve a plan | Its plan is linked to no part |

A refusal is `STAGE_BLOCKED` with the reason (`vision incomplete: users,
goals`; `missing part: database`) — an operation outcome, not a transport
error, like `PLAN_BLOCKED`. The Planner reads the reason and returns to the
missing step. Over HTTP it is `409` with the code `STAGE_BLOCKED`; over MCP
it is a tool result whose text begins `STAGE_BLOCKED:` (§13.6). A hand edit
after the first map meets the same gate: it may not leave a mandatory,
unwaived kind without a part at any depth.
If a standards update adds a mandatory kind to an already approved map,
that pre-existing gap is reported in `missing`; incremental part edits and
approvals may repair it without first repairing every other gap. They may
not introduce another missing kind. Layer places are checked by the single
map policy; the layout follows the stack and can change after approval.
The person and Planner update the map when that layout changes.

## 23.7 Who writes the structure

No Shadows command writes the structure. The approved map records the
layout, and a plan's tasks or the Planner in Agent Mode create it with the
framework's tools when the person asks. No `SHADOWS.md` or per-part contract
is generated. Existing files are left alone. The owner is the
[structure spec §5.4](../../backend/specs/2026-11-05-structure-and-depth-design.md#54-who-writes-the-structure).

## 23.8 Groundwork: what this section fixes first

Done in PR 0: entries carry a card kind and a JSON payload (`card_json`, migration
0020), and the accretion points were split by domain. The list is in Git at
`f1459dcc`.

## 23.9 Delivery

Each PR is one slice a person can try in the browser. PRs 0 to 5 are merged
(#23 to #30); PR 3c is not built and waits on §24.3's OPEN.

| PR | Slice | Tried in the browser |
|---|---|---|
| 0 | Groundwork (§23.8) | Everything works as before; plan and subagent cards still show |
| 1 | Standards and the stage (§23.2, §23.4) | A Standards tab; the Planner proposes a separate backend from the first chat |
| 2 | Vision proposals (§23.3, §23.5) | Vision items are written from the conversation after approval |
| 3 | Map and contracts, with their gates (§23.5, §23.6) | A proposed map approved; a clear refusal when something is missing |
| 3b | §24: principal and agent, the Planner's fixed tools, its approval tools | "Approved by the Planner at your request" beside "Approved by you" |
| 3c | §24.3: the project switches that widen the Planner's reach (connectors, skills, `CLAUDE.md`), after their probe | A switch turns on Google Drive for one project's Planner and nothing else |
| 4 | Original structure writer, removed by the structure-and-layers slice 3 | No current structure command (§23.7) |
| 5 | Roadmap and plans behind their gates | A roadmap approved after the map; no plan before it, and none approved without a part |

## 23.10 Tests and acceptance

Service tests, few and exact:

1. Each gate refuses with its reason: incomplete vision, missing part,
   map not approved.
2. Waiving `api` or `frontend` needs a reason; `backend` and `database`
   cannot be waived.
3. Approving after the workspace changed is refused; a newer proposal
   supersedes the pending one.
4. Map approval records layout and writes no files. Nested part kinds count;
   an external layer place such as a Laravel connection config is accepted.
5. A roadmap proposal is refused before map approval; every plan write is
   refused before roadmap approval; a plan linked to no part cannot be
   approved. Standards changes reach a running session before its next
   message, and a proposal made under older standards cannot be approved.
6. A vision item added after the map's approval appears in the stage as
   unserved.

Acceptance is the browser journey on a copy of the development database with
the real adapter: a new project; a conversation about an idea; vision items
approved; a map approved; Roadmap opens without a Structure step; a
plan tied to a part. A framework scaffold runs without pre-created files.
