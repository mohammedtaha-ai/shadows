# Code Intelligence Baseline and Contract Selection Plan

> **For agentic workers:** Execute this lab inline with
> `superpowers:executing-plans` when instructed to start. No subagents, product
> scaffolding, global agent configuration changes or replacement implementation
> are scheduled by this document.

**Goal:** Establish the smallest proven code-intelligence contract and select its
implementation/dependency boundary before building the independent package.

**Architecture:** Serena is the installed reference. The user approved the real
Shadows project instead of generated fixtures for this run. Read-only real-source
queries exercise MCP/LSP behavior without Core, AYLUL, an editor or model API calls.
Compare alternatives only far enough to make one evidence-backed choice.

**Tech Stack:** Installed Serena `d9e4c796fae0207ba6848d1df4b331122a5567b5`,
its Python 3.13.13 environment, existing Rust toolchain and real TypeScript sources.

**Spec:** [Independent service](../specs/2026-10-18-code-intelligence-service-design.md).

## Global constraints

- Use the authorized real Shadows root; generated fixtures are not required by the updated user request. Never read ignored `.env` values or real credentials. Do not edit real source to manufacture an assertion.
- No changes to current Core/native source, user MCP settings, global Git settings or production services.
- Keep observed behavior separate from desired behavior and documented-only capabilities.
- Preserve raw returned locations; surrounding snippet text is not a reference match.
- No model calls or token-saving claims in the MCP baseline.
- Temporary probe code stays outside product source and is removed after the evidence is recorded.
- No commit/push/branch changes are authorized by the documentation request.
- Worktree-specific validation is deferred by the human; it is not a gate for this pilot. NYLRA relocation is the current first priority.

## Execution checkpoint - 2026-10-18

[Real-project evidence](../../evidence/code-intelligence/2026-10-18-serena-real-shadows.md)
records 29/29 selected local assertions. CI-1 is complete for this sample. CI-2 is
partial: nested real roots and separate stdio instances were tested; distinct
worktrees, edited-source freshness and shared-worker lifecycle were not. CI-3 has
a static three-candidate comparison and a dependency-boundary finding, not a final
implementation choice. Temporary probes stayed outside product source.

Ruling: use existing real Rust and TypeScript same-name operations instead of
creating fixtures, per the user's explicit steering. Keep lifecycle assertions
requiring different real contents pending; do not turn nested roots into evidence
of distinct-worktree isolation. No production code or TDD feature cycle is involved.

## Review focus

| Input/failure | Assertion | Task |
|---|---|---|
| Same method name on unrelated types | Only actual selected-symbol reference locations count | CI-1 |
| Same relative path in two projects | Responses retain the correct root/generation | CI-2 |
| Cancellation from one client | Another client's request survives | CI-2 |
| File changed after lookup | Refresh or explicit stale result, not an old current citation | CI-2 |
| Shutdown of an unrelated application | Independent service query still works | CI-2 |

## Starting evidence

[2026-10-18 smoke](../../evidence/code-intelligence/2026-10-18-serena-baseline.md) already
proved one Rust outline/body/reference query, a four-tool advertised surface and
refusal of an unavailable shell tool. Do not repeat installation or claim that
this completes the following matrix. `tools.json` and `report.json` are retained
at `F:/projects_frok/serena-reference-artifacts/2026-10-18/`.

## CI-1 - Pin schemas and complete the Rust/TypeScript behavior baseline

**Files:** inspect the existing raw artifacts and pinned Serena tool definitions;
write actual dated outputs under `docs/evidence/code-intelligence/` after execution.
No package/source folders are created. Temporary fixture scripts are discarded.

**Produces:** a table mapping required operations to exact input schemas, selected
symbol naming, location conventions, result limits, unsupported outcomes and dependencies.

- [x] Run `F:/projects_frok/serena/.venv/Scripts/serena.exe --version` and verify the recorded commit/lock provenance.
- [x] Query real Rust `Projects.create` and `Threads.create`; compare their selected marked reference locations, not neighboring snippets.
- [x] Query real TypeScript `ThreadStream.close` and `EventSourceLike.close`; install/version language-server dependencies only in the isolated lab and distinguish installation from query success.
- [x] Initialize MCP with isolated `SERENA_HOME`, tools interface and single-project context; call `initial_instructions`, then outline, body and references. Persist actual schemas.
- [x] Check zero-based body/reference locations, empty/ambiguous queries and output caps. Preserve the initial failed TypeScript setup and failed one-based validation predicate.
- [x] Verify unlisted editing/shell/REPL tools refuse; 570 tracked source/manifest hashes remain unchanged.
- [x] Preserve raw output and dated evidence; delete temporary probe scripts after recording results.

**Verification:** isolated MCP clients completed; a source/result validator recorded
29 selected passing assertions in `validation.json`. Both raw reports exist.
No AYLUL/Core process was needed or started. This is local semantic-query evidence.

## CI-2 - Measure independence, two roots and lifecycle behavior

**Consumes:** the exact schemas/location rules established by CI-1.
**Produces:** capability matrix and observed limitations for the standalone package design.

- [ ] Use two authorized actual project roots to check isolation in the selected process mode. The recorded roots were the repository and nested `web/`; keep that evidence's limits. Use separately fixed-root instances; do not race global activation on one instance. Distinct-worktree testing is outside this pilot.
- [ ] Assert simultaneous client responses contain only the requested root's locations. Record how the runtime routes requests; do not claim single-process multi-project support if separate instances provided isolation.
- [ ] Close one client during a slow read. The run cancelled a pending client request and separately closed an idle peer; both surviving-client checks passed. Mid-read disconnect and server cancellation acknowledgement remain pending.
- [ ] Observe an authorized real edit and query again. Do not change product source solely for this experiment. Record freshness and latency; unchanged source digests alone do not prove refresh.
- [ ] Verify the independently launched process remains usable with Core and AYLUL absent. A dedicated stdio process may exit with its owning client; that is a different mode from a shared service.
- [x] Record startup, first-query and repeated-query samples plus returned byte counts. These are local code-query measurements, not end-to-end model/token measurements.

**Verification:** raw reports identify root/client/request IDs and contain assertions
for all selected cases; logs do not disclose user project content. Cleanup targets
only lab-owned children/temporary paths.

## CI-3 - Select one implementation and hand off a concrete package plan

**Reads:** CI-1/CI-2 evidence, the independent service spec, actual licenses/dependency
graphs of candidate versions. No candidate is chosen solely from a README claim.
**Writes:** amend the service owner spec in place and record the decision/evidence links.

- [ ] Compare Serena-as-process, standalone SolidLSP and one appropriate existing native LSP client. Distinguish source inspection from executed behavior for every candidate.
- [ ] Select one language/backend and one package location. Record why it meets the first consumers' needs and its measured cost/limitations. Do not implement three backends or a universal extension framework.
- [ ] Define the smallest wire contract for workspace status, outline, symbol resolution, references, bounded reads, diagnostics and cancellation; encode capability/freshness/ambiguity explicitly.
- [ ] Write the package implementation plan with exact files, public operations, RED/GREEN assertions, independent build/start commands and per-runtime lifecycle checks. Amend existing section 15/14 owners only if their implemented boundaries actually change.
- [ ] Review the resulting spec/plan before replacement code. Keep unproved behavior open with its closing test rather than representing the package as already available.

**Exit:** one selected, reviewable package plan and honest evidence. The AYLUL
integration can proceed against that contract after its own scope/authority gates;
the existing Core and direct coding workflow remain usable throughout.
