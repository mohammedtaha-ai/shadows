# Project plans 1a: Windows checks

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

## Reviewed build and human trial, 2026-10-03

The ordinary debug daemon was rebuilt from review commit
`fbbdde55cd5e54295aee9829cddd1e142ff26e17` with
`cargo build --locked -p shadows --bin shadows`. The build passed; the
executable and PDB were copied from the prescribed C: scratch target to
`E:/Globalprojects/shadows/target/debug/`. The executable hashes matched,
and `shadows.exe serve --help` exited successfully. Codex did not start it.

After receiving this build, Mohammed reported: "جربته اضن كل شي جاهز هل نفتح pr"
(he tried it and believed everything was ready for a PR). This is the human
trial of the reviewed build, reported by Mohammed. No new runtime failure
was reported.

## Scope of the recorded evidence

The trial was not reported scenario by scenario. This record therefore does
not assign individual pass results to migration on a copy of the dev
database, continuing a plan across conversations, version writers and
reasons, archive/unarchive, deletion during a turn, grant refusal, or Stop
during another session's startup. Task 6b's separate scratch browser check
used fake ACP. These evidence limits do not describe a newly found defect
or a requirement to repeat the completed review before opening the PR.
