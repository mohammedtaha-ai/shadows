# AYLUL documentation scope

Read [README.md](README.md), [vision.md](vision.md) and
[source-structure.md](source-structure.md). The source map is
[codebase/README.md](codebase/README.md); desktop source lives in
`apps/desktop/`, not in Core crates or `web/`.

`character/`, `native/` and `ui/` retain separate design, plan and stage owners.
`shared/` retains AYLUL's accepted cross-side requests. `specs/` contains the
relocated active delivery/hook owners and current integration design;
`history/plans/` preserves older top-level sequencing.
Historical acceptance is under `docs/evidence/nylra/`; do not rewrite dated facts.
Amend a decision in its owner rather than copying it into another document.
Follow [desktop instructions](../../apps/desktop/AGENTS.md) for implementation.

The source NYLRA checkout is preserved as migration provenance. New AYLUL work
belongs here. No worktree, rewrite, Hermes connection or remote operation follows
automatically from this relocation.
