# Milestone 1 Phase B — Run on the real harness (Linux, cloud container)

**Date:** 2026-09-24 (UTC)
**Status:** The Phase B controls ran against the real harness on Linux, in a
browser. The run found four defects, and each was fixed and run again. This is
**not** Mohammed's run, which spec §12.13 requires, and it is not on Windows.
See "What this run does not establish".
**Code:** branch `milestone-1/harness-controls-7p9608`. The run started at
`a382eb6`. The fixes it found are `32c5a0e` and `41bc584`. The whole-branch
review's fixes (`f7b7c51`, `f2d2a80`) were merged in before the final pass.
**Operator:** Claude, in a Claude Code cloud session. The browser steps used
headless Chromium driven by Playwright. The API checks used `curl`. Mohammed
followed the run through screenshots.

## Environment

| Component | Version |
|---|---|
| OS | Linux 6.18.44 (cloud container) |
| rustc | 1.94.1 (e408947bf 2026-03-25) |
| Claude Code CLI (agent) | 2.1.281, at `/opt/claude-code/bin/claude` |
| ACP adapter | `@agentclientprotocol/claude-agent-acp` 0.81.1 |
| Node.js | v22.22.2 |
| Daemon | `target/release/shadows serve --debug --db <tmp> --node <node> --adapter <adapter entry> --harness /opt/claude-code/bin/claude` |
| Web client | `web/`, `npx vite` on `http://localhost:5173` |

## Gates at `41bc584`

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --all-targets --features test-support -- -D warnings` | 0 |
| `cargo test --features test-support` | 197 passed, 0 failed |
| `web/`: typecheck, lint, build | 0 |
| `web/`: `npx vitest run` | 91 passed |

## Steps (final pass, fresh database, one project on a new `git init` folder)

| Step | Result |
|---|---|
| Model menu | ✅ Built from the session: Default (recommended), Sonnet 5, Fable 5.1, Opus 5.5, Haiku 4.5. No model name comes from Shadows' code |
| Effort menu | ✅ Default, Low, Medium, High, Xhigh, Max for Opus 5.5 |
| Mode menu | ✅ Accept edits and Auto only. A new conversation starts at Accept edits |
| Choose a model | ✅ Opus 5.5 chosen; the menu closed |
| A turn that runs shell commands | ✅ `ls -la && git status` ran, and Claude described the folder |
| Stop a long reply (Low effort, Stop after 7 s) | ✅ `Cancelled`; the partial essay stayed as an entry |
| A command outside Accept edits (`curl … -o page.html`) | ✅ A `PermissionRefused` line: "refused in Accept edits. Auto would allow it". No file was written |
| Context ring | ✅ "17k / 1M (2%)". Limits: "no figures yet" |
| Context details | ✅ The breakdown from `/context`: system prompt, system tools, deferred tools, skills, messages, free space, autocompact buffer |
| Copy | ✅ The clipboard held the reply's text |
| Fork from the last message | ✅ A new thread "Conversation 1 (fork)" holding the copied entries |
| Ctrl+C, then restart | ✅ `stop_kind=Graceful`, no adapter process left |
| Ask the source for the secret word | ✅ "PAPAYA", from the first turn |
| Ask the fork for the secret word | ✅ "PAPAYA" |
| Remembered settings after restart | ✅ After the fix below: the composer showed Opus 5.5 and Low, three restarts out of three |
| Each turn's invocation | ✅ `requested_model`, `requested_mode` and `requested_effort` matched the choice. `observed_model` read `claude-opus-5-5`. The stopped turn's observed model stayed NULL |

## Defects found and fixed

1. **No shell command could run.** `process::spawn` clears the environment,
   and the adapter was given only `CLAUDE_CODE_EXECUTABLE`. With no `PATH`,
   every Bash call (`git`, `ls`, `mkdir`) exited 127. `PATH`, `HOME`, `TMPDIR`
   and `LANG` are now passed by name. Phase A did not catch this because its
   commands were refused before they ran. Fixed in `32c5a0e`.
2. **A menu stayed open after a choice** and covered the Send button. Base UI
   keeps a radio menu open by default. Fixed in `32c5a0e`.
3. **Allowed modes could turn a mode back on.** A second click, just after the
   first save answered, was computed from the old set. Fixed in `32c5a0e`.
4. **The remembered effort showed as Default after a restart.** The daemon
   applied Low correctly. But opening sets mode, then model, then effort, and
   each step's `options` frame reached the page before the opening's answer.
   The first step became the composer's settings. Fixed in `41bc584`: frames
   are ignored while the opening is in flight.

## Observed along the way

- **Accept edits deletes without asking.** `rm -rf ./README.md` ran with no
  permission request, so the daemon never saw one. The file had never been
  committed, so it was gone. Claude Code treats file commands inside the
  working directory as edits. Mohammed's ruling and the spec text are in
  §12.5.
- **A model change reaches the effort menu only at the next Send.** The model
  was set during turn validation, so a stale effort was refused once
  (`SettingNotOffered`), then corrected. Closed by §12.7's new model route.
- **The ring's limits read "no figures yet" for the whole run.** The harness
  sent no rate-limit report in these turns. The ring said so and did not spin,
  as §12.8 requires.

## Follow-up run: the rulings (`bcc9ba8`)

After Mohammed's rulings, a fresh database and a new conversation:

| Step | Result |
|---|---|
| Mode menu | ✅ Each mode carries its line; Accept edits: "Claude Code edits, creates and deletes files in the project folder without asking. Other commands are refused." |
| Choose Haiku 4.5, no Send | ✅ `PUT …/session/model {"model":"haiku"}`; the effort menu went away at once |
| Choose Fable 5.1, no Send | ✅ Accepted with its efforts. A turn on it completed, `observed_model` `claude-fable-5-1`. The account had access that day |
| Choose Opus 5.5, no Send | ✅ The effort menu showed Default to Max before any Send. A turn at Max completed and recorded `requested_effort` `max` |
| An unknown model | ✅ `SETTING_NOT_OFFERED`, "model nope is not offered" |
| Fork, then change its CLI | ✅ `HARNESS_LOCKED` on a fork with no turn of its own. The picker shows the lock |

gates at the follow-up: 202 Rust tests, 95 web tests, fmt, clippy, typecheck,
lint and build clean.

## What this run does not establish

- **Mohammed's run, and Windows.** §12.13 asks Mohammed to run every Phase B
  item with the real client. Phase A's Windows run has not happened either.
- **A harness refusing a model.** Fable 5.1, refused when §12.4 was written,
  was accepted in the follow-up run below, so no real refusal was seen.
  `fake_acp` covers the refusal path.
- **A human in the browser.** Playwright drove the page. What is described here
  was checked from screenshots and from the database.
- **Linux parent-death containment.** The daemon was only stopped with
  Ctrl+C. Spec §1.5's OPEN block still stands.
