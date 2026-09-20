# THROWAWAY SPIKE — do not build on this

This is probe code, not product code. It exists only to have answered two
questions, and it is scheduled for deletion.

Findings: [`docs/evidence/harness/SERVE_STREAM_SPIKE.md`](../../docs/evidence/harness/SERVE_STREAM_SPIKE.md)

It has no persistence, no modules, no process-tree containment, no
cancellation, no idempotency and no error taxonomy. None of those were being
probed. Nothing here is a design proposal.

## Run it

```
cargo run
```

Then open `http://127.0.0.1:4317` yourself. The daemon does not open a browser.

Each `Run turn` spawns a real `claude` child process and streams it to the page
over SSE. **This consumes real usage.** The three panes correspond to the three
stream classes described in the findings: transient deltas, durable entries,
and operational events.

`Forget session` drops the stored session id so the next turn starts a fresh
conversation instead of `--resume`-ing the previous one.
