# Independent Code Intelligence Service

Date: 2026-10-18.
Status: Runtime independence agreed in conversation; wire/implementation design
is proposed for review. No replacement product has been implemented.

This file owns the standalone service boundary. AYLUL's use of it in the
explanation journey belongs to AYLUL's later work-profile design. Existing Shadows section 15 continues to
own the implemented `Code` service and `shadows-index`; this document does not
silently replace them.

## 1. Outcome

A person installs and runs this package independently, then connects Claude,
Codex or another supported MCP client to a selected project. The same service
can serve AYLUL and Shadows Core when either is running. It requires neither
application to start, answer a query or remain alive.

Reuse mature language servers and client libraries. Do not implement language
analysis, a compiler, a universal project database or a general agent runtime.
The first capability is bounded read-only code understanding. Editing and shell
execution are not part of this delivery.

## 2. Composition roots

| Runtime | Composition owner | Data/lifetime |
|---|---|---|
| Shadows Core | Existing binary startup builds `AppCore` once | Own project database, rules, journal and managed agent processes |
| AYLUL desktop | Existing Tauri startup plus sidecar `main.ts` | Own UI/observation/dispatch state and local agent connectors |
| Code intelligence | Its own package entry point | Workspace registration, derived semantic state, language-server workers and its listeners |
| Hermes | Upstream runtime | AYLUL model sessions/memory on the server |

There is no parent `AppCore` or service locator for all four. An executable wires
the dependencies it actually needs. Client code may depend on a narrow wire
contract; it does not import another application's composition root or database.

The code-intelligence package must build/start without importing `shadows-core`,
AYLUL or Hermes. Core's client must not require desktop/native packages. Tests
must prove this through independent installation/startup, not only import scans.

## 3. Workspace ownership

An authorized launch/registration associates an opaque workspace ID with an
absolute canonical project root. Every shared-service request identifies its
workspace explicitly. A model-supplied cwd is not authority to add a root.

- Different project roots remain different workspaces. Worktree-specific support
  and acceptance are deferred by human instruction, not prerequisites for this pilot.
- Paths in requests/results are relative to the selected root. Canonical path
  checks reject escapes through parent paths, symlinks or Windows junctions.
- Same-named symbols are resolved within the root and source file. Ambiguous
  results remain multiple candidates; the service never selects by recency.
- A workspace generation changes on removal/re-registration or backend reset.
  In-flight results from an older generation are rejected before disclosure.
- No global mutable active-project selection is shared across unrelated clients.
  A Serena reference adapter therefore needs fixed-root instances for different
  simultaneously active roots; another implementation may use an equivalent
  workspace-safe mechanism its client library supports.

AYLUL's permission to disclose a returned source excerpt to Hermes remains
AYLUL's responsibility. Registering local code intelligence does not grant cloud
sharing. Core's own project sharing and mutation checks remain Core's rules.

## 4. Minimal application interface

Proposed operations are capability-scoped; final generated names/schemas are
pinned after the baseline rather than copied blindly from Serena:

| Operation | Input identity | Result |
|---|---|---|
| Workspace status | workspace ID + generation | ready, starting, unavailable or removed; available languages/capabilities |
| File outline | workspace + relative file | bounded symbol tree with kinds and source positions |
| Find symbol | workspace + qualified name + optional relative file | exact match or explicit candidates, optional bounded body |
| Find references | workspace + selected symbol handle | resolved reference locations, distinct from surrounding context lines |
| Read source | workspace + relative file/range | bounded text, digest and explicit truncation |
| Diagnostics | workspace + relative file | backend-reported diagnostics; absence is not proof that tests passed |
| Cancel request | client/request identity | cancellation of that request, not termination of another client's worker |

Every response states the workspace generation, backend/capability used,
freshness status, omissions and applied result limits. Semantic and name-only
matches must be distinguishable. A symbol handle is scoped to a workspace and
source revision; it is not promised to survive arbitrary edits or resets.

Code evidence can carry line numbers for presentation. Revalidation uses the
source digest/selected symbol identity, never a stored line number alone.
If a language server lacks a capability, return unsupported rather than replacing
semantic results with text matches under the same label.

## 5. Transport and lifecycle

Support two separately tested modes when the implementation is approved:

1. **Client-owned stdio:** the coding client starts one fixed-workspace MCP
   process and owns its shutdown. No daemon or Core process is required.
2. **Shared local service:** a separately launched service accepts MCP clients
   through a configured loopback/private endpoint and owns its workers. A client
   disconnect or Core shutdown does not terminate the shared service.

