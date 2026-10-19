# Cross-runtime agreements

This area routes agreements between AYLUL, code intelligence and Core. Each
operation/data format retains one implementation/contract owner. An integration
document links those owners rather than defining their APIs twice.

- [Product daemon](2026-10-19-product-daemon-hermes-design.md): the host process
  that runs optional planning and AYLUL. Its Hermes assistant is implemented but
  no longer AYLUL's direction.
- [AYLUL brain](../aylul/specs/2026-10-19-aylul-claude-code-brain-design.md):
  Claude Code in its own home, run by Shadows core in the same daemon.
- [Daemon delivery evidence](../evidence/aylul/2026-10-19-product-daemon-hermes.md).

Core web-to-backend requests continue through the existing
[Core shared-request workflow](../shared/README.md). Do not conflate a
cross-runtime agreement with an ordinary `web/` screen.

The runtime-endpoint table, local handoff and daemon plan were deleted on
2026-10-19 and stay in Git `226d0a3`.
