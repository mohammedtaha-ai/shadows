# The code watcher on Linux — reads reported as changes

- **Date:** 2026-10-01
- **OS:** Linux 6.18.44 (a cloud container, 4 cores)
- **Build:** `shadows serve --debug`, a debug build of branch
  `harness/effort-at-once`. Before: at `19d4d8f`. After: `19d4d8f` plus the
  `fix(code)` commit that follows it, which adds the filter in `forward`
  (`crates/shadows-core/src/code/watch.rs`).
- **Database:** a copy of a working database holding four active projects:
  `shadows` (`/home/user/shadows`, 296 files seen) and three small throwaway
  ones (0–1 file each).
- **Method:** start the daemon, touch nothing for 45 s, then read its CPU with
  `ps -o %cpu=` and count its log lines by target and message.

## Numbers

| | Before | After |
|---|---|---|
| `code.scan` in 45 s | 596 | 4 (one first scan per project) |
| `code.watcher_overflow` in 45 s | 489 | 0 |
| CPU (`ps %cpu`, lifetime average at 45 s) | 148 % | 3.4 % |
| CPU over the next 10 s (`/proc/<pid>/stat` ticks) | — | 1.2 % |

After 60 s the periodic scan ran once per project (`indexed=0`), as designed.

## Cause

`notify` 8.2.0's inotify backend (`notify-8.2.0/src/inotify.rs`) adds
`WatchMask::OPEN` to every watch (line 427, beside `CLOSE_WRITE`, `MODIFY`,
`CREATE`, `DELETE`, `ATTRIB` and the moves), and turns the masks it receives
into events:

- `EventMask::OPEN` → `EventKind::Access(AccessKind::Open(AccessMode::Any))`
  (lines 349–356);
- `EventMask::CLOSE_NOWRITE` → `Access(AccessKind::Close(AccessMode::Read))`
  (lines 333–339);
- `EventMask::CLOSE_WRITE` → `Access(AccessKind::Close(AccessMode::Write))`
  (lines 325–331).

So every file a scan reads is reported twice. `forward` looked only at the
paths, never at `event.kind`: each read came back as a changed path. The
paths arrived while the worker was busy scanning, filled the 8192-slot
channel, set `lost`, and `lost` started another scan
(`code.watcher_overflow`), whose own reads came back again; a batch that did
get through became a `Files` job that read its files once more. The loop
needs no outside change to keep going. Which reads filled the channel each
time (the walk's, the ignore files', a new watcher's own walk of the tree)
was not taken apart: once reads are dropped, none of them is a path. Windows'
`ReadDirectoryChangesW` reports no reads, which is why the Windows run
(`WINDOWS_RUN.md`) never showed it.

## The fix

`forward` drops every `EventKind::Access(_)` but
`Access(Close(AccessMode::Write))`, which is kept because some editors' saves
arrive only as one. The unit test `a_read_is_not_a_change` holds it; with the
filter removed it failed at its first assertion.

## A real edit is still seen

With the fixed daemon running, in the throwaway project `throwaway2` (an empty
folder, not the repo): `a.rs` was written with `fn first_probe() {}`, then a
line `fn appended_probe() {}` was appended. Within 3 s the definitions route
answered `appended_probe` at `a.rs:2`, while the project's `updated_at` still
named the periodic scan from before the file existed, so the watcher indexed
it. After `a.rs` was deleted, `first_probe` answered no hit and the project
counted 0 files.
