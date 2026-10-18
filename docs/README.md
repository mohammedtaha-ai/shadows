# Documentation map

Documentation is routed by product/runtime ownership, then by the side doing the
work. Sharing a repository does not make every runtime a service of `AppCore`.

| Area | Owns | Entry point |
|---|---|---|
| Shadows Core backend | Current Rust runtime, its services and API | [Backend](backend/AGENTS.md), [stages](backend/stages.md), [code map](codebase/README.md) |
| Shadows Core frontend | Existing `web/` Planner/project workspace | [Frontend](frontend/AGENTS.md), [code map](frontend/codebase/README.md) |
| AYLUL / NYLRA | Assistant experience, native/sidecar integration and document transition | [AYLUL](aylul/README.md) |
| Code intelligence | Independent semantic service and its package/evaluation | [Code intelligence](code-intelligence/README.md) |
| Integration | Agreements crossing product/runtime boundaries | [Integration](integration/README.md) |
| Evidence | Dated facts grouped by originating runtime/slice | [Evidence directory](evidence/) |
| Earlier Core decisions | Existing authoritative section owners; amended in place | [Spec index](superpowers/specs/README.md) |

`docs/backend/` and `docs/frontend/` remain specific to Shadows Core and its
existing web client. AYLUL and standalone package work must not be placed there
merely because it contains a backend or a screen.

New product-scoped specs/plans live under `docs/aylul/{specs,plans}/` or
`docs/code-intelligence/{specs,plans}/`. Current Core specs/plans keep their
backend/frontend paths. Source `contract.yaml` files stay next to actual code;
documentation moves do not invent modules or change their implementation.

NYLRA's existing vision, character/native owners and dated acceptance currently
remain in its source repository. The [transition manifest](aylul/plans/2026-10-18-nylra-transition.md)
specifies their eventual destinations and reference/provenance checks. Do not
copy them into a second active owner before that transition is executed.
