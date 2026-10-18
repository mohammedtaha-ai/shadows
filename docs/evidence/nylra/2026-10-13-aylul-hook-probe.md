# aylul-hook language and Claude Code permission timing probe (2026-10-13)

Probe source: `docs/evidence/probes/2026-10-13-aylul-hook/` at `6464d0e`. It was
deleted in the next commit; this file keeps its findings. Raw event logs stayed
local because they hold session ids and local paths.

Two questions for step 2(a) of the
[first release](../superpowers/specs/2026-10-10-nylra-delivery-design.md#first-release-and-who-builds-it):
what `aylul-hook` is written in (OPEN in [source structure](../source-structure.md)),
and how Claude Code behaves while a `PermissionRequest` hook waits for a decision
from AYLUL or the phone.

Machine: Windows 11 Pro 10.0.26200, Node 24.15.0, Rust 1.98.1 (cargo), Claude
Code 2.1.292. Hook reference read the same day: `code.claude.com/docs/en/hooks`.

## Start time: Rust against Node

Each hook reads the event from standard input, sends it as one line over the
named pipe `\\.\pipe\aylul-probe`, prints the one-line reply and exits. A Node
pipe server replied at once. `bench.js` spawned each hook 50 times in exec form
(no shell), as Claude Code runs an `.exe`, after one warm-up.

| Hook | Median | p90 | Max |
| --- | --- | --- | --- |
| Rust release `.exe`, 147 KB | 26.2 to 37.9 ms | 32.6 to 60.6 ms | 51.7 to 85.8 ms |
| Node script (`node.exe hook.js`) | 189.2 to 260.5 ms | 234.1 to 393.3 ms | 280.8 to 671.7 ms |

Ranges are two runs. Rust was about seven times faster. The Node hook also needs
an absolute path to `node.exe` in the agent's hook configuration; Claude Code's
exec form on Windows accepts only a real executable, so `.cmd` shims do not work.

## Claude Code permission timing

A throwaway project's `.claude/settings.local.json` registered a logging hook
(`claude-project/log-hook.js`) on `PermissionRequest`, `Notification`,
`PostToolUse` (`async: true`), `SessionStart`, `SessionEnd` and `Stop`. Each run
asked Claude Code to create `probe.txt` with the Write tool, in a new session.

| Run | Setup | Observed |
| --- | --- | --- |
| 1 | The person's default auto mode | No `PermissionRequest`: the Write was approved without a prompt. `SessionStart`, `PostToolUse`, `Stop` fired |
| 2 | `--permission-mode default`; the hook waits 20 s and returns nothing | `PermissionRequest` started at +0 s. The terminal dialog was on screen while the hook waited; `Notification` `permission_prompt` fired at +6.0 s. The hook ended at +20.0 s and the dialog stayed |
| 3 | The hook waits 12 s, then returns `allow` | Dialog on screen during the wait; `permission_prompt` at +6.0 s. At +12.0 s the hook allowed: the dialog closed by itself, the file was written, `PostToolUse` at +12.6 s |
| 4 | The hook waits 40 s, then would return `allow`; the person chose No at once | The tool use was rejected. The hook never logged its end: Claude Code terminated it. No file, no `PostToolUse` |

## Findings

1. **`aylul-hook` should be a Rust executable.** It starts in about 30 ms
   against about 200 ms for Node, needs no Node path, and is the real `.exe`
   Windows exec form requires.
2. **The person and AYLUL answer the same request in parallel.** The terminal
   dialog appears while the hook waits. Whichever answers first decides: a hook
   `allow` closes the dialog, and a terminal answer ends the hook. A late hook
   answer after the person's refusal had no effect.
3. **A hook that returns no decision changes nothing.** The dialog stays for
   the person, which matches "failure is not consent".
4. **Auto mode and allow rules bypass the hook.** `PermissionRequest` fires only
   when Claude Code would prompt. AYLUL sees what the agent decided on its own
   only after the fact, through `PostToolUse`.
5. **`PermissionRequest` has no `tool_use_id`.** Its fields were `session_id`,
   `prompt_id`, `permission_mode`, `tool_name`, `tool_input` and
   `permission_suggestions`. Matching a request to the later `PostToolUse`
   (which has `tool_use_id`) needs session, prompt, tool name and input.
6. **`PostToolUse` carries file contents.** For Write, `tool_response` held
   `content`, `originalFile` and `structuredPatch`. A step sent onward must be
   cut down to tool name, path and one-line summary before it leaves the hook
   path, as the first release requires.
7. **`Notification` arrives from every session in the folder,** including idle
   ones (`idle_prompt` from an older session appeared in run 2). Events must be
   keyed by `session_id`.

Not measured: Codex hooks, `SessionEnd` (sessions were closed by ending the
terminal), a hook `deny`, several hooks on one event, and `onFailure`, which
needs Claude Code 2.1.295.

## Later the same day: refusals and permission rules

Probe source: `docs/evidence/probes/2026-10-13-aylul-rule/` at `97775d0`, deleted
in the next commit. Claude Code had updated itself to 2.1.296 by this run.

- **Refusals have their own hook.** The hook reference documents
  `PermissionDenied`: it runs when auto mode denies a tool call, with
  `tool_name`, `tool_input`, `tool_use_id` and `reason` (for a classifier
  verdict the matched rule in brackets, such as `[Data Exfiltration]`). Its only
  output is `retry: true`, which tells the model it may try again and does not
  reverse the denial. Read from the documentation, not run.
- **A deliberate refusal was not built.** Preparing a task file meant to make a
  second session's request be denied, with a hook standing in for the person's
  approval, was itself refused by the controlling session's classifier as
  `[Auto-Mode Bypass]`. The probe stopped there. Whether an approved allow rule
  lets auto mode run a call its classifier denies stays untested; Claude Code's
  denial text says a permission rule allows the action in future.
- **A rule written during a session applies at once.** In default mode, the
  `PermissionRequest` hook allowed the first Write (`a.txt`) and in the same
  moment added `Write` to `permissions.allow` in the project's
  `.claude/settings.local.json`. The second Write (`b.txt`), three seconds
  later, raised no `PermissionRequest` and ran without a prompt. The terminal
  showed "Allowed by PermissionRequest hook" under the first Write only.
- **The last message is in `Stop`.** `last_assistant_message` held Claude's
  final text for the turn ("Created `probe.txt` containing `hi`."), and every
  hook carries `transcript_path` to the full session transcript.
