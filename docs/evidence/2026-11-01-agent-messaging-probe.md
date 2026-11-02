# Reaching and answering Claude sessions Shadows did not start: 2026-11-01

Dated facts. This file decides nothing. It is the starting point for the
messaging design behind "The manager and the escalation ladder" in
[the future list](../future.md#execution); that design owns every decision.

## Reproduction

- **Machine:** Windows 11, Claude Code 2.1.309, Claude Desktop (Code tab),
  2026-11-01 between 01:30 and 02:20 local time.
- **Probe hook:** a project `.claude/settings.json` in an empty folder
  (`F:\testing5`) with one `Stop` hook, `timeout: 600`, running this script:

  ```python
  import json, sys, time, os
  ev = json.load(sys.stdin)
  log = os.path.join(os.path.dirname(__file__), "events.log")
  open(log, "a", encoding="utf-8").write(json.dumps({k: ev.get(k) for k in (
      "hook_event_name", "stop_hook_active", "last_assistant_message",
      "transcript_path")}, ensure_ascii=False) + "\n")
  if not ev.get("stop_hook_active"):
      time.sleep(20)
      print(json.dumps({"decision": "block", "reason":
          "Mohammed replies through AYLUL: fix the bugs. "
          "Reply with exactly the word FIXING and nothing else."}))
  ```

  The folder and script were deleted after this file was written; they were
  never in the repository.
- **Prompt (Haiku 5.5, new Desktop conversation in that folder):**
  `Ask me one question: should you fix the 3 bugs or open the PR? Then stop and wait.`
- **The `claude` CLI could not be used:** its sign-in had expired
  (`OAuth session expired and could not be refreshed`), so every run was in
  Claude Desktop.

## 1. A `Stop` hook can hold Claude and answer it

| Step | Observed |
|---|---|
| Claude asked and stopped | Hook input: `stop_hook_active: false`, `last_assistant_message: "Should I fix the 3 bugs or open the PR?"`, `transcript_path` set |
| Hook slept 20 s, then printed `decision: block` | Desktop showed "Hook re-prompted Claude" (expandable) under the question |
| Claude continued in the same conversation | It wrote `FIXING` |
| Claude stopped again | Hook input: `stop_hook_active: true`, `last_assistant_message: "FIXING"`; the script printed nothing and Claude stayed stopped |

- The last assistant message arrives in the hook input; the transcript need
  not be read to know what Claude said last.
- A wait of 20 s worked with `timeout: 600`. Longer waits were not tried.
- While the hook waits, Claude is still running from Desktop's point of view.

## 2. Every conversation is a readable transcript

- A Desktop conversation is written to
  `<config dir>\projects\<folder slug>\<session id>.jsonl`, each line tagged
  `"entrypoint":"claude-desktop"`. Every hook event carries its path.
- A conversation opened after the probe ("HI", no probe hook) was read from
  its file as it was written: title, the user's message and Claude's reply.

## 3. Live sessions are registered per config directory

- Each live session writes `<config dir>\sessions\<pid>.json` and a
  `<pid>.<hash>.key` beside it. Observed keys: `sessionId`, `cwd`, `kind`,
  `entrypoint`, `name`, `status` (`idle` or busy), `peerProtocol: 1`,
  `peerFeatures: ["notify_idle","artifact_yield"]`, `hostSessionId`
  (`local_<uuid>`) and `messagingSocketPath`
  (`\\.\pipe\LOCAL\cc-msg-<hash>`).
- The registry is per config directory. AYLUL's agent `ay` runs with its own
  `CLAUDE_CONFIG_DIR`; its `ListAgents` saw only its own sessions (`ay-ef`,
  `ay-ad`, `entrypoint: sdk-ts`), never the Desktop sessions under
  `C:\Users\Mohammed\.claude`.
- None of this is documented. It may change with any Claude Code release.

## 4. Sessions in one config directory can message each other

- From one Desktop session, `SendMessage` to "HI" was delivered ("Forwarded to
  Claude Desktop's session messaging"). HI's Claude answered with
  `SendMessage` to `local_<uuid>`, and `RECEIVED` arrived in the first
  session as a cross-session message. An idle notice also arrived when HI
  finished its turn.
- A Claude Code CLI session (Opus, signed in, `~/.claude`) received a message
  from HI. Mohammed reports that a CLI session on MiniMax could not; which
  config directory it used was not checked.
- Not tried: whether a program that is not a Claude session (AYLUL, Shadows)
  can speak the message pipe. Codex has no counterpart.

## 5. What Coucou does (MIT source, `Louis-CFM/coucou` at `b6a85f2`)

Read from `windows/hook/src/main.rs` and `reply.rs`:

- One hook binary installed on 12 Claude Code events, plus Codex, Copilot CLI
  and others through `--agent`.
- Only `PermissionRequest` waits for an answer, for at most 110 s; every other
  event is fire-and-forget, and a hook that cannot reach the app exits at once.
- It answers `allow` or `deny`, and answers `AskUserQuestion` by returning the
  same input with an `answers` map. Without a human click it prints nothing.
- Claude Desktop sessions are told apart by
  `CLAUDE_CODE_ENTRYPOINT=claude-desktop`.
- It does not answer a free-text question after `Stop`.
- Its artwork and the character Mochi are not under the MIT licence
  (`LICENSE-ASSETS.md`).

## 6. Paths Shadows already has

Stated for the design to weigh, not decided here:

- **ACP:** Shadows sees every event of a session it starts (the Planner today,
  executors later), for Claude, Codex and Gemini alike.
- **MCP (`/mcp`):** an agent Shadows starts can call Shadows' tools, which is
  a way for an executor to report a problem or ask the Planner.
- **Hooks and the session registry:** the only ways found to see sessions
  Shadows did not start (Desktop, the person's own CLI).
