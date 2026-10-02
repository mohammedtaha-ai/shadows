# Project plans 1a: partial Windows check

- Date: 2026-10-02
- Branch: `plans/project-plans`
- Code commit: `51f6a2bdcf18b761f431182ff3a0735919d75a5b`
- Design owner: [§16](../../superpowers/specs/2026-10-01-project-plans-design.md)

## Build verified by Codex

`cargo build --locked -p shadows --bin shadows` passed in the ordinary
`dev` profile, with `CARGO_INCREMENTAL=0` and the prescribed C: scratchpad
target. No test-support feature was requested. The executable and PDB were
copied to `E:/Globalprojects/shadows/target/debug/`; the executable's SHA-256
matched the build output, and `shadows.exe serve --help` exited successfully.
The local `daemon` launch entry now uses this executable with `serve --debug`.
Codex did not start the rebuilt daemon.

## Human check reported by Mohammed

Mohammed previously saw `Cannot read properties of undefined (reading 'kind')`
after pressing **Open plan** on the old daemon. After rebuilding, he reported
that Open plan works and that the old daemon caused the error. This is his
reported result; Codex did not independently reproduce or diagnose it.

## Acceptance still to run

This record does not complete §16.12 or Task I. The full real-harness Windows
run, including migration on a copy of the dev database, continuing a plan
across conversations, version writers and reasons, archive/unarchive,
deletion during a turn, grant refusal and Stop during another session's
startup, remains pending. Task 6b's separate scratch browser check used
fake ACP; it does not replace these steps.
