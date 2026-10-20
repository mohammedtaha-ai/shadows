# AYLUL on the laptop: aylul-hook, sessions, requests and rules (step 2(a))

Date: 2026-10-13. Status: design, reviewed with Mohammed section by section.
This file owns step 2(a) of the first release (the delivery design that named
it was deleted on 2026-10-19; Git `226d0a3`): what AYLUL desktop learns from
Claude Code on the same laptop, how the person answers it there, and how a
permission rule is written. The [code map](../codebase/README.md) owns where
the code lives. Evidence: hook probe (`docs/evidence/nylra/2026-10-13-aylul-hook-probe.md` at
`0187365`).

## 1. What step 2(a) delivers

Everything runs on one Windows laptop with Claude Code. There is no cloud, no
phone and no AYLUL brain in this step.

| The person sees in AYLUL | From Claude Code hook |
| --- | --- |
| **Sessions:** project folder, state (`running`, `waiting`, `ended`), last step, Claude's last message | `SessionStart`, `PostToolUse`, `Notification`, `Stop`, `SessionEnd` |
| **Requests:** a tool call Claude Code is asking about; the person allows or denies it in AYLUL or in the terminal, and the first answer decides | `PermissionRequest` |
| **Refusals:** a tool call auto mode denied, with its reason and a proposed narrow permission rule; approving the rule writes it into the project's `.claude/settings.local.json` | `PermissionDenied` |

The person runs Claude Code in auto mode, where `PermissionRequest` rarely fires.
A refusal is the common case: today the person switches mode, asks Claude to try
again, approves, and switches back. Step 2(a) replaces that with one approval of
a rule in AYLUL.

Not in this step: the phone and cloud (2(b)), the brain (2(c)), rule expiry and
revocation (2(d)), Codex (after its hooks are tested), Shadows.

## 2. Components

| Part | One job |
| --- | --- |
| `aylul-hook` (Rust `.exe`, `apps/desktop/hook/`) | Carry one hook call: read the event, send it over the pipe, print the reply |
| `sidecar/src/hooks/` | Accept hook connections and route each event |
| `sidecar/src/sessions/` | Build sessions and trimmed steps from events |
| `sidecar/src/approvals/` | Hold a request or refusal until the person answers or its time ends |
| `sidecar/src/agents/` | Change Claude Code's settings: install the hooks, write a rule |
| `sidecar/src/store/` | Keep sessions, requests, answers and written rules in `node:sqlite` |
| `src/features/sessions/`, `src/features/approvals/` | The two screens |

Installing the hooks writes them into `~/.claude/settings.json`, so every
session in every project reaches AYLUL. It happens once, after the person has
seen the change. Hook entries use exec form with the absolute path of
`aylul-hook.exe`; `PostToolUse` runs with `async: true`.

## 3. The pipe

- **Name and token.** Each time the sidecar starts it creates a pipe with a
  random name and a random 32-byte token, and writes both to
  `%LOCALAPPDATA%\AYLUL\hook.json`, readable only by the person's Windows
  account. `aylul-hook` reads that file and sends the token with every message;
  the sidecar drops a connection whose first message lacks it. Node cannot set a
  pipe's access list, so the secret is the boundary.
- **Messages.** One JSON line each way. Request: `{"v":1,"token":…,"project_dir":…,"event":<the
  hook input as received>}`. Reply: `{"output":<hook output JSON>}` or `{}`.
  `aylul-hook` prints `output` as received and exits 0.
- **The project is the session's root.** `aylul-hook` sends `CLAUDE_PROJECT_DIR`,
  which Claude Code sets for every hook, as `project_dir`. The event's `cwd`
  follows `cd` into subfolders, and a subfolder's `.claude/settings.local.json`
  is not read by Claude Code, so sessions and rules use `project_dir`. Without it
  no rule is proposed or written.
- **Steps are trimmed in the sidecar** before they are stored or shown: tool
  name, path or command, and one line. `tool_response` content, `originalFile`,
  `structuredPatch` and file text are dropped; the probe saw them in
  `PostToolUse`.
