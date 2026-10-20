use super::preview::reply_preview;
use super::projection::*;
fn scope() -> Scope {
    Scope {
        generation: 3,
        configuration_id: "config-a".into(),
        runtime_id: "runtime-a".into(),
        thread_id: Some("thread-a".into()),
    }
}
fn operation(id: &str, status: &str) -> Operation {
    Operation {
        id: id.into(),
        kind: "PlannerTurn".into(),
        thread_id: Some("thread-a".into()),
        status_kind: status.into(),
    }
}
fn entry(id: &str, op: &str, ordinal: i64, body: &str) -> Entry {
    Entry {
        id: id.into(),
        thread_id: "thread-a".into(),
        operation_id: Some(op.into()),
        ordinal,
        kind: "AgentMessage".into(),
        body: body.into(),
    }
}
fn apply(state: &mut Projection, observation: Observation, now: u64) -> PresenceProjection {
    state.apply(
        PresenceInput {
            scope: scope(),
            observation,
        },
        now,
    )
}
fn live() -> Projection {
    let mut state = Projection::new(scope(), true, true);
    apply(&mut state, Observation::Ready, 0);
    apply(&mut state, Observation::CaughtUp, 0);
    state
}
fn snapshot(
    state: &mut Projection,
    id: &str,
    status: &str,
    entries: Vec<Entry>,
    completed: Option<&str>,
    now: u64,
) -> PresenceProjection {
    apply(
        state,
        Observation::Snapshot {
            cursor: 1,
            operations: vec![operation(id, status)],
            entries,
            live_completed: completed.map(str::to_owned),
        },
        now,
    )
}
#[test]
fn state_table_uses_actual_operation_status_and_terminal_precedence() {
    let mut state = live();
    assert_eq!(
        snapshot(&mut state, "op-a", "Pending", vec![], None, 100).expression,
        "thinking"
    );
    assert_eq!(
        snapshot(&mut state, "op-a", "Running", vec![], None, 200).expression,
        "thinking"
    );
    assert_eq!(
        apply(&mut state, Observation::Tool("op-a".into()), 250).expression,
        "working"
    );
    assert_eq!(
        apply(&mut state, Observation::Delta("op-a".into()), 300).expression,
        "speaking"
    );
    assert_eq!(
        snapshot(&mut state, "op-a", "Completed", vec![], Some("op-a"), 400).expression,
        "success"
    );
    assert_eq!(
        apply(&mut state, Observation::Delta("op-a".into()), 500).expression,
        "success"
    );
    assert_eq!(
        apply(&mut state, Observation::Tick, 2400).expression,
        "idle"
    );
    let mut failed = live();
    assert_eq!(
        snapshot(&mut failed, "failed", "Failed", vec![], None, 0).expression,
        "blocked"
    );
    let mut interrupted = live();
    assert_eq!(
        snapshot(
            &mut interrupted,
            "interrupted",
            "Interrupted",
            vec![],
            None,
            0
        )
        .expression,
        "blocked"
    );
    let mut cancelled = live();
    assert_eq!(
        snapshot(&mut cancelled, "cancelled", "Cancelled", vec![], None, 0).status,
        "Stopped"
    );
}
#[test]
fn terminal_before_final_entry_waits() {
    let mut state = live();
    assert_eq!(
        snapshot(&mut state, "op-a", "Completed", vec![], Some("op-a"), 100).preview,
        None
    );
    assert!(state.needs_entry(200));
    let ready = snapshot(
        &mut state,
        "op-a",
        "Completed",
        vec![entry(
            "entry-final",
            "op-a",
            4,
            "**Ready** [link](https://example.com)",
        )],
        None,
        300,
    );
    assert_eq!(ready.preview.unwrap().text, "Ready link");
}
#[test]
fn terminal_is_monotonic() {
    let mut state = live();
    snapshot(&mut state, "op-a", "Failed", vec![], None, 100);
    assert_eq!(
        snapshot(&mut state, "op-a", "Running", vec![], None, 200).expression,
        "blocked"
    );
    assert_eq!(
        snapshot(&mut state, "op-a", "Completed", vec![], Some("op-a"), 300).expression,
        "blocked"
    );
}
#[test]
fn new_turn_replaces_success_timer() {
    let mut state = live();
    snapshot(&mut state, "op-a", "Completed", vec![], Some("op-a"), 100);
    let next = snapshot(&mut state, "op-b", "Pending", vec![], None, 300);
    assert_eq!(next.expression, "thinking");
    assert_eq!(next.preview, None);
    assert_eq!(
        apply(&mut state, Observation::Tick, 3000).expression,
        "thinking"
    );
}
#[test]
fn fork_provenance_is_not_a_new_reply() {
    let mut state = live();
    let forked = entry("copied", "source-operation", 99, "Historical reply");
    assert_eq!(
        snapshot(
            &mut state,
            "op-a",
            "Completed",
            vec![forked],
            Some("op-a"),
            100
        )
        .preview,
        None
    );
}
#[test]
fn initial_and_reconnected_completions_do_not_celebrate() {
    let mut state = Projection::new(scope(), true, true);
    snapshot(
        &mut state,
        "old",
        "Completed",
        vec![entry("old-entry", "old", 1, "Old reply")],
        Some("old"),
        0,
    );
    apply(&mut state, Observation::Ready, 0);
    let ready = apply(&mut state, Observation::CaughtUp, 0);
    assert_eq!(ready.expression, "idle");
    assert_eq!(ready.preview, None);
    apply(&mut state, Observation::Disconnected, 100);
    snapshot(
        &mut state,
        "missed",
        "Completed",
        vec![entry("missed-entry", "missed", 2, "Missed reply")],
        Some("missed"),
        200,
    );
    apply(&mut state, Observation::Ready, 200);
    let reconnected = apply(&mut state, Observation::CaughtUp, 200);
    assert_eq!(reconnected.preview, None);
    assert_eq!(reconnected.expression, "idle");
}
#[test]
fn unicode_preview_is_bounded() {
    let mut state = live();
    let text = "مرحبا🙂".repeat(70);
    let value = snapshot(
        &mut state,
        "op-a",
        "Completed",
        vec![entry("latest", "op-a", 4, &text)],
        Some("op-a"),
        100,
    );
    assert_eq!(value.preview.unwrap().text.chars().count(), 240);
}
#[test]
fn hidden_expiry_does_not_revive_and_repeat_id_cannot_extend_it() {
    let mut state = live();
    let first = snapshot(
        &mut state,
        "op-a",
        "Completed",
        vec![entry("final", "op-a", 5, "Ready")],
        Some("op-a"),
        100,
    );
    let repeat = snapshot(
        &mut state,
        "op-a",
        "Completed",
        vec![entry("final", "op-a", 5, "Ready")],
        Some("op-a"),
        1000,
    );
    assert_eq!(first.preview, repeat.preview);
    assert_eq!(apply(&mut state, Observation::Tick, 8100).preview, None);
    assert_eq!(
        snapshot(
            &mut state,
            "op-a",
            "Completed",
            vec![entry("final", "op-a", 5, "Ready")],
            Some("op-a"),
            9000
        )
        .preview,
        None
    );
}
#[test]
fn deleted_agent_clears_preview() {
    let mut state = live();
    snapshot(
        &mut state,
        "op-a",
        "Completed",
        vec![entry("final", "op-a", 5, "Ready")],
        Some("op-a"),
        100,
    );
    let deleted = apply(
        &mut state,
        Observation::Unavailable("Agent unavailable. Choose an agent."),
        200,
    );
    assert_eq!(deleted.expression, "blocked");
    assert_eq!(deleted.preview, None);
}
#[test]
fn provider_without_key_is_blocked() {
    let mut state = live();
    assert_eq!(
        apply(
            &mut state,
            Observation::Unavailable("Add a provider key in Settings."),
            100
        )
        .expression,
        "blocked"
    );
}
#[test]
fn unknown_status_is_not_success() {
    let mut state = live();
    assert_eq!(
        snapshot(
            &mut state,
            "op-a",
            "FutureStatus",
            vec![],
            Some("op-a"),
            100
        )
        .expression,
        "blocked"
    );
}
#[test]
fn old_generation_and_scope_updates_are_ignored() {
    let mut state = live();
    let current = snapshot(&mut state, "op-a", "Running", vec![], None, 100);
    let mut old = scope();
    old.generation -= 1;
    assert_eq!(
        state.apply(
            PresenceInput {
                scope: old,
                observation: Observation::Unavailable("Old error")
            },
            10000
        ),
        current
    );
}
#[test]
fn greatest_matching_durable_ordinal_wins_and_disabled_previews_stay_empty() {
    let mut state = live();
    let result = snapshot(
        &mut state,
        "op-a",
        "Completed",
        vec![
            entry("second", "op-a", 2, "Latest"),
            entry("first", "op-a", 1, "Earlier"),
        ],
        Some("op-a"),
        100,
    );
    assert_eq!(result.preview.unwrap().text, "Latest");
    let mut disabled = Projection::new(scope(), true, false);
    apply(&mut disabled, Observation::Ready, 0);
    apply(&mut disabled, Observation::CaughtUp, 0);
    assert_eq!(
        snapshot(
            &mut disabled,
            "op-a",
            "Completed",
            vec![entry("final", "op-a", 3, "Ready")],
            Some("op-a"),
            100
        )
        .preview,
        None
    );
}
#[test]
fn markdown_preview_omits_fenced_code_images_and_hidden_html() {
    assert_eq!(reply_preview("مرحبا **بك** `code` [رابط](https://example.com)\n\n![secret](image.png)\n\n```rs\nsecret();\n```\n\n<span>Done</span>"),Some("مرحبا بك code رابط Done".into()));
    assert_eq!(reply_preview("<script>secret()</script>"), None);
    assert_eq!(reply_preview("```unclosed\nsecret"), None);
    assert_eq!(reply_preview("broken ![image"), None);
    assert_eq!(
        reply_preview("Visible <think>hidden reasoning</think> finish"),
        Some("Visible finish".into())
    );
}

#[test]
fn preview_preserves_inline_punctuation() {
    assert_eq!(reply_preview("One **word**."), Some("One word.".into()));
    assert_eq!(
        reply_preview("Open [chat](https://example.com)."),
        Some("Open chat.".into())
    );
}
