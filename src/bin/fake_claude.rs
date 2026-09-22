//! Test-support binary. Not product code. It exists so `tests/planner_turn.rs`
//! has a real child process that speaks the Claude stream-JSON shape without
//! requiring the real `claude` CLI to be installed.
//!
//! Behavior is selected by the invocation's last argument — the prompt, per
//! `ClaudeHarness::to_process_spec`, which always appends it last. The
//! literal prompt `"hang"` emits one durable entry and then sleeps, so a test
//! can observe the operation Running and cancel it. Anything else emits one
//! durable entry and a turn-end result line, then exits normally.

use std::io::Write;

fn main() {
    let prompt = std::env::args().next_back().unwrap_or_default();

    let entry = serde_json::json!({
        "type": "assistant",
        "uuid": "fake-entry-1",
        "message": {
            "role": "assistant",
            "content": [{ "type": "text", "text": "hello from fake_claude" }],
        },
    });
    println!("{entry}");
    std::io::stdout().flush().expect("stdout flush");

    if prompt == "hang" {
        std::thread::sleep(std::time::Duration::from_secs(600));
        return;
    }

    let turn_end = serde_json::json!({
        "type": "result",
        "subtype": "success",
        "stop_reason": "end_turn",
    });
    println!("{turn_end}");
    std::io::stdout().flush().expect("stdout flush");
}
