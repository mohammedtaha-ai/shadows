## 0. Design Principles
1. **Runnable vertical slice first.** A user-visible end-to-end path is delivered before building later platform layers.
2. **Modular monolith first.** Module boundaries are cheap; crate boundaries are expensive. Split crates only for a concrete build, distribution, reuse, or compile-time isolation reason.
3. **Library-first, not dependency-first.** Prefer mature libraries where they solve the problem; do not add wrappers or dependencies without a real need.
4. **Abstract volatility, not possibility.** Introduce seams where change is already known: storage backend, agent harness, process runtime, protocol adapter, secrets resolution.
5. **Durable current state + durable journal.** Entity/state tables are authoritative current truth. The durable event journal provides history, provenance, replay, and resync. Shadows is **not** full event sourcing.
6. **Native agent sessions are caches, not truth.** Claude/Codex-native session IDs are optional optimization metadata.
7. **Planner ≠ Executor ≠ Reviewer ≠ Orchestrator.**
8. **Role ≠ Harness ≠ Provider ≠ Model.**
9. **The model cannot authorize itself.** Authority comes from Shadows policy/configuration.
10. **Frozen workflow versions are immutable.** Design changes create a new version; old versions are never edited in place.
11. **Every side effect is attributable.** Durable work carries operation/task/runtime/actor/causation provenance as applicable.
12. **Execution is not completion.** Deterministic verification and workflow gates decide completion.
13. **Repository content is untrusted input.**
14. **Ordering is explicit.** Never use physical insertion order or SQLite `rowid` as a domain ordering key.
15. **Secrets are references until spawn.** Secret values never live in durable config/state or logs.

---
