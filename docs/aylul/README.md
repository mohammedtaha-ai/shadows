# AYLUL documentation

AYLUL is the selected primary assistant experience. Opening the existing Shadows
workspace on request and explaining direct coding-agent work are the integration
direction; they are not implemented by source relocation. Its native/local runtime
and remote Hermes brain do not become services inside Shadows `AppCore`.

- [Current integration design](specs/2026-10-18-aylul-hermes-semantic-context-design.md)
- [Integration plan](plans/2026-10-18-aylul-hermes-semantic-context.md)
- [Vision/document/test transition](plans/2026-10-18-nylra-transition.md)
- [Existing NYLRA vision](vision.md)
- [Existing code map](codebase/README.md)
- [Existing character design owner](character/AGENTS.md)
- [Existing native owner](native/AGENTS.md)
- [Existing desktop UI owner](ui/AGENTS.md)

On 2026-10-18 the tracked desktop, sidecar, hook, tests and contracts were imported
to `apps/desktop/`. This is the active AYLUL development location. Its UI, native
and character documentation now lives under `ui/`, `native/` and `character/`.
Character appearance/renderer/studio ownership stays distinct from model
orchestration, local command authority and the native window owner.

The [file manifest](../evidence/aylul/2026-10-18-nylra-relocation-manifest.json)
records original paths/hashes and retained items. The original NYLRA Git checkout
and ignored art archive remain intact for provenance and recovery. Historical
reports were carried byte-for-byte to `docs/evidence/nylra/`; they are not fresh
acceptance. See the [desktop build instructions](../../apps/desktop/README.md).
The [post-import acceptance](../evidence/aylul/2026-10-18-nylra-relocation.md)
separates local tests, real native checks and remaining physical limits.

The former backend-compatible relay/client continuation is not the active path
after the human's 2026-10-18 direction change. Completed implementation, tests and
dated acceptance remain valid for their recorded scope; unfinished old tasks are
not continued just to complete the superseded roadmap.
