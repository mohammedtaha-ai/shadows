//! Test-support binary. Not product code. It exists so `tests/planner_turn.rs`
//! has a real child process that speaks the Claude stream-JSON shape without
//! requiring the real `claude` CLI to be installed.
//!
//! Behavior is selected by the invocation's last argument — the prompt, per
//! `ClaudeHarness::to_process_spec`, which always appends it last:
//!
//! | prompt | behaviour | what it lets a test observe |
//! |---|---|---|
//! | `hang` | one entry, then sleeps | a Running turn that can be cancelled |
//! | `slow-exit` | one entry, a turn-end, then sleeps before exiting | the window in which the turn has already produced its ending but the process is still alive (§8.4 case 4) |
//! | `crash` | one entry, then exits non-zero with no turn-end | a child that dies mid-turn (§8.4 case 4's `Failed` half) |
//! | anything else | one entry, one turn-end, exits 0 | an ordinary completed turn |

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

    // A child that ends mid-turn: no result line, and a status that says so.
    if prompt == "crash" {
        std::process::exit(3);
    }

    let turn_end = serde_json::json!({
        "type": "result",
        "subtype": "success",
        "stop_reason": "end_turn",
    });
    println!("{turn_end}");
    std::io::stdout().flush().expect("stdout flush");

    // The turn is over and the process is not: this is the window in which a
    // cancellation must not claim to have stopped anything (§8.4 case 4).
    // stdout stays open, so the reader is still blocked on it and cannot have
    // reached its own arbitration yet.
    if prompt == "slow-exit" {
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
