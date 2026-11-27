# Section 18 — Project Planning Workspace and Shared API Agreements

- **Date:** 2026-10-03
- **Status:** Stages 1 and 2 built (shared agreements, migrations 0017 and 0018).
  Stage 3 and later are in [`docs/core/future.md`](../future.md).
- **Intent:** [vision §9](../vision.md#9-the-roadmap-shared-contracts-and-the-effect-of-a-change).
- **Delivery:** [the full roadmap](../roadmap.md).
- **Related owners:** §13 Planner sessions, §14 application boundaries,
  §16 project plans, §17 executable task contracts and evidence.

This section owns the first planning workspace and shared API agreement
semantics. It does not replace a task's own contract (§17), task-to-task
links (§16.7), plan freezing (§16.2) or the execution lifecycle (§17.4).

## 18.1 Scope and first release

The first release joins two independently runnable stages:

1. **Workspace:** project vision, freely nested parts, roadmap outcomes,
   associations to existing plans, navigation and the local project map.
2. **Agreements:** structured HTTP API contracts, versions, task bindings,
   proposals, impact review, person approval and explicit adoption.

The success case is one login agreement between two plans: agree v1, bind
tasks, propose v2, inspect impact, agree v2 and continue one plan on v2 while
the other remains on v1. Names such as Backend and Frontend are examples.

A Planner or external agent reads the workspace on request through the read-only
`workspace_get` tool: the vision with the first page of top-level parts and
outcomes. No agent tool writes it; the person does. Automatic Planner context compilation is Stage 3 of the roadmap. ER diagrams,
screen-flow editors, alternative grouping views, executors, verification and
the executive manager are later stages. The first release neither runs tasks
nor labels a participant's implementation as verified.

The workspace preserves the planning knowledge that later feeds focused
executors. §17.5 owns their bounded packets and additional questions; vision
§5 describes the agentic manager above them. The roadmap proves execution and
manager escalation before specialized diagrams. This section adds neither a
manager service nor new approval authority for agents.

## 18.2 Identity and organization

- A project part has a stable id, project, title, responsibility, design text
  and optional parent. Its kind is an optional descriptive label, not a fixed
  hierarchy level. There is no required depth; the mandatory parts every
  project's map must hold are §23.2's.
- Parents belong to the same project. A part cannot be its own ancestor.
  Rename and reparent preserve identity, plan associations and relationships.
  Children have explicit stable ordering. Whole-subtree deletion is outside
  the first release; the initial editor creates, edits, moves and reorders.
- One primary containment tree drives breadcrumbs and navigation. Future
  domain/application/ownership views reference these same identities. They
  do not create parallel copies of a part.
- The project vision is purpose, users, goals, boundaries and agreed technical
  direction; §23.3 makes each field a list of items with stable ids. Part
  design text describes the intended design; it is not a
  claim about running software.
- Vision, parts and outcomes have revisions. Their edits and moves require
  the expected revision and return a conflict instead of overwriting another
  writer. References name identities rather than titles or breadcrumb strings.
  Stage 1 uses one workspace revision as the atomic edit-batch precondition,
  plus revisions on affected elements. An unrelated intervening workspace
  edit can therefore cause a conflict; finer concurrent editing is deferred.
- A plan can be associated with several relevant parts and outcomes. It
  retains its existing id and project ownership. Associations organize work;
  they do not alter the content of its old versions. Existing plans start
  unassigned and stay accessible in the project plan list.

## 18.3 Roadmap outcomes

An outcome has stable identity, project, title, intended result, acceptance
items and optional parent outcome. Its hierarchy is independent of the part
tree. Parents are in the same project and containment is acyclic.

An outcome references any number of parts and plans in its project. For
example, login spans Auth Service, Login Page and their plans. References do
not move or copy the parts. Ordering is explicit. Planned outcomes are not
automatically completed when a plan is approved or archived; who decides an
outcome is achieved is open (see "Outcome acceptance" under Delivery
traceability).

### Delivery traceability (approved design, 2026-11-26)

Shadows links delivery work to its roadmap results, map ownership and execution
state. The links follow saved relationships and task contracts. Initial project
setup belongs to project preparation, outside the delivery plan lifecycle.

- **Draft from an outcome.** Roadmap's **Plan from here** opens a Planner
  conversation anchored to a saved outcome. When the Planner creates a new plan
  there, Core reads the anchor and saves the link in the draft creation
  transaction; the Planner names no outcome. The outcome must still exist in
  the same project. The link raises the workspace and outcome revisions, so an
  outcome editor opened before it adopts the new plan list, and a save built on
  the older revision is refused as stale; the draft's own event tells the views
  to read again. A replayed draft start
  answers from its recorded command and links nothing twice. A failed
  attachment leaves no new plan. The plan shows its outcome by name before
  approval, where the person corrects a wrong one in the outcome editor.
- **A version or a new plan.** When the outcome is known, the Planner reads
  its linked plans before starting one. A changed way of reaching the same plan
  goal is that plan's next version; a feature with acceptance of its own is a
  new plan, linked to the outcome it serves or to a new outcome the Planner
  proposes. While the outcome is still unknown, the Planner may explore in a
  draft; approval asks for the link. A plan is never linked to an outcome it
  does not serve to pass the approval check.
- **Draft from a regular conversation.** A person or agent can explore and create
  a draft before choosing an outcome. When the person selects an existing
  outcome, Shadows saves the link. The outcome's **Linked plans** list also
  manages that association. When a new outcome is needed, the Planner can
  propose one through the existing roadmap flow for the person to approve.
- **Approval.** The same Plans approval operation for the person and Planner
  requires at least one live roadmap outcome and one project map part. The
  approval screen identifies a missing association and directs the person to
  where it can be added. One plan can serve several outcomes and parts.
- **Task ownership.** Shadows resolves a task's read and write scopes against
  the map using the rules that produce its TaskContext. Write scopes identify
  where the task makes changes. Read scopes identify inputs. Unowned paths
  stay visible with the reason. The resolved write owners supply the affected
  parts of a plan; explicit part links cover work with no file changes.
- **Task outcomes.** A task is discoverable from each outcome through its plan.
  When a plan serves several outcomes, the views present its tasks under the
  shared plan and make the level of that association clear. Plan and task
  context use those same links for the Planner and executors.
- **Views.** From a Roadmap outcome, show its linked plans, their tasks and
  their execution progress. From a Project Map part, show its plans and the
  tasks whose resolved scopes touch that part. From a plan or task, link back
  to those outcomes and parts. State composes these views from Design, Plans
  and Runs. Count a task once per result or project total using its exact plan
  version and task number. Task progress comes from Runs. Outcome acceptance
  still depends on evidence against the outcome's acceptance items.
- **History.** The association names the plan, so it covers later versions.
  The rule applies to future approvals, including approvals of new versions.
  Existing approved history needs no backfill. An active approved plan with an
  outcome cannot lose its last outcome. Moving it to a different outcome in
  one workspace edit is allowed.

For example, starting `Data file format and storage` under
`Shared data file format and safe storage` puts that plan under the outcome
as soon as its draft exists. A normal conversation can draft the same work
first, then the person selects the outcome before approving the plan. The
outcome shows the plan's task progress, and each task shows its affected
parts from the saved scope. The outcome, map and State views use the same
identities when moving between them.

> **OPEN — outcome acceptance, 2026-11-26:** These links show what work serves an
> outcome and how far it has run; they do not say the outcome is achieved. §17
> holds evidence for tasks, and no owner yet decides an outcome's acceptance
> items. The next slice gives each acceptance item a decision and the evidence
> behind it (a task report, a check or a test result), gathered by the Planner,
> and the person approves that the outcome is achieved. Close this when an
> outcome carries that decision and the Roadmap shows it beside task progress.

## 18.4 A shared contract

- A contract has stable identity, project and capability name. It describes
  the externally visible interface between providers and consumers.
- Its content is versioned. Each version has a positive sequence number,
  revision, state `Draft` or `Agreed`, writer, creation time and, after v1,
  a non-blank change reason. A contract has at most one editable Draft.
- Starting from an Agreed version copies it into the next Draft. A repeated
  start can return the existing Draft under the normal replay rules. An
  edit is a single batch against its expected revision.
- Agreement freezes the complete version. Content, operations, schemas and
  shared behavior cannot be inserted, changed or removed afterwards, enforced
  by storage as well as the service. An Agreed v2 does not erase or revoke v1.
- Writer provenance identifies a person or live agent grant, with a Planner
  thread/operation when applicable. Revoked grants cannot write. Proposal
  authority does not imply agreement approval authority.
- Parts and plans reference contracts. The contract is not duplicated into
  each participant's design document. Renaming a part leaves references valid.
- Each version declares the provider and known consumer parts, by stable id
  and role. Every operation needs a declared provider before agreement. These
  declarations let the first v1 review show the intended parties before tasks
  can bind an Agreed version; actual task bindings record adoption separately.

## 18.5 HTTP API content

Use the project's existing OpenAPI 3.1 description convention for machine-
readable operation content, with human-readable purpose, shared behavior and
acceptance items alongside it. The first release supports HTTP APIs. Other
contract kinds need their own content model when introduced.

Each operation has a stable project contract operation identity, distinct
from its mutable display name or HTTP path. The same logical operation keeps
its identity across copied versions; removing one and introducing a different
operation never reuses its identity. The service owns the mapping between
those identities and the OpenAPI operations.

The editor exposes method/path, parameters, request body, responses, errors,
security requirements and their descriptions. The structured editor and JSON
view modify the same version. Agreement requires valid OpenAPI structure,
unique operation identities, resolved local references, non-blank purpose and
acceptance items. External reference retrieval is outside the first release;
unresolved references block agreement. A service validates one canonical
representation, using a library rather than a custom schema language.

The contract's OpenAPI document is separate from `api/openapi.json`, which
describes Shadows' own API. Editing a user's contract does not regenerate
Shadows' API types.

Reference: [OpenAPI 3.1.0 specification](https://spec.openapis.org/oas/v3.1.0.html).

## 18.6 Bindings and participants

A task binding names the contract, an exact Agreed version, its declared
participant part, a role (`provides` or `uses`) and the operation identities
relevant to that task. The part must belong to the plan's project and the
role must match the version's declaration. Part plan views include bindings
as well as manual associations, with history distinguished from current work.
Shared contract
behavior is included for either role. Several plans or tasks can provide or
consume different operations of one contract; no Backend/Frontend roles are
hard-coded. These records derive the participant view.

Bindings are part of the plan version's content. Adding, changing or removing
one is a Plans edit with its expected revision and existing writer/archival
checks. Task removal removes that task's Draft bindings in the same batch.
Copied plan versions copy bindings. Frozen bindings never change. An old
plan without bindings remains valid under its existing rules.

The agreement and plan must be in the same project in the first release.
Named operations must exist in the pinned contract version. A binding is
checked inside the same write as the plan edit, and again before approval.
Invalid references refuse the whole edit; they are never silently discarded.
Archived plans remain in participant/history reads but cannot adopt a revision
until the person unarchives them under §16.2.

This binding does not supply an execution order by itself. §16.7's task links
still express `needs` and `completes_after`, with their own target rules. A
contract binding follows its exact agreed version, never the latest version.

## 18.7 Review and agreement

> **OPEN — implementation gate, 2026-10-05:** Agreement edits currently resolve
> the latest version from identity plus expected revision. A delayed edit can
> match a later Draft whose revision has restarted at zero. The edit target
> must identify the exact Draft version as well as its revision. Close this
> gate when core, HTTP, MCP, Web and command fingerprints carry that target
> and a regression proves that a request for vN/rev0 cannot edit vN+1/rev0.
> This checkpoint does not establish Stage 2 completion.

1. A writer proposes a Draft with a reason. Its base Agreed version is named.
2. The service computes a structural difference and registered participants
   whose pinned versions/operations or shared behavior are affected. For an
   older pin, compare against that pinned version, not an assumed common v1.
   The review also includes the candidate's declared parties and reports
   new or removed declarations, including a party with no bound task yet.
3. Review shows the difference, reason, involved parts/plans/tasks, current
   plan states, participant pins and proposed next actions. Both provider and
   consumer roles are included. Historical bindings remain separately visible
   from each plan's latest version, so they do not falsely report current work.
4. Compatibility is `Needs review` in the first release. A structural diff
   alone does not certify compatibility. Unregistered code dependencies and
   absent execution evidence remain explicit limits of the impact report.
5. The person approves against the Draft revision and the review identity.
   Agreement recomputes the review basis inside its write transaction. If
   content, declared part revisions, participant pins, relevant plan state or project removal changed,
   the write returns a conflict and requires a fresh review; nothing is agreed.
6. Success freezes exactly the reviewed version and journals the approval.
   Participants keep their old pins. No approval step runs or stops agents.

The review identity covers the base/candidate content, declared part revisions
and relevant participant versions, revisions and state. It is derived by the service, not accepted as
a caller's assertion of impact.

Approval is offered only on the person-facing surface; MCP can read, propose
and edit Drafts but cannot agree a contract. The existing local person-
attribution limitation and its execution prerequisite remain owned by §13.2
and §17.11. This slice must not claim stronger authority isolation than that
baseline; task execution stays outside it.

## 18.8 Adoption and work state

- Agreement and adoption are separate journaled facts. A consumer still pinned
  to v1 is not silently migrated because v2 exists or a notification arrived.
- A latest Draft can explicitly edit its bindings to an Agreed v2. This
  revalidates operations and relevant acceptance descriptions. Removing an
  operation requires removing/replacing its reference in that same batch.
- For a Frozen plan, continue it under §16.4 and change bindings in the new
  Draft. The old Frozen plan still reads its exact v1 content.
- Draft/Frozen/Archived describe planning, not implemented/verified software.
  Participant cards show plan state and version pins. In this slice, execution
  status says `Not recorded`; it is not inferred from approval or archive.
- The later execution stage uses §17's immutable packets and attempts. A
  proposal alone does not invalidate a running task's agreed inputs. If work
  needs a new agreement, the manager's eventual policy determines stop and
  replanning, and execution uses a new packet rather than silently replacing
  the running attempt's inputs. Manager policy is not implemented here.

## 18.9 Workspace UI

The project planning entry offers Vision, Roadmap, Map, Contracts and Plans.
Existing conversations remain accessible. An unassigned old plan is listed
alongside associated plans; changing navigation does not hide its history.

The map initially shows a selected part's children, its plan references and
relevant contracts/participants. Opening a child updates the breadcrumb. The
sidebar presents the primary tree with collapsed branches, and lists support
large projects without rendering every element simultaneously.

The roadmap shows nested outcomes and related parts/plans. Contract pages
show versions, operations, pinned participant bindings and the proposal review.
An Agreed version is read only; the next-version action creates/opens its Draft.
Review never reports execution state from plan state. Errors and conflicts
explain which item needs rereading or fixing.

The first release can choose a part or outcome as a future planning anchor
and display related sources. This is an inspection preview, not an automatic
ACP context compiler or a claim of reduced token consumption. Browsing a
contract does not change that chosen anchor. Stage 3 supplies the actual
context policy through §13's session owner.

## 18.10 Core ownership and interfaces

Following §14, a proposed `Design` service inside `shadows-core` owns the
project design workspace: vision/parts, roadmap outcomes and shared contract
versions/review. It is introduced with its first browser caller in Stage 1;
no module or crate is created by this draft. `Plans` owns task bindings and
their edits/approval. `Events` owns journal delivery. Each cross-service store
call must be declared in the involved service contracts.

Person HTTP and agent MCP operations translate to the same owning service
methods. HTTP supplies workspace reads/edits, version reads/starts/edits,
review and agreement. MCP supplies project-scoped reads and proposals with
no agreement operation. Grant validity/project reach is checked before reads
and rechecked inside every write. Removed projects refuse new writes.

The detailed implementation plan fixes method/route/tool names, payloads and
library choices against the code map and current contracts before product
code changes. It updates §14's service composition when Design is introduced,
the touched service contracts, generated OpenAPI/Web types and code ownership
map in the same implementation commits. No new operation bypasses a service.

## 18.11 Storage, replay and events

SQLx migrations preserve existing project, plan, workflow, task, conversation
and event identities. New data uses project-qualified foreign keys; contract
version numbers and one Draft per contract are unique. Storage refuses edits
to Agreed contract versions and Frozen bindings, including direct row writes.
Hierarchy acyclicity is checked in the same serialized write as a move.

The Stage 1 implementation plan allocates its migrations. §16.9 owns the
following delivery order for 1b and Stage 2 agreements; their separate plans
allocate migrations in that order. If unshipped numbering changes, amend
that owner before implementation; do not create a placeholder or reuse a number.
Test the migration first on a copy of the dev database. This design work does
not open, migrate or modify Mohammed's real database.

Mutations use §5's command identity, normalized fingerprint and durable replay.
A replay returns the first successful result without journaling again, even
if the review basis changed afterwards. A new command validates current state.
Revision/fingerprint mismatch writes nothing. Errors use the existing core
taxonomy, with actionable stale-review/reference/immutable-state details.

Changes journal project events naming affected identities and revisions.
The existing project SSE stream invalidates workspace/contract/participant
queries, including after reconnect; the UI refetches authoritative reads.
Unchanged participants can receive a change notice without adopting it.
No polling or broker is introduced.

## 18.12 Acceptance and design handoff

Focused tests must demonstrate:

1. Rename/move retains references; a containment cycle or foreign-project
   reference is refused with no partial write; children retain stable order.
2. A roadmap outcome references two parts/plans without duplicating them.
3. Agreement freezes every content row; one Draft is created under competing
   starts; replay preserves its original result; revoked grants write nothing.
4. Provider and consumer tasks bind exact operations/v1. Frozen rows cannot
   be rebound; copying into a Draft preserves the old bindings.
5. A v2 proposal finds a changed operation and shared-behavior consumers,
   distinguishes latest/historical pins and marks compatibility unresolved.
6. Changing a participant after review refuses stale approval atomically;
   approval changes no pins; explicit Draft adoption affects that plan alone.
7. HTTP/MCP use the same service rules; MCP exposes no agreement approval;
   project SSE refreshes another tab; malformed content gives useful feedback.

The Windows trial is the roadmap's complete login journey, including daemon
restart and reads of old Frozen plans. Record its actual results separately
from the tests. Neither the presence of this spec nor a plan claims a pass.

After Mohammed reviews this written spec, prepare the Stage 1 implementation
plan and select its execution method. The written spec review and plan review
precede product implementation. Specialized diagrams, context compilation
and manager/executor designs remain in their later stages; the first diagrams,
an ER diagram and a screen flow built from §18.13's elements, are the
[code record design §5.2](../../backend/specs/2026-11-19-project-code-record-design.md#52-the-diagrams-are-built-from-the-design-and-the-plans).

## 18.13 Tables and screens: design elements (Mohammed, 2026-11-19)

A table of the project's database and a screen of its interface are **design elements**, owned
by Design beside parts, outcomes and decisions. A task does not describe them; it pins the
revision it builds (§17 "The task contract"). This section owns their identity, content, rules
and writer; what the code holds of them and their diagrams are the
[code record design §5](../../backend/specs/2026-11-19-project-code-record-design.md#5-the-projects-state-shadows-answers-first-the-code-confirms-mohammed-2026-11-19).

**Identity and revisions.** An element's id is `E{n}`, the next number in its project, never
reused. Its kind (`table` or `screen`) is fixed when it is created. Each write is a new revision
(1, 2, ...) and nothing is overwritten. A write names the revision it follows
(`expected_revision`); any other is refused stale. Deleting an element writes a last revision
marked deleted; a deleted element takes no further revision. Rename keeps the id: references
name ids, never names.

**Tied to the project.** Every element belongs to one part of the approved map and serves at
least one roadmap outcome, so it is written only at the Plans stage (a roadmap is approved).
Its part's kind follows the standards' kinds (§23.2): a table belongs to a part whose kind is
`database`, a screen to a part whose kind is `frontend`. The part's `serves` ties it to the
vision's goals; the element adds no second goal link.

**Content.**
- A table: name, purpose, and its columns, each with a name, a type as the project writes it,
  a meaning, whether it is part of the primary key, and, for a foreign key, the table (an element
  id) and column it refers to.
- A screen: name, purpose, its route when it has one (`/login`), and where it leads: each target
  screen (an element id) with when ("after a successful login"). Not its look.

**Rules**, each a refusal that writes nothing:
- names, purposes, column fields, routes and link conditions are trimmed and bounded; a table has
  1 to 200 columns with names unique ignoring case; a screen leads to at most 50 screens, each
  (screen, when) once; a route starts with `/`;
- a table name is unique among the project's live tables, a screen name among its live screens,
  ignoring case; a route among its live screens;
- a foreign key names a live table of the project and a column that table has (a table may refer
  to itself); a screen leads to live screens of the project;
- a write may not drop or rename a column another live table's latest revision refers to, and an
  element another live element refers to may not be deleted: the refusal names the referrer;
- a project holds at most 500 live elements.

**Writer.** The Planner writes elements directly, in its running turn, through MCP; there is no
card and no person approval. They are seen where a task pins them, in the plan the person reviews
and approves. No route writes them in this release. Editing a diagram later edits this same
record ([vision §12](../vision.md#12-clear-navigation-and-focused-work)): a hand edit is a new
revision of the element, by the same rules.

**Events.** Each write journals `DesignElementChanged { project_id, element_id, revision }` in its
own transaction; a replay journals nothing. Project removal leaves the rows, as it leaves
decisions; reads of a removed project refuse.
