# Documentation map

Documentation is routed by product, then by the side doing the work. AYLUL's
conversations run on `AppCore` in the same daemon (2026-10-19); the hook/sidecar,
native shell and code intelligence keep their own runtimes.

| Area | Owns | Entry point |
|---|---|---|
| Shadows Core | The vision, the future list, the delivery roadmap and the earlier decisions §0–§24, amended in place | [Vision](core/vision.md), [future list](core/future.md), [roadmap](core/roadmap.md), [spec index](core/specs/README.md) |
| Shadows Core backend | The Rust runtime, its services and API: newer specs, plans, stages | [Backend](backend/AGENTS.md), [stages](backend/stages.md), [code map](codebase/README.md) |
| Shadows Core frontend | The `web/` client: newer specs, plans, stages | [Frontend](frontend/AGENTS.md), [stages](frontend/stages.md), [code map](frontend/codebase/README.md) |
| Shared requests | What crosses between the web client and Core, agreed before either side builds | [Shared](shared/README.md) |
| AYLUL | The assistant: brain, agents and panel, character, native shell and hook | [AYLUL](aylul/README.md) |
| Code intelligence | An independent code-index package: its design and plan, no package yet | [Code intelligence](code-intelligence/README.md) |
| Evidence | Dated facts. The folder holds only its rules; every record is in Git, cited as `<path> at <commit>` | [Evidence rules](evidence/README.md) |

`docs/backend/` and `docs/frontend/` are specific to Shadows Core and its web
client. AYLUL and standalone package work must not be placed there merely because
it contains a backend or a screen.

New specs and plans live with their product and side: `docs/backend/`,
`docs/frontend/`, `docs/aylul/<side>/` or `docs/code-intelligence/`.
`docs/core/specs/` is amended, never added to. Source `contract.yaml` files stay
next to the code.

Documents superseded on 2026-10-19 stay in Git at `226d0a3`; the Hermes
integration design and the other files removed on 2026-11-02 are at `f1459dcc`.
