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
//! | `failing-turn-end` | one entry, a turn-end whose subtype is not `success`, exits 0 | a turn the harness says failed while the process says it is fine |
//! | `report-invocation` | one entry whose text is `{"cwd", "args"}` as JSON, one turn-end, exits 0 | where a turn ran, and with which session flags |
//! | anything else | one entry, one turn-end, exits 0 | an ordinary completed turn |

use std::io::Write;

fn main() {
    let prompt = std::env::args().next_back().unwrap_or_default();

    let text = if prompt == "report-invocation" {
        serde_json::json!({
            "cwd": std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default(),
            "args": std::env::args().skip(1).collect::<Vec<_>>(),
        })
        .to_string()
    } else {
        "hello from fake_claude".to_string()
    };
    let entry = serde_json::json!({
        "type": "assistant",
        "uuid": "fake-entry-1",
        "message": {
            "role": "assistant",
            "content": [{ "type": "text", "text": text }],
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

    // A turn the harness itself says did not succeed, delivered exactly the
    // way a successful one is: a result line and a clean exit. The subtype
    // string is not a measured value and nothing depends on which one it is —
    // what is under test is that a verdict other than `success` is not
    // recorded as a completed turn.
    let subtype = if prompt == "failing-turn-end" {
        "error_max_turns"
    } else {
        "success"
    };
    let turn_end = serde_json::json!({
        "type": "result",
        "subtype": subtype,
        "stop_reason": "end_turn",
    });
    println!("{turn_end}");
    std::io::stdout().flush().expect("stdout flush");

    // The turn is over and the process is not. stdout stays open, so the
    // reader is still blocked on it and cannot have reached its own
    // arbitration: a cancellation arriving here is §8.4 case 3 (a live tree,
    // terminate and reap it) whose ending was already decided by the turn.
    //
    // The sleep is long on purpose. At two seconds a `stop` that terminated
    // nothing would still see the process disappear on its own, and a test
    // could not tell "we killed it" from "we waited for it".
    if prompt == "slow-exit" {
        std::thread::sleep(std::time::Duration::from_secs(120));
    }
}
