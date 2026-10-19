# Serena local baseline

Date: 2026-10-18, Windows host.
Kind: executed installation and synthetic MCP/LSP smoke; not a design decision.

## Installed reference

- Source: `https://github.com/oraios/serena.git`.
- Checkout: `F:/projects_frok/serena`.
- Commit: `d9e4c796fae0207ba6848d1df4b331122a5567b5`.
- Version command returned `Serena 2.0.1.dev0-d9e4c796`.
- Environment: `F:/projects_frok/serena/.venv`, managed CPython 3.13.13.
- Install: `uv sync --locked --no-dev --python 3.13` from the checkout.
- 77 runtime packages installed; upstream source/lockfile stayed unchanged.
- CLI `--help` and `start-mcp-server --help` completed successfully.
- Existing rust-analyzer toolchain component was available; no user project was initialized.

## Executed fixture

A temporary dependency-free Rust crate contained two types, `Account` and
`Wallet`, each defining `balance`, plus `read_account` and `read_wallet` callers.
No real project files, ignored environment values or model credentials were read.
No model API was invoked. `SERENA_HOME`, project settings, generated config,
caches and Rust build outputs were confined to the temporary fixture.

The Python MCP client launched the installed executable with:

```text
start-mcp-server --project <temporary fixture> --context <single-project context>
  --agent-interface tools --mode planning
  --enable-web-dashboard false --open-web-dashboard false
  --enable-gui-log-window false --log-level WARNING
```

The fixture selected Rust, `read_only: true`, and a fixed set containing
`initial_instructions`, `get_symbols_overview`, `find_symbol`,
`find_referencing_symbols`. The client called `initial_instructions` first.

| Check | Actual result |
|---|---|
| MCP initialize and tool listing | Passed |
| Advertised surface in single-project context | Exactly the four requested tools |
| File overview | Both structs, their impl blocks/methods, and both callers returned |
| Definition query | `impl Account/balance` returned only `self.value` method body |
| Reference query | Marked reference points to `read_account(a: &Account)` |
| Same-named Wallet method | Its caller was not marked as a reference to Account's method |
| Direct `execute_shell_command` call | Refused; tool unavailable |
| Rust source digest before/after | Identical |

Single-run timings: MCP ready in 3.934 s; overview 0.101 s; definition 0.102 s;
references 4.056 s. These are one synthetic run, not warm/cold distributions,
production performance guarantees, or evidence of token savings.

## Failed attempts and corrections

1. The probe initially used old SDK conventions. Installed MCP 2.2 expects a float
   timeout and snake_case model fields (`input_schema`, `is_error`). Inspecting the
   installed signatures/model fields identified the issue. No upstream code changed.
2. `Account/balance` found no definition in Rust. The overview exposed the actual
   symbol path `impl Account/balance`; using it made the query succeed.
3. The general desktop context advertised a superset of inactive tools; execution
   still refused the disabled shell. A custom `single_project: true` context also
   reduced the advertised list to the four active tools.
4. An initial reference-check predicate incorrectly searched the entire context
   snippet for `read_wallet`. The returned snippet legitimately includes an adjacent
   line. Validation was corrected to inspect the marked reference location instead.
   The raw reference output was preserved; semantic isolation is about marked
   locations, not the absence of neighboring source text.

## Retained artifacts

Local raw artifacts, kept outside product source:

- `F:/projects_frok/serena-reference-artifacts/2026-10-18/report.json`
- `F:/projects_frok/serena-reference-artifacts/2026-10-18/tools.json`
- `F:/projects_frok/serena-reference-artifacts/2026-10-18/server.log`
- `F:/projects_frok/serena-reference-artifacts/2026-10-18/LICENSE.upstream.txt`

`tools.json` records actual tool descriptions and input schemas. Keep upstream
provenance/license with it; it is research material, not our newly licensed API.
The one-off probe was outside the repositories and is removed after writing this
record. The temporary Rust fixture was removed on client shutdown. No probe crate
or source is added under product `crates/`. There is no source commit for that
uncommitted temporary script; the fixture, calls and corrections are recorded here.

## Limits

Not tested: TypeScript, real repository scale, concurrent clients/worktrees,
server deployment, remote MCP, source-sharing revocation, arbitrary existing
Claude/Codex control, AYLUL integration, packaging, or token/cost improvement.
No agent's MCP configuration or project settings were changed. The smoke client
closed its server; this record does not claim an always-running service.

The own-package direction is a draft in
[the design](../../aylul/specs/2026-10-18-aylul-hermes-semantic-context-design.md#11-standalone-package-direction).
