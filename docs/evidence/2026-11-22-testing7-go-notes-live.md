# Go notes from planning to execution on Windows

Date: 2026-11-22 (Asia/Aden). Shadows source `19b64132c28aa1340c71033a77f54215d35f032b`
on `ccr-4d8285a3-xgpdjb`. Built the debug daemon and ran it with the `daemon` and `web`
entries in `.claude/launch.json`, using the existing local database. `testing7` was a new
project with an empty folder in that database. Go was `go1.26.5 windows/amd64`.

The Planner used native Codex `gpt-6.2-sol`, medium effort, in Agent Mode. Setup's first
executor was `gpt-6-luna` at low effort. The retry and all feature tasks used
`gpt-6.1-sol` at medium effort. The initial English request was sent through the web
client using Playwright. After the user asked to keep their existing tab, the separate
browser session was closed. The rest of the interaction used the local HTTP API against
the same Planner conversation. The run did not exercise later approvals or navigation by
clicking through the UI.

This is a Go and Codex follow-up to `docs/evidence/2026-11-21-code-record-2-live.md` at
`19b64132`. The fixture is a standard-library Go CLI with `add` and `list`, a local JSON
file, and a `--file` override for isolated tests.

## Planning and execution

| Step | Observed result |
|---|---|
| Vision and map | The Planner recorded the vision. Its first map used one path for both the JSON schema and connection layers. `map_propose` returned `MAP_REFUSED: layer_place_shared`. A second proposal used separate paths and was approved with waivers for the web API and graphical frontend. |
| Roadmap and Setup draft | The roadmap was approved. The first `plan_edit` request was missing `id`. The next request produced a draft. A review caught that the empty folder had no Git repository and that `git status --porcelain` could exit zero while printing changes. Revision 2 made Git initialization a prerequisite and made the clean-worktree check fail on nonempty output. The Planner initialized Git through `project_git_init`. |
| Setup T1, first attempt | The executor created `go.mod` and `cmd/notes/main.go` and reported passing Go checks. Git refused staging with `.git/index.lock` permission denied through the retries. The task stayed `running` for a time after the executor reported the failure, then ended `failed` with `ended without a commit`. No `index.lock` remained on disk. |
| Setup T1, retry | The Planner said that the two untracked files prevented a retry. The task API returned `start.allowed: true`, and Start successfully admitted both files as leftovers from the failed attempt. With `gpt-6.1-sol`, sandboxed staging was refused again. Scoped approval for the exact Git operations let the executor stage, commit and report `fa15e24`. Shadows reran all four checks, and the Planner marked Setup T1 done. |
| Feature draft | The first two `plan_edit` calls were refused. The symbol kind `interface` was outside the accepted enum (`function`, `method`, `class`), and four Go symbols with no return value failed return-type validation. The draft was saved after the Planner represented the repository interface as `class` and used `no return values` where necessary. |
| Feature execution | T1 (storage) and T2 (application) ran in parallel. Once both were marked done, the Planner started T3 (CLI and subprocess tests). The T3 executor first hit a temporary-file rename `Access denied` in its sandbox. It reran the checks with `TMP` and `TEMP` set to `cmd/notes` for that check process, then used scoped Git approval to commit. Shadows passed the CLI checks, full Go suite and contract checks and reran the prerequisite task checks. The Planner marked all three tasks done. |
| Recheck | Shadows reported that Setup T1's `go run ./cmd/notes` had begun exiting 1 and printing `usage: notes [--file PATH] add TITLE \| list`. The check expected the empty scaffold's behavior. The no-argument invocation is invalid in the finished CLI. The done mark stayed in place. The follow-up draft proposes `go test ./cmd/notes -run '^$'` as a compile-only replacement. |

The local fixture had no Git remote and was clean at the end of the run.

| Commit | Fixture change |
|---|---|
| `40c6bd6270f97d78c29154a97244cf5f6c30d5ac` | Initial Git commit created by Shadows |
| `fa15e240d682122c597b20cab050696062e9141d` | Setup module and command scaffold |
| `cdbd3ebdfac64cb421f933c95d78457186a57ad2` | Title validation and add/list use cases |
| `2a059a935d93dc916bac99bbb51f0bad7c69abec` | Validated local JSON storage |
| `a9297baf8018ccf5af6d7dd1e8919a5cf527c950` | File-backed CLI and subprocess tests |

## Independent CLI checks at `a9297ba`

`go test ./... -count=1`, `go vet ./...` and `go build -o <temp>/notes.exe ./cmd/notes`
exited zero. The tests passed in `cmd/notes`, `internal/notes/application` and
`internal/notestore`. The built executable passed these checks with temporary `--file`
paths:

- Listing a missing file produced empty stdout and left the file absent.
- Adding the first note succeeded.
- Adding an Arabic title succeeded.
- A fresh `list` process returned both titles in the order they were saved.
- Listing a second storage path returned an empty list and left that path absent.
- Listing a file containing `[]` succeeded with empty stdout.
- An empty title and a whitespace-only title each returned a nonzero exit and a title
  error on stderr. The existing file's SHA-256 stayed equal to the baseline.
- `list` and `add` each rejected malformed JSON and left its SHA-256 unchanged.
- An unknown command returned a nonzero exit and stderr without changing the stored notes.

### Lost note during concurrent adds

