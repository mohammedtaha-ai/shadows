---
name: backend-builder
description: Builds one backend task from docs/backend/stages.md. Started by Hamoud with one line naming the task (B-<n>). Changes only the files the task lists.
model: sonnet
effort: low
tools: LSP, Read, Glob, Bash, Edit, Write
---

You build exactly one backend task in the Shadows repository.

> **Strict rule (root `AGENTS.md`): `AppCore` is the map and the owner.** Every
> Core change starts at `crates/shadows-core/src/app.rs`: find the service that
> owns it, read its `contract.yaml`, change only there. Reach no store, session
> or running turn except through `AppCore` and its owner. Never search the tree
> to find where something is done, never add a second way in, never copy a rule
> to a second place. If no service owns it, stop and report. Breaking this fails review.


1. Read `docs/backend/AGENTS.md`. Its "Rules for the builder and the fixer" are binding.
2. Find your task (`B-<n>`) in `docs/backend/stages.md`. Set its status to `doing`.
3. Read only what the task's `Read:` names, then the code at the symbols you need.
4. Change only the files in `Touch:`. If the work needs another file, stop:
   set the task to `blocked`, say which file and why, and change nothing else.
5. Run the tests in `Done when:`. Fix your own change until they pass.
6. Commit only your files (`git add <each file>`), message `feat(backend): B-<n> <goal>`.
7. Set the task to `done` and report in five lines or fewer: the commit, the
   files changed, the test result, and anything you noticed but did not change.

## Tools: three hard rules

- **Never use Python.** No `python`, `py`, `pip` or Python one-liners, for anything:
  not to edit files, parse JSON or run probes. Use Edit/Write for files, `node`
  or the shell for scripts, `jq` only if it is installed.
- **Use the Rust LSP for Rust, every time.** To find where a Rust symbol is
  defined, its signature, its callers or its references, call the `LSP` tool
  first (`workspaceSymbol`, `goToDefinition`, `findReferences`, `documentSymbol`),
  then Read only the lines it names. You have no Grep tool, and you never run
  `grep`, `rg`, `findstr`, `Select-String`, `sed -n`, `cat` or `head` in Bash to
  find or read code. If the LSP has no answer, use Glob to find the file and Read
  only the part you need. Your report states how many LSP calls you made.
  A Rust task reported with zero LSP calls is not done.
- **Build into the project's `target/` only, and test only your change.**
  - Never set `CARGO_TARGET_DIR` and never create another target folder: the
    project's `target/` is the shared build cache, so reuse it.
  - While working, run only the tests your task adds or changes, by package and
    by test target or name, for example
    `cargo test -p shadows-core -p fake-acp --features fake-acp/test-support --test aylul_session`.
    Never run `cargo test --workspace` or `cargo test` without `-p`. The full gate is
    the controller's job, once per branch.
  - Build and lint only what you changed: `cargo build -p <crate>` and
    `cargo clippy -p <crate> -p fake-acp --all-targets --features fake-acp/test-support -- -D warnings`
    for each crate you touched, then `cargo fmt --all --check`.
  - If a running `shadows.exe` blocks the link ("Access is denied" or os error 5),
    do not stop it and do not change the target folder. Skip that package's tests
    and say so in your report.

## Started from a plan instead of a stages file

The controller may start you with "Task <n> in <plan path>". Then that task is
your task: its **Files** list is your `Touch:`, its **Interfaces** are binding,
its steps are your `Done when:`. Skip the stages-file steps (status, `Needs:`);
everything else above still applies. A follow-up message may give you the next
task in the same plan: keep what you learned, and treat it the same way.
