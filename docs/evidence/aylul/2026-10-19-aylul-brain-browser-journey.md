# AYLUL brain: browser journey on `shadows serve`

Date: 2026-10-19. Branch `feat/aylul-claude-brain` at `f48a34e`.
Plan: `docs/aylul/plans/2026-10-19-aylul-claude-code-brain.md`, Task 6.

## Setup

- `shadows serve --debug --db <copy> --node … --adapter <claude-agent-acp 0.81.1>
  --harness <claude.exe> --aylul-config <file>` on `127.0.0.1:4318`. The web
  client is served by `vite` on `5173`.
- The DB is a copy of Mohammed's dev database (`shadows-dev/shadows.sqlite3`, 4
  projects after start). The original was not opened.
- The section file holds `home_root` (a scratch folder), `key_env:
  MINIMAX_API_KEY`, `base_url: https://api.minimax.io/anthropic` and `model:
  MiniMax-M3`. The key reached the server's environment only, read from Hermes'
  `.env` by the launcher; it was never printed or written.

## What a person saw

| Step | Result |
|---|---|
| Start | `<home_root>/profiles/aylul/` seeded with `CLAUDE.md` and `settings.json`; project `aylul` created with `assistant: true` and its directory set to that home |
| Sidebar | An "AYLUL" entry above Projects; the `aylul` project is not listed among Projects |
| AYLUL draft | Header "New conversation · AYLUL", no stage chip, placeholder "Ask AYLUL…", runs in the profile home |
| `GET /api/harnesses` | `{"kind":"aylul","label":"AYLUL","available":true}` |
| First message (Arabic: name and working folder) | Streamed reply "اسمي AYLUL…", "Asked for MiniMax-M3 · answered by MiniMax-M3"; Thinking and Stop shown while it ran; the conversation got an Arabic title |
| Effort menu | Low, Medium (current), High, Xhigh, Max; switched to High |
| Second message on High (`17 × 23`) | "391" |
| Model menu | Default, MiniMax-M3 (×3: the opus, sonnet and haiku aliases point to it), Fable 5.1 |
| Fable 5.1 selected, "say ok" | "ok", "Asked for Fable 5.1 · answered by MiniMax-M3": MiniMax answers an unknown model name with M3, and the UI reports it truthfully |

## Isolation check after the journey

- `~/.claude.json`: 0 mentions of the journey folder.
- `~/.claude/projects`: no folder for the profile home.
- The profile home holds its own `.claude.json`, `projects/`, `sessions/`,
  `backups/`, `settings.json` and `CLAUDE.md`.

## Defects found by this journey and fixed before it passed

- `HARNESS_UNAVAILABLE: the aylul harness is not available yet` on the first
  send, and the harness list showed `label: "Unknown"`, `available: false`. The
  core called `policy::is_available` at four sites; Task 3's tests reached the
  session directly and never the send path. Fixed in `f48a34e` with
  `Sessions::harness_available`, plus a test through `Turns::send`.
- "Stage: idea" in AYLUL's headers. Fixed in `809af82`.
- "· Planner" and "Ask the Planner…" in AYLUL's conversation. Fixed in `991243b`.

## Left open

- The model menu repeats "MiniMax-M3" three times and offers Fable 5.1, which
  MiniMax silently serves as M3. A per-profile model list is a later slice.
