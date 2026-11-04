# Planner context: what a Planner session is given, and why

- Status: **slice 1 implemented and locally verified**, 2026-11-04; CI pending push.
  Accepted by Mohammed. Later slices remain proposals. Direction was decided in conversation on 2026-11-03 and 2026-11-04.
- Roadmap: Stage 3, "Planner context" (`docs/core/roadmap.md`): "Start planning at an explicit
  part or stage with bounded, attributable context; inspect what was included."
- Builds on, and does not replace:
  - §13, the Planner's session (`docs/core/specs/2026-09-25-planner-workflow-design.md`);
  - §17.3 and §17.5, version rationale and the execution packet
    (`docs/core/specs/2026-10-01-task-contracts-and-evidence-design.md`);
  - §18.9, the anchor preview this spec makes real
    (`docs/core/specs/2026-10-03-project-planning-workspace-design.md`);
  - §24, who wrote a plan version (`docs/core/specs/2026-10-08-approval-authority-design.md`);
  - the messaging design §6.5, the escalation ladder
    (`docs/backend/specs/2026-11-03-agent-messaging-design.md`).
- Closes §18.9's "Stage 3 supplies the actual context policy through §13's session owner".

## 1. Intent and why

Today a Planner turn carries Mohammed's text and images, what changed in its instructions and
standards since the session last heard, an optional focus task, an optional plan to continue, and
one stage line (`crates/core-turns/src/prompt.rs`). The vision, the parts, the outcomes and the
agreements reach it only if it asks with `workspace_get`, or searches. It spends time and tokens
finding what Shadows already knows, and it can miss something that matters.

Stage 3 lets Mohammed start planning at one part or one roadmap stage, and gives the Planner what
belongs to that point: bounded, with every piece's source named, and open to inspection. The same
machinery later builds the executor's narrower packet (§17.5) and the manager's wider view; this
is where it is built and proven, on the Planner.

## 2. Decided by Mohammed

1. **Layers, not everything.** Like a book with a table of contents and one open chapter:
   - **the index, always:** small; tells the Planner what exists;
   - **the open chapter, in full:** the part or stage being planned, and what is tied to it;
   - **the rest on request,** through tools.
2. **The conversation is helper memory; the durable record is the truth.** Mohammed often plans
   in one long session and compacts it. The session's conversation carries the *why*; every
   decision that matters is written into the durable record (a plan version and its rationale,
   §17.3), and Shadows gives the session its context again when that record changed, so a compact
   loses nothing that was decided.
3. **A problem goes to the plan's owner, not to "the last session".**
   - The owner is the Planner thread that wrote the plan's versions; §24 already records it.
   - If that thread's session is gone or has lost its context, Shadows opens a new session **on
     the same thread** and gives it the compiled context plus the problem.
   - A side session Mohammed opens for a second opinion (Codex, another conversation) is an
     adviser, never the owner. The manager never asks it. A conclusion from it that matters is
     written into the plan as a decision, so it reaches everyone.
4. **Inspection** (from the roadmap, confirmed): Mohammed can see exactly what the Planner was
   given, so a bad plan can be traced to missing context or not.

## 3. Goals, non-goals, anti-goals

**Goals**
1. A Planner conversation can have an **anchor**: one part or one outcome (a roadmap stage).
2. Its session receives the index and the anchor's detail, within a budget, each piece with its
   source and version.
3. It receives them again only when something it was given has changed, or Mohammed asks.
4. Each turn records what context it carried; Mohammed can read it.
5. The plan's owner thread is a fact Core can answer, for the manager later.

**Non-goals**
- The executor's packet (§17.5) and the manager's view: they reuse this, later.
- Semantic search, embeddings, or ranking by relevance. Selection follows the design's links.
- A new store of "decisions". Decisions are plan version rationales and standards today (§9 OPEN).

**Anti-goals**
- The whole project pasted into every turn.
- Context the Planner cannot cite: a piece with no named source and version.
- A silent cut: if the budget drops something, the context says what was dropped.
- Re-sending the same context every turn.

## 4. The design

### 4.1 The anchor (Claude's proposal)

- A Planner thread has an optional anchor: `Part(PartId)` or `Outcome(OutcomeId)`, the same
  `DesignAnchor` the design already uses for plan links.
- **Set:** from a part's or an outcome's page, "Plan this" opens a new Planner conversation
  anchored there; an existing conversation's anchor can be changed or cleared from its header.
- **Owner:** `Design`, which owns `DesignAnchor`; `Threads` cannot hold it, because
  `core-design` depends on `core-threads`. A change is a command (`thread.anchor`), so it is
  replayable and recorded.
- A conversation with no anchor works as today, plus the index (§4.2).

### 4.2 The compiled context

Three layers, in this order:

