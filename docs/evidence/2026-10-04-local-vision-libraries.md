# Local vision — library checks, 2026-10-04

This records consulted interfaces and inspected dependencies. It does not
claim a completed executor, a new formatter gate or provider compatibility.

## Existing graph library

The workspace already declares petgraph 0.8; Plans' pure cycle check already
uses its `tarjan_scc` over a start/complete event graph. Cross-plan approval
will reuse this library and event semantics, rather than introduce another
graph implementation. Name-matched code-index references are not semantic
graph proof.

Primary interface: [petgraph SCC documentation](https://docs.rs/petgraph/latest/petgraph/algo/fn.tarjan_scc.html).
The implementation inspected is `crates/shadows-core/src/plans/rules.rs`.
The existing tests explicitly distinguish `needs` and `completes_after`.

## Web formatting candidate

[Oxfmt's official documentation](https://oxc.rs/docs/guide/usage/formatter.html)
describes a dedicated formatter supporting TS/TSX and the Web project's other
file types. The client already uses Oxlint, so Oxfmt is a candidate for a
consistent formatter/check workflow. It has not been installed or selected
by this research. Compatibility with this project's style and generated
declarations must be demonstrated before making it part of the gate.

## Execution boundary

[ACP filesystem methods](https://agentclientprotocol.com/protocol/v1/file-system)
let a client expose selected filesystem capabilities; an agent must inspect
the negotiated capabilities before using them. This is protocol behavior,
not proof that an arbitrary harness cannot write through its own shell or
native tools. Scope enforcement still requires the adapter-specific execution
trial required by §17/§19.

[ACP session modes](https://agentclientprotocol.com/protocol/v1/session-modes)
describe operating modes and current configuration-option evolution. A mode
label alone is not evidence of enforced write boundaries or of another
provider/model's compatibility. No credentials or real provider were exercised.
