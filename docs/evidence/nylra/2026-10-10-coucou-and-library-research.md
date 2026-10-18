# NYLRA and backend planning research

Date: 2026-10-10. Read-only source/documentation research, not runtime validation.
User chose Windows first, followed by mobile. Full product scope remains intact.
Library recommendations below are candidates until the existing architecture
prerequisites are exercised. No implementation plan or dependency lock is approved.

## Character inputs inspected

Both root PNGs were viewed: `AYLUL Companion Character Reference Board.png` and
`AYLUL Shadow Companion Design Sheet.png`. They show a dark hood, purple eyes and
crystals, six directional views, nine expression states and accessory ideas.
The second sheet adds flowing cloth/smoke and hanging ornaments. Neither is a
transparent production sprite atlas or animation rig. Preserve both originals.

Suggested base: the simpler reference-board silhouette for small desktop display,
with the richer design sheet informing larger views and later motion. Obtain
consistent transparent state assets with fixed canvas, pivot and scale; test at
actual small sizes. Idle, happy, listening, speaking, thinking, working, success,
blocked and sleeping are visual states, not authoritative domain outcomes.
Disconnected/reconnecting and pending confirmation also need honest UI indicators.

## Coucou inspected source

Repository snapshot: `08393b657c79e99382eaf15185f480e6745997b0`, resolved via GitHub
tree API. Read manifests, asset license and selected desktop/state source. No
Coucou build, Windows trial or deep security audit was performed.

- [Windows manifest](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/windows/package.json):
  TypeScript, Vite and Tauri; no React dependency in this manifest.
- [Rust manifest](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/windows/src-tauri/Cargo.toml):
  Tauri tray, single-instance, autostart and global-shortcut plugins plus native
  Windows integration. These are research leads, not dependencies to copy blindly.
- [Desktop logic](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/windows/src/mochi/desktop-logic.ts):
  separates lifecycle logic from drawing/native window effects through ports;
  useful for planning testable presence behavior. Asset-specific expressions,
  outfits and motion must not be treated as freely reusable character content.
- [Island state machine](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/windows/src/island/fsm.ts):
  inspected transitions and pinned-alert handling. Learn from the separation of
  compact/expanded/hidden state and pending interaction, without adopting its UI.
- [Native desktop window](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/windows/src-tauri/src/desktop.rs):
  candidate reference for window placement; the returned source exceeded the
  inspection output budget, so a complete behavioral review is still required
  before borrowing any implementation. Multi-monitor/DPI behavior needs local proof.
- [Relay manifest](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/relay/package.json):
  Cloudflare tooling. It is not evidence that its relay matches our Node/PostgreSQL
  topic, confirmation or durable-delivery requirements.
- [Asset license](https://github.com/Louis-CFM/coucou/blob/08393b657c79e99382eaf15185f480e6745997b0/LICENSE-ASSETS.md):
  source code and branded artwork have different licensing. Keep AYLUL, original
  icons and sounds. Any future code reuse requires the source license notice,
  exact provenance and a per-file review; no source or assets were copied here.

## Candidate choices and alternatives

| Area | First candidate | Alternative / decision test |
| --- | --- | --- |
| Desktop | React + TypeScript + Vite + Tauri 2 | Existing agreed direction; prove transparent window, focus, DPI, tray and reconnect on Windows |
| AYLUL rendering | Prepared raster states/atlases with existing browser drawing primitives | PixiJS if layered animation warrants it; Rive if authoring a rig preserves the desired style |
| UI state | Feature state plus server cache kept separate | Do not introduce a global store or state-machine package without a real state-complexity need |
| HTTP backend | Fastify with schema-derived OpenAPI | Hono/Express are comparison candidates, not adopted; prove streaming/cancel and schema compatibility |
| Persistence | PostgreSQL with Kysely and its PostgreSQL driver | Compare direct SQL/driver and Drizzle on real store/transaction needs; one migration owner, no custom ORM |
| Durable jobs | pg-boss using existing PostgreSQL | Compare a managed queue when deployment requires it; prove atomic enqueue and interrupted-worker recovery |
| Model transport | AI SDK Core behind our provider boundary | Direct provider SDK if required semantics are not preserved; prove real stream, abort and error behavior |
| Identity | Established OIDC implementation/provider | Select through device/browser login and revocation proof; never build a custom authentication protocol casually |
| Mobile | Flutter, with shared protocol artifacts | Remains a separate Dart client, scheduled after Windows; choose state/audio/push libraries during its capability proof |

Do not select exact versions from Coucou's manifest. Resolve and pin compatible
maintained versions when the candidate proof is approved. No library above has
been tested in these repositories yet.

Our own code should own source manifests, scoped context, topic transitions,
confirmation binding, command outcomes and AYLUL presentation state. It should
not recreate rendering engines, migration engines, queues, model SDKs or login.
Keep these as internal cohesive modules initially. Extract a package only for a
second real consumer. Publish versioned wire artifacts for TS/Dart rather than
sharing Rust domain logic or sibling source imports across repositories.

## Primary references consulted

- [Fastify encapsulation](https://fastify.dev/docs/latest/Reference/Encapsulation/).
- [Fastify OpenAPI plugin](https://github.com/fastify/fastify-swagger).
- [Kysely](https://kysely.dev/): typed SQL building; does not replace access control.
- [pg-boss](https://github.com/timgit/pg-boss): PostgreSQL jobs and transaction
  integration; domain idempotency and external side-effect reconciliation remain ours.
- [AI SDK cancellation](https://ai-sdk.dev/docs/advanced/stopping-streams) and
  [error handling](https://ai-sdk.dev/docs/ai-sdk-core/error-handling).
- [PixiJS sprites](https://pixijs.com/8.x/guides/components/scene-objects/sprite).
- [Rive runtimes](https://rive.app/docs/runtimes/getting-started).

## Coupled delivery outline for design review

NYLRA: reference assets -> Windows companion shell -> cloud topic/source display
-> Shadows-linked evidence -> trusted confirmation -> voice -> mobile continuity
-> production incident experience. Use real observation to trigger success/blocked
states; a completed animation never establishes a completed operation.

Backend: candidate capability proofs -> reviewed identity/protocol/data lifecycle
-> scoped links/context/topics -> durable conversation and replay -> enforced
assessment/confirmation integration -> voice/notifications -> team and production
extensions. Cloud runtime project rules continue to belong to Rust Shadows.

Joint gates: protocol fixtures and generated consumers; snapshot/replay boundary;
revoked project access; stale confirmation; identical retry; disconnected owner;
exact source/version provenance; real Windows and later phone acceptance.
Shadows work is a named dependency, not hidden implementation in either plan:
connector lifecycle, bounded context production and §24.6 enforcement must exist
before either client claims an integrated approval journey.
