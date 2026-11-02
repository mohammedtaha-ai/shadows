# AYLUL documentation

AYLUL is Mohammed's assistant: it stands between him and his agents (Claude,
Codex, Gemini) and later handles personal tasks. Its desktop code lives in
`apps/desktop/`.

2026-10-19 direction: AYLUL's brain is Claude Code in its own home folder,
run by Shadows core inside the same product daemon and database, shown in the
Shadows web client. Hermes is no longer AYLUL's brain. The order of work is in
[brain §6](specs/2026-10-19-aylul-claude-code-brain-design.md#6-decided-by-mohammed).

Specs (one owner per decision):
- [Brain design](specs/2026-10-19-aylul-claude-code-brain-design.md): the brain, providers and agents. Built on main (PRs #35-#37).
- [Agents and panel design](specs/2026-11-01-aylul-agents-and-panel-design.md): agent instructions, MCP tools, outfits, the panel and permission requests. Built (PR #49, `b40e077b`).
- [Desktop and companion design](native/specs/2026-10-20-aylul-desktop-web-companion-design.md): shared web main, automatic local Core startup, and a character bound to a changeable agent. Built on main (PRs #38, #39).
- [Laptop hook design](native/specs/2026-10-13-aylul-hook-laptop-design.md): what AYLUL learns from Claude Code on the laptop. Built.
- [Independent companion design](native/specs/2026-10-16-independent-companion-design.md) and [desktop architecture](native/specs/2026-10-26-desktop-architecture-design.md): native owners.
- [In-window character design](character/specs/2026-10-16-aylul-in-window-design.md): the drawing and studio.

Other references:
- The desktop implementation plan (`docs/aylul/plans/2026-10-20-aylul-desktop-web-companion.md` at `0187365`) is deleted from the tree; its evidence is `docs/evidence/aylul/2026-10-20-desktop-web-companion.md` at `0187365`.
- Isolated-home probe (`docs/evidence/aylul/2026-10-19-claude-code-isolated-home.md` at `0187365`): its measured facts.
- [Vision](vision.md): the full product idea. Decides nothing.
- [Future list](future.md): what is meant and not built yet, one line each, linked to the vision.
- Product daemon (`docs/integration/2026-10-19-product-daemon-hermes-design.md` at `f1459dcc`): the superseded
  Hermes host, removed on 2026-10-29.
- [Code map](codebase/README.md): what each implemented desktop module owns.
- Shared rules and owners: [AGENTS](AGENTS.md); [character](character/AGENTS.md), [native](native/AGENTS.md), [desktop UI](ui/AGENTS.md).

Superseded documents (the Hermes brain, the NYLRA transition, the desktop
workspace UI, older delivery plans) were deleted on 2026-10-19 and stay in Git
history at `feat/agent-workspace` commit `226d0a3`. The workspace UI code
(U-W1 to U-W4) has been replaced by the shared `web/` dashboard under the
approved desktop and companion design above. Dated
evidence stays under `docs/evidence/aylul/` at `0187365` and `docs/evidence/nylra/` at `0187365`.