| Layer | Holds | Budget (Claude's numbers) |
|---|---|---|
| **Index** | the vision's purpose items; the part tree to depth 2 (names and ids); outcome names and ids; agreement names with their agreed version | about 1 500 tokens |
| **Detail** (anchored only) | the anchor's full text; its parent chain (names); its children (names and one line each); the plans linked to it: each latest version's title, task list and the rationale of its last 3 versions (§17.3); the agreements those plans bind, at their agreed versions; the project standards in effect | about 8 000 tokens |
| **On request** | everything else, through the existing workspace tools; a missing read is a tool the plan adds | none |

- **Sources.** Every piece carries `(kind, id, version or revision)`. The context block ends with
  its source list. A budget cut removes a source only when none of its content remains;
  child names and ids survive a cut of their descriptive lines. Each earlier rationale names
  its logical plan and version. The record stores the SHA-256 digest of that exact text alongside it.
- The source footer and cut markers count toward the index estimate. Detail includes its
  heading. The budgets are soft after the allowed cuts, and the record exposes the excess.
  Parts and outcomes read one page per parent through depth two and announce further pages
  without inventing their total.
- **Cut order** when Detail is over budget:
  1. older rationales;
  2. children's lines;
  3. agreements' bodies, which keep their names and versions.

  The anchor's own text is never cut. The block states what was cut, as `[cut: …]`.
- Tokens are estimated as characters / 4; the estimate is labelled as one.
- **Owner:** `Turns` already owns "what a Planner turn sends the model". The compiler is
  `Turns::planning_context(thread) -> PlanningContext`, reading `Design` and `Plans` through their
  public methods. Neither of them assembles part of the answer.

### 4.3 When it is sent

- **First turn of a session** (no record of what it heard): the index, and the detail if
  anchored.
- **Later turns:** only when the context's digest differs from the one **this harness session**
  last heard. A context counts as heard only when its turn ended normally; a stopped or failed
  turn leaves it owed. A new session on an old thread has heard nothing.
  This happens when the anchor changed, a source changed version (a plan version, an agreement, the
  design revision of a part in the detail), or the index changed. This extends the harness's
  existing "versions the session last heard" record (`core-harness/src/setup.rs`, `Heard`) with
  the context digest.
- **After a compact, automatically.** Mohammed: most people compact by hand, and others let
  Claude Code compact itself at its limit, so both must count. The adapter reports both
  (evidence `docs/evidence/2026-11-04-planner-context-probe.md` §1): today as a `tool_call`
  whose `_meta` carries `contextCompaction` and `claudeCode.toolName = "compact"`, with
  `trigger` `manual` or `automatic`. A completed compaction clears the session's heard context
  digest, so the next turn carries the context again.
- **On request:** a "send the context again" action in the conversation, for anything the
  compaction rule misses. (Claude's call.)
- A turn that carries a command alone (`/compact`, `/clear`) carries no context, as today, and
  the context stays owed.

### 4.4 Inspection

- An invocation that includes planning context records its prepared text, sources, estimated
  sizes, cuts and digest, plus whether the turn completed normally. A turn with no record
  reports that no planning context record is available. Absence alone does not establish why.
- **Owner:** `Harness`, which already owns invocation rows.
- The conversation shows, per turn, whether context was acknowledged, prepared, or has no
  available record. Opening a record shows its text, sources, versions and cuts.
  (Frontend; its own plan.)

### 4.5 The plan's owner thread

- **Rule:** a plan's owner thread is the thread that wrote its latest version (§24 records the
  writer of each version as `planner:<harness>` on a thread grant).
- A plan written only by a person or an external grant has no owner thread. An escalation then goes
  to Mohammed.
- **Owner:** `Plans::owner_thread(plan) -> Option<ThreadId>`.
- The manager (messaging slice 3 and 4) escalates to that thread:
  - if its session is live, it gets a turn;
  - if not, a new session on the same thread gets the compiled context anchored at the plan's
    anchor, and the problem.
- A session Mohammed opens on another thread for a second opinion is never an owner thread, unless
  it writes the plan.

## 5. Owners

| Rule | Owner |
|---|---|
| A thread's anchor | `Design` |
| What the context holds, its layers, budget and cuts | `Turns::planning_context` |
| Whether a session has heard the current context | `Harness` (the heard record) |
| The record of what each turn carried | `Harness` (invocations) |
| A plan's owner thread | `Plans` |
| Reads of parts, outcomes, agreements, revisions | `Design` (existing methods, plus any missing read) |

No new service.

## 6. Order of building

1. **The anchor and the first-turn context:** anchor on a thread; the index and detail compiled
   and sent on a session's first turn; the record per invocation. Run it in the browser on a real
   project.
2. **Re-send after a compaction:** a completed compaction (manual or automatic) clears the
   heard digest; the "send again" action. (Re-sending on any change already ships in slice 1,
   through the digest.) A live `/compact` and a live automatic
   compaction through `shadows serve` confirm the evidence's read.
3. **The inspection view**, in the conversation (frontend).
4. **The owner thread:** `Plans::owner_thread`. It ships when messaging slice 3 (the manager)
   needs it.

## 7. Evidence

- `docs/evidence/2026-11-04-planner-context-probe.md`: how the adapter reports a compaction
  (§1), and the sizes measured on testing4 (§2) and on testing55 (§3). On testing55, after one
  plan, the index with the whole vision is about 1 260 tokens and the detail about 3 140, so the
  index uses the vision's purpose items.
- Both are well under §4.2's budgets on that project, so the budgets bind only on bigger
  projects. Slice 1's own record (§4.4) keeps measuring every context it sends.

## 8. Left to the builder

- The context block's exact wording and headings.
- Table and column names for the anchor and the invocation record.
- Whether the index lists parts beyond depth 2 when the tree is small.

## 9. Open

> **OPEN — a decisions record.** Decisions live in plan version rationales and standards. Whether
> a project needs its own decision log (Mohammed: "the vision and the decisions") is decided when
> the manager's slice needs to read them.

> **OPEN — the budgets.** §4.2's numbers are Claude's starting values. testing4 fits with room
> to spare (§7), so they are not yet tested at their limit. Closed when a project's recorded
> context first reaches a budget; the numbers are one constant block, so changing them is a
> one-line change.
