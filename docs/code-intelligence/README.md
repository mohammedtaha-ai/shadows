# Code intelligence documentation

This area owns the independently usable code-intelligence package. Core, AYLUL
and direct coding clients are consumers; package startup/lifetime does not require
any of them. Runtime/backend selection remains evidence-driven.

- [Service design](specs/2026-10-18-code-intelligence-service-design.md)
- [Stages](stages.md)
- Executed Serena Rust smoke (`docs/evidence/code-intelligence/2026-10-18-serena-baseline.md` at
  `0187365`)

Serena was installed as a learning baseline, not a completed replacement package. The lab's
selection (CI-1 to CI-3) was overtaken: Core runs the official language servers itself since
PR #67 ([language servers design](../backend/specs/2026-11-13-language-servers-design.md)). The
lab plan and its stages are in Git at `e78e07dc`.
No empty source package or new Core service is created by these documents.
