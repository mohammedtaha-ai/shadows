# Cross-runtime agreements

This area routes agreements between AYLUL, code intelligence, Core and the remote
brain. Each operation/data format retains one implementation/contract owner.
An integration document links those owners rather than defining their APIs twice.

2026-10-19 direction: the human selected a product daemon with optional planning
and an independently available Hermes-backed assistant. The
[product-daemon design](2026-10-19-product-daemon-hermes-design.md) is pending
written review; no source move or routing cutover has occurred. It proposes
keeping the Hermes client in TypeScript and cloud account services separate.

Start with [runtime cores and endpoint disposition](2026-10-18-runtime-endpoints.md).
The currently implemented NYLRA chat uses shadows-backend as one public entry; its private Hermes client uses
the installed native API. Local Docker reuses Windows Hermes; server deployment
changes that internal target. Core, AYLUL and code intelligence remain independent.

Core web-to-backend requests continue through the existing
[Core shared-request workflow](../shared/README.md). Future agreements crossing
product/runtime boundaries belong to this area with an explicitly named caller,
owner, schema and acceptance. Do not conflate them with an ordinary `web/` screen.

The current integration design is
[AYLUL's interruption-to-direction path](../aylul/specs/2026-10-18-aylul-hermes-semantic-context-design.md).
Its Core projection/control requirements still need their scoped caller/authority
contracts. No new request directory or schema is scaffolded before that need is addressed.
