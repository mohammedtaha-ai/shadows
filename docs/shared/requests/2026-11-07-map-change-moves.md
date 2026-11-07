# A map change can move a part

- Status: accepted
- Asked by: controller, under Mohammed's instruction, 2026-11-07
- Owner specification: `docs/core/specs/2026-10-08-guided-planning-design.md` §23, map proposals ("Who writes it")

## Need

After the first map, the Planner can propose only content edits; `map_propose` refuses a change that names a parent ("a change edits content; use the page to move a part"). The layout is the Planner's and the person's call and may change during work, so a map change carries the part's parent, exactly as a roadmap change carries its outcome's parent.

## Fields

- `MapPropose` with `action: change` accepts `parts[0].parent` (`parent_key` stays refused). A change replaces the part's content and parent: the current parent keeps it in place, another part's id moves it under that part, and none moves it to the top level.
- `MapProposal` gains `old_parent: Option<PartId>`, the part's parent when the card was proposed, mirroring `RoadmapProposal.old_parent`.

## Errors

Existing error transport. A parent that is the part itself or one of its descendants, or a part of another project, is a named refusal at propose time. Approval applies the move through the same containment and folder rules as a hand move (`WorkspaceOp::PartMove`).

## Backend answer

Accepted: Design owns it (B-82). Regenerate OpenAPI and the web schema.

## Frontend reply

Accepted (F-95): the map change card shows "Moves to the top level" or "Moves under: <part title>" when `parts[0].parent` differs from `old_parent`, like the roadmap card's move line.
