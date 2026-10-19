# Documentation map

Documentation is routed by product/runtime ownership, then by the side doing the
work. Sharing a repository does not make every runtime a service of `AppCore`.

| Area | Owns | Entry point |
|---|---|---|
| Product daemon | Optional capability composition and local assistant lifetime | [Runtime design](integration/2026-10-19-product-daemon-hermes-design.md) |
| Shadows Core backend | Current Rust runtime, its services and API | [Backend](backend/AGENTS.md), [stages](backend/stages.md), [code map](codebase/README.md) |
| Shadows Core frontend | Existing `web/` Planner/project workspace | [Frontend](frontend/AGENTS.md), [code map](frontend/codebase/README.md) |
| AYLUL / NYLRA | Assistant experience, native/sidecar integration and document transition | [AYLUL](aylul/README.md) |
| Code intelligence | Independent semantic service and its package/evaluation | [Code intelligence](code-intelligence/README.md) |
| Integration | Agreements crossing product/runtime boundaries | [Integration](integration/README.md) |
| Evidence | Dated facts grouped by originating runtime/slice | [Evidence directory](evidence) |
| Earlier Core decisions | Existing authoritative section owners; amended in place | [Spec index](superpowers/specs/README.md) |

`docs/backend/` and `docs/frontend/` remain specific to Shadows Core and its
existing web client. AYLUL and standalone package work must not be placed there
merely because it contains a backend or a screen.

New product-scoped specs/plans live under `docs/aylul/{specs,plans}/` or
`docs/code-intelligence/{specs,plans}/`. Current Core specs/plans keep their
backend/frontend paths. Source `contract.yaml` files stay next to actual code;
documentation moves do not invent modules or change their implementation.

NYLRA's tracked desktop code is now under `apps/desktop/`; its active vision,
maps and character/native/UI owners are under `docs/aylul/`. Historical acceptance
is under `docs/evidence/nylra/`. The [transition plan](aylul/plans/2026-10-18-nylra-transition.md)
and per-file manifest record provenance and remaining acceptance. The original
checkout remains a preserved snapshot, not a second active development owner.