Two processes added different titles to a temporary JSON file seeded with 30,000 titles.
Both exited `0`. The file contained 30,001 titles, one short of the expected 30,002.
`parallel-B` was present and `parallel-A` was absent. `internal/notestore/file.go` reads
the existing JSON, writes a new temporary file and renames it over the original. There
is no lock covering the read and replacement, so a second writer can publish a copy of
the same starting state.

Run the following in PowerShell 7 from the fixture root.

```powershell
$scratch = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ('notes-race-' + [guid]::NewGuid()))
$exe = Join-Path $scratch.FullName 'notes.exe'
$db = Join-Path $scratch.FullName 'parallel.json'
go build -o $exe ./cmd/notes
if ($LASTEXITCODE -ne 0) { throw 'Go build failed' }
$note = '"' + ('x' * 256) + '"'
Set-Content -LiteralPath $db -NoNewline -Encoding utf8NoBOM -Value ('[' + ((@($note) * 30000) -join ',') + ']')
$argsPrefix = '--file "' + $db + '" add '
$a = Start-Process -WindowStyle Hidden -FilePath $exe -ArgumentList ($argsPrefix + 'parallel-A') -PassThru
$b = Start-Process -WindowStyle Hidden -FilePath $exe -ArgumentList ($argsPrefix + 'parallel-B') -PassThru
$a.WaitForExit()
$b.WaitForExit()
$titles = Get-Content -Raw -LiteralPath $db | ConvertFrom-Json -NoEnumerate
@($a.ExitCode, $b.ExitCode, $titles.Count, ($titles -contains 'parallel-A'), ($titles -contains 'parallel-B'))
```

Running this block during documentation verification again returned
`0, 0, 30001, False, True`.

The Planner drafted **Concurrent Note Adds**, with a per-file lock and a fresh-process
regression test. The plan remains `Draft`. No task from that plan ran.

## Code record and live Go references

- Before Setup was done, the Planner's carried context had `code_record` version `0`
  and the message `No code recorded yet`.
- After Setup, `GET /api/operations/{id}/context` returned `heard: true` for the feature
  planning turn. The block held `go.mod` and `cmd/notes/main.go`, with `code_record`
  version `1`. `code_where` for `cmd/notes/main.go` returned `file_only`, a live
  confirmed path and the Setup commit `fa15e24`.
- After the three feature tasks were done, a Planner notification carried version `4`
  with `internal/notestore/file.go` and the updated description for `cmd/notes/main.go`.
  `heard` was `true`. The next request in the same conversation drafted the concurrency
  fix using `code_where` and `task_file`; that request had no separate carried-context
  record (`CONTEXT_NOT_CARRIED`).
- `code_where` for `ValidateTitle` returned `at_done: found`, live `confirmed`,
  `uses_live: answered` and one usage. A new Go file in the same package referencing the
  function increased the count to two within 0.5 seconds while `gopls` was running. The
  new file appeared in the references. The probe was `package domain` followed by
  `var shadowsProbeValidate = ValidateTitle` in a new `internal/notes/domain/*.go` file.
  Removing that probe restored the count to one, with `gopls` and the daemon still running.

## Diagnostics and evidence anchors

When the Planner's file-change detection saw files and directories made by the executor,
it asked the Planner to explain them as changes from its own turn. `project_change_note`
rejected a directory with `INVALID_COMMAND: the file is not in the project`. The Planner
successfully wrote a note for the two files and said that the executor created them.

The daemon inherited `RUST_LOG=warn`, which overrides the debug level selected by
`--debug`. The debug file was empty when inspected. Operation entries and captured
stderr supplied the detail above. Requesting `GET /api/harnesses/claude-code/catalog`
failed with `HARNESS_START_FAILED`. The relevant stderr entry at 02:21:07 Asia/Aden
reported `model claude-fable-5-1` and `API error: 403 Request not allowed · model not
changed`. This run used Codex for all successful attempts. It did not establish Sonnet
availability or a provider quota status.

The local IDs below let a reader with the same database look up entry and attempt
records. Operations expose carried context through `/api/operations/{id}/context`.
Planner entries are available from `/api/threads/{id}/entries` and task attempts from
`/api/workflows/{id}/versions/1/tasks/{number}/attempts`.

| Record | Local ID |
|---|---|
| `testing7` project | `309b4ae1-db5e-44e3-9c30-9140b5d676e7` |
| Planner conversation | `16c0301b-0145-4d2b-bcad-ec4e08f91bfa` |
| Setup workflow | `458953c0-3363-4c1f-a0be-9ddea53408b3` |
| Setup failed attempt, then successful retry | `423fa392-0fcc-4352-ba6c-5cda4982ec1c`, `51af234f-bff1-4d6e-86de-585d4eecf152` |
| Add and List Notes workflow | `289f22f6-903f-4810-ad8e-e506690bad38` |
| Feature planning turn with record version 1 | `55e7ab54-002c-40bd-a29e-a31144849b28` |
| Notification with record version 4 | `87849ed1-6e4c-4ed0-9199-4139c83a0b03` |
| Concurrent Note Adds draft | `1326e8ce-da5f-4abe-8ba0-6baa5899a694` |

The experiment covers the Go CLI, Codex execution and live Go references. The later
Planner turns used the existing conversation. It did not repeat the new-conversation
acceptance, stopped-language-server or Python cases from the earlier evidence record.
