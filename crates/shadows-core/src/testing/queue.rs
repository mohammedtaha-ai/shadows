//! Shared apparatus: a waiting message for the queue's store tests.

pub use core_turns::{NewQueued, QueueAnswer};

/// A waiting message with the fixture's settings (`fake-small`, `acceptEdits`, `high`).
pub fn new_queued(prompt: &'static str) -> NewQueued<'static> {
    NewQueued {
        prompt,
        attachments: &[],
        model: "fake-small",
        mode: "acceptEdits",
        effort: Some("high"),
        focus: None,
        plan: None,
    }
}
