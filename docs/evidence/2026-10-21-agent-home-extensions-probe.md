# Agent home extension isolation probe - 2026-10-21

## Reproduction

Temporary probe commit: `ffe3260`. Its ignored unit test
`b8_live_extensions_probe` was run with `cargo test -p shadows-agent --lib
b8_live_extensions_probe -- --ignored --nocapture`. The evidence commit removes
the probe, restoring acp.rs exactly to its pre-probe state. This is the same
Windows stack as B-6: Claude Code 2.1.303, adapter 0.81.1, ACP Rust 2.2.0.
API_KEY_MINIMAX from .env was injected into one command environment. There was
no credential file, credential argument or value in recorded output. Both
sessions used MiniMax-M3, the international Anthropic-compatible base URL,
output limit 1024, their own CLAUDE_CONFIG_DIR and cwd, and ACP options
settingSources ["user"]. Only `say hi` was prompted; model text was discarded.
Each owned process tree was terminated after its turn.

Home A had skills/b8-skill/SKILL.md with frontmatter name b8-skill and a short
description. A separate temporary local marketplace contained:

```text
marketplace/.claude-plugin/marketplace.json
marketplace/plugins/b8-plugin/.claude-plugin/plugin.json
marketplace/plugins/b8-plugin/commands/hello.md
```

The marketplace manifest had name b8-market, owner.name Shadows probe, and
plugins [{name: b8-plugin, source: ./plugins/b8-plugin}]. The plugin manifest
had name b8-plugin, version 1.0.0 and a description. Its hello command and the
skill body were the trivial instruction `say hi`.

Home A settings.json declared (the path is the temporary marketplace root,
not an API response or stored product id):

```json
{
  "extraKnownMarketplaces": {
    "b8-market": {"source": {"source": "directory", "path": "<marketplace>"}}
  },
  "enabledPlugins": {"b8-plugin@b8-market": true}
}
```

Home B had neither skill files nor marketplace/plugin settings. Both were
fresh homes; only an onboarding flag was seeded. There was no CLI install,
no --plugin-dir flag, no global settings edit, and no remote marketplace.
The local directory manifest follows the official
[marketplace layout](https://code.claude.com/docs/en/plugin-marketplaces).

## Raw observations

```text
B8 home=a result=Ok(Ok(Ended)) markers=["b8-skill", "b8-plugin:hello"]
B8 home=b result=Ok(Ok(Ended)) markers=[]
1 passed; 0 failed
```

The marker names came from ACP AvailableCommandsUpdate, observed through the
existing HarnessEvent::Commands mapping, not from model self-report. The test
asserted both names in A and neither in B. Both live turns completed.

## What this establishes and limits

A user skill and a locally declared plugin load under an isolated home with
settingSources ["user"]. Settings-only directory marketplace registration and
enabledPlugins were sufficient on this pinned runtime. The second home did
not inherit either command. The planned shared library with per-agent
projection is feasible without changing the accepted HTTP shape.

This proves discovery/isolation, not that plugin code was invoked or every
plugin component works. The fixture plugin contained one slash command, no
hooks, MCP servers or executable scripts. Disabling a previously cached plugin,
removing an item, preserving unrelated settings, path containment and concurrent
openings require B-9 tests; fresh-home success does not prove those cases.
B-9 must remove its own stale projection and disable its previously managed
plugin ids explicitly, so a cached plugin cannot survive a toggle-off.
