# Claude Code in an isolated AYLUL home

Recorded 2026-10-19, Asia/Aden. Ad hoc probe run from a Claude Code session on
Mohammed's Windows laptop. Evidence, not design authority. No repository source
was written; the probe home lived in that session's scratchpad.

## Setup

- Claude Code 2.1.303 (`claude --version`).
- `CLAUDE_CONFIG_DIR` pointed at an empty probe folder (the "home").
- Provider by process environment: `ANTHROPIC_BASE_URL=https://api.minimax.io/anthropic`,
  `ANTHROPIC_AUTH_TOKEN` read at launch from Hermes' `.env` (never printed),
  `ANTHROPIC_MODEL` and the three `ANTHROPIC_DEFAULT_*_MODEL` set to `MiniMax-M3`,
  `ANTHROPIC_API_KEY` empty.

## Observations

| Check | Result |
| --- | --- |
| Home only, no provider | `Not logged in`: the person's own sign-in was not used |
| Files created | `.claude.json`, `projects/`, `sessions/`, `backups/` inside the home only |
| Real reply | MiniMax-M3 answered |
| Person's setup | Init event listed only built-in skills, agents and plugins; no MCP servers; auto memory path inside the home; the person's hooks did not run |
| Own instructions | `CLAUDE.md` in the home ("Your name is AYLUL") was obeyed |
| Own `settings.json` | `env` (base URL, models) and `permissions.allow` applied |
| Token in `settings.json` `env` | `Not logged in`: the token must be in the process environment |
| `apiKeyHelper` | `Invalid API key`: MiniMax rejected the key sent that way |
| Tools | Wrote `note.txt` and ran `ls`, no permission denials in `-p` mode |
| Resume | `--resume <id>` recalled the earlier turn; transcripts stay in the home |
| Output | `--output-format json` and `stream-json --verbose` (init lists tools, agents, MCP, memory paths) |
| Work folder under `C:\Users\Mohammed` | Leak: `C:\Users\Mohammed\.claude\CLAUDE.md` loaded as a parent project file |
| Same, with `--setting-sources user` | No leak; the home's `CLAUDE.md` still loaded |
| Work folder outside the user profile | No leak |
| Work folder `E:\Globalprojects\shadows` | Project `CLAUDE.md`, project agents and `.mcp.json` (`context7`) loaded |

Warnings: Claude Code does not recognize `MiniMax-M3` and assumes a 200k context
window; its reported cost (about $0.11 per request) uses Anthropic prices, not
MiniMax billing.

## Related observation

The Shadows Planner session of 2026-10-19 (`entrypoint: "sdk-ts"`, transcript
under the person's `~/.claude/projects/E--Globalprojects-shadows/`) used the
person's own sign-in and history folder. `shadows-agent/src/claude.rs` passes
`PATH`, `HOME`, `TMPDIR`, `LANG` and notes that no isolation profile exists yet.
