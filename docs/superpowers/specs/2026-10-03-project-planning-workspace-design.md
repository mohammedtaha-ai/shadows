# Section 18 — Project Planning Workspace and Shared API Agreements

- **Date:** 2026-10-03
- **Status:** Stage 1 implemented. Stage 2 semantics accepted for delivery
  under Mohammed's full-vision objective and explicit instruction on
  2026-10-04 to complete shared agreements. Later stages retain their owners.
- **Intent:** [vision §9](../../vision.md#9-the-roadmap-shared-contracts-and-the-effect-of-a-change).
- **Delivery:** [the full roadmap](../plans/2026-10-03-project-planning-roadmap.md).
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

Automatic Planner context compilation is Stage 3 of the roadmap. ER diagrams,
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
  hierarchy level. There is no required depth or Backend/Frontend structure.
- Parents belong to the same project. A part cannot be its own ancestor.
  Rename and reparent preserve identity, plan associations and relationships.
  Children have explicit stable ordering. Whole-subtree deletion is outside
  the first release; the initial editor creates, edits, moves and reorders.
- One primary containment tree drives breadcrumbs and navigation. Future
  domain/application/ownership views reference these same identities. They
  do not create parallel copies of a part.
- The project vision is purpose, users, goals, boundaries and agreed technical
  direction. Part design text describes the intended design; it is not a
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
automatically completed when a plan is approved or archived; execution
evidence later supplies completion under §17.

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
and manager/executor designs remain in their later stages.
