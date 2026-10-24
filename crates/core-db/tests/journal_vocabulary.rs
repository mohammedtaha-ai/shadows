use core_db::{Actor, JournalEntry};

#[test]
fn publication_preserves_subjects_actor_causation_and_exact_payload_bytes() {
    let payload = serde_json::json!({"text": "synthetic\nvalue", "n": 7});
    let expected_payload = payload.to_string();
    let event = JournalEntry::new("FixturePublished", Actor::user("fixture-principal"))
        .with_project("project-fixture")
        .with_thread("thread-fixture")
        .with_operation("operation-fixture")
        .with_causation("Command", "command-fixture")
        .with_correlation("correlation-fixture")
        .with_payload(payload);
    assert_eq!(event.kind, "FixturePublished");
    assert_eq!(event.project_id.as_deref(), Some("project-fixture"));
    assert_eq!(event.thread_id.as_deref(), Some("thread-fixture"));
    assert_eq!(event.operation_id.as_deref(), Some("operation-fixture"));
    assert_eq!(event.actor.kind, "User");
    assert_eq!(event.actor.id, "fixture-principal");
    let causation = event.causation.unwrap();
    assert_eq!(causation.kind, "Command");
    assert_eq!(causation.reference, "command-fixture");
    assert_eq!(event.correlation_id.as_deref(), Some("correlation-fixture"));
    assert_eq!(event.payload_json.as_bytes(), expected_payload.as_bytes());
    assert_eq!(
        uuid::Uuid::parse_str(&event.event_id)
            .unwrap()
            .get_version_num(),
        4
    );
}
