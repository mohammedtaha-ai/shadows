# Code intelligence stages

2026-10-18: the synthetic Rust smoke and a real Shadows Rust/TypeScript baseline
are recorded. The latter passed 29 selected assertions; CI-2 acceptance remains
partial. No replacement package implementation has started. This task list is
independent of Core backend stages.

## CI-1 - Complete the Rust/TypeScript baseline

- Status: complete for the approved real-project sample
- Plan: docs/code-intelligence/plans/2026-10-18-code-intelligence-baseline.md#ci-1---pin-schemas-and-complete-the-rusttypescript-behavior-baseline
- Read: docs/code-intelligence/specs/2026-10-18-code-intelligence-service-design.md; docs/evidence/code-intelligence/2026-10-18-serena-baseline.md.
- Touch: dated evidence under docs/evidence/code-intelligence/; temporary isolated lab artifacts outside product source.
- Needs: none for the already requested learning baseline; no agent/project settings or product source changes.
- Done when: exact-schema and same-name-reference cases have raw reports and honest outcomes without model calls.
- Evidence: docs/evidence/code-intelligence/2026-10-18-serena-real-shadows.md; actual Rust `Projects.create`/`Threads.create` and TypeScript `ThreadStream.close`/`EventSourceLike.close`, bodies, references, empty/ambiguous/capped results and read-scope refusal. Source hashes unchanged; no model calls.

## CI-2 - Independence and lifecycle acceptance

- Status: partial
- Plan: docs/code-intelligence/plans/2026-10-18-code-intelligence-baseline.md#ci-2---measure-independence-two-roots-and-lifecycle-behavior
- Evidence: docs/evidence/code-intelligence/2026-10-18-serena-real-shadows.md.
- Proven: fixed stdio instances at the real repository root and nested web root; cancellation at one client did not prevent the other's query; surviving instance answered after peer disconnect; no Core/AYLUL process required; no owned child remained.
- Pending for the selected implementation mode: changed-source freshness, project isolation, cancellation/lifetime ownership and recovery. Shared-worker acknowledgements, shared daemon lifetime and revocation apply only if that mode is selected. Worktree-specific tests are deferred by human instruction and do not block this pilot; the nested-root result remains limited to its recorded scope.

## CI-3 - Select one independently installable implementation

- Status: blocked
- Plan: docs/code-intelligence/plans/2026-10-18-code-intelligence-baseline.md#ci-3---select-one-implementation-and-hand-off-a-concrete-package-plan
- Read: service design and CI-1/CI-2 evidence.
- Touch: the service owner spec and resulting package plan; no speculative source scaffolds.
- Needs: CI-1 plus the CI-2 root-isolation/lifecycle results and reviewed dependency/license boundaries.
- Current comparison: Serena executed; SolidLSP and native async-lsp source/manifests inspected only. Pinned SolidLSP imports GPL-tagged Serena utilities; standalone MIT packaging is not verified. No implementation/language/location has been selected.
- Done when: one backend/language/location and a reviewable package implementation plan are selected.
