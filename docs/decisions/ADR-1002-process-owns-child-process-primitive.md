# process/ owns child-process primitive

**Doc ID:** 1002
**Status:** accepted
**Tags:** architecture, ownership, process, verification
**Source slug:** process-owns-child-process-primitive

---

# Decision — process/ owns ProcessSpec; agents translate, process/ does not know

**Status:** accepted · 2026-09-20

## Rule

`process/` is the SOLE owner of the child-process primitive
(`tokio::process`, `process-wrap`, platform-specific semantics).

`process/` knows ONLY about `ProcessSpec` and `ProcessHandle`. It
does NOT know about `AgentInvocation`, Role, Harness, Provider,
Model, Permissions, Claude, Codex, or any domain concept.

## ProcessSpec — the only thing process/ understands

```rust
struct ProcessSpec {
    executable: PathBuf,
    argv: Vec<String>,
    cwd: Option<PathBuf>,
    env: Vec<(String, String)>, // explicit, no inheritance
    timeout: Option<Duration>,
    stdio: StdioSpec,
    isolation: IsolationSpec,
}

struct ProcessHandle {
    pid: u32,
    events: ...,    // stdout / stderr streams
    wait: ...,
    cancel: ...,    // semantic, not signal
}

impl process {
    fn spawn(spec: ProcessSpec) -> Result<ProcessHandle>;
    fn cancel(handle: &ProcessHandle) -> Result<()>;
    fn wait(handle: &ProcessHandle) -> impl Future<...>;
}
```

## Translation lives in the harness, not in process/

```text
agent::start(AgentInvocation)
   ↓
AgentHarness (ClaudeCodeHarness, CodexHarness, …)
   ↓ translates
ProcessSpec { executable, argv, cwd, env, timeout, stdio, isolation }
   ↓
process::spawn(ProcessSpec) → ProcessHandle
```

`process/` does not know whether the executable is `claude`, `codex`,
`cargo`, `pnpm`, or anything else. The harness knows the tool; the
harness builds the spec.

Same boundary for `verification/`:

```text
verification::run(check, output)
   ↓ builds ProcessSpec (e.g., "cargo test --workspace")
process::spawn(spec) → ProcessHandle
```

## Why this matters

If `process/` knew about `AgentInvocation`, it would carry Role /
HarnessProfile / Provider / Model / Permissions — domain concepts.
Then a future Git tooling or shell-runner would have to either go
through `agent/` or carry a parallel concept. Both are wrong.

The boundary says: anything that needs a process turns itself into a
`ProcessSpec`. `process/` is the only thing that turns a spec into
reality.

## Strict ownership consequences

```text
tokio::process outside process/  → fail
process-wrap outside process/    → fail
AgentInvocation outside agent/   → fail
ProcessSpec outside process/ | agent/ | verification/ → fail
```

The third rule is new: `AgentInvocation` only exists in `agent/` (it
is the agent seam contract). `ProcessSpec` is the only thing process
exposes; only `agent/` and `verification/` are allowed to construct
it.

## Who calls process::spawn

- `agent/` (via the harness) — for AI subprocesses
- `verification/` — for deterministic checks (cargo, pnpm, etc.)

`planner/` and `execution/` do NOT call `process::` directly. They
go through `agent::start(...)`.

## Cancellation

`process::cancel(handle)` performs semantic termination of the
managed process tree:

```text
Windows → Job Object kill-on-owner-close with breakaway prevented,
          or an equivalent proven mechanism
Linux   → parent-death/supervisor containment plus process-group cleanup,
          or an equivalent proven mechanism
```

It is NOT `SIGTERM`. The contract is "terminate the managed process
tree", not "send signal N".

## Runtime-loss containment

A managed child process must not survive loss of its owning Shadows
runtime indefinitely. This guarantee covers the complete managed tree,
including grandchildren. A Unix process group alone is not proof of
parent-death containment.

`process/` owns this capability and must prove it with real tests on
Windows and Linux:

```text
daemon → long-lived child → long-lived grandchild
uncleanly terminate daemon
assert child and grandchild do not remain alive indefinitely
```

If `process-wrap` does not provide the full guarantee on a platform,
`process/` adds the required platform-specific mechanism; callers do not
work around it.

## What stays the same

- `storage/` is the sole SQLite owner.
- `protocol/` is the sole HTTP / SSE owner.
- `secrets/` is the sole resolver of secret values.

Related: [[agent-seam-role-agentharnessstart-agentrunhandle]], [[operation-lifecycle-with-two-phase-spawn]], [[architecture-tests-are-mechanical-defense-in-depth]]
