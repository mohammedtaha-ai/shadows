# Serena baseline on the real Shadows repository

Date: 2026-10-18, Asia/Aden. Kind: executed local MCP/LSP experiment plus
explicitly separated static dependency inspection. No product implementation.

## Scope and provenance

The user authorized using the real project instead of generating fixtures.
The roots were `E:/Globalprojects/shadows` and its existing `web/` subdirectory.
They overlap; they are not two distinct repositories or Git worktrees.
Shadows was on `codex/independent-runtime-docs`, commit
`8cefebcfb7d1f0fb679b4a90c34e769fd8cdcbee`. No branch or commit was changed.

The code maps and Code/Projects/frontend contracts selected the primary queries.
Shadows MCP was not connected, so Serena's LSP backend supplied navigation.
The supplementary Threads same-name reference query preceded reading its contract;
this navigation-order deviation did not change source or service behavior.

Reference environment:

| Component | Actual version |
|---|---|
| Serena | `2.0.1.dev0-d9e4c796` |
| Serena source | `d9e4c796fae0207ba6848d1df4b331122a5567b5` |
| Python | 3.13.13 |
| MCP SDK | 2.2.0 |
| rust-analyzer | `1.98.1 (48a229ce 2026-09-01)` |
| Node / npm | 24.15.0 / 11.12.1 |
| TypeScript / typescript-language-server | 5.9.3 / 5.1.3 |

`versions.json` retains the source commit, versions and `uv.lock` SHA-256.
Serena was already installed; it was not reinstalled. Its checkout stayed clean.
The TypeScript server dependencies were installed under the isolated lab home.

## Launch and restrictions

Raw artifacts live at:
`F:/projects_frok/serena-reference-artifacts/2026-10-18-real-shadows/`.

The installed executable was launched by a Python MCP SDK stdio client:

```text
F:/projects_frok/serena/.venv/Scripts/serena.exe start-mcp-server
  --project E:/Globalprojects/shadows --context <lab>/context.yml
  --agent-interface tools --mode planning
  --enable-web-dashboard false --open-web-dashboard false
  --enable-gui-log-window false --log-level WARNING --tool-timeout 180
```

`SERENA_HOME=<lab>/home`, `CARGO_TARGET_DIR=<lab>/cargo-target` and
`CARGO_NET_OFFLINE=true` were supplied only to experiment children. Serena's
`project_serena_folder_location` kept metadata under `<lab>/metadata/`.
The final setting uses the supported `$projectFolderName` placeholder.
Project configuration selected `rust` and `typescript`, `read_only: true`, the
tools interface, Git ignore handling, and explicit `.env`/build/dependency ignores.
The context had `single_project: true` and exactly these fixed tools:

```text
initial_instructions
get_symbols_overview
find_symbol
find_referencing_symbols
```

Every connection called `initial_instructions` before source queries. Exact
returned descriptions and input schemas are in `tools.json`; request arguments
and raw responses are in `report.json`. Bodies use `include_body: true`.
The corrected run disabled Rust check-on-save, build scripts and proc macros in
the isolated project settings. Consequently its observations do not establish
macro-generated symbol coverage or full-feature Cargo analysis.

## CI-1 observations

| Real query | Actual observation |
|---|---|
| Code and Projects file outlines | Returned their structs, impl blocks and methods |
| `impl Code/definitions` | Exact body; 10 marked references including HTTP and MCP callers |
| `impl Projects/create` | Exact body; 11 marked references including the HTTP route |
| `impl Threads/create` | Seven marked references; disjoint from Projects' selected references |
| `ThreadStream/close` | Exact body; five marked callers, including hooks and tests |
| `EventSourceLike/close` | Two marked locations, including `#source?.close()`; disjoint from ThreadStream's callers |
| `parseDurable` | Exact body; six marked references, including imports and calls |
| Missing symbol | Empty list, no fabricated match |
| Broad Rust `create`, `max_matches: 1` | Explicit ambiguity error naming three candidates |
| Body query with `max_answer_chars: 80` | Explicit too-long message; no body, `is_error: false` |
| Parent-path escape | Refused before reading the requested outside-root file |
| Shell, symbol edit and REPL calls | Unknown-tool errors for all three |

Four returned bodies were compared to the exact inclusive source ranges. All 32
marked reference lines in the primary report matched their source lines. The
same-name checks compare marked locations, not neighboring snippet text. They
establish correctness for the selected real cases, not complete reference recall
across the workspace, all Cargo features, generated code or every TypeScript type.

Both `body_location` and snippet line markers are **zero-based** in these outputs.
Rust paths include `impl Code/definitions`; reference dictionary keys use Windows
backslashes. The API returns JSON serialized inside a string-valued `result`.
These conventions need normalization before presenting user-facing citations.

Validation produced **29/29 selected assertions passed** in `validation.json`.
This includes source identity, scope/refusal and applicable lifecycle assertions;
it is not a replacement for the unexecuted acceptance cases below.

## CI-2 observations and limits

Two separately owned fixed-root stdio instances were connected simultaneously:
client A at the repository root and client B at `web/`. Each found `parseDurable`
under its correct relative path. A Rust reference request was still pending when
its client task was cancelled after approximately 20 ms. B's next body query
succeeded, as did A's next body query. A also answered after B disconnected.

