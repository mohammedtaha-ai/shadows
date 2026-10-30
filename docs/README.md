# Documentation map

Documentation is routed by product/runtime ownership, then by the side doing the
work. AYLUL's conversations run on `AppCore` in the same daemon (2026-10-19); the
hook/sidecar, native shell and code intelligence keep their own runtimes.

| Area | Owns | Entry point |
|---|---|---|
| Shadows Core backend | Current Rust runtime, its services and API | [Backend](backend/AGENTS.md), [stages](backend/stages.md), [code map](codebase/README.md) |
| Shadows Core frontend | Existing `web/` Planner/project workspace | [Frontend](frontend/AGENTS.md), [code map](frontend/codebase/README.md) |
| AYLUL | The assistant: brain design, character, native shell and hook | [AYLUL](aylul/README.md) |
| Code intelligence | Independent semantic service and its package/evaluation | [Code intelligence](code-intelligence/README.md) |
| Integration | Agreements crossing product/runtime boundaries | [Integration](integration/README.md) |
| Evidence | Dated facts grouped by originating runtime/slice | [Evidence directory](evidence) |
| Future lists | What is meant and not built yet, one line each; decides nothing | [Shadows](future.md), [AYLUL](aylul/future.md) |
| Earlier Core decisions | Existing authoritative section owners; amended in place | [Spec index](superpowers/specs/README.md) |

`docs/backend/` and `docs/frontend/` remain specific to Shadows Core and its
existing web client. AYLUL and standalone package work must not be placed there
merely because it contains a backend or a screen.

New product-scoped specs/plans live under `docs/aylul/{specs,plans}/` or
`docs/code-intelligence/{specs,plans}/`. Current Core specs/plans keep their
backend/frontend paths. Source `contract.yaml` files stay next to actual code;
documentation moves do not invent modules or change their implementation.

AYLUL's desktop code is under `apps/desktop/`; its vision, maps and
character/native/UI owners are under `docs/aylul/`. Historical acceptance is
under `docs/evidence/nylra/` at `0187365`. Documents superseded on 2026-10-19 stay in Git at
`226d0a3`.
