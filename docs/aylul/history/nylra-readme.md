# NYLRA

Cloud-backed human liaison with a desktop companion and a mobile application.
AYLUL is its companion character.

**Status:** Product vision and agreements recorded. A Windows main shell and independent AYLUL companion with
local native-login primitives are implemented in [apps/desktop](../../../apps/desktop/README.md).
Backend auth-v1 login is wired into Rust with protected enrollment/session storage;
see [joint authentication evidence](../../evidence/nylra/2026-10-11-joint-native-auth.md).
The procedural companion uses local Canvas source with native visibility and motion controls.
Full device/performance acceptance remains separately recorded. The earlier
[continuation report](../../evidence/nylra/2026-10-11-windows-shell-continuation.md) is historical.
No conversations, mobile or Production features are implemented.

[AYLUL artwork](../../../assets/aylul/README.md) points to the current procedural renderer
and the ignored local archive of earlier image experiments. Historical dated
reports retain their original asset references.

## Start here

| Document | Owns |
| --- | --- |
| [Vision](../vision.md) | The complete companion experience and future capabilities |
| [Delivery spec](../specs/2026-10-10-nylra-delivery-design.md) | AYLUL, Windows-first experiments and client acceptance gates |
| [Initial plan](plans/2026-10-10-nylra-initial-plan.md) | Experiment-first tasks, revised from results before dependent work |
| [Source structure](../source-structure.md) | Prepared desktop/mobile layout and dependency boundaries |
| [Contract convention](../../../../shadows-backend/docs/contracts/README.md) | Applying source contracts to implemented modules |
| [Shared architecture](../../../../shadows-backend/docs/architecture.md) | Languages, hosting, integration paths, scoped context, and authority boundaries |
| [Agent instructions](../../../apps/desktop/AGENTS.md) | Source ownership, contracts, and verification workflow |
| [Source contract template](../../../../shadows-backend/docs/contracts/TEMPLATE.yaml) | The evidence-based structure for describing an implemented source module |

The desktop direction is a TypeScript/React interface with Rust integration
inside Tauri 2. The mobile direction is Flutter/Dart. NYLRA's cloud logic belongs
to the shared TypeScript/Node.js application in `shadows-backend`.

Linking is optional per Shadows project. NYLRA follows the current work and
intervention topics, retrieves permitted sources, and explains them through its
own selected model. It does not replicate complete Planner histories.

Personal workspace authority stays on its owning computer; team authority belongs
to the future Rust Shadows server. Desktop/mobile sync continues NYLRA topics,
not local databases or live execution. See the
[sharing inventory](../../../../shadows-backend/docs/architecture.md#what-lives-where) and
[delivery order](../../../../shadows-backend/docs/architecture.md#14-delivery-order).

Sibling links assume repositories named `nylra`, `shadows-backend`, and
`shadows` are checked out beside each other.

Future production assistance is described in Vision section 46 and owned by
Shadows §17.12. The [dated research](../../evidence/nylra/2026-10-10-production-and-hackathon-research.md)
records AWS references and a separate optional hackathon opportunity; contest
requirements do not define the product.
