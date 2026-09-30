# Milestone 3 on Windows — the run

- **Date:** 2026-09-30
- **Build:** `shadows serve --debug`, a debug build of branch
  `milestone-3/code-index` at `56ca6ad`
- **Harness:** adapter 0.81.1, Claude Code 2.1.281 (`harness.versions`)
- **Database:** a fresh one, deleted after the run
- **OS:** Windows 11 Pro 10.0.26200, Windows Defender on
- **Projects:** `shadows` (`E:\Globalprojects\shadows`, 277 indexed files) and
  the old project `shadow` (`E:\Globalprojects\shadow`, 242 files), linked
  `shadows` → `shadow` for step 4
- **Asker:** a Claude Code session outside Shadows, holding a project grant
  from Project settings → External agents → Connect, calling the tools over
  `/mcp`

## §15.11's acceptance

| Step | Result |
|---|---|
| 1. First index | `code.scan … seen=277 indexed=277 elapsed_ms=7806` for `shadows`; `seen=242 indexed=242 elapsed_ms=15684` for `shadow`. The periodic scan then ran every 60 s at 87–218 ms with `indexed=0`. |
| 2. "Where is it defined?" over `/mcp` | `where_is`, `who_uses` and `outline` answered with file, line, kind and signature, and the session did not read the file. For example, `where_is AppCore` answered `crates/shadows-core/src/app.rs:55 class AppCore — pub struct AppCore {`. The controller checked every line against the file. |
| 3. Edit, save, ask again | Two lines were added above `adapter_version`. `where_is` answered `app.rs:298` before the edit and `shadows: crates/shadows-core/src/app.rs:300 function adapter_version — …` after it. The edit was reverted. |
| 4. A linked project answers | `where_is verify_digest` answered `shadow: crates/adapters/shadow-context/src/lib.rs:334 function verify_digest — pub fn verify_digest(packet: &ExecutionPacket) -> Result<bool, ContextError> {`, with and without `project: "shadow"`. It carried a status line for each project. The name exists only in `shadow`. |
| 5. Restart | After the daemon was killed and started again, both projects scanned with `indexed=0` (277 and 242 files) in about 1 s. Nothing was rebuilt from nothing. |

## The final review's further checks

All were run on `E:\Globalprojects\shadow`, and every change was undone
afterwards: its `git status` is empty and its folder has its name back.

| Check | Result |
|---|---|
| The watcher alone | A new `.rs` file was answered after 171 ms. After it was deleted, it was gone within 209 ms. No question re-check was involved. |
| `cargo build` inside the project | A small crate was built in the project, writing 15 files into its `target/`. None of them was indexed, and its `src/main.rs` was. The watcher reported no overflow. There was one `code.folder_changed` scan, for the new folder. |
| A folder renamed inside the project | `verify_digest` answered at `shadow-context-m3/…` 863 ms after the rename, and at its old path 955 ms after the rename back. |
| The project folder renamed away and back | The next periodic scan logged `code.directory_missing`, and the status answered `directory_missing`. Questions still answered from the kept index (`lib.rs:334`). After the rename back, the next periodic scan logged `code.directory_back`, and a new file was indexed 968 ms after it was written. |
| A junction inside the project pointing outside it | `mklink /J shadow\m3junction <scratch>\m3out`, then a `.rs` file written and rewritten in the target. Nothing in it was answered after a watcher event or after a periodic scan. |
| `active_limit` 1 with a link | `shadows` answered `ready` and the linked `shadow` answered `inactive`, from the index it had. Setting the limit back to 5 made `shadow` answer `ready`. |
| Ctrl+C during a first index | **Not run.** This shell cannot send Ctrl+C, so the daemon was stopped only with a hard kill between steps. |

## Observed, not yet explained

- **A save by a tool that writes through a non-hidden temporary name starts a
  full scan.** The edit in step 3 logged `code.folder_changed`, then a scan of
  207 ms with `indexed=1`, instead of a `Files` batch. The likely cause is that
  the temporary file is gone after its rename and is not a code file, so it
  meets §15.4's rule for a gone path. The answer was right; the cost was one
  walk.
- **The periodic scan costs 87–218 ms in the daemon**, against the probe's
  9 ms for the walk alone. The daemon's scan also reads each file's row, and
  this was a debug build.
