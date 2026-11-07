# ACP Upgrade Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task by task.
> Steps use checkbox syntax for tracking.

**Goal:** Upgrade the Rust SDK to 5.0.0 and the Claude adapter to 0.101.0 while preserving
Shadows' ACP v1 behavior.

**Architecture:** Shadows Agent owns ACP requests and event mapping. Core Harness still
chooses and applies the session's mode before the first prompt. The daemon runs the
published npm adapter as installed.

**Tech stack:** Rust, Cargo, Node.js 22+, npm, Claude Code.

**Spec:** [Harness controls, §12.2–12.9](../../core/specs/2026-09-24-harness-controls-design.md#122-the-harness-connection).

## Constraints

- Work on the current branch and use the existing build cache. Keep the two unstable
  Rust features for session fork and end-of-turn token usage. Keep ACP v1 on the wire.
- The adapter is installed from npm. Any required behavior correction belongs to Shadows.
- Keep dated measurements intact. The full real-provider journey suite remains an idea in
  [Housekeeping](../../core/future.md#housekeeping).
- For live tests, use only the authorized MiniMax and OpenRouter keys through process
  environment injection. Read the allowed names with a loop and keep secrets out of output.
- Mohammed owns the browser acceptance. Keep it pending until he confirms the Planner run.

## Evidence before implementation

Measured on Windows on 2026-11-07, from f50b4d9d. The baseline uses the installed
adapter 0.81.1 and the configured Claude Code executable reporting 2.1.317. The
extension test's printed 2.1.303 is a fixture literal from an earlier run.

- The baseline extension test completed its MiniMax-M3 turns. The enabled profile saw
  its skill and plugin, the second profile saw neither, and disabling removed both.
- The provider probe listed 362 OpenRouter and 9 MiniMax models. It calls providers
  directly and does not exercise the adapter.
- A direct ACP v1 probe used the same MiniMax-M3 prompt and Claude executable with
  each official adapter. Both streamed a reply, ended with "end_turn", reported
  usage and emitted command updates. Both accepted "acceptEdits", returned a distinct
  session from fork, accepted a resume of that fork, and resumed the original
  session after the adapter restarted.

| Direct ACP observation | 0.81.1 | 0.101.0 |
|---|---|---|
| Initial mode | Manual ("default") | Auto ("auto") |
| Mode returned by fork | No mode option | "acceptEdits" |
| Mode after resuming fork or original | Manual ("default") | "acceptEdits" |

The candidate was exercised from a separate npm installation. The wire probe
does not cover the Rust SDK migration, Shadows' end-to-end turn path, or a prompt
sent after the fork. Run the focused gates and browser acceptance below.

The existing daemon binary also started on a temporary database with the
candidate adapter path. With RUST_LOG=info, the startup log reported
adapter_version="0.101.0" and claude_version=2.1.317. This confirms version
detection. Repeat startup with the rebuilt binary after migration.

## Task 1 · Migrate the Rust SDK

**Files:** Cargo.toml, Cargo.lock, the compiler-reported files under
crates/shadows-agent/src and crates/fake-acp/src, crates/fake-acp/Cargo.toml for
the fake agent's native stdio feature, both crate contracts and the backend code map.
Existing tests are the behavior baseline.

- [ ] Pin agent-client-protocol to "=5.0.0". Run
  "cargo check -p shadows-agent -p fake-acp --features fake-acp/test-support" and
  compare the errors to the stage's recorded compilation failure.
- [ ] Apply the [SDK v5 migration](https://github.com/agentclientprotocol/rust-sdk/blob/v5.0.0/md/migration-v5.md).
  Move message imports to v1::schema and use Client.v1() and Agent.v1(), with the
  v1 connection, responder and request types. Enable stdio only for fake-acp under
  test-support. Keep the product transport on the managed process and ByteStreams.
- [ ] Preserve the ordered untyped notification fallback for Claude extensions, and
  the raw request paths for steering and task stop. Check error mapping against the
  existing peer error vs. transport closure test. Keep the public Connection methods.
- [ ] Add `crates/shadows-agent/contract.yaml` and `crates/fake-acp/contract.yaml`,
  grounded in their source and tests. Link both from `docs/codebase/README.md`.
- [ ] Run "cargo test -p shadows-agent", "cargo test -p fake-acp --features
  fake-acp/test-support", and the Core and harness targets below against the existing
  adapter pin. Fix compile or behavior regressions in their owner. Commit the SDK migration.

## Task 2 · Pin the adapter and verify the integration

**Files:** harness/claude/package.json and package-lock.json, the live fixture under
crates/shadows-core/tests/aylul_extensions/, crates/shadows/tests/remembered_mode.rs,
crates/fake-acp/src/handlers/session.rs, the fake and Core Harness contracts, and
docs/backend/stages.md.
If a measured regression requires a product fix, record the exact owner files and
the failing case before widening this list.

- [ ] In harness/claude run "npm install --save-exact
  @agentclientprotocol/claude-agent-acp@0.101.0". Confirm the installed package
  version. Update the live fixture's adapter and Claude Code versions to those
  actually used. Leave the independent harness_config fixture alone.
- [ ] Verify the first prompt runs in Shadows' selected mode. The adapter's own
  default changed to Auto; the existing opening path must still apply "acceptEdits"
  when no allowed mode was remembered. The direct probe confirmed the existing
  fork-then-resume request sequence is accepted. Keep it and verify a prompt can
  continue in the fork and in a resumed session through Shadows.
- [ ] Verify the allowed remembered mode "auto" survives resume and fork. In a
  project that allows "auto", make the fake report "acceptEdits" at each opening
  and check its actual mode when the first prompt arrives. Use the existing new-session
  test for the inverse case, where the fake offers "auto" and Shadows applies its
  "acceptEdits" default. Check the sequence once live through Shadows. Manual
  ("default") is an adapter mode outside Shadows' allowed modes. If a regression
  requires a fix, record the Core Harness owner paths and failing case first.
- [ ] Run the focused gates below. Run the two ignored tests once against the
  upgraded checkout, with both provider keys available, and record each result.
- [ ] Build and start the daemon with the installed adapter path. Capture
  harness.versions with adapter_version="0.101.0" and the actual Claude version.
  Set RUST_LOG=info for this run so an inherited filter cannot hide the event.
  Commit the adapter pin and validation notes. Hand the running configuration to
  Mohammed for a Planner turn in the browser before the PR.

## Focused commands

Run from the workspace root after the adapter install unless noted above.

~~~powershell
cargo test -p shadows-core --test acp_connection --test sessions --test thread_session --test aylul_session --test harness_schema --test planner_isolation --test aylul_usage --test aylul_extensions --test aylul_profiles
cargo test -p core-harness -p core-turns
cargo test -p shadows --test harness_config --test planner_mcp --test mcp_tools --test remembered_mode --test architecture --test codemap --test contract_tests
cargo fmt --all --check
cargo clippy -p shadows-agent -p fake-acp --all-targets --features fake-acp/test-support -- -D warnings
cargo clippy -p shadows-agent -p fake-acp -- -D warnings
cargo test -p shadows-core --test aylul_extensions live_agent_extension_isolation -- --ignored --nocapture
cargo test -p shadows-core --test aylul_profiles live_keys_list_models -- --ignored --nocapture
cargo build -p shadows
~~~

The extension test exercises the real adapter and MiniMax prompts. The profile
probe checks model listing for both providers. A missing key must be reported
before starting the probe so a skipped provider cannot appear as a pass.
