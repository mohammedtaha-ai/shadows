# NYLRA tracked-source relocation — 2026-10-18

## Scope and provenance

The human selected NYLRA relocation before standalone code-service implementation
and excluded worktrees. The receiving checkout is `E:/Globalprojects/shadows`,
branch `codex/independent-runtime-docs`, base `8cefebc`. The preserved source is
`E:/Globalprojects/nylra`, branch `codex/aylul-relay-client`, HEAD
`2b0427339a4e92d07cc7a0811dbafb2c5faecffd`, including its six previously authorized
working documentation amendments. The measurement checkpoint preceded local
commits. The human subsequently requested committing the completed work; no push,
merge or deployment is part of that request.

The [per-file manifest](2026-10-18-nylra-relocation-manifest.json) covers all 237
tracked source files: 225 imported, 12 retained at origin. It records absolute
source/destination paths and SHA-256 values for imported files. Differences are
documentation path/routing changes and workflow display/concurrency names.
Manifest file hashes describe working-file bytes at the import checkpoint;
normal source files follow the destination's Git line-ending policy. Historical
reports and captured raw artifacts have explicit `-text` attributes so their
recorded bytes survive a checkout. Raw `.log` artifacts are intentionally staged
by exact path despite the general runtime-log ignore rule.
Application code, tests, fixtures, lockfiles and generated native ACL remain
byte-identical to the imported source. Thirteen historical acceptance reports and
the former repository README were preserved byte-for-byte. Their old paths and
commit names retain their original meaning; they are not fresh acceptance.

`apps/desktop/` keeps its UI, independent Tauri and hook Cargo workspaces, Node
sidecar, contracts and tests. It joins neither Core's workspace nor AppCore.
Active documents are under `docs/aylul/`, with separate character/native/UI owners
and internal requests. Historical reports are in `docs/evidence/nylra/`; current
art provenance is in `assets/aylul/README.md`. The previous custom backend and
`shadows-backend` checkout were not changed by relocation. No Hermes or semantic
service client was implemented.

Ignored dependencies, personal stores/settings, credentials and art archives were
not imported. Dependencies were installed from the unchanged lockfiles. Nine
Claude launcher definitions and local MCP configuration remain at origin; global
agent settings were not changed. The original checkout/history and ignored art
remain intact, with source README/AGENTS marked as a preserved snapshot. No source
files, old decisions or personal runtime data were deleted or retired.

## Observed local verification

Windows; Node 24.15.0, npm 11.12.1. Exact Rust versions and raw evidence hashes are
in the [local verification report](2026-10-18-nylra-relocation/verification.json).
Commands ran from the destination roots, using debug binaries.

| Destination / command | Observed result |
|---|---|
| `apps/desktop`: `npm ci --no-audit --no-fund`; `npm run build` | Install, TypeScript check and both HTML builds passed |
| `apps/desktop`: `npm test` | 7 passed |
| `apps/desktop`: `npm run test:companion:unit` | 15 passed |
| `apps/desktop`: `npm run test:companion:production` | 1 passed; studio excluded |
| `apps/desktop`: `npx playwright test tests/companion/shell.spec.ts` | Final complete run: 7 passed |
| `apps/desktop/sidecar`: install, `npm run typecheck`, `npm test` | 71 passed, none skipped; `AYLUL_HOOK_EXE` pointed at the freshly built destination debug hook, including actual Windows pipe tests |
| Root: `cargo test --locked --manifest-path apps/desktop/hook/Cargo.toml`; debug build | 12 passed; build passed |
| Root: native `cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml`; debug build | 29 passed, 1 ignored (opt-in Docker auth); build passed |
| Hook/native `cargo fmt ... --check`; hook `cargo clippy --locked ... --all-targets -- -D warnings` | Passed; native clippy was not rerun |
| Manifest hash audit and active Markdown navigation | No application/test/fixture/lockfile/ACL differences; no historical-report changes; no broken active relative links |
| Imported source contracts | All 18 parse successfully and name existing source paths; parsing alone does not prove their rules |
| Core vs native/hook Cargo metadata; root agent-document parity; `git diff --check` | Independent workspace roots, mirrored root instructions and whitespace checks passed |

142 selected automated test cases passed in the final checks. The initial browser
run had 5 passes and 2 failures (a navigation timeout and missing alert). The two
failed cases passed when rerun without code/config edits, followed by all 7 in one
fresh run. Their initial failure logs are retained. The underlying transient cause
was not established; no product fix or increased timeout was applied.

## Actual Windows debug run

The destination `nylra-desktop.exe` was launched by the existing
`windows-companion.ps1` driver. `AYLUL_DATA_DIR` and `CLAUDE_CONFIG_DIR` used fresh
test-owned directories; the hook was the destination debug executable. The backend
URL was an unused loopback endpoint; no login, personal hook installation or remote
model/server call was exercised. No existing NYLRA process was running at launch.

Observed: actual main/companion window ownership and 240px companion start,
companion surviving main close, Canvas draws stopping under explicit hide even
while the WebView reported visible, actual native IPC permission denials,
controlled renderer failure and reconstruction, tray quit, process exit and removal
of the isolated sidecar `hook.json` endpoint. The actual companion screenshot was
visually inspected: [destination debug companion](2026-10-18-nylra-relocation/native.png).

The first native run requested two focus cycles but failed **before** measuring
show behavior: the owned foreground witness was not the actual foreground window.
It is not a pass or proof of a product focus regression. The second run explicitly
used `-WebViewOnly -FocusCycles 0`; remaining window/IPC/recovery/quit checks passed.
The zero-cycle run does not prove no-focus-steal acceptance. Both raw native reports
are retained. The isolated endpoint was absent after the successful tray quit.

## Limits

No remote CI, full Core gate, actual Claude/Codex conversation, Docker auth journey,
Hermes integration, token measurement, release/installer, DPI transitions or native
performance acceptance is claimed. Native no-focus-steal remains unverified after
relocation; the unchanged source's older evidence remains dated evidence only.
The original repository, art archives and data stay retained; retirement is a
separate decision. Follow new development through `docs/aylul/README.md`.

Raw logs, manifest checks, failed/successful browser runs and both native reports
are preserved in [the dated artifact directory](2026-10-18-nylra-relocation/).
