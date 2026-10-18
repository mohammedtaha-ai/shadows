# AYLUL documentation

AYLUL is the primary assistant experience. It can open the existing Shadows
workspace on request and also observe direct coding-agent work. Its native/local
runtime and remote Hermes brain do not become services inside Shadows `AppCore`.

- [Current integration design](specs/2026-10-18-aylul-hermes-semantic-context-design.md)
- [Integration plan](plans/2026-10-18-aylul-hermes-semantic-context.md)
- [Vision/document/test transition](plans/2026-10-18-nylra-transition.md)
- [Existing NYLRA vision](../../../nylra/Vision.md)
- [Existing code map](../../../nylra/docs/codebase/README.md)
- [Existing character design owner](../../../nylra/docs/character/AGENTS.md)
- [Existing native owner](../../../nylra/docs/native/AGENTS.md)
- [Existing desktop UI owner](../../../nylra/docs/desktop-ui/AGENTS.md)

After a reviewed source/document relocation, this product's UI, native and
character documentation belongs under `docs/aylul/ui/`, `docs/aylul/native/` and
`docs/aylul/character/` respectively. Those are destinations, not additional
copied owners today. Character appearance/renderer/studio ownership stays distinct
from model orchestration, local command authority and the native window owner.

The former backend-compatible relay/client continuation is not the active path
after the human's 2026-10-18 direction change. Completed implementation, tests and
dated acceptance remain valid for their recorded scope; unfinished old tasks are
not continued just to complete the superseded roadmap.
