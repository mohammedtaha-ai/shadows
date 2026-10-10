# Guided planning PR 1 - Windows verification

Date: 2026-10-10. Branch: `next/guided-planning-pr1`; implementation was
uncommitted during these measurements. Scope: standards and the stage only.

## Executed behavior

- The development SQLite database was copied using SQLite's online backup
  API, including committed WAL contents, into an ignored trial directory.
  The source was not migrated. The copy upgraded from migration 0012 to 0021;
  `foreign_key_check` returned no violations.
- A real browser created a project, opened Standards, read the four base
  parts and contract template, saved `billing` / `Payments.`, and showed
  additions version 1 after navigation/reload.
- Vision was saved through the browser. The authoritative stage API answered
  `map`, missing `backend`, `database`, `api`, `frontend`, and `billing`.
  Live header refresh after that save was not independently observed before
  browser-control access was blocked. The automated UI test covers refresh.
- The draft header initially lacked the stage. A failing UI test reproduced
  the omission; the same StageChip now appears before the first message.
- The first real Claude opening timed out. Retry opened the session, but the
  provider returned `403 Request not allowed`; Mohammed identified exhausted
  Claude limits. This is not successful Claude-provider acceptance.
- Mohammed authorized temporary MiniMax settings. Setting the daemon's
  environment alone did not work: `shadows-agent/src/claude.rs` builds a
  child environment from an explicit list that excludes those variables.
  A temporary adapter wrapper set the requested environment before importing
  the installed ACP adapter. No product configuration was changed.
- The MiniMax turn `760bdf47-8049-4aa8-9246-4b634d24d6e5` completed. Its
  recorded observed model was `MiniMax-M3`; base standards version was 1.
  The reply named backend, database, API, frontend and the saved billing
  addition, then proposed image storage as an additional part. This proves
  delivery of the effective standards through the real adapter, via HTTP.
  It does not prove the provider's one-million-token context capacity.
- Both temporary credential-bearing scripts were removed. Neither the token
  nor the scripts belong to the Git change. `.claude/launch.json` was untouched.

## Verification and limitations

Targeted checks passed: base parsing, standards/replay/rollback/restart,
stage and ownership, routes, OpenAPI, contracts, code map, Planner MCP,
conversation focus, queue/Send now and turn lifecycle. The pending-turn
version race was reproduced before the pin-based fix.

Full Web tests before the draft-header correction: 230 passed. The draft
test suite then passed 4 tests; typecheck, lint and production build passed.
The final full gate passed: 445 Rust tests, one manual test ignored, and
231 Web tests. Both clippy modes, production feature isolation, Rust
formatting, line width, generated API consistency and diff checks passed.
The Web build passed with its existing large-chunk warning.

The first full Rust build met stale rmeta/rlib artifacts; cleaning only the
`shadows-core` cache repaired that. Another build found the running Windows
trial binary locked; the trial was moved to a separate executable. A later
full run found the schema inventory missing the new table; it was corrected.

The independent reviewer fixed a delayed save response downgrading newer
cached additions, with a deterministic regression. The initial complete
verdict was delayed by the account usage limit. The completed whole-branch
review, recorded in PR #24, reported no new confirmed defects and freshly
passed 66 distinct Rust tests and 15 targeted Web tests. That review did not
rerun the full gate because it made no code changes.

Browser acceptance began in Playwright. Mohammed subsequently requested
the Codex in-app browser exclusively. Its control tool rejected tab access
under URL policy, explicitly forbidding alternate browser workarounds; no
further external-browser automation was attempted. The UI-control part of
acceptance therefore remains partial. Linux and human full acceptance were
not run.

## Merge and CI follow-up (2026-10-10)

PR [#24](https://github.com/mohammedtaha-ai/shadows/pull/24) was squash-merged
into `main` as `ced2095a99e0d7b84006eaf558c843adf6f5746a`. Its PR checks passed.
The post-merge [CI run](https://github.com/mohammedtaha-ai/shadows/actions/runs/38018230623)
also completed successfully: Windows acceptance gate, Web client build gate
and Linux compile gate only. This establishes Linux compilation, not Linux
runtime acceptance. The branch was deleted locally and from the remote.
Browser and full human acceptance remain partial as recorded above.
