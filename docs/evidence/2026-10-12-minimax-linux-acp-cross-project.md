# MiniMax ACP and cross-project Linux probe — 2026-10-12

Scope: a disposable Shadows SQLite database and **Synthetic Lighthouse**
project (`63e24e0a-f587-4518-b389-d45a7f93cc4a`) on `127.0.0.1:4318`.
No real project or account was used. Credentials were process environment only.

## What ran

The real Shadows daemon launched its pinned Claude ACP adapter and Claude Code
with the user's temporary MiniMax Anthropic-compatible settings. The requested
model was `MiniMax-M3[1m]`; `ANTHROPIC_BASE_URL` was
`https://api.minimax.io/anthropic`; the three default model variables also
selected `MiniMax-M3[1m]`; both context-window variables were `1000000`.
`ANTHROPIC_AUTH_TOKEN` came from the runtime MiniMax key and is not recorded.

The first real turn stalled because the ACP child did not inherit the managed
network proxy and CA settings. `claude.rs` now forwards only an explicit set of
provider and network variables. Its test proves the named values reach the
child and an unrelated secret does not. With those settings, an actual Planner
turn read the synthetic project README and completed. A subsequent turn with
the requested `[1m]` model completed as operation
`7320f10c-ffea-4e80-a85a-638bf918c23d` in thread
`b7f749d5-a682-4a67-a6b8-29f1b62c9f83`; the ACP model identity reported
back was `MiniMax-M3`. This proves a turn completed through the adapter with
the requested setting, **not** that a one-million-token context window was
accepted or exercised. No large-context test was run.

The companion backend probe separately read the selected project through
Shadows' loopback project API and streamed an AI SDK/MiniMax answer. That is a
same-host data-flow test; the ACP Planner and the backend conversation are
two different model clients and no shared chat/session contract exists yet.
See the backend probe's `RESULTS.md` for its tool-error finding.

## Reproduction and boundary

Use a fresh disposable database and synthetic project, run the daemon locally,
and provide the user's Anthropic-compatible variables **to the daemon process**
at launch. On managed proxy environments also pass `NODE_USE_ENV_PROXY=1`,
proxy variables and `NODE_EXTRA_CA_CERTS`; the child receives only the named
allowlist. Run `cargo test --locked -p shadows-agent
only_named_provider_settings_reach_the_acp_child --lib`,
`cargo test --locked -p shadows --test harness_config`, and
`cargo build --locked -p shadows` before the live turn.

Future cloud integration must use an outbound authenticated Shadows connector.
Shadows must enforce selected-project grants and revisions on each read or
action, and return only the authorized projection. A model tool call is a
request, not evidence of user approval. This probe neither validates the
connector transport nor authorizes production writes.
