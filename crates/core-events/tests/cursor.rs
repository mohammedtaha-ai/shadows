use core_events::EventCursor;

#[test]
fn event_cursor_preserves_sequence_order_and_serialization() {
    let cursor: EventCursor = serde_json::from_str("42").unwrap();
    assert_eq!(cursor, EventCursor(42));
    assert!(EventCursor(41) < cursor);
    assert_eq!(serde_json::to_string(&cursor).unwrap(), "42");
}
