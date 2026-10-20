# AYLUL documentation

AYLUL is Mohammed's assistant: it stands between him and his agents (Claude,
Codex, Gemini) and later handles personal tasks. Its desktop code lives in
`apps/desktop/`.

2026-10-19 direction: AYLUL's brain is Claude Code in its own home folder,
run by Shadows core inside the same product daemon and database, shown in the
Shadows web client. Hermes is no longer AYLUL's brain. Order of work: brain,
work profile (watching agents through hooks), personal profile, voice; remote
alerts later.

- [Brain design](specs/2026-10-19-aylul-claude-code-brain-design.md): the brain, providers and agents.
- [Desktop and companion design](specs/2026-10-20-aylul-desktop-web-companion-design.md):
  approved shared web main, automatic local Core startup and a character bound
  to a changeable agent.
- [Desktop implementation plan](plans/2026-10-20-aylul-desktop-web-companion.md):
  packaged dashboard, retained native functions and agent-bound presence;
  Tasks 1–9 implemented with passed local gates and actual-profile readiness.
  Mohammed performs the assistant/native journey; final reviewer waived
  ([evidence](../evidence/aylul/2026-10-20-desktop-web-companion.md)).
- [Isolated-home probe](../evidence/aylul/2026-10-19-claude-code-isolated-home.md): its measured facts.
- [Vision](vision.md): the full product idea. Decides nothing.
- [Laptop hook design](specs/2026-10-13-aylul-hook-laptop-design.md): what AYLUL learns from Claude Code on the laptop.
- [Product daemon](../integration/2026-10-19-product-daemon-hermes-design.md): the host process.
- [Code map](codebase/README.md): what each implemented desktop module owns.
- Owners: [character](character/AGENTS.md), [native](native/AGENTS.md), [desktop UI](ui/AGENTS.md).

Superseded documents (the Hermes brain, the NYLRA transition, the desktop
workspace UI, older delivery plans) were deleted on 2026-10-19 and stay in Git
history at `feat/agent-workspace` commit `226d0a3`. The workspace UI code
(U-W1 to U-W4) has been replaced by the shared `web/` dashboard under the
approved desktop and companion design above. Dated
evidence stays under `docs/evidence/aylul/` and `docs/evidence/nylra/`.