- **Keys.** Events are keyed by `session_id`; `Notification` arrives from idle
  sessions too. `PermissionRequest` has no `tool_use_id`, so a request is
  matched to its later `PostToolUse` by session, tool name and trimmed target
  (the latest unmatched one wins).

## 4. Answers

| Event | Sidecar reply |
| --- | --- |
| Sessions, steps, last message | `{}` at once |
| `PermissionRequest` | Held until the person answers in AYLUL: `allow` or `deny` in `decision.behavior`. If the person answers in the terminal first, Claude Code ends the hook, the pipe closes, and the request is recorded as answered in the terminal; a following matching `PostToolUse` shows it was allowed |
| `PermissionDenied` | Held for two minutes. If the person approves the proposed rule within that time, the rule is written first, then `{"retry": true}` is returned so Claude tries again by itself. Otherwise `{}`: the refusal stays listed and the rule can be approved later, after which the person asks Claude to retry |

## 5. Rules

- **Proposed exact.** Bash and PowerShell: the exact command,
  `Bash(npm run deploy:staging)`. Edit, Write and Read: one file from the project
  root, `Edit(/src/app.ts)`. WebFetch: the host, `WebFetch(domain:example.com)`.
  An MCP tool: its full name. Nothing else is proposed.
- **No wildcard, ever.** Claude Code reads `*` in a Bash rule as any text, so
  `Bash(rm -rf build/*)` would allow more than that command; a path without a
  leading `/` matches the same name in every folder, so `Edit(a.txt)` would
  allow every `a.txt`. A command containing `*`, or a path containing `*`, `?`,
  `[` or `]`, gets no proposal. The person may edit the rule before approving,
  and an edited rule is accepted only in the same exact shapes
  ([permissions reference](https://code.claude.com/docs/en/permissions), read
  2026-10-13).
- **Written where.** The project's `.claude/settings.local.json`, under
  `permissions.allow`. It stays with that project, is not committed, and does not
  reach the person's other projects.
- **Written how.** Read the file; refuse and report if it is not valid JSON;
  copy it to `%LOCALAPPDATA%\AYLUL\backups\` with a timestamp; show the diff; on
  approval write a temporary file and rename it over the original. The approval
  carries a fingerprint of the file the preview showed; if the file changed
  since, nothing is written and the person previews again.
- **Recorded.** Each rule: the text, the project, the time, the person's
  approval and the backup path. A later `PostToolUse` that the rule covers is
  recorded as `aylul_grant` with the rule's id.
- **Applied live.** The probe wrote an allow rule while a session ran; the next
  matching call ran without a prompt three seconds later.

> **OPEN: does a project allow rule let auto mode run a call its classifier
> denies?** Claude Code's own denial text says a permission rule allows the
> action in future. It was not tested: building a denial on purpose was refused
> by the classifier as `[Auto-Mode Bypass]`. Closes at the first real refusal in
> the person's work: approve the proposed rule, ask Claude to retry, record
> whether it ran.

## 6. Failure

- No sidecar, no `hook.json`, a wrong token, a closed pipe or a timeout:
  `aylul-hook` prints nothing and exits 0. Claude Code then behaves as if AYLUL
  were absent: the dialog stays, the refusal stands.
- AYLUL never answers `allow` or approves a rule without the person.
- A refusal is never turned into an allowed call by a hook decision. The only
  path is a rule the person approved, which Claude Code itself enforces.
- A settings file that is not valid JSON is never written.

## 7. Tests

- **`aylul-hook`:** no `hook.json` or no pipe gives no output and exit 0; a
  reply is printed unchanged; a stalled server is cut at the hook's own deadline.
- **Sidecar:** no file content survives step trimming; the first answer decides
  and a closed pipe records a terminal answer; a request with no answer returns
  `{}`; proposed rules are narrow for each tool; the settings writer backs up,
  writes atomically and refuses invalid JSON; a message without the token is
  dropped.
- **Acceptance on Windows with real Claude Code:** the probe's runs as a
  journey: a session appears with its steps and last message; a request is
  allowed from AYLUL and the dialog closes; the person answers in the terminal
  and AYLUL records it; with AYLUL off nothing changes. Plus the OPEN above at
  the first real refusal.
