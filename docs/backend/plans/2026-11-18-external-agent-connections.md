# Global external agent connections implementation plan

**Spec:** [Global external agent connections](../specs/2026-11-18-external-agent-connections-design.md)

- [x] Core Grants owns the global credential’s lifecycle. Add a migration that revokes existing project credentials and preserves authorship. Verify issue, replay, revocation and transaction-local project binding. Keep Planner and executor scopes in their existing service paths.
- [ ] Route authenticated connections through MCP. Expose project discovery and add a per-call project selector to the external tool schemas. Bind the selected project for the call and restrict draft refs to it. Test two projects with one token, target mismatches and missing project input.
- [ ] Replace the HTTP project grant issue/list surface with a global one and regenerate OpenAPI and web types. Move connection management and client setup into Settings. Verify affected Rust and web suites. Live acceptance belongs to the user.