A client may release its workspace lease. The service closes an idle worker only
when no remaining client/request needs it and its configured idle policy permits.
Request IDs are client-scoped; one client's cancellation cannot affect another.

The first package can be started manually. Installing a Windows service or
auto-start task is not required to demonstrate independence and is not implicitly
authorized. No public listener is enabled by default. Shared mode must authenticate
clients and authorize each workspace; loopback/CORS alone is not workspace authority.
Reuse an existing MCP implementation and LSP client rather than writing protocol stacks.

## 6. State and failures

Service state is derived from source plus explicit local workspace registration.
Caches can be rebuilt. It owns no Shadows plans, agent conversations, approval
records or cloud memory, and never opens their databases.

| Condition | Expected behavior |
|---|---|
| Core is not running | Direct MCP clients and AYLUL queries continue |
| AYLUL quits | Independently launched shared service continues for other clients |
| A stdio client exits | Its owned process closes; other service instances remain |
| Language server dies | Affected queries return unavailable; bounded recovery does not repeat a write |
| File changes during lookup | Return a checked revision or stale outcome, not a fabricated current citation |
| Workspace revoked mid-query | Release no result after revocation |
| Service stops | Queries fail visibly; running agent sessions and Core are not stopped |

Status and diagnostics must be observable without raw source/secrets in logs.
Deployment credentials are references/native secret storage concerns; a process
argument or generated source contract is not a place to put secret values.

## 7. Package layout and dependencies

The previous monorepo tree is a placement proposal, not an accepted migration.
Current repositories stay where they are while the first path is proven.
`packages/code-intelligence/` is a possible independent package location; a
separate repository is equally valid if independently owned releases warrant it.

Before replacement implementation starts, record one selected language, location,
package manifest/lock strategy, runtime dependencies and minimal source map.
Do not create both a Python service and a Rust semantic implementation to postpone
the choice. Keep `shadows-index` parser-only unless its existing owner is explicitly amended.

Regardless of placement, each application has its own build/test/release commands.
A root convenience command may compose them, but independent package installation
cannot pull in AYLUL assets, the Core database, Hermes or unrelated language servers.

## 8. Serena learning baseline

[The executed baseline](../../evidence/code-intelligence/2026-10-18-serena-baseline.md) proves
one synthetic Rust lookup/reference path on Windows, not a replacement architecture.
Serena remains installed at `F:/projects_frok/serena` with a pinned source revision.
Its live tool schemas/raw outputs are research artifacts outside product source.

Before choosing the implementation, compare the needed behavior with Serena,
standalone SolidLSP and an existing native LSP client integration. Check dependency
and licensing boundaries for the actual version; do not treat GPL Serena source
as MIT because SolidLSP has a different license. Keep provenance with reused code.

Measure correct reference locations, two-root isolation, changed-file freshness,
Windows lifecycle, returned bytes and cold/warm latency. Improvement over Serena
is a measured result, not a product claim assumed by the design.

### 8.1 Real-project checkpoint and provisional recommendation

[The real Shadows baseline](../../evidence/code-intelligence/2026-10-18-serena-real-shadows.md)
proved selected Rust/TypeScript outlines, exact bodies, same-name reference
separation, bounded-result behavior and fixed read-tool scope. Two separately
owned stdio instances survived peer cancellation/disconnect without Core or AYLUL.
The two roots were nested; distinct-worktree isolation, edited-source freshness,
server-side cancellation acknowledgement and shared-daemon lifetime remain unproved.

Keep Serena as the learning reference. Do not approve an extracted standalone
SolidLSP package solely from its MIT label: in the pinned checkout `solidlsp/ls.py`
imports GPL-tagged `serena.util.file_system` and `serena.util.text_utils`.
The evidence records exact paths/version. Resolve that dependency boundary before
describing its distributable runtime as MIT-only.

Rust `async-lsp` is a source-inspected client-framework candidate, not an executed
replacement. No package language/location is selected by this checkpoint. CI-3
must combine the remaining isolation/freshness/lifecycle evidence with a resolved
dependency boundary before producing its implementation plan.

## 9. Acceptance and next gate

Acceptance requires the package to be used by a direct MCP client while both Core
and AYLUL are stopped, then by two clients with distinct workspaces without crossed
results. Also test same-named symbols, client-specific cancellation, root revocation,
worker failure and reinstall/start from an independent package artifact.

The next executable work is the
[baseline/contract-selection plan](../plans/2026-10-18-code-intelligence-baseline.md).
It produces evidence and an implementation choice, not a speculative new service.
Replacement code starts only after its resulting package/contract plan is reviewed.
