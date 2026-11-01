---
name: env-loading
description: Live-key tasks must load .env with a read loop, never `. ./.env` or `set -a; . ./.env`
metadata:
  type: feedback
---

Never write `set -a; . ./.env; set +a` into a task. Tell the builder to load `.env` with a read loop.

**Why:** on 2026-10-21, B-1's builder sourced it. The file has no final newline, the source failed, and the error echoed part of the MiniMax key into tool output. Mohammed was told to rotate the key.

**How to apply:** use this in every probe or acceptance task that uses `OPENROUTER` or `API_KEY_MINIMAX`. Older plans, such as the settings-dialog one, still show the bad command.