`lifecycle-report.json` records client/request labels, roots, calls and outcomes.
This proves client-side cancellation and independent-instance survival. It does
not prove server-side cancellation acknowledgement, release of all cancelled work,
shared-process fairness or isolation of two clients sharing one worker.

No `shadows.exe`, `nylra.exe` or `aylul.exe` was running in the recorded process
snapshot. The MCP/LSP queries needed none of them. The stdio instances closed
with their owners; this does not establish an independently owned shared daemon.
All tracked experiment descendants had exited after shutdown; none required a kill.

Not executed: distinct-worktree/root content isolation, edits followed by fresh
queries, generation/revocation races, symlink/junction escapes, worker-crash
recovery, diagnostic correctness, shared HTTP service/authentication, native
Claude/Codex control, AYLUL or Hermes integration, packaging/reinstallation,
browser acceptance, remote server calls, model calls or token/cost measurement.
No source was edited merely to manufacture a freshness result.

## Local timing samples

These are single-machine samples, not distributions or model latency measurements.
The first default/autodetected Rust setup took 91.322 s inside initial instructions.
The next explicit two-language setup, including first TypeScript dependency setup,
took 33.538 s there. These setups differ and are not a cold/warm comparison.

In the final primary run, MCP initialize took 4.357 s and initial instructions
4.584 s. Selected body queries took 0.100-0.130 s. First reference samples took
22.642 s for Code, 2.426 s for Projects, 8.913 s for ThreadStream and 0.234 s for
parseDurable. Three repeated parseDurable body queries took 0.094/0.099/0.101 s.
Its serialized MCP response was 1,540 bytes each time; the Code reference response
was 6,662 bytes. Reports define bytes as UTF-8 JSON of the complete SDK result,
not just the source text or tokens sent to a model.

## Failed experiment predicates and corrections

1. The first metadata template used `${projectFolderName}`. Serena supports
   `$projectFolderName` without braces and created a literal-braces directory in
   the external lab. It therefore autogenerated Rust-only configuration and both
   TypeScript queries failed. `attempt-1-report.json` preserves that failure.
2. Correcting the external location and explicitly selecting both languages made
   both TypeScript outlines work (`attempt-2-report.json`). No product settings changed.
3. The first source comparison assumed one-based reference markers and failed all
   32 marked-line checks. Inspection showed zero-based numbering; the corrected
   predicate matched every selected line. `validation-first-predicate.json` retains
   the failed assertion report. This was a predicate error, not an LSP fix.

## CI-3 static comparison and recommendation

| Candidate | Inspected evidence | Execution status / remaining work |
|---|---|---|
| Serena as separate process | Pinned code, license, schemas and this experiment | Real Rust/TS queries proven; simplest continued learning reference; GPL application distribution boundary remains relevant |
| Standalone SolidLSP | Pinned MIT notices plus actual imports | Not independently packaged or run; extraction boundary is not clean in this checkout |
| Rust `async-lsp` | Upstream commit `e3d479d500c177ea338cb16e193e33e5000aad6e`, manifest version 0.2.4, client/server documentation and direct dependency manifest | Static only; not installed/built/run; no resolved dependency audit or Windows acceptance |

The pinned root LICENSE identifies SolidLSP as MIT, but `src/solidlsp/ls.py:26-27`
imports `serena.util.file_system.match_path` and
`serena.util.text_utils.MatchedConsecutiveLines`. Both imported files declare
`GPL-3.0-or-later`. C#/F# adapters have further Serena utility imports. Therefore
do not label extraction of the current runtime a verified standalone MIT package.
This is an observed dependency/provenance obstacle, not a legal conclusion.

The [async-lsp manifest](https://github.com/oxalica/async-lsp/blob/e3d479d500c177ea338cb16e193e33e5000aad6e/Cargo.toml)
declares `MIT OR Apache-2.0`; its README explicitly supports language clients.
Its direct dependencies include lsp-types, futures, serde, serde_json and Tower
traits with optional Tokio integration. No source or dependency was added to Shadows.

Recommendation: retain Serena as the experiment reference. Before selecting a
replacement, resolve SolidLSP's utility boundary or prove the native client path,
then close distinct-root/freshness/lifecycle acceptance on the selected candidate.
CI-3 has comparison evidence but no approved language, package location or wire
implementation plan. No package scaffold is justified by this baseline alone.

## Retained artifacts and cleanup

The external lab retains `report.json`, `tools.json`, `validation.json`,
`validation-command.log`, `lifecycle-report.json`, `lifecycle-A.log`,
`lifecycle-B.log`, `server.log`, the two attempt reports/logs, failed predicate
report, `versions.json`, `LICENSE.upstream.txt`, exact YAML configuration,
`async-lsp-reference.json`, `async-lsp-Cargo.toml` and `async-lsp-README.md`.
Metadata/dependency/build caches remain only in this external lab. Temporary probe,
setup and validation scripts were deleted after recording their requests/results;
none was committed, and no product probe crate was created.

SHA-256 snapshots of 570 tracked files under crates/web plus Cargo manifests/lock
were identical before queries, after the primary run and after lifecycle checks.
No `.serena` metadata remains in the project root or `web/`. User MCP settings,
global configuration, real source/tests and the other repositories were untouched.
Only this evidence and existing code-intelligence progress/design documents changed.
