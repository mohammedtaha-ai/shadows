use shadows::agent::{AgentHarness, AgentInvocation, StreamItem, claude::ClaudeHarness};
use shadows::operation::OperationId;

fn harness() -> ClaudeHarness {
    ClaudeHarness::new("claude".into(), "2.1.278".into())
}

/// Spec §1.4 and the harness evidence report: the executable comes from
/// configuration, and the measured flags are part of the contract.
#[test]
fn the_invocation_uses_the_measured_flags_and_the_configured_executable() {
    let spec = harness().to_process_spec(&AgentInvocation {
        operation_id: OperationId::from_literal("op-1"),
        role: "Planner".into(),
        model: "sonnet".into(),
        prompt: "hello".into(),
        cwd: std::env::temp_dir(),
        resume_session_id: None,
        session_id: "00000000-0000-4000-8000-000000000001".into(),
    });

    assert_eq!(spec.executable, std::path::PathBuf::from("claude"));
    let args = spec.args.join(" ");
    for required in [
        "--print",
        "--output-format stream-json",
        "--verbose",
        "--include-partial-messages",
        "--permission-prompts none",
        "--session-id 00000000-0000-4000-8000-000000000001",
    ] {
        assert!(args.contains(required), "missing {required} in: {args}");
    }
    assert!(!args.contains("--resume"), "a first turn must not resume");
    assert!(spec.capture_stdout);
}

/// A resumed turn carries --resume and not --session-id. `--resume` gives
/// continuity across separate OS processes and does not re-emit history.
#[test]
fn a_resumed_turn_uses_resume_instead_of_session_id() {
    let spec = harness().to_process_spec(&AgentInvocation {
        operation_id: OperationId::from_literal("op-2"),
        role: "Planner".into(),
        model: "sonnet".into(),
        prompt: "again".into(),
        cwd: std::env::temp_dir(),
        resume_session_id: Some("00000000-0000-4000-8000-000000000001".into()),
        session_id: "00000000-0000-4000-8000-000000000001".into(),
    });
    let args = spec.args.join(" ");
    assert!(args.contains("--resume 00000000-0000-4000-8000-000000000001"));
    assert!(!args.contains("--session-id"));
}

/// The durable/transient split, against a real captured turn. A `stream_event`
/// never carries information the following `assistant` line does not also
/// carry, so deltas are forwarded and never stored.
#[test]
fn a_real_turn_classifies_into_transient_durable_and_terminal() {
    let raw = std::fs::read_to_string("tests/fixtures/claude_turn.jsonl")
        .expect("fixture must exist; capture it with the command in Step 1");
    let h = harness();

    let mut deltas = 0;
    let mut durable = 0;
    let mut terminal = 0;
    let mut session_id: Option<String> = None;

    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        match h.classify(line) {
            StreamItem::Delta { .. } => deltas += 1,
            StreamItem::Entry { .. } => durable += 1,
            StreamItem::TurnEnd { .. } => terminal += 1,
            StreamItem::Operational { session, .. } => {
                if session_id.is_none() {
                    session_id = session;
                }
            }
            StreamItem::Unparsed(l) => panic!("classifier failed on a real line: {l}"),
        }
    }

    assert!(
        durable >= 1,
        "a real turn produces at least one durable entry"
    );
    assert_eq!(terminal, 1, "exactly one result line, always last");
    assert!(session_id.is_some(), "system/init carries the session id");
    assert!(
        deltas >= 1,
        "the transient class must be recognised on a real turn. This count was \
         discarded until a review disabled the Delta arm outright and all four \
         tests still passed."
    );
}

/// The decisive property of the whole stream contract, from the harness evidence
/// report: a `stream_event` never carries information the following `assistant`
/// line does not also carry. That is what lets the daemon forward deltas
/// straight to SSE without touching storage, and write a `ThreadEntry` only when
/// the durable line arrives.
///
/// If this ever stops holding, the daemon is dropping text the user saw stream
/// past and the durable history no longer matches what was displayed — silently,
/// because nothing else in this suite compares the two classes against each
/// other.
#[test]
fn the_deltas_reassemble_into_exactly_the_durable_entry_text() {
    let raw = std::fs::read_to_string("tests/fixtures/claude_turn.jsonl")
        .expect("fixture must exist; capture it with the command in Step 1");
    let h = harness();

    let mut streamed = String::new();
    let mut durable: Vec<String> = Vec::new();

    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        match h.classify(line) {
            StreamItem::Delta { text } => streamed.push_str(&text),
            StreamItem::Entry { role, text, .. } if role == "assistant" => durable.push(text),
            _ => {}
        }
    }

    assert!(!streamed.is_empty(), "the fixture carries no text deltas");
    assert_eq!(
        streamed,
        durable.concat(),
        "the streamed text and the durable text diverged, so one of them is a lie"
    );
}

/// Turn end is an explicit line, not a heuristic, and it carries a structured
/// verdict the daemon cross-checks against the process exit.
#[test]
fn turn_end_is_the_result_line_and_carries_its_verdict() {
    let raw = std::fs::read_to_string("tests/fixtures/claude_turn.jsonl").unwrap();
    let last = raw.lines().rfind(|l| !l.trim().is_empty()).unwrap();
    match harness().classify(last) {
        StreamItem::TurnEnd {
            subtype,
            stop_reason,
        } => {
            assert_eq!(subtype, "success");
            assert_eq!(stop_reason.as_deref(), Some("end_turn"));
        }
        other => panic!("the last line must be the turn end, got {other:?}"),
    }
}
